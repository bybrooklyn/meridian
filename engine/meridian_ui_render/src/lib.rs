//! Renderer-neutral Meridian UI frame primitives.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_ui_core::{
    ThemeId, UiColor, UiContrast, UiNodeId, UiPoint, UiRect, MAX_DISPLAY_PRIMITIVES, MAX_TEXT_BYTES,
};
use meridian_ui_text::{UiTextLayout, UiTextRaster, MAX_GLYPH_RASTER_BYTES};

/// Shared path complexity bound checked before a primitive is accepted.
pub const MAX_PATH_COMMANDS_PER_PRIMITIVE: usize = 4_096;

/// Corner radii in logical pixels, ordered clockwise from top-left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiCornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl UiCornerRadii {
    #[must_use]
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    fn is_finite_nonnegative(self) -> bool {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
        .iter()
        .all(|radius| radius.is_finite() && *radius >= 0.0)
    }
}

/// Renderer-neutral path segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiPathCommand {
    MoveTo(UiPoint),
    LineTo(UiPoint),
    QuadraticTo {
        control: UiPoint,
        end: UiPoint,
    },
    CubicTo {
        control_a: UiPoint,
        control_b: UiPoint,
        end: UiPoint,
    },
    Close,
}

/// Process-local image cache handle. It is never serialized as source identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UiImageHandle(pub u64);

/// Process-local mesh cache handle. Logical UI state remains authoritative.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UiMeshHandle(pub u64);

/// Identity for one nested clip scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UiClipId(pub u64);

/// Identity for one isolated compositing layer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UiLayerId(pub u64);

/// Geometrically bounded backdrop request with a required opaque fallback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiBackdropDescriptor {
    pub bounds: UiRect,
    pub sample_bounds: UiRect,
    pub tint: UiColor,
    pub opaque_fallback: UiColor,
}

/// Renderer capabilities used to resolve optional effects before submission.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiEffectCapabilities {
    pub backdrop_filtering: bool,
}

/// Backdrop policy resolved without exposing a backend effect type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolvedBackdrop {
    Effect(UiBackdropDescriptor),
    Opaque { bounds: UiRect, color: UiColor },
}

/// Resolves selective backdrop effects to their mandatory opaque fallback for
/// high contrast or a renderer without bounded backdrop support.
#[must_use]
pub const fn resolve_backdrop(
    descriptor: UiBackdropDescriptor,
    contrast: UiContrast,
    capabilities: UiEffectCapabilities,
) -> ResolvedBackdrop {
    if matches!(contrast, UiContrast::Standard) && capabilities.backdrop_filtering {
        ResolvedBackdrop::Effect(descriptor)
    } else {
        ResolvedBackdrop::Opaque {
            bounds: descriptor.bounds,
            color: descriptor.opaque_fallback,
        }
    }
}

/// Renderer cache identity includes every presentation input that changes pixels.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UiRenderCacheKey {
    pub content_revision: u64,
    pub theme: ThemeId,
    pub scale_milli: u16,
    pub contrast: UiContrast,
    pub capability_profile: u64,
}

/// A renderer-neutral visual primitive consumed by the UI render adapter.
#[derive(Clone, Debug, PartialEq)]
pub enum DisplayPrimitive {
    Rect {
        node: UiNodeId,
        bounds: UiRect,
        color: UiColor,
    },
    Border {
        node: UiNodeId,
        bounds: UiRect,
        color: UiColor,
        width: u8,
    },
    Text {
        node: UiNodeId,
        bounds: UiRect,
        text: String,
        color: UiColor,
        layout: UiTextLayout,
        raster: UiTextRaster,
    },
    GlyphRun {
        node: UiNodeId,
        bounds: UiRect,
        text: String,
        color: UiColor,
        layout: UiTextLayout,
        raster: UiTextRaster,
    },
    FocusIndicator {
        node: UiNodeId,
        bounds: UiRect,
        color: UiColor,
    },
    RoundedRect {
        node: UiNodeId,
        bounds: UiRect,
        radii: UiCornerRadii,
        color: UiColor,
    },
    Path {
        node: UiNodeId,
        commands: Vec<UiPathCommand>,
        color: UiColor,
    },
    Image {
        node: UiNodeId,
        bounds: UiRect,
        image: UiImageHandle,
        opacity: f32,
    },
    Mesh {
        node: UiNodeId,
        bounds: UiRect,
        mesh: UiMeshHandle,
        tint: UiColor,
    },
    PushClip {
        id: UiClipId,
        bounds: UiRect,
        radii: UiCornerRadii,
    },
    PopClip {
        id: UiClipId,
    },
    BeginLayer {
        id: UiLayerId,
        opacity: f32,
    },
    EndLayer {
        id: UiLayerId,
    },
    Shadow {
        node: UiNodeId,
        bounds: UiRect,
        radii: UiCornerRadii,
        offset: UiPoint,
        spread: f32,
        color: UiColor,
    },
    Backdrop {
        node: UiNodeId,
        descriptor: UiBackdropDescriptor,
    },
}

/// Immutable frame display output.  It never contains GPU or text-adapter types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub primitives: Vec<DisplayPrimitive>,
}

/// A malformed scope or unbounded effect is rejected before a renderer sees it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayListError {
    TooManyPrimitives {
        count: usize,
        maximum: usize,
    },
    PathTooComplex {
        index: usize,
        count: usize,
        maximum: usize,
    },
    InvalidGeometry {
        index: usize,
    },
    InvalidOpacity {
        index: usize,
    },
    InvalidTextRaster {
        index: usize,
    },
    TextRasterTooLarge {
        index: usize,
        bytes: usize,
        maximum: usize,
    },
    ClipMismatch {
        index: usize,
    },
    LayerMismatch {
        index: usize,
    },
    UnclosedClip(UiClipId),
    UnclosedLayer(UiLayerId),
    UnboundedBackdrop {
        index: usize,
    },
}

impl Display for DisplayListError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyPrimitives { count, maximum } => {
                write!(formatter, "display list has {count} primitives; maximum is {maximum}")
            }
            Self::PathTooComplex {
                index,
                count,
                maximum,
            } => write!(
                formatter,
                "path primitive {index} has {count} commands; maximum is {maximum}"
            ),
            Self::InvalidGeometry { index } => {
                write!(formatter, "primitive {index} contains invalid geometry or color")
            }
            Self::InvalidOpacity { index } => {
                write!(formatter, "primitive {index} contains invalid opacity")
            }
            Self::InvalidTextRaster { index } => {
                write!(formatter, "text primitive {index} contains an invalid raster")
            }
            Self::TextRasterTooLarge {
                index,
                bytes,
                maximum,
            } => write!(
                formatter,
                "text primitive {index} raises frame raster storage to {bytes} bytes; maximum is {maximum}"
            ),
            Self::ClipMismatch { index } => {
                write!(formatter, "clip scope is mismatched at primitive {index}")
            }
            Self::LayerMismatch { index } => {
                write!(formatter, "layer scope is mismatched at primitive {index}")
            }
            Self::UnclosedClip(id) => write!(formatter, "clip scope {id:?} is not closed"),
            Self::UnclosedLayer(id) => write!(formatter, "layer scope {id:?} is not closed"),
            Self::UnboundedBackdrop { index } => {
                write!(formatter, "backdrop primitive {index} exceeds its sample bounds")
            }
        }
    }
}

impl Error for DisplayListError {}

impl DisplayList {
    /// Adds one already-owned primitive after enforcing aggregate and path bounds.
    ///
    /// # Errors
    ///
    /// Rejects aggregate or path complexity before mutating the list.
    pub fn try_push(&mut self, primitive: DisplayPrimitive) -> Result<(), DisplayListError> {
        let next_count = self.primitives.len().saturating_add(1);
        if next_count > MAX_DISPLAY_PRIMITIVES {
            return Err(DisplayListError::TooManyPrimitives {
                count: next_count,
                maximum: MAX_DISPLAY_PRIMITIVES,
            });
        }
        if let DisplayPrimitive::Path { commands, .. } = &primitive {
            if commands.len() > MAX_PATH_COMMANDS_PER_PRIMITIVE {
                return Err(DisplayListError::PathTooComplex {
                    index: self.primitives.len(),
                    count: commands.len(),
                    maximum: MAX_PATH_COMMANDS_PER_PRIMITIVE,
                });
            }
        }
        self.primitives.push(primitive);
        Ok(())
    }

    /// Validates primitive ordering, finite geometry, nesting, and effect bounds.
    ///
    /// # Errors
    ///
    /// Returns the first malformed primitive or unclosed scope.
    pub fn validate(&self) -> Result<(), DisplayListError> {
        if self.primitives.len() > MAX_DISPLAY_PRIMITIVES {
            return Err(DisplayListError::TooManyPrimitives {
                count: self.primitives.len(),
                maximum: MAX_DISPLAY_PRIMITIVES,
            });
        }
        let mut clips = Vec::new();
        let mut layers = Vec::new();
        let mut text_raster_bytes = 0_usize;
        for (index, primitive) in self.primitives.iter().enumerate() {
            validate_primitive_colors(primitive, index)?;
            if let DisplayPrimitive::Text {
                text,
                layout,
                raster,
                ..
            }
            | DisplayPrimitive::GlyphRun {
                text,
                layout,
                raster,
                ..
            } = primitive
            {
                validate_text_raster(text, layout, raster, index, &mut text_raster_bytes)?;
            }
            validate_primitive(primitive, index, &mut clips, &mut layers)?;
        }
        if let Some(id) = clips.pop() {
            return Err(DisplayListError::UnclosedClip(id));
        }
        if let Some(id) = layers.pop() {
            return Err(DisplayListError::UnclosedLayer(id));
        }
        Ok(())
    }
}

fn validate_primitive(
    primitive: &DisplayPrimitive,
    index: usize,
    clips: &mut Vec<UiClipId>,
    layers: &mut Vec<UiLayerId>,
) -> Result<(), DisplayListError> {
    match primitive {
        DisplayPrimitive::Rect { bounds, .. }
        | DisplayPrimitive::Border { bounds, .. }
        | DisplayPrimitive::Text { bounds, .. }
        | DisplayPrimitive::GlyphRun { bounds, .. }
        | DisplayPrimitive::FocusIndicator { bounds, .. }
        | DisplayPrimitive::Mesh { bounds, .. } => validate_rect(*bounds, index),
        DisplayPrimitive::Image {
            bounds, opacity, ..
        } => {
            validate_rect(*bounds, index)?;
            validate_opacity(*opacity, index)
        }
        DisplayPrimitive::RoundedRect { bounds, radii, .. } => {
            validate_rect(*bounds, index)?;
            valid_radii(*radii, index)
        }
        DisplayPrimitive::Shadow {
            bounds,
            radii,
            offset,
            spread,
            ..
        } => {
            validate_rect(*bounds, index)?;
            if radii.is_finite_nonnegative()
                && offset.x.is_finite()
                && offset.y.is_finite()
                && spread.is_finite()
            {
                Ok(())
            } else {
                Err(DisplayListError::InvalidGeometry { index })
            }
        }
        DisplayPrimitive::Path { commands, .. } => validate_path(commands, index),
        DisplayPrimitive::PushClip { id, bounds, radii } => {
            validate_rect(*bounds, index)?;
            valid_radii(*radii, index)?;
            clips.push(*id);
            Ok(())
        }
        DisplayPrimitive::PopClip { id } => {
            if clips.pop() == Some(*id) {
                Ok(())
            } else {
                Err(DisplayListError::ClipMismatch { index })
            }
        }
        DisplayPrimitive::BeginLayer { id, opacity } => {
            validate_opacity(*opacity, index)?;
            layers.push(*id);
            Ok(())
        }
        DisplayPrimitive::EndLayer { id } => {
            if layers.pop() == Some(*id) {
                Ok(())
            } else {
                Err(DisplayListError::LayerMismatch { index })
            }
        }
        DisplayPrimitive::Backdrop { descriptor, .. } => {
            validate_rect(descriptor.bounds, index)?;
            validate_rect(descriptor.sample_bounds, index)?;
            if contains_rect(descriptor.sample_bounds, descriptor.bounds) {
                Ok(())
            } else {
                Err(DisplayListError::UnboundedBackdrop { index })
            }
        }
    }
}

fn valid_radii(radii: UiCornerRadii, index: usize) -> Result<(), DisplayListError> {
    if radii.is_finite_nonnegative() {
        Ok(())
    } else {
        Err(DisplayListError::InvalidGeometry { index })
    }
}

fn validate_path(commands: &[UiPathCommand], index: usize) -> Result<(), DisplayListError> {
    if commands.len() > MAX_PATH_COMMANDS_PER_PRIMITIVE {
        return Err(DisplayListError::PathTooComplex {
            index,
            count: commands.len(),
            maximum: MAX_PATH_COMMANDS_PER_PRIMITIVE,
        });
    }
    if commands
        .iter()
        .any(|command| !path_command_is_finite(*command))
    {
        Err(DisplayListError::InvalidGeometry { index })
    } else {
        Ok(())
    }
}

fn validate_text_raster(
    text: &str,
    layout: &UiTextLayout,
    raster: &UiTextRaster,
    index: usize,
    aggregate_bytes: &mut usize,
) -> Result<(), DisplayListError> {
    if text.len() > MAX_TEXT_BYTES
        || !layout.width.is_finite()
        || !layout.height.is_finite()
        || layout.width < 0.0
        || layout.height < 0.0
        || raster.glyphs.len() > layout.glyph_count
    {
        return Err(DisplayListError::InvalidTextRaster { index });
    }
    for glyph in &raster.glyphs {
        let Some(expected_bytes) = usize::try_from(glyph.width).ok().and_then(|width| {
            usize::try_from(glyph.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        }) else {
            return Err(DisplayListError::InvalidTextRaster { index });
        };
        if !glyph.origin.x.is_finite()
            || !glyph.origin.y.is_finite()
            || glyph.alpha.len() != expected_bytes
        {
            return Err(DisplayListError::InvalidTextRaster { index });
        }
        *aggregate_bytes = aggregate_bytes.saturating_add(expected_bytes);
        if *aggregate_bytes > MAX_GLYPH_RASTER_BYTES {
            return Err(DisplayListError::TextRasterTooLarge {
                index,
                bytes: *aggregate_bytes,
                maximum: MAX_GLYPH_RASTER_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_opacity(opacity: f32, index: usize) -> Result<(), DisplayListError> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(DisplayListError::InvalidOpacity { index })
    }
}

fn validate_primitive_colors(
    primitive: &DisplayPrimitive,
    index: usize,
) -> Result<(), DisplayListError> {
    let valid = |color: UiColor| {
        [color.red, color.green, color.blue, color.alpha]
            .iter()
            .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
    };
    let colors_valid = match primitive {
        DisplayPrimitive::Rect { color, .. }
        | DisplayPrimitive::Border { color, .. }
        | DisplayPrimitive::Text { color, .. }
        | DisplayPrimitive::GlyphRun { color, .. }
        | DisplayPrimitive::FocusIndicator { color, .. }
        | DisplayPrimitive::RoundedRect { color, .. }
        | DisplayPrimitive::Path { color, .. }
        | DisplayPrimitive::Shadow { color, .. } => valid(*color),
        DisplayPrimitive::Mesh { tint, .. } => valid(*tint),
        DisplayPrimitive::Backdrop { descriptor, .. } => {
            valid(descriptor.tint) && valid(descriptor.opaque_fallback)
        }
        DisplayPrimitive::Image { .. }
        | DisplayPrimitive::PushClip { .. }
        | DisplayPrimitive::PopClip { .. }
        | DisplayPrimitive::BeginLayer { .. }
        | DisplayPrimitive::EndLayer { .. } => true,
    };
    if colors_valid {
        Ok(())
    } else {
        Err(DisplayListError::InvalidGeometry { index })
    }
}

fn validate_rect(bounds: UiRect, index: usize) -> Result<(), DisplayListError> {
    let values = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ];
    if values.iter().all(|value| value.is_finite())
        && bounds.size.width >= 0.0
        && bounds.size.height >= 0.0
    {
        Ok(())
    } else {
        Err(DisplayListError::InvalidGeometry { index })
    }
}

fn contains_rect(outer: UiRect, inner: UiRect) -> bool {
    outer.contains(inner.origin)
        && outer.contains(UiPoint {
            x: inner.origin.x + inner.size.width,
            y: inner.origin.y + inner.size.height,
        })
}

fn path_command_is_finite(command: UiPathCommand) -> bool {
    let point_is_finite = |point: UiPoint| point.x.is_finite() && point.y.is_finite();
    match command {
        UiPathCommand::MoveTo(point) | UiPathCommand::LineTo(point) => point_is_finite(point),
        UiPathCommand::QuadraticTo { control, end } => {
            point_is_finite(control) && point_is_finite(end)
        }
        UiPathCommand::CubicTo {
            control_a,
            control_b,
            end,
        } => point_is_finite(control_a) && point_is_finite(control_b) && point_is_finite(end),
        UiPathCommand::Close => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_ui_core::{UiPoint, UiSize};
    use meridian_ui_text::UiGlyphBitmap;

    #[test]
    fn nested_scopes_and_bounded_backdrop_validate() {
        let clip = UiClipId(1);
        let layer = UiLayerId(2);
        let bounds = UiRect::new(UiPoint::default(), UiSize::new(100.0, 80.0));
        let list = DisplayList {
            primitives: vec![
                DisplayPrimitive::PushClip {
                    id: clip,
                    bounds,
                    radii: UiCornerRadii::uniform(10.0),
                },
                DisplayPrimitive::BeginLayer {
                    id: layer,
                    opacity: 0.8,
                },
                DisplayPrimitive::Backdrop {
                    node: UiNodeId::new(7),
                    descriptor: UiBackdropDescriptor {
                        bounds,
                        sample_bounds: bounds,
                        tint: UiColor::surface(),
                        opaque_fallback: UiColor::surface(),
                    },
                },
                DisplayPrimitive::EndLayer { id: layer },
                DisplayPrimitive::PopClip { id: clip },
            ],
        };
        assert_eq!(list.validate(), Ok(()));
    }

    #[test]
    fn high_contrast_and_missing_effect_support_use_the_opaque_fallback() {
        let bounds = UiRect::new(UiPoint::default(), UiSize::new(100.0, 80.0));
        let descriptor = UiBackdropDescriptor {
            bounds,
            sample_bounds: bounds,
            tint: UiColor::surface(),
            opaque_fallback: UiColor::background(),
        };
        assert!(matches!(
            resolve_backdrop(
                descriptor,
                UiContrast::Standard,
                UiEffectCapabilities {
                    backdrop_filtering: true
                }
            ),
            ResolvedBackdrop::Effect(_)
        ));
        for resolution in [
            resolve_backdrop(
                descriptor,
                UiContrast::High,
                UiEffectCapabilities {
                    backdrop_filtering: true,
                },
            ),
            resolve_backdrop(
                descriptor,
                UiContrast::Standard,
                UiEffectCapabilities::default(),
            ),
        ] {
            assert_eq!(
                resolution,
                ResolvedBackdrop::Opaque {
                    bounds,
                    color: UiColor::background(),
                }
            );
        }
    }

    #[test]
    fn mismatched_scope_and_unbounded_backdrop_are_rejected() {
        let bounds = UiRect::new(UiPoint::default(), UiSize::new(100.0, 80.0));
        let mismatched = DisplayList {
            primitives: vec![DisplayPrimitive::PopClip { id: UiClipId(1) }],
        };
        assert_eq!(
            mismatched.validate(),
            Err(DisplayListError::ClipMismatch { index: 0 })
        );
        let unbounded = DisplayList {
            primitives: vec![DisplayPrimitive::Backdrop {
                node: UiNodeId::new(7),
                descriptor: UiBackdropDescriptor {
                    bounds,
                    sample_bounds: UiRect::new(UiPoint::default(), UiSize::new(50.0, 40.0)),
                    tint: UiColor::surface(),
                    opaque_fallback: UiColor::surface(),
                },
            }],
        };
        assert_eq!(
            unbounded.validate(),
            Err(DisplayListError::UnboundedBackdrop { index: 0 })
        );
    }

    #[test]
    fn bounded_builder_rejects_path_complexity_without_mutation() {
        let mut list = DisplayList::default();
        let result = list.try_push(DisplayPrimitive::Path {
            node: UiNodeId::new(9),
            commands: vec![UiPathCommand::Close; MAX_PATH_COMMANDS_PER_PRIMITIVE.saturating_add(1)],
            color: UiColor::text(),
        });
        assert_eq!(
            result,
            Err(DisplayListError::PathTooComplex {
                index: 0,
                count: MAX_PATH_COMMANDS_PER_PRIMITIVE + 1,
                maximum: MAX_PATH_COMMANDS_PER_PRIMITIVE,
            })
        );
        assert!(list.primitives.is_empty());
    }

    #[test]
    fn malformed_and_aggregate_text_rasters_are_rejected() {
        let bounds = UiRect::new(UiPoint::default(), UiSize::new(100.0, 80.0));
        let text = |raster: UiTextRaster| DisplayPrimitive::Text {
            node: UiNodeId::new(7),
            bounds,
            text: "text".to_owned(),
            color: UiColor::text(),
            layout: UiTextLayout {
                line_count: 1,
                glyph_count: 2,
                width: 20.0,
                height: 16.0,
                used_fallback_metrics: false,
            },
            raster,
        };
        let malformed = DisplayList {
            primitives: vec![text(UiTextRaster {
                glyphs: vec![UiGlyphBitmap {
                    origin: UiPoint::default(),
                    width: 2,
                    height: 2,
                    alpha: vec![255; 3],
                }],
                has_unrasterized_glyphs: false,
            })],
        };
        assert_eq!(
            malformed.validate(),
            Err(DisplayListError::InvalidTextRaster { index: 0 })
        );

        let glyph_bytes = MAX_GLYPH_RASTER_BYTES / 2 + 1;
        let raster = || UiTextRaster {
            glyphs: vec![UiGlyphBitmap {
                origin: UiPoint::default(),
                width: u32::try_from(glyph_bytes).expect("test raster fits u32"),
                height: 1,
                alpha: vec![255; glyph_bytes],
            }],
            has_unrasterized_glyphs: false,
        };
        let aggregate = DisplayList {
            primitives: vec![text(raster()), text(raster())],
        };
        assert!(matches!(
            aggregate.validate(),
            Err(DisplayListError::TextRasterTooLarge {
                index: 1,
                maximum: MAX_GLYPH_RASTER_BYTES,
                ..
            })
        ));
    }
}
