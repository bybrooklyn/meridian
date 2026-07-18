//! Hidden native-window qualification runner for Meridian's direct UI path.
//!
//! It captures deterministic display-list corpus scenes into bounded offscreen
//! targets. The resulting raw RGBA and PNG artifacts prove only profile-bound
//! renderer/recovery behavior; they are not a presented visual review.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use meridian_assets::ArtifactHash;
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
    UiDirectRgba8Image, UI_DIRECT_QUALIFICATION_SCHEMA,
};
use meridian_rhi::{
    AdapterKind, Backend, CapabilityStatus, CaptureOutcome, CaptureRequest, CaptureSource,
    CapturedFrame, CapturedPixelFormat, ClearColor, MemoryClass, PowerPreference, PresentPolicy,
    Rhi, RhiConfig, RhiError, RhiErrorKind,
};
use serde::{Deserialize, Serialize};

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const QUALIFICATION_CLEAR: ClearColor = ClearColor::new(0.0, 0.0, 0.0, 1.0);
const GOLDEN_FIXTURE_SCHEMA: &str = "meridian.ui-direct-golden-fixtures/v1";
const GOLDEN_FIXTURE_VERSION: u32 = 1;
const GOLDEN_FIXTURE_GENERATOR: &str = "meridian-benchmark/ui_direct_qualification";
const GOLDEN_FIXTURE_WRITE_ENVIRONMENT: &str = "MERIDIAN_ALLOW_GOLDEN_FIXTURE_WRITE";
const GOLDEN_FIXTURE_REGENERATION_COMMAND: &str = "MERIDIAN_ALLOW_GOLDEN_FIXTURE_WRITE=1 MERIDIAN_SOURCE_STATE=clean-commit MERIDIAN_SOURCE_CHECKPOINT=<40-lowercase-commit-sha> cargo run -p meridian-benchmark --example ui_direct_qualification --features ui-direct-qualification -- --write-fixtures --evidence target/meridian-evidence/ui-direct-qualification/<unique>";
const FIXTURE_PROPOSAL_DIRECTORY: &str = "fixture-proposal";

#[derive(Clone, Debug, Serialize)]
struct CaseReport {
    id: String,
    corpus_hash: String,
    width: u32,
    height: u32,
    frame_id: u64,
    capture_id: u64,
    pixel_hash: String,
    png_hash: String,
    png: String,
    png_metadata: String,
    rgba: String,
    golden: GoldenComparisonReport,
}

#[derive(Clone, Debug, Serialize)]
struct QualificationProfile {
    backend: String,
    adapter_name: String,
    driver: String,
    driver_info: String,
    vendor_id: u32,
    device_id: u32,
    surface_format: String,
    operating_system: String,
    architecture: String,
    adapter_kind: String,
    memory_class: String,
    timestamp_query_capability: String,
    hdr_surface_formats_capability: String,
    max_sampled_textures_per_shader_stage: u32,
    enabled_features: Vec<String>,
    configuration: RhiConfigurationProfile,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RhiConfigurationProfile {
    power_preference: String,
    preferred_backend: Option<String>,
    allow_software_adapter: bool,
    present_policy: String,
    desired_maximum_frame_latency: u32,
    enable_timestamps: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GoldenFixtureProfile {
    backend: String,
    adapter_name: String,
    driver: String,
    driver_info: String,
    vendor_id: u32,
    device_id: u32,
    surface_format: String,
    operating_system: String,
    architecture: String,
    adapter_kind: String,
    memory_class: String,
    timestamp_query_capability: String,
    hdr_surface_formats_capability: String,
    max_sampled_textures_per_shader_stage: u32,
    enabled_features: Vec<String>,
    configuration: RhiConfigurationProfile,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GoldenFixtureCase {
    id: String,
    corpus_hash: String,
    width: u32,
    height: u32,
    rgba: String,
    pixel_hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GoldenFixtureManifest {
    schema: String,
    version: u32,
    generator: String,
    regeneration_command: String,
    input_schema: String,
    source_checkpoint: String,
    profile: GoldenFixtureProfile,
    cases: Vec<GoldenFixtureCase>,
}

#[derive(Clone, Debug, Serialize)]
struct GoldenComparisonReport {
    status: &'static str,
    fixture_manifest: Option<String>,
    fixture_rgba: Option<String>,
    fixture_source_checkpoint: Option<String>,
    message: String,
    differing_pixel_count: Option<u64>,
    maximum_channel_delta: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
struct QualificationReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    capture_status: &'static str,
    source_checkpoint: String,
    source_state: &'static str,
    source_provenance_verification: &'static str,
    source_provenance_limit: &'static str,
    evidence_scope: &'static str,
    promotion_eligibility: &'static str,
    evidence_directory: String,
    environment: QualificationEvidenceEnvironment,
    profile: QualificationProfile,
    cases: Vec<CaseReport>,
    fixture_proposal: Option<FixtureProposalReport>,
    limits: [&'static str; 3],
}

#[derive(Clone, Debug, Serialize)]
struct FixtureProposalReport {
    status: &'static str,
    profile_key: String,
    proposal_directory: String,
    manifest: String,
    raw_rgba: Vec<String>,
    source_checkpoint: String,
    message: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct QualificationFailureReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    code: &'static str,
    source_checkpoint: Option<String>,
    source_state: Option<&'static str>,
    source_provenance_verification: &'static str,
    evidence_directory: &'static str,
    stage: &'static str,
    case_id: Option<String>,
    environment: QualificationEvidenceEnvironment,
    profile: Option<QualificationProfile>,
    error: String,
    limits: [&'static str; 2],
}

#[derive(Clone, Debug, Serialize)]
struct QualificationEvidenceEnvironment {
    requirement_ids: [&'static str; 2],
    work_package_id: &'static str,
    milestone_id: &'static str,
    research_gate_id: &'static str,
    runner_package: &'static str,
    runner_package_version: &'static str,
    build_identity: EvidenceAvailability,
    build_hash: EvidenceAvailability,
    toolchain_profile: EvidenceAvailability,
    dependency_profile: EvidenceAvailability,
    capability_profile: EvidenceAvailability,
    execution_scope: QualificationExecutionScope,
    memory_telemetry: QualificationMemoryTelemetry,
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceAvailability {
    availability: &'static str,
    value: Option<String>,
    reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct QualificationExecutionScope {
    invocations: usize,
    warmup_iterations: usize,
    cache_scope: &'static str,
    cross_run_cache_state: EvidenceAvailability,
    repetition_scope: &'static str,
    required_example_feature: &'static str,
    missing_required_features: Vec<String>,
    exact_fixture_comparisons: usize,
    independent_process_repetitions: usize,
}

#[derive(Clone, Debug, Serialize)]
struct QualificationMemoryTelemetry {
    actual_backend_allocations: &'static str,
    vram_usage: &'static str,
    driver_residency: &'static str,
    reason: &'static str,
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
            Self::CleanCommit => "NotEligibleLocalStructuralEvidence",
            Self::WorkingTree => "NotEligibleWorkingTree",
        }
    }
}

#[derive(Clone, Debug)]
struct SourceProvenance {
    checkpoint: String,
    state: SourceState,
}

#[derive(Clone, Debug)]
struct QualificationFailure {
    evidence_status: &'static str,
    code: &'static str,
    detail: String,
}

impl QualificationFailure {
    fn new(evidence_status: &'static str, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            evidence_status,
            code,
            detail: detail.into(),
        }
    }

    fn from_rhi_kind(kind: RhiErrorKind, detail: impl Into<String>) -> Self {
        let (evidence_status, code) = match kind {
            RhiErrorKind::AdapterUnavailable => ("NotRun", "AdapterUnavailable"),
            RhiErrorKind::SurfaceCreation | RhiErrorKind::SurfaceUnsupported => {
                ("UnsupportedPlatform", "SurfaceUnavailable")
            }
            RhiErrorKind::DeviceCreation => ("Inconclusive", "DeviceCreationInconclusive"),
            RhiErrorKind::CaptureTargetUnsupported => {
                ("UnsupportedCapability", "CaptureUnsupported")
            }
            RhiErrorKind::DeviceLost => ("Inconclusive", "DeviceLost"),
            _ => ("Fail", "RhiFailure"),
        };
        Self::new(evidence_status, code, detail)
    }

    fn from_error(error: &(dyn Error + 'static)) -> Self {
        if let Some(failure) = error.downcast_ref::<Self>() {
            return failure.clone();
        }
        if let Some(rhi) = error.downcast_ref::<RhiError>() {
            return Self::from_rhi_kind(rhi.kind(), rhi.to_string());
        }
        if let Some(direct) = error.downcast_ref::<UiDirectRendererError>() {
            if matches!(
                direct,
                UiDirectRendererError::OffscreenCaptureUnsupported { .. }
            ) {
                return Self::new(
                    "UnsupportedCapability",
                    "OffscreenCaptureCopySourceUnsupported",
                    direct.to_string(),
                );
            }
            if let Some(kind) = direct.rhi_kind() {
                return Self::from_rhi_kind(kind, direct.to_string());
            }
            let (evidence_status, code) = match direct {
                UiDirectRendererError::UnsupportedSurfaceColorSpace => {
                    ("UnsupportedCapability", "SurfaceColorSpaceUnsupported")
                }
                _ => ("Fail", "DirectRendererFailure"),
            };
            return Self::new(evidence_status, code, direct.to_string());
        }
        Self::new("Fail", "RunnerFailure", error.to_string())
    }
}

impl std::fmt::Display for QualificationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {}: {}",
            self.evidence_status, self.code, self.detail
        )
    }
}

impl Error for QualificationFailure {}

struct QualificationRunner {
    evidence_directory: PathBuf,
    source: SourceProvenance,
    failure: Arc<Mutex<Option<String>>>,
    rhi: Option<Rhi>,
    renderer: Option<UiDirectGpuRenderer>,
    profile: Option<QualificationProfile>,
    cases: Vec<UiDirectQualificationCase>,
    case_index: usize,
    plan: Option<UiDirectFramePlan>,
    gpu: Option<UiDirectGpuFrame>,
    capture_deadline: Option<Instant>,
    reports: Vec<CaseReport>,
    write_fixtures: bool,
}

impl QualificationRunner {
    fn fail(&mut self, failure: &QualificationFailure, context: &mut PlatformContext<'_>) {
        let message = match self.write_failure_report(failure) {
            Ok(()) => failure.to_string(),
            Err(write_error) => format!(
                "{failure}; additionally failed to write qualification failure evidence: {write_error}"
            ),
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

    fn fail_message(&mut self, detail: impl Into<String>, context: &mut PlatformContext<'_>) {
        self.fail(
            &QualificationFailure::new("Fail", "RunnerFailure", detail),
            context,
        );
    }

    fn fail_from_error(
        &mut self,
        error: &(dyn Error + 'static),
        context: &mut PlatformContext<'_>,
    ) {
        self.fail(&QualificationFailure::from_error(error), context);
    }

    fn failure_stage(&self) -> &'static str {
        if self.profile.is_none() {
            "initialization"
        } else if self.case_index < self.cases.len() {
            "capture-or-golden-comparison"
        } else {
            "final-report"
        }
    }

    fn write_failure_report(&self, failure: &QualificationFailure) -> Result<(), Box<dyn Error>> {
        let report = QualificationFailureReport {
            schema: UI_DIRECT_QUALIFICATION_SCHEMA,
            runner_status: "Fail",
            evidence_status: failure.evidence_status,
            code: failure.code,
            source_checkpoint: Some(self.source.checkpoint.clone()),
            source_state: Some(self.source.state.as_str()),
            source_provenance_verification: "CallerDeclaredNotVerified",
            evidence_directory: ".",
            stage: self.failure_stage(),
            case_id: self
                .cases
                .get(self.case_index)
                .map(|case| case.id.to_owned()),
            environment: qualification_evidence_environment(
                self.profile.clone(),
                self.reports.len(),
            ),
            profile: self.profile.clone(),
            error: sanitize_failure_detail(&failure.detail),
            limits: [
                "This failure artifact records a runner failure, not a visual-quality result.",
                "Completed case artifacts remain available but cannot promote qualification.",
            ],
        };
        write_evidence_json(
            self.evidence_directory.join("qualification-failure.json"),
            &report,
        )?;
        Ok(())
    }

    fn current_case(&self) -> Result<&UiDirectQualificationCase, Box<dyn Error>> {
        self.cases
            .get(self.case_index)
            .ok_or_else(|| format!("qualification case {} is unavailable", self.case_index).into())
    }

    fn validate_current_capture(&self, frame: &CapturedFrame) -> Result<(), Box<dyn Error>> {
        let plan = self
            .plan
            .as_ref()
            .ok_or_else(|| "qualification plan is unavailable".to_owned())?;
        let cache_key = plan.cache_key();
        let expected_frame_id = FrameId::new(u64::try_from(self.case_index.saturating_add(1))?);
        let expected_bytes = usize::try_from(
            u64::from(cache_key.surface_width)
                .checked_mul(u64::from(cache_key.surface_height))
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| "qualification pixel length overflowed".to_owned())?,
        )?;
        if frame.frame_id != expected_frame_id
            || frame.width != cache_key.surface_width
            || frame.height != cache_key.surface_height
            || frame.format != CapturedPixelFormat::Rgba8Srgb
            || frame.source != CaptureSource::Offscreen
            || frame.surface_outcome.is_some()
            || frame.pixels.len() != expected_bytes
            || !has_multiple_pixel_values(frame)
        {
            return Err(format!("qualification capture metadata is invalid: {frame:?}").into());
        }
        Ok(())
    }

    fn begin_current_case(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let case = self.current_case()?.clone();
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| "qualification RHI is unavailable".to_owned())?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| "qualification renderer is unavailable".to_owned())?;
        let plan = renderer.prepare_frame(case.prepare_request())?;
        let cache_key = plan.cache_key();
        let frame_id = FrameId::new(u64::try_from(self.case_index.saturating_add(1))?);
        let max_bytes = u64::from(cache_key.surface_width)
            .checked_mul(u64::from(cache_key.surface_height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "qualification capture byte count overflowed".to_owned())?;
        let gpu = plan.upload_gpu_frame(rhi)?;
        rhi.request_capture(CaptureRequest::new(
            frame_id,
            cache_key.surface_width,
            cache_key.surface_height,
            max_bytes,
        ))?;
        gpu.submit_offscreen_capture(rhi, &plan, QUALIFICATION_CLEAR)?;
        self.plan = Some(plan);
        self.gpu = Some(gpu);
        self.capture_deadline = Some(Instant::now() + CAPTURE_TIMEOUT);
        context.request_redraw();
        Ok(())
    }

    fn finish_current_capture(
        &mut self,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let capture = self
            .rhi
            .as_mut()
            .ok_or_else(|| "qualification RHI is unavailable".to_owned())?
            .take_capture();
        let Some(capture) = capture else {
            if self
                .capture_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err(QualificationFailure::new(
                    "Inconclusive",
                    "CaptureTimedOut",
                    "qualification pixel readback timed out",
                )
                .into());
            }
            context.request_redraw();
            return Ok(());
        };
        let CaptureOutcome::Captured(frame) = capture else {
            let failure = match capture {
                CaptureOutcome::UnsupportedCapability { failure, .. } => QualificationFailure::new(
                    "UnsupportedCapability",
                    "CaptureUnsupported",
                    format!("qualification offscreen capture is unsupported: {failure:?}"),
                ),
                CaptureOutcome::Inconclusive { failure, .. } => QualificationFailure::new(
                    "Inconclusive",
                    "CaptureInconclusive",
                    format!("qualification offscreen capture is inconclusive: {failure:?}"),
                ),
                CaptureOutcome::Captured(_) => unreachable!("captured outcome matched above"),
            };
            return Err(failure.into());
        };
        self.validate_current_capture(&frame)?;
        let case = self.current_case()?.clone();
        let png = write_capture_png(
            self.evidence_directory.join(format!("{}.png", case.id)),
            &frame,
        )?;
        let rgba = write_capture_rgba(
            self.evidence_directory.join(format!("{}.rgba", case.id)),
            &frame,
        )?;
        if rgba.pixel_hash != png.metadata.pixel_hash {
            return Err("qualification raw and PNG capture hashes differ".into());
        }
        let golden = self.compare_golden(&case, &frame)?;
        let golden_failed = golden.status == "Fail";
        self.reports.push(CaseReport {
            id: case.id.to_owned(),
            corpus_hash: case.corpus_hash(),
            width: frame.width,
            height: frame.height,
            frame_id: frame.frame_id.get(),
            capture_id: frame.capture_id.get(),
            pixel_hash: png.metadata.pixel_hash,
            png_hash: png.metadata.png_hash,
            png: format!("{}.png", case.id),
            png_metadata: format!("{}.png.json", case.id),
            rgba: format!("{}.rgba", case.id),
            golden,
        });
        if golden_failed {
            self.write_report()?;
            return Err(format!("golden qualification failed for {}", case.id).into());
        }
        self.case_index = self.case_index.saturating_add(1);
        self.plan = None;
        self.gpu = None;
        self.capture_deadline = None;
        if self.case_index == self.cases.len() {
            self.write_report()?;
            context.exit();
        } else {
            self.begin_current_case(context)?;
        }
        Ok(())
    }

    fn write_report(&self) -> Result<(), Box<dyn Error>> {
        let fixture_proposal = self
            .write_fixtures
            .then(|| self.write_fixture_proposal())
            .transpose()?;
        let report = QualificationReport {
            schema: UI_DIRECT_QUALIFICATION_SCHEMA,
            runner_status: qualification_status(&self.reports),
            evidence_status: qualification_evidence_status(&self.reports),
            capture_status: "Pass",
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            source_provenance_verification: "CallerDeclaredNotVerified",
            source_provenance_limit: "Source state and checkpoint are caller-declared and cannot attest the compiled binary or checkout cleanliness.",
            evidence_scope: "LocalStructuralEvidence",
            promotion_eligibility: self.source.state.promotion_eligibility(),
            evidence_directory: ".".to_owned(),
            environment: qualification_evidence_environment(
                self.profile.clone(),
                self.reports.len(),
            ),
            profile: self
                .profile
                .clone()
                .ok_or_else(|| "qualification GPU profile is unavailable".to_owned())?,
            cases: self.reports.clone(),
            fixture_proposal,
            limits: [
                "Offscreen raw RGBA comparison is profile-bound renderer evidence, not presented visual review.",
                "A missing profile fixture is NotRun; a matched fixture mismatch fails this runner.",
                "Capture-copy GPU timing is UnsupportedCapability; the copy report retains CPU encoding only.",
            ],
        };
        write_evidence_json(self.evidence_directory.join("qualification.json"), &report)?;
        println!(
            "Meridian direct UI qualification captured {} cases at {}",
            self.reports.len(),
            self.evidence_directory.display()
        );
        Ok(())
    }

    fn compare_golden(
        &self,
        case: &UiDirectQualificationCase,
        frame: &meridian_rhi::CapturedFrame,
    ) -> Result<GoldenComparisonReport, Box<dyn Error>> {
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| "qualification GPU profile is unavailable".to_owned())?;
        if self.write_fixtures {
            return Ok(GoldenComparisonReport {
                status: "NotRun",
                fixture_manifest: None,
                fixture_rgba: None,
                fixture_source_checkpoint: None,
                message: "fixture regeneration mode wrote a proposal after capture; this invocation did not consume a golden fixture".to_owned(),
                differing_pixel_count: None,
                maximum_channel_delta: None,
            });
        }
        let fixture_directory = golden_fixture_directory();
        let profile_key = fixture_profile_key(profile);
        let manifest_path = fixture_directory.join(format!("{profile_key}.json"));
        if !manifest_path.exists() {
            return Ok(GoldenComparisonReport {
                status: "NotRun",
                fixture_manifest: None,
                fixture_rgba: None,
                fixture_source_checkpoint: None,
                message: format!("no exact golden fixture is registered for {profile_key}"),
                differing_pixel_count: None,
                maximum_channel_delta: None,
            });
        }
        validated_fixture_metadata(&manifest_path)?;
        let manifest: GoldenFixtureManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
        validate_fixture_manifest(&manifest)?;
        let profile_mismatches = fixture_profile_mismatches(&manifest.profile, profile);
        if !profile_mismatches.is_empty() {
            return Ok(GoldenComparisonReport {
                status: "NotRun",
                fixture_manifest: Some(fixture_manifest_name(&profile_key)),
                fixture_rgba: None,
                fixture_source_checkpoint: Some(manifest.source_checkpoint.clone()),
                message: format!(
                    "fixture profile differs from active RHI profile in {}",
                    profile_mismatches.join(", ")
                ),
                differing_pixel_count: None,
                maximum_channel_delta: None,
            });
        }
        let fixture = manifest
            .cases
            .iter()
            .find(|fixture| fixture.id == case.id)
            .ok_or_else(|| format!("golden manifest omits qualification case {}", case.id))?;
        if fixture.corpus_hash != case.corpus_hash()
            || fixture.width != frame.width
            || fixture.height != frame.height
        {
            return Err(format!(
                "golden fixture contract differs for {}: corpus or dimensions are stale",
                case.id
            )
            .into());
        }
        let expected = read_fixture_rgba(
            &fixture_directory,
            fixture,
            u64::try_from(frame.pixels.len())?,
        )?;
        let comparison = compare_ui_direct_rgba8_exact(
            UiDirectRgba8Image::new(fixture.width, fixture.height, &expected),
            UiDirectRgba8Image::new(frame.width, frame.height, &frame.pixels),
        )?;
        if !comparison.passed() {
            return Ok(GoldenComparisonReport {
                status: "Fail",
                fixture_manifest: Some(fixture_manifest_name(&profile_key)),
                fixture_rgba: Some(fixture.rgba.clone()),
                fixture_source_checkpoint: Some(manifest.source_checkpoint.clone()),
                message: format!(
                    "golden mismatch: {} pixels differ; max channel delta {}; first difference {:?}",
                    comparison.differing_pixel_count,
                    comparison.maximum_channel_delta,
                    comparison.first_difference
                ),
                differing_pixel_count: Some(comparison.differing_pixel_count),
                maximum_channel_delta: Some(comparison.maximum_channel_delta),
            });
        }
        Ok(GoldenComparisonReport {
            status: "Pass",
            fixture_manifest: Some(fixture_manifest_name(&profile_key)),
            fixture_rgba: Some(fixture.rgba.clone()),
            fixture_source_checkpoint: Some(manifest.source_checkpoint.clone()),
            message: "exact profile-bound RGBA golden comparison passed".to_owned(),
            differing_pixel_count: Some(comparison.differing_pixel_count),
            maximum_channel_delta: Some(comparison.maximum_channel_delta),
        })
    }

    /// Publishes this run's profile-bound raw RGBA captures as an evidence proposal.
    ///
    /// Generation-qualified raw files are durably created before the manifest
    /// is atomically replaced. The manifest stays in this invocation's evidence
    /// directory rather than the active source-fixture directory, so an
    /// interrupted or caller-declared run cannot self-activate a golden. The
    /// current invocation remains `NotRun` because it never consumes its own
    /// proposal as a passing golden.
    fn write_fixture_proposal(&self) -> Result<FixtureProposalReport, Box<dyn Error>> {
        if self.source.state != SourceState::CleanCommit {
            return Err(
                "--write-fixtures requires MERIDIAN_SOURCE_STATE=clean-commit; working-tree captures are evidence proposals only and cannot replace profile fixtures"
                    .into(),
            );
        }
        if self.reports.len() != self.cases.len() {
            return Err("fixture proposal requires every qualification case to capture".into());
        }
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| "qualification GPU profile is unavailable".to_owned())?;
        let profile_key = fixture_profile_key(profile);
        let proposal_directory = fixture_proposal_directory(&self.evidence_directory);
        fs::create_dir_all(&proposal_directory)?;

        let mut case_fixtures = Vec::with_capacity(self.reports.len());
        let mut raw_rgba = Vec::with_capacity(self.reports.len());
        let mut names = BTreeSet::new();
        for report in &self.reports {
            let source_name = validated_fixture_name(&report.rgba)?;
            let source_path = self.evidence_directory.join(source_name);
            let bytes = fs::read(&source_path)?;
            let expected_bytes = u64::from(report.width)
                .checked_mul(u64::from(report.height))
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| "fixture proposal byte count overflowed".to_owned())?;
            if u64::try_from(bytes.len())? != expected_bytes {
                return Err(format!(
                    "fixture proposal evidence {} has {} bytes; expected {expected_bytes}",
                    report.rgba,
                    bytes.len()
                )
                .into());
            }
            let pixel_hash = ArtifactHash::digest(&bytes).to_string();
            if pixel_hash != report.pixel_hash {
                return Err(format!(
                    "fixture proposal evidence hash differs for {}: report {}, file {}",
                    report.id, report.pixel_hash, pixel_hash
                )
                .into());
            }
            let hash_suffix = pixel_hash
                .get(..16)
                .ok_or_else(|| "fixture proposal pixel hash is too short".to_owned())?;
            let name = format!(
                "{profile_key}-{}-{hash_suffix}.rgba",
                fixture_token(&report.id)
            );
            validated_fixture_name(&name)?;
            if !names.insert(name.clone()) {
                return Err(format!("fixture proposal has colliding raw name {name}").into());
            }
            atomic_write(&proposal_directory.join(&name), &bytes)?;
            raw_rgba.push(relative_proposal_artifact(&name));
            case_fixtures.push(GoldenFixtureCase {
                id: report.id.clone(),
                corpus_hash: report.corpus_hash.clone(),
                width: report.width,
                height: report.height,
                rgba: name,
                pixel_hash,
            });
        }

        let manifest = GoldenFixtureManifest {
            schema: GOLDEN_FIXTURE_SCHEMA.to_owned(),
            version: GOLDEN_FIXTURE_VERSION,
            generator: GOLDEN_FIXTURE_GENERATOR.to_owned(),
            regeneration_command: GOLDEN_FIXTURE_REGENERATION_COMMAND.to_owned(),
            input_schema: UI_DIRECT_QUALIFICATION_SCHEMA.to_owned(),
            source_checkpoint: self.source.checkpoint.clone(),
            profile: golden_fixture_profile(profile),
            cases: case_fixtures,
        };
        validate_fixture_manifest(&manifest)?;
        let manifest_name = fixture_manifest_name(&profile_key);
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        atomic_write(&proposal_directory.join(&manifest_name), &manifest_bytes)?;

        println!(
            "Meridian direct UI fixture proposal wrote {} and {} raw RGBA files under {}; this run remains NotRun pending review",
            relative_proposal_artifact(&manifest_name),
            raw_rgba.len(),
            FIXTURE_PROPOSAL_DIRECTORY
        );
        Ok(FixtureProposalReport {
            status: "ProposalWritten",
            profile_key,
            proposal_directory: FIXTURE_PROPOSAL_DIRECTORY.to_owned(),
            manifest: relative_proposal_artifact(&manifest_name),
            raw_rgba,
            source_checkpoint: self.source.checkpoint.clone(),
            message: "Fixture proposal was written into this evidence bundle. Review it and intentionally add it to source fixtures before any later run can consume it; this invocation did not compare against the new fixture.",
        })
    }

    fn initialize(
        &mut self,
        window: PlatformWindow,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let config = RhiConfig::default();
        let rhi = Rhi::new(window, config)?;
        self.profile = Some(qualification_profile(&rhi, config));
        self.renderer = Some(UiDirectGpuRenderer::new(rhi.render_identity()));
        self.rhi = Some(rhi);
        self.begin_current_case(context)
    }
}

impl PlatformApplication for QualificationRunner {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail_message("qualification window was not available", context);
                    return;
                };
                if let Err(error) = self.initialize(window, context) {
                    self.fail_from_error(error.as_ref(), context);
                }
            }
            PlatformEvent::RedrawRequested => {
                if let Err(error) = self.finish_current_capture(context) {
                    self.fail_from_error(error.as_ref(), context);
                }
            }
            PlatformEvent::CloseRequested if self.case_index < self.cases.len() => {
                self.fail_message(
                    "qualification was closed before all evidence completed",
                    context,
                );
            }
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn golden_fixture_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../engine/meridian_renderer/tests/fixtures/ui_direct/v1")
}

fn fixture_proposal_directory(evidence_directory: &Path) -> PathBuf {
    evidence_directory.join(FIXTURE_PROPOSAL_DIRECTORY)
}

fn relative_proposal_artifact(name: &str) -> String {
    format!("{FIXTURE_PROPOSAL_DIRECTORY}/{name}")
}

fn fixture_profile_key(profile: &QualificationProfile) -> String {
    format!(
        "{}-{}",
        fixture_token(&profile.backend),
        fixture_token(&profile.adapter_name)
    )
}

fn fixture_token(value: &str) -> String {
    let mut token = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !token.is_empty() {
            token.push('-');
            previous_dash = true;
        }
    }
    let trimmed = token.trim_end_matches('-');
    if trimmed.is_empty() {
        "unknown".to_owned()
    } else {
        trimmed.to_owned()
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

fn qualification_profile(rhi: &Rhi, config: RhiConfig) -> QualificationProfile {
    let capabilities = rhi.capabilities();
    QualificationProfile {
        backend: backend_name(capabilities.backend).to_owned(),
        adapter_name: capabilities.adapter_name.clone(),
        driver: explicit_profile_metadata(&capabilities.driver),
        driver_info: explicit_profile_metadata(&capabilities.driver_info),
        vendor_id: capabilities.vendor_id,
        device_id: capabilities.device_id,
        surface_format: rhi.surface_format().name,
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
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
            .map(|feature| format!("{feature:?}"))
            .collect(),
        configuration: rhi_configuration_profile(config),
    }
}

fn explicit_profile_metadata(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "NotAvailable".to_owned()
    } else {
        value.to_owned()
    }
}

fn rhi_configuration_profile(config: RhiConfig) -> RhiConfigurationProfile {
    RhiConfigurationProfile {
        power_preference: power_preference_name(config.power_preference).to_owned(),
        preferred_backend: config
            .preferred_backend
            .map(|backend| backend_name(backend).to_owned()),
        allow_software_adapter: config.allow_software_adapter,
        present_policy: present_policy_name(config.present_policy).to_owned(),
        desired_maximum_frame_latency: config.desired_maximum_frame_latency,
        enable_timestamps: config.enable_timestamps,
    }
}

fn unavailable_evidence_value(reason: &'static str) -> EvidenceAvailability {
    EvidenceAvailability {
        availability: "NotAvailable",
        value: None,
        reason,
    }
}

fn qualification_evidence_environment(
    profile: Option<QualificationProfile>,
    captured_cases: usize,
) -> QualificationEvidenceEnvironment {
    let capability_profile = match profile {
        Some(profile) => EvidenceAvailability {
            availability: "Available",
            value: Some(format!(
                "{} / {} / {}",
                profile.backend, profile.adapter_name, profile.surface_format
            )),
            reason:
                "The complete bounded capability profile is emitted in the top-level profile field.",
        },
        None => unavailable_evidence_value(
            "No RHI profile exists because initialization did not complete.",
        ),
    };
    QualificationEvidenceEnvironment {
        requirement_ids: ["REQ-UI-001", "REQ-UI-002"],
        work_package_id: "WP-UI-005",
        milestone_id: "MS-03",
        research_gate_id: "RG-UI-001",
        runner_package: env!("CARGO_PKG_NAME"),
        runner_package_version: env!("CARGO_PKG_VERSION"),
        build_identity: unavailable_evidence_value(
            "The native qualification runner is not invoked through a durable Meridian build operation.",
        ),
        build_hash: unavailable_evidence_value(
            "This runner does not infer a binary hash from caller-declared source provenance.",
        ),
        toolchain_profile: unavailable_evidence_value(
            "The runner does not execute or trust a host toolchain query at runtime.",
        ),
        dependency_profile: unavailable_evidence_value(
            "The runner does not infer dependency provenance beyond Cargo's locked build inputs.",
        ),
        capability_profile,
        execution_scope: QualificationExecutionScope {
            invocations: 1,
            warmup_iterations: 0,
            cache_scope: "Each corpus case prepares and uploads a bounded direct frame; cross-run cache state is not measured.",
            cross_run_cache_state: unavailable_evidence_value(
                "Shader, driver, and operating-system cache state are outside this runner's observation boundary.",
            ),
            repetition_scope: "One native process performs one exact comparison per registered corpus case; this is not a statistical performance sample.",
            required_example_feature: "ui-direct-qualification",
            missing_required_features: Vec::new(),
            exact_fixture_comparisons: captured_cases,
            independent_process_repetitions: 1,
        },
        memory_telemetry: QualificationMemoryTelemetry {
            actual_backend_allocations: "NotAvailable",
            vram_usage: "NotAvailable",
            driver_residency: "NotAvailable",
            reason: "The qualification runner records exact capture payloads but does not infer allocator, VRAM, or driver-residency telemetry.",
        },
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

fn fixture_manifest_name(profile_key: &str) -> String {
    format!("{profile_key}.json")
}

fn qualification_status(reports: &[CaseReport]) -> &'static str {
    if reports.is_empty() {
        "NotRun"
    } else if reports.iter().any(|report| report.golden.status == "Fail") {
        "Fail"
    } else if reports
        .iter()
        .any(|report| report.golden.status == "NotRun")
    {
        "NotRun"
    } else {
        "Pass"
    }
}

fn qualification_evidence_status(reports: &[CaseReport]) -> &'static str {
    match qualification_status(reports) {
        // A runner-local exact comparison is useful structural evidence, but
        // neither caller-declared source provenance nor an offscreen target can
        // qualify the package on its own.
        "Pass" => "Inconclusive",
        status => status,
    }
}

fn sanitize_failure_detail(detail: &str) -> String {
    if detail.len() > 512 || detail.contains(['/', '\\']) || detail.chars().any(char::is_control) {
        "Failure detail omitted because it could disclose a path, control character, or oversized value."
            .to_owned()
    } else {
        detail.to_owned()
    }
}

fn validate_fixture_manifest(manifest: &GoldenFixtureManifest) -> Result<(), Box<dyn Error>> {
    if manifest.schema != GOLDEN_FIXTURE_SCHEMA {
        return Err("golden fixture schema is unsupported".into());
    }
    if manifest.version != GOLDEN_FIXTURE_VERSION {
        return Err("golden fixture version is unsupported".into());
    }
    if manifest.generator != GOLDEN_FIXTURE_GENERATOR {
        return Err("golden fixture generator identity is unsupported".into());
    }
    if manifest.regeneration_command != GOLDEN_FIXTURE_REGENERATION_COMMAND {
        return Err("golden fixture regeneration command is unsupported".into());
    }
    if manifest.input_schema != UI_DIRECT_QUALIFICATION_SCHEMA {
        return Err("golden fixture input schema is unsupported".into());
    }
    validate_path_free_identifier(&manifest.source_checkpoint)?;
    if manifest.cases.is_empty() {
        return Err("golden fixture manifest must contain at least one case".into());
    }
    let mut ids = BTreeSet::new();
    for fixture in &manifest.cases {
        if fixture.id.is_empty() || !ids.insert(&fixture.id) {
            return Err("golden fixture case identifiers must be present and unique".into());
        }
        if fixture.corpus_hash.is_empty() || fixture.pixel_hash.is_empty() {
            return Err("golden fixture case hashes must be present".into());
        }
        if fixture.width == 0 || fixture.height == 0 {
            return Err("golden fixture dimensions must be nonzero".into());
        }
        validated_fixture_name(&fixture.rgba)?;
    }
    Ok(())
}

fn fixture_profile_mismatches(
    fixture: &GoldenFixtureProfile,
    active: &QualificationProfile,
) -> Vec<&'static str> {
    let mut mismatches = Vec::new();
    if fixture.backend != active.backend {
        mismatches.push("backend");
    }
    if fixture.adapter_name != active.adapter_name {
        mismatches.push("adapter_name");
    }
    if fixture.driver != active.driver {
        mismatches.push("driver");
    }
    if fixture.driver_info != active.driver_info {
        mismatches.push("driver_info");
    }
    if fixture.vendor_id != active.vendor_id {
        mismatches.push("vendor_id");
    }
    if fixture.device_id != active.device_id {
        mismatches.push("device_id");
    }
    if fixture.surface_format != active.surface_format {
        mismatches.push("surface_format");
    }
    if fixture.operating_system != active.operating_system {
        mismatches.push("operating_system");
    }
    if fixture.architecture != active.architecture {
        mismatches.push("architecture");
    }
    if fixture.adapter_kind != active.adapter_kind {
        mismatches.push("adapter_kind");
    }
    if fixture.memory_class != active.memory_class {
        mismatches.push("memory_class");
    }
    if fixture.timestamp_query_capability != active.timestamp_query_capability {
        mismatches.push("timestamp_query_capability");
    }
    if fixture.hdr_surface_formats_capability != active.hdr_surface_formats_capability {
        mismatches.push("hdr_surface_formats_capability");
    }
    if fixture.max_sampled_textures_per_shader_stage != active.max_sampled_textures_per_shader_stage
    {
        mismatches.push("max_sampled_textures_per_shader_stage");
    }
    if fixture.enabled_features != active.enabled_features {
        mismatches.push("enabled_features");
    }
    if fixture.configuration.power_preference != active.configuration.power_preference {
        mismatches.push("configuration.power_preference");
    }
    if fixture.configuration.preferred_backend != active.configuration.preferred_backend {
        mismatches.push("configuration.preferred_backend");
    }
    if fixture.configuration.allow_software_adapter != active.configuration.allow_software_adapter {
        mismatches.push("configuration.allow_software_adapter");
    }
    if fixture.configuration.present_policy != active.configuration.present_policy {
        mismatches.push("configuration.present_policy");
    }
    if fixture.configuration.desired_maximum_frame_latency
        != active.configuration.desired_maximum_frame_latency
    {
        mismatches.push("configuration.desired_maximum_frame_latency");
    }
    if fixture.configuration.enable_timestamps != active.configuration.enable_timestamps {
        mismatches.push("configuration.enable_timestamps");
    }
    mismatches
}

fn golden_fixture_profile(profile: &QualificationProfile) -> GoldenFixtureProfile {
    GoldenFixtureProfile {
        backend: profile.backend.clone(),
        adapter_name: profile.adapter_name.clone(),
        driver: profile.driver.clone(),
        driver_info: profile.driver_info.clone(),
        vendor_id: profile.vendor_id,
        device_id: profile.device_id,
        surface_format: profile.surface_format.clone(),
        operating_system: profile.operating_system.clone(),
        architecture: profile.architecture.clone(),
        adapter_kind: profile.adapter_kind.clone(),
        memory_class: profile.memory_class.clone(),
        timestamp_query_capability: profile.timestamp_query_capability.clone(),
        hdr_surface_formats_capability: profile.hdr_surface_formats_capability.clone(),
        max_sampled_textures_per_shader_stage: profile.max_sampled_textures_per_shader_stage,
        enabled_features: profile.enabled_features.clone(),
        configuration: profile.configuration.clone(),
    }
}

/// Replaces one fixture through a same-directory, durable temporary file.
///
/// A failure to replace leaves the previous destination intact. The caller
/// writes all raw files before replacing the manifest, so fixture discovery
/// never observes a manifest naming a raw file that was not written first.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "fixture path has no parent directory".to_owned())?;
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "fixture path has no UTF-8 file name".to_owned())?;
    let seed = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    for attempt in 0_u8..16 {
        let temporary = parent.join(format!(
            ".{name}.{}-{}-{attempt}.tmp",
            std::process::id(),
            seed
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary);
        let mut file = match file {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        };
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        drop(file);
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        return Ok(());
    }
    Err("could not allocate a unique temporary fixture path".into())
}

fn validated_fixture_name(value: &str) -> Result<&Path, Box<dyn Error>> {
    let path = Path::new(value);
    let mut components = path.components();
    let Some(Component::Normal(_)) = components.next() else {
        return Err("golden fixture name must be one normal relative path component".into());
    };
    if components.next().is_some() {
        return Err("golden fixture name must not contain a directory".into());
    }
    if path.extension().is_none_or(|extension| extension != "rgba") {
        return Err("golden fixture name must have an .rgba extension".into());
    }
    Ok(path)
}

fn validated_fixture_metadata(path: &Path) -> Result<fs::Metadata, Box<dyn Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("golden fixture must be a regular non-symlink file".into());
    }
    Ok(metadata)
}

fn read_fixture_rgba(
    fixture_directory: &Path,
    fixture: &GoldenFixtureCase,
    expected_len: u64,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture_name = validated_fixture_name(&fixture.rgba)?;
    let fixture_path = fixture_directory.join(fixture_name);
    let metadata = validated_fixture_metadata(&fixture_path)?;
    if metadata.len() != expected_len {
        return Err(format!(
            "golden fixture length differs for {}: expected {expected_len} bytes, found {}",
            fixture.id,
            metadata.len()
        )
        .into());
    }
    let expected = fs::read(&fixture_path)?;
    let expected_hash = ArtifactHash::digest(&expected).to_string();
    if expected_hash != fixture.pixel_hash {
        return Err(format!(
            "golden fixture hash differs for {}: manifest {}, file {}",
            fixture.id, fixture.pixel_hash, expected_hash
        )
        .into());
    }
    Ok(expected)
}

fn source_provenance_from_environment() -> Result<SourceProvenance, Box<dyn Error>> {
    let state = std::env::var("MERIDIAN_SOURCE_STATE").map_err(|_| {
        "MERIDIAN_SOURCE_STATE is required and must be clean-commit or working-tree"
    })?;
    let checkpoint = std::env::var("MERIDIAN_SOURCE_CHECKPOINT").map_err(|_| {
        "MERIDIAN_SOURCE_CHECKPOINT is required for reproducible qualification evidence"
    })?;
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
    let checkpoint = validate_path_free_identifier(checkpoint_value)?;
    if state == SourceState::CleanCommit && !is_lowercase_commit_hash(&checkpoint) {
        return Err(
            "MERIDIAN_SOURCE_CHECKPOINT must be exactly 40 lowercase hexadecimal characters when MERIDIAN_SOURCE_STATE=clean-commit"
                .into(),
        );
    }
    Ok(SourceProvenance { checkpoint, state })
}

fn validate_path_free_identifier(value: &str) -> Result<String, Box<dyn Error>> {
    let allowed = value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'));
    if value.is_empty() || value == "NotAvailable" || value.len() > 512 || !allowed {
        return Err(
            "source checkpoint must be a nonempty path-free identifier of at most 512 ASCII letters, digits, '-', '_', or '.'"
                .into(),
        );
    }
    Ok(value.to_owned())
}

fn is_lowercase_commit_hash(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RunnerOptions {
    evidence_directory: PathBuf,
    write_fixtures: bool,
}

fn runner_options_from_values(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<RunnerOptions, Box<dyn Error>> {
    let mut arguments = arguments.into_iter();
    let mut explicit = None;
    let mut write_fixtures = false;
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            let path = arguments
                .next()
                .ok_or_else(|| "--evidence requires a path".to_owned())?;
            if explicit.replace(PathBuf::from(path)).is_some() {
                return Err("--evidence may be provided only once".into());
            }
        } else if argument == "--write-fixtures" {
            if std::mem::replace(&mut write_fixtures, true) {
                return Err("--write-fixtures may be provided only once".into());
            }
        } else {
            let printable = argument.to_string_lossy();
            return Err(format!("unrecognized qualification argument: {printable}").into());
        }
    }
    let evidence_directory = if let Some(path) = explicit {
        path
    } else {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        PathBuf::from("target/meridian-evidence/ui-direct-qualification")
            .join(format!("{}-{nonce}", std::process::id()))
    };
    Ok(RunnerOptions {
        evidence_directory,
        write_fixtures,
    })
}

fn authorize_fixture_write(source: &SourceProvenance) -> Result<(), Box<dyn Error>> {
    if source.state != SourceState::CleanCommit {
        return Err(
            "--write-fixtures requires MERIDIAN_SOURCE_STATE=clean-commit and a 40-character commit checkpoint"
                .into(),
        );
    }
    if std::env::var(GOLDEN_FIXTURE_WRITE_ENVIRONMENT).as_deref() != Ok("1") {
        return Err(format!(
            "--write-fixtures requires {GOLDEN_FIXTURE_WRITE_ENVIRONMENT}=1; normal qualification never writes repository fixtures"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> QualificationProfile {
        QualificationProfile {
            backend: "Metal".to_owned(),
            adapter_name: "Apple M4".to_owned(),
            driver: "NotAvailable".to_owned(),
            driver_info: "NotAvailable".to_owned(),
            vendor_id: 0,
            device_id: 0,
            surface_format: "Bgra8UnormSrgb".to_owned(),
            operating_system: "macos".to_owned(),
            architecture: "aarch64".to_owned(),
            adapter_kind: "Integrated".to_owned(),
            memory_class: "Unified".to_owned(),
            timestamp_query_capability: "Enabled".to_owned(),
            hdr_surface_formats_capability: "Supported".to_owned(),
            max_sampled_textures_per_shader_stage: 16,
            enabled_features: vec!["TimestampQueries".to_owned()],
            configuration: RhiConfigurationProfile {
                power_preference: "HighPerformance".to_owned(),
                preferred_backend: None,
                allow_software_adapter: false,
                present_policy: "Vsync".to_owned(),
                desired_maximum_frame_latency: 2,
                enable_timestamps: true,
            },
        }
    }

    #[test]
    fn fixture_profile_key_is_stable_and_path_safe() {
        assert_eq!(fixture_profile_key(&profile()), "metal-apple-m4");
        assert_eq!(fixture_token("  Vulkan / A+B  "), "vulkan-a-b");
        assert_eq!(fixture_token("---"), "unknown");
    }

    #[test]
    fn fixture_names_reject_traversal_and_accept_one_file() {
        assert!(validated_fixture_name("metal-apple-m4-standard-1x.rgba").is_ok());
        assert!(validated_fixture_name("not-rgba.txt").is_err());
        assert!(validated_fixture_name("../outside.rgba").is_err());
        assert!(validated_fixture_name("subdir/file.rgba").is_err());
        assert!(validated_fixture_name("/absolute.rgba").is_err());
    }

    fn fixture_manifest() -> GoldenFixtureManifest {
        GoldenFixtureManifest {
            schema: GOLDEN_FIXTURE_SCHEMA.to_owned(),
            version: GOLDEN_FIXTURE_VERSION,
            generator: GOLDEN_FIXTURE_GENERATOR.to_owned(),
            regeneration_command: GOLDEN_FIXTURE_REGENERATION_COMMAND.to_owned(),
            input_schema: UI_DIRECT_QUALIFICATION_SCHEMA.to_owned(),
            source_checkpoint: "meridian.ui-direct-qualification-corpus.v1".to_owned(),
            profile: GoldenFixtureProfile {
                backend: "Metal".to_owned(),
                adapter_name: "Apple M4".to_owned(),
                driver: "NotAvailable".to_owned(),
                driver_info: "NotAvailable".to_owned(),
                vendor_id: 0,
                device_id: 0,
                surface_format: "Bgra8UnormSrgb".to_owned(),
                operating_system: "macos".to_owned(),
                architecture: "aarch64".to_owned(),
                adapter_kind: "Integrated".to_owned(),
                memory_class: "Unified".to_owned(),
                timestamp_query_capability: "Enabled".to_owned(),
                hdr_surface_formats_capability: "Supported".to_owned(),
                max_sampled_textures_per_shader_stage: 16,
                enabled_features: vec!["TimestampQueries".to_owned()],
                configuration: RhiConfigurationProfile {
                    power_preference: "HighPerformance".to_owned(),
                    preferred_backend: None,
                    allow_software_adapter: false,
                    present_policy: "Vsync".to_owned(),
                    desired_maximum_frame_latency: 2,
                    enable_timestamps: true,
                },
            },
            cases: vec![GoldenFixtureCase {
                id: "standard-1x".to_owned(),
                corpus_hash: "fnv1a64:example".to_owned(),
                width: 320,
                height: 180,
                rgba: "metal-apple-m4-standard-1x.rgba".to_owned(),
                pixel_hash: "fixture-pixel-hash".to_owned(),
            }],
        }
    }

    #[test]
    fn fixture_manifest_requires_a_valid_contract_and_exact_rhi_profile() {
        let mut manifest = fixture_manifest();
        assert!(validate_fixture_manifest(&manifest).is_ok());
        assert!(fixture_profile_mismatches(&manifest.profile, &profile()).is_empty());
        manifest.profile.adapter_name = "Other GPU".to_owned();
        assert_eq!(
            fixture_profile_mismatches(&manifest.profile, &profile()),
            vec!["adapter_name"]
        );
        manifest.profile.adapter_name = "Apple M4".to_owned();
        manifest.profile.surface_format = "Rgba8UnormSrgb".to_owned();
        assert_eq!(
            fixture_profile_mismatches(&manifest.profile, &profile()),
            vec!["surface_format"]
        );
        manifest.profile.surface_format = "Bgra8UnormSrgb".to_owned();
        manifest.profile.driver = String::new();
        assert_eq!(
            fixture_profile_mismatches(&manifest.profile, &profile()),
            vec!["driver"]
        );
    }

    #[test]
    fn fixture_manifest_rejects_duplicate_cases_and_invalid_metadata() {
        let mut manifest = fixture_manifest();
        manifest.cases.push(manifest.cases[0].clone());
        assert!(validate_fixture_manifest(&manifest).is_err());
        let mut manifest = fixture_manifest();
        manifest.generator = "untrusted-generator".to_owned();
        assert!(validate_fixture_manifest(&manifest).is_err());
        assert!(validate_path_free_identifier("").is_err());
        assert!(validate_path_free_identifier("checkpoint\n").is_err());
        assert!(validate_path_free_identifier("/Users/example").is_err());
    }

    #[test]
    fn qualification_status_never_promotes_missing_or_failed_goldens() {
        let mut report = CaseReport {
            id: "case".to_owned(),
            corpus_hash: "hash".to_owned(),
            width: 1,
            height: 1,
            frame_id: 1,
            capture_id: 1,
            pixel_hash: "hash".to_owned(),
            png_hash: "hash".to_owned(),
            png: "case.png".to_owned(),
            png_metadata: "case.png.json".to_owned(),
            rgba: "case.rgba".to_owned(),
            golden: GoldenComparisonReport {
                status: "Pass",
                fixture_manifest: None,
                fixture_rgba: None,
                fixture_source_checkpoint: None,
                message: String::new(),
                differing_pixel_count: None,
                maximum_channel_delta: None,
            },
        };
        assert_eq!(qualification_status(&[report.clone()]), "Pass");
        assert_eq!(
            qualification_evidence_status(&[report.clone()]),
            "Inconclusive"
        );
        report.golden.status = "NotRun";
        assert_eq!(qualification_status(&[report.clone()]), "NotRun");
        report.golden.status = "Fail";
        assert_eq!(qualification_status(&[report]), "Fail");
        assert_eq!(qualification_status(&[]), "NotRun");
    }

    #[test]
    fn qualification_failure_status_preserves_typed_unavailable_rhi_errors() {
        assert_eq!(
            QualificationFailure::from_rhi_kind(
                RhiErrorKind::AdapterUnavailable,
                "qualification adapter was unavailable"
            )
            .evidence_status,
            "NotRun"
        );
        assert_eq!(
            QualificationFailure::from_rhi_kind(
                RhiErrorKind::CaptureTargetUnsupported,
                "qualification capture target was unavailable"
            )
            .evidence_status,
            "UnsupportedCapability"
        );
        assert_eq!(
            QualificationFailure::from_error(&std::io::Error::other("fixture mismatch"))
                .evidence_status,
            "Fail"
        );
        let offscreen = UiDirectRendererError::OffscreenCaptureUnsupported {
            rhi_kind: RhiErrorKind::SurfaceUnsupported,
        };
        let failure = QualificationFailure::from_error(&offscreen);
        assert_eq!(failure.evidence_status, "UnsupportedCapability");
        assert_eq!(failure.code, "OffscreenCaptureCopySourceUnsupported");
    }

    #[test]
    fn capture_timeout_is_inconclusive() {
        let failure = QualificationFailure::new(
            "Inconclusive",
            "CaptureTimedOut",
            "qualification pixel readback timed out",
        );
        assert_eq!(failure.evidence_status, "Inconclusive");
        assert_eq!(failure.code, "CaptureTimedOut");
    }

    #[test]
    fn source_provenance_distinguishes_clean_and_working_tree_evidence() {
        let commit = "a804d2b63d8e41338ba629700e1de3df5c9e8adb";
        let clean = source_provenance_from_values("clean-commit", commit).expect("clean source");
        assert_eq!(clean.state, SourceState::CleanCommit);
        let working =
            source_provenance_from_values("working-tree", "ui-direct-qualification-working-tree")
                .expect("working source");
        assert_eq!(working.state, SourceState::WorkingTree);
        assert!(source_provenance_from_values("clean-commit", "working-tree").is_err());
    }

    #[test]
    fn fixture_write_flag_is_explicit_and_argument_parsing_is_bounded() {
        let options = runner_options_from_values(vec![
            std::ffi::OsString::from("--write-fixtures"),
            std::ffi::OsString::from("--evidence"),
            std::ffi::OsString::from("target/evidence"),
        ])
        .expect("explicit fixture options parse");
        assert!(options.write_fixtures);
        assert_eq!(options.evidence_directory, PathBuf::from("target/evidence"));

        assert!(runner_options_from_values(vec![
            std::ffi::OsString::from("--write-fixtures"),
            std::ffi::OsString::from("--write-fixtures"),
        ])
        .is_err());
        assert!(runner_options_from_values(vec![std::ffi::OsString::from("--unknown")]).is_err());
    }

    #[test]
    fn atomic_writer_replaces_complete_file_contents() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("meridian-fixture-atomic-{nonce}"));
        let path = directory.join("fixture.rgba");

        atomic_write(&path, b"old fixture").expect("write old fixture");
        atomic_write(&path, b"new fixture bytes").expect("replace fixture");

        assert_eq!(fs::read(&path).expect("read fixture"), b"new fixture bytes");
        fs::remove_file(&path).expect("remove fixture");
        fs::remove_dir(&directory).expect("remove fixture directory");
    }

    #[test]
    fn fixture_proposals_stay_under_the_evidence_bundle_with_portable_names() {
        let evidence = Path::new("target/meridian-evidence/ui-direct-qualification/example");
        assert_eq!(
            fixture_proposal_directory(evidence),
            evidence.join(FIXTURE_PROPOSAL_DIRECTORY)
        );
        assert_eq!(
            relative_proposal_artifact("metal-apple-m4.json"),
            "fixture-proposal/metal-apple-m4.json"
        );
    }

    #[cfg(unix)]
    #[test]
    fn golden_fixture_rejects_symlinks_before_reading_them() {
        use std::os::unix::fs::symlink;

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("meridian-fixture-link-{nonce}"));
        fs::create_dir_all(&directory).expect("fixture directory");
        let target = directory.join("target.rgba");
        let link = directory.join("linked.rgba");
        fs::write(&target, b"fixture").expect("fixture target");
        symlink(&target, &link).expect("fixture symlink");

        assert!(validated_fixture_metadata(&link).is_err());

        fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn preflight_failure_writes_a_portable_failure_artifact() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("meridian-qualification-preflight-{nonce}"));
        let error = std::io::Error::other("missing source state");

        write_preflight_failure(&directory, "SourceProvenanceError", &error, None)
            .expect("write preflight evidence");
        let artifact = fs::read_to_string(directory.join("qualification-preflight-failure.json"))
            .expect("read preflight evidence");
        assert!(artifact.contains("\"runner_status\": \"Fail\""));
        assert!(artifact.contains("\"evidence_status\": \"Fail\""));
        assert!(artifact.contains("\"code\": \"SourceProvenanceError\""));

        fs::remove_file(directory.join("qualification-preflight-failure.json"))
            .expect("remove preflight evidence");
        fs::remove_dir(&directory).expect("remove preflight directory");
    }
}

fn preflight_evidence_directory(arguments: &[std::ffi::OsString]) -> PathBuf {
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--evidence" {
            if let Some(path) = arguments.next() {
                return PathBuf::from(path);
            }
            break;
        }
    }
    PathBuf::from("target/meridian-evidence/ui-direct-qualification/preflight")
}

fn write_preflight_failure(
    evidence_directory: &Path,
    code: &'static str,
    error: &dyn Error,
    source: Option<&SourceProvenance>,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(evidence_directory)?;
    let report = QualificationFailureReport {
        schema: UI_DIRECT_QUALIFICATION_SCHEMA,
        runner_status: "Fail",
        evidence_status: "Fail",
        code,
        source_checkpoint: source.map(|source| source.checkpoint.clone()),
        source_state: source.map(|source| source.state.as_str()),
        source_provenance_verification: "CallerDeclaredNotVerified",
        evidence_directory: ".",
        stage: "preflight",
        case_id: None,
        environment: qualification_evidence_environment(None, 0),
        profile: None,
        error: sanitize_failure_detail(&error.to_string()),
        limits: [
            "This failure artifact records a preflight failure, not a visual-quality result.",
            "No capture, golden comparison, or package qualification occurred.",
        ],
    };
    write_evidence_json(
        evidence_directory.join("qualification-preflight-failure.json"),
        &report,
    )?;
    Ok(())
}

fn return_preflight_failure(
    evidence_directory: &Path,
    code: &'static str,
    error: Box<dyn Error>,
    source: Option<&SourceProvenance>,
) -> Result<(), Box<dyn Error>> {
    if let Err(write_error) =
        write_preflight_failure(evidence_directory, code, error.as_ref(), source)
    {
        return Err(format!(
            "{error}; additionally failed to write qualification preflight evidence: {write_error}"
        )
        .into());
    }
    Err(error)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let preflight_directory = preflight_evidence_directory(&arguments);
    let options = match runner_options_from_values(arguments) {
        Ok(options) => options,
        Err(error) => {
            return return_preflight_failure(&preflight_directory, "ArgumentError", error, None)
        }
    };
    let source = match source_provenance_from_environment() {
        Ok(source) => source,
        Err(error) => {
            return return_preflight_failure(
                &options.evidence_directory,
                "SourceProvenanceError",
                error,
                None,
            );
        }
    };
    if options.write_fixtures {
        if let Err(error) = authorize_fixture_write(&source) {
            return return_preflight_failure(
                &options.evidence_directory,
                "FixtureWriteUnauthorized",
                error,
                Some(&source),
            );
        }
    }
    if let Err(error) = fs::create_dir_all(&options.evidence_directory) {
        return return_preflight_failure(
            &preflight_directory,
            "EvidenceDirectoryError",
            error.into(),
            Some(&source),
        );
    }
    println!(
        "Meridian direct UI qualification evidence: {}",
        options.evidence_directory.display()
    );
    if options.write_fixtures {
        println!(
            "Meridian direct UI fixture regeneration is enabled; this run will write a proposal and remain NotRun"
        );
    }
    let failure = Arc::new(Mutex::new(None));
    let cases = match ui_direct_qualification_cases() {
        Ok(cases) => cases,
        Err(error) => {
            return return_preflight_failure(
                &options.evidence_directory,
                "CorpusInitializationError",
                Box::new(error),
                Some(&source),
            );
        }
    };
    run(
        PlatformConfig {
            title: "Meridian Direct UI Qualification".to_owned(),
            initial_size: WindowSize::new(320, 180),
            resizable: false,
            visible: false,
            event_loop_mode: EventLoopMode::Wait,
        },
        QualificationRunner {
            evidence_directory: options.evidence_directory,
            source,
            failure: Arc::clone(&failure),
            rhi: None,
            renderer: None,
            profile: None,
            cases,
            case_index: 0,
            plan: None,
            gpu: None,
            capture_deadline: None,
            reports: Vec::new(),
            write_fixtures: options.write_fixtures,
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
