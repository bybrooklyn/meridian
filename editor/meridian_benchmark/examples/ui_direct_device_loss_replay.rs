//! Controlled device-destruction recovery runner for Meridian's direct UI path.
//!
//! This hidden native-window runner destroys the actual backend device after a
//! bounded offscreen baseline capture. It proves controlled cache recovery and
//! exact replay from the immutable display-list corpus. It does not represent
//! a hardware, driver, power, or spontaneous device-loss qualification.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use meridian_benchmark::{
    has_multiple_pixel_values, write_capture_png, write_capture_rgba, write_evidence_json,
};
use meridian_core::FrameId;
use meridian_platform::{
    run, EventLoopMode, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent,
    PlatformWindow, WindowSize,
};
use meridian_renderer::{
    compare_ui_direct_rgba8_exact, ui_direct_qualification_cases, UiDirectFramePlan,
    UiDirectGpuFrame, UiDirectGpuRenderer, UiDirectQualificationCase, UiDirectRendererError,
    UiDirectRendererRecoveryAction, UiDirectRgba8Image, UI_DIRECT_QUALIFICATION_SCHEMA,
};
use meridian_rhi::{
    AdapterKind, Backend, CapabilityStatus, CaptureFailure, CaptureOutcome, CaptureRequest,
    CaptureSource, CapturedFrame, CapturedPixelFormat, ClearColor, DeviceLossReason, GpuFeature,
    MemoryClass, PowerPreference, PresentPolicy, Rhi, RhiConfig, RhiError, RhiErrorKind,
    RhiRenderIdentity,
};
use serde::Serialize;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const REPLAY_CLEAR: ClearColor = ClearColor::new(0.0, 0.0, 0.0, 1.0);
const DEVICE_LOSS_REPLAY_SCHEMA: &str = "meridian.ui-direct-device-loss-replay/v1";
const STANDARD_CASE_ID: &str = "standard-1x";
const REQUIREMENT_IDS: [&str; 2] = ["REQ-UI-001", "REQ-UI-002"];
const WORK_PACKAGE_ID: &str = "WP-UI-005";
const MILESTONE_ID: &str = "MS-03";
const RESEARCH_GATE_ID: &str = "RG-UI-001";
const MAX_FAILURE_DETAIL_CHARS: usize = 240;
const PROFILE_METADATA_AVAILABLE: &str =
    "The active RHI capability profile supplied this metadata field.";
const DRIVER_UNAVAILABLE: &str =
    "The active RHI capability profile did not supply a driver identifier.";
const DRIVER_INFO_UNAVAILABLE: &str =
    "The active RHI capability profile did not supply driver-information text.";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum EvidenceStatus {
    Pass,
    Inconclusive,
    NotRun,
    UnsupportedPlatform,
    UnsupportedCapability,
    Fail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum FailureCode {
    ArgumentsInvalid,
    SourceProvenanceInvalid,
    EvidenceDirectoryUnavailable,
    WindowUnavailable,
    RhiAdapterUnavailable,
    RhiSurfaceUnsupported,
    RhiDeviceUnavailable,
    RhiCapabilityUnavailable,
    RhiInitialization,
    RhiDeviceLost,
    DirectRendererUnsupported,
    OffscreenCaptureCopySourceUnsupported,
    DirectRendererFailure,
    CaptureUnsupportedCapability,
    CaptureInconclusive,
    CaptureTimedOut,
    CaptureInvalid,
    ArtifactWrite,
    DestroyedCallbackUnavailable,
    DestroyedCallbackUnexpected,
    OldSubmissionExpectation,
    DeviceRebuild,
    StaleIdentityExpectation,
    RecoveryExpectation,
    WindowClosed,
    EvidenceWrite,
    RunnerFailure,
}

#[derive(Clone, Debug)]
struct RunnerFailure {
    status: EvidenceStatus,
    code: FailureCode,
    detail: String,
}

impl RunnerFailure {
    fn new(status: EvidenceStatus, code: FailureCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            code,
            detail: detail.into(),
        }
    }

    fn from_rhi(error: &RhiError) -> Self {
        Self::from_rhi_kind(error.kind(), error.to_string())
    }

    fn from_rhi_kind(kind: RhiErrorKind, detail: impl Into<String>) -> Self {
        let (status, code) = match kind {
            RhiErrorKind::AdapterUnavailable => {
                (EvidenceStatus::NotRun, FailureCode::RhiAdapterUnavailable)
            }
            RhiErrorKind::SurfaceCreation | RhiErrorKind::SurfaceUnsupported => (
                EvidenceStatus::UnsupportedPlatform,
                FailureCode::RhiSurfaceUnsupported,
            ),
            RhiErrorKind::DeviceCreation => (
                EvidenceStatus::UnsupportedPlatform,
                FailureCode::RhiDeviceUnavailable,
            ),
            RhiErrorKind::CaptureTargetUnsupported => (
                EvidenceStatus::UnsupportedCapability,
                FailureCode::RhiCapabilityUnavailable,
            ),
            RhiErrorKind::DeviceLost => (EvidenceStatus::Inconclusive, FailureCode::RhiDeviceLost),
            _ => (EvidenceStatus::Fail, FailureCode::RhiInitialization),
        };
        Self::new(status, code, detail)
    }

    fn from_ui_direct(error: &UiDirectRendererError) -> Self {
        if matches!(
            error,
            UiDirectRendererError::OffscreenCaptureUnsupported { .. }
        ) {
            return Self::new(
                EvidenceStatus::UnsupportedCapability,
                FailureCode::OffscreenCaptureCopySourceUnsupported,
                error.to_string(),
            );
        }
        if let Some(kind) = error.rhi_kind() {
            return Self::from_rhi_kind(kind, error.to_string());
        }
        let (status, code) = match error {
            UiDirectRendererError::UnsupportedSurfaceColorSpace => (
                EvidenceStatus::UnsupportedCapability,
                FailureCode::DirectRendererUnsupported,
            ),
            _ => (EvidenceStatus::Fail, FailureCode::DirectRendererFailure),
        };
        Self::new(status, code, error.to_string())
    }

    fn from_capture_outcome(outcome: &CaptureOutcome) -> Self {
        match outcome {
            CaptureOutcome::UnsupportedCapability { failure, .. } => Self::new(
                EvidenceStatus::UnsupportedCapability,
                FailureCode::CaptureUnsupportedCapability,
                format!("offscreen capture is unsupported: {failure:?}"),
            ),
            CaptureOutcome::Inconclusive { failure, .. } => Self::new(
                EvidenceStatus::Inconclusive,
                match failure {
                    CaptureFailure::DeviceLost => FailureCode::RhiDeviceLost,
                    _ => FailureCode::CaptureInconclusive,
                },
                format!("offscreen capture is inconclusive: {failure:?}"),
            ),
            CaptureOutcome::Captured(_) => Self::new(
                EvidenceStatus::Fail,
                FailureCode::CaptureInvalid,
                "captured outcome was classified as a failure",
            ),
        }
    }

    fn from_error(error: &(dyn Error + 'static)) -> Self {
        if let Some(failure) = error.downcast_ref::<Self>() {
            return failure.clone();
        }
        if let Some(error) = error.downcast_ref::<RhiError>() {
            return Self::from_rhi(error);
        }
        if let Some(error) = error.downcast_ref::<UiDirectRendererError>() {
            return Self::from_ui_direct(error);
        }
        Self::new(
            EvidenceStatus::Fail,
            FailureCode::RunnerFailure,
            error.to_string(),
        )
    }
}

impl std::fmt::Display for RunnerFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:?}: {:?}: {}",
            self.status, self.code, self.detail
        )
    }
}

impl Error for RunnerFailure {}

#[derive(Clone, Debug, Serialize)]
struct AvailabilityReport {
    availability: &'static str,
    value: Option<String>,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ProfileMetadataAvailability {
    availability: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ExecutionScopeReport {
    invocations: u8,
    warmup_iterations: u8,
    cache_scope: &'static str,
    cross_run_cache_state: &'static str,
    repetition_scope: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct MemoryTelemetryReport {
    actual_backend_allocations: &'static str,
    vram_usage: &'static str,
    driver_residency: &'static str,
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
struct RhiConfigReport {
    power_preference: &'static str,
    preferred_backend: Option<&'static str>,
    allow_software_adapter: bool,
    present_policy: &'static str,
    desired_maximum_frame_latency: u32,
    enable_timestamps: bool,
}

#[derive(Clone, Debug, Serialize)]
struct RhiProfileReport {
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
    configuration: RhiConfigReport,
}

#[derive(Clone, Debug, Serialize)]
struct RenderIdentityReport {
    device_generation: u64,
    surface_generation: u64,
    surface_format: String,
    surface_width: u32,
    surface_height: u32,
    surface_configured: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReplayStage {
    BaselineCapture,
    AwaitDestroyedCallback,
    RecoveredCapture,
    Completed,
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
}

#[derive(Clone, Debug)]
struct SourceProvenance {
    checkpoint: String,
    state: SourceState,
}

#[derive(Clone, Debug, Serialize)]
struct SourceProvenanceReport {
    declaration: &'static str,
    verification: &'static str,
    checkpoint: String,
    declared_state: &'static str,
    promotion_eligibility: &'static str,
}

impl SourceProvenance {
    fn report(&self) -> SourceProvenanceReport {
        SourceProvenanceReport {
            declaration: "CallerDeclared",
            verification: "CallerDeclaredNotVerified",
            checkpoint: self.checkpoint.clone(),
            declared_state: self.state.as_str(),
            promotion_eligibility: self.state.promotion_eligibility(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CaptureReport {
    frame_id: u64,
    capture_id: u64,
    width: u32,
    height: u32,
    pixel_hash: String,
    png_hash: String,
    png: String,
    png_metadata: String,
    rgba: String,
}

#[derive(Clone, Debug)]
struct BaselineCapture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    report: CaptureReport,
}

#[derive(Clone, Debug, Serialize)]
struct ExactReplayReport {
    compared_pixel_count: u64,
    differing_pixel_count: u64,
    maximum_channel_delta: u8,
}

#[derive(Clone, Debug, Serialize)]
struct DeviceLossReplayReport {
    schema: &'static str,
    runner_status: EvidenceStatus,
    evidence_status: EvidenceStatus,
    source_checkpoint: String,
    source_state: &'static str,
    source_provenance_verification: &'static str,
    source_provenance: SourceProvenanceReport,
    evidence_scope: &'static str,
    promotion_eligibility: &'static str,
    evidence_directory: String,
    environment: EvidenceEnvironmentReport,
    baseline_profile: RhiProfileReport,
    recovered_profile: RhiProfileReport,
    old_render_identity: RenderIdentityReport,
    recovered_render_identity: RenderIdentityReport,
    qualification_schema: &'static str,
    case_id: &'static str,
    corpus_hash: String,
    controlled_device_destruction: bool,
    device_loss_reason: &'static str,
    old_submission_rhi_error_kind: &'static str,
    stale_plan_gpu_rejection: &'static str,
    recovery_action: &'static str,
    preserved_revision: u64,
    dropped_cache_count: u32,
    baseline: CaptureReport,
    recovered: CaptureReport,
    exact_replay: ExactReplayReport,
    limits: [&'static str; 3],
}

#[derive(Clone, Debug, Serialize)]
struct DeviceLossReplayFailureReport {
    schema: &'static str,
    runner_status: EvidenceStatus,
    evidence_status: EvidenceStatus,
    failure_code: FailureCode,
    source_checkpoint: Option<String>,
    source_state: Option<&'static str>,
    source_provenance_verification: &'static str,
    source_provenance: Option<SourceProvenanceReport>,
    evidence_directory: &'static str,
    stage: &'static str,
    environment: EvidenceEnvironmentReport,
    baseline_profile: Option<RhiProfileReport>,
    detail: String,
    limits: [&'static str; 2],
}

fn unavailable(reason: &'static str) -> AvailabilityReport {
    AvailabilityReport {
        availability: "NotAvailable",
        value: None,
        reason,
    }
}

fn evidence_environment(capability_profile_available: bool) -> EvidenceEnvironmentReport {
    let capability_profile = if capability_profile_available {
        AvailabilityReport {
            availability: "Available",
            value: Some("baseline_profile and recovered_profile".to_owned()),
            reason: "The reports contain only public RHI capability descriptors; they do not prove platform qualification.",
        }
    } else {
        unavailable("RHI initialization did not complete, so no capability profile was observed.")
    };
    EvidenceEnvironmentReport {
        requirement_ids: REQUIREMENT_IDS,
        work_package_id: WORK_PACKAGE_ID,
        milestone_id: MILESTONE_ID,
        research_gate_id: RESEARCH_GATE_ID,
        runner_package: "meridian-benchmark",
        runner_package_version: env!("CARGO_PKG_VERSION"),
        build_identity: unavailable(
            "This standalone runner does not embed a generated build identity or CI run identity.",
        ),
        build_hash: unavailable(
            "This standalone runner does not embed a source-tree or artifact hash.",
        ),
        toolchain_profile: unavailable(
            "This runner does not embed a generated Rust toolchain profile.",
        ),
        dependency_profile: unavailable(
            "This runner does not embed a Cargo.lock or dependency-graph hash.",
        ),
        capability_profile,
        execution_scope: ExecutionScopeReport {
            invocations: 1,
            warmup_iterations: 0,
            cache_scope: "One renderer cache is created for the baseline, then intentionally dropped and rebuilt after controlled device destruction.",
            cross_run_cache_state: "NotMeasured",
            repetition_scope: "One bounded baseline-to-recovery replay only; no repeated-run, startup-cache, or persistent-cache evidence is collected.",
        },
        memory_telemetry: MemoryTelemetryReport {
            actual_backend_allocations: "NotAvailable",
            vram_usage: "NotAvailable",
            driver_residency: "NotAvailable",
            reason: "The public direct-UI/RHI contracts expose memory class and planned payloads, not allocator, VRAM, or driver-residency telemetry.",
        },
    }
}

const fn device_loss_evidence_status() -> EvidenceStatus {
    // Even a clean-commit declaration is not independently verified by this
    // standalone runner, and the corpus remains local/offscreen structural
    // evidence. The controlled replay result therefore never promotes itself.
    EvidenceStatus::Inconclusive
}

struct DeviceLossReplayRunner {
    evidence_directory: PathBuf,
    source: SourceProvenance,
    config: RhiConfig,
    failure: Arc<Mutex<Option<String>>>,
    case: UiDirectQualificationCase,
    stage: ReplayStage,
    rhi: Option<Rhi>,
    renderer: Option<UiDirectGpuRenderer>,
    plan: Option<UiDirectFramePlan>,
    gpu: Option<UiDirectGpuFrame>,
    capture_deadline: Option<Instant>,
    baseline: Option<BaselineCapture>,
    baseline_profile: Option<RhiProfileReport>,
    recovered_profile: Option<RhiProfileReport>,
    old_render_identity: Option<RenderIdentityReport>,
    recovered_render_identity: Option<RenderIdentityReport>,
    dropped_cache_count: Option<u32>,
}

impl DeviceLossReplayRunner {
    fn fail(&mut self, failure: RunnerFailure, context: &mut PlatformContext<'_>) {
        let terminal_failure = match self.write_failure_report(&failure) {
            Ok(()) => failure,
            Err(error) => RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::EvidenceWrite,
                format!(
                    "failed to write the controlled recovery failure artifact: {}",
                    sanitize_failure_detail(&error.to_string())
                ),
            ),
        };
        let mut failure = self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failure.is_none() {
            *failure = Some(terminal_failure.to_string());
        }
        context.exit();
    }

    fn fail_from_error(
        &mut self,
        error: &(dyn Error + 'static),
        context: &mut PlatformContext<'_>,
    ) {
        self.fail(RunnerFailure::from_error(error), context);
    }

    const fn failure_stage(&self) -> &'static str {
        match self.stage {
            ReplayStage::BaselineCapture => "baseline-capture",
            ReplayStage::AwaitDestroyedCallback => "destroyed-callback",
            ReplayStage::RecoveredCapture => "recovered-capture",
            ReplayStage::Completed => "final-report",
        }
    }

    fn write_failure_report(&self, failure: &RunnerFailure) -> Result<(), Box<dyn Error>> {
        let report = DeviceLossReplayFailureReport {
            schema: DEVICE_LOSS_REPLAY_SCHEMA,
            runner_status: failure.status,
            evidence_status: failure.status,
            failure_code: failure.code,
            source_checkpoint: Some(self.source.checkpoint.clone()),
            source_state: Some(self.source.state.as_str()),
            source_provenance_verification: "CallerDeclaredNotVerified",
            source_provenance: Some(self.source.report()),
            evidence_directory: ".",
            stage: self.failure_stage(),
            environment: evidence_environment(self.baseline_profile.is_some()),
            baseline_profile: self.baseline_profile.clone(),
            detail: sanitize_failure_detail(&failure.detail),
            limits: [
                "This failure artifact records a controlled recovery-runner failure only.",
                "It cannot establish hardware, driver, power, or spontaneous device-loss behavior.",
            ],
        };
        write_evidence_json(
            self.evidence_directory
                .join("device-loss-replay-failure.json"),
            &report,
        )?;
        Ok(())
    }

    fn initialize(
        &mut self,
        window: PlatformWindow,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let rhi = Rhi::new(window, self.config)?;
        self.baseline_profile = Some(rhi_profile(&rhi, self.config));
        self.renderer = Some(UiDirectGpuRenderer::new(rhi.render_identity()));
        self.rhi = Some(rhi);
        self.stage = ReplayStage::BaselineCapture;
        self.begin_capture(FrameId::new(1), context)
    }

    fn begin_capture(
        &mut self,
        frame_id: FrameId,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let plan = self
            .renderer
            .as_mut()
            .ok_or_else(|| "device-loss replay renderer is unavailable".to_owned())?
            .prepare_frame(self.case.prepare_request())?;
        let cache_key = plan.cache_key();
        let max_bytes = capture_byte_count(cache_key.surface_width, cache_key.surface_height)?;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?;
        let gpu = plan.upload_gpu_frame(rhi)?;
        rhi.request_capture(CaptureRequest::new(
            frame_id,
            cache_key.surface_width,
            cache_key.surface_height,
            max_bytes,
        ))?;
        gpu.submit_offscreen_capture(rhi, &plan, REPLAY_CLEAR)?;
        self.plan = Some(plan);
        self.gpu = Some(gpu);
        self.capture_deadline = Some(Instant::now() + CAPTURE_TIMEOUT);
        context.request_redraw();
        Ok(())
    }

    fn take_ready_capture(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<Option<CapturedFrame>, Box<dyn Error>> {
        let capture = self
            .rhi
            .as_mut()
            .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?
            .take_capture();
        let Some(capture) = capture else {
            if self
                .capture_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err(RunnerFailure::new(
                    EvidenceStatus::Inconclusive,
                    FailureCode::CaptureTimedOut,
                    "device-loss replay pixel readback timed out",
                )
                .into());
            }
            context.request_redraw();
            return Ok(None);
        };
        match capture {
            CaptureOutcome::Captured(frame) => Ok(Some(frame)),
            outcome => Err(RunnerFailure::from_capture_outcome(&outcome).into()),
        }
    }

    fn validate_capture(
        &self,
        frame: &CapturedFrame,
        expected_frame_id: FrameId,
    ) -> Result<(), Box<dyn Error>> {
        let cache_key = self
            .plan
            .as_ref()
            .ok_or_else(|| "device-loss replay plan is unavailable".to_owned())?
            .cache_key();
        let expected_bytes = usize::try_from(capture_byte_count(
            cache_key.surface_width,
            cache_key.surface_height,
        )?)?;
        if frame.frame_id != expected_frame_id
            || frame.width != cache_key.surface_width
            || frame.height != cache_key.surface_height
            || frame.format != CapturedPixelFormat::Rgba8Srgb
            || frame.source != CaptureSource::Offscreen
            || frame.surface_outcome.is_some()
            || frame.pixels.len() != expected_bytes
            || !has_multiple_pixel_values(frame)
        {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::CaptureInvalid,
                format!("device-loss replay capture metadata is invalid: {frame:?}"),
            )
            .into());
        }
        Ok(())
    }

    fn capture_report(
        &self,
        label: &str,
        frame: &CapturedFrame,
    ) -> Result<CaptureReport, Box<dyn Error>> {
        let png = write_capture_png(self.evidence_directory.join(format!("{label}.png")), frame)
            .map_err(|error| {
                RunnerFailure::new(
                    EvidenceStatus::Fail,
                    FailureCode::ArtifactWrite,
                    error.to_string(),
                )
            })?;
        let rgba = write_capture_rgba(self.evidence_directory.join(format!("{label}.rgba")), frame)
            .map_err(|error| {
                RunnerFailure::new(
                    EvidenceStatus::Fail,
                    FailureCode::ArtifactWrite,
                    error.to_string(),
                )
            })?;
        if rgba.pixel_hash != png.metadata.pixel_hash {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::ArtifactWrite,
                "device-loss replay raw and PNG capture hashes differ",
            )
            .into());
        }
        Ok(CaptureReport {
            frame_id: frame.frame_id.get(),
            capture_id: frame.capture_id.get(),
            width: frame.width,
            height: frame.height,
            pixel_hash: png.metadata.pixel_hash,
            png_hash: png.metadata.png_hash,
            png: format!("{label}.png"),
            png_metadata: format!("{label}.png.json"),
            rgba: format!("{label}.rgba"),
        })
    }

    fn finish_baseline_capture(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let Some(frame) = self.take_ready_capture(context)? else {
            return Ok(());
        };
        self.validate_capture(&frame, FrameId::new(1))?;
        let report = self.capture_report("baseline", &frame)?;
        self.baseline = Some(BaselineCapture {
            width: frame.width,
            height: frame.height,
            pixels: frame.pixels,
            report,
        });
        self.rhi
            .as_mut()
            .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?
            .destroy_device_for_fault_injection();
        self.stage = ReplayStage::AwaitDestroyedCallback;
        self.capture_deadline = Some(Instant::now() + CAPTURE_TIMEOUT);
        context.request_redraw();
        Ok(())
    }

    fn poll_destroyed_callback(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let loss = {
            let rhi = self
                .rhi
                .as_mut()
                .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?;
            rhi.poll_captures();
            rhi.device_loss()
        };
        let Some(loss) = loss else {
            if self
                .capture_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err(RunnerFailure::new(
                    EvidenceStatus::Inconclusive,
                    FailureCode::DestroyedCallbackUnavailable,
                    "controlled device destruction did not report a device-loss callback",
                )
                .into());
            }
            context.request_redraw();
            return Ok(());
        };
        if loss.reason != DeviceLossReason::Destroyed {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::DestroyedCallbackUnexpected,
                format!(
                    "controlled device destruction reported unexpected reason {:?}",
                    loss.reason
                ),
            )
            .into());
        }
        self.require_old_submission_device_lost()?;
        self.rebuild_and_replay(context)
    }

    fn require_old_submission_device_lost(&mut self) -> Result<(), Box<dyn Error>> {
        let plan = self
            .plan
            .as_ref()
            .ok_or_else(|| "device-loss replay old plan is unavailable".to_owned())?;
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| "device-loss replay old GPU frame is unavailable".to_owned())?;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?;
        match gpu.submit_offscreen_capture(rhi, plan, REPLAY_CLEAR) {
            Err(error) if error.rhi_kind() == Some(RhiErrorKind::DeviceLost) => Ok(()),
            Err(error) => Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::OldSubmissionExpectation,
                format!("old direct UI GPU submission returned {error}, not typed DeviceLost"),
            )
            .into()),
            Ok(()) => Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::OldSubmissionExpectation,
                "old direct UI GPU submission unexpectedly succeeded after destruction",
            )
            .into()),
        }
    }

    fn rebuild_and_replay(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let old_rhi = self
            .rhi
            .take()
            .ok_or_else(|| "device-loss replay RHI is unavailable".to_owned())?;
        let old_identity = old_rhi.render_identity();
        self.old_render_identity = Some(render_identity_report(&old_identity));
        let old_plan = self
            .plan
            .take()
            .ok_or_else(|| "device-loss replay old plan is unavailable".to_owned())?;
        let old_gpu = self
            .gpu
            .take()
            .ok_or_else(|| "device-loss replay old GPU frame is unavailable".to_owned())?;
        let mut recovered_rhi = old_rhi.rebuild_device().map_err(|error| {
            let mut failure = RunnerFailure::from_rhi(&error);
            failure.code = FailureCode::DeviceRebuild;
            failure
        })?;
        let recovered_identity = recovered_rhi.render_identity();
        self.recovered_profile = Some(rhi_profile(&recovered_rhi, self.config));
        self.recovered_render_identity = Some(render_identity_report(&recovered_identity));
        if old_identity.device_generation == recovered_identity.device_generation {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::DeviceRebuild,
                "RHI rebuild retained its destroyed device generation",
            )
            .into());
        }
        match old_gpu.submit_offscreen_capture(&mut recovered_rhi, &old_plan, REPLAY_CLEAR) {
            Err(UiDirectRendererError::StaleRhiIdentity { .. }) => {}
            Err(error) => return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::StaleIdentityExpectation,
                format!(
                    "old direct UI plan/GPU frame returned {error}, not StaleRhiIdentity after rebuild"
                ),
            )
            .into()),
            Ok(()) => return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::StaleIdentityExpectation,
                "old direct UI plan/GPU frame unexpectedly submitted after RHI rebuild",
            )
            .into()),
        }
        let recovery = self
            .renderer
            .as_mut()
            .ok_or_else(|| "device-loss replay renderer is unavailable".to_owned())?
            .recover_identity(recovered_identity, self.case.display_revision);
        if recovery.action != UiDirectRendererRecoveryAction::RebuildDeviceCaches
            || recovery.preserved_revision != self.case.display_revision
        {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::RecoveryExpectation,
                format!(
                    "direct UI recovery did not rebuild device caches while preserving revision {}: {:?}",
                    self.case.display_revision, recovery
                ),
            )
            .into());
        }
        self.dropped_cache_count = Some(recovery.dropped_cache_count);
        self.rhi = Some(recovered_rhi);
        self.stage = ReplayStage::RecoveredCapture;
        self.begin_capture(FrameId::new(2), context)
    }

    fn finish_recovered_capture(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let Some(frame) = self.take_ready_capture(context)? else {
            return Ok(());
        };
        self.validate_capture(&frame, FrameId::new(2))?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or_else(|| "device-loss replay baseline is unavailable".to_owned())?;
        let comparison = compare_ui_direct_rgba8_exact(
            UiDirectRgba8Image::new(baseline.width, baseline.height, &baseline.pixels),
            UiDirectRgba8Image::new(frame.width, frame.height, &frame.pixels),
        )?;
        if !comparison.passed() {
            return Err(RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::RecoveryExpectation,
                format!(
                    "recovered direct UI pixels differ from the baseline: {} pixels; max channel delta {}; first difference {:?}",
                    comparison.differing_pixel_count,
                    comparison.maximum_channel_delta,
                    comparison.first_difference
                ),
            )
            .into());
        }
        let recovered = self.capture_report("recovered", &frame)?;
        let report = DeviceLossReplayReport {
            schema: DEVICE_LOSS_REPLAY_SCHEMA,
            runner_status: EvidenceStatus::Pass,
            evidence_status: device_loss_evidence_status(),
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            source_provenance_verification: "CallerDeclaredNotVerified",
            source_provenance: self.source.report(),
            evidence_scope: "LocalStructuralEvidence",
            promotion_eligibility: self.source.state.promotion_eligibility(),
            evidence_directory: ".".to_owned(),
            environment: evidence_environment(true),
            baseline_profile: self
                .baseline_profile
                .clone()
                .ok_or_else(|| "device-loss replay baseline profile is unavailable".to_owned())?,
            recovered_profile: self
                .recovered_profile
                .clone()
                .ok_or_else(|| "device-loss replay recovered profile is unavailable".to_owned())?,
            old_render_identity: self
                .old_render_identity
                .clone()
                .ok_or_else(|| "device-loss replay old identity is unavailable".to_owned())?,
            recovered_render_identity: self
                .recovered_render_identity
                .clone()
                .ok_or_else(|| "device-loss replay recovered identity is unavailable".to_owned())?,
            qualification_schema: UI_DIRECT_QUALIFICATION_SCHEMA,
            case_id: self.case.id,
            corpus_hash: self.case.corpus_hash(),
            controlled_device_destruction: true,
            device_loss_reason: "Destroyed",
            old_submission_rhi_error_kind: "DeviceLost",
            stale_plan_gpu_rejection: "StaleRhiIdentity",
            recovery_action: "RebuildDeviceCaches",
            preserved_revision: self.case.display_revision,
            dropped_cache_count: self
                .dropped_cache_count
                .ok_or_else(|| "device-loss replay recovery report is unavailable".to_owned())?,
            baseline: baseline.report.clone(),
            recovered,
            exact_replay: ExactReplayReport {
                compared_pixel_count: comparison.compared_pixel_count,
                differing_pixel_count: comparison.differing_pixel_count,
                maximum_channel_delta: comparison.maximum_channel_delta,
            },
            limits: [
                "Controlled wgpu device destruction only; hardware, driver, power, and spontaneous loss were not induced.",
                "Hidden offscreen RGBA equality is renderer recovery evidence, not presented visual review or cross-platform qualification.",
                "This fixed standard-1x corpus does not establish latency, memory, cache-budget, screen-reader, build-identity, or dependency-profile evidence.",
            ],
        };
        write_evidence_json(
            self.evidence_directory.join("device-loss-replay.json"),
            &report,
        )
        .map_err(|error| {
            RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::ArtifactWrite,
                error.to_string(),
            )
        })?;
        println!(
            "Meridian direct UI controlled device-loss replay captured baseline and recovery at {}",
            self.evidence_directory.display()
        );
        self.stage = ReplayStage::Completed;
        self.capture_deadline = None;
        self.plan = None;
        self.gpu = None;
        context.exit();
        Ok(())
    }
}

impl PlatformApplication for DeviceLossReplayRunner {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail(
                        RunnerFailure::new(
                            EvidenceStatus::NotRun,
                            FailureCode::WindowUnavailable,
                            "device-loss replay window was not available",
                        ),
                        context,
                    );
                    return;
                };
                if let Err(error) = self.initialize(window, context) {
                    self.fail_from_error(error.as_ref(), context);
                }
            }
            PlatformEvent::RedrawRequested => {
                let result = match self.stage {
                    ReplayStage::BaselineCapture => self.finish_baseline_capture(context),
                    ReplayStage::AwaitDestroyedCallback => self.poll_destroyed_callback(context),
                    ReplayStage::RecoveredCapture => self.finish_recovered_capture(context),
                    ReplayStage::Completed => Ok(()),
                };
                if let Err(error) = result {
                    self.fail_from_error(error.as_ref(), context);
                }
            }
            PlatformEvent::CloseRequested if self.stage != ReplayStage::Completed => {
                self.fail(
                    RunnerFailure::new(
                        EvidenceStatus::Inconclusive,
                        FailureCode::WindowClosed,
                        "device-loss replay was closed before completing evidence",
                    ),
                    context,
                );
            }
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn capture_byte_count(width: u32, height: u32) -> Result<u64, Box<dyn Error>> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "device-loss replay capture byte count overflowed".into())
}

fn explicit_profile_metadata(
    value: &str,
    unavailable_reason: &'static str,
) -> (String, ProfileMetadataAvailability) {
    let value = value.trim();
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
        value.to_owned(),
        ProfileMetadataAvailability {
            availability: "Available",
            reason: PROFILE_METADATA_AVAILABLE,
        },
    )
}

fn rhi_profile(rhi: &Rhi, config: RhiConfig) -> RhiProfileReport {
    let capabilities = rhi.capabilities();
    let (driver, driver_availability) =
        explicit_profile_metadata(&capabilities.driver, DRIVER_UNAVAILABLE);
    let (driver_info, driver_info_availability) =
        explicit_profile_metadata(&capabilities.driver_info, DRIVER_INFO_UNAVAILABLE);
    RhiProfileReport {
        backend: backend_name(capabilities.backend).to_owned(),
        adapter_name: capabilities.adapter_name.clone(),
        driver,
        driver_availability,
        driver_info,
        driver_info_availability,
        vendor_id: capabilities.vendor_id,
        device_id: capabilities.device_id,
        adapter_kind: adapter_kind_name(capabilities.adapter_kind).to_owned(),
        memory_class: memory_class_name(capabilities.memory_class).to_owned(),
        timestamp_query_capability: capability_status_name(capabilities.timestamp_queries)
            .to_owned(),
        hdr_surface_formats_capability: capability_status_name(capabilities.hdr_surface_formats)
            .to_owned(),
        max_sampled_textures_per_shader_stage: capabilities.max_sampled_textures_per_shader_stage,
        enabled_features: capabilities
            .features
            .iter()
            .copied()
            .map(gpu_feature_name)
            .map(str::to_owned)
            .collect(),
        missing_features: known_gpu_features()
            .into_iter()
            .filter(|feature| !capabilities.features.contains(feature))
            .map(gpu_feature_name)
            .map(str::to_owned)
            .collect(),
        surface_format: rhi.surface_format().name,
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        configuration: RhiConfigReport {
            power_preference: power_preference_name(config.power_preference),
            preferred_backend: config.preferred_backend.map(backend_name),
            allow_software_adapter: config.allow_software_adapter,
            present_policy: present_policy_name(config.present_policy),
            desired_maximum_frame_latency: config.desired_maximum_frame_latency,
            enable_timestamps: config.enable_timestamps,
        },
    }
}

fn render_identity_report(identity: &RhiRenderIdentity) -> RenderIdentityReport {
    RenderIdentityReport {
        device_generation: identity.device_generation,
        surface_generation: identity.surface_generation,
        surface_format: identity.surface_format.name.clone(),
        surface_width: identity.surface_size.width,
        surface_height: identity.surface_size.height,
        surface_configured: identity.surface_configured,
    }
}

const fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Noop => "Noop",
        Backend::Vulkan => "Vulkan",
        Backend::Metal => "Metal",
        Backend::Direct3D12 => "Direct3D12",
        Backend::OpenGl => "OpenGl",
        Backend::BrowserWebGpu => "BrowserWebGpu",
    }
}

const fn adapter_kind_name(value: AdapterKind) -> &'static str {
    match value {
        AdapterKind::Discrete => "Discrete",
        AdapterKind::Integrated => "Integrated",
        AdapterKind::Virtual => "Virtual",
        AdapterKind::Software => "Software",
        AdapterKind::Other => "Other",
    }
}

const fn memory_class_name(value: MemoryClass) -> &'static str {
    match value {
        MemoryClass::Discrete => "Discrete",
        MemoryClass::Unified => "Unified",
        MemoryClass::Unknown => "Unknown",
    }
}

const fn capability_status_name(value: CapabilityStatus) -> &'static str {
    match value {
        CapabilityStatus::Unsupported => "Unsupported",
        CapabilityStatus::Supported => "Supported",
        CapabilityStatus::Enabled => "Enabled",
    }
}

const fn known_gpu_features() -> [GpuFeature; 7] {
    [
        GpuFeature::IndirectDrawCount,
        GpuFeature::MeshShaders,
        GpuFeature::SubgroupOperations,
        GpuFeature::TextureAtomics,
        GpuFeature::RayQueries,
        GpuFeature::RayTracingPipelines,
        GpuFeature::BindlessTextures,
    ]
}

const fn gpu_feature_name(feature: GpuFeature) -> &'static str {
    match feature {
        GpuFeature::IndirectDrawCount => "IndirectDrawCount",
        GpuFeature::MeshShaders => "MeshShaders",
        GpuFeature::SubgroupOperations => "SubgroupOperations",
        GpuFeature::TextureAtomics => "TextureAtomics",
        GpuFeature::RayQueries => "RayQueries",
        GpuFeature::RayTracingPipelines => "RayTracingPipelines",
        GpuFeature::BindlessTextures => "BindlessTextures",
    }
}

const fn power_preference_name(value: PowerPreference) -> &'static str {
    match value {
        PowerPreference::HighPerformance => "HighPerformance",
        PowerPreference::LowPower => "LowPower",
    }
}

const fn present_policy_name(value: PresentPolicy) -> &'static str {
    match value {
        PresentPolicy::Vsync => "Vsync",
        PresentPolicy::AllowTearing => "AllowTearing",
    }
}

fn source_provenance_from_environment() -> Result<SourceProvenance, Box<dyn Error>> {
    let state = std::env::var("MERIDIAN_SOURCE_STATE").map_err(|_| {
        "MERIDIAN_SOURCE_STATE is required and must be clean-commit or working-tree"
    })?;
    let checkpoint = std::env::var("MERIDIAN_SOURCE_CHECKPOINT")
        .map_err(|_| "MERIDIAN_SOURCE_CHECKPOINT is required for reproducible recovery evidence")?;
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
            return Err("MERIDIAN_SOURCE_STATE must be exactly clean-commit or working-tree".into())
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

fn validate_path_free_checkpoint(checkpoint: &str) -> Result<String, Box<dyn Error>> {
    let allowed = checkpoint
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'));
    if checkpoint.is_empty() || checkpoint == "NotAvailable" || checkpoint.len() > 512 || !allowed {
        return Err(
            "source checkpoint must be a nonempty path-free identifier of at most 512 ASCII letters, digits, '-', '_', or '.'"
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

fn write_preflight_failure(
    evidence_directory: &std::path::Path,
    failure: &RunnerFailure,
    source: Option<&SourceProvenance>,
) -> Result<(), Box<dyn Error>> {
    write_evidence_json(
        evidence_directory.join("device-loss-replay-failure.json"),
        &DeviceLossReplayFailureReport {
            schema: DEVICE_LOSS_REPLAY_SCHEMA,
            runner_status: failure.status,
            evidence_status: failure.status,
            failure_code: failure.code,
            source_checkpoint: source.map(|source| source.checkpoint.clone()),
            source_state: source.map(|source| source.state.as_str()),
            source_provenance_verification: source
                .map_or("NotAvailable", |_| "CallerDeclaredNotVerified"),
            source_provenance: source.map(SourceProvenance::report),
            evidence_directory: ".",
            stage: "preflight",
            environment: evidence_environment(false),
            baseline_profile: None,
            detail: sanitize_failure_detail(&failure.detail),
            limits: [
                "This preflight artifact records a bounded runner setup failure only.",
                "It cannot establish device-loss, renderer, visual, or platform qualification.",
            ],
        },
    )?;
    Ok(())
}

fn evidence_write_failure(original: &RunnerFailure, error: &dyn Error) -> RunnerFailure {
    RunnerFailure::new(
        EvidenceStatus::Fail,
        FailureCode::EvidenceWrite,
        format!(
            "{}; failed to write evidence artifact: {}",
            original,
            sanitize_failure_detail(&error.to_string())
        ),
    )
}

fn evidence_directory_from_args() -> Result<PathBuf, Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let mut explicit = None;
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            let path = arguments
                .next()
                .ok_or_else(|| "--evidence requires a path".to_owned())?;
            if explicit.replace(PathBuf::from(path)).is_some() {
                return Err("--evidence may be provided only once".into());
            }
        } else {
            return Err("unrecognized device-loss replay argument".into());
        }
    }
    if let Some(path) = explicit {
        return Ok(path);
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(
        PathBuf::from("target/meridian-evidence/ui-direct-device-loss-replay")
            .join(format!("{}-{nonce}", std::process::id())),
    )
}

fn evidence_directory_hint_from_args() -> Option<PathBuf> {
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            return arguments.next().map(PathBuf::from);
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(
        PathBuf::from("target/meridian-evidence/ui-direct-device-loss-replay")
            .join(format!("{}-{nonce}", std::process::id())),
    )
}

fn standard_case() -> Result<UiDirectQualificationCase, Box<dyn Error>> {
    ui_direct_qualification_cases()?
        .into_iter()
        .find(|case| case.id == STANDARD_CASE_ID)
        .ok_or_else(|| format!("direct UI qualification corpus omits {STANDARD_CASE_ID}").into())
}

struct PreparedRun {
    evidence_directory: PathBuf,
    source: SourceProvenance,
    case: UiDirectQualificationCase,
}

fn preflight_error(
    evidence_directory: Option<&std::path::Path>,
    failure: RunnerFailure,
    source: Option<&SourceProvenance>,
) -> Box<dyn Error> {
    if let Some(directory) = evidence_directory {
        if let Err(write_error) = write_preflight_failure(directory, &failure, source) {
            return Box::new(evidence_write_failure(&failure, write_error.as_ref()));
        }
    }
    Box::new(failure)
}

fn prepare_run() -> Result<PreparedRun, Box<dyn Error>> {
    let evidence_directory = match evidence_directory_from_args() {
        Ok(directory) => directory,
        Err(error) => {
            let failure = RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::ArgumentsInvalid,
                error.to_string(),
            );
            let directory = evidence_directory_hint_from_args();
            return Err(preflight_error(directory.as_deref(), failure, None));
        }
    };
    let source = match source_provenance_from_environment() {
        Ok(source) => source,
        Err(error) => {
            let failure = RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::SourceProvenanceInvalid,
                error.to_string(),
            );
            return Err(preflight_error(Some(&evidence_directory), failure, None));
        }
    };
    if let Err(error) = fs::create_dir_all(&evidence_directory) {
        let failure = RunnerFailure::new(
            EvidenceStatus::Fail,
            FailureCode::EvidenceDirectoryUnavailable,
            error.to_string(),
        );
        return Err(preflight_error(
            Some(&evidence_directory),
            failure,
            Some(&source),
        ));
    }
    let case = match standard_case() {
        Ok(case) => case,
        Err(error) => {
            let failure = RunnerFailure::new(
                EvidenceStatus::Fail,
                FailureCode::DirectRendererFailure,
                error.to_string(),
            );
            return Err(preflight_error(
                Some(&evidence_directory),
                failure,
                Some(&source),
            ));
        }
    };
    Ok(PreparedRun {
        evidence_directory,
        source,
        case,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let PreparedRun {
        evidence_directory,
        source,
        case,
    } = prepare_run()?;
    println!(
        "Meridian direct UI controlled device-loss replay evidence: {}",
        evidence_directory.display()
    );
    let failure = Arc::new(Mutex::new(None));
    let run_result = run(
        PlatformConfig {
            title: "Meridian Direct UI Device-Loss Replay".to_owned(),
            initial_size: WindowSize::new(320, 180),
            resizable: false,
            visible: false,
            event_loop_mode: EventLoopMode::Wait,
        },
        DeviceLossReplayRunner {
            evidence_directory: evidence_directory.clone(),
            source: source.clone(),
            config: RhiConfig::default(),
            failure: Arc::clone(&failure),
            case,
            stage: ReplayStage::BaselineCapture,
            rhi: None,
            renderer: None,
            plan: None,
            gpu: None,
            capture_deadline: None,
            baseline: None,
            baseline_profile: None,
            recovered_profile: None,
            old_render_identity: None,
            recovered_render_identity: None,
            dropped_cache_count: None,
        },
    );
    if let Err(error) = run_result {
        let failure = RunnerFailure::from_error(&error);
        return Err(preflight_error(
            Some(&evidence_directory),
            failure,
            Some(&source),
        ));
    }
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

    #[test]
    fn source_provenance_requires_an_explicit_state_and_safe_checkpoint() {
        assert_eq!(
            source_provenance_from_values("working-tree", "local-device-loss-replay-evidence")
                .expect("checkpoint")
                .state,
            SourceState::WorkingTree
        );
        assert!(source_provenance_from_values("clean-commit", "working-tree").is_err());
        assert!(validate_path_free_checkpoint("NotAvailable").is_err());
        assert!(validate_path_free_checkpoint("/Users/example").is_err());
    }

    #[test]
    fn source_provenance_stays_caller_declared_and_non_promoting() {
        let checkpoint = "0123456789abcdef0123456789abcdef01234567";
        let source = source_provenance_from_values("clean-commit", checkpoint).expect("source");
        let report = source.report();
        assert_eq!(report.declaration, "CallerDeclared");
        assert_eq!(report.verification, "CallerDeclaredNotVerified");
        assert_eq!(
            report.promotion_eligibility,
            "NotEligibleCallerDeclaredCleanCommit"
        );
        assert_eq!(device_loss_evidence_status(), EvidenceStatus::Inconclusive);
    }

    #[test]
    fn known_unavailable_rhi_conditions_keep_typed_evidence_statuses() {
        assert_eq!(
            RunnerFailure::from_rhi_kind(RhiErrorKind::AdapterUnavailable, "adapter").status,
            EvidenceStatus::NotRun
        );
        assert_eq!(
            RunnerFailure::from_rhi_kind(RhiErrorKind::SurfaceUnsupported, "surface").status,
            EvidenceStatus::UnsupportedPlatform
        );
        assert_eq!(
            RunnerFailure::from_rhi_kind(RhiErrorKind::CaptureTargetUnsupported, "capture").status,
            EvidenceStatus::UnsupportedCapability
        );
        assert_eq!(
            RunnerFailure::from_rhi_kind(RhiErrorKind::DeviceLost, "loss").status,
            EvidenceStatus::Inconclusive
        );
        let failure =
            RunnerFailure::from_ui_direct(&UiDirectRendererError::OffscreenCaptureUnsupported {
                rhi_kind: RhiErrorKind::SurfaceUnsupported,
            });
        assert_eq!(failure.status, EvidenceStatus::UnsupportedCapability);
        assert_eq!(
            failure.code,
            FailureCode::OffscreenCaptureCopySourceUnsupported
        );
    }

    #[test]
    fn environment_declares_unavailable_build_inputs_and_single_run_scope() {
        let environment = evidence_environment(false);
        assert_eq!(environment.build_identity.availability, "NotAvailable");
        assert_eq!(environment.build_hash.availability, "NotAvailable");
        assert_eq!(environment.toolchain_profile.availability, "NotAvailable");
        assert_eq!(environment.dependency_profile.availability, "NotAvailable");
        assert_eq!(environment.execution_scope.invocations, 1);
        assert_eq!(environment.execution_scope.warmup_iterations, 0);
        assert_eq!(
            environment.execution_scope.cross_run_cache_state,
            "NotMeasured"
        );
    }

    #[test]
    fn absent_driver_metadata_is_explicit_and_explained() {
        let (driver, driver_availability) = explicit_profile_metadata("   ", DRIVER_UNAVAILABLE);
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
    fn failure_detail_is_bounded_and_path_safe() {
        assert_eq!(
            sanitize_failure_detail("failed to map capture"),
            "failed to map capture"
        );
        assert_eq!(
            sanitize_failure_detail("failed at /private/path"),
            "PathOrControlCharacterRedacted"
        );
        assert_eq!(
            sanitize_failure_detail(&"x".repeat(MAX_FAILURE_DETAIL_CHARS.saturating_add(1))),
            format!("{}...", "x".repeat(MAX_FAILURE_DETAIL_CHARS))
        );
    }

    #[test]
    fn evidence_write_failures_remain_typed_and_surface_the_original_cause() {
        let original = RunnerFailure::new(
            EvidenceStatus::Inconclusive,
            FailureCode::CaptureTimedOut,
            "capture timed out",
        );
        let write_error = std::io::Error::other("evidence write denied");
        let surfaced = evidence_write_failure(&original, &write_error);
        assert_eq!(surfaced.status, EvidenceStatus::Fail);
        assert_eq!(surfaced.code, FailureCode::EvidenceWrite);
        assert!(surfaced.detail.contains("CaptureTimedOut"));
    }
}
