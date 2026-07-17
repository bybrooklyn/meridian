//! Retained, renderer-independent UI contracts.
//!
//! The data crossing this crate boundary is Meridian-owned.  Platform and
//! renderer adapters consume [`DisplayList`] and [`SemanticTree`] rather than
//! borrowing widget state or exposing their native types here.

use std::collections::{BTreeMap, BTreeSet};

use meridian_core::StableId;

/// Maximum UTF-8 payload accepted by one retained text node.
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
/// Maximum renderer-neutral primitives accepted in one immutable UI frame.
pub const MAX_DISPLAY_PRIMITIVES: usize = 4_096;
/// Worst-case primitive contribution of one retained node in the core emitter.
pub const MAX_PRIMITIVES_PER_RETAINED_NODE: usize = 6;
/// Structural retained-tree limit derived from the immutable frame bound.
pub const MAX_RETAINED_NODES: usize = MAX_DISPLAY_PRIMITIVES / MAX_PRIMITIVES_PER_RETAINED_NODE;

/// Sanitizes untrusted platform scale input to the supported 50-400% interval.
#[must_use]
pub fn sanitized_scale_factor(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.5, 4.0)
    } else {
        1.0
    }
}

/// A stable retained-document identity, suitable for saved UI state and tests.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UiNodeId(StableId);

impl UiNodeId {
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self(StableId::new(value))
    }

    #[must_use]
    pub const fn stable_id(self) -> StableId {
        self.0
    }
}

macro_rules! stable_ui_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(StableId);

        impl $name {
            #[must_use]
            pub const fn new(value: u128) -> Self {
                Self(StableId::new(value))
            }

            #[must_use]
            pub const fn stable_id(self) -> StableId {
                self.0
            }
        }
    };
}

stable_ui_id!(ThemeId, "Stable identity for a Meridian-owned theme.");
stable_ui_id!(TokenId, "Stable identity for a resolved design token.");
stable_ui_id!(
    FocusId,
    "Stable identity for a focus scope or remembered target."
);
stable_ui_id!(CommandId, "Stable identity for a typed UI command.");

/// User-selected information density. It never reduces accessible hit targets.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum UiDensity {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

/// Contrast preference resolved before display-list emission.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum UiContrast {
    #[default]
    Standard,
    High,
}

/// Motion preference carried independently of animation implementation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}

/// Normative Meridian layout families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutMode {
    Flex,
    Grid,
    Overlay,
    Absolute,
    Scroll,
}

/// Typeface purpose. Font-library handles remain private to text adapters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiFontRole {
    Interface,
    Display,
    Monospace,
}

/// A Meridian-owned font request resolved by an audited platform asset adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiFontDescriptor {
    pub role: UiFontRole,
    pub family: &'static str,
}

impl UiFontDescriptor {
    #[must_use]
    pub const fn locked(role: UiFontRole) -> Self {
        let family = match role {
            UiFontRole::Interface => "Mona Sans",
            UiFontRole::Display => "Hubot Sans",
            UiFontRole::Monospace => "JetBrains Mono",
        };
        Self { role, family }
    }
}

/// Stable icon vocabulary. SVGs and parser types remain inside audited adapters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IconId {
    Play,
    Stop,
    Build,
    Search,
    Settings,
    More,
    Close,
    ChevronDown,
    ChevronRight,
    Warning,
    Error,
    Success,
}

/// Locked geometry values shared by application and editor composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGeometryTokens {
    pub spacing_base: f32,
    pub dock_gutter: f32,
    pub border: f32,
    pub radius_compact: f32,
    pub radius_control: f32,
    pub radius_panel: f32,
    pub radius_floating: f32,
    pub application_row: f32,
    pub workspace_row: f32,
    pub status_row: f32,
    pub activity_rail_collapsed: f32,
    pub activity_rail_expanded: f32,
    pub browser_width: f32,
    pub world_inspector_width: f32,
    pub bottom_shelf_peek: f32,
    pub bottom_shelf_expanded: f32,
}

/// Locked timing descriptors; spring behavior remains a later presentation package.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiMotionTokens {
    pub state_transition_min_ms: u16,
    pub state_transition_max_ms: u16,
}

/// Locked semantic colours. Render adapters own colour-space conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiColorTokens {
    pub background: UiColor,
    pub surface: UiColor,
    pub border: UiColor,
    pub primary_text: UiColor,
    pub secondary_text: UiColor,
    pub muted: UiColor,
    pub destructive: UiColor,
    pub destructive_hover: UiColor,
    pub positive: UiColor,
    pub warning: UiColor,
}

/// Resolved design-system contract used to build immutable frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiTheme {
    pub id: ThemeId,
    pub colors: UiColorTokens,
    pub geometry: UiGeometryTokens,
    pub motion: UiMotionTokens,
    pub interface_font: UiFontDescriptor,
    pub display_font: UiFontDescriptor,
    pub monospace_font: UiFontDescriptor,
}

/// Logical-space point; adapters apply display scale at their boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiPoint {
    pub x: f32,
    pub y: f32,
}

/// Logical-space size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiSize {
    pub width: f32,
    pub height: f32,
}

impl UiSize {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn sanitized(self) -> Self {
        let dimension = |value: f32| {
            if value.is_finite() {
                value.max(1.0)
            } else {
                1.0
            }
        };
        Self::new(dimension(self.width), dimension(self.height))
    }
}

/// Logical-space axis-aligned bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiRect {
    pub origin: UiPoint,
    pub size: UiSize,
}

impl UiRect {
    #[must_use]
    pub const fn new(origin: UiPoint, size: UiSize) -> Self {
        Self { origin, size }
    }

    #[must_use]
    pub fn contains(self, point: UiPoint) -> bool {
        let end_x = self.origin.x + self.size.width;
        let end_y = self.origin.y + self.size.height;
        point.x >= self.origin.x && point.x <= end_x && point.y >= self.origin.y && point.y <= end_y
    }
}

/// Normalized RGBA colour owned by the UI contract.
///
/// Named design tokens preserve their authored sRGB channel values. Renderer
/// adapters own any target colour-space conversion; third-party colour types
/// never cross this boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl UiColor {
    #[must_use]
    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    #[must_use]
    pub const fn panel() -> Self {
        Self::surface()
    }

    #[must_use]
    pub const fn foreground() -> Self {
        Self::text()
    }

    #[must_use]
    pub const fn focus() -> Self {
        Self::amber()
    }

    /// Meridian website background token (`#090b0b`).
    #[must_use]
    pub const fn background() -> Self {
        Self::rgba(0.035_294_12, 0.043_137_256, 0.043_137_256, 1.0)
    }

    /// Meridian website surface token (`#121515`).
    #[must_use]
    pub const fn surface() -> Self {
        Self::rgba(0.070_588_24, 0.082_352_94, 0.082_352_94, 1.0)
    }

    /// Meridian website border token (`#292d2c`).
    #[must_use]
    pub const fn border() -> Self {
        Self::rgba(0.160_784_32, 0.176_470_6, 0.172_549_02, 1.0)
    }

    /// Meridian website primary-text token (`#e3e1d8`).
    #[must_use]
    pub const fn text() -> Self {
        Self::rgba(0.890_196_1, 0.882_352_95, 0.847_058_83, 1.0)
    }

    /// Meridian website secondary-text token (`#929790`).
    #[must_use]
    pub const fn secondary_text() -> Self {
        Self::rgba(0.572_549_05, 0.592_156_9, 0.564_705_9, 1.0)
    }

    /// Meridian website muted-text token (`#686e68`).
    #[must_use]
    pub const fn muted_text() -> Self {
        Self::rgba(0.407_843_14, 0.431_372_55, 0.407_843_14, 1.0)
    }

    /// Meridian website destructive token (`#a73732`).
    #[must_use]
    pub const fn red() -> Self {
        Self::rgba(0.654_902, 0.215_686_28, 0.196_078_43, 1.0)
    }

    /// Meridian website destructive-hover token (`#c04b44`).
    #[must_use]
    pub const fn red_hover() -> Self {
        Self::rgba(0.752_941_2, 0.294_117_66, 0.266_666_68, 1.0)
    }

    /// Meridian website positive/grass token (`#8d8961`).
    #[must_use]
    pub const fn grass() -> Self {
        Self::rgba(0.552_941_2, 0.537_254_9, 0.380_392_16, 1.0)
    }

    /// Meridian website warning/emphasis token (`#c0964e`).
    #[must_use]
    pub const fn amber() -> Self {
        Self::rgba(0.752_941_2, 0.588_235_3, 0.305_882_36, 1.0)
    }
}

impl UiTheme {
    /// The normative Meridian dark theme sourced from the public website palette.
    #[must_use]
    pub const fn meridian_dark() -> Self {
        Self {
            id: ThemeId::new(1),
            colors: UiColorTokens {
                background: UiColor::background(),
                surface: UiColor::surface(),
                border: UiColor::border(),
                primary_text: UiColor::text(),
                secondary_text: UiColor::secondary_text(),
                muted: UiColor::muted_text(),
                destructive: UiColor::red(),
                destructive_hover: UiColor::red_hover(),
                positive: UiColor::grass(),
                warning: UiColor::amber(),
            },
            geometry: UiGeometryTokens {
                spacing_base: 4.0,
                dock_gutter: 8.0,
                border: 1.0,
                radius_compact: 4.0,
                radius_control: 6.0,
                radius_panel: 10.0,
                radius_floating: 14.0,
                application_row: 44.0,
                workspace_row: 36.0,
                status_row: 24.0,
                activity_rail_collapsed: 44.0,
                activity_rail_expanded: 160.0,
                browser_width: 264.0,
                world_inspector_width: 344.0,
                bottom_shelf_peek: 32.0,
                bottom_shelf_expanded: 240.0,
            },
            motion: UiMotionTokens {
                state_transition_min_ms: 100,
                state_transition_max_ms: 160,
            },
            interface_font: UiFontDescriptor::locked(UiFontRole::Interface),
            display_font: UiFontDescriptor::locked(UiFontRole::Display),
            monospace_font: UiFontDescriptor::locked(UiFontRole::Monospace),
        }
    }
}

/// Widget behavior is intentionally a small, retained set for the MS-02 seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWidgetKind {
    Panel,
    Label,
    Button,
    IconButton,
    Toggle,
    Progress,
    TextInput,
    Overlay,
}

/// Primary direction for Flex and Scroll layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAxis {
    Horizontal,
    Vertical,
}

/// Layout policy used by the retained document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiLayout {
    Overlay,
    Flex {
        axis: UiAxis,
        gap: f32,
    },
    VerticalStack {
        gap: f32,
    },
    HorizontalStack {
        gap: f32,
    },
    /// Equal-sized cells arranged left-to-right, then top-to-bottom.
    Grid {
        columns: u8,
        gap: f32,
    },
    Absolute,
    Scroll {
        axis: UiAxis,
        offset: f32,
    },
}

impl UiLayout {
    #[must_use]
    pub const fn mode(self) -> LayoutMode {
        match self {
            Self::VerticalStack { .. } | Self::HorizontalStack { .. } | Self::Flex { .. } => {
                LayoutMode::Flex
            }
            Self::Grid { .. } => LayoutMode::Grid,
            Self::Overlay => LayoutMode::Overlay,
            Self::Absolute => LayoutMode::Absolute,
            Self::Scroll { .. } => LayoutMode::Scroll,
        }
    }
}

/// Alignment applied after a child resolves its constraints.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiAlignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

/// Bounded logical constraints independent of a renderer or platform toolkit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiConstraints {
    pub minimum: UiSize,
    pub maximum: Option<UiSize>,
    pub aspect_ratio: Option<f32>,
    pub horizontal_alignment: UiAlignment,
    pub vertical_alignment: UiAlignment,
    pub clip: bool,
}

impl Default for UiConstraints {
    fn default() -> Self {
        Self {
            minimum: UiSize::default(),
            maximum: None,
            aspect_ratio: None,
            horizontal_alignment: UiAlignment::Stretch,
            vertical_alignment: UiAlignment::Stretch,
            clip: false,
        }
    }
}

/// Edge offsets for a child of an Absolute container.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiAbsolutePosition {
    pub left: f32,
    pub top: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Preferred sizing and flexible growth for a retained node.
///
/// A preferred dimension is a bounded starting point, not an absolute pixel
/// requirement: the layout engine scales it down before overflowing a smaller
/// viewport. Remaining space is shared by positive `grow` values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiLayoutHints {
    pub preferred_width: Option<f32>,
    pub preferred_height: Option<f32>,
    pub grow: f32,
}

impl UiLayoutHints {
    /// Returns flexible hints that share remaining axis space equally.
    #[must_use]
    pub const fn flexible() -> Self {
        Self {
            preferred_width: None,
            preferred_height: None,
            grow: 1.0,
        }
    }

    /// Returns a fixed-height hint that does not absorb surplus vertical space.
    #[must_use]
    pub const fn fixed_height(height: f32) -> Self {
        Self {
            preferred_width: None,
            preferred_height: Some(height),
            grow: 0.0,
        }
    }

    /// Returns a fixed-width hint that does not absorb surplus horizontal space.
    #[must_use]
    pub const fn fixed_width(width: f32) -> Self {
        Self {
            preferred_width: Some(width),
            preferred_height: None,
            grow: 0.0,
        }
    }

    /// Returns a preferred two-axis size without flexible growth.
    #[must_use]
    pub const fn fixed_size(width: f32, height: f32) -> Self {
        Self {
            preferred_width: Some(width),
            preferred_height: Some(height),
            grow: 0.0,
        }
    }
}

impl Default for UiLayoutHints {
    fn default() -> Self {
        Self::flexible()
    }
}

/// Public semantic role independent of a platform accessibility API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticRole {
    Group,
    Status,
    Button,
    ToggleButton,
    ProgressIndicator,
    TextInput,
}

/// Named semantics and a typed action token declared by a UI node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSemantics {
    pub role: SemanticRole,
    pub name: String,
    pub action: Option<String>,
    pub value: Option<String>,
}

impl UiSemantics {
    #[must_use]
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Group,
            name: name.into(),
            action: None,
            value: None,
        }
    }

    #[must_use]
    pub fn status(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Status,
            name: name.into(),
            action: None,
            value: None,
        }
    }

    #[must_use]
    pub fn button(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Button,
            name: name.into(),
            action: Some(action.into()),
            value: None,
        }
    }

    #[must_use]
    pub fn text_input(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::TextInput,
            name: name.into(),
            action: None,
            value: None,
        }
    }

    #[must_use]
    pub fn toggle(name: impl Into<String>, action: impl Into<String>, value: bool) -> Self {
        Self {
            role: SemanticRole::ToggleButton,
            name: name.into(),
            action: Some(action.into()),
            value: Some(if value { "on" } else { "off" }.to_owned()),
        }
    }

    #[must_use]
    pub fn progress(name: impl Into<String>, value: u8) -> Self {
        Self {
            role: SemanticRole::ProgressIndicator,
            name: name.into(),
            action: None,
            value: Some(format!("{}%", value.min(100))),
        }
    }
}

/// Rendering style, expressed only in Meridian-owned values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStyle {
    pub background: Option<UiColor>,
    pub border: Option<UiBorder>,
    pub foreground: UiColor,
    pub padding: f32,
    pub font_size: f32,
}

/// A bounded rectangular stroke drawn around a retained node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiBorder {
    pub color: UiColor,
    pub width: u8,
}

/// Policy applied to one retained text-input node.
///
/// Password values stay in the private runtime state: they are masked in the
/// display list and never emitted through semantic or clipboard output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTextInputOptions {
    pub password: bool,
}

impl UiStyle {
    #[must_use]
    pub const fn panel() -> Self {
        Self {
            background: Some(UiColor::panel()),
            border: None,
            foreground: UiColor::foreground(),
            padding: 12.0,
            font_size: 16.0,
        }
    }

    #[must_use]
    pub const fn text() -> Self {
        Self {
            background: None,
            border: None,
            foreground: UiColor::foreground(),
            padding: 6.0,
            font_size: 16.0,
        }
    }

    /// A transparent structural group that does not create a visual surface.
    #[must_use]
    pub const fn transparent() -> Self {
        Self {
            background: None,
            border: None,
            foreground: UiColor::foreground(),
            padding: 0.0,
            font_size: 16.0,
        }
    }

    /// The application canvas used behind a full native workspace.
    #[must_use]
    pub const fn canvas() -> Self {
        Self {
            background: Some(UiColor::background()),
            border: None,
            foreground: UiColor::foreground(),
            padding: 24.0,
            font_size: 16.0,
        }
    }

    /// A raised application surface with a restrained border.
    #[must_use]
    pub const fn surface() -> Self {
        Self {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::border(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 14.0,
            font_size: 16.0,
        }
    }

    /// A subtly brighter surface for headers and currently important regions.
    #[must_use]
    pub const fn elevated_surface() -> Self {
        Self {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::grass(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 16.0,
            font_size: 16.0,
        }
    }

    /// Large, high-contrast display text.
    #[must_use]
    pub const fn heading() -> Self {
        Self {
            background: None,
            border: None,
            foreground: UiColor::text(),
            padding: 0.0,
            font_size: 28.0,
        }
    }

    /// Compact section-label text.
    #[must_use]
    pub const fn section_heading() -> Self {
        Self {
            background: None,
            border: None,
            foreground: UiColor::text(),
            padding: 0.0,
            font_size: 16.0,
        }
    }

    /// Supporting text that does not compete with the primary action.
    #[must_use]
    pub const fn muted_text() -> Self {
        Self {
            background: None,
            border: None,
            foreground: UiColor::muted_text(),
            padding: 0.0,
            font_size: 13.0,
        }
    }

    /// The primary action treatment used once per decision group.
    #[must_use]
    pub const fn primary_action() -> Self {
        Self {
            background: Some(UiColor::red()),
            border: Some(UiBorder {
                color: UiColor::red_hover(),
                width: 1,
            }),
            foreground: UiColor::text(),
            padding: 12.0,
            font_size: 16.0,
        }
    }

    /// A secondary action with the same keyboard semantics as a primary action.
    #[must_use]
    pub const fn secondary_action() -> Self {
        Self {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::border(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 10.0,
            font_size: 14.0,
        }
    }

    /// A dense but still focusable action used inside bounded tool panels.
    #[must_use]
    pub const fn compact_action() -> Self {
        Self {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::border(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 3.0,
            font_size: 11.0,
        }
    }

    /// A clearly editable field surface.
    #[must_use]
    pub const fn text_field() -> Self {
        Self {
            background: Some(UiColor::background()),
            border: Some(UiBorder {
                color: UiColor::border(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 12.0,
            font_size: 16.0,
        }
    }

    /// A compact numeric or short-token field for a dense inspector row.
    #[must_use]
    pub const fn compact_text_field() -> Self {
        Self {
            background: Some(UiColor::background()),
            border: Some(UiBorder {
                color: UiColor::border(),
                width: 1,
            }),
            foreground: UiColor::foreground(),
            padding: 4.0,
            font_size: 14.0,
        }
    }
}

/// One retained node.  Children are ordered for traversal, focus, and layout.
#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    pub id: UiNodeId,
    pub kind: UiWidgetKind,
    pub layout: UiLayout,
    pub style: UiStyle,
    pub layout_hints: UiLayoutHints,
    pub constraints: UiConstraints,
    pub absolute_position: Option<UiAbsolutePosition>,
    pub icon: Option<IconId>,
    pub semantics: UiSemantics,
    pub text: Option<String>,
    pub text_input: Option<UiTextInputOptions>,
    pub focusable: bool,
    pub children: Vec<UiNodeId>,
}

impl UiNode {
    #[must_use]
    pub fn container(
        id: UiNodeId,
        name: impl Into<String>,
        layout: UiLayout,
        children: Vec<UiNodeId>,
    ) -> Self {
        Self {
            id,
            kind: UiWidgetKind::Panel,
            layout,
            style: UiStyle::panel(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::group(name),
            text: None,
            text_input: None,
            focusable: false,
            children,
        }
    }

    #[must_use]
    pub fn label(id: UiNodeId, name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id,
            kind: UiWidgetKind::Label,
            layout: UiLayout::Overlay,
            style: UiStyle::text(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::status(name),
            text: Some(text.into()),
            text_input: None,
            focusable: false,
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn button(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id,
            kind: UiWidgetKind::Button,
            layout: UiLayout::Overlay,
            style: UiStyle::secondary_action(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::button(name, action),
            text: Some(text.into()),
            text_input: None,
            focusable: true,
            children: Vec::new(),
        }
    }

    /// Creates a focusable retained text input with Meridian-owned editing state.
    ///
    /// Password inputs always start empty so a retained document cannot carry a
    /// password value. Their value can arrive only through a bounded text-input
    /// event at the runtime boundary.
    #[must_use]
    pub fn text_input(
        id: UiNodeId,
        name: impl Into<String>,
        initial_value: impl Into<String>,
        options: UiTextInputOptions,
    ) -> Self {
        Self {
            id,
            kind: UiWidgetKind::TextInput,
            layout: UiLayout::Overlay,
            style: UiStyle::text_field(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::text_input(name),
            text: Some(if options.password {
                String::new()
            } else {
                initial_value.into()
            }),
            text_input: Some(options),
            focusable: true,
            children: Vec::new(),
        }
    }

    /// Replaces this node's Meridian-owned visual treatment.
    #[must_use]
    pub const fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }

    /// Replaces this node's preferred size and flexible-growth behavior.
    #[must_use]
    pub const fn with_layout_hints(mut self, layout_hints: UiLayoutHints) -> Self {
        self.layout_hints = layout_hints;
        self
    }

    /// Applies bounded sizing, alignment, aspect, and clipping rules.
    #[must_use]
    pub const fn with_constraints(mut self, constraints: UiConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Positions this node inside an Absolute parent.
    #[must_use]
    pub const fn with_absolute_position(mut self, position: UiAbsolutePosition) -> Self {
        self.absolute_position = Some(position);
        self
    }

    /// Creates a named icon button without exposing an SVG implementation type.
    #[must_use]
    pub fn icon_button(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        icon: IconId,
    ) -> Self {
        let mut node = Self::button(id, name, action, "");
        node.kind = UiWidgetKind::IconButton;
        node.icon = Some(icon);
        node.text = None;
        node
    }

    /// Creates a typed two-state control. Interaction behavior arrives in WP-UI-003.
    #[must_use]
    pub fn toggle(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        value: bool,
    ) -> Self {
        Self {
            id,
            kind: UiWidgetKind::Toggle,
            layout: UiLayout::Overlay,
            style: UiStyle::secondary_action(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::toggle(name, action, value),
            text: Some(if value { "On" } else { "Off" }.to_owned()),
            text_input: None,
            focusable: true,
            children: Vec::new(),
        }
    }

    /// Creates a read-only bounded progress indicator.
    #[must_use]
    pub fn progress(id: UiNodeId, name: impl Into<String>, value: u8) -> Self {
        let value = value.min(100);
        Self {
            id,
            kind: UiWidgetKind::Progress,
            layout: UiLayout::Overlay,
            style: UiStyle::surface(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            semantics: UiSemantics::progress(name, value),
            text: Some(format!("{value}%")),
            text_input: None,
            focusable: false,
            children: Vec::new(),
        }
    }
}

/// Invalid retained documents are rejected before rendering or accessibility output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiDocumentError {
    TooManyNodes {
        count: usize,
        maximum: usize,
    },
    MissingRoot(UiNodeId),
    DuplicateNode(UiNodeId),
    DuplicateChild(UiNodeId),
    MissingChild {
        parent: UiNodeId,
        child: UiNodeId,
    },
    MultipleParents(UiNodeId),
    Cycle(UiNodeId),
    Unreachable(UiNodeId),
    UnnamedFocusable(UiNodeId),
    MissingButtonAction(UiNodeId),
    TextInputNotFocusable(UiNodeId),
    MissingTextInputOptions(UiNodeId),
    UnexpectedTextInputOptions(UiNodeId),
    PasswordInitialValue(UiNodeId),
    TextTooLong {
        node: UiNodeId,
        bytes: usize,
        maximum: usize,
    },
    NonFiniteConstraint(UiNodeId),
    MinimumExceedsMaximum(UiNodeId),
    InvalidAspectRatio(UiNodeId),
    InvalidLayoutValue(UiNodeId),
    SemanticTextTooLong {
        node: UiNodeId,
        field: UiSemanticField,
        bytes: usize,
        maximum: usize,
    },
}

/// Bounded semantic string field reported by document validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSemanticField {
    Name,
    Action,
    Value,
}

/// Validated retained UI document.
#[derive(Clone, Debug, PartialEq)]
pub struct UiDocument {
    root: UiNodeId,
    nodes: BTreeMap<UiNodeId, UiNode>,
    parents: BTreeMap<UiNodeId, UiNodeId>,
}

impl UiDocument {
    /// Builds and validates the whole tree before any frame can observe it.
    ///
    /// # Errors
    ///
    /// Returns an error when identity, reachability, focus semantics, or action
    /// declarations would make the retained tree ambiguous or inaccessible.
    pub fn new(root: UiNodeId, nodes: Vec<UiNode>) -> Result<Self, UiDocumentError> {
        if nodes.len() > MAX_RETAINED_NODES {
            return Err(UiDocumentError::TooManyNodes {
                count: nodes.len(),
                maximum: MAX_RETAINED_NODES,
            });
        }
        let mut by_id = BTreeMap::new();
        for node in nodes {
            let id = node.id;
            if by_id.insert(id, node).is_some() {
                return Err(UiDocumentError::DuplicateNode(id));
            }
        }
        if !by_id.contains_key(&root) {
            return Err(UiDocumentError::MissingRoot(root));
        }

        let mut parents = BTreeMap::new();
        for (id, node) in &by_id {
            Self::validate_geometry(*id, node)?;
            Self::validate_control(*id, node)?;
            Self::register_children(*id, node, &by_id, &mut parents)?;
        }

        let mut visited = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        Self::validate_reachable(root, &by_id, &mut visited, &mut visiting)?;
        if let Some(id) = by_id.keys().find(|id| !visited.contains(id)) {
            return Err(UiDocumentError::Unreachable(*id));
        }

        Ok(Self {
            root,
            nodes: by_id,
            parents,
        })
    }

    fn validate_geometry(id: UiNodeId, node: &UiNode) -> Result<(), UiDocumentError> {
        let layout_is_valid = match node.layout {
            UiLayout::Overlay | UiLayout::Absolute => true,
            UiLayout::Flex { gap, .. }
            | UiLayout::VerticalStack { gap }
            | UiLayout::HorizontalStack { gap } => gap.is_finite() && gap >= 0.0,
            UiLayout::Grid { columns, gap } => columns > 0 && gap.is_finite() && gap >= 0.0,
            UiLayout::Scroll { offset, .. } => offset.is_finite() && offset >= 0.0,
        };
        let values = [
            node.layout_hints.preferred_width.unwrap_or(0.0),
            node.layout_hints.preferred_height.unwrap_or(0.0),
            node.layout_hints.grow,
            node.style.padding,
            node.style.font_size,
        ];
        if !layout_is_valid
            || values
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(UiDocumentError::InvalidLayoutValue(id));
        }
        let constraint_values = [
            node.constraints.minimum.width,
            node.constraints.minimum.height,
            node.constraints.maximum.map_or(0.0, |size| size.width),
            node.constraints.maximum.map_or(0.0, |size| size.height),
            node.absolute_position.map_or(0.0, |position| position.left),
            node.absolute_position.map_or(0.0, |position| position.top),
            node.absolute_position
                .and_then(|position| position.width)
                .unwrap_or(0.0),
            node.absolute_position
                .and_then(|position| position.height)
                .unwrap_or(0.0),
        ];
        if constraint_values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(UiDocumentError::NonFiniteConstraint(id));
        }
        if node.constraints.maximum.is_some_and(|maximum| {
            node.constraints.minimum.width > maximum.width
                || node.constraints.minimum.height > maximum.height
        }) {
            return Err(UiDocumentError::MinimumExceedsMaximum(id));
        }
        if node
            .constraints
            .aspect_ratio
            .is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
        {
            return Err(UiDocumentError::InvalidAspectRatio(id));
        }
        Ok(())
    }

    fn validate_control(id: UiNodeId, node: &UiNode) -> Result<(), UiDocumentError> {
        for (field, value) in [
            (UiSemanticField::Name, Some(node.semantics.name.as_str())),
            (UiSemanticField::Action, node.semantics.action.as_deref()),
            (UiSemanticField::Value, node.semantics.value.as_deref()),
        ] {
            if let Some(value) = value {
                if value.len() > MAX_TEXT_BYTES {
                    return Err(UiDocumentError::SemanticTextTooLong {
                        node: id,
                        field,
                        bytes: value.len(),
                        maximum: MAX_TEXT_BYTES,
                    });
                }
            }
        }
        if node.focusable && node.semantics.name.trim().is_empty() {
            return Err(UiDocumentError::UnnamedFocusable(id));
        }
        if matches!(
            node.kind,
            UiWidgetKind::Button | UiWidgetKind::IconButton | UiWidgetKind::Toggle
        ) && node.semantics.action.is_none()
        {
            return Err(UiDocumentError::MissingButtonAction(id));
        }
        if node.kind == UiWidgetKind::TextInput && !node.focusable {
            return Err(UiDocumentError::TextInputNotFocusable(id));
        }
        if node.kind == UiWidgetKind::TextInput && node.text_input.is_none() {
            return Err(UiDocumentError::MissingTextInputOptions(id));
        }
        if node.kind != UiWidgetKind::TextInput && node.text_input.is_some() {
            return Err(UiDocumentError::UnexpectedTextInputOptions(id));
        }
        if node.text_input.is_some_and(|options| options.password)
            && node.text.as_ref().is_some_and(|text| !text.is_empty())
        {
            return Err(UiDocumentError::PasswordInitialValue(id));
        }
        if let Some(text) = &node.text {
            if text.len() > MAX_TEXT_BYTES {
                return Err(UiDocumentError::TextTooLong {
                    node: id,
                    bytes: text.len(),
                    maximum: MAX_TEXT_BYTES,
                });
            }
        }
        Ok(())
    }

    fn register_children(
        id: UiNodeId,
        node: &UiNode,
        nodes: &BTreeMap<UiNodeId, UiNode>,
        parents: &mut BTreeMap<UiNodeId, UiNodeId>,
    ) -> Result<(), UiDocumentError> {
        let mut children = BTreeSet::new();
        for child in &node.children {
            if !children.insert(*child) {
                return Err(UiDocumentError::DuplicateChild(*child));
            }
            if !nodes.contains_key(child) {
                return Err(UiDocumentError::MissingChild {
                    parent: id,
                    child: *child,
                });
            }
            if parents.insert(*child, id).is_some() {
                return Err(UiDocumentError::MultipleParents(*child));
            }
        }
        Ok(())
    }

    fn validate_reachable(
        id: UiNodeId,
        nodes: &BTreeMap<UiNodeId, UiNode>,
        visited: &mut BTreeSet<UiNodeId>,
        visiting: &mut BTreeSet<UiNodeId>,
    ) -> Result<(), UiDocumentError> {
        if !visiting.insert(id) {
            return Err(UiDocumentError::Cycle(id));
        }
        let node = nodes.get(&id).ok_or(UiDocumentError::MissingRoot(id))?;
        for child in &node.children {
            if !visited.contains(child) {
                Self::validate_reachable(*child, nodes, visited, visiting)?;
            }
        }
        visiting.remove(&id);
        visited.insert(id);
        Ok(())
    }

    #[must_use]
    pub const fn root(&self) -> UiNodeId {
        self.root
    }

    #[must_use]
    pub fn node(&self, id: UiNodeId) -> Option<&UiNode> {
        self.nodes.get(&id)
    }

    /// Iterates validated nodes in stable identity order.
    #[must_use]
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &UiNode> {
        self.nodes.values()
    }

    /// Computes an identity-stable incremental reconciliation summary.
    #[must_use]
    pub fn delta_to(&self, next: &Self) -> UiDocumentDelta {
        let retained = self
            .nodes
            .keys()
            .filter(|id| next.nodes.contains_key(id))
            .copied()
            .collect();
        let inserted = next
            .nodes
            .keys()
            .filter(|id| !self.nodes.contains_key(id))
            .copied()
            .collect();
        let removed = self
            .nodes
            .keys()
            .filter(|id| !next.nodes.contains_key(id))
            .copied()
            .collect();
        let updated = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                next.nodes
                    .get(id)
                    .is_some_and(|next_node| next_node != node)
                    .then_some(*id)
            })
            .collect();
        UiDocumentDelta {
            retained,
            inserted,
            removed,
            updated,
        }
    }

    #[must_use]
    pub fn route_to(&self, target: UiNodeId) -> Option<Vec<UiNodeId>> {
        self.nodes.get(&target)?;
        let mut route = vec![target];
        let mut current = target;
        while let Some(parent) = self.parents.get(&current) {
            route.push(*parent);
            current = *parent;
        }
        route.reverse();
        Some(route)
    }

    #[must_use]
    pub fn focus_order(&self) -> Vec<UiNodeId> {
        let mut order = Vec::new();
        self.collect_focus_order(self.root, &mut order);
        order
    }

    fn collect_focus_order(&self, id: UiNodeId, order: &mut Vec<UiNodeId>) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        if node.focusable {
            order.push(id);
        }
        for child in &node.children {
            self.collect_focus_order(*child, order);
        }
    }
}

/// Identity-stable changes between two accepted immutable documents.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiDocumentDelta {
    pub retained: Vec<UiNodeId>,
    pub inserted: Vec<UiNodeId>,
    pub removed: Vec<UiNodeId>,
    pub updated: Vec<UiNodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_theme_matches_registered_palette_geometry_and_typography() {
        let theme = UiTheme::meridian_dark();
        let assert_token = |actual: f32, expected: f32| {
            assert!((actual - expected).abs() < f32::EPSILON);
        };
        assert_eq!(theme.colors.background, UiColor::background());
        assert_eq!(theme.colors.surface, UiColor::surface());
        assert_eq!(theme.colors.destructive, UiColor::red());
        assert_eq!(theme.colors.warning, UiColor::amber());
        assert_token(theme.geometry.spacing_base, 4.0);
        assert_token(theme.geometry.dock_gutter, 8.0);
        assert_token(theme.geometry.radius_panel, 10.0);
        assert_token(theme.geometry.application_row, 44.0);
        assert_token(theme.geometry.workspace_row, 36.0);
        assert_token(theme.geometry.activity_rail_expanded, 160.0);
        assert_token(theme.geometry.browser_width, 264.0);
        assert_token(theme.geometry.world_inspector_width, 344.0);
        assert_token(theme.geometry.bottom_shelf_expanded, 240.0);
        assert_eq!(theme.motion.state_transition_min_ms, 100);
        assert_eq!(theme.motion.state_transition_max_ms, 160);
        assert_eq!(theme.interface_font.family, "Mona Sans");
        assert_eq!(theme.display_font.family, "Hubot Sans");
        assert_eq!(theme.monospace_font.family, "JetBrains Mono");
    }

    #[test]
    fn document_delta_uses_stable_identity_instead_of_row_position() {
        let root = UiNodeId::new(1);
        let retained = UiNodeId::new(2);
        let removed = UiNodeId::new(3);
        let inserted = UiNodeId::new(4);
        let before = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Before",
                    UiLayout::Flex {
                        axis: UiAxis::Vertical,
                        gap: 4.0,
                    },
                    vec![retained, removed],
                ),
                UiNode::label(retained, "Retained", "Before"),
                UiNode::label(removed, "Removed", "Removed"),
            ],
        )
        .expect("before document");
        let after = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "After",
                    UiLayout::Flex {
                        axis: UiAxis::Vertical,
                        gap: 4.0,
                    },
                    vec![inserted, retained],
                ),
                UiNode::label(inserted, "Inserted", "Inserted"),
                UiNode::label(retained, "Retained", "After"),
            ],
        )
        .expect("after document");

        let delta = before.delta_to(&after);
        assert_eq!(delta.retained, vec![root, retained]);
        assert_eq!(delta.inserted, vec![inserted]);
        assert_eq!(delta.removed, vec![removed]);
        assert_eq!(delta.updated, vec![root, retained]);
    }

    #[test]
    fn invalid_constraints_and_tree_cycles_are_rejected_before_commit() {
        let root = UiNodeId::new(1);
        let child = UiNodeId::new(2);
        let constrained =
            UiNode::label(root, "Invalid", "Invalid").with_constraints(UiConstraints {
                minimum: UiSize::new(200.0, 100.0),
                maximum: Some(UiSize::new(100.0, 80.0)),
                ..UiConstraints::default()
            });
        assert_eq!(
            UiDocument::new(root, vec![constrained]),
            Err(UiDocumentError::MinimumExceedsMaximum(root))
        );

        let cycle = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Root", UiLayout::Overlay, vec![child]),
                UiNode::container(child, "Child", UiLayout::Overlay, vec![root]),
            ],
        );
        assert_eq!(cycle, Err(UiDocumentError::Cycle(root)));
    }

    #[test]
    fn basic_controls_have_named_typed_semantics() {
        let icon = UiNode::icon_button(UiNodeId::new(1), "Build", "build.start", IconId::Build);
        let toggle = UiNode::toggle(UiNodeId::new(2), "Snap", "model.snap", true);
        let progress = UiNode::progress(UiNodeId::new(3), "Build progress", 120);
        assert_eq!(icon.kind, UiWidgetKind::IconButton);
        assert_eq!(icon.icon, Some(IconId::Build));
        assert_eq!(icon.text, None);
        assert_eq!(icon.semantics.action.as_deref(), Some("build.start"));
        assert_eq!(toggle.semantics.value.as_deref(), Some("on"));
        assert_eq!(progress.semantics.value.as_deref(), Some("100%"));
    }

    #[test]
    fn retained_tree_limit_is_derived_from_the_frame_primitive_bound() {
        assert_eq!(MAX_RETAINED_NODES * MAX_PRIMITIVES_PER_RETAINED_NODE, 4_092);
        let nodes = vec![
            UiNode::label(UiNodeId::new(1), "bounded node", "bounded");
            MAX_RETAINED_NODES + 1
        ];
        assert_eq!(
            UiDocument::new(UiNodeId::new(1), nodes),
            Err(UiDocumentError::TooManyNodes {
                count: MAX_RETAINED_NODES + 1,
                maximum: MAX_RETAINED_NODES,
            })
        );
    }
}
