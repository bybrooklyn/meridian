//! World-space coordinates and renderer/physics-independent spatial records.

use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_CELL_SIZE_METERS: f64 = 128.0;

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
    cell_index: BTreeMap<WorldCell, BTreeSet<SpatialId>>,
    next_id: u64,
    origin: WorldPosition,
}

impl SpatialDatabase {
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
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
        self.records.insert(id, record);
        id
    }

    pub fn remove(&mut self, id: SpatialId) -> Option<SpatialRecord> {
        let record = self.records.remove(&id)?;
        self.remove_from_cell(record.cell, id);
        Some(record)
    }

    #[must_use]
    pub fn get(&self, id: SpatialId) -> Option<&SpatialRecord> {
        self.records.get(&id)
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

impl Default for SpatialDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
