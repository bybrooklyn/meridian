//! Renderer-independent engine identity and timing contracts.

use std::num::NonZeroU32;
use std::time::Duration;

pub const ENGINE_NAME: &str = "Meridian";
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A positive frame or simulation rate measured in hertz.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameRate(NonZeroU32);

impl FrameRate {
    /// Creates a rate when `hz` is between 1 Hz and 1 GHz.
    #[must_use]
    pub const fn new(hz: u32) -> Option<Self> {
        if hz > 1_000_000_000 {
            return None;
        }
        match NonZeroU32::new(hz) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the rate in hertz.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    /// Returns the duration of one frame or fixed simulation step.
    #[must_use]
    pub fn period(self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.get()))
    }
}

/// Configuration for a fixed-step simulation clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedStepConfig {
    pub rate: FrameRate,
    pub max_steps_per_frame: NonZeroU32,
}

impl FixedStepConfig {
    #[must_use]
    pub const fn new(rate: FrameRate, max_steps_per_frame: NonZeroU32) -> Self {
        Self {
            rate,
            max_steps_per_frame,
        }
    }
}

impl Default for FixedStepConfig {
    fn default() -> Self {
        Self {
            rate: FrameRate::new(60).expect("60 is non-zero"),
            max_steps_per_frame: NonZeroU32::new(4).expect("4 is non-zero"),
        }
    }
}

/// Work selected for one rendered frame by [`FixedStepClock::advance`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStepBatch {
    /// Tick number assigned to the first step in this batch.
    pub first_tick: u64,
    /// Number of fixed simulation steps to execute.
    pub steps: u32,
    /// Fractional progress from the last completed tick to the next tick.
    pub interpolation_alpha: f64,
    /// Accumulated whole-step time discarded to prevent a catch-up spiral.
    pub dropped_time: Duration,
}

/// Deterministic fixed-step accumulator with bounded catch-up.
#[derive(Clone, Debug)]
pub struct FixedStepClock {
    config: FixedStepConfig,
    step: Duration,
    /// Elapsed nanoseconds multiplied by the fixed rate. One step is 1e9 units.
    accumulator_scaled: u128,
    next_tick: u64,
}

impl FixedStepClock {
    #[must_use]
    pub fn new(config: FixedStepConfig) -> Self {
        Self {
            step: config.rate.period(),
            config,
            accumulator_scaled: 0,
            next_tick: 0,
        }
    }

    #[must_use]
    pub const fn config(&self) -> FixedStepConfig {
        self.config
    }

    #[must_use]
    pub const fn step_duration(&self) -> Duration {
        self.step
    }

    #[must_use]
    pub const fn next_tick(&self) -> u64 {
        self.next_tick
    }

    /// Adds elapsed wall-clock time and chooses a bounded number of steps.
    ///
    /// Whole steps above `max_steps_per_frame` are deliberately discarded,
    /// while the fractional remainder is preserved for interpolation.
    pub fn advance(&mut self, frame_delta: Duration) -> FixedStepBatch {
        let rate = u128::from(self.config.rate.get());
        let scaled_delta = frame_delta.as_nanos().saturating_mul(rate);
        self.accumulator_scaled = self.accumulator_scaled.saturating_add(scaled_delta);

        let available_steps = self.accumulator_scaled / 1_000_000_000;
        let max_steps = u128::from(self.config.max_steps_per_frame.get());
        let executed_steps = available_steps.min(max_steps);
        let dropped_steps = available_steps.saturating_sub(executed_steps);
        self.accumulator_scaled %= 1_000_000_000;

        let first_tick = self.next_tick;
        let steps = u32::try_from(executed_steps).unwrap_or(u32::MAX);
        self.next_tick = self.next_tick.saturating_add(u64::from(steps));
        let remainder_for_alpha = u32::try_from(self.accumulator_scaled).unwrap_or(u32::MAX);
        let dropped_nanos = dropped_steps
            .saturating_mul(1_000_000_000)
            .checked_div(rate)
            .unwrap_or_default();

        FixedStepBatch {
            first_tick,
            steps,
            interpolation_alpha: f64::from(remainder_for_alpha) / 1_000_000_000.0,
            dropped_time: duration_from_nanos(dropped_nanos),
        }
    }

    /// Clears accumulated time and starts tick numbering from `next_tick`.
    pub fn reset(&mut self, next_tick: u64) {
        self.accumulator_scaled = 0;
        self.next_tick = next_tick;
    }
}

impl Default for FixedStepClock {
    fn default() -> Self {
        Self::new(FixedStepConfig::default())
    }
}

fn duration_from_nanos(nanos: u128) -> Duration {
    let seconds = nanos / 1_000_000_000;
    let subsecond_nanos = nanos % 1_000_000_000;
    Duration::new(
        u64::try_from(seconds).unwrap_or(u64::MAX),
        u32::try_from(subsecond_nanos).expect("subsecond nanoseconds fit in u32"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rate_rejects_zero() {
        assert_eq!(FrameRate::new(0), None);
        assert_eq!(FrameRate::new(60).map(FrameRate::get), Some(60));
    }

    #[test]
    fn fixed_clock_preserves_fraction_for_interpolation() {
        let mut clock = FixedStepClock::default();
        let half_step = clock.step_duration().div_f64(2.0);
        let remaining_half = clock
            .step_duration()
            .checked_sub(half_step)
            .expect("half a step is not greater than a full step");

        let first = clock.advance(half_step);
        assert_eq!(first.steps, 0);
        assert!((first.interpolation_alpha - 0.5).abs() < 0.000_001);

        let second = clock.advance(remaining_half);
        assert_eq!(second.steps, 1);
        assert_eq!(second.first_tick, 0);
        assert_eq!(clock.next_tick(), 1);
        assert!(second.interpolation_alpha < 0.000_001);
    }

    #[test]
    fn fixed_clock_caps_catch_up_and_reports_dropped_time() {
        let mut clock = FixedStepClock::default();
        let ten_steps = clock.step_duration().saturating_mul(10);

        let batch = clock.advance(ten_steps);

        assert_eq!(batch.steps, 4);
        assert_eq!(batch.dropped_time, Duration::from_millis(100));
        assert_eq!(clock.next_tick(), 4);
        assert!(batch.interpolation_alpha < 0.000_001);
    }

    #[test]
    fn reset_clears_accumulation_and_sets_tick() {
        let mut clock = FixedStepClock::default();
        let _ = clock.advance(clock.step_duration().div_f64(2.0));

        clock.reset(42);
        let batch = clock.advance(Duration::ZERO);

        assert_eq!(batch.first_tick, 42);
        assert_eq!(batch.steps, 0);
        assert!(batch.interpolation_alpha.abs() < f64::EPSILON);
    }

    #[test]
    fn one_second_produces_exactly_sixty_steps_without_period_rounding_drift() {
        let config = FixedStepConfig::new(
            FrameRate::new(60).expect("60 is valid"),
            NonZeroU32::new(120).expect("120 is non-zero"),
        );
        let mut clock = FixedStepClock::new(config);

        let batch = clock.advance(Duration::from_secs(1));

        assert_eq!(batch.steps, 60);
        assert_eq!(batch.dropped_time, Duration::ZERO);
        assert!(batch.interpolation_alpha.abs() < f64::EPSILON);
    }
}
