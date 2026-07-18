//! Temporary raster bridge for Meridian-owned UI display lists.
//!
//! This bounded adapter composites immutable panel and glyph data into one
//! uploadable image. It is not a production UI renderer selection; glyph atlas,
//! batching, effects, and cache policy remain behind `RG-UI-001`.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_platform::WindowSize;
use meridian_rhi::{
    BufferUsage, ClearColor, FrameOutcome, GpuBuffer, GpuRenderPipeline, GpuTexture,
    GpuTextureBindGroup, Rhi, RhiError, RhiRenderIdentity, TextureFormat, VertexAttribute,
    VertexFormat, VertexLayout, VertexLayoutError,
};
use meridian_ui::{
    DisplayList, DisplayListError, DisplayPrimitive, UiClipId, UiColor, UiCornerRadii,
    UiFrameSnapshot, UiGlyphBitmap, UiPathCommand, UiPoint, UiRect, UiSize, UiStroke,
    MAX_DISPLAY_PRIMITIVES,
};

const MAX_RASTER_PIXELS: u64 = 16 * 1024 * 1024;
const UI_VERTEX_BYTES: u64 = 20;
const UI_INDEX_COUNT: u32 = 6;
const PATH_CURVE_SEGMENTS: u8 = 16;
const ROUNDED_RECT_SAMPLE_OFFSETS: [(f32, f32); 4] =
    [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

/// Observable contents and limits of one temporary UI overlay submission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiOverlayRenderReport {
    pub solid_primitives: usize,
    pub text_primitives: usize,
    pub rasterized_glyphs: usize,
    pub incomplete_text_primitives: usize,
}

/// Renderer-neutral primitive categories measured by `RG-UI-001`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UiPrimitiveKind {
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

/// Structural coverage of one validated real display-list corpus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiRendererQualificationReport {
    pub primitive_count: usize,
    pub observed_kinds: BTreeSet<UiPrimitiveKind>,
    pub raster_bridge_unsupported: BTreeSet<UiPrimitiveKind>,
}

/// Raster-bridge cache invalidation required before GPU resource reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiRasterBridgeRecoveryAction {
    None,
    RebuildSurfaceCaches,
    RebuildDeviceCaches,
}

/// Typed report proving recovery preserves the last immutable UI snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiRasterBridgeRecovery {
    pub action: UiRasterBridgeRecoveryAction,
    pub preserved_revision: u64,
    pub dropped_cache_count: u32,
}

/// Renderer-owned identity tracker for the bounded raster bridge.
///
/// This keeps the uploaded recovery texture and surface pipeline from being
/// reused after their owning RHI identity changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiRasterBridgeRecoveryState {
    identity: RhiRenderIdentity,
    cached_surface_resources: u32,
    cached_device_resources: u32,
    last_revision: Option<u64>,
}

impl UiRasterBridgeRecoveryState {
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

    /// Records a rebuilt bridge cache set after a successful submission.
    pub fn record_cache_rebuild(&mut self, surface_resources: u32, device_resources: u32) {
        self.cached_surface_resources = surface_resources;
        self.cached_device_resources = device_resources;
    }

    /// Drops stale caches and returns the recovery action for this frame.
    pub fn prepare_frame(
        &mut self,
        identity: RhiRenderIdentity,
        snapshot: &UiFrameSnapshot,
    ) -> UiRasterBridgeRecovery {
        let action = if identity.device_generation != self.identity.device_generation {
            UiRasterBridgeRecoveryAction::RebuildDeviceCaches
        } else if identity.surface_generation != self.identity.surface_generation
            || identity.surface_format != self.identity.surface_format
            || identity.surface_configured != self.identity.surface_configured
        {
            UiRasterBridgeRecoveryAction::RebuildSurfaceCaches
        } else {
            UiRasterBridgeRecoveryAction::None
        };
        let dropped_cache_count = match action {
            UiRasterBridgeRecoveryAction::None => 0,
            UiRasterBridgeRecoveryAction::RebuildSurfaceCaches => {
                let dropped = self.cached_surface_resources;
                self.cached_surface_resources = 0;
                dropped
            }
            UiRasterBridgeRecoveryAction::RebuildDeviceCaches => {
                let dropped = self
                    .cached_surface_resources
                    .saturating_add(self.cached_device_resources);
                self.cached_surface_resources = 0;
                self.cached_device_resources = 0;
                dropped
            }
        };
        self.identity = identity;
        self.last_revision = Some(snapshot.revision);
        UiRasterBridgeRecovery {
            action,
            preserved_revision: snapshot.revision,
            dropped_cache_count,
        }
    }
}

/// Validates and classifies a real Meridian display-list corpus without
/// presenting structural coverage as visual or performance qualification.
///
/// # Errors
///
/// Returns the display-list validation failure before reporting coverage.
pub fn qualify_ui_display_list(
    display_list: &DisplayList,
) -> Result<UiRendererQualificationReport, DisplayListError> {
    display_list.validate()?;
    let observed_kinds = display_list
        .primitives
        .iter()
        .map(primitive_kind)
        .collect::<BTreeSet<_>>();
    let raster_bridge_unsupported = observed_kinds
        .iter()
        .copied()
        .filter(|kind| !raster_bridge_supports(*kind))
        .collect();
    Ok(UiRendererQualificationReport {
        primitive_count: display_list.primitives.len(),
        observed_kinds,
        raster_bridge_unsupported,
    })
}

fn primitive_kind(primitive: &DisplayPrimitive) -> UiPrimitiveKind {
    match primitive {
        DisplayPrimitive::Rect { .. } => UiPrimitiveKind::Rect,
        DisplayPrimitive::Border { .. } => UiPrimitiveKind::Border,
        DisplayPrimitive::Text { .. } => UiPrimitiveKind::Text,
        DisplayPrimitive::GlyphRun { .. } => UiPrimitiveKind::GlyphRun,
        DisplayPrimitive::FocusIndicator { .. } => UiPrimitiveKind::FocusIndicator,
        DisplayPrimitive::RoundedRect { .. } => UiPrimitiveKind::RoundedRect,
        DisplayPrimitive::Path { .. } => UiPrimitiveKind::Path,
        DisplayPrimitive::Image { .. } => UiPrimitiveKind::Image,
        DisplayPrimitive::Mesh { .. } => UiPrimitiveKind::Mesh,
        DisplayPrimitive::PushClip { .. } => UiPrimitiveKind::PushClip,
        DisplayPrimitive::PopClip { .. } => UiPrimitiveKind::PopClip,
        DisplayPrimitive::BeginLayer { .. } => UiPrimitiveKind::BeginLayer,
        DisplayPrimitive::EndLayer { .. } => UiPrimitiveKind::EndLayer,
        DisplayPrimitive::Shadow { .. } => UiPrimitiveKind::Shadow,
        DisplayPrimitive::Backdrop { .. } => UiPrimitiveKind::Backdrop,
    }
}

const fn raster_bridge_supports(kind: UiPrimitiveKind) -> bool {
    matches!(
        kind,
        UiPrimitiveKind::Rect
            | UiPrimitiveKind::Border
            | UiPrimitiveKind::Text
            | UiPrimitiveKind::GlyphRun
            | UiPrimitiveKind::FocusIndicator
            | UiPrimitiveKind::RoundedRect
            | UiPrimitiveKind::Path
            | UiPrimitiveKind::PushClip
            | UiPrimitiveKind::PopClip
    )
}

/// Errors raised while creating an owned temporary UI overlay pass.
#[derive(Debug)]
pub enum UiOverlayRendererError {
    InvalidViewport,
    RasterTooLarge { pixels: u64, maximum: u64 },
    TooManyPrimitives { count: usize, maximum: usize },
    InvalidDisplayList(DisplayListError),
    UnsupportedPrimitive { kind: &'static str },
    BufferSizeOverflow,
    TextureRowOverflow,
    VertexLayout(VertexLayoutError),
    Rhi(RhiError),
}

impl Display for UiOverlayRendererError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidViewport => formatter.write_str("UI viewport must be finite and non-zero"),
            Self::RasterTooLarge { pixels, maximum } => {
                write!(
                    formatter,
                    "UI raster has {pixels} pixels; maximum is {maximum}"
                )
            }
            Self::TooManyPrimitives { count, maximum } => {
                write!(
                    formatter,
                    "UI display list has {count} primitives; maximum is {maximum}"
                )
            }
            Self::InvalidDisplayList(error) => {
                write!(formatter, "invalid UI display list: {error}")
            }
            Self::UnsupportedPrimitive { kind } => {
                write!(
                    formatter,
                    "temporary UI raster bridge does not support {kind}"
                )
            }
            Self::BufferSizeOverflow => formatter.write_str("UI bridge buffer size overflow"),
            Self::TextureRowOverflow => formatter.write_str("UI raster row size overflow"),
            Self::VertexLayout(error) => Display::fmt(error, formatter),
            Self::Rhi(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for UiOverlayRendererError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::VertexLayout(error) => Some(error),
            Self::Rhi(error) => Some(error),
            Self::InvalidViewport
            | Self::RasterTooLarge { .. }
            | Self::TooManyPrimitives { .. }
            | Self::UnsupportedPrimitive { .. }
            | Self::BufferSizeOverflow
            | Self::TextureRowOverflow => None,
            Self::InvalidDisplayList(error) => Some(error),
        }
    }
}

impl From<VertexLayoutError> for UiOverlayRendererError {
    fn from(error: VertexLayoutError) -> Self {
        Self::VertexLayout(error)
    }
}

impl From<RhiError> for UiOverlayRendererError {
    fn from(error: RhiError) -> Self {
        Self::Rhi(error)
    }
}

/// Renders one immutable UI snapshot through an owned RHI texture pass.
pub struct UiOverlayRenderer {
    pipeline: GpuRenderPipeline,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    _texture: GpuTexture,
    texture_binding: GpuTextureBindGroup,
    report: UiOverlayRenderReport,
}

impl UiOverlayRenderer {
    /// Creates fixed resources for one immutable display-list snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when viewport or display-list limits are exceeded, or
    /// RHI setup rejects the bounded temporary pass.
    pub fn new(
        rhi: &mut Rhi,
        display_list: &DisplayList,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<Self, UiOverlayRendererError> {
        let raster = UiOverlayRaster::from_display_list(display_list, viewport, scale_factor)?;
        let vertices = fullscreen_vertex_bytes();
        let indices = fullscreen_index_bytes();
        let vertex_size = u64::try_from(vertices.len())
            .map_err(|_| UiOverlayRendererError::BufferSizeOverflow)?;
        let index_size =
            u64::try_from(indices.len()).map_err(|_| UiOverlayRendererError::BufferSizeOverflow)?;
        let vertex_layout = VertexLayout::new(
            UI_VERTEX_BYTES,
            [
                VertexAttribute::new(VertexFormat::Float32x3, 0, 0),
                VertexAttribute::new(VertexFormat::Float32x2, 12, 1),
            ],
        )?;
        let pipeline = rhi.create_render_pipeline_with_layout(
            "Meridian UI temporary raster bridge",
            include_str!("../../../shaders/ui_raster.wgsl"),
            "vs_main",
            "fs_main",
            Some(&vertex_layout),
        )?;
        let vertex_buffer = rhi.create_buffer(
            "Meridian UI raster bridge vertices",
            vertex_size,
            BufferUsage::Vertex,
        )?;
        let index_buffer = rhi.create_buffer(
            "Meridian UI raster bridge indices",
            index_size,
            BufferUsage::Index,
        )?;
        let texture = rhi.create_texture(
            "Meridian UI raster bridge texture",
            WindowSize::new(raster.width, raster.height),
            1,
            TextureFormat::Rgba8Unorm,
        )?;
        let bytes_per_row = raster
            .width
            .checked_mul(4)
            .ok_or(UiOverlayRendererError::TextureRowOverflow)?;
        rhi.write_buffer(&vertex_buffer, 0, &vertices)?;
        rhi.write_buffer(&index_buffer, 0, &indices)?;
        rhi.write_texture(&texture, 0, &raster.pixels, bytes_per_row)?;
        let texture_binding = rhi.create_texture_bind_group(
            "Meridian UI raster bridge texture binding",
            &pipeline,
            &texture,
        )?;
        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            _texture: texture,
            texture_binding,
            report: raster.report,
        })
    }

    /// Draws the immutable UI snapshot and presents the surface.
    ///
    /// # Errors
    ///
    /// Returns a typed RHI draw, surface, or device error.
    pub fn render_frame(&self, rhi: &mut Rhi, clear: ClearColor) -> Result<FrameOutcome, RhiError> {
        rhi.render_indexed_mesh_with_texture_and_present(
            &self.pipeline,
            &self.vertex_buffer,
            &self.index_buffer,
            UI_INDEX_COUNT,
            &self.texture_binding,
            clear,
        )
    }

    /// Submits the raster bridge to an unreadable offscreen target when native
    /// presentation is unavailable.
    ///
    /// This is structural evidence only; it cannot establish presentation or
    /// visual-quality evidence.
    ///
    /// # Errors
    ///
    /// Returns a typed RHI draw or device error.
    pub fn submit_structural_validation(
        &self,
        rhi: &mut Rhi,
        clear: ClearColor,
    ) -> Result<(), RhiError> {
        rhi.submit_textured_indexed_mesh_structural_validation(
            &self.pipeline,
            &self.vertex_buffer,
            &self.index_buffer,
            UI_INDEX_COUNT,
            &self.texture_binding,
            clear,
        )
    }

    #[must_use]
    pub const fn report(&self) -> UiOverlayRenderReport {
        self.report
    }
}

struct UiOverlayRaster {
    width: u32,
    height: u32,
    scale_factor: f32,
    pixels: Vec<u8>,
    clips: Vec<(UiClipId, UiRect, UiCornerRadii)>,
    report: UiOverlayRenderReport,
}

fn unsupported<T>(kind: &'static str) -> Result<T, UiOverlayRendererError> {
    Err(UiOverlayRendererError::UnsupportedPrimitive { kind })
}

impl UiOverlayRaster {
    fn from_display_list(
        display_list: &DisplayList,
        viewport: UiSize,
        scale_factor: f32,
    ) -> Result<Self, UiOverlayRendererError> {
        if !scale_factor.is_finite() || !(0.5..=4.0).contains(&scale_factor) {
            return Err(UiOverlayRendererError::InvalidViewport);
        }
        let width = finite_dimension_to_u32(viewport.width * scale_factor)?;
        let height = finite_dimension_to_u32(viewport.height * scale_factor)?;
        let pixel_count = u64::from(width) * u64::from(height);
        if pixel_count > MAX_RASTER_PIXELS {
            return Err(UiOverlayRendererError::RasterTooLarge {
                pixels: pixel_count,
                maximum: MAX_RASTER_PIXELS,
            });
        }
        if display_list.primitives.len() > MAX_DISPLAY_PRIMITIVES {
            return Err(UiOverlayRendererError::TooManyPrimitives {
                count: display_list.primitives.len(),
                maximum: MAX_DISPLAY_PRIMITIVES,
            });
        }
        display_list
            .validate()
            .map_err(UiOverlayRendererError::InvalidDisplayList)?;
        let byte_count = usize::try_from(pixel_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(UiOverlayRendererError::BufferSizeOverflow)?;
        let mut raster = Self {
            width,
            height,
            scale_factor,
            pixels: vec![0; byte_count],
            clips: Vec::new(),
            report: UiOverlayRenderReport {
                solid_primitives: 0,
                text_primitives: 0,
                rasterized_glyphs: 0,
                incomplete_text_primitives: 0,
            },
        };
        raster.clear(ClearColor::default());
        for primitive in &display_list.primitives {
            raster.draw_primitive(primitive)?;
        }
        Ok(raster)
    }

    fn draw_primitive(
        &mut self,
        primitive: &DisplayPrimitive,
    ) -> Result<(), UiOverlayRendererError> {
        match primitive {
            DisplayPrimitive::Rect { bounds, color, .. } => {
                self.fill_rect(*bounds, *color);
                self.report.solid_primitives += 1;
            }
            DisplayPrimitive::Border {
                bounds,
                color,
                width,
                ..
            } => {
                self.stroke_rect(*bounds, *color, u32::from(*width).max(1));
                self.report.solid_primitives += 1;
            }
            DisplayPrimitive::FocusIndicator { bounds, color, .. } => {
                self.stroke_rect(*bounds, *color, 3);
                self.report.solid_primitives += 1;
            }
            DisplayPrimitive::Text {
                bounds,
                color,
                raster: text,
                ..
            }
            | DisplayPrimitive::GlyphRun {
                bounds,
                color,
                raster: text,
                ..
            } => self.draw_text(*bounds, *color, text),
            DisplayPrimitive::RoundedRect {
                bounds,
                radii,
                color,
                ..
            } => {
                self.fill_rounded_rect(*bounds, *radii, *color);
                self.report.solid_primitives += 1;
            }
            DisplayPrimitive::Path {
                commands,
                fill,
                stroke,
                ..
            } => {
                self.draw_path(commands, *fill, *stroke);
                self.report.solid_primitives += 1;
            }
            DisplayPrimitive::Image { .. } => return unsupported("images"),
            DisplayPrimitive::Mesh { .. } => return unsupported("meshes"),
            DisplayPrimitive::PushClip { id, bounds, radii } => {
                self.clips.push((*id, *bounds, *radii));
            }
            DisplayPrimitive::PopClip { id } => {
                let Some((active, _, _)) = self.clips.pop() else {
                    return unsupported("unbalanced clip pop");
                };
                if active != *id {
                    return unsupported("mismatched clip pop");
                }
            }
            DisplayPrimitive::BeginLayer { .. } | DisplayPrimitive::EndLayer { .. } => {
                return unsupported("layers");
            }
            DisplayPrimitive::Shadow { .. } => return unsupported("shadows"),
            DisplayPrimitive::Backdrop { .. } => return unsupported("backdrop effects"),
        }
        Ok(())
    }

    fn draw_text(&mut self, bounds: UiRect, color: UiColor, text: &meridian_ui::UiTextRaster) {
        self.report.text_primitives += 1;
        if text.has_unrasterized_glyphs {
            self.report.incomplete_text_primitives += 1;
        }
        for glyph in &text.glyphs {
            if self.draw_glyph(bounds, glyph, color) {
                self.report.rasterized_glyphs += 1;
            } else {
                self.report.incomplete_text_primitives += 1;
            }
        }
    }

    fn clear(&mut self, color: ClearColor) {
        let color = [
            unit_f64_to_u8(color.red),
            unit_f64_to_u8(color.green),
            unit_f64_to_u8(color.blue),
            unit_f64_to_u8(color.alpha),
        ];
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    fn fill_rect(&mut self, bounds: UiRect, color: UiColor) {
        let (left, top, right, bottom) = self.pixel_bounds(bounds);
        for y in top..bottom {
            for x in left..right {
                self.blend(x, y, color, 1.0);
            }
        }
    }

    #[allow(clippy::cast_precision_loss)] // Raster coordinates are bounded to 65,535 pixels.
    fn fill_rounded_rect(&mut self, bounds: UiRect, radii: UiCornerRadii, color: UiColor) {
        let (left, top, right, bottom) = self.pixel_bounds(bounds);
        for y in top..bottom {
            for x in left..right {
                let coverage = ROUNDED_RECT_SAMPLE_OFFSETS
                    .iter()
                    .filter(|(sample_x, sample_y)| {
                        rounded_rect_contains(
                            bounds,
                            radii,
                            (x as f32 + sample_x) / self.scale_factor,
                            (y as f32 + sample_y) / self.scale_factor,
                        )
                    })
                    .count() as f32
                    / ROUNDED_RECT_SAMPLE_OFFSETS.len() as f32;
                if coverage > 0.0 {
                    self.blend(x, y, color, coverage);
                }
            }
        }
    }

    fn stroke_rect(&mut self, bounds: UiRect, color: UiColor, thickness: u32) {
        let (left, top, right, bottom) = self.pixel_bounds(bounds);
        let thickness = thickness
            .min(right.saturating_sub(left))
            .min(bottom.saturating_sub(top));
        for y in top..bottom {
            for x in left..right {
                let on_border = x.saturating_sub(left) < thickness
                    || right.saturating_sub(x + 1) < thickness
                    || y.saturating_sub(top) < thickness
                    || bottom.saturating_sub(y + 1) < thickness;
                if on_border {
                    self.blend(x, y, color, 1.0);
                }
            }
        }
    }

    fn draw_path(
        &mut self,
        commands: &[UiPathCommand],
        fill: Option<UiColor>,
        stroke: Option<UiStroke>,
    ) {
        let mut points = Vec::new();
        for command in commands {
            match *command {
                UiPathCommand::MoveTo(point) => {
                    if let Some(fill) = fill {
                        self.fill_polygon(&points, fill);
                    }
                    if let Some(stroke) = stroke {
                        self.stroke_polyline(&points, stroke, false);
                    }
                    points.clear();
                    points.push(point);
                }
                UiPathCommand::LineTo(point) => points.push(point),
                UiPathCommand::QuadraticTo { control, end } => {
                    if let Some(start) = points.last().copied() {
                        for segment in 1..=PATH_CURVE_SEGMENTS {
                            let t = f32::from(segment) / f32::from(PATH_CURVE_SEGMENTS);
                            let inverse = 1.0 - t;
                            points.push(UiPoint {
                                x: inverse * inverse * start.x
                                    + 2.0 * inverse * t * control.x
                                    + t * t * end.x,
                                y: inverse * inverse * start.y
                                    + 2.0 * inverse * t * control.y
                                    + t * t * end.y,
                            });
                        }
                    }
                }
                UiPathCommand::CubicTo {
                    control_a,
                    control_b,
                    end,
                } => {
                    if let Some(start) = points.last().copied() {
                        for segment in 1..=PATH_CURVE_SEGMENTS {
                            let t = f32::from(segment) / f32::from(PATH_CURVE_SEGMENTS);
                            let inverse = 1.0 - t;
                            points.push(UiPoint {
                                x: inverse * inverse * inverse * start.x
                                    + 3.0 * inverse * inverse * t * control_a.x
                                    + 3.0 * inverse * t * t * control_b.x
                                    + t * t * t * end.x,
                                y: inverse * inverse * inverse * start.y
                                    + 3.0 * inverse * inverse * t * control_a.y
                                    + 3.0 * inverse * t * t * control_b.y
                                    + t * t * t * end.y,
                            });
                        }
                    }
                }
                UiPathCommand::Close => {
                    if let Some(fill) = fill {
                        self.fill_polygon(&points, fill);
                    }
                    if let Some(stroke) = stroke {
                        self.stroke_polyline(&points, stroke, true);
                    }
                    points.clear();
                }
            }
        }
        if let Some(fill) = fill {
            self.fill_polygon(&points, fill);
        }
        if let Some(stroke) = stroke {
            self.stroke_polyline(&points, stroke, false);
        }
    }

    fn stroke_polyline(&mut self, points: &[UiPoint], stroke: UiStroke, closed: bool) {
        if points.len() < 2 {
            return;
        }
        for segment in points.windows(2) {
            self.stroke_line(segment[0], segment[1], stroke);
        }
        if closed {
            self.stroke_line(points[points.len() - 1], points[0], stroke);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    fn stroke_line(&mut self, start: UiPoint, end: UiPoint, stroke: UiStroke) {
        let start_x = start.x * self.scale_factor;
        let start_y = start.y * self.scale_factor;
        let end_x = end.x * self.scale_factor;
        let end_y = end.y * self.scale_factor;
        let delta_x = end_x - start_x;
        let delta_y = end_y - start_y;
        let steps = delta_x.abs().max(delta_y.abs()).ceil().max(1.0) as u32;
        let thickness = (stroke.width * self.scale_factor).ceil().clamp(1.0, 16.0) as i32;
        for step in 0..=steps {
            let progress = step as f32 / steps as f32;
            let x = delta_x.mul_add(progress, start_x).round() as i32;
            let y = delta_y.mul_add(progress, start_y).round() as i32;
            for offset_y in 0..thickness {
                for offset_x in 0..thickness {
                    let (Ok(x), Ok(y)) = (u32::try_from(x + offset_x), u32::try_from(y + offset_y))
                    else {
                        continue;
                    };
                    if x < self.width && y < self.height {
                        self.blend(x, y, stroke.color, 1.0);
                    }
                }
            }
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    fn fill_polygon(&mut self, points: &[UiPoint], color: UiColor) {
        if points.len() < 3 {
            return;
        }
        let minimum_x = points
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        let maximum_x = points
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let minimum_y = points
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        let maximum_y = points
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let left = floor_to_u32(minimum_x * self.scale_factor, self.width);
        let right = ceil_to_u32(maximum_x * self.scale_factor, self.width);
        let top = floor_to_u32(minimum_y * self.scale_factor, self.height);
        let bottom = ceil_to_u32(maximum_y * self.scale_factor, self.height);
        for y in top..bottom {
            for x in left..right {
                let logical = UiPoint {
                    x: (x as f32 + 0.5) / self.scale_factor,
                    y: (y as f32 + 0.5) / self.scale_factor,
                };
                if point_in_polygon(logical, points) {
                    self.blend(x, y, color, 1.0);
                }
            }
        }
    }

    fn draw_glyph(&mut self, bounds: UiRect, glyph: &UiGlyphBitmap, color: UiColor) -> bool {
        let expected = usize::try_from(glyph.width).ok().and_then(|width| {
            usize::try_from(glyph.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        });
        if expected != Some(glyph.alpha.len()) {
            return false;
        }
        let origin_x = floor_to_i32(bounds.origin.x * self.scale_factor + glyph.origin.x);
        let origin_y = floor_to_i32(bounds.origin.y * self.scale_factor + glyph.origin.y);
        let (clip_left, clip_top, clip_right, clip_bottom) = self.pixel_bounds(bounds);
        for glyph_y in 0..glyph.height {
            for glyph_x in 0..glyph.width {
                let Some(x) = origin_x.checked_add(i32::try_from(glyph_x).unwrap_or(i32::MAX))
                else {
                    continue;
                };
                let Some(y) = origin_y.checked_add(i32::try_from(glyph_y).unwrap_or(i32::MAX))
                else {
                    continue;
                };
                let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
                    continue;
                };
                if x < clip_left || x >= clip_right || y < clip_top || y >= clip_bottom {
                    continue;
                }
                let offset = usize::try_from(glyph_y)
                    .ok()
                    .and_then(|row| {
                        usize::try_from(glyph.width)
                            .ok()
                            .and_then(|width| row.checked_mul(width))
                    })
                    .and_then(|start| {
                        usize::try_from(glyph_x)
                            .ok()
                            .and_then(|column| start.checked_add(column))
                    });
                if let Some(offset) = offset {
                    self.blend(x, y, color, f32::from(glyph.alpha[offset]) / 255.0);
                }
            }
        }
        true
    }

    fn pixel_bounds(&self, bounds: UiRect) -> (u32, u32, u32, u32) {
        let left = floor_to_u32(bounds.origin.x * self.scale_factor, self.width);
        let top = floor_to_u32(bounds.origin.y * self.scale_factor, self.height);
        let right = ceil_to_u32(
            (bounds.origin.x + bounds.size.width) * self.scale_factor,
            self.width,
        )
        .max(left);
        let bottom = ceil_to_u32(
            (bounds.origin.y + bounds.size.height) * self.scale_factor,
            self.height,
        )
        .max(top);
        (left, top, right, bottom)
    }

    fn blend(&mut self, x: u32, y: u32, color: UiColor, coverage: f32) {
        #[allow(clippy::cast_precision_loss)]
        let logical = UiPoint {
            x: (x as f32 + 0.5) / self.scale_factor,
            y: (y as f32 + 0.5) / self.scale_factor,
        };
        if self
            .clips
            .iter()
            .any(|(_, bounds, radii)| !rounded_rect_contains(*bounds, *radii, logical.x, logical.y))
        {
            return;
        }
        let offset = usize::try_from(y)
            .ok()
            .and_then(|row| {
                usize::try_from(self.width)
                    .ok()
                    .and_then(|width| row.checked_mul(width))
            })
            .and_then(|start| {
                usize::try_from(x)
                    .ok()
                    .and_then(|column| start.checked_add(column))
            })
            .and_then(|pixel| pixel.checked_mul(4));
        let Some(offset) = offset else {
            return;
        };
        let alpha = (color.alpha * coverage).clamp(0.0, 1.0);
        let destination = [
            f32::from(self.pixels[offset]) / 255.0,
            f32::from(self.pixels[offset + 1]) / 255.0,
            f32::from(self.pixels[offset + 2]) / 255.0,
        ];
        self.pixels[offset] = unit_f32_to_u8(color.red * alpha + destination[0] * (1.0 - alpha));
        self.pixels[offset + 1] =
            unit_f32_to_u8(color.green * alpha + destination[1] * (1.0 - alpha));
        self.pixels[offset + 2] =
            unit_f32_to_u8(color.blue * alpha + destination[2] * (1.0 - alpha));
        self.pixels[offset + 3] = 255;
    }
}

fn point_in_polygon(point: UiPoint, polygon: &[UiPoint]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn rounded_rect_contains(bounds: UiRect, radii: UiCornerRadii, x: f32, y: f32) -> bool {
    let left = bounds.origin.x;
    let top = bounds.origin.y;
    let right = left + bounds.size.width;
    let bottom = top + bounds.size.height;
    if x < left || x > right || y < top || y > bottom {
        return false;
    }
    let maximum = bounds.size.width.min(bounds.size.height) / 2.0;
    let corners = [
        (radii.top_left.min(maximum), left, top, true, true),
        (radii.top_right.min(maximum), right, top, false, true),
        (radii.bottom_right.min(maximum), right, bottom, false, false),
        (radii.bottom_left.min(maximum), left, bottom, true, false),
    ];
    for (radius, edge_x, edge_y, left_corner, top_corner) in corners {
        if radius <= 0.0 {
            continue;
        }
        let in_x = if left_corner {
            x < edge_x + radius
        } else {
            x > edge_x - radius
        };
        let in_y = if top_corner {
            y < edge_y + radius
        } else {
            y > edge_y - radius
        };
        if in_x && in_y {
            let center_x = if left_corner {
                edge_x + radius
            } else {
                edge_x - radius
            };
            let center_y = if top_corner {
                edge_y + radius
            } else {
                edge_y - radius
            };
            let normalized_x = (x - center_x) / radius;
            let normalized_y = (y - center_y) / radius;
            return normalized_x.mul_add(normalized_x, normalized_y * normalized_y) <= 1.0;
        }
    }
    true
}

fn fullscreen_vertex_bytes() -> Vec<u8> {
    let vertices = [
        ([-1.0_f32, -1.0, 0.0], [0.0_f32, 1.0]),
        ([1.0_f32, -1.0, 0.0], [1.0_f32, 1.0]),
        ([1.0_f32, 1.0, 0.0], [1.0_f32, 0.0]),
        ([-1.0_f32, 1.0, 0.0], [0.0_f32, 0.0]),
    ];
    let mut bytes = Vec::with_capacity(80);
    for (position, uv) in vertices {
        for value in position.into_iter().chain(uv) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn fullscreen_index_bytes() -> Vec<u8> {
    let indices = [0_u32, 1, 2, 0, 2, 3];
    let mut bytes = Vec::with_capacity(24);
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}

fn finite_dimension_to_u32(value: f32) -> Result<u32, UiOverlayRendererError> {
    if !value.is_finite() || value <= 0.0 || value > 65_535.0 {
        return Err(UiOverlayRendererError::InvalidViewport);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let dimension = value.ceil() as u32;
    Ok(dimension.max(1))
}

fn floor_to_u32(value: f32, maximum: u32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = value.floor() as u32;
    value.min(maximum)
}

fn ceil_to_u32(value: f32, maximum: u32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = value.ceil() as u32;
    value.min(maximum)
}

fn floor_to_i32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)]
    let value = value.floor() as i32;
    value
}

fn unit_f64_to_u8(value: f64) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    value
}

fn unit_f32_to_u8(value: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    value
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meridian_rhi::SurfaceFormat;
    use meridian_ui::{
        recovery_panel_document, DisplayList, DisplayPrimitive, UiBackdropDescriptor, UiClipId,
        UiCornerRadii, UiFontRole, UiFrameInput, UiGlyphBitmap, UiImageHandle, UiLayerId,
        UiMeshHandle, UiNodeId, UiPathCommand, UiPoint, UiRect, UiRuntime, UiTextLayout,
        UiTextRaster,
    };

    use super::*;

    fn corpus_text(node: UiNodeId, bounds: UiRect, glyph_run: bool) -> DisplayPrimitive {
        let text = "M".to_owned();
        let color = UiColor::text();
        let layout = UiTextLayout {
            line_count: 1,
            glyph_count: 0,
            width: 8.0,
            height: 12.0,
            used_fallback_metrics: false,
            used_fallback_font: false,
            font_role: UiFontRole::Interface,
        };
        let raster = UiTextRaster::default();
        if glyph_run {
            DisplayPrimitive::GlyphRun {
                node,
                bounds,
                text,
                color,
                layout,
                raster,
            }
        } else {
            DisplayPrimitive::Text {
                node,
                bounds,
                text,
                color,
                layout,
                raster,
            }
        }
    }

    fn qualification_corpus() -> DisplayList {
        let node = UiNodeId::new(1);
        let bounds = UiRect::new(UiPoint { x: 1.0, y: 1.0 }, UiSize::new(32.0, 24.0));
        let clip = UiClipId(1);
        let layer = UiLayerId(2);
        DisplayList {
            primitives: vec![
                DisplayPrimitive::PushClip {
                    id: clip,
                    bounds,
                    radii: UiCornerRadii::uniform(6.0),
                },
                DisplayPrimitive::BeginLayer {
                    id: layer,
                    opacity: 0.9,
                },
                DisplayPrimitive::Rect {
                    node,
                    bounds,
                    color: UiColor::background(),
                },
                DisplayPrimitive::Border {
                    node,
                    bounds,
                    color: UiColor::border(),
                    width: 1,
                },
                corpus_text(node, bounds, false),
                corpus_text(node, bounds, true),
                DisplayPrimitive::FocusIndicator {
                    node,
                    bounds,
                    color: UiColor::focus(),
                },
                DisplayPrimitive::RoundedRect {
                    node,
                    bounds,
                    radii: UiCornerRadii::uniform(6.0),
                    color: UiColor::surface(),
                },
                DisplayPrimitive::Path {
                    node,
                    commands: vec![
                        UiPathCommand::MoveTo(bounds.origin),
                        UiPathCommand::LineTo(UiPoint { x: 8.0, y: 8.0 }),
                    ],
                    fill: None,
                    stroke: Some(UiStroke::new(UiColor::text(), 1.0)),
                },
                DisplayPrimitive::Image {
                    node,
                    bounds,
                    image: UiImageHandle(1),
                    opacity: 1.0,
                },
                DisplayPrimitive::Mesh {
                    node,
                    bounds,
                    mesh: UiMeshHandle(1),
                    tint: UiColor::text(),
                },
                DisplayPrimitive::Shadow {
                    node,
                    bounds,
                    radii: UiCornerRadii::uniform(6.0),
                    offset: UiPoint { x: 0.0, y: 4.0 },
                    spread: 2.0,
                    color: UiColor::rgba(0.0, 0.0, 0.0, 0.5),
                },
                DisplayPrimitive::Backdrop {
                    node,
                    descriptor: UiBackdropDescriptor {
                        bounds,
                        sample_bounds: bounds,
                        tint: UiColor::surface(),
                        opaque_fallback: UiColor::background(),
                    },
                },
                DisplayPrimitive::EndLayer { id: layer },
                DisplayPrimitive::PopClip { id: clip },
            ],
        }
    }

    #[test]
    fn recovery_display_list_produces_rasterized_text_and_panel_geometry() {
        let document = recovery_panel_document().expect("fixture valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(960.0, 540.0)));
        let raster = UiOverlayRaster::from_display_list(
            &output.display_list,
            UiSize::new(960.0, 540.0),
            1.0,
        )
        .expect("recovery display list is bridgeable");
        assert_eq!(raster.report.solid_primitives, 3);
        assert_eq!(raster.report.text_primitives, 2);
        assert!(raster.report.rasterized_glyphs > 0);
        assert_eq!(raster.report.incomplete_text_primitives, 0);
        assert!(raster.pixels.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn qualification_corpus_covers_every_display_primitive_without_overclaiming_bridge_support() {
        let corpus = qualification_corpus();
        let report = qualify_ui_display_list(&corpus).expect("full corpus validates");
        assert_eq!(report.primitive_count, 15);
        assert_eq!(report.observed_kinds.len(), 15);
        assert_eq!(
            report.raster_bridge_unsupported,
            BTreeSet::from([
                UiPrimitiveKind::Image,
                UiPrimitiveKind::Mesh,
                UiPrimitiveKind::BeginLayer,
                UiPrimitiveKind::EndLayer,
                UiPrimitiveKind::Shadow,
                UiPrimitiveKind::Backdrop,
            ])
        );
    }

    #[test]
    fn text_raster_is_clipped_to_its_retained_text_bounds() {
        let text_bounds = UiRect::new(UiPoint { x: 3.0, y: 3.0 }, UiSize::new(2.0, 2.0));
        let display_list = DisplayList {
            primitives: vec![DisplayPrimitive::Text {
                node: UiNodeId::new(1),
                bounds: text_bounds,
                text: "clipped".to_owned(),
                color: UiColor::rgba(1.0, 1.0, 1.0, 1.0),
                layout: UiTextLayout {
                    line_count: 1,
                    glyph_count: 1,
                    width: 6.0,
                    height: 6.0,
                    used_fallback_metrics: false,
                    used_fallback_font: false,
                    font_role: UiFontRole::Interface,
                },
                raster: UiTextRaster {
                    glyphs: vec![UiGlyphBitmap {
                        origin: UiPoint { x: 0.0, y: 0.0 },
                        width: 6,
                        height: 6,
                        alpha: vec![255; 36],
                    }],
                    has_unrasterized_glyphs: false,
                },
            }],
        };

        let raster =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(10.0, 10.0), 1.0)
                .expect("clipped text display list is bridgeable");
        let pixel = |x: u32, y: u32| {
            let offset = usize::try_from(y * raster.width + x).expect("pixel offset") * 4;
            raster.pixels[offset]
        };

        let clear = pixel(0, 0);
        assert_ne!(pixel(3, 3), clear);
        assert_ne!(pixel(4, 4), clear);
        assert_eq!(pixel(2, 3), clear);
        assert_eq!(pixel(5, 4), clear);
        assert_eq!(pixel(3, 5), clear);
    }

    fn red_channel(raster: &UiOverlayRaster, x: u32, y: u32) -> u8 {
        let pixel = y
            .checked_mul(raster.width)
            .and_then(|row| row.checked_add(x))
            .and_then(|pixel| usize::try_from(pixel).ok())
            .and_then(|pixel| pixel.checked_mul(4))
            .expect("bounded raster pixel offset");
        raster.pixels[pixel]
    }

    fn identity(device_generation: u64, surface_generation: u64) -> RhiRenderIdentity {
        RhiRenderIdentity {
            device_generation,
            surface_generation,
            surface_format: SurfaceFormat {
                name: "Bgra8UnormSrgb".to_owned(),
                srgb: true,
            },
            surface_size: WindowSize::new(64, 64),
            surface_configured: true,
        }
    }

    fn recovery_snapshot() -> Arc<UiFrameSnapshot> {
        let document = recovery_panel_document().expect("recovery document");
        let mut runtime = UiRuntime::new(document);
        runtime.reconcile(UiFrameInput::new(UiSize::new(64.0, 64.0)))
    }

    #[test]
    fn direct_renderer_state_drops_surface_caches_without_losing_snapshot_revision() {
        let snapshot = recovery_snapshot();
        let mut state = UiRasterBridgeRecoveryState::new(identity(1, 1));
        state.record_cache_rebuild(3, 7);

        let report = state.prepare_frame(identity(1, 2), &snapshot);

        assert_eq!(
            report.action,
            UiRasterBridgeRecoveryAction::RebuildSurfaceCaches
        );
        assert_eq!(report.preserved_revision, snapshot.revision);
        assert_eq!(report.dropped_cache_count, 3);
        assert_eq!(state.last_revision(), Some(snapshot.revision));

        let follow_up = state.prepare_frame(identity(1, 2), &snapshot);
        assert_eq!(follow_up.action, UiRasterBridgeRecoveryAction::None);
    }

    #[test]
    fn direct_renderer_state_drops_all_caches_after_device_generation_change() {
        let snapshot = recovery_snapshot();
        let mut state = UiRasterBridgeRecoveryState::new(identity(10, 4));
        state.record_cache_rebuild(3, 7);

        let report = state.prepare_frame(identity(11, 1), &snapshot);

        assert_eq!(
            report.action,
            UiRasterBridgeRecoveryAction::RebuildDeviceCaches
        );
        assert_eq!(report.preserved_revision, snapshot.revision);
        assert_eq!(report.dropped_cache_count, 10);
        assert_eq!(state.last_revision(), Some(snapshot.revision));
    }

    #[test]
    fn rounded_rect_raster_preserves_transparent_corners_and_filled_center() {
        let display_list = DisplayList {
            primitives: vec![DisplayPrimitive::RoundedRect {
                node: UiNodeId::new(1),
                bounds: UiRect::new(UiPoint { x: 2.0, y: 2.0 }, UiSize::new(8.0, 8.0)),
                radii: UiCornerRadii::uniform(4.0),
                color: UiColor::rgba(1.0, 0.0, 0.0, 1.0),
            }],
        };
        let raster =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(12.0, 12.0), 1.0)
                .expect("rounded rectangle is bridgeable");

        let clear = red_channel(&raster, 0, 0);
        assert_eq!(red_channel(&raster, 2, 2), clear);
        assert!(red_channel(&raster, 6, 6) > clear);
    }

    #[test]
    fn nested_clip_raster_rejects_pixels_outside_every_active_clip() {
        let outer = UiClipId(1);
        let inner = UiClipId(2);
        let display_list = DisplayList {
            primitives: vec![
                DisplayPrimitive::PushClip {
                    id: outer,
                    bounds: UiRect::new(UiPoint { x: 1.0, y: 1.0 }, UiSize::new(8.0, 8.0)),
                    radii: UiCornerRadii::uniform(0.0),
                },
                DisplayPrimitive::PushClip {
                    id: inner,
                    bounds: UiRect::new(UiPoint { x: 3.0, y: 3.0 }, UiSize::new(4.0, 4.0)),
                    radii: UiCornerRadii::uniform(0.0),
                },
                DisplayPrimitive::Rect {
                    node: UiNodeId::new(1),
                    bounds: UiRect::new(UiPoint { x: 0.0, y: 0.0 }, UiSize::new(10.0, 10.0)),
                    color: UiColor::rgba(1.0, 0.0, 0.0, 1.0),
                },
                DisplayPrimitive::PopClip { id: inner },
                DisplayPrimitive::PopClip { id: outer },
            ],
        };
        let raster =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(10.0, 10.0), 1.0)
                .expect("nested clips are bridgeable");

        let clear = red_channel(&raster, 0, 0);
        assert_eq!(red_channel(&raster, 2, 4), clear);
        assert!(red_channel(&raster, 4, 4) > clear);
        assert_eq!(red_channel(&raster, 8, 4), clear);
    }

    #[test]
    fn linear_paths_fill_closed_geometry_and_stroke_open_segments() {
        let display_list = DisplayList {
            primitives: vec![
                DisplayPrimitive::Path {
                    node: UiNodeId::new(1),
                    commands: vec![
                        UiPathCommand::MoveTo(UiPoint { x: 2.0, y: 2.0 }),
                        UiPathCommand::LineTo(UiPoint { x: 8.0, y: 2.0 }),
                        UiPathCommand::LineTo(UiPoint { x: 5.0, y: 8.0 }),
                        UiPathCommand::Close,
                    ],
                    fill: Some(UiColor::rgba(1.0, 0.0, 0.0, 1.0)),
                    stroke: None,
                },
                DisplayPrimitive::Path {
                    node: UiNodeId::new(2),
                    commands: vec![
                        UiPathCommand::MoveTo(UiPoint { x: 1.0, y: 10.0 }),
                        UiPathCommand::LineTo(UiPoint { x: 10.0, y: 10.0 }),
                    ],
                    fill: None,
                    stroke: Some(UiStroke::new(UiColor::rgba(1.0, 0.0, 0.0, 1.0), 1.0)),
                },
            ],
        };
        let raster =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(12.0, 12.0), 1.0)
                .expect("linear paths are bridgeable");

        let clear = red_channel(&raster, 0, 0);
        assert!(red_channel(&raster, 5, 4) > clear);
        assert!(red_channel(&raster, 6, 10) > clear);
        assert_eq!(red_channel(&raster, 1, 6), clear);
    }

    #[test]
    fn filled_open_subpath_is_rejected_instead_of_disappearing() {
        let display_list = DisplayList {
            primitives: vec![DisplayPrimitive::Path {
                node: UiNodeId::new(1),
                commands: vec![
                    UiPathCommand::MoveTo(UiPoint { x: 2.0, y: 2.0 }),
                    UiPathCommand::LineTo(UiPoint { x: 8.0, y: 2.0 }),
                    UiPathCommand::LineTo(UiPoint { x: 5.0, y: 8.0 }),
                ],
                fill: Some(UiColor::rgba(1.0, 0.0, 0.0, 1.0)),
                stroke: None,
            }],
        };
        let result =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(10.0, 10.0), 1.0);
        assert!(matches!(
            result,
            Err(UiOverlayRendererError::InvalidDisplayList(
                DisplayListError::InvalidGeometry { index: 0 }
            ))
        ));
    }

    #[test]
    fn curved_path_is_flattened_with_bounded_structural_coverage() {
        let display_list = DisplayList {
            primitives: vec![DisplayPrimitive::Path {
                node: UiNodeId::new(1),
                commands: vec![
                    UiPathCommand::MoveTo(UiPoint { x: 1.0, y: 1.0 }),
                    UiPathCommand::QuadraticTo {
                        control: UiPoint { x: 4.0, y: 8.0 },
                        end: UiPoint { x: 8.0, y: 1.0 },
                    },
                ],
                fill: None,
                stroke: Some(UiStroke::new(UiColor::text(), 1.0)),
            }],
        };

        let raster =
            UiOverlayRaster::from_display_list(&display_list, UiSize::new(10.0, 10.0), 1.0)
                .expect("bounded curved path is bridgeable");
        assert_eq!(raster.report.solid_primitives, 1);
    }
}
