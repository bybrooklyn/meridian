//! MS-01 Meridian application composition and qualification smoke.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use meridian_asset_tools::{
    decode_compiled_visual, AssetImportDatabase, CompiledVisualFacet, ImportedFixtureMesh,
};
use meridian_assets::{
    ArtifactHash, AssetId, AssetLoadRequest, AssetLoadResult, CancellationToken, PackIndexEntry,
    SourceId, UncompressedDecoder,
};
use meridian_benchmark::{has_multiple_pixel_values, write_capture_png};
use meridian_core::{FrameId, MonotonicNs, OperationId, RuntimeEpoch, StableId, TraceId};
use meridian_diagnostics::{
    DiagnosticEvent, DiagnosticSeverity, DiagnosticTimeline, RecoveryAction, RedactionClass,
};
use meridian_input::{
    Action, ButtonControl, InputActionMap, InputState, KeyCode, NativeInputEvent,
};
use meridian_package::{MountedPackage, PackageBuilder, PackageChunk, PackageLimits};
use meridian_platform::{
    run as run_platform, EventLoopMode, PlatformApplication, PlatformConfig, PlatformContext,
    PlatformEvent, PlatformEventEnvelope, RuntimeLifecycle, SurfaceSignal, WindowSize,
};
use meridian_renderer::{
    FoundationMeshDescriptor, MaterialHandle, MeshHandle, PenumbraFoundationRenderer,
    RenderInstanceId, RenderInstanceSource, Transform, UiOverlayRenderer,
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
    recovery_panel_document, runtime_overlay_document, DisplayList, SemanticDelta, UiDiagnostic,
    UiEvent, UiFrameInput, UiRuntime, UiSize,
};
use meridian_world::{CompiledWorldCell, SpatialDatabase};
use meridian_world_tools::compile_world_source;
use serde::Serialize;
use serde_json::json;

const MESH_SOURCE: &str = "assets_source/ms01/fixture_triangle.json";
const WORLD_SOURCE: &str = "assets_source/ms01/world_cell.json";
const DEFAULT_EVIDENCE: &str = "target/meridian-evidence/ms01";
const DEFAULT_CAPTURE: &str = "visible-source-frame.png";
const VISUAL_ASSET_NAME: &str = "fixtures/ms01/public-triangle.visual";
const COLLISION_ASSET_NAME: &str = "fixtures/ms01/public-triangle.collision";
const CELL_ASSET_NAME: &str = "fixtures/ms01/world-cell-0-0-0";
const SAVE_COMPONENT: &str = "meridian.ms01.position";
const EVIDENCE_CAPACITY: usize = 512;

type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    Interactive,
    Smoke,
    HeadlessSmoke,
    UiHeadlessSmoke,
    UiSmoke,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeridianOptions {
    pub mode: RunMode,
    pub project: Option<PathBuf>,
    pub capture: Option<PathBuf>,
    pub evidence: Option<PathBuf>,
    pub frames: u32,
}

impl Default for MeridianOptions {
    fn default() -> Self {
        Self {
            mode: RunMode::Interactive,
            project: None,
            capture: None,
            evidence: None,
            frames: 120,
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
                "--project" => options.project = Some(next_path(&mut arguments, "--project")?),
                "--capture" => options.capture = Some(next_path(&mut arguments, "--capture")?),
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
    ConflictingModes,
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
            Self::ConflictingModes => formatter.write_str("smoke modes are mutually exclusive"),
            Self::HelpRequested => formatter.write_str(usage()),
        }
    }
}

impl Error for MeridianArgumentError {}

#[must_use]
pub const fn usage() -> &'static str {
    "Meridian\n\nUsage: meridian [--smoke | --headless-smoke | --ui-headless-smoke | --ui-smoke] [--project PATH] [--capture PATH] [--evidence PATH] [--frames N]"
}

/// Runs the requested Meridian application mode.
///
/// # Errors
///
/// Returns source, package, streaming, save, platform, rendering, capture, or
/// evidence IO failures without claiming milestone completion.
pub fn run(options: &MeridianOptions) -> AppResult<()> {
    if options.mode == RunMode::UiHeadlessSmoke {
        return run_ui_headless_smoke();
    }
    if options.mode == RunMode::UiSmoke {
        return run_ui_native_smoke();
    }
    let project_root = resolve_project_root(options.project.as_deref())?;
    let evidence_root = resolve_output_path(
        &project_root,
        options.evidence.as_deref(),
        Path::new(DEFAULT_EVIDENCE),
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
        SemanticDelta::Unchanged => 0,
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

struct UiNativeSmokeApplication {
    runtime: UiRuntime,
    display_list: DisplayList,
    logical_viewport: UiSize,
    scale_factor: f32,
    rhi: Option<Rhi>,
    renderer: Option<UiOverlayRenderer>,
    structural_fallback_submitted: bool,
}

impl UiNativeSmokeApplication {
    fn new() -> AppResult<Self> {
        let document = recovery_panel_document()
            .map_err(|error| io::Error::other(format!("recovery UI fixture invalid: {error:?}")))?;
        let mut application = Self {
            runtime: UiRuntime::new(document),
            display_list: DisplayList::default(),
            logical_viewport: UiSize::new(960.0, 540.0),
            scale_factor: 1.0,
            rhi: None,
            renderer: None,
            structural_fallback_submitted: false,
        };
        application.refresh_display(WindowSize::new(960, 540), 1.0);
        Ok(application)
    }

    fn refresh_display(&mut self, physical_size: WindowSize, scale_factor: f64) {
        self.scale_factor = f64_to_f32(scale_factor).clamp(0.5, 4.0);
        self.logical_viewport = UiSize::new(
            f64_to_f32(f64::from(physical_size.width) / f64::from(self.scale_factor)),
            f64_to_f32(f64::from(physical_size.height) / f64::from(self.scale_factor)),
        );
        let mut input = UiFrameInput::new(self.logical_viewport);
        input.scale_factor = self.scale_factor;
        input.high_contrast = true;
        if self.display_list.primitives.is_empty() {
            input.events.push(UiEvent::FocusNext);
        }
        self.display_list = self.runtime.reconcile(input).display_list;
    }

    fn build_renderer(&self, rhi: &mut Rhi) -> AppResult<UiOverlayRenderer> {
        UiOverlayRenderer::new(
            rhi,
            &self.display_list,
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
                "Meridian UI native smoke submitted {} solid primitives and {} rasterized glyphs; {} text primitive(s) were incomplete",
                report.solid_primitives,
                report.rasterized_glyphs,
                report.incomplete_text_primitives
            );
            context.exit();
        } else if outcome.recoverable() {
            context.request_redraw();
        } else {
            self.submit_structural_fallback(context)?;
        }
        Ok(())
    }

    fn submit_structural_fallback(&mut self, context: &mut PlatformContext<'_>) -> AppResult<()> {
        if self.structural_fallback_submitted {
            context.exit();
            return Ok(());
        }
        let (Some(rhi), Some(renderer)) = (self.rhi.as_mut(), self.renderer.as_ref()) else {
            return Err(io::Error::other("UI structural fallback has no renderer").into());
        };
        renderer.submit_structural_validation(rhi, ClearColor::default())?;
        self.structural_fallback_submitted = true;
        println!(
            "Meridian UI native smoke surface unavailable; raster bridge submitted offscreen structural validation only"
        );
        context.exit();
        Ok(())
    }

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
            | PlatformEvent::Input(_)
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
    fn handle_event(&mut self, envelope: PlatformEventEnvelope, context: &mut PlatformContext<'_>) {
        let transition = self.lifecycle.observe_platform(envelope.event);
        self.prepared.runtime_epoch = transition.epoch;
        let mut platform_event = DiagnosticEvent::new(
            "RUN-PLATFORM-EVENT",
            "RUN",
            DiagnosticSeverity::Trace,
            "meridian-platform",
        )
        .correlated(self.prepared.operation_id, self.prepared.trace_id)
        .with_field("event", platform_event_name(envelope.event));
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

fn platform_event_name(event: PlatformEvent) -> &'static str {
    match event {
        PlatformEvent::Resumed => "resumed",
        PlatformEvent::Suspended => "suspended",
        PlatformEvent::WindowCreated { .. } => "window-created",
        PlatformEvent::Resized(_) => "resized",
        PlatformEvent::ScaleFactorChanged { .. } => "scale-factor-changed",
        PlatformEvent::Focused(true) => "focused",
        PlatformEvent::Focused(false) => "unfocused",
        PlatformEvent::Input(_) => "input",
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
    let temporary = suffixed_path(path, ".tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
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

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::time::{SystemTime, UNIX_EPOCH};

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
}
