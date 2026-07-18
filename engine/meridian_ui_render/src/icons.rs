//! Meridian-owned geometry generated from the pinned reviewed icon subset.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use meridian_ui_core::{IconId, UiPoint, UiRect};

use crate::{UiPathCommand, MAX_PATH_COMMANDS_PER_PRIMITIVE};

const SOURCE_ICON_SIZE: f32 = 24.0;
const SOURCE_STROKE_WIDTH: f32 = 2.0;

mod generated_icons {
    #![allow(clippy::excessive_precision, clippy::unreadable_literal)]

    use meridian_ui_core::{IconId, UiPoint};

    use crate::UiPathCommand;

    include!("generated_icons.rs");
}

/// Renderer-neutral paths for one stable Meridian icon.
#[derive(Clone, Debug, PartialEq)]
pub struct UiIconGeometry {
    pub paths: Vec<Vec<UiPathCommand>>,
    pub stroke_width: f32,
}

/// Rejected icon fitting before malformed geometry reaches a display list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiIconGeometryError {
    InvalidBounds,
    SourcePathTooComplex,
}

impl Display for UiIconGeometryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => formatter.write_str("icon bounds must be finite and positive"),
            Self::SourcePathTooComplex => {
                formatter.write_str("generated icon source exceeds the shared path bound")
            }
        }
    }
}

impl Error for UiIconGeometryError {}

/// Fits generated 24-unit icon paths into a centered square within `bounds`.
///
/// # Errors
///
/// Rejects non-finite or empty bounds and generated data exceeding shared path
/// limits. SVG/parser types never enter this runtime boundary.
pub fn icon_geometry(icon: IconId, bounds: UiRect) -> Result<UiIconGeometry, UiIconGeometryError> {
    if !bounds.origin.x.is_finite()
        || !bounds.origin.y.is_finite()
        || !bounds.size.width.is_finite()
        || !bounds.size.height.is_finite()
        || bounds.size.width <= 0.0
        || bounds.size.height <= 0.0
    {
        return Err(UiIconGeometryError::InvalidBounds);
    }
    let side = bounds.size.width.min(bounds.size.height);
    let scale = side / SOURCE_ICON_SIZE;
    let origin = UiPoint {
        x: bounds.origin.x + (bounds.size.width - side) * 0.5,
        y: bounds.origin.y + (bounds.size.height - side) * 0.5,
    };
    let source_paths = generated_icons::source_icon_paths(icon);
    let mut paths = Vec::with_capacity(source_paths.len());
    for source in source_paths {
        if source.len() > MAX_PATH_COMMANDS_PER_PRIMITIVE {
            return Err(UiIconGeometryError::SourcePathTooComplex);
        }
        paths.push(
            source
                .iter()
                .copied()
                .map(|command| transform_command(command, origin, scale))
                .collect(),
        );
    }
    Ok(UiIconGeometry {
        paths,
        stroke_width: SOURCE_STROKE_WIDTH * scale,
    })
}

fn transform_command(command: UiPathCommand, origin: UiPoint, scale: f32) -> UiPathCommand {
    match command {
        UiPathCommand::MoveTo(point) => UiPathCommand::MoveTo(transform(point, origin, scale)),
        UiPathCommand::LineTo(point) => UiPathCommand::LineTo(transform(point, origin, scale)),
        UiPathCommand::QuadraticTo { control, end } => UiPathCommand::QuadraticTo {
            control: transform(control, origin, scale),
            end: transform(end, origin, scale),
        },
        UiPathCommand::CubicTo {
            control_a,
            control_b,
            end,
        } => UiPathCommand::CubicTo {
            control_a: transform(control_a, origin, scale),
            control_b: transform(control_b, origin, scale),
            end: transform(end, origin, scale),
        },
        UiPathCommand::Close => UiPathCommand::Close,
    }
}

fn transform(point: UiPoint, origin: UiPoint, scale: f32) -> UiPoint {
    UiPoint {
        x: origin.x + point.x * scale,
        y: origin.y + point.y * scale,
    }
}

#[cfg(test)]
mod tests {
    use meridian_ui_core::UiSize;

    use super::*;

    #[test]
    fn every_registered_icon_generates_bounded_runtime_geometry() {
        let bounds = UiRect::new(UiPoint { x: 10.0, y: 20.0 }, UiSize::new(32.0, 24.0));
        for icon in IconId::ALL {
            let geometry = icon_geometry(icon, bounds).expect("registered icon geometry");
            assert!(!geometry.paths.is_empty(), "{icon:?}");
            assert!(geometry.stroke_width.is_finite() && geometry.stroke_width > 0.0);
            assert!(geometry
                .paths
                .iter()
                .all(|path| { !path.is_empty() && path.len() <= MAX_PATH_COMMANDS_PER_PRIMITIVE }));
            for path in geometry.paths {
                for command in path {
                    for point in command_points(command) {
                        assert!(point.x >= 10.0 && point.x <= 42.0, "{icon:?}: {point:?}");
                        assert!(point.y >= 20.0 && point.y <= 44.0, "{icon:?}: {point:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn malformed_bounds_are_rejected_without_geometry() {
        let bounds = UiRect::new(UiPoint::default(), UiSize::new(f32::NAN, 16.0));
        assert_eq!(
            icon_geometry(IconId::Play, bounds),
            Err(UiIconGeometryError::InvalidBounds)
        );
    }

    fn command_points(command: UiPathCommand) -> Vec<UiPoint> {
        match command {
            UiPathCommand::MoveTo(point) | UiPathCommand::LineTo(point) => vec![point],
            UiPathCommand::QuadraticTo { control, end } => vec![control, end],
            UiPathCommand::CubicTo {
                control_a,
                control_b,
                end,
            } => vec![control_a, control_b, end],
            UiPathCommand::Close => Vec::new(),
        }
    }
}
