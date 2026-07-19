//! Retained, renderer-independent UI contracts.
//!
//! The data crossing this crate boundary is Meridian-owned.  Platform and
//! renderer adapters consume [`DisplayList`] and [`SemanticTree`] rather than
//! borrowing widget state or exposing their native types here.

use std::collections::{BTreeMap, BTreeSet};

use meridian_core::StableId;

const MIN_SUPPORTED_SCALE_FACTOR: f32 = 0.5;
const MAX_SUPPORTED_SCALE_FACTOR: f32 = 4.0;

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
/// Maximum host-bound assistive operations declared by one retained node.
///
/// This is the complete current vocabulary rather than an arbitrary capacity:
/// each operation needs an explicit, validated host command before it can be
/// exposed to an assistive adapter.
pub const MAX_ASSISTIVE_ACTION_BINDINGS: usize = 5;

/// Sanitizes untrusted platform scale input to the supported 50-400% interval.
#[must_use]
pub fn sanitized_scale_factor(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_SUPPORTED_SCALE_FACTOR, MAX_SUPPORTED_SCALE_FACTOR)
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
stable_ui_id!(
    UiSharedElementId,
    "Stable identity pairing source and destination presentations across retained nodes."
);

/// Maximum UTF-8 bytes accepted by one canonical command name.
pub const MAX_COMMAND_NAME_BYTES: usize = 256;

impl CommandId {
    /// Derives a deterministic identity from Meridian's canonical command-name
    /// grammar. Command names are intentionally limited to printable ASCII
    /// identifiers so a UI frame cannot smuggle whitespace, control bytes, or
    /// an arbitrary serialized payload across the command boundary.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        if name.is_empty() || name.len() > MAX_COMMAND_NAME_BYTES {
            return None;
        }
        let mut bytes = name.bytes();
        let first = bytes.next()?;
        if !is_command_name_start(first) || !bytes.all(is_command_name_continue) {
            return None;
        }
        // FNV-1a over 128 bits is stable across processes and platforms. The
        // command text remains available as an audit label; this ID is the
        // typed identity consumed by command adapters.
        let mut hash = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d_u128;
        for byte in name.bytes() {
            hash ^= u128::from(byte);
            hash = hash.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b_u128);
        }
        Some(Self::new(hash))
    }
}

const fn is_command_name_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

const fn is_command_name_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-' | b'_' | b'/')
}

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

/// The only retained reasons that may request spatial presentation motion.
///
/// Logical layout, hit testing, focus, and semantics always use the accepted
/// target geometry. This descriptor authorizes only the renderer-facing
/// presentation interpolation between retained layouts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSpatialMotionKind {
    PhysicalPanel,
    SharedElement,
}

/// Presentation-only intent declared by one retained node.
///
/// The opacity target is part of the retained visual source, while its current
/// interpolated value is owned by the runtime. It cannot change layout,
/// interaction, or semantic authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPresentationOptions {
    pub spatial_motion: Option<UiSpatialMotionKind>,
    /// Required when `spatial_motion` is `SharedElement`; never aliases a node ID.
    pub shared_element: Option<UiSharedElementId>,
    pub opacity: f32,
}

impl Default for UiPresentationOptions {
    fn default() -> Self {
        Self {
            spatial_motion: None,
            shared_element: None,
            opacity: 1.0,
        }
    }
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
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiFontRole {
    #[default]
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

impl IconId {
    /// Complete stable icon vocabulary used by asset verification and renderer tests.
    pub const ALL: [Self; 12] = [
        Self::Play,
        Self::Stop,
        Self::Build,
        Self::Search,
        Self::Settings,
        Self::More,
        Self::Close,
        Self::ChevronDown,
        Self::ChevronRight,
        Self::Warning,
        Self::Error,
        Self::Success,
    ];
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

/// Locked icon geometry selected from the active theme rather than text metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiIconTokens {
    pub size: f32,
    pub stroke_width: f32,
    pub text_gap: f32,
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
    pub high_contrast_colors: UiColorTokens,
    pub geometry: UiGeometryTokens,
    pub motion: UiMotionTokens,
    pub icons: UiIconTokens,
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
            high_contrast_colors: UiColorTokens {
                background: UiColor::rgba(0.0, 0.0, 0.0, 1.0),
                surface: UiColor::rgba(0.035_294_12, 0.035_294_12, 0.035_294_12, 1.0),
                border: UiColor::rgba(1.0, 1.0, 1.0, 1.0),
                primary_text: UiColor::rgba(1.0, 1.0, 1.0, 1.0),
                secondary_text: UiColor::rgba(0.92, 0.92, 0.92, 1.0),
                muted: UiColor::rgba(0.75, 0.75, 0.75, 1.0),
                destructive: UiColor::rgba(1.0, 0.42, 0.38, 1.0),
                destructive_hover: UiColor::rgba(1.0, 0.58, 0.54, 1.0),
                positive: UiColor::rgba(0.95, 0.91, 0.62, 1.0),
                warning: UiColor::rgba(1.0, 0.82, 0.35, 1.0),
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
            icons: UiIconTokens {
                size: 16.0,
                stroke_width: 2.0,
                text_gap: 8.0,
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

/// A Meridian-owned relationship between two retained semantic nodes.
///
/// The owning node declares the relationship and document validation resolves
/// every target against the accepted retained tree. Platform adapters remain
/// private and may only project this bounded vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiSemanticRelationshipKind {
    LabelledBy,
    DescribedBy,
    Controls,
    Details,
    FlowTo,
    ErrorMessage,
}

/// Stable relationships declared by one retained semantic node.
///
/// Structural parent/child ownership remains the retained document tree. These
/// relations add only non-structural assistive context and never grant command
/// or filesystem authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiSemanticRelationships {
    pub labelled_by: Vec<UiNodeId>,
    pub described_by: Vec<UiNodeId>,
    pub controls: Vec<UiNodeId>,
    pub details: Vec<UiNodeId>,
    pub flow_to: Vec<UiNodeId>,
    pub error_message: Option<UiNodeId>,
}

/// Position metadata for one realized item in a potentially virtualized
/// collection. The collection itself is never materialized merely to publish
/// this bounded semantic context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSemanticCollectionItem {
    pub position: u32,
    pub set_size: u32,
}

/// An assistive operation whose source mutation remains owned by a typed host
/// command adapter.
///
/// Focus, activation, text edits, and scrolling have retained-runtime paths.
/// These operations require an explicit host command because they change a
/// domain-owned tree, splitter, timeline, or context-menu state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UiHostAssistiveAction {
    Expand,
    Collapse,
    Increment,
    Decrement,
    ShowContextMenu,
}

impl UiHostAssistiveAction {
    const fn is_compatible_with(
        self,
        role: SemanticRole,
        state: UiControlState,
        focusable: bool,
    ) -> bool {
        if !focusable {
            return false;
        }
        match self {
            Self::Expand => {
                matches!(role, SemanticRole::TreeItem | SemanticRole::ComboBox) && !state.expanded
            }
            Self::Collapse => {
                matches!(role, SemanticRole::TreeItem | SemanticRole::ComboBox) && state.expanded
            }
            Self::Increment | Self::Decrement => {
                matches!(role, SemanticRole::Splitter | SemanticRole::Timeline)
            }
            // A context command is independent from primary activation: a
            // focusable canvas, hierarchy row, or property region can expose
            // contextual operations without claiming that it is clickable.
            Self::ShowContextMenu => true,
        }
    }
}

/// Validated binding between one advertised assistive operation and one
/// canonical Meridian command name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAssistiveActionBinding {
    pub action: UiHostAssistiveAction,
    pub command: String,
}

/// Named semantics and a typed action token declared by a UI node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSemantics {
    pub role: SemanticRole,
    pub name: String,
    pub description: Option<String>,
    pub action: Option<String>,
    /// Explicit host commands for assistive operations that the retained
    /// runtime must not guess or substitute with ordinary activation.
    pub assistive_actions: Vec<UiAssistiveActionBinding>,
    pub value: Option<String>,
    pub state: UiControlState,
    pub relationships: UiSemanticRelationships,
    pub collection_item: Option<UiSemanticCollectionItem>,
}

impl UiSemantics {
    #[must_use]
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Group,
            name: name.into(),
            description: None,
            action: None,
            assistive_actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn status(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Status,
            name: name.into(),
            description: None,
            action: None,
            assistive_actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn button(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Button,
            name: name.into(),
            description: None,
            action: Some(action.into()),
            assistive_actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn text_input(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::TextInput,
            name: name.into(),
            description: None,
            action: None,
            assistive_actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn toggle(name: impl Into<String>, action: impl Into<String>, value: bool) -> Self {
        Self {
            role: SemanticRole::ToggleButton,
            name: name.into(),
            description: None,
            action: Some(action.into()),
            assistive_actions: Vec::new(),
            value: Some(if value { "on" } else { "off" }.to_owned()),
            state: UiControlState {
                selected: value,
                ..UiControlState::default()
            },
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn progress(name: impl Into<String>, value: u8) -> Self {
        Self {
            role: SemanticRole::ProgressIndicator,
            name: name.into(),
            description: None,
            action: None,
            assistive_actions: Vec::new(),
            value: Some(format!("{}%", value.min(100))),
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }

    #[must_use]
    pub fn professional(name: impl Into<String>, role: SemanticRole) -> Self {
        Self {
            role,
            name: name.into(),
            description: None,
            action: None,
            assistive_actions: Vec::new(),
            value: None,
            state: UiControlState::default(),
            relationships: UiSemanticRelationships::default(),
            collection_item: None,
        }
    }
}

/// Semantic color roles resolved through the active theme at a frame boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiColorRole {
    Background,
    Surface,
    Border,
    PrimaryText,
    SecondaryText,
    Muted,
    Destructive,
    DestructiveHover,
    Positive,
    Emphasis,
}

/// Locked component-level visual treatments available to retained nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiStyleVariant {
    Panel,
    Text,
    Transparent,
    Canvas,
    Surface,
    ElevatedSurface,
    Heading,
    SectionHeading,
    MutedText,
    PrimaryAction,
    DestructiveAction,
    SecondaryAction,
    CompactAction,
    TextField,
    CompactTextField,
}

/// Styling authority retained by a node.
///
/// `LegacyTokenResolved` preserves source compatibility for callers that still
/// build [`UiStyle`] values. The runtime maps every legacy color and metric to
/// an active-theme token before layout or display-list emission; raw values are
/// never renderer authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiStyleReference {
    Variant(UiStyleVariant),
    LegacyTokenResolved,
}

/// Retained interaction flags used by deterministic state-selector resolution.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiVisualState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
    pub selected: bool,
    pub invalid: bool,
}

/// Highest-priority state selector applied to one retained node.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiStyleSelector {
    #[default]
    Idle,
    Hovered,
    Focused,
    Selected,
    Pressed,
    Invalid,
    Disabled,
}

impl UiVisualState {
    /// Resolves mutually competing selectors in a stable accessibility-first order.
    #[must_use]
    pub const fn selector(self) -> UiStyleSelector {
        if self.disabled {
            UiStyleSelector::Disabled
        } else if self.invalid {
            UiStyleSelector::Invalid
        } else if self.pressed {
            UiStyleSelector::Pressed
        } else if self.selected {
            UiStyleSelector::Selected
        } else if self.focused {
            UiStyleSelector::Focused
        } else if self.hovered {
            UiStyleSelector::Hovered
        } else {
            UiStyleSelector::Idle
        }
    }
}

/// Rendering style compatibility request and resolved frame value.
///
/// Nodes also carry a [`UiStyleReference`]. Consumers must resolve this value
/// through [`UiTheme::resolve_style`] rather than treating its raw fields as
/// design-system authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStyle {
    pub background: Option<UiColor>,
    pub border: Option<UiBorder>,
    pub corner_radius: f32,
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

/// Result of resolving a retained style reference and interaction selector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStyleResolution {
    pub style: UiStyle,
    pub selector: UiStyleSelector,
    pub used_token_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiSpacingToken {
    None,
    ThreeQuarter,
    Base,
    FiveQuarter,
    ThreeHalf,
    Double,
    FiveHalf,
    Triple,
    SevenHalf,
    Quadruple,
    Sextuple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiRadiusToken {
    None,
    Compact,
    Control,
    Panel,
    Floating,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiTextSizeToken {
    Caption,
    Small,
    Metadata,
    CompactBody,
    Brand,
    Body,
    Title,
    Display,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UiTokenColor {
    role: UiColorRole,
    alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UiStyleTokens {
    background: Option<UiTokenColor>,
    border: Option<UiColorRole>,
    radius: UiRadiusToken,
    foreground: UiColorRole,
    padding: UiSpacingToken,
    font_size: UiTextSizeToken,
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
    /// Returns this style with a bounded uniform corner radius.
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    #[must_use]
    pub const fn panel() -> Self {
        Self {
            background: Some(UiColor::panel()),
            border: None,
            corner_radius: 10.0,
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
            corner_radius: 0.0,
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
            corner_radius: 0.0,
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
            corner_radius: 0.0,
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
            corner_radius: 10.0,
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
            corner_radius: 10.0,
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
            corner_radius: 0.0,
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
            corner_radius: 0.0,
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
            corner_radius: 0.0,
            foreground: UiColor::muted_text(),
            padding: 0.0,
            font_size: 13.0,
        }
    }

    /// The primary action treatment used once per decision group.
    #[must_use]
    pub const fn primary_action() -> Self {
        Self {
            background: Some(UiColor::surface()),
            border: Some(UiBorder {
                color: UiColor::amber(),
                width: 1,
            }),
            corner_radius: 6.0,
            foreground: UiColor::text(),
            padding: 12.0,
            font_size: 16.0,
        }
    }

    /// A destructive action that remains distinct from ordinary emphasis.
    #[must_use]
    pub const fn destructive_action() -> Self {
        Self {
            background: Some(UiColor::red()),
            border: Some(UiBorder {
                color: UiColor::red_hover(),
                width: 1,
            }),
            corner_radius: 6.0,
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
            corner_radius: 6.0,
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
            corner_radius: 4.0,
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
            corner_radius: 6.0,
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
            corner_radius: 4.0,
            foreground: UiColor::foreground(),
            padding: 4.0,
            font_size: 14.0,
        }
    }
}

impl UiStyleVariant {
    fn tokens(self) -> UiStyleTokens {
        match self {
            Self::Panel | Self::Canvas | Self::Surface | Self::ElevatedSurface => {
                self.surface_tokens()
            }
            Self::Text
            | Self::Transparent
            | Self::Heading
            | Self::SectionHeading
            | Self::MutedText => self.text_tokens(),
            Self::PrimaryAction
            | Self::DestructiveAction
            | Self::SecondaryAction
            | Self::CompactAction
            | Self::TextField
            | Self::CompactTextField => self.control_tokens(),
        }
    }

    fn surface_tokens(self) -> UiStyleTokens {
        let color = |role| UiTokenColor { role, alpha: 1.0 };
        match self {
            Self::Panel => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: None,
                radius: UiRadiusToken::Panel,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Triple,
                font_size: UiTextSizeToken::Body,
            },
            Self::Canvas => UiStyleTokens {
                background: Some(color(UiColorRole::Background)),
                border: None,
                radius: UiRadiusToken::None,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Sextuple,
                font_size: UiTextSizeToken::Body,
            },
            Self::Surface => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: Some(UiColorRole::Border),
                radius: UiRadiusToken::Panel,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::SevenHalf,
                font_size: UiTextSizeToken::Body,
            },
            Self::ElevatedSurface => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: Some(UiColorRole::Positive),
                radius: UiRadiusToken::Panel,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Quadruple,
                font_size: UiTextSizeToken::Body,
            },
            _ => unreachable!("surface variant dispatch is exhaustive"),
        }
    }

    fn text_tokens(self) -> UiStyleTokens {
        match self {
            Self::Text => UiStyleTokens {
                background: None,
                border: None,
                radius: UiRadiusToken::None,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::ThreeHalf,
                font_size: UiTextSizeToken::Body,
            },
            Self::Transparent | Self::SectionHeading => UiStyleTokens {
                background: None,
                border: None,
                radius: UiRadiusToken::None,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::None,
                font_size: UiTextSizeToken::Body,
            },
            Self::Heading => UiStyleTokens {
                background: None,
                border: None,
                radius: UiRadiusToken::None,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::None,
                font_size: UiTextSizeToken::Display,
            },
            Self::MutedText => UiStyleTokens {
                background: None,
                border: None,
                radius: UiRadiusToken::None,
                foreground: UiColorRole::Muted,
                padding: UiSpacingToken::None,
                font_size: UiTextSizeToken::Metadata,
            },
            _ => unreachable!("text variant dispatch is exhaustive"),
        }
    }

    fn control_tokens(self) -> UiStyleTokens {
        let color = |role| UiTokenColor { role, alpha: 1.0 };
        match self {
            Self::PrimaryAction => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: Some(UiColorRole::Emphasis),
                radius: UiRadiusToken::Control,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Triple,
                font_size: UiTextSizeToken::Body,
            },
            Self::DestructiveAction => UiStyleTokens {
                background: Some(color(UiColorRole::Destructive)),
                border: Some(UiColorRole::DestructiveHover),
                radius: UiRadiusToken::Control,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Triple,
                font_size: UiTextSizeToken::Body,
            },
            Self::SecondaryAction => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: Some(UiColorRole::Border),
                radius: UiRadiusToken::Control,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::FiveHalf,
                font_size: UiTextSizeToken::CompactBody,
            },
            Self::CompactAction => UiStyleTokens {
                background: Some(color(UiColorRole::Surface)),
                border: Some(UiColorRole::Border),
                radius: UiRadiusToken::Compact,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::ThreeQuarter,
                font_size: UiTextSizeToken::Caption,
            },
            Self::TextField => UiStyleTokens {
                background: Some(color(UiColorRole::Background)),
                border: Some(UiColorRole::Border),
                radius: UiRadiusToken::Control,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Triple,
                font_size: UiTextSizeToken::Body,
            },
            Self::CompactTextField => UiStyleTokens {
                background: Some(color(UiColorRole::Background)),
                border: Some(UiColorRole::Border),
                radius: UiRadiusToken::Compact,
                foreground: UiColorRole::PrimaryText,
                padding: UiSpacingToken::Base,
                font_size: UiTextSizeToken::CompactBody,
            },
            _ => unreachable!("control variant dispatch is exhaustive"),
        }
    }

    /// Compatibility value for source builders migrating to variant references.
    #[must_use]
    pub const fn compatibility_style(self) -> UiStyle {
        match self {
            Self::Panel => UiStyle::panel(),
            Self::Text => UiStyle::text(),
            Self::Transparent => UiStyle::transparent(),
            Self::Canvas => UiStyle::canvas(),
            Self::Surface => UiStyle::surface(),
            Self::ElevatedSurface => UiStyle::elevated_surface(),
            Self::Heading => UiStyle::heading(),
            Self::SectionHeading => UiStyle::section_heading(),
            Self::MutedText => UiStyle::muted_text(),
            Self::PrimaryAction => UiStyle::primary_action(),
            Self::DestructiveAction => UiStyle::destructive_action(),
            Self::SecondaryAction => UiStyle::secondary_action(),
            Self::CompactAction => UiStyle::compact_action(),
            Self::TextField => UiStyle::text_field(),
            Self::CompactTextField => UiStyle::compact_text_field(),
        }
    }
}

impl UiTheme {
    /// Resolves variants or compatibility styles entirely through active-theme tokens.
    #[must_use]
    pub fn resolve_style(
        self,
        reference: UiStyleReference,
        compatibility: UiStyle,
        state: UiVisualState,
        contrast: UiContrast,
    ) -> UiStyleResolution {
        let (mut tokens, mut used_token_fallback) = match reference {
            UiStyleReference::Variant(variant) => (variant.tokens(), false),
            UiStyleReference::LegacyTokenResolved => self.legacy_style_tokens(compatibility),
        };
        let (_, geometry_token_fallback) = self.resolved_geometry_tokens();
        used_token_fallback |= geometry_token_fallback;
        let selector = state.selector();
        match selector {
            UiStyleSelector::Idle => {}
            UiStyleSelector::Hovered | UiStyleSelector::Focused => {
                tokens.border = Some(UiColorRole::Emphasis);
            }
            UiStyleSelector::Selected => {
                tokens.background = Some(UiTokenColor {
                    role: UiColorRole::Surface,
                    alpha: 1.0,
                });
                tokens.border = Some(UiColorRole::Positive);
            }
            UiStyleSelector::Pressed => {
                tokens.background = Some(UiTokenColor {
                    role: UiColorRole::Background,
                    alpha: 1.0,
                });
                tokens.border = Some(UiColorRole::Emphasis);
            }
            UiStyleSelector::Invalid => {
                tokens.border = Some(UiColorRole::Destructive);
            }
            UiStyleSelector::Disabled => {
                tokens.foreground = UiColorRole::Muted;
                tokens.border = tokens.border.map(|_| UiColorRole::Muted);
            }
        }
        let (background, background_fallback) = tokens.background.map_or((None, false), |token| {
            let (mut color, fallback) = self.resolve_color(token.role, contrast);
            color.alpha = if contrast == UiContrast::High {
                1.0
            } else {
                token.alpha
            };
            (Some(color), fallback)
        });
        let (border, border_fallback) = tokens.border.map_or((None, false), |role| {
            let (color, fallback) = self.resolve_color(role, contrast);
            (Some(UiBorder { color, width: 1 }), fallback)
        });
        let (foreground, foreground_fallback) = self.resolve_color(tokens.foreground, contrast);
        used_token_fallback |= background_fallback || border_fallback || foreground_fallback;
        UiStyleResolution {
            style: UiStyle {
                background,
                border,
                corner_radius: self.radius_value(tokens.radius),
                foreground,
                padding: self.spacing_value(tokens.padding),
                font_size: text_size_value(tokens.font_size),
            },
            selector,
            used_token_fallback,
        }
    }

    /// Returns bounded icon tokens, falling back to the registered default as a unit.
    #[must_use]
    pub fn resolved_icon_tokens(self) -> (UiIconTokens, bool) {
        let locked = Self::meridian_dark().icons;
        if metric_within_locked_bound(self.icons.size, locked.size, false)
            && metric_within_locked_bound(self.icons.stroke_width, locked.stroke_width, false)
            && metric_within_locked_bound(self.icons.text_gap, locked.text_gap, true)
        {
            (self.icons, false)
        } else {
            (locked, true)
        }
    }

    /// Bounds theme geometry against registered Meridian tokens and the
    /// framework's existing 400% scale ceiling. Invalid fields fall back
    /// independently so one malformed metric cannot amplify layout geometry.
    #[must_use]
    pub fn resolved_geometry_tokens(self) -> (UiGeometryTokens, bool) {
        let locked = Self::meridian_dark().geometry;
        let mut used_fallback = false;
        let mut resolve = |value: f32, fallback: f32, allow_zero: bool| {
            if metric_within_locked_bound(value, fallback, allow_zero) {
                value
            } else {
                used_fallback = true;
                fallback
            }
        };
        let resolved = UiGeometryTokens {
            spacing_base: resolve(self.geometry.spacing_base, locked.spacing_base, false),
            dock_gutter: resolve(self.geometry.dock_gutter, locked.dock_gutter, true),
            border: resolve(self.geometry.border, locked.border, false),
            radius_compact: resolve(self.geometry.radius_compact, locked.radius_compact, true),
            radius_control: resolve(self.geometry.radius_control, locked.radius_control, true),
            radius_panel: resolve(self.geometry.radius_panel, locked.radius_panel, true),
            radius_floating: resolve(self.geometry.radius_floating, locked.radius_floating, true),
            application_row: resolve(self.geometry.application_row, locked.application_row, false),
            workspace_row: resolve(self.geometry.workspace_row, locked.workspace_row, false),
            status_row: resolve(self.geometry.status_row, locked.status_row, false),
            activity_rail_collapsed: resolve(
                self.geometry.activity_rail_collapsed,
                locked.activity_rail_collapsed,
                false,
            ),
            activity_rail_expanded: resolve(
                self.geometry.activity_rail_expanded,
                locked.activity_rail_expanded,
                false,
            ),
            browser_width: resolve(self.geometry.browser_width, locked.browser_width, false),
            world_inspector_width: resolve(
                self.geometry.world_inspector_width,
                locked.world_inspector_width,
                false,
            ),
            bottom_shelf_peek: resolve(
                self.geometry.bottom_shelf_peek,
                locked.bottom_shelf_peek,
                false,
            ),
            bottom_shelf_expanded: resolve(
                self.geometry.bottom_shelf_expanded,
                locked.bottom_shelf_expanded,
                false,
            ),
        };
        (resolved, used_fallback)
    }

    /// Returns locked motion timing if an untrusted theme supplies invalid timing.
    #[must_use]
    pub const fn resolved_motion_tokens(self) -> (UiMotionTokens, bool) {
        if self.motion.state_transition_min_ms == 100 && self.motion.state_transition_max_ms == 160
        {
            (self.motion, false)
        } else {
            (Self::meridian_dark().motion, true)
        }
    }

    fn legacy_style_tokens(self, style: UiStyle) -> (UiStyleTokens, bool) {
        let (background, background_fallback) = style.background.map_or((None, false), |color| {
            let (token, fallback) = self.legacy_color_token(color, UiColorRole::Surface);
            (Some(token), fallback)
        });
        let (border, border_fallback) = style.border.map_or((None, false), |border| {
            let (token, color_fallback) =
                self.legacy_color_token(border.color, UiColorRole::Border);
            (Some(token.role), color_fallback || border.width != 1)
        });
        let (foreground, foreground_fallback) =
            self.legacy_color_token(style.foreground, UiColorRole::PrimaryText);
        let (radius, radius_fallback) = self.legacy_radius_token(style.corner_radius);
        let (padding, padding_fallback) = self.legacy_spacing_token(style.padding);
        let (font_size, font_fallback) = legacy_text_size_token(style.font_size);
        (
            UiStyleTokens {
                background,
                border,
                radius,
                foreground: foreground.role,
                padding,
                font_size,
            },
            background_fallback
                || border_fallback
                || foreground_fallback
                || radius_fallback
                || padding_fallback
                || font_fallback,
        )
    }

    fn legacy_color_token(self, color: UiColor, fallback: UiColorRole) -> (UiTokenColor, bool) {
        let alpha_valid = color.alpha.is_finite() && (0.0..=1.0).contains(&color.alpha);
        let role = self
            .role_for_color(color)
            .or_else(|| Self::meridian_dark().role_for_color(color));
        (
            UiTokenColor {
                role: role.unwrap_or(fallback),
                alpha: if alpha_valid { color.alpha } else { 1.0 },
            },
            role.is_none() || !alpha_valid,
        )
    }

    fn role_for_color(self, color: UiColor) -> Option<UiColorRole> {
        const ROLES: [UiColorRole; 10] = [
            UiColorRole::Background,
            UiColorRole::Surface,
            UiColorRole::Border,
            UiColorRole::PrimaryText,
            UiColorRole::SecondaryText,
            UiColorRole::Muted,
            UiColorRole::Destructive,
            UiColorRole::DestructiveHover,
            UiColorRole::Positive,
            UiColorRole::Emphasis,
        ];
        ROLES.into_iter().find(|role| {
            let (candidate, _) = self.resolve_color(*role, UiContrast::Standard);
            color_rgb_near(color, candidate)
        })
    }

    fn resolve_color(self, role: UiColorRole, contrast: UiContrast) -> (UiColor, bool) {
        let palette = if contrast == UiContrast::High {
            self.high_contrast_colors
        } else {
            self.colors
        };
        let color = color_for_role(palette, role);
        if color_is_valid(color) {
            (color, false)
        } else {
            let fallback = if contrast == UiContrast::High {
                Self::meridian_dark().high_contrast_colors
            } else {
                Self::meridian_dark().colors
            };
            (color_for_role(fallback, role), true)
        }
    }

    fn legacy_radius_token(self, value: f32) -> (UiRadiusToken, bool) {
        let (geometry, _) = self.resolved_geometry_tokens();
        nearest_token(
            value,
            &[
                (UiRadiusToken::None, 0.0),
                (UiRadiusToken::Compact, geometry.radius_compact),
                (UiRadiusToken::Control, geometry.radius_control),
                (UiRadiusToken::Panel, geometry.radius_panel),
                (UiRadiusToken::Floating, geometry.radius_floating),
            ],
        )
    }

    fn legacy_spacing_token(self, value: f32) -> (UiSpacingToken, bool) {
        let (geometry, _) = self.resolved_geometry_tokens();
        let base = geometry.spacing_base;
        nearest_token(
            value,
            &[
                (UiSpacingToken::None, 0.0),
                (UiSpacingToken::ThreeQuarter, base * 0.75),
                (UiSpacingToken::Base, base),
                (UiSpacingToken::FiveQuarter, base * 1.25),
                (UiSpacingToken::ThreeHalf, base * 1.5),
                (UiSpacingToken::Double, base * 2.0),
                (UiSpacingToken::FiveHalf, base * 2.5),
                (UiSpacingToken::Triple, base * 3.0),
                (UiSpacingToken::SevenHalf, base * 3.5),
                (UiSpacingToken::Quadruple, base * 4.0),
                (UiSpacingToken::Sextuple, base * 6.0),
            ],
        )
    }

    fn radius_value(self, token: UiRadiusToken) -> f32 {
        let (geometry, _) = self.resolved_geometry_tokens();
        match token {
            UiRadiusToken::None => 0.0,
            UiRadiusToken::Compact => geometry.radius_compact,
            UiRadiusToken::Control => geometry.radius_control,
            UiRadiusToken::Panel => geometry.radius_panel,
            UiRadiusToken::Floating => geometry.radius_floating,
        }
    }

    fn spacing_value(self, token: UiSpacingToken) -> f32 {
        let (geometry, _) = self.resolved_geometry_tokens();
        let base = geometry.spacing_base;
        match token {
            UiSpacingToken::None => 0.0,
            UiSpacingToken::ThreeQuarter => base * 0.75,
            UiSpacingToken::Base => base,
            UiSpacingToken::FiveQuarter => base * 1.25,
            UiSpacingToken::ThreeHalf => base * 1.5,
            UiSpacingToken::Double => base * 2.0,
            UiSpacingToken::FiveHalf => base * 2.5,
            UiSpacingToken::Triple => base * 3.0,
            UiSpacingToken::SevenHalf => base * 3.5,
            UiSpacingToken::Quadruple => base * 4.0,
            UiSpacingToken::Sextuple => base * 6.0,
        }
    }
}

fn color_for_role(tokens: UiColorTokens, role: UiColorRole) -> UiColor {
    match role {
        UiColorRole::Background => tokens.background,
        UiColorRole::Surface => tokens.surface,
        UiColorRole::Border => tokens.border,
        UiColorRole::PrimaryText => tokens.primary_text,
        UiColorRole::SecondaryText => tokens.secondary_text,
        UiColorRole::Muted => tokens.muted,
        UiColorRole::Destructive => tokens.destructive,
        UiColorRole::DestructiveHover => tokens.destructive_hover,
        UiColorRole::Positive => tokens.positive,
        UiColorRole::Emphasis => tokens.warning,
    }
}

fn color_is_valid(color: UiColor) -> bool {
    [color.red, color.green, color.blue, color.alpha]
        .into_iter()
        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
}

fn color_rgb_near(left: UiColor, right: UiColor) -> bool {
    (left.red - right.red).abs() <= 0.002
        && (left.green - right.green).abs() <= 0.002
        && (left.blue - right.blue).abs() <= 0.002
}

fn metric_within_locked_bound(value: f32, locked: f32, allow_zero: bool) -> bool {
    let minimum = if allow_zero { 0.0 } else { f32::EPSILON };
    value.is_finite() && value >= minimum && value <= locked * MAX_SUPPORTED_SCALE_FACTOR
}

fn nearest_token<T: Copy>(value: f32, candidates: &[(T, f32)]) -> (T, bool) {
    let mut nearest = candidates[0];
    let mut distance = (value - nearest.1).abs();
    for candidate in &candidates[1..] {
        let candidate_distance = (value - candidate.1).abs();
        if candidate_distance < distance {
            nearest = *candidate;
            distance = candidate_distance;
        }
    }
    (nearest.0, !value.is_finite() || distance > 0.01)
}

fn legacy_text_size_token(value: f32) -> (UiTextSizeToken, bool) {
    nearest_token(
        value,
        &[
            (UiTextSizeToken::Caption, 11.0),
            (UiTextSizeToken::Small, 12.0),
            (UiTextSizeToken::Metadata, 13.0),
            (UiTextSizeToken::CompactBody, 14.0),
            (UiTextSizeToken::Brand, 15.0),
            (UiTextSizeToken::Body, 16.0),
            (UiTextSizeToken::Title, 20.0),
            (UiTextSizeToken::Display, 28.0),
        ],
    )
}

const fn text_size_value(token: UiTextSizeToken) -> f32 {
    match token {
        UiTextSizeToken::Caption => 11.0,
        UiTextSizeToken::Small => 12.0,
        UiTextSizeToken::Metadata => 13.0,
        UiTextSizeToken::CompactBody => 14.0,
        UiTextSizeToken::Brand => 15.0,
        UiTextSizeToken::Body => 16.0,
        UiTextSizeToken::Title => 20.0,
        UiTextSizeToken::Display => 28.0,
    }
}

/// One retained node.  Children are ordered for traversal, focus, and layout.
#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    pub id: UiNodeId,
    pub kind: UiWidgetKind,
    pub layout: UiLayout,
    /// Authoritative token/variant styling contract.
    pub style_reference: UiStyleReference,
    /// Compatibility request retained for `LegacyTokenResolved` callers.
    pub style: UiStyle,
    pub layout_hints: UiLayoutHints,
    pub constraints: UiConstraints,
    pub absolute_position: Option<UiAbsolutePosition>,
    pub icon: Option<IconId>,
    pub font_role: UiFontRole,
    /// Retained presentation intent; never authoritative geometry or state.
    pub presentation: UiPresentationOptions,
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::Panel),
            style: UiStyle::panel(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::Text),
            style: UiStyle::text(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::SecondaryAction),
            style: UiStyle::secondary_action(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::TextField),
            style: UiStyle::text_field(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
        self.style_reference = UiStyleReference::LegacyTokenResolved;
        self.style = style;
        self
    }

    /// Selects an active-theme style variant without exposing raw renderer values.
    #[must_use]
    pub const fn with_style_variant(mut self, variant: UiStyleVariant) -> Self {
        self.style_reference = UiStyleReference::Variant(variant);
        self.style = variant.compatibility_style();
        self
    }

    /// Selects one locked bundled typography role without exposing font handles.
    #[must_use]
    pub const fn with_font_role(mut self, font_role: UiFontRole) -> Self {
        self.font_role = font_role;
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

    /// Permits only physical-panel or shared-element presentation interpolation.
    #[must_use]
    pub const fn with_spatial_motion(mut self, kind: UiSpatialMotionKind) -> Self {
        self.presentation.spatial_motion = Some(kind);
        self
    }

    /// Pairs this retained node with a prior or future shared-element target.
    ///
    /// The identity remains distinct from `UiNodeId` so relocation can bridge
    /// an outgoing and incoming retained node without changing source or
    /// interaction authority.
    #[must_use]
    pub const fn with_shared_element_motion(mut self, shared_element: UiSharedElementId) -> Self {
        self.presentation.spatial_motion = Some(UiSpatialMotionKind::SharedElement);
        self.presentation.shared_element = Some(shared_element);
        self
    }

    /// Sets a bounded retained opacity target for runtime presentation.
    ///
    /// Document validation rejects non-finite values and values outside the
    /// closed unit interval before a frame can observe them.
    #[must_use]
    pub const fn with_presentation_opacity(mut self, opacity: f32) -> Self {
        self.presentation.opacity = opacity;
        self
    }

    /// Applies bounded built-in validation to a retained text control.
    #[must_use]
    pub const fn with_text_validation(mut self, validation: UiTextValidation) -> Self {
        self.text_validation = Some(validation);
        self
    }

    /// Adds bounded explanatory text to this node's semantic contract.
    #[must_use]
    pub fn with_semantic_description(mut self, description: impl Into<String>) -> Self {
        self.semantics.description = Some(description.into());
        self
    }

    /// Exposes one host-bound assistive operation only when a canonical typed
    /// command can carry it across the frame barrier.
    ///
    /// Validation rejects duplicate, malformed, or role-incompatible bindings
    /// before an adapter can advertise an action it cannot execute.
    #[must_use]
    pub fn with_assistive_action(
        mut self,
        action: UiHostAssistiveAction,
        command: impl Into<String>,
    ) -> Self {
        self.semantics
            .assistive_actions
            .push(UiAssistiveActionBinding {
                action,
                command: command.into(),
            });
        self
    }

    /// Declares validated non-structural semantic relationships for this node.
    #[must_use]
    pub fn with_semantic_relationships(mut self, relationships: UiSemanticRelationships) -> Self {
        self.semantics.relationships = relationships;
        self
    }

    /// Publishes this realized row's position without realizing its full
    /// collection for assistive technology.
    #[must_use]
    pub const fn with_semantic_collection_item(
        mut self,
        collection_item: UiSemanticCollectionItem,
    ) -> Self {
        self.semantics.collection_item = Some(collection_item);
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::SecondaryAction),
            style: UiStyle::secondary_action(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
            style_reference: UiStyleReference::Variant(UiStyleVariant::Surface),
            style: UiStyle::surface(),
            layout_hints: UiLayoutHints::default(),
            constraints: UiConstraints::default(),
            absolute_position: None,
            icon: None,
            font_role: UiFontRole::Interface,
            presentation: UiPresentationOptions::default(),
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
        let mut node = Self::button(id, name.clone(), action, value)
            .with_style_variant(UiStyleVariant::TextField);
        node.kind = UiWidgetKind::ComboBox;
        node.layout = UiLayout::VerticalStack { gap: 0.0 };
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
    InteractiveNodeNotFocusable(UiNodeId),
    MissingButtonAction(UiNodeId),
    InvalidCommandName(UiNodeId),
    TooManyAssistiveActionBindings {
        node: UiNodeId,
        count: usize,
        maximum: usize,
    },
    DuplicateAssistiveActionBinding {
        node: UiNodeId,
        action: UiHostAssistiveAction,
    },
    InvalidAssistiveActionBinding {
        node: UiNodeId,
        action: UiHostAssistiveAction,
    },
    TextInputNotFocusable(UiNodeId),
    MissingTextInputOptions(UiNodeId),
    UnexpectedTextInputOptions(UiNodeId),
    UnexpectedTextValidation(UiNodeId),
    PasswordInitialValue(UiNodeId),
    PasswordSemanticValue(UiNodeId),
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
    InvalidPresentationOpacity(UiNodeId),
    SharedElementIdentityMissing(UiNodeId),
    UnexpectedSharedElementIdentity(UiNodeId),
    DuplicateSharedElementIdentity {
        first: UiNodeId,
        second: UiNodeId,
        shared_element: UiSharedElementId,
    },
    SemanticTextTooLong {
        node: UiNodeId,
        field: UiSemanticField,
        bytes: usize,
        maximum: usize,
    },
    SemanticRelationshipMissingTarget {
        node: UiNodeId,
        relationship: UiSemanticRelationshipKind,
        target: UiNodeId,
    },
    SemanticRelationshipSelfReference {
        node: UiNodeId,
        relationship: UiSemanticRelationshipKind,
    },
    DuplicateSemanticRelationship {
        node: UiNodeId,
        relationship: UiSemanticRelationshipKind,
        target: UiNodeId,
    },
    TooManySemanticRelationshipTargets {
        node: UiNodeId,
        relationship: UiSemanticRelationshipKind,
        count: usize,
        maximum: usize,
    },
    InvalidSemanticCollectionItem(UiNodeId),
}

/// Bounded semantic string field reported by document validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSemanticField {
    Name,
    Description,
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
        Self::validate_presentation_identities(&by_id)?;
        for (id, node) in &by_id {
            Self::validate_semantic_relationships(*id, node, &by_id)?;
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
            node.style.corner_radius,
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
        if !node.presentation.opacity.is_finite()
            || !(0.0..=1.0).contains(&node.presentation.opacity)
        {
            return Err(UiDocumentError::InvalidPresentationOpacity(id));
        }
        Ok(())
    }

    fn validate_presentation_identities(
        nodes: &BTreeMap<UiNodeId, UiNode>,
    ) -> Result<(), UiDocumentError> {
        let mut shared_elements = BTreeMap::new();
        for (id, node) in nodes {
            match (
                node.presentation.spatial_motion,
                node.presentation.shared_element,
            ) {
                (Some(UiSpatialMotionKind::SharedElement), None) => {
                    return Err(UiDocumentError::SharedElementIdentityMissing(*id));
                }
                (Some(UiSpatialMotionKind::SharedElement), Some(shared_element)) => {
                    if let Some(first) = shared_elements.insert(shared_element, *id) {
                        return Err(UiDocumentError::DuplicateSharedElementIdentity {
                            first,
                            second: *id,
                            shared_element,
                        });
                    }
                }
                (None | Some(UiSpatialMotionKind::PhysicalPanel), Some(_)) => {
                    return Err(UiDocumentError::UnexpectedSharedElementIdentity(*id));
                }
                (None | Some(UiSpatialMotionKind::PhysicalPanel), None) => {}
            }
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
            (
                UiSemanticField::Description,
                node.semantics.description.as_deref(),
            ),
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
        if node
            .semantics
            .action
            .as_deref()
            .is_some_and(|action| CommandId::from_name(action).is_none())
        {
            return Err(UiDocumentError::InvalidCommandName(id));
        }
        let bindings = &node.semantics.assistive_actions;
        if bindings.len() > MAX_ASSISTIVE_ACTION_BINDINGS {
            return Err(UiDocumentError::TooManyAssistiveActionBindings {
                node: id,
                count: bindings.len(),
                maximum: MAX_ASSISTIVE_ACTION_BINDINGS,
            });
        }
        let mut bound_actions = BTreeSet::new();
        for binding in bindings {
            if !bound_actions.insert(binding.action) {
                return Err(UiDocumentError::DuplicateAssistiveActionBinding {
                    node: id,
                    action: binding.action,
                });
            }
            if CommandId::from_name(&binding.command).is_none()
                || !binding.action.is_compatible_with(
                    node.semantics.role,
                    node.semantics.state,
                    node.focusable,
                )
            {
                return Err(UiDocumentError::InvalidAssistiveActionBinding {
                    node: id,
                    action: binding.action,
                });
            }
        }
        if node.focusable && node.semantics.name.trim().is_empty() {
            return Err(UiDocumentError::UnnamedFocusable(id));
        }
        if node.semantics.action.is_some() && !node.focusable {
            return Err(UiDocumentError::InteractiveNodeNotFocusable(id));
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

    fn validate_semantic_relationships(
        id: UiNodeId,
        node: &UiNode,
        nodes: &BTreeMap<UiNodeId, UiNode>,
    ) -> Result<(), UiDocumentError> {
        let relationships = &node.semantics.relationships;
        for (relationship, targets) in [
            (
                UiSemanticRelationshipKind::LabelledBy,
                relationships.labelled_by.as_slice(),
            ),
            (
                UiSemanticRelationshipKind::DescribedBy,
                relationships.described_by.as_slice(),
            ),
            (
                UiSemanticRelationshipKind::Controls,
                relationships.controls.as_slice(),
            ),
            (
                UiSemanticRelationshipKind::Details,
                relationships.details.as_slice(),
            ),
            (
                UiSemanticRelationshipKind::FlowTo,
                relationships.flow_to.as_slice(),
            ),
        ] {
            if targets.len() > MAX_RETAINED_NODES {
                return Err(UiDocumentError::TooManySemanticRelationshipTargets {
                    node: id,
                    relationship,
                    count: targets.len(),
                    maximum: MAX_RETAINED_NODES,
                });
            }
            let mut seen = BTreeSet::new();
            for target in targets {
                Self::validate_semantic_relationship_target(
                    id,
                    relationship,
                    *target,
                    nodes,
                    &mut seen,
                )?;
            }
        }
        if let Some(target) = relationships.error_message {
            let mut seen = BTreeSet::new();
            Self::validate_semantic_relationship_target(
                id,
                UiSemanticRelationshipKind::ErrorMessage,
                target,
                nodes,
                &mut seen,
            )?;
        }
        if let Some(item) = node.semantics.collection_item {
            let maximum = u32::from(u16::MAX);
            if item.position == 0
                || item.set_size == 0
                || item.position > item.set_size
                || item.set_size > maximum
            {
                return Err(UiDocumentError::InvalidSemanticCollectionItem(id));
            }
        }
        Ok(())
    }

    fn validate_semantic_relationship_target(
        node: UiNodeId,
        relationship: UiSemanticRelationshipKind,
        target: UiNodeId,
        nodes: &BTreeMap<UiNodeId, UiNode>,
        seen: &mut BTreeSet<UiNodeId>,
    ) -> Result<(), UiDocumentError> {
        if node == target {
            return Err(UiDocumentError::SemanticRelationshipSelfReference { node, relationship });
        }
        if !nodes.contains_key(&target) {
            return Err(UiDocumentError::SemanticRelationshipMissingTarget {
                node,
                relationship,
                target,
            });
        }
        if !seen.insert(target) {
            return Err(UiDocumentError::DuplicateSemanticRelationship {
                node,
                relationship,
                target,
            });
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
        if node.text_input.is_some_and(|options| options.password) && node.semantics.value.is_some()
        {
            return Err(UiDocumentError::PasswordSemanticValue(id));
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
        assert_token(theme.icons.size, 16.0);
        assert_token(theme.icons.stroke_width, 2.0);
        assert_token(theme.icons.text_gap, 8.0);
        assert_eq!(theme.interface_font.family, "Mona Sans");
        assert_eq!(theme.display_font.family, "Hubot Sans");
        assert_eq!(theme.monospace_font.family, "JetBrains Mono");
    }

    #[test]
    fn huge_finite_icon_and_geometry_metrics_fall_back_to_registered_tokens() {
        let locked = UiTheme::meridian_dark();
        let assert_token = |actual: f32, expected: f32| {
            assert!((actual - expected).abs() < f32::EPSILON);
        };
        let mut theme = locked;
        theme.icons = UiIconTokens {
            size: f32::MAX,
            stroke_width: f32::MAX,
            text_gap: f32::MAX,
        };
        theme.geometry.spacing_base = f32::MAX;
        theme.geometry.radius_compact = f32::MAX;
        theme.geometry.radius_control = f32::MAX;
        theme.geometry.radius_panel = f32::MAX;
        theme.geometry.radius_floating = f32::MAX;

        let (icons, icon_fallback) = theme.resolved_icon_tokens();
        assert!(icon_fallback);
        assert_eq!(icons, locked.icons);

        let (geometry, geometry_fallback) = theme.resolved_geometry_tokens();
        assert!(geometry_fallback);
        assert_token(geometry.spacing_base, locked.geometry.spacing_base);
        assert_token(geometry.radius_compact, locked.geometry.radius_compact);
        assert_token(geometry.radius_control, locked.geometry.radius_control);
        assert_token(geometry.radius_panel, locked.geometry.radius_panel);
        assert_token(geometry.radius_floating, locked.geometry.radius_floating);

        let resolution = theme.resolve_style(
            UiStyleReference::Variant(UiStyleVariant::SecondaryAction),
            UiStyle::secondary_action(),
            UiVisualState::default(),
            UiContrast::Standard,
        );
        assert!(resolution.used_token_fallback);
        assert_token(
            resolution.style.corner_radius,
            locked.geometry.radius_control,
        );
        assert_token(resolution.style.padding, locked.geometry.spacing_base * 2.5);
    }

    #[test]
    fn style_variants_and_legacy_requests_resolve_through_theme_tokens() {
        let theme = UiTheme::meridian_dark();
        let primary = theme.resolve_style(
            UiStyleReference::Variant(UiStyleVariant::PrimaryAction),
            UiStyle::destructive_action(),
            UiVisualState::default(),
            UiContrast::Standard,
        );
        assert_eq!(primary.style.background, Some(theme.colors.surface));
        assert_eq!(
            primary.style.border.map(|border| border.color),
            Some(theme.colors.warning)
        );
        assert_ne!(primary.style.background, Some(theme.colors.destructive));
        assert!(!primary.used_token_fallback);

        let legacy = theme.resolve_style(
            UiStyleReference::LegacyTokenResolved,
            UiStyle {
                background: Some(UiColor::rgba(0.2, 0.4, 0.6, 1.0)),
                padding: 9.0,
                ..UiStyle::secondary_action()
            },
            UiVisualState::default(),
            UiContrast::Standard,
        );
        assert!(legacy.used_token_fallback);
        assert_eq!(legacy.style.background, Some(theme.colors.surface));
        assert!((legacy.style.padding - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn visual_selector_resolution_is_stable_and_high_contrast_is_role_mapped() {
        let theme = UiTheme::meridian_dark();
        let selected = theme.resolve_style(
            UiStyleReference::Variant(UiStyleVariant::SecondaryAction),
            UiStyle::secondary_action(),
            UiVisualState {
                hovered: true,
                focused: true,
                selected: true,
                ..UiVisualState::default()
            },
            UiContrast::High,
        );
        assert_eq!(selected.selector, UiStyleSelector::Selected);
        assert_eq!(
            selected.style.background,
            Some(theme.high_contrast_colors.surface)
        );
        assert_eq!(
            selected.style.border.map(|border| border.color),
            Some(theme.high_contrast_colors.positive)
        );

        let disabled_invalid = theme.resolve_style(
            UiStyleReference::Variant(UiStyleVariant::TextField),
            UiStyle::text_field(),
            UiVisualState {
                invalid: true,
                disabled: true,
                ..UiVisualState::default()
            },
            UiContrast::High,
        );
        assert_eq!(disabled_invalid.selector, UiStyleSelector::Disabled);
        assert_eq!(
            disabled_invalid.style.foreground,
            theme.high_contrast_colors.muted
        );
    }

    #[test]
    fn retained_nodes_select_locked_typography_by_meridian_role() {
        let node = UiNode::label(UiNodeId::new(7), "Build log", "compiled")
            .with_font_role(UiFontRole::Monospace);
        assert_eq!(node.font_role, UiFontRole::Monospace);
        assert_eq!(
            UiFontDescriptor::locked(node.font_role).family,
            "JetBrains Mono"
        );
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
    fn presentation_and_password_semantic_contracts_reject_unsafe_source() {
        let root = UiNodeId::new(70);
        let field = UiNodeId::new(71);
        let mut password = UiNode::text_input(
            field,
            "Recovery password",
            "",
            UiTextInputOptions { password: true },
        );
        password.semantics.value = Some("must-not-project".to_owned());
        assert_eq!(
            UiDocument::new(
                root,
                vec![
                    UiNode::container(root, "Recovery", UiLayout::Overlay, vec![field]),
                    password,
                ],
            ),
            Err(UiDocumentError::PasswordSemanticValue(field))
        );

        assert_eq!(
            UiDocument::new(
                root,
                vec![UiNode::label(root, "Opacity", "Invalid").with_presentation_opacity(1.01)],
            ),
            Err(UiDocumentError::InvalidPresentationOpacity(root))
        );

        let first = UiNodeId::new(72);
        let second = UiNodeId::new(73);
        let shared = UiSharedElementId::new(0xabc);
        assert_eq!(
            UiDocument::new(
                root,
                vec![
                    UiNode::container(root, "Shared", UiLayout::Overlay, vec![first, second]),
                    UiNode::label(first, "First", "First").with_shared_element_motion(shared),
                    UiNode::label(second, "Second", "Second").with_shared_element_motion(shared),
                ],
            ),
            Err(UiDocumentError::DuplicateSharedElementIdentity {
                first,
                second,
                shared_element: shared,
            })
        );
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
    fn retained_semantic_metadata_preserves_descriptions_relations_and_virtual_items() {
        let root = UiNodeId::new(100);
        let label = UiNodeId::new(101);
        let description = UiNodeId::new(102);
        let error = UiNodeId::new(103);
        let field = UiNodeId::new(104);
        let row = UiNodeId::new(105);
        let relationships = UiSemanticRelationships {
            labelled_by: vec![label],
            described_by: vec![description],
            controls: vec![row],
            details: vec![error],
            flow_to: vec![row],
            error_message: Some(error),
        };
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Semantic metadata",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![label, description, error, field, row],
                ),
                UiNode::label(label, "Project label", "Project label"),
                UiNode::tooltip(description, "Project help", "Used in the title bar"),
                UiNode::label(error, "Project error", "Project names must be unique"),
                UiNode::text_input(
                    field,
                    "Project name",
                    "Creator Alpha",
                    UiTextInputOptions::default(),
                )
                .with_control_state(UiControlState {
                    invalid: true,
                    ..UiControlState::default()
                })
                .with_semantic_description("The saved project display name")
                .with_semantic_relationships(relationships.clone()),
                UiNode::list_item(row, "Scene", "scene.open", false).with_semantic_collection_item(
                    UiSemanticCollectionItem {
                        position: 3,
                        set_size: 9,
                    },
                ),
            ],
        )
        .expect("valid semantic metadata is retained");
        let field = document.node(field).expect("field is retained");
        assert_eq!(
            field.semantics.description.as_deref(),
            Some("The saved project display name")
        );
        assert_eq!(field.semantics.relationships, relationships);
        assert_eq!(
            document
                .node(row)
                .expect("row is retained")
                .semantics
                .collection_item,
            Some(UiSemanticCollectionItem {
                position: 3,
                set_size: 9,
            })
        );
    }

    #[test]
    fn malformed_semantic_relationships_and_collection_items_are_rejected() {
        let root = UiNodeId::new(110);
        let field = UiNodeId::new(111);
        let valid_root =
            || UiNode::container(root, "Semantic metadata", UiLayout::Overlay, vec![field]);
        let valid_field =
            || UiNode::text_input(field, "Project name", "", UiTextInputOptions::default());

        let missing = valid_field().with_semantic_relationships(UiSemanticRelationships {
            described_by: vec![UiNodeId::new(999)],
            ..UiSemanticRelationships::default()
        });
        assert_eq!(
            UiDocument::new(root, vec![valid_root(), missing]),
            Err(UiDocumentError::SemanticRelationshipMissingTarget {
                node: field,
                relationship: UiSemanticRelationshipKind::DescribedBy,
                target: UiNodeId::new(999),
            })
        );

        let self_reference = valid_field().with_semantic_relationships(UiSemanticRelationships {
            controls: vec![field],
            ..UiSemanticRelationships::default()
        });
        assert_eq!(
            UiDocument::new(root, vec![valid_root(), self_reference]),
            Err(UiDocumentError::SemanticRelationshipSelfReference {
                node: field,
                relationship: UiSemanticRelationshipKind::Controls,
            })
        );

        let duplicate = valid_field().with_semantic_relationships(UiSemanticRelationships {
            flow_to: vec![root, root],
            ..UiSemanticRelationships::default()
        });
        assert_eq!(
            UiDocument::new(root, vec![valid_root(), duplicate]),
            Err(UiDocumentError::DuplicateSemanticRelationship {
                node: field,
                relationship: UiSemanticRelationshipKind::FlowTo,
                target: root,
            })
        );

        let too_many = valid_field().with_semantic_relationships(UiSemanticRelationships {
            controls: vec![root; MAX_RETAINED_NODES + 1],
            ..UiSemanticRelationships::default()
        });
        assert_eq!(
            UiDocument::new(root, vec![valid_root(), too_many]),
            Err(UiDocumentError::TooManySemanticRelationshipTargets {
                node: field,
                relationship: UiSemanticRelationshipKind::Controls,
                count: MAX_RETAINED_NODES + 1,
                maximum: MAX_RETAINED_NODES,
            })
        );

        let invalid_item = UiNode::list_item(field, "Scene", "scene.open", false)
            .with_semantic_collection_item(UiSemanticCollectionItem {
                position: 0,
                set_size: 1,
            });
        assert_eq!(
            UiDocument::new(root, vec![valid_root(), invalid_item]),
            Err(UiDocumentError::InvalidSemanticCollectionItem(field))
        );
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
    fn host_assistive_bindings_are_explicit_bounded_and_role_validated() {
        let node = UiNodeId::new(12);
        let valid = UiNode::tree_item(node, "Camera", "world.select-camera", false, false)
            .with_assistive_action(UiHostAssistiveAction::Expand, "world.expand-camera")
            .with_assistive_action(
                UiHostAssistiveAction::ShowContextMenu,
                "world.camera-context",
            );
        assert!(UiDocument::new(node, vec![valid]).is_ok());

        let context_only = UiNode::container(node, "Camera", UiLayout::Overlay, Vec::new())
            .with_focusable(true)
            .with_assistive_action(
                UiHostAssistiveAction::ShowContextMenu,
                "world.camera-context",
            );
        assert!(UiDocument::new(node, vec![context_only]).is_ok());

        let duplicate = UiNode::tree_item(node, "Camera", "world.select-camera", false, false)
            .with_assistive_action(UiHostAssistiveAction::Expand, "world.expand-camera")
            .with_assistive_action(UiHostAssistiveAction::Expand, "world.expand-camera-again");
        assert_eq!(
            UiDocument::new(node, vec![duplicate]),
            Err(UiDocumentError::DuplicateAssistiveActionBinding {
                node,
                action: UiHostAssistiveAction::Expand,
            })
        );

        let incompatible = UiNode::tree_item(node, "Camera", "world.select-camera", false, true)
            .with_assistive_action(UiHostAssistiveAction::Expand, "world.expand-camera");
        assert_eq!(
            UiDocument::new(node, vec![incompatible]),
            Err(UiDocumentError::InvalidAssistiveActionBinding {
                node,
                action: UiHostAssistiveAction::Expand,
            })
        );

        let non_focusable = UiNode::tree_item(node, "Camera", "world.select-camera", false, false)
            .with_focusable(false)
            .with_assistive_action(UiHostAssistiveAction::Expand, "world.expand-camera");
        assert_eq!(
            UiDocument::new(node, vec![non_focusable]),
            Err(UiDocumentError::InvalidAssistiveActionBinding {
                node,
                action: UiHostAssistiveAction::Expand,
            })
        );

        let malformed = UiNode::tree_item(node, "Camera", "world.select-camera", false, false)
            .with_assistive_action(UiHostAssistiveAction::Expand, "world expand camera");
        assert_eq!(
            UiDocument::new(node, vec![malformed]),
            Err(UiDocumentError::InvalidAssistiveActionBinding {
                node,
                action: UiHostAssistiveAction::Expand,
            })
        );
    }

    #[test]
    fn interactive_nodes_must_be_keyboard_focusable() {
        let button = UiNode::button(UiNodeId::new(14), "Build project", "build.start", "Build")
            .with_focusable(false);
        assert_eq!(
            UiDocument::new(UiNodeId::new(14), vec![button]),
            Err(UiDocumentError::InteractiveNodeNotFocusable(UiNodeId::new(
                14
            )))
        );
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

    #[test]
    fn command_ids_are_deterministic_and_reject_malformed_names() {
        let first = CommandId::from_name("workspace.world").expect("canonical command name");
        let same = CommandId::from_name("workspace.world").expect("same command name");
        let other = CommandId::from_name("workspace.modeler").expect("other command name");
        assert_eq!(first, same);
        assert_ne!(first, other);
        assert!(CommandId::from_name("").is_none());
        assert!(CommandId::from_name("workspace world").is_none());
        assert!(CommandId::from_name("workspace.world!").is_none());
        assert!(CommandId::from_name(&"a".repeat(MAX_COMMAND_NAME_BYTES + 1)).is_none());
    }

    #[test]
    fn retained_documents_reject_malformed_command_names_before_runtime() {
        let node = UiNodeId::new(91);
        let invalid = UiNode::button(node, "Build", "build start", "Build");
        assert_eq!(
            UiDocument::new(node, vec![invalid]),
            Err(UiDocumentError::InvalidCommandName(node))
        );
    }
}
