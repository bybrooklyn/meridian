//! Hidden native-window performance evidence runner for Meridian's direct UI path.
//!
//! This runner intentionally measures a small, fixed direct-display-list corpus
//! without presenting a window. It records wall-clock work, typed RHI timing
//! outcomes, and capture diagnostics. It does not set performance thresholds,
//! infer backend allocation/residency, or treat asynchronous capture wait as
//! GPU or interactive latency.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use meridian_benchmark::{
    append_evidence_json_line, write_capture_png, write_capture_rgba, write_evidence_json,
};
use meridian_core::FrameId;
use meridian_platform::{
    run, EventLoopMode, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent,
    PlatformWindow, WindowSize,
};
use meridian_renderer::{
    ui_direct_qualification_cases, UiDirectFrameFootprint, UiDirectFramePlan, UiDirectGpuFrame,
    UiDirectGpuRenderer, UiDirectQualificationCase, UiDirectRendererError,
};
use meridian_rhi::{
    CaptureDiagnostics, CaptureOutcome, CaptureRequest, CaptureSource, CapturedFrame,
    CapturedPixelFormat, ClearColor, GpuFeature, GpuTimingOutcome, PassTimingSample, Rhi,
    RhiConfig, RhiError, RhiErrorKind, TimingAvailability, TimingDiagnostics, TimingFrameId,
};
use serde::Serialize;

const PERFORMANCE_SCHEMA: &str = "meridian.ui-direct-performance-report/v1";
const PERFORMANCE_SAMPLE_SCHEMA: &str = "meridian.ui-direct-performance-sample/v1";
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_WARMUP: usize = 3;
const DEFAULT_SAMPLES: usize = 10;
// This bounds a local evidence invocation, not a renderer capacity or
// performance target. Each sample remains sequential and independently
// capture-bounded.
const MAX_ITERATIONS_PER_MODE: usize = 64;
const MAX_FAILURE_DETAIL_CHARS: usize = 240;
const QUALIFICATION_CLEAR: ClearColor = ClearColor::new(0.0, 0.0, 0.0, 1.0);
const SOURCE_PROVENANCE_LIMIT: &str = "MERIDIAN_SOURCE_STATE and MERIDIAN_SOURCE_CHECKPOINT are caller-declared labels; this runner does not verify checkout, executable, or artifact identity.";
const SOURCE_PROVENANCE_UNAVAILABLE: &str =
    "No declared source provenance was available before this preflight failure.";
const BUILD_IDENTITY_UNAVAILABLE: &str = "This runner does not embed a verified executable BuildId; Cargo package metadata is not executable identity.";
const BUILD_HASH_UNAVAILABLE: &str =
    "This runner does not embed or verify an executable/build hash.";
const TOOLCHAIN_PROFILE_UNAVAILABLE: &str =
    "Compiler version, target triple, and compiler flags are not embedded by this runner.";
const DEPENDENCY_PROFILE_UNAVAILABLE: &str = "Resolved Cargo dependency graph, Cargo.lock identity, and dependency feature graph are not embedded by this runner.";
const CACHE_RESET_UNAVAILABLE: &str = "The runner does not reset or observe shader, pipeline, operating-system, or driver caches between warmup and measurement.";
const SHADER_PIPELINE_CACHE_UNAVAILABLE: &str =
    "Shader and pipeline cache state is not exposed by the public direct-UI/RHI contract.";
const OPERATING_SYSTEM_DRIVER_CACHE_UNAVAILABLE: &str = "Operating-system and driver cache state is outside this runner's authority and is not observed.";
const PROFILE_METADATA_AVAILABLE: &str =
    "The active RHI capability profile supplied this metadata field.";
const DRIVER_UNAVAILABLE: &str =
    "The active RHI capability profile did not supply a driver identifier.";
const DRIVER_INFO_UNAVAILABLE: &str =
    "The active RHI capability profile did not supply driver-information text.";
const TRACKED_GPU_FEATURES: [GpuFeature; 7] = [
    GpuFeature::IndirectDrawCount,
    GpuFeature::MeshShaders,
    GpuFeature::SubgroupOperations,
    GpuFeature::TextureAtomics,
    GpuFeature::RayQueries,
    GpuFeature::RayTracingPipelines,
    GpuFeature::BindlessTextures,
];

#[derive(Clone, Debug)]
struct RunnerArgs {
    evidence_directory: PathBuf,
    warmup: usize,
    samples: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HarnessMode {
    ResourceSetup,
    SteadyReuse,
}

impl HarnessMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ResourceSetup => "resource_setup",
            Self::SteadyReuse => "steady_reuse",
        }
    }

    const fn definition(self) -> &'static str {
        match self {
            Self::ResourceSetup => {
                "Each iteration prepares and uploads a fresh immutable plan before submission."
            }
            Self::SteadyReuse => {
                "One immutable plan and its GPU frame are prepared once outside the sampled loop; sampled iterations reuse both."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SampleKind {
    Warmup,
    Measurement,
}

impl SampleKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Warmup => "warmup",
            Self::Measurement => "measurement",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WorkItem {
    sequence: u64,
    mode: HarnessMode,
    kind: SampleKind,
    ordinal: usize,
}

#[derive(Clone, Debug, Serialize)]
struct GpuProfileReport {
    backend: String,
    adapter_name: String,
    driver: String,
    driver_availability: ProfileMetadataAvailability,
    driver_info: String,
    driver_info_availability: ProfileMetadataAvailability,
    vendor_id: u32,
    device_id: u32,
    adapter_kind: String,
    memory_class: String,
    timestamp_query_capability: String,
    hdr_surface_formats_capability: String,
    max_sampled_textures_per_shader_stage: u32,
    enabled_features: Vec<String>,
    missing_features: Vec<String>,
    surface_format: String,
    operating_system: String,
    architecture: String,
}

#[derive(Clone, Debug, Serialize)]
struct RhiConfigurationReport {
    power_preference: String,
    preferred_backend: Option<String>,
    allow_software_adapter: bool,
    present_policy: String,
    desired_maximum_frame_latency: u32,
    timestamps_requested: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DurationReport {
    status: &'static str,
    code: &'static str,
    nanoseconds: Option<u64>,
}

impl DurationReport {
    fn measured(duration: Duration) -> Self {
        Self {
            status: "Pass",
            code: "Measured",
            nanoseconds: Some(duration_nanos(duration)),
        }
    }

    const fn not_applicable() -> Self {
        Self {
            status: "NotRun",
            code: "NotApplicable",
            nanoseconds: None,
        }
    }

    const fn unavailable() -> Self {
        Self {
            status: "Inconclusive",
            code: "NotAvailable",
            nanoseconds: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct PlanFootprintReport {
    cpu_vertex_bytes: u64,
    cpu_index_bytes: u64,
    cpu_atlas_bytes: u64,
    gpu_upload_payload_bytes: u64,
    planned_color_target_bytes: u64,
    primitive_count: usize,
    batch_count: usize,
    layer_count: usize,
    shadow_count: usize,
    backdrop_effect_count: usize,
    backdrop_fallback_count: usize,
    limitations: [&'static str; 2],
}

impl From<UiDirectFrameFootprint> for PlanFootprintReport {
    fn from(footprint: UiDirectFrameFootprint) -> Self {
        Self {
            cpu_vertex_bytes: footprint.cpu_vertex_bytes,
            cpu_index_bytes: footprint.cpu_index_bytes,
            cpu_atlas_bytes: footprint.cpu_atlas_bytes,
            gpu_upload_payload_bytes: footprint.gpu_upload_payload_bytes,
            planned_color_target_bytes: footprint.planned_color_target_bytes,
            primitive_count: footprint.primitive_count,
            batch_count: footprint.batch_count,
            layer_count: footprint.layer_count,
            shadow_count: footprint.shadow_count,
            backdrop_effect_count: footprint.backdrop_effect_count,
            backdrop_fallback_count: footprint.backdrop_fallback_count,
            limitations: [
                "Payload accounting describes prepared CPU bytes and requested upload payload only.",
                "It does not describe backend allocation size, cache residency, VRAM, or driver memory.",
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CaptureDiagnosticsReport {
    readback_capacity: usize,
    readbacks_in_flight: usize,
    pending_requests: usize,
    queued_results: usize,
    dropped_results: u64,
}

impl From<CaptureDiagnostics> for CaptureDiagnosticsReport {
    fn from(diagnostics: CaptureDiagnostics) -> Self {
        Self {
            readback_capacity: diagnostics.readback_capacity,
            readbacks_in_flight: diagnostics.readbacks_in_flight,
            pending_requests: diagnostics.pending_requests,
            queued_results: diagnostics.queued_results,
            dropped_results: diagnostics.dropped_results,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TimingDiagnosticsReport {
    availability: TimingAvailabilityReport,
    readback_capacity: usize,
    readbacks_in_flight: usize,
    queued_results: usize,
    dropped_results: u64,
}

impl From<TimingDiagnostics> for TimingDiagnosticsReport {
    fn from(diagnostics: TimingDiagnostics) -> Self {
        Self {
            availability: TimingAvailabilityReport::from(diagnostics.availability),
            readback_capacity: diagnostics.readback_capacity,
            readbacks_in_flight: diagnostics.readbacks_in_flight,
            queued_results: diagnostics.queued_results,
            dropped_results: diagnostics.dropped_results,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TimingAvailabilityReport {
    status: &'static str,
    code: &'static str,
    failure: Option<String>,
}

impl From<TimingAvailability> for TimingAvailabilityReport {
    fn from(availability: TimingAvailability) -> Self {
        match availability {
            TimingAvailability::Available => Self {
                status: "Pass",
                code: "Available",
                failure: None,
            },
            TimingAvailability::NotRequested => Self {
                status: "NotRun",
                code: "NotRequested",
                failure: None,
            },
            TimingAvailability::UnsupportedCapability => Self {
                status: "UnsupportedCapability",
                code: "UnsupportedCapability",
                failure: None,
            },
            TimingAvailability::UnsupportedPlatform(failure) => Self {
                status: "UnsupportedPlatform",
                code: "UnsupportedPlatform",
                failure: Some(format!("{failure:?}")),
            },
            TimingAvailability::Inconclusive(failure) => Self {
                status: "Inconclusive",
                code: "Inconclusive",
                failure: Some(format!("{failure:?}")),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct GpuTimingOutcomeReport {
    status: &'static str,
    code: &'static str,
    nanoseconds: Option<u64>,
    failure: Option<String>,
}

impl From<GpuTimingOutcome> for GpuTimingOutcomeReport {
    fn from(outcome: GpuTimingOutcome) -> Self {
        match outcome {
            GpuTimingOutcome::Measured(duration) => Self {
                status: "Pass",
                code: "Measured",
                nanoseconds: Some(duration_nanos(duration)),
                failure: None,
            },
            GpuTimingOutcome::NotRequested => Self {
                status: "NotRun",
                code: "NotRequested",
                nanoseconds: None,
                failure: None,
            },
            GpuTimingOutcome::UnsupportedCapability => Self {
                status: "UnsupportedCapability",
                code: "UnsupportedCapability",
                nanoseconds: None,
                failure: None,
            },
            GpuTimingOutcome::UnsupportedPlatform(failure) => Self {
                status: "UnsupportedPlatform",
                code: "UnsupportedPlatform",
                nanoseconds: None,
                failure: Some(format!("{failure:?}")),
            },
            GpuTimingOutcome::Inconclusive(failure) => Self {
                status: "Inconclusive",
                code: "Inconclusive",
                nanoseconds: None,
                failure: Some(format!("{failure:?}")),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct PassTimingReport {
    timing_frame_id: u64,
    runtime_frame_id: Option<u64>,
    submission_id: u64,
    pass: String,
    cpu_encode_nanoseconds: u64,
    gpu: GpuTimingOutcomeReport,
}

impl From<PassTimingSample> for PassTimingReport {
    fn from(sample: PassTimingSample) -> Self {
        Self {
            timing_frame_id: sample.frame_id.get(),
            runtime_frame_id: sample.runtime_frame_id.map(FrameId::get),
            submission_id: sample.submission_id,
            pass: sample.pass.as_str().to_owned(),
            cpu_encode_nanoseconds: duration_nanos(sample.cpu_encode_time),
            gpu: GpuTimingOutcomeReport::from(sample.gpu),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TimingFrameReport {
    explicit_timing_frame_id: u64,
    runtime_frame_id: u64,
    collection_status: &'static str,
    collection_code: &'static str,
    diagnostics: TimingDiagnosticsReport,
    pass_samples: Vec<PassTimingReport>,
    unrelated_pass_samples: Vec<PassTimingReport>,
}

#[derive(Clone, Debug, Serialize)]
struct CaptureReport {
    status: &'static str,
    code: &'static str,
    capture_id: Option<u64>,
    runtime_frame_id: u64,
    width: Option<u32>,
    height: Option<u32>,
    format: Option<&'static str>,
    source: Option<&'static str>,
    pixel_bytes: Option<usize>,
    failure: Option<String>,
    diagnostics_before: CaptureDiagnosticsReport,
    diagnostics_after: CaptureDiagnosticsReport,
}

#[derive(Clone, Debug, Serialize)]
struct SampleReport {
    schema: &'static str,
    sequence: u64,
    harness_mode: &'static str,
    harness_definition: &'static str,
    sample_kind: &'static str,
    ordinal: usize,
    corpus_case: String,
    corpus_hash: String,
    wall: WallTimingReport,
    plan_footprint: PlanFootprintReport,
    timing_frame: TimingFrameReport,
    capture: CaptureReport,
}

#[derive(Clone, Debug, Serialize)]
struct WallTimingReport {
    prepare: DurationReport,
    upload: DurationReport,
    submit: DurationReport,
    capture_wait: DurationReport,
    capture_wait_definition: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PercentileSummary {
    status: &'static str,
    code: &'static str,
    observations: usize,
    missing_observations: usize,
    p50_nanoseconds: Option<u64>,
    p95_nanoseconds: Option<u64>,
    p99_nanoseconds: Option<u64>,
    worst_nanoseconds: Option<u64>,
    method: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ModeSummary {
    harness_mode: &'static str,
    harness_definition: &'static str,
    measured_samples_requested: usize,
    measured_samples_recorded: usize,
    prepare: PercentileSummary,
    upload: PercentileSummary,
    submit: PercentileSummary,
    capture_wait: PercentileSummary,
}

#[derive(Clone, Debug, Serialize)]
struct ReuseSetupReport {
    prepare: DurationReport,
    upload: DurationReport,
    plan_footprint: PlanFootprintReport,
}

#[derive(Clone, Debug, Serialize)]
struct MemoryAvailabilityReport {
    actual_backend_allocations: &'static str,
    vram_usage: &'static str,
    driver_residency: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ProfileMetadataAvailability {
    availability: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AvailabilityReport {
    availability: &'static str,
    value: Option<String>,
    reason: &'static str,
}

impl AvailabilityReport {
    const fn not_available(reason: &'static str) -> Self {
        Self {
            availability: "NotAvailable",
            value: None,
            reason,
        }
    }

    fn available(value: impl Into<String>, reason: &'static str) -> Self {
        Self {
            availability: "Available",
            value: Some(value.into()),
            reason,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CacheScopeReport {
    resource_setup_policy: &'static str,
    steady_reuse_policy: &'static str,
    shader_pipeline_cache_state: AvailabilityReport,
    operating_system_driver_cache_state: AvailabilityReport,
}

#[derive(Clone, Debug, Serialize)]
struct WarmupScopeReport {
    requested_per_mode: Option<usize>,
    retained_in_raw_sample_log: bool,
    included_in_percentile_summaries: bool,
    included_in_gpu_timing_aggregate: bool,
    cache_reset_between_warmup_and_measurement: AvailabilityReport,
}

#[derive(Clone, Debug, Serialize)]
struct RepetitionScopeReport {
    measurement_samples_requested_per_mode: Option<usize>,
    independent_process_repetitions: u8,
    mode_execution_order: [&'static str; 2],
    randomized_mode_order: bool,
    statistical_scope: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ExecutionScopeReport {
    invocations: u8,
    warmup_iterations: Option<usize>,
    cache_scope: CacheScopeReport,
    cross_run_cache_state: &'static str,
    repetition_scope: RepetitionScopeReport,
    warmup_scope: WarmupScopeReport,
}

#[derive(Clone, Debug, Serialize)]
struct MemoryTelemetryReport {
    planned_payload_accounting: &'static str,
    actual_backend_allocations: &'static str,
    vram_usage: &'static str,
    driver_residency: &'static str,
    cache_residency: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceEnvironmentReport {
    requirement_ids: [&'static str; 2],
    work_package_id: &'static str,
    milestone_id: &'static str,
    research_gate_id: &'static str,
    runner_package: &'static str,
    runner_package_version: &'static str,
    build_identity: AvailabilityReport,
    build_hash: AvailabilityReport,
    toolchain_profile: AvailabilityReport,
    dependency_profile: AvailabilityReport,
    capability_profile: AvailabilityReport,
    execution_scope: ExecutionScopeReport,
    memory_telemetry: MemoryTelemetryReport,
}

#[derive(Clone, Debug, Serialize)]
struct FinalCaptureArtifactsReport {
    selection: &'static str,
    sequence: u64,
    harness_mode: &'static str,
    sample_kind: &'static str,
    ordinal: usize,
    runtime_frame_id: u64,
    capture_id: u64,
    png: String,
    png_metadata: String,
    rgba: String,
    pixel_hash: String,
    png_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceState {
    CleanCommit,
    WorkingTree,
}

impl SourceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CleanCommit => "CleanCommit",
            Self::WorkingTree => "WorkingTree",
        }
    }

    const fn promotion_eligibility(self) -> &'static str {
        match self {
            Self::CleanCommit => "NotEligibleCallerDeclaredCleanCommit",
            Self::WorkingTree => "NotEligibleWorkingTree",
        }
    }

    const fn verification(self) -> &'static str {
        let _ = self;
        "CallerDeclaredNotVerified"
    }
}

#[derive(Clone, Debug)]
struct SourceProvenance {
    checkpoint: String,
    state: SourceState,
}

#[derive(Clone, Debug, Serialize)]
struct PerformanceReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    source_checkpoint: String,
    source_state: &'static str,
    source_provenance_verification: &'static str,
    source_provenance_limit: &'static str,
    evidence_scope: &'static str,
    promotion_eligibility: &'static str,
    evidence_directory: String,
    environment: EvidenceEnvironmentReport,
    profile: GpuProfileReport,
    rhi_configuration: RhiConfigurationReport,
    corpus_case: String,
    corpus_hash: String,
    warmup_count: usize,
    sample_count: usize,
    harness_modes: [HarnessModeDescription; 2],
    steady_reuse_setup: Option<ReuseSetupReport>,
    summaries: [ModeSummary; 2],
    gpu_timing_status: &'static str,
    gpu_timing_code: &'static str,
    gpu_timing_by_mode: [ModeGpuTimingReport; 2],
    memory_availability: MemoryAvailabilityReport,
    final_capture_artifacts: Option<FinalCaptureArtifactsReport>,
    raw_sample_log: String,
    limits: [&'static str; 4],
}

#[derive(Clone, Debug, Serialize)]
struct FailureReport {
    schema: &'static str,
    status: &'static str,
    code: &'static str,
    cause: &'static str,
    source_checkpoint: Option<String>,
    source_state: Option<&'static str>,
    source_provenance_verification: &'static str,
    source_provenance_limit: &'static str,
    evidence_directory: &'static str,
    environment: EvidenceEnvironmentReport,
    detail: String,
    detail_policy: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct HarnessModeDescription {
    mode: &'static str,
    definition: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ModeGpuTimingReport {
    harness_mode: &'static str,
    sample_kind: &'static str,
    measurement_samples_requested: usize,
    measurement_samples_recorded: usize,
    pass_samples_considered: usize,
    status: &'static str,
    code: &'static str,
}

struct ReusableFrame {
    plan: UiDirectFramePlan,
    gpu: UiDirectGpuFrame,
    setup: ReuseSetupReport,
}

struct ResourceSetupFrame {
    plan: UiDirectFramePlan,
    gpu: UiDirectGpuFrame,
}

struct SubmissionInput {
    resource_setup_frame: Option<ResourceSetupFrame>,
    footprint: PlanFootprintReport,
    prepare: DurationReport,
    upload: DurationReport,
    width: u32,
    height: u32,
}

struct PendingMeasurement {
    work: WorkItem,
    // Keeps per-sample GPU resources alive until the asynchronous capture is
    // complete. Steady reuse owns its frame separately.
    _resource_setup_frame: Option<ResourceSetupFrame>,
    runtime_frame_id: FrameId,
    timing_frame_id: TimingFrameId,
    expected_width: u32,
    expected_height: u32,
    footprint: PlanFootprintReport,
    prepare: DurationReport,
    upload: DurationReport,
    submit: DurationReport,
    capture_wait_started: Instant,
    capture_deadline: Instant,
    capture_diagnostics_before: CaptureDiagnosticsReport,
    capture: Option<CaptureReport>,
    captured_frame: Option<CapturedFrame>,
    capture_wait: Option<DurationReport>,
    timing_samples: Vec<PassTimingReport>,
    unrelated_timing_samples: Vec<PassTimingReport>,
}

struct FinalCapture {
    work: WorkItem,
    frame: CapturedFrame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalDisposition {
    Completed,
    NotRun,
    Inconclusive,
}

impl TerminalDisposition {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "Pass",
            Self::NotRun => "NotRun",
            Self::Inconclusive => "Inconclusive",
        }
    }
}

struct PerformanceRunner {
    args: RunnerArgs,
    failure: Arc<Mutex<Option<String>>>,
    source: SourceProvenance,
    rhi: Option<Rhi>,
    renderer: Option<UiDirectGpuRenderer>,
    case: Option<UiDirectQualificationCase>,
    profile: Option<GpuProfileReport>,
    rhi_configuration: Option<RhiConfigurationReport>,
    work: Vec<WorkItem>,
    work_index: usize,
    reusable: Option<ReusableFrame>,
    pending: Option<PendingMeasurement>,
    samples: Vec<SampleReport>,
    final_capture: Option<FinalCapture>,
    disposition: TerminalDisposition,
    disposition_detail: Option<String>,
}

impl PerformanceRunner {
    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        self.fail_with_outcome("Fail", "Failure", "RunnerError", message, context);
    }

    fn fail_error(&mut self, error: &(dyn Error + 'static), context: &mut PlatformContext<'_>) {
        let (status, code) = error_evidence_outcome(error);
        self.fail_with_outcome(status, code, "RunnerError", error.to_string(), context);
    }

    fn fail_with_outcome(
        &mut self,
        status: &'static str,
        code: &'static str,
        cause: &'static str,
        message: impl Into<String>,
        context: &mut PlatformContext<'_>,
    ) {
        let message = message.into();
        let message = if self.write_failure(status, code, cause, &message).is_err() {
            "performance runner failed and its failure evidence could not be persisted".to_owned()
        } else {
            message
        };
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(message);
        }
        context.exit();
    }

    fn write_failure(
        &self,
        status: &'static str,
        code: &'static str,
        cause: &'static str,
        detail: &str,
    ) -> Result<(), Box<dyn Error>> {
        write_evidence_json(
            self.args.evidence_directory.join("failure.json"),
            &FailureReport {
                schema: "meridian.ui-direct-performance-failure/v1",
                status,
                code,
                cause,
                source_checkpoint: Some(self.source.checkpoint.clone()),
                source_state: Some(self.source.state.as_str()),
                source_provenance_verification: self.source.state.verification(),
                source_provenance_limit: SOURCE_PROVENANCE_LIMIT,
                evidence_directory: ".",
                environment: evidence_environment(Some(&self.args), self.profile.as_ref()),
                detail: sanitize_failure_detail(detail),
                detail_policy: "Failure detail is capped and omitted when it contains a path or control character.",
            },
        )?;
        Ok(())
    }

    fn initialize(
        &mut self,
        window: PlatformWindow,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let config = RhiConfig::default();
        let rhi = Rhi::new(window, config)?;
        let profile = profile_report(&rhi);
        let case = ui_direct_qualification_cases()?
            .into_iter()
            .find(|candidate| candidate.id == "standard-1x")
            .ok_or_else(|| "direct UI qualification corpus lacks standard-1x".to_owned())?;
        self.renderer = Some(UiDirectGpuRenderer::new(rhi.render_identity()));
        self.profile = Some(profile);
        self.rhi_configuration = Some(rhi_configuration_report(config));
        self.case = Some(case);
        self.rhi = Some(rhi);
        self.start_next(context)
    }

    fn prepare_reusable_frame(&mut self) -> Result<(), Box<dyn Error>> {
        let case = self
            .case
            .as_ref()
            .ok_or_else(|| "performance corpus case is unavailable".to_owned())?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| "performance renderer is unavailable".to_owned())?;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "performance RHI is unavailable".to_owned())?;
        let prepare_started = Instant::now();
        let plan = renderer.prepare_frame(case.prepare_request())?;
        let prepare = DurationReport::measured(prepare_started.elapsed());
        let footprint = PlanFootprintReport::from(plan.footprint());
        let upload_started = Instant::now();
        let gpu = plan.upload_gpu_frame(rhi)?;
        let upload = DurationReport::measured(upload_started.elapsed());
        self.reusable = Some(ReusableFrame {
            plan,
            gpu,
            setup: ReuseSetupReport {
                prepare,
                upload,
                plan_footprint: footprint,
            },
        });
        Ok(())
    }

    fn prepare_submission_input(
        &mut self,
        mode: HarnessMode,
    ) -> Result<SubmissionInput, Box<dyn Error>> {
        match mode {
            HarnessMode::ResourceSetup => self.prepare_resource_setup_input(),
            HarnessMode::SteadyReuse => self.prepare_steady_reuse_input(),
        }
    }

    fn prepare_resource_setup_input(&mut self) -> Result<SubmissionInput, Box<dyn Error>> {
        let case = self
            .case
            .as_ref()
            .ok_or_else(|| "performance corpus case is unavailable".to_owned())?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| "performance renderer is unavailable".to_owned())?;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "performance RHI is unavailable".to_owned())?;
        let prepare_started = Instant::now();
        let plan = renderer.prepare_frame(case.prepare_request())?;
        let prepare = DurationReport::measured(prepare_started.elapsed());
        let footprint = PlanFootprintReport::from(plan.footprint());
        let cache_key = plan.cache_key();
        let upload_started = Instant::now();
        let gpu = plan.upload_gpu_frame(rhi)?;
        let upload = DurationReport::measured(upload_started.elapsed());
        Ok(SubmissionInput {
            resource_setup_frame: Some(ResourceSetupFrame { plan, gpu }),
            footprint,
            prepare,
            upload,
            width: cache_key.surface_width,
            height: cache_key.surface_height,
        })
    }

    fn prepare_steady_reuse_input(&mut self) -> Result<SubmissionInput, Box<dyn Error>> {
        if self.reusable.is_none() {
            self.prepare_reusable_frame()?;
        }
        let reusable = self
            .reusable
            .as_ref()
            .ok_or_else(|| "steady-reuse resources are unavailable".to_owned())?;
        let cache_key = reusable.plan.cache_key();
        Ok(SubmissionInput {
            resource_setup_frame: None,
            footprint: PlanFootprintReport::from(reusable.plan.footprint()),
            prepare: DurationReport::not_applicable(),
            upload: DurationReport::not_applicable(),
            width: cache_key.surface_width,
            height: cache_key.surface_height,
        })
    }

    fn submit_input(
        &mut self,
        work: WorkItem,
        input: SubmissionInput,
    ) -> Result<PendingMeasurement, Box<dyn Error>> {
        let runtime_frame_id = FrameId::new(work.sequence);
        let max_bytes = u64::from(input.width)
            .checked_mul(u64::from(input.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "performance capture byte count overflowed".to_owned())?;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "performance RHI is unavailable".to_owned())?;
        let timing_frame_id = rhi.begin_timing_frame_for(runtime_frame_id)?;
        if let Err(error) = rhi.request_capture(CaptureRequest::new(
            runtime_frame_id,
            input.width,
            input.height,
            max_bytes,
        )) {
            let _ = rhi.end_timing_frame(timing_frame_id);
            return Err(error.into());
        }
        let submit_started = Instant::now();
        let submit_result = if let Some(frame) = input.resource_setup_frame.as_ref() {
            frame
                .gpu
                .submit_offscreen_capture(rhi, &frame.plan, QUALIFICATION_CLEAR)
        } else {
            let reusable = self
                .reusable
                .as_ref()
                .ok_or_else(|| "steady-reuse resources are unavailable".to_owned())?;
            reusable
                .gpu
                .submit_offscreen_capture(rhi, &reusable.plan, QUALIFICATION_CLEAR)
        };
        let submit = DurationReport::measured(submit_started.elapsed());
        let end_result = rhi.end_timing_frame(timing_frame_id);
        submit_result?;
        end_result?;
        let capture_diagnostics_before = CaptureDiagnosticsReport::from(rhi.capture_diagnostics());
        Ok(PendingMeasurement {
            work,
            _resource_setup_frame: input.resource_setup_frame,
            runtime_frame_id,
            timing_frame_id,
            expected_width: input.width,
            expected_height: input.height,
            footprint: input.footprint,
            prepare: input.prepare,
            upload: input.upload,
            submit,
            capture_wait_started: Instant::now(),
            capture_deadline: Instant::now() + CAPTURE_TIMEOUT,
            capture_diagnostics_before,
            capture: None,
            captured_frame: None,
            capture_wait: None,
            timing_samples: Vec::new(),
            unrelated_timing_samples: Vec::new(),
        })
    }

    fn start_next(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        if self.work_index >= self.work.len() {
            return self.finish(context);
        }
        let work = *self
            .work
            .get(self.work_index)
            .ok_or_else(|| "performance work item is unavailable".to_owned())?;
        let input = self.prepare_submission_input(work.mode)?;
        self.pending = Some(self.submit_input(work, input)?);
        context.request_redraw();
        Ok(())
    }

    fn poll_pending(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        let (expected_timing_frame, capture_pending) = self
            .pending
            .as_ref()
            .map(|pending| (pending.timing_frame_id, pending.capture.is_none()))
            .ok_or_else(|| "performance sample is not pending".to_owned())?;
        let (pass_samples, capture, timing_diagnostics, capture_diagnostics) = {
            let rhi = self
                .rhi
                .as_mut()
                .ok_or_else(|| "performance RHI is unavailable".to_owned())?;
            let mut pass_samples = Vec::new();
            while let Some(sample) = rhi.take_pass_timing() {
                pass_samples.push(sample);
            }
            let capture = capture_pending.then(|| rhi.take_capture()).flatten();
            (
                pass_samples,
                capture,
                TimingDiagnosticsReport::from(rhi.timing_diagnostics()),
                CaptureDiagnosticsReport::from(rhi.capture_diagnostics()),
            )
        };
        let now = Instant::now();
        let pending = self
            .pending
            .as_mut()
            .ok_or_else(|| "performance sample disappeared".to_owned())?;
        for sample in pass_samples {
            let report = PassTimingReport::from(sample);
            if report.timing_frame_id == expected_timing_frame.get() {
                pending.timing_samples.push(report);
            } else {
                pending.unrelated_timing_samples.push(report);
            }
        }
        if let Some(capture) = capture {
            let (report, frame) = validate_capture(
                capture,
                pending.runtime_frame_id,
                pending.expected_width,
                pending.expected_height,
                &pending.footprint,
                pending.capture_diagnostics_before.clone(),
                capture_diagnostics.clone(),
            )?;
            pending.capture = Some(report);
            pending.captured_frame = frame;
            pending.capture_wait = Some(DurationReport::measured(
                pending.capture_wait_started.elapsed(),
            ));
        }
        let capture_timed_out = pending.capture.is_none() && now >= pending.capture_deadline;
        if capture_timed_out {
            pending.capture = Some(CaptureReport {
                status: "Inconclusive",
                code: "TimedOut",
                capture_id: None,
                runtime_frame_id: pending.runtime_frame_id.get(),
                width: None,
                height: None,
                format: None,
                source: None,
                pixel_bytes: None,
                failure: Some("bounded asynchronous capture deadline elapsed".to_owned()),
                diagnostics_before: pending.capture_diagnostics_before.clone(),
                diagnostics_after: capture_diagnostics.clone(),
            });
            pending.capture_wait = Some(DurationReport::unavailable());
        }
        let capture_ready = pending.capture.is_some();
        let timing_drained = timing_diagnostics.readbacks_in_flight == 0;
        let timing_timed_out = capture_ready && !timing_drained && now >= pending.capture_deadline;
        if capture_ready && (timing_drained || timing_timed_out) {
            self.complete_pending(
                timing_diagnostics,
                capture_diagnostics,
                timing_timed_out,
                context,
            )?;
        } else {
            context.request_redraw();
        }
        Ok(())
    }

    fn complete_pending(
        &mut self,
        timing_diagnostics: TimingDiagnosticsReport,
        _capture_diagnostics: CaptureDiagnosticsReport,
        timing_timed_out: bool,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let pending = self
            .pending
            .take()
            .ok_or_else(|| "performance sample is unavailable".to_owned())?;
        let capture = pending
            .capture
            .ok_or_else(|| "performance sample has no capture outcome".to_owned())?;
        let capture_wait = pending
            .capture_wait
            .unwrap_or_else(DurationReport::unavailable);
        let (collection_status, collection_code) = if timing_timed_out {
            ("Inconclusive", "TimedOut")
        } else if pending.timing_samples.is_empty() {
            match timing_diagnostics.availability.code {
                "Available" => ("Inconclusive", "NoSamples"),
                other => (timing_diagnostics.availability.status, other),
            }
        } else {
            ("Pass", "Drained")
        };
        let report = SampleReport {
            schema: PERFORMANCE_SAMPLE_SCHEMA,
            sequence: pending.work.sequence,
            harness_mode: pending.work.mode.as_str(),
            harness_definition: pending.work.mode.definition(),
            sample_kind: pending.work.kind.as_str(),
            ordinal: pending.work.ordinal,
            corpus_case: self
                .case
                .as_ref()
                .ok_or_else(|| "performance corpus case is unavailable".to_owned())?
                .id
                .to_owned(),
            corpus_hash: self
                .case
                .as_ref()
                .ok_or_else(|| "performance corpus case is unavailable".to_owned())?
                .corpus_hash(),
            wall: WallTimingReport {
                prepare: pending.prepare,
                upload: pending.upload,
                submit: pending.submit,
                capture_wait,
                capture_wait_definition: "Wall time from successful submit return until asynchronous capture outcome; it is not GPU execution or interactive latency.",
            },
            plan_footprint: pending.footprint,
            timing_frame: TimingFrameReport {
                explicit_timing_frame_id: pending.timing_frame_id.get(),
                runtime_frame_id: pending.runtime_frame_id.get(),
                collection_status,
                collection_code,
                diagnostics: timing_diagnostics,
                pass_samples: pending.timing_samples,
                unrelated_pass_samples: pending.unrelated_timing_samples,
            },
            capture: capture.clone(),
        };
        append_evidence_json_line(self.args.evidence_directory.join("samples.jsonl"), &report)?;
        if report.sample_kind == SampleKind::Measurement.as_str() {
            if let Some(frame) = pending.captured_frame {
                self.final_capture = Some(FinalCapture {
                    work: pending.work,
                    frame,
                });
            }
        }
        let disposition = match capture.code {
            "Captured" => None,
            "UnsupportedCapability" => Some((
                TerminalDisposition::NotRun,
                "offscreen capture is unsupported on the active RHI profile".to_owned(),
            )),
            "Inconclusive" | "TimedOut" => Some((
                TerminalDisposition::Inconclusive,
                capture
                    .failure
                    .clone()
                    .unwrap_or_else(|| "offscreen capture did not produce pixels".to_owned()),
            )),
            other => Some((
                TerminalDisposition::Inconclusive,
                format!("unrecognized capture code {other}"),
            )),
        };
        self.samples.push(report);
        self.work_index = self.work_index.saturating_add(1);
        if let Some((disposition, detail)) = disposition {
            self.disposition = disposition;
            self.disposition_detail = Some(detail);
            self.finish(context)?;
        } else {
            self.start_next(context)?;
        }
        Ok(())
    }

    fn final_capture_artifacts(
        &self,
    ) -> Result<Option<FinalCaptureArtifactsReport>, Box<dyn Error>> {
        let artifacts = if let Some(captured) = self.final_capture.as_ref() {
            let frame = &captured.frame;
            let png = write_capture_png(
                self.args.evidence_directory.join("final-capture.png"),
                frame,
            )?;
            let _rgba = write_capture_rgba(
                self.args.evidence_directory.join("final-capture.rgba"),
                frame,
            )?;
            Some(FinalCaptureArtifactsReport {
                selection: "LastSuccessfulMeasurementCaptureInFixedModeOrder",
                sequence: captured.work.sequence,
                harness_mode: captured.work.mode.as_str(),
                sample_kind: captured.work.kind.as_str(),
                ordinal: captured.work.ordinal,
                runtime_frame_id: frame.frame_id.get(),
                capture_id: frame.capture_id.get(),
                png: "final-capture.png".to_owned(),
                png_metadata: "final-capture.png.json".to_owned(),
                rgba: "final-capture.rgba".to_owned(),
                pixel_hash: png.metadata.pixel_hash,
                png_hash: png.metadata.png_hash,
            })
        } else {
            None
        };
        Ok(artifacts)
    }

    fn build_report(
        &self,
        profile: GpuProfileReport,
        rhi_configuration: RhiConfigurationReport,
        case: &UiDirectQualificationCase,
        artifacts: Option<FinalCaptureArtifactsReport>,
    ) -> PerformanceReport {
        let resource_setup_summary =
            mode_summary(HarnessMode::ResourceSetup, &self.samples, self.args.samples);
        let steady_reuse_summary =
            mode_summary(HarnessMode::SteadyReuse, &self.samples, self.args.samples);
        let gpu_timing_by_mode = [
            mode_gpu_timing_report(HarnessMode::ResourceSetup, &self.samples, self.args.samples),
            mode_gpu_timing_report(HarnessMode::SteadyReuse, &self.samples, self.args.samples),
        ];
        let (gpu_timing_status, gpu_timing_code) = gpu_timing_outcome(&self.samples);
        PerformanceReport {
            schema: PERFORMANCE_SCHEMA,
            runner_status: self.disposition.as_str(),
            evidence_status: performance_evidence_status(self.disposition),
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            source_provenance_verification: self.source.state.verification(),
            source_provenance_limit: SOURCE_PROVENANCE_LIMIT,
            evidence_scope: "LocalStructuralEvidence",
            promotion_eligibility: self.source.state.promotion_eligibility(),
            evidence_directory: ".".to_owned(),
            environment: evidence_environment(Some(&self.args), Some(&profile)),
            profile,
            rhi_configuration,
            corpus_case: case.id.to_owned(),
            corpus_hash: case.corpus_hash(),
            warmup_count: self.args.warmup,
            sample_count: self.args.samples,
            harness_modes: [
                HarnessModeDescription {
                    mode: HarnessMode::ResourceSetup.as_str(),
                    definition: HarnessMode::ResourceSetup.definition(),
                },
                HarnessModeDescription {
                    mode: HarnessMode::SteadyReuse.as_str(),
                    definition: HarnessMode::SteadyReuse.definition(),
                },
            ],
            steady_reuse_setup: self
                .reusable
                .as_ref()
                .map(|reusable| reusable.setup.clone()),
            summaries: [resource_setup_summary, steady_reuse_summary],
            gpu_timing_status,
            gpu_timing_code,
            gpu_timing_by_mode,
            memory_availability: MemoryAvailabilityReport {
                actual_backend_allocations: "UnsupportedCapability",
                vram_usage: "UnsupportedCapability",
                driver_residency: "UnsupportedCapability",
                reason: "The public direct-UI/RHI contract accounts planned payload, not backend allocator, VRAM, or driver residency telemetry; this does not claim the underlying backend lacks those facilities.",
            },
            final_capture_artifacts: artifacts,
            raw_sample_log: "samples.jsonl".to_owned(),
            limits: [
                "No numeric performance thresholds are evaluated or implied by this report.",
                "Wall-clock capture wait is asynchronous readback completion time, not GPU execution or interactive latency.",
                "Typed unsupported or inconclusive GPU timing outcomes remain evidence statuses; they are never converted to a pass.",
                "Hidden offscreen capture is renderer evidence only and is not presented visual review or cross-platform qualification.",
            ],
        }
    }

    fn record_inconclusive_failure(&mut self) -> Result<(), Box<dyn Error>> {
        if self.disposition != TerminalDisposition::Inconclusive {
            return Ok(());
        }
        let detail = self
            .disposition_detail
            .clone()
            .unwrap_or_else(|| "performance evidence is inconclusive".to_owned());
        self.write_failure(
            "Inconclusive",
            "Inconclusive",
            "CaptureOrTimingInconclusive",
            &detail,
        )?;
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(detail);
        }
        Ok(())
    }

    fn finish(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        let profile = self
            .profile
            .clone()
            .ok_or_else(|| "performance profile is unavailable".to_owned())?;
        let rhi_configuration = self
            .rhi_configuration
            .clone()
            .ok_or_else(|| "performance RHI configuration is unavailable".to_owned())?;
        let case = self
            .case
            .as_ref()
            .ok_or_else(|| "performance corpus case is unavailable".to_owned())?;
        let report = self.build_report(
            profile,
            rhi_configuration,
            case,
            self.final_capture_artifacts()?,
        );
        write_evidence_json(
            self.args.evidence_directory.join("performance.json"),
            &report,
        )?;
        println!(
            "Meridian direct UI performance evidence: {} (runner {}, evidence {})",
            self.args.evidence_directory.display(),
            report.runner_status,
            report.evidence_status
        );
        self.record_inconclusive_failure()?;
        context.exit();
        Ok(())
    }
}

impl PlatformApplication for PerformanceRunner {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail("performance window was not available", context);
                    return;
                };
                if let Err(error) = self.initialize(window, context) {
                    self.fail_error(error.as_ref(), context);
                }
            }
            PlatformEvent::RedrawRequested => {
                if self.pending.is_some() {
                    if let Err(error) = self.poll_pending(context) {
                        self.fail_error(error.as_ref(), context);
                    }
                }
            }
            PlatformEvent::CloseRequested => {
                self.fail("native evidence window closed before completion", context);
            }
            _ => {}
        }
    }
}

fn error_evidence_outcome(error: &(dyn Error + 'static)) -> (&'static str, &'static str) {
    if let Some(error) = error.downcast_ref::<RhiError>() {
        return rhi_error_evidence_outcome(error);
    }
    if let Some(error) = error.downcast_ref::<UiDirectRendererError>() {
        return ui_direct_error_evidence_outcome(error);
    }
    ("Fail", "Failure")
}

fn rhi_error_evidence_outcome(error: &RhiError) -> (&'static str, &'static str) {
    match error.kind() {
        RhiErrorKind::AdapterUnavailable => ("UnsupportedPlatform", "AdapterUnavailable"),
        RhiErrorKind::SurfaceUnsupported => ("UnsupportedPlatform", "SurfaceUnsupported"),
        RhiErrorKind::CaptureTargetUnsupported => {
            ("UnsupportedCapability", "CaptureTargetUnsupported")
        }
        RhiErrorKind::SurfaceCreation => ("Inconclusive", "SurfaceCreation"),
        RhiErrorKind::DeviceCreation => ("Inconclusive", "DeviceCreation"),
        RhiErrorKind::DeviceLost => ("Inconclusive", "DeviceLost"),
        RhiErrorKind::TimestampReadback => ("Inconclusive", "TimestampReadback"),
        _ => ("Fail", "RhiFailure"),
    }
}

fn ui_direct_error_evidence_outcome(error: &UiDirectRendererError) -> (&'static str, &'static str) {
    match error {
        UiDirectRendererError::UnsupportedSurfaceColorSpace => {
            ("UnsupportedCapability", "UnsupportedSurfaceColorSpace")
        }
        UiDirectRendererError::UnsupportedPrimitiveKind { .. } => {
            ("UnsupportedCapability", "UnsupportedPrimitiveKind")
        }
        UiDirectRendererError::OffscreenCaptureUnsupported { .. } => (
            "UnsupportedCapability",
            "OffscreenCaptureCopySourceUnsupported",
        ),
        UiDirectRendererError::Rhi(error) => rhi_error_evidence_outcome(error),
        UiDirectRendererError::StaleGpuFrame { .. } => ("Inconclusive", "StaleGpuFrame"),
        UiDirectRendererError::StaleRhiIdentity { .. } => ("Inconclusive", "StaleRhiIdentity"),
        _ => ("Fail", "UiDirectFailure"),
    }
}

fn profile_report(rhi: &Rhi) -> GpuProfileReport {
    let capabilities = rhi.capabilities();
    let (driver, driver_availability) =
        explicit_profile_metadata(&capabilities.driver, DRIVER_UNAVAILABLE);
    let (driver_info, driver_info_availability) =
        explicit_profile_metadata(&capabilities.driver_info, DRIVER_INFO_UNAVAILABLE);
    let enabled_features = capabilities
        .features
        .iter()
        .map(|feature| format!("{feature:?}"))
        .collect::<Vec<_>>();
    let missing_features = TRACKED_GPU_FEATURES
        .iter()
        .filter(|feature| !capabilities.features.contains(feature))
        .map(|feature| format!("{feature:?}"))
        .collect::<Vec<_>>();
    GpuProfileReport {
        backend: json_safe_machine_text(&format!("{:?}", capabilities.backend)),
        adapter_name: json_safe_machine_text(&capabilities.adapter_name),
        driver,
        driver_availability,
        driver_info,
        driver_info_availability,
        vendor_id: capabilities.vendor_id,
        device_id: capabilities.device_id,
        adapter_kind: json_safe_machine_text(&format!("{:?}", capabilities.adapter_kind)),
        memory_class: json_safe_machine_text(&format!("{:?}", capabilities.memory_class)),
        timestamp_query_capability: json_safe_machine_text(&format!(
            "{:?}",
            capabilities.timestamp_queries
        )),
        hdr_surface_formats_capability: json_safe_machine_text(&format!(
            "{:?}",
            capabilities.hdr_surface_formats
        )),
        max_sampled_textures_per_shader_stage: capabilities.max_sampled_textures_per_shader_stage,
        enabled_features,
        missing_features,
        surface_format: json_safe_machine_text(&rhi.surface_format().name),
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
    }
}

fn rhi_configuration_report(config: RhiConfig) -> RhiConfigurationReport {
    RhiConfigurationReport {
        power_preference: format!("{:?}", config.power_preference),
        preferred_backend: config
            .preferred_backend
            .map(|backend| format!("{backend:?}")),
        allow_software_adapter: config.allow_software_adapter,
        present_policy: format!("{:?}", config.present_policy),
        desired_maximum_frame_latency: config.desired_maximum_frame_latency,
        timestamps_requested: config.enable_timestamps,
    }
}

fn evidence_environment(
    args: Option<&RunnerArgs>,
    profile: Option<&GpuProfileReport>,
) -> EvidenceEnvironmentReport {
    EvidenceEnvironmentReport {
        requirement_ids: ["REQ-UI-001", "REQ-UI-002"],
        work_package_id: "WP-UI-005",
        milestone_id: "MS-03",
        research_gate_id: "RG-UI-001",
        runner_package: env!("CARGO_PKG_NAME"),
        runner_package_version: env!("CARGO_PKG_VERSION"),
        build_identity: AvailabilityReport::not_available(BUILD_IDENTITY_UNAVAILABLE),
        build_hash: AvailabilityReport::not_available(BUILD_HASH_UNAVAILABLE),
        toolchain_profile: AvailabilityReport::not_available(TOOLCHAIN_PROFILE_UNAVAILABLE),
        dependency_profile: AvailabilityReport::not_available(DEPENDENCY_PROFILE_UNAVAILABLE),
        capability_profile: match profile {
            Some(profile) => AvailabilityReport::available(
                format!(
                    "backend={}, adapter={}, enabled_features={}, missing_features={}",
                    profile.backend,
                    profile.adapter_name,
                    profile.enabled_features.len(),
                    profile.missing_features.len()
                ),
                "Complete per-run RHI capability details are recorded in this report's profile field.",
            ),
            None => AvailabilityReport::not_available(
                "No RHI capability profile was available before this report was emitted.",
            ),
        },
        execution_scope: ExecutionScopeReport {
            invocations: 1,
            warmup_iterations: args.map(|args| args.warmup),
            cache_scope: CacheScopeReport {
                resource_setup_policy: "Each sampled iteration prepares and uploads a fresh immutable plan and GPU frame; cache state is not purged or observed.",
                steady_reuse_policy: "One immutable plan and GPU frame are prepared once before the sampled loop and then reused; cache state is not purged or observed.",
                shader_pipeline_cache_state: AvailabilityReport::not_available(
                    SHADER_PIPELINE_CACHE_UNAVAILABLE,
                ),
                operating_system_driver_cache_state: AvailabilityReport::not_available(
                    OPERATING_SYSTEM_DRIVER_CACHE_UNAVAILABLE,
                ),
            },
            cross_run_cache_state: "NotMeasured",
            repetition_scope: RepetitionScopeReport {
                measurement_samples_requested_per_mode: args.map(|args| args.samples),
                independent_process_repetitions: 1,
                mode_execution_order: ["resource_setup", "steady_reuse"],
                randomized_mode_order: false,
                statistical_scope: "Percentiles summarize measurement samples within this one process and fixed mode order; no cross-run distribution is inferred.",
            },
            warmup_scope: WarmupScopeReport {
                requested_per_mode: args.map(|args| args.warmup),
                retained_in_raw_sample_log: true,
                included_in_percentile_summaries: false,
                included_in_gpu_timing_aggregate: false,
                cache_reset_between_warmup_and_measurement: AvailabilityReport::not_available(
                    CACHE_RESET_UNAVAILABLE,
                ),
            },
        },
        memory_telemetry: MemoryTelemetryReport {
            planned_payload_accounting: "Available",
            actual_backend_allocations: "UnsupportedCapability",
            vram_usage: "UnsupportedCapability",
            driver_residency: "UnsupportedCapability",
            cache_residency: "UnsupportedCapability",
            reason: "The public direct-UI/RHI contract accounts planned payload, not backend allocator, VRAM, cache-residency, or driver-residency telemetry; this does not claim the underlying backend lacks those facilities.",
        },
    }
}

fn json_safe_machine_text(value: &str) -> String {
    if value.contains('/') || value.contains('\\') {
        "PathRedacted".to_owned()
    } else {
        value.to_owned()
    }
}

fn explicit_profile_metadata(
    value: &str,
    unavailable_reason: &'static str,
) -> (String, ProfileMetadataAvailability) {
    let value = json_safe_machine_text(value.trim());
    if value.is_empty() {
        return (
            "NotAvailable".to_owned(),
            ProfileMetadataAvailability {
                availability: "NotAvailable",
                reason: unavailable_reason,
            },
        );
    }
    (
        value,
        ProfileMetadataAvailability {
            availability: "Available",
            reason: PROFILE_METADATA_AVAILABLE,
        },
    )
}

fn sanitize_failure_detail(value: &str) -> String {
    if value.contains('/') || value.contains('\\') || value.chars().any(char::is_control) {
        return "PathOrControlCharacterRedacted".to_owned();
    }
    let mut detail = value
        .chars()
        .take(MAX_FAILURE_DETAIL_CHARS)
        .collect::<String>();
    if value.chars().count() > MAX_FAILURE_DETAIL_CHARS {
        detail.push_str("...");
    }
    if detail.trim().is_empty() {
        "NoFailureDetail".to_owned()
    } else {
        detail
    }
}

fn validate_capture(
    outcome: CaptureOutcome,
    expected_frame_id: FrameId,
    expected_width: u32,
    expected_height: u32,
    footprint: &PlanFootprintReport,
    diagnostics_before: CaptureDiagnosticsReport,
    diagnostics_after: CaptureDiagnosticsReport,
) -> Result<(CaptureReport, Option<CapturedFrame>), Box<dyn Error>> {
    match outcome {
        CaptureOutcome::Captured(frame) => {
            let expected_bytes = u64::from(frame.width)
                .checked_mul(u64::from(frame.height))
                .and_then(|pixels| pixels.checked_mul(4))
                .and_then(|bytes| usize::try_from(bytes).ok())
                .ok_or_else(|| "performance capture byte count overflowed".to_owned())?;
            if frame.frame_id != expected_frame_id
                || frame.width != expected_width
                || frame.height != expected_height
                || frame.format != CapturedPixelFormat::Rgba8Srgb
                || frame.source != CaptureSource::Offscreen
                || frame.surface_outcome.is_some()
                || frame.pixels.len() != expected_bytes
                || frame.pixels.is_empty()
                || footprint.primitive_count == 0
            {
                return Err(format!("performance capture metadata is invalid: {frame:?}").into());
            }
            Ok((
                CaptureReport {
                    status: "Pass",
                    code: "Captured",
                    capture_id: Some(frame.capture_id.get()),
                    runtime_frame_id: frame.frame_id.get(),
                    width: Some(frame.width),
                    height: Some(frame.height),
                    format: Some("rgba8-srgb"),
                    source: Some("offscreen"),
                    pixel_bytes: Some(frame.pixels.len()),
                    failure: None,
                    diagnostics_before,
                    diagnostics_after,
                },
                Some(frame),
            ))
        }
        CaptureOutcome::UnsupportedCapability {
            capture_id,
            frame_id,
            failure,
        } => Ok((
            CaptureReport {
                status: "UnsupportedCapability",
                code: "UnsupportedCapability",
                capture_id: Some(capture_id.get()),
                runtime_frame_id: frame_id.get(),
                width: None,
                height: None,
                format: None,
                source: None,
                pixel_bytes: None,
                failure: Some(format!("{failure:?}")),
                diagnostics_before,
                diagnostics_after,
            },
            None,
        )),
        CaptureOutcome::Inconclusive {
            capture_id,
            frame_id,
            failure,
        } => Ok((
            CaptureReport {
                status: "Inconclusive",
                code: "Inconclusive",
                capture_id: Some(capture_id.get()),
                runtime_frame_id: frame_id.get(),
                width: None,
                height: None,
                format: None,
                source: None,
                pixel_bytes: None,
                failure: Some(format!("{failure:?}")),
                diagnostics_before,
                diagnostics_after,
            },
            None,
        )),
    }
}

fn mode_summary(mode: HarnessMode, samples: &[SampleReport], requested: usize) -> ModeSummary {
    let measured = samples
        .iter()
        .filter(|sample| {
            sample.harness_mode == mode.as_str()
                && sample.sample_kind == SampleKind::Measurement.as_str()
        })
        .collect::<Vec<_>>();
    ModeSummary {
        harness_mode: mode.as_str(),
        harness_definition: mode.definition(),
        measured_samples_requested: requested,
        measured_samples_recorded: measured.len(),
        prepare: duration_summary(
            measured.iter().map(|sample| &sample.wall.prepare),
            requested,
        ),
        upload: duration_summary(measured.iter().map(|sample| &sample.wall.upload), requested),
        submit: duration_summary(measured.iter().map(|sample| &sample.wall.submit), requested),
        capture_wait: duration_summary(
            measured.iter().map(|sample| &sample.wall.capture_wait),
            requested,
        ),
    }
}

fn duration_summary<'a>(
    reports: impl IntoIterator<Item = &'a DurationReport>,
    requested: usize,
) -> PercentileSummary {
    let reports = reports.into_iter().collect::<Vec<_>>();
    let all_not_applicable =
        !reports.is_empty() && reports.iter().all(|report| report.code == "NotApplicable");
    let mut values = reports
        .iter()
        .filter_map(|report| report.nanoseconds)
        .collect::<Vec<_>>();
    values.sort_unstable();
    if values.is_empty() {
        return PercentileSummary {
            status: if all_not_applicable {
                "NotRun"
            } else {
                "Inconclusive"
            },
            code: if all_not_applicable {
                "NotApplicable"
            } else {
                "NotAvailable"
            },
            observations: 0,
            missing_observations: requested,
            p50_nanoseconds: None,
            p95_nanoseconds: None,
            p99_nanoseconds: None,
            worst_nanoseconds: None,
            method: "nearest-rank over measured samples only",
        };
    }
    let observations = values.len();
    PercentileSummary {
        status: if observations == requested {
            "Pass"
        } else {
            "Inconclusive"
        },
        code: if observations == requested {
            "Measured"
        } else {
            "Partial"
        },
        observations,
        missing_observations: requested.saturating_sub(observations),
        p50_nanoseconds: nearest_rank(&values, 50, 100),
        p95_nanoseconds: nearest_rank(&values, 95, 100),
        p99_nanoseconds: nearest_rank(&values, 99, 100),
        worst_nanoseconds: values.last().copied(),
        method: "nearest-rank over measured samples only",
    }
}

fn nearest_rank(values: &[u64], numerator: u64, denominator: u64) -> Option<u64> {
    if values.is_empty() || numerator == 0 || denominator == 0 {
        return None;
    }
    let length = u64::try_from(values.len()).ok()?;
    let rank = length
        .checked_mul(numerator)?
        .checked_add(denominator.saturating_sub(1))?
        / denominator;
    let index = usize::try_from(rank.saturating_sub(1)).ok()?;
    values.get(index).copied()
}

fn gpu_timing_outcome(samples: &[SampleReport]) -> (&'static str, &'static str) {
    let outcomes = gpu_timing_outcomes(samples, None);
    aggregate_gpu_timing_outcome(&outcomes)
}

fn mode_gpu_timing_report(
    mode: HarnessMode,
    samples: &[SampleReport],
    requested: usize,
) -> ModeGpuTimingReport {
    let measurement_samples = samples
        .iter()
        .filter(|sample| {
            sample.harness_mode == mode.as_str()
                && sample.sample_kind == SampleKind::Measurement.as_str()
        })
        .count();
    let outcomes = gpu_timing_outcomes(samples, Some(mode));
    let (status, code) = aggregate_gpu_timing_outcome(&outcomes);
    ModeGpuTimingReport {
        harness_mode: mode.as_str(),
        sample_kind: SampleKind::Measurement.as_str(),
        measurement_samples_requested: requested,
        measurement_samples_recorded: measurement_samples,
        pass_samples_considered: outcomes.len(),
        status,
        code,
    }
}

fn gpu_timing_outcomes(samples: &[SampleReport], mode: Option<HarnessMode>) -> Vec<&'static str> {
    samples
        .iter()
        .filter(|sample| sample.sample_kind == SampleKind::Measurement.as_str())
        .filter(|sample| mode.is_none_or(|mode| sample.harness_mode == mode.as_str()))
        .flat_map(|sample| sample.timing_frame.pass_samples.iter())
        .map(|sample| sample.gpu.code)
        .collect()
}

fn aggregate_gpu_timing_outcome(outcomes: &[&'static str]) -> (&'static str, &'static str) {
    if outcomes.is_empty() {
        return ("NotRun", "NoPassTimingSamples");
    }
    let measured = outcomes
        .iter()
        .filter(|status| **status == "Measured")
        .count();
    if measured == outcomes.len() {
        ("Pass", "Measured")
    } else if measured > 0 {
        ("Inconclusive", "Mixed")
    } else if outcomes.contains(&"Inconclusive") {
        ("Inconclusive", "Inconclusive")
    } else if outcomes.contains(&"UnsupportedPlatform") {
        ("UnsupportedPlatform", "UnsupportedPlatform")
    } else if outcomes.contains(&"UnsupportedCapability") {
        ("UnsupportedCapability", "UnsupportedCapability")
    } else if outcomes.contains(&"NotRequested") {
        ("NotRun", "NotRequested")
    } else {
        ("Inconclusive", "UnknownTimingOutcome")
    }
}

fn performance_evidence_status(runner_status: TerminalDisposition) -> &'static str {
    match runner_status {
        TerminalDisposition::NotRun => "NotRun",
        TerminalDisposition::Inconclusive | TerminalDisposition::Completed => "Inconclusive",
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn evidence_directory_from_args() -> Result<RunnerArgs, Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let mut evidence_directory = None;
    let mut warmup = None;
    let mut samples = None;
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            let path = arguments
                .next()
                .ok_or_else(|| "--evidence requires a path".to_owned())?;
            if evidence_directory.replace(PathBuf::from(path)).is_some() {
                return Err("--evidence may be provided only once".into());
            }
        } else if argument == "--warmup" {
            let value = arguments
                .next()
                .ok_or_else(|| "--warmup requires an integer".to_owned())?;
            if warmup
                .replace(parse_iterations("--warmup", &value, true)?)
                .is_some()
            {
                return Err("--warmup may be provided only once".into());
            }
        } else if argument == "--samples" {
            let value = arguments
                .next()
                .ok_or_else(|| "--samples requires an integer".to_owned())?;
            if samples
                .replace(parse_iterations("--samples", &value, false)?)
                .is_some()
            {
                return Err("--samples may be provided only once".into());
            }
        } else {
            return Err(
                format!("unrecognized performance argument: {}", argument.display()).into(),
            );
        }
    }
    let evidence_directory = evidence_directory.map_or_else(default_evidence_directory, Ok)?;
    Ok(RunnerArgs {
        evidence_directory,
        warmup: warmup.unwrap_or(DEFAULT_WARMUP),
        samples: samples.unwrap_or(DEFAULT_SAMPLES),
    })
}

fn default_evidence_directory() -> Result<PathBuf, Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(
        PathBuf::from("target/meridian-evidence/ui-direct-performance")
            .join(format!("{}-{nonce}", std::process::id())),
    )
}

fn source_provenance_from_environment() -> Result<SourceProvenance, Box<dyn Error>> {
    let state = std::env::var("MERIDIAN_SOURCE_STATE").map_err(|_| {
        "MERIDIAN_SOURCE_STATE is required and must be clean-commit or working-tree"
    })?;
    let checkpoint = std::env::var("MERIDIAN_SOURCE_CHECKPOINT")
        .map_err(|_| "MERIDIAN_SOURCE_CHECKPOINT is required for performance evidence")?;
    source_provenance_from_values(&state, &checkpoint)
}

fn source_provenance_from_values(
    state_value: &str,
    checkpoint_value: &str,
) -> Result<SourceProvenance, Box<dyn Error>> {
    let state = match state_value.trim() {
        "clean-commit" => SourceState::CleanCommit,
        "working-tree" => SourceState::WorkingTree,
        _ => {
            return Err(
                "MERIDIAN_SOURCE_STATE must be exactly clean-commit or working-tree".into(),
            );
        }
    };
    let checkpoint = validate_path_free_checkpoint(checkpoint_value)?;
    if state == SourceState::CleanCommit && !is_lowercase_commit_hash(&checkpoint) {
        return Err(
            "MERIDIAN_SOURCE_CHECKPOINT must be exactly 40 lowercase hexadecimal characters when MERIDIAN_SOURCE_STATE=clean-commit"
                .into(),
        );
    }
    Ok(SourceProvenance { checkpoint, state })
}

fn validate_path_free_checkpoint(value: &str) -> Result<String, Box<dyn Error>> {
    let checkpoint = value.trim();
    let allowed = checkpoint
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'));
    if checkpoint.is_empty() || checkpoint == "NotAvailable" || !allowed {
        return Err(
            "MERIDIAN_SOURCE_CHECKPOINT must be a non-empty path-free identifier using only ASCII letters, digits, '-', '_', or '.'"
                .into(),
        );
    }
    Ok(checkpoint.to_owned())
}

fn is_lowercase_commit_hash(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn evidence_directory_hint_from_args() -> Option<PathBuf> {
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            return arguments.next().map(PathBuf::from);
        }
    }
    default_evidence_directory().ok()
}

fn write_preflight_failure(
    evidence_directory: &std::path::Path,
    cause: &'static str,
    detail: &str,
    source: Option<&SourceProvenance>,
    args: Option<&RunnerArgs>,
) -> Result<(), Box<dyn Error>> {
    write_evidence_json(
        evidence_directory.join("failure.json"),
        &FailureReport {
            schema: "meridian.ui-direct-performance-failure/v1",
            status: "Fail",
            code: "PreflightFailure",
            cause,
            source_checkpoint: source.map(|source| source.checkpoint.clone()),
            source_state: source.map(|source| source.state.as_str()),
            source_provenance_verification: source
                .map_or("NotAvailable", |source| source.state.verification()),
            source_provenance_limit: source
                .map_or(SOURCE_PROVENANCE_UNAVAILABLE, |_| SOURCE_PROVENANCE_LIMIT),
            evidence_directory: ".",
            environment: evidence_environment(args, None),
            detail: sanitize_failure_detail(detail),
            detail_policy:
                "Failure detail is capped and omitted when it contains a path or control character.",
        },
    )?;
    Ok(())
}

fn parse_iterations(
    flag: &str,
    value: &std::ffi::OsStr,
    zero_allowed: bool,
) -> Result<usize, Box<dyn Error>> {
    let text = value
        .to_str()
        .ok_or_else(|| format!("{flag} must be valid UTF-8"))?;
    let count = text
        .parse::<usize>()
        .map_err(|_| format!("{flag} must be an integer"))?;
    if (!zero_allowed && count == 0) || count > MAX_ITERATIONS_PER_MODE {
        let lower = usize::from(!zero_allowed);
        return Err(format!(
            "{flag} must be within {lower}..={MAX_ITERATIONS_PER_MODE} for this bounded evidence runner"
        )
        .into());
    }
    Ok(count)
}

fn work_items(warmup: usize, samples: usize) -> Vec<WorkItem> {
    let mut work = Vec::with_capacity(warmup.saturating_add(samples).saturating_mul(2));
    let mut sequence = 1_u64;
    for mode in [HarnessMode::ResourceSetup, HarnessMode::SteadyReuse] {
        for (kind, count) in [
            (SampleKind::Warmup, warmup),
            (SampleKind::Measurement, samples),
        ] {
            for ordinal in 1..=count {
                work.push(WorkItem {
                    sequence,
                    mode,
                    kind,
                    ordinal,
                });
                sequence = sequence.saturating_add(1);
            }
        }
    }
    work
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = match evidence_directory_from_args() {
        Ok(args) => args,
        Err(error) => {
            // A malformed explicit `--evidence` without a following path has
            // no deterministic artifact destination; that preflight failure
            // remains stderr-only. All other parse failures use an explicit
            // path or the unique default below when it can be constructed.
            if let Some(evidence_directory) = evidence_directory_hint_from_args() {
                if write_preflight_failure(
                    &evidence_directory,
                    "ArgumentsInvalid",
                    &error.to_string(),
                    None,
                    None,
                )
                .is_err()
                {
                    return Err("performance preflight evidence could not be persisted".into());
                }
            }
            return Err(error);
        }
    };
    let source = match source_provenance_from_environment() {
        Ok(source) => source,
        Err(error) => {
            if write_preflight_failure(
                &args.evidence_directory,
                "SourceProvenanceInvalid",
                &error.to_string(),
                None,
                Some(&args),
            )
            .is_err()
            {
                return Err("performance preflight evidence could not be persisted".into());
            }
            return Err(error);
        }
    };
    if let Err(error) = fs::create_dir_all(&args.evidence_directory) {
        if write_preflight_failure(
            &args.evidence_directory,
            "EvidenceDirectoryUnavailable",
            &error.to_string(),
            Some(&source),
            Some(&args),
        )
        .is_err()
        {
            return Err("performance preflight evidence could not be persisted".into());
        }
        return Err(error.into());
    }
    println!(
        "Meridian direct UI performance evidence: {}",
        args.evidence_directory.display()
    );
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: "Meridian Direct UI Performance Evidence".to_owned(),
            initial_size: WindowSize::new(320, 180),
            resizable: false,
            visible: false,
            event_loop_mode: EventLoopMode::Wait,
        },
        PerformanceRunner {
            work: work_items(args.warmup, args.samples),
            args,
            failure: Arc::clone(&failure),
            source,
            rhi: None,
            renderer: None,
            case: None,
            profile: None,
            rhi_configuration: None,
            work_index: 0,
            reusable: None,
            pending: None,
            samples: Vec::new(),
            final_capture: None,
            disposition: TerminalDisposition::Completed,
            disposition_detail: None,
        },
    )?;
    if let Some(message) = failure
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
    {
        return Err(message.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn nearest_rank_is_deterministic_for_small_samples() {
        let values = [10, 20, 30, 40];
        assert_eq!(nearest_rank(&values, 50, 100), Some(20));
        assert_eq!(nearest_rank(&values, 95, 100), Some(40));
        assert_eq!(nearest_rank(&values, 99, 100), Some(40));
        assert_eq!(nearest_rank(&[], 50, 100), None);
    }

    #[test]
    fn bounded_iteration_parser_allows_zero_warmup_but_not_zero_samples() {
        assert_eq!(
            parse_iterations("--warmup", &OsString::from("0"), true).expect("warmup"),
            0
        );
        assert!(parse_iterations("--samples", &OsString::from("0"), false).is_err());
        assert!(parse_iterations("--samples", &OsString::from("65"), false).is_err());
    }

    #[test]
    fn work_items_keep_modes_and_frame_ids_sequential() {
        let work = work_items(1, 2);
        assert_eq!(work.len(), 6);
        assert_eq!(work[0].sequence, 1);
        assert_eq!(work[5].sequence, 6);
        assert_eq!(work[0].mode, HarnessMode::ResourceSetup);
        assert_eq!(work[3].mode, HarnessMode::SteadyReuse);
        assert_eq!(work[1].kind, SampleKind::Measurement);
    }

    #[test]
    fn no_timing_samples_is_never_reported_as_pass() {
        assert_eq!(gpu_timing_outcome(&[]), ("NotRun", "NoPassTimingSamples"));
        assert_eq!(
            aggregate_gpu_timing_outcome(&["Measured"]),
            ("Pass", "Measured")
        );
        assert_eq!(
            aggregate_gpu_timing_outcome(&["Measured", "UnsupportedPlatform"]),
            ("Inconclusive", "Mixed")
        );
        assert_eq!(
            aggregate_gpu_timing_outcome(&["UnsupportedCapability"]),
            ("UnsupportedCapability", "UnsupportedCapability")
        );
    }

    #[test]
    fn evidence_status_is_non_promoting_for_declared_source_provenance() {
        assert_eq!(
            performance_evidence_status(TerminalDisposition::Completed),
            "Inconclusive"
        );
        assert_eq!(
            performance_evidence_status(TerminalDisposition::Inconclusive),
            "Inconclusive"
        );
        assert_eq!(
            performance_evidence_status(TerminalDisposition::NotRun),
            "NotRun"
        );
    }

    #[test]
    fn source_provenance_requires_an_explicit_state() {
        let clean = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            source_provenance_from_values("clean-commit", clean)
                .expect("clean commit")
                .state,
            SourceState::CleanCommit
        );
        assert_eq!(
            source_provenance_from_values("working-tree", "local-ui-direct-performance")
                .expect("working tree")
                .state,
            SourceState::WorkingTree
        );
        assert!(source_provenance_from_values("clean-commit", "local-ui").is_err());
        assert!(source_provenance_from_values("working-tree", "/Users/example").is_err());
        assert!(source_provenance_from_values("unknown", clean).is_err());
    }

    #[test]
    fn failure_detail_is_bounded_and_path_safe() {
        assert_eq!(
            sanitize_failure_detail("failed to map capture"),
            "failed to map capture"
        );
        assert_eq!(
            sanitize_failure_detail("failed at /private/tmp/example"),
            "PathOrControlCharacterRedacted"
        );
        assert!(
            sanitize_failure_detail(&"x".repeat(MAX_FAILURE_DETAIL_CHARS + 1)).len()
                <= MAX_FAILURE_DETAIL_CHARS + 3
        );
    }

    #[test]
    fn preflight_failure_keeps_output_paths_relative() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("meridian-ui-perf-preflight-{nonce}"));
        write_preflight_failure(&directory, "ArgumentsInvalid", "bad --samples", None, None)
            .expect("preflight artifact");
        let contents = fs::read_to_string(directory.join("failure.json")).expect("failure json");
        assert!(contents.contains("\"status\": \"Fail\""));
        assert!(contents.contains("\"code\": \"PreflightFailure\""));
        assert!(contents.contains("\"cause\": \"ArgumentsInvalid\""));
        assert!(!contents.contains("\"status\": \"PreflightFailure\""));
        assert!(contents.contains("\"work_package_id\": \"WP-UI-005\""));
        assert!(contents.contains("\"runner_package\": \"meridian-benchmark\""));
        assert!(!contents.contains(&directory.display().to_string()));
        fs::remove_dir_all(directory).expect("remove preflight artifact");
    }

    #[test]
    fn environment_marks_unembedded_identity_and_measurement_scope() {
        let args = RunnerArgs {
            evidence_directory: PathBuf::from("target/meridian-evidence/test"),
            warmup: 3,
            samples: 10,
        };
        let environment = evidence_environment(Some(&args), None);
        assert_eq!(environment.requirement_ids, ["REQ-UI-001", "REQ-UI-002"]);
        assert_eq!(environment.work_package_id, "WP-UI-005");
        assert_eq!(environment.build_identity.availability, "NotAvailable");
        assert_eq!(environment.build_hash.availability, "NotAvailable");
        assert_eq!(
            environment.execution_scope.warmup_iterations,
            Some(args.warmup)
        );
        assert!(
            !environment
                .execution_scope
                .warmup_scope
                .included_in_percentile_summaries
        );
        assert!(
            !environment
                .execution_scope
                .warmup_scope
                .included_in_gpu_timing_aggregate
        );
        assert_eq!(
            environment.memory_telemetry.actual_backend_allocations,
            "UnsupportedCapability"
        );
    }

    #[test]
    fn absent_driver_metadata_is_explicit_and_explained() {
        let (driver, driver_availability) = explicit_profile_metadata("\t", DRIVER_UNAVAILABLE);
        assert_eq!(driver, "NotAvailable");
        assert_eq!(driver_availability.availability, "NotAvailable");
        assert_eq!(driver_availability.reason, DRIVER_UNAVAILABLE);

        let (driver_info, driver_info_availability) =
            explicit_profile_metadata("Metal driver 1.2", DRIVER_INFO_UNAVAILABLE);
        assert_eq!(driver_info, "Metal driver 1.2");
        assert_eq!(driver_info_availability.availability, "Available");
        assert_eq!(driver_info_availability.reason, PROFILE_METADATA_AVAILABLE);
    }

    #[test]
    fn typed_direct_renderer_capability_failure_stays_typed() {
        assert_eq!(
            ui_direct_error_evidence_outcome(&UiDirectRendererError::UnsupportedSurfaceColorSpace),
            ("UnsupportedCapability", "UnsupportedSurfaceColorSpace")
        );
    }
}
