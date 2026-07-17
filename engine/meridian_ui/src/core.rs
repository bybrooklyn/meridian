//! Retained, renderer-independent UI contracts.
//!
//! The data crossing this crate boundary is Meridian-owned.  Platform and
//! renderer adapters consume [`DisplayList`] and [`SemanticTree`] rather than
//! borrowing widget state or exposing their native types here.

use std::collections::{BTreeMap, BTreeSet};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use meridian_core::StableId;
use unicode_segmentation::UnicodeSegmentation;

const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_GLYPH_RASTER_BYTES: usize = 1024 * 1024;

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
    fn sanitized(self) -> Self {
        Self::new(self.width.max(1.0), self.height.max(1.0))
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

/// Widget behavior is intentionally a small, retained set for the MS-02 seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWidgetKind {
    Panel,
    Label,
    Button,
    TextInput,
    Overlay,
}

/// Layout policy used by the retained document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiLayout {
    Overlay,
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
    TextInput,
}

/// Named semantics and a typed action token declared by a UI node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSemantics {
    pub role: SemanticRole,
    pub name: String,
    pub action: Option<String>,
}

impl UiSemantics {
    #[must_use]
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Group,
            name: name.into(),
            action: None,
        }
    }

    #[must_use]
    pub fn status(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Status,
            name: name.into(),
            action: None,
        }
    }

    #[must_use]
    pub fn button(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::Button,
            name: name.into(),
            action: Some(action.into()),
        }
    }

    #[must_use]
    pub fn text_input(name: impl Into<String>) -> Self {
        Self {
            role: SemanticRole::TextInput,
            name: name.into(),
            action: None,
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
}

/// Invalid retained documents are rejected before rendering or accessibility output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiDocumentError {
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
}

/// Validated retained UI document.
#[derive(Clone, Debug)]
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
            if node.focusable && node.semantics.name.trim().is_empty() {
                return Err(UiDocumentError::UnnamedFocusable(*id));
            }
            if node.kind == UiWidgetKind::Button && node.semantics.action.is_none() {
                return Err(UiDocumentError::MissingButtonAction(*id));
            }
            if node.kind == UiWidgetKind::TextInput && !node.focusable {
                return Err(UiDocumentError::TextInputNotFocusable(*id));
            }
            if node.kind == UiWidgetKind::TextInput && node.text_input.is_none() {
                return Err(UiDocumentError::MissingTextInputOptions(*id));
            }
            if node.kind != UiWidgetKind::TextInput && node.text_input.is_some() {
                return Err(UiDocumentError::UnexpectedTextInputOptions(*id));
            }
            if node.text_input.is_some_and(|options| options.password)
                && node.text.as_ref().is_some_and(|text| !text.is_empty())
            {
                return Err(UiDocumentError::PasswordInitialValue(*id));
            }
            if let Some(text) = &node.text {
                if text.len() > MAX_TEXT_BYTES {
                    return Err(UiDocumentError::TextTooLong {
                        node: *id,
                        bytes: text.len(),
                        maximum: MAX_TEXT_BYTES,
                    });
                }
            }
            let mut children = BTreeSet::new();
            for child in &node.children {
                if !children.insert(*child) {
                    return Err(UiDocumentError::DuplicateChild(*child));
                }
                if !by_id.contains_key(child) {
                    return Err(UiDocumentError::MissingChild {
                        parent: *id,
                        child: *child,
                    });
                }
                if parents.insert(*child, *id).is_some() {
                    return Err(UiDocumentError::MultipleParents(*child));
                }
            }
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

/// Platform-normalized event delivered to the retained interaction model.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    FocusNext,
    FocusPrevious,
    Activate,
    TextCommit(String),
    ImePreedit {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    MoveTextCursor {
        direction: UiTextCursorDirection,
        extend_selection: bool,
    },
    DeleteTextBackward,
    DeleteTextForward,
    SelectAllText,
    CopySelection,
    PointerDown(UiPoint),
    PointerUp(UiPoint),
    PointerCancel,
    /// Requests focus for a named focusable control through a semantic adapter.
    ///
    /// This does not activate the control or expose its private text value. It
    /// exists so an accessibility adapter can use the same focus model as
    /// keyboard and pointer input.
    AssistiveFocus(UiNodeId),
    AssistiveActivate(UiNodeId),
}

/// Each route is represented explicitly so adapters can audit dispatch behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiEventPhase {
    Capture,
    Target,
    Bubble,
}

/// A single dispatch observation, without any borrowed platform event state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiEventRoute {
    pub phase: UiEventPhase,
    pub node: UiNodeId,
}

/// The only state-changing result emitted by a UI interaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCommandRequest {
    pub source: UiNodeId,
    pub action: String,
}

/// A cursor movement expressed in extended-grapheme positions, not UTF-8 bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextCursorDirection {
    Backward,
    Forward,
    Start,
    End,
}

/// A half-open text selection in extended-grapheme positions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTextSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl UiTextSelection {
    #[must_use]
    pub const fn cursor(position: usize) -> Self {
        Self {
            anchor: position,
            focus: position,
        }
    }

    #[must_use]
    pub const fn start(self) -> usize {
        if self.anchor < self.focus {
            self.anchor
        } else {
            self.focus
        }
    }

    #[must_use]
    pub const fn end(self) -> usize {
        if self.anchor > self.focus {
            self.anchor
        } else {
            self.focus
        }
    }

    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

/// Redacted observable editing state for one retained text-input node.
///
/// It intentionally reports no text value, so password text cannot escape
/// through frame output, semantic output, or diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTextInputSnapshot {
    pub node: UiNodeId,
    pub selection: UiTextSelection,
    pub grapheme_count: usize,
    pub password: bool,
    pub has_preedit: bool,
}

/// A capability-gated request for a platform clipboard adapter.
///
/// The adapter must obtain normal clipboard permission before performing it.
/// Password inputs never generate this request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiClipboardRequest {
    pub source: UiNodeId,
    pub text: String,
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
    FocusRing {
        node: UiNodeId,
        bounds: UiRect,
        color: UiColor,
    },
}

/// Immutable frame display output.  It never contains GPU or text-adapter types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub primitives: Vec<DisplayPrimitive>,
}

/// Owned layout statistics, not glyphs or adapter structures.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiTextLayout {
    pub line_count: usize,
    pub glyph_count: usize,
    pub width: f32,
    pub height: f32,
    pub used_fallback_metrics: bool,
}

/// One alpha-mask glyph bitmap relative to its text primitive's origin.
#[derive(Clone, Debug, PartialEq)]
pub struct UiGlyphBitmap {
    pub origin: UiPoint,
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}

/// Meridian-owned text raster data. It exposes no font or shaping-library types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiTextRaster {
    pub glyphs: Vec<UiGlyphBitmap>,
    pub has_unrasterized_glyphs: bool,
}

/// Flat semantic tree; platform adapters turn this into their native tree/delta.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: UiNodeId,
    pub parent: Option<UiNodeId>,
    pub role: SemanticRole,
    pub name: String,
    pub action: Option<String>,
    pub bounds: UiRect,
    pub focused: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SemanticTree {
    pub nodes: Vec<SemanticNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticDelta {
    Unchanged,
    Replace(SemanticTree),
}

/// Non-fatal behavior reported to diagnostics and tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiDiagnostic {
    NoFocusableNode,
    PointerOutsideDocument,
    TextFallbackMetrics { node: UiNodeId },
    TextRasterIncomplete { node: UiNodeId },
    TextInputNotFocused,
    TextInputLimitExceeded { node: UiNodeId, maximum: usize },
    ClipboardDeniedForPassword { node: UiNodeId },
    AssistiveFocusDenied { node: UiNodeId },
}

/// Input captured at a stable frame boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameInput {
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub events: Vec<UiEvent>,
}

impl UiFrameInput {
    #[must_use]
    pub fn new(viewport: UiSize) -> Self {
        Self {
            viewport,
            scale_factor: 1.0,
            high_contrast: false,
            reduced_motion: false,
            events: Vec::new(),
        }
    }
}

/// Immutable frame result handed to a renderer and semantic adapter.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameOutput {
    pub display_list: DisplayList,
    pub semantic_delta: SemanticDelta,
    pub event_routes: Vec<UiEventRoute>,
    pub commands: Vec<UiCommandRequest>,
    pub clipboard_requests: Vec<UiClipboardRequest>,
    pub text_inputs: Vec<UiTextInputSnapshot>,
    pub diagnostics: Vec<UiDiagnostic>,
    pub focused: Option<UiNodeId>,
    pub preedit: Option<String>,
}

/// Private text adapter.  The public result is [`UiTextLayout`].
#[derive(Debug)]
struct UiTextEngine {
    fonts: FontSystem,
    swash: SwashCache,
}

impl Default for UiTextEngine {
    fn default() -> Self {
        let mut fonts = FontSystem::new();
        fonts.db_mut().load_system_fonts();
        Self {
            fonts,
            swash: SwashCache::new(),
        }
    }
}

impl UiTextEngine {
    fn layout(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
    ) -> UiTextOutput {
        let scale_factor = scale_factor.clamp(0.5, 4.0);
        let metrics = Metrics::relative((font_size * scale_factor).max(1.0), 1.25);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        buffer.set_size(Some((width * scale_factor).max(1.0)), None);
        buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
        let mut line_count = 0;
        let mut glyph_count = 0;
        let mut observed_width = 0.0_f32;
        let mut height = 0.0_f32;
        let mut physical_glyphs = Vec::new();
        {
            let mut borrowed = buffer.borrow_with(&mut self.fonts);
            for run in borrowed.layout_runs() {
                line_count += 1;
                glyph_count += run.glyphs.len();
                observed_width = observed_width.max(run.line_w);
                height += run.line_height;
                physical_glyphs.extend(
                    run.glyphs
                        .iter()
                        .map(|glyph| glyph.physical((0.0, run.line_y), 1.0)),
                );
            }
        }
        let used_fallback_metrics = line_count == 0 && !text.is_empty();
        if used_fallback_metrics {
            line_count = text.lines().count().max(1);
            observed_width =
                (bounded_count_as_f32(text.chars().count()) * font_size * 0.6).min(width.max(1.0));
            height = bounded_count_as_f32(line_count) * font_size * 1.25;
        } else {
            observed_width /= scale_factor;
            height /= scale_factor;
        }
        let layout = UiTextLayout {
            line_count,
            glyph_count,
            width: observed_width,
            height,
            used_fallback_metrics,
        };
        let mut raster = UiTextRaster::default();
        let mut raster_bytes = 0_usize;
        for glyph in physical_glyphs {
            let Some(image) = self.swash.get_image(&mut self.fonts, glyph.cache_key) else {
                raster.has_unrasterized_glyphs = true;
                continue;
            };
            if image.content != SwashContent::Mask {
                raster.has_unrasterized_glyphs = true;
                continue;
            }
            let width = image.placement.width;
            let height = image.placement.height;
            let Some(byte_count) = usize::try_from(width).ok().and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            }) else {
                raster.has_unrasterized_glyphs = true;
                continue;
            };
            if image.data.len() != byte_count
                || raster_bytes.saturating_add(byte_count) > MAX_GLYPH_RASTER_BYTES
            {
                raster.has_unrasterized_glyphs = true;
                break;
            }
            raster_bytes += byte_count;
            raster.glyphs.push(UiGlyphBitmap {
                origin: UiPoint {
                    x: i32_to_f32(glyph.x.saturating_add(image.placement.left)),
                    y: i32_to_f32(glyph.y.saturating_sub(image.placement.top)),
                },
                width,
                height,
                alpha: image.data.clone(),
            });
        }
        UiTextOutput { layout, raster }
    }
}

struct UiTextOutput {
    layout: UiTextLayout,
    raster: UiTextRaster,
}

fn grapheme_count(text: &str) -> usize {
    text.graphemes(true).count()
}

fn clamp_selection(selection: UiTextSelection, grapheme_count: usize) -> UiTextSelection {
    UiTextSelection {
        anchor: selection.anchor.min(grapheme_count),
        focus: selection.focus.min(grapheme_count),
    }
}

fn byte_index_at_grapheme(text: &str, position: usize) -> usize {
    text.grapheme_indices(true)
        .nth(position)
        .map_or(text.len(), |(byte_index, _)| byte_index)
}

fn selected_text(text: &str, selection: UiTextSelection) -> Option<&str> {
    let selection = clamp_selection(selection, grapheme_count(text));
    if selection.is_collapsed() {
        return None;
    }
    let start = byte_index_at_grapheme(text, selection.start());
    let end = byte_index_at_grapheme(text, selection.end());
    text.get(start..end)
}

fn replace_selection(state: &mut UiTextInputState, replacement: &str) -> bool {
    let selection = clamp_selection(state.selection, grapheme_count(&state.value));
    let start = byte_index_at_grapheme(&state.value, selection.start());
    let end = byte_index_at_grapheme(&state.value, selection.end());
    let retained_bytes = state.value.len().saturating_sub(end.saturating_sub(start));
    if replacement.len() > MAX_TEXT_BYTES.saturating_sub(retained_bytes) {
        return false;
    }
    state.value.replace_range(start..end, replacement);
    state.selection = UiTextSelection::cursor(selection.start() + grapheme_count(replacement));
    true
}

fn password_mask(text: &str) -> String {
    "•".repeat(grapheme_count(text))
}

fn bounded_count_as_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn inset_bounds(bounds: UiRect, inset: f32) -> UiRect {
    let inset = finite_nonnegative(inset).min(bounds.size.width.min(bounds.size.height) / 2.0);
    UiRect::new(
        UiPoint {
            x: bounds.origin.x + inset,
            y: bounds.origin.y + inset,
        },
        UiSize::new(
            (bounds.size.width - inset * 2.0).max(0.0),
            (bounds.size.height - inset * 2.0).max(0.0),
        ),
    )
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

struct UiEmission<'a> {
    layout: &'a BTreeMap<UiNodeId, UiRect>,
    scale_factor: f32,
    high_contrast: bool,
    display: &'a mut DisplayList,
    semantic_nodes: &'a mut Vec<SemanticNode>,
    diagnostics: &'a mut Vec<UiDiagnostic>,
}

#[derive(Clone, Debug)]
struct UiTextInputState {
    value: String,
    selection: UiTextSelection,
    preedit: Option<(String, Option<(usize, usize)>)>,
    password: bool,
}

impl UiTextInputState {
    fn snapshot(&self, node: UiNodeId) -> UiTextInputSnapshot {
        UiTextInputSnapshot {
            node,
            selection: self.selection,
            grapheme_count: grapheme_count(&self.value),
            password: self.password,
            has_preedit: self.preedit.is_some(),
        }
    }
}

/// Retained runtime state.  All mutation is applied between immutable outputs.
#[derive(Debug)]
pub struct UiRuntime {
    document: UiDocument,
    text: UiTextEngine,
    text_inputs: BTreeMap<UiNodeId, UiTextInputState>,
    focused: Option<UiNodeId>,
    pointer_capture: Option<UiNodeId>,
    previous_semantics: Option<SemanticTree>,
}

impl UiRuntime {
    #[must_use]
    pub fn new(document: UiDocument) -> Self {
        let text_inputs = document
            .nodes
            .values()
            .filter_map(|node| {
                node.text_input.map(|options| {
                    (
                        node.id,
                        UiTextInputState {
                            value: node.text.clone().unwrap_or_default(),
                            selection: UiTextSelection::default(),
                            preedit: None,
                            password: options.password,
                        },
                    )
                })
            })
            .collect();
        Self {
            document,
            text: UiTextEngine::default(),
            text_inputs,
            focused: None,
            pointer_capture: None,
            previous_semantics: None,
        }
    }

    #[must_use]
    pub const fn document(&self) -> &UiDocument {
        &self.document
    }

    /// Replaces the retained document at a frame boundary while preserving
    /// compatible private text-input state by stable node ID.
    ///
    /// A caller may rebuild presentation from authoritative source without
    /// copying text values into that source or emitting them through semantics.
    pub fn replace_document(&mut self, document: UiDocument) {
        let previous_inputs = std::mem::take(&mut self.text_inputs);
        self.text_inputs = document
            .nodes
            .values()
            .filter_map(|node| {
                let options = node.text_input?;
                let state = previous_inputs
                    .get(&node.id)
                    .filter(|state| state.password == options.password);
                Some((
                    node.id,
                    state.cloned().unwrap_or(UiTextInputState {
                        value: if options.password {
                            String::new()
                        } else {
                            node.text.clone().unwrap_or_default()
                        },
                        selection: UiTextSelection::default(),
                        preedit: None,
                        password: options.password,
                    }),
                ))
            })
            .collect();
        if self
            .focused
            .is_some_and(|id| !document.node(id).is_some_and(|node| node.focusable))
        {
            self.focused = None;
        }
        self.pointer_capture = None;
        self.previous_semantics = None;
        self.document = document;
    }

    /// Returns one non-password text value kept privately by the runtime.
    ///
    /// Password values are deliberately never exposed through this API.
    #[must_use]
    pub fn text_input_value(&self, node: UiNodeId) -> Option<&str> {
        self.text_inputs
            .get(&node)
            .filter(|state| !state.password)
            .map(|state| state.value.as_str())
    }

    /// Restores one non-password text control to the current document value.
    ///
    /// Documents normally preserve a user's in-progress text by stable node
    /// ID. Callers use this narrow reset only after an authoritative source
    /// change or selection change makes that retained draft stale. Password
    /// inputs are deliberately never restored from document text.
    pub fn reset_text_input_from_document(&mut self, node: UiNodeId) -> bool {
        let Some(default_value) = self
            .document
            .node(node)
            .filter(|document_node| {
                document_node
                    .text_input
                    .is_some_and(|options| !options.password)
            })
            .and_then(|document_node| document_node.text.as_deref())
        else {
            return false;
        };
        let Some(state) = self.text_inputs.get_mut(&node) else {
            return false;
        };
        default_value.clone_into(&mut state.value);
        state.selection = UiTextSelection::default();
        state.preedit = None;
        true
    }

    /// Processes events, resolves retained layout, and returns only immutable output.
    pub fn reconcile(&mut self, input: UiFrameInput) -> UiFrameOutput {
        let mut layout = BTreeMap::new();
        self.layout_node(
            self.document.root(),
            UiRect::new(UiPoint::default(), input.viewport.sanitized()),
            &mut layout,
        );
        let mut routes = Vec::new();
        let mut commands = Vec::new();
        let mut clipboard_requests = Vec::new();
        let mut diagnostics = Vec::new();
        for event in input.events {
            self.process_event(
                event,
                &layout,
                &mut routes,
                &mut commands,
                &mut clipboard_requests,
                &mut diagnostics,
            );
        }
        let mut display_list = DisplayList::default();
        let mut semantic_nodes = Vec::new();
        let mut emission = UiEmission {
            layout: &layout,
            scale_factor: input.scale_factor.clamp(0.5, 4.0),
            high_contrast: input.high_contrast,
            display: &mut display_list,
            semantic_nodes: &mut semantic_nodes,
            diagnostics: &mut diagnostics,
        };
        self.emit_node(self.document.root(), None, &mut emission);
        let tree = SemanticTree {
            nodes: semantic_nodes,
        };
        let semantic_delta = if self.previous_semantics.as_ref() == Some(&tree) {
            SemanticDelta::Unchanged
        } else {
            SemanticDelta::Replace(tree.clone())
        };
        self.previous_semantics = Some(tree);
        UiFrameOutput {
            display_list,
            semantic_delta,
            event_routes: routes,
            commands,
            clipboard_requests,
            text_inputs: self
                .text_inputs
                .iter()
                .map(|(node, state)| state.snapshot(*node))
                .collect(),
            diagnostics,
            focused: self.focused,
            preedit: self.focused_preedit(),
        }
    }

    fn layout_node(&self, id: UiNodeId, bounds: UiRect, layout: &mut BTreeMap<UiNodeId, UiRect>) {
        layout.insert(id, bounds);
        let Some(node) = self.document.node(id) else {
            return;
        };
        let count = node.children.len();
        if count == 0 {
            return;
        }
        let content_bounds = inset_bounds(bounds, node.style.padding);
        match node.layout {
            UiLayout::Overlay => {
                for child in &node.children {
                    self.layout_node(*child, content_bounds, layout);
                }
            }
            UiLayout::Grid { columns, gap } => {
                self.layout_grid(&node.children, content_bounds, columns, gap, layout);
            }
            UiLayout::VerticalStack { gap } => {
                self.layout_stack(&node.children, content_bounds, gap, true, layout);
            }
            UiLayout::HorizontalStack { gap } => {
                self.layout_stack(&node.children, content_bounds, gap, false, layout);
            }
        }
    }

    fn layout_stack(
        &self,
        children: &[UiNodeId],
        bounds: UiRect,
        gap: f32,
        vertical: bool,
        layout: &mut BTreeMap<UiNodeId, UiRect>,
    ) {
        let gap = finite_nonnegative(gap);
        let item_count = bounded_count_as_f32(children.len());
        let total_gap = gap * bounded_count_as_f32(children.len().saturating_sub(1));
        let axis_extent = if vertical {
            bounds.size.height
        } else {
            bounds.size.width
        };
        let available = (axis_extent - total_gap).max(0.0);
        let preferred_total = children.iter().fold(0.0, |total, child| {
            let preference = self
                .document
                .node(*child)
                .and_then(|node| {
                    if vertical {
                        node.layout_hints.preferred_height
                    } else {
                        node.layout_hints.preferred_width
                    }
                })
                .map_or(0.0, finite_nonnegative);
            total + preference
        });
        let preferred_scale = if preferred_total > available && preferred_total > 0.0 {
            available / preferred_total
        } else {
            1.0
        };
        let remaining = (available - preferred_total * preferred_scale).max(0.0);
        let grow_total = children.iter().fold(0.0, |total, child| {
            total
                + self
                    .document
                    .node(*child)
                    .map_or(0.0, |node| finite_nonnegative(node.layout_hints.grow))
        });
        let mut cursor = if vertical {
            bounds.origin.y
        } else {
            bounds.origin.x
        };
        for child in children {
            let node = self.document.node(*child);
            let preferred = node
                .and_then(|node| {
                    if vertical {
                        node.layout_hints.preferred_height
                    } else {
                        node.layout_hints.preferred_width
                    }
                })
                .map_or(0.0, finite_nonnegative)
                * preferred_scale;
            let grow = node.map_or(0.0, |node| finite_nonnegative(node.layout_hints.grow));
            let grown = if grow_total > 0.0 {
                remaining * grow / grow_total
            } else if preferred_total == 0.0 {
                available / item_count
            } else {
                0.0
            };
            let extent = (preferred + grown).max(0.0);
            let child_bounds = if vertical {
                UiRect::new(
                    UiPoint {
                        x: bounds.origin.x,
                        y: cursor,
                    },
                    UiSize::new(bounds.size.width, extent),
                )
            } else {
                UiRect::new(
                    UiPoint {
                        x: cursor,
                        y: bounds.origin.y,
                    },
                    UiSize::new(extent, bounds.size.height),
                )
            };
            self.layout_node(*child, child_bounds, layout);
            cursor += extent + gap;
        }
    }

    fn layout_grid(
        &self,
        children: &[UiNodeId],
        bounds: UiRect,
        columns: u8,
        gap: f32,
        layout: &mut BTreeMap<UiNodeId, UiRect>,
    ) {
        let requested_columns = usize::from(columns.max(1));
        let columns = requested_columns.min(children.len()).max(1);
        let rows = children.len().div_ceil(columns);
        let gap = finite_nonnegative(gap);
        let width = ((bounds.size.width - gap * bounded_count_as_f32(columns.saturating_sub(1)))
            .max(0.0)
            / bounded_count_as_f32(columns))
        .max(0.0);
        let height = ((bounds.size.height - gap * bounded_count_as_f32(rows.saturating_sub(1)))
            .max(0.0)
            / bounded_count_as_f32(rows))
        .max(0.0);
        for (index, child) in children.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            let child_bounds = UiRect::new(
                UiPoint {
                    x: bounds.origin.x + bounded_count_as_f32(column) * (width + gap),
                    y: bounds.origin.y + bounded_count_as_f32(row) * (height + gap),
                },
                UiSize::new(width, height),
            );
            self.layout_node(*child, child_bounds, layout);
        }
    }

    fn process_event(
        &mut self,
        event: UiEvent,
        layout: &BTreeMap<UiNodeId, UiRect>,
        routes: &mut Vec<UiEventRoute>,
        commands: &mut Vec<UiCommandRequest>,
        clipboard_requests: &mut Vec<UiClipboardRequest>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        match event {
            UiEvent::FocusNext => self.move_focus(true, diagnostics),
            UiEvent::FocusPrevious => self.move_focus(false, diagnostics),
            UiEvent::Activate => {
                if let Some(target) = self.focused {
                    self.dispatch(target, routes);
                    self.activate(target, commands);
                }
            }
            UiEvent::AssistiveFocus(target) => {
                if self
                    .document
                    .node(target)
                    .is_some_and(|node| node.focusable)
                {
                    self.dispatch(target, routes);
                    self.focused = Some(target);
                } else {
                    diagnostics.push(UiDiagnostic::AssistiveFocusDenied { node: target });
                }
            }
            UiEvent::AssistiveActivate(target) => {
                self.dispatch(target, routes);
                self.activate(target, commands);
            }
            UiEvent::TextCommit(text) => self.commit_text(&text, routes, diagnostics),
            UiEvent::ImePreedit { text, cursor } => {
                self.set_preedit(text, cursor, routes, diagnostics);
            }
            UiEvent::MoveTextCursor {
                direction,
                extend_selection,
            } => self.move_text_cursor(direction, extend_selection, routes, diagnostics),
            UiEvent::DeleteTextBackward => self.delete_text(true, routes, diagnostics),
            UiEvent::DeleteTextForward => self.delete_text(false, routes, diagnostics),
            UiEvent::SelectAllText => self.select_all_text(routes, diagnostics),
            UiEvent::CopySelection => self.copy_selection(routes, clipboard_requests, diagnostics),
            UiEvent::PointerDown(point) => {
                let target = self.hit_test(point, layout);
                self.pointer_capture = target;
                if let Some(target) = target {
                    self.dispatch(target, routes);
                    if self
                        .document
                        .node(target)
                        .is_some_and(|node| node.focusable)
                    {
                        self.focused = Some(target);
                    }
                } else if !layout
                    .get(&self.document.root())
                    .is_some_and(|bounds| bounds.contains(point))
                {
                    diagnostics.push(UiDiagnostic::PointerOutsideDocument);
                }
            }
            UiEvent::PointerUp(point) => {
                let captured = self.pointer_capture.take();
                let released_over = self.hit_test(point, layout);
                if let Some(target) = captured {
                    self.dispatch(target, routes);
                    if released_over == Some(target) {
                        self.activate(target, commands);
                    }
                }
            }
            UiEvent::PointerCancel => self.pointer_capture = None,
        }
    }

    fn focused_text_input(&self, diagnostics: &mut Vec<UiDiagnostic>) -> Option<UiNodeId> {
        let target = self.focused.filter(|id| self.text_inputs.contains_key(id));
        if target.is_none() {
            diagnostics.push(UiDiagnostic::TextInputNotFocused);
        }
        target
    }

    fn commit_text(
        &mut self,
        text: &str,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        let state = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state");
        if !replace_selection(state, text) {
            diagnostics.push(UiDiagnostic::TextInputLimitExceeded {
                node: target,
                maximum: MAX_TEXT_BYTES,
            });
            return;
        }
        state.preedit = None;
    }

    fn set_preedit(
        &mut self,
        text: String,
        cursor: Option<(usize, usize)>,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        if text.len() > MAX_TEXT_BYTES {
            diagnostics.push(UiDiagnostic::TextInputLimitExceeded {
                node: target,
                maximum: MAX_TEXT_BYTES,
            });
            return;
        }
        self.text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .preedit = Some((text, cursor));
    }

    fn move_text_cursor(
        &mut self,
        direction: UiTextCursorDirection,
        extend_selection: bool,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        let state = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state");
        let count = grapheme_count(&state.value);
        let selection = clamp_selection(state.selection, count);
        let origin = if !extend_selection && !selection.is_collapsed() {
            match direction {
                UiTextCursorDirection::Backward | UiTextCursorDirection::Start => selection.start(),
                UiTextCursorDirection::Forward | UiTextCursorDirection::End => selection.end(),
            }
        } else {
            selection.focus
        };
        let destination = match direction {
            UiTextCursorDirection::Backward => origin.saturating_sub(1),
            UiTextCursorDirection::Forward => origin.saturating_add(1).min(count),
            UiTextCursorDirection::Start => 0,
            UiTextCursorDirection::End => count,
        };
        state.selection = if extend_selection {
            UiTextSelection {
                anchor: selection.anchor,
                focus: destination,
            }
        } else {
            UiTextSelection::cursor(destination)
        };
    }

    fn delete_text(
        &mut self,
        backward: bool,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        let state = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state");
        let count = grapheme_count(&state.value);
        let selection = clamp_selection(state.selection, count);
        if selection.is_collapsed() {
            state.selection = if backward {
                UiTextSelection {
                    anchor: selection.focus.saturating_sub(1),
                    focus: selection.focus,
                }
            } else {
                UiTextSelection {
                    anchor: selection.focus,
                    focus: selection.focus.saturating_add(1).min(count),
                }
            };
        } else {
            state.selection = selection;
        }
        if !state.selection.is_collapsed() {
            let _ = replace_selection(state, "");
        }
        state.preedit = None;
    }

    fn select_all_text(
        &mut self,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        let state = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state");
        state.selection = UiTextSelection {
            anchor: 0,
            focus: grapheme_count(&state.value),
        };
    }

    fn copy_selection(
        &mut self,
        routes: &mut Vec<UiEventRoute>,
        clipboard_requests: &mut Vec<UiClipboardRequest>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(target) = self.focused_text_input(diagnostics) else {
            return;
        };
        self.dispatch(target, routes);
        let state = self
            .text_inputs
            .get(&target)
            .expect("focused text input has retained state");
        if state.password {
            diagnostics.push(UiDiagnostic::ClipboardDeniedForPassword { node: target });
            return;
        }
        if let Some(text) = selected_text(&state.value, state.selection) {
            clipboard_requests.push(UiClipboardRequest {
                source: target,
                text: text.to_owned(),
            });
        }
    }

    fn focused_preedit(&self) -> Option<String> {
        self.focused
            .and_then(|target| self.text_inputs.get(&target))
            .filter(|state| !state.password)
            .and_then(|state| state.preedit.as_ref().map(|(text, _)| text.clone()))
    }

    fn move_focus(&mut self, forward: bool, diagnostics: &mut Vec<UiDiagnostic>) {
        let order = self.document.focus_order();
        if order.is_empty() {
            diagnostics.push(UiDiagnostic::NoFocusableNode);
            return;
        }
        let current = self
            .focused
            .and_then(|id| order.iter().position(|candidate| *candidate == id));
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % order.len(),
            (Some(0), false) => order.len() - 1,
            (Some(index), false) => index - 1,
            (None, _) => 0,
        };
        self.focused = Some(order[next]);
    }

    fn dispatch(&self, target: UiNodeId, routes: &mut Vec<UiEventRoute>) {
        let Some(path) = self.document.route_to(target) else {
            return;
        };
        for node in path.iter().take(path.len().saturating_sub(1)) {
            routes.push(UiEventRoute {
                phase: UiEventPhase::Capture,
                node: *node,
            });
        }
        routes.push(UiEventRoute {
            phase: UiEventPhase::Target,
            node: target,
        });
        for node in path.iter().rev().skip(1) {
            routes.push(UiEventRoute {
                phase: UiEventPhase::Bubble,
                node: *node,
            });
        }
    }

    fn activate(&self, target: UiNodeId, commands: &mut Vec<UiCommandRequest>) {
        let Some(node) = self.document.node(target) else {
            return;
        };
        if let Some(action) = &node.semantics.action {
            commands.push(UiCommandRequest {
                source: target,
                action: action.clone(),
            });
        }
    }

    fn hit_test(&self, point: UiPoint, layout: &BTreeMap<UiNodeId, UiRect>) -> Option<UiNodeId> {
        self.hit_test_node(self.document.root(), point, layout)
    }

    fn hit_test_node(
        &self,
        id: UiNodeId,
        point: UiPoint,
        layout: &BTreeMap<UiNodeId, UiRect>,
    ) -> Option<UiNodeId> {
        let bounds = layout.get(&id)?;
        if !bounds.contains(point) {
            return None;
        }
        let node = self.document.node(id)?;
        for child in node.children.iter().rev() {
            if let Some(target) = self.hit_test_node(*child, point, layout) {
                return Some(target);
            }
        }
        node.focusable.then_some(id)
    }

    fn emit_node(&mut self, id: UiNodeId, parent: Option<UiNodeId>, emission: &mut UiEmission<'_>) {
        let Some(node) = self.document.node(id) else {
            return;
        };
        let Some(bounds) = emission.layout.get(&id).copied() else {
            return;
        };
        let foreground = if emission.high_contrast {
            UiColor::rgba(1.0, 1.0, 1.0, 1.0)
        } else {
            node.style.foreground
        };
        if let Some(background) = node.style.background {
            emission.display.primitives.push(DisplayPrimitive::Rect {
                node: id,
                bounds,
                color: background,
            });
        }
        if let Some(border) = node.style.border {
            emission.display.primitives.push(DisplayPrimitive::Border {
                node: id,
                bounds,
                color: border.color,
                width: border.width.max(1),
            });
        }
        let rendered_text = self
            .text_inputs
            .get(&id)
            .map(|state| {
                if state.password {
                    password_mask(&state.value)
                } else {
                    state.value.clone()
                }
            })
            .or_else(|| node.text.clone());
        if let Some(text) = rendered_text {
            let text_bounds = UiRect::new(
                UiPoint {
                    x: bounds.origin.x + node.style.padding,
                    y: bounds.origin.y + node.style.padding,
                },
                UiSize::new(
                    (bounds.size.width - node.style.padding * 2.0).max(1.0),
                    (bounds.size.height - node.style.padding * 2.0).max(1.0),
                ),
            );
            let text_output = self.text.layout(
                &text,
                text_bounds.size.width,
                node.style.font_size,
                emission.scale_factor,
            );
            if text_output.layout.used_fallback_metrics {
                emission
                    .diagnostics
                    .push(UiDiagnostic::TextFallbackMetrics { node: id });
            }
            if text_output.raster.has_unrasterized_glyphs {
                emission
                    .diagnostics
                    .push(UiDiagnostic::TextRasterIncomplete { node: id });
            }
            emission.display.primitives.push(DisplayPrimitive::Text {
                node: id,
                bounds: text_bounds,
                text,
                color: foreground,
                layout: text_output.layout,
                raster: text_output.raster,
            });
        }
        if self.focused == Some(id) {
            emission
                .display
                .primitives
                .push(DisplayPrimitive::FocusRing {
                    node: id,
                    bounds,
                    color: UiColor::focus(),
                });
        }
        emission.semantic_nodes.push(SemanticNode {
            id,
            parent,
            role: node.semantics.role,
            name: node.semantics.name.clone(),
            action: node.semantics.action.clone(),
            bounds,
            focused: self.focused == Some(id),
        });
        let children = node.children.clone();
        for child in children {
            self.emit_node(child, Some(id), emission);
        }
    }
}

/// A keyboard-operable recovery panel fixture used by native and headless smoke tests.
///
/// # Errors
///
/// Returns an error only if this fixed retained fixture violates document rules.
pub fn recovery_panel_document() -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(0x100);
    let message = UiNodeId::new(0x101);
    let retry = UiNodeId::new(0x102);
    UiDocument::new(
        root,
        vec![
            UiNode::container(
                root,
                "Recovery panel",
                UiLayout::VerticalStack { gap: 8.0 },
                vec![message, retry],
            ),
            UiNode::label(
                message,
                "Recovery status",
                "The project could not be opened.",
            ),
            UiNode::button(retry, "Retry project open", "project.retry_open", "Retry"),
        ],
    )
}

/// Minimal runtime overlay fixture.  It has no focusable nodes and therefore no input cost.
///
/// # Errors
///
/// Returns an error only if this fixed retained fixture violates document rules.
pub fn runtime_overlay_document() -> Result<UiDocument, UiDocumentError> {
    let root = UiNodeId::new(0x200);
    let label = UiNodeId::new(0x201);
    let mut overlay = UiNode::container(root, "Runtime overlay", UiLayout::Overlay, vec![label]);
    overlay.kind = UiWidgetKind::Overlay;
    overlay.style.background = None;
    UiDocument::new(
        root,
        vec![
            overlay,
            UiNode::label(label, "Runtime status", "Loading Meridian runtime…"),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(events: Vec<UiEvent>) -> UiFrameInput {
        UiFrameInput {
            events,
            ..UiFrameInput::new(UiSize::new(800.0, 600.0))
        }
    }

    #[test]
    fn recovery_panel_emits_display_and_semantics() {
        let document = recovery_panel_document().expect("recovery fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));
        assert!(output.display_list.primitives.len() >= 3);
        assert!(matches!(output.semantic_delta, SemanticDelta::Replace(_)));
    }

    #[test]
    fn preferred_stack_and_grid_keep_controls_in_distinct_visible_cells() {
        let root = UiNodeId::new(0x400);
        let header = UiNodeId::new(0x401);
        let body = UiNodeId::new(0x402);
        let actions = UiNodeId::new(0x403);
        let first = UiNodeId::new(0x404);
        let second = UiNodeId::new(0x405);
        let third = UiNodeId::new(0x406);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::label(header, "Header", "Creator tools")
                    .with_style(UiStyle::section_heading())
                    .with_layout_hints(UiLayoutHints::fixed_height(40.0)),
                UiNode::button(first, "First action", "first", "First")
                    .with_style(UiStyle::primary_action()),
                UiNode::button(second, "Second action", "second", "Second")
                    .with_style(UiStyle::secondary_action()),
                UiNode::button(third, "Third action", "third", "Third")
                    .with_style(UiStyle::secondary_action()),
                UiNode::container(
                    actions,
                    "Action grid",
                    UiLayout::Grid {
                        columns: 3,
                        gap: 8.0,
                    },
                    vec![first, second, third],
                )
                .with_style(UiStyle::transparent()),
                UiNode::container(body, "Tool body", UiLayout::Overlay, vec![actions])
                    .with_style(UiStyle::surface()),
                UiNode::container(
                    root,
                    "Creator layout",
                    UiLayout::VerticalStack { gap: 8.0 },
                    vec![header, body],
                )
                .with_style(UiStyle::canvas()),
            ],
        )
        .expect("valid preferred-layout document");
        let mut runtime = UiRuntime::new(document);
        let mut input = frame(Vec::new());
        input.viewport = UiSize::new(400.0, 220.0);
        let output = runtime.reconcile(input);
        let tree = match &output.semantic_delta {
            SemanticDelta::Replace(tree) => tree,
            SemanticDelta::Unchanged => panic!("first frame must publish semantics"),
        };
        let bounds = |id| {
            tree.nodes
                .iter()
                .find(|node| node.id == id)
                .map(|node| node.bounds)
                .expect("declared node has semantics")
        };
        let header_bounds = bounds(header);
        let first_bounds = bounds(first);
        let second_bounds = bounds(second);
        let third_bounds = bounds(third);
        assert!((header_bounds.size.height - 40.0).abs() < 0.1);
        assert!(first_bounds.size.width > 80.0 && first_bounds.size.height > 40.0);
        assert!(first_bounds.origin.x < second_bounds.origin.x);
        assert!(second_bounds.origin.x < third_bounds.origin.x);
        assert!(output
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::Border { .. })));
    }

    #[test]
    fn keyboard_activation_emits_declared_command() {
        let document = recovery_panel_document().expect("recovery fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![UiEvent::FocusNext, UiEvent::Activate]));
        assert_eq!(output.commands.len(), 1);
        assert_eq!(output.commands[0].action, "project.retry_open");
        assert!(output
            .event_routes
            .iter()
            .any(|route| route.phase == UiEventPhase::Target));
    }

    #[test]
    fn focus_order_follows_declared_tree_order_instead_of_stable_id_order() {
        let root = UiNodeId::new(1);
        let first = UiNodeId::new(30);
        let second = UiNodeId::new(10);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Focus fixture",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![first, second],
                ),
                UiNode::button(first, "First declared", "fixture.first", "First"),
                UiNode::button(second, "Second declared", "fixture.second", "Second"),
            ],
        )
        .expect("focus-order fixture is valid");

        assert_eq!(document.focus_order(), vec![first, second]);
    }

    #[test]
    fn pointer_ignores_structural_groups_and_activates_the_visible_button() {
        let root = UiNodeId::new(1);
        let group = UiNodeId::new(90);
        let action = UiNodeId::new(2);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Pointer fixture", UiLayout::Overlay, vec![group]),
                UiNode::container(group, "Structural group", UiLayout::Overlay, vec![action])
                    .with_style(UiStyle::transparent()),
                UiNode::button(action, "Visible action", "fixture.activate", "Activate"),
            ],
        )
        .expect("pointer fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::PointerDown(UiPoint { x: 100.0, y: 40.0 }),
            UiEvent::PointerUp(UiPoint { x: 100.0, y: 40.0 }),
        ]));

        assert_eq!(output.focused, Some(action));
        assert_eq!(
            output.commands,
            vec![UiCommandRequest {
                source: action,
                action: "fixture.activate".to_owned(),
            }]
        );
    }

    #[test]
    fn pointer_activation_requires_release_over_the_captured_control() {
        let root = UiNodeId::new(1);
        let action = UiNodeId::new(2);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Pointer fixture", UiLayout::Overlay, vec![action]),
                UiNode::button(action, "Visible action", "fixture.activate", "Activate"),
            ],
        )
        .expect("pointer fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::PointerDown(UiPoint { x: 100.0, y: 40.0 }),
            UiEvent::PointerUp(UiPoint {
                x: 1_000.0,
                y: 1_000.0,
            }),
        ]));

        assert_eq!(output.focused, Some(action));
        assert!(output.commands.is_empty());
    }

    #[test]
    fn pointer_release_without_press_and_cancelled_press_do_not_activate() {
        let root = UiNodeId::new(1);
        let action = UiNodeId::new(2);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Pointer fixture", UiLayout::Overlay, vec![action]),
                UiNode::button(action, "Visible action", "fixture.activate", "Activate"),
            ],
        )
        .expect("pointer fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let orphan_release = runtime.reconcile(frame(vec![UiEvent::PointerUp(UiPoint {
            x: 100.0,
            y: 40.0,
        })]));
        assert!(orphan_release.commands.is_empty());

        let cancelled = runtime.reconcile(frame(vec![
            UiEvent::PointerDown(UiPoint { x: 100.0, y: 40.0 }),
            UiEvent::PointerCancel,
            UiEvent::PointerUp(UiPoint { x: 100.0, y: 40.0 }),
        ]));
        assert!(cancelled.commands.is_empty());
    }

    #[test]
    fn meridian_palette_tokens_match_the_website_contract() {
        assert_eq!(
            UiColor::background(),
            UiColor::rgba(0.035_294_12, 0.043_137_256, 0.043_137_256, 1.0)
        );
        assert_eq!(
            UiColor::surface(),
            UiColor::rgba(0.070_588_24, 0.082_352_94, 0.082_352_94, 1.0)
        );
        assert_eq!(
            UiColor::border(),
            UiColor::rgba(0.160_784_32, 0.176_470_6, 0.172_549_02, 1.0)
        );
        assert_eq!(
            UiColor::text(),
            UiColor::rgba(0.890_196_1, 0.882_352_95, 0.847_058_83, 1.0)
        );
        assert_eq!(UiColor::focus(), UiColor::amber());
    }

    #[test]
    fn invalid_unnamed_focusable_node_is_rejected() {
        let root = UiNodeId::new(1);
        let mut node = UiNode::button(root, "Open", "open", "Open");
        node.semantics.name.clear();
        let result = UiDocument::new(root, vec![node]);
        assert!(matches!(
            result,
            Err(UiDocumentError::UnnamedFocusable(id)) if id == root
        ));
    }

    #[test]
    fn text_layout_does_not_expose_adapter_glyphs() {
        let document = recovery_panel_document().expect("recovery fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));
        let layout = output
            .display_list
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                DisplayPrimitive::Text { layout, .. } => Some(layout),
                _ => None,
            });
        assert!(layout.is_some_and(|layout| layout.line_count >= 1));
        assert!(layout.is_some_and(|layout| !layout.used_fallback_metrics));
        let glyph_count =
            output
                .display_list
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    DisplayPrimitive::Text { raster, .. } => Some(raster.glyphs.len()),
                    _ => None,
                });
        assert!(glyph_count.is_some_and(|count| count > 0));
    }

    #[test]
    fn high_dpi_and_contrast_preserve_semantics_and_scale_text_raster() {
        let mut normal_runtime =
            UiRuntime::new(recovery_panel_document().expect("recovery fixture is valid"));
        let normal = normal_runtime.reconcile(frame(Vec::new()));
        let mut hidpi_runtime =
            UiRuntime::new(recovery_panel_document().expect("recovery fixture is valid"));
        let mut hidpi_input = frame(vec![UiEvent::FocusNext]);
        hidpi_input.scale_factor = 2.0;
        hidpi_input.high_contrast = true;
        hidpi_input.reduced_motion = true;
        let hidpi = hidpi_runtime.reconcile(hidpi_input);
        let normal_text =
            normal
                .display_list
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    DisplayPrimitive::Text { layout, raster, .. } => Some((layout, raster)),
                    _ => None,
                });
        let hidpi_text =
            hidpi
                .display_list
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    DisplayPrimitive::Text {
                        layout,
                        raster,
                        color,
                        ..
                    } => Some((layout, raster, color)),
                    _ => None,
                });
        let (normal_layout, normal_raster) = normal_text.expect("normal text primitive exists");
        let (hidpi_layout, hidpi_raster, hidpi_color) =
            hidpi_text.expect("high-DPI text primitive exists");
        assert!((normal_layout.width - hidpi_layout.width).abs() < 0.1);
        assert!(hidpi_raster.glyphs[0].width >= normal_raster.glyphs[0].width);
        assert_eq!(*hidpi_color, UiColor::rgba(1.0, 1.0, 1.0, 1.0));
        assert!(hidpi
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::FocusRing { .. })));
    }

    #[test]
    fn runtime_overlay_has_no_focusable_nodes() {
        let document = runtime_overlay_document().expect("overlay fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![UiEvent::FocusNext]));
        assert_eq!(output.focused, None);
        assert_eq!(output.diagnostics, vec![UiDiagnostic::NoFocusableNode]);
        assert!(output
            .display_list
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, DisplayPrimitive::Rect { .. })));
        assert!(output
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::Text { .. })));
    }

    fn text_input_document(initial_value: &str, password: bool) -> (UiDocument, UiNodeId) {
        let input = UiNodeId::new(0x300);
        let document = UiDocument::new(
            input,
            vec![UiNode::text_input(
                input,
                "Project title",
                initial_value,
                UiTextInputOptions { password },
            )],
        )
        .expect("text-input fixture is valid");
        (document, input)
    }

    #[test]
    fn text_input_edits_on_grapheme_boundaries_and_preserves_ime_composition() {
        let (document, input) = text_input_document("a👩‍🔬e\u{301}", false);
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::MoveTextCursor {
                direction: UiTextCursorDirection::Forward,
                extend_selection: false,
            },
            UiEvent::MoveTextCursor {
                direction: UiTextCursorDirection::Forward,
                extend_selection: false,
            },
            UiEvent::MoveTextCursor {
                direction: UiTextCursorDirection::Backward,
                extend_selection: true,
            },
            UiEvent::CopySelection,
            UiEvent::TextCommit("x".to_owned()),
            UiEvent::ImePreedit {
                text: "é".to_owned(),
                cursor: Some((1, 1)),
            },
        ]));

        assert_eq!(output.clipboard_requests.len(), 1);
        assert_eq!(output.clipboard_requests[0].source, input);
        assert_eq!(output.clipboard_requests[0].text, "👩‍🔬");
        assert_eq!(output.preedit.as_deref(), Some("é"));
        assert_eq!(
            output.text_inputs,
            vec![UiTextInputSnapshot {
                node: input,
                selection: UiTextSelection::cursor(2),
                grapheme_count: 3,
                password: false,
                has_preedit: true,
            }]
        );
        assert_eq!(
            runtime
                .text_inputs
                .get(&input)
                .map(|state| state.value.as_str()),
            Some("axe\u{301}")
        );

        let output = runtime.reconcile(frame(vec![
            UiEvent::TextCommit("!".to_owned()),
            UiEvent::DeleteTextBackward,
        ]));
        assert_eq!(output.preedit, None);
        assert_eq!(
            runtime
                .text_inputs
                .get(&input)
                .map(|state| state.value.as_str()),
            Some("axe\u{301}")
        );
    }

    #[test]
    fn password_input_masks_rendering_and_denies_clipboard_output() {
        let (document, input) = text_input_document("must-not-persist", true);
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::TextCommit("s3cr3t".to_owned()),
            UiEvent::SelectAllText,
            UiEvent::CopySelection,
            UiEvent::ImePreedit {
                text: "replacement".to_owned(),
                cursor: None,
            },
        ]));

        assert!(output.clipboard_requests.is_empty());
        assert_eq!(output.preedit, None);
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::ClipboardDeniedForPassword { node: input }));
        assert!(output
            .display_list
            .primitives
            .iter()
            .all(|primitive| match primitive {
                DisplayPrimitive::Text { text, .. } => text != "s3cr3t",
                _ => true,
            }));
        assert_eq!(
            output.text_inputs,
            vec![UiTextInputSnapshot {
                node: input,
                selection: UiTextSelection {
                    anchor: 0,
                    focus: 6
                },
                grapheme_count: 6,
                password: true,
                has_preedit: true,
            }]
        );
    }

    #[test]
    fn text_input_rejects_over_limit_commits_without_mutating_state() {
        let (document, input) = text_input_document("safe", false);
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::TextCommit("a".repeat(MAX_TEXT_BYTES + 1)),
        ]));

        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::TextInputLimitExceeded {
                node: input,
                maximum: MAX_TEXT_BYTES,
            }));
        assert_eq!(
            runtime
                .text_inputs
                .get(&input)
                .map(|state| state.value.as_str()),
            Some("safe")
        );
    }

    #[test]
    fn replacing_document_preserves_non_password_text_by_stable_id() {
        let (document, input) = text_input_document("Meridian Project", false);
        let mut runtime = UiRuntime::new(document);
        runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("Creator Sample".to_owned()),
        ]));
        let (replacement, _) = text_input_document("Ignored initial value", false);
        runtime.replace_document(replacement);
        assert_eq!(runtime.text_input_value(input), Some("Creator Sample"));
    }

    #[test]
    fn assistive_focus_edits_a_named_text_control_and_rejects_nonfocusable_targets() {
        let root = UiNodeId::new(0x401);
        let input = UiNodeId::new(0x402);
        let label = UiNodeId::new(0x403);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Fixture",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![input, label],
                ),
                UiNode::text_input(input, "Placement X", "0", UiTextInputOptions::default()),
                UiNode::label(label, "Read only", "Read only"),
            ],
        )
        .expect("assistive focus fixture is valid");
        let mut runtime = UiRuntime::new(document);

        let output = runtime.reconcile(frame(vec![
            UiEvent::AssistiveFocus(input),
            UiEvent::SelectAllText,
            UiEvent::TextCommit("250".to_owned()),
            UiEvent::AssistiveFocus(label),
        ]));

        assert_eq!(output.focused, Some(input));
        assert_eq!(runtime.text_input_value(input), Some("250"));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveFocusDenied { node: label }));
    }

    #[test]
    fn targeted_text_input_reset_uses_current_document_default_without_resetting_passwords() {
        let (document, input) = text_input_document("0", false);
        let mut runtime = UiRuntime::new(document);
        runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("250".to_owned()),
        ]));
        assert_eq!(runtime.text_input_value(input), Some("250"));

        assert!(runtime.reset_text_input_from_document(input));
        assert_eq!(runtime.text_input_value(input), Some("0"));

        let (password_document, password) = text_input_document("ignored", true);
        let mut password_runtime = UiRuntime::new(password_document);
        assert!(!password_runtime.reset_text_input_from_document(password));
        assert_eq!(password_runtime.text_input_value(password), None);
    }
}
