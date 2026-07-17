//! Source-authoritative native editable-model foundation.
//!
//! This crate owns versioned model source, stable mesh-element identity,
//! immutable revisions, bounded semantic edits, topology lineage, and durable
//! recovery. It deliberately does not depend on UI, renderer, runtime, game,
//! physics, or animation crates. Penumbra consumes its derived preview
//! descriptor through an editor-side adapter; that descriptor never becomes
//! editable source.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use meridian_alluvium::{GeneratedOverride, OverrideReconciliation, OverrideStatus};
use meridian_core::StableId;
use meridian_save::{SaveConfig, SaveStore};
use serde::{Deserialize, Serialize};

/// The only editable-model source schema accepted by this foundation.
pub const MODEL_SCHEMA: &str = "meridian.editable-model/v1";
/// Current editable-model schema version.
pub const MODEL_VERSION: u32 = 1;
const RECOVERY_SCHEMA: &str = "meridian.modeler-recovery/v1";
const COORDINATE_SYSTEM: &str = "right-handed-y-up-millimetres";
static SOURCE_TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Persistent model IDs always use fixed-width hexadecimal JSON strings. This
/// avoids lossy JSON number handling and permits durable map/set recovery.
mod stable_id_hex {
    use super::StableId;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(id: &StableId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&id.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StableId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse(&value).map_err(serde::de::Error::custom)
    }

    pub(super) fn parse(value: &str) -> Result<StableId, String> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("stable model IDs must be 32 hexadecimal characters".to_owned());
        }
        u128::from_str_radix(value, 16)
            .map(StableId::new)
            .map_err(|error| error.to_string())
    }
}

mod stable_id_hex_array4 {
    use super::{stable_id_hex, StableId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(ids: &[StableId; 4], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ids.map(|id| id.to_string()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[StableId; 4], D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = Vec::<String>::deserialize(deserializer)?;
        let ids = values
            .iter()
            .map(|value| stable_id_hex::parse(value).map_err(serde::de::Error::custom))
            .collect::<Result<Vec<_>, D::Error>>()?;
        ids.try_into()
            .map_err(|_: Vec<StableId>| serde::de::Error::custom("expected four stable IDs"))
    }
}

mod stable_id_hex_vec {
    use super::{stable_id_hex, StableId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(ids: &[StableId], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ids.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<StableId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|value| stable_id_hex::parse(value).map_err(serde::de::Error::custom))
            .collect()
    }
}

mod stable_id_hex_set {
    use std::collections::BTreeSet;

    use super::{stable_id_hex, StableId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(ids: &BTreeSet<StableId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ids.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<StableId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|value| stable_id_hex::parse(value).map_err(serde::de::Error::custom))
            .collect()
    }
}

/// A source-space position or translation in millimetres.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Millimetres3 {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl Millimetres3 {
    fn checked_add(self, other: Self) -> Result<Self, ModelError> {
        Ok(Self {
            x: self
                .x
                .checked_add(other.x)
                .ok_or(ModelError::CoordinateOverflow)?,
            y: self
                .y
                .checked_add(other.y)
                .ok_or(ModelError::CoordinateOverflow)?,
            z: self
                .z
                .checked_add(other.z)
                .ok_or(ModelError::CoordinateOverflow)?,
        })
    }
}

/// A source vertex with a persistent element identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Vertex {
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    pub position_mm: Millimetres3,
}

/// A source edge with a persistent element identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    #[serde(with = "stable_id_hex")]
    pub start: StableId,
    #[serde(with = "stable_id_hex")]
    pub end: StableId,
}

impl Edge {
    fn other(&self, id: StableId) -> Option<StableId> {
        if self.start == id {
            Some(self.end)
        } else if self.end == id {
            Some(self.start)
        } else {
            None
        }
    }

    fn unordered_endpoints(&self) -> (StableId, StableId) {
        if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }
}

/// A source face whose boundary is expressed in stable vertex IDs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    #[serde(with = "stable_id_hex_vec")]
    pub vertices: Vec<StableId>,
}

/// A source-object transform. It is source authority, not a renderer handle.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTransform {
    pub translation_mm: Millimetres3,
}

/// One independently selectable editable mesh object.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeshObject {
    #[serde(with = "stable_id_hex")]
    pub id: StableId,
    pub label: String,
    #[serde(default)]
    pub transform: ModelTransform,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub faces: Vec<Face>,
}

/// Versioned, human-readable editable-model source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDocument {
    pub schema: String,
    pub version: u32,
    #[serde(with = "stable_id_hex")]
    pub document_id: StableId,
    pub label: String,
    pub coordinate_system: String,
    #[serde(default)]
    pub document_generation: u64,
    pub objects: Vec<MeshObject>,
}

impl ModelDocument {
    /// Creates an empty, valid source document ready for a typed primitive edit.
    #[must_use]
    pub fn new(document_id: StableId, label: impl Into<String>) -> Self {
        Self {
            schema: MODEL_SCHEMA.to_owned(),
            version: MODEL_VERSION,
            document_id,
            label: label.into(),
            coordinate_system: COORDINATE_SYSTEM.to_owned(),
            document_generation: 0,
            objects: Vec::new(),
        }
    }

    /// Reads, parses, and validates one regular, bounded model-source file.
    ///
    /// # Errors
    ///
    /// Rejects non-regular paths and inputs exceeding the existing durable-save
    /// payload bound before allocating a source buffer.
    pub fn read_source(path: impl AsRef<Path>) -> Result<Self, ModelError> {
        let path = path.as_ref();
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| ModelError::Io(format!("metadata {}: {error}", path.display())))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(ModelError::InvalidSourcePath(path.display().to_string()));
        }
        let size = usize::try_from(metadata.len()).map_err(|_| ModelError::SourceTooLarge {
            size: usize::MAX,
            max: SaveConfig::default().max_payload_bytes,
        })?;
        let max = SaveConfig::default().max_payload_bytes;
        if size > max {
            return Err(ModelError::SourceTooLarge { size, max });
        }
        let source = fs::read_to_string(path)
            .map_err(|error| ModelError::Io(format!("read {}: {error}", path.display())))?;
        Self::from_json(&source)
    }

    /// Parses and validates versioned editable-model source.
    ///
    /// # Errors
    ///
    /// Rejects source exceeding the existing durable-save payload bound, invalid
    /// JSON, unsupported schemas, or malformed topology.
    pub fn from_json(source: &str) -> Result<Self, ModelError> {
        let max = SaveConfig::default().max_payload_bytes;
        if source.len() > max {
            return Err(ModelError::SourceTooLarge {
                size: source.len(),
                max,
            });
        }
        let document: Self =
            serde_json::from_str(source).map_err(|error| ModelError::Json(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    /// Emits deterministic pretty JSON after sorting source collections by ID.
    ///
    /// # Errors
    ///
    /// Returns a typed validation or serialization failure without changing the
    /// source document.
    pub fn canonical_json(&self) -> Result<String, ModelError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.objects.sort_unstable_by_key(|object| object.id);
        for object in &mut canonical.objects {
            object.vertices.sort_unstable_by_key(|vertex| vertex.id);
            object.edges.sort_unstable_by_key(|edge| edge.id);
            object.faces.sort_unstable_by_key(|face| face.id);
        }
        serde_json::to_string_pretty(&canonical)
            .map_err(|error| ModelError::Json(error.to_string()))
    }

    /// Atomically writes canonical editable-model source to a regular project
    /// path. Derived previews and recovery data are never written here.
    ///
    /// # Errors
    ///
    /// Rejects unsafe paths and leaves the existing accepted source intact when
    /// a temporary write or replacement fails.
    pub fn write_source(&self, path: impl AsRef<Path>) -> Result<(), ModelError> {
        let path = path.as_ref();
        let source = self.canonical_json()?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| ModelError::Io("model source has no parent directory".to_owned()))?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|error| ModelError::Io(format!("{}: {error}", parent.display())))?;
        if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
            return Err(ModelError::InvalidSourcePath(parent.display().to_string()));
        }
        if path.exists() {
            let metadata = fs::symlink_metadata(path)
                .map_err(|error| ModelError::Io(format!("{}: {error}", path.display())))?;
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(ModelError::InvalidSourcePath(path.display().to_string()));
            }
        }
        let temporary = source_temporary_path(path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| ModelError::Io(format!("{}: {error}", temporary.display())))?;
        let result = (|| {
            file.write_all(source.as_bytes())
                .map_err(|error| ModelError::Io(error.to_string()))?;
            file.sync_all()
                .map_err(|error| ModelError::Io(error.to_string()))?;
            fs::rename(&temporary, path).map_err(|error| ModelError::Io(error.to_string()))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Validates schema, source authority, stable identities, and topology.
    ///
    /// # Errors
    ///
    /// Rejects every invalid source condition before a transaction publishes a
    /// new immutable revision.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.schema != MODEL_SCHEMA {
            return Err(ModelError::UnsupportedSchema(self.schema.clone()));
        }
        if self.version != MODEL_VERSION {
            return Err(ModelError::UnsupportedVersion(self.version));
        }
        if self.label.trim().is_empty() {
            return Err(ModelError::InvalidLabel(self.document_id));
        }
        if self.coordinate_system != COORDINATE_SYSTEM {
            return Err(ModelError::UnsupportedCoordinateSystem(
                self.coordinate_system.clone(),
            ));
        }

        let mut all_ids = BTreeSet::new();
        register_id(&mut all_ids, self.document_id)?;
        for object in &self.objects {
            if object.label.trim().is_empty() {
                return Err(ModelError::InvalidLabel(object.id));
            }
            register_id(&mut all_ids, object.id)?;

            let mut vertices = BTreeSet::new();
            for vertex in &object.vertices {
                register_id(&mut all_ids, vertex.id)?;
                if !vertices.insert(vertex.id) {
                    return Err(ModelError::DuplicateId(vertex.id));
                }
            }

            let mut edge_pairs = BTreeSet::new();
            for edge in &object.edges {
                register_id(&mut all_ids, edge.id)?;
                if edge.start == edge.end
                    || !vertices.contains(&edge.start)
                    || !vertices.contains(&edge.end)
                {
                    return Err(ModelError::InvalidEdge(edge.id));
                }
                if !edge_pairs.insert(edge.unordered_endpoints()) {
                    return Err(ModelError::DuplicateEdge(edge.id));
                }
            }

            for face in &object.faces {
                register_id(&mut all_ids, face.id)?;
                if face.vertices.len() < 3 {
                    return Err(ModelError::InvalidFace(face.id));
                }
                let mut boundary = BTreeSet::new();
                for vertex in &face.vertices {
                    if !vertices.contains(vertex) || !boundary.insert(*vertex) {
                        return Err(ModelError::InvalidFace(face.id));
                    }
                }
                for index in 0..face.vertices.len() {
                    let start = face.vertices[index];
                    let end = face.vertices[(index + 1) % face.vertices.len()];
                    let pair = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    if !edge_pairs.contains(&pair) {
                        return Err(ModelError::FaceBoundaryMissingEdge {
                            face: face.id,
                            start,
                            end,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns the object with the requested stable source identity.
    #[must_use]
    pub fn object(&self, id: StableId) -> Option<&MeshObject> {
        self.objects.iter().find(|object| object.id == id)
    }

    /// Returns the stable kind of one selectable source identity.
    #[must_use]
    pub fn element_kind(&self, id: StableId) -> Option<ModelElementKind> {
        if self.objects.iter().any(|object| object.id == id) {
            return Some(ModelElementKind::Object);
        }
        for object in &self.objects {
            if object.vertices.iter().any(|vertex| vertex.id == id) {
                return Some(ModelElementKind::Vertex);
            }
            if object.edges.iter().any(|edge| edge.id == id) {
                return Some(ModelElementKind::Edge);
            }
            if object.faces.iter().any(|face| face.id == id) {
                return Some(ModelElementKind::Face);
            }
        }
        None
    }

    /// Builds a derived, renderer-owned preview input without mutating source.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown objects or index conversion overflow.
    pub fn penumbra_preview(&self, object_id: StableId) -> Result<PenumbraPreview, ModelError> {
        self.validate()?;
        let object = self
            .object(object_id)
            .ok_or(ModelError::UnknownObject(object_id))?;
        let mut vertex_indices = BTreeMap::new();
        let mut positions = Vec::with_capacity(object.vertices.len());
        for (index, vertex) in object.vertices.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| ModelError::PreviewIndexOverflow)?;
            vertex_indices.insert(vertex.id, index);
            positions.push(
                vertex
                    .position_mm
                    .checked_add(object.transform.translation_mm)?,
            );
        }
        let mut triangle_indices = Vec::new();
        for face in &object.faces {
            let first = *vertex_indices
                .get(&face.vertices[0])
                .ok_or(ModelError::InvalidFace(face.id))?;
            for index in 1..face.vertices.len() - 1 {
                let second = *vertex_indices
                    .get(&face.vertices[index])
                    .ok_or(ModelError::InvalidFace(face.id))?;
                let third = *vertex_indices
                    .get(&face.vertices[index + 1])
                    .ok_or(ModelError::InvalidFace(face.id))?;
                triangle_indices.extend([first, second, third]);
            }
        }
        Ok(PenumbraPreview {
            source_document_id: self.document_id,
            source_generation: self.document_generation,
            object_id,
            positions_mm: positions,
            triangle_indices,
        })
    }

    fn contains_any_id(&self, id: StableId) -> bool {
        id == self.document_id || self.element_kind(id).is_some()
    }
}

fn source_temporary_path(path: &Path) -> PathBuf {
    let sequence = SOURCE_TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary = OsString::from(path.as_os_str());
    temporary.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    PathBuf::from(temporary)
}

/// The selectable kind of a source-model element.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelElementKind {
    Object,
    Vertex,
    Edge,
    Face,
}

/// A generation-checked source selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSelection {
    pub document_generation: u64,
    pub kind: ModelElementKind,
    #[serde(with = "stable_id_hex_set")]
    pub ids: BTreeSet<StableId>,
}

impl ModelSelection {
    /// Selects source elements against a specific immutable document revision.
    ///
    /// # Errors
    ///
    /// Rejects unknown IDs and mixed element kinds without changing source.
    pub fn new(
        document: &ModelDocument,
        kind: ModelElementKind,
        ids: impl IntoIterator<Item = StableId>,
    ) -> Result<Self, ModelError> {
        let selection = Self {
            document_generation: document.document_generation,
            kind,
            ids: ids.into_iter().collect(),
        };
        selection.validate(document)?;
        Ok(selection)
    }

    fn empty(document_generation: u64) -> Self {
        Self {
            document_generation,
            kind: ModelElementKind::Object,
            ids: BTreeSet::new(),
        }
    }

    /// Rejects a selection from another document generation or kind.
    ///
    /// # Errors
    ///
    /// Returns a typed stale-selection error before a command can mutate source.
    pub fn validate(&self, document: &ModelDocument) -> Result<(), ModelError> {
        if self.document_generation != document.document_generation {
            return Err(ModelError::StaleSelection {
                selection_generation: self.document_generation,
                document_generation: document.document_generation,
            });
        }
        for id in &self.ids {
            match document.element_kind(*id) {
                Some(kind) if kind == self.kind => {}
                Some(kind) => {
                    return Err(ModelError::SelectionKindMismatch {
                        id: *id,
                        expected: self.kind,
                        actual: kind,
                    })
                }
                None => return Err(ModelError::UnknownElement(*id)),
            }
        }
        Ok(())
    }
}

/// Stable identities assigned by the caller for a bounded quad primitive.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuadPrimitiveIds {
    #[serde(with = "stable_id_hex")]
    pub object: StableId,
    #[serde(with = "stable_id_hex_array4")]
    pub vertices: [StableId; 4],
    #[serde(with = "stable_id_hex_array4")]
    pub edges: [StableId; 4],
    #[serde(with = "stable_id_hex")]
    pub face: StableId,
}

/// A bounded source primitive supported by the MS-03 foundation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuadPrimitive {
    pub ids: QuadPrimitiveIds,
    pub label: String,
    pub origin_mm: Millimetres3,
    pub half_extent_mm: i64,
}

/// The one primitive family intentionally exposed by this foundation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum PrimitiveCreate {
    Quad(QuadPrimitive),
}

/// A typed split-edge request, the sole topology-changing operation in MS-03.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SplitEdge {
    #[serde(with = "stable_id_hex")]
    pub object_id: StableId,
    #[serde(with = "stable_id_hex")]
    pub edge_id: StableId,
    pub new_vertex: Vertex,
    pub replacement_edges: [Edge; 2],
}

/// A semantic source mutation. UI events never enter this type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ModelCommand {
    CreatePrimitive(PrimitiveCreate),
    TranslateObject {
        #[serde(with = "stable_id_hex")]
        object_id: StableId,
        translation_mm: Millimetres3,
    },
    SplitEdge(SplitEdge),
}

/// One typed source transaction with an optional generation-checked selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTransaction {
    pub label: String,
    pub command: ModelCommand,
    pub selection: Option<ModelSelection>,
}

impl ModelTransaction {
    /// Creates a transaction with an explicit semantic label.
    #[must_use]
    pub fn new(label: impl Into<String>, command: ModelCommand) -> Self {
        Self {
            label: label.into(),
            command,
            selection: None,
        }
    }

    /// Binds a previously validated selection to this source transaction.
    #[must_use]
    pub fn with_selection(mut self, selection: ModelSelection) -> Self {
        self.selection = Some(selection);
        self
    }
}

/// Explicit identity lineage emitted by every topology-changing transaction.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyMap {
    #[serde(with = "stable_id_hex_set")]
    pub prior_elements: BTreeSet<StableId>,
    #[serde(with = "stable_id_hex_set")]
    pub resulting_elements: BTreeSet<StableId>,
    pub split_merge_lineage: Vec<TopologyLineage>,
    #[serde(with = "stable_id_hex_set")]
    pub orphaned_elements: BTreeSet<StableId>,
}

impl TopologyMap {
    /// Returns the resulting identities for one prior stable identity.
    #[must_use]
    pub fn lineage_for(&self, prior: StableId) -> Option<&[StableId]> {
        self.split_merge_lineage
            .iter()
            .find(|lineage| lineage.prior == prior)
            .map(|lineage| lineage.resulting.as_slice())
    }

    /// Migrates unambiguous selection identities and retains ambiguity as an
    /// explicit orphan requiring user resolution.
    #[must_use]
    pub fn migrate_selection(
        &self,
        selection: &ModelSelection,
        document_generation: u64,
    ) -> SelectionMigration {
        let mut migrated = BTreeSet::new();
        let mut orphaned = BTreeSet::new();
        for id in &selection.ids {
            match self.lineage_for(*id) {
                Some(targets) if targets.len() == 1 => {
                    migrated.insert(targets[0]);
                }
                Some(_) => {
                    orphaned.insert(*id);
                }
                None if self.orphaned_elements.contains(id) => {
                    orphaned.insert(*id);
                }
                None => {
                    migrated.insert(*id);
                }
            }
        }
        SelectionMigration {
            selection: ModelSelection {
                document_generation,
                kind: selection.kind,
                ids: migrated,
            },
            orphaned,
        }
    }

    /// Reconciles an Alluvium-owned generated override without taking recipe or
    /// override authority away from Alluvium.
    #[must_use]
    pub fn reconcile_generated_override(
        &self,
        generated_override: &GeneratedOverride,
        document: &ModelDocument,
    ) -> OverrideReconciliation {
        let target = generated_override.target;
        if document.element_kind(target).is_some() {
            return OverrideReconciliation {
                target,
                status: OverrideStatus::Applied,
                detail: "target stable identity survived the model edit".to_owned(),
            };
        }
        match self.lineage_for(target) {
            Some(targets) if targets.len() == 1 => OverrideReconciliation {
                target,
                status: OverrideStatus::Migrated,
                detail: format!("target migrated to {}", targets[0]),
            },
            Some(targets) => OverrideReconciliation {
                target,
                status: OverrideStatus::Conflicted,
                detail: format!(
                    "target split or merged into {}; an explicit override decision is required",
                    targets
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
            None if self.orphaned_elements.contains(&target) => OverrideReconciliation {
                target,
                status: OverrideStatus::Orphaned,
                detail: "target was explicitly orphaned by the model edit".to_owned(),
            },
            None => OverrideReconciliation {
                target,
                status: OverrideStatus::Orphaned,
                detail: "target is absent from the accepted source revision".to_owned(),
            },
        }
    }
}

/// One JSON-safe stable identity lineage record emitted by a topology change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyLineage {
    #[serde(with = "stable_id_hex")]
    pub prior: StableId,
    #[serde(with = "stable_id_hex_vec")]
    pub resulting: Vec<StableId>,
}

/// Result of a topology-map selection migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionMigration {
    pub selection: ModelSelection,
    pub orphaned: BTreeSet<StableId>,
}

/// One immutable source revision accepted by the modeler.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRevision {
    document: ModelDocument,
}

impl ModelRevision {
    fn new(document: ModelDocument) -> Self {
        Self { document }
    }

    /// Returns the source-authoritative document for this immutable revision.
    #[must_use]
    pub fn document(&self) -> &ModelDocument {
        &self.document
    }

    /// Returns the source generation of this immutable revision.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.document.document_generation
    }
}

/// A renderer-owned, derived preview input with no editable-source authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PenumbraPreview {
    pub source_document_id: StableId,
    pub source_generation: u64,
    pub object_id: StableId,
    pub positions_mm: Vec<Millimetres3>,
    pub triangle_indices: Vec<u32>,
}

impl PenumbraPreview {
    /// Returns whether this preview is derived rather than source authority.
    #[must_use]
    pub const fn is_derived(&self) -> bool {
        true
    }
}

/// Outcome of one accepted semantic model transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelOperationReceipt {
    pub generation: u64,
    pub topology_map: Option<TopologyMap>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelHistoryEntry {
    transaction: ModelTransaction,
    before: ModelRevision,
    after: ModelRevision,
    topology_map: Option<TopologyMap>,
}

/// Source-authoritative session history with immutable semantic revisions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSession {
    current: ModelRevision,
    selection: ModelSelection,
    undo: Vec<ModelHistoryEntry>,
    redo: Vec<ModelHistoryEntry>,
}

impl ModelSession {
    /// Opens a valid source document as the first immutable model revision.
    ///
    /// # Errors
    ///
    /// Returns source validation failures without constructing a session.
    pub fn open(document: ModelDocument) -> Result<Self, ModelError> {
        document.validate()?;
        let generation = document.document_generation;
        Ok(Self {
            current: ModelRevision::new(document),
            selection: ModelSelection::empty(generation),
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// Returns the accepted immutable source revision.
    #[must_use]
    pub fn current(&self) -> &ModelRevision {
        &self.current
    }

    /// Returns the session's generation-checked selection.
    #[must_use]
    pub fn selection(&self) -> &ModelSelection {
        &self.selection
    }

    /// Replaces the selection against the current immutable revision.
    ///
    /// # Errors
    ///
    /// Rejects unknown or wrong-kind identities without changing selection.
    pub fn select(
        &mut self,
        kind: ModelElementKind,
        ids: impl IntoIterator<Item = StableId>,
    ) -> Result<(), ModelError> {
        self.selection = ModelSelection::new(self.current.document(), kind, ids)?;
        Ok(())
    }

    /// Applies one typed source transaction atomically and retains semantic
    /// before/after revisions for undo, redo, and durable recovery.
    ///
    /// # Errors
    ///
    /// Rejects invalid labels, stale selections, invalid topology, or generation
    /// overflow without replacing accepted source.
    pub fn apply(
        &mut self,
        transaction: ModelTransaction,
    ) -> Result<ModelOperationReceipt, ModelError> {
        let before = self.current.clone();
        let (after, topology_map) = accept_transaction(before.document(), &transaction)?;
        let receipt = ModelOperationReceipt {
            generation: after.generation(),
            topology_map: topology_map.clone(),
        };
        self.current = after.clone();
        self.undo.push(ModelHistoryEntry {
            transaction,
            before,
            after,
            topology_map,
        });
        self.redo.clear();
        self.selection = ModelSelection::empty(self.current.generation());
        Ok(receipt)
    }

    /// Restores the exact prior semantic source revision.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::NothingToUndo`] when no accepted transaction remains.
    pub fn undo(&mut self) -> Result<(), ModelError> {
        let entry = self.undo.pop().ok_or(ModelError::NothingToUndo)?;
        self.current = entry.before.clone();
        self.selection = ModelSelection::empty(self.current.generation());
        self.redo.push(entry);
        Ok(())
    }

    /// Restores the exact accepted semantic revision that was undone.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::NothingToRedo`] when no undone transaction remains.
    pub fn redo(&mut self) -> Result<(), ModelError> {
        let entry = self.redo.pop().ok_or(ModelError::NothingToRedo)?;
        self.current = entry.after.clone();
        self.selection = ModelSelection::empty(self.current.generation());
        self.undo.push(entry);
        Ok(())
    }

    fn validate_recovered(&mut self) -> Result<(), ModelError> {
        self.current.document().validate()?;
        for entry in self.undo.iter().chain(&self.redo) {
            validate_history_entry(entry)?;
        }
        if !history_stack_is_contiguous(&self.undo)
            || !redo_stack_is_contiguous(&self.redo)
            || self
                .undo
                .last()
                .is_some_and(|entry| entry.after != self.current)
            || self
                .redo
                .last()
                .is_some_and(|entry| entry.before != self.current)
        {
            return Err(ModelError::RecoveryHistoryMismatch);
        }
        if self.selection.validate(self.current.document()).is_err() {
            self.selection = ModelSelection::empty(self.current.generation());
        }
        Ok(())
    }
}

/// Durable crash-recovery boundary for a [`ModelSession`].
pub struct ModelRecoveryStore {
    store: SaveStore,
}

impl ModelRecoveryStore {
    /// Creates a durable recovery store rooted at a caller-owned path.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            store: SaveStore::new(path.as_ref(), SaveConfig::default()),
        }
    }

    /// Returns the primary recovery path.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.store.path()
    }

    /// Atomically persists source revisions and semantic history.
    ///
    /// # Errors
    ///
    /// Returns a typed encoding or durable-save failure without changing the
    /// in-memory accepted revision.
    pub fn save(&self, session: &ModelSession) -> Result<(), ModelError> {
        let snapshot = ModelRecoverySnapshot {
            schema: RECOVERY_SCHEMA.to_owned(),
            session: session.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| ModelError::RecoveryEncoding(error.to_string()))?;
        self.store
            .save(bytes)
            .map_err(|error| ModelError::RecoverySave(error.to_string()))
    }

    /// Recovers the latest valid source/history snapshot or its prior backup.
    ///
    /// # Errors
    ///
    /// Rejects invalid recovery schemas or corrupted source/history without
    /// publishing an invalid session.
    pub fn load(&self) -> Result<ModelSession, ModelError> {
        let bytes = self
            .store
            .load()
            .map_err(|error| ModelError::RecoverySave(error.to_string()))?;
        let snapshot: ModelRecoverySnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| ModelError::RecoveryDecoding(error.to_string()))?;
        if snapshot.schema != RECOVERY_SCHEMA {
            return Err(ModelError::UnsupportedRecoverySchema(snapshot.schema));
        }
        let mut session = snapshot.session;
        session.validate_recovered()?;
        Ok(session)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelRecoverySnapshot {
    schema: String,
    session: ModelSession,
}

/// Typed failures for editable source, selection, topology, and recovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelError {
    Json(String),
    Io(String),
    InvalidSourcePath(String),
    SourceTooLarge {
        size: usize,
        max: usize,
    },
    UnsupportedSchema(String),
    UnsupportedVersion(u32),
    UnsupportedCoordinateSystem(String),
    InvalidLabel(StableId),
    DuplicateId(StableId),
    DuplicateEdge(StableId),
    InvalidEdge(StableId),
    InvalidFace(StableId),
    FaceBoundaryMissingEdge {
        face: StableId,
        start: StableId,
        end: StableId,
    },
    InvalidPrimitive,
    CoordinateOverflow,
    UnknownObject(StableId),
    UnknownElement(StableId),
    StaleSelection {
        selection_generation: u64,
        document_generation: u64,
    },
    SelectionKindMismatch {
        id: StableId,
        expected: ModelElementKind,
        actual: ModelElementKind,
    },
    SelectionRequired(ModelElementKind),
    SelectionMissing(StableId),
    InvalidSplitEdge(StableId),
    InvalidTransactionLabel,
    GenerationExhausted,
    PreviewIndexOverflow,
    NothingToUndo,
    NothingToRedo,
    RecoveryEncoding(String),
    RecoveryDecoding(String),
    RecoverySave(String),
    UnsupportedRecoverySchema(String),
    RecoveryHistoryMismatch,
}

impl Display for ModelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid editable-model JSON: {error}"),
            Self::Io(error) => write!(formatter, "editable-model IO failed: {error}"),
            Self::InvalidSourcePath(path) => write!(formatter, "model source path is not a regular file: {path}"),
            Self::SourceTooLarge { size, max } => write!(formatter, "model source has {size} bytes; maximum is {max}"),
            Self::UnsupportedSchema(schema) => write!(formatter, "unsupported editable-model schema: {schema}"),
            Self::UnsupportedVersion(version) => write!(formatter, "unsupported editable-model version: {version}"),
            Self::UnsupportedCoordinateSystem(system) => write!(formatter, "unsupported model coordinate system: {system}"),
            Self::InvalidLabel(id) => write!(formatter, "model source label is empty for {id}"),
            Self::DuplicateId(id) => write!(formatter, "duplicate stable model ID: {id}"),
            Self::DuplicateEdge(id) => write!(formatter, "duplicate edge endpoints for {id}"),
            Self::InvalidEdge(id) => write!(formatter, "invalid edge {id}"),
            Self::InvalidFace(id) => write!(formatter, "invalid face {id}"),
            Self::FaceBoundaryMissingEdge { face, start, end } => write!(formatter, "face {face} has no source edge for boundary {start} to {end}"),
            Self::InvalidPrimitive => formatter.write_str("primitive parameters or identities are invalid"),
            Self::CoordinateOverflow => formatter.write_str("model coordinate calculation overflowed"),
            Self::UnknownObject(id) => write!(formatter, "unknown model object {id}"),
            Self::UnknownElement(id) => write!(formatter, "unknown model element {id}"),
            Self::StaleSelection { selection_generation, document_generation } => write!(formatter, "selection generation {selection_generation} is stale for document generation {document_generation}"),
            Self::SelectionKindMismatch { id, expected, actual } => write!(formatter, "selection {id} has kind {actual:?}; expected {expected:?}"),
            Self::SelectionRequired(kind) => write!(formatter, "a current {kind:?} selection is required"),
            Self::SelectionMissing(id) => write!(formatter, "required source selection does not contain {id}"),
            Self::InvalidSplitEdge(id) => write!(formatter, "invalid bounded split-edge request for {id}"),
            Self::InvalidTransactionLabel => formatter.write_str("model transaction label is empty"),
            Self::GenerationExhausted => formatter.write_str("model document generation is exhausted"),
            Self::PreviewIndexOverflow => formatter.write_str("model preview index cannot fit u32"),
            Self::NothingToUndo => formatter.write_str("no model transaction is available to undo"),
            Self::NothingToRedo => formatter.write_str("no model transaction is available to redo"),
            Self::RecoveryEncoding(error) => write!(formatter, "model recovery encoding failed: {error}"),
            Self::RecoveryDecoding(error) => write!(formatter, "model recovery decoding failed: {error}"),
            Self::RecoverySave(error) => write!(formatter, "model recovery durable save failed: {error}"),
            Self::UnsupportedRecoverySchema(schema) => write!(formatter, "unsupported model recovery schema: {schema}"),
            Self::RecoveryHistoryMismatch => formatter.write_str(
                "model recovery history does not reproduce its accepted source revision",
            ),
        }
    }
}

impl Error for ModelError {}

fn register_id(ids: &mut BTreeSet<StableId>, id: StableId) -> Result<(), ModelError> {
    if ids.insert(id) {
        Ok(())
    } else {
        Err(ModelError::DuplicateId(id))
    }
}

fn validate_command_selection(
    document: &ModelDocument,
    transaction: &ModelTransaction,
) -> Result<(), ModelError> {
    if let Some(selection) = &transaction.selection {
        selection.validate(document)?;
    }
    let required = match &transaction.command {
        ModelCommand::CreatePrimitive(_) => return Ok(()),
        ModelCommand::TranslateObject { object_id, .. } => (ModelElementKind::Object, *object_id),
        ModelCommand::SplitEdge(split) => (ModelElementKind::Edge, split.edge_id),
    };
    let selection = transaction
        .selection
        .as_ref()
        .ok_or(ModelError::SelectionRequired(required.0))?;
    selection.validate(document)?;
    if selection.kind != required.0 {
        return Err(ModelError::SelectionRequired(required.0));
    }
    if !selection.ids.contains(&required.1) {
        return Err(ModelError::SelectionMissing(required.1));
    }
    Ok(())
}

fn accept_transaction(
    document: &ModelDocument,
    transaction: &ModelTransaction,
) -> Result<(ModelRevision, Option<TopologyMap>), ModelError> {
    if transaction.label.trim().is_empty() {
        return Err(ModelError::InvalidTransactionLabel);
    }
    validate_command_selection(document, transaction)?;
    let (mut updated, topology_map) = apply_command(document, &transaction.command)?;
    updated.document_generation = updated
        .document_generation
        .checked_add(1)
        .ok_or(ModelError::GenerationExhausted)?;
    updated.validate()?;
    Ok((ModelRevision::new(updated), topology_map))
}

fn validate_history_entry(entry: &ModelHistoryEntry) -> Result<(), ModelError> {
    entry.before.document().validate()?;
    entry.after.document().validate()?;
    let (expected_after, expected_topology_map) =
        accept_transaction(entry.before.document(), &entry.transaction)?;
    if expected_after != entry.after || expected_topology_map != entry.topology_map {
        return Err(ModelError::RecoveryHistoryMismatch);
    }
    Ok(())
}

fn history_stack_is_contiguous(history: &[ModelHistoryEntry]) -> bool {
    history
        .windows(2)
        .all(|entries| entries[0].after == entries[1].before)
}

fn redo_stack_is_contiguous(history: &[ModelHistoryEntry]) -> bool {
    (1..history.len())
        .rev()
        .all(|index| history[index].after == history[index - 1].before)
}

fn apply_command(
    current: &ModelDocument,
    command: &ModelCommand,
) -> Result<(ModelDocument, Option<TopologyMap>), ModelError> {
    let mut document = current.clone();
    match command {
        ModelCommand::CreatePrimitive(primitive) => {
            let object = create_primitive(&document, primitive)?;
            document.objects.push(object);
            Ok((document, None))
        }
        ModelCommand::TranslateObject {
            object_id,
            translation_mm,
        } => {
            let object = document
                .objects
                .iter_mut()
                .find(|object| object.id == *object_id)
                .ok_or(ModelError::UnknownObject(*object_id))?;
            object.transform.translation_mm = object
                .transform
                .translation_mm
                .checked_add(*translation_mm)?;
            Ok((document, None))
        }
        ModelCommand::SplitEdge(split) => {
            let topology_map = split_edge(&mut document, split)?;
            Ok((document, Some(topology_map)))
        }
    }
}

fn create_primitive(
    document: &ModelDocument,
    primitive: &PrimitiveCreate,
) -> Result<MeshObject, ModelError> {
    match primitive {
        PrimitiveCreate::Quad(quad) => {
            if quad.label.trim().is_empty() || quad.half_extent_mm <= 0 {
                return Err(ModelError::InvalidPrimitive);
            }
            let mut ids = BTreeSet::new();
            for id in [quad.ids.object, quad.ids.face]
                .into_iter()
                .chain(quad.ids.vertices)
                .chain(quad.ids.edges)
            {
                if document.contains_any_id(id) || !ids.insert(id) {
                    return Err(ModelError::InvalidPrimitive);
                }
            }
            let extent = quad.half_extent_mm;
            let point = |x: i64, y: i64| -> Result<Millimetres3, ModelError> {
                quad.origin_mm.checked_add(Millimetres3 { x, y, z: 0 })
            };
            let vertices = vec![
                Vertex {
                    id: quad.ids.vertices[0],
                    position_mm: point(-extent, -extent)?,
                },
                Vertex {
                    id: quad.ids.vertices[1],
                    position_mm: point(extent, -extent)?,
                },
                Vertex {
                    id: quad.ids.vertices[2],
                    position_mm: point(extent, extent)?,
                },
                Vertex {
                    id: quad.ids.vertices[3],
                    position_mm: point(-extent, extent)?,
                },
            ];
            let edges = vec![
                Edge {
                    id: quad.ids.edges[0],
                    start: vertices[0].id,
                    end: vertices[1].id,
                },
                Edge {
                    id: quad.ids.edges[1],
                    start: vertices[1].id,
                    end: vertices[2].id,
                },
                Edge {
                    id: quad.ids.edges[2],
                    start: vertices[2].id,
                    end: vertices[3].id,
                },
                Edge {
                    id: quad.ids.edges[3],
                    start: vertices[3].id,
                    end: vertices[0].id,
                },
            ];
            Ok(MeshObject {
                id: quad.ids.object,
                label: quad.label.clone(),
                transform: ModelTransform::default(),
                vertices,
                edges,
                faces: vec![Face {
                    id: quad.ids.face,
                    vertices: quad.ids.vertices.to_vec(),
                }],
            })
        }
    }
}

fn split_edge(document: &mut ModelDocument, split: &SplitEdge) -> Result<TopologyMap, ModelError> {
    if document.contains_any_id(split.new_vertex.id)
        || split.replacement_edges[0].id == split.replacement_edges[1].id
        || document.contains_any_id(split.replacement_edges[0].id)
        || document.contains_any_id(split.replacement_edges[1].id)
    {
        return Err(ModelError::InvalidSplitEdge(split.edge_id));
    }
    let object = document
        .objects
        .iter_mut()
        .find(|object| object.id == split.object_id)
        .ok_or(ModelError::UnknownObject(split.object_id))?;
    let edge_index = object
        .edges
        .iter()
        .position(|edge| edge.id == split.edge_id)
        .ok_or(ModelError::UnknownElement(split.edge_id))?;
    let prior = object.edges[edge_index].clone();
    let replacement_other = [
        split.replacement_edges[0].other(split.new_vertex.id),
        split.replacement_edges[1].other(split.new_vertex.id),
    ];
    if replacement_other.iter().any(Option::is_none)
        || replacement_other[0] == replacement_other[1]
        || ![prior.start, prior.end].contains(&replacement_other[0].unwrap_or(prior.start))
        || ![prior.start, prior.end].contains(&replacement_other[1].unwrap_or(prior.end))
    {
        return Err(ModelError::InvalidSplitEdge(split.edge_id));
    }
    object.vertices.push(split.new_vertex.clone());
    object.edges.remove(edge_index);
    object
        .edges
        .insert(edge_index, split.replacement_edges[1].clone());
    object
        .edges
        .insert(edge_index, split.replacement_edges[0].clone());

    let mut affected_faces = BTreeSet::new();
    for face in &mut object.faces {
        if let Some(index) = boundary_index(&face.vertices, prior.start, prior.end) {
            face.vertices.insert(index + 1, split.new_vertex.id);
            affected_faces.insert(face.id);
        }
    }
    let mut resulting_elements = BTreeSet::from([
        split.new_vertex.id,
        split.replacement_edges[0].id,
        split.replacement_edges[1].id,
    ]);
    resulting_elements.extend(affected_faces);
    Ok(TopologyMap {
        prior_elements: BTreeSet::from([prior.id]),
        resulting_elements,
        split_merge_lineage: vec![TopologyLineage {
            prior: prior.id,
            resulting: vec![split.replacement_edges[0].id, split.replacement_edges[1].id],
        }],
        orphaned_elements: BTreeSet::new(),
    })
}

fn boundary_index(vertices: &[StableId], start: StableId, end: StableId) -> Option<usize> {
    vertices.iter().enumerate().find_map(|(index, vertex)| {
        let next = vertices[(index + 1) % vertices.len()];
        ((*vertex == start && next == end) || (*vertex == end && next == start)).then_some(index)
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use meridian_alluvium::{GeneratedOverride, OverrideAction};

    use super::*;

    fn id(value: u128) -> StableId {
        StableId::new(value)
    }

    fn source() -> ModelDocument {
        let mut session = ModelSession::open(ModelDocument::new(id(1), "Public model"))
            .expect("empty document opens");
        session
            .apply(ModelTransaction::new(
                "Create public quad",
                ModelCommand::CreatePrimitive(PrimitiveCreate::Quad(QuadPrimitive {
                    ids: QuadPrimitiveIds {
                        object: id(2),
                        vertices: [id(3), id(4), id(5), id(6)],
                        edges: [id(7), id(8), id(9), id(10)],
                        face: id(11),
                    },
                    label: "Public quad".to_owned(),
                    origin_mm: Millimetres3::default(),
                    half_extent_mm: 500,
                })),
            ))
            .expect("primitive creates");
        session.current().document().clone()
    }

    fn split() -> SplitEdge {
        SplitEdge {
            object_id: id(2),
            edge_id: id(7),
            new_vertex: Vertex {
                id: id(12),
                position_mm: Millimetres3 {
                    x: 0,
                    y: -500,
                    z: 0,
                },
            },
            replacement_edges: [
                Edge {
                    id: id(13),
                    start: id(3),
                    end: id(12),
                },
                Edge {
                    id: id(14),
                    start: id(12),
                    end: id(4),
                },
            ],
        }
    }

    #[test]
    fn source_round_trips_to_canonical_json() {
        let source = source();
        let parsed = ModelDocument::from_json(&source.canonical_json().expect("canonical"))
            .expect("source parses");
        assert_eq!(parsed, source);
        assert_eq!(parsed.schema, MODEL_SCHEMA);
        assert_eq!(parsed.version, MODEL_VERSION);
    }

    #[test]
    fn source_writes_atomically_to_a_regular_project_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("meridian-model-source-{unique}"));
        fs::create_dir(&root).expect("temporary project directory");
        let path = root.join("public.model.json");
        let source = source();
        source.write_source(&path).expect("source writes");
        assert_eq!(
            ModelDocument::read_source(&path).expect("source reads"),
            source
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_rejects_unknown_fields_instead_of_losing_them_on_canonical_write() {
        let source = source();
        let mut value: serde_json::Value =
            serde_json::from_str(&source.canonical_json().expect("canonical"))
                .expect("canonical JSON parses");
        value
            .as_object_mut()
            .expect("source object")
            .insert("unexpected".to_owned(), serde_json::Value::Bool(true));
        assert!(matches!(
            ModelDocument::from_json(&value.to_string()),
            Err(ModelError::Json(_))
        ));
    }

    #[test]
    fn primitive_transform_and_stale_selection_are_typed() {
        let mut session = ModelSession::open(source()).expect("source opens");
        session
            .select(ModelElementKind::Object, [id(2)])
            .expect("object selects");
        let selection = session.selection().clone();
        session
            .apply(
                ModelTransaction::new(
                    "Translate quad",
                    ModelCommand::TranslateObject {
                        object_id: id(2),
                        translation_mm: Millimetres3 {
                            x: 10,
                            y: 20,
                            z: 30,
                        },
                    },
                )
                .with_selection(selection.clone()),
            )
            .expect("transform applies");
        assert_eq!(
            session
                .current()
                .document()
                .object(id(2))
                .expect("object")
                .transform
                .translation_mm,
            Millimetres3 {
                x: 10,
                y: 20,
                z: 30
            }
        );
        assert_eq!(
            session
                .current()
                .document()
                .penumbra_preview(id(2))
                .expect("translated preview")
                .positions_mm[0],
            Millimetres3 {
                x: -490,
                y: -480,
                z: 30
            }
        );
        assert!(matches!(
            session.apply(
                ModelTransaction::new(
                    "Stale transform",
                    ModelCommand::TranslateObject {
                        object_id: id(2),
                        translation_mm: Millimetres3::default(),
                    },
                )
                .with_selection(selection)
            ),
            Err(ModelError::StaleSelection { .. })
        ));
    }

    #[test]
    fn split_edge_preserves_lineage_and_updates_face_boundary() {
        let mut session = ModelSession::open(source()).expect("source opens");
        session
            .select(ModelElementKind::Edge, [id(7)])
            .expect("edge selects");
        let receipt = session
            .apply(
                ModelTransaction::new("Split edge", ModelCommand::SplitEdge(split()))
                    .with_selection(session.selection().clone()),
            )
            .expect("split applies");
        let topology = receipt.topology_map.expect("topology map");
        assert_eq!(topology.lineage_for(id(7)), Some(&[id(13), id(14)][..]));
        let object = session.current().document().object(id(2)).expect("object");
        assert!(!object.edges.iter().any(|edge| edge.id == id(7)));
        assert_eq!(
            object.faces[0].vertices,
            vec![id(3), id(12), id(4), id(5), id(6)]
        );
        session
            .current()
            .document()
            .validate()
            .expect("topology stays valid");
    }

    #[test]
    fn semantic_undo_redo_and_recovery_restore_accepted_revision() {
        let mut session = ModelSession::open(source()).expect("source opens");
        session
            .select(ModelElementKind::Edge, [id(7)])
            .expect("edge selects");
        session
            .apply(
                ModelTransaction::new("Split edge", ModelCommand::SplitEdge(split()))
                    .with_selection(session.selection().clone()),
            )
            .expect("split applies");
        let accepted = session.current().clone();
        session.undo().expect("semantic undo");
        assert!(session.current().document().element_kind(id(7)).is_some());
        session.redo().expect("semantic redo");
        assert_eq!(session.current(), &accepted);

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("meridian-modeler-{nonce}.save"));
        let store = ModelRecoveryStore::new(&path);
        store.save(&session).expect("recovery saves");
        let recovered = store.load().expect("recovery loads");
        assert_eq!(recovered.current(), session.current());
        fs::remove_file(&path).expect("primary removes");
        let backup = path.with_extension("save.bak");
        if backup.exists() {
            fs::remove_file(backup).expect("backup removes");
        }
    }

    #[test]
    fn recovery_rejects_history_that_cannot_replay_the_accepted_revision() {
        let mut session = ModelSession::open(source()).expect("source opens");
        session
            .select(ModelElementKind::Edge, [id(7)])
            .expect("edge selects");
        session
            .apply(
                ModelTransaction::new("Split edge", ModelCommand::SplitEdge(split()))
                    .with_selection(session.selection().clone()),
            )
            .expect("split applies");
        let mut snapshot = ModelRecoverySnapshot {
            schema: RECOVERY_SCHEMA.to_owned(),
            session,
        };
        snapshot.session.undo[0].after.document.document_generation = 42;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("meridian-modeler-history-{nonce}.save"));
        let store = ModelRecoveryStore::new(&path);
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot serializes");
        store.store.save(bytes).expect("snapshot saves");
        assert!(matches!(
            store.load(),
            Err(ModelError::RecoveryHistoryMismatch)
        ));
        fs::remove_file(&path).expect("primary removes");
        let backup = path.with_extension("save.bak");
        if backup.exists() {
            fs::remove_file(backup).expect("backup removes");
        }
    }

    #[test]
    fn preview_is_derived_and_cannot_mutate_source() {
        let source = source();
        let before = source.clone();
        let preview = source.penumbra_preview(id(2)).expect("preview builds");
        assert!(preview.is_derived());
        assert_eq!(preview.positions_mm.len(), 4);
        assert_eq!(preview.triangle_indices.len(), 6);
        assert_eq!(source, before);
    }

    #[test]
    fn split_override_conflict_is_retained_for_alluvium_resolution() {
        let mut session = ModelSession::open(source()).expect("source opens");
        session
            .select(ModelElementKind::Edge, [id(7)])
            .expect("edge selects");
        let topology = session
            .apply(
                ModelTransaction::new("Split edge", ModelCommand::SplitEdge(split()))
                    .with_selection(session.selection().clone()),
            )
            .expect("split applies")
            .topology_map
            .expect("topology map");
        let generated_override = GeneratedOverride {
            target: id(7),
            expected_source: None,
            action: OverrideAction::Suppress,
        };
        let reconciliation = topology
            .reconcile_generated_override(&generated_override, session.current().document());
        assert_eq!(reconciliation.status, OverrideStatus::Conflicted);
    }

    #[test]
    fn invalid_face_boundary_is_rejected() {
        let mut source = source();
        source.objects[0].edges.pop();
        assert!(matches!(
            source.validate(),
            Err(ModelError::FaceBoundaryMissingEdge { .. })
        ));
    }
}
