//! Backend-neutral GPU adapter, device, surface, and clear-frame boundary.

use std::borrow::Cow;
use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use meridian_core::FrameId;
use meridian_platform::{PlatformWindow, WindowSize};
use meridian_render_graph::{
    CompiledRenderGraph, RenderGraphBuilder, ResourceDescriptor, ResourceKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    Noop,
    Vulkan,
    Metal,
    Direct3D12,
    OpenGl,
    BrowserWebGpu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterKind {
    Discrete,
    Integrated,
    Virtual,
    Software,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryClass {
    Discrete,
    Unified,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerPreference {
    HighPerformance,
    LowPower,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentPolicy {
    Vsync,
    AllowTearing,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GpuFeature {
    IndirectDrawCount,
    MeshShaders,
    SubgroupOperations,
    TextureAtomics,
    RayQueries,
    RayTracingPipelines,
    BindlessTextures,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityStatus {
    Unsupported,
    Supported,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl ClearColor {
    #[must_use]
    pub const fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Default for ClearColor {
    fn default() -> Self {
        Self::new(0.008, 0.012, 0.020, 1.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RhiConfig {
    pub power_preference: PowerPreference,
    pub preferred_backend: Option<Backend>,
    pub allow_software_adapter: bool,
    pub present_policy: PresentPolicy,
    pub desired_maximum_frame_latency: u32,
    pub enable_timestamps: bool,
}

impl Default for RhiConfig {
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::HighPerformance,
            preferred_backend: None,
            allow_software_adapter: false,
            present_policy: PresentPolicy::Vsync,
            desired_maximum_frame_latency: 2,
            enable_timestamps: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuCapabilities {
    pub adapter_name: String,
    pub driver: String,
    pub driver_info: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend: Backend,
    pub adapter_kind: AdapterKind,
    pub memory_class: MemoryClass,
    pub features: BTreeSet<GpuFeature>,
    pub timestamp_queries: CapabilityStatus,
    pub max_sampled_textures_per_shader_stage: u32,
    pub hdr_surface_formats: CapabilityStatus,
}

impl GpuCapabilities {
    #[must_use]
    pub fn supports(&self, feature: GpuFeature) -> bool {
        self.features.contains(&feature)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceFormat {
    pub name: String,
    pub srgb: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameOutcome {
    Presented,
    PresentedSuboptimal,
    SkippedZeroSize,
    SkippedTimeout,
    SkippedOccluded,
    ReconfiguredOutdated,
    RecreatedLostSurface,
    DeviceLost,
    UnsupportedSurface,
}

impl FrameOutcome {
    #[must_use]
    pub const fn submitted(self) -> bool {
        matches!(self, Self::Presented | Self::PresentedSuboptimal)
    }

    #[must_use]
    pub const fn visible(self) -> bool {
        matches!(self, Self::Presented | Self::PresentedSuboptimal)
    }

    #[must_use]
    pub const fn recoverable(self) -> bool {
        matches!(
            self,
            Self::SkippedTimeout | Self::ReconfiguredOutdated | Self::RecreatedLostSurface
        )
    }

    #[must_use]
    pub const fn skipped(self) -> bool {
        matches!(
            self,
            Self::SkippedZeroSize
                | Self::SkippedTimeout
                | Self::SkippedOccluded
                | Self::ReconfiguredOutdated
                | Self::RecreatedLostSurface
                | Self::DeviceLost
                | Self::UnsupportedSurface
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CaptureId(u64);

impl CaptureId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureRequest {
    pub frame_id: FrameId,
    pub max_width: u32,
    pub max_height: u32,
    pub max_bytes: u64,
}

impl CaptureRequest {
    #[must_use]
    pub const fn new(frame_id: FrameId, max_width: u32, max_height: u32, max_bytes: u64) -> Self {
        Self {
            frame_id,
            max_width,
            max_height,
            max_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureSource {
    PresentedSurface,
    Offscreen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapturedPixelFormat {
    Rgba8Srgb,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedFrame {
    pub capture_id: CaptureId,
    pub frame_id: FrameId,
    pub width: u32,
    pub height: u32,
    pub format: CapturedPixelFormat,
    pub source: CaptureSource,
    pub surface_outcome: Option<FrameOutcome>,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureFailure {
    ZeroExtent,
    DimensionLimit,
    ByteLimit,
    SizeOverflow,
    UnsupportedFormat,
    SurfaceCopyUnsupported,
    ReadbackSaturated,
    MappingFailed,
    StaleReadback,
    InvalidRowData,
    DeviceLost,
}

impl Display for CaptureFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "capture failed: {self:?}")
    }
}

impl Error for CaptureFailure {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureOutcome {
    Captured(CapturedFrame),
    UnsupportedCapability {
        capture_id: CaptureId,
        frame_id: FrameId,
        failure: CaptureFailure,
    },
    Inconclusive {
        capture_id: CaptureId,
        frame_id: FrameId,
        failure: CaptureFailure,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureDiagnostics {
    pub readback_capacity: usize,
    pub readbacks_in_flight: usize,
    pub pending_requests: usize,
    pub queued_results: usize,
    pub dropped_results: u64,
}

/// Correlates all pass timings recorded for one renderer frame.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TimingFrameId(u64);

impl TimingFrameId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable, allocation-free label for a timed renderer pass.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PassTimingLabel(&'static str);

impl PassTimingLabel {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self(label)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuTimingFailure {
    ZeroTimestamp,
    ZeroDuration,
    EndBeforeBegin,
    InvalidTimestampPeriod,
    DurationOutOfRange,
    MappingFailed,
    StaleReadback,
    ReadbackSaturated,
    DeviceLost,
    MetalTimestampDataInvalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuTimingOutcome {
    Measured(Duration),
    NotRequested,
    UnsupportedCapability,
    UnsupportedPlatform(GpuTimingFailure),
    Inconclusive(GpuTimingFailure),
}

impl GpuTimingOutcome {
    #[must_use]
    pub const fn measured(self) -> Option<Duration> {
        match self {
            Self::Measured(duration) => Some(duration),
            Self::NotRequested
            | Self::UnsupportedCapability
            | Self::UnsupportedPlatform(_)
            | Self::Inconclusive(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PassTimingSample {
    pub frame_id: TimingFrameId,
    pub runtime_frame_id: Option<FrameId>,
    pub submission_id: u64,
    pub pass: PassTimingLabel,
    pub cpu_encode_time: Duration,
    pub gpu: GpuTimingOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingAvailability {
    Available,
    NotRequested,
    UnsupportedCapability,
    UnsupportedPlatform(GpuTimingFailure),
    Inconclusive(GpuTimingFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimingDiagnostics {
    pub availability: TimingAvailability,
    pub readback_capacity: usize,
    pub readbacks_in_flight: usize,
    pub queued_results: usize,
    pub dropped_results: u64,
}

/// Backend-neutral usage class for a GPU buffer allocated by the RHI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
    Indirect,
}

/// Backend-neutral depth attachment formats supported by the initial raster path.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DepthFormat {
    /// 32-bit floating-point depth without stencil.
    #[default]
    Depth32Float,
    /// Implementation-selected 24-bit depth without stencil.
    Depth24Plus,
    /// Implementation-selected 24-bit depth with 8-bit stencil.
    Depth24PlusStencil8,
}

impl DepthFormat {
    #[must_use]
    pub const fn has_stencil(self) -> bool {
        matches!(self, Self::Depth24PlusStencil8)
    }

    fn wgpu_format(self) -> wgpu::TextureFormat {
        match self {
            Self::Depth32Float => wgpu::TextureFormat::Depth32Float,
            Self::Depth24Plus => wgpu::TextureFormat::Depth24Plus,
            Self::Depth24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        }
    }
}

/// Backend-neutral formats supported by the initial material-texture path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba16Float,
}

impl TextureFormat {
    #[must_use]
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::Rgba8UnormSrgb)
    }

    const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb => 4,
            Self::Rgba16Float => 8,
        }
    }

    const fn wgpu_format(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        }
    }
}

/// Backend-neutral vertex attribute formats supported by the initial mesh
/// submission path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
}

impl VertexFormat {
    const fn byte_size(self) -> u64 {
        match self {
            Self::Float32x2 => 8,
            Self::Float32x3 => 12,
            Self::Float32x4 => 16,
        }
    }

    const fn wgpu_format(self) -> wgpu::VertexFormat {
        match self {
            Self::Float32x2 => wgpu::VertexFormat::Float32x2,
            Self::Float32x3 => wgpu::VertexFormat::Float32x3,
            Self::Float32x4 => wgpu::VertexFormat::Float32x4,
        }
    }
}

/// One shader-location mapping in a vertex layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexAttribute {
    format: VertexFormat,
    offset: u64,
    shader_location: u32,
}

impl VertexAttribute {
    #[must_use]
    pub const fn new(format: VertexFormat, offset: u64, shader_location: u32) -> Self {
        Self {
            format,
            offset,
            shader_location,
        }
    }

    #[must_use]
    pub const fn format(self) -> VertexFormat {
        self.format
    }

    #[must_use]
    pub const fn offset(self) -> u64 {
        self.offset
    }

    #[must_use]
    pub const fn shader_location(self) -> u32 {
        self.shader_location
    }
}

/// Validated, backend-neutral layout for one vertex buffer stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexLayout {
    stride: u64,
    attributes: Vec<VertexAttribute>,
}

impl VertexLayout {
    /// Creates a vertex layout and validates that attributes fit its stride.
    ///
    /// # Errors
    ///
    /// Returns [`VertexLayoutError`] for a zero or misaligned stride,
    /// overlapping shader locations, or an attribute outside the stride.
    pub fn new(
        stride: u64,
        attributes: impl Into<Vec<VertexAttribute>>,
    ) -> Result<Self, VertexLayoutError> {
        let attributes = attributes.into();
        if stride == 0 || !stride.is_multiple_of(4) {
            return Err(VertexLayoutError::InvalidStride(stride));
        }
        let mut locations = BTreeSet::new();
        for attribute in &attributes {
            if !locations.insert(attribute.shader_location) {
                return Err(VertexLayoutError::DuplicateShaderLocation(
                    attribute.shader_location,
                ));
            }
            let end = attribute
                .offset
                .checked_add(attribute.format.byte_size())
                .ok_or(VertexLayoutError::AttributeOutsideStride {
                    offset: attribute.offset,
                    size: attribute.format.byte_size(),
                    stride,
                })?;
            if end > stride || !attribute.offset.is_multiple_of(4) {
                return Err(VertexLayoutError::AttributeOutsideStride {
                    offset: attribute.offset,
                    size: attribute.format.byte_size(),
                    stride,
                });
            }
        }
        Ok(Self { stride, attributes })
    }

    #[must_use]
    pub const fn stride(&self) -> u64 {
        self.stride
    }

    #[must_use]
    pub fn attributes(&self) -> &[VertexAttribute] {
        &self.attributes
    }
}

/// Invalid backend-neutral vertex layout metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexLayoutError {
    InvalidStride(u64),
    DuplicateShaderLocation(u32),
    AttributeOutsideStride { offset: u64, size: u64, stride: u64 },
}

impl Display for VertexLayoutError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStride(stride) => write!(formatter, "invalid vertex stride: {stride}"),
            Self::DuplicateShaderLocation(location) => {
                write!(formatter, "duplicate vertex shader location: {location}")
            }
            Self::AttributeOutsideStride {
                offset,
                size,
                stride,
            } => write!(
                formatter,
                "vertex attribute {offset}..{} exceeds stride {stride}",
                offset.saturating_add(*size)
            ),
        }
    }
}

impl Error for VertexLayoutError {}

/// Device-owned GPU buffer whose backend handle remains private to the RHI.
pub struct GpuBuffer {
    buffer: wgpu::Buffer,
    size: u64,
    usage: BufferUsage,
}

/// Device-owned render pipeline whose backend handle remains private to the RHI.
pub struct GpuRenderPipeline {
    pipeline: wgpu::RenderPipeline,
}

/// Device-owned sampled 2D texture whose backend resources remain private.
pub struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: WindowSize,
    mip_level_count: u32,
    format: TextureFormat,
}

/// Device-owned cube texture and sampler containing pre-convolved diffuse
/// environment irradiance.
pub struct GpuEnvironmentMap {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    face_size: u32,
    mip_level_count: u32,
    format: TextureFormat,
}

impl GpuEnvironmentMap {
    #[must_use]
    pub const fn face_size(&self) -> u32 {
        self.face_size
    }

    #[must_use]
    pub const fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }
}

/// Device-owned texture/sampler binding for the initial material contract.
pub struct GpuTextureBindGroup {
    bind_group: wgpu::BindGroup,
}

/// Device-owned cascaded depth texture and comparison sampler for sun shadows.
pub struct GpuShadowMap {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    resolution: u32,
    cascade_count: u8,
}

impl GpuShadowMap {
    #[must_use]
    pub const fn resolution(&self) -> u32 {
        self.resolution
    }

    #[must_use]
    pub const fn cascade_count(&self) -> u8 {
        self.cascade_count
    }
}

/// Resources consumed by the direct PBR lighting bind group.
pub struct PbrLightingResources<'a> {
    lighting_buffer: &'a GpuBuffer,
    shadow_map: &'a GpuShadowMap,
    shadow_parameters: &'a GpuBuffer,
    environment_map: &'a GpuEnvironmentMap,
    environment_parameters: &'a GpuBuffer,
}

impl<'a> PbrLightingResources<'a> {
    #[must_use]
    pub const fn new(
        lighting_buffer: &'a GpuBuffer,
        shadow_map: &'a GpuShadowMap,
        shadow_parameters: &'a GpuBuffer,
        environment_map: &'a GpuEnvironmentMap,
        environment_parameters: &'a GpuBuffer,
    ) -> Self {
        Self {
            lighting_buffer,
            shadow_map,
            shadow_parameters,
            environment_map,
            environment_parameters,
        }
    }
}

/// Device-owned base-color, normal, and metallic-roughness texture binding.
pub struct GpuMaterialTextureBindGroup {
    bind_group: wgpu::BindGroup,
}

/// Device-owned camera/object uniform binding for the direct PBR material path.
pub struct GpuUniformBindGroup {
    bind_group: wgpu::BindGroup,
}

/// Device-owned base-color/metallic/roughness uniform binding.
pub struct GpuMaterialParameterBindGroup {
    bind_group: wgpu::BindGroup,
}

/// Device-owned sun-light uniform binding for the initial diffuse path.
pub struct GpuLightingBindGroup {
    bind_group: wgpu::BindGroup,
}

/// Material texture, parameter, lighting, and camera/object bindings for one draw.
pub struct GpuMaterialBindings {
    textures: GpuMaterialTextureBindGroup,
    uniforms: GpuUniformBindGroup,
    parameters: GpuMaterialParameterBindGroup,
    lighting: GpuLightingBindGroup,
}

impl GpuMaterialBindings {
    #[must_use]
    pub fn new(
        textures: GpuMaterialTextureBindGroup,
        uniforms: GpuUniformBindGroup,
        parameters: GpuMaterialParameterBindGroup,
        lighting: GpuLightingBindGroup,
    ) -> Self {
        Self {
            textures,
            uniforms,
            parameters,
            lighting,
        }
    }
}

#[derive(Clone, Copy)]
struct IndexedDraw<'a> {
    pipeline: &'a GpuRenderPipeline,
    vertex_buffer: &'a GpuBuffer,
    index_buffer: &'a GpuBuffer,
    index_count: u32,
    texture_bind_group: Option<&'a GpuTextureBindGroup>,
    material_texture_bind_group: Option<&'a GpuMaterialTextureBindGroup>,
    uniform_bind_group: Option<&'a GpuUniformBindGroup>,
    material_parameter_bind_group: Option<&'a GpuMaterialParameterBindGroup>,
    lighting_bind_group: Option<&'a GpuLightingBindGroup>,
}

/// Inputs for one indexed cascade shadow-depth submission.
pub struct ShadowDepthDraw<'a> {
    pub pipeline: &'a GpuRenderPipeline,
    pub shadow_map: &'a GpuShadowMap,
    pub cascade_index: u8,
    pub vertex_buffer: &'a GpuBuffer,
    pub index_buffer: &'a GpuBuffer,
    pub index_count: u32,
    pub uniform_bind_group: &'a GpuUniformBindGroup,
}

pub struct OffscreenIndexedCaptureDraw<'a> {
    pub pipeline: &'a GpuRenderPipeline,
    pub vertex_buffer: &'a GpuBuffer,
    pub index_buffer: &'a GpuBuffer,
    pub index_count: u32,
    pub material_bindings: &'a GpuMaterialBindings,
    pub color: ClearColor,
    pub size: WindowSize,
}

impl GpuBuffer {
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[must_use]
    pub const fn usage(&self) -> BufferUsage {
        self.usage
    }
}

/// Device-owned depth attachment whose backend texture and view remain private.
pub struct DepthBuffer {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: WindowSize,
    format: DepthFormat,
}

impl DepthBuffer {
    #[must_use]
    pub const fn size(&self) -> WindowSize {
        self.size
    }

    #[must_use]
    pub const fn format(&self) -> DepthFormat {
        self.format
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceLossReason {
    Unknown,
    Destroyed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceLoss {
    pub reason: DeviceLossReason,
    pub message: String,
}

struct TimestampQueryState {
    slots: Vec<TimestampReadbackSlot>,
    timestamp_period_ns: f32,
    sender: mpsc::Sender<TimestampReadbackMessage>,
    receiver: mpsc::Receiver<TimestampReadbackMessage>,
}

struct TimestampReadbackSlot {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    generation: u64,
    in_flight: Option<TimestampCorrelation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TimestampCorrelation {
    generation: u64,
    frame_id: TimingFrameId,
    runtime_frame_id: Option<FrameId>,
    submission_id: u64,
    pass: PassTimingLabel,
    cpu_encode_time: Duration,
}

struct TimestampReadbackMessage {
    slot_index: usize,
    correlation: TimestampCorrelation,
    result: TimestampReadbackResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimestampReadbackResult {
    Timestamps { begin: u64, end: u64 },
    MappingFailed,
}

#[derive(Clone, Copy)]
struct PendingPassTiming {
    frame_id: TimingFrameId,
    runtime_frame_id: Option<FrameId>,
    submission_id: u64,
    pass: PassTimingLabel,
    gpu: PendingGpuTiming,
}

#[derive(Clone, Copy)]
enum PendingGpuTiming {
    Readback { slot_index: usize, generation: u64 },
    Final(GpuTimingOutcome),
}

struct CaptureState {
    slots: Vec<CaptureReadbackSlot>,
    sender: mpsc::Sender<CaptureReadbackMessage>,
    receiver: mpsc::Receiver<CaptureReadbackMessage>,
    pending: VecDeque<PendingCaptureRequest>,
    results: VecDeque<CaptureOutcome>,
    dropped_results: u64,
}

struct CaptureReadbackSlot {
    buffer: Option<wgpu::Buffer>,
    buffer_size: u64,
    generation: u64,
    in_flight: Option<CaptureCorrelation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingCaptureRequest {
    id: CaptureId,
    request: CaptureRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureCorrelation {
    generation: u64,
    capture_id: CaptureId,
    frame_id: FrameId,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    source_format: CaptureSourceFormat,
    source: CaptureSource,
    surface_outcome: Option<FrameOutcome>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureSourceFormat {
    Rgba8Srgb,
    Bgra8Srgb,
}

struct CaptureReadbackMessage {
    slot_index: usize,
    correlation: CaptureCorrelation,
    result: CaptureReadbackResult,
}

enum CaptureReadbackResult {
    Bytes(Vec<u8>),
    MappingFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingCapture {
    slot_index: usize,
    generation: u64,
    correlation: CaptureCorrelation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureLayout {
    padded_bytes_per_row: u32,
    buffer_size: u64,
}

const TIMESTAMP_READBACK_SLOT_COUNT: usize = 8;
const TIMING_RESULT_CAPACITY: usize = 64;
const CAPTURE_READBACK_SLOT_COUNT: usize = 3;
const CAPTURE_RESULT_CAPACITY: usize = 16;
const CAPTURE_REQUEST_CAPACITY: usize = 3;
const CLEAR_PASS_LABEL: PassTimingLabel = PassTimingLabel::new("clear");
const BOOTSTRAP_PIPELINE_PASS_LABEL: PassTimingLabel = PassTimingLabel::new("bootstrap_pipeline");
const SHADOW_DEPTH_PASS_LABEL: PassTimingLabel = PassTimingLabel::new("shadow_depth");
const INDEXED_MESH_PASS_LABEL: PassTimingLabel = PassTimingLabel::new("indexed_mesh");

pub struct Rhi {
    instance: wgpu::Instance,
    window: PlatformWindow,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: WindowSize,
    configured: bool,
    capabilities: GpuCapabilities,
    clear_graph: CompiledRenderGraph,
    depth_buffer: Option<DepthBuffer>,
    timestamp_query: Option<TimestampQueryState>,
    timing_availability: TimingAvailability,
    timing_results: VecDeque<PassTimingSample>,
    dropped_timing_results: u64,
    next_timing_frame_id: u64,
    active_timing_frame: Option<TimingFrameId>,
    active_runtime_frame: Option<FrameId>,
    next_submission_id: u64,
    latest_measured_gpu_duration: Option<Duration>,
    device_loss: Arc<Mutex<Option<DeviceLoss>>>,
    surface_copy_supported: bool,
    capture: CaptureState,
    next_capture_id: u64,
}

impl Rhi {
    /// Creates a GPU instance, selects a presentation-capable adapter, and opens a device.
    ///
    /// # Errors
    ///
    /// Returns [`RhiError`] when surface creation, adapter selection, device
    /// creation, surface configuration, or render-graph compilation fails.
    pub fn new(window: PlatformWindow, config: RhiConfig) -> Result<Self, RhiError> {
        pollster::block_on(Self::new_async(window, config))
    }

    async fn new_async(window: PlatformWindow, config: RhiConfig) -> Result<Self, RhiError> {
        let instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        let enabled_backends = instance_descriptor.backends;
        let instance = wgpu::Instance::new(instance_descriptor);
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| RhiError::new(RhiErrorKind::SurfaceCreation, error.to_string()))?;

        let adapter = select_adapter(&instance, &surface, enabled_backends, config).await?;
        let adapter_features = adapter.features();
        let required_features = if config.enable_timestamps
            && adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY)
        {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };
        let timing_availability = if !config.enable_timestamps {
            TimingAvailability::NotRequested
        } else if required_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            TimingAvailability::Available
        } else {
            TimingAvailability::UnsupportedCapability
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Meridian primary device"),
                required_features,
                ..Default::default()
            })
            .await
            .map_err(|error| RhiError::new(RhiErrorKind::DeviceCreation, error.to_string()))?;

        let device_loss = Arc::new(Mutex::new(None));
        let callback_state = Arc::clone(&device_loss);
        device.set_device_lost_callback(move |reason, message| {
            let reason = match reason {
                wgpu::DeviceLostReason::Destroyed => DeviceLossReason::Destroyed,
                wgpu::DeviceLostReason::Unknown => DeviceLossReason::Unknown,
            };
            *callback_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some(DeviceLoss { reason, message });
        });

        let size = window.size();
        let surface_capabilities = surface.get_capabilities(&adapter);
        let capabilities = gpu_capabilities(
            &adapter,
            &surface_capabilities,
            required_features.contains(wgpu::Features::TIMESTAMP_QUERY),
        );
        let surface_config =
            make_surface_config(&surface, &adapter, size, &surface_capabilities, config)?;
        let surface_copy_supported = surface_capabilities
            .usages
            .contains(wgpu::TextureUsages::COPY_SRC);
        let configured = !size.is_zero();
        if configured {
            surface.configure(&device, &surface_config);
        }
        let depth_buffer = configured.then(|| {
            create_depth_buffer_for_device(&device, size, DepthFormat::default(), "Meridian depth")
        });
        let timestamp_query = required_features
            .contains(wgpu::Features::TIMESTAMP_QUERY)
            .then(|| create_timestamp_query_state(&device, queue.get_timestamp_period()));

        Ok(Self {
            instance,
            window,
            surface,
            adapter,
            device,
            queue,
            surface_config,
            size,
            configured,
            capabilities,
            clear_graph: build_clear_graph()?,
            depth_buffer,
            timestamp_query,
            timing_availability,
            timing_results: VecDeque::with_capacity(TIMING_RESULT_CAPACITY),
            dropped_timing_results: 0,
            next_timing_frame_id: 1,
            active_timing_frame: None,
            active_runtime_frame: None,
            next_submission_id: 1,
            latest_measured_gpu_duration: None,
            device_loss,
            surface_copy_supported,
            capture: create_capture_state(),
            next_capture_id: 1,
        })
    }

    #[must_use]
    pub const fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    /// Opens an explicit timing frame so separately submitted passes share one
    /// correlation identifier. Calls made without an explicit frame receive a
    /// one-submission automatic frame identifier.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::TimingFrameState`] when another explicit timing
    /// frame is already active.
    pub fn begin_timing_frame(&mut self) -> Result<TimingFrameId, RhiError> {
        self.begin_timing_frame_internal(None)
    }

    /// Opens a timing frame correlated to the shared runtime frame identity.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::TimingFrameState`] if another scope is active.
    pub fn begin_timing_frame_for(
        &mut self,
        runtime_frame_id: FrameId,
    ) -> Result<TimingFrameId, RhiError> {
        self.begin_timing_frame_internal(Some(runtime_frame_id))
    }

    fn begin_timing_frame_internal(
        &mut self,
        runtime_frame_id: Option<FrameId>,
    ) -> Result<TimingFrameId, RhiError> {
        if let Some(active) = self.active_timing_frame {
            return Err(RhiError::new(
                RhiErrorKind::TimingFrameState,
                format!("timing frame {} is already active", active.get()),
            ));
        }
        let frame_id = self.allocate_timing_frame_id();
        self.active_timing_frame = Some(frame_id);
        self.active_runtime_frame = runtime_frame_id;
        Ok(frame_id)
    }

    /// Closes the matching explicit timing frame.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::TimingFrameState`] when no frame is active or
    /// `frame_id` does not identify the active frame.
    pub fn end_timing_frame(&mut self, frame_id: TimingFrameId) -> Result<(), RhiError> {
        match self.active_timing_frame {
            Some(active) if active == frame_id => {
                self.active_timing_frame = None;
                self.active_runtime_frame = None;
                Ok(())
            }
            Some(active) => Err(RhiError::new(
                RhiErrorKind::TimingFrameState,
                format!(
                    "cannot end timing frame {}; frame {} is active",
                    frame_id.get(),
                    active.get()
                ),
            )),
            None => Err(RhiError::new(
                RhiErrorKind::TimingFrameState,
                format!("timing frame {} is not active", frame_id.get()),
            )),
        }
    }

    /// Advances timestamp mapping callbacks without waiting for GPU work.
    pub fn poll_pass_timings(&mut self) {
        let poll_failed = self.device.poll(wgpu::PollType::Poll).is_err();
        if self.device_loss().is_some() {
            self.timing_availability =
                TimingAvailability::Inconclusive(GpuTimingFailure::DeviceLost);
            self.fail_all_timing_readbacks(GpuTimingFailure::DeviceLost);
        } else if poll_failed {
            self.fail_all_timing_readbacks(GpuTimingFailure::MappingFailed);
        }
        self.collect_timing_readbacks();
        if self.device_loss().is_some() {
            self.timing_availability =
                TimingAvailability::Inconclusive(GpuTimingFailure::DeviceLost);
            self.fail_all_timing_readbacks(GpuTimingFailure::DeviceLost);
        }
    }

    /// Returns one finalized pass timing, if available, without waiting.
    pub fn take_pass_timing(&mut self) -> Option<PassTimingSample> {
        self.poll_pass_timings();
        self.timing_results.pop_front()
    }

    #[must_use]
    pub fn timing_diagnostics(&self) -> TimingDiagnostics {
        TimingDiagnostics {
            availability: self.timing_availability,
            readback_capacity: self
                .timestamp_query
                .as_ref()
                .map_or(0, |state| state.slots.len()),
            readbacks_in_flight: self.timestamp_query.as_ref().map_or(0, |state| {
                state
                    .slots
                    .iter()
                    .filter(|slot| slot.in_flight.is_some())
                    .count()
            }),
            queued_results: self.timing_results.len(),
            dropped_results: self.dropped_timing_results,
        }
    }

    /// Requests capture of the next compatible draw. No capture work occurs otherwise.
    ///
    /// # Errors
    ///
    /// Returns saturation or invalid-limit failures without submitting GPU work.
    pub fn request_capture(
        &mut self,
        request: CaptureRequest,
    ) -> Result<CaptureId, CaptureFailure> {
        if request.max_width == 0 || request.max_height == 0 || request.max_bytes == 0 {
            return Err(CaptureFailure::ZeroExtent);
        }
        if self.capture.pending.len() >= CAPTURE_REQUEST_CAPACITY {
            return Err(CaptureFailure::ReadbackSaturated);
        }
        let id = CaptureId(self.next_capture_id);
        self.next_capture_id = self.next_capture_id.wrapping_add(1).max(1);
        self.capture
            .pending
            .push_back(PendingCaptureRequest { id, request });
        Ok(id)
    }

    /// Advances capture callbacks without waiting for GPU completion.
    pub fn poll_captures(&mut self) {
        let poll_failed = self.device.poll(wgpu::PollType::Poll).is_err();
        if self.device_loss().is_some() {
            self.fail_all_capture_readbacks(CaptureFailure::DeviceLost);
        } else if poll_failed {
            self.fail_all_capture_readbacks(CaptureFailure::MappingFailed);
        }
        self.collect_capture_readbacks();
    }

    /// Returns one capture outcome without waiting.
    pub fn take_capture(&mut self) -> Option<CaptureOutcome> {
        self.poll_captures();
        self.capture.results.pop_front()
    }

    #[must_use]
    pub fn capture_diagnostics(&self) -> CaptureDiagnostics {
        CaptureDiagnostics {
            readback_capacity: self.capture.slots.len(),
            readbacks_in_flight: self
                .capture
                .slots
                .iter()
                .filter(|slot| slot.in_flight.is_some())
                .count(),
            pending_requests: self.capture.pending.len(),
            queued_results: self.capture.results.len(),
            dropped_results: self.capture.dropped_results,
        }
    }

    /// Compatibility shim for the former blocking frame-duration API.
    ///
    /// This method no longer waits. New code should use [`Self::take_pass_timing`]
    /// so unavailable outcomes remain explicit.
    ///
    /// # Errors
    ///
    /// Retains the former `Result` signature for source compatibility. The
    /// nonblocking compatibility implementation currently returns `Ok` only.
    #[deprecated(note = "use poll_pass_timings and take_pass_timing for typed outcomes")]
    pub fn take_last_gpu_duration(&mut self) -> Result<Option<Duration>, RhiError> {
        self.poll_pass_timings();
        Ok(self.latest_measured_gpu_duration.take())
    }

    #[must_use]
    pub fn surface_format(&self) -> SurfaceFormat {
        SurfaceFormat {
            name: format!("{:?}", self.surface_config.format),
            srgb: self.surface_config.format.is_srgb(),
        }
    }

    #[must_use]
    pub const fn size(&self) -> WindowSize {
        self.size
    }

    #[must_use]
    pub fn clear_pass_names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.clear_graph.ordered_passes().map(|(_, name)| name)
    }

    #[must_use]
    pub fn device_loss(&self) -> Option<DeviceLoss> {
        self.device_loss
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn resize(&mut self, size: WindowSize) {
        self.size = size;
        if size.is_zero() {
            self.configured = false;
            self.depth_buffer = None;
            return;
        }

        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_buffer = Some(create_depth_buffer_for_device(
            &self.device,
            size,
            self.depth_buffer
                .as_ref()
                .map_or(DepthFormat::default(), DepthBuffer::format),
            "Meridian depth",
        ));
        self.configured = true;
    }

    /// Allocates a depth attachment for a renderer-owned pass.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidDepthSize`] for a zero-sized attachment
    /// or [`RhiErrorKind::DeviceLost`] when the device is already lost.
    pub fn create_depth_buffer(
        &self,
        label: &str,
        size: WindowSize,
        format: DepthFormat,
    ) -> Result<DepthBuffer, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if size.is_zero() {
            return Err(RhiError::new(
                RhiErrorKind::InvalidDepthSize,
                "depth attachments must have a non-zero size",
            ));
        }
        Ok(create_depth_buffer_for_device(
            &self.device,
            size,
            format,
            label,
        ))
    }

    /// Allocates a comparison-sampled depth texture array for cascaded sun
    /// shadows. Each cascade is one layer in the array.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidShadowMapSize`] for an unsupported
    /// resolution, [`RhiErrorKind::InvalidShadowCascade`] for a cascade count
    /// outside `1..=8`, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_shadow_map(
        &self,
        label: &str,
        resolution: u32,
        cascade_count: u8,
    ) -> Result<GpuShadowMap, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if resolution == 0 || resolution > self.adapter.limits().max_texture_dimension_2d {
            return Err(RhiError::new(
                RhiErrorKind::InvalidShadowMapSize,
                format!("shadow map resolution {resolution} exceeds the adapter limit"),
            ));
        }
        if !(1..=8).contains(&cascade_count)
            || u32::from(cascade_count) > self.adapter.limits().max_texture_array_layers
        {
            return Err(RhiError::new(
                RhiErrorKind::InvalidShadowCascade,
                format!("shadow cascade count {cascade_count} is unsupported"),
            ));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: u32::from(cascade_count),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(label),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            array_layer_count: Some(u32::from(cascade_count)),
            ..Default::default()
        });
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Ok(GpuShadowMap {
            texture,
            view,
            sampler,
            resolution,
            cascade_count,
        })
    }

    /// Allocates a sampled 2D texture for material data.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidTextureSize`] for a zero-sized texture,
    /// [`RhiErrorKind::InvalidTextureMipLevels`] for an invalid mip count, or
    /// [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_texture(
        &self,
        label: &str,
        size: WindowSize,
        mip_level_count: u32,
        format: TextureFormat,
    ) -> Result<GpuTexture, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if size.is_zero() {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureSize,
                "textures must have a non-zero size",
            ));
        }
        if !(1..=32).contains(&mip_level_count) {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureMipLevels,
                "texture mip level count must be between 1 and 32",
            ));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.wgpu_format(),
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Ok(GpuTexture {
            texture,
            view,
            sampler,
            size,
            mip_level_count,
            format,
        })
    }

    /// Allocates a sampled cube texture for diffuse environment irradiance.
    ///
    /// The six cube faces share one square size, format, and mip chain. Face
    /// data is uploaded independently through [`Self::write_environment_face`].
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidTextureSize`] for a zero or unsupported
    /// face size, [`RhiErrorKind::InvalidTextureMipLevels`] for an invalid mip
    /// count, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_environment_map(
        &self,
        label: &str,
        face_size: u32,
        mip_level_count: u32,
        format: TextureFormat,
    ) -> Result<GpuEnvironmentMap, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if face_size == 0 || face_size > self.adapter.limits().max_texture_dimension_2d {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureSize,
                format!("environment face size {face_size} exceeds the adapter limit"),
            ));
        }
        let max_mips = u32::BITS - face_size.leading_zeros();
        if mip_level_count == 0 || mip_level_count > max_mips {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureMipLevels,
                format!(
                    "environment map with face size {face_size} supports 1..={max_mips} mip levels"
                ),
            ));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: face_size,
                height: face_size,
                depth_or_array_layers: 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.wgpu_format(),
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(label),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Ok(GpuEnvironmentMap {
            texture,
            view,
            sampler,
            face_size,
            mip_level_count,
            format,
        })
    }

    /// Creates the initial sampled base-color binding for a startup pipeline.
    ///
    /// The pipeline's group 0 must declare a texture at binding 0 and a
    /// filtering sampler at binding 1. This deliberately narrow contract is
    /// the first material boundary; camera/object uniform groups are separate
    /// follow-up work.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] when the shader layout does
    /// not match the contract or the device is already lost.
    pub fn create_texture_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        texture: &GpuTexture,
    ) -> Result<GpuTextureBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(0);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuTextureBindGroup { bind_group })
    }

    /// Creates the material texture binding for base color, normal, and
    /// metallic-roughness channels.
    ///
    /// Group 0 uses texture/sampler pairs at bindings 0/1, 2/3, and 4/5.
    /// Keeping the channel set together makes material activation atomic at
    /// the draw boundary and leaves fallback textures explicit to the caller.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] when the shader layout does
    /// not match the six-binding contract or the device is already lost.
    pub fn create_material_texture_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        base_color: &GpuTexture,
        normal: &GpuTexture,
        metallic_roughness: &GpuTexture,
    ) -> Result<GpuMaterialTextureBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if normal.format.is_srgb() || metallic_roughness.format.is_srgb() {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "normal and metallic-roughness textures must use linear formats",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(0);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&base_color.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&base_color.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&metallic_roughness.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&metallic_roughness.sampler),
                },
            ],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuMaterialTextureBindGroup { bind_group })
    }

    /// Creates the camera/object uniform binding for a startup material
    /// pipeline.
    ///
    /// The pipeline's group 1 must declare uniform buffers at bindings 0 and
    /// 1, and both buffers must have been created with
    /// [`BufferUsage::Uniform`].
    /// The camera block contains a 64-byte view-projection matrix followed by
    /// 16-byte world-space camera position and forward vectors; the object
    /// block contains one 64-byte model matrix.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] for incompatible buffers or
    /// shader layouts, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_uniform_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        camera_buffer: &GpuBuffer,
        object_buffer: &GpuBuffer,
    ) -> Result<GpuUniformBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if camera_buffer.usage != BufferUsage::Uniform
            || object_buffer.usage != BufferUsage::Uniform
        {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "camera and object bindings require uniform buffers",
            ));
        }
        if camera_buffer.size < 96 || object_buffer.size < 64 {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "camera bindings require at least 96 bytes and object bindings 64 bytes",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(1);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: object_buffer.buffer.as_entire_binding(),
                },
            ],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuUniformBindGroup { bind_group })
    }

    /// Creates the initial material-parameter binding for a startup pipeline.
    ///
    /// Group 2 binding 0 is a uniform buffer containing a four-float base
    /// color followed by metallic and roughness scalars. The remaining bytes
    /// in the 32-byte block are reserved for alignment and future parameters.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] for an incompatible buffer
    /// or shader layout, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_material_parameter_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        material_buffer: &GpuBuffer,
    ) -> Result<GpuMaterialParameterBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if material_buffer.usage != BufferUsage::Uniform {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "material parameters require a uniform buffer",
            ));
        }
        if material_buffer.size < 32 {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "material parameters require at least 32 bytes",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(2);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: material_buffer.buffer.as_entire_binding(),
            }],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuMaterialParameterBindGroup { bind_group })
    }

    /// Creates the initial sun-light binding for a startup pipeline.
    ///
    /// Group 3 binding 0 is a 32-byte block containing direction-to-light as
    /// a `vec4` and RGB color plus normalized intensity as a second `vec4`.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] for an incompatible buffer
    /// or shader layout, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_lighting_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        lighting_buffer: &GpuBuffer,
    ) -> Result<GpuLightingBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if lighting_buffer.usage != BufferUsage::Uniform || lighting_buffer.size < 32 {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "sun lighting requires at least 32 bytes of uniform storage",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(3);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lighting_buffer.buffer.as_entire_binding(),
            }],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuLightingBindGroup { bind_group })
    }

    /// Creates the sun-light and filtered cascaded-shadow binding used by the
    /// direct material path.
    ///
    /// Group 3 contains the 32-byte sun block, the depth texture array, its
    /// comparison sampler, and a 560-byte block containing eight cascade
    /// matrices plus split and bias data. Keeping these resources in one
    /// group stays within the default wgpu bind-group limit.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] for incompatible buffers or
    /// shader layout, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_lighting_shadow_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        lighting_buffer: &GpuBuffer,
        shadow_map: &GpuShadowMap,
        shadow_parameters: &GpuBuffer,
    ) -> Result<GpuLightingBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if lighting_buffer.usage != BufferUsage::Uniform || lighting_buffer.size < 32 {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "sun lighting requires at least 32 bytes of uniform storage",
            ));
        }
        if shadow_parameters.usage != BufferUsage::Uniform || shadow_parameters.size < 560 {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "shadow parameters require a 560-byte uniform buffer",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(3);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lighting_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: shadow_parameters.buffer.as_entire_binding(),
                },
            ],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuLightingBindGroup { bind_group })
    }

    /// Creates the direct PBR lighting binding with sun, cascaded shadows, and
    /// diffuse environment irradiance.
    ///
    /// Group 3 extends the sun/shadow contract with a cube texture and
    /// filtering sampler at bindings 4/5 plus a 16-byte environment intensity
    /// block at binding 6. Specular prefiltering and a BRDF lookup texture are
    /// intentionally outside this first IBL slice.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::BindGroupCreation`] for incompatible buffers or
    /// shader layout, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn create_lighting_shadow_environment_bind_group(
        &self,
        label: &str,
        pipeline: &GpuRenderPipeline,
        resources: &PbrLightingResources<'_>,
    ) -> Result<GpuLightingBindGroup, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if resources.lighting_buffer.usage != BufferUsage::Uniform
            || resources.lighting_buffer.size < 32
        {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "sun lighting requires at least 32 bytes of uniform storage",
            ));
        }
        if resources.shadow_parameters.usage != BufferUsage::Uniform
            || resources.shadow_parameters.size < 560
        {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "shadow parameters require a 560-byte uniform buffer",
            ));
        }
        if resources.environment_parameters.usage != BufferUsage::Uniform
            || resources.environment_parameters.size < 16
        {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                "environment lighting requires a 16-byte uniform buffer",
            ));
        }
        let layout = pipeline.pipeline.get_bind_group_layout(3);
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resources.lighting_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&resources.shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&resources.shadow_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: resources.shadow_parameters.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&resources.environment_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&resources.environment_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: resources.environment_parameters.buffer.as_entire_binding(),
                },
            ],
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::BindGroupCreation,
                error.to_string(),
            ));
        }
        Ok(GpuLightingBindGroup { bind_group })
    }

    /// Queues a bounded upload into one texture mip level.
    ///
    /// `bytes_per_row` may include padding, but the supplied data must contain
    /// every padded row except that the final row only needs its texel bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidTextureWrite`] for an invalid mip,
    /// row layout, or short data, or [`RhiErrorKind::DeviceLost`] after device
    /// loss.
    pub fn write_texture(
        &self,
        texture: &GpuTexture,
        mip_level: u32,
        data: &[u8],
        bytes_per_row: u32,
    ) -> Result<(), RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let mip_size = validate_texture_write(
            texture.size,
            texture.mip_level_count,
            texture.format,
            mip_level,
            bytes_per_row,
            data.len(),
        )
        .map_err(|message| RhiError::new(RhiErrorKind::InvalidTextureWrite, message))?;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture.texture,
                mip_level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(mip_size.height),
            },
            wgpu::Extent3d {
                width: mip_size.width,
                height: mip_size.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    /// Queues a bounded upload into one face and mip of an environment cube.
    ///
    /// Faces use the backend's cube-array order and are indexed `0..6`.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidTextureWrite`] for an invalid face, mip,
    /// row layout, or short data, or [`RhiErrorKind::DeviceLost`] after device
    /// loss.
    pub fn write_environment_face(
        &self,
        environment_map: &GpuEnvironmentMap,
        face_index: u8,
        mip_level: u32,
        data: &[u8],
        bytes_per_row: u32,
    ) -> Result<(), RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if face_index >= 6 {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureWrite,
                format!("environment cube face {face_index} is outside 0..6"),
            ));
        }
        let mip_size = validate_texture_write(
            WindowSize::new(environment_map.face_size, environment_map.face_size),
            environment_map.mip_level_count,
            environment_map.format,
            mip_level,
            bytes_per_row,
            data.len(),
        )
        .map_err(|message| RhiError::new(RhiErrorKind::InvalidTextureWrite, message))?;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &environment_map.texture,
                mip_level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: u32::from(face_index),
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(mip_size.height),
            },
            wgpu::Extent3d {
                width: mip_size.width,
                height: mip_size.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    /// Allocates a device-local buffer that can receive queue writes.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidBufferSize`] for a zero-sized buffer or
    /// [`RhiErrorKind::DeviceLost`] when the device is already lost.
    pub fn create_buffer(
        &self,
        label: &str,
        size: u64,
        usage: BufferUsage,
    ) -> Result<GpuBuffer, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if size == 0 {
            return Err(RhiError::new(
                RhiErrorKind::InvalidBufferSize,
                "GPU buffers must have a non-zero size",
            ));
        }

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu_buffer_usage(usage),
            mapped_at_creation: false,
        });
        Ok(GpuBuffer {
            buffer,
            size,
            usage,
        })
    }

    /// Validates and constructs a render pipeline during startup.
    ///
    /// The pipeline uses the current sRGB surface format and no vertex buffers;
    /// this is sufficient for fullscreen or `vertex_index`-driven bootstrap
    /// passes. More specialized layouts belong in renderer-owned pipeline
    /// descriptors built on top of this boundary.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::DeviceLost`] after device loss or
    /// [`RhiErrorKind::PipelineCreation`] when `wgpu` rejects the shader or
    /// pipeline descriptor.
    pub fn create_render_pipeline(
        &self,
        label: &str,
        shader_source: &str,
        vertex_entry_point: &str,
        fragment_entry_point: &str,
    ) -> Result<GpuRenderPipeline, RhiError> {
        self.create_render_pipeline_with_layout(
            label,
            shader_source,
            vertex_entry_point,
            fragment_entry_point,
            None,
        )
    }

    /// Validates and constructs a render pipeline with one optional vertex
    /// buffer layout during startup.
    ///
    /// The layout is translated at this boundary; callers never handle
    /// backend vertex-format types.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::DeviceLost`] after device loss or
    /// [`RhiErrorKind::PipelineCreation`] when `wgpu` rejects the shader or
    /// pipeline descriptor.
    pub fn create_render_pipeline_with_layout(
        &self,
        label: &str,
        shader_source: &str,
        vertex_entry_point: &str,
        fragment_entry_point: &str,
        layout: Option<&VertexLayout>,
    ) -> Result<GpuRenderPipeline, RhiError> {
        self.create_pipeline_with_layout(
            label,
            shader_source,
            vertex_entry_point,
            Some(fragment_entry_point),
            layout,
            false,
        )
    }

    /// Validates and constructs a depth-only pipeline for a shadow-map pass.
    ///
    /// The pipeline has no color target and writes `Depth32Float` with a
    /// less-or-equal comparison. It is created during startup like all other
    /// renderer pipelines.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::DeviceLost`] after device loss or
    /// [`RhiErrorKind::PipelineCreation`] when `wgpu` rejects the shader or
    /// pipeline descriptor.
    pub fn create_shadow_depth_pipeline_with_layout(
        &self,
        label: &str,
        shader_source: &str,
        vertex_entry_point: &str,
        layout: Option<&VertexLayout>,
    ) -> Result<GpuRenderPipeline, RhiError> {
        self.create_pipeline_with_layout(
            label,
            shader_source,
            vertex_entry_point,
            None,
            layout,
            true,
        )
    }

    fn create_pipeline_with_layout(
        &self,
        label: &str,
        shader_source: &str,
        vertex_entry_point: &str,
        fragment_entry_point: Option<&str>,
        layout: Option<&VertexLayout>,
        depth_only: bool,
    ) -> Result<GpuRenderPipeline, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source)),
            });
        let color_targets = [Some(wgpu::ColorTargetState {
            format: self.surface_config.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let attributes = layout
            .map(|layout| {
                layout
                    .attributes()
                    .iter()
                    .map(|attribute| wgpu::VertexAttribute {
                        format: attribute.format.wgpu_format(),
                        offset: attribute.offset,
                        shader_location: attribute.shader_location,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let vertex_buffers = layout
            .map(|layout| {
                vec![Some(wgpu::VertexBufferLayout {
                    array_stride: layout.stride,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })]
            })
            .unwrap_or_default();
        let fragment = fragment_entry_point.map(|entry_point| wgpu::FragmentState {
            module: &module,
            entry_point: Some(entry_point),
            targets: &color_targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });
        let depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(depth_only || layout.is_some()),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vertex_entry_point),
                    buffers: &vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment,
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RhiError::new(
                RhiErrorKind::PipelineCreation,
                error.to_string(),
            ));
        }
        Ok(GpuRenderPipeline { pipeline })
    }

    /// Queues a bounded, four-byte-aligned write into an RHI-owned buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidBufferWrite`] for misalignment or an
    /// out-of-bounds range, or [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn write_buffer(
        &self,
        buffer: &GpuBuffer,
        offset: u64,
        data: &[u8],
    ) -> Result<(), RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        validate_buffer_write(buffer.size, offset, data.len())
            .map_err(|message| RhiError::new(RhiErrorKind::InvalidBufferWrite, message))?;
        if data.is_empty() {
            return Ok(());
        }
        self.queue.write_buffer(&buffer.buffer, offset, data);
        Ok(())
    }

    /// Clears and presents one surface frame without creating a render pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`RhiError`] if the device has been lost, the surface reports a
    /// validation failure, or a lost surface cannot be recreated.
    pub fn clear_and_present(&mut self, color: ClearColor) -> Result<FrameOutcome, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if !self.configured || self.size.is_zero() {
            return Ok(FrameOutcome::SkippedZeroSize);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => {
                self.encode_clear_and_present(texture, color, FrameOutcome::Presented);
                Ok(FrameOutcome::Presented)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.encode_clear_and_present(texture, color, FrameOutcome::PresentedSuboptimal);
                self.resize(self.size);
                Ok(FrameOutcome::PresentedSuboptimal)
            }
            wgpu::CurrentSurfaceTexture::Timeout => Ok(FrameOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => Ok(FrameOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.size);
                Ok(FrameOutcome::ReconfiguredOutdated)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                Ok(FrameOutcome::RecreatedLostSurface)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(RhiError::new(
                RhiErrorKind::SurfaceValidation,
                "surface acquisition raised a validation error",
            )),
        }
    }

    /// Submits the clear pass to a small offscreen target for structural GPU
    /// validation when a presentation surface is unavailable.
    ///
    /// This performs no readback or capture and makes no visual-quality claim.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::DeviceLost`] after device loss.
    #[doc(hidden)]
    pub fn submit_clear_structural_validation(
        &mut self,
        color: ClearColor,
    ) -> Result<(), RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let size = WindowSize::new(64, 64);
        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Meridian clear structural validation color"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_buffer_for_device(
            &self.device,
            size,
            DepthFormat::Depth32Float,
            "Meridian clear structural validation depth",
        );
        self.encode_clear(&color_view, Some(&depth.view), color, None, None);
        Ok(())
    }

    /// Draws the clear path into an explicitly offscreen capture target.
    /// This cannot satisfy presentation evidence.
    ///
    /// # Errors
    ///
    /// Returns invalid-size or device-loss errors before submission.
    pub fn submit_clear_offscreen_capture(
        &mut self,
        color: ClearColor,
        size: WindowSize,
    ) -> Result<(), RhiError> {
        if size.is_zero() {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureSize,
                "offscreen capture must have non-zero dimensions",
            ));
        }
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let texture = self.create_capture_target("Meridian clear offscreen capture", size);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_buffer_for_device(
            &self.device,
            size,
            DepthFormat::Depth32Float,
            "Meridian clear offscreen capture depth",
        );
        let capture = self.begin_capture_for_texture(
            size,
            self.surface_config.format,
            CaptureSource::Offscreen,
            None,
        );
        self.encode_clear(&view, Some(&depth.view), color, Some(&texture), capture);
        Ok(())
    }

    /// Executes a three-vertex pipeline draw and presents one surface frame.
    ///
    /// The pipeline must have been constructed during startup. This is the
    /// first executable raster boundary; material bind groups and indexed mesh
    /// draws are layered on top of it by the renderer.
    ///
    /// # Errors
    ///
    /// Returns [`RhiError`] if the device has been lost, the surface reports a
    /// validation failure, or a lost surface cannot be recreated.
    pub fn render_pipeline_and_present(
        &mut self,
        pipeline: &GpuRenderPipeline,
        color: ClearColor,
    ) -> Result<FrameOutcome, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if !self.configured || self.size.is_zero() {
            return Ok(FrameOutcome::SkippedZeroSize);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => {
                self.encode_pipeline_and_present(texture, pipeline, color);
                Ok(FrameOutcome::Presented)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.encode_pipeline_and_present(texture, pipeline, color);
                self.resize(self.size);
                Ok(FrameOutcome::PresentedSuboptimal)
            }
            wgpu::CurrentSurfaceTexture::Timeout => Ok(FrameOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => Ok(FrameOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.size);
                Ok(FrameOutcome::ReconfiguredOutdated)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                Ok(FrameOutcome::RecreatedLostSurface)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(RhiError::new(
                RhiErrorKind::SurfaceValidation,
                "surface acquisition raised a validation error",
            )),
        }
    }

    /// Executes an indexed `u32` mesh draw and presents one surface frame.
    ///
    /// Vertex and index buffers must have been created with the matching RHI
    /// usage classes. The pipeline must have been constructed with a vertex
    /// layout compatible with the bound vertex buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidDraw`] for incompatible buffers or an
    /// out-of-range index count, or a surface/device error.
    pub fn render_indexed_mesh_and_present(
        &mut self,
        pipeline: &GpuRenderPipeline,
        vertex_buffer: &GpuBuffer,
        index_buffer: &GpuBuffer,
        index_count: u32,
        color: ClearColor,
    ) -> Result<FrameOutcome, RhiError> {
        self.render_indexed_mesh_internal(
            IndexedDraw {
                pipeline,
                vertex_buffer,
                index_buffer,
                index_count,
                texture_bind_group: None,
                material_texture_bind_group: None,
                uniform_bind_group: None,
                material_parameter_bind_group: None,
                lighting_bind_group: None,
            },
            color,
        )
    }

    /// Executes an indexed mesh draw with the initial sampled base-color
    /// binding and presents one surface frame.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidDraw`] for incompatible buffers or an
    /// out-of-range index count, or a surface/device error.
    pub fn render_indexed_mesh_with_texture_and_present(
        &mut self,
        pipeline: &GpuRenderPipeline,
        vertex_buffer: &GpuBuffer,
        index_buffer: &GpuBuffer,
        index_count: u32,
        texture_bind_group: &GpuTextureBindGroup,
        color: ClearColor,
    ) -> Result<FrameOutcome, RhiError> {
        self.render_indexed_mesh_internal(
            IndexedDraw {
                pipeline,
                vertex_buffer,
                index_buffer,
                index_count,
                texture_bind_group: Some(texture_bind_group),
                material_texture_bind_group: None,
                uniform_bind_group: None,
                material_parameter_bind_group: None,
                lighting_bind_group: None,
            },
            color,
        )
    }

    /// Executes an indexed mesh draw with sampled texture and camera/object
    /// uniform bindings, then presents one surface frame.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidDraw`] for incompatible buffers or an
    /// out-of-range index count, or a surface/device error.
    pub fn render_indexed_mesh_with_material_bindings_and_present(
        &mut self,
        pipeline: &GpuRenderPipeline,
        vertex_buffer: &GpuBuffer,
        index_buffer: &GpuBuffer,
        index_count: u32,
        material_bindings: &GpuMaterialBindings,
        color: ClearColor,
    ) -> Result<FrameOutcome, RhiError> {
        self.render_indexed_mesh_internal(
            IndexedDraw {
                pipeline,
                vertex_buffer,
                index_buffer,
                index_count,
                texture_bind_group: None,
                material_texture_bind_group: Some(&material_bindings.textures),
                uniform_bind_group: Some(&material_bindings.uniforms),
                material_parameter_bind_group: Some(&material_bindings.parameters),
                lighting_bind_group: Some(&material_bindings.lighting),
            },
            color,
        )
    }

    /// Submits the indexed material path to a small offscreen target for
    /// structural GPU validation when a presentation surface is unavailable.
    ///
    /// This method performs no readback or capture and makes no visual-quality
    /// claim. It exists so native smoke validation can execute the real indexed
    /// pipeline even when the window system reports an occluded surface.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidDraw`] for incompatible buffers or an
    /// out-of-range index count, or [`RhiErrorKind::DeviceLost`] after device
    /// loss.
    #[doc(hidden)]
    pub fn submit_indexed_mesh_structural_validation(
        &mut self,
        pipeline: &GpuRenderPipeline,
        vertex_buffer: &GpuBuffer,
        index_buffer: &GpuBuffer,
        index_count: u32,
        material_bindings: &GpuMaterialBindings,
        color: ClearColor,
    ) -> Result<(), RhiError> {
        let draw = IndexedDraw {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            texture_bind_group: None,
            material_texture_bind_group: Some(&material_bindings.textures),
            uniform_bind_group: Some(&material_bindings.uniforms),
            material_parameter_bind_group: Some(&material_bindings.parameters),
            lighting_bind_group: Some(&material_bindings.lighting),
        };
        validate_indexed_draw(draw.vertex_buffer, draw.index_buffer, draw.index_count)?;
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let size = WindowSize::new(64, 64);
        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Meridian indexed structural validation color"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_buffer_for_device(
            &self.device,
            size,
            DepthFormat::Depth32Float,
            "Meridian indexed structural validation depth",
        );
        self.encode_indexed_mesh(&color_view, Some(&depth.view), draw, color, None, None);
        Ok(())
    }

    /// Draws the indexed material path into an offscreen capture target.
    /// This uses the same pipeline and bindings but makes no presentation claim.
    ///
    /// # Errors
    ///
    /// Returns draw, size, or device-loss errors before submission.
    pub fn submit_indexed_mesh_offscreen_capture(
        &mut self,
        capture_draw: &OffscreenIndexedCaptureDraw<'_>,
    ) -> Result<(), RhiError> {
        let draw = IndexedDraw {
            pipeline: capture_draw.pipeline,
            vertex_buffer: capture_draw.vertex_buffer,
            index_buffer: capture_draw.index_buffer,
            index_count: capture_draw.index_count,
            texture_bind_group: None,
            material_texture_bind_group: Some(&capture_draw.material_bindings.textures),
            uniform_bind_group: Some(&capture_draw.material_bindings.uniforms),
            material_parameter_bind_group: Some(&capture_draw.material_bindings.parameters),
            lighting_bind_group: Some(&capture_draw.material_bindings.lighting),
        };
        validate_indexed_draw(
            capture_draw.vertex_buffer,
            capture_draw.index_buffer,
            capture_draw.index_count,
        )?;
        if capture_draw.size.is_zero() {
            return Err(RhiError::new(
                RhiErrorKind::InvalidTextureSize,
                "offscreen capture must have non-zero dimensions",
            ));
        }
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let texture =
            self.create_capture_target("Meridian indexed offscreen capture", capture_draw.size);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_buffer_for_device(
            &self.device,
            capture_draw.size,
            DepthFormat::Depth32Float,
            "Meridian indexed offscreen capture depth",
        );
        let capture = self.begin_capture_for_texture(
            capture_draw.size,
            self.surface_config.format,
            CaptureSource::Offscreen,
            None,
        );
        self.encode_indexed_mesh(
            &view,
            Some(&depth.view),
            draw,
            capture_draw.color,
            Some(&texture),
            capture,
        );
        Ok(())
    }

    /// Renders one indexed mesh into one cascade layer of a shadow map.
    ///
    /// The supplied group-1 uniform binding must contain the light
    /// view-projection matrix at camera binding 0 and the model matrix at
    /// object binding 1. This pass does not touch or present the swapchain.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::InvalidShadowCascade`] for an out-of-range
    /// layer, [`RhiErrorKind::InvalidDraw`] for incompatible mesh buffers, or
    /// [`RhiErrorKind::DeviceLost`] after device loss.
    pub fn render_shadow_depth(&mut self, draw: &ShadowDepthDraw<'_>) -> Result<(), RhiError> {
        if draw.cascade_index >= draw.shadow_map.cascade_count {
            return Err(RhiError::new(
                RhiErrorKind::InvalidShadowCascade,
                format!(
                    "shadow cascade index {} is outside 0..{}",
                    draw.cascade_index, draw.shadow_map.cascade_count
                ),
            ));
        }
        validate_indexed_draw(draw.vertex_buffer, draw.index_buffer, draw.index_count)?;
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }

        let view = draw
            .shadow_map
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: u32::from(draw.cascade_index),
                array_layer_count: Some(1),
                ..Default::default()
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian shadow depth encoder"),
            });
        let timing = self.begin_pass_timing(SHADOW_DEPTH_PASS_LABEL);
        self.prepare_timestamp_resolve(&mut encoder, timing);
        let color_attachments: [Option<wgpu::RenderPassColorAttachment<'_>>; 0] = [];
        let cpu_start = Instant::now();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Meridian shadow depth pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timestamp_writes(timing),
                ..Default::default()
            });
            pass.set_pipeline(&draw.pipeline.pipeline);
            pass.set_bind_group(1, &draw.uniform_bind_group.bind_group, &[]);
            pass.set_vertex_buffer(0, draw.vertex_buffer.buffer.slice(..));
            pass.set_index_buffer(
                draw.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
        self.submit_timed_encoder(encoder, timing, cpu_start.elapsed());
        Ok(())
    }

    fn render_indexed_mesh_internal(
        &mut self,
        draw: IndexedDraw<'_>,
        color: ClearColor,
    ) -> Result<FrameOutcome, RhiError> {
        validate_indexed_draw(draw.vertex_buffer, draw.index_buffer, draw.index_count)?;
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        if !self.configured || self.size.is_zero() {
            return Ok(FrameOutcome::SkippedZeroSize);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => {
                self.encode_indexed_mesh_and_present(texture, draw, color, FrameOutcome::Presented);
                Ok(FrameOutcome::Presented)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.encode_indexed_mesh_and_present(
                    texture,
                    draw,
                    color,
                    FrameOutcome::PresentedSuboptimal,
                );
                self.resize(self.size);
                Ok(FrameOutcome::PresentedSuboptimal)
            }
            wgpu::CurrentSurfaceTexture::Timeout => Ok(FrameOutcome::SkippedTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => Ok(FrameOutcome::SkippedOccluded),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.resize(self.size);
                Ok(FrameOutcome::ReconfiguredOutdated)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                Ok(FrameOutcome::RecreatedLostSurface)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(RhiError::new(
                RhiErrorKind::SurfaceValidation,
                "surface acquisition raised a validation error",
            )),
        }
    }

    fn encode_clear_and_present(
        &mut self,
        texture: wgpu::SurfaceTexture,
        color: ClearColor,
        outcome: FrameOutcome,
    ) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.depth_buffer.as_ref().map(|depth| depth.view.clone());
        let capture = self.begin_capture_for_texture(
            self.size,
            self.surface_config.format,
            CaptureSource::PresentedSurface,
            Some(outcome),
        );
        self.encode_clear(
            &view,
            depth_view.as_ref(),
            color,
            Some(&texture.texture),
            capture,
        );
        self.queue.present(texture);
    }

    fn encode_clear(
        &mut self,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        color: ClearColor,
        source_texture: Option<&wgpu::Texture>,
        capture: Option<PendingCapture>,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian clear frame encoder"),
            });
        let timing = self.begin_pass_timing(CLEAR_PASS_LABEL);
        self.prepare_timestamp_resolve(&mut encoder, timing);
        let cpu_start = Instant::now();
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: color.red,
                        g: color.green,
                        b: color.blue,
                        a: color.alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Meridian clear pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: depth_view.map(|depth_view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: self.timestamp_writes(timing),
                ..Default::default()
            });
        }
        if let (Some(source_texture), Some(capture)) = (source_texture, capture) {
            record_capture_copy(&mut encoder, source_texture, &self.capture, capture);
        }
        self.submit_timed_encoder_with_capture(encoder, timing, cpu_start.elapsed(), capture);
    }

    fn encode_pipeline_and_present(
        &mut self,
        texture: wgpu::SurfaceTexture,
        pipeline: &GpuRenderPipeline,
        color: ClearColor,
    ) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian bootstrap pipeline encoder"),
            });
        let timing = self.begin_pass_timing(BOOTSTRAP_PIPELINE_PASS_LABEL);
        self.prepare_timestamp_resolve(&mut encoder, timing);
        let cpu_start = Instant::now();
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: color.red,
                        g: color.green,
                        b: color.blue,
                        a: color.alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Meridian bootstrap pipeline pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: self.depth_buffer.as_ref().map(|depth| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: self.timestamp_writes(timing),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.draw(0..3, 0..1);
        }
        self.submit_timed_encoder(encoder, timing, cpu_start.elapsed());
        self.queue.present(texture);
    }

    fn encode_indexed_mesh_and_present(
        &mut self,
        texture: wgpu::SurfaceTexture,
        draw: IndexedDraw<'_>,
        color: ClearColor,
        outcome: FrameOutcome,
    ) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.depth_buffer.as_ref().map(|depth| depth.view.clone());
        let capture = self.begin_capture_for_texture(
            self.size,
            self.surface_config.format,
            CaptureSource::PresentedSurface,
            Some(outcome),
        );
        self.encode_indexed_mesh(
            &view,
            depth_view.as_ref(),
            draw,
            color,
            Some(&texture.texture),
            capture,
        );
        self.queue.present(texture);
    }

    fn encode_indexed_mesh(
        &mut self,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        draw: IndexedDraw<'_>,
        color: ClearColor,
        source_texture: Option<&wgpu::Texture>,
        capture: Option<PendingCapture>,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian indexed mesh encoder"),
            });
        let timing = self.begin_pass_timing(INDEXED_MESH_PASS_LABEL);
        self.prepare_timestamp_resolve(&mut encoder, timing);
        let cpu_start = Instant::now();
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: color.red,
                        g: color.green,
                        b: color.blue,
                        a: color.alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Meridian indexed mesh pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: depth_view.map(|depth_view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: self.timestamp_writes(timing),
                ..Default::default()
            });
            pass.set_pipeline(&draw.pipeline.pipeline);
            if let Some(material_texture_bind_group) = draw.material_texture_bind_group {
                pass.set_bind_group(0, &material_texture_bind_group.bind_group, &[]);
            }
            if let Some(texture_bind_group) = draw.texture_bind_group {
                pass.set_bind_group(0, &texture_bind_group.bind_group, &[]);
            }
            if let Some(uniform_bind_group) = draw.uniform_bind_group {
                pass.set_bind_group(1, &uniform_bind_group.bind_group, &[]);
            }
            if let Some(material_parameter_bind_group) = draw.material_parameter_bind_group {
                pass.set_bind_group(2, &material_parameter_bind_group.bind_group, &[]);
            }
            if let Some(lighting_bind_group) = draw.lighting_bind_group {
                pass.set_bind_group(3, &lighting_bind_group.bind_group, &[]);
            }
            pass.set_vertex_buffer(0, draw.vertex_buffer.buffer.slice(..));
            pass.set_index_buffer(
                draw.index_buffer.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
        if let (Some(source_texture), Some(capture)) = (source_texture, capture) {
            record_capture_copy(&mut encoder, source_texture, &self.capture, capture);
        }
        self.submit_timed_encoder_with_capture(encoder, timing, cpu_start.elapsed(), capture);
    }

    fn create_capture_target(&self, label: &str, size: WindowSize) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn begin_capture_for_texture(
        &mut self,
        size: WindowSize,
        format: wgpu::TextureFormat,
        source: CaptureSource,
        surface_outcome: Option<FrameOutcome>,
    ) -> Option<PendingCapture> {
        self.poll_captures();
        let pending = self.capture.pending.pop_front()?;
        if source == CaptureSource::PresentedSurface && !self.surface_copy_supported {
            self.push_capture_result(CaptureOutcome::UnsupportedCapability {
                capture_id: pending.id,
                frame_id: pending.request.frame_id,
                failure: CaptureFailure::SurfaceCopyUnsupported,
            });
            return None;
        }
        let Some(source_format) = capture_source_format(format) else {
            self.push_capture_result(CaptureOutcome::UnsupportedCapability {
                capture_id: pending.id,
                frame_id: pending.request.frame_id,
                failure: CaptureFailure::UnsupportedFormat,
            });
            return None;
        };
        let layout = match capture_layout(size, pending.request) {
            Ok(layout) => layout,
            Err(failure) => {
                self.push_capture_result(CaptureOutcome::Inconclusive {
                    capture_id: pending.id,
                    frame_id: pending.request.frame_id,
                    failure,
                });
                return None;
            }
        };
        let Some(slot_index) = first_free_slot_index(
            self.capture
                .slots
                .iter()
                .map(|slot| slot.in_flight.is_some()),
        ) else {
            self.push_capture_result(CaptureOutcome::Inconclusive {
                capture_id: pending.id,
                frame_id: pending.request.frame_id,
                failure: CaptureFailure::ReadbackSaturated,
            });
            return None;
        };
        let slot = &mut self.capture.slots[slot_index];
        if slot.buffer_size != layout.buffer_size || slot.buffer.is_none() {
            slot.buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Meridian capture readback"),
                size: layout.buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
            slot.buffer_size = layout.buffer_size;
        }
        slot.generation = slot.generation.wrapping_add(1).max(1);
        let correlation = CaptureCorrelation {
            generation: slot.generation,
            capture_id: pending.id,
            frame_id: pending.request.frame_id,
            width: size.width,
            height: size.height,
            padded_bytes_per_row: layout.padded_bytes_per_row,
            source_format,
            source,
            surface_outcome,
        };
        Some(PendingCapture {
            slot_index,
            generation: slot.generation,
            correlation,
        })
    }

    fn allocate_timing_frame_id(&mut self) -> TimingFrameId {
        let frame_id = TimingFrameId(self.next_timing_frame_id);
        self.next_timing_frame_id = self.next_timing_frame_id.wrapping_add(1).max(1);
        frame_id
    }

    fn allocate_submission_id(&mut self) -> u64 {
        let submission_id = self.next_submission_id;
        self.next_submission_id = self.next_submission_id.wrapping_add(1).max(1);
        submission_id
    }

    fn begin_pass_timing(&mut self, pass: PassTimingLabel) -> PendingPassTiming {
        self.poll_pass_timings();
        let frame_id = self
            .active_timing_frame
            .unwrap_or_else(|| self.allocate_timing_frame_id());
        let submission_id = self.allocate_submission_id();
        let gpu = match self.timing_availability {
            TimingAvailability::Available => {
                let slot = self.timestamp_query.as_mut().and_then(|state| {
                    let slot_index = first_free_slot_index(
                        state.slots.iter().map(|slot| slot.in_flight.is_some()),
                    )?;
                    Some((slot_index, &mut state.slots[slot_index]))
                });
                match slot {
                    Some((slot_index, slot)) => {
                        slot.generation = slot.generation.wrapping_add(1).max(1);
                        PendingGpuTiming::Readback {
                            slot_index,
                            generation: slot.generation,
                        }
                    }
                    None => PendingGpuTiming::Final(GpuTimingOutcome::Inconclusive(
                        GpuTimingFailure::ReadbackSaturated,
                    )),
                }
            }
            availability => PendingGpuTiming::Final(timing_unavailable_outcome(availability)),
        };
        PendingPassTiming {
            frame_id,
            runtime_frame_id: self.active_runtime_frame,
            submission_id,
            pass,
            gpu,
        }
    }

    fn prepare_timestamp_resolve(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timing: PendingPassTiming,
    ) {
        let PendingGpuTiming::Readback { slot_index, .. } = timing.gpu else {
            return;
        };
        let Some(slot) = self
            .timestamp_query
            .as_ref()
            .and_then(|state| state.slots.get(slot_index))
        else {
            return;
        };
        encoder.clear_buffer(&slot.resolve_buffer, 0, None);
    }

    fn timestamp_writes(
        &self,
        timing: PendingPassTiming,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let PendingGpuTiming::Readback { slot_index, .. } = timing.gpu else {
            return None;
        };
        self.timestamp_query
            .as_ref()
            .and_then(|state| state.slots.get(slot_index))
            .map(|slot| wgpu::RenderPassTimestampWrites {
                query_set: &slot.query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            })
    }

    fn submit_timed_encoder(
        &mut self,
        encoder: wgpu::CommandEncoder,
        timing: PendingPassTiming,
        cpu_encode_time: Duration,
    ) {
        self.submit_timed_encoder_with_capture(encoder, timing, cpu_encode_time, None);
    }

    fn submit_timed_encoder_with_capture(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
        timing: PendingPassTiming,
        cpu_encode_time: Duration,
        capture: Option<PendingCapture>,
    ) {
        match timing.gpu {
            PendingGpuTiming::Readback {
                slot_index,
                generation,
            } => {
                let correlation = TimestampCorrelation {
                    generation,
                    frame_id: timing.frame_id,
                    runtime_frame_id: timing.runtime_frame_id,
                    submission_id: timing.submission_id,
                    pass: timing.pass,
                    cpu_encode_time,
                };
                let state = self
                    .timestamp_query
                    .as_mut()
                    .expect("available timing has timestamp state");
                let sender = state.sender.clone();
                let slot = state
                    .slots
                    .get_mut(slot_index)
                    .expect("pending timing references a timestamp slot");
                encoder.resolve_query_set(&slot.query_set, 0..2, &slot.resolve_buffer, 0);
                encoder.copy_buffer_to_buffer(
                    &slot.resolve_buffer,
                    0,
                    &slot.readback_buffer,
                    0,
                    16,
                );
                let command_buffer = encoder.finish();
                let readback_buffer = slot.readback_buffer.clone();
                command_buffer.map_buffer_on_submit(
                    &slot.readback_buffer,
                    wgpu::MapMode::Read,
                    ..,
                    move |map_result| {
                        let result = if map_result.is_ok() {
                            read_mapped_timestamps(&readback_buffer)
                        } else {
                            readback_buffer.unmap();
                            TimestampReadbackResult::MappingFailed
                        };
                        let _ = sender.send(TimestampReadbackMessage {
                            slot_index,
                            correlation,
                            result,
                        });
                    },
                );
                slot.in_flight = Some(correlation);
                self.register_capture_mapping(&command_buffer, capture);
                self.queue.submit([command_buffer]);
            }
            PendingGpuTiming::Final(gpu) => {
                let command_buffer = encoder.finish();
                self.register_capture_mapping(&command_buffer, capture);
                self.queue.submit([command_buffer]);
                self.push_timing_result(PassTimingSample {
                    frame_id: timing.frame_id,
                    runtime_frame_id: timing.runtime_frame_id,
                    submission_id: timing.submission_id,
                    pass: timing.pass,
                    cpu_encode_time,
                    gpu,
                });
            }
        }
    }

    fn register_capture_mapping(
        &mut self,
        command_buffer: &wgpu::CommandBuffer,
        pending: Option<PendingCapture>,
    ) {
        let Some(pending) = pending else {
            return;
        };
        let Some(slot) = self.capture.slots.get_mut(pending.slot_index) else {
            self.push_capture_result(CaptureOutcome::Inconclusive {
                capture_id: pending.correlation.capture_id,
                frame_id: pending.correlation.frame_id,
                failure: CaptureFailure::StaleReadback,
            });
            return;
        };
        if slot.generation != pending.generation || slot.in_flight.is_some() {
            self.push_capture_result(CaptureOutcome::Inconclusive {
                capture_id: pending.correlation.capture_id,
                frame_id: pending.correlation.frame_id,
                failure: CaptureFailure::StaleReadback,
            });
            return;
        }
        let Some(buffer) = slot.buffer.clone() else {
            self.push_capture_result(CaptureOutcome::Inconclusive {
                capture_id: pending.correlation.capture_id,
                frame_id: pending.correlation.frame_id,
                failure: CaptureFailure::StaleReadback,
            });
            return;
        };
        let sender = self.capture.sender.clone();
        let slot_index = pending.slot_index;
        let correlation = pending.correlation;
        let callback_buffer = buffer.clone();
        command_buffer.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
            let result = if result.is_ok() {
                read_mapped_capture(&callback_buffer)
            } else {
                callback_buffer.unmap();
                CaptureReadbackResult::MappingFailed
            };
            let _ = sender.send(CaptureReadbackMessage {
                slot_index,
                correlation,
                result,
            });
        });
        slot.in_flight = Some(correlation);
    }

    fn collect_timing_readbacks(&mut self) {
        loop {
            let message = self
                .timestamp_query
                .as_ref()
                .and_then(|state| state.receiver.try_recv().ok());
            let Some(message) = message else {
                break;
            };
            self.complete_timing_readback(&message);
        }
    }

    fn collect_capture_readbacks(&mut self) {
        loop {
            let message = self.capture.receiver.try_recv().ok();
            let Some(message) = message else {
                break;
            };
            self.complete_capture_readback(message);
        }
    }

    fn complete_capture_readback(&mut self, message: CaptureReadbackMessage) {
        let Some(slot) = self.capture.slots.get_mut(message.slot_index) else {
            self.push_capture_result(capture_failure_outcome(
                message.correlation,
                CaptureFailure::StaleReadback,
            ));
            return;
        };
        match take_matching_capture(&mut slot.in_flight, message.correlation) {
            Ok(Some(_)) => {}
            Err(failure) => {
                self.push_capture_result(capture_failure_outcome(message.correlation, failure));
                return;
            }
            Ok(None) => return,
        }
        let outcome = match message.result {
            CaptureReadbackResult::MappingFailed => {
                capture_failure_outcome(message.correlation, CaptureFailure::MappingFailed)
            }
            CaptureReadbackResult::Bytes(bytes) => {
                match normalize_capture_rows(message.correlation, &bytes) {
                    Ok(pixels) => CaptureOutcome::Captured(CapturedFrame {
                        capture_id: message.correlation.capture_id,
                        frame_id: message.correlation.frame_id,
                        width: message.correlation.width,
                        height: message.correlation.height,
                        format: CapturedPixelFormat::Rgba8Srgb,
                        source: message.correlation.source,
                        surface_outcome: message.correlation.surface_outcome,
                        pixels,
                    }),
                    Err(failure) => capture_failure_outcome(message.correlation, failure),
                }
            }
        };
        self.push_capture_result(outcome);
    }

    fn fail_all_capture_readbacks(&mut self, failure: CaptureFailure) {
        let correlations = self
            .capture
            .slots
            .iter_mut()
            .filter_map(|slot| slot.in_flight.take())
            .collect::<Vec<_>>();
        for correlation in correlations {
            self.push_capture_result(capture_failure_outcome(correlation, failure));
        }
    }

    fn push_capture_result(&mut self, result: CaptureOutcome) {
        push_bounded_capture_result(
            &mut self.capture.results,
            &mut self.capture.dropped_results,
            result,
        );
    }

    fn complete_timing_readback(&mut self, message: &TimestampReadbackMessage) {
        let correlation = {
            let Some(state) = self.timestamp_query.as_mut() else {
                return;
            };
            let Some(slot) = state.slots.get_mut(message.slot_index) else {
                self.push_timing_result(timing_sample_from_correlation(
                    message.correlation,
                    GpuTimingOutcome::Inconclusive(GpuTimingFailure::StaleReadback),
                ));
                return;
            };
            match take_matching_correlation(&mut slot.in_flight, message.correlation) {
                Ok(Some(correlation)) => correlation,
                Ok(None) => return,
                Err(failure) => {
                    self.push_timing_result(timing_sample_from_correlation(
                        message.correlation,
                        GpuTimingOutcome::Inconclusive(failure),
                    ));
                    return;
                }
            }
        };
        let gpu = match self.timing_availability {
            TimingAvailability::UnsupportedPlatform(failure) => {
                GpuTimingOutcome::UnsupportedPlatform(failure)
            }
            TimingAvailability::Inconclusive(failure) => GpuTimingOutcome::Inconclusive(failure),
            TimingAvailability::Available => {
                let timestamp_period_ns = self
                    .timestamp_query
                    .as_ref()
                    .expect("available timing has timestamp state")
                    .timestamp_period_ns;
                let (gpu, availability) = classify_timestamp_readback(
                    message.result,
                    timestamp_period_ns,
                    self.capabilities.backend,
                );
                if let Some(availability) = availability {
                    self.timing_availability = availability;
                }
                gpu
            }
            TimingAvailability::NotRequested => GpuTimingOutcome::NotRequested,
            TimingAvailability::UnsupportedCapability => GpuTimingOutcome::UnsupportedCapability,
        };
        self.push_timing_result(timing_sample_from_correlation(correlation, gpu));
    }

    fn fail_all_timing_readbacks(&mut self, failure: GpuTimingFailure) {
        let correlations = self
            .timestamp_query
            .as_mut()
            .map_or_else(Vec::new, |state| {
                state
                    .slots
                    .iter_mut()
                    .filter_map(|slot| slot.in_flight.take())
                    .collect::<Vec<_>>()
            });
        for correlation in correlations {
            self.push_timing_result(timing_sample_from_correlation(
                correlation,
                GpuTimingOutcome::Inconclusive(failure),
            ));
        }
    }

    fn push_timing_result(&mut self, sample: PassTimingSample) {
        push_bounded_timing_result(
            &mut self.timing_results,
            &mut self.dropped_timing_results,
            &mut self.latest_measured_gpu_duration,
            sample,
        );
    }

    fn recreate_surface(&mut self) -> Result<(), RhiError> {
        self.surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(|error| RhiError::new(RhiErrorKind::SurfaceCreation, error.to_string()))?;
        if !self.adapter.is_surface_supported(&self.surface) {
            return Err(RhiError::new(
                RhiErrorKind::SurfaceUnsupported,
                "selected adapter no longer supports the recreated surface",
            ));
        }
        let capabilities = self.surface.get_capabilities(&self.adapter);
        self.surface_copy_supported = capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC);
        if self.surface_copy_supported {
            self.surface_config.usage |= wgpu::TextureUsages::COPY_SRC;
        } else {
            self.surface_config
                .usage
                .remove(wgpu::TextureUsages::COPY_SRC);
        }
        self.surface_config.format =
            choose_surface_format(&capabilities.formats).ok_or_else(|| {
                RhiError::new(
                    RhiErrorKind::SurfaceUnsupported,
                    "recreated surface reports no texture formats",
                )
            })?;
        self.resize(self.size);
        Ok(())
    }
}

async fn select_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    enabled_backends: wgpu::Backends,
    config: RhiConfig,
) -> Result<wgpu::Adapter, RhiError> {
    instance
        .enumerate_adapters(enabled_backends)
        .await
        .into_iter()
        .filter(|adapter| adapter.is_surface_supported(surface))
        .filter(|adapter| {
            config.allow_software_adapter || adapter.get_info().device_type != wgpu::DeviceType::Cpu
        })
        .max_by_key(|adapter| {
            let info = adapter.get_info();
            adapter_score(
                adapter_kind(info.device_type),
                backend(info.backend),
                config,
            )
        })
        .ok_or_else(|| {
            RhiError::new(
                RhiErrorKind::AdapterUnavailable,
                "no presentation-capable GPU adapter matched the RHI configuration",
            )
        })
}

fn adapter_score(kind: AdapterKind, candidate_backend: Backend, config: RhiConfig) -> u32 {
    let power_score = match (config.power_preference, kind) {
        (PowerPreference::HighPerformance, AdapterKind::Discrete)
        | (PowerPreference::LowPower, AdapterKind::Integrated) => 500,
        (PowerPreference::HighPerformance, AdapterKind::Integrated)
        | (PowerPreference::LowPower, AdapterKind::Discrete) => 400,
        (_, AdapterKind::Virtual) => 200,
        (_, AdapterKind::Other) => 100,
        (_, AdapterKind::Software) => 10,
    };
    let backend_score = u32::from(config.preferred_backend == Some(candidate_backend)) * 1_000;
    power_score + backend_score
}

fn make_surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: WindowSize,
    capabilities: &wgpu::SurfaceCapabilities,
    config: RhiConfig,
) -> Result<wgpu::SurfaceConfiguration, RhiError> {
    let width = size.width.max(1);
    let height = size.height.max(1);
    let mut surface_config = surface
        .get_default_config(adapter, width, height)
        .ok_or_else(|| {
            RhiError::new(
                RhiErrorKind::SurfaceUnsupported,
                "selected adapter cannot configure the native surface",
            )
        })?;
    surface_config.format = choose_surface_format(&capabilities.formats).ok_or_else(|| {
        RhiError::new(
            RhiErrorKind::SurfaceUnsupported,
            "native surface reports no texture formats",
        )
    })?;
    surface_config.present_mode = match config.present_policy {
        PresentPolicy::Vsync => wgpu::PresentMode::AutoVsync,
        PresentPolicy::AllowTearing => wgpu::PresentMode::AutoNoVsync,
    };
    surface_config.desired_maximum_frame_latency = config.desired_maximum_frame_latency.clamp(1, 4);
    if capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC) {
        surface_config.usage |= wgpu::TextureUsages::COPY_SRC;
    }
    Ok(surface_config)
}

fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| formats.first().copied())
}

fn build_clear_graph() -> Result<CompiledRenderGraph, RhiError> {
    let mut graph = RenderGraphBuilder::new();
    let swapchain = graph.add_resource(ResourceDescriptor::imported(
        "swapchain",
        ResourceKind::Texture,
    ));
    graph.add_pass("clear", [], [swapchain], []);
    graph
        .compile()
        .map_err(|error| RhiError::new(RhiErrorKind::RenderGraph, error.to_string()))
}

fn create_depth_buffer_for_device(
    device: &wgpu::Device,
    size: WindowSize,
    format: DepthFormat,
    label: &str,
) -> DepthBuffer {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.wgpu_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    DepthBuffer {
        _texture: texture,
        view,
        size,
        format,
    }
}

fn create_timestamp_query_state(
    device: &wgpu::Device,
    timestamp_period_ns: f32,
) -> TimestampQueryState {
    let slots = (0..TIMESTAMP_READBACK_SLOT_COUNT)
        .map(|slot_index| {
            let query_label = format!("Meridian pass timestamps {slot_index}");
            let resolve_label = format!("Meridian timestamp resolve {slot_index}");
            let readback_label = format!("Meridian timestamp readback {slot_index}");
            TimestampReadbackSlot {
                query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some(&query_label),
                    ty: wgpu::QueryType::Timestamp,
                    count: 2,
                }),
                resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&resolve_label),
                    size: 16,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                readback_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&readback_label),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                generation: 0,
                in_flight: None,
            }
        })
        .collect();
    let (sender, receiver) = mpsc::channel();
    TimestampQueryState {
        slots,
        timestamp_period_ns,
        sender,
        receiver,
    }
}

fn create_capture_state() -> CaptureState {
    let (sender, receiver) = mpsc::channel();
    CaptureState {
        slots: (0..CAPTURE_READBACK_SLOT_COUNT)
            .map(|_| CaptureReadbackSlot {
                buffer: None,
                buffer_size: 0,
                generation: 0,
                in_flight: None,
            })
            .collect(),
        sender,
        receiver,
        pending: VecDeque::with_capacity(CAPTURE_REQUEST_CAPACITY),
        results: VecDeque::with_capacity(CAPTURE_RESULT_CAPACITY),
        dropped_results: 0,
    }
}

fn capture_source_format(format: wgpu::TextureFormat) -> Option<CaptureSourceFormat> {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb => Some(CaptureSourceFormat::Rgba8Srgb),
        wgpu::TextureFormat::Bgra8UnormSrgb => Some(CaptureSourceFormat::Bgra8Srgb),
        _ => None,
    }
}

fn capture_layout(
    size: WindowSize,
    request: CaptureRequest,
) -> Result<CaptureLayout, CaptureFailure> {
    if size.is_zero() {
        return Err(CaptureFailure::ZeroExtent);
    }
    if size.width > request.max_width || size.height > request.max_height {
        return Err(CaptureFailure::DimensionLimit);
    }
    let tight_bytes_per_row = size
        .width
        .checked_mul(4)
        .ok_or(CaptureFailure::SizeOverflow)?;
    let padded_bytes_per_row = tight_bytes_per_row
        .checked_add(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)
        .ok_or(CaptureFailure::SizeOverflow)?
        / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer_size = u64::from(padded_bytes_per_row)
        .checked_mul(u64::from(size.height))
        .ok_or(CaptureFailure::SizeOverflow)?;
    if buffer_size > request.max_bytes {
        return Err(CaptureFailure::ByteLimit);
    }
    Ok(CaptureLayout {
        padded_bytes_per_row,
        buffer_size,
    })
}

fn record_capture_copy(
    encoder: &mut wgpu::CommandEncoder,
    source_texture: &wgpu::Texture,
    state: &CaptureState,
    pending: PendingCapture,
) {
    let Some(buffer) = state
        .slots
        .get(pending.slot_index)
        .and_then(|slot| slot.buffer.as_ref())
    else {
        return;
    };
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: source_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pending.correlation.padded_bytes_per_row),
                rows_per_image: Some(pending.correlation.height),
            },
        },
        wgpu::Extent3d {
            width: pending.correlation.width,
            height: pending.correlation.height,
            depth_or_array_layers: 1,
        },
    );
}

fn read_mapped_capture(buffer: &wgpu::Buffer) -> CaptureReadbackResult {
    let Ok(view) = buffer.get_mapped_range(..) else {
        buffer.unmap();
        return CaptureReadbackResult::MappingFailed;
    };
    let bytes = view.to_vec();
    drop(view);
    buffer.unmap();
    CaptureReadbackResult::Bytes(bytes)
}

fn normalize_capture_rows(
    correlation: CaptureCorrelation,
    bytes: &[u8],
) -> Result<Vec<u8>, CaptureFailure> {
    let tight_row = correlation
        .width
        .checked_mul(4)
        .ok_or(CaptureFailure::SizeOverflow)?;
    let expected = u64::from(correlation.padded_bytes_per_row)
        .checked_mul(u64::from(correlation.height))
        .ok_or(CaptureFailure::SizeOverflow)?;
    if u64::try_from(bytes.len()).map_err(|_| CaptureFailure::SizeOverflow)? != expected
        || correlation.padded_bytes_per_row < tight_row
    {
        return Err(CaptureFailure::InvalidRowData);
    }
    let output_size = usize::try_from(
        u64::from(tight_row)
            .checked_mul(u64::from(correlation.height))
            .ok_or(CaptureFailure::SizeOverflow)?,
    )
    .map_err(|_| CaptureFailure::SizeOverflow)?;
    let padded_row = usize::try_from(correlation.padded_bytes_per_row)
        .map_err(|_| CaptureFailure::SizeOverflow)?;
    let tight_row = usize::try_from(tight_row).map_err(|_| CaptureFailure::SizeOverflow)?;
    let mut pixels = Vec::with_capacity(output_size);
    for row in bytes.chunks_exact(padded_row) {
        let row = row.get(..tight_row).ok_or(CaptureFailure::InvalidRowData)?;
        match correlation.source_format {
            CaptureSourceFormat::Rgba8Srgb => pixels.extend_from_slice(row),
            CaptureSourceFormat::Bgra8Srgb => {
                for pixel in row.chunks_exact(4) {
                    pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            }
        }
    }
    if pixels.len() != output_size {
        return Err(CaptureFailure::InvalidRowData);
    }
    Ok(pixels)
}

const fn capture_failure_outcome(
    correlation: CaptureCorrelation,
    failure: CaptureFailure,
) -> CaptureOutcome {
    CaptureOutcome::Inconclusive {
        capture_id: correlation.capture_id,
        frame_id: correlation.frame_id,
        failure,
    }
}

fn take_matching_capture(
    active: &mut Option<CaptureCorrelation>,
    incoming: CaptureCorrelation,
) -> Result<Option<CaptureCorrelation>, CaptureFailure> {
    let Some(current) = *active else {
        return Ok(None);
    };
    if current.generation != incoming.generation || current.capture_id != incoming.capture_id {
        return Err(CaptureFailure::StaleReadback);
    }
    Ok(active.take())
}

fn push_bounded_capture_result(
    results: &mut VecDeque<CaptureOutcome>,
    dropped_results: &mut u64,
    result: CaptureOutcome,
) {
    if results.len() == CAPTURE_RESULT_CAPACITY {
        results.pop_front();
        *dropped_results = dropped_results.saturating_add(1);
    }
    results.push_back(result);
}

fn read_mapped_timestamps(readback_buffer: &wgpu::Buffer) -> TimestampReadbackResult {
    let Ok(view) = readback_buffer.get_mapped_range(..) else {
        readback_buffer.unmap();
        return TimestampReadbackResult::MappingFailed;
    };
    if view.len() != 16 {
        drop(view);
        readback_buffer.unmap();
        return TimestampReadbackResult::MappingFailed;
    }
    let mut begin_bytes = [0; 8];
    let mut end_bytes = [0; 8];
    begin_bytes.copy_from_slice(&view[..8]);
    end_bytes.copy_from_slice(&view[8..16]);
    drop(view);
    readback_buffer.unmap();
    TimestampReadbackResult::Timestamps {
        begin: u64::from_ne_bytes(begin_bytes),
        end: u64::from_ne_bytes(end_bytes),
    }
}

const fn timing_unavailable_outcome(availability: TimingAvailability) -> GpuTimingOutcome {
    match availability {
        TimingAvailability::Available => {
            GpuTimingOutcome::Inconclusive(GpuTimingFailure::ReadbackSaturated)
        }
        TimingAvailability::NotRequested => GpuTimingOutcome::NotRequested,
        TimingAvailability::UnsupportedCapability => GpuTimingOutcome::UnsupportedCapability,
        TimingAvailability::UnsupportedPlatform(failure) => {
            GpuTimingOutcome::UnsupportedPlatform(failure)
        }
        TimingAvailability::Inconclusive(failure) => GpuTimingOutcome::Inconclusive(failure),
    }
}

const fn timing_sample_from_correlation(
    correlation: TimestampCorrelation,
    gpu: GpuTimingOutcome,
) -> PassTimingSample {
    PassTimingSample {
        frame_id: correlation.frame_id,
        runtime_frame_id: correlation.runtime_frame_id,
        submission_id: correlation.submission_id,
        pass: correlation.pass,
        cpu_encode_time: correlation.cpu_encode_time,
        gpu,
    }
}

fn first_free_slot_index(in_flight: impl IntoIterator<Item = bool>) -> Option<usize> {
    in_flight
        .into_iter()
        .enumerate()
        .find_map(|(index, in_flight)| (!in_flight).then_some(index))
}

fn take_matching_correlation(
    active: &mut Option<TimestampCorrelation>,
    incoming: TimestampCorrelation,
) -> Result<Option<TimestampCorrelation>, GpuTimingFailure> {
    let Some(current) = *active else {
        return Ok(None);
    };
    if current.generation != incoming.generation {
        return Err(GpuTimingFailure::StaleReadback);
    }
    Ok(active.take())
}

fn classify_timestamp_readback(
    result: TimestampReadbackResult,
    timestamp_period_ns: f32,
    backend: Backend,
) -> (GpuTimingOutcome, Option<TimingAvailability>) {
    match result {
        TimestampReadbackResult::MappingFailed => (
            GpuTimingOutcome::Inconclusive(GpuTimingFailure::MappingFailed),
            None,
        ),
        TimestampReadbackResult::Timestamps { begin, end } => {
            match timestamp_duration_from_raw(begin, end, timestamp_period_ns) {
                Ok(duration) => (GpuTimingOutcome::Measured(duration), None),
                Err(
                    GpuTimingFailure::ZeroTimestamp
                    | GpuTimingFailure::ZeroDuration
                    | GpuTimingFailure::EndBeforeBegin,
                ) if backend == Backend::Metal => (
                    GpuTimingOutcome::UnsupportedPlatform(
                        GpuTimingFailure::MetalTimestampDataInvalid,
                    ),
                    Some(TimingAvailability::UnsupportedPlatform(
                        GpuTimingFailure::MetalTimestampDataInvalid,
                    )),
                ),
                Err(failure) => {
                    let availability = matches!(
                        failure,
                        GpuTimingFailure::ZeroTimestamp
                            | GpuTimingFailure::ZeroDuration
                            | GpuTimingFailure::EndBeforeBegin
                            | GpuTimingFailure::InvalidTimestampPeriod
                            | GpuTimingFailure::DurationOutOfRange
                    )
                    .then_some(TimingAvailability::Inconclusive(failure));
                    (GpuTimingOutcome::Inconclusive(failure), availability)
                }
            }
        }
    }
}

fn push_bounded_timing_result(
    results: &mut VecDeque<PassTimingSample>,
    dropped_results: &mut u64,
    latest_measured_gpu_duration: &mut Option<Duration>,
    sample: PassTimingSample,
) {
    if let GpuTimingOutcome::Measured(duration) = sample.gpu {
        *latest_measured_gpu_duration = Some(duration);
    }
    if results.len() == TIMING_RESULT_CAPACITY {
        results.pop_front();
        *dropped_results = (*dropped_results).saturating_add(1);
    }
    results.push_back(sample);
}

fn timestamp_duration_from_raw(
    begin: u64,
    end: u64,
    timestamp_period_ns: f32,
) -> Result<Duration, GpuTimingFailure> {
    if begin == 0 || end == 0 {
        return Err(GpuTimingFailure::ZeroTimestamp);
    }
    if begin == end {
        return Err(GpuTimingFailure::ZeroDuration);
    }
    let ticks = end
        .checked_sub(begin)
        .ok_or(GpuTimingFailure::EndBeforeBegin)?;
    timestamp_duration(ticks, timestamp_period_ns)
}

#[allow(clippy::cast_precision_loss)]
fn timestamp_duration(ticks: u64, timestamp_period_ns: f32) -> Result<Duration, GpuTimingFailure> {
    if !timestamp_period_ns.is_finite() || timestamp_period_ns <= 0.0 {
        return Err(GpuTimingFailure::InvalidTimestampPeriod);
    }
    let seconds = (ticks as f64 * f64::from(timestamp_period_ns)) / 1_000_000_000.0;
    if !seconds.is_finite() || seconds < 0.0 || seconds > Duration::MAX.as_secs_f64() {
        return Err(GpuTimingFailure::DurationOutOfRange);
    }
    Ok(Duration::from_secs_f64(seconds))
}

fn gpu_capabilities(
    adapter: &wgpu::Adapter,
    surface: &wgpu::SurfaceCapabilities,
    timestamps_enabled: bool,
) -> GpuCapabilities {
    let info = adapter.get_info();
    let features = adapter.features();
    let limits = adapter.limits();
    let kind = adapter_kind(info.device_type);
    let mut supported_features = BTreeSet::new();
    for (feature, supported) in [
        (
            GpuFeature::IndirectDrawCount,
            features.contains(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT),
        ),
        (
            GpuFeature::MeshShaders,
            features.contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER),
        ),
        (
            GpuFeature::SubgroupOperations,
            features.contains(wgpu::Features::SUBGROUP),
        ),
        (
            GpuFeature::TextureAtomics,
            features.contains(wgpu::Features::TEXTURE_ATOMIC),
        ),
        (
            GpuFeature::RayQueries,
            features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY),
        ),
        (
            GpuFeature::RayTracingPipelines,
            features.contains(wgpu::Features::EXPERIMENTAL_RAY_TRACING_PIPELINES),
        ),
        (
            GpuFeature::BindlessTextures,
            features.contains(wgpu::Features::TEXTURE_BINDING_ARRAY),
        ),
    ] {
        if supported {
            supported_features.insert(feature);
        }
    }
    let timestamp_queries = if timestamps_enabled {
        CapabilityStatus::Enabled
    } else if features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        CapabilityStatus::Supported
    } else {
        CapabilityStatus::Unsupported
    };
    let hdr_surface_formats = if surface.formats.iter().any(|format| {
        matches!(
            format,
            wgpu::TextureFormat::Rgb10a2Unorm
                | wgpu::TextureFormat::Rg11b10Ufloat
                | wgpu::TextureFormat::Rgba16Float
        )
    }) {
        CapabilityStatus::Supported
    } else {
        CapabilityStatus::Unsupported
    };
    GpuCapabilities {
        adapter_name: info.name,
        driver: info.driver,
        driver_info: info.driver_info,
        vendor_id: info.vendor,
        device_id: info.device,
        backend: backend(info.backend),
        adapter_kind: kind,
        memory_class: match kind {
            AdapterKind::Discrete => MemoryClass::Discrete,
            AdapterKind::Integrated => MemoryClass::Unified,
            AdapterKind::Virtual | AdapterKind::Software | AdapterKind::Other => {
                MemoryClass::Unknown
            }
        },
        features: supported_features,
        timestamp_queries,
        max_sampled_textures_per_shader_stage: limits.max_sampled_textures_per_shader_stage,
        hdr_surface_formats,
    }
}

const fn backend(value: wgpu::Backend) -> Backend {
    match value {
        wgpu::Backend::Noop => Backend::Noop,
        wgpu::Backend::Vulkan => Backend::Vulkan,
        wgpu::Backend::Metal => Backend::Metal,
        wgpu::Backend::Dx12 => Backend::Direct3D12,
        wgpu::Backend::Gl => Backend::OpenGl,
        wgpu::Backend::BrowserWebGpu => Backend::BrowserWebGpu,
    }
}

const fn adapter_kind(value: wgpu::DeviceType) -> AdapterKind {
    match value {
        wgpu::DeviceType::DiscreteGpu => AdapterKind::Discrete,
        wgpu::DeviceType::IntegratedGpu => AdapterKind::Integrated,
        wgpu::DeviceType::VirtualGpu => AdapterKind::Virtual,
        wgpu::DeviceType::Cpu => AdapterKind::Software,
        wgpu::DeviceType::Other => AdapterKind::Other,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RhiErrorKind {
    SurfaceCreation,
    SurfaceUnsupported,
    AdapterUnavailable,
    DeviceCreation,
    DeviceLost,
    SurfaceValidation,
    RenderGraph,
    InvalidBufferSize,
    InvalidBufferWrite,
    InvalidDraw,
    InvalidDepthSize,
    BindGroupCreation,
    PipelineCreation,
    InvalidTextureSize,
    InvalidTextureMipLevels,
    InvalidTextureWrite,
    InvalidShadowMapSize,
    InvalidShadowCascade,
    TimestampReadback,
    TimingFrameState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RhiError {
    kind: RhiErrorKind,
    message: String,
}

impl RhiError {
    fn new(kind: RhiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> RhiErrorKind {
        self.kind
    }
}

impl Display for RhiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for RhiError {}

fn wgpu_buffer_usage(usage: BufferUsage) -> wgpu::BufferUsages {
    match usage {
        BufferUsage::Vertex => wgpu::BufferUsages::VERTEX,
        BufferUsage::Index => wgpu::BufferUsages::INDEX,
        BufferUsage::Uniform => wgpu::BufferUsages::UNIFORM,
        BufferUsage::Storage => wgpu::BufferUsages::STORAGE,
        BufferUsage::Indirect => wgpu::BufferUsages::INDIRECT,
    }
}

fn validate_buffer_write(size: u64, offset: u64, data_len: usize) -> Result<(), String> {
    let data_len = u64::try_from(data_len).map_err(|_| "buffer write is too large".to_owned())?;
    if !offset.is_multiple_of(4) || !data_len.is_multiple_of(4) {
        return Err("buffer write offset and size must be four-byte aligned".to_owned());
    }
    let end = offset
        .checked_add(data_len)
        .ok_or_else(|| "buffer write range overflowed".to_owned())?;
    if end > size {
        return Err(format!(
            "buffer write range {offset}..{end} exceeds buffer size {size}"
        ));
    }
    Ok(())
}

fn validate_indexed_draw(
    vertex_buffer: &GpuBuffer,
    index_buffer: &GpuBuffer,
    index_count: u32,
) -> Result<(), RhiError> {
    if vertex_buffer.usage != BufferUsage::Vertex {
        return Err(RhiError::new(
            RhiErrorKind::InvalidDraw,
            "indexed draw requires a vertex buffer",
        ));
    }
    if index_buffer.usage != BufferUsage::Index {
        return Err(RhiError::new(
            RhiErrorKind::InvalidDraw,
            "indexed draw requires an index buffer",
        ));
    }
    let required_bytes = u64::from(index_count)
        .checked_mul(4)
        .ok_or_else(|| RhiError::new(RhiErrorKind::InvalidDraw, "index count overflowed"))?;
    if index_count == 0 || required_bytes > index_buffer.size {
        return Err(RhiError::new(
            RhiErrorKind::InvalidDraw,
            format!(
                "indexed draw needs {required_bytes} index bytes but buffer has {}",
                index_buffer.size
            ),
        ));
    }
    Ok(())
}

fn validate_texture_write(
    size: WindowSize,
    mip_level_count: u32,
    format: TextureFormat,
    mip_level: u32,
    bytes_per_row: u32,
    data_len: usize,
) -> Result<WindowSize, String> {
    if mip_level >= mip_level_count {
        return Err(format!(
            "texture mip level {mip_level} is outside 0..{mip_level_count}"
        ));
    }
    let mip_size = WindowSize::new(
        (size.width >> mip_level).max(1),
        (size.height >> mip_level).max(1),
    );
    let bytes_per_pixel = u64::from(format.bytes_per_pixel());
    let row_bytes = u64::from(mip_size.width) * bytes_per_pixel;
    let bytes_per_row = u64::from(bytes_per_row);
    if bytes_per_row < row_bytes || !bytes_per_row.is_multiple_of(bytes_per_pixel) {
        return Err(format!(
            "texture row stride {bytes_per_row} cannot hold {row_bytes} bytes"
        ));
    }
    let required_bytes = bytes_per_row
        .checked_mul(u64::from(mip_size.height.saturating_sub(1)))
        .and_then(|value| value.checked_add(row_bytes))
        .ok_or_else(|| "texture upload size overflowed".to_owned())?;
    if u64::try_from(data_len).map_or(true, |length| length < required_bytes) {
        return Err(format!(
            "texture upload has {data_len} bytes but needs {required_bytes}"
        ));
    }
    Ok(mip_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing_correlation(generation: u64) -> TimestampCorrelation {
        TimestampCorrelation {
            generation,
            frame_id: TimingFrameId(41),
            runtime_frame_id: Some(FrameId::new(42)),
            submission_id: 73,
            pass: PassTimingLabel::new("test_pass"),
            cpu_encode_time: Duration::from_micros(125),
        }
    }

    fn capture_correlation(generation: u64, format: CaptureSourceFormat) -> CaptureCorrelation {
        CaptureCorrelation {
            generation,
            capture_id: CaptureId(5),
            frame_id: FrameId::new(7),
            width: 2,
            height: 2,
            padded_bytes_per_row: 256,
            source_format: format,
            source: CaptureSource::Offscreen,
            surface_outcome: None,
        }
    }

    #[test]
    fn capture_layout_enforces_alignment_dimensions_bytes_and_zero_extent() {
        let request = CaptureRequest::new(FrameId::new(1), 64, 64, 64 * 64 * 4 + 16_384);
        let layout = capture_layout(WindowSize::new(2, 2), request).expect("layout valid");
        assert_eq!(layout.padded_bytes_per_row, 256);
        assert_eq!(layout.buffer_size, 512);
        assert_eq!(
            capture_layout(WindowSize::new(0, 2), request),
            Err(CaptureFailure::ZeroExtent)
        );
        assert_eq!(
            capture_layout(WindowSize::new(65, 2), request),
            Err(CaptureFailure::DimensionLimit)
        );
        let tiny = CaptureRequest::new(FrameId::new(1), 64, 64, 1);
        assert_eq!(
            capture_layout(WindowSize::new(2, 2), tiny),
            Err(CaptureFailure::ByteLimit)
        );
        let overflow = CaptureRequest::new(FrameId::new(1), u32::MAX, 1, u64::MAX);
        assert_eq!(
            capture_layout(WindowSize::new(u32::MAX, 1), overflow),
            Err(CaptureFailure::SizeOverflow)
        );
    }

    #[test]
    fn capture_rows_strip_padding_and_convert_bgra_to_rgba() {
        let mut bytes = vec![0_u8; 512];
        bytes[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        bytes[256..264].copy_from_slice(&[9, 10, 11, 12, 13, 14, 15, 16]);
        let rgba = normalize_capture_rows(
            capture_correlation(1, CaptureSourceFormat::Bgra8Srgb),
            &bytes,
        )
        .expect("rows normalize");
        assert_eq!(
            rgba,
            vec![3, 2, 1, 4, 7, 6, 5, 8, 11, 10, 9, 12, 15, 14, 13, 16]
        );
        assert_eq!(
            normalize_capture_rows(
                capture_correlation(1, CaptureSourceFormat::Rgba8Srgb),
                &bytes[..511],
            ),
            Err(CaptureFailure::InvalidRowData)
        );
    }

    #[test]
    fn capture_generation_saturation_failures_and_result_overflow_stay_typed() {
        let current = capture_correlation(2, CaptureSourceFormat::Rgba8Srgb);
        let mut active = Some(current);
        assert_eq!(
            take_matching_capture(
                &mut active,
                capture_correlation(1, CaptureSourceFormat::Rgba8Srgb)
            ),
            Err(CaptureFailure::StaleReadback)
        );
        assert_eq!(active, Some(current));
        assert_eq!(first_free_slot_index([true, true, true]), None);

        let mut results = VecDeque::new();
        let mut dropped = 0;
        for id in 0..=CAPTURE_RESULT_CAPACITY {
            push_bounded_capture_result(
                &mut results,
                &mut dropped,
                CaptureOutcome::Inconclusive {
                    capture_id: CaptureId(u64::try_from(id).expect("fits")),
                    frame_id: FrameId::new(1),
                    failure: if id == 0 {
                        CaptureFailure::MappingFailed
                    } else {
                        CaptureFailure::DeviceLost
                    },
                },
            );
        }
        assert_eq!(results.len(), CAPTURE_RESULT_CAPACITY);
        assert_eq!(dropped, 1);
        assert!(matches!(
            results.back(),
            Some(CaptureOutcome::Inconclusive {
                failure: CaptureFailure::DeviceLost,
                ..
            })
        ));
    }

    #[test]
    fn clear_graph_contains_one_clear_pass() {
        let graph = build_clear_graph().expect("clear graph should compile");
        let names = graph
            .ordered_passes()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();

        assert_eq!(names, ["clear"]);
    }

    #[test]
    fn surface_format_prefers_srgb() {
        let formats = [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
        ];

        assert_eq!(
            choose_surface_format(&formats),
            Some(wgpu::TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn adapter_scoring_respects_power_and_backend_preferences() {
        let config = RhiConfig {
            preferred_backend: Some(Backend::Vulkan),
            ..RhiConfig::default()
        };

        assert!(
            adapter_score(AdapterKind::Integrated, Backend::Vulkan, config)
                > adapter_score(AdapterKind::Discrete, Backend::Metal, config)
        );

        let no_backend_preference = RhiConfig::default();
        assert!(
            adapter_score(AdapterKind::Discrete, Backend::Metal, no_backend_preference)
                > adapter_score(
                    AdapterKind::Integrated,
                    Backend::Metal,
                    no_backend_preference
                )
        );
    }

    #[test]
    fn frame_latency_is_clamped_to_a_small_runtime_window() {
        assert_eq!(0_u32.clamp(1, 4), 1);
        assert_eq!(9_u32.clamp(1, 4), 4);
    }

    #[test]
    fn buffer_usage_maps_to_backend_flags_without_exposing_them() {
        assert!(wgpu_buffer_usage(BufferUsage::Vertex).contains(wgpu::BufferUsages::VERTEX));
        assert!(wgpu_buffer_usage(BufferUsage::Storage).contains(wgpu::BufferUsages::STORAGE));
        assert!(wgpu_buffer_usage(BufferUsage::Indirect).contains(wgpu::BufferUsages::INDIRECT));
    }

    #[test]
    fn depth_formats_and_stencil_contract_are_backend_neutral() {
        assert!(!DepthFormat::Depth32Float.has_stencil());
        assert!(!DepthFormat::Depth24Plus.has_stencil());
        assert!(DepthFormat::Depth24PlusStencil8.has_stencil());
        assert_eq!(DepthFormat::default(), DepthFormat::Depth32Float);
        assert_eq!(
            DepthFormat::Depth32Float.wgpu_format(),
            wgpu::TextureFormat::Depth32Float
        );
    }

    #[test]
    fn zero_sized_depth_attachments_are_rejected_before_device_work() {
        assert!(WindowSize::new(0, 720).is_zero());
        assert!(WindowSize::new(1280, 0).is_zero());
    }

    #[test]
    fn buffer_write_validation_rejects_misaligned_and_out_of_bounds_ranges() {
        assert!(validate_buffer_write(16, 4, 8).is_ok());
        assert!(validate_buffer_write(16, 2, 8).is_err());
        assert!(validate_buffer_write(16, 12, 8).is_err());
        assert!(validate_buffer_write(16, 16, 0).is_ok());
    }

    #[test]
    fn timestamp_duration_converts_ticks_and_rejects_invalid_periods() {
        assert_eq!(
            timestamp_duration(2, 500_000.0).expect("valid timestamp period"),
            Duration::from_millis(1)
        );
        assert_eq!(
            timestamp_duration(1, 0.0).expect_err("zero period is invalid"),
            GpuTimingFailure::InvalidTimestampPeriod
        );
        assert_eq!(
            timestamp_duration(u64::MAX, f32::MAX).expect_err("overflowing duration is invalid"),
            GpuTimingFailure::DurationOutOfRange
        );
    }

    #[test]
    fn raw_timestamps_reject_zero_reversed_and_equal_values() {
        assert_eq!(
            timestamp_duration_from_raw(0, 0, 1.0),
            Err(GpuTimingFailure::ZeroTimestamp)
        );
        assert_eq!(
            timestamp_duration_from_raw(0, 8, 1.0),
            Err(GpuTimingFailure::ZeroTimestamp)
        );
        assert_eq!(
            timestamp_duration_from_raw(9, 8, 1.0),
            Err(GpuTimingFailure::EndBeforeBegin)
        );
        assert_eq!(
            timestamp_duration_from_raw(9, 9, 1.0),
            Err(GpuTimingFailure::ZeroDuration)
        );
    }

    #[test]
    fn metal_invalid_timestamps_disable_gpu_timing_for_the_rhi_lifetime() {
        for result in [
            TimestampReadbackResult::Timestamps { begin: 0, end: 0 },
            TimestampReadbackResult::Timestamps { begin: 2, end: 1 },
            TimestampReadbackResult::Timestamps { begin: 2, end: 2 },
        ] {
            let (outcome, availability) = classify_timestamp_readback(result, 1.0, Backend::Metal);
            assert_eq!(
                outcome,
                GpuTimingOutcome::UnsupportedPlatform(GpuTimingFailure::MetalTimestampDataInvalid)
            );
            assert_eq!(
                availability,
                Some(TimingAvailability::UnsupportedPlatform(
                    GpuTimingFailure::MetalTimestampDataInvalid
                ))
            );
        }
    }

    #[test]
    fn map_failure_is_inconclusive_without_disabling_future_queries() {
        assert_eq!(
            classify_timestamp_readback(
                TimestampReadbackResult::MappingFailed,
                1.0,
                Backend::Vulkan
            ),
            (
                GpuTimingOutcome::Inconclusive(GpuTimingFailure::MappingFailed),
                None
            )
        );
    }

    #[test]
    fn timestamp_slot_selection_reports_saturation() {
        assert_eq!(
            first_free_slot_index([true; TIMESTAMP_READBACK_SLOT_COUNT]),
            None
        );
        assert_eq!(
            first_free_slot_index([true, true, false, true, true, true, true, true]),
            Some(2)
        );
    }

    #[test]
    fn stale_generation_does_not_release_active_slot() {
        let current = timing_correlation(2);
        let stale = timing_correlation(1);
        let mut active = Some(current);

        assert_eq!(
            take_matching_correlation(&mut active, stale),
            Err(GpuTimingFailure::StaleReadback)
        );
        assert_eq!(active, Some(current));
        assert_eq!(
            take_matching_correlation(&mut active, current),
            Ok(Some(current))
        );
        assert_eq!(active, None);
    }

    #[test]
    fn timing_sample_preserves_frame_submission_pass_and_device_loss() {
        let correlation = timing_correlation(7);
        let sample = timing_sample_from_correlation(
            correlation,
            GpuTimingOutcome::Inconclusive(GpuTimingFailure::DeviceLost),
        );

        assert_eq!(sample.frame_id, TimingFrameId(41));
        assert_eq!(sample.submission_id, 73);
        assert_eq!(sample.pass, PassTimingLabel::new("test_pass"));
        assert_eq!(sample.cpu_encode_time, Duration::from_micros(125));
        assert_eq!(
            sample.gpu,
            GpuTimingOutcome::Inconclusive(GpuTimingFailure::DeviceLost)
        );
    }

    #[test]
    fn result_queue_is_bounded_and_tracks_latest_measurement() {
        let mut results = VecDeque::new();
        let mut dropped = 0;
        let mut latest = None;

        for submission_id in 1..=u64::try_from(TIMING_RESULT_CAPACITY + 1).unwrap() {
            push_bounded_timing_result(
                &mut results,
                &mut dropped,
                &mut latest,
                PassTimingSample {
                    frame_id: TimingFrameId(1),
                    runtime_frame_id: None,
                    submission_id,
                    pass: PassTimingLabel::new("bounded"),
                    cpu_encode_time: Duration::ZERO,
                    gpu: GpuTimingOutcome::Measured(Duration::from_nanos(submission_id)),
                },
            );
        }

        assert_eq!(results.len(), TIMING_RESULT_CAPACITY);
        assert_eq!(results.front().map(|sample| sample.submission_id), Some(2));
        assert_eq!(dropped, 1);
        assert_eq!(latest, Some(Duration::from_nanos(65)));
    }

    #[test]
    fn unavailable_timing_states_keep_cpu_samples_typed() {
        for (availability, expected) in [
            (
                TimingAvailability::NotRequested,
                GpuTimingOutcome::NotRequested,
            ),
            (
                TimingAvailability::UnsupportedCapability,
                GpuTimingOutcome::UnsupportedCapability,
            ),
        ] {
            let sample = timing_sample_from_correlation(
                timing_correlation(1),
                timing_unavailable_outcome(availability),
            );
            assert_eq!(sample.cpu_encode_time, Duration::from_micros(125));
            assert_eq!(sample.gpu, expected);
        }
    }

    #[test]
    fn texture_formats_and_mip_upload_validation_are_backend_neutral() {
        assert!(TextureFormat::Rgba8UnormSrgb.is_srgb());
        assert!(!TextureFormat::Rgba8Unorm.is_srgb());
        assert_eq!(TextureFormat::Rgba16Float.bytes_per_pixel(), 8);
        assert_eq!(
            validate_texture_write(
                WindowSize::new(4, 4),
                2,
                TextureFormat::Rgba8UnormSrgb,
                1,
                8,
                16,
            ),
            Ok(WindowSize::new(2, 2))
        );
        assert!(validate_texture_write(
            WindowSize::new(4, 4),
            2,
            TextureFormat::Rgba8UnormSrgb,
            2,
            4,
            4,
        )
        .is_err());
        assert!(validate_texture_write(
            WindowSize::new(4, 4),
            1,
            TextureFormat::Rgba8UnormSrgb,
            0,
            4,
            4,
        )
        .is_err());
    }

    #[test]
    fn vertex_layout_validation_accepts_position_and_rejects_bad_attributes() {
        let layout = VertexLayout::new(12, [VertexAttribute::new(VertexFormat::Float32x3, 0, 0)])
            .expect("position layout is valid");
        assert_eq!(layout.stride(), 12);
        assert_eq!(layout.attributes()[0].shader_location(), 0);

        assert!(matches!(
            VertexLayout::new(10, [VertexAttribute::new(VertexFormat::Float32x3, 0, 0)]),
            Err(VertexLayoutError::InvalidStride(10))
        ));
        assert!(matches!(
            VertexLayout::new(
                16,
                [
                    VertexAttribute::new(VertexFormat::Float32x2, 0, 0),
                    VertexAttribute::new(VertexFormat::Float32x2, 8, 0),
                ]
            ),
            Err(VertexLayoutError::DuplicateShaderLocation(0))
        ));
        assert!(matches!(
            VertexLayout::new(12, [VertexAttribute::new(VertexFormat::Float32x3, 4, 0)]),
            Err(VertexLayoutError::AttributeOutsideStride { .. })
        ));
    }
}
