//! Temporary raster bridge for Meridian-owned UI display lists.
//!
//! This bounded adapter composites immutable panel and glyph data into one
//! uploadable image. It is not a production UI renderer selection; glyph atlas,
//! batching, effects, and cache policy remain behind `RG-UI-001`.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_platform::WindowSize;
use meridian_rhi::{
    BufferUsage, ClearColor, FrameOutcome, GpuBuffer, GpuRenderPipeline, GpuTexture,
    GpuTextureBindGroup, Rhi, RhiError, TextureFormat, VertexAttribute, VertexFormat, VertexLayout,
    VertexLayoutError,
};
use meridian_ui::{
    DisplayList, DisplayListError, DisplayPrimitive, UiColor, UiGlyphBitmap, UiRect, UiSize,
    MAX_DISPLAY_PRIMITIVES,
};

const MAX_RASTER_PIXELS: u64 = 16 * 1024 * 1024;
const UI_VERTEX_BYTES: u64 = 20;
const UI_INDEX_COUNT: u32 = 6;

/// Observable contents and limits of one temporary UI overlay submission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiOverlayRenderReport {
    pub solid_primitives: usize,
    pub text_primitives: usize,
    pub rasterized_glyphs: usize,
    pub incomplete_text_primitives: usize,
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
            DisplayPrimitive::FocusRing { bounds, color, .. } => {
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
            DisplayPrimitive::RoundedRect { .. } => return unsupported("rounded rectangles"),
            DisplayPrimitive::Path { .. } => return unsupported("paths"),
            DisplayPrimitive::Image { .. } => return unsupported("images"),
            DisplayPrimitive::Mesh { .. } => return unsupported("meshes"),
            DisplayPrimitive::PushClip { .. } | DisplayPrimitive::PopClip { .. } => {
                return unsupported("clips");
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
    use meridian_ui::{
        recovery_panel_document, DisplayList, DisplayPrimitive, UiFrameInput, UiGlyphBitmap,
        UiNodeId, UiPoint, UiRect, UiRuntime, UiTextLayout, UiTextRaster,
    };

    use super::*;

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
}
