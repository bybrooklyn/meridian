//! Runtime asset identity, metadata, and lifecycle contracts.
//!
//! Pack IO and compression are kept behind small runtime-facing traits so the
//! engine can load built assets without depending on source files.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use meridian_core::StableId;
use serde::{Deserialize, Serialize};

/// Stable source-document identity, separate from compiled artifact identity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceId(StableId);

impl SourceId {
    #[must_use]
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn from_canonical_name(name: &str) -> Self {
        let digest = blake3::hash(name.as_bytes());
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest.as_bytes()[..16]);
        Self(StableId::new(u128::from_le_bytes(bytes)))
    }

    #[must_use]
    pub const fn stable_id(self) -> StableId {
        self.0
    }
}

impl Display for SourceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

/// BLAKE3 content identity used behind Meridian-owned package/data APIs.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ArtifactHash([u8; 32]);

impl ArtifactHash {
    #[must_use]
    pub fn digest(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parses a lowercase or uppercase 64-digit hexadecimal digest.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-64-digit or non-hexadecimal value.
    pub fn from_hex(value: &str) -> Result<Self, ArtifactHashParseError> {
        if value.len() != 64 {
            return Err(ArtifactHashParseError::InvalidLength(value.len()));
        }
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
                .map_err(|_| ArtifactHashParseError::InvalidHex { offset })?;
        }
        Ok(Self(bytes))
    }
}

impl Display for ArtifactHash {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactHashParseError {
    InvalidLength(usize),
    InvalidHex { offset: usize },
}

impl Display for ArtifactHashParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(length) => {
                write!(formatter, "artifact hash length is {length}; expected 64")
            }
            Self::InvalidHex { offset } => write!(
                formatter,
                "artifact hash has invalid hexadecimal at byte offset {offset}"
            ),
        }
    }
}

impl Error for ArtifactHashParseError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAuthority {
    ProjectSource,
    EngineFixture,
    GeneratedSource,
    DerivedCache,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceProvenance {
    pub origin: String,
    pub license: String,
    pub attribution: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMetadata {
    pub source_id: SourceId,
    pub schema: String,
    pub schema_version: u32,
    pub authority: SourceAuthority,
    pub importer_version: String,
    pub source_hash: ArtifactHash,
    pub dependencies: Vec<SourceId>,
    pub provenance: SourceProvenance,
}

/// Stable content identity used by runtime-facing asset references.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetId(u128);

impl AssetId {
    #[must_use]
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    /// Derives a repeatable ID from a canonical asset name.
    ///
    /// The name must be normalized by the asset database before calling this
    /// function. This is an identity helper, not a cryptographic hash.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let mut high = 0xcbf2_9ce4_8422_2325_u64;
        let mut low = 0x8422_2325_cbf2_9ce4_u64;
        for byte in name.bytes() {
            high ^= u64::from(byte);
            high = high.wrapping_mul(0x0100_0000_01b3);
            low ^= u64::from(byte).rotate_left(1);
            low = low.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self((u128::from(high) << 64) | u128::from(low))
    }

    #[must_use]
    pub const fn value(self) -> u128 {
        self.0
    }
}

impl Display for AssetId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AssetKind {
    Texture,
    Mesh,
    Material,
    Audio,
    Shader,
    WorldCell,
    Other,
}

impl AssetKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::Mesh => "mesh",
            Self::Material => "material",
            Self::Audio => "audio",
            Self::Shader => "shader",
            Self::WorldCell => "world-cell",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetDependency {
    pub id: AssetId,
    pub required: bool,
}

impl AssetDependency {
    #[must_use]
    pub const fn required(id: AssetId) -> Self {
        Self { id, required: true }
    }

    #[must_use]
    pub const fn optional(id: AssetId) -> Self {
        Self {
            id,
            required: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub name: String,
    pub kind: AssetKind,
    pub source_path: String,
    pub source_hash: String,
    pub importer_version: String,
    pub dependencies: Vec<AssetDependency>,
    pub runtime_tags: Vec<String>,
}

impl AssetMetadata {
    #[must_use]
    pub fn new(
        id: AssetId,
        name: impl Into<String>,
        kind: AssetKind,
        source_path: impl Into<String>,
        source_hash: impl Into<String>,
        importer_version: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            source_path: source_path.into(),
            source_hash: source_hash.into(),
            importer_version: importer_version.into(),
            dependencies: Vec::new(),
            runtime_tags: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_dependencies(
        mut self,
        dependencies: impl IntoIterator<Item = AssetDependency>,
    ) -> Self {
        self.dependencies = dependencies.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_runtime_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.runtime_tags = tags.into_iter().collect();
        self
    }
}

/// Compression recorded for one independently addressable pack chunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetCompression {
    None,
    Zstandard,
}

impl AssetCompression {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Zstandard => "zstd",
        }
    }
}

/// Location and integrity metadata for one built asset chunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackIndexEntry {
    pub asset_id: AssetId,
    pub offset: u64,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compression: AssetCompression,
    pub content_hash: String,
}

impl PackIndexEntry {
    #[must_use]
    pub fn new(
        asset_id: AssetId,
        offset: u64,
        compressed_size: u64,
        uncompressed_size: u64,
        compression: AssetCompression,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            asset_id,
            offset,
            compressed_size,
            uncompressed_size,
            compression,
            content_hash: content_hash.into(),
        }
    }
}

/// Deterministic lookup table for independently readable pack chunks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackIndex {
    pub format_version: u32,
    pub pack_name: String,
    entries: BTreeMap<AssetId, PackIndexEntry>,
}

impl PackIndex {
    #[must_use]
    pub fn new(format_version: u32, pack_name: impl Into<String>) -> Self {
        Self {
            format_version,
            pack_name: pack_name.into(),
            entries: BTreeMap::new(),
        }
    }

    /// Adds one chunk location without replacing an existing identity.
    ///
    /// # Errors
    ///
    /// Returns [`PackIndexError::DuplicateId`] when the asset already has an
    /// entry in this pack.
    pub fn insert(&mut self, entry: PackIndexEntry) -> Result<(), PackIndexError> {
        let id = entry.asset_id;
        if self.entries.contains_key(&id) {
            return Err(PackIndexError::DuplicateId(id));
        }
        self.entries.insert(id, entry);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&PackIndexEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Emits stable line-oriented data suitable for hashing and build checks.
    #[must_use]
    pub fn canonical_text(&self) -> String {
        let mut output = format!(
            "meridian-pack-index-v{}\npack={}\n",
            self.format_version,
            escape_field(&self.pack_name)
        );
        for entry in self.entries.values() {
            let _ = writeln!(
                output,
                "entry|{}|{}|{}|{}|{}|{}",
                entry.asset_id,
                entry.offset,
                entry.compressed_size,
                entry.uncompressed_size,
                entry.compression.as_str(),
                escape_field(&entry.content_hash)
            );
        }
        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackIndexError {
    DuplicateId(AssetId),
}

impl Display for PackIndexError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "pack entry already indexed: {id}"),
        }
    }
}

impl Error for PackIndexError {}

/// A built-asset manifest entry joined to its pack location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetManifestEntry {
    pub metadata: AssetMetadata,
    pub pack_entry: PackIndexEntry,
}

/// Deterministic metadata index for runtime asset lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetManifest {
    pub format_version: u32,
    entries: BTreeMap<AssetId, AssetManifestEntry>,
    names: BTreeSet<String>,
}

impl AssetManifest {
    #[must_use]
    pub fn new(format_version: u32) -> Self {
        Self {
            format_version,
            entries: BTreeMap::new(),
            names: BTreeSet::new(),
        }
    }

    /// Adds one built asset and its independently addressable pack chunk.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate IDs, duplicate names, or mismatched
    /// metadata and pack-entry identities.
    pub fn insert(
        &mut self,
        metadata: AssetMetadata,
        pack_entry: PackIndexEntry,
    ) -> Result<(), AssetManifestError> {
        let id = metadata.id;
        if id != pack_entry.asset_id {
            return Err(AssetManifestError::MismatchedIds {
                metadata_id: id,
                pack_id: pack_entry.asset_id,
            });
        }
        if self.entries.contains_key(&id) {
            return Err(AssetManifestError::DuplicateId(id));
        }
        if self.names.contains(&metadata.name) {
            return Err(AssetManifestError::DuplicateName(metadata.name));
        }
        self.names.insert(metadata.name.clone());
        self.entries.insert(
            id,
            AssetManifestEntry {
                metadata,
                pack_entry,
            },
        );
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&AssetManifestEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Validates required dependency closure across one or more manifests.
    ///
    /// Optional dependencies may be absent because their loaders can provide
    /// a fallback. Asset IDs must be unique across the supplied pack set.
    ///
    /// # Errors
    ///
    /// Returns an error when two packs claim the same asset ID or when a
    /// required dependency is absent from every supplied manifest.
    pub fn validate_across<'a>(
        manifests: impl IntoIterator<Item = &'a Self>,
    ) -> Result<(), AssetManifestError> {
        let mut ids = BTreeSet::new();
        let mut required_dependencies = Vec::new();
        for manifest in manifests {
            for entry in manifest.entries.values() {
                if !ids.insert(entry.metadata.id) {
                    return Err(AssetManifestError::DuplicateIdAcrossPacks(
                        entry.metadata.id,
                    ));
                }
                required_dependencies.extend(
                    entry
                        .metadata
                        .dependencies
                        .iter()
                        .filter(|dependency| dependency.required)
                        .map(|dependency| (entry.metadata.id, dependency.id)),
                );
            }
        }
        required_dependencies.sort_unstable();
        for (asset_id, dependency_id) in required_dependencies {
            if !ids.contains(&dependency_id) {
                return Err(AssetManifestError::MissingDependency {
                    asset_id,
                    dependency_id,
                });
            }
        }
        Ok(())
    }

    /// Emits stable line-oriented data with entries sorted by [`AssetId`].
    #[must_use]
    pub fn canonical_text(&self) -> String {
        let mut output = format!("meridian-asset-manifest-v{}\n", self.format_version);
        for entry in self.entries.values() {
            let mut dependencies = entry.metadata.dependencies.clone();
            dependencies.sort_unstable_by_key(|dependency| (dependency.id, dependency.required));
            let dependencies = dependencies
                .iter()
                .map(|dependency| {
                    format!(
                        "{}:{}",
                        dependency.id,
                        if dependency.required {
                            "required"
                        } else {
                            "optional"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let mut tags = entry.metadata.runtime_tags.clone();
            tags.sort_unstable();
            let _ = writeln!(
                output,
                "asset|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                entry.metadata.id,
                entry.metadata.kind.as_str(),
                escape_field(&entry.metadata.name),
                escape_field(&entry.metadata.source_path),
                escape_field(&entry.metadata.source_hash),
                escape_field(&entry.metadata.importer_version),
                escape_field(&tags.join(",")),
                escape_field(&dependencies),
                entry.pack_entry.offset,
                entry.pack_entry.compressed_size,
                entry.pack_entry.uncompressed_size,
                entry.pack_entry.compression.as_str(),
                escape_field(&entry.pack_entry.content_hash),
            );
        }
        output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetManifestError {
    DuplicateId(AssetId),
    DuplicateIdAcrossPacks(AssetId),
    DuplicateName(String),
    MissingDependency {
        asset_id: AssetId,
        dependency_id: AssetId,
    },
    MismatchedIds {
        metadata_id: AssetId,
        pack_id: AssetId,
    },
}

impl Display for AssetManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "manifest asset ID already exists: {id}"),
            Self::DuplicateIdAcrossPacks(id) => {
                write!(formatter, "asset ID is claimed by multiple packs: {id}")
            }
            Self::DuplicateName(name) => {
                write!(formatter, "manifest asset name already exists: {name}")
            }
            Self::MissingDependency {
                asset_id,
                dependency_id,
            } => write!(
                formatter,
                "asset {asset_id} requires missing dependency {dependency_id}"
            ),
            Self::MismatchedIds {
                metadata_id,
                pack_id,
            } => write!(
                formatter,
                "manifest metadata ID {metadata_id} does not match pack ID {pack_id}"
            ),
        }
    }
}

impl Error for AssetManifestError {}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Cooperative cancellation shared by a queued asset load and its worker.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn checkpoint(&self) -> Result<(), AssetLoadError> {
        if self.is_cancelled() {
            Err(AssetLoadError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Backend-neutral byte-range reader supplied by the pack/IO worker.
pub trait PackReader {
    type Error: Display;

    /// Reads exactly the requested logical pack range unless the source fails.
    ///
    /// # Errors
    ///
    /// Implementations return their source-specific error when the range
    /// cannot be read.
    fn read_range(
        &mut self,
        offset: u64,
        length: u64,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, Self::Error>;
}

/// File-backed reader for one built pack, with bounded range reads.
pub struct FilePackReader {
    path: PathBuf,
    file: File,
}

impl FilePackReader {
    /// Opens a built pack for worker-owned sequential range reads.
    ///
    /// # Errors
    ///
    /// Returns the operating-system error when the pack cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        Ok(Self { path, file })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl PackReader for FilePackReader {
    type Error = io::Error;

    fn read_range(
        &mut self,
        offset: u64,
        length: u64,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, Self::Error> {
        if cancellation.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "asset load cancelled",
            ));
        }
        let length = usize::try_from(length)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pack range is too large"))?;
        self.file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| io::Error::other("pack range could not be reserved in memory"))?;
        bytes.resize(length, 0);

        let mut filled = 0;
        while filled < bytes.len() {
            if cancellation.is_cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "asset load cancelled",
                ));
            }
            let read = self.file.read(&mut bytes[filled..])?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "pack ended before the requested range was read",
                ));
            }
            filled += read;
        }
        Ok(bytes)
    }
}

/// Backend-neutral decompressor supplied by the asset runtime.
pub trait AssetDecoder {
    type Error: Display;

    /// Decodes one independently stored chunk while honoring cancellation.
    ///
    /// # Errors
    ///
    /// Implementations return their format-specific error when decoding fails.
    fn decode(
        &self,
        compression: AssetCompression,
        compressed: Vec<u8>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, Self::Error>;
}

/// Decoder for packs whose chunk is already uncompressed.
#[derive(Clone, Copy, Debug, Default)]
pub struct UncompressedDecoder;

impl AssetDecoder for UncompressedDecoder {
    type Error = DecoderError;

    fn decode(
        &self,
        compression: AssetCompression,
        compressed: Vec<u8>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, Self::Error> {
        if cancellation.is_cancelled() {
            return Err(DecoderError::Cancelled);
        }
        match compression {
            AssetCompression::None => Ok(compressed),
            AssetCompression::Zstandard => Err(DecoderError::UnsupportedCompression),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderError {
    Cancelled,
    UnsupportedCompression,
}

impl Display for DecoderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("asset decode was cancelled"),
            Self::UnsupportedCompression => {
                formatter.write_str("zstandard decoding is not installed")
            }
        }
    }
}

impl Error for DecoderError {}

/// A worker-ready load operation derived from one pack-index entry.
#[derive(Clone, Debug)]
pub struct AssetLoadRequest {
    asset_id: AssetId,
    pack_entry: PackIndexEntry,
    cancellation: CancellationToken,
}

impl AssetLoadRequest {
    #[must_use]
    pub fn new(pack_entry: PackIndexEntry, cancellation: CancellationToken) -> Self {
        Self {
            asset_id: pack_entry.asset_id,
            pack_entry,
            cancellation,
        }
    }

    #[must_use]
    pub const fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    #[must_use]
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Executes the read/decode boundary on the caller's worker.
    ///
    /// This method performs no scheduling and never reads a source asset path.
    /// A task pool can move the request to a worker and call it with the
    /// platform's pack reader and decompressor.
    ///
    /// # Errors
    ///
    /// Returns an actionable error when cancellation, range reading,
    /// decompression, or decoded-size validation fails.
    pub fn execute<R, D>(
        self,
        reader: &mut R,
        decoder: &D,
    ) -> Result<AssetLoadResult, AssetLoadError>
    where
        R: PackReader,
        D: AssetDecoder,
    {
        self.cancellation.checkpoint()?;
        let compressed = reader
            .read_range(
                self.pack_entry.offset,
                self.pack_entry.compressed_size,
                &self.cancellation,
            )
            .map_err(|error| {
                if self.cancellation.is_cancelled() {
                    AssetLoadError::Cancelled
                } else {
                    AssetLoadError::ReadFailed {
                        asset_id: self.asset_id,
                        message: error.to_string(),
                    }
                }
            })?;
        self.cancellation.checkpoint()?;
        let decoded_bytes = decoder
            .decode(self.pack_entry.compression, compressed, &self.cancellation)
            .map_err(|error| {
                if self.cancellation.is_cancelled() {
                    AssetLoadError::Cancelled
                } else {
                    AssetLoadError::DecodeFailed {
                        asset_id: self.asset_id,
                        message: error.to_string(),
                    }
                }
            })?;
        self.cancellation.checkpoint()?;
        let actual = decoded_bytes.len() as u64;
        if actual != self.pack_entry.uncompressed_size {
            return Err(AssetLoadError::DecodedSizeMismatch {
                asset_id: self.asset_id,
                expected: self.pack_entry.uncompressed_size,
                actual,
            });
        }
        Ok(AssetLoadResult {
            asset_id: self.asset_id,
            bytes: decoded_bytes,
        })
    }

    /// Converts this request into a fixed-worker-pool-compatible job.
    ///
    /// The returned closure satisfies the existing task-pool contract without
    /// coupling this crate to a particular scheduler implementation.
    pub fn into_job<R, D>(
        self,
        reader: R,
        decoder: D,
    ) -> impl FnOnce() -> Result<AssetLoadResult, AssetLoadError> + Send + 'static
    where
        R: PackReader + Send + 'static,
        D: AssetDecoder + Send + 'static,
    {
        move || {
            let mut reader = reader;
            self.execute(&mut reader, &decoder)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetLoadResult {
    pub asset_id: AssetId,
    pub bytes: Vec<u8>,
}

/// Runtime facade that resolves asset IDs through a manifest and loads their
/// built pack ranges through injected IO and decoding adapters.
pub struct AssetRuntime<R, D> {
    manifest: AssetManifest,
    reader: R,
    decoder: D,
}

impl<R, D> AssetRuntime<R, D> {
    #[must_use]
    pub fn new(manifest: AssetManifest, reader: R, decoder: D) -> Self {
        Self {
            manifest,
            reader,
            decoder,
        }
    }

    #[must_use]
    pub fn manifest(&self) -> &AssetManifest {
        &self.manifest
    }

    /// Resolves an ID into a worker-ready load request.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRuntimeError::UnknownAsset`] when the manifest has no
    /// entry for the requested ID.
    pub fn request(
        &self,
        asset_id: AssetId,
        cancellation: CancellationToken,
    ) -> Result<AssetLoadRequest, AssetRuntimeError> {
        let entry = self
            .manifest
            .get(asset_id)
            .ok_or(AssetRuntimeError::UnknownAsset(asset_id))?;
        Ok(AssetLoadRequest::new(
            entry.pack_entry.clone(),
            cancellation,
        ))
    }
}

impl<R, D> AssetRuntime<R, D>
where
    R: PackReader,
    D: AssetDecoder,
{
    /// Loads one built asset by stable ID.
    ///
    /// # Errors
    ///
    /// Returns an actionable error when the ID is absent or the indexed range
    /// cannot be read, decoded, or validated.
    pub fn load_by_id(
        &mut self,
        asset_id: AssetId,
        cancellation: CancellationToken,
    ) -> Result<AssetLoadResult, AssetRuntimeError> {
        let request = self.request(asset_id, cancellation)?;
        request
            .execute(&mut self.reader, &self.decoder)
            .map_err(AssetRuntimeError::Load)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum AssetRuntimeError {
    UnknownAsset(AssetId),
    Load(AssetLoadError),
}

impl Display for AssetRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAsset(id) => write!(formatter, "asset ID is not in the manifest: {id}"),
            Self::Load(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for AssetRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownAsset(_) => None,
            Self::Load(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetLoadError {
    Cancelled,
    ReadFailed {
        asset_id: AssetId,
        message: String,
    },
    DecodeFailed {
        asset_id: AssetId,
        message: String,
    },
    DecodedSizeMismatch {
        asset_id: AssetId,
        expected: u64,
        actual: u64,
    },
}

impl Display for AssetLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("asset load was cancelled"),
            Self::ReadFailed { asset_id, message } => {
                write!(formatter, "asset {asset_id} pack read failed: {message}")
            }
            Self::DecodeFailed { asset_id, message } => {
                write!(formatter, "asset {asset_id} decode failed: {message}")
            }
            Self::DecodedSizeMismatch {
                asset_id,
                expected,
                actual,
            } => write!(
                formatter,
                "asset {asset_id} decoded size mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for AssetLoadError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetFailureKind {
    MissingBuiltOutput,
    CorruptBuiltOutput,
    MissingDependency,
    UnsupportedFormat,
    ImportFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetFailure {
    pub kind: AssetFailureKind,
    pub message: String,
}

impl AssetFailure {
    #[must_use]
    pub fn new(kind: AssetFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetState {
    Discovered,
    Queued,
    Loading,
    Ready,
    Failed(AssetFailure),
}

impl AssetState {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Runtime residency state independent of asset build/load state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetResidencyState {
    Unresident,
    Loading,
    Resident,
}

/// Memory and recency data used by a streaming policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetResidencyRecord {
    pub asset_id: AssetId,
    pub size_bytes: u64,
    pub priority: u8,
    pub last_used_tick: u64,
    pub pinned: bool,
    pub state: AssetResidencyState,
}

/// Bounded accounting layer for CPU/GPU-resident asset payloads.
#[derive(Debug)]
pub struct AssetResidencyTracker {
    budget_bytes: u64,
    resident_bytes: u64,
    records: BTreeMap<AssetId, AssetResidencyRecord>,
}

impl AssetResidencyTracker {
    #[must_use]
    pub const fn new(budget_bytes: u64) -> Self {
        Self {
            budget_bytes,
            resident_bytes: 0,
            records: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    #[must_use]
    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    #[must_use]
    pub const fn is_over_budget(&self) -> bool {
        self.resident_bytes > self.budget_bytes
    }

    #[must_use]
    pub fn get(&self, asset_id: AssetId) -> Option<&AssetResidencyRecord> {
        self.records.get(&asset_id)
    }

    /// Registers or updates the expected resident size and eviction priority.
    pub fn upsert(&mut self, asset_id: AssetId, size_bytes: u64, priority: u8) {
        if let Some(record) = self.records.get_mut(&asset_id) {
            if record.state == AssetResidencyState::Resident {
                self.resident_bytes = self
                    .resident_bytes
                    .saturating_sub(record.size_bytes)
                    .saturating_add(size_bytes);
            }
            record.size_bytes = size_bytes;
            record.priority = priority;
            return;
        }
        self.records.insert(
            asset_id,
            AssetResidencyRecord {
                asset_id,
                size_bytes,
                priority,
                last_used_tick: 0,
                pinned: false,
                state: AssetResidencyState::Unresident,
            },
        );
    }

    /// Marks an asset as being loaded and removes any prior resident charge.
    ///
    /// # Errors
    ///
    /// Returns [`AssetResidencyError::UnknownId`] when the asset is not
    /// registered with this tracker.
    pub fn mark_loading(&mut self, asset_id: AssetId) -> Result<(), AssetResidencyError> {
        self.set_state(asset_id, AssetResidencyState::Loading)
    }

    /// Marks an asset resident and records the tick at which it was used.
    ///
    /// # Errors
    ///
    /// Returns [`AssetResidencyError::UnknownId`] when the asset is not
    /// registered with this tracker.
    pub fn mark_resident(
        &mut self,
        asset_id: AssetId,
        tick: u64,
    ) -> Result<(), AssetResidencyError> {
        self.set_state(asset_id, AssetResidencyState::Resident)?;
        self.records
            .get_mut(&asset_id)
            .ok_or(AssetResidencyError::UnknownId(asset_id))?
            .last_used_tick = tick;
        Ok(())
    }

    /// Updates last-use recency without changing residency or accounting.
    ///
    /// # Errors
    ///
    /// Returns [`AssetResidencyError::UnknownId`] when the asset is not
    /// registered with this tracker.
    pub fn touch(&mut self, asset_id: AssetId, tick: u64) -> Result<(), AssetResidencyError> {
        self.records
            .get_mut(&asset_id)
            .ok_or(AssetResidencyError::UnknownId(asset_id))?
            .last_used_tick = tick;
        Ok(())
    }

    /// Pins or unpins an asset so eviction can respect an active dependency.
    ///
    /// # Errors
    ///
    /// Returns [`AssetResidencyError::UnknownId`] when the asset is not
    /// registered with this tracker.
    pub fn set_pinned(
        &mut self,
        asset_id: AssetId,
        pinned: bool,
    ) -> Result<(), AssetResidencyError> {
        self.records
            .get_mut(&asset_id)
            .ok_or(AssetResidencyError::UnknownId(asset_id))?
            .pinned = pinned;
        Ok(())
    }

    /// Removes an asset and releases any resident bytes charged to it.
    pub fn remove(&mut self, asset_id: AssetId) -> bool {
        let Some(record) = self.records.remove(&asset_id) else {
            return false;
        };
        if record.state == AssetResidencyState::Resident {
            self.resident_bytes = self.resident_bytes.saturating_sub(record.size_bytes);
        }
        true
    }

    /// Returns unpinned resident IDs from lowest priority and oldest use first.
    #[must_use]
    pub fn eviction_candidates(&self) -> Vec<AssetId> {
        let mut candidates = self
            .records
            .values()
            .filter(|record| record.state == AssetResidencyState::Resident && !record.pinned)
            .map(|record| (record.priority, record.last_used_tick, record.asset_id))
            .collect::<Vec<_>>();
        candidates.sort_unstable();
        candidates
            .into_iter()
            .map(|(_, _, asset_id)| asset_id)
            .collect()
    }

    fn set_state(
        &mut self,
        asset_id: AssetId,
        state: AssetResidencyState,
    ) -> Result<(), AssetResidencyError> {
        let record = self
            .records
            .get(&asset_id)
            .ok_or(AssetResidencyError::UnknownId(asset_id))?;
        let was_resident = record.state == AssetResidencyState::Resident;
        let will_be_resident = state == AssetResidencyState::Resident;
        if was_resident && !will_be_resident {
            self.resident_bytes = self.resident_bytes.saturating_sub(record.size_bytes);
        } else if !was_resident && will_be_resident {
            self.resident_bytes = self.resident_bytes.saturating_add(record.size_bytes);
        }
        self.records
            .get_mut(&asset_id)
            .ok_or(AssetResidencyError::UnknownId(asset_id))?
            .state = state;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetResidencyError {
    UnknownId(AssetId),
}

impl Display for AssetResidencyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownId(id) => write!(formatter, "asset residency ID is not registered: {id}"),
        }
    }
}

impl Error for AssetResidencyError {}

/// Runtime-visible replacement information for a failed asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailurePlaceholder {
    pub asset_id: AssetId,
    pub asset_name: String,
    pub kind: AssetFailureKind,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRecord {
    pub metadata: AssetMetadata,
    pub state: AssetState,
}

impl AssetRecord {
    #[must_use]
    pub fn new(metadata: AssetMetadata) -> Self {
        Self {
            metadata,
            state: AssetState::Discovered,
        }
    }

    #[must_use]
    pub fn failure_placeholder(&self) -> Option<FailurePlaceholder> {
        let AssetState::Failed(failure) = &self.state else {
            return None;
        };
        Some(FailurePlaceholder {
            asset_id: self.metadata.id,
            asset_name: self.metadata.name.clone(),
            kind: failure.kind,
            message: failure.message.clone(),
        })
    }
}

#[derive(Default)]
pub struct AssetRegistry {
    records: BTreeMap<AssetId, AssetRecord>,
}

impl AssetRegistry {
    /// Adds one metadata record before any built output is loaded.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRegistryError::DuplicateId`] when the ID already exists.
    pub fn insert(&mut self, record: AssetRecord) -> Result<(), AssetRegistryError> {
        let id = record.metadata.id;
        if self.records.contains_key(&id) {
            return Err(AssetRegistryError::DuplicateId(id));
        }
        self.records.insert(id, record);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&AssetRecord> {
        self.records.get(&id)
    }

    /// Marks an asset ready only when every required dependency is ready.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRegistryError::MissingDependencies`] when a dependency
    /// is absent or has not reached [`AssetState::Ready`].
    pub fn mark_ready(&mut self, id: AssetId) -> Result<(), AssetRegistryError> {
        let record = self
            .records
            .get(&id)
            .ok_or(AssetRegistryError::UnknownId(id))?;
        let missing = record
            .metadata
            .dependencies
            .iter()
            .filter(|dependency| {
                dependency.required
                    && !self
                        .records
                        .get(&dependency.id)
                        .is_some_and(|dependency| dependency.state.is_ready())
            })
            .map(|dependency| dependency.id)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(AssetRegistryError::MissingDependencies {
                asset_id: id,
                dependencies: missing,
            });
        }
        match self.records.get_mut(&id) {
            Some(record) => {
                record.state = AssetState::Ready;
                Ok(())
            }
            None => Err(AssetRegistryError::UnknownId(id)),
        }
    }

    /// Moves a known asset into the asynchronous loading queue.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRegistryError::UnknownId`] when the asset is not
    /// registered.
    pub fn mark_queued(&mut self, id: AssetId) -> Result<(), AssetRegistryError> {
        self.set_state(id, AssetState::Queued)
    }

    /// Marks a known asset as actively loading.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRegistryError::UnknownId`] when the asset is not
    /// registered.
    pub fn mark_loading(&mut self, id: AssetId) -> Result<(), AssetRegistryError> {
        self.set_state(id, AssetState::Loading)
    }

    /// Records a load/build failure while retaining a visible placeholder.
    ///
    /// # Errors
    ///
    /// Returns [`AssetRegistryError::UnknownId`] when the asset is not
    /// registered.
    pub fn mark_failed(
        &mut self,
        id: AssetId,
        failure: AssetFailure,
    ) -> Result<(), AssetRegistryError> {
        self.set_state(id, AssetState::Failed(failure))
    }

    fn set_state(&mut self, id: AssetId, state: AssetState) -> Result<(), AssetRegistryError> {
        self.records
            .get_mut(&id)
            .ok_or(AssetRegistryError::UnknownId(id))?
            .state = state;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetRegistryError {
    DuplicateId(AssetId),
    UnknownId(AssetId),
    MissingDependencies {
        asset_id: AssetId,
        dependencies: Vec<AssetId>,
    },
}

impl Display for AssetRegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "asset ID already registered: {id}"),
            Self::UnknownId(id) => write!(formatter, "asset ID is not registered: {id}"),
            Self::MissingDependencies {
                asset_id,
                dependencies,
            } => write!(formatter, "asset {asset_id} is waiting on {dependencies:?}"),
        }
    }
}

impl Error for AssetRegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_artifact_identity_are_deterministic_and_round_trip_hex() {
        assert_eq!(
            SourceId::from_canonical_name("fixtures/triangle"),
            SourceId::from_canonical_name("fixtures/triangle")
        );
        let hash = ArtifactHash::digest(b"Meridian");
        assert_eq!(ArtifactHash::from_hex(&hash.to_string()), Ok(hash));
        assert!(matches!(
            ArtifactHash::from_hex("short"),
            Err(ArtifactHashParseError::InvalidLength(5))
        ));
    }

    fn metadata(id: AssetId, name: &str) -> AssetMetadata {
        AssetMetadata::new(
            id,
            name,
            AssetKind::Mesh,
            "source/mesh.glb",
            "hash",
            "importer-1",
        )
    }

    fn pack_entry(id: AssetId, offset: u64) -> PackIndexEntry {
        PackIndexEntry::new(
            id,
            offset,
            40,
            80,
            AssetCompression::Zstandard,
            "content-hash",
        )
    }

    fn metadata_with(
        id: AssetId,
        name: &str,
        dependencies: &[AssetDependency],
        tags: &[&str],
    ) -> AssetMetadata {
        metadata(id, name)
            .with_dependencies(dependencies.iter().copied())
            .with_runtime_tags(tags.iter().map(|tag| (*tag).to_owned()))
    }

    struct TestReader {
        bytes: Vec<u8>,
        calls: Vec<(u64, u64)>,
        cancel_after_read: bool,
    }

    impl PackReader for TestReader {
        type Error = &'static str;

        fn read_range(
            &mut self,
            offset: u64,
            length: u64,
            cancellation: &CancellationToken,
        ) -> Result<Vec<u8>, Self::Error> {
            self.calls.push((offset, length));
            if self.cancel_after_read {
                cancellation.cancel();
            }
            Ok(self.bytes.clone())
        }
    }

    struct TestDecoder {
        cancel_during_decode: bool,
    }

    impl AssetDecoder for TestDecoder {
        type Error = &'static str;

        fn decode(
            &self,
            _compression: AssetCompression,
            compressed: Vec<u8>,
            cancellation: &CancellationToken,
        ) -> Result<Vec<u8>, Self::Error> {
            if self.cancel_during_decode {
                cancellation.cancel();
            }
            Ok(compressed)
        }
    }

    #[test]
    fn asset_ids_are_stable_and_display_as_fixed_width_hex() {
        let first = AssetId::from_name("forest/tree/oak");
        assert_eq!(first, AssetId::from_name("forest/tree/oak"));
        assert_ne!(first, AssetId::from_name("forest/tree/pine"));
        assert_eq!(first.to_string().len(), 32);
    }

    #[test]
    fn required_dependencies_gate_readiness() {
        let material_id = AssetId::from_name("forest/material");
        let mesh_id = AssetId::from_name("forest/tree");
        let material = AssetRecord::new(metadata(material_id, "forest/material"));
        let mesh = AssetRecord::new(
            metadata(mesh_id, "forest/tree")
                .with_dependencies([AssetDependency::required(material_id)]),
        );
        let mut registry = AssetRegistry::default();
        registry.insert(material).expect("unique material");
        registry.insert(mesh).expect("unique mesh");

        let error = registry
            .mark_ready(mesh_id)
            .expect_err("mesh must wait for material");
        assert_eq!(
            error,
            AssetRegistryError::MissingDependencies {
                asset_id: mesh_id,
                dependencies: vec![material_id],
            }
        );
        registry
            .mark_ready(material_id)
            .expect("material has no dependencies");
        registry.mark_ready(mesh_id).expect("dependency is ready");
        assert!(registry.get(mesh_id).expect("mesh exists").state.is_ready());
    }

    #[test]
    fn failed_assets_expose_a_visible_placeholder() {
        let id = AssetId::from_name("forest/tree");
        let mut registry = AssetRegistry::default();
        registry
            .insert(AssetRecord::new(metadata(id, "forest/tree")))
            .expect("unique asset");
        registry
            .mark_failed(
                id,
                AssetFailure::new(
                    AssetFailureKind::CorruptBuiltOutput,
                    "chunk checksum did not match",
                ),
            )
            .expect("asset exists");

        let placeholder = registry
            .get(id)
            .expect("asset exists")
            .failure_placeholder()
            .expect("failed assets have a placeholder");
        assert_eq!(placeholder.asset_id, id);
        assert_eq!(placeholder.kind, AssetFailureKind::CorruptBuiltOutput);
        assert_eq!(placeholder.message, "chunk checksum did not match");
    }

    #[test]
    fn duplicate_ids_are_rejected_without_replacing_existing_metadata() {
        let id = AssetId::from_name("shared");
        let mut registry = AssetRegistry::default();
        registry
            .insert(AssetRecord::new(metadata(id, "first")))
            .expect("first record is unique");
        assert_eq!(
            registry.insert(AssetRecord::new(metadata(id, "second"))),
            Err(AssetRegistryError::DuplicateId(id))
        );
        assert_eq!(
            registry.get(id).expect("record remains").metadata.name,
            "first"
        );
    }

    #[test]
    fn pack_index_is_deterministic_and_sorted_by_asset_id() {
        let oak = AssetId::from_name("forest/oak");
        let pine = AssetId::from_name("forest/pine");
        let mut first = PackIndex::new(1, "base.pack");
        first.insert(pack_entry(pine, 200)).expect("unique pine");
        first.insert(pack_entry(oak, 100)).expect("unique oak");

        let mut second = PackIndex::new(1, "base.pack");
        second.insert(pack_entry(oak, 100)).expect("unique oak");
        second.insert(pack_entry(pine, 200)).expect("unique pine");

        assert_eq!(first.canonical_text(), second.canonical_text());
        assert_eq!(first.get(oak).expect("oak is indexed").offset, 100);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn manifest_canonicalization_is_independent_of_input_order() {
        let oak = AssetId::from_name("forest/oak");
        let material = AssetId::from_name("forest/material");
        let mut first = AssetManifest::new(1);
        first
            .insert(
                metadata_with(
                    oak,
                    "forest/oak",
                    &[
                        AssetDependency::optional(material),
                        AssetDependency::required(material),
                    ],
                    &["tree", "vegetation"],
                ),
                pack_entry(oak, 100),
            )
            .expect("manifest entry is valid");

        let mut second = AssetManifest::new(1);
        second
            .insert(
                metadata_with(
                    oak,
                    "forest/oak",
                    &[
                        AssetDependency::required(material),
                        AssetDependency::optional(material),
                    ],
                    &["vegetation", "tree"],
                ),
                pack_entry(oak, 100),
            )
            .expect("manifest entry is valid");

        assert_eq!(first.canonical_text(), second.canonical_text());
        assert_eq!(
            first.get(oak).expect("oak is in manifest").metadata.kind,
            AssetKind::Mesh
        );
    }

    #[test]
    fn manifest_rejects_duplicate_names_and_mismatched_pack_identity() {
        let first_id = AssetId::from_name("first");
        let second_id = AssetId::from_name("second");
        let mut manifest = AssetManifest::new(1);
        manifest
            .insert(metadata(first_id, "shared-name"), pack_entry(first_id, 0))
            .expect("first entry is valid");
        assert_eq!(
            manifest.insert(metadata(second_id, "shared-name"), pack_entry(second_id, 1)),
            Err(AssetManifestError::DuplicateName("shared-name".to_owned()))
        );

        let wrong_pack_id = AssetId::from_name("wrong-pack-id");
        assert!(matches!(
            manifest.insert(metadata(second_id, "second"), pack_entry(wrong_pack_id, 2)),
            Err(AssetManifestError::MismatchedIds { .. })
        ));
    }

    #[test]
    fn manifests_validate_required_cross_pack_dependencies() {
        let material_id = AssetId::from_name("forest/material");
        let tree_id = AssetId::from_name("forest/tree");
        let missing_id = AssetId::from_name("forest/missing");

        let mut base_pack = AssetManifest::new(1);
        base_pack
            .insert(
                metadata(material_id, "forest/material"),
                pack_entry(material_id, 0),
            )
            .expect("material entry is valid");

        let mut scene_pack = AssetManifest::new(1);
        scene_pack
            .insert(
                metadata_with(
                    tree_id,
                    "forest/tree",
                    &[
                        AssetDependency::required(material_id),
                        AssetDependency::optional(missing_id),
                    ],
                    &["vegetation"],
                ),
                pack_entry(tree_id, 32),
            )
            .expect("tree entry is valid");

        assert!(AssetManifest::validate_across([&scene_pack, &base_pack]).is_ok());

        let mut missing_pack = AssetManifest::new(1);
        missing_pack
            .insert(
                metadata(tree_id, "forest/tree")
                    .with_dependencies([AssetDependency::required(missing_id)]),
                pack_entry(tree_id, 32),
            )
            .expect("missing-dependency entry is valid");
        assert_eq!(
            AssetManifest::validate_across([&missing_pack, &base_pack]),
            Err(AssetManifestError::MissingDependency {
                asset_id: tree_id,
                dependency_id: missing_id,
            })
        );
    }

    #[test]
    fn manifests_reject_duplicate_ids_across_packs() {
        let id = AssetId::from_name("shared");
        let mut first = AssetManifest::new(1);
        first
            .insert(metadata(id, "first"), pack_entry(id, 0))
            .expect("first entry is valid");
        let mut second = AssetManifest::new(2);
        second
            .insert(metadata(id, "second"), pack_entry(id, 1))
            .expect("second manifest can be built independently");

        assert_eq!(
            AssetManifest::validate_across([&first, &second]),
            Err(AssetManifestError::DuplicateIdAcrossPacks(id))
        );
    }

    #[test]
    fn load_request_reads_the_indexed_range_and_validates_decoded_bytes() {
        let id = AssetId::from_name("forest/oak");
        let token = CancellationToken::new();
        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 128, 3, 3, AssetCompression::None, "hash"),
            token,
        );
        let mut reader = TestReader {
            bytes: vec![1, 2, 3],
            calls: Vec::new(),
            cancel_after_read: false,
        };
        let result = request
            .execute(
                &mut reader,
                &TestDecoder {
                    cancel_during_decode: false,
                },
            )
            .expect("valid indexed load");

        assert_eq!(reader.calls, vec![(128, 3)]);
        assert_eq!(result.asset_id, id);
        assert_eq!(result.bytes, vec![1, 2, 3]);
    }

    #[test]
    fn load_request_cancels_before_decode_and_during_decode() {
        let id = AssetId::from_name("forest/oak");
        let token = CancellationToken::new();
        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 0, 3, 3, AssetCompression::None, "hash"),
            token,
        );
        let mut reader = TestReader {
            bytes: vec![1, 2, 3],
            calls: Vec::new(),
            cancel_after_read: true,
        };
        assert_eq!(
            request.execute(
                &mut reader,
                &TestDecoder {
                    cancel_during_decode: false,
                }
            ),
            Err(AssetLoadError::Cancelled)
        );

        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 0, 3, 3, AssetCompression::None, "hash"),
            CancellationToken::new(),
        );
        let mut reader = TestReader {
            bytes: vec![1, 2, 3],
            calls: Vec::new(),
            cancel_after_read: false,
        };
        assert_eq!(
            request.execute(
                &mut reader,
                &TestDecoder {
                    cancel_during_decode: true,
                }
            ),
            Err(AssetLoadError::Cancelled)
        );
    }

    #[test]
    fn load_request_rejects_pre_cancel_and_decoded_size_mismatch() {
        let id = AssetId::from_name("forest/oak");
        let token = CancellationToken::new();
        token.cancel();
        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 0, 3, 3, AssetCompression::None, "hash"),
            token,
        );
        let mut reader = TestReader {
            bytes: vec![1, 2, 3],
            calls: Vec::new(),
            cancel_after_read: false,
        };
        assert_eq!(
            request.execute(
                &mut reader,
                &TestDecoder {
                    cancel_during_decode: false,
                }
            ),
            Err(AssetLoadError::Cancelled)
        );
        assert!(reader.calls.is_empty());

        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 0, 3, 4, AssetCompression::None, "hash"),
            CancellationToken::new(),
        );
        let mut reader = TestReader {
            bytes: vec![1, 2, 3],
            calls: Vec::new(),
            cancel_after_read: false,
        };
        assert_eq!(
            request.execute(
                &mut reader,
                &TestDecoder {
                    cancel_during_decode: false,
                }
            ),
            Err(AssetLoadError::DecodedSizeMismatch {
                asset_id: id,
                expected: 4,
                actual: 3,
            })
        );
    }

    #[test]
    fn file_pack_reader_loads_a_real_indexed_range() {
        let path = std::env::temp_dir().join(format!(
            "meridian-pack-test-{}.bin",
            AssetId::from_name("file-pack-reader")
        ));
        std::fs::write(&path, b"xxcdefyy").expect("write test pack");
        let mut reader = FilePackReader::open(&path).expect("open test pack");
        assert_eq!(reader.path(), path.as_path());

        let id = AssetId::from_name("forest/oak");
        let result = AssetLoadRequest::new(
            PackIndexEntry::new(id, 2, 4, 4, AssetCompression::None, "hash"),
            CancellationToken::new(),
        )
        .execute(&mut reader, &UncompressedDecoder)
        .expect("read uncompressed range");

        assert_eq!(result.asset_id, id);
        assert_eq!(result.bytes, b"cdef");
        std::fs::remove_file(path).expect("remove test pack");
    }

    #[test]
    fn runtime_loads_a_built_asset_by_id_and_rejects_unknown_ids() {
        let path = std::env::temp_dir().join(format!(
            "meridian-runtime-pack-test-{}.bin",
            AssetId::from_name("runtime-round-trip")
        ));
        std::fs::write(&path, b"headerasset").expect("write test pack");

        let asset_id = AssetId::from_name("forest/tree");
        let mut manifest = AssetManifest::new(1);
        manifest
            .insert(
                metadata(asset_id, "forest/tree"),
                PackIndexEntry::new(asset_id, 6, 5, 5, AssetCompression::None, "hash"),
            )
            .expect("manifest entry is valid");
        let reader = FilePackReader::open(&path).expect("open test pack");
        let mut runtime = AssetRuntime::new(manifest, reader, UncompressedDecoder);

        let result = runtime
            .load_by_id(asset_id, CancellationToken::new())
            .expect("built asset loads by stable ID");
        assert_eq!(result.asset_id, asset_id);
        assert_eq!(result.bytes, b"asset");
        assert_eq!(
            runtime.load_by_id(AssetId::from_name("missing"), CancellationToken::new()),
            Err(AssetRuntimeError::UnknownAsset(AssetId::from_name(
                "missing"
            )))
        );
        std::fs::remove_file(path).expect("remove test pack");
    }

    #[test]
    fn uncompressed_decoder_reports_unsupported_zstandard() {
        let error = UncompressedDecoder
            .decode(
                AssetCompression::Zstandard,
                vec![1, 2, 3],
                &CancellationToken::new(),
            )
            .expect_err("zstandard needs a separate decoder");
        assert_eq!(error, DecoderError::UnsupportedCompression);
    }

    #[test]
    fn load_request_job_runs_on_a_worker_thread() {
        let id = AssetId::from_name("forest/oak");
        let request = AssetLoadRequest::new(
            PackIndexEntry::new(id, 0, 3, 3, AssetCompression::None, "hash"),
            CancellationToken::new(),
        );
        let reader = TestReader {
            bytes: vec![7, 8, 9],
            calls: Vec::new(),
            cancel_after_read: false,
        };
        let job = request.into_job(reader, UncompressedDecoder);
        let result = std::thread::spawn(job)
            .join()
            .expect("worker job did not panic")
            .expect("worker load succeeded");
        assert_eq!(result.asset_id, id);
        assert_eq!(result.bytes, vec![7, 8, 9]);
    }

    #[test]
    fn residency_tracks_budget_and_deterministic_eviction_candidates() {
        let old_low = AssetId::from_name("old-low");
        let recent_high = AssetId::from_name("recent-high");
        let pinned = AssetId::from_name("pinned");
        let mut tracker = AssetResidencyTracker::new(100);
        tracker.upsert(old_low, 60, 1);
        tracker.upsert(recent_high, 30, 0);
        tracker.upsert(pinned, 40, 9);
        tracker
            .mark_resident(old_low, 10)
            .expect("old asset exists");
        tracker
            .mark_resident(recent_high, 20)
            .expect("recent asset exists");
        tracker
            .mark_resident(pinned, 1)
            .expect("pinned asset exists");
        tracker
            .set_pinned(pinned, true)
            .expect("pinned asset exists");

        assert_eq!(tracker.resident_bytes(), 130);
        assert!(tracker.is_over_budget());
        assert_eq!(tracker.eviction_candidates(), vec![recent_high, old_low]);
    }

    #[test]
    fn residency_reaccounts_resizes_and_loading_transitions() {
        let id = AssetId::from_name("forest/oak");
        let mut tracker = AssetResidencyTracker::new(256);
        tracker.upsert(id, 100, 1);
        tracker.mark_resident(id, 1).expect("asset exists");
        assert_eq!(tracker.resident_bytes(), 100);

        tracker.upsert(id, 40, 2);
        assert_eq!(tracker.resident_bytes(), 40);
        assert_eq!(tracker.get(id).expect("asset exists").priority, 2);
        tracker.mark_loading(id).expect("asset exists");
        assert_eq!(tracker.resident_bytes(), 0);
        assert_eq!(
            tracker.get(id).expect("asset exists").state,
            AssetResidencyState::Loading
        );
        assert!(tracker.remove(id));
        assert!(!tracker.remove(id));
    }

    #[test]
    fn residency_operations_report_unknown_assets() {
        let id = AssetId::from_name("missing");
        let mut tracker = AssetResidencyTracker::new(1);
        assert_eq!(
            tracker.mark_loading(id),
            Err(AssetResidencyError::UnknownId(id))
        );
        assert_eq!(
            tracker.touch(id, 4),
            Err(AssetResidencyError::UnknownId(id))
        );
        assert_eq!(
            tracker.set_pinned(id, true),
            Err(AssetResidencyError::UnknownId(id))
        );
    }
}
