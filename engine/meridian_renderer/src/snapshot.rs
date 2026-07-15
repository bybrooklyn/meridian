//! Immutable render-extraction data shared across the simulation/render boundary.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    #[must_use]
    pub const fn from_translation(translation: [f32; 3]) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Interpolates fixed-step transforms for a variable-rate render frame.
    #[must_use]
    pub fn lerp(previous: Self, current: Self, alpha: f32) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        Self {
            translation: lerp_array(previous.translation, current.translation, alpha),
            rotation: lerp_array(previous.rotation, current.rotation, alpha),
            scale: lerp_array(previous.scale, current.scale, alpha),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

fn lerp_array<const N: usize>(previous: [f32; N], current: [f32; N], alpha: f32) -> [f32; N] {
    std::array::from_fn(|index| previous[index] + (current[index] - previous[index]) * alpha)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RenderInstanceId(u64);

impl RenderInstanceId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderFlags(u32);

impl RenderFlags {
    pub const VISIBLE: Self = Self(1 << 0);
    pub const CASTS_SHADOW: Self = Self(1 << 1);
    pub const STATIC: Self = Self(1 << 2);

    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for RenderFlags {
    fn default() -> Self {
        Self::VISIBLE.union(Self::CASTS_SHADOW)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MeshHandle(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaterialHandle(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TextureHandle(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct RenderInstance {
    pub id: RenderInstanceId,
    pub previous_transform: Transform,
    pub transform: Transform,
    pub bounds_radius: f32,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub flags: RenderFlags,
}

impl RenderInstance {
    #[must_use]
    pub fn new(
        id: RenderInstanceId,
        transform: Transform,
        bounds_radius: f32,
        mesh: MeshHandle,
        material: MaterialHandle,
    ) -> Self {
        Self {
            id,
            previous_transform: transform,
            transform,
            bounds_radius: bounds_radius.max(0.0),
            mesh,
            material,
            flags: RenderFlags::default(),
        }
    }

    #[must_use]
    pub fn interpolated_transform(&self, alpha: f32) -> Transform {
        Transform::lerp(self.previous_transform, self.transform, alpha)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSnapshot {
    frame_id: u64,
    fixed_tick: u64,
    interpolation_alpha: f32,
    instances: Vec<RenderInstance>,
}

impl RenderSnapshot {
    #[must_use]
    pub const fn frame_id(&self) -> u64 {
        self.frame_id
    }

    #[must_use]
    pub const fn fixed_tick(&self) -> u64 {
        self.fixed_tick
    }

    #[must_use]
    pub const fn interpolation_alpha(&self) -> f32 {
        self.interpolation_alpha
    }

    #[must_use]
    pub fn instances(&self) -> &[RenderInstance] {
        &self.instances
    }

    #[must_use]
    pub fn instance(&self, id: RenderInstanceId) -> Option<&RenderInstance> {
        self.instances.iter().find(|instance| instance.id == id)
    }
}

pub struct RenderSnapshotBuilder {
    frame_id: u64,
    fixed_tick: u64,
    interpolation_alpha: f32,
    instances: Vec<RenderInstance>,
    ids: BTreeSet<RenderInstanceId>,
}

impl RenderSnapshotBuilder {
    #[must_use]
    pub fn new(frame_id: u64, fixed_tick: u64, interpolation_alpha: f32) -> Self {
        Self {
            frame_id,
            fixed_tick,
            interpolation_alpha: interpolation_alpha.clamp(0.0, 1.0),
            instances: Vec::new(),
            ids: BTreeSet::new(),
        }
    }

    /// Adds one extracted render instance.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::DuplicateInstanceId`] when the same instance
    /// is extracted twice in one frame.
    pub fn push(&mut self, instance: RenderInstance) -> Result<(), SnapshotError> {
        if !self.ids.insert(instance.id) {
            return Err(SnapshotError::DuplicateInstanceId(instance.id));
        }
        self.instances.push(instance);
        Ok(())
    }

    #[must_use]
    pub fn build(mut self) -> RenderSnapshot {
        self.instances.sort_unstable_by_key(|instance| instance.id);
        RenderSnapshot {
            frame_id: self.frame_id,
            fixed_tick: self.fixed_tick,
            interpolation_alpha: self.interpolation_alpha,
            instances: self.instances,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    DuplicateInstanceId(RenderInstanceId),
}

impl Display for SnapshotError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateInstanceId(id) => {
                write!(formatter, "render instance {} extracted twice", id.value())
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(id: u64, x: f32) -> RenderInstance {
        RenderInstance::new(
            RenderInstanceId::new(id),
            Transform::from_translation([x, 0.0, 0.0]),
            1.0,
            MeshHandle(1),
            MaterialHandle(1),
        )
    }

    #[test]
    fn snapshots_are_sorted_and_carry_frame_metadata() {
        let mut builder = RenderSnapshotBuilder::new(7, 11, 1.5);
        builder.push(instance(9, 9.0)).expect("unique ID");
        builder.push(instance(2, 2.0)).expect("unique ID");
        let snapshot = builder.build();

        assert_eq!(snapshot.frame_id(), 7);
        assert_eq!(snapshot.fixed_tick(), 11);
        assert!((snapshot.interpolation_alpha() - 1.0).abs() < f32::EPSILON);
        assert_eq!(
            snapshot
                .instances()
                .iter()
                .map(|instance| instance.id.value())
                .collect::<Vec<_>>(),
            [2, 9]
        );
    }

    #[test]
    fn duplicate_instances_are_rejected() {
        let mut builder = RenderSnapshotBuilder::new(0, 0, 0.0);
        let duplicate = instance(4, 0.0);
        builder.push(duplicate.clone()).expect("first ID is unique");
        assert_eq!(
            builder.push(duplicate),
            Err(SnapshotError::DuplicateInstanceId(RenderInstanceId::new(4)))
        );
    }

    #[test]
    fn interpolation_is_clamped_and_snapshot_is_immutable() {
        let mut render_instance = instance(1, 10.0);
        render_instance.previous_transform = Transform::from_translation([0.0, 0.0, 0.0]);
        let snapshot = {
            let mut builder = RenderSnapshotBuilder::new(1, 1, 0.5);
            builder.push(render_instance).expect("unique ID");
            builder.build()
        };

        let transform = snapshot.instances()[0].interpolated_transform(0.5);
        assert!((transform.translation[0] - 5.0).abs() < f32::EPSILON);
        assert!((snapshot.instances()[0].transform.translation[0] - 10.0).abs() < f32::EPSILON);
    }
}
