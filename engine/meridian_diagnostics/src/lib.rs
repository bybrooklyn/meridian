//! Bounded runtime frame diagnostics and stutter classification.

use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter, Write};
use std::num::NonZeroUsize;
use std::time::Duration;

use meridian_core::FrameRate;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameBudget {
    pub target: Duration,
    pub noticeable_hitch: Duration,
    pub severe_hitch: Duration,
    pub unacceptable_stall: Duration,
}

impl FrameBudget {
    /// Builds the performance-spec thresholds for the requested frame rate.
    #[must_use]
    pub fn for_rate(rate: FrameRate) -> Self {
        let target = rate.period();
        let unacceptable_multiplier = if rate.get() >= 100 { 4.0 } else { 3.0 };
        Self {
            target,
            noticeable_hitch: target.mul_f64(1.2),
            severe_hitch: target.mul_f64(2.0),
            unacceptable_stall: target.mul_f64(unacceptable_multiplier),
        }
    }

    #[must_use]
    pub fn classify(self, frame_time: Duration) -> StutterClass {
        if frame_time <= self.target {
            StutterClass::OnBudget
        } else if frame_time < self.noticeable_hitch {
            StutterClass::MinorMiss
        } else if frame_time < self.severe_hitch {
            StutterClass::NoticeableHitch
        } else if frame_time <= self.unacceptable_stall {
            StutterClass::SevereHitch
        } else {
            StutterClass::UnacceptableStall
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StutterClass {
    OnBudget,
    MinorMiss,
    NoticeableHitch,
    SevereHitch,
    UnacceptableStall,
}

/// Startup/runtime pipeline state captured alongside frame diagnostics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PipelineDiagnostics {
    pub total_pipelines: u32,
    pub required_pipelines: u32,
    pub warmed_pipelines: u32,
    pub startup_creation_events: u64,
    pub runtime_creation_attempts: u64,
    pub runtime_ready: bool,
}

impl PipelineDiagnostics {
    #[must_use]
    pub const fn new(
        total_pipelines: u32,
        required_pipelines: u32,
        warmed_pipelines: u32,
        startup_creation_events: u64,
        runtime_creation_attempts: u64,
        runtime_ready: bool,
    ) -> Self {
        Self {
            total_pipelines,
            required_pipelines,
            warmed_pipelines,
            startup_creation_events,
            runtime_creation_attempts,
            runtime_ready,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameSample {
    pub frame_time: Duration,
    pub main_thread_time: Duration,
    pub gpu_time: Option<Duration>,
    pub pipeline_creation_events: u32,
    pub upload_stall: bool,
    pub pipeline_diagnostics: Option<PipelineDiagnostics>,
}

impl FrameSample {
    #[must_use]
    pub const fn new(frame_time: Duration, main_thread_time: Duration) -> Self {
        Self {
            frame_time,
            main_thread_time,
            gpu_time: None,
            pipeline_creation_events: 0,
            upload_stall: false,
            pipeline_diagnostics: None,
        }
    }

    #[must_use]
    pub const fn with_gpu_time(mut self, gpu_time: Option<Duration>) -> Self {
        self.gpu_time = gpu_time;
        self
    }

    #[must_use]
    pub const fn with_pipeline_diagnostics(
        mut self,
        pipeline_diagnostics: PipelineDiagnostics,
    ) -> Self {
        self.pipeline_diagnostics = Some(pipeline_diagnostics);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameAssessment {
    pub stutter: StutterClass,
    pub consecutive_missed_budgets: usize,
}

#[derive(Clone, Debug)]
pub struct FrameHistory {
    capacity: NonZeroUsize,
    budget: FrameBudget,
    samples: VecDeque<FrameSample>,
    consecutive_missed_budgets: usize,
    latest_pipeline_diagnostics: Option<PipelineDiagnostics>,
}

impl FrameHistory {
    #[must_use]
    pub fn new(capacity: NonZeroUsize, budget: FrameBudget) -> Self {
        Self {
            capacity,
            budget,
            samples: VecDeque::with_capacity(capacity.get()),
            consecutive_missed_budgets: 0,
            latest_pipeline_diagnostics: None,
        }
    }

    pub fn push(&mut self, sample: FrameSample) -> FrameAssessment {
        let stutter = self.budget.classify(sample.frame_time);
        if stutter == StutterClass::OnBudget {
            self.consecutive_missed_budgets = 0;
        } else {
            self.consecutive_missed_budgets = self.consecutive_missed_budgets.saturating_add(1);
        }

        if self.samples.len() == self.capacity.get() {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
        if sample.pipeline_diagnostics.is_some() {
            self.latest_pipeline_diagnostics = sample.pipeline_diagnostics;
        }

        FrameAssessment {
            stutter,
            consecutive_missed_budgets: self.consecutive_missed_budgets,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Replaces the optional GPU duration on the most recently recorded frame.
    ///
    /// This lets a runtime record CPU/frame timing before presentation and
    /// attach the RHI result after the submitted frame has completed.
    pub fn set_latest_gpu_time(&mut self, gpu_time: Option<Duration>) -> bool {
        let Some(sample) = self.samples.back_mut() else {
            return false;
        };
        sample.gpu_time = gpu_time;
        true
    }

    #[must_use]
    pub fn summary(&self) -> Option<FrameSummary> {
        if self.samples.is_empty() {
            return None;
        }

        let mut frame_times = self
            .samples
            .iter()
            .map(|sample| sample.frame_time)
            .collect::<Vec<_>>();
        frame_times.sort_unstable();

        let average_frame_time = average_duration(&frame_times);
        let median_frame_time = percentile(&frame_times, 50, 100);
        let p99_frame_time = percentile(&frame_times, 99, 100);
        let &worst_frame_time = frame_times.last()?;
        let mut gpu_times = self
            .samples
            .iter()
            .filter_map(|sample| sample.gpu_time)
            .collect::<Vec<_>>();
        gpu_times.sort_unstable();
        let gpu_time_samples = gpu_times.len();
        let average_gpu_time = (!gpu_times.is_empty()).then(|| average_duration(&gpu_times));
        let worst_gpu_time = gpu_times.last().copied();
        let missed_budget_frames = frame_times
            .iter()
            .filter(|frame_time| **frame_time > self.budget.target)
            .count();
        let pipeline_creation_events = self
            .samples
            .iter()
            .map(|sample| u64::from(sample.pipeline_creation_events))
            .sum();
        let upload_stall_frames = self
            .samples
            .iter()
            .filter(|sample| sample.upload_stall)
            .count();

        Some(FrameSummary {
            sample_count: frame_times.len(),
            average_frame_time,
            median_frame_time,
            p99_frame_time,
            worst_frame_time,
            one_percent_low_fps: low_fps(&frame_times, 1, 100),
            zero_point_one_percent_low_fps: low_fps(&frame_times, 1, 1_000),
            missed_budget_frames,
            pipeline_creation_events,
            upload_stall_frames,
            gpu_time_samples,
            average_gpu_time,
            worst_gpu_time,
            pipeline_diagnostics: self.latest_pipeline_diagnostics,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSummary {
    pub sample_count: usize,
    pub average_frame_time: Duration,
    pub median_frame_time: Duration,
    pub p99_frame_time: Duration,
    pub worst_frame_time: Duration,
    pub one_percent_low_fps: f64,
    pub zero_point_one_percent_low_fps: f64,
    pub missed_budget_frames: usize,
    pub pipeline_creation_events: u64,
    pub upload_stall_frames: usize,
    pub gpu_time_samples: usize,
    pub average_gpu_time: Option<Duration>,
    pub worst_gpu_time: Option<Duration>,
    pub pipeline_diagnostics: Option<PipelineDiagnostics>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkMetadata {
    pub scene: String,
    pub build_hash: String,
    pub asset_hash: String,
    pub backend: String,
    pub preset: String,
    pub camera_path: Option<String>,
    pub time_of_day: Option<String>,
    pub weather: Option<String>,
}

impl BenchmarkMetadata {
    #[must_use]
    pub fn new(
        scene: impl Into<String>,
        build_hash: impl Into<String>,
        asset_hash: impl Into<String>,
        backend: impl Into<String>,
        preset: impl Into<String>,
    ) -> Self {
        Self {
            scene: scene.into(),
            build_hash: build_hash.into(),
            asset_hash: asset_hash.into(),
            backend: backend.into(),
            preset: preset.into(),
            camera_path: None,
            time_of_day: None,
            weather: None,
        }
    }

    #[must_use]
    pub fn with_context(
        mut self,
        camera_path: impl Into<String>,
        time_of_day: impl Into<String>,
        weather: impl Into<String>,
    ) -> Self {
        self.camera_path = Some(camera_path.into());
        self.time_of_day = Some(time_of_day.into());
        self.weather = Some(weather.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchmarkMetrics {
    pub cpu_frame_ms: f64,
    pub gpu_frame_ms: f64,
    pub gpu_time_samples: usize,
    pub average_gpu_frame_ms: f64,
    pub worst_gpu_frame_ms: f64,
    pub median_fps: f64,
    pub one_percent_low_fps: f64,
    pub zero_point_one_percent_low_fps: f64,
    pub worst_frame_ms: f64,
    pub pipeline_creation_events: u64,
    pub pipeline_count: u32,
    pub required_pipeline_count: u32,
    pub warmed_pipeline_count: u32,
    pub startup_pipeline_creation_events: u64,
    pub runtime_pipeline_creation_attempts: u64,
    pub pipeline_runtime_ready: bool,
}

impl BenchmarkMetrics {
    #[must_use]
    pub fn from_summary(
        summary: FrameSummary,
        cpu_frame_time: Duration,
        gpu_frame_time: Duration,
    ) -> Self {
        let average_gpu_time = summary.average_gpu_time.unwrap_or(gpu_frame_time);
        Self {
            cpu_frame_ms: milliseconds(cpu_frame_time),
            gpu_frame_ms: milliseconds(average_gpu_time),
            gpu_time_samples: summary.gpu_time_samples,
            average_gpu_frame_ms: milliseconds(average_gpu_time),
            worst_gpu_frame_ms: summary.worst_gpu_time.map_or(0.0, milliseconds),
            median_fps: frames_per_second(summary.median_frame_time),
            one_percent_low_fps: summary.one_percent_low_fps,
            zero_point_one_percent_low_fps: summary.zero_point_one_percent_low_fps,
            worst_frame_ms: milliseconds(summary.worst_frame_time),
            pipeline_creation_events: summary.pipeline_creation_events,
            pipeline_count: summary
                .pipeline_diagnostics
                .map_or(0, |diagnostics| diagnostics.total_pipelines),
            required_pipeline_count: summary
                .pipeline_diagnostics
                .map_or(0, |diagnostics| diagnostics.required_pipelines),
            warmed_pipeline_count: summary
                .pipeline_diagnostics
                .map_or(0, |diagnostics| diagnostics.warmed_pipelines),
            startup_pipeline_creation_events: summary
                .pipeline_diagnostics
                .map_or(0, |diagnostics| diagnostics.startup_creation_events),
            runtime_pipeline_creation_attempts: summary
                .pipeline_diagnostics
                .map_or(0, |diagnostics| diagnostics.runtime_creation_attempts),
            pipeline_runtime_ready: summary
                .pipeline_diagnostics
                .is_some_and(|diagnostics| diagnostics.runtime_ready),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkResult {
    pub metadata: BenchmarkMetadata,
    pub metrics: BenchmarkMetrics,
}

impl BenchmarkResult {
    #[must_use]
    pub const fn new(metadata: BenchmarkMetadata, metrics: BenchmarkMetrics) -> Self {
        Self { metadata, metrics }
    }

    /// Serializes a deterministic record matching the benchmark result schema.
    ///
    /// # Errors
    ///
    /// Returns [`BenchmarkJsonError`] when a metric is not finite and therefore
    /// cannot be represented as valid JSON.
    pub fn to_json(&self) -> Result<String, BenchmarkJsonError> {
        validate_benchmark_metrics(self.metrics)?;

        let mut output = String::from("{");
        let mut first = true;
        string_field(&mut output, &mut first, "scene", &self.metadata.scene);
        string_field(
            &mut output,
            &mut first,
            "build_hash",
            &self.metadata.build_hash,
        );
        string_field(
            &mut output,
            &mut first,
            "asset_hash",
            &self.metadata.asset_hash,
        );
        string_field(&mut output, &mut first, "backend", &self.metadata.backend);
        string_field(&mut output, &mut first, "preset", &self.metadata.preset);
        optional_string_field(
            &mut output,
            &mut first,
            "camera_path",
            self.metadata.camera_path.as_deref(),
        );
        optional_string_field(
            &mut output,
            &mut first,
            "time_of_day",
            self.metadata.time_of_day.as_deref(),
        );
        optional_string_field(
            &mut output,
            &mut first,
            "weather",
            self.metadata.weather.as_deref(),
        );

        field_start(&mut output, &mut first, "metrics");
        output.push('{');
        let mut first_metric = true;
        number_field(
            &mut output,
            &mut first_metric,
            "cpu_frame_ms",
            self.metrics.cpu_frame_ms,
        );
        append_gpu_metrics(&mut output, &mut first_metric, self.metrics);
        number_field(
            &mut output,
            &mut first_metric,
            "median_fps",
            self.metrics.median_fps,
        );
        number_field(
            &mut output,
            &mut first_metric,
            "one_percent_low_fps",
            self.metrics.one_percent_low_fps,
        );
        number_field(
            &mut output,
            &mut first_metric,
            "zero_point_one_percent_low_fps",
            self.metrics.zero_point_one_percent_low_fps,
        );
        number_field(
            &mut output,
            &mut first_metric,
            "worst_frame_ms",
            self.metrics.worst_frame_ms,
        );
        integer_field(
            &mut output,
            &mut first_metric,
            "pipeline_creation_events",
            self.metrics.pipeline_creation_events,
        );
        append_pipeline_metrics(&mut output, &mut first_metric, self.metrics);
        output.push('}');
        output.push('}');
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkJsonError {
    metric: &'static str,
}

impl BenchmarkJsonError {
    #[must_use]
    pub const fn metric(self) -> &'static str {
        self.metric
    }
}

impl Display for BenchmarkJsonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "benchmark metric is not finite: {}", self.metric)
    }
}

impl std::error::Error for BenchmarkJsonError {}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn frames_per_second(duration: Duration) -> f64 {
    1.0 / duration.as_secs_f64()
}

fn validate_metric(metric: &'static str, value: f64) -> Result<(), BenchmarkJsonError> {
    value
        .is_finite()
        .then_some(())
        .ok_or(BenchmarkJsonError { metric })
}

fn validate_benchmark_metrics(metrics: BenchmarkMetrics) -> Result<(), BenchmarkJsonError> {
    for (name, value) in [
        ("cpu_frame_ms", metrics.cpu_frame_ms),
        ("gpu_frame_ms", metrics.gpu_frame_ms),
        ("average_gpu_frame_ms", metrics.average_gpu_frame_ms),
        ("worst_gpu_frame_ms", metrics.worst_gpu_frame_ms),
        ("median_fps", metrics.median_fps),
        ("one_percent_low_fps", metrics.one_percent_low_fps),
        (
            "zero_point_one_percent_low_fps",
            metrics.zero_point_one_percent_low_fps,
        ),
        ("worst_frame_ms", metrics.worst_frame_ms),
    ] {
        validate_metric(name, value)?;
    }
    Ok(())
}

fn field_start(output: &mut String, first: &mut bool, key: &str) {
    if !*first {
        output.push(',');
    }
    *first = false;
    push_json_string(output, key);
    output.push(':');
}

fn string_field(output: &mut String, first: &mut bool, key: &str, value: &str) {
    field_start(output, first, key);
    push_json_string(output, value);
}

fn optional_string_field(output: &mut String, first: &mut bool, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        string_field(output, first, key, value);
    }
}

fn number_field(output: &mut String, first: &mut bool, key: &str, value: f64) {
    field_start(output, first, key);
    let _ = write!(output, "{value:.6}");
}

fn integer_field(output: &mut String, first: &mut bool, key: &str, value: u64) {
    field_start(output, first, key);
    let _ = write!(output, "{value}");
}

fn boolean_field(output: &mut String, first: &mut bool, key: &str, value: bool) {
    field_start(output, first, key);
    output.push_str(if value { "true" } else { "false" });
}

fn append_pipeline_metrics(output: &mut String, first: &mut bool, metrics: BenchmarkMetrics) {
    for (key, value) in [
        ("pipeline_count", u64::from(metrics.pipeline_count)),
        (
            "required_pipeline_count",
            u64::from(metrics.required_pipeline_count),
        ),
        (
            "warmed_pipeline_count",
            u64::from(metrics.warmed_pipeline_count),
        ),
        (
            "startup_pipeline_creation_events",
            metrics.startup_pipeline_creation_events,
        ),
        (
            "runtime_pipeline_creation_attempts",
            metrics.runtime_pipeline_creation_attempts,
        ),
    ] {
        integer_field(output, first, key, value);
    }
    boolean_field(
        output,
        first,
        "pipeline_runtime_ready",
        metrics.pipeline_runtime_ready,
    );
}

fn append_gpu_metrics(output: &mut String, first: &mut bool, metrics: BenchmarkMetrics) {
    number_field(output, first, "gpu_frame_ms", metrics.gpu_frame_ms);
    integer_field(
        output,
        first,
        "gpu_time_samples",
        u64::try_from(metrics.gpu_time_samples).unwrap_or(u64::MAX),
    );
    number_field(
        output,
        first,
        "average_gpu_frame_ms",
        metrics.average_gpu_frame_ms,
    );
    number_field(
        output,
        first,
        "worst_gpu_frame_ms",
        metrics.worst_gpu_frame_ms,
    );
}

fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn average_duration(samples: &[Duration]) -> Duration {
    let total_nanos = samples
        .iter()
        .map(Duration::as_nanos)
        .fold(0_u128, u128::saturating_add);
    duration_from_nanos(total_nanos / samples.len() as u128)
}

fn percentile(sorted_samples: &[Duration], numerator: usize, denominator: usize) -> Duration {
    let rank = sorted_samples
        .len()
        .saturating_mul(numerator)
        .div_ceil(denominator);
    sorted_samples[rank.saturating_sub(1).min(sorted_samples.len() - 1)]
}

fn low_fps(sorted_frame_times: &[Duration], numerator: usize, denominator: usize) -> f64 {
    let sample_count = sorted_frame_times
        .len()
        .saturating_mul(numerator)
        .div_ceil(denominator)
        .max(1);
    let worst_start = sorted_frame_times.len() - sample_count;
    let average_worst = average_duration(&sorted_frame_times[worst_start..]);
    let seconds = average_worst.as_secs_f64();
    if seconds == 0.0 {
        f64::INFINITY
    } else {
        1.0 / seconds
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

    fn sixty_fps_budget() -> FrameBudget {
        FrameBudget::for_rate(FrameRate::new(60).expect("60 is non-zero"))
    }

    #[test]
    fn classifies_sixty_fps_stutter_thresholds() {
        let budget = sixty_fps_budget();

        assert_eq!(
            budget.classify(Duration::from_millis(16)),
            StutterClass::OnBudget
        );
        assert_eq!(
            budget.classify(Duration::from_millis(18)),
            StutterClass::MinorMiss
        );
        assert_eq!(
            budget.classify(Duration::from_millis(25)),
            StutterClass::NoticeableHitch
        );
        assert_eq!(
            budget.classify(Duration::from_millis(40)),
            StutterClass::SevereHitch
        );
        assert_eq!(
            budget.classify(Duration::from_millis(51)),
            StutterClass::UnacceptableStall
        );
    }

    #[test]
    fn scales_stutter_thresholds_for_one_twenty_fps() {
        let budget = FrameBudget::for_rate(FrameRate::new(120).expect("120 is valid"));

        assert_eq!(
            budget.classify(Duration::from_millis(8)),
            StutterClass::OnBudget
        );
        assert_eq!(
            budget.classify(Duration::from_millis(9)),
            StutterClass::MinorMiss
        );
        assert_eq!(
            budget.classify(Duration::from_millis(12)),
            StutterClass::NoticeableHitch
        );
        assert_eq!(
            budget.classify(Duration::from_millis(20)),
            StutterClass::SevereHitch
        );
        assert_eq!(
            budget.classify(Duration::from_millis(34)),
            StutterClass::UnacceptableStall
        );
    }

    #[test]
    fn tracks_consecutive_misses_and_resets_on_budget() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(4).expect("4 is non-zero"),
            sixty_fps_budget(),
        );

        let first = history.push(FrameSample::new(
            Duration::from_millis(20),
            Duration::from_millis(4),
        ));
        let second = history.push(FrameSample::new(
            Duration::from_millis(22),
            Duration::from_millis(4),
        ));
        let recovered = history.push(FrameSample::new(
            Duration::from_millis(16),
            Duration::from_millis(3),
        ));

        assert_eq!(first.consecutive_missed_budgets, 1);
        assert_eq!(second.consecutive_missed_budgets, 2);
        assert_eq!(recovered.consecutive_missed_budgets, 0);
    }

    #[test]
    fn frame_samples_can_carry_optional_gpu_duration() {
        let gpu_time = Duration::from_micros(900);
        let sample = FrameSample::new(Duration::from_millis(16), Duration::from_millis(4))
            .with_gpu_time(Some(gpu_time));

        assert_eq!(sample.gpu_time, Some(gpu_time));
        assert_eq!(
            FrameSample::new(Duration::ZERO, Duration::ZERO).gpu_time,
            None
        );
    }

    #[test]
    fn latest_gpu_duration_can_be_attached_after_frame_recording() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(2).expect("2 is non-zero"),
            sixty_fps_budget(),
        );

        assert!(!history.set_latest_gpu_time(Some(Duration::from_millis(1))));
        history.push(FrameSample::new(
            Duration::from_millis(16),
            Duration::from_millis(4),
        ));
        assert!(history.set_latest_gpu_time(Some(Duration::from_millis(3))));
        assert_eq!(
            history
                .summary()
                .expect("history has one sample")
                .average_gpu_time,
            Some(Duration::from_millis(3))
        );
    }

    #[test]
    fn history_is_bounded_and_summarizes_retained_samples() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(3).expect("3 is non-zero"),
            sixty_fps_budget(),
        );
        for milliseconds in [10, 16, 20, 40] {
            let mut sample = FrameSample::new(
                Duration::from_millis(milliseconds),
                Duration::from_millis(4),
            );
            sample.pipeline_creation_events = u32::from(milliseconds == 40);
            sample.upload_stall = milliseconds == 20;
            history.push(sample);
        }

        let summary = history.summary().expect("history has samples");

        assert_eq!(history.len(), 3);
        assert_eq!(summary.sample_count, 3);
        assert_eq!(summary.median_frame_time, Duration::from_millis(20));
        assert_eq!(summary.worst_frame_time, Duration::from_millis(40));
        assert_eq!(summary.missed_budget_frames, 2);
        assert_eq!(summary.pipeline_creation_events, 1);
        assert_eq!(summary.upload_stall_frames, 1);
        assert!((summary.one_percent_low_fps - 25.0).abs() < 0.000_001);
    }

    #[test]
    fn history_aggregates_retained_gpu_timings_for_benchmark_metrics() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(3).expect("3 is non-zero"),
            sixty_fps_budget(),
        );
        for (frame_millis, gpu_micros) in [(10, 1_000), (16, 3_000), (20, 2_000)] {
            history.push(
                FrameSample::new(
                    Duration::from_millis(frame_millis),
                    Duration::from_millis(4),
                )
                .with_gpu_time(Some(Duration::from_micros(gpu_micros))),
            );
        }

        let summary = history.summary().expect("history has samples");

        assert_eq!(summary.gpu_time_samples, 3);
        assert_eq!(summary.average_gpu_time, Some(Duration::from_millis(2)));
        assert_eq!(summary.worst_gpu_time, Some(Duration::from_millis(3)));

        let metrics = BenchmarkMetrics::from_summary(
            summary,
            Duration::from_millis(5),
            Duration::from_millis(9),
        );
        assert_eq!(metrics.gpu_time_samples, 3);
        assert!((metrics.gpu_frame_ms - 2.0).abs() < f64::EPSILON);
        assert!((metrics.average_gpu_frame_ms - 2.0).abs() < f64::EPSILON);
        assert!((metrics.worst_gpu_frame_ms - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn history_retains_pipeline_state_for_benchmark_export() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(2).expect("2 is non-zero"),
            sixty_fps_budget(),
        );
        let diagnostics = PipelineDiagnostics::new(4, 3, 4, 4, 0, true);
        history.push(
            FrameSample::new(Duration::from_millis(16), Duration::from_millis(4))
                .with_pipeline_diagnostics(diagnostics),
        );

        let summary = history.summary().expect("history has samples");

        assert_eq!(summary.pipeline_diagnostics, Some(diagnostics));
        let metrics = BenchmarkMetrics::from_summary(
            summary,
            Duration::from_millis(4),
            Duration::from_millis(3),
        );
        assert_eq!(metrics.pipeline_count, 4);
        assert_eq!(metrics.required_pipeline_count, 3);
        assert_eq!(metrics.warmed_pipeline_count, 4);
        assert_eq!(metrics.startup_pipeline_creation_events, 4);
        assert_eq!(metrics.runtime_pipeline_creation_attempts, 0);
        assert!(metrics.pipeline_runtime_ready);
    }

    #[test]
    fn benchmark_result_serializes_required_fields_and_escapes_metadata() {
        let mut history = FrameHistory::new(
            NonZeroUsize::new(3).expect("3 is non-zero"),
            sixty_fps_budget(),
        );
        for milliseconds in [10, 16, 20] {
            history.push(FrameSample::new(
                Duration::from_millis(milliseconds),
                Duration::from_millis(4),
            ));
        }
        let summary = history.summary().expect("history has samples");
        let metrics = BenchmarkMetrics::from_summary(
            summary,
            Duration::from_millis(5),
            Duration::from_millis(3),
        );
        let metadata =
            BenchmarkMetadata::new("B01 \"forest\"", "build-123", "assets-456", "Metal", "Low")
                .with_context("opening-route", "midnight", "clear");

        let json = BenchmarkResult::new(metadata, metrics)
            .to_json()
            .expect("finite metrics serialize");

        assert!(json.contains("\"scene\":\"B01 \\\"forest\\\"\""));
        assert!(json.contains("\"camera_path\":\"opening-route\""));
        assert!(json.contains("\"metrics\":{\"cpu_frame_ms\":5.000000"));
        assert!(json.contains("\"pipeline_creation_events\":0"));
        assert!(json.ends_with("}}"));
    }

    #[test]
    fn benchmark_result_rejects_non_finite_metrics() {
        let metadata = BenchmarkMetadata::new("B01", "build", "assets", "Metal", "Low");
        let metrics = BenchmarkMetrics {
            cpu_frame_ms: 5.0,
            gpu_frame_ms: 3.0,
            gpu_time_samples: 0,
            average_gpu_frame_ms: 3.0,
            worst_gpu_frame_ms: 0.0,
            median_fps: f64::NAN,
            one_percent_low_fps: 55.0,
            zero_point_one_percent_low_fps: 45.0,
            worst_frame_ms: 20.0,
            pipeline_creation_events: 0,
            pipeline_count: 0,
            required_pipeline_count: 0,
            warmed_pipeline_count: 0,
            startup_pipeline_creation_events: 0,
            runtime_pipeline_creation_attempts: 0,
            pipeline_runtime_ready: false,
        };

        let error = BenchmarkResult::new(metadata, metrics)
            .to_json()
            .expect_err("NaN is not valid JSON");

        assert_eq!(error.metric(), "median_fps");
    }
}
