//! Direct Penumbra UI display-list preparation.
//!
//! This path consumes Meridian's retained display list directly. It prepares
//! bounded GPU geometry/resource batches from the immutable frame contract; it
//! does not rasterize the whole UI into a CPU framebuffer.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use meridian_platform::WindowSize;
use meridian_rhi::{
    BufferUsage, ClearColor, FrameOutcome, GpuBuffer, GpuRenderPipeline, GpuRenderTarget,
    GpuTexture, GpuTextureBindGroup, PipelineStencilConfig, RenderPipelineBindings,
    RenderPipelineConfig, RenderScissor, RenderTargetLoadPolicy, Rhi, RhiError, RhiErrorKind,
    RhiRenderBatch, RhiRenderIdentity, TextureFormat, VertexAttribute, VertexFormat, VertexLayout,
    VertexLayoutError,
};
use meridian_ui_core::{UiColor, UiContrast, UiPoint, UiRect, UiSize, MAX_DISPLAY_PRIMITIVES};
use meridian_ui_render::{
    validate_backdrop_resolution_at_scale, DisplayList, DisplayListError, DisplayPrimitive,
    ResolvedBackdrop, UiBackdropDescriptor, UiBackdropValidationError, UiClipId, UiCornerRadii,
    UiEffectCapabilities, UiImageHandle, UiLayerId, UiLineCap, UiLineJoin, UiMeshHandle,
    UiPathCommand, UiStroke, MAX_PATH_COMMANDS_PER_PRIMITIVE,
};
use meridian_ui_text::{UiGlyphBitmap, UiTextRaster, MAX_GLYPH_RASTER_BYTES};

const ROUND_STROKE_SEGMENTS: u32 = 12;
const SHADOW_FALLOFF_WEIGHTS: [f32; 4] = [0.10, 0.15, 0.25, 0.50];
const VERTEX_STRIDE_BYTES: u64 = 32;
const VERTEX_STRIDE_BYTES_USIZE: usize = 32;
const INDEX_STRIDE_BYTES: u64 = 4;
const INDEX_STRIDE_BYTES_USIZE: usize = 4;
const MAX_DIRECT_BATCHES: usize = MAX_DISPLAY_PRIMITIVES;
const MAX_DIRECT_GEOMETRY_BYTES: u64 = MAX_GLYPH_RASTER_BYTES as u64;
const MAX_DIRECT_IMAGE_BYTES: u64 = MAX_DIRECT_GEOMETRY_BYTES;
const MAX_PATH_TESSELLATION_WORK: usize = MAX_PATH_COMMANDS_PER_PRIMITIVE * 16;
// Reuse the recovery bridge's established 16,777,216-pixel full-frame service
// guard. Direct atlas uploads and layer targets share that aggregate RGBA
// safety envelope so this path does not invent uncalibrated memory limits.
const MAX_DIRECT_RGBA_SERVICE_BYTES: u64 = 16 * 1024 * 1024 * 4;
const MAX_DIRECT_ATLAS_BYTES: u64 = MAX_DIRECT_RGBA_SERVICE_BYTES;
const MAX_DIRECT_LAYER_TARGET_BYTES: u64 = MAX_DIRECT_RGBA_SERVICE_BYTES;
const UI_DIRECT_SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var ui_atlas: texture_2d<f32>;
@group(0) @binding(1) var ui_sampler: sampler;

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sample = textureSample(ui_atlas, ui_sampler, input.tex_coord);
    let alpha = input.color.a * sample.a;
    return vec4<f32>(input.color.rgb * sample.rgb * alpha, alpha);
}
";

const UI_DIRECT_COMPOSITE_SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) opacity: f32,
};

@group(0) @binding(0) var layer_texture: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.opacity = input.color.a;
    return output;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sample = textureSample(layer_texture, layer_sampler, input.tex_coord);
    return sample * input.opacity;
}
";

const UI_DIRECT_BACKDROP_SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@group(0) @binding(0) var backdrop_texture: texture_2d<f32>;
@group(0) @binding(1) var backdrop_sampler: sampler;

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.tint = input.tint;
    return output;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(backdrop_texture));
    let texel = vec2<f32>(1.0, 1.0) / dimensions;
    let uv = input.tex_coord;
    var blurred = textureSample(backdrop_texture, backdrop_sampler, uv) * 4.0;
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(-texel.x, 0.0)) * 2.0;
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(texel.x, 0.0)) * 2.0;
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(0.0, -texel.y)) * 2.0;
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(0.0, texel.y)) * 2.0;
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(-texel.x, -texel.y));
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(texel.x, -texel.y));
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(-texel.x, texel.y));
    blurred += textureSample(backdrop_texture, backdrop_sampler, uv + vec2<f32>(texel.x, texel.y));
    blurred /= 16.0;
    let tint = vec4<f32>(input.tint.rgb * input.tint.a, input.tint.a);
    return tint + blurred * (1.0 - input.tint.a);
}
";

type UiDirectFrameResources = (
    Vec<UiDirectBatch>,
    Vec<u8>,
    Vec<u8>,
    UiDirectAtlas,
    Vec<UiDirectLayerPass>,
    Vec<UiDirectBackdropPass>,
);

/// Resource authority supplied by the renderer host for one immutable UI frame.
///
/// Handles are process-local UI cache identities; the renderer never treats
/// them as source-document authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiDirectResourceSet {
    pub image_revision: u64,
    pub mesh_revision: u64,
    images: BTreeMap<UiImageHandle, UiDirectImage>,
    meshes: BTreeMap<UiMeshHandle, UiDirectMesh>,
}

impl UiDirectResourceSet {
    #[must_use]
    pub const fn new(image_revision: u64, mesh_revision: u64) -> Self {
        Self {
            image_revision,
            mesh_revision,
            images: BTreeMap::new(),
            meshes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_image(mut self, image: UiImageHandle) -> Self {
        self.images
            .insert(image, UiDirectImage::single_white(image));
        self
    }

    #[must_use]
    pub fn with_mesh(mut self, mesh: UiMeshHandle) -> Self {
        self.meshes.insert(mesh, UiDirectMesh::unit_quad(mesh));
        self
    }

    #[must_use]
    pub fn with_image_descriptor(mut self, image: UiDirectImage) -> Self {
        self.images.insert(image.handle, image);
        self
    }

    #[must_use]
    pub fn with_mesh_descriptor(mut self, mesh: UiDirectMesh) -> Self {
        self.meshes.insert(mesh.handle, mesh);
        self
    }

    #[must_use]
    pub fn contains_image(&self, image: UiImageHandle) -> bool {
        self.images.contains_key(&image)
    }

    #[must_use]
    pub fn contains_mesh(&self, mesh: UiMeshHandle) -> bool {
        self.meshes.contains_key(&mesh)
    }

    #[must_use]
    pub fn image(&self, image: UiImageHandle) -> Option<&UiDirectImage> {
        self.images.get(&image)
    }

    #[must_use]
    pub fn mesh(&self, mesh: UiMeshHandle) -> Option<&UiDirectMesh> {
        self.meshes.get(&mesh)
    }
}

/// Bounded image resource accepted by the direct UI adapter.
///
/// `rgba` is row-major, unpremultiplied RGBA8 with sRGB color channels and a
/// linear alpha channel. The direct shader converts sampled sRGB channels to
/// linear light through the sRGB texture view and premultiplies exactly once
/// before blending. Transparent source RGB is therefore never compositor
/// authority and cannot create a second-alpha edge halo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectImage {
    pub handle: UiImageHandle,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl UiDirectImage {
    #[must_use]
    pub fn single_white(handle: UiImageHandle) -> Self {
        Self {
            handle,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        }
    }

    /// Creates a bounded solid RGBA image descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`UiDirectRendererError::InvalidImage`] for zero dimensions or
    /// [`UiDirectRendererError::TooManyImageBytes`] before allocating an
    /// oversized pixel buffer.
    pub fn try_solid(
        handle: UiImageHandle,
        width: u32,
        height: u32,
        rgba: [u8; 4],
    ) -> Result<Self, UiDirectRendererError> {
        let texel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(UiDirectRendererError::InvalidImage(handle))?;
        let Some(byte_count) = texel_count.checked_mul(4) else {
            return Err(UiDirectRendererError::InvalidImage(handle));
        };
        let byte_count_u64 =
            u64::try_from(byte_count).map_err(|_| UiDirectRendererError::InvalidImage(handle))?;
        if width == 0 || height == 0 {
            return Err(UiDirectRendererError::InvalidImage(handle));
        }
        if byte_count_u64 > MAX_DIRECT_IMAGE_BYTES {
            return Err(UiDirectRendererError::TooManyImageBytes {
                bytes: byte_count_u64,
                maximum: MAX_DIRECT_IMAGE_BYTES,
            });
        }
        let mut pixels = Vec::with_capacity(byte_count);
        for _ in 0..texel_count {
            pixels.extend_from_slice(&rgba);
        }
        Ok(Self {
            handle,
            width,
            height,
            rgba: pixels,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDirectMeshVertex {
    pub x_milli: u16,
    pub y_milli: u16,
    pub u_milli: u16,
    pub v_milli: u16,
}

impl UiDirectMeshVertex {
    #[must_use]
    pub const fn new(x_milli: u16, y_milli: u16, u_milli: u16, v_milli: u16) -> Self {
        Self {
            x_milli,
            y_milli,
            u_milli,
            v_milli,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectMesh {
    pub handle: UiMeshHandle,
    pub vertices: Vec<UiDirectMeshVertex>,
    pub indices: Vec<u32>,
}

impl UiDirectMesh {
    #[must_use]
    pub fn unit_quad(handle: UiMeshHandle) -> Self {
        Self {
            handle,
            vertices: vec![
                UiDirectMeshVertex::new(0, 0, 0, 0),
                UiDirectMeshVertex::new(1000, 0, 1000, 0),
                UiDirectMeshVertex::new(1000, 1000, 1000, 1000),
                UiDirectMeshVertex::new(0, 1000, 0, 1000),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }
}

/// Direct UI renderer cache invalidation required before GPU resource reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiDirectRendererRecoveryAction {
    None,
    RebuildSurfaceCaches,
    RebuildDeviceCaches,
}

/// Typed report proving immutable frame authority survives renderer recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDirectRendererRecovery {
    pub action: UiDirectRendererRecoveryAction,
    pub preserved_revision: u64,
    pub dropped_cache_count: u32,
}

/// Kind of direct GPU batch prepared from a display primitive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UiDirectBatchKind {
    Content,
    GlyphMask,
    Image,
    Mesh,
    ClipPush,
    ClipPop,
    Layer,
    Shadow,
    BackdropFallback,
    BackdropEffect,
}

/// Renderer-neutral primitive categories measured by the direct UI path.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UiDirectPrimitiveKind {
    Rect,
    Border,
    Text,
    GlyphRun,
    FocusIndicator,
    RoundedRect,
    Path,
    Image,
    Mesh,
    PushClip,
    PopClip,
    BeginLayer,
    EndLayer,
    Shadow,
    Backdrop,
}

/// Renderer-neutral direct batch metadata.
///
/// The concrete [`RhiRenderBatch`] is built after GPU buffers/pipelines exist;
/// this metadata is deterministic and testable without a surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectBatch {
    pub kind: UiDirectBatchKind,
    pub primitive: UiDirectPrimitiveKind,
    pub vertex_range: Range<u32>,
    pub index_range: Range<u32>,
    pub scissor: Option<RenderScissor>,
    pub stencil_reference: u32,
}

/// Diagnostics emitted by direct display-list preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectFrameDiagnostics {
    pub display_revision: u64,
    pub primitive_count: usize,
    pub observed_kinds: BTreeSet<UiDirectPrimitiveKind>,
    pub prepared_kinds: BTreeSet<UiDirectPrimitiveKind>,
    pub unsupported_kinds: BTreeSet<UiDirectPrimitiveKind>,
    pub preparation_only_kinds: BTreeSet<UiDirectPrimitiveKind>,
    pub batch_count: usize,
    pub vertex_count: u32,
    pub index_count: u32,
    pub glyph_mask_count: usize,
    pub image_count: usize,
    pub mesh_count: usize,
    pub clip_scope_count: usize,
    pub layer_count: usize,
    pub layer_target_bytes: u64,
    pub shadow_count: usize,
    pub backdrop_fallback_count: usize,
    pub backdrop_effect_count: usize,
    pub full_frame_cpu_rasterized: bool,
}

/// Exact payload accounting for one prepared direct-UI frame.
///
/// These values describe immutable CPU bytes, upload payload bytes, and
/// planned color-target bytes.  They do not claim backend allocation size,
/// cache residency, driver memory, or peak process memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDirectFrameFootprint {
    /// CPU-owned packed vertex bytes before GPU upload.
    pub cpu_vertex_bytes: u64,
    /// CPU-owned packed index bytes before GPU upload.
    pub cpu_index_bytes: u64,
    /// CPU-owned RGBA atlas bytes before GPU upload.
    pub cpu_atlas_bytes: u64,
    /// Sum of vertex, index, and atlas payload bytes requested for upload.
    pub gpu_upload_payload_bytes: u64,
    /// Planned full-size isolated layer and backdrop color-target bytes.
    pub planned_color_target_bytes: u64,
    /// Display primitive count before direct preparation.
    pub primitive_count: usize,
    /// Prepared indexed batch count.
    pub batch_count: usize,
    /// Isolated compositing layer count, excluding the root pass.
    pub layer_count: usize,
    /// Prepared shadow primitive count.
    pub shadow_count: usize,
    /// Backdrop effects realized through the bounded filter path.
    pub backdrop_effect_count: usize,
    /// Backdrops resolved to their required opaque fallback.
    pub backdrop_fallback_count: usize,
}

/// Bounded RGBA atlas produced by direct UI preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectAtlas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UiDirectPassBatch {
    batch_index: usize,
    source_layer: Option<usize>,
    backdrop_source: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UiDirectBackdropPass {
    consumer_pass: usize,
    source_pass: usize,
    source_batch_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UiDirectLayerPass {
    id: Option<UiLayerId>,
    opacity_bits: u32,
    children: Vec<usize>,
    batches: Vec<UiDirectPassBatch>,
}

impl UiDirectLayerPass {
    fn root() -> Self {
        Self {
            id: None,
            opacity_bits: 1.0_f32.to_bits(),
            children: Vec::new(),
            batches: Vec::new(),
        }
    }

    fn layer(id: UiLayerId, opacity: f32) -> Self {
        Self {
            id: Some(id),
            opacity_bits: opacity.to_bits(),
            children: Vec::new(),
            batches: Vec::new(),
        }
    }

    fn opacity(&self) -> f32 {
        f32::from_bits(self.opacity_bits)
    }
}

/// Prepared direct frame authority and deterministic batch plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectFramePlan {
    cache_key: UiDirectCacheKey,
    rhi_identity: RhiRenderIdentity,
    recovery: UiDirectRendererRecovery,
    diagnostics: UiDirectFrameDiagnostics,
    batches: Vec<UiDirectBatch>,
    vertex_bytes: Vec<u8>,
    index_bytes: Vec<u8>,
    atlas: UiDirectAtlas,
    layer_passes: Vec<UiDirectLayerPass>,
    backdrop_passes: Vec<UiDirectBackdropPass>,
}

/// Device-owned direct UI submission resources for one prepared frame.
pub struct UiDirectGpuFrame {
    cache_key: UiDirectCacheKey,
    rhi_identity: RhiRenderIdentity,
    layer_plan_fingerprint: u64,
    vertex_bytes_len: usize,
    index_bytes_len: usize,
    atlas_size: (u32, u32),
    content_pipeline: GpuRenderPipeline,
    composite_pipeline: GpuRenderPipeline,
    backdrop_pipeline: GpuRenderPipeline,
    clip_push_pipeline: GpuRenderPipeline,
    clip_pop_pipeline: GpuRenderPipeline,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    _atlas_texture: GpuTexture,
    atlas_bind_group: GpuTextureBindGroup,
    clip_push_atlas_bind_group: GpuTextureBindGroup,
    clip_pop_atlas_bind_group: GpuTextureBindGroup,
    layer_targets: Vec<GpuRenderTarget>,
    layer_bind_groups: Vec<GpuTextureBindGroup>,
    backdrop_targets: Vec<GpuRenderTarget>,
    backdrop_bind_groups: Vec<GpuTextureBindGroup>,
}

struct UiDirectPipelines {
    content: GpuRenderPipeline,
    composite: GpuRenderPipeline,
    backdrop: GpuRenderPipeline,
    clip_push: GpuRenderPipeline,
    clip_pop: GpuRenderPipeline,
}

/// Borrowed inputs for direct UI frame preparation.
#[derive(Clone, Copy, Debug)]
pub struct UiDirectPrepareRequest<'a> {
    pub display_revision: u64,
    pub display_list: &'a DisplayList,
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub contrast: UiContrast,
    pub effects: UiEffectCapabilities,
    pub resources: &'a UiDirectResourceSet,
}

/// Pixel-affecting renderer cache key for direct UI resources.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UiDirectCacheKey {
    /// Caller-provided immutable UI snapshot revision.
    pub display_revision: u64,
    /// RHI device generation used to prepare this frame.
    pub device_generation: u64,
    /// RHI surface generation used to prepare this frame.
    pub surface_generation: u64,
    /// Physical width of the prepared direct-UI target.
    pub surface_width: u32,
    /// Physical height of the prepared direct-UI target.
    pub surface_height: u32,
    /// Exact active RHI surface width, independent of the UI target viewport.
    pub rhi_surface_width: u32,
    /// Exact active RHI surface height, independent of the UI target viewport.
    pub rhi_surface_height: u32,
    /// Stable cache-only fingerprint of the active RHI surface format.
    pub surface_format_fingerprint: u64,
    /// Whether the active RHI surface format exposes an sRGB view.
    pub surface_format_srgb: bool,
    /// Whether the active RHI surface was configured when the frame prepared.
    pub surface_configured: bool,
    /// Rounded direct-UI display scale in thousandths.
    pub scale_milli: u16,
    /// Resolved high-contrast mode.
    pub contrast_high: bool,
    /// Host-provided image cache revision.
    pub image_revision: u64,
    /// Host-provided mesh cache revision.
    pub mesh_revision: u64,
    /// Resolved optional effect capability profile.
    pub effect_profile: u64,
    /// Deterministic prepared payload and draw-plan cache fingerprint.
    ///
    /// This guards process-local GPU upload reuse only; it is not a source or
    /// evidence integrity hash.
    pub content_fingerprint: u64,
}

/// Direct UI renderer state. GPU handles stay outside public Meridian API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDirectGpuRenderer {
    identity: RhiRenderIdentity,
    cached_surface_resources: u32,
    cached_device_resources: u32,
    last_revision: Option<u64>,
}

impl UiDirectGpuRenderer {
    #[must_use]
    pub fn new(identity: RhiRenderIdentity) -> Self {
        Self {
            identity,
            cached_surface_resources: 0,
            cached_device_resources: 0,
            last_revision: None,
        }
    }

    #[must_use]
    pub const fn last_revision(&self) -> Option<u64> {
        self.last_revision
    }

    pub fn record_cache_rebuild(&mut self, surface_resources: u32, device_resources: u32) {
        self.cached_surface_resources = surface_resources;
        self.cached_device_resources = device_resources;
    }

    /// Prepares one immutable UI frame for direct GPU submission.
    ///
    /// # Errors
    ///
    /// Returns typed display-list, resource, geometry, and bound failures
    /// before backend resource reuse or draw submission.
    pub fn prepare_frame(
        &mut self,
        request: UiDirectPrepareRequest<'_>,
    ) -> Result<UiDirectFramePlan, UiDirectRendererError> {
        request
            .display_list
            .validate()
            .map_err(UiDirectRendererError::InvalidDisplayList)?;
        validate_viewport(request.viewport, request.scale_factor)?;
        let mut cache_key = UiDirectCacheKey::new(
            request.display_revision,
            &self.identity,
            request.viewport,
            request.scale_factor,
            request.contrast,
            request.effects,
            request.resources,
        )?;
        let target_bytes_per_target = layer_target_bytes_per_target(WindowSize::new(
            cache_key.surface_width,
            cache_key.surface_height,
        ))?;
        let atlas = atlas_builder_for(request.display_list, request.resources)?;
        let mut builder =
            DirectBatchBuilder::new(request.display_revision, atlas, target_bytes_per_target)?;
        let context = DirectFrameContext {
            viewport: request.viewport,
            scale_factor: request.scale_factor,
            contrast: request.contrast,
            effects: request.effects,
            resources: request.resources,
        };
        for primitive in &request.display_list.primitives {
            builder.push_primitive(primitive, context)?;
        }
        let diagnostics = builder.diagnostics();
        if !diagnostics.unsupported_kinds.is_empty() {
            return Err(UiDirectRendererError::UnsupportedPrimitiveKind {
                kinds: diagnostics.unsupported_kinds,
            });
        }
        let (batches, vertex_bytes, index_bytes, atlas, layer_passes, backdrop_passes) =
            builder.finish()?;
        cache_key.content_fingerprint = prepared_frame_content_fingerprint(
            &vertex_bytes,
            &index_bytes,
            &atlas,
            &batches,
            &layer_passes,
            &backdrop_passes,
            diagnostics.layer_target_bytes,
        );
        let recovery = self.prepare_recovery(request.display_revision);
        Ok(UiDirectFramePlan {
            cache_key,
            rhi_identity: self.identity.clone(),
            recovery,
            diagnostics,
            batches,
            vertex_bytes,
            index_bytes,
            atlas,
            layer_passes,
            backdrop_passes,
        })
    }

    fn prepare_recovery(&mut self, display_revision: u64) -> UiDirectRendererRecovery {
        self.last_revision = Some(display_revision);
        UiDirectRendererRecovery {
            action: UiDirectRendererRecoveryAction::None,
            preserved_revision: display_revision,
            dropped_cache_count: 0,
        }
    }

    /// Drops caches after the host reports a fresh RHI identity.
    #[must_use]
    pub fn recover_identity(
        &mut self,
        identity: RhiRenderIdentity,
        display_revision: u64,
    ) -> UiDirectRendererRecovery {
        let action = if identity.device_generation != self.identity.device_generation {
            UiDirectRendererRecoveryAction::RebuildDeviceCaches
        } else if identity.surface_generation != self.identity.surface_generation
            || identity.surface_format != self.identity.surface_format
            || identity.surface_size != self.identity.surface_size
            || identity.surface_configured != self.identity.surface_configured
        {
            UiDirectRendererRecoveryAction::RebuildSurfaceCaches
        } else {
            UiDirectRendererRecoveryAction::None
        };
        let dropped_cache_count = match action {
            UiDirectRendererRecoveryAction::None => 0,
            UiDirectRendererRecoveryAction::RebuildSurfaceCaches => {
                let dropped = self.cached_surface_resources;
                self.cached_surface_resources = 0;
                dropped
            }
            UiDirectRendererRecoveryAction::RebuildDeviceCaches => {
                let dropped = self
                    .cached_surface_resources
                    .saturating_add(self.cached_device_resources);
                self.cached_surface_resources = 0;
                self.cached_device_resources = 0;
                dropped
            }
        };
        self.identity = identity;
        self.last_revision = Some(display_revision);
        UiDirectRendererRecovery {
            action,
            preserved_revision: display_revision,
            dropped_cache_count,
        }
    }
}

impl UiDirectCacheKey {
    fn new(
        display_revision: u64,
        identity: &RhiRenderIdentity,
        viewport: UiSize,
        scale_factor: f32,
        contrast: UiContrast,
        effects: UiEffectCapabilities,
        resources: &UiDirectResourceSet,
    ) -> Result<Self, UiDirectRendererError> {
        if !identity.surface_format.srgb {
            return Err(UiDirectRendererError::UnsupportedSurfaceColorSpace);
        }
        Ok(Self {
            display_revision,
            device_generation: identity.device_generation,
            surface_generation: identity.surface_generation,
            surface_width: logical_to_pixels(viewport.width, scale_factor)?,
            surface_height: logical_to_pixels(viewport.height, scale_factor)?,
            rhi_surface_width: identity.surface_size.width,
            rhi_surface_height: identity.surface_size.height,
            surface_format_fingerprint: surface_format_fingerprint(&identity.surface_format.name),
            surface_format_srgb: identity.surface_format.srgb,
            surface_configured: identity.surface_configured,
            scale_milli: scale_milli(scale_factor)?,
            contrast_high: contrast == UiContrast::High,
            image_revision: resources.image_revision,
            mesh_revision: resources.mesh_revision,
            effect_profile: u64::from(effects.backdrop_filtering),
            content_fingerprint: 0,
        })
    }
}

/// Error raised before a direct UI frame can be submitted.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UiDirectRendererError {
    InvalidDisplayList(DisplayListError),
    InvalidBackdropEffect(UiBackdropValidationError),
    InvalidViewport,
    UnsupportedSurfaceColorSpace,
    /// The final qualification target cannot support its required copy-source
    /// capability on this active RHI profile.
    ///
    /// This is distinct from presentation-surface availability: normal direct
    /// rendering may remain usable when the optional offscreen comparison path
    /// cannot be captured.
    OffscreenCaptureUnsupported {
        /// RHI reason that made the copy-source target unavailable.
        rhi_kind: RhiErrorKind,
    },
    InvalidImage(UiImageHandle),
    InvalidMesh(UiMeshHandle),
    MissingImage(UiImageHandle),
    MissingMesh(UiMeshHandle),
    TooManyPrimitives {
        count: usize,
        maximum: usize,
    },
    TooManyBatches {
        count: usize,
        maximum: usize,
    },
    TooManyGeometryBytes {
        bytes: u64,
        maximum: u64,
    },
    TooManyImageBytes {
        bytes: u64,
        maximum: u64,
    },
    TooManyAtlasBytes {
        bytes: u64,
        maximum: u64,
    },
    TooManyLayerTargetBytes {
        bytes: u64,
        maximum: u64,
    },
    ClipDepthOverflow {
        depth: u32,
        maximum: u32,
    },
    FullyClippedRequiredPrimitive(UiDirectPrimitiveKind),
    /// A text or glyph-run primitive has an incomplete raster payload for its
    /// immutable layout.
    IncompleteTextRaster(UiDirectPrimitiveKind),
    UnsupportedPathGeometry,
    InvalidShadowSpread,
    PathTessellationBudgetExceeded {
        work: usize,
        maximum: usize,
    },
    StaleGpuFrame {
        uploaded_fingerprint: u64,
        requested_fingerprint: u64,
    },
    StaleRhiIdentity {
        expected_device_generation: u64,
        actual_device_generation: u64,
        expected_surface_generation: u64,
        actual_surface_generation: u64,
        expected_surface_format: String,
        actual_surface_format: String,
        expected_surface_format_srgb: bool,
        actual_surface_format_srgb: bool,
        expected_surface_width: u32,
        actual_surface_width: u32,
        expected_surface_height: u32,
        actual_surface_height: u32,
        expected_surface_configured: bool,
        actual_surface_configured: bool,
    },
    InvalidVertexLayout(VertexLayoutError),
    InvalidLayerPlan,
    GeometryOverflow,
    UnsupportedPrimitiveKind {
        kinds: BTreeSet<UiDirectPrimitiveKind>,
    },
    Rhi(RhiError),
}

impl Display for UiDirectRendererError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDisplayList(error) => {
                write!(formatter, "invalid UI display list: {error}")
            }
            Self::InvalidBackdropEffect(error) => {
                write!(formatter, "invalid UI backdrop effect: {error}")
            }
            Self::InvalidViewport => formatter.write_str("UI viewport must be finite and non-zero"),
            Self::UnsupportedSurfaceColorSpace => formatter.write_str(
                "direct UI rendering requires an sRGB surface format for deterministic color",
            ),
            Self::OffscreenCaptureUnsupported { rhi_kind } => write!(
                formatter,
                "direct UI offscreen qualification capture requires an unavailable copy-source target capability ({rhi_kind:?})"
            ),
            Self::InvalidImage(handle) => write!(formatter, "invalid UI image resource {handle:?}"),
            Self::InvalidMesh(handle) => write!(formatter, "invalid UI mesh resource {handle:?}"),
            Self::MissingImage(handle) => write!(formatter, "missing UI image resource {handle:?}"),
            Self::MissingMesh(handle) => write!(formatter, "missing UI mesh resource {handle:?}"),
            Self::TooManyPrimitives { count, maximum } => {
                write!(
                    formatter,
                    "UI display list has {count} primitives; maximum is {maximum}"
                )
            }
            Self::TooManyBatches { count, maximum } => {
                write!(
                    formatter,
                    "direct UI frame has {count} batches; maximum is {maximum}"
                )
            }
            Self::TooManyGeometryBytes { bytes, maximum } => {
                fmt_direct_geometry_limit(formatter, *bytes, *maximum)
            }
            Self::TooManyImageBytes { bytes, maximum } => {
                fmt_direct_image_limit(formatter, *bytes, *maximum)
            }
            Self::TooManyAtlasBytes { bytes, maximum } => {
                fmt_direct_atlas_limit(formatter, *bytes, *maximum)
            }
            Self::TooManyLayerTargetBytes { bytes, maximum } => {
                write!(
                    formatter,
                    "direct UI layer targets require {bytes} bytes; aggregate maximum is {maximum}"
                )
            }
            Self::ClipDepthOverflow { depth, maximum } => {
                write!(
                    formatter,
                    "direct UI clip depth {depth} exceeds stencil maximum {maximum}"
                )
            }
            Self::FullyClippedRequiredPrimitive(kind) => {
                write!(
                    formatter,
                    "direct UI required primitive is fully clipped: {kind:?}"
                )
            }
            Self::IncompleteTextRaster(kind) => write!(
                formatter,
                "direct UI {kind:?} has an incomplete glyph raster and cannot be submitted as a complete frame"
            ),
            Self::UnsupportedPathGeometry => formatter.write_str(
                "direct UI path is degenerate, self-intersecting, or uses an unsupported fill topology",
            ),
            Self::InvalidShadowSpread => {
                formatter.write_str("direct UI shadow spread must be finite and non-negative")
            }
            Self::PathTessellationBudgetExceeded { work, maximum } => write!(
                formatter,
                "direct UI path tessellation requires more than {work} work units; maximum is {maximum}"
            ),
            Self::StaleGpuFrame {
                uploaded_fingerprint,
                requested_fingerprint,
            } => write!(
                formatter,
                "direct UI GPU frame fingerprint {uploaded_fingerprint:#018x} does not match requested plan {requested_fingerprint:#018x}"
            ),
            Self::StaleRhiIdentity { .. } => self.fmt_stale_rhi_identity(formatter),
            Self::InvalidVertexLayout(error) => {
                write!(formatter, "invalid direct UI vertex layout: {error}")
            }
            Self::InvalidLayerPlan => {
                formatter.write_str("direct UI layer pass plan is inconsistent")
            }
            Self::GeometryOverflow => formatter.write_str("direct UI geometry range overflowed"),
            Self::UnsupportedPrimitiveKind { kinds } => {
                write!(
                    formatter,
                    "direct UI renderer rejected primitive kinds: {kinds:?}"
                )
            }
            Self::Rhi(error) => write!(formatter, "direct UI RHI failure: {error}"),
        }
    }
}

fn fmt_direct_byte_limit(
    formatter: &mut Formatter<'_>,
    subject: &str,
    bytes: u64,
    maximum: u64,
    limit: &str,
) -> fmt::Result {
    write!(
        formatter,
        "direct UI {subject} {bytes} bytes; {limit} is {maximum}"
    )
}

fn fmt_direct_geometry_limit(
    formatter: &mut Formatter<'_>,
    bytes: u64,
    maximum: u64,
) -> fmt::Result {
    fmt_direct_byte_limit(formatter, "geometry has", bytes, maximum, "maximum")
}

fn fmt_direct_image_limit(formatter: &mut Formatter<'_>, bytes: u64, maximum: u64) -> fmt::Result {
    fmt_direct_byte_limit(formatter, "image has", bytes, maximum, "per-image maximum")
}

fn fmt_direct_atlas_limit(formatter: &mut Formatter<'_>, bytes: u64, maximum: u64) -> fmt::Result {
    fmt_direct_byte_limit(
        formatter,
        "atlas requires",
        bytes,
        maximum,
        "aggregate maximum",
    )
}

impl Error for UiDirectRendererError {}

impl From<UiBackdropValidationError> for UiDirectRendererError {
    fn from(error: UiBackdropValidationError) -> Self {
        Self::InvalidBackdropEffect(error)
    }
}

impl UiDirectRendererError {
    fn fmt_stale_rhi_identity(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let Self::StaleRhiIdentity {
            expected_device_generation,
            actual_device_generation,
            expected_surface_generation,
            actual_surface_generation,
            expected_surface_format,
            actual_surface_format,
            expected_surface_format_srgb,
            actual_surface_format_srgb,
            expected_surface_width,
            actual_surface_width,
            expected_surface_height,
            actual_surface_height,
            expected_surface_configured,
            actual_surface_configured,
        } = self
        else {
            return Err(fmt::Error);
        };
        write!(
            formatter,
            "direct UI plan expected RHI device/surface generations {expected_device_generation}/{expected_surface_generation}, format {expected_surface_format} (sRGB {expected_surface_format_srgb}), size {expected_surface_width}x{expected_surface_height}, configured {expected_surface_configured}; got {actual_device_generation}/{actual_surface_generation}, format {actual_surface_format} (sRGB {actual_surface_format_srgb}), size {actual_surface_width}x{actual_surface_height}, configured {actual_surface_configured}"
        )
    }

    #[must_use]
    pub const fn rhi_kind(&self) -> Option<RhiErrorKind> {
        match self {
            Self::OffscreenCaptureUnsupported { rhi_kind } => Some(*rhi_kind),
            Self::Rhi(error) => Some(error.kind()),
            _ => None,
        }
    }
}

impl From<RhiError> for UiDirectRendererError {
    fn from(error: RhiError) -> Self {
        Self::Rhi(error)
    }
}

fn map_offscreen_capture_target_error(error: RhiError) -> UiDirectRendererError {
    if is_offscreen_capture_target_capability_error(error.kind()) {
        UiDirectRendererError::OffscreenCaptureUnsupported {
            rhi_kind: error.kind(),
        }
    } else {
        UiDirectRendererError::Rhi(error)
    }
}

const fn is_offscreen_capture_target_capability_error(kind: RhiErrorKind) -> bool {
    matches!(kind, RhiErrorKind::SurfaceUnsupported)
}

#[derive(Clone, Copy)]
struct DirectFrameContext<'a> {
    viewport: UiSize,
    scale_factor: f32,
    contrast: UiContrast,
    effects: UiEffectCapabilities,
    resources: &'a UiDirectResourceSet,
}

#[derive(Clone, Copy)]
struct UiDirectVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
    color: [f32; 4],
}

impl UiDirectVertex {
    fn encode(self, bytes: &mut Vec<u8>) {
        for value in [
            self.position[0],
            self.position[1],
            self.tex_coord[0],
            self.tex_coord[1],
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AtlasRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl AtlasRegion {
    #[allow(clippy::cast_precision_loss)]
    fn uv(self, atlas_width: u32, atlas_height: u32) -> [[f32; 2]; 4] {
        let left = (self.x as f32 + 0.5) / atlas_width as f32;
        let top = (self.y as f32 + 0.5) / atlas_height as f32;
        let right = (self.x.saturating_add(self.width) as f32 - 0.5) / atlas_width as f32;
        let bottom = (self.y.saturating_add(self.height) as f32 - 0.5) / atlas_height as f32;
        [[left, top], [right, top], [right, bottom], [left, bottom]]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScissorDecision {
    Draw(Option<RenderScissor>),
    Skip,
}

struct AtlasBuilder {
    width: u32,
    cursor_y: u32,
    rows: Vec<AtlasRow>,
    content_rows: BTreeMap<u64, Vec<usize>>,
}

struct AtlasRow {
    region: AtlasRegion,
    rgba: Vec<u8>,
}

fn atlas_content_fingerprint(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let state = fingerprint_mix(FRAME_FINGERPRINT_OFFSET, u64::from(width));
    let state = fingerprint_mix(state, u64::from(height));
    fingerprint_bytes(state, rgba)
}

impl AtlasBuilder {
    fn new(width: u32) -> Self {
        Self {
            width,
            cursor_y: 0,
            rows: Vec::new(),
            content_rows: BTreeMap::new(),
        }
    }

    fn height(&self) -> u32 {
        self.cursor_y.max(1)
    }

    fn push_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<AtlasRegion, UiDirectRendererError> {
        let Some(expected) = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            return Err(UiDirectRendererError::GeometryOverflow);
        };
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err(UiDirectRendererError::GeometryOverflow);
        }
        let fingerprint = atlas_content_fingerprint(width, height, rgba);
        if let Some(row_indexes) = self.content_rows.get(&fingerprint) {
            for row_index in row_indexes {
                let row = self
                    .rows
                    .get(*row_index)
                    .ok_or(UiDirectRendererError::GeometryOverflow)?;
                if row.region.width == width && row.region.height == height && row.rgba == rgba {
                    return Ok(row.region);
                }
            }
        }
        let padded_width = width
            .checked_add(2)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let padded_height = height
            .checked_add(2)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let next_cursor = self
            .cursor_y
            .checked_add(padded_height)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let padded_bytes = u64::from(self.width)
            .checked_mul(u64::from(next_cursor))
            .and_then(|texels| texels.checked_mul(4))
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        if padded_bytes > MAX_DIRECT_ATLAS_BYTES {
            return Err(UiDirectRendererError::TooManyAtlasBytes {
                bytes: padded_bytes,
                maximum: MAX_DIRECT_ATLAS_BYTES,
            });
        }
        if padded_width > self.width {
            return Err(UiDirectRendererError::GeometryOverflow);
        }
        let region = AtlasRegion {
            x: 1,
            y: self
                .cursor_y
                .checked_add(1)
                .ok_or(UiDirectRendererError::GeometryOverflow)?,
            width,
            height,
        };
        self.cursor_y = next_cursor;
        let row_index = self.rows.len();
        self.rows.push(AtlasRow {
            region,
            rgba: rgba.to_vec(),
        });
        self.content_rows
            .entry(fingerprint)
            .or_default()
            .push(row_index);
        Ok(region)
    }

    fn push_white(&mut self) -> Result<AtlasRegion, UiDirectRendererError> {
        self.push_rgba(1, 1, &[255, 255, 255, 255])
    }

    fn push_glyph(&mut self, glyph: &UiGlyphBitmap) -> Result<AtlasRegion, UiDirectRendererError> {
        let Some(expected) = usize::try_from(glyph.width).ok().and_then(|width| {
            usize::try_from(glyph.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        }) else {
            return Err(UiDirectRendererError::GeometryOverflow);
        };
        if glyph.width == 0 || glyph.height == 0 || glyph.alpha.len() != expected {
            return Err(UiDirectRendererError::GeometryOverflow);
        }
        let Some(rgba_len) = expected.checked_mul(4) else {
            return Err(UiDirectRendererError::GeometryOverflow);
        };
        if u64::try_from(rgba_len).map_or(true, |bytes| bytes > MAX_DIRECT_ATLAS_BYTES) {
            return Err(UiDirectRendererError::TooManyAtlasBytes {
                bytes: u64::try_from(rgba_len)
                    .map_err(|_| UiDirectRendererError::GeometryOverflow)?,
                maximum: MAX_DIRECT_ATLAS_BYTES,
            });
        }
        let mut rgba = Vec::with_capacity(rgba_len);
        for alpha in &glyph.alpha {
            rgba.extend_from_slice(&[255, 255, 255, *alpha]);
        }
        self.push_rgba(glyph.width, glyph.height, &rgba)
    }

    fn finish(self) -> Result<UiDirectAtlas, UiDirectRendererError> {
        let height = self.height();
        let row_bytes = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let total = row_bytes
            .checked_mul(
                usize::try_from(height).map_err(|_| UiDirectRendererError::GeometryOverflow)?,
            )
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let mut rgba = vec![0; total];
        for row in self.rows {
            let region = row.region;
            let source_width = usize::try_from(region.width)
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
            let source_height = usize::try_from(region.height)
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
            let padded_width = source_width
                .checked_add(2)
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            let padded_height = source_height
                .checked_add(2)
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            let destination_x = usize::try_from(region.x.saturating_sub(1))
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
            let destination_y = usize::try_from(region.y.saturating_sub(1))
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
            for padded_y in 0..padded_height {
                let source_y = padded_y.saturating_sub(1).min(source_height - 1);
                for padded_x in 0..padded_width {
                    let source_x = padded_x.saturating_sub(1).min(source_width - 1);
                    let src = source_y
                        .checked_mul(source_width)
                        .and_then(|offset| offset.checked_add(source_x))
                        .and_then(|offset| offset.checked_mul(4))
                        .ok_or(UiDirectRendererError::GeometryOverflow)?;
                    let dst = destination_y
                        .checked_add(padded_y)
                        .and_then(|row_y| row_y.checked_mul(row_bytes))
                        .and_then(|offset| {
                            destination_x
                                .checked_add(padded_x)
                                .and_then(|column| column.checked_mul(4))
                                .and_then(|column| offset.checked_add(column))
                        })
                        .ok_or(UiDirectRendererError::GeometryOverflow)?;
                    rgba[dst..dst + 4].copy_from_slice(&row.rgba[src..src + 4]);
                }
            }
        }
        Ok(UiDirectAtlas {
            width: self.width,
            height,
            rgba,
        })
    }
}

struct DirectBatchBuilder {
    display_revision: u64,
    primitive_count: usize,
    observed_kinds: BTreeSet<UiDirectPrimitiveKind>,
    prepared_kinds: BTreeSet<UiDirectPrimitiveKind>,
    unsupported_kinds: BTreeSet<UiDirectPrimitiveKind>,
    preparation_only_kinds: BTreeSet<UiDirectPrimitiveKind>,
    batches: Vec<UiDirectBatch>,
    vertices: Vec<UiDirectVertex>,
    indices: Vec<u32>,
    atlas: AtlasBuilder,
    white_region: AtlasRegion,
    vertex_count: u32,
    index_count: u32,
    glyph_mask_count: usize,
    image_count: usize,
    mesh_count: usize,
    clip_scope_count: usize,
    layer_count: usize,
    layer_target_bytes: u64,
    target_bytes_per_target: u64,
    shadow_count: usize,
    backdrop_fallback_count: usize,
    backdrop_effect_count: usize,
    active_pass: ActiveLayerPass,
    parent_passes: Vec<ActiveLayerPass>,
    layer_passes: Vec<UiDirectLayerPass>,
    backdrop_passes: Vec<UiDirectBackdropPass>,
    culled_layers: Vec<UiLayerId>,
}

struct ActiveLayerPass {
    pass_index: usize,
    active_stencil_depth: u32,
    clip_stack: Vec<(UiClipId, Option<UiDirectBatch>)>,
    batches: Vec<UiDirectPassBatch>,
    backdrop_parent_prefix: Option<(usize, usize)>,
}

impl ActiveLayerPass {
    fn new(pass_index: usize) -> Self {
        Self {
            pass_index,
            active_stencil_depth: 0,
            clip_stack: Vec::new(),
            batches: Vec::new(),
            backdrop_parent_prefix: None,
        }
    }
}

impl DirectBatchBuilder {
    fn new(
        display_revision: u64,
        mut atlas: AtlasBuilder,
        target_bytes_per_target: u64,
    ) -> Result<Self, UiDirectRendererError> {
        let white_region = atlas.push_white()?;
        Ok(Self {
            display_revision,
            primitive_count: 0,
            observed_kinds: BTreeSet::new(),
            prepared_kinds: BTreeSet::new(),
            unsupported_kinds: BTreeSet::new(),
            preparation_only_kinds: BTreeSet::new(),
            batches: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            atlas,
            white_region,
            vertex_count: 0,
            index_count: 0,
            glyph_mask_count: 0,
            image_count: 0,
            mesh_count: 0,
            clip_scope_count: 0,
            layer_count: 0,
            layer_target_bytes: 0,
            target_bytes_per_target,
            shadow_count: 0,
            backdrop_fallback_count: 0,
            backdrop_effect_count: 0,
            active_pass: ActiveLayerPass::new(0),
            parent_passes: Vec::new(),
            layer_passes: vec![UiDirectLayerPass::root()],
            backdrop_passes: Vec::new(),
            culled_layers: Vec::new(),
        })
    }

    #[allow(clippy::too_many_lines)] // Exhaustive enum dispatcher keeps display-list coverage obvious.
    fn push_primitive(
        &mut self,
        primitive: &DisplayPrimitive,
        context: DirectFrameContext<'_>,
    ) -> Result<(), UiDirectRendererError> {
        if self.batches.len() >= MAX_DISPLAY_PRIMITIVES {
            return Err(UiDirectRendererError::TooManyPrimitives {
                count: self.batches.len().saturating_add(1),
                maximum: MAX_DISPLAY_PRIMITIVES,
            });
        }
        let kind = primitive_kind(primitive);
        self.primitive_count = self.primitive_count.saturating_add(1);
        self.observed_kinds.insert(kind);
        self.prepared_kinds.insert(kind);
        if let Some(active_culled_layer) = self.culled_layers.last().copied() {
            match primitive {
                DisplayPrimitive::BeginLayer { id, .. } => self.culled_layers.push(*id),
                DisplayPrimitive::EndLayer { id } if *id == active_culled_layer => {
                    self.culled_layers.pop();
                }
                DisplayPrimitive::EndLayer { .. } => {
                    return Err(UiDirectRendererError::InvalidLayerPlan);
                }
                _ => {}
            }
            return Ok(());
        }
        if self.has_empty_clip()
            && !matches!(
                primitive,
                DisplayPrimitive::PushClip { .. }
                    | DisplayPrimitive::PopClip { .. }
                    | DisplayPrimitive::BeginLayer { .. }
                    | DisplayPrimitive::EndLayer { .. }
                    | DisplayPrimitive::Backdrop { .. }
            )
        {
            return Ok(());
        }
        match primitive {
            DisplayPrimitive::Rect { bounds, color, .. } => self.push_rect_batch(
                UiDirectBatchKind::Content,
                kind,
                *bounds,
                *color,
                context.viewport,
                context.scale_factor,
            ),
            DisplayPrimitive::Border {
                bounds,
                color,
                width,
                ..
            } => self.push_rect_stroke_batch(
                kind,
                *bounds,
                f32::from(*width).max(1.0),
                *color,
                context.viewport,
                context.scale_factor,
            ),
            DisplayPrimitive::Text {
                bounds,
                layout,
                raster,
                color,
                ..
            }
            | DisplayPrimitive::GlyphRun {
                bounds,
                layout,
                raster,
                color,
                ..
            } => {
                validate_complete_text_raster(kind, layout.glyph_count, raster)?;
                self.push_glyphs(
                    kind,
                    *bounds,
                    raster,
                    *color,
                    context.viewport,
                    context.scale_factor,
                )
            }
            DisplayPrimitive::FocusIndicator { bounds, color, .. } => self.push_rect_stroke_batch(
                kind,
                *bounds,
                3.0,
                *color,
                context.viewport,
                context.scale_factor,
            ),
            DisplayPrimitive::RoundedRect {
                bounds,
                radii,
                color,
                ..
            } => self.push_round_rect_batch(
                kind,
                *bounds,
                *radii,
                *color,
                context.viewport,
                context.scale_factor,
            ),
            DisplayPrimitive::Path {
                commands,
                fill,
                stroke,
                ..
            } => self.push_path(
                kind,
                commands,
                *fill,
                *stroke,
                context.viewport,
                context.scale_factor,
            ),
            DisplayPrimitive::Image {
                bounds,
                image,
                opacity,
                ..
            } => {
                let image_descriptor = context
                    .resources
                    .image(*image)
                    .ok_or(UiDirectRendererError::MissingImage(*image))?;
                self.image_count = self.image_count.saturating_add(1);
                self.push_image_batch(
                    UiDirectBatchKind::Image,
                    kind,
                    *bounds,
                    image_descriptor,
                    *opacity,
                    context.viewport,
                    context.scale_factor,
                )
            }
            DisplayPrimitive::Mesh {
                bounds, mesh, tint, ..
            } => {
                let mesh_descriptor = context
                    .resources
                    .mesh(*mesh)
                    .ok_or(UiDirectRendererError::MissingMesh(*mesh))?;
                self.mesh_count = self.mesh_count.saturating_add(1);
                self.push_mesh_batch(
                    kind,
                    *bounds,
                    mesh_descriptor,
                    *tint,
                    context.viewport,
                    context.scale_factor,
                )
            }
            DisplayPrimitive::PushClip { id, bounds, radii } => {
                self.clip_scope_count = self.clip_scope_count.saturating_add(1);
                if self.has_empty_clip() {
                    self.active_pass.clip_stack.push((*id, None));
                    return Ok(());
                }
                let next_depth = self
                    .active_pass
                    .active_stencil_depth
                    .checked_add(1)
                    .ok_or(UiDirectRendererError::GeometryOverflow)?;
                if next_depth > 255 {
                    return Err(UiDirectRendererError::ClipDepthOverflow {
                        depth: next_depth,
                        maximum: 255,
                    });
                }
                let batch = self.push_round_rect_batch_with(
                    UiDirectBatchKind::ClipPush,
                    kind,
                    *bounds,
                    *radii,
                    UiColor::rgba(1.0, 1.0, 1.0, 0.0),
                    context.viewport,
                    context.scale_factor,
                    self.active_pass.active_stencil_depth,
                )?;
                if batch.is_some() {
                    self.active_pass.active_stencil_depth = next_depth;
                }
                self.active_pass.clip_stack.push((*id, batch));
                Ok(())
            }
            DisplayPrimitive::PopClip { id } => {
                let Some((_, push_batch)) = self
                    .active_pass
                    .clip_stack
                    .pop()
                    .filter(|(active, _)| active == id)
                else {
                    self.unsupported_kinds.insert(kind);
                    return self.push_marker_batch(UiDirectBatchKind::ClipPop, kind);
                };
                if let Some(push_batch) = push_batch {
                    self.push_existing_geometry_batch(
                        UiDirectBatchKind::ClipPop,
                        kind,
                        &push_batch,
                        self.active_pass.active_stencil_depth,
                    )?;
                    self.active_pass.active_stencil_depth =
                        self.active_pass.active_stencil_depth.saturating_sub(1);
                }
                Ok(())
            }
            DisplayPrimitive::BeginLayer { id, opacity } => {
                if self.has_empty_clip() {
                    self.culled_layers.push(*id);
                    return Ok(());
                }
                self.begin_layer(*id, *opacity)
            }
            DisplayPrimitive::EndLayer { id } => {
                self.end_layer(*id, kind, context.viewport, context.scale_factor)
            }
            DisplayPrimitive::Shadow {
                bounds,
                radii,
                offset,
                spread,
                color,
                ..
            } => {
                self.shadow_count = self.shadow_count.saturating_add(1);
                self.push_shadow(
                    kind,
                    *bounds,
                    *radii,
                    *offset,
                    *spread,
                    *color,
                    context.viewport,
                    context.scale_factor,
                )
            }
            DisplayPrimitive::Backdrop { descriptor, .. } => {
                let resolved = validate_backdrop_resolution_at_scale(
                    *descriptor,
                    context.contrast,
                    context.effects,
                    context.scale_factor,
                )?;
                if let ResolvedBackdrop::Effect(effect) = resolved {
                    self.backdrop_effect_count = self.backdrop_effect_count.saturating_add(1);
                    return self.push_backdrop_effect(
                        kind,
                        effect,
                        context.viewport,
                        context.scale_factor,
                    );
                }
                let ResolvedBackdrop::Opaque { bounds, color } = resolved else {
                    return Err(UiDirectRendererError::InvalidLayerPlan);
                };
                if self.has_empty_clip() {
                    return Ok(());
                }
                self.backdrop_fallback_count = self.backdrop_fallback_count.saturating_add(1);
                self.push_rect_batch(
                    UiDirectBatchKind::BackdropFallback,
                    kind,
                    bounds,
                    color,
                    context.viewport,
                    context.scale_factor,
                )
            }
        }
    }

    fn has_empty_clip(&self) -> bool {
        self.active_pass
            .clip_stack
            .iter()
            .any(|(_, batch)| batch.is_none())
    }

    fn begin_layer(&mut self, id: UiLayerId, opacity: f32) -> Result<(), UiDirectRendererError> {
        self.reserve_layer_target()?;
        let pass_index = self.layer_passes.len();
        self.layer_passes
            .push(UiDirectLayerPass::layer(id, opacity));
        self.layer_passes[self.active_pass.pass_index]
            .children
            .push(pass_index);
        let parent_prefix = (self.active_pass.pass_index, self.active_pass.batches.len());
        let mut child = ActiveLayerPass::new(pass_index);
        child.backdrop_parent_prefix = Some(parent_prefix);
        let parent = std::mem::replace(&mut self.active_pass, child);
        self.parent_passes.push(parent);
        self.layer_count = self.layer_count.saturating_add(1);
        Ok(())
    }

    fn reserve_layer_target(&mut self) -> Result<(), UiDirectRendererError> {
        let bytes = self
            .layer_target_bytes
            .checked_add(self.target_bytes_per_target)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        if bytes > MAX_DIRECT_LAYER_TARGET_BYTES {
            return Err(UiDirectRendererError::TooManyLayerTargetBytes {
                bytes,
                maximum: MAX_DIRECT_LAYER_TARGET_BYTES,
            });
        }
        self.layer_target_bytes = bytes;
        Ok(())
    }

    fn end_layer(
        &mut self,
        id: UiLayerId,
        primitive: UiDirectPrimitiveKind,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        let Some(parent) = self.parent_passes.pop() else {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        };
        let child = std::mem::replace(&mut self.active_pass, parent);
        if !child.clip_stack.is_empty() || child.active_stencil_depth != 0 {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        }
        let Some(layer_pass) = self.layer_passes.get_mut(child.pass_index) else {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        };
        if layer_pass.id != Some(id) {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        }
        layer_pass.batches = child.batches;
        let opacity = layer_pass.opacity();
        if self.has_empty_clip() {
            return Ok(());
        }
        self.push_layer_composite_batch(
            child.pass_index,
            primitive,
            opacity,
            viewport,
            scale_factor,
        )
    }

    fn push_layer_composite_batch(
        &mut self,
        source_layer: usize,
        primitive: UiDirectPrimitiveKind,
        opacity: f32,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        let bounds = full_viewport_rect(viewport);
        let vertices = layer_composite_vertices(viewport, opacity)?;
        self.push_geometry_batch_with_source(
            UiDirectBatchKind::Layer,
            primitive,
            &vertices,
            &[0, 1, 2, 0, 2, 3],
            scissor_for(bounds, viewport, scale_factor)?,
            self.active_pass.active_stencil_depth,
            Some(source_layer),
            None,
        )?
        .ok_or(UiDirectRendererError::FullyClippedRequiredPrimitive(
            primitive,
        ))?;
        Ok(())
    }

    fn diagnostics(&self) -> UiDirectFrameDiagnostics {
        UiDirectFrameDiagnostics {
            display_revision: self.display_revision,
            primitive_count: self.primitive_count,
            observed_kinds: self.observed_kinds.clone(),
            prepared_kinds: self.prepared_kinds.clone(),
            unsupported_kinds: self.unsupported_kinds.clone(),
            preparation_only_kinds: self.preparation_only_kinds.clone(),
            batch_count: self.batches.len(),
            vertex_count: self.vertex_count,
            index_count: self.index_count,
            glyph_mask_count: self.glyph_mask_count,
            image_count: self.image_count,
            mesh_count: self.mesh_count,
            clip_scope_count: self.clip_scope_count,
            layer_count: self.layer_count,
            layer_target_bytes: self.layer_target_bytes,
            shadow_count: self.shadow_count,
            backdrop_fallback_count: self.backdrop_fallback_count,
            backdrop_effect_count: self.backdrop_effect_count,
            full_frame_cpu_rasterized: false,
        }
    }

    fn push_marker_batch(
        &mut self,
        batch_kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
    ) -> Result<(), UiDirectRendererError> {
        self.push_batch(UiDirectBatch {
            kind: batch_kind,
            primitive,
            vertex_range: self.vertex_count..self.vertex_count,
            index_range: self.index_count..self.index_count,
            scissor: None,
            stencil_reference: self.active_pass.active_stencil_depth,
        })
    }

    fn push_rect_batch(
        &mut self,
        batch_kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        self.push_quad_batch(
            batch_kind,
            primitive,
            bounds,
            color,
            self.white_region,
            viewport,
            scale_factor,
            self.active_pass.active_stencil_depth,
        )
    }

    fn push_round_rect_batch(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        radii: UiCornerRadii,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        self.push_round_rect_batch_with(
            UiDirectBatchKind::Content,
            primitive,
            bounds,
            radii,
            color,
            viewport,
            scale_factor,
            self.active_pass.active_stencil_depth,
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)] // Geometry batches keep explicit stencil and bounds inputs.
    fn push_round_rect_batch_with(
        &mut self,
        batch_kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        radii: UiCornerRadii,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
        stencil_reference: u32,
    ) -> Result<Option<UiDirectBatch>, UiDirectRendererError> {
        let bounds = snap_rect_to_physical(bounds, scale_factor)?;
        let antialias = !matches!(
            batch_kind,
            UiDirectBatchKind::ClipPush | UiDirectBatchKind::ClipPop
        );
        let (vertices, indices) = rounded_rect_geometry(
            bounds,
            radii,
            color,
            self.white_region,
            self.atlas.width,
            self.atlas.height(),
            viewport,
            scale_factor,
            antialias,
        )?;
        let scissor_bounds = if antialias {
            expand_rect(bounds, 1.0 / scale_factor)
        } else {
            bounds
        };
        self.push_geometry_batch(
            batch_kind,
            primitive,
            &vertices,
            &indices,
            scissor_for(scissor_bounds, viewport, scale_factor)?,
            stencil_reference,
        )
    }

    fn push_rect_stroke_batch(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        width: f32,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        let bounds = snap_rect_to_physical(bounds, scale_factor)?;
        let width = snap_length_to_physical(width, scale_factor);
        let rects = rect_stroke_rects(bounds, width);
        let (vertices, indices) = rects_to_geometry(
            &rects,
            color,
            self.white_region,
            self.atlas.width,
            self.atlas.height(),
            viewport,
        )?;
        self.push_geometry_batch(
            UiDirectBatchKind::Content,
            primitive,
            &vertices,
            &indices,
            scissor_for(bounds, viewport, scale_factor)?,
            self.active_pass.active_stencil_depth,
        )
        .map(|_| ())
    }

    fn push_glyphs(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        raster: &UiTextRaster,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        self.glyph_mask_count = self.glyph_mask_count.saturating_add(raster.glyphs.len());
        let mut emitted_glyph = false;
        for glyph in &raster.glyphs {
            if glyph.width == 0 || glyph.height == 0 {
                continue;
            }
            let glyph_bounds = glyph_rect(bounds, glyph, scale_factor);
            let region = self.atlas.push_glyph(glyph)?;
            self.push_quad_batch(
                UiDirectBatchKind::GlyphMask,
                primitive,
                glyph_bounds,
                color,
                region,
                viewport,
                scale_factor,
                self.active_pass.active_stencil_depth,
            )?;
            emitted_glyph = true;
        }
        if !emitted_glyph {
            self.push_marker_batch(UiDirectBatchKind::GlyphMask, primitive)?;
        }
        Ok(())
    }

    fn push_path(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        commands: &[UiPathCommand],
        fill: Option<UiColor>,
        stroke: Option<UiStroke>,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        let subpaths = flatten_path(commands, scale_factor)?;
        if let Some(fill) = fill {
            ensure_geometry_estimate(
                self.remaining_geometry_bytes()?,
                estimated_fill_geometry(&subpaths)?,
            )?;
            let (vertices, indices) = filled_path_geometry(
                &subpaths,
                fill,
                self.white_region,
                self.atlas.width,
                self.atlas.height(),
                viewport,
            )?;
            self.push_geometry_batch(
                UiDirectBatchKind::Content,
                primitive,
                &vertices,
                &indices,
                scissor_for(full_viewport_rect(viewport), viewport, scale_factor)?,
                self.active_pass.active_stencil_depth,
            )?;
        }
        if let Some(stroke) = stroke {
            ensure_geometry_estimate(
                self.remaining_geometry_bytes()?,
                estimated_stroke_geometry(&subpaths, stroke)?,
            )?;
            self.push_path_stroke_batch(primitive, &subpaths, stroke, viewport, scale_factor)?;
        }
        Ok(())
    }

    fn push_path_stroke_batch(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        subpaths: &[FlatSubpath],
        stroke: UiStroke,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        let (vertices, indices) = stroked_path_geometry(
            subpaths,
            stroke,
            self.white_region,
            self.atlas.width,
            self.atlas.height(),
            viewport,
        )?;
        self.push_geometry_batch(
            UiDirectBatchKind::Content,
            primitive,
            &vertices,
            &indices,
            scissor_for(full_viewport_rect(viewport), viewport, scale_factor)?,
            self.active_pass.active_stencil_depth,
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)] // Shadow geometry keeps source shape, offset, spread, and viewport explicit.
    fn push_shadow(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        radii: UiCornerRadii,
        offset: UiPoint,
        spread: f32,
        color: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        if !spread.is_finite() || spread < 0.0 {
            return Err(UiDirectRendererError::InvalidShadowSpread);
        }
        let base_bounds = UiRect::new(
            UiPoint {
                x: bounds.origin.x + offset.x,
                y: bounds.origin.y + offset.y,
            },
            bounds.size,
        );
        for (index, weight) in SHADOW_FALLOFF_WEIGHTS.iter().copied().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let extent = spread
                * (SHADOW_FALLOFF_WEIGHTS.len().saturating_sub(index) as f32
                    / SHADOW_FALLOFF_WEIGHTS.len() as f32);
            let ring_bounds = expand_rect(base_bounds, extent);
            let ring_radii = UiCornerRadii {
                top_left: radii.top_left + extent,
                top_right: radii.top_right + extent,
                bottom_right: radii.bottom_right + extent,
                bottom_left: radii.bottom_left + extent,
            };
            self.push_round_rect_batch(
                primitive,
                ring_bounds,
                ring_radii,
                UiColor::rgba(color.red, color.green, color.blue, color.alpha * weight),
                viewport,
                scale_factor,
            )?;
        }
        Ok(())
    }

    fn push_backdrop_effect(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        descriptor: UiBackdropDescriptor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        if self.has_empty_clip() {
            return Ok(());
        }
        self.reserve_layer_target()?;
        let (source_pass, source_batch_count) = self
            .active_pass
            .backdrop_parent_prefix
            .unwrap_or((self.active_pass.pass_index, self.active_pass.batches.len()));
        let backdrop_source = self.backdrop_passes.len();
        self.backdrop_passes.push(UiDirectBackdropPass {
            consumer_pass: self.active_pass.pass_index,
            source_pass,
            source_batch_count,
        });
        let bounds = snap_rect_to_physical(descriptor.bounds, scale_factor)?;
        validate_backdrop_resolution_at_scale(
            UiBackdropDescriptor {
                bounds,
                ..descriptor
            },
            UiContrast::Standard,
            UiEffectCapabilities {
                backdrop_filtering: true,
            },
            scale_factor,
        )?;
        let vertices = backdrop_effect_vertices(bounds, descriptor.tint, viewport)?;
        self.push_geometry_batch_with_source(
            UiDirectBatchKind::BackdropEffect,
            primitive,
            &vertices,
            &[0, 1, 2, 0, 2, 3],
            scissor_for(bounds, viewport, scale_factor)?,
            self.active_pass.active_stencil_depth,
            None,
            Some(backdrop_source),
        )?
        .ok_or(UiDirectRendererError::FullyClippedRequiredPrimitive(
            primitive,
        ))?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_quad_batch(
        &mut self,
        kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        color: UiColor,
        region: AtlasRegion,
        viewport: UiSize,
        scale_factor: f32,
        stencil_reference: u32,
    ) -> Result<(), UiDirectRendererError> {
        let bounds = snap_rect_to_physical(bounds, scale_factor)?;
        let vertices = quad_vertices(
            bounds,
            color,
            region,
            self.atlas.width,
            self.atlas.height(),
            viewport,
        )?;
        self.push_geometry_batch(
            kind,
            primitive,
            &vertices,
            &[0, 1, 2, 0, 2, 3],
            scissor_for(bounds, viewport, scale_factor)?,
            stencil_reference,
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_image_batch(
        &mut self,
        batch_kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        image: &UiDirectImage,
        opacity: f32,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        validate_image(image)?;
        let region = self
            .atlas
            .push_rgba(image.width, image.height, &image.rgba)?;
        self.push_quad_batch(
            batch_kind,
            primitive,
            bounds,
            UiColor::rgba(1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)),
            region,
            viewport,
            scale_factor,
            self.active_pass.active_stencil_depth,
        )
    }

    fn push_mesh_batch(
        &mut self,
        primitive: UiDirectPrimitiveKind,
        bounds: UiRect,
        mesh: &UiDirectMesh,
        tint: UiColor,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<(), UiDirectRendererError> {
        validate_mesh(mesh)?;
        self.check_geometry_capacity(mesh.vertices.len(), mesh.indices.len())?;
        let bounds = snap_rect_to_physical(bounds, scale_factor)?;
        let vertices = mesh_vertices(
            mesh,
            bounds,
            tint,
            self.white_region,
            self.atlas.width,
            self.atlas.height(),
            viewport,
        )?;
        self.push_geometry_batch(
            UiDirectBatchKind::Mesh,
            primitive,
            &vertices,
            &mesh.indices,
            scissor_for(bounds, viewport, scale_factor)?,
            self.active_pass.active_stencil_depth,
        )
        .map(|_| ())
    }

    fn push_existing_geometry_batch(
        &mut self,
        kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        source: &UiDirectBatch,
        stencil_reference: u32,
    ) -> Result<(), UiDirectRendererError> {
        self.push_batch(UiDirectBatch {
            kind,
            primitive,
            vertex_range: source.vertex_range.clone(),
            index_range: source.index_range.clone(),
            scissor: source.scissor,
            stencil_reference,
        })
    }

    fn push_geometry_batch(
        &mut self,
        kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        vertices: &[UiDirectVertex],
        indices: &[u32],
        scissor: ScissorDecision,
        stencil_reference: u32,
    ) -> Result<Option<UiDirectBatch>, UiDirectRendererError> {
        self.push_geometry_batch_with_source(
            kind,
            primitive,
            vertices,
            indices,
            scissor,
            stencil_reference,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn push_geometry_batch_with_source(
        &mut self,
        kind: UiDirectBatchKind,
        primitive: UiDirectPrimitiveKind,
        vertices: &[UiDirectVertex],
        indices: &[u32],
        scissor: ScissorDecision,
        stencil_reference: u32,
        source_layer: Option<usize>,
        backdrop_source: Option<usize>,
    ) -> Result<Option<UiDirectBatch>, UiDirectRendererError> {
        if stencil_reference > 255 {
            return Err(UiDirectRendererError::ClipDepthOverflow {
                depth: stencil_reference,
                maximum: 255,
            });
        }
        let scissor = match scissor {
            ScissorDecision::Skip => return Ok(None),
            ScissorDecision::Draw(scissor) => scissor,
        };
        let vertex_count =
            u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
        let index_count =
            u32::try_from(indices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
        self.check_geometry_capacity(vertices.len(), indices.len())?;
        let vertex_start = self.vertex_count;
        let index_start = self.index_count;
        self.vertex_count = self
            .vertex_count
            .checked_add(vertex_count)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        self.index_count = self
            .index_count
            .checked_add(index_count)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let vertex_range = vertex_start..self.vertex_count;
        let index_range = index_start..self.index_count;
        for vertex in vertices {
            self.vertices.push(*vertex);
        }
        for index in indices {
            if *index >= vertex_count {
                return Err(UiDirectRendererError::GeometryOverflow);
            }
            self.indices.push(*index);
        }
        let batch = UiDirectBatch {
            kind,
            primitive,
            vertex_range: vertex_range.clone(),
            index_range: index_range.clone(),
            scissor,
            stencil_reference,
        };
        self.push_batch_with_source(batch.clone(), source_layer, backdrop_source)?;
        Ok(Some(batch))
    }

    fn push_batch(&mut self, batch: UiDirectBatch) -> Result<(), UiDirectRendererError> {
        self.push_batch_with_source(batch, None, None)
    }

    fn push_batch_with_source(
        &mut self,
        batch: UiDirectBatch,
        source_layer: Option<usize>,
        backdrop_source: Option<usize>,
    ) -> Result<(), UiDirectRendererError> {
        let next_count = self.batches.len().saturating_add(1);
        if next_count > MAX_DIRECT_BATCHES {
            return Err(UiDirectRendererError::TooManyBatches {
                count: next_count,
                maximum: MAX_DIRECT_BATCHES,
            });
        }
        let batch_index = self.batches.len();
        self.batches.push(batch);
        self.active_pass.batches.push(UiDirectPassBatch {
            batch_index,
            source_layer,
            backdrop_source,
        });
        Ok(())
    }

    fn check_geometry_capacity(
        &self,
        additional_vertices: usize,
        additional_indices: usize,
    ) -> Result<(), UiDirectRendererError> {
        let total = geometry_bytes_for_counts(
            self.vertices
                .len()
                .checked_add(additional_vertices)
                .ok_or(UiDirectRendererError::GeometryOverflow)?,
            self.indices
                .len()
                .checked_add(additional_indices)
                .ok_or(UiDirectRendererError::GeometryOverflow)?,
        )?;
        if total > MAX_DIRECT_GEOMETRY_BYTES {
            return Err(UiDirectRendererError::TooManyGeometryBytes {
                bytes: total,
                maximum: MAX_DIRECT_GEOMETRY_BYTES,
            });
        }
        Ok(())
    }

    fn remaining_geometry_bytes(&self) -> Result<u64, UiDirectRendererError> {
        let used = geometry_bytes_for_counts(self.vertices.len(), self.indices.len())?;
        MAX_DIRECT_GEOMETRY_BYTES
            .checked_sub(used)
            .ok_or(UiDirectRendererError::GeometryOverflow)
    }

    fn vertex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            self.vertices
                .len()
                .saturating_mul(VERTEX_STRIDE_BYTES_USIZE),
        );
        for vertex in &self.vertices {
            vertex.encode(&mut bytes);
        }
        bytes
    }

    fn index_bytes(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(self.indices.len().saturating_mul(INDEX_STRIDE_BYTES_USIZE));
        for index in &self.indices {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
        bytes
    }

    fn finish(mut self) -> Result<UiDirectFrameResources, UiDirectRendererError> {
        if !self.parent_passes.is_empty()
            || self.active_pass.pass_index != 0
            || !self.active_pass.clip_stack.is_empty()
            || self.active_pass.active_stencil_depth != 0
        {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        }
        let Some(root_pass) = self.layer_passes.first_mut() else {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        };
        root_pass.batches = std::mem::take(&mut self.active_pass.batches);
        let vertex_bytes = self.vertex_bytes();
        let index_bytes = self.index_bytes();
        let atlas = self.atlas.finish()?;
        Ok((
            self.batches,
            vertex_bytes,
            index_bytes,
            atlas,
            self.layer_passes,
            self.backdrop_passes,
        ))
    }
}

fn validate_complete_text_raster(
    primitive: UiDirectPrimitiveKind,
    expected_glyph_count: usize,
    raster: &UiTextRaster,
) -> Result<(), UiDirectRendererError> {
    if raster.has_unrasterized_glyphs || raster.glyphs.len() != expected_glyph_count {
        Err(UiDirectRendererError::IncompleteTextRaster(primitive))
    } else {
        Ok(())
    }
}

fn atlas_builder_for(
    display_list: &DisplayList,
    resources: &UiDirectResourceSet,
) -> Result<AtlasBuilder, UiDirectRendererError> {
    // Every region has a one-texel duplicated-edge gutter on all sides.
    let mut width = 3_u32;
    for primitive in &display_list.primitives {
        match primitive {
            DisplayPrimitive::Text { raster, .. } | DisplayPrimitive::GlyphRun { raster, .. } => {
                for glyph in &raster.glyphs {
                    let padded_width = glyph
                        .width
                        .max(1)
                        .checked_add(2)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?;
                    width = width.max(padded_width);
                }
            }
            DisplayPrimitive::Image { image, .. } => {
                if let Some(descriptor) = resources.image(*image) {
                    validate_image(descriptor)?;
                    let padded_width = descriptor
                        .width
                        .checked_add(2)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?;
                    width = width.max(padded_width);
                }
            }
            _ => {}
        }
    }

    let mut atlas = AtlasBuilder::new(width);
    atlas.push_white()?;
    for primitive in &display_list.primitives {
        match primitive {
            DisplayPrimitive::Text { raster, .. } | DisplayPrimitive::GlyphRun { raster, .. } => {
                for glyph in &raster.glyphs {
                    if glyph.width > 0 && glyph.height > 0 {
                        atlas.push_glyph(glyph)?;
                    }
                }
            }
            DisplayPrimitive::Image { image, .. } => {
                if let Some(descriptor) = resources.image(*image) {
                    atlas.push_rgba(descriptor.width, descriptor.height, &descriptor.rgba)?;
                }
            }
            _ => {}
        }
    }
    Ok(atlas)
}

fn validate_image(image: &UiDirectImage) -> Result<(), UiDirectRendererError> {
    let Some(expected) = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
    else {
        return Err(UiDirectRendererError::InvalidImage(image.handle));
    };
    if image.width == 0 || image.height == 0 || image.rgba.len() != expected {
        Err(UiDirectRendererError::InvalidImage(image.handle))
    } else {
        Ok(())
    }
}

fn validate_mesh(mesh: &UiDirectMesh) -> Result<(), UiDirectRendererError> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err(UiDirectRendererError::InvalidMesh(mesh.handle));
    }
    if mesh.vertices.iter().any(|vertex| {
        vertex.x_milli > 1000
            || vertex.y_milli > 1000
            || vertex.u_milli > 1000
            || vertex.v_milli > 1000
    }) {
        return Err(UiDirectRendererError::InvalidMesh(mesh.handle));
    }
    let vertex_count = u32::try_from(mesh.vertices.len())
        .map_err(|_| UiDirectRendererError::InvalidMesh(mesh.handle))?;
    if mesh.indices.iter().any(|index| *index >= vertex_count) {
        return Err(UiDirectRendererError::InvalidMesh(mesh.handle));
    }
    Ok(())
}

fn color_array(color: UiColor) -> [f32; 4] {
    [
        srgb_channel_to_linear(color.red),
        srgb_channel_to_linear(color.green),
        srgb_channel_to_linear(color.blue),
        color.alpha.clamp(0.0, 1.0),
    ]
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn snap_rect_to_physical(
    bounds: UiRect,
    scale_factor: f32,
) -> Result<UiRect, UiDirectRendererError> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let left = (bounds.origin.x * scale_factor).round();
    let top = (bounds.origin.y * scale_factor).round();
    let right = ((bounds.origin.x + bounds.size.width) * scale_factor).round();
    let bottom = ((bounds.origin.y + bounds.size.height) * scale_factor).round();
    if !left.is_finite()
        || !top.is_finite()
        || !right.is_finite()
        || !bottom.is_finite()
        || right < left
        || bottom < top
    {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    Ok(UiRect::new(
        UiPoint {
            x: left / scale_factor,
            y: top / scale_factor,
        },
        UiSize::new((right - left) / scale_factor, (bottom - top) / scale_factor),
    ))
}

fn snap_length_to_physical(length: f32, scale_factor: f32) -> f32 {
    (length * scale_factor).round().max(1.0) / scale_factor
}

fn quad_vertices(
    bounds: UiRect,
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<Vec<UiDirectVertex>, UiDirectRendererError> {
    let left_top = ndc(bounds.origin, viewport)?;
    let right_top = ndc(
        UiPoint {
            x: bounds.origin.x + bounds.size.width,
            y: bounds.origin.y,
        },
        viewport,
    )?;
    let right_bottom = ndc(
        UiPoint {
            x: bounds.origin.x + bounds.size.width,
            y: bounds.origin.y + bounds.size.height,
        },
        viewport,
    )?;
    let left_bottom = ndc(
        UiPoint {
            x: bounds.origin.x,
            y: bounds.origin.y + bounds.size.height,
        },
        viewport,
    )?;
    let uv = region.uv(atlas_width, atlas_height);
    let color = color_array(color);
    Ok(vec![
        UiDirectVertex {
            position: left_top,
            tex_coord: uv[0],
            color,
        },
        UiDirectVertex {
            position: right_top,
            tex_coord: uv[1],
            color,
        },
        UiDirectVertex {
            position: right_bottom,
            tex_coord: uv[2],
            color,
        },
        UiDirectVertex {
            position: left_bottom,
            tex_coord: uv[3],
            color,
        },
    ])
}

fn layer_composite_vertices(
    viewport: UiSize,
    opacity: f32,
) -> Result<Vec<UiDirectVertex>, UiDirectRendererError> {
    let bounds = full_viewport_rect(viewport);
    let left_top = ndc(bounds.origin, viewport)?;
    let right_top = ndc(
        UiPoint {
            x: bounds.size.width,
            y: 0.0,
        },
        viewport,
    )?;
    let right_bottom = ndc(
        UiPoint {
            x: bounds.size.width,
            y: bounds.size.height,
        },
        viewport,
    )?;
    let left_bottom = ndc(
        UiPoint {
            x: 0.0,
            y: bounds.size.height,
        },
        viewport,
    )?;
    let color = color_array(UiColor::rgba(1.0, 1.0, 1.0, opacity));
    Ok(vec![
        UiDirectVertex {
            position: left_top,
            tex_coord: [0.0, 0.0],
            color,
        },
        UiDirectVertex {
            position: right_top,
            tex_coord: [1.0, 0.0],
            color,
        },
        UiDirectVertex {
            position: right_bottom,
            tex_coord: [1.0, 1.0],
            color,
        },
        UiDirectVertex {
            position: left_bottom,
            tex_coord: [0.0, 1.0],
            color,
        },
    ])
}

fn backdrop_effect_vertices(
    bounds: UiRect,
    tint: UiColor,
    viewport: UiSize,
) -> Result<Vec<UiDirectVertex>, UiDirectRendererError> {
    let left = bounds.origin.x / viewport.width;
    let top = bounds.origin.y / viewport.height;
    let right = (bounds.origin.x + bounds.size.width) / viewport.width;
    let bottom = (bounds.origin.y + bounds.size.height) / viewport.height;
    if [left, top, right, bottom]
        .iter()
        .any(|component| !component.is_finite())
    {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let color = color_array(tint);
    let points = [
        (bounds.origin, [left, top]),
        (
            UiPoint {
                x: bounds.origin.x + bounds.size.width,
                y: bounds.origin.y,
            },
            [right, top],
        ),
        (
            UiPoint {
                x: bounds.origin.x + bounds.size.width,
                y: bounds.origin.y + bounds.size.height,
            },
            [right, bottom],
        ),
        (
            UiPoint {
                x: bounds.origin.x,
                y: bounds.origin.y + bounds.size.height,
            },
            [left, bottom],
        ),
    ];
    points
        .into_iter()
        .map(|(point, tex_coord)| {
            Ok(UiDirectVertex {
                position: ndc(point, viewport)?,
                tex_coord,
                color,
            })
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_arguments)] // Tessellation needs source shape, atlas region, viewport, and physical scale.
fn rounded_rect_geometry(
    bounds: UiRect,
    radii: UiCornerRadii,
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
    scale_factor: f32,
    antialias: bool,
) -> Result<(Vec<UiDirectVertex>, Vec<u32>), UiDirectRendererError> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let uv = region.uv(atlas_width, atlas_height)[0];
    let solid_color = color_array(color);
    vertices.push(UiDirectVertex {
        position: ndc(
            UiPoint {
                x: bounds.origin.x + bounds.size.width * 0.5,
                y: bounds.origin.y + bounds.size.height * 0.5,
            },
            viewport,
        )?,
        tex_coord: uv,
        color: solid_color,
    });
    let fringe = if antialias { 1.0 / scale_factor } else { 0.0 };
    let (inner_points, outer_points) = rounded_rect_perimeters(bounds, radii, scale_factor, fringe);
    for point in &inner_points {
        vertices.push(UiDirectVertex {
            position: ndc(*point, viewport)?,
            tex_coord: uv,
            color: solid_color,
        });
    }
    let inner_count =
        u32::try_from(inner_points.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    for index in 0..inner_count {
        let current = index + 1;
        let next = (index + 1) % inner_count + 1;
        indices.extend_from_slice(&[0, current, next]);
    }
    if antialias {
        let outer_base = inner_count
            .checked_add(1)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let mut transparent_color = solid_color;
        transparent_color[3] = 0.0;
        for point in outer_points {
            vertices.push(UiDirectVertex {
                position: ndc(point, viewport)?,
                tex_coord: uv,
                color: transparent_color,
            });
        }
        for index in 0..inner_count {
            let inner = index + 1;
            let inner_next = (index + 1) % inner_count + 1;
            let outer = outer_base + index;
            let outer_next = outer_base + (index + 1) % inner_count;
            indices.extend_from_slice(&[inner, outer, outer_next, inner, outer_next, inner_next]);
        }
    }
    Ok((vertices, indices))
}

#[allow(clippy::cast_precision_loss)]
fn rounded_rect_perimeters(
    bounds: UiRect,
    radii: UiCornerRadii,
    scale_factor: f32,
    fringe: f32,
) -> (Vec<UiPoint>, Vec<UiPoint>) {
    let max_radius = (bounds.size.width.min(bounds.size.height) * 0.5).max(0.0);
    let corners = [
        (
            UiPoint {
                x: bounds.origin.x + radii.top_left.min(max_radius),
                y: bounds.origin.y + radii.top_left.min(max_radius),
            },
            radii.top_left.min(max_radius),
            std::f32::consts::PI,
            std::f32::consts::PI * 1.5,
        ),
        (
            UiPoint {
                x: bounds.origin.x + bounds.size.width - radii.top_right.min(max_radius),
                y: bounds.origin.y + radii.top_right.min(max_radius),
            },
            radii.top_right.min(max_radius),
            std::f32::consts::PI * 1.5,
            std::f32::consts::TAU,
        ),
        (
            UiPoint {
                x: bounds.origin.x + bounds.size.width - radii.bottom_right.min(max_radius),
                y: bounds.origin.y + bounds.size.height - radii.bottom_right.min(max_radius),
            },
            radii.bottom_right.min(max_radius),
            0.0,
            std::f32::consts::FRAC_PI_2,
        ),
        (
            UiPoint {
                x: bounds.origin.x + radii.bottom_left.min(max_radius),
                y: bounds.origin.y + bounds.size.height - radii.bottom_left.min(max_radius),
            },
            radii.bottom_left.min(max_radius),
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ),
    ];
    let mut inner = Vec::new();
    let mut outer = Vec::new();
    for (center, radius, start, end) in corners {
        let segments = rounded_rect_corner_segments(radius + fringe, scale_factor);
        for step in 0..=segments {
            let t = step as f32 / segments as f32;
            let angle = start + (end - start) * t;
            inner.push(UiPoint {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            });
            if fringe > 0.0 {
                outer.push(UiPoint {
                    x: center.x + (radius + fringe) * angle.cos(),
                    y: center.y + (radius + fringe) * angle.sin(),
                });
            }
        }
    }
    (inner, outer)
}

fn expand_rect(bounds: UiRect, amount: f32) -> UiRect {
    UiRect::new(
        UiPoint {
            x: bounds.origin.x - amount,
            y: bounds.origin.y - amount,
        },
        UiSize::new(
            bounds.size.width + amount * 2.0,
            bounds.size.height + amount * 2.0,
        ),
    )
}

fn rounded_rect_corner_segments(radius: f32, scale_factor: f32) -> u32 {
    let physical_radius = (radius * scale_factor).max(0.0);
    if physical_radius <= 0.25 {
        return 1;
    }
    let maximum_error = 0.25_f32;
    let angle = (1.0 - maximum_error / physical_radius)
        .clamp(-1.0, 1.0)
        .acos()
        .max(f32::EPSILON);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let segments = (std::f32::consts::FRAC_PI_2 / angle).ceil() as u32;
    segments.clamp(
        1,
        u32::try_from(MAX_PATH_COMMANDS_PER_PRIMITIVE).unwrap_or(u32::MAX),
    )
}

fn rect_stroke_rects(bounds: UiRect, width: f32) -> [UiRect; 4] {
    let width = width
        .max(1.0)
        .min(bounds.size.width.max(bounds.size.height));
    let left = bounds.origin.x;
    let top = bounds.origin.y;
    let right = bounds.origin.x + bounds.size.width;
    let bottom = bounds.origin.y + bounds.size.height;
    [
        UiRect::new(
            UiPoint { x: left, y: top },
            UiSize::new(bounds.size.width, width),
        ),
        UiRect::new(
            UiPoint {
                x: left,
                y: bottom - width,
            },
            UiSize::new(bounds.size.width, width),
        ),
        UiRect::new(
            UiPoint { x: left, y: top },
            UiSize::new(width, bounds.size.height),
        ),
        UiRect::new(
            UiPoint {
                x: right - width,
                y: top,
            },
            UiSize::new(width, bounds.size.height),
        ),
    ]
}

fn rects_to_geometry(
    rects: &[UiRect],
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(Vec<UiDirectVertex>, Vec<u32>), UiDirectRendererError> {
    let estimated_vertices = rects
        .len()
        .checked_mul(4)
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    let estimated_indices = rects
        .len()
        .checked_mul(6)
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    vertices
        .try_reserve(estimated_vertices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    indices
        .try_reserve(estimated_indices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    for rect in rects {
        let base =
            u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
        vertices.extend(quad_vertices(
            *rect,
            color,
            region,
            atlas_width,
            atlas_height,
            viewport,
        )?);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Ok((vertices, indices))
}

#[derive(Clone, Debug, PartialEq)]
struct FlatSubpath {
    points: Vec<UiPoint>,
    closed: bool,
}

fn flatten_path(
    commands: &[UiPathCommand],
    scale_factor: f32,
) -> Result<Vec<FlatSubpath>, UiDirectRendererError> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let flatness = 0.25 / scale_factor;
    let mut tessellation_work = 0_usize;
    let mut subpaths = Vec::new();
    let mut points = Vec::new();
    for command in commands {
        match *command {
            UiPathCommand::MoveTo(point) => {
                finish_flat_subpath(&mut subpaths, &mut points, false);
                points.push(point);
            }
            UiPathCommand::LineTo(point) => {
                if points.is_empty() {
                    return Err(UiDirectRendererError::UnsupportedPathGeometry);
                }
                points.push(point);
            }
            UiPathCommand::QuadraticTo { control, end } => {
                let start = points
                    .last()
                    .copied()
                    .ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
                flatten_quadratic(
                    start,
                    control,
                    end,
                    flatness,
                    &mut points,
                    &mut tessellation_work,
                )?;
            }
            UiPathCommand::CubicTo {
                control_a,
                control_b,
                end,
            } => {
                let start = points
                    .last()
                    .copied()
                    .ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
                flatten_cubic(
                    start,
                    control_a,
                    control_b,
                    end,
                    flatness,
                    &mut points,
                    &mut tessellation_work,
                )?;
            }
            UiPathCommand::Close => {
                if points.len() < 2 {
                    return Err(UiDirectRendererError::UnsupportedPathGeometry);
                }
                finish_flat_subpath(&mut subpaths, &mut points, true);
            }
        }
    }
    finish_flat_subpath(&mut subpaths, &mut points, false);
    if subpaths.is_empty() {
        Err(UiDirectRendererError::UnsupportedPathGeometry)
    } else {
        Ok(subpaths)
    }
}

fn flatten_quadratic(
    start: UiPoint,
    control: UiPoint,
    end: UiPoint,
    flatness: f32,
    output: &mut Vec<UiPoint>,
    work: &mut usize,
) -> Result<(), UiDirectRendererError> {
    let mut stack = vec![(start, control, end)];
    while let Some((start, control, end)) = stack.pop() {
        consume_tessellation_work(work)?;
        if point_line_distance(control, start, end) <= flatness {
            output.push(end);
            continue;
        }
        let start_control = midpoint(start, control);
        let control_end = midpoint(control, end);
        let split = midpoint(start_control, control_end);
        stack.push((split, control_end, end));
        stack.push((start, start_control, split));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn flatten_cubic(
    start: UiPoint,
    control_a: UiPoint,
    control_b: UiPoint,
    end: UiPoint,
    flatness: f32,
    output: &mut Vec<UiPoint>,
    work: &mut usize,
) -> Result<(), UiDirectRendererError> {
    let mut stack = vec![(start, control_a, control_b, end)];
    while let Some((start, control_a, control_b, end)) = stack.pop() {
        consume_tessellation_work(work)?;
        let distance = point_line_distance(control_a, start, end)
            .max(point_line_distance(control_b, start, end));
        if distance <= flatness {
            output.push(end);
            continue;
        }
        let start_a = midpoint(start, control_a);
        let a_b = midpoint(control_a, control_b);
        let b_end = midpoint(control_b, end);
        let left_b = midpoint(start_a, a_b);
        let right_a = midpoint(a_b, b_end);
        let split = midpoint(left_b, right_a);
        stack.push((split, right_a, b_end, end));
        stack.push((start, start_a, left_b, split));
    }
    Ok(())
}

fn midpoint(left: UiPoint, right: UiPoint) -> UiPoint {
    UiPoint {
        x: (left.x + right.x) * 0.5,
        y: (left.y + right.y) * 0.5,
    }
}

fn point_line_distance(point: UiPoint, start: UiPoint, end: UiPoint) -> f32 {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length = delta_x.hypot(delta_y);
    if length <= f32::EPSILON {
        return (point.x - start.x).hypot(point.y - start.y);
    }
    ((point.x - start.x) * delta_y - (point.y - start.y) * delta_x).abs() / length
}

fn finish_flat_subpath(subpaths: &mut Vec<FlatSubpath>, points: &mut Vec<UiPoint>, closed: bool) {
    if !points.is_empty() {
        let mut owned = std::mem::take(points);
        owned.dedup_by(|left, right| same_point(*left, *right));
        if owned.len() > 1 && same_point(owned[0], owned[owned.len() - 1]) {
            owned.pop();
        }
        if !owned.is_empty() {
            subpaths.push(FlatSubpath {
                points: owned,
                closed,
            });
        }
    }
}

fn same_point(left: UiPoint, right: UiPoint) -> bool {
    (left.x - right.x).abs() <= f32::EPSILON && (left.y - right.y).abs() <= f32::EPSILON
}

fn geometry_bytes_for_counts(
    vertex_count: usize,
    index_count: usize,
) -> Result<u64, UiDirectRendererError> {
    let vertex_bytes = u64::try_from(vertex_count)
        .ok()
        .and_then(|count| count.checked_mul(VERTEX_STRIDE_BYTES))
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    let index_bytes = u64::try_from(index_count)
        .ok()
        .and_then(|count| count.checked_mul(INDEX_STRIDE_BYTES))
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    vertex_bytes
        .checked_add(index_bytes)
        .ok_or(UiDirectRendererError::GeometryOverflow)
}

fn ensure_geometry_estimate(
    remaining_bytes: u64,
    counts: (usize, usize),
) -> Result<(), UiDirectRendererError> {
    let bytes = geometry_bytes_for_counts(counts.0, counts.1)?;
    if bytes > remaining_bytes {
        Err(UiDirectRendererError::TooManyGeometryBytes {
            bytes: MAX_DIRECT_GEOMETRY_BYTES
                .checked_sub(remaining_bytes)
                .and_then(|used| used.checked_add(bytes))
                .ok_or(UiDirectRendererError::GeometryOverflow)?,
            maximum: MAX_DIRECT_GEOMETRY_BYTES,
        })
    } else {
        Ok(())
    }
}

fn estimated_fill_geometry(
    subpaths: &[FlatSubpath],
) -> Result<(usize, usize), UiDirectRendererError> {
    subpaths.iter().filter(|subpath| subpath.closed).try_fold(
        (0_usize, 0_usize),
        |(vertices, indices), subpath| {
            let next_vertices = vertices
                .checked_add(subpath.points.len())
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            let next_indices = indices
                .checked_add(subpath.points.len().saturating_sub(2).saturating_mul(3))
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            Ok((next_vertices, next_indices))
        },
    )
}

fn estimated_stroke_geometry(
    subpaths: &[FlatSubpath],
    stroke: UiStroke,
) -> Result<(usize, usize), UiDirectRendererError> {
    let mut vertices = 0_usize;
    let mut indices = 0_usize;
    for subpath in subpaths.iter().filter(|subpath| subpath.points.len() >= 2) {
        let segment_count = if subpath.closed {
            subpath.points.len()
        } else {
            subpath.points.len() - 1
        };
        vertices = vertices
            .checked_add(
                segment_count
                    .checked_mul(4)
                    .ok_or(UiDirectRendererError::GeometryOverflow)?,
            )
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        indices = indices
            .checked_add(
                segment_count
                    .checked_mul(6)
                    .ok_or(UiDirectRendererError::GeometryOverflow)?,
            )
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        let join_count = if subpath.closed {
            subpath.points.len()
        } else {
            subpath.points.len().saturating_sub(2)
        };
        let round_patch_count = join_count
            .checked_add(usize::from(!subpath.closed && stroke.line_cap == UiLineCap::Round) * 2)
            .ok_or(UiDirectRendererError::GeometryOverflow)?;
        if stroke.line_join == UiLineJoin::Round {
            let circle_vertices = usize::try_from(ROUND_STROKE_SEGMENTS)
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?
                .saturating_add(2);
            let circle_indices = usize::try_from(ROUND_STROKE_SEGMENTS)
                .map_err(|_| UiDirectRendererError::GeometryOverflow)?
                .saturating_mul(3);
            vertices = vertices
                .checked_add(
                    round_patch_count
                        .checked_mul(circle_vertices)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?,
                )
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            indices = indices
                .checked_add(
                    round_patch_count
                        .checked_mul(circle_indices)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?,
                )
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
        } else {
            vertices = vertices
                .checked_add(
                    join_count
                        .checked_mul(4)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?,
                )
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            indices = indices
                .checked_add(
                    join_count
                        .checked_mul(6)
                        .ok_or(UiDirectRendererError::GeometryOverflow)?,
                )
                .ok_or(UiDirectRendererError::GeometryOverflow)?;
            if !subpath.closed && stroke.line_cap == UiLineCap::Round {
                let circle_vertices = usize::try_from(ROUND_STROKE_SEGMENTS)
                    .map_err(|_| UiDirectRendererError::GeometryOverflow)?
                    .saturating_add(2);
                let circle_indices = usize::try_from(ROUND_STROKE_SEGMENTS)
                    .map_err(|_| UiDirectRendererError::GeometryOverflow)?
                    .saturating_mul(3);
                vertices = vertices
                    .checked_add(circle_vertices.saturating_mul(2))
                    .ok_or(UiDirectRendererError::GeometryOverflow)?;
                indices = indices
                    .checked_add(circle_indices.saturating_mul(2))
                    .ok_or(UiDirectRendererError::GeometryOverflow)?;
            }
        }
    }
    Ok((vertices, indices))
}

fn stroked_path_geometry(
    subpaths: &[FlatSubpath],
    stroke: UiStroke,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(Vec<UiDirectVertex>, Vec<u32>), UiDirectRendererError> {
    let (estimated_vertices, estimated_indices) = estimated_stroke_geometry(subpaths, stroke)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    vertices
        .try_reserve(estimated_vertices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    indices
        .try_reserve(estimated_indices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    let half_width = stroke.width.max(1.0) * 0.5;
    let mut emitted_segment = false;
    for subpath in subpaths {
        if subpath.points.len() < 2 {
            continue;
        }
        let segment_count = if subpath.closed {
            subpath.points.len()
        } else {
            subpath.points.len() - 1
        };
        for segment_index in 0..segment_count {
            let start = subpath.points[segment_index];
            let end = subpath.points[(segment_index + 1) % subpath.points.len()];
            let first = segment_index == 0;
            let last = segment_index + 1 == segment_count;
            if append_stroke_segment(
                &mut vertices,
                &mut indices,
                start,
                end,
                half_width,
                stroke.line_cap,
                first && !subpath.closed,
                last && !subpath.closed,
                stroke.color,
                region,
                atlas_width,
                atlas_height,
                viewport,
            )? {
                emitted_segment = true;
            }
        }
        append_stroke_patches(
            &mut vertices,
            &mut indices,
            subpath,
            stroke,
            half_width,
            region,
            atlas_width,
            atlas_height,
            viewport,
        )?;
    }
    if emitted_segment {
        Ok((vertices, indices))
    } else {
        Err(UiDirectRendererError::UnsupportedPathGeometry)
    }
}

#[allow(clippy::too_many_arguments)]
fn append_stroke_segment(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    start: UiPoint,
    end: UiPoint,
    half_width: f32,
    cap: UiLineCap,
    first: bool,
    last: bool,
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<bool, UiDirectRendererError> {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length = delta_x.hypot(delta_y);
    if !length.is_finite() || length <= f32::EPSILON {
        return Ok(false);
    }
    let direction = UiPoint {
        x: delta_x / length,
        y: delta_y / length,
    };
    let normal = UiPoint {
        x: -direction.y * half_width,
        y: direction.x * half_width,
    };
    let start_extension = if first && cap == UiLineCap::Square {
        half_width
    } else {
        0.0
    };
    let end_extension = if last && cap == UiLineCap::Square {
        half_width
    } else {
        0.0
    };
    let start_center = UiPoint {
        x: start.x - direction.x * start_extension,
        y: start.y - direction.y * start_extension,
    };
    let end_center = UiPoint {
        x: end.x + direction.x * end_extension,
        y: end.y + direction.y * end_extension,
    };
    let points = [
        UiPoint {
            x: start_center.x + normal.x,
            y: start_center.y + normal.y,
        },
        UiPoint {
            x: end_center.x + normal.x,
            y: end_center.y + normal.y,
        },
        UiPoint {
            x: end_center.x - normal.x,
            y: end_center.y - normal.y,
        },
        UiPoint {
            x: start_center.x - normal.x,
            y: start_center.y - normal.y,
        },
    ];
    append_colored_quad(
        vertices,
        indices,
        points,
        color,
        region,
        atlas_width,
        atlas_height,
        viewport,
    )?;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn append_stroke_patches(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    subpath: &FlatSubpath,
    stroke: UiStroke,
    half_width: f32,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(), UiDirectRendererError> {
    let point_count = subpath.points.len();
    let join_range = if subpath.closed {
        0..point_count
    } else {
        1..point_count.saturating_sub(1)
    };
    for index in join_range {
        append_stroke_join(
            vertices,
            indices,
            subpath.points[(index + point_count - 1) % point_count],
            subpath.points[index],
            subpath.points[(index + 1) % point_count],
            stroke,
            half_width,
            region,
            atlas_width,
            atlas_height,
            viewport,
        )?;
    }
    if !subpath.closed && stroke.line_cap == UiLineCap::Round {
        let start = subpath.points[0];
        let start_direction = direction_between(start, subpath.points[1])
            .ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
        append_round_sector(
            vertices,
            indices,
            start,
            half_width,
            start_direction.y.atan2(start_direction.x) + std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            stroke.color,
            region,
            atlas_width,
            atlas_height,
            viewport,
        )?;
        let end = subpath.points[point_count - 1];
        let end_direction = direction_between(subpath.points[point_count - 2], end)
            .ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
        append_round_sector(
            vertices,
            indices,
            end,
            half_width,
            end_direction.y.atan2(end_direction.x) - std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            stroke.color,
            region,
            atlas_width,
            atlas_height,
            viewport,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_stroke_join(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    previous: UiPoint,
    center: UiPoint,
    next: UiPoint,
    stroke: UiStroke,
    half_width: f32,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(), UiDirectRendererError> {
    let incoming = direction_between(previous, center)
        .ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
    let outgoing =
        direction_between(center, next).ok_or(UiDirectRendererError::UnsupportedPathGeometry)?;
    let turn = incoming.x * outgoing.y - incoming.y * outgoing.x;
    if turn.abs() <= f32::EPSILON {
        return Ok(());
    }
    let side = turn.signum();
    let incoming_normal = UiPoint {
        x: -incoming.y * half_width * side,
        y: incoming.x * half_width * side,
    };
    let outgoing_normal = UiPoint {
        x: -outgoing.y * half_width * side,
        y: outgoing.x * half_width * side,
    };
    let outer_incoming = add_points(center, incoming_normal);
    let outer_outgoing = add_points(center, outgoing_normal);
    match stroke.line_join {
        UiLineJoin::Bevel => append_colored_triangle(
            vertices,
            indices,
            [center, outer_incoming, outer_outgoing],
            stroke.color,
            region,
            atlas_width,
            atlas_height,
            viewport,
        ),
        UiLineJoin::Miter => {
            let miter = line_intersection(outer_incoming, incoming, outer_outgoing, outgoing);
            if let Some(miter) = miter
                .filter(|point| (point.x - center.x).hypot(point.y - center.y) <= half_width * 4.0)
            {
                append_colored_triangle(
                    vertices,
                    indices,
                    [outer_incoming, miter, outer_outgoing],
                    stroke.color,
                    region,
                    atlas_width,
                    atlas_height,
                    viewport,
                )
            } else {
                append_colored_triangle(
                    vertices,
                    indices,
                    [center, outer_incoming, outer_outgoing],
                    stroke.color,
                    region,
                    atlas_width,
                    atlas_height,
                    viewport,
                )
            }
        }
        UiLineJoin::Round => {
            let start_angle = incoming_normal.y.atan2(incoming_normal.x);
            let end_angle = outgoing_normal.y.atan2(outgoing_normal.x);
            let sweep = directed_angle_sweep(start_angle, end_angle, side);
            append_round_sector(
                vertices,
                indices,
                center,
                half_width,
                start_angle,
                sweep,
                stroke.color,
                region,
                atlas_width,
                atlas_height,
                viewport,
            )
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
fn append_round_sector(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    center: UiPoint,
    radius: f32,
    start_angle: f32,
    sweep: f32,
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(), UiDirectRendererError> {
    let base =
        u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    let uv = region.uv(atlas_width, atlas_height)[0];
    let color = color_array(color);
    vertices.push(UiDirectVertex {
        position: ndc(center, viewport)?,
        tex_coord: uv,
        color,
    });
    let sweep_fraction = sweep.abs() / std::f32::consts::TAU;
    let segments = if sweep_fraction <= 0.25 {
        ROUND_STROKE_SEGMENTS / 4
    } else if sweep_fraction <= 0.5 {
        ROUND_STROKE_SEGMENTS / 2
    } else if sweep_fraction <= 0.75 {
        ROUND_STROKE_SEGMENTS * 3 / 4
    } else {
        ROUND_STROKE_SEGMENTS
    }
    .max(1);
    for segment in 0..=segments {
        let angle = start_angle + sweep * segment as f32 / segments as f32;
        vertices.push(UiDirectVertex {
            position: ndc(
                UiPoint {
                    x: center.x + radius * angle.cos(),
                    y: center.y + radius * angle.sin(),
                },
                viewport,
            )?,
            tex_coord: uv,
            color,
        });
    }
    for segment in 0..segments {
        indices.extend_from_slice(&[base, base + segment + 1, base + segment + 2]);
    }
    Ok(())
}

fn direction_between(start: UiPoint, end: UiPoint) -> Option<UiPoint> {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length = delta_x.hypot(delta_y);
    (length.is_finite() && length > f32::EPSILON).then_some(UiPoint {
        x: delta_x / length,
        y: delta_y / length,
    })
}

fn add_points(left: UiPoint, right: UiPoint) -> UiPoint {
    UiPoint {
        x: left.x + right.x,
        y: left.y + right.y,
    }
}

fn line_intersection(
    first_point: UiPoint,
    first_direction: UiPoint,
    second_point: UiPoint,
    second_direction: UiPoint,
) -> Option<UiPoint> {
    let denominator =
        first_direction.x * second_direction.y - first_direction.y * second_direction.x;
    if denominator.abs() <= f32::EPSILON {
        return None;
    }
    let offset = UiPoint {
        x: second_point.x - first_point.x,
        y: second_point.y - first_point.y,
    };
    let distance = (offset.x * second_direction.y - offset.y * second_direction.x) / denominator;
    Some(UiPoint {
        x: first_point.x + first_direction.x * distance,
        y: first_point.y + first_direction.y * distance,
    })
}

fn directed_angle_sweep(start: f32, end: f32, direction: f32) -> f32 {
    let mut sweep = end - start;
    if direction > 0.0 {
        while sweep < 0.0 {
            sweep += std::f32::consts::TAU;
        }
    } else {
        while sweep > 0.0 {
            sweep -= std::f32::consts::TAU;
        }
    }
    sweep
}

#[allow(clippy::too_many_arguments)]
fn append_colored_triangle(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    points: [UiPoint; 3],
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(), UiDirectRendererError> {
    let base =
        u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    let uv = region.uv(atlas_width, atlas_height)[0];
    let color = color_array(color);
    for point in points {
        vertices.push(UiDirectVertex {
            position: ndc(point, viewport)?,
            tex_coord: uv,
            color,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2]);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_colored_quad(
    vertices: &mut Vec<UiDirectVertex>,
    indices: &mut Vec<u32>,
    points: [UiPoint; 4],
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(), UiDirectRendererError> {
    let base =
        u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    let uv = region.uv(atlas_width, atlas_height)[0];
    let color = color_array(color);
    for point in points {
        vertices.push(UiDirectVertex {
            position: ndc(point, viewport)?,
            tex_coord: uv,
            color,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    Ok(())
}

fn filled_path_geometry(
    subpaths: &[FlatSubpath],
    color: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<(Vec<UiDirectVertex>, Vec<u32>), UiDirectRendererError> {
    let uv = region.uv(atlas_width, atlas_height)[0];
    let color = color_array(color);
    let (estimated_vertices, estimated_indices) = estimated_fill_geometry(subpaths)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    vertices
        .try_reserve(estimated_vertices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    indices
        .try_reserve(estimated_indices)
        .map_err(|_| UiDirectRendererError::GeometryOverflow)?;
    let mut filled = false;
    let mut winding = None;
    for subpath in subpaths.iter().filter(|subpath| subpath.closed) {
        let polygon = normalized_polygon(&subpath.points)?;
        let counter_clockwise = signed_polygon_area(&polygon) > 0.0;
        if winding.is_some_and(|expected| expected != counter_clockwise) {
            return Err(UiDirectRendererError::UnsupportedPathGeometry);
        }
        winding = Some(counter_clockwise);
        let local_indices = triangulate_polygon(&polygon)?;
        let base =
            u32::try_from(vertices.len()).map_err(|_| UiDirectRendererError::GeometryOverflow)?;
        for point in polygon {
            vertices.push(UiDirectVertex {
                position: ndc(point, viewport)?,
                tex_coord: uv,
                color,
            });
        }
        for index in local_indices {
            indices.push(
                base.checked_add(index)
                    .ok_or(UiDirectRendererError::GeometryOverflow)?,
            );
        }
        filled = true;
    }
    if filled {
        Ok((vertices, indices))
    } else {
        Err(UiDirectRendererError::UnsupportedPathGeometry)
    }
}

fn normalized_polygon(points: &[UiPoint]) -> Result<Vec<UiPoint>, UiDirectRendererError> {
    let mut polygon = points.to_vec();
    polygon.dedup_by(|left, right| same_point(*left, *right));
    if polygon.len() > 1 && same_point(polygon[0], polygon[polygon.len() - 1]) {
        polygon.pop();
    }
    if polygon.len() < 3 || signed_polygon_area(&polygon).abs() <= f64::EPSILON {
        return Err(UiDirectRendererError::UnsupportedPathGeometry);
    }
    Ok(polygon)
}

fn triangulate_polygon(points: &[UiPoint]) -> Result<Vec<u32>, UiDirectRendererError> {
    let counter_clockwise = signed_polygon_area(points) > 0.0;
    let mut remaining = (0..points.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2).saturating_mul(3));
    let mut work = 0_usize;
    while remaining.len() > 3 {
        let mut ear = None;
        for cursor in 0..remaining.len() {
            consume_tessellation_work(&mut work)?;
            let previous = remaining[(cursor + remaining.len() - 1) % remaining.len()];
            let current = remaining[cursor];
            let next = remaining[(cursor + 1) % remaining.len()];
            let turn = cross(points[previous], points[current], points[next]);
            if (counter_clockwise && turn <= f64::EPSILON)
                || (!counter_clockwise && turn >= -f64::EPSILON)
            {
                continue;
            }
            let mut contains_vertex = false;
            for candidate in remaining.iter().copied() {
                if candidate == previous || candidate == current || candidate == next {
                    continue;
                }
                consume_tessellation_work(&mut work)?;
                if point_in_triangle(
                    points[candidate],
                    points[previous],
                    points[current],
                    points[next],
                ) {
                    contains_vertex = true;
                    break;
                }
            }
            if !contains_vertex {
                ear = Some((cursor, previous, current, next));
                break;
            }
        }
        let Some((cursor, previous, current, next)) = ear else {
            return Err(UiDirectRendererError::UnsupportedPathGeometry);
        };
        triangles.extend_from_slice(&[
            u32::try_from(previous).map_err(|_| UiDirectRendererError::GeometryOverflow)?,
            u32::try_from(current).map_err(|_| UiDirectRendererError::GeometryOverflow)?,
            u32::try_from(next).map_err(|_| UiDirectRendererError::GeometryOverflow)?,
        ]);
        remaining.remove(cursor);
    }
    if remaining.len() != 3
        || cross(
            points[remaining[0]],
            points[remaining[1]],
            points[remaining[2]],
        )
        .abs()
            <= f64::EPSILON
    {
        return Err(UiDirectRendererError::UnsupportedPathGeometry);
    }
    for index in remaining {
        triangles.push(u32::try_from(index).map_err(|_| UiDirectRendererError::GeometryOverflow)?);
    }
    Ok(triangles)
}

fn consume_tessellation_work(work: &mut usize) -> Result<(), UiDirectRendererError> {
    *work = work
        .checked_add(1)
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    if *work > MAX_PATH_TESSELLATION_WORK {
        Err(UiDirectRendererError::PathTessellationBudgetExceeded {
            work: *work,
            maximum: MAX_PATH_TESSELLATION_WORK,
        })
    } else {
        Ok(())
    }
}

fn signed_polygon_area(points: &[UiPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| {
            f64::from(left.x) * f64::from(right.y) - f64::from(right.x) * f64::from(left.y)
        })
        .sum::<f64>()
        * 0.5
}

fn cross(origin: UiPoint, left: UiPoint, right: UiPoint) -> f64 {
    f64::from(left.x - origin.x) * f64::from(right.y - origin.y)
        - f64::from(left.y - origin.y) * f64::from(right.x - origin.x)
}

fn point_in_triangle(point: UiPoint, a: UiPoint, b: UiPoint, c: UiPoint) -> bool {
    let first = cross(a, b, point);
    let second = cross(b, c, point);
    let third = cross(c, a, point);
    let has_negative = first < -f64::EPSILON || second < -f64::EPSILON || third < -f64::EPSILON;
    let has_positive = first > f64::EPSILON || second > f64::EPSILON || third > f64::EPSILON;
    !(has_negative && has_positive)
}

fn mesh_vertices(
    mesh: &UiDirectMesh,
    bounds: UiRect,
    tint: UiColor,
    region: AtlasRegion,
    atlas_width: u32,
    atlas_height: u32,
    viewport: UiSize,
) -> Result<Vec<UiDirectVertex>, UiDirectRendererError> {
    let uv_base = region.uv(atlas_width, atlas_height);
    let color = color_array(tint);
    mesh.vertices
        .iter()
        .map(|vertex| {
            let x = bounds.origin.x + bounds.size.width * f32::from(vertex.x_milli) / 1000.0;
            let y = bounds.origin.y + bounds.size.height * f32::from(vertex.y_milli) / 1000.0;
            let u = uv_base[0][0]
                + (uv_base[2][0] - uv_base[0][0]) * f32::from(vertex.u_milli) / 1000.0;
            let v = uv_base[0][1]
                + (uv_base[2][1] - uv_base[0][1]) * f32::from(vertex.v_milli) / 1000.0;
            Ok(UiDirectVertex {
                position: ndc(UiPoint { x, y }, viewport)?,
                tex_coord: [u, v],
                color,
            })
        })
        .collect()
}

fn ndc(point: UiPoint, viewport: UiSize) -> Result<[f32; 2], UiDirectRendererError> {
    if viewport.width <= 0.0
        || viewport.height <= 0.0
        || !point.x.is_finite()
        || !point.y.is_finite()
    {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let position = [
        point.x / viewport.width * 2.0 - 1.0,
        1.0 - point.y / viewport.height * 2.0,
    ];
    if position[0].is_finite() && position[1].is_finite() {
        Ok(position)
    } else {
        Err(UiDirectRendererError::InvalidViewport)
    }
}

impl UiDirectFramePlan {
    #[must_use]
    pub const fn cache_key(&self) -> UiDirectCacheKey {
        self.cache_key
    }

    #[must_use]
    pub const fn recovery(&self) -> UiDirectRendererRecovery {
        self.recovery
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &UiDirectFrameDiagnostics {
        &self.diagnostics
    }

    #[must_use]
    pub fn batches(&self) -> &[UiDirectBatch] {
        &self.batches
    }

    /// Returns payload accounting without exposing GPU allocation internals.
    #[must_use]
    pub fn footprint(&self) -> UiDirectFrameFootprint {
        let cpu_vertex_bytes = usize_to_u64(self.vertex_bytes.len());
        let cpu_index_bytes = usize_to_u64(self.index_bytes.len());
        let cpu_atlas_bytes = usize_to_u64(self.atlas.rgba.len());
        UiDirectFrameFootprint {
            cpu_vertex_bytes,
            cpu_index_bytes,
            cpu_atlas_bytes,
            gpu_upload_payload_bytes: cpu_vertex_bytes
                .saturating_add(cpu_index_bytes)
                .saturating_add(cpu_atlas_bytes),
            planned_color_target_bytes: self.diagnostics.layer_target_bytes,
            primitive_count: self.diagnostics.primitive_count,
            batch_count: self.diagnostics.batch_count,
            layer_count: self.diagnostics.layer_count,
            shadow_count: self.diagnostics.shadow_count,
            backdrop_effect_count: self.diagnostics.backdrop_effect_count,
            backdrop_fallback_count: self.diagnostics.backdrop_fallback_count,
        }
    }

    #[must_use]
    pub const fn atlas(&self) -> &UiDirectAtlas {
        &self.atlas
    }

    /// Uploads the prepared direct UI frame into device-owned RHI resources.
    ///
    /// # Errors
    ///
    /// Returns typed RHI failures for pipeline, buffer, texture, bind-group, or
    /// upload rejection. The immutable CPU plan remains reusable after failure.
    pub fn upload_gpu_frame(
        &self,
        rhi: &mut Rhi,
    ) -> Result<UiDirectGpuFrame, UiDirectRendererError> {
        validate_rhi_identity(&self.rhi_identity, &rhi.render_identity())?;
        let layout = ui_vertex_layout()?;
        let UiDirectPipelines {
            content: content_pipeline,
            composite: composite_pipeline,
            backdrop: backdrop_pipeline,
            clip_push: clip_push_pipeline,
            clip_pop: clip_pop_pipeline,
        } = create_ui_direct_pipelines(rhi, &layout)?;
        let vertex_size = u64::try_from(self.vertex_bytes.len())
            .map_err(|_| UiDirectRendererError::GeometryOverflow)?
            .max(4);
        let index_size = u64::try_from(self.index_bytes.len())
            .map_err(|_| UiDirectRendererError::GeometryOverflow)?
            .max(4);
        let vertex_buffer = rhi.create_buffer(
            "Meridian direct UI vertices",
            vertex_size,
            BufferUsage::Vertex,
        )?;
        let index_buffer =
            rhi.create_buffer("Meridian direct UI indices", index_size, BufferUsage::Index)?;
        rhi.write_buffer(&vertex_buffer, 0, &self.vertex_bytes)?;
        rhi.write_buffer(&index_buffer, 0, &self.index_bytes)?;
        let atlas_texture = rhi.create_texture(
            "Meridian direct UI atlas",
            meridian_platform::WindowSize::new(self.atlas.width, self.atlas.height),
            1,
            TextureFormat::Rgba8UnormSrgb,
        )?;
        rhi.write_texture(
            &atlas_texture,
            0,
            &self.atlas.rgba,
            self.atlas.width.saturating_mul(4),
        )?;
        let atlas_bind_group = rhi.create_texture_bind_group(
            "Meridian direct UI atlas binding",
            &content_pipeline,
            &atlas_texture,
        )?;
        let clip_push_atlas_bind_group = rhi.create_texture_bind_group(
            "Meridian direct UI clip-push atlas binding",
            &clip_push_pipeline,
            &atlas_texture,
        )?;
        let clip_pop_atlas_bind_group = rhi.create_texture_bind_group(
            "Meridian direct UI clip-pop atlas binding",
            &clip_pop_pipeline,
            &atlas_texture,
        )?;
        let (layer_targets, layer_bind_groups) =
            self.upload_layer_targets(rhi, &composite_pipeline)?;
        let (backdrop_targets, backdrop_bind_groups) =
            self.upload_backdrop_targets(rhi, &backdrop_pipeline)?;
        Ok(UiDirectGpuFrame {
            cache_key: self.cache_key,
            rhi_identity: self.rhi_identity.clone(),
            layer_plan_fingerprint: layer_plan_fingerprint(
                &self.layer_passes,
                &self.backdrop_passes,
                self.diagnostics.layer_target_bytes,
            ),
            vertex_bytes_len: self.vertex_bytes.len(),
            index_bytes_len: self.index_bytes.len(),
            atlas_size: (self.atlas.width, self.atlas.height),
            content_pipeline,
            composite_pipeline,
            backdrop_pipeline,
            clip_push_pipeline,
            clip_pop_pipeline,
            vertex_buffer,
            index_buffer,
            _atlas_texture: atlas_texture,
            atlas_bind_group,
            clip_push_atlas_bind_group,
            clip_pop_atlas_bind_group,
            layer_targets,
            layer_bind_groups,
            backdrop_targets,
            backdrop_bind_groups,
        })
    }

    fn upload_layer_targets(
        &self,
        rhi: &Rhi,
        composite_pipeline: &GpuRenderPipeline,
    ) -> Result<(Vec<GpuRenderTarget>, Vec<GpuTextureBindGroup>), UiDirectRendererError> {
        let target_size =
            WindowSize::new(self.cache_key.surface_width, self.cache_key.surface_height);
        let mut targets = Vec::with_capacity(self.layer_passes.len().saturating_sub(1));
        for pass in self.layer_passes.iter().skip(1) {
            let id = pass.id.ok_or(UiDirectRendererError::InvalidLayerPlan)?;
            targets.push(rhi.create_surface_render_target(
                &format!("Meridian direct UI layer {}", id.0),
                target_size,
            )?);
        }
        let mut bindings = Vec::with_capacity(targets.len());
        for (index, target) in targets.iter().enumerate() {
            bindings.push(rhi.create_render_target_bind_group(
                &format!("Meridian direct UI layer {} composite", index + 1),
                composite_pipeline,
                target,
            )?);
        }
        Ok((targets, bindings))
    }

    fn upload_backdrop_targets(
        &self,
        rhi: &Rhi,
        backdrop_pipeline: &GpuRenderPipeline,
    ) -> Result<(Vec<GpuRenderTarget>, Vec<GpuTextureBindGroup>), UiDirectRendererError> {
        let target_size =
            WindowSize::new(self.cache_key.surface_width, self.cache_key.surface_height);
        let mut targets = Vec::with_capacity(self.backdrop_passes.len());
        let mut bindings = Vec::with_capacity(self.backdrop_passes.len());
        for index in 0..self.backdrop_passes.len() {
            let target = rhi.create_surface_render_target(
                &format!("Meridian direct UI backdrop source {index}"),
                target_size,
            )?;
            let binding = rhi.create_render_target_bind_group(
                &format!("Meridian direct UI backdrop binding {index}"),
                backdrop_pipeline,
                &target,
            )?;
            targets.push(target);
            bindings.push(binding);
        }
        Ok((targets, bindings))
    }

    #[must_use]
    pub fn vertex_bytes(&self) -> u64 {
        u64::from(self.diagnostics.vertex_count) * VERTEX_STRIDE_BYTES
    }

    #[must_use]
    pub fn index_bytes(&self) -> u64 {
        u64::from(self.diagnostics.index_count) * INDEX_STRIDE_BYTES
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

impl UiDirectGpuFrame {
    fn validate_plan(&self, plan: &UiDirectFramePlan) -> Result<(), UiDirectRendererError> {
        validate_gpu_frame_identity(
            self.cache_key,
            &self.rhi_identity,
            self.layer_plan_fingerprint,
            self.vertex_bytes_len,
            self.index_bytes_len,
            self.atlas_size,
            plan,
        )
    }

    fn validate_rhi(&self, rhi: &Rhi) -> Result<(), UiDirectRendererError> {
        validate_rhi_identity(&self.rhi_identity, &rhi.render_identity())
    }

    /// Builds bounded RHI batches while preserving each batch's local index base.
    ///
    /// # Errors
    ///
    /// Rejects a mutated or malformed plan whose vertex base cannot be
    /// represented by the indexed-draw contract.
    pub fn rhi_batches<'a>(
        &'a self,
        plan: &'a UiDirectFramePlan,
    ) -> Result<Vec<RhiRenderBatch<'a>>, UiDirectRendererError> {
        self.rhi_batches_for_pass(plan, 0, None)
    }

    fn rhi_batches_for_pass<'a>(
        &'a self,
        plan: &'a UiDirectFramePlan,
        pass_index: usize,
        batch_limit: Option<usize>,
    ) -> Result<Vec<RhiRenderBatch<'a>>, UiDirectRendererError> {
        self.validate_plan(plan)?;
        let pass = plan
            .layer_passes
            .get(pass_index)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let batch_limit = batch_limit.unwrap_or(pass.batches.len());
        let planned_batches = pass
            .batches
            .get(..batch_limit)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let mut rhi_batches = Vec::with_capacity(planned_batches.len());
        for planned_batch in planned_batches {
            let batch = plan
                .batches
                .get(planned_batch.batch_index)
                .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
            if batch.index_range.start >= batch.index_range.end {
                continue;
            }
            let (pipeline, texture_bind_group) = match batch.kind {
                UiDirectBatchKind::ClipPush => {
                    if planned_batch.source_layer.is_some()
                        || planned_batch.backdrop_source.is_some()
                    {
                        return Err(UiDirectRendererError::InvalidLayerPlan);
                    }
                    (&self.clip_push_pipeline, &self.clip_push_atlas_bind_group)
                }
                UiDirectBatchKind::ClipPop => {
                    if planned_batch.source_layer.is_some()
                        || planned_batch.backdrop_source.is_some()
                    {
                        return Err(UiDirectRendererError::InvalidLayerPlan);
                    }
                    (&self.clip_pop_pipeline, &self.clip_pop_atlas_bind_group)
                }
                UiDirectBatchKind::Layer => {
                    if planned_batch.backdrop_source.is_some() {
                        return Err(UiDirectRendererError::InvalidLayerPlan);
                    }
                    let source_layer = planned_batch
                        .source_layer
                        .and_then(|index| index.checked_sub(1))
                        .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
                    let binding = self
                        .layer_bind_groups
                        .get(source_layer)
                        .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
                    (&self.composite_pipeline, binding)
                }
                UiDirectBatchKind::BackdropEffect => {
                    if planned_batch.source_layer.is_some() {
                        return Err(UiDirectRendererError::InvalidLayerPlan);
                    }
                    let source = planned_batch
                        .backdrop_source
                        .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
                    let binding = self
                        .backdrop_bind_groups
                        .get(source)
                        .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
                    (&self.backdrop_pipeline, binding)
                }
                UiDirectBatchKind::Content
                | UiDirectBatchKind::GlyphMask
                | UiDirectBatchKind::Image
                | UiDirectBatchKind::Mesh
                | UiDirectBatchKind::Shadow
                | UiDirectBatchKind::BackdropFallback => {
                    if planned_batch.source_layer.is_some()
                        || planned_batch.backdrop_source.is_some()
                    {
                        return Err(UiDirectRendererError::InvalidLayerPlan);
                    }
                    (&self.content_pipeline, &self.atlas_bind_group)
                }
            };
            let mut rhi_batch = RhiRenderBatch::unbound(
                pipeline,
                &self.vertex_buffer,
                &self.index_buffer,
                batch.index_range.clone(),
            )
            .with_base_vertex(batch.vertex_range.start)?
            .with_texture_bind_group(texture_bind_group)
            .with_stencil_reference(batch.stencil_reference);
            if let Some(scissor) = batch.scissor {
                rhi_batch = rhi_batch.with_scissor(scissor);
            }
            rhi_batches.push(rhi_batch);
        }
        Ok(rhi_batches)
    }

    fn render_layer_targets(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        root_color: ClearColor,
    ) -> Result<(), UiDirectRendererError> {
        self.validate_plan(plan)?;
        let root = plan
            .layer_passes
            .first()
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let mut rendered = vec![false; plan.layer_passes.len()];
        let mut rendered_backdrops = vec![false; plan.backdrop_passes.len()];
        for child_layer in &root.children {
            self.render_layer_target(
                rhi,
                plan,
                *child_layer,
                &mut rendered,
                &mut rendered_backdrops,
                root_color,
            )?;
        }
        for pass_index in 1..plan.layer_passes.len() {
            self.render_layer_target(
                rhi,
                plan,
                pass_index,
                &mut rendered,
                &mut rendered_backdrops,
                root_color,
            )?;
        }
        for backdrop_index in 0..plan.backdrop_passes.len() {
            self.render_backdrop_source(
                rhi,
                plan,
                backdrop_index,
                &mut rendered_backdrops,
                root_color,
            )?;
        }
        Ok(())
    }

    fn render_layer_target(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        pass_index: usize,
        rendered: &mut [bool],
        rendered_backdrops: &mut [bool],
        root_color: ClearColor,
    ) -> Result<(), UiDirectRendererError> {
        if pass_index == 0 {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        }
        let already_rendered = rendered
            .get(pass_index)
            .copied()
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        if already_rendered {
            return Ok(());
        }
        let pass = plan
            .layer_passes
            .get(pass_index)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let child_layers = pass.children.clone();
        for child_layer in child_layers {
            self.render_layer_target(
                rhi,
                plan,
                child_layer,
                rendered,
                rendered_backdrops,
                root_color,
            )?;
        }
        for planned_batch in &pass.batches {
            if let Some(backdrop_source) = planned_batch.backdrop_source {
                self.render_backdrop_source(
                    rhi,
                    plan,
                    backdrop_source,
                    rendered_backdrops,
                    root_color,
                )?;
            }
        }
        let target = self
            .layer_targets
            .get(pass_index.saturating_sub(1))
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let expected_size =
            WindowSize::new(plan.cache_key.surface_width, plan.cache_key.surface_height);
        if target.size() != expected_size {
            return Err(UiDirectRendererError::InvalidLayerPlan);
        }
        let batches = self.rhi_batches_for_pass(plan, pass_index, None)?;
        let clear = ClearColor {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        };
        if batches.is_empty() {
            rhi.clear_render_target(target, clear)?;
        } else {
            rhi.render_indexed_batches_to_target(
                target,
                &batches,
                RenderTargetLoadPolicy::Clear(clear),
            )?;
        }
        rendered[pass_index] = true;
        Ok(())
    }

    fn render_backdrop_source(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        backdrop_index: usize,
        rendered: &mut [bool],
        root_color: ClearColor,
    ) -> Result<(), UiDirectRendererError> {
        if rendered
            .get(backdrop_index)
            .copied()
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?
        {
            return Ok(());
        }
        let backdrop = plan
            .backdrop_passes
            .get(backdrop_index)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let source_pass = plan
            .layer_passes
            .get(backdrop.source_pass)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let source_prefix = source_pass
            .batches
            .get(..backdrop.source_batch_count)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        for planned_batch in source_prefix {
            if let Some(dependency) = planned_batch.backdrop_source {
                self.render_backdrop_source(rhi, plan, dependency, rendered, root_color)?;
            }
        }
        let target = self
            .backdrop_targets
            .get(backdrop_index)
            .ok_or(UiDirectRendererError::InvalidLayerPlan)?;
        let batches = self.rhi_batches_for_pass(
            plan,
            backdrop.source_pass,
            Some(backdrop.source_batch_count),
        )?;
        let clear = if backdrop.source_pass == 0 {
            root_color
        } else {
            ClearColor {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            }
        };
        if batches.is_empty() {
            rhi.clear_render_target(target, clear)?;
        } else {
            rhi.render_indexed_batches_to_target(
                target,
                &batches,
                RenderTargetLoadPolicy::Clear(clear),
            )?;
        }
        rendered[backdrop_index] = true;
        Ok(())
    }

    /// Submits the frame to the offscreen structural validation path.
    ///
    /// # Errors
    ///
    /// Returns backend validation failures from the RHI submission contract.
    pub fn submit_structural_validation(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        color: ClearColor,
    ) -> Result<(), UiDirectRendererError> {
        self.validate_rhi(rhi)?;
        self.render_layer_targets(rhi, plan, color)?;
        let batches = self.rhi_batches(plan)?;
        if batches.is_empty() {
            rhi.submit_clear_structural_validation(color)?;
        } else {
            rhi.submit_indexed_batches_structural_validation(&batches, color)?;
        }
        Ok(())
    }

    /// Renders this immutable frame into a capture-capable offscreen target.
    ///
    /// A caller queues one bounded RHI capture request before this call, then
    /// retrieves its asynchronous result through the RHI capture API.  The
    /// normal sampled layer/backdrop targets remain non-capture targets; only
    /// this final qualification target carries copy-source capability.
    ///
    /// # Errors
    ///
    /// Returns typed identity, preparation, target, or submission errors.  It
    /// does not claim presented visual quality.
    pub fn submit_offscreen_capture(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        color: ClearColor,
    ) -> Result<(), UiDirectRendererError> {
        self.validate_rhi(rhi)?;
        self.render_layer_targets(rhi, plan, color)?;
        let target = rhi
            .create_capture_render_target(
                "Meridian direct UI qualification capture",
                WindowSize::new(plan.cache_key.surface_width, plan.cache_key.surface_height),
            )
            .map_err(map_offscreen_capture_target_error)?;
        let batches = self.rhi_batches(plan)?;
        if batches.is_empty() {
            rhi.clear_render_target(&target, color)?;
        } else {
            rhi.render_indexed_batches_to_target(
                &target,
                &batches,
                RenderTargetLoadPolicy::Clear(color),
            )?;
        }
        rhi.capture_render_target(&target)?;
        Ok(())
    }

    /// Presents the frame through the configured RHI surface when available.
    ///
    /// # Errors
    ///
    /// Returns typed RHI surface, validation, or submission failures.
    pub fn present(
        &self,
        rhi: &mut Rhi,
        plan: &UiDirectFramePlan,
        color: ClearColor,
    ) -> Result<FrameOutcome, UiDirectRendererError> {
        self.validate_rhi(rhi)?;
        self.render_layer_targets(rhi, plan, color)?;
        let batches = self.rhi_batches(plan)?;
        if batches.is_empty() {
            Ok(rhi.clear_and_present(color)?)
        } else {
            Ok(rhi.render_indexed_batches_and_present(&batches, color)?)
        }
    }
}

fn validate_rhi_identity(
    expected: &RhiRenderIdentity,
    actual: &RhiRenderIdentity,
) -> Result<(), UiDirectRendererError> {
    if expected == actual {
        Ok(())
    } else {
        Err(UiDirectRendererError::StaleRhiIdentity {
            expected_device_generation: expected.device_generation,
            actual_device_generation: actual.device_generation,
            expected_surface_generation: expected.surface_generation,
            actual_surface_generation: actual.surface_generation,
            expected_surface_format: expected.surface_format.name.clone(),
            actual_surface_format: actual.surface_format.name.clone(),
            expected_surface_format_srgb: expected.surface_format.srgb,
            actual_surface_format_srgb: actual.surface_format.srgb,
            expected_surface_width: expected.surface_size.width,
            actual_surface_width: actual.surface_size.width,
            expected_surface_height: expected.surface_size.height,
            actual_surface_height: actual.surface_size.height,
            expected_surface_configured: expected.surface_configured,
            actual_surface_configured: actual.surface_configured,
        })
    }
}

fn validate_gpu_frame_identity(
    uploaded: UiDirectCacheKey,
    uploaded_rhi_identity: &RhiRenderIdentity,
    uploaded_layer_fingerprint: u64,
    vertex_bytes_len: usize,
    index_bytes_len: usize,
    atlas_size: (u32, u32),
    plan: &UiDirectFramePlan,
) -> Result<(), UiDirectRendererError> {
    let requested_layer_fingerprint = layer_plan_fingerprint(
        &plan.layer_passes,
        &plan.backdrop_passes,
        plan.diagnostics.layer_target_bytes,
    );
    if uploaded != plan.cache_key
        || uploaded_rhi_identity != &plan.rhi_identity
        || uploaded_layer_fingerprint != requested_layer_fingerprint
        || vertex_bytes_len != plan.vertex_bytes.len()
        || index_bytes_len != plan.index_bytes.len()
        || atlas_size != (plan.atlas.width, plan.atlas.height)
    {
        Err(UiDirectRendererError::StaleGpuFrame {
            uploaded_fingerprint: frame_fingerprint(
                uploaded,
                uploaded_layer_fingerprint,
                vertex_bytes_len,
                index_bytes_len,
                atlas_size,
            ),
            requested_fingerprint: frame_fingerprint(
                plan.cache_key,
                requested_layer_fingerprint,
                plan.vertex_bytes.len(),
                plan.index_bytes.len(),
                (plan.atlas.width, plan.atlas.height),
            ),
        })
    } else {
        Ok(())
    }
}

fn layer_plan_fingerprint(
    passes: &[UiDirectLayerPass],
    backdrops: &[UiDirectBackdropPass],
    target_bytes: u64,
) -> u64 {
    let mut state = fingerprint_mix(
        0xcbf2_9ce4_8422_2325,
        u64::try_from(passes.len()).unwrap_or(u64::MAX),
    );
    state = fingerprint_mix(state, target_bytes);
    for pass in passes {
        state = fingerprint_mix(state, pass.id.map_or(0, |id| id.0.wrapping_add(1)));
        state = fingerprint_mix(state, u64::from(pass.opacity_bits));
        state = fingerprint_mix(
            state,
            u64::try_from(pass.children.len()).unwrap_or(u64::MAX),
        );
        for child in &pass.children {
            state = fingerprint_mix(state, u64::try_from(*child).unwrap_or(u64::MAX));
        }
        state = fingerprint_mix(state, u64::try_from(pass.batches.len()).unwrap_or(u64::MAX));
        for batch in &pass.batches {
            state = fingerprint_mix(state, u64::try_from(batch.batch_index).unwrap_or(u64::MAX));
            state = fingerprint_mix(
                state,
                batch
                    .source_layer
                    .and_then(|index| u64::try_from(index).ok())
                    .map_or(0, |index| index.wrapping_add(1)),
            );
            state = fingerprint_mix(
                state,
                batch
                    .backdrop_source
                    .and_then(|index| u64::try_from(index).ok())
                    .map_or(0, |index| index.wrapping_add(1)),
            );
        }
    }
    for backdrop in backdrops {
        state = fingerprint_mix(
            state,
            u64::try_from(backdrop.consumer_pass).unwrap_or(u64::MAX),
        );
        state = fingerprint_mix(
            state,
            u64::try_from(backdrop.source_pass).unwrap_or(u64::MAX),
        );
        state = fingerprint_mix(
            state,
            u64::try_from(backdrop.source_batch_count).unwrap_or(u64::MAX),
        );
    }
    state
}

const fn fingerprint_mix(state: u64, value: u64) -> u64 {
    state.wrapping_mul(0x0000_0100_0000_01b3) ^ value
}

const FRAME_FINGERPRINT_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn surface_format_fingerprint(format_name: &str) -> u64 {
    fingerprint_bytes(FRAME_FINGERPRINT_OFFSET, format_name.as_bytes())
}

fn prepared_frame_content_fingerprint(
    vertex_bytes: &[u8],
    index_bytes: &[u8],
    atlas: &UiDirectAtlas,
    batches: &[UiDirectBatch],
    layer_passes: &[UiDirectLayerPass],
    backdrop_passes: &[UiDirectBackdropPass],
    layer_target_bytes: u64,
) -> u64 {
    let mut state = fingerprint_bytes(FRAME_FINGERPRINT_OFFSET, vertex_bytes);
    state = fingerprint_bytes(state, index_bytes);
    state = fingerprint_mix(state, u64::from(atlas.width));
    state = fingerprint_mix(state, u64::from(atlas.height));
    state = fingerprint_bytes(state, &atlas.rgba);
    state = fingerprint_mix(state, usize_to_u64(batches.len()));
    for batch in batches {
        state = fingerprint_mix(state, ui_direct_batch_kind_fingerprint(batch.kind));
        state = fingerprint_mix(state, ui_direct_primitive_kind_fingerprint(batch.primitive));
        state = fingerprint_mix(state, u64::from(batch.vertex_range.start));
        state = fingerprint_mix(state, u64::from(batch.vertex_range.end));
        state = fingerprint_mix(state, u64::from(batch.index_range.start));
        state = fingerprint_mix(state, u64::from(batch.index_range.end));
        state = match batch.scissor {
            Some(scissor) => {
                let state = fingerprint_mix(state, 1);
                let state = fingerprint_mix(state, u64::from(scissor.x));
                let state = fingerprint_mix(state, u64::from(scissor.y));
                let state = fingerprint_mix(state, u64::from(scissor.width));
                fingerprint_mix(state, u64::from(scissor.height))
            }
            None => fingerprint_mix(state, 0),
        };
        state = fingerprint_mix(state, u64::from(batch.stencil_reference));
    }
    fingerprint_mix(
        state,
        layer_plan_fingerprint(layer_passes, backdrop_passes, layer_target_bytes),
    )
}

fn fingerprint_bytes(mut state: u64, bytes: &[u8]) -> u64 {
    state = fingerprint_mix(state, usize_to_u64(bytes.len()));
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let mut word = [0_u8; 8];
        word.copy_from_slice(chunk);
        state = fingerprint_mix(state, u64::from_le_bytes(word));
    }
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let mut word = [0_u8; 8];
        word[..remainder.len()].copy_from_slice(remainder);
        state = fingerprint_mix(state, u64::from_le_bytes(word));
    }
    state
}

const fn ui_direct_batch_kind_fingerprint(kind: UiDirectBatchKind) -> u64 {
    match kind {
        UiDirectBatchKind::Content => 1,
        UiDirectBatchKind::GlyphMask => 2,
        UiDirectBatchKind::Image => 3,
        UiDirectBatchKind::Mesh => 4,
        UiDirectBatchKind::ClipPush => 5,
        UiDirectBatchKind::ClipPop => 6,
        UiDirectBatchKind::Layer => 7,
        UiDirectBatchKind::Shadow => 8,
        UiDirectBatchKind::BackdropFallback => 9,
        UiDirectBatchKind::BackdropEffect => 10,
    }
}

const fn ui_direct_primitive_kind_fingerprint(kind: UiDirectPrimitiveKind) -> u64 {
    match kind {
        UiDirectPrimitiveKind::Rect => 1,
        UiDirectPrimitiveKind::Border => 2,
        UiDirectPrimitiveKind::Text => 3,
        UiDirectPrimitiveKind::GlyphRun => 4,
        UiDirectPrimitiveKind::FocusIndicator => 5,
        UiDirectPrimitiveKind::RoundedRect => 6,
        UiDirectPrimitiveKind::Path => 7,
        UiDirectPrimitiveKind::Image => 8,
        UiDirectPrimitiveKind::Mesh => 9,
        UiDirectPrimitiveKind::PushClip => 10,
        UiDirectPrimitiveKind::PopClip => 11,
        UiDirectPrimitiveKind::BeginLayer => 12,
        UiDirectPrimitiveKind::EndLayer => 13,
        UiDirectPrimitiveKind::Shadow => 14,
        UiDirectPrimitiveKind::Backdrop => 15,
    }
}

fn frame_fingerprint(
    cache_key: UiDirectCacheKey,
    layer_fingerprint: u64,
    vertex_bytes_len: usize,
    index_bytes_len: usize,
    atlas_size: (u32, u32),
) -> u64 {
    [
        cache_key.display_revision,
        cache_key.device_generation,
        cache_key.surface_generation,
        u64::from(cache_key.surface_width),
        u64::from(cache_key.surface_height),
        u64::from(cache_key.rhi_surface_width),
        u64::from(cache_key.rhi_surface_height),
        cache_key.surface_format_fingerprint,
        u64::from(cache_key.surface_format_srgb),
        u64::from(cache_key.surface_configured),
        u64::from(cache_key.scale_milli),
        u64::from(cache_key.contrast_high),
        cache_key.image_revision,
        cache_key.mesh_revision,
        cache_key.effect_profile,
        cache_key.content_fingerprint,
        layer_fingerprint,
        usize_to_u64(vertex_bytes_len),
        usize_to_u64(index_bytes_len),
        u64::from(atlas_size.0),
        u64::from(atlas_size.1),
    ]
    .into_iter()
    .fold(FRAME_FINGERPRINT_OFFSET, fingerprint_mix)
}

fn ui_vertex_layout() -> Result<VertexLayout, UiDirectRendererError> {
    VertexLayout::new(
        VERTEX_STRIDE_BYTES,
        [
            VertexAttribute::new(VertexFormat::Float32x2, 0, 0),
            VertexAttribute::new(VertexFormat::Float32x2, 8, 1),
            VertexAttribute::new(VertexFormat::Float32x4, 16, 2),
        ],
    )
    .map_err(UiDirectRendererError::InvalidVertexLayout)
}

fn create_ui_direct_pipelines(
    rhi: &Rhi,
    layout: &VertexLayout,
) -> Result<UiDirectPipelines, UiDirectRendererError> {
    let binding = RenderPipelineBindings::single_texture();
    let pipeline = |label: &str,
                    shader: &str,
                    stencil: PipelineStencilConfig|
     -> Result<GpuRenderPipeline, UiDirectRendererError> {
        Ok(rhi.create_render_pipeline_with_layout_config(
            label,
            shader,
            "vs_main",
            Some("fs_main"),
            Some(layout),
            RenderPipelineConfig::premultiplied_alpha_surface_stencil(stencil)
                .with_bindings(binding),
        )?)
    };
    Ok(UiDirectPipelines {
        content: pipeline(
            "Meridian direct UI content",
            UI_DIRECT_SHADER,
            PipelineStencilConfig::read_equal_keep(),
        )?,
        composite: pipeline(
            "Meridian direct UI layer composite",
            UI_DIRECT_COMPOSITE_SHADER,
            PipelineStencilConfig::read_equal_keep(),
        )?,
        backdrop: pipeline(
            "Meridian direct UI backdrop filter",
            UI_DIRECT_BACKDROP_SHADER,
            PipelineStencilConfig::read_equal_keep(),
        )?,
        clip_push: pipeline(
            "Meridian direct UI clip push",
            UI_DIRECT_SHADER,
            PipelineStencilConfig::increment_equal(),
        )?,
        clip_pop: pipeline(
            "Meridian direct UI clip pop",
            UI_DIRECT_SHADER,
            PipelineStencilConfig::decrement_equal(),
        )?,
    })
}

fn primitive_kind(primitive: &DisplayPrimitive) -> UiDirectPrimitiveKind {
    match primitive {
        DisplayPrimitive::Rect { .. } => UiDirectPrimitiveKind::Rect,
        DisplayPrimitive::Border { .. } => UiDirectPrimitiveKind::Border,
        DisplayPrimitive::Text { .. } => UiDirectPrimitiveKind::Text,
        DisplayPrimitive::GlyphRun { .. } => UiDirectPrimitiveKind::GlyphRun,
        DisplayPrimitive::FocusIndicator { .. } => UiDirectPrimitiveKind::FocusIndicator,
        DisplayPrimitive::RoundedRect { .. } => UiDirectPrimitiveKind::RoundedRect,
        DisplayPrimitive::Path { .. } => UiDirectPrimitiveKind::Path,
        DisplayPrimitive::Image { .. } => UiDirectPrimitiveKind::Image,
        DisplayPrimitive::Mesh { .. } => UiDirectPrimitiveKind::Mesh,
        DisplayPrimitive::PushClip { .. } => UiDirectPrimitiveKind::PushClip,
        DisplayPrimitive::PopClip { .. } => UiDirectPrimitiveKind::PopClip,
        DisplayPrimitive::BeginLayer { .. } => UiDirectPrimitiveKind::BeginLayer,
        DisplayPrimitive::EndLayer { .. } => UiDirectPrimitiveKind::EndLayer,
        DisplayPrimitive::Shadow { .. } => UiDirectPrimitiveKind::Shadow,
        DisplayPrimitive::Backdrop { .. } => UiDirectPrimitiveKind::Backdrop,
    }
}

fn layer_target_bytes_per_target(target_size: WindowSize) -> Result<u64, UiDirectRendererError> {
    let bytes_per_target = u64::from(target_size.width)
        .checked_mul(u64::from(target_size.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(UiDirectRendererError::GeometryOverflow)?;
    Ok(bytes_per_target)
}

fn validate_viewport(viewport: UiSize, scale_factor: f32) -> Result<(), UiDirectRendererError> {
    if viewport.width.is_finite()
        && viewport.height.is_finite()
        && viewport.width > 0.0
        && viewport.height > 0.0
        && scale_factor.is_finite()
        && (0.5..=4.0).contains(&scale_factor)
    {
        Ok(())
    } else {
        Err(UiDirectRendererError::InvalidViewport)
    }
}

fn logical_to_pixels(value: f32, scale_factor: f32) -> Result<u32, UiDirectRendererError> {
    let pixels = logical_coordinate_to_pixels(value, scale_factor)?;
    if pixels == 0 {
        Ok(1)
    } else {
        Ok(pixels)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn logical_coordinate_to_pixels(
    value: f32,
    scale_factor: f32,
) -> Result<u32, UiDirectRendererError> {
    if !value.is_finite() || value < 0.0 || !scale_factor.is_finite() {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let pixels = (value * scale_factor).ceil();
    if pixels <= u32::MAX as f32 {
        Ok(pixels as u32)
    } else {
        Err(UiDirectRendererError::InvalidViewport)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn logical_min_coordinate_to_pixels(
    value: f32,
    scale_factor: f32,
) -> Result<u32, UiDirectRendererError> {
    if !value.is_finite() || value < 0.0 || !scale_factor.is_finite() {
        return Err(UiDirectRendererError::InvalidViewport);
    }
    let pixels = (value * scale_factor).floor();
    if pixels <= u32::MAX as f32 {
        Ok(pixels as u32)
    } else {
        Err(UiDirectRendererError::InvalidViewport)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scale_milli(scale_factor: f32) -> Result<u16, UiDirectRendererError> {
    if scale_factor.is_finite() && (0.5..=4.0).contains(&scale_factor) {
        Ok((scale_factor * 1000.0).round() as u16)
    } else {
        Err(UiDirectRendererError::InvalidViewport)
    }
}

fn scissor_for(
    bounds: UiRect,
    viewport: UiSize,
    scale_factor: f32,
) -> Result<ScissorDecision, UiDirectRendererError> {
    let max_width = logical_to_pixels(viewport.width, scale_factor)?;
    let max_height = logical_to_pixels(viewport.height, scale_factor)?;
    let left =
        logical_min_coordinate_to_pixels(bounds.origin.x.max(0.0), scale_factor)?.min(max_width);
    let top =
        logical_min_coordinate_to_pixels(bounds.origin.y.max(0.0), scale_factor)?.min(max_height);
    let right = logical_coordinate_to_pixels(
        (bounds.origin.x + bounds.size.width).clamp(0.0, viewport.width),
        scale_factor,
    )?
    .min(max_width);
    let bottom = logical_coordinate_to_pixels(
        (bounds.origin.y + bounds.size.height).clamp(0.0, viewport.height),
        scale_factor,
    )?
    .min(max_height);
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    if width == 0 || height == 0 {
        Ok(ScissorDecision::Skip)
    } else {
        Ok(ScissorDecision::Draw(Some(RenderScissor::new(
            left, top, width, height,
        ))))
    }
}

fn full_viewport_rect(viewport: UiSize) -> UiRect {
    UiRect::new(UiPoint { x: 0.0, y: 0.0 }, viewport)
}

fn glyph_rect(origin: UiRect, glyph: &UiGlyphBitmap, scale_factor: f32) -> UiRect {
    UiRect::new(
        UiPoint {
            x: origin.origin.x + glyph.origin.x / scale_factor,
            y: origin.origin.y + glyph.origin.y / scale_factor,
        },
        glyph_size(glyph, scale_factor),
    )
}

#[allow(clippy::cast_precision_loss)]
fn glyph_size(glyph: &UiGlyphBitmap, scale_factor: f32) -> UiSize {
    UiSize::new(
        glyph.width as f32 / scale_factor,
        glyph.height as f32 / scale_factor,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use meridian_platform::WindowSize;
    use meridian_rhi::{RhiRenderIdentity, SurfaceFormat};
    use meridian_ui_core::{UiColor, UiFontRole, UiNodeId};
    use meridian_ui_render::{
        UiBackdropDescriptor, UiImageHandle, UiLayerId, UiMeshHandle, UiPathCommand,
    };
    use meridian_ui_text::{UiTextLayout, UiTextRaster};

    use super::*;

    #[test]
    fn renderable_corpus_prepares_without_raster_bridge_fallback() {
        let image = UiImageHandle(42);
        let mesh = UiMeshHandle(77);
        let list = full_corpus(image, mesh);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let resources = UiDirectResourceSet::new(10, 20)
            .with_image(image)
            .with_mesh(mesh);

        let plan = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 99,
                display_list: &list,
                viewport: UiSize::new(320.0, 180.0),
                scale_factor: 2.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: false,
                },
                resources: &resources,
            })
            .expect("direct frame prepares renderable primitive corpus");

        assert_eq!(plan.diagnostics.observed_kinds.len(), 13);
        assert_eq!(plan.diagnostics.prepared_kinds.len(), 13);
        assert!(plan.diagnostics.unsupported_kinds.is_empty());
        assert!(plan.diagnostics.preparation_only_kinds.is_empty());
        assert!(!plan.diagnostics.full_frame_cpu_rasterized);
        assert_eq!(plan.diagnostics.image_count, 1);
        assert_eq!(plan.diagnostics.mesh_count, 1);
        assert_eq!(plan.diagnostics.backdrop_fallback_count, 1);
        assert!(plan.vertex_bytes() > 0);
        assert!(plan.index_bytes() > 0);
        assert_eq!(
            plan.vertex_bytes.len(),
            usize::try_from(plan.vertex_bytes()).expect("vertex bytes fit usize")
        );
        assert_eq!(
            plan.index_bytes.len(),
            usize::try_from(plan.index_bytes()).expect("index bytes fit usize")
        );
        assert_eq!(
            plan.atlas.rgba.len(),
            usize::try_from(plan.atlas.width)
                .expect("atlas width fits")
                .saturating_mul(usize::try_from(plan.atlas.height).expect("atlas height fits"))
                .saturating_mul(4)
        );
    }

    #[test]
    fn nested_layers_preserve_parent_order_and_reset_local_stencil() {
        let outer_layer = UiLayerId(10);
        let inner_layer = UiLayerId(20);
        let list = nested_layer_display_list(outer_layer, inner_layer);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let plan = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("nested isolated layer plan prepares");

        assert_eq!(plan.layer_passes.len(), 3);
        assert_eq!(plan.diagnostics.layer_count, 2);
        assert_eq!(plan.diagnostics.layer_target_bytes, 80_000);
        assert!(plan.diagnostics.unsupported_kinds.is_empty());
        assert!(plan.diagnostics.preparation_only_kinds.is_empty());

        let root = &plan.layer_passes[0];
        let outer = &plan.layer_passes[1];
        let inner = &plan.layer_passes[2];
        assert_eq!(root.id, None);
        assert_eq!(outer.id, Some(outer_layer));
        assert_eq!(inner.id, Some(inner_layer));
        assert_eq!(root.children, vec![1]);
        assert_eq!(outer.children, vec![2]);
        assert!(inner.children.is_empty());
        assert_eq!(root.batches.len(), 5);
        assert_eq!(outer.batches.len(), 6);
        assert_eq!(inner.batches.len(), 1);

        let root_kinds = root
            .batches
            .iter()
            .map(|planned| plan.batches[planned.batch_index].kind)
            .collect::<Vec<_>>();
        assert_eq!(
            root_kinds,
            vec![
                UiDirectBatchKind::Content,
                UiDirectBatchKind::ClipPush,
                UiDirectBatchKind::Layer,
                UiDirectBatchKind::ClipPop,
                UiDirectBatchKind::Content,
            ]
        );
        assert_eq!(root.batches[2].source_layer, Some(1));
        assert_eq!(outer.batches[3].source_layer, Some(2));
        assert_eq!(
            plan.batches[root.batches[2].batch_index].stencil_reference,
            1
        );
        assert_eq!(
            plan.batches[outer.batches[0].batch_index].stencil_reference,
            0
        );
        assert_eq!(
            plan.batches[outer.batches[1].batch_index].stencil_reference,
            0
        );
        assert_eq!(
            plan.batches[outer.batches[2].batch_index].stencil_reference,
            1
        );
        assert_eq!(
            plan.batches[outer.batches[3].batch_index].stencil_reference,
            1
        );
        assert_eq!(
            plan.batches[inner.batches[0].batch_index].stencil_reference,
            0
        );

        let outer_composite = &plan.batches[root.batches[2].batch_index];
        let inner_composite = &plan.batches[outer.batches[3].batch_index];
        let outer_alpha = decode_f32(
            &plan.vertex_bytes,
            usize::try_from(outer_composite.vertex_range.start).expect("vertex fits")
                * VERTEX_STRIDE_BYTES_USIZE
                + 28,
        );
        let inner_alpha = decode_f32(
            &plan.vertex_bytes,
            usize::try_from(inner_composite.vertex_range.start).expect("vertex fits")
                * VERTEX_STRIDE_BYTES_USIZE
                + 28,
        );
        assert_eq!(outer_alpha.to_bits(), 0.5_f32.to_bits());
        assert_eq!(inner_alpha.to_bits(), 0.25_f32.to_bits());
    }

    #[test]
    fn empty_isolated_layer_remains_a_clear_only_composite_target() {
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::BeginLayer {
                    id: UiLayerId(1),
                    opacity: 0.75,
                },
                DisplayPrimitive::EndLayer { id: UiLayerId(1) },
            ],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("empty isolated layer prepares as a transparent target");

        assert_eq!(plan.layer_passes.len(), 2);
        assert!(plan.layer_passes[1].batches.is_empty());
        assert_eq!(plan.layer_passes[0].children, vec![1]);
        assert_eq!(plan.layer_passes[0].batches[0].source_layer, Some(1));
        assert_eq!(plan.diagnostics.layer_target_bytes, 40_000);
    }

    #[test]
    fn aggregate_layer_target_limit_rejects_and_preserves_last_accepted_revision() {
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let accepted = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color: UiColor::surface(),
            }],
        };
        renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 7,
                display_list: &accepted,
                viewport: UiSize::new(512.0, 512.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("accepted frame prepares");

        let mut primitives = Vec::new();
        for index in 0_u64..65 {
            let id = UiLayerId(index + 1);
            primitives.push(DisplayPrimitive::BeginLayer { id, opacity: 1.0 });
            primitives.push(DisplayPrimitive::EndLayer { id });
        }
        let rejected = DisplayList { primitives };
        assert_eq!(
            layer_target_bytes_per_target(WindowSize::new(512, 512)),
            Ok(512 * 512 * 4)
        );
        let error = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 8,
                display_list: &rejected,
                viewport: UiSize::new(512.0, 512.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("65 full 512-square targets exceed the shared service guard");
        assert_eq!(
            error,
            UiDirectRendererError::TooManyLayerTargetBytes {
                bytes: 65 * 512 * 512 * 4,
                maximum: MAX_DIRECT_LAYER_TARGET_BYTES,
            }
        );
        assert_eq!(renderer.last_revision(), Some(7));
    }

    #[test]
    fn layer_opacity_participates_in_uploaded_plan_identity() {
        let resources = UiDirectResourceSet::default();
        let list = |opacity| DisplayList {
            primitives: vec![
                DisplayPrimitive::BeginLayer {
                    id: UiLayerId(1),
                    opacity,
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(1),
                    bounds: bounds(),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::EndLayer { id: UiLayerId(1) },
            ],
        };
        let first_list = list(0.25);
        let second_list = list(0.75);
        let request = |display_list| UiDirectPrepareRequest {
            display_revision: 9,
            display_list,
            viewport: UiSize::new(100.0, 100.0),
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities::default(),
            resources: &resources,
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let first = renderer
            .prepare_frame(request(&first_list))
            .expect("first layer plan prepares");
        let second = renderer
            .prepare_frame(request(&second_list))
            .expect("second layer plan prepares");
        assert_ne!(first.cache_key, second.cache_key);
        let first_fingerprint = layer_plan_fingerprint(
            &first.layer_passes,
            &first.backdrop_passes,
            first.diagnostics.layer_target_bytes,
        );
        let second_fingerprint = layer_plan_fingerprint(
            &second.layer_passes,
            &second.backdrop_passes,
            second.diagnostics.layer_target_bytes,
        );
        assert_ne!(first_fingerprint, second_fingerprint);
        assert!(matches!(
            validate_gpu_frame_identity(
                first.cache_key,
                &first.rhi_identity,
                first_fingerprint,
                first.vertex_bytes.len(),
                first.index_bytes.len(),
                (first.atlas.width, first.atlas.height),
                &second,
            ),
            Err(UiDirectRendererError::StaleGpuFrame { .. })
        ));
    }

    #[test]
    fn gpu_identity_rejects_same_revision_rect_payload_changes() {
        let list = |color| DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color,
            }],
        };
        let first_list = list(UiColor::surface());
        let second_list = list(UiColor::text());
        let resources = UiDirectResourceSet::default();
        let request = |display_list| UiDirectPrepareRequest {
            display_revision: 17,
            display_list,
            viewport: UiSize::new(100.0, 100.0),
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities::default(),
            resources: &resources,
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let uploaded = renderer
            .prepare_frame(request(&first_list))
            .expect("first rectangle prepares");
        let requested = renderer
            .prepare_frame(request(&second_list))
            .expect("second rectangle prepares");

        let mut uploaded_without_content = uploaded.cache_key;
        uploaded_without_content.content_fingerprint = 0;
        let mut requested_without_content = requested.cache_key;
        requested_without_content.content_fingerprint = 0;
        assert_eq!(uploaded_without_content, requested_without_content);
        assert_eq!(uploaded.vertex_bytes.len(), requested.vertex_bytes.len());
        assert_eq!(uploaded.index_bytes.len(), requested.index_bytes.len());
        assert_eq!(
            (uploaded.atlas.width, uploaded.atlas.height),
            (requested.atlas.width, requested.atlas.height)
        );
        assert_ne!(
            uploaded.cache_key.content_fingerprint,
            requested.cache_key.content_fingerprint
        );
        assert!(matches!(
            validate_gpu_frame_identity(
                uploaded.cache_key,
                &uploaded.rhi_identity,
                layer_plan_fingerprint(
                    &uploaded.layer_passes,
                    &uploaded.backdrop_passes,
                    uploaded.diagnostics.layer_target_bytes,
                ),
                uploaded.vertex_bytes.len(),
                uploaded.index_bytes.len(),
                (uploaded.atlas.width, uploaded.atlas.height),
                &requested,
            ),
            Err(UiDirectRendererError::StaleGpuFrame { .. })
        ));
    }

    #[test]
    fn gpu_identity_rejects_same_revision_image_payload_changes() {
        let image = UiImageHandle(9);
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Image {
                node: UiNodeId::new(1),
                bounds: bounds(),
                image,
                opacity: 1.0,
            }],
        };
        let uploaded_resources = UiDirectResourceSet::new(22, 0).with_image_descriptor(
            UiDirectImage::try_solid(image, 1, 1, [16, 32, 48, 255])
                .expect("bounded first image descriptor"),
        );
        let requested_resources = UiDirectResourceSet::new(22, 0).with_image_descriptor(
            UiDirectImage::try_solid(image, 1, 1, [48, 32, 16, 255])
                .expect("bounded second image descriptor"),
        );
        let request = |resources| UiDirectPrepareRequest {
            display_revision: 23,
            display_list: &list,
            viewport: UiSize::new(100.0, 100.0),
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities::default(),
            resources,
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let uploaded = renderer
            .prepare_frame(request(&uploaded_resources))
            .expect("first image prepares");
        let requested = renderer
            .prepare_frame(request(&requested_resources))
            .expect("second image prepares");

        let mut uploaded_without_content = uploaded.cache_key;
        uploaded_without_content.content_fingerprint = 0;
        let mut requested_without_content = requested.cache_key;
        requested_without_content.content_fingerprint = 0;
        assert_eq!(uploaded_without_content, requested_without_content);
        assert_eq!(uploaded.vertex_bytes.len(), requested.vertex_bytes.len());
        assert_eq!(uploaded.index_bytes.len(), requested.index_bytes.len());
        assert_eq!(
            (uploaded.atlas.width, uploaded.atlas.height),
            (requested.atlas.width, requested.atlas.height)
        );
        assert_ne!(uploaded.atlas.rgba, requested.atlas.rgba);
        assert_ne!(
            uploaded.cache_key.content_fingerprint,
            requested.cache_key.content_fingerprint
        );
        assert!(matches!(
            validate_gpu_frame_identity(
                uploaded.cache_key,
                &uploaded.rhi_identity,
                layer_plan_fingerprint(
                    &uploaded.layer_passes,
                    &uploaded.backdrop_passes,
                    uploaded.diagnostics.layer_target_bytes,
                ),
                uploaded.vertex_bytes.len(),
                uploaded.index_bytes.len(),
                (uploaded.atlas.width, uploaded.atlas.height),
                &requested,
            ),
            Err(UiDirectRendererError::StaleGpuFrame { .. })
        ));
    }

    #[test]
    fn missing_image_and_mesh_resources_are_typed_recoverable_errors() {
        let image = UiImageHandle(42);
        let mesh = UiMeshHandle(77);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let resources = UiDirectResourceSet::default();
        let image_list = DisplayList {
            primitives: vec![DisplayPrimitive::Image {
                node: UiNodeId::new(1),
                bounds: bounds(),
                image,
                opacity: 1.0,
            }],
        };
        let image_error = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &image_list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect_err("missing image is rejected");
        assert_eq!(image_error, UiDirectRendererError::MissingImage(image));

        let mesh_list = DisplayList {
            primitives: vec![DisplayPrimitive::Mesh {
                node: UiNodeId::new(1),
                bounds: bounds(),
                mesh,
                tint: UiColor::text(),
            }],
        };
        let mesh_error = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &mesh_list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect_err("missing mesh is rejected");
        assert_eq!(mesh_error, UiDirectRendererError::MissingMesh(mesh));
    }

    #[test]
    fn cache_key_changes_for_revision_surface_scale_contrast_and_assets() {
        let image = UiImageHandle(1);
        let mesh = UiMeshHandle(2);
        let list = full_corpus(image, mesh);
        let resources = UiDirectResourceSet::new(1, 1)
            .with_image(image)
            .with_mesh(mesh);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let base = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 80.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect("base prepares")
            .cache_key;

        let mut changed_identity = UiDirectGpuRenderer::new(identity(1, 2));
        let changed_surface = changed_identity
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 80.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect("surface prepares")
            .cache_key;
        assert_ne!(base, changed_surface);

        let changed_scale = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 80.0),
                scale_factor: 2.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect("scale prepares")
            .cache_key;
        assert_ne!(base, changed_scale);

        let changed_assets = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 2,
                display_list: &list,
                viewport: UiSize::new(100.0, 80.0),
                scale_factor: 1.0,
                contrast: UiContrast::High,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::new(2, 3)
                    .with_image(image)
                    .with_mesh(mesh),
            })
            .expect("asset prepares")
            .cache_key;
        assert_ne!(base, changed_assets);
    }

    #[test]
    fn device_and_surface_identity_recovery_preserves_frame_revision() {
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        renderer.record_cache_rebuild(3, 7);
        let surface = renderer.recover_identity(identity(1, 2), 44);
        assert_eq!(
            surface.action,
            UiDirectRendererRecoveryAction::RebuildSurfaceCaches
        );
        assert_eq!(surface.preserved_revision, 44);
        assert_eq!(surface.dropped_cache_count, 3);
        renderer.record_cache_rebuild(5, 11);
        let device = renderer.recover_identity(identity(2, 1), 45);
        assert_eq!(
            device.action,
            UiDirectRendererRecoveryAction::RebuildDeviceCaches
        );
        assert_eq!(device.preserved_revision, 45);
        assert_eq!(device.dropped_cache_count, 16);
        assert_eq!(renderer.last_revision(), Some(45));
    }

    #[test]
    fn opaque_backdrop_fallback_is_recorded_without_unsupported_category() {
        let image = UiImageHandle(42);
        let mesh = UiMeshHandle(77);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let list = full_corpus(image, mesh);
        let resources = UiDirectResourceSet::new(1, 1)
            .with_image(image)
            .with_mesh(mesh);
        let plan = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(320.0, 180.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &resources,
            })
            .expect("full corpus prepares");

        assert_eq!(plan.diagnostics.unsupported_kinds, BTreeSet::new());
        assert_eq!(plan.diagnostics.backdrop_effect_count, 0);
        assert_eq!(plan.diagnostics.backdrop_fallback_count, 1);
    }

    #[test]
    fn requested_backdrop_effect_builds_a_bounded_filter_source_pass() {
        let image = UiImageHandle(42);
        let mesh = UiMeshHandle(77);
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let mut list = full_corpus(image, mesh);
        let DisplayPrimitive::Backdrop { descriptor, .. } = list
            .primitives
            .last_mut()
            .expect("corpus includes backdrop")
        else {
            panic!("corpus ends with backdrop");
        };
        descriptor.sample_bounds = expand_rect(descriptor.bounds, 1.0);
        let resources = UiDirectResourceSet::new(1, 1)
            .with_image(image)
            .with_mesh(mesh);
        let plan = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(320.0, 180.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &resources,
            })
            .expect("bounded effect prepares");
        assert_eq!(plan.diagnostics.backdrop_effect_count, 1);
        assert_eq!(plan.diagnostics.backdrop_fallback_count, 0);
        assert_eq!(plan.backdrop_passes.len(), 1);
        assert_eq!(plan.diagnostics.layer_target_bytes, 320 * 180 * 4);
        assert!(plan.diagnostics.unsupported_kinds.is_empty());
    }

    #[test]
    fn first_backdrop_effect_uses_a_clear_only_root_source() {
        let b = bounds();
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Backdrop {
                node: UiNodeId::new(1),
                descriptor: UiBackdropDescriptor {
                    bounds: b,
                    sample_bounds: expand_rect(b, 1.0),
                    tint: UiColor::surface(),
                    opaque_fallback: UiColor::background(),
                },
            }],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect("root-first backdrop prepares");

        assert_eq!(plan.backdrop_passes.len(), 1);
        assert_eq!(plan.backdrop_passes[0].source_pass, 0);
        assert_eq!(plan.backdrop_passes[0].source_batch_count, 0);
        assert_eq!(plan.diagnostics.layer_target_bytes, 40_000);
    }

    #[test]
    fn layer_first_backdrop_uses_the_empty_parent_prefix() {
        let b = bounds();
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::BeginLayer {
                    id: UiLayerId(1),
                    opacity: 1.0,
                },
                DisplayPrimitive::Backdrop {
                    node: UiNodeId::new(1),
                    descriptor: UiBackdropDescriptor {
                        bounds: b,
                        sample_bounds: expand_rect(b, 1.0),
                        tint: UiColor::surface(),
                        opaque_fallback: UiColor::background(),
                    },
                },
                DisplayPrimitive::EndLayer { id: UiLayerId(1) },
            ],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect("layer-first backdrop prepares");

        assert_eq!(plan.backdrop_passes.len(), 1);
        assert_eq!(plan.backdrop_passes[0].source_pass, 0);
        assert_eq!(plan.backdrop_passes[0].source_batch_count, 0);
        assert_eq!(plan.diagnostics.layer_target_bytes, 80_000);
    }

    #[test]
    fn layered_backdrop_reconstructs_the_parent_prefix_and_shares_target_budget() {
        let b = bounds();
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(1),
                    bounds: b,
                    color: UiColor::background(),
                },
                DisplayPrimitive::BeginLayer {
                    id: UiLayerId(9),
                    opacity: 0.9,
                },
                DisplayPrimitive::Backdrop {
                    node: UiNodeId::new(2),
                    descriptor: UiBackdropDescriptor {
                        bounds: b,
                        sample_bounds: expand_rect(b, 1.0),
                        tint: UiColor::rgba(18.0 / 255.0, 21.0 / 255.0, 21.0 / 255.0, 0.5),
                        opaque_fallback: UiColor::surface(),
                    },
                },
                DisplayPrimitive::EndLayer { id: UiLayerId(9) },
            ],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect("layered backdrop prepares");
        assert_eq!(
            plan.backdrop_passes,
            vec![UiDirectBackdropPass {
                consumer_pass: 1,
                source_pass: 0,
                source_batch_count: 1,
            }]
        );
        assert_eq!(plan.diagnostics.layer_target_bytes, 80_000);
        assert!(plan.layer_passes[1]
            .batches
            .iter()
            .any(|batch| batch.backdrop_source == Some(0)));
    }

    #[test]
    fn insufficient_backdrop_padding_is_a_typed_direct_renderer_error() {
        let b = bounds();
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Backdrop {
                node: UiNodeId::new(1),
                descriptor: UiBackdropDescriptor {
                    bounds: b,
                    sample_bounds: b,
                    tint: UiColor::surface(),
                    opaque_fallback: UiColor::background(),
                },
            }],
        };
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("insufficient effect padding is rejected");
        assert_eq!(
            error,
            UiDirectRendererError::InvalidBackdropEffect(
                UiBackdropValidationError::InsufficientSamplePadding,
            )
        );
    }

    #[test]
    fn backdrop_padding_is_rejected_when_low_scale_expands_texel_reach() {
        let b = bounds();
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Backdrop {
                node: UiNodeId::new(1),
                descriptor: UiBackdropDescriptor {
                    bounds: b,
                    sample_bounds: expand_rect(b, 1.0),
                    tint: UiColor::surface(),
                    opaque_fallback: UiColor::background(),
                },
            }],
        };
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 0.5,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("one logical pixel is insufficient for one texel at half scale");
        assert_eq!(
            error,
            UiDirectRendererError::InvalidBackdropEffect(
                UiBackdropValidationError::InsufficientSamplePadding,
            )
        );
    }

    #[test]
    fn backdrop_padding_is_revalidated_after_physical_edge_snapping() {
        let bounds = UiRect::new(UiPoint { x: 10.49, y: 10.49 }, UiSize::new(20.02, 20.02));
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Backdrop {
                node: UiNodeId::new(1),
                descriptor: UiBackdropDescriptor {
                    bounds,
                    sample_bounds: expand_rect(bounds, 1.0),
                    tint: UiColor::surface(),
                    opaque_fallback: UiColor::background(),
                },
            }],
        };
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities {
                    backdrop_filtering: true,
                },
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("snapped bounds must retain one physical texel of declared padding");
        assert_eq!(
            error,
            UiDirectRendererError::InvalidBackdropEffect(
                UiBackdropValidationError::InsufficientSamplePadding,
            )
        );
    }

    #[test]
    fn negative_shadow_spread_is_rejected_before_direct_preparation() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Shadow {
                node: UiNodeId::new(1),
                bounds: bounds(),
                radii: UiCornerRadii::uniform(4.0),
                offset: UiPoint::default(),
                spread: -1.0,
                color: UiColor::background(),
            }],
        };
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("negative spread must fail display validation");
        assert_eq!(
            error,
            UiDirectRendererError::InvalidDisplayList(DisplayListError::InvalidGeometry {
                index: 0,
            })
        );
    }

    #[test]
    fn rejected_frame_preserves_last_prepared_revision() {
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let accepted = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color: UiColor::surface(),
            }],
        };
        renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 7,
                display_list: &accepted,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("accepted frame prepares");
        let rejected = DisplayList {
            primitives: vec![DisplayPrimitive::Image {
                node: UiNodeId::new(2),
                bounds: bounds(),
                image: UiImageHandle(999),
                opacity: 1.0,
            }],
        };
        assert!(renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 8,
                display_list: &rejected,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .is_err());
        assert_eq!(renderer.last_revision(), Some(7));
    }

    #[test]
    fn uploaded_gpu_identity_rejects_a_different_immutable_plan() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color: UiColor::surface(),
            }],
        };
        let resources = UiDirectResourceSet::default();
        let request = |display_revision| UiDirectPrepareRequest {
            display_revision,
            display_list: &list,
            viewport: UiSize::new(100.0, 100.0),
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities::default(),
            resources: &resources,
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let uploaded = renderer.prepare_frame(request(1)).expect("plan A prepares");
        let requested = renderer.prepare_frame(request(2)).expect("plan B prepares");
        assert_eq!(
            validate_gpu_frame_identity(
                uploaded.cache_key,
                &uploaded.rhi_identity,
                layer_plan_fingerprint(
                    &uploaded.layer_passes,
                    &uploaded.backdrop_passes,
                    uploaded.diagnostics.layer_target_bytes,
                ),
                uploaded.vertex_bytes.len(),
                uploaded.index_bytes.len(),
                (uploaded.atlas.width, uploaded.atlas.height),
                &uploaded,
            ),
            Ok(())
        );
        assert_eq!(
            validate_gpu_frame_identity(
                uploaded.cache_key,
                &uploaded.rhi_identity,
                layer_plan_fingerprint(
                    &uploaded.layer_passes,
                    &uploaded.backdrop_passes,
                    uploaded.diagnostics.layer_target_bytes,
                ),
                uploaded.vertex_bytes.len(),
                uploaded.index_bytes.len(),
                (uploaded.atlas.width, uploaded.atlas.height),
                &requested,
            ),
            Err(UiDirectRendererError::StaleGpuFrame {
                uploaded_fingerprint: frame_fingerprint(
                    uploaded.cache_key,
                    layer_plan_fingerprint(
                        &uploaded.layer_passes,
                        &uploaded.backdrop_passes,
                        uploaded.diagnostics.layer_target_bytes,
                    ),
                    uploaded.vertex_bytes.len(),
                    uploaded.index_bytes.len(),
                    (uploaded.atlas.width, uploaded.atlas.height),
                ),
                requested_fingerprint: frame_fingerprint(
                    requested.cache_key,
                    layer_plan_fingerprint(
                        &requested.layer_passes,
                        &requested.backdrop_passes,
                        requested.diagnostics.layer_target_bytes,
                    ),
                    requested.vertex_bytes.len(),
                    requested.index_bytes.len(),
                    (requested.atlas.width, requested.atlas.height),
                ),
            })
        );
    }

    #[test]
    fn fully_clipped_primitive_is_skipped_not_drawn_without_scissor() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: UiRect::new(UiPoint { x: 200.0, y: 200.0 }, UiSize::new(20.0, 20.0)),
                color: UiColor::surface(),
            }],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("offscreen content is a valid empty direct plan");
        assert!(plan.batches.is_empty());
        assert_eq!(plan.diagnostics.batch_count, 0);
        assert_eq!(plan.diagnostics.vertex_count, 0);
    }

    #[test]
    fn empty_clip_scope_skips_children_and_resumes_after_pop() {
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::PushClip {
                    id: UiClipId(1),
                    bounds: UiRect::new(UiPoint { x: 200.0, y: 200.0 }, UiSize::new(20.0, 20.0)),
                    radii: UiCornerRadii::uniform(2.0),
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(2),
                    bounds: bounds(),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::PopClip { id: UiClipId(1) },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(3),
                    bounds: bounds(),
                    color: UiColor::text(),
                },
            ],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("offscreen clip is a valid empty scope");
        assert_eq!(plan.batches.len(), 1);
        assert_eq!(plan.batches[0].kind, UiDirectBatchKind::Content);
        assert_eq!(plan.diagnostics.clip_scope_count, 1);
    }

    #[test]
    fn layer_inside_empty_parent_clip_never_becomes_an_unscissored_composite() {
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::PushClip {
                    id: UiClipId(1),
                    bounds: UiRect::new(UiPoint { x: 200.0, y: 200.0 }, UiSize::new(20.0, 20.0)),
                    radii: UiCornerRadii::uniform(2.0),
                },
                DisplayPrimitive::BeginLayer {
                    id: UiLayerId(1),
                    opacity: 0.8,
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(2),
                    bounds: bounds(),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::EndLayer { id: UiLayerId(1) },
                DisplayPrimitive::PopClip { id: UiClipId(1) },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(3),
                    bounds: bounds(),
                    color: UiColor::text(),
                },
            ],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("empty parent clip remains a valid isolated layer plan");
        assert_eq!(plan.layer_passes.len(), 1);
        assert!(plan.layer_passes[0].children.is_empty());
        assert!(plan.layer_passes[0]
            .batches
            .iter()
            .all(|batch| batch.source_layer.is_none()));
        assert_eq!(plan.layer_passes[0].batches.len(), 1);
        assert_eq!(
            plan.batches[plan.layer_passes[0].batches[0].batch_index].kind,
            UiDirectBatchKind::Content
        );
        assert_eq!(plan.diagnostics.layer_count, 0);
        assert_eq!(plan.diagnostics.layer_target_bytes, 0);
    }

    #[test]
    fn clip_depth_over_stencil_range_is_rejected_before_rhi_submission() {
        let mut primitives = Vec::new();
        for depth in 0..=255_u64 {
            primitives.push(DisplayPrimitive::PushClip {
                id: UiClipId(depth + 1),
                bounds: bounds(),
                radii: UiCornerRadii::uniform(2.0),
            });
        }
        for depth in (0..=255_u64).rev() {
            primitives.push(DisplayPrimitive::PopClip {
                id: UiClipId(depth + 1),
            });
        }
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &DisplayList { primitives },
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("256 nested clips exceed the eight-bit stencil contract");
        assert_eq!(
            error,
            UiDirectRendererError::ClipDepthOverflow {
                depth: 256,
                maximum: 255,
            }
        );
    }

    #[test]
    fn atlas_uvs_use_final_height_and_half_texel_gutters() {
        let image = UiImageHandle(42);
        let mesh = UiMeshHandle(77);
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &full_corpus(image, mesh),
                viewport: UiSize::new(320.0, 180.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::new(1, 1)
                    .with_image(image)
                    .with_mesh(mesh),
            })
            .expect("corpus prepares");
        assert!(plan.atlas.height > 1);
        let third_vertex_v = decode_f32(&plan.vertex_bytes, VERTEX_STRIDE_BYTES_USIZE * 2 + 12);
        let expected = 1.5_f64 / f64::from(plan.atlas.height);
        assert!((f64::from(third_vertex_v) - expected).abs() < 0.000_001);
        assert!(third_vertex_v < 1.0);

        let mut atlas = AtlasBuilder::new(3);
        let red = atlas
            .push_rgba(1, 1, &[255, 0, 0, 255])
            .expect("red region fits");
        let blue = atlas
            .push_rgba(1, 1, &[0, 0, 255, 255])
            .expect("blue region fits");
        let atlas = atlas.finish().expect("guttered atlas finishes");
        assert_eq!((red.y, blue.y), (1, 4));
        assert_eq!(
            &atlas.rgba[0..12],
            &[255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
        );
        assert_eq!(
            &atlas.rgba[36..48],
            &[0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255]
        );
    }

    #[test]
    fn atlas_reuses_identical_content_without_growing_the_upload() {
        let mut atlas = AtlasBuilder::new(3);
        let first = atlas
            .push_rgba(1, 1, &[255, 0, 0, 255])
            .expect("first region fits");
        let repeated = atlas
            .push_rgba(1, 1, &[255, 0, 0, 255])
            .expect("identical region reuses prior content");
        assert_eq!(repeated, first);
        assert_eq!(atlas.rows.len(), 1);
        assert_eq!(atlas.height(), 3);
        assert_eq!(atlas.finish().expect("atlas finishes").rgba.len(), 36);
    }

    #[test]
    fn diagonal_and_curved_path_strokes_emit_oriented_bounded_geometry() {
        let curve = [
            UiPathCommand::MoveTo(UiPoint { x: 10.0, y: 10.0 }),
            UiPathCommand::QuadraticTo {
                control: UiPoint { x: 15.0, y: 30.0 },
                end: UiPoint { x: 30.0, y: 30.0 },
            },
        ];
        let subpaths = flatten_path(&curve, 1.0).expect("curve flattens");
        let high_dpi = flatten_path(&curve, 4.0).expect("high-DPI curve flattens");
        assert!(subpaths[0].points.len() > 2);
        assert!(high_dpi[0].points.len() > subpaths[0].points.len());
        let (vertices, indices) = stroked_path_geometry(
            &subpaths,
            UiStroke::new(UiColor::text(), 2.0),
            AtlasRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            1,
            1,
            UiSize::new(100.0, 100.0),
        )
        .expect("curve stroke tessellates");
        assert!(vertices.len() > 4);
        assert!(indices.len() > 6);

        let diagonal = flatten_path(
            &[
                UiPathCommand::MoveTo(UiPoint { x: 10.0, y: 10.0 }),
                UiPathCommand::LineTo(UiPoint { x: 20.0, y: 20.0 }),
            ],
            1.0,
        )
        .expect("diagonal flattens");
        let (vertices, _) = stroked_path_geometry(
            &diagonal,
            UiStroke {
                color: UiColor::text(),
                width: 2.0,
                line_cap: UiLineCap::Butt,
                line_join: UiLineJoin::Bevel,
            },
            AtlasRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            1,
            1,
            UiSize::new(100.0, 100.0),
        )
        .expect("diagonal stroke tessellates");
        let edge_vector = [
            vertices[1].position[0] - vertices[0].position[0],
            vertices[1].position[1] - vertices[0].position[1],
        ];
        assert!(edge_vector[0].abs() > 0.1);
        assert!(edge_vector[1].abs() > 0.1);
        assert!((edge_vector[0].abs() - edge_vector[1].abs()).abs() < 0.000_1);
    }

    #[test]
    fn concave_fill_uses_bounded_ear_clipping_and_degenerate_fill_is_rejected() {
        let concave = FlatSubpath {
            points: vec![
                UiPoint { x: 0.0, y: 0.0 },
                UiPoint { x: 4.0, y: 0.0 },
                UiPoint { x: 4.0, y: 4.0 },
                UiPoint { x: 2.0, y: 2.0 },
                UiPoint { x: 0.0, y: 4.0 },
            ],
            closed: true,
        };
        let (vertices, indices) = filled_path_geometry(
            &[concave],
            UiColor::text(),
            AtlasRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            1,
            1,
            UiSize::new(10.0, 10.0),
        )
        .expect("concave simple polygon triangulates");
        assert_eq!(vertices.len(), 5);
        assert_eq!(indices.len(), 9);

        assert!(matches!(
            filled_path_geometry(
                &[FlatSubpath {
                    points: vec![
                        UiPoint { x: 0.0, y: 0.0 },
                        UiPoint { x: 1.0, y: 1.0 },
                        UiPoint { x: 2.0, y: 2.0 },
                    ],
                    closed: true,
                }],
                UiColor::text(),
                AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                1,
                1,
                UiSize::new(10.0, 10.0),
            ),
            Err(UiDirectRendererError::UnsupportedPathGeometry)
        ));

        let large_convex = (0_u16..512)
            .map(|index| {
                let angle = f32::from(index) / 512.0 * std::f32::consts::TAU;
                UiPoint {
                    x: angle.cos() * 10.0,
                    y: angle.sin() * 10.0,
                }
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            triangulate_polygon(&large_convex),
            Err(UiDirectRendererError::PathTessellationBudgetExceeded { .. })
        ));
    }

    #[test]
    fn fractional_scissors_and_hidpi_glyphs_preserve_logical_coverage() {
        assert_eq!(
            scissor_for(
                UiRect::new(UiPoint { x: 0.2, y: 0.7 }, UiSize::new(1.0, 1.0)),
                UiSize::new(10.0, 10.0),
                1.0,
            ),
            Ok(ScissorDecision::Draw(Some(RenderScissor::new(0, 0, 2, 2))))
        );
        let glyph = UiGlyphBitmap {
            origin: UiPoint { x: 4.0, y: 6.0 },
            width: 8,
            height: 10,
            alpha: vec![255; 80],
        };
        assert_eq!(
            glyph_rect(
                UiRect::new(UiPoint { x: 10.0, y: 20.0 }, UiSize::new(20.0, 20.0)),
                &glyph,
                2.0,
            ),
            UiRect::new(UiPoint { x: 12.0, y: 23.0 }, UiSize::new(4.0, 5.0))
        );
        assert_eq!(
            ndc(
                UiPoint {
                    x: f32::MAX,
                    y: 0.0,
                },
                UiSize::new(f32::MIN_POSITIVE, 1.0),
            ),
            Err(UiDirectRendererError::InvalidViewport)
        );
    }

    #[test]
    fn image_mesh_and_geometry_bounds_are_typed_before_growth() {
        let oversized_width =
            u32::try_from(MAX_DIRECT_IMAGE_BYTES / 4 + 1).expect("test width fits u32");
        assert!(matches!(
            UiDirectImage::try_solid(UiImageHandle(1), oversized_width, 1, [255; 4]),
            Err(UiDirectRendererError::TooManyImageBytes { .. })
        ));
        let invalid_mesh = UiDirectMesh {
            handle: UiMeshHandle(2),
            vertices: vec![
                UiDirectMeshVertex::new(0, 0, 0, 0),
                UiDirectMeshVertex::new(1001, 0, 1000, 0),
                UiDirectMeshVertex::new(0, 1000, 0, 1000),
            ],
            indices: vec![0, 1, 2],
        };
        assert_eq!(
            validate_mesh(&invalid_mesh),
            Err(UiDirectRendererError::InvalidMesh(invalid_mesh.handle))
        );
        assert!(matches!(
            ensure_geometry_estimate(1, (1, 1)),
            Err(UiDirectRendererError::TooManyGeometryBytes { .. })
        ));
        assert!(matches!(
            AtlasBuilder::new(u32::MAX).push_white(),
            Err(UiDirectRendererError::TooManyAtlasBytes { .. })
        ));
    }

    #[test]
    fn zero_area_whitespace_glyphs_preserve_text_without_atlas_geometry() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Text {
                node: UiNodeId::new(1),
                bounds: bounds(),
                text: " ".to_owned(),
                color: UiColor::text(),
                layout: UiTextLayout {
                    line_count: 1,
                    glyph_count: 1,
                    width: 8.0,
                    height: 16.0,
                    used_fallback_metrics: false,
                    used_fallback_font: false,
                    font_role: UiFontRole::Interface,
                },
                raster: UiTextRaster {
                    glyphs: vec![UiGlyphBitmap {
                        origin: UiPoint { x: 0.0, y: 0.0 },
                        width: 0,
                        height: 0,
                        alpha: Vec::new(),
                    }],
                    has_unrasterized_glyphs: false,
                },
            }],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("whitespace-only text prepares");
        assert_eq!(plan.diagnostics.glyph_mask_count, 1);
        assert_eq!(plan.diagnostics.batch_count, 1);
        assert_eq!(plan.atlas.width, 3);
        assert_eq!(plan.atlas.height, 3);
    }

    #[test]
    fn glyph_batches_are_bounded_independently_of_primitive_count() {
        let glyph = UiGlyphBitmap {
            origin: UiPoint { x: 0.0, y: 0.0 },
            width: 1,
            height: 1,
            alpha: vec![255],
        };
        let glyph_count = MAX_DIRECT_BATCHES + 1;
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Text {
                node: UiNodeId::new(1),
                bounds: bounds(),
                text: "bounded".to_owned(),
                color: UiColor::text(),
                layout: UiTextLayout {
                    line_count: 1,
                    glyph_count,
                    width: 32.0,
                    height: 16.0,
                    used_fallback_metrics: false,
                    used_fallback_font: false,
                    font_role: UiFontRole::Interface,
                },
                raster: UiTextRaster {
                    glyphs: vec![glyph; glyph_count],
                    has_unrasterized_glyphs: false,
                },
            }],
        };
        let error = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect_err("one primitive cannot create unbounded glyph batches");
        assert_eq!(
            error,
            UiDirectRendererError::TooManyBatches {
                count: MAX_DIRECT_BATCHES + 1,
                maximum: MAX_DIRECT_BATCHES,
            }
        );
    }

    #[test]
    fn incomplete_text_rasters_are_rejected_before_frame_recovery() {
        let resources = UiDirectResourceSet::default();
        let accepted = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color: UiColor::surface(),
            }],
        };
        let incomplete_text = DisplayList {
            primitives: vec![DisplayPrimitive::Text {
                node: UiNodeId::new(2),
                bounds: bounds(),
                text: "incomplete text".to_owned(),
                color: UiColor::text(),
                layout: text_layout(),
                raster: UiTextRaster {
                    glyphs: Vec::new(),
                    has_unrasterized_glyphs: true,
                },
            }],
        };
        let incomplete_glyph_run = DisplayList {
            primitives: vec![DisplayPrimitive::GlyphRun {
                node: UiNodeId::new(3),
                bounds: bounds(),
                text: "incomplete glyph run".to_owned(),
                color: UiColor::text(),
                layout: text_layout(),
                raster: UiTextRaster {
                    glyphs: Vec::new(),
                    has_unrasterized_glyphs: true,
                },
            }],
        };
        let missing_text_payload = DisplayList {
            primitives: vec![DisplayPrimitive::Text {
                node: UiNodeId::new(4),
                bounds: bounds(),
                text: "missing text glyph payload".to_owned(),
                color: UiColor::text(),
                layout: text_layout(),
                raster: UiTextRaster {
                    glyphs: Vec::new(),
                    has_unrasterized_glyphs: false,
                },
            }],
        };
        let missing_glyph_run_payload = DisplayList {
            primitives: vec![DisplayPrimitive::GlyphRun {
                node: UiNodeId::new(5),
                bounds: bounds(),
                text: "missing glyph run payload".to_owned(),
                color: UiColor::text(),
                layout: text_layout(),
                raster: UiTextRaster {
                    glyphs: Vec::new(),
                    has_unrasterized_glyphs: false,
                },
            }],
        };
        let request = |display_revision, display_list| UiDirectPrepareRequest {
            display_revision,
            display_list,
            viewport: UiSize::new(100.0, 100.0),
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            effects: UiEffectCapabilities::default(),
            resources: &resources,
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        renderer
            .prepare_frame(request(1, &accepted))
            .expect("accepted frame establishes the last successful revision");
        assert_eq!(renderer.last_revision(), Some(1));
        assert_eq!(
            renderer
                .prepare_frame(request(2, &incomplete_text))
                .expect_err("incomplete text must not submit a partial glyph frame"),
            UiDirectRendererError::IncompleteTextRaster(UiDirectPrimitiveKind::Text)
        );
        assert_eq!(
            renderer
                .prepare_frame(request(3, &incomplete_glyph_run))
                .expect_err("incomplete glyph runs must not submit a partial glyph frame"),
            UiDirectRendererError::IncompleteTextRaster(UiDirectPrimitiveKind::GlyphRun)
        );
        assert_eq!(
            renderer
                .prepare_frame(request(4, &missing_text_payload))
                .expect_err("a nonempty text layout cannot omit its glyph payload"),
            UiDirectRendererError::IncompleteTextRaster(UiDirectPrimitiveKind::Text)
        );
        assert_eq!(renderer.last_revision(), Some(1));
        assert_eq!(
            renderer
                .prepare_frame(request(5, &missing_glyph_run_payload))
                .expect_err("a nonempty glyph run layout cannot omit its glyph payload"),
            UiDirectRendererError::IncompleteTextRaster(UiDirectPrimitiveKind::GlyphRun)
        );
        assert_eq!(renderer.last_revision(), Some(1));
    }

    #[test]
    fn rhi_identity_changes_are_rejected_before_direct_submission() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(1),
                bounds: bounds(),
                color: UiColor::surface(),
            }],
        };
        let plan = UiDirectGpuRenderer::new(identity(3, 5))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 8,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("plan prepares");
        assert_eq!(
            validate_rhi_identity(&plan.rhi_identity, &identity(3, 5)),
            Ok(())
        );
        let generation_changed = validate_rhi_identity(&plan.rhi_identity, &identity(3, 6))
            .expect_err("a changed surface generation invalidates a prepared frame");
        assert!(matches!(
            generation_changed,
            UiDirectRendererError::StaleRhiIdentity {
                expected_surface_generation: 5,
                actual_surface_generation: 6,
                ..
            }
        ));

        let mut format_changed = identity(3, 5);
        format_changed.surface_format.name = "Rgba8UnormSrgb".to_owned();
        let format_changed = validate_rhi_identity(&plan.rhi_identity, &format_changed)
            .expect_err("a changed surface format invalidates a prepared frame");
        assert!(matches!(
            format_changed,
            UiDirectRendererError::StaleRhiIdentity {
                expected_surface_format,
                actual_surface_format,
                ..
            } if expected_surface_format == "Bgra8UnormSrgb"
                && actual_surface_format == "Rgba8UnormSrgb"
        ));

        let mut size_changed = identity(3, 5);
        size_changed.surface_size = WindowSize::new(640, 360);
        let size_changed = validate_rhi_identity(&plan.rhi_identity, &size_changed)
            .expect_err("a changed surface size invalidates a prepared frame");
        assert!(matches!(
            size_changed,
            UiDirectRendererError::StaleRhiIdentity {
                expected_surface_width: 320,
                expected_surface_height: 180,
                actual_surface_width: 640,
                actual_surface_height: 360,
                ..
            }
        ));

        let mut configuration_changed = identity(3, 5);
        configuration_changed.surface_configured = false;
        let configuration_changed =
            validate_rhi_identity(&plan.rhi_identity, &configuration_changed)
                .expect_err("a changed surface configuration invalidates a prepared frame");
        assert!(matches!(
            configuration_changed,
            UiDirectRendererError::StaleRhiIdentity {
                expected_surface_configured: true,
                actual_surface_configured: false,
                ..
            }
        ));
    }

    fn decode_f32(bytes: &[u8], offset: usize) -> f32 {
        f32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("encoded f32 is present"),
        )
    }

    #[test]
    fn authored_srgb_tokens_are_linearized_before_vertex_upload() {
        let dark = color_array(UiColor::rgba(9.0 / 255.0, 11.0 / 255.0, 11.0 / 255.0, 0.75));
        assert!((dark[0] - 0.002_731_742_9).abs() < 0.000_000_1);
        assert!((dark[1] - 0.003_346_535_8).abs() < 0.000_000_1);
        assert!((dark[2] - 0.003_346_535_8).abs() < 0.000_000_1);
        assert!((dark[3] - 0.75).abs() < f32::EPSILON);

        let text = color_array(UiColor::rgba(
            227.0 / 255.0,
            225.0 / 255.0,
            216.0 / 255.0,
            1.0,
        ));
        assert!((text[0] - 0.768_151_2).abs() < 0.000_001);
        assert!((text[1] - 0.752_942_3).abs() < 0.000_001);
        assert!((text[2] - 0.686_685_3).abs() < 0.000_001);
        assert!((text[3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn non_srgb_surface_is_rejected_instead_of_silently_misencoding_ui_color() {
        let mut non_srgb = identity(1, 1);
        non_srgb.surface_format.srgb = false;
        non_srgb.surface_format.name = "Bgra8Unorm".to_owned();
        let mut renderer = UiDirectGpuRenderer::new(non_srgb);
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(100),
                bounds: bounds(),
                color: UiColor::background(),
            }],
        };
        assert_eq!(
            renderer.prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            }),
            Err(UiDirectRendererError::UnsupportedSurfaceColorSpace)
        );
    }

    #[test]
    fn offscreen_capture_copy_source_unavailability_is_typed_separately() {
        assert!(is_offscreen_capture_target_capability_error(
            RhiErrorKind::SurfaceUnsupported
        ));
        assert!(!is_offscreen_capture_target_capability_error(
            RhiErrorKind::DeviceLost
        ));
        assert!(!is_offscreen_capture_target_capability_error(
            RhiErrorKind::InvalidTextureSize
        ));
        assert_eq!(
            UiDirectRendererError::OffscreenCaptureUnsupported {
                rhi_kind: RhiErrorKind::SurfaceUnsupported,
            }
            .rhi_kind(),
            Some(RhiErrorKind::SurfaceUnsupported)
        );
    }

    #[test]
    fn axis_aligned_geometry_snaps_to_physical_pixels() {
        let snapped = snap_rect_to_physical(
            UiRect::new(UiPoint { x: 0.26, y: 0.24 }, UiSize::new(10.1, 5.25)),
            2.0,
        )
        .expect("finite 2x geometry snaps");
        assert_eq!(snapped.origin, UiPoint { x: 0.5, y: 0.0 });
        assert_eq!(snapped.size, UiSize::new(10.0, 5.5));
        assert!((snap_length_to_physical(0.2, 2.0) - 0.5).abs() < f32::EPSILON);
        assert!((snap_length_to_physical(1.0, 2.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rounded_rect_tessellation_adapts_to_physical_radius() {
        let small = rounded_rect_corner_segments(2.0, 1.0);
        let large = rounded_rect_corner_segments(48.0, 2.0);
        let high_dpi = rounded_rect_corner_segments(48.0, 4.0);
        assert!(small >= 1);
        assert!(large > small);
        assert!(high_dpi > large);
        assert!(usize::try_from(high_dpi)
            .is_ok_and(|segments| segments <= MAX_PATH_COMMANDS_PER_PRIMITIVE));
    }

    #[test]
    fn rounded_rect_content_has_one_physical_pixel_alpha_fringe() {
        let (vertices, indices) = rounded_rect_geometry(
            bounds(),
            UiCornerRadii::uniform(6.0),
            UiColor::surface(),
            AtlasRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            1,
            1,
            UiSize::new(100.0, 100.0),
            2.0,
            true,
        )
        .expect("antialiased rounded rect tessellates");
        assert!(vertices.len() > 16);
        assert!(indices.len() > 24);
        assert!((vertices[0].color[3] - 1.0).abs() < f32::EPSILON);
        assert!((vertices.last().expect("outer fringe").color[3]).abs() < f32::EPSILON);
    }

    #[test]
    fn shadow_spread_emits_bounded_outer_to_inner_alpha_falloff() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Shadow {
                node: UiNodeId::new(50),
                bounds: bounds(),
                radii: UiCornerRadii::uniform(6.0),
                offset: UiPoint { x: 0.0, y: 2.0 },
                spread: 4.0,
                color: UiColor::rgba(0.0, 0.0, 0.0, 0.4),
            }],
        };
        let plan = UiDirectGpuRenderer::new(identity(1, 1))
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 1.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("soft shadow prepares");
        assert_eq!(plan.diagnostics.shadow_count, 1);
        assert_eq!(plan.batches.len(), SHADOW_FALLOFF_WEIGHTS.len());
        let alphas = plan
            .batches
            .iter()
            .map(|batch| {
                usize::try_from(batch.vertex_range.start)
                    .ok()
                    .and_then(|vertex| vertex.checked_mul(VERTEX_STRIDE_BYTES_USIZE))
                    .map(|offset| decode_f32(&plan.vertex_bytes, offset + 28))
                    .expect("vertex alpha exists")
            })
            .collect::<Vec<_>>();
        assert!(alphas.windows(2).all(|pair| pair[0] < pair[1]));
        assert!((alphas[0] - 0.04).abs() < 0.000_001);
        assert!((alphas[3] - 0.20).abs() < 0.000_001);
    }

    #[test]
    fn path_joins_and_caps_use_declared_wedges_and_sectors() {
        let open = FlatSubpath {
            points: vec![
                UiPoint { x: 10.0, y: 20.0 },
                UiPoint { x: 20.0, y: 20.0 },
                UiPoint { x: 20.0, y: 30.0 },
            ],
            closed: false,
        };
        let geometry = |line_cap, line_join| {
            stroked_path_geometry(
                std::slice::from_ref(&open),
                UiStroke {
                    color: UiColor::text(),
                    width: 4.0,
                    line_cap,
                    line_join,
                },
                AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                1,
                1,
                UiSize::new(100.0, 100.0),
            )
            .expect("stroke tessellates")
        };
        let (_, bevel_indices) = geometry(UiLineCap::Butt, UiLineJoin::Bevel);
        let (_, miter_indices) = geometry(UiLineCap::Butt, UiLineJoin::Miter);
        let (_, round_indices) = geometry(UiLineCap::Round, UiLineJoin::Round);
        assert_eq!(bevel_indices.len(), miter_indices.len());
        assert!(round_indices.len() > bevel_indices.len());
        assert!(
            (directed_angle_sweep(0.0, std::f32::consts::FRAC_PI_2, 1.0)
                - std::f32::consts::FRAC_PI_2)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn prepared_rect_vertices_use_snapped_physical_edges_at_two_x() {
        let list = DisplayList {
            primitives: vec![DisplayPrimitive::Rect {
                node: UiNodeId::new(99),
                bounds: UiRect::new(UiPoint { x: 0.26, y: 0.24 }, UiSize::new(10.1, 5.25)),
                color: UiColor::background(),
            }],
        };
        let mut renderer = UiDirectGpuRenderer::new(identity(1, 1));
        let plan = renderer
            .prepare_frame(UiDirectPrepareRequest {
                display_revision: 1,
                display_list: &list,
                viewport: UiSize::new(100.0, 100.0),
                scale_factor: 2.0,
                contrast: UiContrast::Standard,
                effects: UiEffectCapabilities::default(),
                resources: &UiDirectResourceSet::default(),
            })
            .expect("snapped rect prepares");
        assert!((decode_f32(&plan.vertex_bytes, 0) - -0.99).abs() < 0.000_001);
        assert!((decode_f32(&plan.vertex_bytes, 4) - 1.0).abs() < 0.000_001);
    }

    fn nested_layer_display_list(outer_layer: UiLayerId, inner_layer: UiLayerId) -> DisplayList {
        let outer_clip = UiClipId(1);
        let inner_clip = UiClipId(2);
        DisplayList {
            primitives: vec![
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(1),
                    bounds: bounds(),
                    color: UiColor::background(),
                },
                DisplayPrimitive::PushClip {
                    id: outer_clip,
                    bounds: bounds(),
                    radii: UiCornerRadii::uniform(2.0),
                },
                DisplayPrimitive::BeginLayer {
                    id: outer_layer,
                    opacity: 0.5,
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(2),
                    bounds: bounds(),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::PushClip {
                    id: inner_clip,
                    bounds: bounds(),
                    radii: UiCornerRadii::uniform(2.0),
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(3),
                    bounds: bounds(),
                    color: UiColor::text(),
                },
                DisplayPrimitive::BeginLayer {
                    id: inner_layer,
                    opacity: 0.25,
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(4),
                    bounds: bounds(),
                    color: UiColor::amber(),
                },
                DisplayPrimitive::EndLayer { id: inner_layer },
                DisplayPrimitive::PopClip { id: inner_clip },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(5),
                    bounds: bounds(),
                    color: UiColor::grass(),
                },
                DisplayPrimitive::EndLayer { id: outer_layer },
                DisplayPrimitive::PopClip { id: outer_clip },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(6),
                    bounds: bounds(),
                    color: UiColor::text(),
                },
            ],
        }
    }

    fn identity(device_generation: u64, surface_generation: u64) -> RhiRenderIdentity {
        RhiRenderIdentity {
            device_generation,
            surface_generation,
            surface_format: SurfaceFormat {
                name: "Bgra8UnormSrgb".to_owned(),
                srgb: true,
            },
            surface_size: WindowSize::new(320, 180),
            surface_configured: true,
        }
    }

    fn bounds() -> UiRect {
        UiRect::new(UiPoint { x: 8.0, y: 8.0 }, UiSize::new(32.0, 24.0))
    }

    fn text_layout() -> UiTextLayout {
        UiTextLayout {
            line_count: 1,
            glyph_count: 1,
            width: 8.0,
            height: 8.0,
            used_fallback_metrics: false,
            used_fallback_font: false,
            font_role: UiFontRole::Interface,
        }
    }

    fn text_raster() -> UiTextRaster {
        UiTextRaster {
            glyphs: vec![UiGlyphBitmap {
                origin: UiPoint { x: 0.0, y: 0.0 },
                width: 8,
                height: 8,
                alpha: vec![255; 64],
            }],
            has_unrasterized_glyphs: false,
        }
    }

    fn full_corpus(image: UiImageHandle, mesh: UiMeshHandle) -> DisplayList {
        let clip = UiClipId(1);
        let b = bounds();
        DisplayList {
            primitives: vec![
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(1),
                    bounds: b,
                    color: UiColor::panel(),
                },
                DisplayPrimitive::Border {
                    node: UiNodeId::new(2),
                    bounds: b,
                    color: UiColor::text(),
                    width: 1,
                },
                DisplayPrimitive::Text {
                    node: UiNodeId::new(3),
                    bounds: b,
                    text: "Text".to_owned(),
                    color: UiColor::text(),
                    layout: text_layout(),
                    raster: text_raster(),
                },
                DisplayPrimitive::GlyphRun {
                    node: UiNodeId::new(4),
                    bounds: b,
                    text: "Glyph".to_owned(),
                    color: UiColor::text(),
                    layout: text_layout(),
                    raster: text_raster(),
                },
                DisplayPrimitive::FocusIndicator {
                    node: UiNodeId::new(5),
                    bounds: b,
                    color: UiColor::focus(),
                },
                DisplayPrimitive::RoundedRect {
                    node: UiNodeId::new(6),
                    bounds: b,
                    radii: UiCornerRadii::uniform(6.0),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::Path {
                    node: UiNodeId::new(7),
                    commands: vec![
                        UiPathCommand::MoveTo(UiPoint { x: 8.0, y: 8.0 }),
                        UiPathCommand::LineTo(UiPoint { x: 24.0, y: 8.0 }),
                        UiPathCommand::LineTo(UiPoint { x: 24.0, y: 24.0 }),
                        UiPathCommand::Close,
                    ],
                    fill: Some(UiColor::text()),
                    stroke: Some(UiStroke::new(UiColor::focus(), 1.0)),
                },
                DisplayPrimitive::Image {
                    node: UiNodeId::new(8),
                    bounds: b,
                    image,
                    opacity: 1.0,
                },
                DisplayPrimitive::Mesh {
                    node: UiNodeId::new(9),
                    bounds: b,
                    mesh,
                    tint: UiColor::text(),
                },
                DisplayPrimitive::PushClip {
                    id: clip,
                    bounds: b,
                    radii: UiCornerRadii::uniform(6.0),
                },
                DisplayPrimitive::PopClip { id: clip },
                DisplayPrimitive::Shadow {
                    node: UiNodeId::new(10),
                    bounds: b,
                    radii: UiCornerRadii::uniform(6.0),
                    offset: UiPoint { x: 2.0, y: 3.0 },
                    spread: 2.0,
                    color: UiColor::rgba(0.0, 0.0, 0.0, 0.4),
                },
                DisplayPrimitive::Backdrop {
                    node: UiNodeId::new(11),
                    descriptor: UiBackdropDescriptor {
                        bounds: b,
                        sample_bounds: b,
                        tint: UiColor::surface(),
                        opaque_fallback: UiColor::background(),
                    },
                },
            ],
        }
    }
}
