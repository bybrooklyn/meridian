//! Presented-surface review artifact runner for Meridian's direct UI renderer.
//!
//! Unlike the hidden qualification runner, this example maps a native window,
//! submits the canonical 2x framework corpus to that presented surface, and
//! durably writes the pixels copied from the presented surface. The resulting
//! PNG is review input only: the runner never manufactures a human visual-
//! quality verdict, screen-reader evidence, or cross-platform qualification.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_benchmark::{
    has_multiple_pixel_values, write_capture_png, write_capture_rgba, write_evidence_json,
};
use meridian_core::FrameId;
use meridian_platform::{
    run, EventLoopMode, PlatformApplication, PlatformConfig, PlatformContext, PlatformEvent,
    PlatformWindow, WindowSize,
};
use meridian_renderer::{
    ui_direct_qualification_cases, UiDirectFramePlan, UiDirectGpuFrame, UiDirectGpuRenderer,
    UiDirectQualificationCase, UiDirectResourceSet,
};
use meridian_rhi::{
    CaptureOutcome, CaptureRequest, CaptureSource, CapturedFrame, CapturedPixelFormat, ClearColor,
    FrameOutcome, Rhi, RhiConfig,
};
use meridian_ui_core::{
    UiControlState, UiDocument, UiLayout, UiLayoutHints, UiNode, UiNodeId, UiSize, UiStyleVariant,
    UiTextInputOptions, UiTextValidation,
};
use meridian_ui_render::UiEffectCapabilities;
use meridian_ui_runtime::{UiDiagnostic, UiEvent, UiFrameInput, UiRuntime};
use serde::Serialize;

const REPORT_SCHEMA: &str = "meridian.ui-direct-presented-review/v1";
const CANONICAL_REVIEW_CASE: &str = "standard-2x";
const FRAMEWORK_GALLERY_CASE: &str = "framework-gallery-2x";
const FRAMEWORK_GALLERY_VIEWPORT: UiSize = UiSize::new(720.0, 450.0);
const FRAMEWORK_GALLERY_INVALID_FIELD: UiNodeId = UiNodeId::new(0x1_065);
const PRESENTATION_TIMEOUT: Duration = Duration::from_secs(5);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);
const PRESENT_RETRY_DELAY: Duration = Duration::from_millis(50);
const MAX_FAILURE_DETAIL_CHARS: usize = 240;
const REVIEW_CLEAR: ClearColor = ClearColor::new(0.002_731_743, 0.003_346_536, 0.003_346_536, 1.0);

#[derive(Clone, Debug, Eq, PartialEq)]
struct RunnerOptions {
    evidence_directory: PathBuf,
    surface: ReviewSurface,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ReviewSurface {
    #[default]
    Canonical,
    FrameworkGallery,
}

impl ReviewSurface {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "canonical" => Ok(Self::Canonical),
            "framework-gallery" => Ok(Self::FrameworkGallery),
            _ => Err(format!(
                "--surface must be canonical or framework-gallery, got {value:?}"
            )),
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Canonical => "Meridian Direct UI Presented Review",
            Self::FrameworkGallery => "Meridian UI Framework Gallery Review",
        }
    }
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
            Self::CleanCommit => "NotEligiblePendingHumanReviewAndPlatformQualification",
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
struct GpuProfileReport {
    backend: String,
    adapter_name: String,
    driver: String,
    driver_info: String,
    vendor_id: u32,
    device_id: u32,
    adapter_kind: String,
    memory_class: String,
    surface_format: String,
    surface_srgb: bool,
    operating_system: &'static str,
    architecture: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PresentedReviewReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    review_status: &'static str,
    source_checkpoint: String,
    source_state: &'static str,
    source_provenance_verification: &'static str,
    promotion_eligibility: &'static str,
    requirement_ids: [&'static str; 2],
    work_package_id: &'static str,
    research_gate_id: &'static str,
    case_id: &'static str,
    corpus_hash: String,
    profile: GpuProfileReport,
    frame_outcome: String,
    presentation_attempts: u8,
    width: u32,
    height: u32,
    pixel_hash: String,
    png_hash: String,
    png: &'static str,
    png_metadata: &'static str,
    rgba: &'static str,
    evidence_directory: &'static str,
    limits: [&'static str; 4],
}

#[derive(Clone, Debug, Serialize)]
struct PresentedReviewFailureReport {
    schema: &'static str,
    runner_status: &'static str,
    evidence_status: &'static str,
    review_status: &'static str,
    source_checkpoint: String,
    source_state: &'static str,
    stage: &'static str,
    error: String,
    limits: [&'static str; 2],
}

struct PresentedReviewRunner {
    evidence_directory: PathBuf,
    source: SourceProvenance,
    case: UiDirectQualificationCase,
    failure: Arc<Mutex<Option<String>>>,
    rhi: Option<Rhi>,
    plan: Option<UiDirectFramePlan>,
    gpu: Option<UiDirectGpuFrame>,
    profile: Option<GpuProfileReport>,
    presentation_attempts: u8,
    visible_outcome: Option<FrameOutcome>,
    presentation_deadline: Option<Instant>,
    capture_deadline: Option<Instant>,
}

impl PresentedReviewRunner {
    fn stage(&self) -> &'static str {
        if self.rhi.is_none() {
            "initialization"
        } else if self.visible_outcome.is_none() {
            "presentation"
        } else {
            "capture-or-artifact-write"
        }
    }

    fn fail(&mut self, message: impl Into<String>, context: &mut PlatformContext<'_>) {
        let message = sanitize_failure_detail(&message.into());
        {
            let mut failure = self
                .failure
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if failure.is_some() {
                context.exit();
                return;
            }
            *failure = Some(message.clone());
        }
        let report = PresentedReviewFailureReport {
            schema: REPORT_SCHEMA,
            runner_status: "Fail",
            evidence_status: "Inconclusive",
            review_status: "NotRun",
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            stage: self.stage(),
            error: message.clone(),
            limits: [
                "A runner failure is not a renderer-quality or platform-support conclusion.",
                "No human visual, screen-reader, accessibility, or cross-platform review occurred.",
            ],
        };
        let report_error = write_evidence_json(
            self.evidence_directory
                .join("presented-review-failure.json"),
            &report,
        )
        .err();
        let final_message = report_error.map_or(message.clone(), |error| {
            format!("{message}; additionally failed to write failure evidence: {error}")
        });
        *self
            .failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(final_message);
        context.exit();
    }

    fn initialize(&mut self, window: PlatformWindow) -> Result<(), Box<dyn Error>> {
        window.set_visible(true);
        window.request_focus();
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let identity = rhi.render_identity();
        let mut renderer = UiDirectGpuRenderer::new(identity.clone());
        let plan = renderer.prepare_frame(self.case.prepare_request())?;
        let cache_key = plan.cache_key();
        if cache_key.surface_width != identity.surface_size.width
            || cache_key.surface_height != identity.surface_size.height
        {
            return Err(format!(
                "presented review surface is {}x{} but corpus requires {}x{}",
                identity.surface_size.width,
                identity.surface_size.height,
                cache_key.surface_width,
                cache_key.surface_height
            )
            .into());
        }
        let gpu = plan.upload_gpu_frame(&mut rhi)?;
        let capture_bytes = u64::from(cache_key.surface_width)
            .checked_mul(u64::from(cache_key.surface_height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or("presented review capture byte count overflowed")?;
        rhi.request_capture(CaptureRequest::new(
            FrameId::new(1),
            cache_key.surface_width,
            cache_key.surface_height,
            capture_bytes,
        ))?;
        self.profile = Some(gpu_profile(&rhi));
        self.rhi = Some(rhi);
        self.plan = Some(plan);
        self.gpu = Some(gpu);
        self.presentation_deadline = Some(Instant::now() + PRESENTATION_TIMEOUT);
        Ok(())
    }

    fn redraw(&mut self, context: &mut PlatformContext<'_>) -> Result<(), Box<dyn Error>> {
        if let Some(outcome) = self.visible_outcome {
            return self.finish_capture(outcome, context);
        }
        let outcome = self
            .gpu
            .as_ref()
            .ok_or("presented review GPU frame is unavailable")?
            .present(
                self.rhi
                    .as_mut()
                    .ok_or("presented review RHI is unavailable")?,
                self.plan
                    .as_ref()
                    .ok_or("presented review frame plan is unavailable")?,
                REVIEW_CLEAR,
            )?;
        self.presentation_attempts = self.presentation_attempts.saturating_add(1);
        if outcome.visible() {
            self.visible_outcome = Some(outcome);
            self.presentation_deadline = None;
            self.capture_deadline = Some(Instant::now() + CAPTURE_TIMEOUT);
            return self.finish_capture(outcome, context);
        }
        if matches!(
            outcome,
            FrameOutcome::DeviceLost | FrameOutcome::UnsupportedSurface
        ) {
            return Err(format!("presented review surface failed: {outcome:?}").into());
        }
        if self
            .presentation_deadline
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            context.request_redraw_after(PRESENT_RETRY_DELAY);
            return Ok(());
        }
        Err(format!(
            "presented review remained unavailable after {} attempts: {outcome:?}",
            self.presentation_attempts
        )
        .into())
    }

    fn finish_capture(
        &mut self,
        outcome: FrameOutcome,
        context: &mut PlatformContext<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let capture = self
            .rhi
            .as_mut()
            .ok_or("presented review RHI is unavailable")?
            .take_capture();
        let Some(capture) = capture else {
            if self
                .capture_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err("presented review pixel readback timed out".into());
            }
            context.request_redraw();
            return Ok(());
        };
        let CaptureOutcome::Captured(frame) = capture else {
            return Err(format!("presented review capture failed: {capture:?}").into());
        };
        self.validate_capture(&frame, outcome)?;
        let png = write_capture_png(self.evidence_directory.join("presented-review.png"), &frame)?;
        let rgba = write_capture_rgba(
            self.evidence_directory.join("presented-review.rgba"),
            &frame,
        )?;
        if png.metadata.pixel_hash != rgba.pixel_hash {
            return Err("presented review PNG and raw pixel hashes differ".into());
        }
        let report = PresentedReviewReport {
            schema: REPORT_SCHEMA,
            runner_status: "Pass",
            evidence_status: "Inconclusive",
            review_status: "AwaitingHumanReview",
            source_checkpoint: self.source.checkpoint.clone(),
            source_state: self.source.state.as_str(),
            source_provenance_verification: "CallerDeclaredNotVerified",
            promotion_eligibility: self.source.state.promotion_eligibility(),
            requirement_ids: ["REQ-UI-001", "REQ-UI-002"],
            work_package_id: "WP-UI-005",
            research_gate_id: "RG-UI-001",
            case_id: self.case.id,
            corpus_hash: self.case.corpus_hash(),
            profile: self
                .profile
                .clone()
                .ok_or("presented review GPU profile is unavailable")?,
            frame_outcome: format!("{outcome:?}"),
            presentation_attempts: self.presentation_attempts,
            width: frame.width,
            height: frame.height,
            pixel_hash: png.metadata.pixel_hash,
            png_hash: png.metadata.png_hash,
            png: "presented-review.png",
            png_metadata: "presented-review.png.json",
            rgba: "presented-review.rgba",
            evidence_directory: ".",
            limits: [
                "The PNG contains pixels copied from a mapped presented surface, not hidden offscreen output.",
                "Runner Pass proves artifact production only; human visual review is still pending.",
                "One local profile does not establish cross-platform renderer qualification.",
                "This artifact is not screen-reader or accessibility qualification.",
            ],
        };
        write_evidence_json(
            self.evidence_directory.join("presented-review.json"),
            &report,
        )?;
        println!(
            "Meridian direct UI presented review captured {}x{} at {}",
            frame.width,
            frame.height,
            self.evidence_directory.display()
        );
        context.exit();
        Ok(())
    }

    fn validate_capture(
        &self,
        frame: &CapturedFrame,
        outcome: FrameOutcome,
    ) -> Result<(), Box<dyn Error>> {
        let plan = self
            .plan
            .as_ref()
            .ok_or("presented review frame plan is unavailable")?;
        let cache_key = plan.cache_key();
        let expected_bytes = usize::try_from(
            u64::from(cache_key.surface_width)
                .checked_mul(u64::from(cache_key.surface_height))
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or("presented review expected byte count overflowed")?,
        )?;
        if frame.frame_id != FrameId::new(1)
            || frame.width != cache_key.surface_width
            || frame.height != cache_key.surface_height
            || frame.format != CapturedPixelFormat::Rgba8Srgb
            || frame.source != CaptureSource::PresentedSurface
            || frame.surface_outcome != Some(outcome)
            || frame.pixels.len() != expected_bytes
            || !has_multiple_pixel_values(frame)
        {
            return Err(format!("presented review capture metadata is invalid: {frame:?}").into());
        }
        Ok(())
    }
}

impl PlatformApplication for PresentedReviewRunner {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        match event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.fail("presented review window was unavailable", context);
                    return;
                };
                match self.initialize(window) {
                    Ok(()) => context.request_redraw(),
                    Err(error) => self.fail(error.to_string(), context),
                }
            }
            PlatformEvent::RedrawRequested => {
                if let Err(error) = self.redraw(context) {
                    self.fail(error.to_string(), context);
                }
            }
            PlatformEvent::CloseRequested => {
                self.fail(
                    "presented review closed before artifact completion",
                    context,
                );
            }
            _ => {}
        }
    }
}

fn runner_options_from_values(values: Vec<std::ffi::OsString>) -> Result<RunnerOptions, String> {
    let mut values = values.into_iter();
    let mut evidence_directory = None;
    let mut surface = None;
    while let Some(argument) = values.next() {
        if argument == "--evidence" {
            if evidence_directory.is_some() {
                return Err("--evidence may be supplied only once".to_owned());
            }
            let value = values
                .next()
                .ok_or_else(|| "--evidence requires a path".to_owned())?;
            let path = PathBuf::from(value);
            if path.as_os_str().is_empty() {
                return Err("--evidence path cannot be empty".to_owned());
            }
            evidence_directory = Some(path);
        } else if argument == "--surface" {
            if surface.is_some() {
                return Err("--surface may be supplied only once".to_owned());
            }
            let value = values
                .next()
                .ok_or_else(|| "--surface requires a value".to_owned())?;
            surface = Some(ReviewSurface::parse(&value.to_string_lossy())?);
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()));
        }
    }
    Ok(RunnerOptions {
        evidence_directory: evidence_directory
            .ok_or_else(|| "--evidence PATH is required".to_owned())?,
        surface: surface.unwrap_or_default(),
    })
}

fn source_provenance_from_environment() -> Result<SourceProvenance, String> {
    let state = match std::env::var("MERIDIAN_SOURCE_STATE").as_deref() {
        Ok("working-tree") => SourceState::WorkingTree,
        Ok("clean-commit") => SourceState::CleanCommit,
        Ok(value) => {
            return Err(format!(
                "MERIDIAN_SOURCE_STATE must be working-tree or clean-commit, got {value:?}"
            ))
        }
        Err(_) => return Err("MERIDIAN_SOURCE_STATE is required".to_owned()),
    };
    let checkpoint = std::env::var("MERIDIAN_SOURCE_CHECKPOINT")
        .map_err(|_| "MERIDIAN_SOURCE_CHECKPOINT is required".to_owned())?;
    validate_checkpoint(&checkpoint)?;
    Ok(SourceProvenance { checkpoint, state })
}

fn validate_checkpoint(checkpoint: &str) -> Result<(), String> {
    if checkpoint.is_empty() || checkpoint.len() > 96 {
        return Err("source checkpoint must contain 1..=96 characters".to_owned());
    }
    if !checkpoint
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("source checkpoint must be path-free ASCII metadata".to_owned());
    }
    Ok(())
}

fn review_case(surface: ReviewSurface) -> Result<UiDirectQualificationCase, Box<dyn Error>> {
    match surface {
        ReviewSurface::Canonical => ui_direct_qualification_cases()?
            .into_iter()
            .find(|case| case.id == CANONICAL_REVIEW_CASE)
            .ok_or_else(|| format!("qualification corpus omits {CANONICAL_REVIEW_CASE}").into()),
        ReviewSurface::FrameworkGallery => framework_gallery_case(),
    }
}

fn framework_gallery_case() -> Result<UiDirectQualificationCase, Box<dyn Error>> {
    let document = framework_gallery_document()?;
    let mut runtime = UiRuntime::new(document);
    let mut input = UiFrameInput::new(FRAMEWORK_GALLERY_VIEWPORT);
    input.scale_factor = 2.0;
    input.events.push(UiEvent::FocusNext);
    let frame = runtime
        .try_reconcile(input)
        .map_err(|error| format!("framework gallery reconciliation failed: {error:?}"))?;
    if frame.diagnostics
        != [UiDiagnostic::TextValidationFailed {
            node: FRAMEWORK_GALLERY_INVALID_FIELD,
        }]
    {
        return Err(format!(
            "framework gallery emitted unexpected diagnostics: {:?}",
            frame.diagnostics
        )
        .into());
    }
    Ok(UiDirectQualificationCase {
        id: FRAMEWORK_GALLERY_CASE,
        display_revision: frame.revision,
        viewport: FRAMEWORK_GALLERY_VIEWPORT,
        scale_factor: frame.scale_factor,
        contrast: frame.contrast,
        effects: UiEffectCapabilities::default(),
        display_list: frame.display_list.clone(),
        resources: UiDirectResourceSet::default(),
    })
}

#[allow(clippy::too_many_lines)]
fn framework_gallery_document() -> Result<UiDocument, Box<dyn Error>> {
    let id = UiNodeId::new;
    let root = id(0x1_000);
    let header = id(0x1_001);
    let title = id(0x1_002);
    let subtitle = id(0x1_003);
    let badge = id(0x1_004);
    let content = id(0x1_010);
    let controls = id(0x1_020);
    let controls_heading = id(0x1_021);
    let search = id(0x1_022);
    let primary = id(0x1_023);
    let secondary = id(0x1_024);
    let destructive = id(0x1_025);
    let toggle_label = id(0x1_026);
    let toggle = id(0x1_027);
    let disabled = id(0x1_028);
    let data = id(0x1_030);
    let data_heading = id(0x1_031);
    let data_description = id(0x1_032);
    let data_actions = id(0x1_033);
    let filter = id(0x1_034);
    let group = id(0x1_035);
    let export = id(0x1_036);
    let table = id(0x1_040);
    let table_header = id(0x1_041);
    let table_header_name = id(0x1_042);
    let table_header_state = id(0x1_043);
    let table_header_owner = id(0x1_044);
    let table_first = id(0x1_045);
    let table_first_name = id(0x1_046);
    let table_first_state = id(0x1_047);
    let table_first_owner = id(0x1_048);
    let table_second = id(0x1_049);
    let table_second_name = id(0x1_04a);
    let table_second_state = id(0x1_04b);
    let table_second_owner = id(0x1_04c);
    let state = id(0x1_060);
    let state_heading = id(0x1_061);
    let state_status = id(0x1_062);
    let progress = id(0x1_063);
    let field_label = id(0x1_064);
    let invalid_field = FRAMEWORK_GALLERY_INVALID_FIELD;
    let field_help = id(0x1_066);
    let footer = id(0x1_070);
    let footer_status = id(0x1_071);
    let footer_hint = id(0x1_072);

    let fixed_height = |height| UiLayoutHints::fixed_height(height);
    let fixed_width = |width| UiLayoutHints::fixed_width(width);
    let table_cell = |node, name, text, width| {
        UiNode::table_cell(node, name, text).with_layout_hints(fixed_width(width))
    };

    let nodes = vec![
        UiNode::container(
            root,
            "Meridian UI framework gallery",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![header, content, footer],
        )
        .with_style_variant(UiStyleVariant::Canvas),
        UiNode::container(
            header,
            "Gallery header",
            UiLayout::HorizontalStack { gap: 12.0 },
            vec![title, subtitle, badge],
        )
        .with_style_variant(UiStyleVariant::Surface)
        .with_layout_hints(fixed_height(64.0)),
        UiNode::label(title, "Meridian UI", "Meridian UI")
            .with_style_variant(UiStyleVariant::Heading)
            .with_layout_hints(UiLayoutHints::fixed_size(180.0, 32.0)),
        UiNode::label(
            subtitle,
            "Framework gallery description",
            "Retained, accessible, renderer-neutral components",
        )
        .with_style_variant(UiStyleVariant::MutedText)
        .with_layout_hints(fixed_height(24.0)),
        UiNode::label(badge, "Framework version", "FRAMEWORK 1.0")
            .with_style_variant(UiStyleVariant::CompactAction)
            .with_layout_hints(UiLayoutHints::fixed_size(112.0, 28.0)),
        UiNode::container(
            content,
            "Gallery content",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![controls, data, state],
        )
        .with_style_variant(UiStyleVariant::Transparent),
        UiNode::container(
            controls,
            "Control gallery",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                controls_heading,
                search,
                primary,
                secondary,
                destructive,
                toggle_label,
                toggle,
                disabled,
            ],
        )
        .with_style_variant(UiStyleVariant::Surface)
        .with_layout_hints(fixed_width(190.0)),
        UiNode::label(controls_heading, "Controls", "CONTROLS")
            .with_style_variant(UiStyleVariant::SectionHeading)
            .with_layout_hints(fixed_height(24.0)),
        UiNode::search_input(search, "Search components", "Search components")
            .with_layout_hints(fixed_height(42.0)),
        UiNode::button(
            primary,
            "Create component",
            "gallery.create",
            "Create component",
        )
        .with_style_variant(UiStyleVariant::PrimaryAction)
        .with_layout_hints(fixed_height(42.0)),
        UiNode::button(secondary, "Duplicate", "gallery.duplicate", "Duplicate")
            .with_layout_hints(fixed_height(40.0)),
        UiNode::button(
            destructive,
            "Delete component",
            "gallery.delete",
            "Delete component",
        )
        .with_style_variant(UiStyleVariant::DestructiveAction)
        .with_layout_hints(fixed_height(40.0)),
        UiNode::label(toggle_label, "Toggle label", "SNAP TO GRID")
            .with_style_variant(UiStyleVariant::MutedText)
            .with_layout_hints(fixed_height(20.0)),
        UiNode::toggle(toggle, "Snap to grid", "gallery.snap", true)
            .with_layout_hints(fixed_height(40.0)),
        UiNode::button(
            disabled,
            "Unavailable action",
            "gallery.disabled",
            "Unavailable",
        )
        .with_control_state(UiControlState {
            disabled: true,
            ..UiControlState::default()
        })
        .with_layout_hints(fixed_height(40.0)),
        UiNode::container(
            data,
            "Data component gallery",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![data_heading, data_description, data_actions, table],
        )
        .with_style_variant(UiStyleVariant::ElevatedSurface),
        UiNode::label(data_heading, "Data views", "Retained component gallery")
            .with_style_variant(UiStyleVariant::Heading)
            .with_layout_hints(fixed_height(38.0)),
        UiNode::label(
            data_description,
            "Data view description",
            "Stable identity, dense professional controls, and explicit state.",
        )
        .with_style_variant(UiStyleVariant::MutedText)
        .with_layout_hints(fixed_height(24.0)),
        UiNode::container(
            data_actions,
            "Data actions",
            UiLayout::HorizontalStack { gap: 8.0 },
            vec![filter, group, export],
        )
        .with_style_variant(UiStyleVariant::Transparent)
        .with_layout_hints(fixed_height(36.0)),
        UiNode::button(filter, "Filter", "gallery.filter", "Filter")
            .with_style_variant(UiStyleVariant::CompactAction)
            .with_layout_hints(fixed_width(86.0)),
        UiNode::button(group, "Group", "gallery.group", "Group")
            .with_style_variant(UiStyleVariant::CompactAction)
            .with_layout_hints(fixed_width(86.0)),
        UiNode::button(export, "Export", "gallery.export", "Export")
            .with_style_variant(UiStyleVariant::CompactAction)
            .with_layout_hints(fixed_width(86.0)),
        UiNode::table(
            table,
            "Component states",
            vec![table_header, table_first, table_second],
        ),
        UiNode::table_row(
            table_header,
            "Component table header",
            vec![table_header_name, table_header_state, table_header_owner],
        )
        .with_style_variant(UiStyleVariant::Surface)
        .with_layout_hints(fixed_height(42.0)),
        table_cell(table_header_name, "Name heading", "COMPONENT", 150.0)
            .with_style_variant(UiStyleVariant::SectionHeading),
        table_cell(table_header_state, "State heading", "STATE", 100.0)
            .with_style_variant(UiStyleVariant::SectionHeading),
        table_cell(table_header_owner, "Owner heading", "OWNER", 110.0)
            .with_style_variant(UiStyleVariant::SectionHeading),
        UiNode::table_row(
            table_first,
            "Button component row",
            vec![table_first_name, table_first_state, table_first_owner],
        )
        .with_layout_hints(fixed_height(48.0)),
        table_cell(table_first_name, "Button component", "Button", 150.0),
        table_cell(table_first_state, "Button state", "Focused", 100.0),
        table_cell(table_first_owner, "Button owner", "Runtime", 110.0),
        UiNode::table_row(
            table_second,
            "Property grid component row",
            vec![table_second_name, table_second_state, table_second_owner],
        )
        .with_layout_hints(fixed_height(48.0)),
        table_cell(
            table_second_name,
            "Property grid component",
            "Property grid",
            150.0,
        ),
        table_cell(table_second_state, "Property grid state", "Editing", 100.0),
        table_cell(table_second_owner, "Property grid owner", "Editor", 110.0),
        UiNode::container(
            state,
            "State and token gallery",
            UiLayout::VerticalStack { gap: 8.0 },
            vec![
                state_heading,
                state_status,
                progress,
                field_label,
                invalid_field,
                field_help,
            ],
        )
        .with_style_variant(UiStyleVariant::Surface)
        .with_layout_hints(fixed_width(210.0)),
        UiNode::label(state_heading, "States and tokens", "STATE & TOKENS")
            .with_style_variant(UiStyleVariant::SectionHeading)
            .with_layout_hints(fixed_height(24.0)),
        UiNode::label(state_status, "Framework status", "Renderer connected")
            .with_style_variant(UiStyleVariant::MutedText)
            .with_layout_hints(fixed_height(24.0)),
        UiNode::progress(progress, "Gallery completeness", 68)
            .with_layout_hints(fixed_height(46.0)),
        UiNode::label(field_label, "Validation label", "VALIDATION")
            .with_style_variant(UiStyleVariant::MutedText)
            .with_layout_hints(fixed_height(20.0)),
        UiNode::text_input(
            invalid_field,
            "Required token name",
            "",
            UiTextInputOptions::default(),
        )
        .with_text_validation(UiTextValidation::NonEmpty)
        .with_control_state(UiControlState {
            invalid: true,
            ..UiControlState::default()
        })
        .with_layout_hints(fixed_height(42.0)),
        UiNode::label(field_help, "Validation help", "A token name is required.")
            .with_style_variant(UiStyleVariant::MutedText)
            .with_layout_hints(fixed_height(22.0)),
        UiNode::container(
            footer,
            "Gallery status bar",
            UiLayout::HorizontalStack { gap: 12.0 },
            vec![footer_status, footer_hint],
        )
        .with_style_variant(UiStyleVariant::Surface)
        .with_layout_hints(fixed_height(32.0)),
        UiNode::label(footer_status, "Framework state", "READY · 2× DIRECT")
            .with_style_variant(UiStyleVariant::MutedText)
            .with_layout_hints(fixed_width(180.0)),
        UiNode::label(
            footer_hint,
            "Keyboard hint",
            "Tab moves focus · Enter activates · Escape cancels",
        )
        .with_style_variant(UiStyleVariant::MutedText),
    ];
    UiDocument::new(root, nodes)
        .map_err(|error| format!("gallery document rejected: {error:?}").into())
}

fn physical_size(case: &UiDirectQualificationCase) -> Result<WindowSize, Box<dyn Error>> {
    let width = case.viewport.width * case.scale_factor;
    let height = case.viewport.height * case.scale_factor;
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width.fract() != 0.0
        || height.fract() != 0.0
        || f64::from(width) > f64::from(u32::MAX)
        || f64::from(height) > f64::from(u32::MAX)
    {
        return Err("qualification case does not resolve to a bounded integer surface".into());
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(WindowSize::new(width as u32, height as u32))
}

fn gpu_profile(rhi: &Rhi) -> GpuProfileReport {
    let capabilities = rhi.capabilities();
    let surface = rhi.surface_format();
    GpuProfileReport {
        backend: format!("{:?}", capabilities.backend),
        adapter_name: capabilities.adapter_name.clone(),
        driver: capabilities.driver.clone(),
        driver_info: capabilities.driver_info.clone(),
        vendor_id: capabilities.vendor_id,
        device_id: capabilities.device_id,
        adapter_kind: format!("{:?}", capabilities.adapter_kind),
        memory_class: format!("{:?}", capabilities.memory_class),
        surface_format: surface.name,
        surface_srgb: surface.srgb,
        operating_system: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
    }
}

fn sanitize_failure_detail(detail: &str) -> String {
    detail
        .chars()
        .filter(|character| !character.is_control() || *character == ' ')
        .take(MAX_FAILURE_DETAIL_CHARS)
        .collect()
}

fn prepare_evidence_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() && fs::read_dir(path)?.next().transpose()?.is_some() {
        return Err(format!("evidence directory is not empty: {}", path.display()).into());
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = runner_options_from_values(std::env::args_os().skip(1).collect())?;
    let source = source_provenance_from_environment()?;
    prepare_evidence_directory(&options.evidence_directory)?;
    let case = review_case(options.surface)?;
    let size = physical_size(&case)?;
    println!(
        "Meridian direct UI presented review evidence: {}",
        options.evidence_directory.display()
    );
    let failure = Arc::new(Mutex::new(None));
    run(
        PlatformConfig {
            title: options.surface.title().to_owned(),
            initial_size: size,
            resizable: false,
            visible: true,
            event_loop_mode: EventLoopMode::Poll,
        },
        PresentedReviewRunner {
            evidence_directory: options.evidence_directory,
            source,
            case,
            failure: Arc::clone(&failure),
            rhi: None,
            plan: None,
            gpu: None,
            profile: None,
            presentation_attempts: 0,
            visible_outcome: None,
            presentation_deadline: None,
            capture_deadline: None,
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

    #[test]
    fn arguments_require_one_explicit_evidence_directory() {
        assert!(runner_options_from_values(Vec::new()).is_err());
        assert!(runner_options_from_values(vec!["--unknown".into()]).is_err());
        assert!(runner_options_from_values(vec!["--evidence".into()]).is_err());
        assert_eq!(
            runner_options_from_values(vec!["--evidence".into(), "target/review".into()]),
            Ok(RunnerOptions {
                evidence_directory: PathBuf::from("target/review"),
                surface: ReviewSurface::Canonical,
            })
        );
        assert_eq!(
            runner_options_from_values(vec![
                "--evidence".into(),
                "target/gallery".into(),
                "--surface".into(),
                "framework-gallery".into(),
            ]),
            Ok(RunnerOptions {
                evidence_directory: PathBuf::from("target/gallery"),
                surface: ReviewSurface::FrameworkGallery,
            })
        );
    }

    #[test]
    fn review_case_has_exact_bounded_two_x_surface() {
        let case = review_case(ReviewSurface::Canonical).expect("review case constructs");
        assert_eq!(case.id, CANONICAL_REVIEW_CASE);
        assert_eq!(
            physical_size(&case).expect("review size resolves"),
            WindowSize::new(640, 360)
        );
    }

    #[test]
    fn framework_gallery_is_a_real_retained_two_x_surface() {
        let case = review_case(ReviewSurface::FrameworkGallery).expect("gallery case constructs");
        assert_eq!(case.id, FRAMEWORK_GALLERY_CASE);
        assert_eq!(
            physical_size(&case).expect("gallery size resolves"),
            WindowSize::new(1440, 900)
        );
        assert!(case.display_list.primitives.len() >= 50);
        let identity = meridian_rhi::RhiRenderIdentity {
            device_generation: 1,
            surface_generation: 1,
            surface_format: meridian_rhi::SurfaceFormat {
                name: "Bgra8UnormSrgb".to_owned(),
                srgb: true,
            },
            surface_size: WindowSize::new(1440, 900),
            surface_configured: true,
        };
        let plan = UiDirectGpuRenderer::new(identity)
            .prepare_frame(case.prepare_request())
            .expect("the retained framework gallery prepares without adapter-specific state");
        assert!(plan.footprint().cpu_atlas_bytes > 0);
    }

    #[test]
    fn source_checkpoint_rejects_paths_and_control_characters() {
        assert!(validate_checkpoint("wp-ui-005-local").is_ok());
        assert!(validate_checkpoint("../private/path").is_err());
        assert!(validate_checkpoint("bad\ncheckpoint").is_err());
    }
}
