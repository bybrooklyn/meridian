//! World-space coordinates and renderer/physics-independent spatial records.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_assets::{ArtifactHash, SourceId};
use meridian_core::StableId;

pub const DEFAULT_CELL_SIZE_METERS: f64 = 128.0;
const COMPILED_CELL_MAGIC: &[u8; 4] = b"MCEL";
const COMPILED_CELL_VERSION: u32 = 1;
const COMPILED_CELL_HEADER_SIZE: usize = 4 + 4 + 8 * 3 + 4;
const COMPILED_ENTITY_SIZE: usize = 16 + 16 + 8 * 3 + 4 * 3;
pub const MAX_COMPILED_CELL_ENTITIES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl WorldPosition {
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn distance_squared(self, other: Self) -> f64 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        x.mul_add(x, y.mul_add(y, z * z))
    }
}

impl Default for WorldPosition {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl LocalPosition {
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldCell {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl WorldCell {
    #[must_use]
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }
}

impl WorldCell {
    #[must_use]
    pub fn from_position(position: WorldPosition) -> Self {
        Self {
            x: cell_coordinate(position.x),
            y: cell_coordinate(position.y),
            z: cell_coordinate(position.z),
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn cell_coordinate(value: f64) -> i64 {
    (value / DEFAULT_CELL_SIZE_METERS).floor() as i64
}

#[allow(clippy::cast_possible_truncation)]
fn local_coordinate(value: f64) -> f32 {
    value as f32
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingSphere {
    pub center: WorldPosition,
    pub radius: f64,
}

impl BoundingSphere {
    #[must_use]
    pub fn new(center: WorldPosition, radius: f64) -> Self {
        Self {
            center,
            radius: radius.max(0.0),
        }
    }

    #[must_use]
    pub fn intersects_sphere(self, center: WorldPosition, radius: f64) -> bool {
        let combined_radius = self.radius + radius.max(0.0);
        self.center.distance_squared(center) <= combined_radius * combined_radius
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisibilityCategory {
    Visible,
    ShadowCaster,
    AudioOnly,
    Hidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Residency {
    Unloaded,
    Requested,
    Loading,
    Resident,
    Evicting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpatialKind {
    Static,
    Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SpatialId(u64);

impl SpatialId {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderHandle(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicsHandle(pub u64);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpatialHandles {
    pub render: Option<RenderHandle>,
    pub physics: Option<PhysicsHandle>,
    pub audio_zone: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialRecord {
    id: SpatialId,
    stable_id: Option<StableId>,
    cell: WorldCell,
    pub position: WorldPosition,
    pub bounds: BoundingSphere,
    pub visibility: VisibilityCategory,
    pub residency: Residency,
    pub kind: SpatialKind,
    pub handles: SpatialHandles,
}

impl SpatialRecord {
    #[must_use]
    pub fn new(position: WorldPosition, radius: f64, kind: SpatialKind) -> Self {
        Self {
            id: SpatialId(0),
            stable_id: None,
            cell: WorldCell::from_position(position),
            position,
            bounds: BoundingSphere::new(position, radius),
            visibility: VisibilityCategory::Visible,
            residency: Residency::Resident,
            kind,
            handles: SpatialHandles::default(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> SpatialId {
        self.id
    }

    #[must_use]
    pub const fn stable_id(&self) -> Option<StableId> {
        self.stable_id
    }

    #[must_use]
    pub const fn with_stable_id(mut self, stable_id: StableId) -> Self {
        self.stable_id = Some(stable_id);
        self
    }

    #[must_use]
    pub const fn cell(&self) -> WorldCell {
        self.cell
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OriginShift {
    pub from: WorldPosition,
    pub to: WorldPosition,
    /// Offset to apply to existing local coordinates when the origin changes.
    pub local_offset: WorldPosition,
}

pub struct SpatialDatabase {
    records: BTreeMap<SpatialId, SpatialRecord>,
    stable_entities: BTreeMap<StableId, SpatialId>,
    cell_index: BTreeMap<WorldCell, BTreeSet<SpatialId>>,
    next_id: u64,
    origin: WorldPosition,
}

impl SpatialDatabase {
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            stable_entities: BTreeMap::new(),
            cell_index: BTreeMap::new(),
            next_id: 1,
            origin: WorldPosition::default(),
        }
    }

    #[must_use]
    pub const fn origin(&self) -> WorldPosition {
        self.origin
    }

    pub fn insert(&mut self, mut record: SpatialRecord) -> SpatialId {
        let id = SpatialId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        record.id = id;
        record.cell = WorldCell::from_position(record.position);
        self.cell_index.entry(record.cell).or_default().insert(id);
        if let Some(stable_id) = record.stable_id {
            self.stable_entities.insert(stable_id, id);
        }
        self.records.insert(id, record);
        id
    }

    pub fn remove(&mut self, id: SpatialId) -> Option<SpatialRecord> {
        let record = self.records.remove(&id)?;
        if let Some(stable_id) = record.stable_id {
            self.stable_entities.remove(&stable_id);
        }
        self.remove_from_cell(record.cell, id);
        Some(record)
    }

    #[must_use]
    pub fn get(&self, id: SpatialId) -> Option<&SpatialRecord> {
        self.records.get(&id)
    }

    #[must_use]
    pub fn get_stable(&self, stable_id: StableId) -> Option<&SpatialRecord> {
        self.stable_entities
            .get(&stable_id)
            .and_then(|id| self.records.get(id))
    }

    /// Validates then activates an entire compiled cell without partial insertion.
    ///
    /// # Errors
    ///
    /// Rejects duplicate stable IDs, non-finite transforms, or entity-count overflow.
    pub fn activate_compiled_cell(
        &mut self,
        compiled: &CompiledWorldCell,
    ) -> Result<Vec<(StableId, SpatialId)>, CompiledCellError> {
        if compiled.entities.len() > MAX_COMPILED_CELL_ENTITIES {
            return Err(CompiledCellError::EntityCountExceeded {
                count: compiled.entities.len(),
                max: MAX_COMPILED_CELL_ENTITIES,
            });
        }
        let mut staged_ids = BTreeSet::new();
        for entity in &compiled.entities {
            if !staged_ids.insert(entity.stable_id)
                || self.stable_entities.contains_key(&entity.stable_id)
            {
                return Err(CompiledCellError::DuplicateStableId(entity.stable_id));
            }
            if !entity.position.x.is_finite()
                || !entity.position.y.is_finite()
                || !entity.position.z.is_finite()
                || entity.scale.iter().any(|value| !value.is_finite())
            {
                return Err(CompiledCellError::InvalidTransform(entity.stable_id));
            }
        }

        let mut activated = Vec::with_capacity(compiled.entities.len());
        for entity in &compiled.entities {
            let record = SpatialRecord::new(entity.position, 1.0, SpatialKind::Static)
                .with_stable_id(entity.stable_id);
            let id = self.insert(record);
            self.stable_entities.insert(entity.stable_id, id);
            activated.push((entity.stable_id, id));
        }
        Ok(activated)
    }

    pub fn update_position(&mut self, id: SpatialId, position: WorldPosition) -> bool {
        let Some(record) = self.records.get_mut(&id) else {
            return false;
        };
        let old_cell = record.cell;
        let new_cell = WorldCell::from_position(position);
        record.position = position;
        record.bounds.center = position;
        record.cell = new_cell;
        if old_cell != new_cell {
            self.remove_from_cell(old_cell, id);
            self.cell_index.entry(new_cell).or_default().insert(id);
        }
        true
    }

    #[must_use]
    pub fn ids_in_cell(&self, cell: WorldCell) -> Vec<SpatialId> {
        self.cell_index
            .get(&cell)
            .map_or_else(Vec::new, |ids| ids.iter().copied().collect())
    }

    #[must_use]
    pub fn query_radius(&self, center: WorldPosition, radius: f64) -> Vec<SpatialId> {
        self.records
            .values()
            .filter(|record| record.bounds.intersects_sphere(center, radius))
            .map(SpatialRecord::id)
            .collect()
    }

    #[must_use]
    pub fn local_position(&self, id: SpatialId) -> Option<LocalPosition> {
        let position = self.get(id)?.position;
        Some(LocalPosition::new(
            local_coordinate(position.x - self.origin.x),
            local_coordinate(position.y - self.origin.y),
            local_coordinate(position.z - self.origin.z),
        ))
    }

    /// Changes the local origin while keeping all stable world positions intact.
    #[must_use]
    pub fn rebase_origin(&mut self, origin: WorldPosition) -> OriginShift {
        let from = self.origin;
        self.origin = origin;
        OriginShift {
            from,
            to: origin,
            local_offset: WorldPosition::new(
                from.x - origin.x,
                from.y - origin.y,
                from.z - origin.z,
            ),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn remove_from_cell(&mut self, cell: WorldCell, id: SpatialId) {
        if let Some(ids) = self.cell_index.get_mut(&cell) {
            ids.remove(&id);
            if ids.is_empty() {
                self.cell_index.remove(&cell);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledEntity {
    pub stable_id: StableId,
    pub visual_source: SourceId,
    pub position: WorldPosition,
    pub scale: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledWorldCell {
    pub cell: WorldCell,
    pub entities: Vec<CompiledEntity>,
    pub artifact_hash: ArtifactHash,
}

impl CompiledWorldCell {
    #[must_use]
    pub fn new(cell: WorldCell, entities: Vec<CompiledEntity>) -> Self {
        let mut value = Self {
            cell,
            entities,
            artifact_hash: ArtifactHash::digest(&[]),
        };
        value.artifact_hash = ArtifactHash::digest(&value.encode_without_hash());
        value
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.encode_without_hash()
    }

    fn encode_without_hash(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            COMPILED_CELL_HEADER_SIZE
                .saturating_add(self.entities.len().saturating_mul(COMPILED_ENTITY_SIZE)),
        );
        bytes.extend_from_slice(COMPILED_CELL_MAGIC);
        bytes.extend_from_slice(&COMPILED_CELL_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.cell.x.to_le_bytes());
        bytes.extend_from_slice(&self.cell.y.to_le_bytes());
        bytes.extend_from_slice(&self.cell.z.to_le_bytes());
        bytes.extend_from_slice(
            &u32::try_from(self.entities.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        for entity in &self.entities {
            bytes.extend_from_slice(&entity.stable_id.get().to_le_bytes());
            bytes.extend_from_slice(&entity.visual_source.stable_id().get().to_le_bytes());
            for value in [entity.position.x, entity.position.y, entity.position.z] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            for value in entity.scale {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    /// Decodes a bounded version-one runtime cell.
    ///
    /// # Errors
    ///
    /// Rejects malformed magic/version/length/count, duplicates, and invalid transforms.
    pub fn decode(bytes: &[u8]) -> Result<Self, CompiledCellError> {
        if bytes.len() < COMPILED_CELL_HEADER_SIZE {
            return Err(CompiledCellError::Malformed("cell is shorter than header"));
        }
        if &bytes[..4] != COMPILED_CELL_MAGIC {
            return Err(CompiledCellError::Malformed("cell magic does not match"));
        }
        let version = read_u32(bytes, 4)?;
        if version != COMPILED_CELL_VERSION {
            return Err(CompiledCellError::UnsupportedVersion(version));
        }
        let cell = WorldCell::new(
            read_i64(bytes, 8)?,
            read_i64(bytes, 16)?,
            read_i64(bytes, 24)?,
        );
        let count = usize::try_from(read_u32(bytes, 32)?).unwrap_or(usize::MAX);
        if count > MAX_COMPILED_CELL_ENTITIES {
            return Err(CompiledCellError::EntityCountExceeded {
                count,
                max: MAX_COMPILED_CELL_ENTITIES,
            });
        }
        let expected = COMPILED_CELL_HEADER_SIZE
            .checked_add(
                count
                    .checked_mul(COMPILED_ENTITY_SIZE)
                    .ok_or(CompiledCellError::Malformed("entity byte count overflows"))?,
            )
            .ok_or(CompiledCellError::Malformed("cell byte count overflows"))?;
        if bytes.len() != expected {
            return Err(CompiledCellError::Malformed(
                "cell length does not match index",
            ));
        }
        let mut entities = Vec::with_capacity(count);
        let mut stable_ids = BTreeSet::new();
        for index in 0..count {
            let offset = COMPILED_CELL_HEADER_SIZE + index * COMPILED_ENTITY_SIZE;
            let stable_id = StableId::new(read_u128(bytes, offset)?);
            if !stable_ids.insert(stable_id) {
                return Err(CompiledCellError::DuplicateStableId(stable_id));
            }
            let visual_source = SourceId::new(StableId::new(read_u128(bytes, offset + 16)?));
            let position = WorldPosition::new(
                read_f64(bytes, offset + 32)?,
                read_f64(bytes, offset + 40)?,
                read_f64(bytes, offset + 48)?,
            );
            let scale = [
                read_f32(bytes, offset + 56)?,
                read_f32(bytes, offset + 60)?,
                read_f32(bytes, offset + 64)?,
            ];
            if !position.x.is_finite()
                || !position.y.is_finite()
                || !position.z.is_finite()
                || scale.iter().any(|value| !value.is_finite())
            {
                return Err(CompiledCellError::InvalidTransform(stable_id));
            }
            entities.push(CompiledEntity {
                stable_id,
                visual_source,
                position,
                scale,
            });
        }
        Ok(Self {
            cell,
            entities,
            artifact_hash: ArtifactHash::digest(bytes),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledCellError {
    Malformed(&'static str),
    UnsupportedVersion(u32),
    EntityCountExceeded { count: usize, max: usize },
    DuplicateStableId(StableId),
    InvalidTransform(StableId),
}

impl Display for CompiledCellError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(message) => write!(formatter, "malformed compiled cell: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported compiled cell version: {version}")
            }
            Self::EntityCountExceeded { count, max } => {
                write!(
                    formatter,
                    "compiled cell has {count} entities; maximum is {max}"
                )
            }
            Self::DuplicateStableId(id) => write!(formatter, "duplicate stable entity ID: {id}"),
            Self::InvalidTransform(id) => write!(formatter, "entity {id} has an invalid transform"),
        }
    }
}

impl Error for CompiledCellError {}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], CompiledCellError> {
    bytes
        .get(offset..offset.saturating_add(N))
        .and_then(|slice| slice.try_into().ok())
        .ok_or(CompiledCellError::Malformed("numeric field is truncated"))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, CompiledCellError> {
    Ok(u32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64, CompiledCellError> {
    Ok(i64::from_le_bytes(read_array(bytes, offset)?))
}

fn read_u128(bytes: &[u8], offset: usize) -> Result<u128, CompiledCellError> {
    Ok(u128::from_le_bytes(read_array(bytes, offset)?))
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, CompiledCellError> {
    Ok(f32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_f64(bytes: &[u8], offset: usize) -> Result<f64, CompiledCellError> {
    Ok(f64::from_le_bytes(read_array(bytes, offset)?))
}

impl Default for SpatialDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled_cell() -> CompiledWorldCell {
        CompiledWorldCell::new(
            WorldCell::new(0, 0, 0),
            vec![CompiledEntity {
                stable_id: StableId::new(7),
                visual_source: SourceId::from_canonical_name("fixtures/ms01/triangle"),
                position: WorldPosition::new(1.0, 2.0, 3.0),
                scale: [1.0; 3],
            }],
        )
    }

    #[test]
    fn compiled_cell_round_trip_and_activation_preserve_stable_identity() {
        let expected = compiled_cell();
        let decoded = CompiledWorldCell::decode(&expected.encode()).expect("cell decodes");
        let mut database = SpatialDatabase::new();
        let activated = database
            .activate_compiled_cell(&decoded)
            .expect("cell activates");

        assert_eq!(decoded, expected);
        assert_eq!(activated.len(), 1);
        assert_eq!(
            database
                .get_stable(StableId::new(7))
                .expect("stable entity exists")
                .position,
            WorldPosition::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn invalid_compiled_cell_never_partially_activates() {
        let mut cell = compiled_cell();
        cell.entities.push(cell.entities[0].clone());
        let mut database = SpatialDatabase::new();

        assert!(matches!(
            database.activate_compiled_cell(&cell),
            Err(CompiledCellError::DuplicateStableId(_))
        ));
        assert!(database.get_stable(StableId::new(7)).is_none());
    }

    #[test]
    fn cell_membership_handles_boundaries_and_negative_coordinates() {
        assert_eq!(
            WorldCell::from_position(WorldPosition::new(127.9, 0.0, -0.1)),
            WorldCell { x: 0, y: 0, z: -1 }
        );
        assert_eq!(
            WorldCell::from_position(WorldPosition::new(128.0, -128.0, 0.0)),
            WorldCell { x: 1, y: -1, z: 0 }
        );
    }

    #[test]
    fn records_are_indexed_and_reindexed_when_moved() {
        let mut database = SpatialDatabase::new();
        let id = database.insert(SpatialRecord::new(
            WorldPosition::new(10.0, 0.0, 0.0),
            2.0,
            SpatialKind::Dynamic,
        ));
        let first_cell = WorldCell::from_position(WorldPosition::new(10.0, 0.0, 0.0));
        let second_cell = WorldCell::from_position(WorldPosition::new(130.0, 0.0, 0.0));
        assert_eq!(database.ids_in_cell(first_cell), [id]);

        assert!(database.update_position(id, WorldPosition::new(130.0, 0.0, 0.0)));
        assert!(database.ids_in_cell(first_cell).is_empty());
        assert_eq!(database.ids_in_cell(second_cell), [id]);
    }

    #[test]
    fn radius_queries_include_bounding_sphere_extent() {
        let mut database = SpatialDatabase::new();
        let near = database.insert(SpatialRecord::new(
            WorldPosition::new(4.0, 0.0, 0.0),
            2.0,
            SpatialKind::Static,
        ));
        let far = database.insert(SpatialRecord::new(
            WorldPosition::new(20.0, 0.0, 0.0),
            1.0,
            SpatialKind::Static,
        ));

        let results = database.query_radius(WorldPosition::default(), 3.0);
        assert_eq!(results, [near]);
        assert!(!results.contains(&far));
    }

    #[test]
    fn rebasing_changes_local_coordinates_but_not_world_coordinates() {
        let mut database = SpatialDatabase::new();
        let id = database.insert(SpatialRecord::new(
            WorldPosition::new(300.25, 2.0, -4.0),
            1.0,
            SpatialKind::Dynamic,
        ));
        assert_eq!(
            database.local_position(id),
            Some(LocalPosition::new(300.25, 2.0, -4.0))
        );

        let shift = database.rebase_origin(WorldPosition::new(256.0, 0.0, 0.0));
        assert_eq!(shift.local_offset, WorldPosition::new(-256.0, 0.0, 0.0));
        assert_eq!(
            database.local_position(id),
            Some(LocalPosition::new(44.25, 2.0, -4.0))
        );
        assert!(
            (database.get(id).expect("record exists").position.x - 300.25).abs() < f64::EPSILON
        );
    }
}
