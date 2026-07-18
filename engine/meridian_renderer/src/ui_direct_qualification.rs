//! Deterministic direct-UI qualification corpus and raw RGBA comparison.
//!
//! This module deliberately owns no backend handles.  Native qualification
//! runners prepare the corpus through [`UiDirectGpuRenderer`] and compare its
//! bounded offscreen readback with a profile-specific golden artifact.  An
//! offscreen comparison is renderer evidence, not a claim of presented visual
//! review or cross-platform qualification.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_ui_core::{UiColor, UiContrast, UiFontRole, UiNodeId, UiPoint, UiRect, UiSize};
use meridian_ui_render::{
    DisplayList, DisplayPrimitive, UiBackdropDescriptor, UiClipId, UiCornerRadii,
    UiEffectCapabilities, UiImageHandle, UiLayerId, UiMeshHandle, UiPathCommand, UiStroke,
};
use meridian_ui_text::{UiGlyphBitmap, UiTextLayout, UiTextRaster};

use crate::ui_direct::{
    UiDirectImage, UiDirectMesh, UiDirectPrepareRequest, UiDirectPrimitiveKind,
    UiDirectRendererError, UiDirectResourceSet,
};

/// Versioned input contract for direct-UI qualification artifacts.
pub const UI_DIRECT_QUALIFICATION_SCHEMA: &str = "meridian.ui-direct-qualification/v1";

/// Fixed logical viewport used by every direct-UI qualification scene.
pub const UI_DIRECT_QUALIFICATION_VIEWPORT: UiSize = UiSize::new(320.0, 180.0);

/// Renderer-neutral categories every qualification scene must exercise.
pub const UI_DIRECT_QUALIFICATION_REQUIRED_KINDS: [UiDirectPrimitiveKind; 15] = [
    UiDirectPrimitiveKind::Rect,
    UiDirectPrimitiveKind::Border,
    UiDirectPrimitiveKind::Text,
    UiDirectPrimitiveKind::GlyphRun,
    UiDirectPrimitiveKind::FocusIndicator,
    UiDirectPrimitiveKind::RoundedRect,
    UiDirectPrimitiveKind::Path,
    UiDirectPrimitiveKind::Image,
    UiDirectPrimitiveKind::Mesh,
    UiDirectPrimitiveKind::PushClip,
    UiDirectPrimitiveKind::PopClip,
    UiDirectPrimitiveKind::BeginLayer,
    UiDirectPrimitiveKind::EndLayer,
    UiDirectPrimitiveKind::Shadow,
    UiDirectPrimitiveKind::Backdrop,
];

/// Immutable deterministic scene consumed by a direct-UI qualification runner.
///
/// Resource handles are process-local cache identities only.  The scalar
/// display revision and display list remain the source of a prepared frame.
#[derive(Clone, Debug, PartialEq)]
pub struct UiDirectQualificationCase {
    /// Stable identifier used by golden manifests and evidence artifacts.
    pub id: &'static str,
    /// Immutable display revision used by the direct renderer cache key.
    pub display_revision: u64,
    /// Logical viewport before the supplied display scale is applied.
    pub viewport: UiSize,
    /// Fixed scale factor for the scene's physical capture target.
    pub scale_factor: f32,
    /// Resolved contrast mode already reflected by the emitted display list.
    pub contrast: UiContrast,
    /// Effect capability profile selected before renderer preparation.
    pub effects: UiEffectCapabilities,
    /// Deterministic retained display list covering every primitive category.
    pub display_list: DisplayList,
    /// Deterministic image and mesh resources referenced by the display list.
    pub resources: UiDirectResourceSet,
}

impl UiDirectQualificationCase {
    /// Borrows this immutable scene as one direct-UI preparation request.
    #[must_use]
    pub const fn prepare_request(&self) -> UiDirectPrepareRequest<'_> {
        UiDirectPrepareRequest {
            display_revision: self.display_revision,
            display_list: &self.display_list,
            viewport: self.viewport,
            scale_factor: self.scale_factor,
            contrast: self.contrast,
            effects: self.effects,
            resources: &self.resources,
        }
    }

    /// Returns a stable source identity for this canonical qualification case.
    ///
    /// The value is a versioned FNV-1a digest of the scalar scene contract,
    /// every display primitive, and every referenced deterministic resource.
    /// It identifies generated corpus inputs for evidence reports; it is not a
    /// security hash and must not be used for trust decisions.
    #[must_use]
    pub fn corpus_hash(&self) -> String {
        let mut hasher = CanonicalQualificationHasher::new();
        hasher.string(UI_DIRECT_QUALIFICATION_SCHEMA);
        hasher.string(self.id);
        hasher.u64(self.display_revision);
        hasher.size(self.viewport);
        hasher.f32(self.scale_factor);
        hasher.contrast(self.contrast);
        hasher.bool(self.effects.backdrop_filtering);
        hasher.display_list(&self.display_list);
        hasher.resources(&self.resources, &self.display_list);
        format!("fnv1a64:{:016x}", hasher.finish())
    }
}

struct CanonicalQualificationHasher(u64);

impl CanonicalQualificationHasher {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    const fn new() -> Self {
        Self(Self::OFFSET_BASIS)
    }

    const fn finish(self) -> u64 {
        self.0
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn tag(&mut self, tag: u8) {
        self.bytes(&[tag]);
    }

    fn bool(&mut self, value: bool) {
        self.tag(u8::from(value));
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(u64::try_from(value).unwrap_or(u64::MAX));
    }

    fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    fn contrast(&mut self, value: UiContrast) {
        self.tag(match value {
            UiContrast::Standard => 0,
            UiContrast::High => 1,
        });
    }

    fn string(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes(value.as_bytes());
    }

    fn point(&mut self, value: UiPoint) {
        self.f32(value.x);
        self.f32(value.y);
    }

    fn size(&mut self, value: UiSize) {
        self.f32(value.width);
        self.f32(value.height);
    }

    fn rect(&mut self, value: UiRect) {
        self.point(value.origin);
        self.size(value.size);
    }

    fn color(&mut self, value: UiColor) {
        self.f32(value.red);
        self.f32(value.green);
        self.f32(value.blue);
        self.f32(value.alpha);
    }

    fn radii(&mut self, value: UiCornerRadii) {
        self.f32(value.top_left);
        self.f32(value.top_right);
        self.f32(value.bottom_right);
        self.f32(value.bottom_left);
    }

    fn node(&mut self, value: UiNodeId) {
        self.u128(value.stable_id().get());
    }

    fn stroke(&mut self, value: UiStroke) {
        self.color(value.color);
        self.f32(value.width);
        self.tag(match value.line_cap {
            meridian_ui_render::UiLineCap::Butt => 0,
            meridian_ui_render::UiLineCap::Round => 1,
            meridian_ui_render::UiLineCap::Square => 2,
        });
        self.tag(match value.line_join {
            meridian_ui_render::UiLineJoin::Miter => 0,
            meridian_ui_render::UiLineJoin::Round => 1,
            meridian_ui_render::UiLineJoin::Bevel => 2,
        });
    }

    fn path_command(&mut self, value: UiPathCommand) {
        match value {
            UiPathCommand::MoveTo(point) => {
                self.tag(0);
                self.point(point);
            }
            UiPathCommand::LineTo(point) => {
                self.tag(1);
                self.point(point);
            }
            UiPathCommand::QuadraticTo { control, end } => {
                self.tag(2);
                self.point(control);
                self.point(end);
            }
            UiPathCommand::CubicTo {
                control_a,
                control_b,
                end,
            } => {
                self.tag(3);
                self.point(control_a);
                self.point(control_b);
                self.point(end);
            }
            UiPathCommand::Close => self.tag(4),
        }
    }

    fn text_layout(&mut self, value: &UiTextLayout) {
        self.usize(value.line_count);
        self.usize(value.glyph_count);
        self.f32(value.width);
        self.f32(value.height);
        self.bool(value.used_fallback_metrics);
        self.bool(value.used_fallback_font);
        self.tag(match value.font_role {
            UiFontRole::Interface => 0,
            UiFontRole::Display => 1,
            UiFontRole::Monospace => 2,
        });
    }

    fn text_raster(&mut self, value: &UiTextRaster) {
        self.usize(value.glyphs.len());
        for glyph in &value.glyphs {
            self.point(glyph.origin);
            self.u32(glyph.width);
            self.u32(glyph.height);
            self.usize(glyph.alpha.len());
            self.bytes(&glyph.alpha);
        }
        self.bool(value.has_unrasterized_glyphs);
    }

    fn backdrop(&mut self, value: UiBackdropDescriptor) {
        self.rect(value.bounds);
        self.rect(value.sample_bounds);
        self.color(value.tint);
        self.color(value.opaque_fallback);
    }

    fn display_list(&mut self, list: &DisplayList) {
        self.usize(list.primitives.len());
        for primitive in &list.primitives {
            self.display_primitive(primitive);
        }
    }

    fn display_primitive(&mut self, primitive: &DisplayPrimitive) {
        match primitive {
            DisplayPrimitive::Rect {
                node,
                bounds,
                color,
            } => {
                self.node_rect(0, *node, *bounds, *color);
            }
            DisplayPrimitive::Border {
                node,
                bounds,
                color,
                width,
            } => {
                self.node_rect(1, *node, *bounds, *color);
                self.tag(*width);
            }
            DisplayPrimitive::Text { .. } => self.text_primitive(2, primitive),
            DisplayPrimitive::GlyphRun { .. } => self.text_primitive(3, primitive),
            DisplayPrimitive::FocusIndicator {
                node,
                bounds,
                color,
            } => {
                self.node_rect(4, *node, *bounds, *color);
            }
            DisplayPrimitive::RoundedRect {
                node,
                bounds,
                radii,
                color,
            } => {
                self.rounded_rect(*node, *bounds, *radii, *color);
            }
            DisplayPrimitive::Path {
                node,
                commands,
                fill,
                stroke,
            } => {
                self.path(*node, commands, *fill, *stroke);
            }
            DisplayPrimitive::Image {
                node,
                bounds,
                image,
                opacity,
            } => {
                self.image(*node, *bounds, *image, *opacity);
            }
            DisplayPrimitive::Mesh {
                node,
                bounds,
                mesh,
                tint,
            } => {
                self.mesh(*node, *bounds, *mesh, *tint);
            }
            DisplayPrimitive::PushClip { id, bounds, radii } => {
                self.push_clip(*id, *bounds, *radii);
            }
            DisplayPrimitive::PopClip { id } => self.pop_clip(*id),
            DisplayPrimitive::BeginLayer { id, opacity } => self.begin_layer(*id, *opacity),
            DisplayPrimitive::EndLayer { id } => self.end_layer(*id),
            DisplayPrimitive::Shadow {
                node,
                bounds,
                radii,
                offset,
                spread,
                color,
            } => {
                self.shadow(*node, *bounds, *radii, *offset, *spread, *color);
            }
            DisplayPrimitive::Backdrop { node, descriptor } => {
                self.backdrop_primitive(*node, *descriptor);
            }
        }
    }

    fn node_rect(&mut self, tag: u8, node: UiNodeId, bounds: UiRect, color: UiColor) {
        self.tag(tag);
        self.node(node);
        self.rect(bounds);
        self.color(color);
    }

    fn text_primitive(&mut self, tag: u8, primitive: &DisplayPrimitive) {
        let (node, bounds, text, color, layout, raster) = match primitive {
            DisplayPrimitive::Text {
                node,
                bounds,
                text,
                color,
                layout,
                raster,
            }
            | DisplayPrimitive::GlyphRun {
                node,
                bounds,
                text,
                color,
                layout,
                raster,
            } => (*node, *bounds, text, *color, layout, raster),
            _ => return,
        };
        self.tag(tag);
        self.node(node);
        self.rect(bounds);
        self.string(text);
        self.color(color);
        self.text_layout(layout);
        self.text_raster(raster);
    }

    fn rounded_rect(
        &mut self,
        node: UiNodeId,
        bounds: UiRect,
        radii: UiCornerRadii,
        color: UiColor,
    ) {
        self.tag(5);
        self.node(node);
        self.rect(bounds);
        self.radii(radii);
        self.color(color);
    }

    fn path(
        &mut self,
        node: UiNodeId,
        commands: &[UiPathCommand],
        fill: Option<UiColor>,
        stroke: Option<UiStroke>,
    ) {
        self.tag(6);
        self.node(node);
        self.usize(commands.len());
        for command in commands {
            self.path_command(*command);
        }
        self.bool(fill.is_some());
        if let Some(fill) = fill {
            self.color(fill);
        }
        self.bool(stroke.is_some());
        if let Some(stroke) = stroke {
            self.stroke(stroke);
        }
    }

    fn image(&mut self, node: UiNodeId, bounds: UiRect, image: UiImageHandle, opacity: f32) {
        self.tag(7);
        self.node(node);
        self.rect(bounds);
        self.u64(image.0);
        self.f32(opacity);
    }

    fn mesh(&mut self, node: UiNodeId, bounds: UiRect, mesh: UiMeshHandle, tint: UiColor) {
        self.tag(8);
        self.node(node);
        self.rect(bounds);
        self.u64(mesh.0);
        self.color(tint);
    }

    fn push_clip(&mut self, id: UiClipId, bounds: UiRect, radii: UiCornerRadii) {
        self.tag(9);
        self.u64(id.0);
        self.rect(bounds);
        self.radii(radii);
    }

    fn pop_clip(&mut self, id: UiClipId) {
        self.tag(10);
        self.u64(id.0);
    }

    fn begin_layer(&mut self, id: UiLayerId, opacity: f32) {
        self.tag(11);
        self.u64(id.0);
        self.f32(opacity);
    }

    fn end_layer(&mut self, id: UiLayerId) {
        self.tag(12);
        self.u64(id.0);
    }

    fn shadow(
        &mut self,
        node: UiNodeId,
        bounds: UiRect,
        radii: UiCornerRadii,
        offset: UiPoint,
        spread: f32,
        color: UiColor,
    ) {
        self.tag(13);
        self.node(node);
        self.rect(bounds);
        self.radii(radii);
        self.point(offset);
        self.f32(spread);
        self.color(color);
    }

    fn backdrop_primitive(&mut self, node: UiNodeId, descriptor: UiBackdropDescriptor) {
        self.tag(14);
        self.node(node);
        self.backdrop(descriptor);
    }

    fn resources(&mut self, resources: &UiDirectResourceSet, list: &DisplayList) {
        self.u64(resources.image_revision);
        self.u64(resources.mesh_revision);
        let mut images = BTreeSet::new();
        let mut meshes = BTreeSet::new();
        for primitive in &list.primitives {
            match primitive {
                DisplayPrimitive::Image { image, .. } => {
                    images.insert(*image);
                }
                DisplayPrimitive::Mesh { mesh, .. } => {
                    meshes.insert(*mesh);
                }
                _ => {}
            }
        }
        self.usize(images.len());
        for handle in images {
            self.u64(handle.0);
            let image = resources.image(handle);
            self.bool(image.is_some());
            if let Some(image) = image {
                self.u32(image.width);
                self.u32(image.height);
                self.usize(image.rgba.len());
                self.bytes(&image.rgba);
            }
        }
        self.usize(meshes.len());
        for handle in meshes {
            self.u64(handle.0);
            let mesh = resources.mesh(handle);
            self.bool(mesh.is_some());
            if let Some(mesh) = mesh {
                self.usize(mesh.vertices.len());
                for vertex in &mesh.vertices {
                    self.u32(u32::from(vertex.x_milli));
                    self.u32(u32::from(vertex.y_milli));
                    self.u32(u32::from(vertex.u_milli));
                    self.u32(u32::from(vertex.v_milli));
                }
                self.usize(mesh.indices.len());
                for index in &mesh.indices {
                    self.u32(*index);
                }
            }
        }
    }
}

/// Creates deterministic qualification scenes for normal 1x, normal 2x, and
/// high-contrast opaque-fallback capture.
///
/// # Errors
///
/// Returns an owned direct-renderer error before allocation if a bounded image
/// descriptor cannot be constructed.  Every returned display list is valid.
pub fn ui_direct_qualification_cases(
) -> Result<Vec<UiDirectQualificationCase>, UiDirectRendererError> {
    Ok(vec![
        qualification_case(
            "standard-1x",
            1,
            1.0,
            UiContrast::Standard,
            UiEffectCapabilities {
                backdrop_filtering: true,
            },
        )?,
        qualification_case(
            "standard-2x",
            2,
            2.0,
            UiContrast::Standard,
            UiEffectCapabilities {
                backdrop_filtering: true,
            },
        )?,
        qualification_case(
            "high-contrast-opaque-1x",
            3,
            1.0,
            UiContrast::High,
            UiEffectCapabilities::default(),
        )?,
    ])
}

/// Returns required categories as a set for diagnostic comparison.
#[must_use]
pub fn ui_direct_qualification_required_kinds() -> BTreeSet<UiDirectPrimitiveKind> {
    UI_DIRECT_QUALIFICATION_REQUIRED_KINDS.into_iter().collect()
}

fn qualification_case(
    id: &'static str,
    display_revision: u64,
    scale_factor: f32,
    contrast: UiContrast,
    effects: UiEffectCapabilities,
) -> Result<UiDirectQualificationCase, UiDirectRendererError> {
    let colors = QualificationColors::for_contrast(contrast);
    let image = UiImageHandle(71);
    let mesh = UiMeshHandle(72);
    let resources = UiDirectResourceSet::new(1, 1)
        .with_image_descriptor(UiDirectImage::try_solid(image, 2, 2, [212, 177, 96, 255])?)
        .with_mesh_descriptor(UiDirectMesh::unit_quad(mesh));
    let display_list = qualification_display_list(image, mesh, colors);
    display_list
        .validate()
        .map_err(UiDirectRendererError::InvalidDisplayList)?;
    Ok(UiDirectQualificationCase {
        id,
        display_revision,
        viewport: UI_DIRECT_QUALIFICATION_VIEWPORT,
        scale_factor,
        contrast,
        effects,
        display_list,
        resources,
    })
}

#[derive(Clone, Copy)]
struct QualificationColors {
    background: UiColor,
    surface: UiColor,
    border: UiColor,
    foreground: UiColor,
    accent: UiColor,
    muted: UiColor,
    shadow: UiColor,
}

impl QualificationColors {
    const fn for_contrast(contrast: UiContrast) -> Self {
        match contrast {
            UiContrast::Standard => Self {
                background: UiColor::background(),
                surface: UiColor::surface(),
                border: UiColor::border(),
                foreground: UiColor::text(),
                accent: UiColor::amber(),
                muted: UiColor::muted_text(),
                shadow: UiColor::rgba(0.0, 0.0, 0.0, 0.48),
            },
            UiContrast::High => Self {
                background: UiColor::rgba(0.0, 0.0, 0.0, 1.0),
                surface: UiColor::rgba(0.035, 0.035, 0.035, 1.0),
                border: UiColor::rgba(1.0, 1.0, 1.0, 1.0),
                foreground: UiColor::rgba(1.0, 1.0, 1.0, 1.0),
                accent: UiColor::rgba(1.0, 0.82, 0.35, 1.0),
                muted: UiColor::rgba(0.75, 0.75, 0.75, 1.0),
                shadow: UiColor::rgba(0.0, 0.0, 0.0, 1.0),
            },
        }
    }
}

fn qualification_display_list(
    image: UiImageHandle,
    mesh: UiMeshHandle,
    colors: QualificationColors,
) -> DisplayList {
    let mut primitives = qualification_chrome_primitives(colors);
    primitives.extend(qualification_graphics_primitives(image, mesh, colors));
    primitives.extend(qualification_clip_primitives(colors));
    primitives.extend(qualification_layer_primitives(colors));
    DisplayList { primitives }
}

fn qualification_chrome_primitives(colors: QualificationColors) -> Vec<DisplayPrimitive> {
    let viewport = UiRect::new(UiPoint { x: 0.0, y: 0.0 }, UI_DIRECT_QUALIFICATION_VIEWPORT);
    let header = rect(8.0, 8.0, 304.0, 28.0);
    vec![
        DisplayPrimitive::Rect {
            node: UiNodeId::new(1),
            bounds: viewport,
            color: colors.background,
        },
        DisplayPrimitive::Rect {
            node: UiNodeId::new(2),
            bounds: header,
            color: colors.surface,
        },
        DisplayPrimitive::Border {
            node: UiNodeId::new(3),
            bounds: header,
            color: colors.border,
            width: 1,
        },
        DisplayPrimitive::Text {
            node: UiNodeId::new(4),
            bounds: rect(24.0, 55.0, 38.0, 16.0),
            text: "MU".to_owned(),
            color: colors.foreground,
            layout: qualification_text_layout(),
            raster: qualification_text_raster(QualificationGlyphPair::Mu),
        },
        DisplayPrimitive::GlyphRun {
            node: UiNodeId::new(5),
            bounds: rect(82.0, 55.0, 38.0, 16.0),
            text: "UI".to_owned(),
            color: colors.muted,
            layout: qualification_text_layout(),
            raster: qualification_text_raster(QualificationGlyphPair::Ui),
        },
        DisplayPrimitive::FocusIndicator {
            node: UiNodeId::new(6),
            bounds: rect(130.0, 48.0, 42.0, 30.0),
            color: colors.accent,
        },
        DisplayPrimitive::RoundedRect {
            node: UiNodeId::new(7),
            bounds: rect(184.0, 48.0, 48.0, 30.0),
            radii: UiCornerRadii::uniform(6.0),
            color: colors.surface,
        },
    ]
}

fn qualification_graphics_primitives(
    image: UiImageHandle,
    mesh: UiMeshHandle,
    colors: QualificationColors,
) -> Vec<DisplayPrimitive> {
    vec![
        DisplayPrimitive::Path {
            node: UiNodeId::new(8),
            commands: vec![
                UiPathCommand::MoveTo(UiPoint { x: 88.0, y: 104.0 }),
                UiPathCommand::LineTo(UiPoint { x: 128.0, y: 104.0 }),
                UiPathCommand::LineTo(UiPoint { x: 108.0, y: 140.0 }),
                UiPathCommand::Close,
            ],
            fill: Some(colors.accent),
            stroke: Some(UiStroke::new(colors.foreground, 1.0)),
        },
        DisplayPrimitive::Image {
            node: UiNodeId::new(9),
            bounds: rect(244.0, 48.0, 24.0, 24.0),
            image,
            opacity: 1.0,
        },
        DisplayPrimitive::Mesh {
            node: UiNodeId::new(10),
            bounds: rect(278.0, 48.0, 24.0, 24.0),
            mesh,
            tint: colors.foreground,
        },
        DisplayPrimitive::Shadow {
            node: UiNodeId::new(11),
            bounds: rect(16.0, 48.0, 54.0, 30.0),
            radii: UiCornerRadii::uniform(5.0),
            offset: UiPoint { x: 2.0, y: 3.0 },
            spread: 2.0,
            color: colors.shadow,
        },
    ]
}

fn qualification_clip_primitives(colors: QualificationColors) -> Vec<DisplayPrimitive> {
    let clip_scope = UiClipId(301);
    vec![
        DisplayPrimitive::PushClip {
            id: clip_scope,
            bounds: rect(16.0, 96.0, 52.0, 48.0),
            radii: UiCornerRadii::uniform(8.0),
        },
        DisplayPrimitive::RoundedRect {
            node: UiNodeId::new(12),
            bounds: rect(10.0, 102.0, 64.0, 34.0),
            radii: UiCornerRadii::uniform(10.0),
            color: colors.surface,
        },
        DisplayPrimitive::PopClip { id: clip_scope },
    ]
}

fn qualification_layer_primitives(colors: QualificationColors) -> Vec<DisplayPrimitive> {
    let layer = UiLayerId(401);
    let backdrop_bounds = rect(196.0, 102.0, 96.0, 42.0);
    vec![
        DisplayPrimitive::BeginLayer {
            id: layer,
            opacity: 0.86,
        },
        DisplayPrimitive::Backdrop {
            node: UiNodeId::new(13),
            descriptor: UiBackdropDescriptor {
                bounds: backdrop_bounds,
                sample_bounds: rect(192.0, 98.0, 104.0, 50.0),
                tint: colors.surface,
                opaque_fallback: colors.background,
            },
        },
        DisplayPrimitive::Border {
            node: UiNodeId::new(14),
            bounds: backdrop_bounds,
            color: colors.border,
            width: 1,
        },
        DisplayPrimitive::EndLayer { id: layer },
    ]
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> UiRect {
    UiRect::new(UiPoint { x, y }, UiSize::new(width, height))
}

fn qualification_text_layout() -> UiTextLayout {
    UiTextLayout {
        line_count: 1,
        glyph_count: 2,
        width: 11.0,
        height: 7.0,
        used_fallback_metrics: false,
        used_fallback_font: false,
        font_role: UiFontRole::Interface,
    }
}

#[derive(Clone, Copy)]
enum QualificationGlyphPair {
    Mu,
    Ui,
}

const GLYPH_M: [u8; 35] = [
    255, 0, 0, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 0, 255, 255, 0, 0, 0, 255, 255, 0, 0, 0,
    255, 255, 0, 0, 0, 255, 255, 0, 0, 0, 255,
];
const GLYPH_U: [u8; 35] = [
    255, 0, 0, 0, 255, 255, 0, 0, 0, 255, 255, 0, 0, 0, 255, 255, 0, 0, 0, 255, 255, 0, 0, 0, 255,
    255, 0, 0, 0, 255, 0, 255, 255, 255, 0,
];
const GLYPH_I: [u8; 35] = [
    0, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 0,
    255, 0, 0, 0, 255, 255, 255, 0,
];

fn qualification_text_raster(pair: QualificationGlyphPair) -> UiTextRaster {
    let (first, second) = match pair {
        QualificationGlyphPair::Mu => (&GLYPH_M, &GLYPH_U),
        QualificationGlyphPair::Ui => (&GLYPH_U, &GLYPH_I),
    };
    UiTextRaster {
        glyphs: vec![glyph_bitmap(0.0, first), glyph_bitmap(6.0, second)],
        has_unrasterized_glyphs: false,
    }
}

fn glyph_bitmap(origin_x: f32, alpha: &[u8; 35]) -> UiGlyphBitmap {
    UiGlyphBitmap {
        origin: UiPoint {
            x: origin_x,
            y: 0.0,
        },
        width: 5,
        height: 7,
        alpha: alpha.to_vec(),
    }
}

/// Borrowed raw RGBA8 image used by deterministic golden comparison.
#[derive(Clone, Copy, Debug)]
pub struct UiDirectRgba8Image<'a> {
    /// Physical pixel width.
    pub width: u32,
    /// Physical pixel height.
    pub height: u32,
    /// Tightly packed, row-major sRGB RGBA8 pixels.
    pub pixels: &'a [u8],
}

impl<'a> UiDirectRgba8Image<'a> {
    /// Creates a borrowed raw image.  Comparison validates dimensions and bytes.
    #[must_use]
    pub const fn new(width: u32, height: u32, pixels: &'a [u8]) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// First differing pixel reported by a raw RGBA8 golden comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDirectGoldenDifference {
    /// Zero-based physical x coordinate.
    pub x: u32,
    /// Zero-based physical y coordinate.
    pub y: u32,
    /// Expected RGBA8 channels.
    pub expected: [u8; 4],
    /// Actual RGBA8 channels.
    pub actual: [u8; 4],
}

/// Aggregate result from a raw RGBA8 golden comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDirectGoldenComparison {
    /// Allowed absolute delta for each color channel.
    pub allowed_channel_delta: u8,
    /// Number of pixels compared after input validation.
    pub compared_pixel_count: u64,
    /// Number of pixels with any channel above the allowed delta.
    pub differing_pixel_count: u64,
    /// Greatest absolute delta observed across all compared channels.
    pub maximum_channel_delta: u8,
    /// First differing pixel in row-major order, if any.
    pub first_difference: Option<UiDirectGoldenDifference>,
}

impl UiDirectGoldenComparison {
    /// Whether all compared pixels were within the configured channel delta.
    #[must_use]
    pub const fn passed(self) -> bool {
        self.differing_pixel_count == 0
    }
}

/// Rejection emitted before raw RGBA8 comparison can start.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiDirectGoldenComparisonError {
    /// A golden image had a zero physical dimension.
    ZeroExtent,
    /// Physical dimensions could not be represented as a bounded byte count.
    SizeOverflow,
    /// Expected bytes do not match tightly packed RGBA8 dimensions.
    InvalidExpectedByteLength {
        /// Expected byte count from dimensions.
        expected: usize,
        /// Supplied byte count.
        actual: usize,
    },
    /// Actual bytes do not match tightly packed RGBA8 dimensions.
    InvalidActualByteLength {
        /// Expected byte count from dimensions.
        expected: usize,
        /// Supplied byte count.
        actual: usize,
    },
    /// Golden images have different physical dimensions.
    DimensionMismatch {
        /// Expected physical width.
        expected_width: u32,
        /// Expected physical height.
        expected_height: u32,
        /// Actual physical width.
        actual_width: u32,
        /// Actual physical height.
        actual_height: u32,
    },
}

impl Display for UiDirectGoldenComparisonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent => formatter.write_str("RGBA8 golden image has zero extent"),
            Self::SizeOverflow => formatter.write_str("RGBA8 golden image size overflowed"),
            Self::InvalidExpectedByteLength { expected, actual } => write!(
                formatter,
                "expected RGBA8 golden has {actual} bytes; dimensions require {expected}"
            ),
            Self::InvalidActualByteLength { expected, actual } => write!(
                formatter,
                "actual RGBA8 golden has {actual} bytes; dimensions require {expected}"
            ),
            Self::DimensionMismatch {
                expected_width,
                expected_height,
                actual_width,
                actual_height,
            } => write!(
                formatter,
                "RGBA8 golden dimensions differ: expected {expected_width}x{expected_height}, got {actual_width}x{actual_height}"
            ),
        }
    }
}

impl Error for UiDirectGoldenComparisonError {}

/// Compares raw tightly packed RGBA8 images using an explicit channel delta.
///
/// A value of zero is byte-exact comparison.  Callers choosing a nonzero delta
/// must record its backend-specific justification in their evidence manifest.
///
/// # Errors
///
/// Returns a typed dimensional or byte-layout rejection before reading pixel
/// data.  It never allocates from captured dimensions.
pub fn compare_ui_direct_rgba8(
    expected: UiDirectRgba8Image<'_>,
    actual: UiDirectRgba8Image<'_>,
    allowed_channel_delta: u8,
) -> Result<UiDirectGoldenComparison, UiDirectGoldenComparisonError> {
    let expected_length = rgba8_byte_len(expected.width, expected.height)?;
    let actual_length = rgba8_byte_len(actual.width, actual.height)?;
    if expected.pixels.len() != expected_length {
        return Err(UiDirectGoldenComparisonError::InvalidExpectedByteLength {
            expected: expected_length,
            actual: expected.pixels.len(),
        });
    }
    if actual.pixels.len() != actual_length {
        return Err(UiDirectGoldenComparisonError::InvalidActualByteLength {
            expected: actual_length,
            actual: actual.pixels.len(),
        });
    }
    if expected.width != actual.width || expected.height != actual.height {
        return Err(UiDirectGoldenComparisonError::DimensionMismatch {
            expected_width: expected.width,
            expected_height: expected.height,
            actual_width: actual.width,
            actual_height: actual.height,
        });
    }

    let mut differing_pixel_count = 0_u64;
    let mut maximum_channel_delta = 0_u8;
    let mut first_difference = None;
    for (index, (expected_pixel, actual_pixel)) in expected
        .pixels
        .chunks_exact(4)
        .zip(actual.pixels.chunks_exact(4))
        .enumerate()
    {
        let expected_channels = [
            expected_pixel[0],
            expected_pixel[1],
            expected_pixel[2],
            expected_pixel[3],
        ];
        let actual_channels = [
            actual_pixel[0],
            actual_pixel[1],
            actual_pixel[2],
            actual_pixel[3],
        ];
        let pixel_delta = expected_channels
            .into_iter()
            .zip(actual_channels)
            .map(|(expected_channel, actual_channel)| expected_channel.abs_diff(actual_channel))
            .max()
            .unwrap_or(0);
        maximum_channel_delta = maximum_channel_delta.max(pixel_delta);
        if pixel_delta > allowed_channel_delta {
            differing_pixel_count = differing_pixel_count.saturating_add(1);
            if first_difference.is_none() {
                let index = u32::try_from(index)
                    .map_err(|_| UiDirectGoldenComparisonError::SizeOverflow)?;
                first_difference = Some(UiDirectGoldenDifference {
                    x: index % expected.width,
                    y: index / expected.width,
                    expected: expected_channels,
                    actual: actual_channels,
                });
            }
        }
    }
    Ok(UiDirectGoldenComparison {
        allowed_channel_delta,
        compared_pixel_count: u64::from(expected.width) * u64::from(expected.height),
        differing_pixel_count,
        maximum_channel_delta,
        first_difference,
    })
}

/// Performs byte-exact raw RGBA8 golden comparison.
///
/// # Errors
///
/// Returns the same typed rejection as [`compare_ui_direct_rgba8`].
pub fn compare_ui_direct_rgba8_exact(
    expected: UiDirectRgba8Image<'_>,
    actual: UiDirectRgba8Image<'_>,
) -> Result<UiDirectGoldenComparison, UiDirectGoldenComparisonError> {
    compare_ui_direct_rgba8(expected, actual, 0)
}

fn rgba8_byte_len(width: u32, height: u32) -> Result<usize, UiDirectGoldenComparisonError> {
    if width == 0 || height == 0 {
        return Err(UiDirectGoldenComparisonError::ZeroExtent);
    }
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(UiDirectGoldenComparisonError::SizeOverflow)
}

#[cfg(test)]
mod tests {
    use meridian_platform::WindowSize;
    use meridian_rhi::{RhiRenderIdentity, SurfaceFormat};

    use super::*;
    use crate::ui_direct::UiDirectGpuRenderer;

    fn identity() -> RhiRenderIdentity {
        RhiRenderIdentity {
            device_generation: 1,
            surface_generation: 1,
            surface_format: SurfaceFormat {
                name: "Bgra8UnormSrgb".to_owned(),
                srgb: true,
            },
            surface_size: WindowSize::new(640, 360),
            surface_configured: true,
        }
    }

    #[test]
    fn qualification_corpus_prepares_every_required_category() {
        let expected = ui_direct_qualification_required_kinds();
        for case in ui_direct_qualification_cases().expect("qualification corpus constructs") {
            case.display_list
                .validate()
                .expect("qualification display list is valid");
            let mut renderer = UiDirectGpuRenderer::new(identity());
            let plan = renderer
                .prepare_frame(case.prepare_request())
                .expect("qualification corpus prepares");
            assert_eq!(plan.diagnostics().observed_kinds, expected, "{}", case.id);
            assert!(
                plan.diagnostics().unsupported_kinds.is_empty(),
                "{} has unsupported primitive kinds",
                case.id
            );
            assert!(
                plan.diagnostics().prepared_kinds.is_superset(&expected),
                "{} did not prepare every primitive kind",
                case.id
            );
            let footprint = plan.footprint();
            assert_eq!(
                footprint.primitive_count,
                case.display_list.primitives.len()
            );
            assert_eq!(footprint.batch_count, plan.batches().len());
            assert_eq!(
                footprint.gpu_upload_payload_bytes,
                footprint
                    .cpu_vertex_bytes
                    .saturating_add(footprint.cpu_index_bytes)
                    .saturating_add(footprint.cpu_atlas_bytes),
            );
            assert_eq!(
                footprint.planned_color_target_bytes,
                plan.diagnostics().layer_target_bytes
            );
        }
    }

    #[test]
    fn qualification_corpus_hash_is_stable_and_input_sensitive() {
        let cases = ui_direct_qualification_cases().expect("qualification corpus constructs");
        let first = &cases[0];
        let stable_hash = first.corpus_hash();
        assert_eq!(stable_hash, first.corpus_hash());
        assert!(stable_hash.starts_with("fnv1a64:"));

        let mut changed = first.clone();
        changed.display_revision = changed.display_revision.saturating_add(1);
        assert_ne!(stable_hash, changed.corpus_hash());
    }

    #[test]
    fn raw_rgba8_comparison_reports_exact_difference_location() {
        let expected = [1, 2, 3, 4, 10, 20, 30, 40];
        let actual = [1, 2, 3, 4, 11, 20, 30, 40];
        let comparison = compare_ui_direct_rgba8_exact(
            UiDirectRgba8Image::new(2, 1, &expected),
            UiDirectRgba8Image::new(2, 1, &actual),
        )
        .expect("valid RGBA8 input compares");
        assert!(!comparison.passed());
        assert_eq!(comparison.differing_pixel_count, 1);
        assert_eq!(comparison.maximum_channel_delta, 1);
        assert_eq!(
            comparison.first_difference,
            Some(UiDirectGoldenDifference {
                x: 1,
                y: 0,
                expected: [10, 20, 30, 40],
                actual: [11, 20, 30, 40],
            })
        );
    }

    #[test]
    fn raw_rgba8_comparison_honors_explicit_tolerance() {
        let expected = [0, 40, 90, 255];
        let actual = [1, 39, 92, 255];
        let comparison = compare_ui_direct_rgba8(
            UiDirectRgba8Image::new(1, 1, &expected),
            UiDirectRgba8Image::new(1, 1, &actual),
            2,
        )
        .expect("valid RGBA8 input compares");
        assert!(comparison.passed());
        assert_eq!(comparison.maximum_channel_delta, 2);
    }

    #[test]
    fn raw_rgba8_comparison_rejects_invalid_byte_layout_before_reading() {
        let error = compare_ui_direct_rgba8_exact(
            UiDirectRgba8Image::new(2, 1, &[0; 7]),
            UiDirectRgba8Image::new(2, 1, &[0; 8]),
        )
        .expect_err("truncated expected input is rejected");
        assert_eq!(
            error,
            UiDirectGoldenComparisonError::InvalidExpectedByteLength {
                expected: 8,
                actual: 7,
            }
        );
    }

    #[test]
    fn raw_rgba8_comparison_rejects_mismatched_dimensions() {
        let error = compare_ui_direct_rgba8_exact(
            UiDirectRgba8Image::new(1, 2, &[0; 8]),
            UiDirectRgba8Image::new(2, 1, &[0; 8]),
        )
        .expect_err("different dimensions are rejected");
        assert_eq!(
            error,
            UiDirectGoldenComparisonError::DimensionMismatch {
                expected_width: 1,
                expected_height: 2,
                actual_width: 2,
                actual_height: 1,
            }
        );
    }
}
