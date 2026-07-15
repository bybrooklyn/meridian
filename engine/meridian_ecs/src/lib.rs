//! Meridian's narrow wrapper around standalone `bevy_ecs`.
//!
//! Game-facing code imports ECS traits from this crate. The underlying Bevy
//! world and schedules remain private implementation details of the wrapper.

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule};
use bevy_ecs::system::ScheduleSystem;
use meridian_renderer::{
    extract_render_instances, RenderExtractionFrame, RenderExtractionOutput, RenderSnapshot,
    SnapshotError,
};

pub use bevy_ecs::prelude::{
    Bundle, Commands, Component, Entity, Mut, Query, Res, ResMut, Resource, With, Without,
};
pub use meridian_renderer::RenderInstanceSource;

/// A system configuration accepted by an engine fixed-update schedule.
pub trait IntoFixedSystems<Marker>: IntoScheduleConfigs<ScheduleSystem, Marker> {}

impl<Marker, Systems> IntoFixedSystems<Marker> for Systems where
    Systems: IntoScheduleConfigs<ScheduleSystem, Marker>
{
}

/// A system configuration accepted by the immutable render-extraction schedule.
pub trait IntoRenderExtractSystems<Marker>: IntoScheduleConfigs<ScheduleSystem, Marker> {}

impl<Marker, Systems> IntoRenderExtractSystems<Marker> for Systems where
    Systems: IntoScheduleConfigs<ScheduleSystem, Marker>
{
}

/// ECS world with explicit fixed-update and render-extraction phases.
pub struct EngineWorld {
    world: bevy_ecs::world::World,
    fixed_update: Schedule,
    render_extract: Schedule,
    fixed_tick: u64,
    next_render_frame: u64,
}

impl EngineWorld {
    #[must_use]
    pub fn new() -> Self {
        let mut world = bevy_ecs::world::World::new();
        world.insert_resource(RenderExtractionFrame::default());
        world.insert_resource(RenderExtractionOutput::default());

        let mut render_extract = Schedule::default();
        render_extract.add_systems(extract_render_instances);

        Self {
            world,
            fixed_update: Schedule::default(),
            render_extract,
            fixed_tick: 0,
            next_render_frame: 0,
        }
    }

    /// Adds systems to the fixed 60 Hz simulation phase.
    pub fn add_fixed_systems<Marker, Systems>(&mut self, systems: Systems)
    where
        Systems: IntoFixedSystems<Marker>,
    {
        self.fixed_update.add_systems(systems);
    }

    /// Adds systems to the post-simulation render extraction phase.
    pub fn add_render_extract_systems<Marker, Systems>(&mut self, systems: Systems)
    where
        Systems: IntoRenderExtractSystems<Marker>,
    {
        self.render_extract.add_systems(systems);
    }

    /// Runs exactly `steps` fixed simulation updates and advances the tick ID.
    pub fn run_fixed_steps(&mut self, steps: u32) {
        for _ in 0..steps {
            self.fixed_update.run(&mut self.world);
            self.fixed_tick = self.fixed_tick.saturating_add(1);
        }
    }

    /// Runs render extraction once after simulation has advanced.
    pub fn run_render_extract(&mut self) {
        let frame_id = self.next_render_frame;
        self.next_render_frame = self.next_render_frame.saturating_add(1);
        self.run_render_extract_for_frame(frame_id, 0.0);
    }

    /// Runs render extraction with explicit variable-rate frame metadata.
    pub fn run_render_extract_for_frame(&mut self, frame_id: u64, interpolation_alpha: f32) {
        self.next_render_frame = self.next_render_frame.max(frame_id.saturating_add(1));
        self.world.insert_resource(RenderExtractionFrame::new(
            frame_id,
            self.fixed_tick,
            interpolation_alpha,
        ));
        self.render_extract.run(&mut self.world);
    }

    #[must_use]
    pub fn render_snapshot(&self) -> Option<&RenderSnapshot> {
        self.world
            .get_resource::<RenderExtractionOutput>()
            .and_then(RenderExtractionOutput::snapshot)
    }

    #[must_use]
    pub fn render_extraction_error(&self) -> Option<SnapshotError> {
        self.world
            .get_resource::<RenderExtractionOutput>()
            .and_then(RenderExtractionOutput::error)
    }

    #[must_use]
    pub const fn fixed_tick(&self) -> u64 {
        self.fixed_tick
    }

    #[must_use]
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        self.world.spawn(bundle).id()
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.world.despawn(entity)
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.world.insert_resource(resource);
    }

    #[must_use]
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource::<R>()
    }

    pub fn get_resource_mut<R>(&mut self) -> Option<Mut<'_, R>>
    where
        R: Resource<Mutability = bevy_ecs::component::Mutable>,
    {
        self.world.get_resource_mut::<R>()
    }
}

impl Default for EngineWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Position(u32);

    #[derive(Resource, Default)]
    struct FixedRuns(u32);

    #[derive(Resource, Default)]
    struct ExtractionRuns(u32);

    fn move_positions(mut query: Query<&mut Position>) {
        for mut position in &mut query {
            position.0 = position.0.saturating_add(1);
        }
    }

    fn count_fixed_runs(mut runs: ResMut<FixedRuns>) {
        runs.0 = runs.0.saturating_add(1);
    }

    fn count_extractions(mut runs: ResMut<ExtractionRuns>) {
        runs.0 = runs.0.saturating_add(1);
    }

    #[test]
    fn fixed_schedule_runs_exactly_requested_steps() {
        let mut world = EngineWorld::new();
        world.insert_resource(FixedRuns::default());
        world.add_fixed_systems((move_positions, count_fixed_runs).chain());
        let entity = world.spawn(Position(0));

        world.run_fixed_steps(3);

        assert_eq!(world.fixed_tick(), 3);
        assert_eq!(
            world
                .get_resource::<FixedRuns>()
                .expect("resource exists")
                .0,
            3
        );
        assert!(world.despawn(entity));
        assert!(!world.despawn(entity));
    }

    #[test]
    fn render_extraction_is_a_separate_explicit_phase() {
        let mut world = EngineWorld::new();
        world.insert_resource(ExtractionRuns::default());
        world.add_render_extract_systems(count_extractions);

        world.run_fixed_steps(2);
        assert_eq!(
            world
                .get_resource::<ExtractionRuns>()
                .expect("resource exists")
                .0,
            0
        );
        world.run_render_extract();

        assert_eq!(
            world
                .get_resource::<ExtractionRuns>()
                .expect("resource exists")
                .0,
            1
        );
    }

    #[test]
    fn built_in_extraction_copies_renderables_into_an_immutable_snapshot() {
        let mut world = EngineWorld::new();
        let _entity = world.spawn(RenderInstanceSource::new(
            meridian_renderer::RenderInstanceId::new(8),
            meridian_renderer::Transform::from_translation([3.0, 0.0, 0.0]),
            2.0,
            meridian_renderer::MeshHandle(4),
            meridian_renderer::MaterialHandle(5),
        ));

        world.run_fixed_steps(4);
        world.run_render_extract_for_frame(12, 0.25);

        let snapshot = world.render_snapshot().expect("snapshot exists");
        assert_eq!(snapshot.frame_id(), 12);
        assert_eq!(snapshot.fixed_tick(), 4);
        assert!((snapshot.interpolation_alpha() - 0.25).abs() < f32::EPSILON);
        assert_eq!(snapshot.instances().len(), 1);
        assert_eq!(
            snapshot.instances()[0].mesh,
            meridian_renderer::MeshHandle(4)
        );
        assert!((snapshot.instances()[0].transform.translation[0] - 3.0).abs() < f32::EPSILON);
        assert_eq!(world.render_extraction_error(), None);
    }

    #[test]
    fn duplicate_render_instance_ids_fail_the_whole_extraction() {
        let mut world = EngineWorld::new();
        for x in [1.0, 2.0] {
            let _entity = world.spawn(RenderInstanceSource::new(
                meridian_renderer::RenderInstanceId::new(3),
                meridian_renderer::Transform::from_translation([x, 0.0, 0.0]),
                1.0,
                meridian_renderer::MeshHandle(1),
                meridian_renderer::MaterialHandle(1),
            ));
        }

        world.run_render_extract();

        assert!(world.render_snapshot().is_none());
        assert_eq!(
            world.render_extraction_error(),
            Some(meridian_renderer::SnapshotError::DuplicateInstanceId(
                meridian_renderer::RenderInstanceId::new(3)
            ))
        );
    }
}
