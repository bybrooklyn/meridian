//! Provisional, bounded, uncompressed `.meridian` package v1.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use meridian_assets::{
    ArtifactHash, AssetCompression, AssetId, CancellationToken, PackIndexEntry, PackReader,
};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"MERIDN\0\0";
const FORMAT_VERSION: u32 = 1;
const SUPERBLOCK_SIZE: usize = 80;
const SUPERBLOCK_SIZE_U64: u64 = 80;
const INDEX_ENTRY_SIZE: usize = 64;
const INDEX_ENTRY_SIZE_U64: u64 = 64;
pub const DEFAULT_MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_CHUNKS: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageChunk {
    pub asset_id: AssetId,
    pub kind: String,
    pub bytes: Vec<u8>,
}

impl PackageChunk {
    #[must_use]
    pub fn new(asset_id: AssetId, kind: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            asset_id,
            kind: kind.into(),
            bytes,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageManifestEntry {
    pub asset_id: String,
    pub kind: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageManifest {
    pub schema: String,
    pub version: u32,
    pub entries: Vec<PackageManifestEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageIndexEntry {
    pub asset_id: AssetId,
    pub offset: u64,
    pub size: u64,
    pub hash: ArtifactHash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageLimits {
    pub max_package_bytes: u64,
    pub max_manifest_bytes: usize,
    pub max_chunks: usize,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_package_bytes: DEFAULT_MAX_PACKAGE_BYTES,
            max_manifest_bytes: DEFAULT_MAX_MANIFEST_BYTES,
            max_chunks: DEFAULT_MAX_CHUNKS,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PackageBuilder {
    chunks: Vec<PackageChunk>,
}

impl PackageBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    #[must_use]
    pub fn with_chunk(mut self, chunk: PackageChunk) -> Self {
        self.chunks.push(chunk);
        self
    }

    /// Encodes deterministic package bytes under explicit limits.
    ///
    /// # Errors
    ///
    /// Rejects duplicate IDs, invalid metadata, count/size overflow, or limit excess.
    pub fn encode(&self, limits: PackageLimits) -> Result<Vec<u8>, PackageError> {
        if self.chunks.len() > limits.max_chunks {
            return Err(PackageError::ChunkCountExceeded {
                count: self.chunks.len(),
                max: limits.max_chunks,
            });
        }
        let mut chunks = self.chunks.iter().collect::<Vec<_>>();
        chunks.sort_unstable_by_key(|chunk| chunk.asset_id);
        let mut ids = BTreeSet::new();
        let mut manifest_entries = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            if !ids.insert(chunk.asset_id) {
                return Err(PackageError::DuplicateChunk(chunk.asset_id));
            }
            if chunk.kind.is_empty() || chunk.kind.len() > 64 {
                return Err(PackageError::InvalidChunkKind(chunk.kind.clone()));
            }
            manifest_entries.push(PackageManifestEntry {
                asset_id: chunk.asset_id.to_string(),
                kind: chunk.kind.clone(),
                size: u64::try_from(chunk.bytes.len()).map_err(|_| PackageError::SizeOverflow)?,
                hash: ArtifactHash::digest(&chunk.bytes).to_string(),
            });
        }
        let manifest = PackageManifest {
            schema: "meridian.package-manifest/v1".to_owned(),
            version: FORMAT_VERSION,
            entries: manifest_entries,
        };
        let manifest_bytes = serde_json::to_vec(&manifest)
            .map_err(|error| PackageError::Manifest(error.to_string()))?;
        if manifest_bytes.len() > limits.max_manifest_bytes {
            return Err(PackageError::ManifestTooLarge {
                size: manifest_bytes.len(),
                max: limits.max_manifest_bytes,
            });
        }
        let index_size = chunks
            .len()
            .checked_mul(INDEX_ENTRY_SIZE)
            .ok_or(PackageError::SizeOverflow)?;
        let manifest_offset = SUPERBLOCK_SIZE_U64;
        let manifest_length =
            u64::try_from(manifest_bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
        let index_offset = manifest_offset
            .checked_add(manifest_length)
            .ok_or(PackageError::SizeOverflow)?;
        let index_length = u64::try_from(index_size).map_err(|_| PackageError::SizeOverflow)?;
        let mut chunk_offset = index_offset
            .checked_add(index_length)
            .ok_or(PackageError::SizeOverflow)?;
        let mut index = Vec::with_capacity(index_size);
        for chunk in &chunks {
            let size = u64::try_from(chunk.bytes.len()).map_err(|_| PackageError::SizeOverflow)?;
            index.extend_from_slice(&chunk.asset_id.value().to_le_bytes());
            index.extend_from_slice(&chunk_offset.to_le_bytes());
            index.extend_from_slice(&size.to_le_bytes());
            index.extend_from_slice(ArtifactHash::digest(&chunk.bytes).as_bytes());
            chunk_offset = chunk_offset
                .checked_add(size)
                .ok_or(PackageError::SizeOverflow)?;
        }
        if chunk_offset > limits.max_package_bytes {
            return Err(PackageError::PackageTooLarge {
                size: chunk_offset,
                max: limits.max_package_bytes,
            });
        }
        let capacity = usize::try_from(chunk_offset).map_err(|_| PackageError::SizeOverflow)?;
        let mut payload = Vec::with_capacity(capacity.saturating_sub(SUPERBLOCK_SIZE));
        payload.extend_from_slice(&manifest_bytes);
        payload.extend_from_slice(&index);
        for chunk in chunks {
            payload.extend_from_slice(&chunk.bytes);
        }
        let package_hash = ArtifactHash::digest(&payload);
        let mut bytes = Vec::with_capacity(capacity);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&manifest_offset.to_le_bytes());
        bytes.extend_from_slice(&manifest_length.to_le_bytes());
        bytes.extend_from_slice(&index_offset.to_le_bytes());
        bytes.extend_from_slice(&index_length.to_le_bytes());
        bytes.extend_from_slice(package_hash.as_bytes());
        bytes.extend_from_slice(&payload);
        debug_assert_eq!(bytes.len(), capacity);
        Ok(bytes)
    }

    /// Writes a synced temporary file then replaces the package path.
    ///
    /// # Errors
    ///
    /// Returns encoding or IO errors without retaining a partial primary package.
    pub fn write_atomic(
        &self,
        path: &Path,
        limits: PackageLimits,
    ) -> Result<ArtifactHash, PackageError> {
        let bytes = self.encode(limits)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| PackageError::io("create parent", &error))?;
            }
        }
        let temporary = suffixed_path(path, ".tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| PackageError::io("open temporary", &error))?;
        file.write_all(&bytes)
            .map_err(|error| PackageError::io("write temporary", &error))?;
        file.sync_all()
            .map_err(|error| PackageError::io("sync temporary", &error))?;
        drop(file);
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(PackageError::io("replace", &error));
        }
        Ok(ArtifactHash::digest(&bytes))
    }
}

pub struct MountedPackage {
    path: PathBuf,
    file: File,
    manifest: PackageManifest,
    entries: BTreeMap<AssetId, PackageIndexEntry>,
    package_hash: ArtifactHash,
    file_size: u64,
}

#[derive(Clone, Copy)]
struct DecodedHeader {
    manifest_length: u64,
    index_offset: u64,
    index_length: u64,
    package_hash: ArtifactHash,
}

impl MountedPackage {
    /// Opens and verifies a package before exposing any chunk.
    ///
    /// # Errors
    ///
    /// Rejects malformed, oversized, unsupported, duplicate, truncated, or hash-invalid input.
    pub fn mount(path: impl Into<PathBuf>, limits: PackageLimits) -> Result<Self, PackageError> {
        let path = path.into();
        let mut file = File::open(&path).map_err(|error| PackageError::io("open", &error))?;
        let file_size = file
            .metadata()
            .map_err(|error| PackageError::io("metadata", &error))?
            .len();
        if file_size > limits.max_package_bytes {
            return Err(PackageError::PackageTooLarge {
                size: file_size,
                max: limits.max_package_bytes,
            });
        }
        if file_size < SUPERBLOCK_SIZE_U64 {
            return Err(PackageError::Malformed(
                "package is shorter than superblock",
            ));
        }
        let mut header = [0_u8; SUPERBLOCK_SIZE];
        file.read_exact(&mut header)
            .map_err(|error| PackageError::io("read superblock", &error))?;
        let decoded = decode_header(&header, limits)?;
        let payload = read_payload(&mut file, file_size, decoded.package_hash)?;
        let manifest = decode_manifest(&payload, decoded, limits)?;
        let entries = decode_index(&payload, decoded, &manifest, file_size, limits)?;
        file.seek(SeekFrom::Start(0))
            .map_err(|error| PackageError::io("rewind", &error))?;
        let mut mounted = Self {
            path,
            file,
            manifest,
            entries,
            package_hash: decoded.package_hash,
            file_size,
        };
        mounted.verify_chunks()?;
        Ok(mounted)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn package_hash(&self) -> ArtifactHash {
        self.package_hash
    }

    #[must_use]
    pub fn entry(&self, asset_id: AssetId) -> Option<PackageIndexEntry> {
        self.entries.get(&asset_id).copied()
    }

    /// Reads one independently verified package chunk.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown ID or failed bounded file read.
    pub fn read_chunk(&mut self, asset_id: AssetId) -> Result<Vec<u8>, PackageError> {
        let entry = self
            .entry(asset_id)
            .ok_or(PackageError::UnknownChunk(asset_id))?;
        read_exact_range(&mut self.file, entry.offset, entry.size, self.file_size)
    }

    #[must_use]
    pub fn pack_index_entry(&self, asset_id: AssetId) -> Option<PackIndexEntry> {
        self.entry(asset_id).map(|entry| {
            PackIndexEntry::new(
                entry.asset_id,
                entry.offset,
                entry.size,
                entry.size,
                AssetCompression::None,
                entry.hash.to_string(),
            )
        })
    }

    fn verify_chunks(&mut self) -> Result<(), PackageError> {
        for entry in self.entries.values().copied().collect::<Vec<_>>() {
            let bytes = read_exact_range(&mut self.file, entry.offset, entry.size, self.file_size)?;
            if ArtifactHash::digest(&bytes) != entry.hash {
                return Err(PackageError::ChunkHashMismatch(entry.asset_id));
            }
        }
        Ok(())
    }
}

fn decode_header(
    header: &[u8; SUPERBLOCK_SIZE],
    limits: PackageLimits,
) -> Result<DecodedHeader, PackageError> {
    if &header[..8] != MAGIC {
        return Err(PackageError::Malformed("package magic does not match"));
    }
    let version = u32::from_le_bytes(read_array(header, 8)?);
    if version != FORMAT_VERSION {
        return Err(PackageError::UnsupportedVersion(version));
    }
    let manifest_offset = u64::from_le_bytes(read_array(header, 16)?);
    let manifest_length = u64::from_le_bytes(read_array(header, 24)?);
    let index_offset = u64::from_le_bytes(read_array(header, 32)?);
    let index_length = u64::from_le_bytes(read_array(header, 40)?);
    if manifest_offset != SUPERBLOCK_SIZE_U64
        || index_offset
            != manifest_offset
                .checked_add(manifest_length)
                .ok_or(PackageError::SizeOverflow)?
        || !index_length.is_multiple_of(INDEX_ENTRY_SIZE_U64)
    {
        return Err(PackageError::Malformed(
            "manifest or index offsets are invalid",
        ));
    }
    let manifest_size = usize::try_from(manifest_length).map_err(|_| PackageError::SizeOverflow)?;
    if manifest_size > limits.max_manifest_bytes {
        return Err(PackageError::ManifestTooLarge {
            size: manifest_size,
            max: limits.max_manifest_bytes,
        });
    }
    Ok(DecodedHeader {
        manifest_length,
        index_offset,
        index_length,
        package_hash: ArtifactHash::from_bytes(read_array(header, 48)?),
    })
}

fn read_payload(
    file: &mut File,
    file_size: u64,
    expected_hash: ArtifactHash,
) -> Result<Vec<u8>, PackageError> {
    let payload_size = file_size
        .checked_sub(SUPERBLOCK_SIZE_U64)
        .ok_or(PackageError::SizeOverflow)?;
    let mut payload =
        vec![0_u8; usize::try_from(payload_size).map_err(|_| PackageError::SizeOverflow)?];
    file.read_exact(&mut payload)
        .map_err(|error| PackageError::io("read payload", &error))?;
    if ArtifactHash::digest(&payload) != expected_hash {
        return Err(PackageError::PackageHashMismatch);
    }
    Ok(payload)
}

fn decode_manifest(
    payload: &[u8],
    header: DecodedHeader,
    limits: PackageLimits,
) -> Result<PackageManifest, PackageError> {
    let manifest_end =
        usize::try_from(header.manifest_length).map_err(|_| PackageError::SizeOverflow)?;
    let manifest: PackageManifest = serde_json::from_slice(
        payload
            .get(..manifest_end)
            .ok_or(PackageError::Malformed("manifest is truncated"))?,
    )
    .map_err(|error| PackageError::Manifest(error.to_string()))?;
    if manifest.schema != "meridian.package-manifest/v1" || manifest.version != FORMAT_VERSION {
        return Err(PackageError::Malformed(
            "manifest schema or version is invalid",
        ));
    }
    let chunk_count = usize::try_from(header.index_length / INDEX_ENTRY_SIZE_U64)
        .map_err(|_| PackageError::SizeOverflow)?;
    if chunk_count > limits.max_chunks || manifest.entries.len() != chunk_count {
        return Err(PackageError::ChunkCountExceeded {
            count: chunk_count.max(manifest.entries.len()),
            max: limits.max_chunks,
        });
    }
    Ok(manifest)
}

fn decode_index(
    payload: &[u8],
    header: DecodedHeader,
    manifest: &PackageManifest,
    file_size: u64,
    limits: PackageLimits,
) -> Result<BTreeMap<AssetId, PackageIndexEntry>, PackageError> {
    let index_start = usize::try_from(header.index_offset - SUPERBLOCK_SIZE_U64)
        .map_err(|_| PackageError::SizeOverflow)?;
    let index_end = index_start
        .checked_add(usize::try_from(header.index_length).map_err(|_| PackageError::SizeOverflow)?)
        .ok_or(PackageError::SizeOverflow)?;
    let index_bytes = payload
        .get(index_start..index_end)
        .ok_or(PackageError::Malformed("chunk index is truncated"))?;
    let chunk_data_offset = header
        .index_offset
        .checked_add(header.index_length)
        .ok_or(PackageError::SizeOverflow)?;
    let mut entries = BTreeMap::new();
    for chunk in index_bytes.chunks_exact(INDEX_ENTRY_SIZE) {
        let asset_id = AssetId::from_u128(u128::from_le_bytes(read_array(chunk, 0)?));
        let offset = u64::from_le_bytes(read_array(chunk, 16)?);
        let size = u64::from_le_bytes(read_array(chunk, 24)?);
        let hash = ArtifactHash::from_bytes(read_array(chunk, 32)?);
        let end = offset.checked_add(size).ok_or(PackageError::SizeOverflow)?;
        if offset < chunk_data_offset || end > file_size {
            return Err(PackageError::Malformed("chunk range is outside package"));
        }
        if entries
            .insert(
                asset_id,
                PackageIndexEntry {
                    asset_id,
                    offset,
                    size,
                    hash,
                },
            )
            .is_some()
        {
            return Err(PackageError::DuplicateChunk(asset_id));
        }
    }
    if entries.len() > limits.max_chunks {
        return Err(PackageError::ChunkCountExceeded {
            count: entries.len(),
            max: limits.max_chunks,
        });
    }
    let manifest_ids = manifest
        .entries
        .iter()
        .map(|entry| parse_asset_id(&entry.asset_id))
        .collect::<Result<Vec<_>, _>>()?;
    if manifest_ids.iter().copied().ne(entries.keys().copied()) {
        return Err(PackageError::Malformed("manifest and index IDs differ"));
    }
    Ok(entries)
}

impl PackReader for MountedPackage {
    type Error = PackageError;

    fn read_range(
        &mut self,
        offset: u64,
        length: u64,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, Self::Error> {
        if cancellation.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        let bytes = read_exact_range(&mut self.file, offset, length, self.file_size)?;
        if cancellation.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        Ok(bytes)
    }
}

fn read_exact_range(
    file: &mut File,
    offset: u64,
    length: u64,
    file_size: u64,
) -> Result<Vec<u8>, PackageError> {
    let end = offset
        .checked_add(length)
        .ok_or(PackageError::SizeOverflow)?;
    if end > file_size {
        return Err(PackageError::Malformed("read range exceeds package"));
    }
    let mut bytes = vec![0_u8; usize::try_from(length).map_err(|_| PackageError::SizeOverflow)?];
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| PackageError::io("seek", &error))?;
    file.read_exact(&mut bytes)
        .map_err(|error| PackageError::io("read range", &error))?;
    Ok(bytes)
}

fn parse_asset_id(value: &str) -> Result<AssetId, PackageError> {
    if value.len() != 32 {
        return Err(PackageError::Malformed(
            "manifest asset ID length is invalid",
        ));
    }
    u128::from_str_radix(value, 16)
        .map(AssetId::from_u128)
        .map_err(|_| PackageError::Malformed("manifest asset ID is invalid"))
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], PackageError> {
    bytes
        .get(offset..offset.saturating_add(N))
        .and_then(|value| value.try_into().ok())
        .ok_or(PackageError::Malformed("numeric field is truncated"))
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[derive(Debug)]
pub enum PackageError {
    Io {
        operation: &'static str,
        message: String,
    },
    Malformed(&'static str),
    Manifest(String),
    UnsupportedVersion(u32),
    DuplicateChunk(AssetId),
    UnknownChunk(AssetId),
    InvalidChunkKind(String),
    ChunkCountExceeded {
        count: usize,
        max: usize,
    },
    ManifestTooLarge {
        size: usize,
        max: usize,
    },
    PackageTooLarge {
        size: u64,
        max: u64,
    },
    SizeOverflow,
    PackageHashMismatch,
    ChunkHashMismatch(AssetId),
    Cancelled,
}

impl PackageError {
    fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            message: error.to_string(),
        }
    }
}

impl Display for PackageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, message } => {
                write!(formatter, "package {operation} failed: {message}")
            }
            Self::Malformed(message) => write!(formatter, "malformed package: {message}"),
            Self::Manifest(message) => write!(formatter, "package manifest failed: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported package version: {version}")
            }
            Self::DuplicateChunk(id) => write!(formatter, "duplicate package chunk: {id}"),
            Self::UnknownChunk(id) => write!(formatter, "unknown package chunk: {id}"),
            Self::InvalidChunkKind(kind) => write!(formatter, "invalid package chunk kind: {kind}"),
            Self::ChunkCountExceeded { count, max } => {
                write!(formatter, "package has {count} chunks; maximum is {max}")
            }
            Self::ManifestTooLarge { size, max } => write!(
                formatter,
                "package manifest is {size} bytes; maximum is {max}"
            ),
            Self::PackageTooLarge { size, max } => {
                write!(formatter, "package is {size} bytes; maximum is {max}")
            }
            Self::SizeOverflow => formatter.write_str("package size arithmetic overflowed"),
            Self::PackageHashMismatch => formatter.write_str("package payload hash mismatch"),
            Self::ChunkHashMismatch(id) => write!(formatter, "package chunk hash mismatch: {id}"),
            Self::Cancelled => formatter.write_str("package read was cancelled"),
        }
    }
}

impl Error for PackageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("meridian-package-{name}-{nonce}.meridian"))
    }

    fn builder() -> PackageBuilder {
        PackageBuilder::new()
            .with_chunk(PackageChunk::new(
                AssetId::from_u128(2),
                "world-cell",
                b"cell".to_vec(),
            ))
            .with_chunk(PackageChunk::new(
                AssetId::from_u128(1),
                "mesh",
                b"mesh-data".to_vec(),
            ))
    }

    #[test]
    fn deterministic_roundtrip_mount_and_pack_reader_range() {
        let limits = PackageLimits::default();
        assert_eq!(
            builder().encode(limits).expect("encodes"),
            builder().encode(limits).expect("encodes")
        );
        let path = path("roundtrip");
        builder().write_atomic(&path, limits).expect("writes");
        let mut mounted = MountedPackage::mount(&path, limits).expect("mounts");
        assert_eq!(
            mounted.read_chunk(AssetId::from_u128(1)).expect("reads"),
            b"mesh-data"
        );
        let entry = mounted
            .pack_index_entry(AssetId::from_u128(2))
            .expect("entry");
        assert_eq!(
            PackReader::read_range(
                &mut mounted,
                entry.offset,
                entry.compressed_size,
                &CancellationToken::new()
            )
            .expect("range"),
            b"cell"
        );
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn malformed_version_truncation_hash_and_duplicates_are_rejected() {
        let limits = PackageLimits::default();
        let duplicate = PackageBuilder::new()
            .with_chunk(PackageChunk::new(AssetId::from_u128(1), "a", vec![1]))
            .with_chunk(PackageChunk::new(AssetId::from_u128(1), "b", vec![2]));
        assert!(matches!(
            duplicate.encode(limits),
            Err(PackageError::DuplicateChunk(_))
        ));

        let path = path("invalid");
        let mut bytes = builder().encode(limits).expect("encodes");
        bytes[8..12].copy_from_slice(&2_u32.to_le_bytes());
        fs::write(&path, &bytes).expect("write version");
        assert!(matches!(
            MountedPackage::mount(&path, limits),
            Err(PackageError::UnsupportedVersion(2))
        ));
        bytes[8..12].copy_from_slice(&1_u32.to_le_bytes());
        bytes.pop();
        fs::write(&path, &bytes).expect("write truncation");
        assert!(matches!(
            MountedPackage::mount(&path, limits),
            Err(PackageError::PackageHashMismatch)
        ));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn atomic_replacement_leaves_mountable_new_package() {
        let limits = PackageLimits::default();
        let path = path("replace");
        builder().write_atomic(&path, limits).expect("first write");
        PackageBuilder::new()
            .with_chunk(PackageChunk::new(
                AssetId::from_u128(9),
                "new",
                b"new".to_vec(),
            ))
            .write_atomic(&path, limits)
            .expect("replacement");
        let mut mounted = MountedPackage::mount(&path, limits).expect("new package mounts");
        assert_eq!(
            mounted
                .read_chunk(AssetId::from_u128(9))
                .expect("new chunk"),
            b"new"
        );
        assert!(matches!(
            mounted.read_chunk(AssetId::from_u128(1)),
            Err(PackageError::UnknownChunk(_))
        ));
        fs::remove_file(path).expect("cleanup");
    }
}
