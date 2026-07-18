//! Backend-neutral pipeline warm-up policy and runtime readiness checks.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

mod camera;
mod extraction;
mod foundation;
mod instance_buffer;
mod lighting;
mod mesh;
mod resources;
mod snapshot;
#[cfg(feature = "ui-direct")]
mod ui_direct;
#[cfg(feature = "ui-raster-bridge")]
mod ui_overlay;
mod upload;

pub use camera::{Camera, CameraError, Matrix4, PerspectiveProjection};
pub use extraction::{
    extract_render_instances, RenderExtractionFrame, RenderExtractionOutput, RenderInstanceSource,
};
pub use foundation::{
    FoundationMeshDescriptor, FoundationRendererError, PenumbraFoundationRenderer,
};
pub use instance_buffer::{
    GpuInstanceBuffer, GpuInstanceBufferError, InstanceBufferError, InstanceBufferWrite,
    InstanceUploadPlan, RenderInstanceBuffer, INSTANCE_STRIDE_BYTES,
};
pub use lighting::{
    CascadedShadowLayout, EnvironmentLight, LightingError, ShadowCascade, ShadowSettings, SunLight,
};
pub use mesh::{GpuMesh, GpuMeshError};
pub use resources::{
    MaterialResource, MeshResource, RenderResourceError, RenderResourceRegistry, TextureColorSpace,
    TextureResource,
};
pub use snapshot::{
    MaterialHandle, MeshHandle, RenderFlags, RenderInstance, RenderInstanceId, RenderSnapshot,
    RenderSnapshotBuilder, SnapshotError, TextureHandle, Transform,
};
#[cfg(feature = "ui-direct")]
pub use ui_direct::{
    UiDirectAtlas, UiDirectBatch, UiDirectBatchKind, UiDirectFrameDiagnostics, UiDirectFramePlan,
    UiDirectGpuFrame, UiDirectGpuRenderer, UiDirectImage, UiDirectMesh, UiDirectMeshVertex,
    UiDirectPrepareRequest, UiDirectPrimitiveKind, UiDirectRendererError, UiDirectRendererRecovery,
    UiDirectRendererRecoveryAction, UiDirectResourceSet,
};
#[cfg(feature = "ui-raster-bridge")]
pub use ui_overlay::{
    qualify_ui_display_list, UiOverlayRenderReport, UiOverlayRenderer, UiOverlayRendererError,
    UiPrimitiveKind, UiRasterBridgeRecovery, UiRasterBridgeRecoveryAction,
    UiRasterBridgeRecoveryState, UiRendererQualificationReport,
};
pub use upload::{
    RenderUploadBatch, RenderUploadError, RenderUploadOperation, RenderUploadTracker,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PipelineId(u64);

impl PipelineId {
    /// Creates a deterministic identifier from a pipeline's stable name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in name.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        Self(hash)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipelineKind {
    Render,
    Compute,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineSpec {
    pub id: PipelineId,
    pub name: String,
    pub kind: PipelineKind,
    pub required_for_runtime: bool,
}

impl PipelineSpec {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: PipelineKind, required_for_runtime: bool) -> Self {
        let name = name.into();
        Self {
            id: PipelineId::from_name(&name),
            name,
            kind,
            required_for_runtime,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WarmupPlan {
    specs: Vec<PipelineSpec>,
}

impl WarmupPlan {
    #[must_use]
    pub const fn new() -> Self {
        Self { specs: Vec::new() }
    }

    /// Adds a pipeline declaration. Duplicate IDs and names are rejected when the registry opens.
    #[must_use]
    pub fn with_pipeline(mut self, spec: PipelineSpec) -> Self {
        self.specs.push(spec);
        self
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

impl Default for WarmupPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistryPhase {
    Warmup,
    Runtime,
}

pub struct PipelineRegistry {
    specs: BTreeMap<PipelineId, PipelineSpec>,
    warmed: BTreeSet<PipelineId>,
    phase: RegistryPhase,
    startup_creation_events: u64,
    runtime_creation_attempts: u64,
}

impl PipelineRegistry {
    /// Opens a registry from a declared startup warm-up plan.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError`] when the plan contains duplicate IDs or names.
    pub fn new(plan: WarmupPlan) -> Result<Self, PipelineError> {
        let mut specs = BTreeMap::new();
        let mut names = BTreeSet::new();
        for spec in plan.specs {
            if !names.insert(spec.name.clone()) {
                return Err(PipelineError::DuplicateName(spec.name));
            }
            if specs.insert(spec.id, spec.clone()).is_some() {
                return Err(PipelineError::DuplicateId(spec.id));
            }
        }

        Ok(Self {
            specs,
            warmed: BTreeSet::new(),
            phase: RegistryPhase::Warmup,
            startup_creation_events: 0,
            runtime_creation_attempts: 0,
        })
    }

    /// Records successful creation of a declared pipeline during startup warm-up.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError`] if warm-up is closed, the ID is unknown, or
    /// the pipeline was already recorded as warmed.
    pub fn record_startup_creation(&mut self, id: PipelineId) -> Result<(), PipelineError> {
        self.require_phase(RegistryPhase::Warmup)?;
        self.require_spec(id)?;
        if !self.warmed.insert(id) {
            return Err(PipelineError::AlreadyWarmed(id));
        }
        self.startup_creation_events = self.startup_creation_events.saturating_add(1);
        Ok(())
    }

    /// Rejects attempts to construct a new pipeline after startup.
    ///
    /// # Errors
    ///
    /// Always returns [`PipelineError::RuntimeCreationForbidden`] and records
    /// the attempted runtime creation for diagnostics.
    pub fn record_runtime_creation_attempt(&mut self, id: PipelineId) -> Result<(), PipelineError> {
        self.runtime_creation_attempts = self.runtime_creation_attempts.saturating_add(1);
        Err(PipelineError::RuntimeCreationForbidden {
            id,
            pipeline_name: self
                .specs
                .get(&id)
                .map_or_else(|| "unknown".to_owned(), |spec| spec.name.clone()),
        })
    }

    /// Enters active runtime only when every required pipeline was warmed.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError::MissingRequired`] when one or more required
    /// pipelines were not warmed.
    pub fn enter_runtime(&mut self) -> Result<PipelineCacheReport, PipelineError> {
        let report = self.report();
        if !report.missing_required.is_empty() {
            return Err(PipelineError::MissingRequired(report.missing_required));
        }
        self.phase = RegistryPhase::Runtime;
        Ok(self.report())
    }

    /// Resolves an already-warmed pipeline for use by active rendering.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError`] if the ID is unknown or was not warmed.
    pub fn use_pipeline(&self, id: PipelineId) -> Result<PipelineHandle, PipelineError> {
        self.require_spec(id)?;
        if !self.warmed.contains(&id) {
            return Err(PipelineError::NotWarmed(id));
        }
        Ok(PipelineHandle { id })
    }

    #[must_use]
    pub fn report(&self) -> PipelineCacheReport {
        let missing_required = self
            .specs
            .values()
            .filter(|spec| spec.required_for_runtime && !self.warmed.contains(&spec.id))
            .map(|spec| spec.name.clone())
            .collect::<Vec<_>>();
        PipelineCacheReport {
            total_pipelines: self.specs.len(),
            required_pipelines: self
                .specs
                .values()
                .filter(|spec| spec.required_for_runtime)
                .count(),
            warmed_pipelines: self.warmed.len(),
            missing_required,
            startup_creation_events: self.startup_creation_events,
            runtime_creation_attempts: self.runtime_creation_attempts,
            runtime_ready: self.phase == RegistryPhase::Runtime,
        }
    }

    fn require_phase(&self, expected: RegistryPhase) -> Result<(), PipelineError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(PipelineError::WarmupClosed)
        }
    }

    fn require_spec(&self, id: PipelineId) -> Result<(), PipelineError> {
        if self.specs.contains_key(&id) {
            Ok(())
        } else {
            Err(PipelineError::UnknownPipeline(id))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PipelineHandle {
    id: PipelineId,
}

impl PipelineHandle {
    #[must_use]
    pub const fn id(self) -> PipelineId {
        self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineCacheReport {
    pub total_pipelines: usize,
    pub required_pipelines: usize,
    pub warmed_pipelines: usize,
    pub missing_required: Vec<String>,
    pub startup_creation_events: u64,
    pub runtime_creation_attempts: u64,
    pub runtime_ready: bool,
}

impl PipelineCacheReport {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.runtime_ready && self.missing_required.is_empty()
    }

    /// Converts the cache state into the engine-wide diagnostics contract.
    #[must_use]
    pub fn diagnostics(&self) -> meridian_diagnostics::PipelineDiagnostics {
        meridian_diagnostics::PipelineDiagnostics::new(
            u32::try_from(self.total_pipelines).unwrap_or(u32::MAX),
            u32::try_from(self.required_pipelines).unwrap_or(u32::MAX),
            u32::try_from(self.warmed_pipelines).unwrap_or(u32::MAX),
            self.startup_creation_events,
            self.runtime_creation_attempts,
            self.runtime_ready,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineError {
    DuplicateId(PipelineId),
    DuplicateName(String),
    UnknownPipeline(PipelineId),
    AlreadyWarmed(PipelineId),
    NotWarmed(PipelineId),
    MissingRequired(Vec<String>),
    WarmupClosed,
    RuntimeCreationForbidden {
        id: PipelineId,
        pipeline_name: String,
    },
}

impl Display for PipelineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "duplicate pipeline ID: {}", id.value()),
            Self::DuplicateName(name) => write!(formatter, "duplicate pipeline name: {name}"),
            Self::UnknownPipeline(id) => write!(formatter, "unknown pipeline ID: {}", id.value()),
            Self::AlreadyWarmed(id) => {
                write!(formatter, "pipeline {} was warmed twice", id.value())
            }
            Self::NotWarmed(id) => write!(formatter, "pipeline {} was not warmed", id.value()),
            Self::MissingRequired(names) => {
                write!(
                    formatter,
                    "required pipelines missing before runtime: {names:?}"
                )
            }
            Self::WarmupClosed => write!(formatter, "pipeline warm-up is already closed"),
            Self::RuntimeCreationForbidden { pipeline_name, .. } => write!(
                formatter,
                "runtime pipeline creation is forbidden: {pipeline_name}"
            ),
        }
    }
}

impl Error for PipelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> WarmupPlan {
        WarmupPlan::new()
            .with_pipeline(PipelineSpec::new("clear", PipelineKind::Render, true))
            .with_pipeline(PipelineSpec::new("cull", PipelineKind::Compute, false))
    }

    #[test]
    fn warmup_report_requires_all_required_pipelines() {
        let clear = PipelineId::from_name("clear");
        let mut registry = PipelineRegistry::new(plan()).expect("plan is valid");

        let error = registry
            .enter_runtime()
            .expect_err("missing clear pipeline must block runtime");

        assert_eq!(
            error,
            PipelineError::MissingRequired(vec!["clear".to_owned()])
        );
        registry
            .record_startup_creation(clear)
            .expect("clear can be warmed during startup");
        let report = registry
            .enter_runtime()
            .expect("all required pipelines are now warmed");
        assert!(report.is_ready());
        assert_eq!(report.warmed_pipelines, 1);
        assert_eq!(report.startup_creation_events, 1);
        assert_eq!(
            report.diagnostics(),
            meridian_diagnostics::PipelineDiagnostics::new(2, 1, 1, 1, 0, true)
        );
    }

    #[test]
    fn runtime_creation_is_rejected_and_counted() {
        let mut registry = PipelineRegistry::new(plan()).expect("plan is valid");
        let clear = PipelineId::from_name("clear");
        registry
            .record_startup_creation(clear)
            .expect("clear can be warmed during startup");
        registry.enter_runtime().expect("runtime can start");

        let error = registry
            .record_runtime_creation_attempt(PipelineId::from_name("cull"))
            .expect_err("runtime creation must be forbidden");

        assert!(matches!(
            error,
            PipelineError::RuntimeCreationForbidden { pipeline_name, .. }
                if pipeline_name == "cull"
        ));
        assert_eq!(registry.report().runtime_creation_attempts, 1);
    }

    #[test]
    fn warmed_pipeline_can_be_used_after_runtime_entry() {
        let clear = PipelineId::from_name("clear");
        let mut registry = PipelineRegistry::new(plan()).expect("plan is valid");
        registry
            .record_startup_creation(clear)
            .expect("clear can be warmed during startup");
        registry.enter_runtime().expect("runtime can start");

        let handle = registry
            .use_pipeline(clear)
            .expect("warmed pipeline is available");
        assert_eq!(handle.id(), clear);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let plan = WarmupPlan::new()
            .with_pipeline(PipelineSpec::new("same", PipelineKind::Render, true))
            .with_pipeline(PipelineSpec::new("same", PipelineKind::Compute, false));

        assert!(matches!(
            PipelineRegistry::new(plan),
            Err(PipelineError::DuplicateName(name)) if name == "same"
        ));
    }
}
