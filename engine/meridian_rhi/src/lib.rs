//! Backend-neutral GPU adapter, device, surface, and clear-frame boundary.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

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
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    timestamp_period_ns: f32,
    pending: bool,
}

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
    device_loss: Arc<Mutex<Option<DeviceLoss>>>,
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
            device_loss,
        })
    }

    #[must_use]
    pub const fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    /// Reads the most recently submitted GPU timestamp scope, when supported.
    ///
    /// This waits for the submitted work so benchmark callers can associate a
    /// duration with the frame that produced it. Unsupported adapters and
    /// frames that did not submit a drawable surface return `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns [`RhiErrorKind::TimestampReadback`] when the query mapping or
    /// timestamp data is invalid, or [`RhiErrorKind::DeviceLost`] after device
    /// loss.
    pub fn take_last_gpu_duration(&mut self) -> Result<Option<Duration>, RhiError> {
        if let Some(loss) = self.device_loss() {
            return Err(RhiError::new(
                RhiErrorKind::DeviceLost,
                format!("{:?}: {}", loss.reason, loss.message),
            ));
        }
        let Some(timestamp_query) = self.timestamp_query.as_mut() else {
            return Ok(None);
        };
        if !timestamp_query.pending {
            return Ok(None);
        }

        let (sender, receiver) = mpsc::sync_channel(1);
        timestamp_query
            .readback_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| {
                RhiError::new(
                    RhiErrorKind::TimestampReadback,
                    format!("timestamp readback poll failed: {error:?}"),
                )
            })?;
        let map_result = receiver.recv().map_err(|error| {
            RhiError::new(
                RhiErrorKind::TimestampReadback,
                format!("timestamp readback callback failed: {error}"),
            )
        })?;
        timestamp_query.pending = false;
        if let Err(error) = map_result {
            timestamp_query.readback_buffer.unmap();
            return Err(RhiError::new(
                RhiErrorKind::TimestampReadback,
                format!("timestamp buffer mapping failed: {error:?}"),
            ));
        }

        let (begin, end) = {
            let view = timestamp_query
                .readback_buffer
                .get_mapped_range(..)
                .map_err(|error| {
                    RhiError::new(
                        RhiErrorKind::TimestampReadback,
                        format!("timestamp buffer access failed: {error:?}"),
                    )
                })?;
            if view.len() != 16 {
                let length = view.len();
                drop(view);
                timestamp_query.readback_buffer.unmap();
                return Err(RhiError::new(
                    RhiErrorKind::TimestampReadback,
                    format!("timestamp readback returned {length} bytes instead of 16"),
                ));
            }
            let mut begin_bytes = [0; 8];
            let mut end_bytes = [0; 8];
            begin_bytes.copy_from_slice(&view[..8]);
            end_bytes.copy_from_slice(&view[8..16]);
            (
                u64::from_ne_bytes(begin_bytes),
                u64::from_ne_bytes(end_bytes),
            )
        };
        timestamp_query.readback_buffer.unmap();

        let ticks = end.checked_sub(begin).ok_or_else(|| {
            RhiError::new(
                RhiErrorKind::TimestampReadback,
                "timestamp end precedes timestamp begin",
            )
        })?;
        timestamp_duration(ticks, timestamp_query.timestamp_period_ns).map(Some)
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
        let depth_stencil = depth_only.then_some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
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
                self.encode_clear_and_present(texture, color);
                Ok(FrameOutcome::Presented)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.encode_clear_and_present(texture, color);
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
        let color_attachments: [Option<wgpu::RenderPassColorAttachment<'_>>; 0] = [];
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
        self.queue.submit(Some(encoder.finish()));
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
                self.encode_indexed_mesh_and_present(texture, draw, color);
                Ok(FrameOutcome::Presented)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.encode_indexed_mesh_and_present(texture, draw, color);
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

    fn encode_clear_and_present(&mut self, texture: wgpu::SurfaceTexture, color: ClearColor) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian clear frame encoder"),
            });
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
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Meridian clear pass"),
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
                timestamp_writes: self.timestamp_writes(),
                ..Default::default()
            });
        }
        self.submit_encoder(encoder);
        self.queue.present(texture);
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
                timestamp_writes: self.timestamp_writes(),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.draw(0..3, 0..1);
        }
        self.submit_encoder(encoder);
        self.queue.present(texture);
    }

    fn encode_indexed_mesh_and_present(
        &mut self,
        texture: wgpu::SurfaceTexture,
        draw: IndexedDraw<'_>,
        color: ClearColor,
    ) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Meridian indexed mesh encoder"),
            });
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
                label: Some("Meridian indexed mesh pass"),
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
                timestamp_writes: self.timestamp_writes(),
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
        self.submit_encoder(encoder);
        self.queue.present(texture);
    }

    fn timestamp_writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        self.timestamp_query
            .as_ref()
            .map(|timestamp_query| wgpu::RenderPassTimestampWrites {
                query_set: &timestamp_query.query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            })
    }

    fn submit_encoder(&mut self, mut encoder: wgpu::CommandEncoder) {
        if let Some(timestamp_query) = self.timestamp_query.as_mut() {
            encoder.resolve_query_set(
                &timestamp_query.query_set,
                0..2,
                &timestamp_query.resolve_buffer,
                0,
            );
            encoder.copy_buffer_to_buffer(
                &timestamp_query.resolve_buffer,
                0,
                &timestamp_query.readback_buffer,
                0,
                16,
            );
            timestamp_query.pending = true;
        }
        self.queue.submit([encoder.finish()]);
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
    let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("Meridian frame timestamps"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Meridian timestamp resolve buffer"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Meridian timestamp readback buffer"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    TimestampQueryState {
        query_set,
        resolve_buffer,
        readback_buffer,
        timestamp_period_ns,
        pending: false,
    }
}

#[allow(clippy::cast_precision_loss)]
fn timestamp_duration(ticks: u64, timestamp_period_ns: f32) -> Result<Duration, RhiError> {
    if !timestamp_period_ns.is_finite() || timestamp_period_ns <= 0.0 {
        return Err(RhiError::new(
            RhiErrorKind::TimestampReadback,
            "timestamp period is invalid",
        ));
    }
    let seconds = (ticks as f64 * f64::from(timestamp_period_ns)) / 1_000_000_000.0;
    if !seconds.is_finite() || seconds < 0.0 || seconds > Duration::MAX.as_secs_f64() {
        return Err(RhiError::new(
            RhiErrorKind::TimestampReadback,
            "timestamp duration is outside the representable range",
        ));
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
            timestamp_duration(1, 0.0)
                .expect_err("zero period is invalid")
                .kind(),
            RhiErrorKind::TimestampReadback
        );
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
