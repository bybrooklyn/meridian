//! Engine-owned frame sequencing for fixed simulation and render extraction.

use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use meridian_core::{FixedStepBatch, FixedStepClock, FixedStepConfig, FrameId};
use meridian_diagnostics::{FrameAssessment, FrameBudget, FrameHistory, FrameSample, FrameSummary};
use meridian_ecs::EngineWorld;
use meridian_renderer::{RenderSnapshot, SnapshotError};

/// Runtime configuration that stays independent of game content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    pub fixed_step: FixedStepConfig,
    pub diagnostics_capacity: NonZeroUsize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            fixed_step: FixedStepConfig::default(),
            diagnostics_capacity: NonZeroUsize::new(240).expect("240 is non-zero"),
        }
    }
}

/// Report produced after one variable-rate runtime frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeFrameReport {
    frame_id: FrameId,
    first_fixed_tick: u64,
    fixed_steps: u32,
    fixed_tick_after: u64,
    interpolation_alpha: f32,
    dropped_time: Duration,
    render_extraction_error: Option<SnapshotError>,
    diagnostic_assessment: FrameAssessment,
}

impl RuntimeFrameReport {
    #[must_use]
    pub const fn frame_id(self) -> u64 {
        self.frame_id.get()
    }

    #[must_use]
    pub const fn shared_frame_id(self) -> FrameId {
        self.frame_id
    }

    #[must_use]
    pub const fn first_fixed_tick(self) -> u64 {
        self.first_fixed_tick
    }

    #[must_use]
    pub const fn fixed_steps(self) -> u32 {
        self.fixed_steps
    }

    #[must_use]
    pub const fn fixed_tick_after(self) -> u64 {
        self.fixed_tick_after
    }

    #[must_use]
    pub const fn interpolation_alpha(self) -> f32 {
        self.interpolation_alpha
    }

    #[must_use]
    pub const fn dropped_time(self) -> Duration {
        self.dropped_time
    }

    #[must_use]
    pub const fn render_extraction_error(self) -> Option<SnapshotError> {
        self.render_extraction_error
    }

    #[must_use]
    pub const fn diagnostic_assessment(self) -> FrameAssessment {
        self.diagnostic_assessment
    }
}

/// Coordinates the engine's fixed-step simulation and variable-rate render phase.
pub struct EngineRuntime {
    config: RuntimeConfig,
    clock: FixedStepClock,
    world: EngineWorld,
    next_frame_id: FrameId,
    diagnostics: FrameHistory,
}

impl EngineRuntime {
    #[must_use]
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            clock: FixedStepClock::new(config.fixed_step),
            config,
            world: EngineWorld::new(),
            next_frame_id: FrameId::new(0),
            diagnostics: FrameHistory::new(
                config.diagnostics_capacity,
                FrameBudget::for_rate(config.fixed_step.rate),
            ),
        }
    }

    #[must_use]
    pub const fn config(&self) -> RuntimeConfig {
        self.config
    }

    #[must_use]
    pub const fn next_frame_id(&self) -> u64 {
        self.next_frame_id.get()
    }

    #[must_use]
    pub const fn world(&self) -> &EngineWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut EngineWorld {
        &mut self.world
    }

    #[must_use]
    pub fn render_snapshot(&self) -> Option<&RenderSnapshot> {
        self.world.render_snapshot()
    }

    #[must_use]
    pub fn diagnostics_summary(&self) -> Option<FrameSummary> {
        self.diagnostics.summary()
    }

    /// Attaches the RHI duration to the frame most recently advanced.
    pub fn set_last_gpu_time(&mut self, gpu_time: Option<Duration>) -> bool {
        self.diagnostics.set_latest_gpu_time(gpu_time)
    }

    /// Advances one frame, runs all selected fixed steps, then extracts once.
    pub fn advance(&mut self, frame_delta: Duration) -> RuntimeFrameReport {
        let frame_start = Instant::now();
        let FixedStepBatch {
            first_tick,
            steps,
            interpolation_alpha,
            dropped_time,
        } = self.clock.advance(frame_delta);
        self.world.run_fixed_steps(steps);

        let frame_id = self.next_frame_id;
        self.next_frame_id = self.next_frame_id.next();
        let interpolation_alpha = f64_to_f32(interpolation_alpha);
        self.world
            .run_render_extract_for_frame(frame_id.get(), interpolation_alpha);

        let diagnostic_assessment = self
            .diagnostics
            .push(FrameSample::new(frame_delta, frame_start.elapsed()));

        RuntimeFrameReport {
            frame_id,
            first_fixed_tick: first_tick,
            fixed_steps: steps,
            fixed_tick_after: self.world.fixed_tick(),
            interpolation_alpha,
            dropped_time,
            render_extraction_error: self.world.render_extraction_error(),
            diagnostic_assessment,
        }
    }
}

impl Default for EngineRuntime {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_diagnostics::StutterClass;
    use meridian_ecs::{ResMut, Resource};
    use meridian_renderer::{
        MaterialHandle, MeshHandle, RenderInstanceId, RenderInstanceSource, Transform,
    };

    #[derive(Resource, Default)]
    struct FixedRuns(u32);

    fn count_fixed_steps(mut runs: ResMut<FixedRuns>) {
        runs.0 = runs.0.saturating_add(1);
    }

    #[test]
    fn advance_sequences_fixed_steps_before_one_render_extraction() {
        let mut runtime = EngineRuntime::default();
        runtime.world_mut().insert_resource(FixedRuns::default());
        runtime.world_mut().add_fixed_systems(count_fixed_steps);
        let _entity = runtime.world_mut().spawn(RenderInstanceSource::new(
            RenderInstanceId::new(7),
            Transform::from_translation([1.0, 0.0, 0.0]),
            1.0,
            MeshHandle(1),
            MaterialHandle(1),
        ));

        let report = runtime.advance(Duration::from_millis(34));

        assert_eq!(report.frame_id(), 0);
        assert_eq!(report.first_fixed_tick(), 0);
        assert_eq!(report.fixed_steps(), 2);
        assert_eq!(report.fixed_tick_after(), 2);
        assert_eq!(
            runtime
                .world()
                .get_resource::<FixedRuns>()
                .expect("runs exist")
                .0,
            2
        );
        let snapshot = runtime.render_snapshot().expect("snapshot exists");
        assert_eq!(snapshot.frame_id(), 0);
        assert_eq!(snapshot.fixed_tick(), 2);
        assert_eq!(snapshot.instances().len(), 1);
        assert_eq!(report.render_extraction_error(), None);
    }

    #[test]
    fn runtime_reports_bounded_catch_up_and_keeps_frame_ids_monotonic() {
        let mut runtime = EngineRuntime::default();
        let first = runtime.advance(Duration::from_millis(200));
        let second = runtime.advance(Duration::ZERO);

        assert_eq!(first.fixed_steps(), 4);
        assert!(first.dropped_time() > Duration::ZERO);
        assert_eq!(second.frame_id(), 1);
        assert_eq!(second.fixed_steps(), 0);
        assert_eq!(runtime.next_frame_id(), 2);
    }

    #[test]
    fn zero_delta_still_publishes_a_render_snapshot_for_the_frame() {
        let mut runtime = EngineRuntime::default();
        let report = runtime.advance(Duration::ZERO);

        assert_eq!(report.fixed_steps(), 0);
        assert!(report.interpolation_alpha().abs() < f32::EPSILON);
        assert!(runtime.render_snapshot().is_some());
    }

    #[test]
    fn runtime_records_frame_diagnostics_and_accepts_late_gpu_duration() {
        let mut runtime = EngineRuntime::default();
        let report = runtime.advance(Duration::from_millis(16));

        assert_eq!(
            report.diagnostic_assessment().stutter,
            StutterClass::OnBudget
        );
        let summary = runtime.diagnostics_summary().expect("frame is recorded");
        assert_eq!(summary.sample_count, 1);
        assert_eq!(summary.average_frame_time, Duration::from_millis(16));
        assert_eq!(summary.average_gpu_time, None);

        assert!(runtime.set_last_gpu_time(Some(Duration::from_millis(3))));
        let summary = runtime.diagnostics_summary().expect("frame is recorded");
        assert_eq!(summary.average_gpu_time, Some(Duration::from_millis(3)));
    }
}
