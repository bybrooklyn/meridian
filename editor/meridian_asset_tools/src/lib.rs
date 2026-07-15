//! Bounded source import transactions for editor/build tools.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use meridian_assets::{
    ArtifactHash, CancellationToken, SourceAuthority, SourceId, SourceMetadata, SourceProvenance,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FIXTURE_MESH_SCHEMA: &str = "meridian.fixture-mesh/v1";
pub const FIXTURE_MESH_IMPORTER_VERSION: &str = "meridian-fixture-mesh-importer/1";
pub const DEFAULT_MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_VERTICES: usize = 65_536;
pub const MAX_INDICES: usize = 196_608;
const VISUAL_FACET_MAGIC: [u8; 4] = *b"MVF1";
const COLLISION_FACET_MAGIC: [u8; 4] = *b"MCF1";
const FACET_HEADER_BYTES: usize = 16;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FixtureVertexSource {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureProvenanceSource {
    pub origin: String,
    pub license: String,
    #[serde(default)]
    pub attribution: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FixtureMeshSource {
    pub schema: String,
    pub version: u32,
    pub source_id: String,
    pub authority: SourceAuthority,
    pub provenance: FixtureProvenanceSource,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub vertices: Vec<FixtureVertexSource>,
    pub indices: Vec<u32>,
    #[serde(flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisualMeshFacet {
    pub vertices: Vec<FixtureVertexSource>,
    pub indices: Vec<u32>,
    pub artifact_hash: ArtifactHash,
}

impl VisualMeshFacet {
    #[must_use]
    pub fn encode_vertex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.vertices.len().saturating_mul(48));
        for vertex in &self.vertices {
            for value in vertex
                .position
                .iter()
                .chain(vertex.normal.iter())
                .chain(vertex.color.iter())
                .chain(vertex.uv.iter())
            {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    #[must_use]
    pub fn encode_index_bytes(&self) -> Vec<u8> {
        self.indices
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect()
    }

    /// Encodes the validated visual facet into a provisional runtime artifact.
    #[must_use]
    pub fn encode_compiled(&self) -> Vec<u8> {
        let vertex_bytes = self.encode_vertex_bytes();
        let mut bytes = Vec::with_capacity(
            FACET_HEADER_BYTES
                .saturating_add(vertex_bytes.len())
                .saturating_add(self.indices.len().saturating_mul(4)),
        );
        bytes.extend_from_slice(&VISUAL_FACET_MAGIC);
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(
            &u32::try_from(self.vertices.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &u32::try_from(self.indices.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&vertex_bytes);
        bytes.extend_from_slice(&self.encode_index_bytes());
        bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionMeshFacet {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub artifact_hash: ArtifactHash,
}

impl CollisionMeshFacet {
    /// Encodes the independently addressable collision facet.
    #[must_use]
    pub fn encode_compiled(&self) -> Vec<u8> {
        let payload = encode_collision_bytes(&self.positions, &self.indices);
        let mut bytes = Vec::with_capacity(FACET_HEADER_BYTES.saturating_add(payload.len()));
        bytes.extend_from_slice(&COLLISION_FACET_MAGIC);
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(
            &u32::try_from(self.positions.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &u32::try_from(self.indices.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&payload);
        bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledVisualFacet {
    pub vertex_data: Vec<u8>,
    pub indices: Vec<u32>,
}

/// Decodes the bounded provisional visual artifact without consulting source JSON.
///
/// # Errors
///
/// Rejects malformed headers, unsupported versions, count/size overflow,
/// out-of-bounds indices, and payload lengths that disagree with the header.
pub fn decode_compiled_visual(bytes: &[u8]) -> Result<CompiledVisualFacet, ImportError> {
    let (vertex_count, index_count) = decode_facet_header(bytes, VISUAL_FACET_MAGIC)?;
    if vertex_count > MAX_VERTICES || index_count > MAX_INDICES {
        return Err(ImportError::InvalidGeometry(
            "compiled visual facet exceeds bounded counts",
        ));
    }
    let vertex_bytes = vertex_count
        .checked_mul(48)
        .ok_or(ImportError::InvalidGeometry(
            "compiled vertex size overflows",
        ))?;
    let index_bytes = index_count
        .checked_mul(4)
        .ok_or(ImportError::InvalidGeometry(
            "compiled index size overflows",
        ))?;
    let expected = FACET_HEADER_BYTES
        .checked_add(vertex_bytes)
        .and_then(|size| size.checked_add(index_bytes))
        .ok_or(ImportError::InvalidGeometry(
            "compiled visual size overflows",
        ))?;
    if bytes.len() != expected {
        return Err(ImportError::InvalidGeometry(
            "compiled visual facet length does not match header",
        ));
    }
    let indices = bytes[FACET_HEADER_BYTES + vertex_bytes..]
        .chunks_exact(4)
        .map(|chunk| {
            let mut value = [0_u8; 4];
            value.copy_from_slice(chunk);
            u32::from_le_bytes(value)
        })
        .collect::<Vec<_>>();
    let vertex_count_u32 = u32::try_from(vertex_count).unwrap_or(u32::MAX);
    if indices.iter().any(|index| *index >= vertex_count_u32) {
        return Err(ImportError::InvalidGeometry(
            "compiled index references a missing vertex",
        ));
    }
    Ok(CompiledVisualFacet {
        vertex_data: bytes[FACET_HEADER_BYTES..FACET_HEADER_BYTES + vertex_bytes].to_vec(),
        indices,
    })
}

fn decode_facet_header(bytes: &[u8], magic: [u8; 4]) -> Result<(usize, usize), ImportError> {
    if bytes.len() < FACET_HEADER_BYTES || bytes[..4] != magic {
        return Err(ImportError::InvalidGeometry(
            "compiled facet header is malformed",
        ));
    }
    let version = read_compiled_u32(bytes, 4)?;
    if version != 1 {
        return Err(ImportError::InvalidGeometry(
            "compiled facet version is unsupported",
        ));
    }
    let vertices = usize::try_from(read_compiled_u32(bytes, 8)?).unwrap_or(usize::MAX);
    let indices = usize::try_from(read_compiled_u32(bytes, 12)?).unwrap_or(usize::MAX);
    Ok((vertices, indices))
}

fn read_compiled_u32(bytes: &[u8], offset: usize) -> Result<u32, ImportError> {
    let source =
        bytes
            .get(offset..offset.saturating_add(4))
            .ok_or(ImportError::InvalidGeometry(
                "compiled facet header is truncated",
            ))?;
    let mut value = [0_u8; 4];
    value.copy_from_slice(source);
    Ok(u32::from_le_bytes(value))
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedFixtureMesh {
    pub canonical_name: String,
    pub metadata: SourceMetadata,
    pub visual: VisualMeshFacet,
    pub collision: CollisionMeshFacet,
    pub preserved_unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetDatabaseSnapshot {
    pub generation: u64,
    pub meshes: BTreeMap<SourceId, ImportedFixtureMesh>,
}

#[derive(Clone, Debug)]
pub struct AssetImportDatabase {
    accepted: Arc<AssetDatabaseSnapshot>,
    max_source_bytes: usize,
}

impl AssetImportDatabase {
    #[must_use]
    pub fn new(max_source_bytes: usize) -> Self {
        Self {
            accepted: Arc::new(AssetDatabaseSnapshot::default()),
            max_source_bytes,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Arc<AssetDatabaseSnapshot> {
        Arc::clone(&self.accepted)
    }

    /// Imports every named source, validates dependencies, then replaces the snapshot once.
    ///
    /// # Errors
    ///
    /// Returns without changing the accepted snapshot on any error.
    pub fn import_files_transaction(
        &mut self,
        project_root: &Path,
        source_paths: &[PathBuf],
        cancellation: &CancellationToken,
    ) -> Result<Arc<AssetDatabaseSnapshot>, ImportError> {
        cancellation_checkpoint(cancellation)?;
        let canonical_root = project_root
            .canonicalize()
            .map_err(|error| ImportError::Io(error.to_string()))?;
        let mut staged = BTreeMap::new();
        for source_path in source_paths {
            cancellation_checkpoint(cancellation)?;
            let resolved = resolve_project_path(&canonical_root, source_path)?;
            let metadata =
                fs::metadata(&resolved).map_err(|error| ImportError::Io(error.to_string()))?;
            let source_size = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            if source_size > self.max_source_bytes {
                return Err(ImportError::SourceTooLarge {
                    size: source_size,
                    max: self.max_source_bytes,
                });
            }
            let bytes = fs::read(&resolved).map_err(|error| ImportError::Io(error.to_string()))?;
            cancellation_checkpoint(cancellation)?;
            let imported = import_fixture_mesh(&bytes, cancellation)?;
            let source_id = imported.metadata.source_id;
            if staged.insert(source_id, imported).is_some() {
                return Err(ImportError::DuplicateSourceId(source_id));
            }
        }
        validate_dependencies(&staged)?;
        cancellation_checkpoint(cancellation)?;

        let snapshot = Arc::new(AssetDatabaseSnapshot {
            generation: self.accepted.generation.wrapping_add(1).max(1),
            meshes: staged,
        });
        self.accepted = Arc::clone(&snapshot);
        Ok(snapshot)
    }
}

impl Default for AssetImportDatabase {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_SOURCE_BYTES)
    }
}

/// Parses and validates one trusted public fixture-mesh source document.
///
/// # Errors
///
/// Rejects malformed, oversized, unsupported, cancelled, or invalid geometry.
pub fn import_fixture_mesh(
    bytes: &[u8],
    cancellation: &CancellationToken,
) -> Result<ImportedFixtureMesh, ImportError> {
    cancellation_checkpoint(cancellation)?;
    if bytes.len() > DEFAULT_MAX_SOURCE_BYTES {
        return Err(ImportError::SourceTooLarge {
            size: bytes.len(),
            max: DEFAULT_MAX_SOURCE_BYTES,
        });
    }
    let normalized_bytes = normalize_text_line_endings(bytes);
    let source: FixtureMeshSource = serde_json::from_slice(&normalized_bytes)
        .map_err(|error| ImportError::InvalidJson(error.to_string()))?;
    cancellation_checkpoint(cancellation)?;
    if source.schema != FIXTURE_MESH_SCHEMA || source.version != 1 {
        return Err(ImportError::UnsupportedSchema {
            schema: source.schema,
            version: source.version,
        });
    }
    if source.authority == SourceAuthority::DerivedCache {
        return Err(ImportError::InvalidAuthority);
    }
    validate_mesh(&source.vertices, &source.indices)?;
    let canonical_name = source.source_id.trim().to_owned();
    if canonical_name.is_empty() || canonical_name.len() > 256 {
        return Err(ImportError::InvalidSourceId);
    }
    let source_id = SourceId::from_canonical_name(&canonical_name);
    let dependencies = source
        .dependencies
        .iter()
        .map(|name| SourceId::from_canonical_name(name.trim()))
        .collect::<Vec<_>>();
    let source_hash = ArtifactHash::digest(&normalized_bytes);
    let vertex_bytes = encode_visual_bytes(&source.vertices, &source.indices);
    let collision_positions = source
        .vertices
        .iter()
        .map(|vertex| vertex.position)
        .collect::<Vec<_>>();
    let collision_bytes = encode_collision_bytes(&collision_positions, &source.indices);

    Ok(ImportedFixtureMesh {
        canonical_name,
        metadata: SourceMetadata {
            source_id,
            schema: FIXTURE_MESH_SCHEMA.to_owned(),
            schema_version: 1,
            authority: source.authority,
            importer_version: FIXTURE_MESH_IMPORTER_VERSION.to_owned(),
            source_hash,
            dependencies,
            provenance: SourceProvenance {
                origin: source.provenance.origin,
                license: source.provenance.license,
                attribution: source.provenance.attribution,
            },
        },
        visual: VisualMeshFacet {
            vertices: source.vertices,
            indices: source.indices.clone(),
            artifact_hash: ArtifactHash::digest(&vertex_bytes),
        },
        collision: CollisionMeshFacet {
            positions: collision_positions,
            indices: source.indices,
            artifact_hash: ArtifactHash::digest(&collision_bytes),
        },
        preserved_unknown: source.unknown,
    })
}

fn normalize_text_line_endings(bytes: &[u8]) -> Cow<'_, [u8]> {
    if !bytes.contains(&b'\r') {
        return Cow::Borrowed(bytes);
    }

    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            normalized.push(b'\n');
            if bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
        } else {
            normalized.push(bytes[index]);
        }
        index += 1;
    }
    Cow::Owned(normalized)
}

fn resolve_project_path(canonical_root: &Path, relative: &Path) -> Result<PathBuf, ImportError> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ImportError::ProjectRootEscape(relative.to_path_buf()));
    }
    let candidate = canonical_root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| ImportError::Io(error.to_string()))?;
    if !canonical.starts_with(canonical_root) {
        return Err(ImportError::ProjectRootEscape(relative.to_path_buf()));
    }
    Ok(canonical)
}

fn validate_mesh(vertices: &[FixtureVertexSource], indices: &[u32]) -> Result<(), ImportError> {
    if vertices.len() < 3 || vertices.len() > MAX_VERTICES {
        return Err(ImportError::InvalidGeometry(
            "vertex count is outside bounds",
        ));
    }
    if indices.len() < 3 || indices.len() > MAX_INDICES || !indices.len().is_multiple_of(3) {
        return Err(ImportError::InvalidGeometry(
            "indices must contain bounded triangles",
        ));
    }
    if vertices
        .iter()
        .flat_map(|vertex| {
            vertex
                .position
                .iter()
                .chain(vertex.normal.iter())
                .chain(vertex.color.iter())
                .chain(vertex.uv.iter())
        })
        .any(|value| !value.is_finite())
    {
        return Err(ImportError::InvalidGeometry("vertex values must be finite"));
    }
    let vertex_count = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
    if indices.iter().any(|index| *index >= vertex_count) {
        return Err(ImportError::InvalidGeometry(
            "index references a missing vertex",
        ));
    }
    Ok(())
}

fn validate_dependencies(
    meshes: &BTreeMap<SourceId, ImportedFixtureMesh>,
) -> Result<(), ImportError> {
    let known = meshes.keys().copied().collect::<BTreeSet<_>>();
    for mesh in meshes.values() {
        for dependency in &mesh.metadata.dependencies {
            if !known.contains(dependency) {
                return Err(ImportError::MissingDependency {
                    source: mesh.metadata.source_id,
                    dependency: *dependency,
                });
            }
        }
    }
    Ok(())
}

fn cancellation_checkpoint(cancellation: &CancellationToken) -> Result<(), ImportError> {
    if cancellation.is_cancelled() {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}

fn encode_visual_bytes(vertices: &[FixtureVertexSource], indices: &[u32]) -> Vec<u8> {
    let facet = VisualMeshFacet {
        vertices: vertices.to_vec(),
        indices: indices.to_vec(),
        artifact_hash: ArtifactHash::digest(&[]),
    };
    let mut bytes = facet.encode_vertex_bytes();
    bytes.extend_from_slice(&facet.encode_index_bytes());
    bytes
}

fn encode_collision_bytes(positions: &[[f32; 3]], indices: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        positions
            .len()
            .saturating_mul(12)
            .saturating_add(indices.len().saturating_mul(4)),
    );
    for position in positions {
        for value in position {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportError {
    Io(String),
    InvalidJson(String),
    UnsupportedSchema {
        schema: String,
        version: u32,
    },
    InvalidAuthority,
    InvalidSourceId,
    InvalidGeometry(&'static str),
    SourceTooLarge {
        size: usize,
        max: usize,
    },
    ProjectRootEscape(PathBuf),
    DuplicateSourceId(SourceId),
    MissingDependency {
        source: SourceId,
        dependency: SourceId,
    },
    Cancelled,
}

impl Display for ImportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "source IO failed: {message}"),
            Self::InvalidJson(message) => write!(formatter, "invalid fixture JSON: {message}"),
            Self::UnsupportedSchema { schema, version } => {
                write!(
                    formatter,
                    "unsupported fixture schema {schema} version {version}"
                )
            }
            Self::InvalidAuthority => {
                formatter.write_str("derived-cache authority cannot be imported as source")
            }
            Self::InvalidSourceId => formatter.write_str("source ID is empty or exceeds 256 bytes"),
            Self::InvalidGeometry(message) => {
                write!(formatter, "invalid fixture geometry: {message}")
            }
            Self::SourceTooLarge { size, max } => {
                write!(formatter, "source is {size} bytes; maximum is {max}")
            }
            Self::ProjectRootEscape(path) => {
                write!(
                    formatter,
                    "source path escapes project root: {}",
                    path.display()
                )
            }
            Self::DuplicateSourceId(id) => write!(formatter, "duplicate source ID: {id}"),
            Self::MissingDependency { source, dependency } => {
                write!(
                    formatter,
                    "source {source} is missing dependency {dependency}"
                )
            }
            Self::Cancelled => formatter.write_str("source import was cancelled"),
        }
    }
}

impl Error for ImportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_json(source_id: &str, extra: &str) -> Vec<u8> {
        format!(
            r#"{{
  "schema":"meridian.fixture-mesh/v1","version":1,"source_id":"{source_id}",
  "authority":"engine_fixture","provenance":{{"origin":"Meridian MS-01","license":"CC0-1.0"}},
  "dependencies":[],
  "vertices":[
    {{"position":[-0.5,-0.5,0.0],"normal":[0.0,0.0,1.0],"color":[1.0,0.1,0.1,1.0],"uv":[0.0,1.0]}},
    {{"position":[0.5,-0.5,0.0],"normal":[0.0,0.0,1.0],"color":[0.1,1.0,0.1,1.0],"uv":[1.0,1.0]}},
    {{"position":[0.0,0.5,0.0],"normal":[0.0,0.0,1.0],"color":[0.1,0.1,1.0,1.0],"uv":[0.5,0.0]}}
  ],"indices":[0,1,2]{extra}
}}"#
        )
        .into_bytes()
    }

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("meridian-asset-tools-{name}-{nonce}"))
    }

    #[test]
    fn deterministic_import_preserves_unknown_fields_and_separates_facets() {
        let bytes = fixture_json("fixtures/ms01/triangle", r#", "future":{"kept":true}"#);
        let cancellation = CancellationToken::new();
        let first = import_fixture_mesh(&bytes, &cancellation).expect("fixture imports");
        let second = import_fixture_mesh(&bytes, &cancellation).expect("fixture imports");
        let windows_bytes = bytes
            .split(|byte| *byte == b'\n')
            .collect::<Vec<_>>()
            .join(&b"\r\n"[..]);
        let windows =
            import_fixture_mesh(&windows_bytes, &cancellation).expect("CRLF fixture imports");

        assert_eq!(first, second);
        assert_eq!(first, windows);
        assert!(first.preserved_unknown.contains_key("future"));
        assert_eq!(first.visual.vertices.len(), first.collision.positions.len());
        assert_ne!(first.visual.artifact_hash, first.collision.artifact_hash);
        assert_eq!(first.visual.encode_vertex_bytes().len(), 3 * 48);
        let compiled = first.visual.encode_compiled();
        let decoded = decode_compiled_visual(&compiled).expect("compiled visual decodes");
        assert_eq!(decoded.vertex_data, first.visual.encode_vertex_bytes());
        assert_eq!(decoded.indices, first.visual.indices);
        assert!(first.collision.encode_compiled().len() > FACET_HEADER_BYTES);
    }

    #[test]
    fn transaction_rejects_duplicates_cancel_and_escape_without_replacing_snapshot() {
        let root = test_root("transaction");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("a.json"), fixture_json("same", "")).expect("write source");
        fs::write(root.join("b.json"), fixture_json("same", "")).expect("write source");
        let token = CancellationToken::new();
        let mut database = AssetImportDatabase::default();
        let original = database.snapshot();

        assert!(matches!(
            database.import_files_transaction(
                &root,
                &[PathBuf::from("a.json"), PathBuf::from("b.json")],
                &token,
            ),
            Err(ImportError::DuplicateSourceId(_))
        ));
        assert!(Arc::ptr_eq(&original, &database.snapshot()));
        token.cancel();
        assert!(matches!(
            database.import_files_transaction(&root, &[PathBuf::from("a.json")], &token),
            Err(ImportError::Cancelled)
        ));
        assert!(matches!(
            database.import_files_transaction(
                &root,
                &[PathBuf::from("../outside.json")],
                &CancellationToken::new(),
            ),
            Err(ImportError::ProjectRootEscape(_))
        ));
        fs::remove_dir_all(root).expect("clean root");
    }

    #[test]
    fn invalid_geometry_and_cache_authority_are_rejected() {
        let cancellation = CancellationToken::new();
        let bad_index = String::from_utf8(fixture_json("bad", ""))
            .expect("utf8")
            .replace("[0,1,2]", "[0,1,9]");
        assert!(matches!(
            import_fixture_mesh(bad_index.as_bytes(), &cancellation),
            Err(ImportError::InvalidGeometry(_))
        ));
        let cache = String::from_utf8(fixture_json("cache", ""))
            .expect("utf8")
            .replace("engine_fixture", "derived_cache");
        assert_eq!(
            import_fixture_mesh(cache.as_bytes(), &cancellation),
            Err(ImportError::InvalidAuthority)
        );
    }

    #[cfg(unix)]
    #[test]
    fn transaction_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = test_root("symlink-root");
        let outside = test_root("symlink-outside");
        fs::create_dir_all(&root).expect("create root");
        fs::create_dir_all(&outside).expect("create outside");
        fs::write(outside.join("mesh.json"), fixture_json("outside", ""))
            .expect("write outside source");
        symlink(outside.join("mesh.json"), root.join("linked.json")).expect("create symlink");

        let mut database = AssetImportDatabase::default();
        assert!(matches!(
            database.import_files_transaction(
                &root,
                &[PathBuf::from("linked.json")],
                &CancellationToken::new(),
            ),
            Err(ImportError::ProjectRootEscape(_))
        ));
        fs::remove_dir_all(root).expect("remove root");
        fs::remove_dir_all(outside).expect("remove outside");
    }
}
