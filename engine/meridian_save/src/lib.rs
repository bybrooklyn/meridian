//! Versioned, checksummed, crash-conscious save storage.
//!
//! This crate owns the persistence boundary only. Game state remains an
//! opaque payload so the engine can provide atomic replacement and recovery
//! without depending on any consumer game's content types.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"MSAV";
const HEADER_SIZE: usize = 4 + 4 + 8 + 8;
const DEFAULT_MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const JOURNAL_MAGIC: &[u8; 4] = b"MJNL";
const JOURNAL_FORMAT_VERSION: u32 = 1;
const JOURNAL_HEADER_SIZE: usize = 4 + 4 + 8 + 8 + 8;
const DEFAULT_MAX_JOURNAL_ENTRY_BYTES: usize = 64 * 1024;

/// Configuration for one save slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaveConfig {
    pub schema_version: u32,
    pub max_payload_bytes: usize,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
        }
    }
}

/// Self-describing save payload with a deterministic integrity checksum.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveEnvelope {
    schema_version: u32,
    payload: Vec<u8>,
    checksum: u64,
}

impl SaveEnvelope {
    #[must_use]
    pub fn new(schema_version: u32, payload: impl Into<Vec<u8>>) -> Self {
        let payload = payload.into();
        Self {
            schema_version,
            checksum: checksum(&payload),
            payload,
        }
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[must_use]
    pub const fn checksum(&self) -> u64 {
        self.checksum
    }

    /// Encodes the envelope into the on-disk format.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = u64::try_from(self.payload.len()).unwrap_or(u64::MAX);
        let mut encoded = Vec::with_capacity(HEADER_SIZE.saturating_add(self.payload.len()));
        encoded.extend_from_slice(MAGIC);
        encoded.extend_from_slice(&self.schema_version.to_le_bytes());
        encoded.extend_from_slice(&payload_len.to_le_bytes());
        encoded.extend_from_slice(&self.checksum.to_le_bytes());
        encoded.extend_from_slice(&self.payload);
        encoded
    }

    /// Decodes and integrity-checks one envelope.
    ///
    /// # Errors
    ///
    /// Returns a structured format, size, or checksum error when the bytes do
    /// not describe a valid save envelope.
    pub fn decode(bytes: &[u8], max_payload_bytes: usize) -> Result<Self, SaveError> {
        if bytes.len() < HEADER_SIZE {
            return Err(SaveError::InvalidFormat(
                "save is shorter than its fixed header",
            ));
        }
        if &bytes[..MAGIC.len()] != MAGIC {
            return Err(SaveError::InvalidFormat("save magic does not match"));
        }

        let schema_version = u32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| SaveError::InvalidFormat("schema version header is malformed"))?,
        );
        let payload_len = u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| SaveError::InvalidFormat("payload length header is malformed"))?,
        );
        let expected_checksum = u64::from_le_bytes(
            bytes[16..24]
                .try_into()
                .map_err(|_| SaveError::InvalidFormat("checksum header is malformed"))?,
        );
        let payload_len = usize::try_from(payload_len).map_err(|_| SaveError::PayloadTooLarge {
            size: usize::MAX,
            max: max_payload_bytes,
        })?;
        if payload_len > max_payload_bytes {
            return Err(SaveError::PayloadTooLarge {
                size: payload_len,
                max: max_payload_bytes,
            });
        }
        let expected_size = HEADER_SIZE
            .checked_add(payload_len)
            .ok_or(SaveError::InvalidFormat("save length overflows host size"))?;
        if bytes.len() != expected_size {
            return Err(SaveError::InvalidFormat(
                "save length does not match its payload length",
            ));
        }

        let payload = bytes[HEADER_SIZE..].to_vec();
        let actual_checksum = checksum(&payload);
        if actual_checksum != expected_checksum {
            return Err(SaveError::ChecksumMismatch {
                expected: expected_checksum,
                actual: actual_checksum,
            });
        }

        Ok(Self {
            schema_version,
            payload,
            checksum: expected_checksum,
        })
    }
}

type MigrationStep = Box<dyn Fn(Vec<u8>) -> Result<Vec<u8>, SaveError> + Send + Sync + 'static>;

/// Ordered one-schema-version-at-a-time save migrations.
#[derive(Default)]
pub struct SaveMigrations {
    steps: BTreeMap<u32, MigrationStep>,
}

impl SaveMigrations {
    /// Adds a migration from `from_version` to the immediately following version.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::DuplicateMigration`] when a step has already been
    /// registered for the source version.
    pub fn add<F>(&mut self, from_version: u32, migration: F) -> Result<(), SaveError>
    where
        F: Fn(Vec<u8>) -> Result<Vec<u8>, SaveError> + Send + Sync + 'static,
    {
        if self
            .steps
            .insert(from_version, Box::new(migration))
            .is_some()
        {
            return Err(SaveError::DuplicateMigration { from_version });
        }
        Ok(())
    }

    fn apply(
        &self,
        from_version: u32,
        target_version: u32,
        mut payload: Vec<u8>,
    ) -> Result<Vec<u8>, SaveError> {
        if from_version > target_version {
            return Err(SaveError::UnsupportedVersion {
                expected: target_version,
                actual: from_version,
            });
        }

        let mut version = from_version;
        while version < target_version {
            let migration = self
                .steps
                .get(&version)
                .ok_or(SaveError::MigrationMissing {
                    from_version: version,
                })?;
            payload = migration(payload).map_err(|error| SaveError::MigrationFailed {
                from_version: version,
                message: error.to_string(),
            })?;
            version = version.saturating_add(1);
        }
        Ok(payload)
    }
}

/// One opaque, ordered recent-change record from a save journal.
#[derive(Clone, Debug, PartialEq)]
pub struct JournalEntry {
    sequence: u64,
    payload: Vec<u8>,
}

impl JournalEntry {
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Result of replaying a journal.
#[derive(Clone, Debug, PartialEq)]
pub struct JournalReplay {
    entries: Vec<JournalEntry>,
    truncated_tail: bool,
}

impl JournalReplay {
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    #[must_use]
    pub const fn truncated_tail(&self) -> bool {
        self.truncated_tail
    }
}

/// Append-only persistence for small recent changes.
///
/// Each record is independently framed and checksummed. A partial final
/// record is treated as an interrupted write and ignored during replay; a
/// later append repairs the file by truncating that incomplete tail first.
pub struct SaveJournal {
    path: PathBuf,
    max_entry_bytes: usize,
}

impl SaveJournal {
    /// Creates a journal with the default 64 KiB per-record limit.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_max_entry_bytes(path, DEFAULT_MAX_JOURNAL_ENTRY_BYTES)
    }

    /// Creates a journal with an explicit per-record payload limit.
    #[must_use]
    pub fn with_max_entry_bytes(path: impl Into<PathBuf>, max_entry_bytes: usize) -> Self {
        Self {
            path: path.into(),
            max_entry_bytes,
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one opaque change and returns its monotonically increasing ID.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::PayloadTooLarge`] when the change exceeds the
    /// configured bound, or [`JournalError::Io`] when the journal cannot be
    /// prepared, repaired, written, or synced.
    pub fn append(&self, payload: impl AsRef<[u8]>) -> Result<u64, JournalError> {
        let payload = payload.as_ref();
        if payload.len() > self.max_entry_bytes {
            return Err(JournalError::PayloadTooLarge {
                size: payload.len(),
                max: self.max_entry_bytes,
            });
        }
        ensure_parent_directory_for_journal(&self.path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| JournalError::io("open", &error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| JournalError::io("read", &error))?;
        let scan = scan_journal(&bytes, self.max_entry_bytes)?;
        file.set_len(scan.valid_length as u64)
            .map_err(|error| JournalError::io("repair", &error))?;
        file.seek(SeekFrom::End(0))
            .map_err(|error| JournalError::io("seek", &error))?;

        let sequence = scan
            .entries
            .last()
            .map_or(1, |entry| entry.sequence.checked_add(1).unwrap_or(0));
        if sequence == 0 {
            return Err(JournalError::SequenceExhausted);
        }
        let encoded = encode_journal_entry(sequence, payload)?;
        file.write_all(&encoded)
            .map_err(|error| JournalError::io("append", &error))?;
        file.sync_all()
            .map_err(|error| JournalError::io("sync", &error))?;
        Ok(sequence)
    }

    /// Replays all complete entries in order.
    ///
    /// An incomplete final frame is recoverable and is reported through
    /// [`JournalReplay::truncated_tail`]. Complete frames with invalid magic,
    /// version, sequence, size, or checksum are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError`] when the journal cannot be read or contains a
    /// complete but invalid record.
    pub fn replay(&self) -> Result<JournalReplay, JournalError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(JournalReplay {
                    entries: Vec::new(),
                    truncated_tail: false,
                });
            }
            Err(error) => return Err(JournalError::io("read", &error)),
        };
        let scan = scan_journal(&bytes, self.max_entry_bytes)?;
        Ok(JournalReplay {
            entries: scan.entries,
            truncated_tail: scan.valid_length != bytes.len(),
        })
    }
}

struct JournalScan {
    entries: Vec<JournalEntry>,
    valid_length: usize,
}

fn encode_journal_entry(sequence: u64, payload: &[u8]) -> Result<Vec<u8>, JournalError> {
    let payload_length =
        u64::try_from(payload.len()).map_err(|_| JournalError::PayloadTooLarge {
            size: payload.len(),
            max: usize::MAX,
        })?;
    let capacity =
        JOURNAL_HEADER_SIZE
            .checked_add(payload.len())
            .ok_or(JournalError::InvalidFormat(
                "journal entry size overflows host size",
            ))?;
    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(JOURNAL_MAGIC);
    encoded.extend_from_slice(&JOURNAL_FORMAT_VERSION.to_le_bytes());
    encoded.extend_from_slice(&sequence.to_le_bytes());
    encoded.extend_from_slice(&payload_length.to_le_bytes());
    encoded.extend_from_slice(&checksum(payload).to_le_bytes());
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

fn scan_journal(bytes: &[u8], max_entry_bytes: usize) -> Result<JournalScan, JournalError> {
    let mut entries = Vec::new();
    let mut offset = 0;
    let mut expected_sequence = 1;
    while offset < bytes.len() {
        let remaining = bytes.len() - offset;
        if remaining < JOURNAL_HEADER_SIZE {
            return Ok(JournalScan {
                entries,
                valid_length: offset,
            });
        }
        let header = &bytes[offset..offset + JOURNAL_HEADER_SIZE];
        if &header[..JOURNAL_MAGIC.len()] != JOURNAL_MAGIC {
            return Err(JournalError::InvalidFormat("journal magic does not match"));
        }
        let version = u32::from_le_bytes(
            header[4..8]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat("journal version is malformed"))?,
        );
        if version != JOURNAL_FORMAT_VERSION {
            return Err(JournalError::UnsupportedVersion {
                expected: JOURNAL_FORMAT_VERSION,
                actual: version,
            });
        }
        let sequence = u64::from_le_bytes(
            header[8..16]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat("journal sequence is malformed"))?,
        );
        if sequence != expected_sequence {
            return Err(JournalError::NonContiguousSequence {
                expected: expected_sequence,
                actual: sequence,
            });
        }
        let payload_length = u64::from_le_bytes(
            header[16..24]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat("journal length is malformed"))?,
        );
        let payload_length =
            usize::try_from(payload_length).map_err(|_| JournalError::PayloadTooLarge {
                size: usize::MAX,
                max: max_entry_bytes,
            })?;
        if payload_length > max_entry_bytes {
            return Err(JournalError::PayloadTooLarge {
                size: payload_length,
                max: max_entry_bytes,
            });
        }
        let expected_checksum = u64::from_le_bytes(
            header[24..32]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat("journal checksum is malformed"))?,
        );
        let record_length = JOURNAL_HEADER_SIZE
            .checked_add(payload_length)
            .ok_or(JournalError::InvalidFormat("journal record size overflows"))?;
        let Some(record_end) = offset.checked_add(record_length) else {
            return Err(JournalError::InvalidFormat("journal record end overflows"));
        };
        if record_end > bytes.len() {
            return Ok(JournalScan {
                entries,
                valid_length: offset,
            });
        }
        let payload = bytes[offset + JOURNAL_HEADER_SIZE..record_end].to_vec();
        let actual_checksum = checksum(&payload);
        if actual_checksum != expected_checksum {
            return Err(JournalError::ChecksumMismatch {
                sequence,
                expected: expected_checksum,
                actual: actual_checksum,
            });
        }
        entries.push(JournalEntry { sequence, payload });
        expected_sequence = sequence
            .checked_add(1)
            .ok_or(JournalError::SequenceExhausted)?;
        offset = record_end;
    }
    Ok(JournalScan {
        entries,
        valid_length: offset,
    })
}

fn ensure_parent_directory_for_journal(path: &Path) -> Result<(), JournalError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| JournalError::io("create parent", &error))?;
        }
    }
    Ok(())
}

/// File-backed save slot with atomic replacement and previous-save recovery.
pub struct SaveStore {
    path: PathBuf,
    config: SaveConfig,
}

impl SaveStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, config: SaveConfig) -> Self {
        Self {
            path: path.into(),
            config,
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn backup_path(&self) -> PathBuf {
        suffixed_path(&self.path, ".bak")
    }

    /// Atomically replaces the primary save after syncing its temporary file.
    /// The previous primary is copied to the backup before replacement.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::Io`] when directory creation, writing, syncing,
    /// backup, or replacement fails.
    pub fn save(&self, payload: impl AsRef<[u8]>) -> Result<(), SaveError> {
        let payload = payload.as_ref();
        if payload.len() > self.config.max_payload_bytes {
            return Err(SaveError::PayloadTooLarge {
                size: payload.len(),
                max: self.config.max_payload_bytes,
            });
        }
        ensure_parent_directory(&self.path)?;

        let encoded = SaveEnvelope::new(self.config.schema_version, payload).encode();
        let temporary_path = suffixed_path(&self.path, ".tmp");
        write_synced_file(&temporary_path, &encoded)?;

        if self.path.exists() {
            fs::copy(&self.path, self.backup_path())
                .map_err(|error| SaveError::io("backup", &error))?;
        }

        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(SaveError::io("replace", &error));
        }
        Ok(())
    }

    /// Loads the primary save, falling back to the previous-save backup when
    /// the primary is missing, corrupt, or otherwise unreadable.
    ///
    /// # Errors
    ///
    /// Returns the primary error when no backup exists, or
    /// [`SaveError::RecoveryFailed`] when both copies fail.
    pub fn load(&self) -> Result<Vec<u8>, SaveError> {
        let envelope = self.load_envelope_with_recovery()?;
        ensure_schema_version(envelope.schema_version, self.config.schema_version)?;
        Ok(envelope.payload)
    }

    /// Loads an older save by applying one registered migration per version.
    /// The migrated result is returned but is not implicitly rewritten.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::MigrationMissing`] when a required step is absent,
    /// or [`SaveError::UnsupportedVersion`] for a save newer than this store.
    pub fn load_with_migrations(&self, migrations: &SaveMigrations) -> Result<Vec<u8>, SaveError> {
        let envelope = self.load_envelope_with_recovery()?;
        if envelope.schema_version > self.config.schema_version {
            return Err(SaveError::UnsupportedVersion {
                expected: self.config.schema_version,
                actual: envelope.schema_version,
            });
        }
        let payload = migrations.apply(
            envelope.schema_version,
            self.config.schema_version,
            envelope.payload,
        )?;
        if payload.len() > self.config.max_payload_bytes {
            return Err(SaveError::PayloadTooLarge {
                size: payload.len(),
                max: self.config.max_payload_bytes,
            });
        }
        Ok(payload)
    }

    fn load_envelope_with_recovery(&self) -> Result<SaveEnvelope, SaveError> {
        let primary = read_envelope(&self.path, self.config.max_payload_bytes);
        match primary {
            Ok(envelope) => Ok(envelope),
            Err(primary_error) if !self.backup_path().exists() => Err(primary_error),
            Err(primary_error) => {
                match read_envelope(&self.backup_path(), self.config.max_payload_bytes) {
                    Ok(envelope) => Ok(envelope),
                    Err(backup_error) => Err(SaveError::RecoveryFailed {
                        primary: Box::new(primary_error),
                        backup: Box::new(backup_error),
                    }),
                }
            }
        }
    }
}

fn read_envelope(path: &Path, max_payload_bytes: usize) -> Result<SaveEnvelope, SaveError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|error| SaveError::io("open", &error))?
        .read_to_end(&mut bytes)
        .map_err(|error| SaveError::io("read", &error))?;
    SaveEnvelope::decode(&bytes, max_payload_bytes)
}

fn ensure_schema_version(actual: u32, expected: u32) -> Result<(), SaveError> {
    if actual != expected {
        return Err(SaveError::UnsupportedVersion { expected, actual });
    }
    Ok(())
}

fn ensure_parent_directory(path: &Path) -> Result<(), SaveError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| SaveError::io("create parent", &error))?;
        }
    }
    Ok(())
}

fn write_synced_file(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| SaveError::io("open temporary", &error))?;
    file.write_all(bytes)
        .map_err(|error| SaveError::io("write temporary", &error))?;
    file.sync_all()
        .map_err(|error| SaveError::io("sync temporary", &error))?;
    Ok(())
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[derive(Debug)]
pub enum SaveError {
    Io {
        operation: &'static str,
        message: String,
    },
    InvalidFormat(&'static str),
    PayloadTooLarge {
        size: usize,
        max: usize,
    },
    ChecksumMismatch {
        expected: u64,
        actual: u64,
    },
    UnsupportedVersion {
        expected: u32,
        actual: u32,
    },
    DuplicateMigration {
        from_version: u32,
    },
    MigrationMissing {
        from_version: u32,
    },
    MigrationFailed {
        from_version: u32,
        message: String,
    },
    RecoveryFailed {
        primary: Box<Self>,
        backup: Box<Self>,
    },
}

/// Errors raised while appending or replaying a save journal.
#[derive(Debug, PartialEq, Eq)]
pub enum JournalError {
    Io {
        operation: &'static str,
        message: String,
    },
    InvalidFormat(&'static str),
    PayloadTooLarge {
        size: usize,
        max: usize,
    },
    UnsupportedVersion {
        expected: u32,
        actual: u32,
    },
    NonContiguousSequence {
        expected: u64,
        actual: u64,
    },
    ChecksumMismatch {
        sequence: u64,
        expected: u64,
        actual: u64,
    },
    SequenceExhausted,
}

impl JournalError {
    fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            message: error.to_string(),
        }
    }
}

impl Display for JournalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, message } => {
                write!(formatter, "journal {operation} failed: {message}")
            }
            Self::InvalidFormat(message) => write!(formatter, "invalid journal format: {message}"),
            Self::PayloadTooLarge { size, max } => {
                write!(formatter, "journal entry is {size} bytes; maximum is {max}")
            }
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported journal version {actual}; expected {expected}"
            ),
            Self::NonContiguousSequence { expected, actual } => write!(
                formatter,
                "journal sequence is not contiguous: expected {expected}, got {actual}"
            ),
            Self::ChecksumMismatch {
                sequence,
                expected,
                actual,
            } => write!(
                formatter,
                "journal checksum mismatch at sequence {sequence}: expected {expected:#x}, got {actual:#x}"
            ),
            Self::SequenceExhausted => formatter.write_str("journal sequence is exhausted"),
        }
    }
}

impl Error for JournalError {}

impl SaveError {
    fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            message: error.to_string(),
        }
    }
}

impl Display for SaveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, message } => {
                write!(formatter, "save {operation} failed: {message}")
            }
            Self::InvalidFormat(message) => write!(formatter, "invalid save format: {message}"),
            Self::PayloadTooLarge { size, max } => {
                write!(formatter, "save payload is {size} bytes; maximum is {max}")
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "save checksum mismatch: expected {expected:#x}, got {actual:#x}"
            ),
            Self::UnsupportedVersion { expected, actual } => write!(
                formatter,
                "unsupported save schema version {actual}; expected {expected}"
            ),
            Self::DuplicateMigration { from_version } => {
                write!(
                    formatter,
                    "save migration from version {from_version} is duplicated"
                )
            }
            Self::MigrationMissing { from_version } => write!(
                formatter,
                "save migration from version {from_version} is missing"
            ),
            Self::MigrationFailed {
                from_version,
                message,
            } => write!(
                formatter,
                "save migration from version {from_version} failed: {message}"
            ),
            Self::RecoveryFailed { primary, backup } => write!(
                formatter,
                "primary save and backup recovery failed: primary={primary}; backup={backup}"
            ),
        }
    }
}

impl Error for SaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TemporaryDirectory {
        path: PathBuf,
    }

    impl TemporaryDirectory {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("meridian-save-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).expect("temporary test directory creates");
            Self { path }
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn store() -> (TemporaryDirectory, SaveStore) {
        let directory = TemporaryDirectory::new();
        let save = SaveStore::new(directory.path.join("slot.save"), SaveConfig::default());
        (directory, save)
    }

    #[test]
    fn envelope_round_trips_and_detects_corruption() {
        let envelope = SaveEnvelope::new(7, b"player-state".to_vec());
        let encoded = envelope.encode();
        let decoded = SaveEnvelope::decode(&encoded, 1024).expect("valid envelope decodes");

        assert_eq!(decoded, envelope);
        let mut corrupt = encoded;
        corrupt[HEADER_SIZE] ^= 1;
        assert!(matches!(
            SaveEnvelope::decode(&corrupt, 1024),
            Err(SaveError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn store_replaces_atomically_and_recovers_previous_save() {
        let (_directory, store) = store();
        store.save(b"first").expect("first save succeeds");
        store.save(b"second").expect("second save succeeds");
        assert!(!suffixed_path(store.path(), ".tmp").exists());
        assert_eq!(
            fs::read(store.backup_path()).expect("backup exists").len(),
            29
        );

        fs::write(store.path(), b"corrupt").expect("primary can be corrupted for test");
        assert_eq!(store.load().expect("backup recovers"), b"first");
    }

    #[test]
    fn journal_appends_in_order_and_repairs_a_partial_tail() {
        let directory = TemporaryDirectory::new();
        let journal = SaveJournal::new(directory.path.join("slot.journal"));

        assert_eq!(journal.append(b"first").expect("first entry appends"), 1);
        let mut file = OpenOptions::new()
            .append(true)
            .open(journal.path())
            .expect("journal opens for simulated interrupted write");
        file.write_all(b"MJ")
            .expect("partial journal header writes");

        let replay = journal.replay().expect("partial tail is recoverable");
        assert!(replay.truncated_tail());
        assert_eq!(replay.entries().len(), 1);
        assert_eq!(replay.entries()[0].sequence(), 1);
        assert_eq!(replay.entries()[0].payload(), b"first");

        assert_eq!(journal.append(b"second").expect("tail is repaired"), 2);
        let replay = journal.replay().expect("repaired journal replays");
        assert!(!replay.truncated_tail());
        assert_eq!(
            replay
                .entries()
                .iter()
                .map(JournalEntry::payload)
                .collect::<Vec<_>>(),
            [b"first".as_slice(), b"second".as_slice()]
        );
    }

    #[test]
    fn journal_rejects_complete_checksum_corruption_and_oversized_entries() {
        let directory = TemporaryDirectory::new();
        let journal = SaveJournal::with_max_entry_bytes(directory.path.join("slot.journal"), 4);
        assert!(matches!(
            journal.append(b"five!"),
            Err(JournalError::PayloadTooLarge { size: 5, max: 4 })
        ));
        journal.append(b"safe").expect("entry appends");

        let mut bytes = fs::read(journal.path()).expect("journal bytes read");
        bytes[JOURNAL_HEADER_SIZE] ^= 1;
        fs::write(journal.path(), bytes).expect("corrupted journal writes");
        assert!(matches!(
            journal.replay(),
            Err(JournalError::ChecksumMismatch { sequence: 1, .. })
        ));
    }

    #[test]
    fn version_mismatch_is_rejected_and_payload_limit_is_enforced() {
        let (_directory, store) = store();
        let envelope = SaveEnvelope::new(2, b"future".to_vec());
        fs::write(store.path(), envelope.encode()).expect("future save writes");

        assert!(matches!(
            store.load(),
            Err(SaveError::UnsupportedVersion {
                expected: 1,
                actual: 2
            })
        ));

        let limited = SaveStore::new(
            store.path(),
            SaveConfig {
                schema_version: 1,
                max_payload_bytes: 2,
            },
        );
        assert!(matches!(
            limited.save(b"too large"),
            Err(SaveError::PayloadTooLarge { size: 9, max: 2 })
        ));
    }

    #[test]
    fn older_save_can_be_migrated_one_version_at_a_time_without_rewrite() {
        let directory = TemporaryDirectory::new();
        let store = SaveStore::new(
            directory.path.join("slot.save"),
            SaveConfig {
                schema_version: 3,
                ..SaveConfig::default()
            },
        );
        fs::write(store.path(), SaveEnvelope::new(1, b"v1".to_vec()).encode())
            .expect("old save writes");

        let mut migrations = SaveMigrations::default();
        migrations
            .add(1, |mut payload| {
                payload.extend_from_slice(b"-v2");
                Ok(payload)
            })
            .expect("first migration registers");
        migrations
            .add(2, |mut payload| {
                payload.extend_from_slice(b"-v3");
                Ok(payload)
            })
            .expect("second migration registers");

        assert_eq!(
            store
                .load_with_migrations(&migrations)
                .expect("old save migrates"),
            b"v1-v2-v3"
        );
        assert_eq!(
            SaveEnvelope::decode(
                &fs::read(store.path()).expect("save remains readable"),
                1024
            )
            .expect("envelope remains valid")
            .schema_version(),
            1
        );
    }

    #[test]
    fn missing_migration_is_reported_and_duplicate_steps_are_rejected() {
        let directory = TemporaryDirectory::new();
        let store = SaveStore::new(
            directory.path.join("slot.save"),
            SaveConfig {
                schema_version: 2,
                ..SaveConfig::default()
            },
        );
        fs::write(store.path(), SaveEnvelope::new(1, b"v1".to_vec()).encode())
            .expect("old save writes");

        let mut migrations = SaveMigrations::default();
        migrations.add(1, Ok).expect("first migration registers");
        assert!(matches!(
            migrations.add(1, Ok),
            Err(SaveError::DuplicateMigration { from_version: 1 })
        ));

        let empty = SaveMigrations::default();
        assert!(matches!(
            store.load_with_migrations(&empty),
            Err(SaveError::MigrationMissing { from_version: 1 })
        ));
    }
}
