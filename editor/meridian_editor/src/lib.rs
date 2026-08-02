//! MS-01 Meridian application composition and qualification smoke.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use meridian_alluvium::{
    dirty_report, explain as explain_generated, license_audit, parse_stable_id, AlluviumEngine,
    EvaluationBudget, EvaluationMode, EvaluationRequest, GeneratedOverride, OverrideAction,
    OverrideStatus, ProceduralRecipe, RECIPE_SCHEMA,
};
use meridian_asset_tools::{
    decode_compiled_visual, AssetImportDatabase, CompiledVisualFacet, ImportedFixtureMesh,
};
use meridian_assets::{
    ArtifactHash, AssetId, AssetLoadRequest, AssetLoadResult, CancellationToken, PackIndexEntry,
    SourceId, UncompressedDecoder,
};
use meridian_benchmark::{has_multiple_pixel_values, write_capture_png};
use meridian_build::{
    ArtifactStore, BuildGraph, BuildIdentityInput, BuildNode, BuildNodeId, BuildPhase,
    BuildRequest, BuildServiceStore, CargoBuildSupervisor, CargoCommand, CargoEnvironment,
    CargoInvocation, CargoRunStatus, DurableBuildService,
};
use meridian_core::{FrameId, MonotonicNs, OperationId, RuntimeEpoch, StableId, TraceId};
use meridian_diagnostics::{
    DiagnosticEvent, DiagnosticSeverity, DiagnosticTimeline, RecoveryAction, RedactionClass,
};
use meridian_editor_core::{
    CommandMetadata, EditorCommand, EditorError, EditorSession, ImportedSource, ProjectDocument,
    ProjectRecoveryStatus, ProjectStore, Translation, WorldPlacement,
};
use meridian_input::{
    Action, ButtonControl, InputActionMap, InputState, KeyCode, MouseButton as NativeMouseButton,
    NativeInputEvent, NativeScrollEvent, NativeScrollPhase, NativeScrollUnit,
};
use meridian_modeler::{
    Edge, Millimetres3, ModelCommand, ModelDocument, ModelElementKind, ModelRecoveryStore,
    ModelSelection, ModelSession, ModelTransaction, PrimitiveCreate, QuadPrimitive,
    QuadPrimitiveIds, SplitEdge, TopologyMap, Vertex,
};
use meridian_package::{MountedPackage, PackageBuilder, PackageChunk, PackageLimits};
use meridian_platform::{
    run as run_platform, EventLoopMode, PlatformAccessibilityActionData,
    PlatformAccessibilityActionRequest, PlatformApplication, PlatformConfig, PlatformContext,
    PlatformError, PlatformEvent, PlatformEventEnvelope, PlatformImeCursorArea, PlatformModifiers,
    PlatformWindow, RuntimeLifecycle, SurfaceSignal, WindowSize,
};
use meridian_renderer::{
    FoundationMeshDescriptor, MaterialHandle, MeshHandle, PenumbraFoundationRenderer,
    RenderInstanceId, RenderInstanceSource, Transform, UiDirectFramePlan, UiDirectGpuFrame,
    UiDirectGpuRenderer, UiDirectPrepareRequest, UiDirectResourceSet, UiOverlayRenderer,
};
use meridian_rhi::{
    CaptureFailure, CaptureOutcome, CaptureRequest, CaptureSource, CapturedFrame, ClearColor,
    FrameOutcome, GpuCapabilities, GpuTimingOutcome, PassTimingSample, Rhi, RhiConfig,
    RhiErrorKind, TimingFrameId,
};
use meridian_rt::EngineRuntime;
use meridian_save::{
    ComponentDelta, SaveConfig, SaveJournal, SaveMigrations, SaveState, SaveStore, SaveTransaction,
};
use meridian_streaming::{
    ActivationQueue, ActivationWork, CellLoadCoordinator, CellRequest, CellResidencyState,
    StreamingScheduler,
};
use meridian_tasks::{TaskClass, TaskContext};
use meridian_ui::{
    recovery_panel_document, runtime_overlay_document, CommandId, MotionPreference, SemanticAction,
    SemanticDelta, SemanticTree, UiAssistiveRequest, UiCollectionNavigation, UiCommandRequest,
    UiContrast, UiDensity, UiDiagnostic, UiDocumentCompiler, UiEffectCapabilities, UiEvent,
    UiFrameInput, UiFrameOutput, UiInputDeviceId, UiInputDeviceKind, UiNodeId, UiPoint,
    UiPointerButton, UiPointerEvent, UiPointerPhase, UiRuntime, UiScrollDelta, UiScrollEvent,
    UiScrollPhase, UiScrollUnit, UiSize, UiTextCursorDirection, UiWidgetKind,
};
use meridian_ui_editor::{
    creator_hub_document, creator_settings_document, creator_ui_authoring_target_frame,
    creator_workspace_document, creator_workspace_document_with_view, decorate_modeler_preview,
    decorate_ui_authoring_preview, decorate_world_viewport, model_inspector_document,
    recipe_inspector_document, CodeContextWidth, CreatorModelerPresentation, CreatorSettingsView,
    CreatorWorkspaceView, DockAxis, DockNode, DockNodeId, DockTab, DockTree, EditorPanelId,
    PanelId, RecentProjectView, WorkspaceActivation, WorkspaceExtensions, WorkspaceKind,
    WorkspaceLayout, WorkspaceSessionId, WorkspaceStateDocument, WorkspaceStateStore,
    CREATOR_HUB_PROJECT_NAME, CREATOR_INSPECTOR_X_MM, CREATOR_INSPECTOR_Y_MM,
    CREATOR_INSPECTOR_Z_MM, CREATOR_SETTINGS_SEARCH,
};
use meridian_world::{CompiledWorldCell, SpatialDatabase};
use meridian_world_tools::compile_world_source;
use serde::{Deserialize, Serialize};
use serde_json::json;

const MESH_SOURCE: &str = "assets_source/ms01/fixture_triangle.json";
const WORLD_SOURCE: &str = "assets_source/ms01/world_cell.json";
const DEFAULT_EVIDENCE: &str = "target/meridian-evidence/ms01";
const DEFAULT_CAPTURE: &str = "visible-source-frame.png";
const CREATOR_ALPHA_MANIFEST: &str = "creator-alpha.project.json";
const CREATOR_ALPHA_SCHEMA: &str = "meridian.creator-alpha/v1";
const CREATOR_ALPHA_EVIDENCE_SCHEMA: &str = "meridian.creator-alpha-evidence/v1";
// Creator is a workbench, not a dialog. Start large enough for the World
// viewport to remain the visual centre while source and inspector panels are
// both useful; the platform still lets a user resize it normally.
const CREATOR_INITIAL_WINDOW: WindowSize = WindowSize::new(1600, 960);
const CREATOR_INITIAL_VIEWPORT_WIDTH: f32 = 1600.0;
const CREATOR_INITIAL_VIEWPORT_HEIGHT: f32 = 960.0;
const CREATOR_REVIEW_MIN_SIZE: WindowSize = WindowSize::new(1024, 720);
const CREATOR_REVIEW_MAX_SIZE: WindowSize = WindowSize::new(4096, 4096);
const CREATOR_PROJECT_SOURCE: &str = "project.meridian.json";
const CREATOR_INTERNAL_DIRECTORY: &str = ".meridian";
const CREATOR_WORKSPACE_STATE: &str = "workspace-state.state";
const CREATOR_DEFAULT_LAYOUT: &str = "Default";
const CREATOR_HUB_SCHEMA_V1: &str = "meridian.launch-hub/v1";
const CREATOR_HUB_SCHEMA: &str = "meridian.launch-hub/v2";
const CREATOR_RECENT_LIMIT: usize = 16;
const CREATOR_ALPHA_BUILD_TIMEOUT: Duration = Duration::from_mins(1);
const VISUAL_ASSET_NAME: &str = "fixtures/ms01/public-triangle.visual";
const COLLISION_ASSET_NAME: &str = "fixtures/ms01/public-triangle.collision";
const CELL_ASSET_NAME: &str = "fixtures/ms01/world-cell-0-0-0";
const SAVE_COMPONENT: &str = "meridian.ms01.position";
const EVIDENCE_CAPACITY: usize = 512;
const UI_SMOKE_MAX_PRESENT_ATTEMPTS: u8 = 3;
const CREATOR_UI_SMOKE_VISIBLE_PRESENTATIONS: u8 = 2;
const CREATOR_POINTER_DEVICE: UiInputDeviceId = UiInputDeviceId::new(1);
static NEXT_DEFAULT_EVIDENCE_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_CREATOR_BUILD_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_CREATOR_SMOKE_PROJECT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_ATOMIC_WRITE_ID: AtomicU64 = AtomicU64::new(0);

type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Interactive,
    Smoke,
    HeadlessSmoke,
    UiHeadlessSmoke,
    UiSmoke,
    CreatorAlphaSmoke,
    CreatorAlphaUiSmoke,
    CreatorAlphaUiReview,
    AlluviumCommand,
}

/// Text-first Alluvium commands. Every variant returns structured JSON and uses
/// the same recipe contracts as the editor-facing inspector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlluviumCommand {
    Inspect { recipe: PathBuf },
    Validate { recipe: PathBuf },
    Migrate { recipe: PathBuf, schema: u32 },
    Preview { recipe: PathBuf, region: String },
    Bake { recipe: PathBuf, profile: String },
    Dirty { recipe: PathBuf, since: PathBuf },
    Explain { recipe: PathBuf, object: String },
    Diff { recipe: PathBuf, against: PathBuf },
    Provenance { recipe: PathBuf, output: String },
    LicenseAudit { recipe: PathBuf, target: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeridianOptions {
    pub mode: RunMode,
    pub project: Option<PathBuf>,
    pub capture: Option<PathBuf>,
    /// An in-memory Creator workspace selection used only by local UI review.
    ///
    /// This never changes persisted workspace preferences or project source.
    pub review_workspace: Option<WorkspaceKind>,
    /// Explicit logical review size for local Creator visual inspection only.
    pub review_size: Option<WindowSize>,
    pub evidence: Option<PathBuf>,
    pub frames: u32,
    pub alluvium: Option<AlluviumCommand>,
}

impl Default for MeridianOptions {
    fn default() -> Self {
        Self {
            mode: RunMode::Interactive,
            project: None,
            capture: None,
            review_workspace: None,
            review_size: None,
            evidence: None,
            frames: 120,
            alluvium: None,
        }
    }
}

impl MeridianOptions {
    /// Parses bounded Meridian application arguments.
    ///
    /// # Errors
    ///
    /// Rejects unknown flags, missing values, conflicting smoke modes, and
    /// frame counts outside `1..=10000`.
    pub fn parse<I, S>(arguments: I) -> Result<Self, MeridianArgumentError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut options = Self::default();
        let mut arguments = arguments.into_iter().map(Into::into);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--smoke" => options.set_mode(RunMode::Smoke)?,
                "--headless-smoke" => options.set_mode(RunMode::HeadlessSmoke)?,
                "--ui-headless-smoke" => options.set_mode(RunMode::UiHeadlessSmoke)?,
                "--ui-smoke" => options.set_mode(RunMode::UiSmoke)?,
                "--creator-alpha-smoke" => options.set_mode(RunMode::CreatorAlphaSmoke)?,
                "--creator-alpha-ui-smoke" => options.set_mode(RunMode::CreatorAlphaUiSmoke)?,
                "--creator-alpha-ui-review" => options.set_mode(RunMode::CreatorAlphaUiReview)?,
                "alluvium" => {
                    options.set_mode(RunMode::AlluviumCommand)?;
                    options.alluvium = Some(parse_alluvium_command(&mut arguments)?);
                    break;
                }
                "--project" => options.project = Some(next_path(&mut arguments, "--project")?),
                "--capture" => options.capture = Some(next_path(&mut arguments, "--capture")?),
                "--review-workspace" => {
                    let value = arguments
                        .next()
                        .ok_or(MeridianArgumentError::MissingValue("--review-workspace"))?;
                    options.review_workspace = Some(parse_creator_review_workspace(&value)?);
                }
                "--review-size" => {
                    let value = arguments
                        .next()
                        .ok_or(MeridianArgumentError::MissingValue("--review-size"))?;
                    options.review_size = Some(parse_creator_review_size(&value)?);
                }
                "--evidence" => options.evidence = Some(next_path(&mut arguments, "--evidence")?),
                "--frames" => {
                    let value = arguments
                        .next()
                        .ok_or(MeridianArgumentError::MissingValue("--frames"))?;
                    options.frames = value
                        .parse::<u32>()
                        .map_err(|_| MeridianArgumentError::InvalidFrameCount(value))?;
                    if options.frames == 0 || options.frames > 10_000 {
                        return Err(MeridianArgumentError::FrameCountOutOfRange(options.frames));
                    }
                }
                "--help" | "-h" => return Err(MeridianArgumentError::HelpRequested),
                _ => return Err(MeridianArgumentError::UnknownArgument(argument)),
            }
        }
        if matches!(
            options.mode,
            RunMode::CreatorAlphaSmoke
                | RunMode::CreatorAlphaUiSmoke
                | RunMode::CreatorAlphaUiReview
        ) {
            if options.project.is_none() {
                return Err(MeridianArgumentError::CreatorAlphaProjectRequired);
            }
            if options.mode == RunMode::CreatorAlphaSmoke && options.evidence.is_none() {
                return Err(MeridianArgumentError::CreatorAlphaEvidenceRequired);
            }
        }
        if options.mode == RunMode::AlluviumCommand && options.alluvium.is_none() {
            return Err(MeridianArgumentError::AlluviumCommandRequired);
        }
        if options.review_workspace.is_some() && options.mode != RunMode::CreatorAlphaUiReview {
            return Err(MeridianArgumentError::ReviewWorkspaceRequiresUiReview);
        }
        if options.review_size.is_some() && options.mode != RunMode::CreatorAlphaUiReview {
            return Err(MeridianArgumentError::ReviewSizeRequiresUiReview);
        }
        Ok(options)
    }

    fn set_mode(&mut self, mode: RunMode) -> Result<(), MeridianArgumentError> {
        if self.mode != RunMode::Interactive && self.mode != mode {
            return Err(MeridianArgumentError::ConflictingModes);
        }
        self.mode = mode;
        Ok(())
    }
}

fn parse_creator_review_workspace(value: &str) -> Result<WorkspaceKind, MeridianArgumentError> {
    match value {
        "hub" => Ok(WorkspaceKind::Hub),
        "world" => Ok(WorkspaceKind::World),
        "code" => Ok(WorkspaceKind::Code),
        "modeler" => Ok(WorkspaceKind::Modeler),
        "ui" => Ok(WorkspaceKind::UiAuthoring),
        "materials" => Ok(WorkspaceKind::Materials),
        "alluvium" => Ok(WorkspaceKind::Alluvium),
        "build" => Ok(WorkspaceKind::Build),
        "profile" => Ok(WorkspaceKind::Profile),
        "settings" => Ok(WorkspaceKind::Settings),
        "recovery" => Ok(WorkspaceKind::Recovery),
        _ => Err(MeridianArgumentError::InvalidReviewWorkspace(
            value.to_owned(),
        )),
    }
}

fn parse_creator_review_size(value: &str) -> Result<WindowSize, MeridianArgumentError> {
    let Some((width, height)) = value.split_once('x').or_else(|| value.split_once('X')) else {
        return Err(MeridianArgumentError::InvalidReviewSize(value.to_owned()));
    };
    let (Ok(width), Ok(height)) = (width.parse::<u32>(), height.parse::<u32>()) else {
        return Err(MeridianArgumentError::InvalidReviewSize(value.to_owned()));
    };
    let size = WindowSize::new(width, height);
    if size.width < CREATOR_REVIEW_MIN_SIZE.width
        || size.height < CREATOR_REVIEW_MIN_SIZE.height
        || size.width > CREATOR_REVIEW_MAX_SIZE.width
        || size.height > CREATOR_REVIEW_MAX_SIZE.height
    {
        return Err(MeridianArgumentError::InvalidReviewSize(value.to_owned()));
    }
    Ok(size)
}

fn parse_alluvium_command(
    arguments: &mut impl Iterator<Item = String>,
) -> Result<AlluviumCommand, MeridianArgumentError> {
    let command = arguments
        .next()
        .ok_or(MeridianArgumentError::AlluviumCommandRequired)?;
    let recipe = next_path(arguments, "<recipe.mproc>")?;
    let parsed = match command.as_str() {
        "inspect" => AlluviumCommand::Inspect { recipe },
        "validate" => AlluviumCommand::Validate { recipe },
        "migrate" => AlluviumCommand::Migrate {
            recipe,
            schema: next_alluvium_option(&command, arguments, "--to")?
                .parse()
                .map_err(|_| {
                    MeridianArgumentError::AlluviumSyntax(
                        "--to must be an integer schema version".to_owned(),
                    )
                })?,
        },
        "preview" => AlluviumCommand::Preview {
            recipe,
            region: next_alluvium_option(&command, arguments, "--region")?,
        },
        "bake" => AlluviumCommand::Bake {
            recipe,
            profile: next_alluvium_option(&command, arguments, "--profile")?,
        },
        "dirty" => AlluviumCommand::Dirty {
            recipe,
            since: PathBuf::from(next_alluvium_option(&command, arguments, "--since")?),
        },
        "explain" => AlluviumCommand::Explain {
            recipe,
            object: next_alluvium_option(&command, arguments, "--object")?,
        },
        "diff" => AlluviumCommand::Diff {
            recipe,
            against: PathBuf::from(next_alluvium_option(&command, arguments, "--against")?),
        },
        "provenance" => AlluviumCommand::Provenance {
            recipe,
            output: next_alluvium_option(&command, arguments, "--output")?,
        },
        "license-audit" => AlluviumCommand::LicenseAudit {
            recipe,
            target: next_alluvium_option(&command, arguments, "--target")?,
        },
        _ => {
            return Err(MeridianArgumentError::AlluviumSyntax(format!(
                "unknown Alluvium command: {command}"
            )))
        }
    };
    if let Some(unexpected) = arguments.next() {
        return Err(MeridianArgumentError::AlluviumSyntax(format!(
            "unexpected Alluvium argument: {unexpected}"
        )));
    }
    Ok(parsed)
}

fn next_alluvium_option(
    command: &str,
    arguments: &mut impl Iterator<Item = String>,
    expected: &'static str,
) -> Result<String, MeridianArgumentError> {
    match arguments.next().as_deref() {
        Some(value) if value == expected => arguments
            .next()
            .ok_or(MeridianArgumentError::MissingValue(expected)),
        Some(value) => Err(MeridianArgumentError::AlluviumSyntax(format!(
            "{command} expected {expected}, found {value}"
        ))),
        None => Err(MeridianArgumentError::AlluviumSyntax(format!(
            "{command} requires {expected}"
        ))),
    }
}

fn next_path(
    arguments: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<PathBuf, MeridianArgumentError> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or(MeridianArgumentError::MissingValue(flag))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeridianArgumentError {
    MissingValue(&'static str),
    UnknownArgument(String),
    InvalidFrameCount(String),
    FrameCountOutOfRange(u32),
    InvalidReviewWorkspace(String),
    InvalidReviewSize(String),
    ReviewWorkspaceRequiresUiReview,
    ReviewSizeRequiresUiReview,
    ConflictingModes,
    CreatorAlphaProjectRequired,
    CreatorAlphaEvidenceRequired,
    AlluviumCommandRequired,
    AlluviumSyntax(String),
    HelpRequested,
}

impl Display for MeridianArgumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(flag) => write!(formatter, "missing value for {flag}"),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument: {argument}"),
            Self::InvalidFrameCount(value) => write!(formatter, "invalid frame count: {value}"),
            Self::FrameCountOutOfRange(value) => {
                write!(formatter, "frame count {value} is outside 1..=10000")
            }
            Self::InvalidReviewWorkspace(value) => write!(
                formatter,
                "invalid Creator review workspace: {value} (expected hub, world, code, modeler, ui, materials, alluvium, build, profile, settings, or recovery)"
            ),
            Self::InvalidReviewSize(value) => write!(
                formatter,
                "invalid Creator review size: {value} (expected WIDTHxHEIGHT within 1024x720..=4096x4096)"
            ),
            Self::ReviewWorkspaceRequiresUiReview => write!(
                formatter,
                "--review-workspace is available only with --creator-alpha-ui-review"
            ),
            Self::ReviewSizeRequiresUiReview => write!(
                formatter,
                "--review-size is available only with --creator-alpha-ui-review"
            ),
            Self::ConflictingModes => formatter.write_str("smoke modes are mutually exclusive"),
            Self::CreatorAlphaProjectRequired => {
                formatter.write_str("Creator Alpha smoke modes require --project PATH")
            }
            Self::CreatorAlphaEvidenceRequired => {
                formatter.write_str("--creator-alpha-smoke requires --evidence PATH")
            }
            Self::AlluviumCommandRequired => {
                formatter.write_str("alluvium requires a structured command")
            }
            Self::AlluviumSyntax(detail) => write!(formatter, "Alluvium command syntax: {detail}"),
            Self::HelpRequested => formatter.write_str(usage()),
        }
    }
}

impl Error for MeridianArgumentError {}

#[must_use]
pub const fn usage() -> &'static str {
    "Meridian\n\nUsage: meridian [--smoke | --headless-smoke | --ui-headless-smoke | --ui-smoke | --creator-alpha-smoke --project PATH --evidence PATH | --creator-alpha-ui-smoke --project PATH | --creator-alpha-ui-review --project PATH [--review-workspace WORKSPACE] [--review-size WIDTHxHEIGHT] [--capture PATH]] [--project PATH] [--capture PATH] [--evidence PATH] [--frames N]\n       meridian alluvium <inspect|validate|migrate|preview|bake|dirty|explain|diff|provenance|license-audit> <recipe.mproc> [required command option]"
}

/// Runs the requested Meridian application mode.
///
/// # Errors
///
/// Returns source, package, streaming, save, platform, rendering, capture, or
/// evidence IO failures without claiming milestone completion.
pub fn run(options: &MeridianOptions) -> AppResult<()> {
    if let Some(command) = &options.alluvium {
        return run_alluvium_command(command);
    }
    if options.mode == RunMode::UiHeadlessSmoke {
        return run_ui_headless_smoke();
    }
    if options.mode == RunMode::UiSmoke {
        return run_ui_native_smoke();
    }
    if options.mode == RunMode::CreatorAlphaSmoke {
        return run_creator_alpha_smoke(options);
    }
    if options.mode == RunMode::CreatorAlphaUiSmoke {
        return run_creator_alpha_ui_smoke(options);
    }
    if options.mode == RunMode::CreatorAlphaUiReview {
        return run_creator_alpha_ui_review(options);
    }
    if options.mode == RunMode::Interactive {
        return run_creator_application(options.project.as_deref(), false, None, None, None);
    }
    let project_root = resolve_project_root(options.project.as_deref())?;
    let evidence_root = options.evidence.as_deref().map_or_else(
        || match options.mode {
            RunMode::Smoke | RunMode::HeadlessSmoke => default_evidence_root(&project_root),
            RunMode::Interactive => unreachable!("interactive Creator launch handled above"),
            RunMode::UiHeadlessSmoke
            | RunMode::UiSmoke
            | RunMode::CreatorAlphaSmoke
            | RunMode::CreatorAlphaUiSmoke
            | RunMode::CreatorAlphaUiReview
            | RunMode::AlluviumCommand => {
                unreachable!("handled above")
            }
        },
        |path| resolve_output_path(&project_root, Some(path), Path::new(DEFAULT_EVIDENCE)),
    );
    let capture_path = options.capture.as_deref().map_or_else(
        || evidence_root.join(DEFAULT_CAPTURE),
        |path| resolve_output_path(&project_root, Some(path), Path::new(DEFAULT_CAPTURE)),
    );
    let prepared = prepare_ms01(&project_root, &evidence_root, options.frames)?;
    if options.mode == RunMode::HeadlessSmoke {
        write_evidence_bundle(&evidence_root, &prepared.timeline, &prepared.summary)?;
        println!(
            "Meridian MS-01 headless smoke passed: package {}, cell {}, {} frames, save recovery and roundtrip verified",
            prepared.summary.package_hash,
            prepared.summary.cell_hash,
            prepared.summary.runtime_frames
        );
        return Ok(());
    }

    let app = MeridianApplication::new(prepared, evidence_root, capture_path, options.mode)?;
    run_platform(
        PlatformConfig {
            title: "Meridian".to_owned(),
            initial_size: WindowSize::new(960, 540),
            resizable: true,
            visible: true,
            event_loop_mode: EventLoopMode::Poll,
        },
        app,
    )?;
    Ok(())
}

fn run_alluvium_command(command: &AlluviumCommand) -> AppResult<()> {
    let output = match command {
        AlluviumCommand::Inspect { recipe } => {
            let recipe = read_alluvium_recipe(recipe)?;
            json!({"command":"inspect", "schema":recipe.schema, "recipe":recipe})
        }
        AlluviumCommand::Validate { recipe } => {
            let recipe = read_alluvium_recipe(recipe)?;
            json!({"command":"validate", "valid":true, "recipe_id":recipe.id.to_string()})
        }
        AlluviumCommand::Migrate { recipe, schema } => {
            if *schema != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "only schema version 1 is supported",
                )
                .into());
            }
            let source = read_bounded_regular_file(recipe, "Alluvium recipe")?;
            let raw: ProceduralRecipe = serde_json::from_slice(&source)?;
            let migrated = raw.migrate_one_step()?;
            let canonical_json = migrated.canonical_json()?;
            json!({"command":"migrate", "schema":RECIPE_SCHEMA, "recipe":migrated, "canonical_json":canonical_json})
        }
        AlluviumCommand::Preview { recipe, region } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let result = evaluate_alluvium(&recipe, EvaluationMode::Preview)?;
            json!({"command":"preview", "region":region, "result":result})
        }
        AlluviumCommand::Bake { recipe, profile } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let audit = license_audit(&recipe, profile)?;
            if !audit.accepted {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "authoritative bake rejected by license policy",
                )
                .into());
            }
            let result = evaluate_alluvium(&recipe, EvaluationMode::Bake)?;
            json!({"command":"bake", "profile":profile, "license_audit":audit, "result":result})
        }
        AlluviumCommand::Dirty { recipe, since } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let previous = read_alluvium_recipe(since)?;
            json!({"command":"dirty", "report":dirty_report(&previous, &recipe)})
        }
        AlluviumCommand::Explain { recipe, object } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let object_id = parse_stable_id(object)?;
            json!({"command":"explain", "object":explain_generated(&recipe, object_id)?})
        }
        AlluviumCommand::Diff { recipe, against } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let previous = read_alluvium_recipe(against)?;
            json!({"command":"diff", "report":dirty_report(&previous, &recipe), "equal":previous == recipe})
        }
        AlluviumCommand::Provenance { recipe, output } => {
            let recipe = read_alluvium_recipe(recipe)?;
            let output_id = parse_stable_id(output)?;
            json!({"command":"provenance", "output":explain_generated(&recipe, output_id)?, "provenance":recipe.provenance, "dependencies":recipe.dependencies})
        }
        AlluviumCommand::LicenseAudit { recipe, target } => {
            let recipe = read_alluvium_recipe(recipe)?;
            json!({"command":"license-audit", "audit":license_audit(&recipe, target)?})
        }
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn read_alluvium_recipe(path: &Path) -> AppResult<ProceduralRecipe> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("mproc") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Alluvium recipe paths must use the .mproc extension",
        )
        .into());
    }
    let source = read_bounded_regular_file(path, "Alluvium recipe")?;
    let source = std::str::from_utf8(&source).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Alluvium recipe is not UTF-8: {error}"),
        )
    })?;
    Ok(ProceduralRecipe::from_json(source)?)
}

fn evaluate_alluvium(
    recipe: &ProceduralRecipe,
    mode: EvaluationMode,
) -> AppResult<meridian_alluvium::EvaluationResult> {
    let mut engine = AlluviumEngine::default();
    Ok(engine.evaluate(
        recipe,
        EvaluationRequest {
            mode,
            budget: EvaluationBudget {
                max_objects: usize::try_from(recipe.operation.count)?,
            },
            cancelled: false,
        },
    )?)
}

fn default_evidence_root(project_root: &Path) -> PathBuf {
    let run_id = NEXT_DEFAULT_EVIDENCE_ID.fetch_add(1, Ordering::Relaxed);
    project_root
        .join(DEFAULT_EVIDENCE)
        .join(format!("run-{}-{run_id}", std::process::id()))
}

fn run_ui_headless_smoke() -> AppResult<()> {
    let recovery_document = recovery_panel_document()
        .map_err(|error| io::Error::other(format!("recovery UI fixture invalid: {error:?}")))?;
    let mut recovery = UiRuntime::new(recovery_document);
    let mut recovery_input = UiFrameInput::new(UiSize::new(960.0, 540.0));
    recovery_input.high_contrast = true;
    recovery_input.reduced_motion = true;
    recovery_input.events = vec![UiEvent::FocusNext, UiEvent::Activate];
    let recovery_output = recovery.reconcile(recovery_input);
    let semantic_node_count = match &recovery_output.semantic_delta {
        SemanticDelta::Replace(tree) => tree.nodes.len(),
        SemanticDelta::Update(_) | SemanticDelta::Unchanged => 0,
    };
    if recovery_output.commands.len() != 1
        || recovery_output.commands[0].action != "project.retry_open"
        || semantic_node_count != 3
        || recovery_output.focused.is_none()
    {
        return Err(io::Error::other(
            "recovery UI fixture did not produce its typed command and semantic snapshot",
        )
        .into());
    }

    let overlay_document = runtime_overlay_document()
        .map_err(|error| io::Error::other(format!("runtime UI fixture invalid: {error:?}")))?;
    let mut overlay = UiRuntime::new(overlay_document);
    let mut overlay_input = UiFrameInput::new(UiSize::new(960.0, 540.0));
    overlay_input.events = vec![UiEvent::FocusNext];
    let overlay_output = overlay.reconcile(overlay_input);
    if overlay_output.focused.is_some()
        || !overlay_output
            .diagnostics
            .contains(&UiDiagnostic::NoFocusableNode)
    {
        return Err(
            io::Error::other("runtime overlay unexpectedly created a focusable UI path").into(),
        );
    }
    println!(
        "Meridian UI headless smoke passed: recovery command, high-contrast semantic snapshot, and disabled-input runtime overlay verified"
    );
    Ok(())
}

fn run_ui_native_smoke() -> AppResult<()> {
    run_platform(
        PlatformConfig {
            title: "Meridian UI recovery panel smoke".to_owned(),
            initial_size: WindowSize::new(960, 540),
            resizable: true,
            visible: true,
            event_loop_mode: EventLoopMode::Poll,
        },
        UiNativeSmokeApplication::new()?,
    )?;
    Ok(())
}

/// Renders the Creator Alpha retained workspace through Meridian's native UI
/// smoke path. This is a bounded structural check, not milestone evidence.
fn run_creator_alpha_ui_smoke(options: &MeridianOptions) -> AppResult<()> {
    run_creator_application(options.project.as_deref(), true, None, None, None)
}

/// Captures the actual Creator direct-display-list output for human visual review.
///
/// The resulting PNG is deliberately local review material. It proves neither
/// accessibility nor cross-platform visual qualification.
fn run_creator_alpha_ui_review(options: &MeridianOptions) -> AppResult<()> {
    let project = options
        .project
        .as_deref()
        .ok_or_else(|| io::Error::other("Creator UI review project argument was not retained"))?;
    let capture = options.capture.clone().unwrap_or_else(|| {
        workspace_root()
            .expect("workspace root is available for bounded Creator UI review")
            .join("target/meridian-evidence/creator-alpha-ui-review/creator-alpha-ui.png")
    });
    run_creator_application(
        Some(project),
        true,
        Some(capture),
        options.review_workspace,
        options.review_size,
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CreatorRecentProject {
    label: String,
    path: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CreatorDensityPreference {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

impl CreatorDensityPreference {
    const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Standard => "Standard",
            Self::Comfortable => "Comfortable",
        }
    }

    const fn ui_density(self) -> UiDensity {
        match self {
            Self::Compact => UiDensity::Compact,
            Self::Standard => UiDensity::Standard,
            Self::Comfortable => UiDensity::Comfortable,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
struct CreatorPreferences {
    high_contrast: bool,
    reduced_motion: bool,
    density: CreatorDensityPreference,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CreatorHubState {
    schema: String,
    recents: Vec<CreatorRecentProject>,
    #[serde(default)]
    preferences: CreatorPreferences,
}

impl Default for CreatorHubState {
    fn default() -> Self {
        Self {
            schema: CREATOR_HUB_SCHEMA.to_owned(),
            recents: Vec::new(),
            preferences: CreatorPreferences::default(),
        }
    }
}

impl CreatorHubState {
    fn validate(&self) -> AppResult<()> {
        if self.schema != CREATOR_HUB_SCHEMA && self.schema != CREATOR_HUB_SCHEMA_V1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported Meridian launch-hub schema",
            )
            .into());
        }
        if self.recents.len() > CREATOR_RECENT_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Meridian launch-hub recents exceed the supported bound",
            )
            .into());
        }
        if self
            .recents
            .iter()
            .any(|recent| recent.label.trim().is_empty() || recent.path.trim().is_empty())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Meridian launch-hub contains an incomplete recent project",
            )
            .into());
        }
        Ok(())
    }

    /// Upgrades the backward-compatible v1 recent-project document to the
    /// v2 preference document. The caller persists only after validation.
    fn migrate_preferences_schema(&mut self) -> bool {
        if self.schema == CREATOR_HUB_SCHEMA_V1 {
            CREATOR_HUB_SCHEMA.clone_into(&mut self.schema);
            return true;
        }
        false
    }

    fn remember(&mut self, root: &Path) {
        let path = root.display().to_string();
        let label = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Meridian project")
            .to_owned();
        self.recents.retain(|recent| recent.path != path);
        self.recents.insert(0, CreatorRecentProject { label, path });
        self.recents.truncate(CREATOR_RECENT_LIMIT);
    }

    fn views(&self) -> Vec<RecentProjectView> {
        self.recents
            .iter()
            .map(|recent| {
                let root = Path::new(&recent.path);
                RecentProjectView {
                    label: recent.label.clone(),
                    path: recent.path.clone(),
                    available: root.join(CREATOR_ALPHA_MANIFEST).is_file()
                        && root.join(CREATOR_PROJECT_SOURCE).is_file(),
                }
            })
            .collect()
    }
}

struct CreatorHubStore {
    path: PathBuf,
}

impl CreatorHubStore {
    fn for_run(smoke: bool) -> AppResult<Self> {
        let path = if smoke {
            workspace_root()?.join("target/meridian-evidence/creator-alpha-ui-smoke/hub.json")
        } else {
            creator_user_state_path()?
        };
        Ok(Self { path })
    }

    fn load(&self) -> AppResult<CreatorHubState> {
        match fs::symlink_metadata(&self.path) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(CreatorHubState::default())
            }
            Err(error) => return Err(error.into()),
        }
        let bytes = read_bounded_regular_file(&self.path, "Meridian launch-hub state")?;
        let state: CreatorHubState = serde_json::from_slice(&bytes)?;
        state.validate()?;
        Ok(state)
    }

    fn save(&self, state: &CreatorHubState) -> AppResult<()> {
        state.validate()?;
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::other("Meridian launch-hub state path has no parent directory")
        })?;
        fs::create_dir_all(parent)?;
        write_atomic(&self.path, &serde_json::to_vec_pretty(state)?)
    }
}

/// Private platform-thread adapter for explicit Creator project-directory picks.
///
/// Meridian paths cross this seam only after the picker result has been
/// validated by the Creator hub. No `rfd` type escapes this implementation.
trait CreatorProjectPicker {
    fn pick_directory(&self, window: Option<&meridian_platform::PlatformWindow>)
        -> Option<PathBuf>;
}

struct NativeCreatorProjectPicker;

impl CreatorProjectPicker for NativeCreatorProjectPicker {
    fn pick_directory(
        &self,
        window: Option<&meridian_platform::PlatformWindow>,
    ) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new().set_title("Choose Meridian project directory");
        let dialog = if let Some(window) = window {
            dialog.set_parent(window)
        } else {
            dialog
        };
        dialog.pick_folder()
    }
}

fn creator_user_state_path() -> AppResult<PathBuf> {
    Ok(creator_user_state_directory()?.join("launch-hub.json"))
}

fn creator_user_state_directory() -> AppResult<PathBuf> {
    if let Some(directory) = std::env::var_os("MERIDIAN_STATE_DIR") {
        return Ok(PathBuf::from(directory));
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| io::Error::other("HOME is unavailable for Meridian local state"))?;
        Ok(PathBuf::from(home).join("Library/Application Support/Meridian"))
    }
    #[cfg(target_os = "windows")]
    {
        let directory = std::env::var_os("APPDATA")
            .ok_or_else(|| io::Error::other("APPDATA is unavailable for Meridian local state"))?;
        Ok(PathBuf::from(directory).join("Meridian"))
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        if let Some(directory) = std::env::var_os("XDG_STATE_HOME") {
            return Ok(PathBuf::from(directory).join("meridian"));
        }
        let home = std::env::var_os("HOME")
            .ok_or_else(|| io::Error::other("HOME is unavailable for Meridian local state"))?;
        Ok(PathBuf::from(home).join(".local/state/meridian"))
    }
}

/// Returns a private, user-local workspace-layout state path for a project.
///
/// Workspace layouts are preferences, not project source. Keeping them outside
/// the project prevents opening a tracked example from creating an untracked
/// sidecar while still retaining a stable layout for that project on this host.
fn creator_workspace_state_path(root: &Path) -> AppResult<PathBuf> {
    #[cfg(test)]
    let state_directory =
        workspace_root()?.join("target/meridian-evidence/creator-workspace-test-state");
    #[cfg(not(test))]
    let state_directory = creator_user_state_directory()?.join("workspaces");

    let canonical_root = root.canonicalize()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical_root.hash(&mut hasher);
    Ok(state_directory.join(format!(
        "{:016x}-{CREATOR_WORKSPACE_STATE}",
        hasher.finish()
    )))
}

#[derive(Debug)]
struct CreatorBuildTask {
    receiver: Receiver<Result<CreatorAlphaBuildEvidence, String>>,
}

#[derive(Clone, Debug)]
struct CreatorBuildSummary {
    build_id: String,
    artifact_hash: String,
    artifact_bytes: u64,
}

type CreatorBuildSpawner = fn(Box<dyn FnOnce() + Send>) -> io::Result<()>;

fn spawn_creator_build_worker(task: Box<dyn FnOnce() + Send>) -> io::Result<()> {
    thread::Builder::new()
        .name("meridian-creator-build".to_owned())
        .spawn(task)
        .map(|_| ())
}

fn creator_workspace_kinds() -> [WorkspaceKind; 10] {
    [
        WorkspaceKind::World,
        WorkspaceKind::Code,
        WorkspaceKind::Modeler,
        WorkspaceKind::UiAuthoring,
        WorkspaceKind::Materials,
        WorkspaceKind::Alluvium,
        WorkspaceKind::Build,
        WorkspaceKind::Profile,
        WorkspaceKind::Settings,
        WorkspaceKind::Recovery,
    ]
}

fn creator_workspace_panels(workspace: WorkspaceKind) -> [Vec<EditorPanelId>; 3] {
    match workspace {
        WorkspaceKind::World => [
            vec![EditorPanelId::Hierarchy, EditorPanelId::Assets],
            vec![EditorPanelId::Viewport, EditorPanelId::History],
            vec![
                EditorPanelId::Inspector,
                EditorPanelId::Build,
                EditorPanelId::Diagnostics,
            ],
        ],
        WorkspaceKind::Code => [
            vec![EditorPanelId::Assets, EditorPanelId::Hierarchy],
            vec![EditorPanelId::Viewport, EditorPanelId::History],
            vec![EditorPanelId::Inspector, EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::Modeler => [
            vec![EditorPanelId::Modeler],
            vec![EditorPanelId::Viewport, EditorPanelId::History],
            vec![EditorPanelId::Inspector, EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::UiAuthoring => [
            vec![EditorPanelId::Hierarchy],
            vec![EditorPanelId::Viewport],
            vec![EditorPanelId::Inspector, EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::Materials => [
            vec![EditorPanelId::Assets],
            vec![EditorPanelId::Viewport],
            vec![EditorPanelId::Inspector, EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::Alluvium => [
            vec![EditorPanelId::Recipe],
            vec![EditorPanelId::Viewport],
            vec![
                EditorPanelId::Inspector,
                EditorPanelId::Build,
                EditorPanelId::Diagnostics,
            ],
        ],
        WorkspaceKind::Build => [
            vec![EditorPanelId::Build],
            vec![EditorPanelId::History],
            vec![EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::Profile => [
            vec![EditorPanelId::Diagnostics],
            vec![EditorPanelId::Viewport],
            vec![EditorPanelId::History],
        ],
        WorkspaceKind::Settings => [
            vec![EditorPanelId::ProjectRecovery],
            vec![EditorPanelId::Diagnostics],
            vec![EditorPanelId::History],
        ],
        WorkspaceKind::Recovery => [
            vec![EditorPanelId::ProjectRecovery],
            vec![EditorPanelId::History],
            vec![EditorPanelId::Diagnostics],
        ],
        WorkspaceKind::Hub => unreachable!("the hub has no project workspace dock"),
    }
}

fn creator_default_dock(workspace: WorkspaceKind) -> AppResult<DockTree> {
    let base = match workspace {
        WorkspaceKind::World => 10_u128,
        WorkspaceKind::Code => 20,
        WorkspaceKind::Modeler => 30,
        WorkspaceKind::UiAuthoring => 40,
        WorkspaceKind::Materials => 50,
        WorkspaceKind::Alluvium => 60,
        WorkspaceKind::Build => 70,
        WorkspaceKind::Profile => 80,
        WorkspaceKind::Settings => 90,
        WorkspaceKind::Recovery => 100,
        WorkspaceKind::Hub => unreachable!("the hub has no project workspace dock"),
    };
    let root = DockNodeId::new(base);
    let navigation = DockNodeId::new(base + 1);
    let content = DockNodeId::new(base + 2);
    let primary = DockNodeId::new(base + 3);
    let inspector = DockNodeId::new(base + 4);
    let [navigation_panels, primary_panels, inspector_panels] = creator_workspace_panels(workspace);
    let tabs = |panels: Vec<EditorPanelId>| {
        let tabs = panels
            .into_iter()
            .map(|panel| DockTab::pinned(PanelId::from(panel)))
            .collect::<Vec<_>>();
        let active = tabs
            .first()
            .map(|tab| tab.panel)
            .ok_or_else(|| io::Error::other("Creator workspace layout requires a panel"))?;
        Ok::<_, io::Error>(DockNode::Tabs { tabs, active })
    };
    let mut nodes = BTreeMap::new();
    nodes.insert(
        root,
        DockNode::Split {
            axis: DockAxis::Horizontal,
            ratio_per_mille: 264,
            first: navigation,
            second: content,
        },
    );
    nodes.insert(
        content,
        DockNode::Split {
            axis: DockAxis::Horizontal,
            ratio_per_mille: 650,
            first: primary,
            second: inspector,
        },
    );
    nodes.insert(navigation, tabs(navigation_panels)?);
    nodes.insert(primary, tabs(primary_panels)?);
    nodes.insert(inspector, tabs(inspector_panels)?);
    Ok(DockTree::new(root, nodes)?)
}

fn creator_default_workspace_state(session: &EditorSession) -> AppResult<WorkspaceStateDocument> {
    let mut state = WorkspaceStateDocument::new(
        WorkspaceSessionId::new(session.document().id.get()),
        WorkspaceKind::World,
    );
    for workspace in creator_workspace_kinds() {
        state.save_named_layout(WorkspaceLayout {
            name: CREATOR_DEFAULT_LAYOUT.to_owned(),
            workspace,
            dock: creator_default_dock(workspace)?,
            selected: None,
            active_document: None,
            camera: None,
            browser_query: String::new(),
            expanded: Vec::new(),
            scroll: Vec::new(),
            focused_panel: None,
            focus_layout: false,
            companions: Vec::new(),
            extensions: WorkspaceExtensions::default(),
        })?;
    }
    Ok(state)
}

fn load_creator_workspace_state(
    store: &WorkspaceStateStore,
    fallback: WorkspaceStateDocument,
) -> (WorkspaceStateDocument, String) {
    match store.load_migrated() {
        Ok(outcome) if outcome.document.session == fallback.session => {
            if outcome.migrated_from.is_some() {
                if let Err(error) = store.save(&outcome.document) {
                    return (
                        outcome.document,
                        format!(
                            "Workspace state migrated in memory but could not be saved: {error}."
                        ),
                    );
                }
                return (
                    outcome.document,
                    "Workspace layout migrated and saved.".to_owned(),
                );
            }
            (
                outcome.document,
                "Restored persisted workspace layout.".to_owned(),
            )
        }
        Ok(_) => {
            let detail = "Workspace layout belonged to another project and was reset.".to_owned();
            if let Err(error) = store.save(&fallback) {
                return (
                    fallback,
                    format!("{detail} The replacement could not be saved: {error}."),
                );
            }
            (fallback, detail)
        }
        Err(error) => {
            let detail = format!("Workspace layout was recovered from defaults: {error}.");
            if let Err(save_error) = store.save(&fallback) {
                return (
                    fallback,
                    format!("{detail} The replacement could not be saved: {save_error}."),
                );
            }
            (fallback, detail)
        }
    }
}

struct CreatorWorkspace {
    root: PathBuf,
    manifest: CreatorAlphaManifest,
    project_store: ProjectStore,
    session: EditorSession,
    model_path: PathBuf,
    model_recovery: ModelRecoveryStore,
    model_session: ModelSession,
    recipe: ProceduralRecipe,
    recovery_status: ProjectRecoveryStatus,
    status: String,
    build: Option<CreatorBuildTask>,
    last_build: Option<CreatorBuildSummary>,
    workspace_store: WorkspaceStateStore,
    workspace_state: WorkspaceStateDocument,
}

impl CreatorWorkspace {
    fn open(requested: &Path) -> AppResult<Self> {
        let root = resolve_creator_alpha_project(requested)?;
        let manifest: CreatorAlphaManifest = serde_json::from_slice(&read_bounded_regular_file(
            &root.join(CREATOR_ALPHA_MANIFEST),
            "Creator Alpha manifest",
        )?)?;
        validate_creator_alpha_manifest(&root, &manifest)?;
        let project_store = ProjectStore::new(
            root.join(CREATOR_PROJECT_SOURCE),
            root.join(CREATOR_INTERNAL_DIRECTORY)
                .join("editor-recovery.state"),
        );
        let opened = project_store.open()?;
        let mut session = opened.session;
        // A valid Creator Alpha project has an editable placement. Select its
        // first stable source placement locally on open so the Inspector and
        // viewport agree about their initial subject without mutating source.
        if session.selection().ids.is_empty() {
            if let Some(placement_id) = session.document().placements.keys().next().copied() {
                session.select([placement_id])?;
            }
        }
        let model_path = root.join(validated_project_relative_path(&manifest.editable_model)?);
        let model_recovery = ModelRecoveryStore::new(
            root.join(CREATOR_INTERNAL_DIRECTORY)
                .join("modeler-recovery.state"),
        );
        let source_model = ModelDocument::read_source(&model_path)?;
        let model_session = if model_recovery.path().exists() {
            match model_recovery.load() {
                Ok(recovered) if recovered.current().document() == &source_model => recovered,
                Ok(_) | Err(_) => ModelSession::open(source_model)?,
            }
        } else {
            ModelSession::open(source_model)?
        };
        let recipe_path = root.join(validated_project_relative_path(
            &manifest.procedural_recipe,
        )?);
        let recipe = read_alluvium_recipe(&recipe_path)?;
        let mut status = match opened.recovery {
            ProjectRecoveryStatus::None => {
                "Opened authoritative source; no recovery snapshot exists.".to_owned()
            }
            ProjectRecoveryStatus::Restored => {
                "Opened authoritative source with validated recovered context; history starts fresh."
                    .to_owned()
            }
            ProjectRecoveryStatus::Ignored => {
                "Opened authoritative source; incompatible recovery was ignored.".to_owned()
            }
        };
        let workspace_store = WorkspaceStateStore::new(creator_workspace_state_path(&root)?);
        let workspace_default = creator_default_workspace_state(&session)?;
        let (workspace_state, workspace_detail) =
            load_creator_workspace_state(&workspace_store, workspace_default);
        status.push(' ');
        status.push_str(&workspace_detail);
        Ok(Self {
            root,
            manifest,
            project_store,
            session,
            model_path,
            model_recovery,
            model_session,
            recipe,
            recovery_status: opened.recovery,
            status,
            build: None,
            last_build: None,
            workspace_store,
            workspace_state,
        })
    }

    fn ui_view(&self, viewport: UiSize) -> CreatorWorkspaceView {
        let model = self.model_session.current().document();
        let modeler = model.objects.first().map(|object| {
            let preview = model.penumbra_preview(object.id).ok();
            CreatorModelerPresentation {
                generation: self.model_session.current().generation(),
                document_label: model.label.clone(),
                object_label: object.label.clone(),
                object_count: model.objects.len(),
                vertex_count: object.vertices.len(),
                edge_count: object.edges.len(),
                face_count: object.faces.len(),
                preview,
            }
        });
        let preview_triangles = modeler
            .as_ref()
            .and_then(|presentation| presentation.preview.as_ref())
            .map_or(0, |preview| preview.triangle_indices.len() / 3);
        let build = if self.build.is_some() {
            "Build in progress through the durable worker.".to_owned()
        } else if let Some(last) = &self.last_build {
            let artifact_prefix = last.artifact_hash.chars().take(12).collect::<String>();
            format!(
                "{} · artifact {artifact_prefix} · {} bytes.",
                last.build_id, last.artifact_bytes
            )
        } else if self.status.starts_with("Build failed:") {
            self.status.clone()
        } else {
            "Ready for a bounded local build.".to_owned()
        };
        CreatorWorkspaceView {
            project: self
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Meridian Project")
                .to_owned(),
            activity: self.status.clone(),
            recovery: format!(
                "Recovery: {:?} · {} checkpoint(s).",
                self.recovery_status,
                self.session.checkpoints().len()
            ),
            build,
            recipe: format!(
                "v1 · {} placement(s) · every {} mm.",
                self.recipe.operation.count, self.recipe.operation.spacing_mm
            ),
            model: format!(
                "Revision {} · {} object(s) · {} preview triangle(s).",
                self.model_session.current().generation(),
                model.objects.len(),
                preview_triangles
            ),
            workspace: self.active_workspace(),
            focus_layout: self.active_focus_layout(),
            code_context_width: code_context_width(
                self.active_workspace(),
                self.active_focus_layout(),
                viewport.width,
            ),
            compact_world_context: self.active_workspace() == WorkspaceKind::World
                && viewport.width < 1_180.0,
            compact_ui_authoring: self.active_workspace() == WorkspaceKind::UiAuthoring
                && viewport.width < 1_180.0,
            focused_panel: self.active_focused_panel(),
            project_source: self
                .session
                .document()
                .canonical_json()
                .unwrap_or_else(|error| format!("Project source is unavailable: {error}")),
            recipe_source: self
                .recipe
                .canonical_json()
                .unwrap_or_else(|error| format!("Recipe source is unavailable: {error}")),
            modeler,
        }
    }

    fn active_workspace(&self) -> WorkspaceKind {
        self.workspace_state.active_workspace
    }

    /// Replaces only the in-memory layout used by a local visual review.
    ///
    /// Review captures must not write user preferences or alter project source.
    fn select_workspace_for_review(&mut self, requested: WorkspaceKind) -> AppResult<()> {
        if requested == WorkspaceKind::Hub {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the Creator hub is not a project workspace review destination",
            )
            .into());
        }
        let mut state = creator_default_workspace_state(&self.session)?;
        if requested != WorkspaceKind::World {
            state.activate_workspace(requested, CREATOR_DEFAULT_LAYOUT)?;
        }
        self.workspace_state = state;
        self.status = format!(
            "{} workspace shown with its default local review layout; no workspace preference was saved.",
            creator_workspace_name(requested)
        );
        Ok(())
    }

    fn active_focus_layout(&self) -> bool {
        self.workspace_state
            .layouts
            .iter()
            .find(|layout| {
                layout.workspace == self.workspace_state.active_workspace
                    && layout.name == self.workspace_state.active_layout_name
            })
            .is_some_and(|layout| layout.focus_layout)
    }

    fn active_focused_panel(&self) -> Option<EditorPanelId> {
        self.workspace_state
            .layouts
            .iter()
            .find(|layout| {
                layout.workspace == self.workspace_state.active_workspace
                    && layout.name == self.workspace_state.active_layout_name
            })
            .and_then(|layout| layout.focused_panel)
            .and_then(creator_editor_panel)
    }

    fn activate_workspace(&mut self, workspace: WorkspaceKind) -> AppResult<WorkspaceActivation> {
        let before = self.workspace_state.clone();
        let activation = self
            .workspace_state
            .activate_workspace(workspace, CREATOR_DEFAULT_LAYOUT)?;
        if let Err(error) = self.workspace_store.save(&self.workspace_state) {
            self.workspace_state = before;
            return Err(error.into());
        }
        Ok(activation)
    }

    fn cycle_panel_focus(&mut self) -> AppResult<PanelId> {
        let before = self.workspace_state.clone();
        let panel = self.workspace_state.cycle_panel_focus(
            self.active_workspace(),
            CREATOR_DEFAULT_LAYOUT,
            true,
        )?;
        if let Err(error) = self.workspace_store.save(&self.workspace_state) {
            self.workspace_state = before;
            return Err(error.into());
        }
        Ok(panel)
    }

    fn focus_panel(&mut self, panel: EditorPanelId) -> AppResult<()> {
        let before = self.workspace_state.clone();
        let layout = self
            .workspace_state
            .layouts
            .iter_mut()
            .find(|layout| {
                layout.workspace == self.workspace_state.active_workspace
                    && layout.name == self.workspace_state.active_layout_name
            })
            .ok_or_else(|| io::Error::other("active Creator workspace layout is unavailable"))?;
        layout.focused_panel = Some(PanelId::from(panel));
        if let Err(error) = self.workspace_store.save(&self.workspace_state) {
            self.workspace_state = before;
            return Err(error.into());
        }
        Ok(())
    }

    fn mutate_model<T, F>(&mut self, mutation: F) -> AppResult<T>
    where
        F: FnOnce(&mut ModelSession) -> Result<T, meridian_modeler::ModelError>,
    {
        let before = self.model_session.clone();
        let output = mutation(&mut self.model_session)?;
        if let Err(error) = self
            .model_session
            .current()
            .document()
            .write_source(&self.model_path)
        {
            self.model_session = before;
            return Err(error.into());
        }
        if let Err(error) = self.model_recovery.save(&self.model_session) {
            let rollback = before.current().document().write_source(&self.model_path);
            self.model_session = before;
            if let Err(rollback) = rollback {
                return Err(io::Error::other(format!(
                    "model recovery failed ({error}) and source rollback failed ({rollback})"
                ))
                .into());
            }
            return Err(error.into());
        }
        Ok(output)
    }

    #[allow(clippy::assigning_clones)] // Status changes are infrequent UI diagnostics.
    fn poll_build(&mut self) -> bool {
        let Some(task) = self.build.as_ref() else {
            return false;
        };
        match task.receiver.try_recv() {
            Ok(Ok(evidence)) => {
                self.status = format!(
                    "Build completed: {} (artifact {}, {} bytes).",
                    evidence.build_id, evidence.artifact_hash, evidence.artifact_bytes
                );
                self.last_build = Some(CreatorBuildSummary {
                    build_id: evidence.build_id,
                    artifact_hash: evidence.artifact_hash,
                    artifact_bytes: evidence.artifact_bytes,
                });
                self.build = None;
                true
            }
            Ok(Err(error)) => {
                self.status = format!("Build failed: {error}");
                self.build = None;
                true
            }
            Err(TryRecvError::Disconnected) => {
                self.status = "Build worker ended without a completion result.".to_owned();
                self.build = None;
                true
            }
            Err(TryRecvError::Empty) => false,
        }
    }

    fn start_build(&mut self, wake_window: Option<PlatformWindow>) -> AppResult<()> {
        self.start_build_with_wake(wake_window, spawn_creator_build_worker)
    }

    #[cfg(test)]
    #[allow(clippy::assigning_clones)] // Status changes are infrequent UI diagnostics.
    fn start_build_with_spawner(&mut self, spawn: CreatorBuildSpawner) -> AppResult<()> {
        self.start_build_with_wake(None, spawn)
    }

    #[allow(clippy::assigning_clones)] // Status changes are infrequent UI diagnostics.
    fn start_build_with_wake(
        &mut self,
        wake_window: Option<PlatformWindow>,
        spawn: CreatorBuildSpawner,
    ) -> AppResult<()> {
        if self.build.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "a Creator build is already running",
            )
            .into());
        }
        let evidence_root = self.root.join("target/meridian-build/creator-alpha");
        fs::create_dir_all(&evidence_root)?;
        let build_input = evidence_root.join("creator-alpha-build-input.json");
        write_atomic(
            &build_input,
            &serde_json::to_vec_pretty(&CreatorAlphaBuildInput {
                schema: "meridian.creator-alpha-build-input/v1",
                project_id: self.session.document().id.to_string(),
                document_generation: self.session.document().generation,
                imported_source_count: self.session.document().sources.len(),
                placement_count: self.session.document().placements.len(),
                editable_model: &self.manifest.editable_model,
                model_document_generation: self.model_session.current().generation(),
                model_preview_triangle_count: self
                    .model_session
                    .current()
                    .document()
                    .objects
                    .first()
                    .map_or(0, |object| {
                        self.model_session
                            .current()
                            .document()
                            .penumbra_preview(object.id)
                            .map_or(0, |preview| preview.triangle_indices.len() / 3)
                    }),
                procedural_recipe: &self.manifest.procedural_recipe,
            })?,
        )?;
        let manifest = read_bounded_regular_file(
            &self.root.join(CREATOR_ALPHA_MANIFEST),
            "Creator Alpha manifest",
        )?;
        let (sender, receiver) = mpsc::channel();
        let worker_evidence_root = evidence_root.clone();
        let worker_build_input = build_input.clone();
        if let Err(error) = spawn(Box::new(move || {
            let result =
                run_creator_alpha_build(&manifest, &worker_evidence_root, &worker_build_input)
                    .map_err(|error| error.to_string());
            let _ = sender.send(result);
            if let Some(window) = wake_window {
                window.request_redraw();
            }
        })) {
            let _ = fs::remove_file(&build_input);
            return Err(
                io::Error::other(format!("unable to start Creator build worker: {error}")).into(),
            );
        }
        self.status = "Build submitted through the durable one-worker Cargo service.".to_owned();
        self.build = Some(CreatorBuildTask { receiver });
        Ok(())
    }
}

/// Derives Code's contextual split from the current native viewport.
///
/// The breakpoint is deliberately presentation-only: no project source or
/// persisted workspace preference changes when a window is resized.
fn code_context_width(
    workspace: WorkspaceKind,
    focused: bool,
    viewport_width: f32,
) -> CodeContextWidth {
    if workspace != WorkspaceKind::Code || focused {
        CodeContextWidth::Standard
    } else if viewport_width < 1_440.0 {
        // Yield the file browser first; this preserves a useful live World
        // canvas and a readable source column at ordinary laptop widths.
        CodeContextWidth::Compact
    } else {
        CodeContextWidth::Wide
    }
}

/// A rejected typed UI command that did not preserve its canonical identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiEffectCommandError {
    CommandIdentityMismatch,
}

impl Display for UiEffectCommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandIdentityMismatch => formatter.write_str(
                "Creator UI ignored a malformed command effect; no project action was applied.",
            ),
        }
    }
}

impl Error for UiEffectCommandError {}

/// Collects only the canonical commands accepted at a completed UI frame.
///
/// A host-bound assistive operation deliberately carries its own command
/// rather than borrowing the target node's ordinary activation command. That
/// keeps Expand/Collapse and similar requests from degrading into clicks. A
/// malformed frame effect rejects the complete batch rather than silently
/// dropping one command or granting its text ambient authority.
fn ui_effect_action_names(
    commands: &[UiCommandRequest],
    assistive_requests: &[UiAssistiveRequest],
) -> Result<Vec<String>, UiEffectCommandError> {
    let mut actions = Vec::with_capacity(commands.len().saturating_add(assistive_requests.len()));
    for command in commands {
        if CommandId::from_name(&command.action) != Some(command.command) {
            return Err(UiEffectCommandError::CommandIdentityMismatch);
        }
        actions.push(command.action.clone());
    }
    for request in assistive_requests {
        if CommandId::from_name(&request.command_name) != Some(request.command) {
            return Err(UiEffectCommandError::CommandIdentityMismatch);
        }
        actions.push(request.command_name.clone());
    }
    Ok(actions)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreatorUiAction {
    CreateProject,
    OpenProject,
    OpenRecent(usize),
    LocateRecent(usize),
    RemoveRecent(usize),
    Recover,
    ReturnHub,
    StartPlay,
    ApplyPlay,
    DiscardPlay,
    SelectPlacement,
    EditPlacement,
    PreviewCommand,
    Undo,
    Redo,
    Reimport,
    InspectSource,
    SubmitBuild,
    InspectBuild,
    RecipeInspect,
    RecipeValidate,
    RecipeMigrate,
    RecipePreview,
    RecipeBake,
    RecipeDirty,
    RecipeExplain,
    RecipeProvenance,
    RecipeLicenseAudit,
    ModelInspect,
    ModelCreatePrimitive,
    ModelTransform,
    ModelSplitEdge,
    ModelUndo,
    ModelRedo,
    ModelRecover,
    FocusSelection,
    ShowDiagnostics,
    SwitchWorkspace(WorkspaceKind),
    ShellSearch,
    OpenSettings,
    ReturnFromSettings,
    ToggleHighContrast,
    ToggleReducedMotion,
    SetDensity(CreatorDensityPreference),
    ResetPreferences,
    ShellFavorites,
    ShellPanels,
    ShellOpenShelf,
    ShellPlayUnavailable,
    ShellBuildUnavailable,
}

impl CreatorUiAction {
    fn parse(action: &str) -> AppResult<Self> {
        let fixed = match action {
            "hub.create-project" => Self::CreateProject,
            "hub.open-project" | "editor.open-project" => Self::OpenProject,
            "editor.recover" => Self::Recover,
            "editor.return-hub" => Self::ReturnHub,
            "editor.play-start" => Self::StartPlay,
            "editor.play-apply" => Self::ApplyPlay,
            "editor.play-discard" => Self::DiscardPlay,
            "editor.select-placement" => Self::SelectPlacement,
            "editor.edit-placement" => Self::EditPlacement,
            "editor.preview-command" => Self::PreviewCommand,
            "editor.undo" => Self::Undo,
            "editor.redo" => Self::Redo,
            "asset.reimport" => Self::Reimport,
            "asset.inspect-source" => Self::InspectSource,
            "build.submit" => Self::SubmitBuild,
            "build.inspect" => Self::InspectBuild,
            "procedural.inspect" => Self::RecipeInspect,
            "procedural.validate" => Self::RecipeValidate,
            "procedural.migrate" => Self::RecipeMigrate,
            "procedural.preview" => Self::RecipePreview,
            "procedural.bake" => Self::RecipeBake,
            "procedural.dirty" => Self::RecipeDirty,
            "procedural.explain" => Self::RecipeExplain,
            "procedural.provenance" => Self::RecipeProvenance,
            "procedural.license-audit" => Self::RecipeLicenseAudit,
            "model.inspect-source" => Self::ModelInspect,
            "model.create-primitive" => Self::ModelCreatePrimitive,
            "model.transform" => Self::ModelTransform,
            "model.split-edge" => Self::ModelSplitEdge,
            "model.undo" => Self::ModelUndo,
            "model.redo" => Self::ModelRedo,
            "model.recover" => Self::ModelRecover,
            "editor.focus-selection" => Self::FocusSelection,
            "editor.show-diagnostic" => Self::ShowDiagnostics,
            "workspace.world" => Self::SwitchWorkspace(WorkspaceKind::World),
            "workspace.modeler" => Self::SwitchWorkspace(WorkspaceKind::Modeler),
            "workspace.ui" => Self::SwitchWorkspace(WorkspaceKind::UiAuthoring),
            "workspace.code" => Self::SwitchWorkspace(WorkspaceKind::Code),
            "workspace.materials" => Self::SwitchWorkspace(WorkspaceKind::Materials),
            "workspace.alluvium" => Self::SwitchWorkspace(WorkspaceKind::Alluvium),
            "workspace.build" => Self::SwitchWorkspace(WorkspaceKind::Build),
            "workspace.profile" => Self::SwitchWorkspace(WorkspaceKind::Profile),
            "shell.search" => Self::ShellSearch,
            "shell.settings" => Self::OpenSettings,
            "settings.return" => Self::ReturnFromSettings,
            "settings.toggle-high-contrast" => Self::ToggleHighContrast,
            "settings.toggle-reduced-motion" => Self::ToggleReducedMotion,
            "settings.density-compact" => Self::SetDensity(CreatorDensityPreference::Compact),
            "settings.density-standard" => Self::SetDensity(CreatorDensityPreference::Standard),
            "settings.density-comfortable" => {
                Self::SetDensity(CreatorDensityPreference::Comfortable)
            }
            "settings.reset-preferences" => Self::ResetPreferences,
            "shell.favorites" => Self::ShellFavorites,
            "shell.panels" => Self::ShellPanels,
            "shell.open-shelf" => Self::ShellOpenShelf,
            "shell.play-unavailable" => Self::ShellPlayUnavailable,
            "shell.build-unavailable" => Self::ShellBuildUnavailable,
            _ => {
                if let Some(index) = action.strip_prefix("hub.open-recent:") {
                    return Ok(Self::OpenRecent(index.parse()?));
                }
                if let Some(index) = action.strip_prefix("hub.remove-recent:") {
                    return Ok(Self::RemoveRecent(index.parse()?));
                }
                if let Some(index) = action.strip_prefix("hub.locate-recent:") {
                    return Ok(Self::LocateRecent(index.parse()?));
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unsupported Creator UI action: {action}"),
                )
                .into());
            }
        };
        Ok(fixed)
    }
}

enum CreatorScreen {
    Hub,
    Settings {
        resume_workspace: Option<Box<CreatorWorkspace>>,
    },
    Workspace(Box<CreatorWorkspace>),
}

const fn creator_workspace_name(workspace: WorkspaceKind) -> &'static str {
    match workspace {
        WorkspaceKind::Hub => "Hub",
        WorkspaceKind::World => "World",
        WorkspaceKind::Code => "Code",
        WorkspaceKind::Modeler => "Modeler",
        WorkspaceKind::UiAuthoring => "UI",
        WorkspaceKind::Materials => "Materials",
        WorkspaceKind::Alluvium => "Alluvium",
        WorkspaceKind::Build => "Build",
        WorkspaceKind::Profile => "Profile",
        WorkspaceKind::Settings => "Settings",
        WorkspaceKind::Recovery => "Recovery",
    }
}

const fn creator_panel_name(panel: PanelId) -> &'static str {
    match panel.value() {
        1 => "Project and recovery",
        2 => "Viewport",
        3 => "Hierarchy",
        4 => "Inspector",
        5 => "History",
        6 => "Assets and import",
        7 => "Build",
        8 => "Recipe",
        9 => "Modeler",
        10 => "Diagnostics",
        _ => "unknown panel",
    }
}

const fn creator_editor_panel(panel: PanelId) -> Option<EditorPanelId> {
    match panel.value() {
        1 => Some(EditorPanelId::ProjectRecovery),
        2 => Some(EditorPanelId::Viewport),
        3 => Some(EditorPanelId::Hierarchy),
        4 => Some(EditorPanelId::Inspector),
        5 => Some(EditorPanelId::History),
        6 => Some(EditorPanelId::Assets),
        7 => Some(EditorPanelId::Build),
        8 => Some(EditorPanelId::Recipe),
        9 => Some(EditorPanelId::Modeler),
        10 => Some(EditorPanelId::Diagnostics),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum InspectorFieldSync {
    #[default]
    RetainDraft,
    ResetFromAuthoritativeSource,
}

#[derive(Debug, Default)]
enum CreatorReviewCapture {
    #[default]
    Disabled,
    Pending(PathBuf),
    Requested(PathBuf),
    Written,
}

impl CreatorReviewCapture {
    const fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

struct CreatorApplication {
    hub_store: CreatorHubStore,
    hub: CreatorHubState,
    screen: CreatorScreen,
    /// Canonical authored source plus the shared runtime source-to-frame
    /// compiler. Recovery/overlay fixtures below remain direct runtimes.
    ui: UiDocumentCompiler,
    frame: UiFrameOutput,
    logical_viewport: UiSize,
    scale_factor: f32,
    physical_size: WindowSize,
    pending_events: Vec<UiEvent>,
    pending_actions: Vec<String>,
    pointer: UiPoint,
    modifiers: PlatformModifiers,
    rhi: Option<Rhi>,
    renderer: Option<CreatorDirectUiRenderer>,
    structural_fallback_submitted: bool,
    surface_attempts: u8,
    bootstrap_renderer_refresh_pending: bool,
    native_smoke: bool,
    run_persistence: CreatorRunPersistence,
    review_capture: CreatorReviewCapture,
    visible_presentations: u8,
    hub_status: String,
    settings_status: String,
    inspector_field_sync: InspectorFieldSync,
    terminal_error: Option<PlatformError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreatorRunPersistence {
    Persistent,
    InMemoryReview,
}

/// GPU-owned direct-display-list resources for the Creator shell's immutable UI frame.
///
/// The CPU raster bridge remains available to bounded structural/recovery smokes, but
/// the user-facing Creator surface must retain the direct Penumbra path selected by
/// ADR-0029 so authored UI colors reach an sRGB surface without a texture bridge.
struct CreatorDirectUiRenderer {
    plan: UiDirectFramePlan,
    gpu: UiDirectGpuFrame,
}

fn build_creator_direct_renderer(
    rhi: &mut Rhi,
    frame: &UiFrameOutput,
    viewport: UiSize,
    scale_factor: f32,
) -> AppResult<CreatorDirectUiRenderer> {
    let resources = UiDirectResourceSet::default();
    let mut renderer = UiDirectGpuRenderer::new(rhi.render_identity());
    let plan = renderer.prepare_frame(UiDirectPrepareRequest {
        display_revision: frame.revision,
        display_list: &frame.display_list,
        viewport,
        scale_factor,
        contrast: frame.contrast,
        // Creator's reviewed shell is intentionally opaque. Any future backdrop
        // use must opt in with evidence instead of silently depending on a GPU effect.
        effects: UiEffectCapabilities::default(),
        resources: &resources,
    })?;
    let gpu = plan.upload_gpu_frame(rhi)?;
    Ok(CreatorDirectUiRenderer { plan, gpu })
}

const fn creator_ui_clear_color() -> ClearColor {
    // Linear-sRGB encoding of the reviewed #090b0b Creator canvas background.
    ClearColor {
        red: 0.002_731_743,
        green: 0.003_346_536,
        blue: 0.003_346_536,
        alpha: 1.0,
    }
}

fn run_creator_application(
    project: Option<&Path>,
    native_smoke: bool,
    review_capture_path: Option<PathBuf>,
    review_workspace: Option<WorkspaceKind>,
    review_size: Option<WindowSize>,
) -> AppResult<()> {
    let mut application = if review_capture_path.is_some() {
        CreatorApplication::new_for_local_ui_review(project, native_smoke)?
    } else {
        CreatorApplication::new(project, native_smoke)?
    };
    if let Some(workspace) = review_workspace {
        application.select_workspace_for_review(workspace)?;
    }
    if let Some(size) = review_size {
        application.refresh_for_size(size, 1.0)?;
    }
    application.review_capture = review_capture_path
        .map(CreatorReviewCapture::Pending)
        .unwrap_or_default();
    run_platform(
        PlatformConfig {
            title: "Meridian — Creator".to_owned(),
            initial_size: review_size.unwrap_or(CREATOR_INITIAL_WINDOW),
            resizable: true,
            visible: true,
            event_loop_mode: creator_event_loop_mode(native_smoke),
        },
        application,
    )?;
    Ok(())
}

const fn creator_event_loop_mode(native_smoke: bool) -> EventLoopMode {
    if native_smoke {
        EventLoopMode::Poll
    } else {
        EventLoopMode::Wait
    }
}

const fn creator_exits_after_visible_presentation(native_smoke: bool, presentations: u8) -> bool {
    native_smoke && presentations >= CREATOR_UI_SMOKE_VISIBLE_PRESENTATIONS
}

/// The Creator surface receives a bounded settling redraw after its first
/// presentation. On macOS a surface can report a successful first present
/// before its uploaded UI texture is composited; without this redraw the
/// persistent application can remain visually blank until a later input event.
const fn creator_requests_follow_up_redraw(
    native_smoke: bool,
    presentations: u8,
    build_active: bool,
) -> bool {
    !creator_exits_after_visible_presentation(native_smoke, presentations)
        && (presentations < CREATOR_UI_SMOKE_VISIBLE_PRESENTATIONS || build_active)
}

/// A build belongs to the project session, not the currently visible surface.
/// Settings may temporarily cover that session, so the native event loop must
/// keep asking for frames until its worker reaches a terminal state.
const fn creator_has_active_build(screen: &CreatorScreen) -> bool {
    match screen {
        CreatorScreen::Workspace(workspace)
        | CreatorScreen::Settings {
            resume_workspace: Some(workspace),
        } => workspace.build.is_some(),
        CreatorScreen::Hub
        | CreatorScreen::Settings {
            resume_workspace: None,
        } => false,
    }
}

fn poll_creator_build(screen: &mut CreatorScreen) -> bool {
    match screen {
        CreatorScreen::Workspace(workspace) => workspace.poll_build(),
        CreatorScreen::Settings { resume_workspace } => resume_workspace
            .as_deref_mut()
            .is_some_and(CreatorWorkspace::poll_build),
        CreatorScreen::Hub => false,
    }
}

const fn creator_exits_after_surface_attempt(native_smoke: bool, attempts: u8) -> bool {
    native_smoke && attempts >= UI_SMOKE_MAX_PRESENT_ATTEMPTS
}

const fn creator_surface_is_renderable(size: WindowSize) -> bool {
    !size.is_zero()
}

fn rebuild_for_renderable_creator_surface<T>(
    size: WindowSize,
    rebuild: impl FnOnce() -> AppResult<T>,
) -> AppResult<Option<T>> {
    if !creator_surface_is_renderable(size) {
        return Ok(None);
    }
    rebuild().map(Some)
}

fn parse_creator_translation_component(axis: &str, value: Option<&str>) -> AppResult<i64> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    value
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{axis} coordinate must be a signed whole millimetre value"),
            )
        })?
        .parse::<i64>()
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{axis} coordinate must be a signed whole millimetre value"),
            )
            .into()
        })
}

impl CreatorApplication {
    fn new(project: Option<&Path>, native_smoke: bool) -> AppResult<Self> {
        Self::new_for_run(project, native_smoke, CreatorRunPersistence::Persistent)
    }

    /// Creates an intentionally stateless application instance for an offscreen
    /// visual-review capture. Review frames cannot inherit fixture recents or
    /// write a project preference merely because a screenshot was requested.
    fn new_for_local_ui_review(project: Option<&Path>, native_smoke: bool) -> AppResult<Self> {
        Self::new_for_run(project, native_smoke, CreatorRunPersistence::InMemoryReview)
    }

    fn new_for_run(
        project: Option<&Path>,
        native_smoke: bool,
        run_persistence: CreatorRunPersistence,
    ) -> AppResult<Self> {
        let hub_store = CreatorHubStore::for_run(native_smoke)?;
        let (mut hub, hub_status) = if run_persistence == CreatorRunPersistence::InMemoryReview {
            (
                CreatorHubState::default(),
                "Local UI review uses an in-memory project hub.".to_owned(),
            )
        } else {
            match hub_store.load() {
                Ok(hub) => (
                    hub,
                    "Create a public project or open a validated project directory.".to_owned(),
                ),
                Err(error) => (
                    CreatorHubState::default(),
                    format!("Local hub state was ignored: {error}"),
                ),
            }
        };
        let hub_status = if run_persistence == CreatorRunPersistence::Persistent
            && hub.migrate_preferences_schema()
        {
            match hub_store.save(&hub) {
                Ok(()) => "Local hub preferences were migrated safely.".to_owned(),
                Err(error) => format!(
                    "Local hub preferences were migrated in memory but could not be saved: {error}"
                ),
            }
        } else {
            hub_status
        };
        let document = creator_hub_document(&hub.views(), &hub_status)
            .map_err(|error| io::Error::other(format!("Creator hub UI invalid: {error:?}")))?;
        let mut ui = UiDocumentCompiler::new(document);
        let frame = ui.reconcile(UiFrameInput::new(UiSize::new(
            CREATOR_INITIAL_VIEWPORT_WIDTH,
            CREATOR_INITIAL_VIEWPORT_HEIGHT,
        )));
        let mut application = Self {
            hub_store,
            hub,
            screen: CreatorScreen::Hub,
            ui,
            frame,
            logical_viewport: UiSize::new(
                CREATOR_INITIAL_VIEWPORT_WIDTH,
                CREATOR_INITIAL_VIEWPORT_HEIGHT,
            ),
            scale_factor: 1.0,
            physical_size: CREATOR_INITIAL_WINDOW,
            pending_events: Vec::new(),
            pending_actions: Vec::new(),
            pointer: UiPoint::default(),
            modifiers: PlatformModifiers::default(),
            rhi: None,
            renderer: None,
            structural_fallback_submitted: false,
            surface_attempts: 0,
            bootstrap_renderer_refresh_pending: false,
            native_smoke,
            run_persistence,
            review_capture: CreatorReviewCapture::default(),
            visible_presentations: 0,
            hub_status,
            settings_status: "Local preferences are ready.".to_owned(),
            inspector_field_sync: InspectorFieldSync::RetainDraft,
            terminal_error: None,
        };
        if let Some(project) = project {
            application.open_project(project)?;
        }
        application.refresh_document_and_ui()?;
        Ok(application)
    }

    /// Selects a clean default workspace only for an in-memory review capture.
    fn select_workspace_for_review(&mut self, requested: WorkspaceKind) -> AppResult<()> {
        match requested {
            WorkspaceKind::Hub => {
                self.screen = CreatorScreen::Hub;
                "Meridian hub shown for local UI review; no recent-project state was saved."
                    .clone_into(&mut self.hub_status);
            }
            WorkspaceKind::Settings => {
                let screen = std::mem::replace(&mut self.screen, CreatorScreen::Hub);
                let CreatorScreen::Workspace(workspace) = screen else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "open a project before selecting Settings for local UI review",
                    )
                    .into());
                };
                self.screen = CreatorScreen::Settings {
                    resume_workspace: Some(workspace),
                };
                "Settings shown for local UI review; no preferences were saved."
                    .clone_into(&mut self.settings_status);
            }
            _ => {
                let CreatorScreen::Workspace(workspace) = &mut self.screen else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "open a project before selecting a Creator review workspace",
                    )
                    .into());
                };
                workspace.select_workspace_for_review(requested)?;
            }
        }
        self.refresh_document_and_ui()
    }

    fn refresh_document(&mut self) -> AppResult<()> {
        let settings_query = self
            .ui
            .text_input_value(CREATOR_SETTINGS_SEARCH)
            .unwrap_or_default()
            .to_owned();
        let document = match &self.screen {
            CreatorScreen::Hub => creator_hub_document(&self.hub.views(), &self.hub_status),
            CreatorScreen::Settings { resume_workspace } => {
                let (project, play_active) = resume_workspace.as_deref().map_or_else(
                    || (None, false),
                    |workspace| {
                        (
                            Some(
                                workspace
                                    .root
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("Meridian Project")
                                    .to_owned(),
                            ),
                            workspace.session.play_active(),
                        )
                    },
                );
                creator_settings_document(&CreatorSettingsView {
                    project,
                    play_active,
                    high_contrast: self.hub.preferences.high_contrast,
                    reduced_motion: self.hub.preferences.reduced_motion,
                    density: self.hub.preferences.density.label().to_owned(),
                    query: settings_query,
                    status: self.settings_status.clone(),
                })
            }
            CreatorScreen::Workspace(workspace) => {
                let view = workspace.ui_view(self.logical_viewport);
                creator_workspace_document_with_view(&workspace.session, &view)
            }
        }
        .map_err(|error| io::Error::other(format!("Creator UI document invalid: {error:?}")))?;
        self.ui.replace_document(document);
        Ok(())
    }

    fn refresh_ui_without_events(&mut self) -> AppResult<()> {
        let frame = self.ui.reconcile(self.creator_frame_input(Vec::new()));
        self.frame = match &self.screen {
            CreatorScreen::Workspace(workspace)
                if matches!(
                    workspace.active_workspace(),
                    WorkspaceKind::World | WorkspaceKind::Code
                ) =>
            {
                decorate_world_viewport(&workspace.session, &frame)?
            }
            CreatorScreen::Workspace(workspace)
                if workspace.active_workspace() == WorkspaceKind::UiAuthoring =>
            {
                let target = creator_ui_authoring_target_frame(
                    &workspace.session,
                    &workspace.ui_view(self.logical_viewport),
                )
                .map_err(|error| {
                    io::Error::other(format!("UI authoring target document invalid: {error:?}"))
                })?;
                let target = decorate_world_viewport(&workspace.session, &target)?;
                decorate_ui_authoring_preview(&frame, &target)?
            }
            CreatorScreen::Workspace(workspace)
                if workspace.active_workspace() == WorkspaceKind::Modeler =>
            {
                let view = workspace.ui_view(self.logical_viewport);
                decorate_modeler_preview(view.modeler.as_ref(), &frame)?
            }
            CreatorScreen::Hub | CreatorScreen::Settings { .. } | CreatorScreen::Workspace(_) => {
                frame
            }
        };
        Ok(())
    }

    fn creator_frame_input(&self, events: Vec<UiEvent>) -> UiFrameInput {
        let preferences = self.hub.preferences;
        let mut input = UiFrameInput::new(self.logical_viewport);
        input.scale_factor = self.scale_factor;
        input.high_contrast = preferences.high_contrast;
        input.reduced_motion = preferences.reduced_motion;
        input.density = preferences.density.ui_density();
        input.contrast = if preferences.high_contrast {
            UiContrast::High
        } else {
            UiContrast::Standard
        };
        input.motion = if preferences.reduced_motion {
            MotionPreference::Reduced
        } else {
            MotionPreference::Full
        };
        input.events = events;
        input
    }

    fn refresh_document_and_ui(&mut self) -> AppResult<()> {
        self.refresh_document()?;
        if std::mem::replace(
            &mut self.inspector_field_sync,
            InspectorFieldSync::RetainDraft,
        ) == InspectorFieldSync::ResetFromAuthoritativeSource
        {
            let _ = self
                .ui
                .reset_text_input_from_document(CREATOR_INSPECTOR_X_MM);
            let _ = self
                .ui
                .reset_text_input_from_document(CREATOR_INSPECTOR_Y_MM);
            let _ = self
                .ui
                .reset_text_input_from_document(CREATOR_INSPECTOR_Z_MM);
        }
        self.refresh_ui_without_events()
    }

    fn reconcile_ui(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        let build_changed = poll_creator_build(&mut self.screen);
        let pending_events = std::mem::take(&mut self.pending_events);
        let had_pending_events = !pending_events.is_empty();
        let had_pending_actions = !self.pending_actions.is_empty();
        if !build_changed && !had_pending_events && !had_pending_actions {
            self.sync_ime_cursor_area(context.window());
            return Ok(());
        }
        let output = self.ui.reconcile(self.creator_frame_input(pending_events));
        let clipboard_requested = !output.clipboard_requests.is_empty();
        let mut commands = self.pending_actions.drain(..).collect::<Vec<_>>();
        match ui_effect_action_names(&output.commands, &output.assistive_requests) {
            Ok(frame_actions) => commands.extend(frame_actions),
            Err(error) => self.set_status(error.to_string()),
        }
        self.dispatch_ui_actions(commands, context.window());
        if clipboard_requested {
            self.set_status(
                "Clipboard access is unavailable until Meridian's platform adapter is active."
                    .to_owned(),
            );
        }
        self.refresh_document_and_ui()?;
        self.sync_ime_cursor_area(context.window());
        self.rebuild_renderer_for_display()?;
        self.schedule_bootstrap_renderer_refresh();
        Ok(())
    }

    fn sync_ime_cursor_area(&self, window: Option<&PlatformWindow>) {
        let Some(window) = window else {
            return;
        };
        let area = self
            .frame
            .focused
            .filter(|focused| {
                self.ui.document().node(*focused).is_some_and(|node| {
                    matches!(
                        node.kind,
                        UiWidgetKind::TextInput | UiWidgetKind::SearchInput
                    )
                })
            })
            .and_then(|focused| {
                self.frame
                    .layout
                    .iter()
                    .find(|snapshot| snapshot.node == focused)
                    .and_then(|snapshot| {
                        PlatformImeCursorArea::new(
                            snapshot.bounds.origin.x,
                            snapshot.bounds.origin.y,
                            snapshot.bounds.size.width,
                            snapshot.bounds.size.height,
                        )
                    })
            });
        window.set_ime_allowed(area.is_some());
        if let Some(area) = area {
            window.set_ime_cursor_area(area);
        }
    }

    fn dispatch_ui_actions(
        &mut self,
        commands: impl IntoIterator<Item = String>,
        wake_window: Option<&PlatformWindow>,
    ) {
        for command in commands {
            if let Err(error) = self.execute_action_with_window(&command, wake_window.cloned()) {
                self.set_status(format!("{error}"));
            }
        }
    }

    /// Drives bounded retained UI events through the same action dispatcher as
    /// the native application. The Creator Alpha process smoke uses it only
    /// for an already-open workspace, so it cannot invoke the hub picker.
    fn reconcile_workspace_ui_events_for_smoke(
        &mut self,
        events: Vec<UiEvent>,
    ) -> AppResult<Vec<UiCommandRequest>> {
        if !matches!(self.screen, CreatorScreen::Workspace(_)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Creator UI smoke requires an open workspace",
            )
            .into());
        }
        let output = self.ui.reconcile(self.creator_frame_input(events));
        let clipboard_requested = !output.clipboard_requests.is_empty();
        let commands = output.commands.clone();
        let actions = ui_effect_action_names(&commands, &output.assistive_requests)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        for action in &actions {
            if matches!(
                CreatorUiAction::parse(action)?,
                CreatorUiAction::CreateProject
                    | CreatorUiAction::OpenProject
                    | CreatorUiAction::OpenRecent(_)
                    | CreatorUiAction::LocateRecent(_)
                    | CreatorUiAction::RemoveRecent(_)
            ) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Creator UI smoke may not invoke project-picker actions",
                )
                .into());
            }
        }
        self.dispatch_ui_actions(actions, None);
        if clipboard_requested {
            self.set_status(
                "Clipboard access is unavailable until Meridian's platform adapter is active."
                    .to_owned(),
            );
        }
        self.refresh_document_and_ui()?;
        Ok(commands)
    }

    fn creator_action_node(&self, action: &str) -> AppResult<UiNodeId> {
        self.ui
            .document()
            .focus_order()
            .into_iter()
            .find(|id| {
                self.ui
                    .document()
                    .node(*id)
                    .and_then(|node| node.semantics.action.as_deref())
                    == Some(action)
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Creator UI action is absent from the active workspace: {action}"),
                )
                .into()
            })
    }

    fn activate_workspace_action_for_smoke(
        &mut self,
        action: &str,
    ) -> AppResult<Vec<UiCommandRequest>> {
        let target = self.creator_action_node(action)?;
        self.reconcile_workspace_ui_events_for_smoke(vec![UiEvent::AssistiveActivate(target)])
    }

    fn set_status(&mut self, status: String) {
        match &mut self.screen {
            CreatorScreen::Hub => self.hub_status = status,
            CreatorScreen::Settings { .. } => self.settings_status = status,
            CreatorScreen::Workspace(workspace) => workspace.status = status,
        }
    }

    fn inspector_translation(&self) -> AppResult<Translation> {
        Ok(Translation {
            x_mm: parse_creator_translation_component(
                "X",
                self.ui.text_input_value(CREATOR_INSPECTOR_X_MM),
            )?,
            y_mm: parse_creator_translation_component(
                "Y",
                self.ui.text_input_value(CREATOR_INSPECTOR_Y_MM),
            )?,
            z_mm: parse_creator_translation_component(
                "Z",
                self.ui.text_input_value(CREATOR_INSPECTOR_Z_MM),
            )?,
        })
    }

    fn open_project(&mut self, requested: &Path) -> AppResult<()> {
        let workspace = CreatorWorkspace::open(requested)?;
        self.activate_workspace(workspace);
        Ok(())
    }

    fn activate_workspace(&mut self, mut workspace: CreatorWorkspace) {
        if self.run_persistence == CreatorRunPersistence::Persistent {
            self.hub.remember(&workspace.root);
            if self.hub_store.save(&self.hub).is_err() {
                workspace.status.push_str(
                    " Recent-project state could not be saved; reopen this project manually after exit.",
                );
            }
        }
        self.inspector_field_sync = InspectorFieldSync::ResetFromAuthoritativeSource;
        self.screen = CreatorScreen::Workspace(Box::new(workspace));
    }

    fn open_settings(&mut self) {
        let previous = std::mem::replace(&mut self.screen, CreatorScreen::Hub);
        self.screen = match previous {
            CreatorScreen::Hub => CreatorScreen::Settings {
                resume_workspace: None,
            },
            CreatorScreen::Settings { resume_workspace } => {
                CreatorScreen::Settings { resume_workspace }
            }
            CreatorScreen::Workspace(workspace) => CreatorScreen::Settings {
                resume_workspace: Some(workspace),
            },
        };
        "Local preferences are ready.".clone_into(&mut self.settings_status);
    }

    fn return_from_settings(&mut self) {
        let previous = std::mem::replace(&mut self.screen, CreatorScreen::Hub);
        match previous {
            CreatorScreen::Settings {
                resume_workspace: Some(workspace),
            } => {
                self.screen = CreatorScreen::Workspace(workspace);
                self.set_status(
                    "Returned from Settings without changing project source.".to_owned(),
                );
            }
            CreatorScreen::Settings {
                resume_workspace: None,
            }
            | CreatorScreen::Hub => {
                self.screen = CreatorScreen::Hub;
                "Choose a project to open.".clone_into(&mut self.hub_status);
            }
            CreatorScreen::Workspace(workspace) => {
                self.screen = CreatorScreen::Workspace(workspace);
            }
        }
    }

    fn return_to_hub(&mut self) {
        if creator_has_active_build(&self.screen) {
            match &mut self.screen {
                CreatorScreen::Workspace(workspace) => {
                    "A build is still running. Keep this project open to retain progress and artifact reporting."
                        .clone_into(&mut workspace.status);
                }
                CreatorScreen::Settings { .. } => {
                    "A project build is still running. Return to the project before leaving Meridian."
                        .clone_into(&mut self.settings_status);
                }
                CreatorScreen::Hub => unreachable!("a hub cannot own an active Creator build"),
            }
            return;
        }
        self.screen = CreatorScreen::Hub;
        "Project remains saved; choose a project to open.".clone_into(&mut self.hub_status);
    }

    fn update_preferences(
        &mut self,
        update: impl FnOnce(&mut CreatorPreferences),
        status: String,
    ) -> AppResult<()> {
        if !matches!(self.screen, CreatorScreen::Settings { .. }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Meridian preferences may only change from the Settings surface",
            )
            .into());
        }
        let before = self.hub.preferences;
        update(&mut self.hub.preferences);
        if let Err(error) = self.hub_store.save(&self.hub) {
            self.hub.preferences = before;
            return Err(error);
        }
        self.settings_status = status;
        Ok(())
    }

    fn switch_workspace(&mut self, requested: WorkspaceKind) -> AppResult<()> {
        if requested == WorkspaceKind::Hub {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the Creator hub is not a project workspace destination",
            )
            .into());
        }
        if let CreatorScreen::Settings { resume_workspace } = &mut self.screen {
            let Some(workspace) = resume_workspace.take() else {
                "Open a project before switching workspaces.".clone_into(&mut self.settings_status);
                return Ok(());
            };
            self.screen = CreatorScreen::Workspace(workspace);
        }
        let CreatorScreen::Workspace(workspace) = &mut self.screen else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "open a project before switching Meridian workspaces",
            )
            .into());
        };
        let activation = workspace.activate_workspace(requested)?;
        workspace.status = match activation {
            WorkspaceActivation::Switched => {
                format!(
                    "{} workspace is active and its layout was saved.",
                    creator_workspace_name(requested)
                )
            }
            WorkspaceActivation::FocusEntered => format!(
                "{} entered its remembered focused layout.",
                creator_workspace_name(requested)
            ),
            WorkspaceActivation::FocusExited => format!(
                "{} returned to its contextual layout.",
                creator_workspace_name(requested)
            ),
        };
        Ok(())
    }

    fn locate_recent_project(
        &mut self,
        index: usize,
        picker: &dyn CreatorProjectPicker,
        window: Option<&PlatformWindow>,
    ) -> AppResult<()> {
        if !matches!(self.screen, CreatorScreen::Hub) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "recent-project remediation is available only from the project hub",
            )
            .into());
        }
        if index >= self.hub.recents.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "recent project index is unavailable",
            )
            .into());
        }
        let Some(root) = picker.pick_directory(window) else {
            "Recent-project location was not changed.".clone_into(&mut self.hub_status);
            return Ok(());
        };
        match CreatorWorkspace::open(&root) {
            Ok(workspace) => {
                self.hub.recents.remove(index);
                self.activate_workspace(workspace);
            }
            Err(error) => {
                self.hub_status = format!("The selected replacement is not openable: {error}");
            }
        }
        Ok(())
    }

    /// Runs the native directory adapter only for an explicit Creator hub action.
    ///
    /// Cancellation and invalid selections remain visible hub diagnostics rather
    /// than becoming ambient filesystem authority or application-fatal errors.
    #[allow(clippy::assigning_clones)] // Hub diagnostics are infrequent user-visible state changes.
    fn execute_explicit_picker_action(
        &mut self,
        action: CreatorUiAction,
        picker: &dyn CreatorProjectPicker,
        window: Option<&meridian_platform::PlatformWindow>,
    ) -> AppResult<()> {
        if !matches!(
            action,
            CreatorUiAction::CreateProject | CreatorUiAction::OpenProject
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only explicit Creator create or open actions may invoke the project picker",
            )
            .into());
        }
        if !matches!(self.screen, CreatorScreen::Hub) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "project directory selection is available only from the Creator hub",
            )
            .into());
        }
        let selection = picker.pick_directory(window);
        match (action, selection) {
            (CreatorUiAction::CreateProject, None) => {
                self.hub_status = "Project creation cancelled.".to_owned();
            }
            (CreatorUiAction::OpenProject, None) => {
                self.hub_status = "Project open cancelled.".to_owned();
            }
            (CreatorUiAction::CreateProject, Some(parent)) => {
                let name = self
                    .ui
                    .text_input_value(CREATOR_HUB_PROJECT_NAME)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if let Err(error) = create_public_creator_project(&parent, &name)
                    .and_then(|root| self.open_project(&root))
                {
                    self.hub_status = format!("Unable to create the selected project: {error}");
                }
            }
            (CreatorUiAction::OpenProject, Some(root)) => {
                if let Err(error) = self.open_project(&root) {
                    self.hub_status = format!("Unable to open the selected project: {error}");
                }
            }
            _ => unreachable!("explicit picker actions were validated before picking"),
        }
        Ok(())
    }

    #[allow(clippy::assigning_clones, clippy::too_many_lines)]
    // Hub diagnostics are user-visible and the complete typed action allow-list stays auditable.
    fn execute_action_with_window(
        &mut self,
        action: &str,
        wake_window: Option<PlatformWindow>,
    ) -> AppResult<()> {
        match CreatorUiAction::parse(action)? {
            CreatorUiAction::CreateProject => {
                self.execute_explicit_picker_action(
                    CreatorUiAction::CreateProject,
                    &NativeCreatorProjectPicker,
                    wake_window.as_ref(),
                )?;
            }
            CreatorUiAction::OpenProject => {
                self.execute_explicit_picker_action(
                    CreatorUiAction::OpenProject,
                    &NativeCreatorProjectPicker,
                    wake_window.as_ref(),
                )?;
            }
            CreatorUiAction::OpenRecent(index) => {
                let path = self
                    .hub
                    .recents
                    .get(index)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "recent project index is unavailable",
                        )
                    })?
                    .path
                    .clone();
                self.open_project(Path::new(&path))?;
            }
            CreatorUiAction::LocateRecent(index) => {
                self.locate_recent_project(
                    index,
                    &NativeCreatorProjectPicker,
                    wake_window.as_ref(),
                )?;
            }
            CreatorUiAction::RemoveRecent(index) => {
                if index >= self.hub.recents.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "recent project index is unavailable",
                    )
                    .into());
                }
                self.hub.recents.remove(index);
                self.hub_store.save(&self.hub)?;
                self.hub_status = "Recent project removed.".to_owned();
            }
            CreatorUiAction::ReturnHub => self.return_to_hub(),
            CreatorUiAction::SwitchWorkspace(workspace) => self.switch_workspace(workspace)?,
            CreatorUiAction::ShellSearch => {
                let target = match &self.screen {
                    CreatorScreen::Hub => CREATOR_HUB_PROJECT_NAME,
                    CreatorScreen::Settings { .. } => CREATOR_SETTINGS_SEARCH,
                    CreatorScreen::Workspace(workspace)
                        if workspace.active_workspace() == WorkspaceKind::World =>
                    {
                        meridian_ui_editor::CREATOR_WORLD_SEARCH
                    }
                    CreatorScreen::Workspace(_) => meridian_ui_editor::CREATOR_DOMAIN_SEARCH,
                };
                if !self.ui.focus_retained_node(target) {
                    self.set_status("Search is unavailable in the current surface.".to_owned());
                }
            }
            CreatorUiAction::OpenSettings => self.open_settings(),
            CreatorUiAction::ReturnFromSettings => self.return_from_settings(),
            CreatorUiAction::ToggleHighContrast => {
                let enabled = !self.hub.preferences.high_contrast;
                self.update_preferences(
                    |preferences| preferences.high_contrast = enabled,
                    format!("High contrast is {}.", if enabled { "on" } else { "off" }),
                )?;
            }
            CreatorUiAction::ToggleReducedMotion => {
                let enabled = !self.hub.preferences.reduced_motion;
                self.update_preferences(
                    |preferences| preferences.reduced_motion = enabled,
                    format!("Reduced motion is {}.", if enabled { "on" } else { "off" }),
                )?;
            }
            CreatorUiAction::SetDensity(density) => self.update_preferences(
                |preferences| preferences.density = density,
                format!("Interface density is {}.", density.label()),
            )?,
            CreatorUiAction::ResetPreferences => self.update_preferences(
                |preferences| *preferences = CreatorPreferences::default(),
                "Local preferences were reset to Meridian defaults.".to_owned(),
            )?,
            CreatorUiAction::ShellFavorites => {
                let status = match &self.screen {
                    CreatorScreen::Hub => "Open a project before using favorites.".to_owned(),
                    CreatorScreen::Settings { .. } => {
                        "Favorites are unavailable from application Settings.".to_owned()
                    }
                    CreatorScreen::Workspace(workspace)
                        if workspace.active_workspace() == WorkspaceKind::World =>
                    {
                        "World favorites require their own source package.".to_owned()
                    }
                    CreatorScreen::Workspace(workspace) => format!(
                        "{} favorites are not registered by an active source package.",
                        creator_workspace_name(workspace.active_workspace())
                    ),
                };
                self.set_status(status);
            }
            CreatorUiAction::ShellPanels => match &mut self.screen {
                CreatorScreen::Hub => {
                    self.hub_status = "Open a project before cycling workspace panes.".to_owned();
                }
                CreatorScreen::Settings {
                    resume_workspace: Some(workspace),
                } => {
                    let panel = workspace.cycle_panel_focus()?;
                    self.settings_status = format!(
                        "{} pane is active in the retained project layout.",
                        creator_panel_name(panel)
                    );
                }
                CreatorScreen::Settings {
                    resume_workspace: None,
                } => {
                    self.settings_status =
                        "Open a project before cycling workspace panes.".to_owned();
                }
                CreatorScreen::Workspace(workspace) => {
                    let panel = workspace.cycle_panel_focus()?;
                    workspace.status = format!(
                        "{} pane is active in the persisted {} layout.",
                        creator_panel_name(panel),
                        creator_workspace_name(workspace.active_workspace())
                    );
                }
            },
            CreatorUiAction::ShellOpenShelf => match &mut self.screen {
                CreatorScreen::Hub => {
                    self.hub_status =
                        "Open a project before viewing source history and recovery.".to_owned();
                }
                CreatorScreen::Settings {
                    resume_workspace: Some(workspace),
                }
                | CreatorScreen::Workspace(workspace) => {
                    workspace.focus_panel(EditorPanelId::History)?;
                    workspace.status =
                        "History, build, and recovery are open in the retained project layout."
                            .to_owned();
                }
                CreatorScreen::Settings {
                    resume_workspace: None,
                } => {
                    self.settings_status =
                        "Open a project before viewing source history and recovery.".to_owned();
                }
            },
            CreatorUiAction::ShellPlayUnavailable => {
                self.hub_status = "Open a project before starting Play.".to_owned();
            }
            CreatorUiAction::ShellBuildUnavailable => {
                self.hub_status = "Open a project before building.".to_owned();
            }
            action => self.execute_workspace_action_with_wake(action, wake_window)?,
        }
        Ok(())
    }

    #[allow(clippy::assigning_clones, clippy::too_many_lines)]
    // The platform window is an optional wake-only adapter for asynchronous build completion.
    fn execute_workspace_action_with_wake(
        &mut self,
        action: CreatorUiAction,
        wake_window: Option<PlatformWindow>,
    ) -> AppResult<()> {
        let inspector_translation = matches!(
            action,
            CreatorUiAction::EditPlacement | CreatorUiAction::PreviewCommand
        )
        .then(|| self.inspector_translation())
        .transpose()?;
        let workspace = match &mut self.screen {
            CreatorScreen::Workspace(workspace)
            | CreatorScreen::Settings {
                resume_workspace: Some(workspace),
            } => workspace,
            CreatorScreen::Hub
            | CreatorScreen::Settings {
                resume_workspace: None,
            } => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "this Creator action requires an open project",
                )
                .into());
            }
        };
        let mut reset_inspector_fields = false;
        match action {
            CreatorUiAction::Recover => {
                let opened = workspace.project_store.open()?;
                workspace.session = opened.session;
                workspace.recovery_status = opened.recovery;
                workspace.status = format!("Recovered source session: {:?}.", opened.recovery);
                reset_inspector_fields = true;
            }
            CreatorUiAction::StartPlay => {
                workspace
                    .project_store
                    .mutate_play(&mut workspace.session, EditorSession::start_play)?;
                workspace.status =
                    "Play session started; source remains unchanged until Apply.".to_owned();
            }
            CreatorUiAction::ApplyPlay => {
                let changes = workspace
                    .project_store
                    .mutate(&mut workspace.session, EditorSession::apply_play)?;
                workspace.status = format!(
                    "Applied {} explicit Play change(s) to source.",
                    changes.len()
                );
                reset_inspector_fields = true;
            }
            CreatorUiAction::DiscardPlay => {
                workspace
                    .project_store
                    .mutate_play(&mut workspace.session, EditorSession::discard_play)?;
                workspace.status = "Discarded the isolated Play session.".to_owned();
                reset_inspector_fields = true;
            }
            CreatorUiAction::SelectPlacement => {
                let placement = first_placement_id(&workspace.session)?;
                workspace.session.select([placement])?;
                workspace.status = format!("Selected placement {placement}.");
                reset_inspector_fields = true;
            }
            CreatorUiAction::EditPlacement => {
                let placement_id = selected_or_first_placement(&workspace.session)?;
                let before = workspace.session.document().placements[&placement_id].translation;
                let translation = inspector_translation.ok_or_else(|| {
                    io::Error::other(
                        "Creator placement action omitted parsed inspector coordinates",
                    )
                })?;
                if workspace.session.play_active() {
                    workspace
                        .project_store
                        .mutate_play(&mut workspace.session, |session| {
                            session.set_play_translation(placement_id, translation)
                        })?;
                    workspace.status =
                        "Applied typed X/Y/Z translation to the isolated Play session. Apply keeps it; Discard removes it."
                            .to_owned();
                } else if translation == before {
                    workspace.status = "No source change: the typed placement coordinates already match the authoritative document.".to_owned();
                } else {
                    workspace
                        .project_store
                        .mutate(&mut workspace.session, |session| {
                            session.commit(creator_transaction(
                                EditorCommand::SetPlacementTranslation {
                                    placement_id,
                                    translation,
                                },
                                "Edit placement translation",
                            ))
                        })?;
                    workspace.status =
                        "Applied typed X/Y/Z source translation and persisted canonical project JSON."
                            .to_owned();
                    reset_inspector_fields = true;
                }
            }
            CreatorUiAction::PreviewCommand => {
                let placement_id = selected_or_first_placement(&workspace.session)?;
                let translation = inspector_translation.ok_or_else(|| {
                    io::Error::other("Creator preview action omitted parsed inspector coordinates")
                })?;
                workspace.session.preview(&creator_transaction(
                    EditorCommand::SetPlacementTranslation {
                        placement_id,
                        translation,
                    },
                    "Preview placement translation",
                ))?;
                workspace.status =
                    "Typed placement command preview succeeded without mutation.".to_owned();
            }
            CreatorUiAction::Undo => {
                workspace
                    .project_store
                    .mutate(&mut workspace.session, EditorSession::undo)?;
                workspace.status = "Undid the latest typed source command.".to_owned();
                reset_inspector_fields = true;
            }
            CreatorUiAction::Redo => {
                workspace
                    .project_store
                    .mutate(&mut workspace.session, EditorSession::redo)?;
                workspace.status = "Redid the latest typed source command.".to_owned();
                reset_inspector_fields = true;
            }
            CreatorUiAction::Reimport => {
                let imported = import_creator_alpha_source(
                    &workspace.root,
                    &workspace.manifest.imported_asset,
                )?;
                workspace
                    .project_store
                    .mutate(&mut workspace.session, |session| {
                        session.commit(creator_transaction(
                            EditorCommand::UpdateImportedSource(imported),
                            "Reimport source",
                        ))
                    })?;
                workspace.status =
                    "Reimported source identity and persisted the new hash metadata.".to_owned();
            }
            CreatorUiAction::InspectSource => {
                workspace.status = format!(
                    "Imported source has {} registered source record(s).",
                    workspace.session.document().sources.len()
                );
            }
            CreatorUiAction::SubmitBuild => workspace.start_build(wake_window)?,
            CreatorUiAction::InspectBuild => {
                workspace.status = if workspace.build.is_some() {
                    "Build is running through the durable one-worker Cargo service.".to_owned()
                } else {
                    workspace.status.clone()
                };
            }
            CreatorUiAction::RecipeInspect => {
                workspace.status = format!(
                    "Recipe {} uses {}.",
                    workspace.recipe.id, workspace.recipe.schema
                );
            }
            CreatorUiAction::RecipeValidate => {
                workspace.recipe.validate()?;
                workspace.status = "Canonical procedural recipe validation passed.".to_owned();
            }
            CreatorUiAction::RecipeMigrate => {
                let canonical = workspace.recipe.canonical_json()?;
                workspace.status = format!(
                    "Recipe is already v1; canonical source has {} bytes.",
                    canonical.len()
                );
            }
            CreatorUiAction::RecipePreview => {
                let preview = evaluate_alluvium(&workspace.recipe, EvaluationMode::Preview)?;
                workspace.status = format!(
                    "Recipe preview generated {} placement(s).",
                    preview.field.samples.len()
                );
            }
            CreatorUiAction::RecipeBake => {
                let audit = license_audit(&workspace.recipe, "creator-alpha-editor")?;
                if !audit.accepted {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "recipe bake rejected by license policy",
                    )
                    .into());
                }
                let bake = evaluate_alluvium(&workspace.recipe, EvaluationMode::Bake)?;
                workspace.status = format!(
                    "Recipe bake generated {} placement(s).",
                    bake.field.samples.len()
                );
            }
            CreatorUiAction::RecipeDirty => {
                let report = dirty_report(&workspace.recipe, &workspace.recipe);
                workspace.status =
                    format!("Recipe dirty report: {} reason(s).", report.reasons.len());
            }
            CreatorUiAction::RecipeExplain => {
                let preview = evaluate_alluvium(&workspace.recipe, EvaluationMode::Preview)?;
                let first = preview.field.samples.first().ok_or_else(|| {
                    io::Error::other("recipe preview produced no generated identities")
                })?;
                let explanation =
                    explain_generated(&workspace.recipe, first.id)?.ok_or_else(|| {
                        io::Error::other("recipe generated identity has no explanation")
                    })?;
                workspace.status = format!(
                    "Recipe explanation retained generated ID {}.",
                    explanation.id
                );
            }
            CreatorUiAction::RecipeProvenance => {
                workspace.status = format!(
                    "Recipe provenance: {} ({})",
                    workspace.recipe.provenance.origin, workspace.recipe.provenance.license
                );
            }
            CreatorUiAction::RecipeLicenseAudit => {
                let audit = license_audit(&workspace.recipe, "creator-alpha-editor")?;
                workspace.status = format!("Recipe license audit accepted: {}.", audit.accepted);
            }
            CreatorUiAction::ModelInspect => {
                workspace.status = format!(
                    "Editable model has {} source object(s).",
                    workspace.model_session.current().document().objects.len()
                );
            }
            CreatorUiAction::ModelCreatePrimitive => {
                let first_id =
                    next_model_id_range(workspace.model_session.current().document(), 10)?;
                let primitive = creator_alpha_public_quad_with_first_id(first_id);
                workspace.mutate_model(|session| {
                    session.apply(ModelTransaction::new(
                        "Create Creator primitive",
                        ModelCommand::CreatePrimitive(primitive),
                    ))
                })?;
                workspace.status =
                    "Created one typed editable primitive and saved source.".to_owned();
            }
            CreatorUiAction::ModelTransform => {
                let object_id = workspace
                    .model_session
                    .current()
                    .document()
                    .objects
                    .first()
                    .ok_or_else(|| io::Error::other("editable model has no selectable object"))?
                    .id;
                workspace.mutate_model(|session| {
                    session.select(ModelElementKind::Object, [object_id])?;
                    session.apply(
                        ModelTransaction::new(
                            "Translate editable object",
                            ModelCommand::TranslateObject {
                                object_id,
                                translation_mm: Millimetres3 { x: 50, y: 0, z: 0 },
                            },
                        )
                        .with_selection(session.selection().clone()),
                    )
                })?;
                workspace.status =
                    "Translated selected editable model object and saved source.".to_owned();
            }
            CreatorUiAction::ModelSplitEdge => {
                let object = workspace
                    .model_session
                    .current()
                    .document()
                    .objects
                    .first()
                    .ok_or_else(|| io::Error::other("editable model has no source object"))?;
                let object_id = object.id;
                let edge = object
                    .edges
                    .first()
                    .cloned()
                    .ok_or_else(|| io::Error::other("editable model has no source edge"))?;
                let first_id =
                    next_model_id_range(workspace.model_session.current().document(), 3)?;
                let vertex_id = StableId::new(first_id);
                let first_edge_id = StableId::new(first_id + 1);
                let second_edge_id = StableId::new(first_id + 2);
                workspace.mutate_model(|session| {
                    session.select(ModelElementKind::Edge, [edge.id])?;
                    session.apply(
                        ModelTransaction::new(
                            "Split editable model edge",
                            ModelCommand::SplitEdge(SplitEdge {
                                object_id,
                                edge_id: edge.id,
                                new_vertex: Vertex {
                                    id: vertex_id,
                                    position_mm: Millimetres3 {
                                        x: 0,
                                        y: -500,
                                        z: -500,
                                    },
                                },
                                replacement_edges: [
                                    Edge {
                                        id: first_edge_id,
                                        start: edge.start,
                                        end: vertex_id,
                                    },
                                    Edge {
                                        id: second_edge_id,
                                        start: vertex_id,
                                        end: edge.end,
                                    },
                                ],
                            }),
                        )
                        .with_selection(session.selection().clone()),
                    )
                })?;
                workspace.status =
                    "Applied bounded topology split with stable lineage and saved source."
                        .to_owned();
            }
            CreatorUiAction::ModelUndo => {
                workspace.mutate_model(ModelSession::undo)?;
                workspace.status = "Undid the latest semantic model operation.".to_owned();
            }
            CreatorUiAction::ModelRedo => {
                workspace.mutate_model(ModelSession::redo)?;
                workspace.status = "Redid the latest semantic model operation.".to_owned();
            }
            CreatorUiAction::ModelRecover => {
                let source = ModelDocument::read_source(&workspace.model_path)?;
                let recovered = workspace.model_recovery.load()?;
                if recovered.current().document() != &source {
                    return Err(io::Error::other(
                        "model recovery disagrees with authoritative source",
                    )
                    .into());
                }
                workspace.model_session = recovered;
                workspace.status =
                    "Recovered model history matching authoritative source.".to_owned();
            }
            CreatorUiAction::FocusSelection => {
                workspace.status = format!(
                    "Focused {} selected source element(s).",
                    workspace.session.selection().ids.len()
                );
            }
            CreatorUiAction::ShowDiagnostics => {
                workspace.status = format!(
                    "Diagnostics are visible below: recovery is {:?}; {} source checkpoint(s) are available.",
                    workspace.recovery_status,
                    workspace.session.checkpoints().len()
                );
            }
            CreatorUiAction::CreateProject
            | CreatorUiAction::OpenProject
            | CreatorUiAction::OpenRecent(_)
            | CreatorUiAction::LocateRecent(_)
            | CreatorUiAction::RemoveRecent(_)
            | CreatorUiAction::ReturnHub
            | CreatorUiAction::SwitchWorkspace(_)
            | CreatorUiAction::ShellSearch
            | CreatorUiAction::OpenSettings
            | CreatorUiAction::ReturnFromSettings
            | CreatorUiAction::ToggleHighContrast
            | CreatorUiAction::ToggleReducedMotion
            | CreatorUiAction::SetDensity(_)
            | CreatorUiAction::ResetPreferences
            | CreatorUiAction::ShellFavorites
            | CreatorUiAction::ShellPanels
            | CreatorUiAction::ShellOpenShelf
            | CreatorUiAction::ShellPlayUnavailable
            | CreatorUiAction::ShellBuildUnavailable => {
                unreachable!("handled before workspace dispatch")
            }
        }
        if reset_inspector_fields {
            self.inspector_field_sync = InspectorFieldSync::ResetFromAuthoritativeSource;
        }
        Ok(())
    }

    fn refresh_for_size(&mut self, size: WindowSize, scale_factor: f64) -> AppResult<()> {
        self.physical_size = size;
        self.scale_factor = f64_to_f32(scale_factor).clamp(0.5, 4.0);
        self.logical_viewport = UiSize::new(
            f64_to_f32(f64::from(size.width) / f64::from(self.scale_factor)),
            f64_to_f32(f64::from(size.height) / f64::from(self.scale_factor)),
        );
        self.refresh_document_and_ui()?;
        let Some(mut rhi) = self.rhi.take() else {
            return Ok(());
        };
        rhi.resize(size);
        let renderer = rebuild_for_renderable_creator_surface(size, || {
            build_creator_direct_renderer(
                &mut rhi,
                &self.frame,
                self.logical_viewport,
                self.scale_factor,
            )
        })?;
        let Some(renderer) = renderer else {
            self.rhi = Some(rhi);
            return Ok(());
        };
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        self.schedule_bootstrap_renderer_refresh();
        Ok(())
    }

    fn rebuild_renderer_for_display(&mut self) -> AppResult<()> {
        let Some(mut rhi) = self.rhi.take() else {
            return Ok(());
        };
        let renderer = rebuild_for_renderable_creator_surface(self.physical_size, || {
            build_creator_direct_renderer(
                &mut rhi,
                &self.frame,
                self.logical_viewport,
                self.scale_factor,
            )
        })?;
        let Some(renderer) = renderer else {
            self.rhi = Some(rhi);
            return Ok(());
        };
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        self.structural_fallback_submitted = false;
        self.surface_attempts = 0;
        Ok(())
    }

    fn initialize_gpu(&mut self, window: meridian_platform::PlatformWindow) -> AppResult<()> {
        self.refresh_for_size(window.size(), window.scale_factor())?;
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let renderer = rebuild_for_renderable_creator_surface(self.physical_size, || {
            build_creator_direct_renderer(
                &mut rhi,
                &self.frame,
                self.logical_viewport,
                self.scale_factor,
            )
        })?;
        let Some(renderer) = renderer else {
            self.rhi = Some(rhi);
            return Ok(());
        };
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        self.schedule_bootstrap_renderer_refresh();
        Ok(())
    }

    fn begin_review_capture(
        &mut self,
        rhi: &mut Rhi,
        renderer: &CreatorDirectUiRenderer,
    ) -> AppResult<()> {
        if !matches!(self.review_capture, CreatorReviewCapture::Pending(_)) {
            return Ok(());
        }
        rhi.request_capture(CaptureRequest::new(
            FrameId::new(self.frame.revision.max(1)),
            4_096,
            4_096,
            64 * 1024 * 1024,
        ))?;
        renderer
            .gpu
            .submit_offscreen_capture(rhi, &renderer.plan, creator_ui_clear_color())?;
        let path = match std::mem::replace(&mut self.review_capture, CreatorReviewCapture::Disabled)
        {
            CreatorReviewCapture::Pending(path) => path,
            state => {
                self.review_capture = state;
                return Ok(());
            }
        };
        self.review_capture = CreatorReviewCapture::Requested(path);
        Ok(())
    }

    fn write_review_capture_if_ready(&mut self) -> AppResult<bool> {
        let path = match &self.review_capture {
            CreatorReviewCapture::Disabled | CreatorReviewCapture::Pending(_) => return Ok(false),
            CreatorReviewCapture::Requested(path) => path,
            CreatorReviewCapture::Written => return Ok(true),
        };
        let Some(rhi) = self.rhi.as_mut() else {
            return Ok(false);
        };
        let Some(outcome) = rhi.take_capture() else {
            return Ok(false);
        };
        let captured = match outcome {
            CaptureOutcome::Captured(frame) => frame,
            outcome => {
                return Err(io::Error::other(format!(
                    "Creator UI review capture did not produce pixels: {outcome:?}"
                ))
                .into())
            }
        };
        if !has_multiple_pixel_values(&captured) {
            return Err(io::Error::other(
                "Creator UI review capture contains only one pixel value",
            )
            .into());
        }
        write_capture_png(path, &captured)?;
        println!(
            "Creator UI review capture written to {} ({}x{}, {:?})",
            path.display(),
            captured.width,
            captured.height,
            captured.source
        );
        self.review_capture = CreatorReviewCapture::Written;
        Ok(true)
    }

    /// Schedules one renderer rebuild after a newly configured native surface.
    ///
    /// On macOS the first accepted direct frame can precede compositor-visible
    /// contents. The next frame rebuilds the immutable GPU frame, matching the
    /// normal display-list update path without creating a permanent loop.
    fn schedule_bootstrap_renderer_refresh(&mut self) {
        self.bootstrap_renderer_refresh_pending = true;
    }

    fn take_bootstrap_renderer_refresh(&mut self) -> bool {
        std::mem::take(&mut self.bootstrap_renderer_refresh_pending)
    }

    fn record_terminal_error(&mut self, detail: impl Display) {
        let terminal_error =
            PlatformError::application(format!("Meridian Creator application error: {detail}"));
        eprintln!("{terminal_error}");
        self.terminal_error = Some(terminal_error);
    }

    fn render(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        self.reconcile_ui(context)?;
        if !creator_surface_is_renderable(self.physical_size) {
            return Ok(());
        }
        let outcome = match (self.rhi.as_mut(), self.renderer.as_ref()) {
            (Some(rhi), Some(renderer)) => {
                renderer
                    .gpu
                    .present(rhi, &renderer.plan, creator_ui_clear_color())?
            }
            _ => {
                return Err(
                    io::Error::other("Creator application has no initialized renderer").into(),
                )
            }
        };
        let review_capture_complete = self.write_review_capture_if_ready()?;
        if self.take_bootstrap_renderer_refresh() {
            self.rebuild_renderer_for_display()?;
            context.request_redraw();
            return Ok(());
        }
        if outcome.visible() {
            self.visible_presentations = self.visible_presentations.saturating_add(1);
            if self.native_smoke && self.review_capture.is_enabled() {
                if review_capture_complete {
                    context.exit();
                } else if self.visible_presentations < CREATOR_UI_SMOKE_VISIBLE_PRESENTATIONS {
                    // Let the refreshed direct plan reach the same two visible
                    // presentations required by native smoke before asking
                    // the GPU to copy review pixels. The first refreshed
                    // frame can still be warming text/atlas resources on
                    // macOS even though retained layout is already correct.
                    context.request_redraw();
                } else {
                    // Begin the offscreen readback only after a rebuilt direct
                    // plan has reached the settled native-smoke presentation
                    // count. Capturing during bootstrap can race glyph-atlas
                    // and compositor setup and leave local visual evidence
                    // partially drawn or blank despite a correct retained
                    // frame.
                    let Some(mut rhi) = self.rhi.take() else {
                        return Err(io::Error::other(
                            "Creator application lost its renderer before the settled UI review capture",
                        )
                        .into());
                    };
                    let Some(renderer) = self.renderer.take() else {
                        self.rhi = Some(rhi);
                        return Err(io::Error::other(
                            "Creator application lost its renderer before the settled UI review capture",
                        )
                        .into());
                    };
                    let capture = self.begin_review_capture(&mut rhi, &renderer);
                    self.rhi = Some(rhi);
                    self.renderer = Some(renderer);
                    capture?;
                    context.request_redraw();
                }
                return Ok(());
            }
            let build_active = creator_has_active_build(&self.screen);
            if creator_exits_after_visible_presentation(
                self.native_smoke,
                self.visible_presentations,
            ) {
                context.exit();
            } else if creator_requests_follow_up_redraw(
                self.native_smoke,
                self.visible_presentations,
                build_active,
            ) {
                context.request_redraw();
            }
            return Ok(());
        }
        self.surface_attempts = self.surface_attempts.saturating_add(1);
        if !self.structural_fallback_submitted {
            let (Some(rhi), Some(renderer)) = (self.rhi.as_mut(), self.renderer.as_ref()) else {
                return Err(io::Error::other("Creator structural fallback has no renderer").into());
            };
            renderer.gpu.submit_structural_validation(
                rhi,
                &renderer.plan,
                creator_ui_clear_color(),
            )?;
            self.structural_fallback_submitted = true;
        }
        if creator_exits_after_surface_attempt(self.native_smoke, self.surface_attempts) {
            context.exit();
        } else if self.surface_attempts < UI_SMOKE_MAX_PRESENT_ATTEMPTS {
            context.request_redraw();
        }
        Ok(())
    }

    fn route_input(&mut self, event: NativeInputEvent) {
        match event {
            NativeInputEvent::Button { control, down } => self.route_button_input(control, down),
            NativeInputEvent::Scroll(event) => self.route_scroll_input(event),
            NativeInputEvent::FocusLost => {
                self.pending_events.push(UiEvent::CancelInteraction);
            }
            NativeInputEvent::MouseMotion { .. } => {}
        }
    }

    /// Forwards positioned pointer movement through the retained UI even when
    /// no drag is active. Hover, tooltip timing, canvas panning, and timeline
    /// scrubbing all depend on the same normalized move phase; withholding it
    /// outside a drag would make the native Creator surface disagree with the
    /// framework contract.
    fn route_pointer_move(&mut self, physical_x: f32, physical_y: f32) {
        self.pointer = UiPoint {
            x: physical_x / self.scale_factor,
            y: physical_y / self.scale_factor,
        };
        self.pending_events.push(UiEvent::Pointer(UiPointerEvent {
            device: CREATOR_POINTER_DEVICE,
            kind: UiInputDeviceKind::Mouse,
            phase: UiPointerPhase::Move,
            position: self.pointer,
            button: None,
        }));
    }

    fn route_button_input(&mut self, control: ButtonControl, down: bool) {
        match control {
            ButtonControl::Mouse(button) => {
                let button = match button {
                    NativeMouseButton::Left => UiPointerButton::Primary,
                    NativeMouseButton::Right => UiPointerButton::Secondary,
                    NativeMouseButton::Middle => UiPointerButton::Middle,
                    NativeMouseButton::Other(button) => UiPointerButton::Auxiliary(button),
                };
                self.pending_events.push(UiEvent::Pointer(UiPointerEvent {
                    device: CREATOR_POINTER_DEVICE,
                    kind: UiInputDeviceKind::Mouse,
                    phase: if down {
                        UiPointerPhase::Press
                    } else {
                        UiPointerPhase::Release
                    },
                    position: self.pointer,
                    button: Some(button),
                }));
            }
            control if down => self.route_key_down(control),
            _ => {}
        }
    }

    fn route_key_down(&mut self, control: ButtonControl) {
        match control {
            ButtonControl::Key(KeyCode::Tab) => self.pending_events.push(if self.modifiers.shift {
                UiEvent::FocusPrevious
            } else {
                UiEvent::FocusNext
            }),
            ButtonControl::Key(KeyCode::Enter | KeyCode::Space) => self.queue_activation(),
            ButtonControl::Key(KeyCode::Backspace) => {
                self.pending_events.push(UiEvent::DeleteTextBackward);
            }
            ButtonControl::Key(KeyCode::Delete) => {
                self.pending_events.push(UiEvent::DeleteTextForward);
            }
            ButtonControl::Key(KeyCode::Left) => {
                self.pending_events.push(UiEvent::MoveTextCursor {
                    direction: UiTextCursorDirection::Backward,
                    extend_selection: self.modifiers.shift,
                });
            }
            ButtonControl::Key(KeyCode::Right) => {
                self.pending_events.push(UiEvent::MoveTextCursor {
                    direction: UiTextCursorDirection::Forward,
                    extend_selection: self.modifiers.shift,
                });
            }
            ButtonControl::Key(KeyCode::Up) => self.pending_events.push(
                UiEvent::NavigateCollection(UiCollectionNavigation::Previous),
            ),
            ButtonControl::Key(KeyCode::Down) => self
                .pending_events
                .push(UiEvent::NavigateCollection(UiCollectionNavigation::Next)),
            ButtonControl::Key(KeyCode::Home) => {
                self.route_home_end(UiTextCursorDirection::Start, UiCollectionNavigation::Home);
            }
            ButtonControl::Key(KeyCode::End) => {
                self.route_home_end(UiTextCursorDirection::End, UiCollectionNavigation::End);
            }
            ButtonControl::Key(KeyCode::PageUp) => self.pending_events.push(
                UiEvent::NavigateCollection(UiCollectionNavigation::PageBackward),
            ),
            ButtonControl::Key(KeyCode::PageDown) => self.pending_events.push(
                UiEvent::NavigateCollection(UiCollectionNavigation::PageForward),
            ),
            ButtonControl::Key(KeyCode::Z) if self.modifiers.primary_command() => {
                if self.text_input_focused() {
                    self.pending_events.push(if self.modifiers.shift {
                        UiEvent::RedoText
                    } else {
                        UiEvent::UndoText
                    });
                } else {
                    self.pending_actions.push(if self.modifiers.shift {
                        "editor.redo".to_owned()
                    } else {
                        "editor.undo".to_owned()
                    });
                }
            }
            ButtonControl::Key(KeyCode::Y) if self.modifiers.primary_command() => {
                if self.text_input_focused() {
                    self.pending_events.push(UiEvent::RedoText);
                } else {
                    self.pending_actions.push("editor.redo".to_owned());
                }
            }
            ButtonControl::Key(KeyCode::A) if self.modifiers.primary_command() => {
                self.pending_events.push(UiEvent::SelectAllText);
            }
            ButtonControl::Key(KeyCode::C) if self.modifiers.primary_command() => {
                self.pending_events.push(UiEvent::CopySelection);
            }
            ButtonControl::Key(KeyCode::X) if self.modifiers.primary_command() => {
                self.pending_events.push(UiEvent::CutSelection);
            }
            ButtonControl::Key(KeyCode::Escape) => {
                if matches!(self.screen, CreatorScreen::Settings { .. }) {
                    self.pending_actions.push("settings.return".to_owned());
                    return;
                }
                if let CreatorScreen::Workspace(workspace) = &self.screen {
                    if workspace.active_workspace() == WorkspaceKind::Code
                        && workspace.active_focus_layout()
                    {
                        self.pending_actions.push("workspace.code".to_owned());
                        return;
                    }
                }
                self.pending_events.push(UiEvent::CancelInteraction);
            }
            _ => {}
        }
    }

    fn queue_activation(&mut self) {
        self.pending_events.push(UiEvent::Activate);
    }

    fn route_home_end(
        &mut self,
        text_direction: UiTextCursorDirection,
        collection: UiCollectionNavigation,
    ) {
        if self.text_input_focused() {
            self.pending_events.push(UiEvent::MoveTextCursor {
                direction: text_direction,
                extend_selection: self.modifiers.shift,
            });
        } else {
            self.pending_events
                .push(UiEvent::NavigateCollection(collection));
        }
    }

    fn text_input_focused(&self) -> bool {
        self.frame.focused.is_some_and(|focused| {
            self.ui.document().node(focused).is_some_and(|node| {
                matches!(
                    node.kind,
                    UiWidgetKind::TextInput | UiWidgetKind::SearchInput
                )
            })
        })
    }

    fn route_scroll_input(&mut self, event: NativeScrollEvent) {
        let unit = match event.unit {
            NativeScrollUnit::Pixels => UiScrollUnit::Pixels,
            NativeScrollUnit::Lines => UiScrollUnit::Lines,
        };
        let kind = if unit == UiScrollUnit::Pixels {
            UiInputDeviceKind::Trackpad
        } else {
            UiInputDeviceKind::Mouse
        };
        let phase = match event.phase {
            NativeScrollPhase::Begin => UiScrollPhase::Begin,
            NativeScrollPhase::Update => UiScrollPhase::Update,
            NativeScrollPhase::Momentum => UiScrollPhase::Momentum,
            NativeScrollPhase::End => UiScrollPhase::End,
            NativeScrollPhase::Cancel => UiScrollPhase::Cancel,
        };
        self.pending_events.push(UiEvent::Scroll(UiScrollEvent {
            device: CREATOR_POINTER_DEVICE,
            kind,
            phase,
            position: self.pointer,
            delta: UiScrollDelta {
                x: -event.x,
                y: -event.y,
                unit,
            },
        }));
        if unit == UiScrollUnit::Lines {
            self.pending_events.push(UiEvent::Scroll(UiScrollEvent {
                device: CREATOR_POINTER_DEVICE,
                kind,
                phase: UiScrollPhase::End,
                position: self.pointer,
                delta: UiScrollDelta {
                    x: 0.0,
                    y: 0.0,
                    unit,
                },
            }));
        }
    }

    fn route_accessibility_action(&mut self, request: PlatformAccessibilityActionRequest) {
        match (request.action, request.data) {
            (SemanticAction::Focus, None) => {
                self.pending_events
                    .push(UiEvent::AssistiveFocus(request.target));
            }
            (SemanticAction::Activate, None) => {
                self.pending_events
                    .push(UiEvent::AssistiveActivate(request.target));
            }
            (
                SemanticAction::SetValue | SemanticAction::ReplaceSelectedText,
                Some(PlatformAccessibilityActionData::Text(text)),
            ) => self.pending_events.push(UiEvent::AssistiveSetValue {
                target: request.target,
                text,
                replace_selection: request.action == SemanticAction::ReplaceSelectedText,
            }),
            (SemanticAction::SetValue, Some(PlatformAccessibilityActionData::Numeric(value))) => {
                self.pending_events.push(UiEvent::AssistiveSetValue {
                    target: request.target,
                    text: value.to_string(),
                    replace_selection: false,
                });
            }
            (
                SemanticAction::Expand
                | SemanticAction::Collapse
                | SemanticAction::Increment
                | SemanticAction::Decrement
                | SemanticAction::ScrollIntoView
                | SemanticAction::ShowContextMenu,
                None,
            ) => self.pending_events.push(UiEvent::AssistiveRequest {
                target: request.target,
                action: request.action,
            }),
            _ => {
                "Accessibility action was rejected because the control did not authorize it"
                    .clone_into(&mut self.hub_status);
            }
        }
    }

    fn handle_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        let result: AppResult<()> = match event {
            PlatformEvent::WindowCreated { .. } => match context.window().cloned() {
                Some(window) => self
                    .initialize_gpu(window)
                    .map(|()| context.request_redraw()),
                None => Err(
                    io::Error::other("Creator window creation omitted its native window").into(),
                ),
            },
            PlatformEvent::Resized(size) => self
                .refresh_for_size(size, f64::from(self.scale_factor))
                .map(|()| context.request_redraw()),
            PlatformEvent::ScaleFactorChanged { scale_factor, size } => self
                .refresh_for_size(size, scale_factor)
                .map(|()| context.request_redraw()),
            PlatformEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                Ok(())
            }
            PlatformEvent::PointerMoved { x, y } => {
                self.route_pointer_move(x, y);
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::TextCommit(text) => {
                self.pending_events.push(UiEvent::TextCommit(text));
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::ImePreedit { text, cursor } => {
                self.pending_events
                    .push(UiEvent::ImePreedit { text, cursor });
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::ImeCancelled => {
                self.pending_events.push(UiEvent::ImeCancel);
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::AccessibilityAction(request) => {
                self.route_accessibility_action(request);
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::AccessibilityRejected(error) => {
                self.hub_status = format!("Accessibility request rejected: {error}");
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::Input(event) => {
                self.route_input(event);
                self.reconcile_ui(context)
                    .map(|()| context.request_redraw())
            }
            PlatformEvent::RedrawRequested => self.render(context),
            PlatformEvent::CloseRequested | PlatformEvent::Exiting => {
                context.exit();
                Ok(())
            }
            PlatformEvent::Focused(false) => {
                self.modifiers = PlatformModifiers::default();
                self.pending_events.push(UiEvent::CancelInteraction);
                self.reconcile_ui(context)
            }
            PlatformEvent::Resumed => {
                if self.rhi.is_some() && creator_surface_is_renderable(self.physical_size) {
                    context.request_redraw();
                }
                Ok(())
            }
            // A Creator window can initially be occluded while macOS brings
            // the process forward. Retry only when the platform explicitly
            // reports focus instead of spinning redraws while it is hidden.
            PlatformEvent::Focused(true) => {
                if creator_surface_is_renderable(self.physical_size) {
                    context.request_redraw();
                }
                Ok(())
            }
            PlatformEvent::Suspended | PlatformEvent::MemoryWarning => Ok(()),
        };
        if let Err(error) = result {
            self.record_terminal_error(error);
            context.exit();
        }
    }
}

impl PlatformApplication for CreatorApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        self.handle_event(event, context);
    }

    fn terminal_error(&self) -> Option<PlatformError> {
        self.terminal_error.clone()
    }

    fn accessibility_tree(&self) -> Option<SemanticTree> {
        Some(self.frame.semantic_tree.clone())
    }
}

fn create_public_creator_project(parent: &Path, name: &str) -> AppResult<PathBuf> {
    let name_path = Path::new(name);
    if name.trim().is_empty()
        || name_path.components().count() != 1
        || !matches!(name_path.components().next(), Some(Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "project name must be one nonempty directory name",
        )
        .into());
    }
    let parent_metadata = fs::symlink_metadata(parent)?;
    if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "new project parent must be a real directory",
        )
        .into());
    }
    let root = parent.join(name_path);
    if root.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "new project directory already exists",
        )
        .into());
    }
    fs::create_dir(&root)?;
    let result = (|| {
        let template = workspace_root()?.join("examples/creator-alpha");
        for relative in [
            Path::new(CREATOR_ALPHA_MANIFEST),
            Path::new("assets/public-triangle.mesh.json"),
            Path::new("models/public-box.model.json"),
            Path::new("recipes/public-placement.mproc"),
        ] {
            let source = template.join(relative);
            let destination = root.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, destination)?;
        }
        let manifest: CreatorAlphaManifest =
            serde_json::from_slice(&fs::read(root.join(CREATOR_ALPHA_MANIFEST))?)?;
        validate_creator_alpha_manifest(&root, &manifest)?;
        let imported = import_creator_alpha_source(&root, &manifest.imported_asset)?;
        let mut document = ProjectDocument::new(manifest.project_id);
        document.sources.insert(imported.id, imported.clone());
        document.placements.insert(
            manifest.placement.id,
            WorldPlacement {
                id: manifest.placement.id,
                source_id: imported.id,
                label: manifest.placement.label,
                translation: manifest.placement.translation,
            },
        );
        let store = ProjectStore::new(
            root.join(CREATOR_PROJECT_SOURCE),
            root.join(CREATOR_INTERNAL_DIRECTORY)
                .join("editor-recovery.state"),
        );
        store.create(document)?;
        Ok::<(), Box<dyn Error>>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    Ok(root)
}

fn first_placement_id(session: &EditorSession) -> AppResult<StableId> {
    session
        .document()
        .placements
        .keys()
        .next()
        .copied()
        .ok_or_else(|| io::Error::other("project has no editable placement").into())
}

fn selected_or_first_placement(session: &EditorSession) -> AppResult<StableId> {
    session
        .selection()
        .ids
        .iter()
        .find(|id| session.document().placements.contains_key(id))
        .copied()
        .map_or_else(|| first_placement_id(session), Ok)
}

fn creator_transaction(
    command: EditorCommand,
    label: &str,
) -> meridian_editor_core::EditorTransaction {
    let affected_ids = match &command {
        EditorCommand::RegisterImportedSource(source)
        | EditorCommand::UpdateImportedSource(source) => {
            vec![source.id]
        }
        EditorCommand::RemoveImportedSource { source_id } => vec![*source_id],
        EditorCommand::PlaceObject(placement) => vec![placement.id, placement.source_id],
        EditorCommand::SetPlacementTranslation { placement_id, .. }
        | EditorCommand::RemovePlacement { placement_id } => vec![*placement_id],
    };
    meridian_editor_core::EditorTransaction {
        command,
        metadata: CommandMetadata::local(label, affected_ids),
    }
}

#[derive(Debug, Deserialize)]
struct CreatorAlphaManifest {
    schema: String,
    project_id: StableId,
    imported_asset: CreatorAlphaImportRequest,
    placement: CreatorAlphaPlacement,
    editable_model: String,
    procedural_recipe: String,
}

#[derive(Debug, Deserialize)]
struct CreatorAlphaImportRequest {
    label: String,
    source_path: String,
}

#[derive(Debug, Deserialize)]
struct CreatorAlphaPlacement {
    id: StableId,
    label: String,
    translation: Translation,
}

#[derive(Serialize)]
struct CreatorAlphaBuildInput<'a> {
    schema: &'static str,
    project_id: String,
    document_generation: u64,
    imported_source_count: usize,
    placement_count: usize,
    editable_model: &'a str,
    model_document_generation: u64,
    model_preview_triangle_count: usize,
    procedural_recipe: &'a str,
}

#[derive(Serialize)]
struct CreatorAlphaEvidence<'a> {
    schema: &'static str,
    milestone: &'static str,
    package: &'static str,
    outcome: &'static str,
    project_manifest: &'a str,
    source_authority: &'static str,
    journey: Vec<&'static str>,
    document_generation: u64,
    imported_source_count: usize,
    placement_count: usize,
    source_persistence: CheckStatus,
    recovery: CheckStatus,
    semantic_workspace: CheckStatus,
    procedural: CreatorAlphaProceduralEvidence,
    modeler: CreatorAlphaModelerEvidence,
    build: CreatorAlphaBuildEvidence,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct CreatorAlphaProceduralEvidence {
    recipe_id: String,
    preview_cache_key: String,
    bake_cache_key: String,
    generated_placement_count: usize,
    semantic_inspector: CheckStatus,
    license_audit: CheckStatus,
}

#[derive(Serialize)]
struct CreatorAlphaModelerEvidence {
    source_document_id: String,
    source_generation: u64,
    source_object_count: usize,
    source_vertex_count: usize,
    source_edge_count: usize,
    source_face_count: usize,
    preview_triangle_count: usize,
    topology_lineage: CheckStatus,
    override_migration: CheckStatus,
    semantic_undo_recovery: CheckStatus,
    semantic_inspector: CheckStatus,
    penumbra_preview: CheckStatus,
}

#[derive(Serialize)]
struct CreatorAlphaBuildEvidence {
    build_id: String,
    artifact_hash: String,
    artifact_bytes: u64,
    event_count: usize,
    durable_state: String,
    worker_count: usize,
}

/// Runs the public, generic Creator Alpha journey against caller-selected source
/// and evidence directories.
///
/// The path intentionally exercises editor commands, source authority, undo/redo,
/// isolated Play apply/discard, durable recovery, semantic UI construction, and
/// a real bounded Cargo build with an artifact bound to the build request.
fn run_creator_alpha_smoke(options: &MeridianOptions) -> AppResult<()> {
    let source_project_root = resolve_creator_alpha_project(
        options
            .project
            .as_deref()
            .ok_or_else(|| io::Error::other("Creator Alpha project argument was not retained"))?,
    )?;
    let evidence_root = resolve_output_path(
        &source_project_root,
        options.evidence.as_deref(),
        Path::new("target/meridian-evidence/creator-alpha"),
    );
    fs::create_dir_all(&evidence_root)?;
    let project_root = create_isolated_creator_smoke_project(&source_project_root, &evidence_root)?;
    let manifest_path = project_root.join(CREATOR_ALPHA_MANIFEST);
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: CreatorAlphaManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_creator_alpha_manifest(&project_root, &manifest)?;

    let (recovered, semantic_workspace, mut journey) =
        run_creator_alpha_persistent_journey(&project_root, &manifest)?;
    let procedural = run_creator_alpha_procedural_journey(&project_root, &manifest)?;
    journey.extend(["recipe-validate", "recipe-preview", "recipe-bake"]);
    let (modeler, modeler_journey) =
        run_creator_alpha_modeler_journey(&project_root, &manifest, &evidence_root)?;
    journey.extend(modeler_journey);

    let build_input_path = evidence_root.join("creator-alpha-build-input.json");
    write_atomic(
        &build_input_path,
        &serde_json::to_vec_pretty(&CreatorAlphaBuildInput {
            schema: "meridian.creator-alpha-build-input/v1",
            project_id: recovered.document().id.to_string(),
            document_generation: recovered.document().generation,
            imported_source_count: recovered.document().sources.len(),
            placement_count: recovered.document().placements.len(),
            editable_model: &manifest.editable_model,
            model_document_generation: modeler.source_generation,
            model_preview_triangle_count: modeler.preview_triangle_count,
            procedural_recipe: &manifest.procedural_recipe,
        })?,
    )?;
    let build = run_creator_alpha_build(&manifest_bytes, &evidence_root, &build_input_path)?;
    journey.push("build");

    let evidence = CreatorAlphaEvidence {
        schema: CREATOR_ALPHA_EVIDENCE_SCHEMA,
        milestone: "MS-03",
        package: "WP-EDT-001",
        outcome: "LocalPass",
        project_manifest: CREATOR_ALPHA_MANIFEST,
        source_authority: "imported source and editable world placement remain project source; derived previews are not source authority",
        journey,
        document_generation: recovered.document().generation,
        imported_source_count: recovered.document().sources.len(),
        placement_count: recovered.document().placements.len(),
        source_persistence: CheckStatus::Pass,
        recovery: CheckStatus::Pass,
        semantic_workspace,
        procedural,
        modeler,
        build,
        limitations: vec![
            "WP-MDL-001 remains a partial native-modeler foundation; UVs, broad topology tools, modifiers, collision/LOD, and interchange are not implemented here.",
            "The Penumbra preview descriptor is structural source-to-preview evidence, not a visible native-review or visual-quality claim.",
            "This local smoke is not cross-platform qualification or visible native-review evidence.",
        ],
    };
    write_atomic(
        &evidence_root.join("creator-alpha-evidence.json"),
        &serde_json::to_vec_pretty(&evidence)?,
    )?;
    println!(
        "Meridian Creator Alpha smoke passed: source edit, undo/redo, Play apply/discard, durable recovery, semantic workspace, and request-bound build artifact verified at {}",
        evidence_root.display()
    );
    Ok(())
}

/// Copies public source inputs into an output-owned project before the smoke mutates them.
///
/// The caller-selected project remains read-only; only the isolated copy receives
/// persistent Creator commands and recovery sidecars.
fn create_isolated_creator_smoke_project(
    source_project_root: &Path,
    evidence_root: &Path,
) -> AppResult<PathBuf> {
    let manifest: CreatorAlphaManifest = serde_json::from_slice(&read_bounded_regular_file(
        &source_project_root.join(CREATOR_ALPHA_MANIFEST),
        "Creator Alpha manifest",
    )?)?;
    validate_creator_alpha_manifest(source_project_root, &manifest)?;
    ProjectDocument::read_source(source_project_root.join(CREATOR_PROJECT_SOURCE))?;

    let sequence = NEXT_CREATOR_SMOKE_PROJECT_ID.fetch_add(1, Ordering::Relaxed);
    let root = evidence_root.join(format!(
        "creator-alpha-project-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&root)?;
    let result = (|| {
        for relative in [
            Path::new(CREATOR_ALPHA_MANIFEST),
            Path::new(CREATOR_PROJECT_SOURCE),
            validated_project_relative_path(&manifest.imported_asset.source_path)?,
            validated_project_relative_path(&manifest.editable_model)?,
            validated_project_relative_path(&manifest.procedural_recipe)?,
        ] {
            copy_regular_creator_project_file(source_project_root, &root, relative)?;
        }
        Ok::<(), Box<dyn Error>>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    Ok(root)
}

fn copy_regular_creator_project_file(
    source_project_root: &Path,
    destination_root: &Path,
    relative: &Path,
) -> AppResult<()> {
    let source = source_project_root.join(relative);
    let bytes = read_bounded_regular_file(&source, "Creator Alpha smoke source")?;
    let destination = destination_root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    write_atomic(&destination, &bytes)?;
    Ok(())
}

fn run_creator_alpha_procedural_journey(
    project_root: &Path,
    manifest: &CreatorAlphaManifest,
) -> AppResult<CreatorAlphaProceduralEvidence> {
    let recipe_path = project_root.join(validated_project_relative_path(
        &manifest.procedural_recipe,
    )?);
    let recipe = read_alluvium_recipe(&recipe_path)?;
    let inspector = recipe_inspector_document(&recipe)
        .map_err(|error| io::Error::other(format!("Alluvium inspector invalid: {error:?}")))?;
    let semantic_inspector = CheckStatus::from_bool(
        [10_002_u128, 10_003, 10_004]
            .iter()
            .all(|id| inspector.node(meridian_ui::UiNodeId::new(*id)).is_some()),
    );
    if semantic_inspector != CheckStatus::Pass {
        return Err(
            io::Error::other("Alluvium inspector semantic actions were unavailable").into(),
        );
    }
    let preview = evaluate_alluvium(&recipe, EvaluationMode::Preview)?;
    let audit = license_audit(&recipe, "creator-alpha-public")?;
    if !audit.accepted {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Creator Alpha recipe license audit failed",
        )
        .into());
    }
    let bake = evaluate_alluvium(&recipe, EvaluationMode::Bake)?;
    if preview.field.samples != bake.field.samples {
        return Err(io::Error::other("Alluvium preview and strict bake diverged").into());
    }
    Ok(CreatorAlphaProceduralEvidence {
        recipe_id: recipe.id.to_string(),
        preview_cache_key: preview.cache_key,
        bake_cache_key: bake.cache_key,
        generated_placement_count: bake.field.samples.len(),
        semantic_inspector,
        license_audit: CheckStatus::Pass,
    })
}

fn run_creator_alpha_modeler_journey(
    project_root: &Path,
    manifest: &CreatorAlphaManifest,
    evidence_root: &Path,
) -> AppResult<(CreatorAlphaModelerEvidence, Vec<&'static str>)> {
    let (mut session, source_object_id, source_edge) =
        prepare_creator_alpha_model_session(project_root, manifest)?;
    let topology = split_creator_alpha_model_edge(&mut session, source_object_id, &source_edge)?;
    session.undo()?;
    session.redo()?;
    let evidence = recover_creator_alpha_model_session(
        evidence_root,
        &session,
        source_object_id,
        source_edge.id,
        &topology,
    )?;
    Ok((
        evidence,
        vec![
            "model-open",
            "model-create-primitive",
            "model-transform",
            "model-split-edge",
            "model-undo",
            "model-redo",
            "model-recover",
            "model-inspect",
            "model-preview",
        ],
    ))
}

fn prepare_creator_alpha_model_session(
    project_root: &Path,
    manifest: &CreatorAlphaManifest,
) -> AppResult<(ModelSession, StableId, Edge)> {
    let model_path = project_root.join(validated_project_relative_path(&manifest.editable_model)?);
    let source = ModelDocument::read_source(&model_path)?;
    let source_object = source
        .objects
        .first()
        .ok_or_else(|| io::Error::other("Creator Alpha editable model has no source object"))?;
    let source_object_id = source_object.id;
    let source_edge = source_object
        .edges
        .first()
        .cloned()
        .ok_or_else(|| io::Error::other("Creator Alpha editable model has no source edge"))?;
    let mut session = ModelSession::open(source)?;

    session.apply(ModelTransaction::new(
        "Create public quad primitive",
        ModelCommand::CreatePrimitive(creator_alpha_public_quad()),
    ))?;

    session.select(ModelElementKind::Object, [source_object_id])?;
    session.apply(
        ModelTransaction::new(
            "Translate public editable box",
            ModelCommand::TranslateObject {
                object_id: source_object_id,
                translation_mm: Millimetres3 { x: 250, y: 0, z: 0 },
            },
        )
        .with_selection(session.selection().clone()),
    )?;
    Ok((session, source_object_id, source_edge))
}

fn creator_alpha_public_quad() -> PrimitiveCreate {
    creator_alpha_public_quad_with_first_id(1_301)
}

fn creator_alpha_public_quad_with_first_id(first_id: u128) -> PrimitiveCreate {
    PrimitiveCreate::Quad(QuadPrimitive {
        ids: QuadPrimitiveIds {
            object: StableId::new(first_id),
            vertices: [
                StableId::new(first_id + 1),
                StableId::new(first_id + 2),
                StableId::new(first_id + 3),
                StableId::new(first_id + 4),
            ],
            edges: [
                StableId::new(first_id + 5),
                StableId::new(first_id + 6),
                StableId::new(first_id + 7),
                StableId::new(first_id + 8),
            ],
            face: StableId::new(first_id + 9),
        },
        label: "Creator Alpha public quad".to_owned(),
        origin_mm: Millimetres3 {
            x: 2_000,
            y: 0,
            z: 0,
        },
        half_extent_mm: 250,
    })
}

fn next_model_id_range(document: &ModelDocument, count: u128) -> AppResult<u128> {
    if count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "model stable-ID allocation requires a non-zero count",
        )
        .into());
    }
    let mut maximum = document.document_id.get();
    for object in &document.objects {
        maximum = maximum.max(object.id.get());
        for vertex in &object.vertices {
            maximum = maximum.max(vertex.id.get());
        }
        for edge in &object.edges {
            maximum = maximum.max(edge.id.get());
        }
        for face in &object.faces {
            maximum = maximum.max(face.id.get());
        }
    }
    let first = maximum
        .checked_add(1)
        .ok_or_else(|| io::Error::other("model stable-ID space is exhausted"))?;
    first
        .checked_add(count - 1)
        .ok_or_else(|| io::Error::other("model stable-ID range is exhausted"))?;
    Ok(first)
}

fn split_creator_alpha_model_edge(
    session: &mut ModelSession,
    source_object_id: StableId,
    source_edge: &Edge,
) -> AppResult<TopologyMap> {
    session.select(ModelElementKind::Edge, [source_edge.id])?;
    let topology = session
        .apply(
            ModelTransaction::new(
                "Split public box edge",
                ModelCommand::SplitEdge(SplitEdge {
                    object_id: source_object_id,
                    edge_id: source_edge.id,
                    new_vertex: Vertex {
                        id: StableId::new(1_251),
                        position_mm: Millimetres3 {
                            x: 0,
                            y: -500,
                            z: -500,
                        },
                    },
                    replacement_edges: [
                        Edge {
                            id: StableId::new(1_252),
                            start: source_edge.start,
                            end: StableId::new(1_251),
                        },
                        Edge {
                            id: StableId::new(1_253),
                            start: StableId::new(1_251),
                            end: source_edge.end,
                        },
                    ],
                }),
            )
            .with_selection(session.selection().clone()),
        )?
        .topology_map
        .ok_or_else(|| io::Error::other("model split did not emit a topology map"))?;
    let generated_override = GeneratedOverride {
        target: source_edge.id,
        expected_source: None,
        action: OverrideAction::Suppress,
    };
    let reconciliation =
        topology.reconcile_generated_override(&generated_override, session.current().document());
    if reconciliation.status != OverrideStatus::Conflicted {
        return Err(
            io::Error::other("model split did not retain Alluvium override conflict").into(),
        );
    }
    Ok(topology)
}

fn recover_creator_alpha_model_session(
    evidence_root: &Path,
    session: &ModelSession,
    source_object_id: StableId,
    source_edge_id: StableId,
    topology: &TopologyMap,
) -> AppResult<CreatorAlphaModelerEvidence> {
    let recovery_store = ModelRecoveryStore::new(evidence_root.join("modeler-recovery.state"));
    recovery_store.save(session)?;
    let recovered = recovery_store.load()?;
    if recovered.current() != session.current() {
        return Err(io::Error::other("model recovery changed accepted source revision").into());
    }

    let selection = ModelSelection::new(
        recovered.current().document(),
        ModelElementKind::Object,
        [source_object_id],
    )?;
    let before_preview = recovered.current().document().clone();
    let preview = recovered
        .current()
        .document()
        .penumbra_preview(source_object_id)?;
    if !preview.is_derived() || recovered.current().document() != &before_preview {
        return Err(
            io::Error::other("derived Penumbra preview changed model source authority").into(),
        );
    }
    let inspector = model_inspector_document(recovered.current().document(), &selection, &preview)
        .map_err(|error| io::Error::other(format!("model inspector invalid: {error:?}")))?;
    let semantic_inspector = CheckStatus::from_bool(
        [20_003_u128, 20_004, 20_005, 20_006, 20_007, 20_008]
            .iter()
            .all(|id| {
                inspector
                    .node(meridian_ui::UiNodeId::new(*id))
                    .is_some_and(|node| node.focusable && node.semantics.action.is_some())
            }),
    );
    if semantic_inspector != CheckStatus::Pass {
        return Err(io::Error::other("model inspector semantic actions were unavailable").into());
    }

    let document = recovered.current().document();
    let (source_vertex_count, source_edge_count, source_face_count) =
        model_element_counts(document);
    Ok(CreatorAlphaModelerEvidence {
        source_document_id: document.document_id.to_string(),
        source_generation: document.document_generation,
        source_object_count: document.objects.len(),
        source_vertex_count,
        source_edge_count,
        source_face_count,
        preview_triangle_count: preview.triangle_indices.len() / 3,
        topology_lineage: CheckStatus::from_bool(
            topology.lineage_for(source_edge_id)
                == Some(&[StableId::new(1_252), StableId::new(1_253)][..]),
        ),
        override_migration: CheckStatus::Pass,
        semantic_undo_recovery: CheckStatus::Pass,
        semantic_inspector,
        penumbra_preview: CheckStatus::Pass,
    })
}

fn model_element_counts(document: &ModelDocument) -> (usize, usize, usize) {
    document.objects.iter().fold((0, 0, 0), |counts, object| {
        (
            counts.0 + object.vertices.len(),
            counts.1 + object.edges.len(),
            counts.2 + object.faces.len(),
        )
    })
}

/// Exercises the live Creator action adapter against an output-owned copy of
/// the public project. Every mutation takes the same typed route as a native
/// UI command, then survives authoritative source persistence and a reopen.
#[allow(clippy::too_many_lines)] // The explicit sequence is the Creator Alpha evidence contract.
fn run_creator_alpha_persistent_journey(
    project_root: &Path,
    manifest: &CreatorAlphaManifest,
) -> AppResult<(EditorSession, CheckStatus, Vec<&'static str>)> {
    let mut application = CreatorApplication::new(Some(project_root), true)?;
    let imported = import_creator_alpha_source(project_root, &manifest.imported_asset)?;
    let (source_path, placement_id, original_translation) = {
        let workspace = creator_workspace_for_smoke(&application)?;
        if workspace.recovery_status != ProjectRecoveryStatus::None {
            return Err(
                io::Error::other("isolated Creator Alpha smoke project was not fresh").into(),
            );
        }
        if workspace.session.document().sources.get(&imported.id) != Some(&imported) {
            return Err(io::Error::other(
                "Creator Alpha source document disagrees with the DAT import authority",
            )
            .into());
        }
        let placement_id = first_placement_id(&workspace.session)?;
        (
            workspace.project_store.source_path().to_path_buf(),
            placement_id,
            workspace.session.document().placements[&placement_id].translation,
        )
    };

    let edited_candidate = Translation {
        x_mm: original_translation
            .x_mm
            .checked_add(250)
            .ok_or_else(|| io::Error::other("Creator Alpha placement edit exceeded i64 range"))?,
        ..original_translation
    };
    let select_placement = application.creator_action_node("editor.select-placement")?;
    let edit_placement = application.creator_action_node("editor.edit-placement")?;
    let emitted = application.reconcile_workspace_ui_events_for_smoke(vec![
        UiEvent::AssistiveActivate(select_placement),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(edited_candidate.x_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Y_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(edited_candidate.y_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Z_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(edited_candidate.z_mm.to_string()),
        UiEvent::AssistiveActivate(edit_placement),
    ])?;
    assert_creator_ui_command(&emitted, "editor.select-placement")?;
    assert_creator_ui_command(&emitted, "editor.edit-placement")?;
    let edited_translation = creator_workspace_for_smoke(&application)?
        .session
        .document()
        .placements[&placement_id]
        .translation;
    if edited_translation == original_translation {
        return Err(io::Error::other("Creator Alpha action adapter did not edit source").into());
    }
    assert_persisted_creator_source(
        &source_path,
        &creator_workspace_for_smoke(&application)?.session,
    )?;

    // History actions live in the intentionally compact bottom shelf. Open it
    // through the same semantic control a keyboard or assistive user uses
    // before activating undo/redo; do not bypass the authored UI surface.
    let emitted = application.activate_workspace_action_for_smoke("shell.open-shelf")?;
    assert_creator_ui_command(&emitted, "shell.open-shelf")?;
    let emitted = application.activate_workspace_action_for_smoke("editor.undo")?;
    assert_creator_ui_command(&emitted, "editor.undo")?;
    if creator_workspace_for_smoke(&application)?
        .session
        .document()
        .placements[&placement_id]
        .translation
        != original_translation
    {
        return Err(io::Error::other(
            "Creator Alpha action-adapter undo did not restore persisted placement source",
        )
        .into());
    }
    assert_persisted_creator_source(
        &source_path,
        &creator_workspace_for_smoke(&application)?.session,
    )?;

    let emitted = application.activate_workspace_action_for_smoke("editor.redo")?;
    assert_creator_ui_command(&emitted, "editor.redo")?;
    if creator_workspace_for_smoke(&application)?
        .session
        .document()
        .placements[&placement_id]
        .translation
        != edited_translation
    {
        return Err(io::Error::other(
            "Creator Alpha action-adapter redo did not restore persisted placement source",
        )
        .into());
    }
    assert_persisted_creator_source(
        &source_path,
        &creator_workspace_for_smoke(&application)?.session,
    )?;

    let emitted = application.activate_workspace_action_for_smoke("editor.play-start")?;
    assert_creator_ui_command(&emitted, "editor.play-start")?;
    let play_applied_translation = Translation {
        y_mm: 250,
        ..edited_translation
    };
    let edit_placement = application.creator_action_node("editor.edit-placement")?;
    let emitted = application.reconcile_workspace_ui_events_for_smoke(vec![
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_applied_translation.x_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Y_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_applied_translation.y_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Z_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_applied_translation.z_mm.to_string()),
        UiEvent::AssistiveActivate(edit_placement),
    ])?;
    assert_creator_ui_command(&emitted, "editor.edit-placement")?;
    let emitted = application.activate_workspace_action_for_smoke("editor.play-apply")?;
    assert_creator_ui_command(&emitted, "editor.play-apply")?;
    if creator_workspace_for_smoke(&application)?
        .session
        .document()
        .placements[&placement_id]
        .translation
        .y_mm
        != 250
    {
        return Err(io::Error::other("Creator Alpha Play Apply did not persist its diff").into());
    }
    assert_persisted_creator_source(
        &source_path,
        &creator_workspace_for_smoke(&application)?.session,
    )?;

    let source_before_discard = fs::read(&source_path)?;
    let emitted = application.activate_workspace_action_for_smoke("editor.play-start")?;
    assert_creator_ui_command(&emitted, "editor.play-start")?;
    let applied_translation = creator_workspace_for_smoke(&application)?
        .session
        .document()
        .placements[&placement_id]
        .translation;
    let play_discard_translation = Translation {
        z_mm: 500,
        ..applied_translation
    };
    let edit_placement = application.creator_action_node("editor.edit-placement")?;
    let emitted = application.reconcile_workspace_ui_events_for_smoke(vec![
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_discard_translation.x_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Y_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_discard_translation.y_mm.to_string()),
        UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Z_MM),
        UiEvent::SelectAllText,
        UiEvent::TextCommit(play_discard_translation.z_mm.to_string()),
        UiEvent::AssistiveActivate(edit_placement),
    ])?;
    assert_creator_ui_command(&emitted, "editor.edit-placement")?;
    let emitted = application.activate_workspace_action_for_smoke("editor.play-discard")?;
    assert_creator_ui_command(&emitted, "editor.play-discard")?;
    if fs::read(&source_path)? != source_before_discard {
        return Err(
            io::Error::other("Creator Alpha discarded Play changed authoritative source").into(),
        );
    }

    let emitted = application.activate_workspace_action_for_smoke("asset.reimport")?;
    assert_creator_ui_command(&emitted, "asset.reimport")?;
    assert_persisted_creator_source(
        &source_path,
        &creator_workspace_for_smoke(&application)?.session,
    )?;

    let expected_document = creator_workspace_for_smoke(&application)?
        .session
        .document()
        .clone();
    let mut reopened_application = CreatorApplication::new(Some(project_root), true)?;
    let emitted = reopened_application.activate_workspace_action_for_smoke("editor.recover")?;
    assert_creator_ui_command(&emitted, "editor.recover")?;
    let reopened = creator_workspace_for_smoke_mut(&mut reopened_application)?;
    if reopened.recovery_status != ProjectRecoveryStatus::Restored {
        return Err(io::Error::other(
            "Creator Alpha reopen did not safely restore source while discarding untrusted history",
        )
        .into());
    }
    if reopened.session.document() != &expected_document {
        return Err(io::Error::other(
            "Creator Alpha reopen did not rebuild the authoritative project source",
        )
        .into());
    }
    assert_persisted_creator_source(&source_path, &reopened.session)?;
    if !matches!(reopened.session.undo(), Err(EditorError::NothingToUndo)) {
        return Err(io::Error::other(
            "Creator Alpha reopen accepted untrusted recovery undo history",
        )
        .into());
    }
    let workspace_document = creator_workspace_document(&reopened.session, &reopened.status)
        .map_err(|error| {
            io::Error::other(format!("Creator Alpha workspace UI invalid: {error:?}"))
        })?;
    let semantic_workspace =
        CheckStatus::from_bool(workspace_document.root().stable_id().get() == 1);
    if semantic_workspace != CheckStatus::Pass {
        return Err(io::Error::other("Creator Alpha workspace root changed unexpectedly").into());
    }
    Ok((
        reopened.session.clone(),
        semantic_workspace,
        vec![
            "open",
            "import",
            "edit",
            "undo",
            "redo",
            "play-apply",
            "play-stop-discard",
            "reimport",
            "reopen",
            "recover",
        ],
    ))
}

fn creator_workspace_for_smoke(application: &CreatorApplication) -> AppResult<&CreatorWorkspace> {
    match &application.screen {
        CreatorScreen::Workspace(workspace) => Ok(workspace),
        CreatorScreen::Hub | CreatorScreen::Settings { .. } => Err(io::Error::other(
            "Creator Alpha smoke expected the live application to open a workspace",
        )
        .into()),
    }
}

fn assert_creator_ui_command(commands: &[UiCommandRequest], expected: &str) -> AppResult<()> {
    if commands.iter().any(|command| command.action == expected) {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "Creator UI smoke did not emit the expected semantic action: {expected}"
    ))
    .into())
}

fn creator_workspace_for_smoke_mut(
    application: &mut CreatorApplication,
) -> AppResult<&mut CreatorWorkspace> {
    match &mut application.screen {
        CreatorScreen::Workspace(workspace) => Ok(workspace),
        CreatorScreen::Hub | CreatorScreen::Settings { .. } => Err(io::Error::other(
            "Creator Alpha smoke expected the reopened application to open a workspace",
        )
        .into()),
    }
}

fn assert_persisted_creator_source(source_path: &Path, session: &EditorSession) -> AppResult<()> {
    let expected = session.document().canonical_json()?;
    let source = fs::read(source_path)?;
    if source.as_slice() != expected.as_bytes() {
        return Err(io::Error::other(
            "Creator Alpha source is not canonical accepted session JSON",
        )
        .into());
    }
    let parsed = ProjectDocument::read_source(source_path)?;
    if parsed != *session.document() {
        return Err(
            io::Error::other("Creator Alpha source does not match accepted session").into(),
        );
    }
    Ok(())
}

fn resolve_creator_alpha_project(requested: &Path) -> AppResult<PathBuf> {
    let root = requested.canonicalize()?;
    if !root.is_dir() || !root.join(CREATOR_ALPHA_MANIFEST).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Creator Alpha project must contain creator-alpha.project.json",
        )
        .into());
    }
    Ok(root)
}

fn validate_creator_alpha_manifest(
    project_root: &Path,
    manifest: &CreatorAlphaManifest,
) -> AppResult<()> {
    if manifest.schema != CREATOR_ALPHA_SCHEMA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Creator Alpha manifest has an unsupported schema",
        )
        .into());
    }
    if manifest.imported_asset.label.trim().is_empty() || manifest.placement.label.trim().is_empty()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Creator Alpha import and placement labels must be nonempty",
        )
        .into());
    }
    for path in [
        manifest.imported_asset.source_path.as_str(),
        manifest.editable_model.as_str(),
        manifest.procedural_recipe.as_str(),
    ] {
        let path = validated_project_relative_path(path)?;
        let source_path = project_root.join(path);
        let metadata = fs::symlink_metadata(&source_path)?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Creator Alpha manifest references a non-regular public source file",
            )
            .into());
        }
    }
    Ok(())
}

fn import_creator_alpha_source(
    project_root: &Path,
    request: &CreatorAlphaImportRequest,
) -> AppResult<ImportedSource> {
    let mut database = AssetImportDatabase::default();
    let snapshot = database.import_files_transaction(
        project_root,
        &[PathBuf::from(&request.source_path)],
        &CancellationToken::new(),
    )?;
    let imported = snapshot
        .meshes
        .values()
        .next()
        .ok_or_else(|| io::Error::other("Creator Alpha import produced no source"))?;
    Ok(ImportedSource {
        id: imported.metadata.source_id.stable_id(),
        label: request.label.clone(),
        source_path: request.source_path.clone(),
        source_hash: imported.metadata.source_hash.to_string(),
    })
}

fn validated_project_relative_path(value: &str) -> AppResult<&Path> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Creator Alpha source path must be a project-relative normal path",
        )
        .into());
    }
    Ok(path)
}

fn run_creator_alpha_build(
    manifest_bytes: &[u8],
    evidence_root: &Path,
    build_input: &Path,
) -> AppResult<CreatorAlphaBuildEvidence> {
    let node_id = BuildNodeId::new("creator-alpha-editor-check")?;
    let node = BuildNode::cargo_check(node_id.clone(), "cargo-local")?;
    let graph = BuildGraph::new(vec![node.clone()], vec![node_id.clone()])?;
    let environment = CargoEnvironment::from_host();
    let identity = BuildIdentityInput {
        source_checkpoint: ArtifactHash::digest(manifest_bytes).to_string(),
        resolved_profile: "creator-alpha-smoke".to_owned(),
        cargo_metadata_and_lock: "workspace-cargo-metadata-and-lock-v1".to_owned(),
        build_graph_contract: graph.contract_hash(),
        command_arguments: vec!["-p".to_owned(), "meridian-editor-core".to_owned()],
        toolchain_version: "cargo-local".to_owned(),
        target_and_capabilities: format!(
            "{}-{}-default",
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
        environment_allowlist: environment.identity_values(),
        root_node_ids: vec![node_id.as_str().to_owned()],
    };
    let sequence = NEXT_CREATOR_BUILD_ID.fetch_add(1, Ordering::Relaxed);
    let operation_id = OperationId::new(50_000_u64.saturating_add(sequence));
    let request = BuildRequest::new_with_graph(
        &identity,
        operation_id,
        TraceId::new(60_000_u64.saturating_add(sequence)),
        node,
        &graph,
    )?;
    let state_path = evidence_root.join(format!(
        "creator-alpha-build-{}-{sequence}.state",
        std::process::id()
    ));
    let recovery = DurableBuildService::open(BuildServiceStore::new(&state_path)?)?;
    if !recovery.recovery_events.is_empty() {
        return Err(
            io::Error::other("new Creator Alpha build state unexpectedly recovered work").into(),
        );
    }
    let mut service = recovery.service;
    service.submit(request.clone())?;
    service.transition(operation_id, BuildPhase::Resolving, 10)?;
    service.transition(operation_id, BuildPhase::Ready, 25)?;
    service.transition(operation_id, BuildPhase::Running, 50)?;

    let invocation = CargoInvocation::new(
        workspace_root()?,
        CargoCommand::Check,
        identity.command_arguments.clone(),
        environment,
    )?;
    let artifact_store = ArtifactStore::new(evidence_root.join("build-artifacts"))?;
    let mut supervisor = CargoBuildSupervisor::try_new()?;
    let worker_count = supervisor.worker_count().get();
    supervisor.submit(&service, &request, invocation)?;
    let started_at = Instant::now();
    loop {
        if let Some(result) = supervisor.poll_with(&mut service, |service, operation, _messages| {
            let publication = artifact_store.publish_file_for_request(
                &request,
                "meridian.creator-alpha-build-input/v1",
                build_input,
            )?;
            Ok(vec![
                service.record_published_artifact(operation, publication)?
            ])
        }) {
            let completion = result?;
            if completion.status() != CargoRunStatus::Succeeded {
                return Err(
                    io::Error::other("Creator Alpha bounded Cargo build did not succeed").into(),
                );
            }
            let artifact_event = completion
                .events()
                .iter()
                .find(|event| event.artifact_hash.is_some())
                .ok_or_else(|| io::Error::other("Creator Alpha build did not bind an artifact"))?;
            let artifact_hash = artifact_event
                .artifact_hash
                .clone()
                .ok_or_else(|| io::Error::other("Creator Alpha artifact hash was unavailable"))?;
            let artifact_bytes = fs::metadata(build_input)?.len();
            return Ok(CreatorAlphaBuildEvidence {
                build_id: request.build_id.to_string(),
                artifact_hash,
                artifact_bytes,
                event_count: completion.events().len(),
                durable_state: state_path.display().to_string(),
                worker_count,
            });
        }
        if started_at.elapsed() > CREATOR_ALPHA_BUILD_TIMEOUT {
            let _ = supervisor.cancel(&mut service, operation_id)?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Creator Alpha bounded Cargo build exceeded its timeout",
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn workspace_root() -> AppResult<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("cannot locate Meridian workspace root"))?
        .to_path_buf();
    if !root.join("Cargo.toml").is_file() {
        return Err(io::Error::other("Meridian workspace root has no Cargo.toml").into());
    }
    Ok(root)
}

fn resolve_project_root(requested: Option<&Path>) -> AppResult<PathBuf> {
    let root = if let Some(path) = requested {
        path.to_path_buf()
    } else {
        let current = std::env::current_dir()?;
        if current.join(MESH_SOURCE).is_file() {
            current
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| io::Error::other("cannot locate Meridian repository root"))?
                .to_path_buf()
        }
    };
    let canonical = root.canonicalize()?;
    if !canonical.join(MESH_SOURCE).is_file() || !canonical.join(WORLD_SOURCE).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "project does not contain the public MS-01 source fixtures",
        )
        .into());
    }
    Ok(canonical)
}

fn resolve_output_path(base: &Path, requested: Option<&Path>, default: &Path) -> PathBuf {
    match requested {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => base.join(path),
        None => base.join(default),
    }
}

struct PreparedMs01 {
    visual: CompiledVisualFacet,
    cell: CompiledWorldCell,
    timeline: DiagnosticTimeline,
    summary: EvidenceSummary,
    operation_id: OperationId,
    trace_id: TraceId,
    runtime_epoch: RuntimeEpoch,
    started_at: Instant,
}

#[derive(Clone, Debug, Serialize)]
struct EvidenceSummary {
    schema: &'static str,
    milestone: &'static str,
    milestone_outcome: String,
    local_run_outcome: CheckStatus,
    mode: String,
    source_checkpoint: String,
    build_id: String,
    build_hash: String,
    dependency_lock_hash: String,
    hardware: String,
    operating_system: String,
    backend_and_driver: String,
    capability_profile: String,
    settings_and_resolution: String,
    warmup_and_cache_state: String,
    statistics: String,
    memory_evidence: String,
    operation_id: u64,
    trace_id: u64,
    runtime_epoch: u64,
    source_schema: &'static str,
    source_id: String,
    source_hash: String,
    visual_hash: String,
    collision_hash: String,
    cell_hash: String,
    package_hash: String,
    package_reopened: CheckStatus,
    streamed_cell: CheckStatus,
    activated_entities: usize,
    source_derived_render_instances: usize,
    runtime_frames: u32,
    final_fixed_tick: u64,
    semantic_input_observed: CheckStatus,
    save_atomic_replacement: CheckStatus,
    save_backup_recovery: CheckStatus,
    save_journal_replay: CheckStatus,
    save_partial_tail_recovery: CheckStatus,
    save_migration: CheckStatus,
    fresh_runtime_reconstruction: CheckStatus,
    presentation_outcome: String,
    capture_outcome: String,
    capture_hash: Option<String>,
    capture_dimensions: Option<[u32; 2]>,
    timing_outcomes: Vec<TimingEvidence>,
    limitations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Pass,
    Fail,
}

impl CheckStatus {
    const fn from_bool(value: bool) -> Self {
        if value {
            Self::Pass
        } else {
            Self::Fail
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TimingEvidence {
    timing_frame_id: u64,
    runtime_frame_id: Option<u64>,
    submission_id: u64,
    pass: String,
    cpu_encode_ns: u128,
    gpu_outcome: String,
}

fn prepare_ms01(project_root: &Path, evidence_root: &Path, frames: u32) -> AppResult<PreparedMs01> {
    fs::create_dir_all(evidence_root)?;
    let operation_id = OperationId::new(1);
    let trace_id = TraceId::new(1);
    let runtime_epoch = RuntimeEpoch::new(1);
    let started_at = Instant::now();
    let mut timeline = DiagnosticTimeline::new(
        NonZeroUsize::new(EVIDENCE_CAPACITY).expect("evidence capacity is non-zero"),
    );
    push_event(
        &mut timeline,
        started_at,
        runtime_epoch,
        operation_id,
        trace_id,
        "RUN-MS01-START",
        "RUN",
        DiagnosticSeverity::Info,
        "meridian",
        None,
        None,
    );
    let sources = import_ms01_sources(project_root)?;
    record_import_evidence(
        &mut timeline,
        started_at,
        runtime_epoch,
        operation_id,
        trace_id,
        &sources.mesh,
    );
    let package = write_ms01_package(evidence_root, &sources)?;
    record_package_evidence(
        &mut timeline,
        started_at,
        runtime_epoch,
        operation_id,
        trace_id,
        &package,
    );
    let (streamed_cell, activated_entities) = stream_and_activate(
        &package.path,
        package.cell_asset,
        runtime_epoch,
        operation_id,
        trace_id,
        &mut timeline,
        started_at,
    )?;
    let (visual, fresh_cell) =
        reopen_runtime_artifacts(&package.path, package.visual_asset, package.cell_asset)?;
    if fresh_cell != streamed_cell {
        return Err(io::Error::other("fresh package cell differs from streamed cell").into());
    }
    let save = exercise_save_recovery(evidence_root, fresh_cell.entities[0].stable_id)?;
    record_save_evidence(
        &mut timeline,
        started_at,
        runtime_epoch,
        operation_id,
        trace_id,
        fresh_cell.entities[0].stable_id,
    );
    let runtime = run_runtime_foundation(
        &fresh_cell,
        activated_entities,
        frames,
        &mut timeline,
        started_at,
        runtime_epoch,
        operation_id,
        trace_id,
    )?;
    let summary = make_headless_summary(
        &sources,
        &package,
        &save,
        &runtime,
        project_root,
        frames,
        activated_entities,
        operation_id,
        trace_id,
        runtime_epoch,
    );
    Ok(PreparedMs01 {
        visual,
        cell: fresh_cell,
        timeline,
        summary,
        operation_id,
        trace_id,
        runtime_epoch,
        started_at,
    })
}

struct ImportedSources {
    mesh: ImportedFixtureMesh,
    cell: CompiledWorldCell,
}

fn import_ms01_sources(project_root: &Path) -> AppResult<ImportedSources> {
    let mut imports = AssetImportDatabase::default();
    let snapshot = imports.import_files_transaction(
        project_root,
        &[PathBuf::from(MESH_SOURCE)],
        &CancellationToken::new(),
    )?;
    let source_id = SourceId::from_canonical_name("fixtures/ms01/public-triangle");
    let mesh = snapshot
        .meshes
        .get(&source_id)
        .cloned()
        .ok_or_else(|| io::Error::other("accepted import snapshot omitted fixture mesh"))?;
    let cell = compile_world_source(&fs::read(project_root.join(WORLD_SOURCE))?, &snapshot)?;
    Ok(ImportedSources { mesh, cell })
}

struct BuiltPackage {
    path: PathBuf,
    visual_asset: AssetId,
    cell_asset: AssetId,
    hash: ArtifactHash,
}

fn write_ms01_package(evidence_root: &Path, sources: &ImportedSources) -> AppResult<BuiltPackage> {
    let visual_asset = AssetId::from_name(VISUAL_ASSET_NAME);
    let collision_asset = AssetId::from_name(COLLISION_ASSET_NAME);
    let cell_asset = AssetId::from_name(CELL_ASSET_NAME);
    let path = evidence_root.join("ms01.meridian");
    let hash = PackageBuilder::new()
        .with_chunk(PackageChunk::new(
            visual_asset,
            "visual-mesh-v1",
            sources.mesh.visual.encode_compiled(),
        ))
        .with_chunk(PackageChunk::new(
            collision_asset,
            "collision-mesh-v1",
            sources.mesh.collision.encode_compiled(),
        ))
        .with_chunk(PackageChunk::new(
            cell_asset,
            "compiled-world-cell-v1",
            sources.cell.encode(),
        ))
        .write_atomic(&path, PackageLimits::default())?;
    Ok(BuiltPackage {
        path,
        visual_asset,
        cell_asset,
        hash,
    })
}

struct RuntimeQualification {
    semantic_input_observed: bool,
    render_instances: usize,
    final_fixed_tick: u64,
}

#[allow(clippy::too_many_arguments)]
fn run_runtime_foundation(
    cell: &CompiledWorldCell,
    activated_entities: usize,
    frames: u32,
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
) -> AppResult<RuntimeQualification> {
    let mut input = InputState::new(InputActionMap::default_gameplay());
    input.begin_frame();
    input.apply_native_event(NativeInputEvent::Button {
        control: ButtonControl::Key(KeyCode::W),
        down: true,
    });
    let semantic_input_observed = input.action_state(Action::Move).pressed;
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "RUN-INPUT-ACTION",
        "RUN",
        DiagnosticSeverity::Info,
        "meridian-input",
        None,
        None,
        [
            ("action".to_owned(), "move".to_owned()),
            ("pressed".to_owned(), semantic_input_observed.to_string()),
        ],
    );
    let entity = &cell.entities[0];
    let mut runtime = EngineRuntime::default();
    let _render_entity = runtime.world_mut().spawn(RenderInstanceSource::new(
        RenderInstanceId::new(u64::try_from(entity.stable_id.get()).unwrap_or(1)),
        Transform::from_translation([
            f64_to_f32(entity.position.x),
            f64_to_f32(entity.position.y),
            f64_to_f32(entity.position.z),
        ]),
        1.0,
        MeshHandle(1),
        MaterialHandle(1),
    ));
    let mut final_fixed_tick = 0;
    for _ in 0..frames {
        let report = runtime.advance(Duration::from_nanos(16_666_667));
        final_fixed_tick = report.fixed_tick_after();
        push_event(
            timeline,
            started_at,
            epoch,
            operation,
            trace,
            "RUN-FRAME",
            "RUN",
            DiagnosticSeverity::Trace,
            "meridian-rt",
            Some(report.shared_frame_id()),
            Some(report.fixed_tick_after()),
        );
    }
    let render_instances = runtime
        .render_snapshot()
        .map_or(0, |snapshot| snapshot.instances().len());
    if !semantic_input_observed || render_instances != activated_entities {
        return Err(io::Error::other("runtime correlation or extraction validation failed").into());
    }
    Ok(RuntimeQualification {
        semantic_input_observed,
        render_instances,
        final_fixed_tick,
    })
}

#[allow(clippy::too_many_arguments)]
fn record_import_evidence(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    mesh: &ImportedFixtureMesh,
) {
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-IMPORT-COMMIT",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-asset-tools",
        None,
        None,
        [
            ("source_id".to_owned(), mesh.metadata.source_id.to_string()),
            (
                "source_hash".to_owned(),
                mesh.metadata.source_hash.to_string(),
            ),
        ],
    );
}

#[allow(clippy::too_many_arguments)]
fn record_package_evidence(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    package: &BuiltPackage,
) {
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-PACKAGE-WRITTEN",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-package",
        None,
        None,
        [
            ("package_hash".to_owned(), package.hash.to_string()),
            ("cell_asset_id".to_owned(), package.cell_asset.to_string()),
        ],
    );
}

#[allow(clippy::too_many_arguments)]
fn record_save_evidence(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    stable_id: StableId,
) {
    push_event_with_recovery(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-SAVE-RECOVERED",
        "DAT",
        "meridian-save",
        RecoveryAction::RestoreBackup,
    );
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-SAVE-TRANSACTION",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-save",
        None,
        None,
        [("stable_entity_id".to_owned(), stable_id.to_string())],
    );
}

#[allow(clippy::too_many_arguments)]
fn make_headless_summary(
    sources: &ImportedSources,
    package: &BuiltPackage,
    save: &SaveEvidence,
    runtime: &RuntimeQualification,
    project_root: &Path,
    frames: u32,
    activated_entities: usize,
    operation: OperationId,
    trace: TraceId,
    epoch: RuntimeEpoch,
) -> EvidenceSummary {
    EvidenceSummary {
        schema: "meridian.ms01-evidence/v1",
        milestone: "MS-01",
        milestone_outcome: "Inconclusive".to_owned(),
        local_run_outcome: CheckStatus::Pass,
        mode: "headless-foundation".to_owned(),
        source_checkpoint: "uncommitted working tree; commit and push forbidden by task".to_owned(),
        build_id: format!("meridian-editor {} debug", env!("CARGO_PKG_VERSION")),
        build_hash: current_build_hash(),
        dependency_lock_hash: hash_file_or_unavailable(&project_root.join("Cargo.lock")),
        hardware: std::env::consts::ARCH.to_owned(),
        operating_system: std::env::consts::OS.to_owned(),
        backend_and_driver: "NotRun".to_owned(),
        capability_profile: "headless runtime/data only; GPU NotRun".to_owned(),
        settings_and_resolution: format!("{frames} fixed runtime frames; no GPU resolution"),
        warmup_and_cache_state: "not applicable to headless foundation run".to_owned(),
        statistics: "single bounded qualification smoke; not a calibrated benchmark".to_owned(),
        memory_evidence: "NotRun; no calibrated memory claim".to_owned(),
        operation_id: operation.get(),
        trace_id: trace.get(),
        runtime_epoch: epoch.get(),
        source_schema: meridian_asset_tools::FIXTURE_MESH_SCHEMA,
        source_id: sources.mesh.metadata.source_id.to_string(),
        source_hash: sources.mesh.metadata.source_hash.to_string(),
        visual_hash: sources.mesh.visual.artifact_hash.to_string(),
        collision_hash: sources.mesh.collision.artifact_hash.to_string(),
        cell_hash: sources.cell.artifact_hash.to_string(),
        package_hash: package.hash.to_string(),
        package_reopened: CheckStatus::Pass,
        streamed_cell: CheckStatus::Pass,
        activated_entities,
        source_derived_render_instances: runtime.render_instances,
        runtime_frames: frames,
        final_fixed_tick: runtime.final_fixed_tick,
        semantic_input_observed: CheckStatus::from_bool(runtime.semantic_input_observed),
        save_atomic_replacement: save.atomic_replacement,
        save_backup_recovery: save.backup_recovery,
        save_journal_replay: save.journal_replay,
        save_partial_tail_recovery: save.partial_tail_recovery,
        save_migration: save.migration,
        fresh_runtime_reconstruction: save.fresh_reconstruction,
        presentation_outcome: "NotRun".to_owned(),
        capture_outcome: "NotRun".to_owned(),
        capture_hash: None,
        capture_dimensions: None,
        timing_outcomes: Vec::new(),
        limitations: vec![
            "MS-01 foundation only; no visual-quality claim",
            "JSON, compiled cell, and package v1 formats are provisional",
            "headless evidence cannot satisfy presentation or GPU requirements",
        ],
    }
}

fn current_build_hash() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .map_or_else(
            || "Unavailable".to_owned(),
            |bytes| ArtifactHash::digest(&bytes).to_string(),
        )
}

fn hash_file_or_unavailable(path: &Path) -> String {
    fs::read(path).map_or_else(
        |_| "Unavailable".to_owned(),
        |bytes| ArtifactHash::digest(&bytes).to_string(),
    )
}

#[allow(clippy::too_many_arguments)]
fn stream_and_activate(
    package_path: &Path,
    cell_asset: AssetId,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
) -> AppResult<(CompiledWorldCell, usize)> {
    let mounted = MountedPackage::mount(package_path, PackageLimits::default())?;
    let entry = mounted
        .pack_index_entry(cell_asset)
        .ok_or_else(|| io::Error::other("package omitted world-cell index entry"))?;
    let cell_hint = meridian_world::WorldCell::new(0, 0, 0);
    let mut scheduler = StreamingScheduler::new();
    scheduler.request(CellRequest::new(cell_hint, 100), 0);
    let request = scheduler
        .pop_requests(1)
        .pop()
        .ok_or_else(|| io::Error::other("streaming scheduler did not select cell"))?;
    scheduler.transition(request.cell(), CellResidencyState::MetadataOnly)?;
    scheduler.transition(request.cell(), CellResidencyState::CpuCompressed)?;

    let loaded = load_cell_on_worker(
        mounted, entry, request, epoch, operation, trace, timeline, started_at,
    )?;
    let decoded = CompiledWorldCell::decode(&loaded.bytes)?;
    scheduler.transition(decoded.cell, CellResidencyState::CpuDecoded)?;
    let mut activation = ActivationQueue::new(1, 1024 * 1024);
    activation.enqueue(ActivationWork::new(decoded.cell, loaded.bytes.len(), 100))?;
    let selected = activation.drain_budget(1, 1024 * 1024);
    if selected.len() != 1 {
        return Err(io::Error::other("bounded activation queue did not select cell").into());
    }
    scheduler.transition(decoded.cell, CellResidencyState::GpuQueued)?;
    scheduler.transition(decoded.cell, CellResidencyState::GpuResident)?;
    let mut spatial = SpatialDatabase::new();
    let activated = spatial.activate_compiled_cell(&decoded)?;
    scheduler.transition(decoded.cell, CellResidencyState::Active)?;
    push_event(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-CELL-ACTIVATED",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-world",
        None,
        None,
    );
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-CELL-IDENTIFIED",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-world",
        None,
        None,
        [
            (
                "cell".to_owned(),
                format!("{},{},{}", decoded.cell.x, decoded.cell.y, decoded.cell.z),
            ),
            ("activated_entities".to_owned(), activated.len().to_string()),
        ],
    );
    Ok((decoded, activated.len()))
}

#[allow(clippy::too_many_arguments)]
fn load_cell_on_worker(
    mounted: MountedPackage,
    entry: PackIndexEntry,
    request: CellRequest,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
) -> AppResult<AssetLoadResult> {
    let context = TaskContext::new(TaskClass::Streaming, operation, trace, epoch);
    let mut coordinator =
        CellLoadCoordinator::new(NonZeroUsize::new(1).expect("one streaming worker is non-zero"));
    coordinator.submit_correlated(
        request.cell(),
        request.priority(),
        context,
        AssetLoadRequest::new(entry, CancellationToken::new()),
        mounted,
        UncompressedDecoder,
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let completion = loop {
        if let Some(completion) = coordinator.poll() {
            break completion;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "cell load timed out").into());
        }
        std::thread::yield_now();
    };
    if completion.context() != context {
        return Err(io::Error::other("streaming task correlation changed").into());
    }
    let task_id = completion.task_id();
    let loaded = completion.into_result()?;
    push_event_fields(
        timeline,
        started_at,
        epoch,
        operation,
        trace,
        "DAT-STREAM-TASK-COMPLETED",
        "DAT",
        DiagnosticSeverity::Info,
        "meridian-streaming",
        None,
        None,
        [
            ("task_id".to_owned(), task_id.to_string()),
            ("cell_asset_id".to_owned(), loaded.asset_id.to_string()),
        ],
    );
    Ok(loaded)
}

fn reopen_runtime_artifacts(
    package_path: &Path,
    visual_asset: AssetId,
    cell_asset: AssetId,
) -> AppResult<(CompiledVisualFacet, CompiledWorldCell)> {
    let mut mounted = MountedPackage::mount(package_path, PackageLimits::default())?;
    let visual = decode_compiled_visual(&mounted.read_chunk(visual_asset)?)?;
    let cell = CompiledWorldCell::decode(&mounted.read_chunk(cell_asset)?)?;
    let mut spatial = SpatialDatabase::new();
    let activated = spatial.activate_compiled_cell(&cell)?;
    if activated.len() != cell.entities.len()
        || cell
            .entities
            .iter()
            .any(|entity| spatial.get_stable(entity.stable_id).is_none())
    {
        return Err(io::Error::other("fresh runtime did not reconstruct stable entities").into());
    }
    Ok((visual, cell))
}

struct SaveEvidence {
    atomic_replacement: CheckStatus,
    backup_recovery: CheckStatus,
    journal_replay: CheckStatus,
    partial_tail_recovery: CheckStatus,
    migration: CheckStatus,
    fresh_reconstruction: CheckStatus,
}

fn exercise_save_recovery(evidence_root: &Path, stable_id: StableId) -> AppResult<SaveEvidence> {
    let save_path = evidence_root.join("ms01.save");
    let journal_path = evidence_root.join("ms01.journal");
    let migration_path = evidence_root.join("ms01-migration.save");
    for path in [
        save_path.clone(),
        suffixed_path(&save_path, ".bak"),
        suffixed_path(&save_path, ".tmp"),
        journal_path.clone(),
        migration_path.clone(),
        suffixed_path(&migration_path, ".bak"),
        suffixed_path(&migration_path, ".tmp"),
    ] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let first = position_transaction(stable_id, [0.0, 0.0, 0.0]);
    let second = position_transaction(stable_id, [1.0, 0.0, 0.0]);
    let first_bytes = first.encode()?;
    let second_bytes = second.encode()?;
    let store = SaveStore::new(&save_path, SaveConfig::default());
    store.save(&first_bytes)?;
    store.save(&second_bytes)?;
    let atomic_replacement = store.load()? == second_bytes && store.backup_path().is_file();

    fs::write(&save_path, b"complete-corrupt-record")?;
    let backup_recovery = store.load()? == first_bytes;
    fs::remove_file(&save_path)?;
    store.save(&second_bytes)?;

    let journal = SaveJournal::new(&journal_path);
    journal.append(&first_bytes)?;
    journal.append(&second_bytes)?;
    let journal_replay = journal.replay()?.entries().len() == 2;
    let file = OpenOptions::new().write(true).open(&journal_path)?;
    let truncated_length = file.metadata()?.len().saturating_sub(5);
    file.set_len(truncated_length)?;
    file.sync_all()?;
    let partial = journal.replay()?;
    let partial_tail_recovery = partial.truncated_tail() && partial.entries().len() == 1;
    journal.append(&second_bytes)?;
    if journal.replay()?.entries().len() != 2 {
        return Err(io::Error::other("journal repair did not restore appendability").into());
    }

    let old_store = SaveStore::new(
        &migration_path,
        SaveConfig {
            schema_version: 1,
            ..SaveConfig::default()
        },
    );
    old_store.save(&first_bytes)?;
    let new_store = SaveStore::new(
        &migration_path,
        SaveConfig {
            schema_version: 2,
            ..SaveConfig::default()
        },
    );
    let mut migrations = SaveMigrations::default();
    migrations.add(1, Ok)?;
    let migration = new_store.load_with_migrations(&migrations)? == first_bytes;

    let restored = SaveTransaction::decode(&store.load()?)?;
    let mut state = SaveState::default();
    restored.apply(&mut state)?;
    let fresh_reconstruction = state
        .component(stable_id, SAVE_COMPONENT)
        .is_some_and(|(version, value)| version == 1 && value == &json!([1.0, 0.0, 0.0]));
    if !atomic_replacement
        || !backup_recovery
        || !journal_replay
        || !partial_tail_recovery
        || !migration
        || !fresh_reconstruction
    {
        return Err(io::Error::other("save recovery qualification failed").into());
    }
    Ok(SaveEvidence {
        atomic_replacement: CheckStatus::from_bool(atomic_replacement),
        backup_recovery: CheckStatus::from_bool(backup_recovery),
        journal_replay: CheckStatus::from_bool(journal_replay),
        partial_tail_recovery: CheckStatus::from_bool(partial_tail_recovery),
        migration: CheckStatus::from_bool(migration),
        fresh_reconstruction: CheckStatus::from_bool(fresh_reconstruction),
    })
}

fn position_transaction(stable_id: StableId, position: [f64; 3]) -> SaveTransaction {
    SaveTransaction::new(vec![ComponentDelta {
        entity_id: stable_id,
        component: SAVE_COMPONENT.to_owned(),
        schema_version: 1,
        value: json!(position),
    }])
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiSmokeStage {
    Recovery,
    RuntimeOverlay,
}

struct UiNativeSmokeApplication {
    recovery_runtime: UiRuntime,
    overlay_runtime: UiRuntime,
    frame: UiFrameOutput,
    logical_viewport: UiSize,
    scale_factor: f32,
    physical_size: WindowSize,
    stage: UiSmokeStage,
    rhi: Option<Rhi>,
    renderer: Option<UiOverlayRenderer>,
    structural_fallback_submitted: bool,
    surface_attempts: u8,
}

impl UiNativeSmokeApplication {
    fn new() -> AppResult<Self> {
        let recovery_document = recovery_panel_document()
            .map_err(|error| io::Error::other(format!("recovery UI fixture invalid: {error:?}")))?;
        let overlay_document = runtime_overlay_document()
            .map_err(|error| io::Error::other(format!("runtime UI fixture invalid: {error:?}")))?;
        let mut recovery_runtime = UiRuntime::new(recovery_document);
        let mut initial_input = UiFrameInput::new(UiSize::new(960.0, 540.0));
        initial_input.high_contrast = true;
        initial_input.events.push(UiEvent::FocusNext);
        let frame = recovery_runtime.reconcile(initial_input);
        let mut application = Self {
            recovery_runtime,
            overlay_runtime: UiRuntime::new(overlay_document),
            frame,
            logical_viewport: UiSize::new(960.0, 540.0),
            scale_factor: 1.0,
            physical_size: WindowSize::new(960, 540),
            stage: UiSmokeStage::Recovery,
            rhi: None,
            renderer: None,
            structural_fallback_submitted: false,
            surface_attempts: 0,
        };
        application.refresh_display(WindowSize::new(960, 540), 1.0);
        Ok(application)
    }

    fn refresh_display(&mut self, physical_size: WindowSize, scale_factor: f64) {
        self.physical_size = physical_size;
        self.scale_factor = f64_to_f32(scale_factor).clamp(0.5, 4.0);
        self.logical_viewport = UiSize::new(
            f64_to_f32(f64::from(physical_size.width) / f64::from(self.scale_factor)),
            f64_to_f32(f64::from(physical_size.height) / f64::from(self.scale_factor)),
        );
        let mut input = UiFrameInput::new(self.logical_viewport);
        input.scale_factor = self.scale_factor;
        input.high_contrast = true;
        if self.stage == UiSmokeStage::Recovery {
            input.events.push(UiEvent::FocusNext);
        }
        self.frame = match self.stage {
            UiSmokeStage::Recovery => self.recovery_runtime.reconcile(input),
            UiSmokeStage::RuntimeOverlay => self.overlay_runtime.reconcile(input),
        };
    }

    fn build_renderer(&self, rhi: &mut Rhi) -> AppResult<UiOverlayRenderer> {
        UiOverlayRenderer::new(
            rhi,
            &self.frame.display_list,
            self.logical_viewport,
            self.scale_factor,
        )
        .map_err(Into::into)
    }

    fn initialize_gpu(&mut self, window: meridian_platform::PlatformWindow) -> AppResult<()> {
        self.refresh_display(window.size(), window.scale_factor());
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let renderer = self.build_renderer(&mut rhi)?;
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn rebuild_for_size(&mut self, size: WindowSize, scale_factor: f64) -> AppResult<()> {
        self.refresh_display(size, scale_factor);
        let Some(mut rhi) = self.rhi.take() else {
            return Ok(());
        };
        rhi.resize(size);
        let renderer = self.build_renderer(&mut rhi)?;
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn render(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        let outcome = match (self.rhi.as_mut(), self.renderer.as_ref()) {
            (Some(rhi), Some(renderer)) => renderer.render_frame(rhi, ClearColor::default())?,
            _ => return Err(io::Error::other("UI smoke has no initialized renderer").into()),
        };
        if outcome.visible() {
            let report = self
                .renderer
                .as_ref()
                .map(UiOverlayRenderer::report)
                .ok_or_else(|| io::Error::other("UI smoke renderer disappeared"))?;
            println!(
                "Meridian UI native smoke {:?} submitted {} solid primitives and {} rasterized glyphs; {} text primitive(s) were incomplete",
                self.stage,
                report.solid_primitives,
                report.rasterized_glyphs,
                report.incomplete_text_primitives
            );
            self.advance_stage(context)?;
        } else {
            self.surface_attempts = self.surface_attempts.saturating_add(1);
            self.submit_structural_fallback()?;
            if self.surface_attempts >= UI_SMOKE_MAX_PRESENT_ATTEMPTS {
                println!(
                    "Meridian UI native smoke {:?} did not present after {} attempts; retaining structural-only evidence",
                    self.stage, self.surface_attempts
                );
                self.advance_stage(context)?;
            } else {
                context.request_redraw();
            }
        }
        Ok(())
    }

    fn submit_structural_fallback(&mut self) -> AppResult<()> {
        if self.structural_fallback_submitted {
            return Ok(());
        }
        let (Some(rhi), Some(renderer)) = (self.rhi.as_mut(), self.renderer.as_ref()) else {
            return Err(io::Error::other("UI structural fallback has no renderer").into());
        };
        renderer.submit_structural_validation(rhi, ClearColor::default())?;
        self.structural_fallback_submitted = true;
        println!(
            "Meridian UI native smoke {:?} surface unavailable; raster bridge submitted offscreen structural validation only",
            self.stage
        );
        Ok(())
    }

    fn advance_stage(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        if self.stage == UiSmokeStage::RuntimeOverlay {
            context.exit();
            return Ok(());
        }
        self.stage = UiSmokeStage::RuntimeOverlay;
        self.structural_fallback_submitted = false;
        self.surface_attempts = 0;
        self.rebuild_for_size(self.physical_size, f64::from(self.scale_factor))?;
        context.request_redraw();
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)] // PlatformApplication transfers event ownership.
    fn handle_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        let result: AppResult<()> = match event {
            PlatformEvent::WindowCreated { .. } => match context.window().cloned() {
                Some(window) => self
                    .initialize_gpu(window)
                    .map(|()| context.request_redraw()),
                None => Err(io::Error::other("UI smoke window creation omitted its window").into()),
            },
            PlatformEvent::Resized(size) => self
                .rebuild_for_size(size, f64::from(self.scale_factor))
                .map(|()| context.request_redraw()),
            PlatformEvent::ScaleFactorChanged { scale_factor, size } => self
                .rebuild_for_size(size, scale_factor)
                .map(|()| context.request_redraw()),
            PlatformEvent::RedrawRequested => self.render(context),
            PlatformEvent::CloseRequested | PlatformEvent::Exiting => {
                context.exit();
                Ok(())
            }
            PlatformEvent::Resumed
            | PlatformEvent::Suspended
            | PlatformEvent::Focused(_)
            | PlatformEvent::ModifiersChanged(_)
            | PlatformEvent::Input(_)
            | PlatformEvent::PointerMoved { .. }
            | PlatformEvent::TextCommit(_)
            | PlatformEvent::ImePreedit { .. }
            | PlatformEvent::ImeCancelled
            | PlatformEvent::AccessibilityAction(_)
            | PlatformEvent::AccessibilityRejected(_)
            | PlatformEvent::MemoryWarning => Ok(()),
        };
        if let Err(error) = result {
            eprintln!("Meridian UI native smoke failed: {error}");
            context.exit();
        }
    }
}

impl PlatformApplication for UiNativeSmokeApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        self.handle_event(event, context);
    }

    fn accessibility_tree(&self) -> Option<SemanticTree> {
        Some(self.frame.semantic_tree.clone())
    }
}

struct MeridianApplication {
    prepared: PreparedMs01,
    evidence_root: PathBuf,
    capture_path: PathBuf,
    mode: RunMode,
    lifecycle: RuntimeLifecycle,
    runtime: EngineRuntime,
    frame_id: FrameId,
    rhi: Option<Rhi>,
    renderer: Option<PenumbraFoundationRenderer>,
    timing_frame_id: Option<TimingFrameId>,
    timings: Vec<PassTimingSample>,
    capture: Option<CaptureOutcome>,
    offscreen_fallback_attempted: bool,
    evidence_written: bool,
    deadline: Option<Instant>,
}

impl MeridianApplication {
    fn new(
        prepared: PreparedMs01,
        evidence_root: PathBuf,
        capture_path: PathBuf,
        mode: RunMode,
    ) -> AppResult<Self> {
        let entity = prepared
            .cell
            .entities
            .first()
            .ok_or_else(|| io::Error::other("compiled cell has no render entity"))?;
        let mut runtime = EngineRuntime::default();
        let _render_entity = runtime.world_mut().spawn(RenderInstanceSource::new(
            RenderInstanceId::new(u64::try_from(entity.stable_id.get()).unwrap_or(1)),
            Transform::from_translation([
                f64_to_f32(entity.position.x),
                f64_to_f32(entity.position.y),
                f64_to_f32(entity.position.z),
            ]),
            1.0,
            MeshHandle(1),
            MaterialHandle(1),
        ));
        let report = runtime.advance(Duration::from_nanos(16_666_667));
        Ok(Self {
            frame_id: report.shared_frame_id(),
            prepared,
            evidence_root,
            capture_path,
            mode,
            lifecycle: RuntimeLifecycle::new(),
            runtime,
            rhi: None,
            renderer: None,
            timing_frame_id: None,
            timings: Vec::new(),
            capture: None,
            offscreen_fallback_attempted: false,
            evidence_written: false,
            deadline: None,
        })
    }

    fn initialize_gpu(
        &mut self,
        window: meridian_platform::PlatformWindow,
    ) -> AppResult<FrameOutcome> {
        let mut rhi = Rhi::new(window, RhiConfig::default())?;
        let qualification = !self.evidence_written;
        let timing_frame_id = qualification
            .then(|| rhi.begin_timing_frame_for(self.frame_id))
            .transpose()?;
        let renderer = PenumbraFoundationRenderer::new(
            &mut rhi,
            FoundationMeshDescriptor {
                label: "MS-01 source-derived visual facet",
                vertex_data: &self.prepared.visual.vertex_data,
                indices: &self.prepared.visual.indices,
                bounds_radius: 1.0,
            },
        )?;
        if qualification {
            rhi.request_capture(CaptureRequest::new(
                self.frame_id,
                4096,
                4096,
                64 * 1024 * 1024,
            ))?;
        }
        let outcome = renderer.render_frame(&mut rhi, ClearColor::default())?;
        if qualification && !outcome.visible() {
            renderer.submit_offscreen_capture(
                &mut rhi,
                ClearColor::default(),
                WindowSize::new(256, 256),
            )?;
            self.offscreen_fallback_attempted = true;
        }
        if let Some(timing_frame_id) = timing_frame_id {
            rhi.end_timing_frame(timing_frame_id)?;
            self.timing_frame_id = Some(timing_frame_id);
        }
        self.rhi = Some(rhi);
        self.renderer = Some(renderer);
        self.deadline = Some(Instant::now() + Duration::from_secs(10));
        Ok(outcome)
    }

    fn rebuild_device(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        let window = context
            .window()
            .cloned()
            .ok_or_else(|| io::Error::other("device rebuild has no platform window"))?;
        self.rhi = None;
        self.renderer = None;
        let outcome = self.initialize_gpu(window)?;
        observe_frame_outcome(&mut self.lifecycle, outcome);
        push_event_with_recovery(
            &mut self.prepared.timeline,
            self.prepared.started_at,
            self.prepared.runtime_epoch,
            self.prepared.operation_id,
            self.prepared.trace_id,
            "PEN-DEVICE-REBUILT",
            "PEN",
            "meridian",
            RecoveryAction::RebuildDevice,
        );
        context.request_redraw();
        Ok(())
    }

    fn handle_gpu_error(&mut self, error: &dyn Error, context: &mut PlatformContext<'_>) {
        push_event_with_recovery(
            &mut self.prepared.timeline,
            self.prepared.started_at,
            self.prepared.runtime_epoch,
            self.prepared.operation_id,
            self.prepared.trace_id,
            "PEN-GPU-FAILURE",
            "PEN",
            "meridian-rhi",
            RecoveryAction::RebuildDevice,
        );
        eprintln!("Meridian GPU failure: {error}");
        context.exit();
    }

    fn collect_async(&mut self) {
        let Some(rhi) = self.rhi.as_mut() else {
            return;
        };
        while let Some(sample) = rhi.take_pass_timing() {
            self.timings.push(sample);
        }
        if self.capture.is_none() {
            self.capture = rhi.take_capture();
        }
    }

    fn try_offscreen_capture_fallback(&mut self) -> AppResult<bool> {
        let unsupported_surface_copy = matches!(
            self.capture,
            Some(CaptureOutcome::UnsupportedCapability {
                failure: CaptureFailure::SurfaceCopyUnsupported,
                ..
            })
        );
        if !unsupported_surface_copy || self.offscreen_fallback_attempted {
            return Ok(false);
        }
        self.capture = None;
        let rhi = self
            .rhi
            .as_mut()
            .ok_or_else(|| io::Error::other("offscreen fallback has no RHI"))?;
        rhi.request_capture(CaptureRequest::new(
            self.frame_id,
            4096,
            4096,
            64 * 1024 * 1024,
        ))?;
        self.renderer
            .as_ref()
            .ok_or_else(|| io::Error::other("offscreen fallback has no renderer"))?
            .submit_offscreen_capture(rhi, ClearColor::default(), WindowSize::new(256, 256))?;
        self.offscreen_fallback_attempted = true;
        Ok(true)
    }

    fn finish_evidence_if_ready(&mut self, context: &mut PlatformContext<'_>) -> AppResult<bool> {
        self.collect_async();
        if self.try_offscreen_capture_fallback()? {
            context.request_redraw();
            return Ok(false);
        }
        let Some(expected_timing_frame) = self.timing_frame_id else {
            return Ok(false);
        };
        let Some(captured) = self.qualified_capture(expected_timing_frame)? else {
            return Ok(false);
        };
        self.record_native_evidence(&captured)?;
        if self.mode == RunMode::Smoke {
            context.exit();
        }
        Ok(true)
    }

    fn qualified_capture(
        &self,
        expected_timing_frame: TimingFrameId,
    ) -> AppResult<Option<CapturedFrame>> {
        let has_shadow = self.timings.iter().any(|sample| {
            sample.frame_id == expected_timing_frame
                && sample.runtime_frame_id == Some(self.frame_id)
                && sample.pass.as_str() == "shadow_depth"
        });
        let has_main = self.timings.iter().any(|sample| {
            sample.frame_id == expected_timing_frame
                && sample.runtime_frame_id == Some(self.frame_id)
                && sample.pass.as_str() == "indexed_mesh"
        });
        if !has_shadow || !has_main || self.capture.is_none() {
            return Ok(None);
        }
        if self.timings.iter().any(|sample| {
            sample.gpu == GpuTimingOutcome::Measured(Duration::ZERO)
                || sample.frame_id != expected_timing_frame
                || sample.runtime_frame_id != Some(self.frame_id)
        }) {
            return Err(io::Error::other("invalid or uncorrelated GPU timing evidence").into());
        }
        let captured = match self.capture.as_ref().expect("capture checked") {
            CaptureOutcome::Captured(frame) => frame.clone(),
            outcome => {
                return Err(io::Error::other(format!(
                    "native capture did not produce pixels: {outcome:?}"
                ))
                .into());
            }
        };
        if !has_multiple_pixel_values(&captured) {
            return Err(io::Error::other("captured frame contains only one pixel value").into());
        }
        Ok(Some(captured))
    }

    fn record_native_evidence(&mut self, captured: &CapturedFrame) -> AppResult<()> {
        let artifact = write_capture_png(&self.capture_path, captured)?;
        let capabilities = self
            .rhi
            .as_ref()
            .ok_or_else(|| io::Error::other("native evidence requires an active RHI"))?
            .capabilities()
            .clone();
        "native-smoke".clone_into(&mut self.prepared.summary.mode);
        record_native_environment(&mut self.prepared.summary, captured, &capabilities);
        self.prepared.summary.presentation_outcome = captured
            .surface_outcome
            .map_or_else(|| "OccludedOrUnavailable".to_owned(), frame_outcome_name);
        self.prepared.summary.capture_outcome = match captured.source {
            CaptureSource::PresentedSurface => "PresentedSurface".to_owned(),
            CaptureSource::Offscreen => "OffscreenVisible".to_owned(),
        };
        self.prepared.summary.capture_hash = Some(artifact.metadata.pixel_hash.clone());
        self.prepared.summary.capture_dimensions = Some([captured.width, captured.height]);
        self.prepared.summary.timing_outcomes =
            self.timings.iter().map(timing_evidence).collect::<Vec<_>>();
        self.prepared.summary.runtime_epoch = self.prepared.runtime_epoch.get();
        for sample in &self.timings {
            push_event_fields(
                &mut self.prepared.timeline,
                self.prepared.started_at,
                self.prepared.runtime_epoch,
                self.prepared.operation_id,
                self.prepared.trace_id,
                "PEN-PASS-TIMING",
                "PEN",
                DiagnosticSeverity::Info,
                "meridian-rhi",
                sample.runtime_frame_id,
                Some(self.runtime.world().fixed_tick()),
                [
                    (
                        "timing_frame_id".to_owned(),
                        sample.frame_id.get().to_string(),
                    ),
                    ("submission_id".to_owned(), sample.submission_id.to_string()),
                    ("pass".to_owned(), sample.pass.as_str().to_owned()),
                    ("gpu_outcome".to_owned(), format!("{:?}", sample.gpu)),
                ],
            );
        }
        push_event_fields(
            &mut self.prepared.timeline,
            self.prepared.started_at,
            self.prepared.runtime_epoch,
            self.prepared.operation_id,
            self.prepared.trace_id,
            "PEN-CAPTURE-WRITTEN",
            "PEN",
            DiagnosticSeverity::Info,
            "meridian-benchmark",
            Some(captured.frame_id),
            Some(self.runtime.world().fixed_tick()),
            [
                (
                    "capture_id".to_owned(),
                    captured.capture_id.get().to_string(),
                ),
                (
                    "pixel_hash".to_owned(),
                    artifact.metadata.pixel_hash.clone(),
                ),
                ("source".to_owned(), format!("{:?}", captured.source)),
            ],
        );
        write_evidence_bundle(
            &self.evidence_root,
            &self.prepared.timeline,
            &self.prepared.summary,
        )?;
        self.evidence_written = true;
        println!(
            "Meridian MS-01 native smoke passed: {:?} capture {}x{}, shadow/main timing correlated, package/stream/save reconstruction verified",
            captured.source, captured.width, captured.height
        );
        Ok(())
    }

    fn render_interactive_frame(&mut self, context: &mut PlatformContext<'_>) {
        let report = self.runtime.advance(Duration::from_nanos(16_666_667));
        self.frame_id = report.shared_frame_id();
        let result = match (self.rhi.as_mut(), self.renderer.as_ref()) {
            (Some(rhi), Some(renderer)) => renderer.render_frame(rhi, ClearColor::default()),
            _ => return,
        };
        match result {
            Ok(outcome) => {
                observe_frame_outcome(&mut self.lifecycle, outcome);
                context.request_redraw();
            }
            Err(error) if error.kind() == RhiErrorKind::DeviceLost => {
                self.lifecycle.observe_surface(SurfaceSignal::DeviceLost);
                if let Err(rebuild_error) = self.rebuild_device(context) {
                    self.handle_gpu_error(rebuild_error.as_ref(), context);
                }
            }
            Err(error) => self.handle_gpu_error(&error, context),
        }
    }
}

fn record_native_environment(
    summary: &mut EvidenceSummary,
    captured: &CapturedFrame,
    capabilities: &GpuCapabilities,
) {
    summary.hardware = format!(
        "{}; GPU {} (vendor {:#06x}, device {:#06x})",
        std::env::consts::ARCH,
        capabilities.adapter_name,
        capabilities.vendor_id,
        capabilities.device_id
    );
    let driver = if capabilities.driver.is_empty() {
        "Unavailable"
    } else {
        capabilities.driver.as_str()
    };
    let driver_info = if capabilities.driver_info.is_empty() {
        "Unavailable"
    } else {
        capabilities.driver_info.as_str()
    };
    summary.backend_and_driver = format!(
        "{:?}; driver {}; {}",
        capabilities.backend, driver, driver_info
    );
    summary.capability_profile = format!(
        "{:?}/{:?}; timestamps={:?}; sampled_textures_per_stage={}; hdr={:?}; features={:?}",
        capabilities.adapter_kind,
        capabilities.memory_class,
        capabilities.timestamp_queries,
        capabilities.max_sampled_textures_per_shader_stage,
        capabilities.hdr_surface_formats,
        capabilities.features
    );
    summary.settings_and_resolution = format!(
        "{} fixed runtime frames; {}x{} RGBA8-sRGB {:?} capture",
        summary.runtime_frames, captured.width, captured.height, captured.source
    );
    "foundation pipelines constructed before bounded capture; cache state uncalibrated"
        .clone_into(&mut summary.warmup_and_cache_state);
}

impl PlatformApplication for MeridianApplication {
    fn on_event(&mut self, event: PlatformEvent, context: &mut PlatformContext<'_>) {
        self.handle_event(
            PlatformEventEnvelope {
                metadata: meridian_platform::PlatformEventMetadata {
                    sequence: 0,
                    monotonic_ns: MonotonicNs::default(),
                    runtime_epoch: self.lifecycle.epoch(),
                },
                event,
            },
            context,
        );
    }

    fn on_event_envelope(
        &mut self,
        envelope: PlatformEventEnvelope,
        context: &mut PlatformContext<'_>,
    ) {
        self.handle_event(envelope, context);
    }
}

impl MeridianApplication {
    #[allow(clippy::needless_pass_by_value)] // Event-envelope ownership is defined by the app callback.
    fn handle_event(&mut self, envelope: PlatformEventEnvelope, context: &mut PlatformContext<'_>) {
        let transition = self.lifecycle.observe_platform(&envelope.event);
        self.prepared.runtime_epoch = transition.epoch;
        let mut platform_event = DiagnosticEvent::new(
            "RUN-PLATFORM-EVENT",
            "RUN",
            DiagnosticSeverity::Trace,
            "meridian-platform",
        )
        .correlated(self.prepared.operation_id, self.prepared.trace_id)
        .with_field("event", platform_event_name(&envelope.event));
        platform_event.monotonic_ns = envelope.metadata.monotonic_ns;
        platform_event.runtime_epoch = transition.epoch;
        self.prepared.timeline.push(platform_event);

        match envelope.event {
            PlatformEvent::WindowCreated { .. } => {
                let Some(window) = context.window().cloned() else {
                    self.handle_gpu_error(
                        &io::Error::other("window-created event omitted window"),
                        context,
                    );
                    return;
                };
                match self.initialize_gpu(window) {
                    Ok(outcome) => {
                        self.prepared.summary.presentation_outcome = frame_outcome_name(outcome);
                        observe_frame_outcome(&mut self.lifecycle, outcome);
                        context.request_redraw();
                    }
                    Err(error) => self.handle_gpu_error(error.as_ref(), context),
                }
            }
            PlatformEvent::Resized(size) | PlatformEvent::ScaleFactorChanged { size, .. } => {
                if let Some(rhi) = &mut self.rhi {
                    rhi.resize(size);
                }
            }
            PlatformEvent::Input(event) => {
                let mut input = InputState::new(InputActionMap::default_gameplay());
                input.begin_frame();
                input.apply_native_event(event);
            }
            PlatformEvent::RedrawRequested => {
                if !self.evidence_written {
                    match self.finish_evidence_if_ready(context) {
                        Ok(true) => {}
                        Ok(false) => {
                            if self
                                .deadline
                                .is_some_and(|deadline| Instant::now() >= deadline)
                            {
                                self.handle_gpu_error(
                                    &io::Error::new(
                                        io::ErrorKind::TimedOut,
                                        "native async evidence timed out",
                                    ),
                                    context,
                                );
                            } else {
                                context.request_redraw();
                            }
                        }
                        Err(error) => self.handle_gpu_error(error.as_ref(), context),
                    }
                } else if self.mode == RunMode::Interactive {
                    self.render_interactive_frame(context);
                }
            }
            PlatformEvent::CloseRequested => context.exit(),
            _ => {}
        }
    }
}

fn observe_frame_outcome(lifecycle: &mut RuntimeLifecycle, outcome: FrameOutcome) {
    let signal = match outcome {
        FrameOutcome::Presented | FrameOutcome::PresentedSuboptimal => SurfaceSignal::Presented,
        FrameOutcome::SkippedTimeout => SurfaceSignal::Timeout,
        FrameOutcome::SkippedOccluded | FrameOutcome::SkippedZeroSize => SurfaceSignal::Occluded,
        FrameOutcome::ReconfiguredOutdated => SurfaceSignal::Outdated,
        FrameOutcome::RecreatedLostSurface | FrameOutcome::UnsupportedSurface => {
            SurfaceSignal::Lost
        }
        FrameOutcome::DeviceLost => SurfaceSignal::DeviceLost,
    };
    lifecycle.observe_surface(signal);
}

fn timing_evidence(sample: &PassTimingSample) -> TimingEvidence {
    TimingEvidence {
        timing_frame_id: sample.frame_id.get(),
        runtime_frame_id: sample.runtime_frame_id.map(FrameId::get),
        submission_id: sample.submission_id,
        pass: sample.pass.as_str().to_owned(),
        cpu_encode_ns: sample.cpu_encode_time.as_nanos(),
        gpu_outcome: format!("{:?}", sample.gpu),
    }
}

fn frame_outcome_name(outcome: FrameOutcome) -> String {
    format!("{outcome:?}")
}

fn platform_event_name(event: &PlatformEvent) -> &'static str {
    match event {
        PlatformEvent::Resumed => "resumed",
        PlatformEvent::Suspended => "suspended",
        PlatformEvent::WindowCreated { .. } => "window-created",
        PlatformEvent::Resized(_) => "resized",
        PlatformEvent::ScaleFactorChanged { .. } => "scale-factor-changed",
        PlatformEvent::Focused(true) => "focused",
        PlatformEvent::Focused(false) => "unfocused",
        PlatformEvent::ModifiersChanged(_) => "modifiers-changed",
        PlatformEvent::Input(_) => "input",
        PlatformEvent::PointerMoved { .. } => "pointer-moved",
        PlatformEvent::TextCommit(_) => "text-commit",
        PlatformEvent::ImePreedit { .. } => "ime-preedit",
        PlatformEvent::ImeCancelled => "ime-cancelled",
        PlatformEvent::AccessibilityAction(_) => "accessibility-action",
        PlatformEvent::AccessibilityRejected(_) => "accessibility-rejected",
        PlatformEvent::RedrawRequested => "redraw-requested",
        PlatformEvent::CloseRequested => "close-requested",
        PlatformEvent::MemoryWarning => "memory-warning",
        PlatformEvent::Exiting => "exiting",
    }
}

#[allow(clippy::too_many_arguments)]
fn push_event(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    code: &str,
    domain: &str,
    severity: DiagnosticSeverity,
    source: &str,
    frame: Option<FrameId>,
    tick: Option<u64>,
) {
    let mut event =
        DiagnosticEvent::new(code, domain, severity, source).correlated(operation, trace);
    event.runtime_epoch = epoch;
    event.monotonic_ns =
        MonotonicNs::new(u64::try_from(started_at.elapsed().as_nanos()).unwrap_or(u64::MAX));
    if let Some(frame) = frame {
        event = event.at_frame(frame, tick);
    }
    timeline.push(event);
}

#[allow(clippy::too_many_arguments)]
fn push_event_fields<const N: usize>(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    code: &str,
    domain: &str,
    severity: DiagnosticSeverity,
    source: &str,
    frame: Option<FrameId>,
    tick: Option<u64>,
    fields: [(String, String); N],
) {
    let mut event =
        DiagnosticEvent::new(code, domain, severity, source).correlated(operation, trace);
    event.runtime_epoch = epoch;
    event.monotonic_ns =
        MonotonicNs::new(u64::try_from(started_at.elapsed().as_nanos()).unwrap_or(u64::MAX));
    if let Some(frame) = frame {
        event = event.at_frame(frame, tick);
    }
    for (name, value) in fields {
        event = event.with_field(name, value);
    }
    timeline.push(event);
}

#[allow(clippy::too_many_arguments)]
fn push_event_with_recovery(
    timeline: &mut DiagnosticTimeline,
    started_at: Instant,
    epoch: RuntimeEpoch,
    operation: OperationId,
    trace: TraceId,
    code: &str,
    domain: &str,
    source: &str,
    recovery: RecoveryAction,
) {
    let mut event = DiagnosticEvent::new(code, domain, DiagnosticSeverity::Warning, source)
        .correlated(operation, trace);
    event.runtime_epoch = epoch;
    event.monotonic_ns =
        MonotonicNs::new(u64::try_from(started_at.elapsed().as_nanos()).unwrap_or(u64::MAX));
    event.recovery_action = recovery;
    event.redaction_class = RedactionClass::Public;
    timeline.push(event);
}

fn write_evidence_bundle(
    evidence_root: &Path,
    timeline: &DiagnosticTimeline,
    summary: &EvidenceSummary,
) -> AppResult<()> {
    fs::create_dir_all(evidence_root)?;
    let timeline_json = timeline.to_json_pretty()?;
    let summary_json = serde_json::to_vec_pretty(summary)?;
    write_atomic(
        &evidence_root.join("timeline.json"),
        timeline_json.as_bytes(),
    )?;
    write_atomic(&evidence_root.join("summary.json"), &summary_json)?;
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let maximum = SaveConfig::default().max_payload_bytes;
    if bytes.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "atomic payload is {} bytes; maximum is {maximum}",
                bytes.len()
            ),
        )
        .into());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| io::Error::other("atomic destination has no parent directory"))?;
    let parent_metadata = fs::symlink_metadata(parent)?;
    if !parent_metadata.file_type().is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic destination parent must be a real directory",
        )
        .into());
    }
    if path.exists() {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "atomic destination must be a regular non-symlink file",
            )
            .into());
        }
    }
    let temporary = atomic_temporary_path(path);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn read_bounded_regular_file(path: &Path, label: &str) -> AppResult<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be a regular non-symlink file"),
        )
        .into());
    }
    let size = usize::try_from(metadata.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} length is not supported"),
        )
    })?;
    let maximum = SaveConfig::default().max_payload_bytes;
    if size > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} is {size} bytes; maximum is {maximum}"),
        )
        .into());
    }
    Ok(fs::read(path)?)
}

fn atomic_temporary_path(path: &Path) -> PathBuf {
    let sequence = NEXT_ATOMIC_WRITE_ID.fetch_add(1, Ordering::Relaxed);
    let mut temporary = OsString::from(path.as_os_str());
    temporary.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    PathBuf::from(temporary)
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use meridian_ui_editor::creator_alpha_panels;

    use super::*;
    use serde_json::Value;
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeCreatorProjectPicker {
        selection: Option<PathBuf>,
        invocations: Cell<usize>,
    }

    impl CreatorProjectPicker for FakeCreatorProjectPicker {
        fn pick_directory(
            &self,
            _window: Option<&meridian_platform::PlatformWindow>,
        ) -> Option<PathBuf> {
            self.invocations.set(self.invocations.get() + 1);
            self.selection.clone()
        }
    }

    #[test]
    fn argument_parser_accepts_bounded_smoke_configuration() {
        let options = MeridianOptions::parse([
            "--headless-smoke",
            "--frames",
            "4",
            "--evidence",
            "target/evidence",
        ])
        .expect("arguments parse");
        assert_eq!(options.mode, RunMode::HeadlessSmoke);
        assert_eq!(options.frames, 4);
        assert_eq!(options.evidence, Some(PathBuf::from("target/evidence")));
        assert!(matches!(
            MeridianOptions::parse(["--frames", "0"]),
            Err(MeridianArgumentError::FrameCountOutOfRange(0))
        ));
        assert!(matches!(
            MeridianOptions::parse([
                "alluvium",
                "preview",
                "examples/creator-alpha/recipes/public-placement.mproc",
                "--region",
                "0,0,0:6000,0,0"
            ]),
            Ok(MeridianOptions {
                mode: RunMode::AlluviumCommand,
                alluvium: Some(AlluviumCommand::Preview { .. }),
                ..
            })
        ));
        assert!(matches!(
            MeridianOptions::parse(["alluvium", "bake", "recipe.mproc"]),
            Err(MeridianArgumentError::AlluviumSyntax(_))
        ));
        assert!(matches!(
            MeridianOptions::parse(["--ui-headless-smoke"]),
            Ok(MeridianOptions {
                mode: RunMode::UiHeadlessSmoke,
                ..
            })
        ));
        assert!(matches!(
            MeridianOptions::parse(["--ui-smoke"]),
            Ok(MeridianOptions {
                mode: RunMode::UiSmoke,
                ..
            })
        ));
        assert!(matches!(
            MeridianOptions::parse([
                "--creator-alpha-smoke",
                "--project",
                "examples/creator-alpha",
                "--evidence",
                "target/evidence"
            ]),
            Ok(MeridianOptions {
                mode: RunMode::CreatorAlphaSmoke,
                ..
            })
        ));
        assert!(matches!(
            MeridianOptions::parse([
                "--creator-alpha-ui-smoke",
                "--project",
                "examples/creator-alpha"
            ]),
            Ok(MeridianOptions {
                mode: RunMode::CreatorAlphaUiSmoke,
                ..
            })
        ));
        assert!(matches!(
            MeridianOptions::parse(["--creator-alpha-smoke", "--evidence", "target/evidence"]),
            Err(MeridianArgumentError::CreatorAlphaProjectRequired)
        ));
        assert!(matches!(
            MeridianOptions::parse(["--creator-alpha-ui-smoke"]),
            Err(MeridianArgumentError::CreatorAlphaProjectRequired)
        ));
        assert!(matches!(
            MeridianOptions::parse([
                "--creator-alpha-smoke",
                "--project",
                "examples/creator-alpha"
            ]),
            Err(MeridianArgumentError::CreatorAlphaEvidenceRequired)
        ));
    }

    #[test]
    fn creator_ui_review_workspace_arguments_are_scoped_and_bounded() {
        let review = MeridianOptions::parse([
            "--creator-alpha-ui-review",
            "--project",
            "examples/creator-alpha",
            "--review-workspace",
            "ui",
            "--review-size",
            "1280x800",
        ])
        .expect("Creator UI review workspace parses");
        assert_eq!(review.mode, RunMode::CreatorAlphaUiReview);
        assert_eq!(review.review_workspace, Some(WorkspaceKind::UiAuthoring));
        assert_eq!(review.review_size, Some(WindowSize::new(1280, 800)));
        assert!(matches!(
            MeridianOptions::parse([
                "--creator-alpha-ui-review",
                "--project",
                "examples/creator-alpha",
                "--review-workspace",
                "unknown",
            ]),
            Err(MeridianArgumentError::InvalidReviewWorkspace(value)) if value == "unknown"
        ));
        assert!(matches!(
            MeridianOptions::parse(["--review-workspace", "world"]),
            Err(MeridianArgumentError::ReviewWorkspaceRequiresUiReview)
        ));
        for invalid in ["1023x720", "1280x719", "4097x800", "wide"] {
            assert!(matches!(
                MeridianOptions::parse([
                    "--creator-alpha-ui-review",
                    "--project",
                    "examples/creator-alpha",
                    "--review-size",
                    invalid,
                ]),
                Err(MeridianArgumentError::InvalidReviewSize(value)) if value == invalid
            ));
        }
        assert!(matches!(
            MeridianOptions::parse(["--review-size", "1280x800"]),
            Err(MeridianArgumentError::ReviewSizeRequiresUiReview)
        ));
    }

    #[test]
    fn creator_ui_review_uses_an_empty_ephemeral_hub() {
        let project = workspace_root()
            .expect("workspace root")
            .join("examples/creator-alpha");
        let application = CreatorApplication::new_for_local_ui_review(Some(&project), true)
            .expect("local Creator review opens");
        assert_eq!(
            application.run_persistence,
            CreatorRunPersistence::InMemoryReview
        );
        assert!(application.hub.recents.is_empty());
        assert!(matches!(application.screen, CreatorScreen::Workspace(_)));
    }

    #[test]
    fn implicit_smoke_evidence_paths_are_unique_per_run() {
        let project = resolve_project_root(None).expect("project root resolves");
        let first = default_evidence_root(&project);
        let second = default_evidence_root(&project);
        assert_ne!(first, second);
        assert!(first.starts_with(project.join(DEFAULT_EVIDENCE)));
        assert!(second.starts_with(project.join(DEFAULT_EVIDENCE)));
    }

    #[test]
    fn explicit_smoke_evidence_path_remains_caller_owned() {
        let project = PathBuf::from("/meridian-project");
        let relative = PathBuf::from("evidence/caller-owned");
        let absolute = PathBuf::from("/tmp/meridian-evidence");
        assert_eq!(
            resolve_output_path(&project, Some(&relative), Path::new(DEFAULT_EVIDENCE)),
            project.join(relative)
        );
        assert_eq!(
            resolve_output_path(&project, Some(&absolute), Path::new(DEFAULT_EVIDENCE)),
            absolute
        );
    }

    #[test]
    fn ui_headless_smoke_verifies_recovery_and_disabled_overlay() {
        run_ui_headless_smoke().expect("UI fixture passes");
    }

    #[test]
    fn headless_ms01_builds_streams_activates_and_recovers() {
        let project = resolve_project_root(None).expect("project root resolves");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let evidence = std::env::temp_dir().join(format!("meridian-ms01-{nonce}"));
        let prepared = prepare_ms01(&project, &evidence, 3).expect("MS-01 preparation passes");
        assert_eq!(prepared.summary.streamed_cell, CheckStatus::Pass);
        assert_eq!(
            prepared.summary.fresh_runtime_reconstruction,
            CheckStatus::Pass
        );
        assert_eq!(prepared.summary.source_derived_render_instances, 1);
        assert_eq!(prepared.summary.milestone_outcome, "Inconclusive");
        assert_eq!(prepared.summary.local_run_outcome, CheckStatus::Pass);
        write_evidence_bundle(&evidence, &prepared.timeline, &prepared.summary)
            .expect("evidence writes");
        let summary: Value = serde_json::from_slice(
            &fs::read(evidence.join("summary.json")).expect("summary reads"),
        )
        .expect("summary parses");
        assert_eq!(summary["milestone"], "MS-01");
        fs::remove_dir_all(evidence).expect("remove evidence");
    }

    #[test]
    fn secret_diagnostic_fields_are_redacted_in_bundle() {
        let mut timeline = DiagnosticTimeline::new(NonZeroUsize::new(2).expect("nonzero"));
        let mut event = DiagnosticEvent::new("SEC-TEST", "SEC", DiagnosticSeverity::Info, "test")
            .with_field("token", "must-not-appear");
        event.redaction_class = RedactionClass::Secret;
        timeline.push(event);
        let json = timeline.to_json_pretty().expect("timeline serializes");
        assert!(!json.contains("must-not-appear"));
        assert!(json.contains("redacted"));
    }

    #[test]
    fn fixture_source_hash_is_stable_across_checkout_line_endings() {
        let fixture = fs::read(
            resolve_project_root(None)
                .expect("project root")
                .join(MESH_SOURCE),
        )
        .expect("fixture reads");
        let imported =
            meridian_asset_tools::import_fixture_mesh(&fixture, &CancellationToken::new())
                .expect("fixture imports");
        assert_eq!(
            imported.metadata.source_hash.to_string(),
            "ae037da0e1bdfb1175812ac7333322c7dea5b7c4032a1c5ad8f532f1d7535569"
        );
    }

    #[test]
    fn public_creator_sample_source_matches_the_dat_import_authority() {
        let root = workspace_root()
            .expect("workspace root")
            .join("examples/creator-alpha");
        let manifest: CreatorAlphaManifest = serde_json::from_slice(
            &fs::read(root.join(CREATOR_ALPHA_MANIFEST)).expect("manifest reads"),
        )
        .expect("manifest parses");
        let imported = import_creator_alpha_source(&root, &manifest.imported_asset)
            .expect("public source imports");
        let document = ProjectDocument::read_source(root.join(CREATOR_PROJECT_SOURCE))
            .expect("canonical project source parses");
        assert_eq!(document.sources.get(&imported.id), Some(&imported));
        assert_eq!(
            document.placements[&manifest.placement.id].source_id,
            imported.id
        );
    }

    #[test]
    fn opening_the_public_creator_sample_keeps_workspace_preferences_outside_source() {
        let root = workspace_root()
            .expect("workspace root")
            .join("examples/creator-alpha");
        let legacy_state = root
            .join(CREATOR_INTERNAL_DIRECTORY)
            .join(CREATOR_WORKSPACE_STATE);
        assert!(
            !legacy_state.exists(),
            "public source examples must not contain mutable workspace state"
        );

        let application =
            CreatorApplication::new(Some(&root), true).expect("public Creator workspace opens");
        let state_path = creator_workspace_for_smoke(&application)
            .expect("workspace")
            .workspace_store
            .path()
            .to_path_buf();
        assert!(
            !state_path.starts_with(&root),
            "workspace preferences must remain outside authoritative project source"
        );
        assert!(!legacy_state.exists());
    }

    #[test]
    fn creator_hub_actions_are_bounded_and_unknown_actions_are_rejected() {
        assert!(matches!(
            CreatorUiAction::parse("hub.open-recent:0"),
            Ok(CreatorUiAction::OpenRecent(0))
        ));
        assert!(CreatorUiAction::parse("hub.open-recent:not-an-index").is_err());
        assert!(CreatorUiAction::parse("untrusted.run-shell").is_err());
        for panel in creator_alpha_panels() {
            for action in panel.commands {
                assert!(
                    CreatorUiAction::parse(action).is_ok(),
                    "{} exposes an unhandled Creator action: {action}",
                    panel.title
                );
            }
        }
    }

    #[test]
    fn workspace_switches_are_typed_persisted_and_restore_their_focus_layout() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-workspace-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), true).expect("Creator workspace opens");
        assert_eq!(
            creator_workspace_for_smoke(&application)
                .expect("workspace")
                .active_workspace(),
            WorkspaceKind::World
        );

        application
            .activate_workspace_action_for_smoke("workspace.alluvium")
            .expect("Alluvium tab activates");
        let workspace = creator_workspace_for_smoke(&application).expect("workspace");
        assert_eq!(workspace.active_workspace(), WorkspaceKind::Alluvium);
        assert!(workspace.workspace_store.path().is_file());
        let document = application.ui.document();
        assert!(document
            .node(document.root())
            .is_some_and(|node| node.semantics.name.contains("Alluvium")));

        application
            .activate_workspace_action_for_smoke("workspace.code")
            .expect("Code tab activates");
        assert!(!creator_workspace_for_smoke(&application)
            .expect("workspace")
            .active_focus_layout());
        assert!(application
            .frame
            .display_list
            .primitives
            .iter()
            .any(|primitive| {
                matches!(
                    primitive,
                    meridian_ui::DisplayPrimitive::Path { node, .. }
                        if *node == meridian_ui_editor::CREATOR_WORLD_VIEWPORT_CANVAS
                )
            }));
        application
            .activate_workspace_action_for_smoke("workspace.code")
            .expect("second Code activation enters focus");
        assert!(creator_workspace_for_smoke(&application)
            .expect("workspace")
            .active_focus_layout());

        let reopened =
            CreatorApplication::new(Some(&root), true).expect("persisted workspace reopens");
        let workspace = creator_workspace_for_smoke(&reopened).expect("workspace");
        assert_eq!(workspace.active_workspace(), WorkspaceKind::Code);
        assert!(workspace.active_focus_layout());
        fs::remove_file(workspace.workspace_store.path()).expect("workspace state removes");
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn corrupt_workspace_state_recovers_without_mutating_authoritative_project_source() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent =
            std::env::temp_dir().join(format!("meridian-creator-workspace-recovery-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let application =
            CreatorApplication::new(Some(&root), true).expect("Creator workspace opens");
        let state_path = creator_workspace_for_smoke(&application)
            .expect("workspace")
            .workspace_store
            .path()
            .to_path_buf();
        fs::create_dir_all(state_path.parent().expect("workspace state parent"))
            .expect("workspace state parent creates");
        let source_path = root.join(CREATOR_PROJECT_SOURCE);
        let source_before = fs::read(&source_path).expect("source reads");
        fs::write(&state_path, b"not a workspace state envelope").expect("state corrupts");

        let recovered = CreatorApplication::new(Some(&root), true).expect("corrupt state recovers");
        let workspace = creator_workspace_for_smoke(&recovered).expect("workspace");
        assert_eq!(workspace.active_workspace(), WorkspaceKind::World);
        assert!(workspace
            .status
            .contains("Workspace layout was recovered from defaults"));
        assert_eq!(fs::read(&source_path).expect("source reads"), source_before);
        fs::remove_file(workspace.workspace_store.path()).expect("workspace state removes");
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn host_bound_assistive_actions_keep_their_explicit_command() {
        let target = UiNodeId::new(0xA11);
        let ordinary = UiCommandRequest {
            source: target,
            command: CommandId::from_name("world.select-camera").expect("valid command"),
            action: "world.select-camera".to_owned(),
        };
        let expand = UiAssistiveRequest {
            target,
            action: SemanticAction::Expand,
            command: CommandId::from_name("world.expand-camera").expect("valid command"),
            command_name: "world.expand-camera".to_owned(),
        };
        assert_eq!(
            ui_effect_action_names(&[ordinary], &[expand]),
            Ok(vec![
                "world.select-camera".to_owned(),
                "world.expand-camera".to_owned(),
            ])
        );

        let malformed = UiAssistiveRequest {
            target,
            action: SemanticAction::Expand,
            command: CommandId::from_name("world.expand-camera").expect("valid command"),
            command_name: "world.select-camera".to_owned(),
        };
        assert_eq!(
            ui_effect_action_names(&[], &[malformed]),
            Err(UiEffectCommandError::CommandIdentityMismatch)
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Covers the complete typed Inspector transaction boundary.
    fn typed_inspector_ui_edits_persist_and_reject_invalid_or_noop_source_changes() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-inspector-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), true).expect("public Creator workspace opens");
        let (
            placement_id,
            source_path,
            original,
            original_document,
            original_generation,
            original_undo,
        ) = {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            let placement_id = first_placement_id(&workspace.session).expect("placement");
            (
                placement_id,
                workspace.project_store.source_path().to_path_buf(),
                workspace.session.document().placements[&placement_id].translation,
                workspace.session.document().clone(),
                workspace.session.document().generation,
                workspace.session.undo_depth(),
            )
        };
        let source_before = fs::read(&source_path).expect("source reads");
        let edit = application
            .creator_action_node("editor.edit-placement")
            .expect("edit action exists");
        let emitted = application
            .reconcile_workspace_ui_events_for_smoke(vec![
                UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
                UiEvent::SelectAllText,
                UiEvent::TextCommit("not-a-millimetre".to_owned()),
                UiEvent::AssistiveActivate(edit),
            ])
            .expect("invalid UI action remains a diagnostic");
        assert_creator_ui_command(&emitted, "editor.edit-placement").expect("semantic action");
        {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            assert_eq!(fs::read(&source_path).expect("source reads"), source_before);
            assert_eq!(workspace.session.document(), &original_document);
            assert_eq!(workspace.session.document().generation, original_generation);
            assert_eq!(workspace.session.undo_depth(), original_undo);
            assert!(workspace
                .status
                .contains("X coordinate must be a signed whole millimetre value"));
        }

        let candidate = Translation {
            x_mm: original
                .x_mm
                .checked_add(375)
                .expect("bounded test coordinate"),
            ..original
        };
        let emitted = application
            .reconcile_workspace_ui_events_for_smoke(vec![
                UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
                UiEvent::SelectAllText,
                UiEvent::TextCommit(candidate.x_mm.to_string()),
                UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Y_MM),
                UiEvent::SelectAllText,
                UiEvent::TextCommit(candidate.y_mm.to_string()),
                UiEvent::AssistiveFocus(CREATOR_INSPECTOR_Z_MM),
                UiEvent::SelectAllText,
                UiEvent::TextCommit(candidate.z_mm.to_string()),
                UiEvent::AssistiveActivate(edit),
            ])
            .expect("valid UI edit persists");
        assert_creator_ui_command(&emitted, "editor.edit-placement").expect("semantic action");
        {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            assert_eq!(
                workspace.session.document().placements[&placement_id].translation,
                candidate
            );
            assert_persisted_creator_source(&source_path, &workspace.session)
                .expect("canonical source persists");
        }
        assert_eq!(
            application.ui.text_input_value(CREATOR_INSPECTOR_X_MM),
            Some(candidate.x_mm.to_string().as_str())
        );

        let (source_after_edit, generation_after_edit, undo_after_edit) = {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            (
                fs::read(&source_path).expect("source reads"),
                workspace.session.document().generation,
                workspace.session.undo_depth(),
            )
        };
        let emitted = application
            .reconcile_workspace_ui_events_for_smoke(vec![UiEvent::AssistiveActivate(edit)])
            .expect("no-op UI edit remains a diagnostic");
        assert_creator_ui_command(&emitted, "editor.edit-placement").expect("semantic action");
        {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            assert_eq!(
                fs::read(&source_path).expect("source reads"),
                source_after_edit
            );
            assert_eq!(
                workspace.session.document().generation,
                generation_after_edit
            );
            assert_eq!(workspace.session.undo_depth(), undo_after_edit);
            assert!(workspace.status.starts_with("No source change:"));
        }

        let emitted = application
            .activate_workspace_action_for_smoke("shell.open-shelf")
            .expect("shelf action routes through UI");
        assert_creator_ui_command(&emitted, "shell.open-shelf").expect("semantic shelf action");
        assert_eq!(
            creator_workspace_for_smoke(&application)
                .expect("workspace")
                .active_focused_panel(),
            Some(EditorPanelId::History)
        );

        let emitted = application
            .activate_workspace_action_for_smoke("editor.undo")
            .expect("undo action routes through UI");
        assert_creator_ui_command(&emitted, "editor.undo").expect("semantic undo action");
        assert_eq!(
            creator_workspace_for_smoke(&application)
                .expect("workspace")
                .session
                .document()
                .placements[&placement_id]
                .translation,
            original
        );
        let expected_x = original.x_mm.to_string();
        assert_eq!(
            application.ui.text_input_value(CREATOR_INSPECTOR_X_MM),
            Some(expected_x.as_str())
        );

        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn explicit_picker_cancellation_and_invalid_selection_stay_in_hub_remediation() {
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes without a picker");
        application
            .execute_explicit_picker_action(
                CreatorUiAction::OpenProject,
                &FakeCreatorProjectPicker {
                    selection: None,
                    invocations: Cell::new(0),
                },
                None,
            )
            .expect("picker cancellation remains a hub result");
        assert_eq!(application.hub_status, "Project open cancelled.");
        assert!(matches!(application.screen, CreatorScreen::Hub));

        application
            .execute_explicit_picker_action(
                CreatorUiAction::OpenProject,
                &FakeCreatorProjectPicker {
                    selection: Some(PathBuf::from("not-a-creator-project")),
                    invocations: Cell::new(0),
                },
                None,
            )
            .expect("invalid picker selection remains a hub result");
        assert!(matches!(application.screen, CreatorScreen::Hub));
        assert!(application
            .hub_status
            .starts_with("Unable to open the selected project:"));

        let forbidden_picker = FakeCreatorProjectPicker {
            selection: None,
            invocations: Cell::new(0),
        };
        assert!(application
            .execute_explicit_picker_action(CreatorUiAction::ReturnHub, &forbidden_picker, None)
            .is_err());
        assert_eq!(forbidden_picker.invocations.get(), 0);
    }

    #[test]
    fn missing_recent_location_is_explicit_transactional_and_cancellable() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-locate-recent-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let replacement = create_public_creator_project(&parent, "Located Creator Project")
            .expect("replacement project creates");
        let missing = parent.join("moved-project");
        let mut application = CreatorApplication::new(None, true).expect("Creator hub initializes");
        application.hub.recents = vec![CreatorRecentProject {
            label: "Moved project".to_owned(),
            path: missing.display().to_string(),
        }];
        application.hub_store = CreatorHubStore {
            path: parent.join("launch-hub.json"),
        };

        let cancelled = FakeCreatorProjectPicker {
            selection: None,
            invocations: Cell::new(0),
        };
        application
            .locate_recent_project(0, &cancelled, None)
            .expect("cancellation is nonfatal");
        assert_eq!(cancelled.invocations.get(), 1);
        assert_eq!(
            application.hub.recents[0].path,
            missing.display().to_string()
        );
        assert_eq!(
            application.hub_status,
            "Recent-project location was not changed."
        );

        let invalid = FakeCreatorProjectPicker {
            selection: Some(parent.clone()),
            invocations: Cell::new(0),
        };
        application
            .locate_recent_project(0, &invalid, None)
            .expect("invalid replacement remains hub remediation");
        assert_eq!(invalid.invocations.get(), 1);
        assert!(matches!(application.screen, CreatorScreen::Hub));
        assert_eq!(
            application.hub.recents[0].path,
            missing.display().to_string()
        );
        assert!(application
            .hub_status
            .starts_with("The selected replacement is not openable:"));

        let located = FakeCreatorProjectPicker {
            selection: Some(replacement.clone()),
            invocations: Cell::new(0),
        };
        application
            .locate_recent_project(0, &located, None)
            .expect("valid replacement opens transactionally");
        assert_eq!(located.invocations.get(), 1);
        assert!(matches!(application.screen, CreatorScreen::Workspace(_)));
        let canonical_replacement = fs::canonicalize(&replacement)
            .expect("replacement canonicalizes")
            .display()
            .to_string();
        assert_eq!(application.hub.recents[0].path, canonical_replacement);
        assert!(application
            .hub
            .recents
            .iter()
            .all(|recent| recent.path != missing.display().to_string()));
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn shell_search_restores_focus_to_the_current_surface_search_control() {
        let mut hub =
            CreatorApplication::new(None, true).expect("Creator hub initializes for search");
        hub.execute_action_with_window("shell.search", None)
            .expect("hub search action is typed");
        hub.refresh_ui_without_events()
            .expect("hub search focus reconciles");
        assert_eq!(hub.frame.focused, Some(CREATOR_HUB_PROJECT_NAME));

        let mut workspace = CreatorApplication::new(
            Some(
                &workspace_root()
                    .expect("workspace root")
                    .join("examples/creator-alpha"),
            ),
            true,
        )
        .expect("Creator workspace initializes for search");
        workspace
            .execute_action_with_window("shell.search", None)
            .expect("World search action is typed");
        workspace
            .refresh_ui_without_events()
            .expect("World search focus reconciles");
        assert_eq!(
            workspace.frame.focused,
            Some(meridian_ui_editor::CREATOR_WORLD_SEARCH)
        );
    }

    #[test]
    fn shell_settings_and_favorites_are_contextual_and_nonfatal_from_the_hub() {
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for shell actions");

        application
            .execute_action_with_window("shell.settings", None)
            .expect("hub Settings action remains nonfatal");
        assert!(matches!(
            application.screen,
            CreatorScreen::Settings {
                resume_workspace: None
            }
        ));
        assert_eq!(application.settings_status, "Local preferences are ready.");
        application
            .refresh_document_and_ui()
            .expect("Settings document refreshes");
        application
            .execute_action_with_window("shell.search", None)
            .expect("Settings search action remains typed");
        application
            .refresh_ui_without_events()
            .expect("Settings search focus reconciles");
        assert_eq!(application.frame.focused, Some(CREATOR_SETTINGS_SEARCH));
        application.route_key_down(ButtonControl::Key(KeyCode::Escape));
        assert_eq!(application.pending_actions, vec!["settings.return"]);
        application.pending_actions.clear();
        application
            .execute_action_with_window("settings.return", None)
            .expect("Settings return action remains nonfatal");

        application
            .execute_action_with_window("shell.favorites", None)
            .expect("hub Favorites action remains nonfatal");
        assert!(matches!(application.screen, CreatorScreen::Hub));
        assert_eq!(
            application.hub_status,
            "Open a project before using favorites."
        );
    }

    #[test]
    fn settings_preferences_persist_locally_and_apply_to_retained_frames() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-preferences-{nonce}"));
        fs::create_dir(&parent).expect("temporary preference directory creates");
        let preferences_path = parent.join("hub.json");
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for preferences");
        application.hub_store = CreatorHubStore {
            path: preferences_path.clone(),
        };
        application.hub = CreatorHubState::default();

        application
            .execute_action_with_window("shell.settings", None)
            .expect("Settings opens");
        application
            .execute_action_with_window("settings.toggle-high-contrast", None)
            .expect("high contrast persists");
        application
            .execute_action_with_window("settings.toggle-reduced-motion", None)
            .expect("reduced motion persists");
        application
            .execute_action_with_window("settings.density-comfortable", None)
            .expect("density persists");
        application
            .refresh_document_and_ui()
            .expect("preferences reconcile into the retained frame");
        assert!(application.hub.preferences.high_contrast);
        assert!(application.hub.preferences.reduced_motion);
        assert_eq!(
            application.hub.preferences.density,
            CreatorDensityPreference::Comfortable
        );
        assert_eq!(application.frame.contrast, UiContrast::High);
        assert_eq!(application.frame.motion, MotionPreference::Reduced);
        assert_eq!(application.frame.density, UiDensity::Comfortable);

        let stored = CreatorHubStore {
            path: preferences_path.clone(),
        }
        .load()
        .expect("local preference state reloads");
        assert_eq!(stored.preferences, application.hub.preferences);

        application
            .execute_action_with_window("settings.reset-preferences", None)
            .expect("preferences reset atomically");
        assert_eq!(application.hub.preferences, CreatorPreferences::default());
        fs::remove_dir_all(parent).expect("temporary preference directory removes");
    }

    #[test]
    fn v1_hub_state_migrates_to_versioned_preference_state() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-hub-v1-{nonce}"));
        fs::create_dir(&parent).expect("temporary hub directory creates");
        let store = CreatorHubStore {
            path: parent.join("hub.json"),
        };
        fs::write(
            &store.path,
            serde_json::to_vec_pretty(&json!({
                "schema": CREATOR_HUB_SCHEMA_V1,
                "recents": [],
            }))
            .expect("v1 hub state serializes"),
        )
        .expect("v1 hub state writes");

        let mut state = store.load().expect("v1 hub state loads");
        assert_eq!(state.preferences, CreatorPreferences::default());
        assert!(state.migrate_preferences_schema());
        store.save(&state).expect("migrated hub state saves");
        let reloaded = store.load().expect("migrated hub state reloads");
        assert_eq!(reloaded.schema, CREATOR_HUB_SCHEMA);
        assert_eq!(reloaded.preferences, CreatorPreferences::default());
        fs::remove_dir_all(parent).expect("temporary hub directory removes");
    }

    #[test]
    fn project_settings_return_to_the_same_workspace_without_mutating_source() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent =
            std::env::temp_dir().join(format!("meridian-creator-settings-project-{nonce}"));
        fs::create_dir(&parent).expect("temporary project parent creates");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let source_path = root.join(CREATOR_PROJECT_SOURCE);
        let source_before = fs::read(&source_path).expect("project source reads");
        let mut application =
            CreatorApplication::new(Some(&root), true).expect("public project opens");
        application.hub_store = CreatorHubStore {
            path: parent.join("hub.json"),
        };
        application
            .execute_action_with_window("shell.settings", None)
            .expect("project Settings opens");
        assert!(matches!(
            application.screen,
            CreatorScreen::Settings {
                resume_workspace: Some(_)
            }
        ));
        application
            .execute_action_with_window("settings.toggle-high-contrast", None)
            .expect("preference changes");
        application
            .execute_action_with_window("settings.return", None)
            .expect("project returns from Settings");
        assert_eq!(
            fs::read(&source_path).expect("project source reads"),
            source_before
        );
        let workspace = creator_workspace_for_smoke(&application).expect("workspace restores");
        assert_eq!(workspace.active_workspace(), WorkspaceKind::World);
        fs::remove_dir_all(parent).expect("temporary project parent removes");
    }

    #[test]
    fn settings_keep_project_actions_bound_to_the_suspended_session() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent =
            std::env::temp_dir().join(format!("meridian-creator-settings-context-{nonce}"));
        fs::create_dir(&parent).expect("temporary project parent creates");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), false).expect("public project opens");
        application
            .execute_action_with_window("shell.settings", None)
            .expect("Settings opens");
        application
            .refresh_document_and_ui()
            .expect("Settings document refreshes");
        assert!(application
            .ui
            .document()
            .focus_order()
            .iter()
            .any(|id| application
                .ui
                .document()
                .node(*id)
                .and_then(|node| node.semantics.action.as_deref())
                == Some("build.submit")));

        application
            .execute_action_with_window("editor.play-start", None)
            .expect("Play starts through the suspended project");
        assert!(matches!(application.screen, CreatorScreen::Settings { .. }));
        let CreatorScreen::Settings {
            resume_workspace: Some(workspace),
        } = &application.screen
        else {
            panic!("Settings must retain the project session");
        };
        assert!(workspace.session.play_active());
        application
            .execute_action_with_window("editor.play-discard", None)
            .expect("Play discards through the suspended project");

        fs::remove_dir_all(parent).expect("temporary project parent removes");
    }

    #[test]
    fn suspended_project_builds_continue_through_settings() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-settings-build-{nonce}"));
        fs::create_dir(&parent).expect("temporary project parent creates");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut workspace = CreatorWorkspace::open(&root).expect("public project opens");
        let (sender, receiver) = mpsc::channel();
        workspace.build = Some(CreatorBuildTask { receiver });
        let mut screen = CreatorScreen::Settings {
            resume_workspace: Some(Box::new(workspace)),
        };

        assert!(creator_has_active_build(&screen));
        drop(sender);
        assert!(poll_creator_build(&mut screen));
        assert!(!creator_has_active_build(&screen));

        fs::remove_dir_all(parent).expect("temporary project parent removes");
    }

    #[test]
    fn active_build_cannot_be_dropped_by_returning_to_the_hub() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-return-build-{nonce}"));
        fs::create_dir(&parent).expect("temporary project parent creates");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), false).expect("public project opens");
        let (sender, receiver) = mpsc::channel();
        creator_workspace_for_smoke_mut(&mut application)
            .expect("workspace")
            .build = Some(CreatorBuildTask { receiver });

        application
            .execute_action_with_window("editor.return-hub", None)
            .expect("return action is handled");
        assert!(matches!(application.screen, CreatorScreen::Workspace(_)));
        assert!(creator_workspace_for_smoke(&application)
            .expect("workspace")
            .status
            .contains("build is still running"));

        drop(sender);
        fs::remove_dir_all(parent).expect("temporary project parent removes");
    }

    #[test]
    fn shell_panels_cycles_and_persists_the_active_workspace_pane() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-pane-cycle-{nonce}"));
        fs::create_dir(&parent).expect("temporary project parent creates");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), false).expect("public project opens");

        application
            .execute_action_with_window("shell.panels", None)
            .expect("pane cycle is typed");
        let workspace = creator_workspace_for_smoke(&application).expect("workspace");
        let layout = workspace
            .workspace_state
            .layouts
            .iter()
            .find(|layout| {
                layout.workspace == WorkspaceKind::World && layout.name == CREATOR_DEFAULT_LAYOUT
            })
            .expect("World layout persists");
        assert_eq!(
            layout.focused_panel,
            Some(PanelId::from(EditorPanelId::Hierarchy))
        );
        assert!(workspace.status.contains("Hierarchy pane is active"));
        assert!(workspace.workspace_store.path().is_file());

        fs::remove_dir_all(parent).expect("temporary project parent removes");
    }

    #[test]
    fn interactive_creator_lifetime_is_persistent_while_ui_smoke_is_bounded() {
        assert_eq!(MeridianOptions::default().mode, RunMode::Interactive);
        assert_eq!(creator_event_loop_mode(false), EventLoopMode::Wait);
        assert_eq!(creator_event_loop_mode(true), EventLoopMode::Poll);
        assert!(!creator_exits_after_visible_presentation(false, u8::MAX));
        assert!(!creator_exits_after_surface_attempt(false, u8::MAX));
        assert!(!creator_exits_after_visible_presentation(true, 1));
        assert!(creator_exits_after_visible_presentation(true, 2));
        assert!(creator_requests_follow_up_redraw(false, 1, false));
        assert!(!creator_requests_follow_up_redraw(false, 2, false));
        assert!(creator_requests_follow_up_redraw(false, 2, true));
        assert!(creator_requests_follow_up_redraw(true, 1, false));
        assert!(!creator_requests_follow_up_redraw(true, 2, false));
        assert!(!creator_exits_after_surface_attempt(true, 2));
        assert!(creator_exits_after_surface_attempt(
            true,
            UI_SMOKE_MAX_PRESENT_ATTEMPTS
        ));
        assert!(!creator_surface_is_renderable(WindowSize::new(0, 800)));
        assert!(!creator_surface_is_renderable(WindowSize::new(1280, 0)));
        assert!(creator_surface_is_renderable(WindowSize::new(1280, 800)));

        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for bootstrap");
        application.schedule_bootstrap_renderer_refresh();
        assert!(application.take_bootstrap_renderer_refresh());
        assert!(!application.take_bootstrap_renderer_refresh());
    }

    #[test]
    fn creator_pointer_routes_real_press_and_release() {
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for input");
        application.pointer = UiPoint { x: 120.0, y: 80.0 };

        application.route_input(NativeInputEvent::Button {
            control: ButtonControl::Mouse(meridian_input::MouseButton::Left),
            down: true,
        });
        assert_eq!(
            application.pending_events,
            vec![UiEvent::Pointer(UiPointerEvent {
                device: CREATOR_POINTER_DEVICE,
                kind: UiInputDeviceKind::Mouse,
                phase: UiPointerPhase::Press,
                position: application.pointer,
                button: Some(UiPointerButton::Primary),
            })]
        );

        application.route_input(NativeInputEvent::Button {
            control: ButtonControl::Mouse(meridian_input::MouseButton::Left),
            down: false,
        });
        assert_eq!(
            application.pending_events,
            vec![
                UiEvent::Pointer(UiPointerEvent {
                    device: CREATOR_POINTER_DEVICE,
                    kind: UiInputDeviceKind::Mouse,
                    phase: UiPointerPhase::Press,
                    position: application.pointer,
                    button: Some(UiPointerButton::Primary),
                }),
                UiEvent::Pointer(UiPointerEvent {
                    device: CREATOR_POINTER_DEVICE,
                    kind: UiInputDeviceKind::Mouse,
                    phase: UiPointerPhase::Release,
                    position: application.pointer,
                    button: Some(UiPointerButton::Primary),
                })
            ]
        );
    }

    #[test]
    fn creator_pointer_move_routes_without_a_drag_and_uses_logical_coordinates() {
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for pointer move");
        application.scale_factor = 2.0;
        assert!(application.frame.drag.is_none());

        application.route_pointer_move(240.0, 160.0);

        assert_eq!(application.pointer, UiPoint { x: 120.0, y: 80.0 });
        assert_eq!(
            application.pending_events,
            vec![UiEvent::Pointer(UiPointerEvent {
                device: CREATOR_POINTER_DEVICE,
                kind: UiInputDeviceKind::Mouse,
                phase: UiPointerPhase::Move,
                position: UiPoint { x: 120.0, y: 80.0 },
                button: None,
            })]
        );
    }

    #[test]
    fn creator_keyboard_space_activates_on_press() {
        let mut application = CreatorApplication::new(None, true)
            .expect("Creator hub initializes for keyboard input");

        application.route_input(NativeInputEvent::Button {
            control: ButtonControl::Key(KeyCode::Space),
            down: true,
        });
        application.route_input(NativeInputEvent::Button {
            control: ButtonControl::Key(KeyCode::Space),
            down: false,
        });

        assert_eq!(application.pending_events, vec![UiEvent::Activate]);
    }

    #[test]
    fn creator_escape_and_focus_loss_use_unified_interaction_cancellation() {
        let mut application = CreatorApplication::new(None, true)
            .expect("Creator hub initializes for cancellation routing");

        application.route_input(NativeInputEvent::Button {
            control: ButtonControl::Key(KeyCode::Escape),
            down: true,
        });
        assert_eq!(application.pending_events, vec![UiEvent::CancelInteraction]);

        application.pending_events.clear();
        application.route_input(NativeInputEvent::FocusLost);
        assert_eq!(application.pending_events, vec![UiEvent::CancelInteraction]);
    }

    #[test]
    fn creator_scroll_routes_precise_and_discrete_deltas_without_double_smoothing() {
        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes for scroll input");
        application.pointer = UiPoint { x: 140.0, y: 90.0 };

        application.route_input(NativeInputEvent::Scroll(NativeScrollEvent {
            phase: NativeScrollPhase::Update,
            unit: NativeScrollUnit::Pixels,
            x: 0.25,
            y: -10.5,
        }));
        assert_eq!(
            application.pending_events,
            vec![UiEvent::Scroll(UiScrollEvent {
                device: CREATOR_POINTER_DEVICE,
                kind: UiInputDeviceKind::Trackpad,
                phase: UiScrollPhase::Update,
                position: application.pointer,
                delta: UiScrollDelta {
                    x: -0.25,
                    y: 10.5,
                    unit: UiScrollUnit::Pixels,
                },
            })]
        );

        application.pending_events.clear();
        application.route_input(NativeInputEvent::Scroll(NativeScrollEvent {
            phase: NativeScrollPhase::Update,
            unit: NativeScrollUnit::Lines,
            x: 0.0,
            y: -2.0,
        }));
        assert_eq!(application.pending_events.len(), 2);
        assert!(matches!(
            application.pending_events.as_slice(),
            [
                UiEvent::Scroll(UiScrollEvent {
                    phase: UiScrollPhase::Update,
                    delta: UiScrollDelta {
                        y: 2.0,
                        unit: UiScrollUnit::Lines,
                        ..
                    },
                    ..
                }),
                UiEvent::Scroll(UiScrollEvent {
                    phase: UiScrollPhase::End,
                    ..
                })
            ]
        ));
    }

    #[test]
    fn creator_reports_unavailable_clipboard_without_mutating_source() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-copy-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), true).expect("public Creator workspace opens");
        let source_path = creator_workspace_for_smoke(&application)
            .expect("workspace")
            .project_store
            .source_path()
            .to_path_buf();
        let source_before = fs::read(&source_path).expect("source reads");
        let draft_before = application
            .ui
            .text_input_value(CREATOR_INSPECTOR_X_MM)
            .expect("inspector draft")
            .to_owned();

        application
            .reconcile_workspace_ui_events_for_smoke(vec![
                UiEvent::AssistiveFocus(CREATOR_INSPECTOR_X_MM),
                UiEvent::SelectAllText,
                UiEvent::CopySelection,
            ])
            .expect("copy request remains a typed UI result");
        application
            .reconcile_workspace_ui_events_for_smoke(vec![UiEvent::CutSelection])
            .expect("unconfirmed cut remains non-destructive");

        let workspace = creator_workspace_for_smoke(&application).expect("workspace");
        assert_eq!(fs::read(&source_path).expect("source reads"), source_before);
        assert_eq!(
            application.ui.text_input_value(CREATOR_INSPECTOR_X_MM),
            Some(draft_before.as_str())
        );
        assert_eq!(
            workspace.status,
            "Clipboard access is unavailable until Meridian's platform adapter is active."
        );
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn creator_terminal_errors_remain_typed_for_the_platform_runner() {
        let mut application = CreatorApplication::new(None, true)
            .expect("Creator hub initializes for failure handoff");

        application.record_terminal_error("synthetic renderer failure");

        let error = application
            .terminal_error()
            .expect("Creator retains terminal application failure");
        assert_eq!(
            error.kind(),
            meridian_platform::PlatformErrorKind::Application
        );
        assert!(error.to_string().contains("synthetic renderer failure"));
    }

    #[test]
    fn zero_extent_creator_surface_skips_renderer_rebuild_until_restore() {
        let rebuild_calls = Cell::new(0);
        let skipped = rebuild_for_renderable_creator_surface(WindowSize::new(0, 0), || {
            rebuild_calls.set(rebuild_calls.get() + 1);
            Ok(())
        })
        .expect("minimized Creator surface is nonfatal");
        assert!(skipped.is_none());
        assert_eq!(rebuild_calls.get(), 0);

        let rebuilt = rebuild_for_renderable_creator_surface(WindowSize::new(1280, 800), || {
            rebuild_calls.set(rebuild_calls.get() + 1);
            Ok(())
        })
        .expect("restored Creator surface is nonfatal");
        assert_eq!(rebuilt, Some(()));
        assert_eq!(rebuild_calls.get(), 1);
    }

    #[test]
    fn recent_state_write_failure_does_not_block_an_authoritative_project_open() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-recent-save-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let blocked_parent = parent.join("recents-parent-is-a-file");
        fs::write(&blocked_parent, b"not a directory").expect("blocked state parent writes");

        let mut application =
            CreatorApplication::new(None, true).expect("Creator hub initializes without a project");
        application.hub_store = CreatorHubStore {
            path: blocked_parent.join("launch-hub.json"),
        };
        application
            .open_project(&root)
            .expect("authoritative project opens despite recents failure");

        let CreatorScreen::Workspace(workspace) = &application.screen else {
            panic!("Creator project did not open a workspace");
        };
        assert!(workspace
            .status
            .contains("Recent-project state could not be saved"));
        assert_eq!(
            workspace.session.document().schema,
            meridian_editor_core::PROJECT_SCHEMA
        );
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn creator_hub_rejects_oversized_state_before_reading_it() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-hub-bound-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let path = parent.join("launch-hub.json");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("sparse hub state creates");
        file.set_len(
            u64::try_from(SaveConfig::default().max_payload_bytes + 1)
                .expect("test bound fits u64"),
        )
        .expect("sparse hub state grows");
        let store = CreatorHubStore { path };

        let error = store.load().expect_err("oversized hub state is rejected");
        assert!(error.to_string().contains("maximum"));
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn atomic_writer_does_not_reuse_a_predictable_temporary_path() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-atomic-write-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let path = parent.join("state.json");
        let legacy_temporary = suffixed_path(&path, ".tmp");
        fs::write(&legacy_temporary, b"do not replace").expect("legacy temporary writes");

        write_atomic(&path, b"accepted").expect("unique atomic write succeeds");

        assert_eq!(fs::read(&path).expect("destination reads"), b"accepted");
        assert_eq!(
            fs::read(&legacy_temporary).expect("legacy temporary reads"),
            b"do not replace"
        );
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn failed_creator_build_worker_start_is_typed_and_does_not_publish_a_build() {
        fn reject_worker(_: Box<dyn FnOnce() + Send>) -> io::Result<()> {
            Err(io::Error::other("test worker refusal"))
        }

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-build-start-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut workspace = CreatorWorkspace::open(&root).expect("created project opens");
        let status_before = workspace.status.clone();

        let error = workspace
            .start_build_with_spawner(reject_worker)
            .expect_err("worker start refusal must be typed");
        assert!(error
            .to_string()
            .contains("unable to start Creator build worker"));
        assert!(workspace.build.is_none());
        assert_eq!(workspace.status, status_before);
        assert!(!root
            .join("target/meridian-build/creator-alpha/creator-alpha-build-input.json")
            .exists());
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn repeated_model_actions_allocate_fresh_stable_source_ids() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-model-repeat-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let mut application =
            CreatorApplication::new(Some(&root), true).expect("public Creator workspace opens");
        let (initial_objects, model_path) = {
            let workspace = creator_workspace_for_smoke(&application).expect("workspace");
            (
                workspace.model_session.current().document().objects.len(),
                workspace.model_path.clone(),
            )
        };

        assert!(application
            .activate_workspace_action_for_smoke("model.create-primitive")
            .is_err());
        application
            .activate_workspace_action_for_smoke("workspace.modeler")
            .expect("Modeler workspace activates");
        application
            .activate_workspace_action_for_smoke("model.create-primitive")
            .expect("first primitive creates");
        application
            .activate_workspace_action_for_smoke("model.create-primitive")
            .expect("second primitive creates with fresh IDs");
        application
            .activate_workspace_action_for_smoke("model.split-edge")
            .expect("first edge split succeeds");
        application
            .activate_workspace_action_for_smoke("model.split-edge")
            .expect("second edge split succeeds with fresh IDs");

        let document = creator_workspace_for_smoke(&application)
            .expect("workspace")
            .model_session
            .current()
            .document();
        assert_eq!(document.objects.len(), initial_objects + 2);
        document
            .validate()
            .expect("repeated actions preserve topology and identity");
        let source = ModelDocument::read_source(model_path).expect("model source persists");
        assert_eq!(
            source.canonical_json().expect("source canonicalizes"),
            document.canonical_json().expect("session canonicalizes")
        );
        fs::remove_dir_all(parent).expect("temporary project removes");
    }

    #[test]
    fn create_project_copies_only_public_creator_source_and_opens_it() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("meridian-creator-create-{nonce}"));
        fs::create_dir(&parent).expect("temporary parent");
        let root = create_public_creator_project(&parent, "Public Creator Project")
            .expect("public project creates");
        let workspace = CreatorWorkspace::open(&root).expect("created project opens");
        assert_eq!(
            workspace.session.document().schema,
            meridian_editor_core::PROJECT_SCHEMA
        );
        assert_eq!(workspace.session.document().placements.len(), 1);
        assert_eq!(workspace.session.selection().ids.len(), 1);
        assert!(root.join(CREATOR_PROJECT_SOURCE).is_file());
        assert!(!root.join("game").exists());
        fs::remove_dir_all(parent).expect("temporary project removes");
    }
}
