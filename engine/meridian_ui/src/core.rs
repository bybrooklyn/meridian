//! Retained, renderer-independent UI contracts.
//!
//! The data crossing this crate boundary is Meridian-owned.  Platform and
//! renderer adapters consume [`DisplayList`] and [`SemanticTree`] rather than
//! borrowing widget state or exposing their native types here.

use std::collections::{BTreeMap, BTreeSet};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use meridian_core::StableId;

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

/// Linear RGBA colour owned by the UI contract.
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
        Self::rgba(0.06, 0.07, 0.10, 0.96)
    }

    #[must_use]
    pub const fn foreground() -> Self {
        Self::rgba(0.93, 0.95, 1.0, 1.0)
    }

    #[must_use]
    pub const fn focus() -> Self {
        Self::rgba(0.31, 0.71, 1.0, 1.0)
    }
}

/// Widget behavior is intentionally a small, retained set for the MS-02 seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWidgetKind {
    Panel,
    Label,
    Button,
    Overlay,
}

/// Layout policy used by the retained document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiLayout {
    Overlay,
    VerticalStack { gap: f32 },
    HorizontalStack { gap: f32 },
}

/// Public semantic role independent of a platform accessibility API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticRole {
    Group,
    Status,
    Button,
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
}

/// Rendering style, expressed only in Meridian-owned values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStyle {
    pub background: Option<UiColor>,
    pub foreground: UiColor,
    pub padding: f32,
    pub font_size: f32,
}

impl UiStyle {
    #[must_use]
    pub const fn panel() -> Self {
        Self {
            background: Some(UiColor::panel()),
            foreground: UiColor::foreground(),
            padding: 12.0,
            font_size: 16.0,
        }
    }

    #[must_use]
    pub const fn text() -> Self {
        Self {
            background: None,
            foreground: UiColor::foreground(),
            padding: 6.0,
            font_size: 16.0,
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
    pub semantics: UiSemantics,
    pub text: Option<String>,
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
            semantics: UiSemantics::group(name),
            text: None,
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
            semantics: UiSemantics::status(name),
            text: Some(text.into()),
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
            style: UiStyle::panel(),
            semantics: UiSemantics::button(name, action),
            text: Some(text.into()),
            focusable: true,
            children: Vec::new(),
        }
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
        self.nodes
            .values()
            .filter(|node| node.focusable)
            .map(|node| node.id)
            .collect()
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
    PointerDown(UiPoint),
    PointerUp(UiPoint),
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

/// A renderer-neutral visual primitive consumed by the UI render adapter.
#[derive(Clone, Debug, PartialEq)]
pub enum DisplayPrimitive {
    Rect {
        node: UiNodeId,
        bounds: UiRect,
        color: UiColor,
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

fn bounded_count_as_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
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

/// Retained runtime state.  All mutation is applied between immutable outputs.
#[derive(Debug)]
pub struct UiRuntime {
    document: UiDocument,
    text: UiTextEngine,
    focused: Option<UiNodeId>,
    pointer_capture: Option<UiNodeId>,
    preedit: Option<String>,
    previous_semantics: Option<SemanticTree>,
}

impl UiRuntime {
    #[must_use]
    pub fn new(document: UiDocument) -> Self {
        Self {
            document,
            text: UiTextEngine::default(),
            focused: None,
            pointer_capture: None,
            preedit: None,
            previous_semantics: None,
        }
    }

    #[must_use]
    pub const fn document(&self) -> &UiDocument {
        &self.document
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
        let mut diagnostics = Vec::new();
        for event in input.events {
            self.process_event(event, &layout, &mut routes, &mut commands, &mut diagnostics);
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
            diagnostics,
            focused: self.focused,
            preedit: self.preedit.clone(),
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
        let gap = match node.layout {
            UiLayout::VerticalStack { gap } | UiLayout::HorizontalStack { gap } => gap.max(0.0),
            UiLayout::Overlay => 0.0,
        };
        let available = match node.layout {
            UiLayout::VerticalStack { .. } => {
                (bounds.size.height - gap * bounded_count_as_f32(count.saturating_sub(1))).max(1.0)
            }
            UiLayout::HorizontalStack { .. } => {
                (bounds.size.width - gap * bounded_count_as_f32(count.saturating_sub(1))).max(1.0)
            }
            UiLayout::Overlay => 0.0,
        };
        let item_count = bounded_count_as_f32(count);
        for (index, child) in node.children.iter().enumerate() {
            let item_index = bounded_count_as_f32(index);
            let child_bounds = match node.layout {
                UiLayout::Overlay => bounds,
                UiLayout::VerticalStack { .. } => UiRect::new(
                    UiPoint {
                        x: bounds.origin.x,
                        y: bounds.origin.y + item_index * (available / item_count + gap),
                    },
                    UiSize::new(bounds.size.width, available / item_count),
                ),
                UiLayout::HorizontalStack { .. } => UiRect::new(
                    UiPoint {
                        x: bounds.origin.x + item_index * (available / item_count + gap),
                        y: bounds.origin.y,
                    },
                    UiSize::new(available / item_count, bounds.size.height),
                ),
            };
            self.layout_node(*child, child_bounds, layout);
        }
    }

    fn process_event(
        &mut self,
        event: UiEvent,
        layout: &BTreeMap<UiNodeId, UiRect>,
        routes: &mut Vec<UiEventRoute>,
        commands: &mut Vec<UiCommandRequest>,
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
            UiEvent::AssistiveActivate(target) => {
                self.dispatch(target, routes);
                self.activate(target, commands);
            }
            UiEvent::TextCommit(_) => self.preedit = None,
            UiEvent::ImePreedit { text, .. } => self.preedit = Some(text),
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
                } else {
                    diagnostics.push(UiDiagnostic::PointerOutsideDocument);
                }
            }
            UiEvent::PointerUp(point) => {
                let target = self
                    .pointer_capture
                    .take()
                    .or_else(|| self.hit_test(point, layout));
                if let Some(target) = target {
                    self.dispatch(target, routes);
                    self.activate(target, commands);
                }
            }
        }
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
        self.document
            .nodes
            .keys()
            .rev()
            .find(|id| layout.get(id).is_some_and(|bounds| bounds.contains(point)))
            .copied()
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
        if let Some(text) = &node.text {
            let text_bounds = UiRect::new(
                UiPoint {
                    x: bounds.origin.x + node.style.padding,
                    y: bounds.origin.y + node.style.padding,
                },
                UiSize::new(
                    (bounds.size.width - node.style.padding * 2.0).max(1.0),
                    bounds.size.height,
                ),
            );
            let text_output = self.text.layout(
                text,
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
                text: text.clone(),
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
}
