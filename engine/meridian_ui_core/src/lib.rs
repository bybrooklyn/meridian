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
/// Maximum normalized platform events accepted at one immutable frame boundary.
pub const MAX_FRAME_EVENTS: usize = MAX_RETAINED_NODES * 8;
/// Structural item-count limit for one virtualized collection contract.
pub const MAX_VIRTUAL_ITEMS: usize = u16::MAX as usize;
/// Maximum accepted drag/drop kinds on one retained target.
pub const MAX_DROP_KINDS: usize = 16;
/// Complete built-in drop-operation vocabulary accepted by one target.
pub const MAX_DROP_OPERATIONS: usize = 3;

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
stable_ui_id!(
    UiInputDeviceId,
    "Stable process-local identity for one normalized input device."
);
stable_ui_id!(
    UiDragItemId,
    "Stable identity carried by a typed drag proposal."
);

/// Meridian-owned device family; platform handles never cross this boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiInputDeviceKind {
    Mouse,
    Trackpad,
    Touch,
    Pen,
    Keyboard,
    Controller,
    Assistive,
}

/// Pointer buttons after platform normalization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiPointerButton {
    Primary,
    Secondary,
    Middle,
    Auxiliary(u8),
}

/// Complete pointer lifecycle used for capture and release-based activation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiPointerPhase {
    Move,
    Press,
    Release,
    Cancel,
}

/// Platform-neutral pointer event in logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPointerEvent {
    pub device: UiInputDeviceId,
    pub kind: UiInputDeviceKind,
    pub phase: UiPointerPhase,
    pub position: UiPoint,
    pub button: Option<UiPointerButton>,
}

/// Unit carried by a normalized scroll delta. Pixel input is never rounded.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiScrollUnit {
    Pixels,
    Lines,
}

/// Scroll gesture lifecycle, keeping OS momentum distinct from direct input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiScrollPhase {
    Begin,
    Update,
    Momentum,
    End,
    Cancel,
}

/// Two-axis scroll delta before an axis-specific scroll container consumes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScrollDelta {
    pub x: f32,
    pub y: f32,
    pub unit: UiScrollUnit,
}

/// Platform-neutral scroll event with explicit gesture identity and phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScrollEvent {
    pub device: UiInputDeviceId,
    pub kind: UiInputDeviceKind,
    pub phase: UiScrollPhase,
    pub position: UiPoint,
    pub delta: UiScrollDelta,
}

/// Typed drag families shared by pointer and keyboard alternatives.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiDragKind {
    Asset,
    Entity,
    Panel,
    Text,
    File,
    Command,
    Domain(u16),
}

/// Host operation negotiated by a typed drop target.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiDropOperation {
    Move,
    Copy,
    Link,
}

/// Authority-free drag data. A drop remains a proposal for a typed host command.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UiDragPayload {
    pub kind: UiDragKind,
    pub item: UiDragItemId,
    pub operation: UiDropOperation,
}

/// Bounded text validation that does not execute a caller-supplied expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextValidation {
    NonEmpty,
    Integer,
    Decimal,
    MaximumGraphemes(u16),
}

/// Stable collection-navigation actions shared by trees, tables, and lists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiCollectionNavigation {
    Previous,
    Next,
    Home,
    End,
    PageBackward,
    PageForward,
}

/// Selection cursor that preserves identity when filtering hides the row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiCollectionCursor {
    pub selected: Option<UiNodeId>,
}

impl UiCollectionCursor {
    /// Moves through currently visible stable identities without owning rows.
    pub fn navigate(
        &mut self,
        visible: &[UiNodeId],
        page_size: usize,
        navigation: UiCollectionNavigation,
    ) -> Option<UiNodeId> {
        if visible.is_empty() {
            return self.selected;
        }
        let current = self
            .selected
            .and_then(|selected| visible.iter().position(|id| *id == selected));
        let last = visible.len() - 1;
        let page_size = page_size.max(1);
        let index = match navigation {
            UiCollectionNavigation::Home => 0,
            UiCollectionNavigation::End => last,
            UiCollectionNavigation::Previous => current.unwrap_or(1).saturating_sub(1),
            UiCollectionNavigation::Next => current.map_or(0, |index| (index + 1).min(last)),
            UiCollectionNavigation::PageBackward => {
                current.unwrap_or(page_size).saturating_sub(page_size)
            }
            UiCollectionNavigation::PageForward => {
                current.map_or(0, |index| index.saturating_add(page_size).min(last))
            }
        };
        self.selected = Some(visible[index]);
        self.selected
    }
}

/// Realized half-open range for a virtualized collection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiVirtualRange {
    pub start: usize,
    pub end: usize,
}

/// Rejected virtualization request before row realization or allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiVirtualRangeError {
    TooManyItems { count: usize, maximum: usize },
    InvalidGeometry,
    OverscanTooLarge { count: usize, maximum: usize },
    RealizedRangeTooLarge { count: usize, maximum: usize },
}

/// Calculates only the visible row range; it never allocates the full collection.
///
/// # Errors
///
/// Rejects item counts above the structural bound, non-finite or negative
/// geometry, zero item extent, overscan above the retained-node limit, and a
/// viewport that would realize more rows than one retained frame can own.
pub fn virtual_range(
    item_count: usize,
    item_extent: f32,
    viewport_extent: f32,
    offset: f32,
    overscan: usize,
) -> Result<UiVirtualRange, UiVirtualRangeError> {
    if item_count > MAX_VIRTUAL_ITEMS {
        return Err(UiVirtualRangeError::TooManyItems {
            count: item_count,
            maximum: MAX_VIRTUAL_ITEMS,
        });
    }
    if overscan > MAX_RETAINED_NODES {
        return Err(UiVirtualRangeError::OverscanTooLarge {
            count: overscan,
            maximum: MAX_RETAINED_NODES,
        });
    }
    if !item_extent.is_finite()
        || item_extent <= 0.0
        || !viewport_extent.is_finite()
        || viewport_extent < 0.0
        || !offset.is_finite()
        || offset < 0.0
    {
        return Err(UiVirtualRangeError::InvalidGeometry);
    }
    if item_count == 0 {
        return Ok(UiVirtualRange::default());
    }
    let first = bounded_float_to_usize((offset / item_extent).floor(), item_count);
    let visible_count = bounded_float_to_usize((viewport_extent / item_extent).ceil(), item_count)
        .saturating_add(1);
    let range = UiVirtualRange {
        start: first.saturating_sub(overscan),
        end: first
            .saturating_add(visible_count)
            .saturating_add(overscan)
            .min(item_count),
    };
    let realized = range.end.saturating_sub(range.start);
    if realized > MAX_RETAINED_NODES {
        return Err(UiVirtualRangeError::RealizedRangeTooLarge {
            count: realized,
            maximum: MAX_RETAINED_NODES,
        });
    }
    Ok(range)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn bounded_float_to_usize(value: f32, maximum: usize) -> usize {
    // Geometry is already finite and non-negative. Rust's saturating float cast
    // plus the structural collection bound prevents an unchecked allocation.
    (value as usize).min(maximum)
}

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

/// Locked state-transition timing descriptors consumed by the presentation runtime.
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

/// Retained widget families shared by runtime and professional editor composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWidgetKind {
    Panel,
    Label,
    Button,
    IconButton,
    Toggle,
    Progress,
    TextInput,
    SearchInput,
    ComboBox,
    ComboOption,
    Overlay,
    MenuBar,
    Menu,
    ContextMenu,
    MenuItem,
    Tooltip,
    Toast,
    Tabs,
    Tab,
    Tree,
    TreeItem,
    Table,
    TableRow,
    TableCell,
    PropertyGrid,
    VirtualList,
    ListItem,
    Timeline,
    Splitter,
    CommandPalette,
    Graph,
    Canvas,
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
    SearchBox,
    ComboBox,
    Option,
    MenuBar,
    Menu,
    MenuItem,
    Tooltip,
    LiveRegion,
    TabList,
    Tab,
    Tree,
    TreeItem,
    Table,
    Row,
    Cell,
    PropertyGrid,
    List,
    ListItem,
    Timeline,
    Splitter,
    Dialog,
    Graph,
    Canvas,
}

/// Interaction state projected to semantics without platform-native values.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiControlState {
    pub disabled: bool,
    pub selected: bool,
    pub expanded: bool,
    pub invalid: bool,
}

/// Named semantics and a typed action token declared by a UI node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSemantics {
    pub role: SemanticRole,
    pub name: String,
    pub action: Option<String>,
    pub value: Option<String>,
    pub state: UiControlState,
}

impl UiSemantics {
    #[must_use]
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Group,
            name: name.into(),
            action: None,
            value: None,
            state: UiControlState::default(),
        }
    }

    #[must_use]
    pub fn status(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Status,
            name: name.into(),
            action: None,
            value: None,
            state: UiControlState::default(),
        }
    }

    #[must_use]
    pub fn button(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Button,
            name: name.into(),
            action: Some(action.into()),
            value: None,
            state: UiControlState::default(),
        }
    }

    #[must_use]
    pub fn text_input(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::TextInput,
            name: name.into(),
            action: None,
            value: None,
            state: UiControlState::default(),
        }
    }

    #[must_use]
    pub fn toggle(name: impl Into<String>, action: impl Into<String>, value: bool) -> Self {
        Self {
            role: SemanticRole::ToggleButton,
            name: name.into(),
            action: Some(action.into()),
            value: Some(if value { "on" } else { "off" }.to_owned()),
            state: UiControlState {
                selected: value,
                ..UiControlState::default()
            },
        }
    }

    #[must_use]
    pub fn progress(name: impl Into<String>, value: u8) -> Self {
        Self {
            role: SemanticRole::ProgressIndicator,
            name: name.into(),
            action: None,
            value: Some(format!("{}%", value.min(100))),
            state: UiControlState::default(),
        }
    }

    #[must_use]
    pub fn professional(name: impl Into<String>, role: SemanticRole) -> Self {
        Self {
            role,
            name: name.into(),
            action: None,
            value: None,
            state: UiControlState::default(),
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
    pub text_validation: Option<UiTextValidation>,
    pub drag_source: Option<UiDragKind>,
    pub drop_accepts: Vec<UiDragKind>,
    pub drop_operations: Vec<UiDropOperation>,
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
            focusable: true,
            children: Vec::new(),
        }
    }

    /// Creates a retained search field with ordinary non-secret editing behavior.
    #[must_use]
    pub fn search_input(
        id: UiNodeId,
        name: impl Into<String>,
        initial_value: impl Into<String>,
    ) -> Self {
        let mut node = Self::text_input(id, name, initial_value, UiTextInputOptions::default());
        node.kind = UiWidgetKind::SearchInput;
        node.semantics.role = SemanticRole::SearchBox;
        node
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

    /// Applies bounded built-in validation to a retained text control.
    #[must_use]
    pub const fn with_text_validation(mut self, validation: UiTextValidation) -> Self {
        self.text_validation = Some(validation);
        self
    }

    /// Declares this node as a typed drag source; it grants no mutation authority.
    #[must_use]
    pub const fn with_drag_source(mut self, kind: UiDragKind) -> Self {
        self.drag_source = Some(kind);
        self
    }

    /// Declares a bounded set of typed drop proposals accepted by this node.
    #[must_use]
    pub fn accepting_drop(mut self, kinds: impl IntoIterator<Item = UiDragKind>) -> Self {
        self.drop_accepts = kinds.into_iter().collect();
        self.drop_operations = vec![UiDropOperation::Move];
        self
    }

    /// Declares accepted payload families and host operations for drop negotiation.
    #[must_use]
    pub fn accepting_drop_operations(
        mut self,
        kinds: impl IntoIterator<Item = UiDragKind>,
        operations: impl IntoIterator<Item = UiDropOperation>,
    ) -> Self {
        self.drop_accepts = kinds.into_iter().collect();
        self.drop_operations = operations.into_iter().collect();
        self
    }

    /// Projects enabled, selected, expanded, and validation state to semantics.
    #[must_use]
    pub const fn with_control_state(mut self, state: UiControlState) -> Self {
        self.semantics.state = state;
        self
    }

    /// Makes a named region keyboard focusable for collection or drop behavior.
    #[must_use]
    pub const fn with_focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
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
            text_validation: None,
            drag_source: None,
            drop_accepts: Vec::new(),
            drop_operations: Vec::new(),
            focusable: false,
            children: Vec::new(),
        }
    }

    fn professional_container(
        id: UiNodeId,
        name: impl Into<String>,
        kind: UiWidgetKind,
        role: SemanticRole,
        layout: UiLayout,
        children: Vec<UiNodeId>,
    ) -> Self {
        let name = name.into();
        let mut node = Self::container(id, name.clone(), layout, children);
        node.kind = kind;
        node.semantics = UiSemantics::professional(name, role);
        node
    }

    /// Creates a keyboard-operable combo box with retained option children.
    #[must_use]
    pub fn combo_box(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        value: impl Into<String>,
        children: Vec<UiNodeId>,
    ) -> Self {
        let name = name.into();
        let mut node = Self::button(id, name.clone(), action, value);
        node.kind = UiWidgetKind::ComboBox;
        node.layout = UiLayout::VerticalStack { gap: 0.0 };
        node.style = UiStyle::text_field();
        node.semantics.role = SemanticRole::ComboBox;
        node.children = children;
        node
    }

    /// Creates one typed, keyboard-operable combo-box option.
    #[must_use]
    pub fn combo_option(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        selected: bool,
    ) -> Self {
        let name = name.into();
        let mut node = Self::button(id, name.clone(), action, name);
        node.kind = UiWidgetKind::ComboOption;
        node.semantics.role = SemanticRole::Option;
        node.semantics.state.selected = selected;
        node
    }

    /// Creates a restrained horizontal menu bar.
    #[must_use]
    pub fn menu_bar(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_container(
            id,
            name,
            UiWidgetKind::MenuBar,
            SemanticRole::MenuBar,
            UiLayout::HorizontalStack { gap: 4.0 },
            children,
        )
    }

    /// Creates a semantic menu container; menu items own activation commands.
    #[must_use]
    pub fn menu(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_container(
            id,
            name,
            UiWidgetKind::Menu,
            SemanticRole::Menu,
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        )
    }

    /// Creates a focus-preserving contextual menu surface.
    #[must_use]
    pub fn context_menu(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_container(
            id,
            name,
            UiWidgetKind::ContextMenu,
            SemanticRole::Menu,
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        )
    }

    /// Creates a named menu item with release-based typed activation.
    #[must_use]
    pub fn menu_item(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut node = Self::button(id, name, action, text);
        node.kind = UiWidgetKind::MenuItem;
        node.semantics.role = SemanticRole::MenuItem;
        node
    }

    /// Creates a non-focus-stealing semantic tooltip.
    #[must_use]
    pub fn tooltip(id: UiNodeId, name: impl Into<String>, text: impl Into<String>) -> Self {
        let mut node = Self::label(id, name, text);
        node.kind = UiWidgetKind::Tooltip;
        node.semantics.role = SemanticRole::Tooltip;
        node
    }

    /// Creates a bounded semantic live-region notification.
    #[must_use]
    pub fn toast(id: UiNodeId, name: impl Into<String>, text: impl Into<String>) -> Self {
        let mut node = Self::label(id, name, text);
        node.kind = UiWidgetKind::Toast;
        node.semantics.role = SemanticRole::LiveRegion;
        node
    }

    /// Creates a tab-list container.
    #[must_use]
    pub fn tabs(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_container(
            id,
            name,
            UiWidgetKind::Tabs,
            SemanticRole::TabList,
            UiLayout::HorizontalStack { gap: 4.0 },
            children,
        )
    }

    /// Creates one keyboard-operable tab.
    #[must_use]
    pub fn tab(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        selected: bool,
    ) -> Self {
        let name = name.into();
        let mut node = Self::button(id, name.clone(), action, name);
        node.kind = UiWidgetKind::Tab;
        node.semantics.role = SemanticRole::Tab;
        node.semantics.state.selected = selected;
        node
    }

    /// Creates a virtualizable tree container.
    #[must_use]
    pub fn tree(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_container(
            id,
            name,
            UiWidgetKind::Tree,
            SemanticRole::Tree,
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        )
    }

    /// Creates a stable tree row with explicit selected and expanded state.
    #[must_use]
    pub fn tree_item(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        selected: bool,
        expanded: bool,
    ) -> Self {
        let name = name.into();
        let mut node = Self::button(id, name.clone(), action, name);
        node.kind = UiWidgetKind::TreeItem;
        node.semantics.role = SemanticRole::TreeItem;
        node.semantics.state.selected = selected;
        node.semantics.state.expanded = expanded;
        node
    }

    fn professional_region(
        id: UiNodeId,
        name: impl Into<String>,
        kind: UiWidgetKind,
        role: SemanticRole,
        layout: UiLayout,
        children: Vec<UiNodeId>,
    ) -> Self {
        Self::professional_container(id, name, kind, role, layout, children)
    }

    /// Creates a semantic table container.
    #[must_use]
    pub fn table(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::Table,
            SemanticRole::Table,
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        )
    }

    /// Creates a semantic table row.
    #[must_use]
    pub fn table_row(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::TableRow,
            SemanticRole::Row,
            UiLayout::HorizontalStack { gap: 4.0 },
            children,
        )
    }

    /// Creates a semantic table cell.
    #[must_use]
    pub fn table_cell(id: UiNodeId, name: impl Into<String>, text: impl Into<String>) -> Self {
        let mut node = Self::label(id, name, text);
        node.kind = UiWidgetKind::TableCell;
        node.semantics.role = SemanticRole::Cell;
        node
    }

    /// Creates a stable focusable list row with a typed activation command.
    #[must_use]
    pub fn list_item(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        selected: bool,
    ) -> Self {
        let name = name.into();
        let mut node = Self::button(id, name.clone(), action, name);
        node.kind = UiWidgetKind::ListItem;
        node.semantics.role = SemanticRole::ListItem;
        node.semantics.state.selected = selected;
        node
    }

    /// Creates a semantic property grid.
    #[must_use]
    pub fn property_grid(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::PropertyGrid,
            SemanticRole::PropertyGrid,
            UiLayout::VerticalStack { gap: 4.0 },
            children,
        )
    }

    /// Creates a virtual-list semantic root; callers realize only `virtual_range` rows.
    #[must_use]
    pub fn virtual_list(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::VirtualList,
            SemanticRole::List,
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        )
    }

    /// Creates a semantic timeline container.
    #[must_use]
    pub fn timeline(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::Timeline,
            SemanticRole::Timeline,
            UiLayout::Overlay,
            children,
        )
    }

    /// Creates a keyboard-operable splitter that proposes a typed resize command.
    #[must_use]
    pub fn splitter(
        id: UiNodeId,
        name: impl Into<String>,
        action: impl Into<String>,
        axis: UiAxis,
    ) -> Self {
        let mut node = Self::button(id, name, action, "");
        node.kind = UiWidgetKind::Splitter;
        node.semantics.role = SemanticRole::Splitter;
        node.text = None;
        node.layout = UiLayout::Flex { axis, gap: 0.0 };
        node
    }

    /// Creates a command-palette dialog root with retained filter/list children.
    #[must_use]
    pub fn command_palette(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::CommandPalette,
            SemanticRole::Dialog,
            UiLayout::VerticalStack { gap: 4.0 },
            children,
        )
    }

    /// Creates a renderer-neutral graph interaction region.
    #[must_use]
    pub fn graph(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::Graph,
            SemanticRole::Graph,
            UiLayout::Absolute,
            children,
        )
        .with_focusable(true)
    }

    /// Creates a renderer-neutral direct-manipulation canvas region.
    #[must_use]
    pub fn canvas(id: UiNodeId, name: impl Into<String>, children: Vec<UiNodeId>) -> Self {
        Self::professional_region(
            id,
            name,
            UiWidgetKind::Canvas,
            SemanticRole::Canvas,
            UiLayout::Absolute,
            children,
        )
        .with_focusable(true)
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
    UnexpectedTextValidation(UiNodeId),
    PasswordInitialValue(UiNodeId),
    DragSourceNotFocusable(UiNodeId),
    DropTargetNotFocusable(UiNodeId),
    TooManyDropKinds {
        node: UiNodeId,
        count: usize,
        maximum: usize,
    },
    DuplicateDropKind {
        node: UiNodeId,
        kind: UiDragKind,
    },
    MissingDropOperation(UiNodeId),
    UnexpectedDropOperation(UiNodeId),
    DuplicateDropOperation {
        node: UiNodeId,
        operation: UiDropOperation,
    },
    TooManyDropOperations {
        node: UiNodeId,
        count: usize,
        maximum: usize,
    },
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
        Self::validate_semantics(id, node)?;
        Self::validate_text_control(id, node)?;
        Self::validate_drag_contract(id, node)
    }

    fn validate_semantics(id: UiNodeId, node: &UiNode) -> Result<(), UiDocumentError> {
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
            UiWidgetKind::Button
                | UiWidgetKind::IconButton
                | UiWidgetKind::Toggle
                | UiWidgetKind::MenuItem
                | UiWidgetKind::Tab
                | UiWidgetKind::TreeItem
                | UiWidgetKind::ListItem
                | UiWidgetKind::ComboBox
                | UiWidgetKind::ComboOption
                | UiWidgetKind::Splitter
        ) && node.semantics.action.is_none()
        {
            return Err(UiDocumentError::MissingButtonAction(id));
        }
        Ok(())
    }

    fn validate_text_control(id: UiNodeId, node: &UiNode) -> Result<(), UiDocumentError> {
        let is_text_input = matches!(
            node.kind,
            UiWidgetKind::TextInput | UiWidgetKind::SearchInput
        );
        if is_text_input && !node.focusable {
            return Err(UiDocumentError::TextInputNotFocusable(id));
        }
        if is_text_input && node.text_input.is_none() {
            return Err(UiDocumentError::MissingTextInputOptions(id));
        }
        if !is_text_input && node.text_input.is_some() {
            return Err(UiDocumentError::UnexpectedTextInputOptions(id));
        }
        if !is_text_input && node.text_validation.is_some() {
            return Err(UiDocumentError::UnexpectedTextValidation(id));
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

    fn validate_drag_contract(id: UiNodeId, node: &UiNode) -> Result<(), UiDocumentError> {
        if node.drag_source.is_some() && !node.focusable {
            return Err(UiDocumentError::DragSourceNotFocusable(id));
        }
        if !node.drop_accepts.is_empty() && !node.focusable {
            return Err(UiDocumentError::DropTargetNotFocusable(id));
        }
        if node.drop_accepts.len() > MAX_DROP_KINDS {
            return Err(UiDocumentError::TooManyDropKinds {
                node: id,
                count: node.drop_accepts.len(),
                maximum: MAX_DROP_KINDS,
            });
        }
        let mut drop_kinds = BTreeSet::new();
        for kind in &node.drop_accepts {
            if !drop_kinds.insert(*kind) {
                return Err(UiDocumentError::DuplicateDropKind {
                    node: id,
                    kind: *kind,
                });
            }
        }
        if !node.drop_accepts.is_empty() && node.drop_operations.is_empty() {
            return Err(UiDocumentError::MissingDropOperation(id));
        }
        if node.drop_accepts.is_empty() && !node.drop_operations.is_empty() {
            return Err(UiDocumentError::UnexpectedDropOperation(id));
        }
        if node.drop_operations.len() > MAX_DROP_OPERATIONS {
            return Err(UiDocumentError::TooManyDropOperations {
                node: id,
                count: node.drop_operations.len(),
                maximum: MAX_DROP_OPERATIONS,
            });
        }
        let mut drop_operations = BTreeSet::new();
        for operation in &node.drop_operations {
            if !drop_operations.insert(*operation) {
                return Err(UiDocumentError::DuplicateDropOperation {
                    node: id,
                    operation: *operation,
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
        if node.focusable && !node.semantics.state.disabled {
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

    #[test]
    fn virtual_range_realizes_only_a_bounded_visible_window() {
        assert_eq!(
            virtual_range(MAX_VIRTUAL_ITEMS, 20.0, 100.0, 50.0, 2),
            Ok(UiVirtualRange { start: 0, end: 10 })
        );
        assert_eq!(
            virtual_range(MAX_VIRTUAL_ITEMS + 1, 20.0, 100.0, 0.0, 0),
            Err(UiVirtualRangeError::TooManyItems {
                count: MAX_VIRTUAL_ITEMS + 1,
                maximum: MAX_VIRTUAL_ITEMS,
            })
        );
        assert_eq!(
            virtual_range(10, f32::NAN, 100.0, 0.0, 0),
            Err(UiVirtualRangeError::InvalidGeometry)
        );
        assert_eq!(
            virtual_range(10, 20.0, 100.0, 0.0, MAX_RETAINED_NODES + 1),
            Err(UiVirtualRangeError::OverscanTooLarge {
                count: MAX_RETAINED_NODES + 1,
                maximum: MAX_RETAINED_NODES,
            })
        );
        assert_eq!(
            virtual_range(MAX_VIRTUAL_ITEMS, 1.0, f32::from(u16::MAX), 0.0, 0,),
            Err(UiVirtualRangeError::RealizedRangeTooLarge {
                count: MAX_VIRTUAL_ITEMS,
                maximum: MAX_RETAINED_NODES,
            })
        );
    }

    #[test]
    fn collection_cursor_keeps_hidden_identity_until_the_user_navigates() {
        let hidden = UiNodeId::new(2);
        let first = UiNodeId::new(3);
        let second = UiNodeId::new(4);
        let mut cursor = UiCollectionCursor {
            selected: Some(hidden),
        };
        assert_eq!(
            cursor.navigate(&[], 10, UiCollectionNavigation::Next),
            Some(hidden)
        );
        assert_eq!(
            cursor.navigate(&[first, second], 10, UiCollectionNavigation::Next),
            Some(first)
        );
    }

    #[test]
    fn professional_controls_publish_roles_states_and_keyboard_drop_contracts() {
        let tree = UiNodeId::new(10);
        let item = UiNodeId::new(11);
        let document = UiDocument::new(
            tree,
            vec![
                UiNode::tree(tree, "Hierarchy", vec![item]),
                UiNode::tree_item(item, "Camera", "world.select_camera", true, true)
                    .with_drag_source(UiDragKind::Entity)
                    .accepting_drop([UiDragKind::Entity]),
            ],
        )
        .expect("professional tree is valid");
        let item = document.node(item).expect("item exists");
        assert_eq!(item.kind, UiWidgetKind::TreeItem);
        assert_eq!(item.semantics.role, SemanticRole::TreeItem);
        assert!(item.semantics.state.selected);
        assert!(item.semantics.state.expanded);
        assert_eq!(item.drag_source, Some(UiDragKind::Entity));
        assert_eq!(item.drop_accepts, vec![UiDragKind::Entity]);
        assert_eq!(item.drop_operations, vec![UiDropOperation::Move]);
    }

    #[test]
    fn professional_component_families_publish_owned_semantics() {
        let root = UiNodeId::new(30);
        let search = UiNodeId::new(31);
        let option = UiNodeId::new(32);
        let combo = UiNodeId::new(33);
        let menu_item = UiNodeId::new(34);
        let menu_bar = UiNodeId::new(35);
        let context_item = UiNodeId::new(36);
        let context_menu = UiNodeId::new(37);
        let splitter = UiNodeId::new(38);
        let graph = UiNodeId::new(39);
        let canvas = UiNodeId::new(40);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Professional controls",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![
                        search,
                        combo,
                        menu_bar,
                        context_menu,
                        splitter,
                        graph,
                        canvas,
                    ],
                ),
                UiNode::search_input(search, "Search commands", "build"),
                UiNode::combo_option(option, "Compact", "density.compact", true),
                UiNode::combo_box(combo, "Density", "density.open", "Compact", vec![option]),
                UiNode::menu_item(menu_item, "Build", "build.start", "Build"),
                UiNode::menu_bar(menu_bar, "Application menu", vec![menu_item]),
                UiNode::menu_item(context_item, "Rename", "entity.rename", "Rename"),
                UiNode::context_menu(context_menu, "Entity actions", vec![context_item]),
                UiNode::splitter(
                    splitter,
                    "Resize inspector",
                    "inspector.resize",
                    UiAxis::Horizontal,
                ),
                UiNode::graph(graph, "Material graph", Vec::new()),
                UiNode::canvas(canvas, "World viewport", Vec::new()),
            ],
        )
        .expect("professional component document is valid");

        for (id, kind, role) in [
            (search, UiWidgetKind::SearchInput, SemanticRole::SearchBox),
            (combo, UiWidgetKind::ComboBox, SemanticRole::ComboBox),
            (option, UiWidgetKind::ComboOption, SemanticRole::Option),
            (menu_bar, UiWidgetKind::MenuBar, SemanticRole::MenuBar),
            (context_menu, UiWidgetKind::ContextMenu, SemanticRole::Menu),
            (splitter, UiWidgetKind::Splitter, SemanticRole::Splitter),
            (graph, UiWidgetKind::Graph, SemanticRole::Graph),
            (canvas, UiWidgetKind::Canvas, SemanticRole::Canvas),
        ] {
            let node = document.node(id).expect("professional node exists");
            assert_eq!((node.kind, node.semantics.role), (kind, role));
        }
        assert!(document.focus_order().contains(&graph));
        assert!(document.focus_order().contains(&canvas));
    }

    #[test]
    fn malformed_drag_contracts_are_rejected_before_a_frame() {
        let node = UiNodeId::new(20);
        let invalid =
            UiNode::label(node, "Read only", "Read only").accepting_drop([UiDragKind::Asset]);
        assert_eq!(
            UiDocument::new(node, vec![invalid]),
            Err(UiDocumentError::DropTargetNotFocusable(node))
        );

        let duplicate = UiNode::button(node, "Drop", "drop", "Drop")
            .accepting_drop([UiDragKind::Asset, UiDragKind::Asset]);
        assert_eq!(
            UiDocument::new(node, vec![duplicate]),
            Err(UiDocumentError::DuplicateDropKind {
                node,
                kind: UiDragKind::Asset,
            })
        );

        let duplicate_operation = UiNode::button(node, "Drop", "drop", "Drop")
            .accepting_drop_operations(
                [UiDragKind::Asset],
                [UiDropOperation::Move, UiDropOperation::Move],
            );
        assert_eq!(
            UiDocument::new(node, vec![duplicate_operation]),
            Err(UiDocumentError::DuplicateDropOperation {
                node,
                operation: UiDropOperation::Move,
            })
        );

        let too_many_operations = UiNode::button(node, "Drop", "drop", "Drop")
            .accepting_drop_operations(
                [UiDragKind::Asset],
                [
                    UiDropOperation::Move,
                    UiDropOperation::Copy,
                    UiDropOperation::Link,
                    UiDropOperation::Move,
                ],
            );
        assert_eq!(
            UiDocument::new(node, vec![too_many_operations]),
            Err(UiDocumentError::TooManyDropOperations {
                node,
                count: MAX_DROP_OPERATIONS + 1,
                maximum: MAX_DROP_OPERATIONS,
            })
        );
    }
}
