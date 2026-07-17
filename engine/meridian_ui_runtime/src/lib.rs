//! Retained Meridian UI reconciliation and interaction runtime.

use std::collections::BTreeMap;
use std::sync::Arc;

use meridian_ui_core::{
    sanitized_scale_factor, MotionPreference, ThemeId, UiAlignment, UiAxis, UiColor, UiConstraints,
    UiContrast, UiDensity, UiDocument, UiDocumentDelta, UiDocumentError, UiLayout, UiLayoutHints,
    UiNode, UiNodeId, UiPoint, UiRect, UiSize, UiTheme, UiWidgetKind, MAX_TEXT_BYTES,
};
use meridian_ui_render::{
    DisplayList, DisplayListError, DisplayPrimitive, UiClipId, UiCornerRadii,
};
use meridian_ui_semantics::{SemanticDelta, SemanticNode, SemanticTree};
use meridian_ui_text::{
    UiClipboardRequest, UiTextCursorDirection, UiTextEngine, UiTextInputSnapshot, UiTextInputState,
};

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
    FrameRejected(DisplayListError),
}

/// Input captured at a stable frame boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameInput {
    pub viewport: UiSize,
    pub scale_factor: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub theme: UiTheme,
    pub density: UiDensity,
    pub contrast: UiContrast,
    pub motion: MotionPreference,
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
            theme: UiTheme::meridian_dark(),
            density: UiDensity::Standard,
            contrast: UiContrast::Standard,
            motion: MotionPreference::Full,
            events: Vec::new(),
        }
    }
}

/// Accepted logical geometry for one stable retained node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiLayoutSnapshot {
    pub node: UiNodeId,
    pub bounds: UiRect,
}

/// Immutable frame result handed to renderer and semantic adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameSnapshot {
    pub revision: u64,
    pub layout: Vec<UiLayoutSnapshot>,
    pub theme: ThemeId,
    pub density: UiDensity,
    pub contrast: UiContrast,
    pub motion: MotionPreference,
    pub scale_factor: f32,
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

/// Shared immutable compatibility handle retained while callers migrate.
pub type UiFrameOutput = Arc<UiFrameSnapshot>;

/// Typed frame rejection before any mutated interaction state is committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiFrameError {
    InvalidDisplayList(DisplayListError),
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

fn resolve_constraints(bounds: UiRect, constraints: UiConstraints) -> UiRect {
    let maximum = constraints
        .maximum
        .unwrap_or(UiSize::new(f32::MAX, f32::MAX));
    let mut width = bounds
        .size
        .width
        .clamp(constraints.minimum.width, maximum.width);
    let mut height = bounds
        .size
        .height
        .clamp(constraints.minimum.height, maximum.height);
    if let Some(aspect) = constraints.aspect_ratio {
        let width_from_height = height * aspect;
        if width_from_height <= width {
            width = width_from_height;
        } else {
            height = width / aspect;
        }
    }
    let horizontal_space = (bounds.size.width - width).max(0.0);
    let vertical_space = (bounds.size.height - height).max(0.0);
    let x = bounds.origin.x
        + match constraints.horizontal_alignment {
            UiAlignment::Start | UiAlignment::Stretch => 0.0,
            UiAlignment::Center => horizontal_space / 2.0,
            UiAlignment::End => horizontal_space,
        };
    let y = bounds.origin.y
        + match constraints.vertical_alignment {
            UiAlignment::Start | UiAlignment::Stretch => 0.0,
            UiAlignment::Center => vertical_space / 2.0,
            UiAlignment::End => vertical_space,
        };
    UiRect::new(UiPoint { x, y }, UiSize::new(width, height))
}

struct UiEmission<'a> {
    layout: &'a BTreeMap<UiNodeId, UiRect>,
    scale_factor: f32,
    high_contrast: bool,
    display: &'a mut DisplayList,
    semantic_nodes: &'a mut Vec<SemanticNode>,
    diagnostics: &'a mut Vec<UiDiagnostic>,
    next_scope: u64,
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
    revision: u64,
    last_document_delta: UiDocumentDelta,
    last_snapshot: Option<Arc<UiFrameSnapshot>>,
}

impl UiRuntime {
    #[must_use]
    pub fn new(document: UiDocument) -> Self {
        let text_inputs = document
            .nodes()
            .filter_map(|node| {
                node.text_input.map(|options| {
                    (
                        node.id,
                        UiTextInputState::new(
                            node.text.clone().unwrap_or_default(),
                            options.password,
                        ),
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
            revision: 0,
            last_document_delta: UiDocumentDelta::default(),
            last_snapshot: None,
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
    pub fn replace_document(&mut self, document: UiDocument) -> UiDocumentDelta {
        let delta = self.document.delta_to(&document);
        let previous_inputs = std::mem::take(&mut self.text_inputs);
        self.text_inputs = document
            .nodes()
            .filter_map(|node| {
                let options = node.text_input?;
                let state = previous_inputs
                    .get(&node.id)
                    .filter(|state| state.is_password() == options.password);
                Some((
                    node.id,
                    state.cloned().unwrap_or_else(|| {
                        UiTextInputState::new(
                            node.text.clone().unwrap_or_default(),
                            options.password,
                        )
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
        self.last_document_delta.clone_from(&delta);
        delta
    }

    /// Returns the most recent accepted identity reconciliation summary.
    #[must_use]
    pub const fn last_document_delta(&self) -> &UiDocumentDelta {
        &self.last_document_delta
    }

    /// Validates and atomically accepts a replacement document.
    ///
    /// # Errors
    ///
    /// A rejected tree leaves document, focus, private text, and frame state intact.
    pub fn try_replace_document(
        &mut self,
        root: UiNodeId,
        nodes: Vec<UiNode>,
    ) -> Result<UiDocumentDelta, UiDocumentError> {
        let document = UiDocument::new(root, nodes)?;
        Ok(self.replace_document(document))
    }

    /// Returns one non-password text value kept privately by the runtime.
    ///
    /// Password values are deliberately never exposed through this API.
    #[must_use]
    pub fn text_input_value(&self, node: UiNodeId) -> Option<&str> {
        self.text_inputs
            .get(&node)
            .and_then(UiTextInputState::value)
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
        state.reset_from_document(default_value)
    }

    /// Processes events, resolves retained layout, and returns only immutable output.
    pub fn reconcile(&mut self, input: UiFrameInput) -> UiFrameOutput {
        let fallback_theme = input.theme.id;
        let fallback_density = input.density;
        let fallback_contrast = input.contrast;
        let fallback_motion = input.motion;
        let fallback_scale = sanitized_scale_factor(input.scale_factor);
        match self.try_reconcile(input) {
            Ok(snapshot) => snapshot,
            Err(UiFrameError::InvalidDisplayList(error)) => {
                let mut fallback = self.last_snapshot.clone().unwrap_or_else(|| {
                    Arc::new(UiFrameSnapshot {
                        revision: self.revision,
                        layout: Vec::new(),
                        theme: fallback_theme,
                        density: fallback_density,
                        contrast: fallback_contrast,
                        motion: fallback_motion,
                        scale_factor: fallback_scale,
                        display_list: DisplayList::default(),
                        semantic_delta: SemanticDelta::Unchanged,
                        event_routes: Vec::new(),
                        commands: Vec::new(),
                        clipboard_requests: Vec::new(),
                        text_inputs: Vec::new(),
                        diagnostics: Vec::new(),
                        focused: self.focused,
                        preedit: self.focused_preedit(),
                    })
                });
                Arc::make_mut(&mut fallback)
                    .diagnostics
                    .push(UiDiagnostic::FrameRejected(error));
                fallback
            }
        }
    }

    /// Transactionally computes one immutable frame.
    ///
    /// # Errors
    ///
    /// Invalid display output restores interaction state and preserves the last
    /// accepted snapshot.
    pub fn try_reconcile(&mut self, input: UiFrameInput) -> Result<UiFrameOutput, UiFrameError> {
        let previous_text_inputs = self.text_inputs.clone();
        let previous_focused = self.focused;
        let previous_pointer_capture = self.pointer_capture;
        let previous_semantics = self.previous_semantics.clone();
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
        let contrast = if input.high_contrast {
            UiContrast::High
        } else {
            input.contrast
        };
        let motion = if input.reduced_motion {
            MotionPreference::Reduced
        } else {
            input.motion
        };
        let high_contrast = contrast == UiContrast::High;
        let mut emission = UiEmission {
            layout: &layout,
            scale_factor: sanitized_scale_factor(input.scale_factor),
            high_contrast,
            display: &mut display_list,
            semantic_nodes: &mut semantic_nodes,
            diagnostics: &mut diagnostics,
            next_scope: 1,
        };
        let emission_result = self.emit_node(self.document.root(), None, &mut emission);
        if let Err(error) = emission_result.and_then(|()| display_list.validate()) {
            self.text_inputs = previous_text_inputs;
            self.focused = previous_focused;
            self.pointer_capture = previous_pointer_capture;
            self.previous_semantics = previous_semantics;
            return Err(UiFrameError::InvalidDisplayList(error));
        }
        let tree = SemanticTree {
            nodes: semantic_nodes,
        };
        let semantic_delta = if self.previous_semantics.as_ref() == Some(&tree) {
            SemanticDelta::Unchanged
        } else {
            SemanticDelta::Replace(tree.clone())
        };
        self.previous_semantics = Some(tree);
        self.revision = self.revision.saturating_add(1);
        let snapshot = Arc::new(UiFrameSnapshot {
            revision: self.revision,
            layout: layout
                .iter()
                .map(|(node, bounds)| UiLayoutSnapshot {
                    node: *node,
                    bounds: *bounds,
                })
                .collect(),
            theme: input.theme.id,
            density: input.density,
            contrast,
            motion,
            scale_factor: sanitized_scale_factor(input.scale_factor),
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
        });
        self.last_snapshot = Some(Arc::clone(&snapshot));
        Ok(snapshot)
    }

    fn layout_node(&self, id: UiNodeId, bounds: UiRect, layout: &mut BTreeMap<UiNodeId, UiRect>) {
        let Some(node) = self.document.node(id) else {
            return;
        };
        let bounds = resolve_constraints(bounds, node.constraints);
        layout.insert(id, bounds);
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
            UiLayout::Flex { axis, gap } => {
                self.layout_stack(
                    &node.children,
                    content_bounds,
                    gap,
                    axis == UiAxis::Vertical,
                    layout,
                );
            }
            UiLayout::Absolute => {
                for child in &node.children {
                    let child_bounds = self
                        .document
                        .node(*child)
                        .and_then(|child| child.absolute_position)
                        .map_or(content_bounds, |position| {
                            UiRect::new(
                                UiPoint {
                                    x: content_bounds.origin.x + position.left,
                                    y: content_bounds.origin.y + position.top,
                                },
                                UiSize::new(
                                    position.width.unwrap_or(content_bounds.size.width),
                                    position.height.unwrap_or(content_bounds.size.height),
                                ),
                            )
                        });
                    self.layout_node(*child, child_bounds, layout);
                }
            }
            UiLayout::Scroll { axis, offset } => {
                self.layout_scroll(&node.children, content_bounds, axis, offset, layout);
            }
        }
    }

    fn layout_scroll(
        &self,
        children: &[UiNodeId],
        bounds: UiRect,
        axis: UiAxis,
        offset: f32,
        layout: &mut BTreeMap<UiNodeId, UiRect>,
    ) {
        let mut cursor = if axis == UiAxis::Vertical {
            bounds.origin.y - finite_nonnegative(offset)
        } else {
            bounds.origin.x - finite_nonnegative(offset)
        };
        for child in children {
            let hints = self
                .document
                .node(*child)
                .map_or_else(UiLayoutHints::default, |node| node.layout_hints);
            let child_bounds = if axis == UiAxis::Vertical {
                let height = hints.preferred_height.unwrap_or(bounds.size.height);
                let result = UiRect::new(
                    UiPoint {
                        x: bounds.origin.x,
                        y: cursor,
                    },
                    UiSize::new(bounds.size.width, height),
                );
                cursor += height;
                result
            } else {
                let width = hints.preferred_width.unwrap_or(bounds.size.width);
                let result = UiRect::new(
                    UiPoint {
                        x: cursor,
                        y: bounds.origin.y,
                    },
                    UiSize::new(width, bounds.size.height),
                );
                cursor += width;
                result
            };
            self.layout_node(*child, child_bounds, layout);
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
        if !state.commit(text) {
            diagnostics.push(UiDiagnostic::TextInputLimitExceeded {
                node: target,
                maximum: MAX_TEXT_BYTES,
            });
        }
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
        let accepted = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .set_preedit(text, cursor);
        if !accepted {
            diagnostics.push(UiDiagnostic::TextInputLimitExceeded {
                node: target,
                maximum: MAX_TEXT_BYTES,
            });
        }
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
        self.text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .move_cursor(direction, extend_selection);
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
        self.text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .delete(backward);
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
        self.text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .select_all();
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
        if state.is_password() {
            diagnostics.push(UiDiagnostic::ClipboardDeniedForPassword { node: target });
            return;
        }
        if let Some(text) = state.selected_text() {
            clipboard_requests.push(UiClipboardRequest {
                source: target,
                text: text.to_owned(),
            });
        }
    }

    fn focused_preedit(&self) -> Option<String> {
        self.focused
            .and_then(|target| self.text_inputs.get(&target))
            .and_then(UiTextInputState::preedit_text)
            .map(str::to_owned)
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

    fn emit_node(
        &mut self,
        id: UiNodeId,
        parent: Option<UiNodeId>,
        emission: &mut UiEmission<'_>,
    ) -> Result<(), DisplayListError> {
        let Some(node) = self.document.node(id) else {
            return Ok(());
        };
        let Some(bounds) = emission.layout.get(&id).copied() else {
            return Ok(());
        };
        let foreground = if emission.high_contrast {
            UiColor::rgba(1.0, 1.0, 1.0, 1.0)
        } else {
            node.style.foreground
        };
        let clip = node.constraints.clip;
        let semantics = node.semantics.clone();
        let children = node.children.clone();
        let clip_id = if clip {
            let id = UiClipId(emission.next_scope);
            emission.next_scope = emission.next_scope.saturating_add(1);
            emission.display.try_push(DisplayPrimitive::PushClip {
                id,
                bounds,
                radii: UiCornerRadii::default(),
            })?;
            Some(id)
        } else {
            None
        };
        self.emit_node_visuals(id, bounds, foreground, emission)?;
        emission.semantic_nodes.push(SemanticNode {
            id,
            parent,
            role: semantics.role,
            name: semantics.name,
            action: semantics.action,
            value: semantics.value,
            bounds,
            focused: self.focused == Some(id),
        });
        for child in children {
            self.emit_node(child, Some(id), emission)?;
        }
        if let Some(id) = clip_id {
            emission
                .display
                .try_push(DisplayPrimitive::PopClip { id })?;
        }
        Ok(())
    }

    fn emit_node_visuals(
        &mut self,
        id: UiNodeId,
        bounds: UiRect,
        foreground: UiColor,
        emission: &mut UiEmission<'_>,
    ) -> Result<(), DisplayListError> {
        let Some(node) = self.document.node(id) else {
            return Ok(());
        };
        if let Some(background) = node.style.background {
            emission.display.try_push(DisplayPrimitive::Rect {
                node: id,
                bounds,
                color: background,
            })?;
        }
        if let Some(border) = node.style.border {
            emission.display.try_push(DisplayPrimitive::Border {
                node: id,
                bounds,
                color: border.color,
                width: border.width.max(1),
            })?;
        }
        let rendered_text = self
            .text_inputs
            .get(&id)
            .map(UiTextInputState::rendered_text)
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
            emission.display.try_push(DisplayPrimitive::Text {
                node: id,
                bounds: text_bounds,
                text,
                color: foreground,
                layout: text_output.layout,
                raster: text_output.raster,
            })?;
        }
        if self.focused == Some(id) {
            emission.display.try_push(DisplayPrimitive::FocusRing {
                node: id,
                bounds,
                color: UiColor::focus(),
            })?;
        }
        Ok(())
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
    use meridian_ui_core::{UiAbsolutePosition, UiStyle, UiTextInputOptions};
    use meridian_ui_text::UiTextSelection;

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
        assert_eq!(hidpi.theme, UiTheme::meridian_dark().id);
        assert_eq!(hidpi.contrast, UiContrast::High);
        assert_eq!(hidpi.motion, MotionPreference::Reduced);
        assert!((hidpi.scale_factor - 2.0).abs() < f32::EPSILON);
        assert!(hidpi
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::FocusRing { .. })));
    }

    #[test]
    fn one_x_and_two_x_frames_are_deterministic_for_identical_inputs() {
        let reconcile_at = |scale_factor| {
            let mut runtime =
                UiRuntime::new(recovery_panel_document().expect("recovery fixture is valid"));
            let mut input = frame(vec![UiEvent::FocusNext]);
            input.scale_factor = scale_factor;
            runtime.reconcile(input)
        };
        let one_x_a = reconcile_at(1.0);
        let one_x_b = reconcile_at(1.0);
        let two_x_a = reconcile_at(2.0);
        let two_x_b = reconcile_at(2.0);
        assert_eq!(one_x_a, one_x_b);
        assert_eq!(two_x_a, two_x_b);
        assert_eq!(one_x_a.layout, two_x_a.layout);
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
        assert_eq!(runtime.text_input_value(input), Some("axe\u{301}"));

        let output = runtime.reconcile(frame(vec![
            UiEvent::TextCommit("!".to_owned()),
            UiEvent::DeleteTextBackward,
        ]));
        assert_eq!(output.preedit, None);
        assert_eq!(runtime.text_input_value(input), Some("axe\u{301}"));
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
        assert_eq!(runtime.text_input_value(input), Some("safe"));
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

    #[test]
    fn flex_absolute_scroll_and_clipping_emit_stable_geometry_and_valid_scopes() {
        let root = UiNodeId::new(0x500);
        let absolute = UiNodeId::new(0x501);
        let positioned = UiNodeId::new(0x502);
        let scroll = UiNodeId::new(0x503);
        let first = UiNodeId::new(0x504);
        let second = UiNodeId::new(0x505);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Layout modes",
                    UiLayout::Flex {
                        axis: UiAxis::Horizontal,
                        gap: 8.0,
                    },
                    vec![absolute, scroll],
                )
                .with_style(UiStyle::transparent()),
                UiNode::container(absolute, "Absolute", UiLayout::Absolute, vec![positioned])
                    .with_style(UiStyle::transparent()),
                UiNode::label(positioned, "Positioned", "Positioned").with_absolute_position(
                    UiAbsolutePosition {
                        left: 12.0,
                        top: 16.0,
                        width: Some(80.0),
                        height: Some(32.0),
                    },
                ),
                UiNode::container(
                    scroll,
                    "Scroll",
                    UiLayout::Scroll {
                        axis: UiAxis::Vertical,
                        offset: 10.0,
                    },
                    vec![first, second],
                )
                .with_style(UiStyle::transparent())
                .with_constraints(UiConstraints {
                    clip: true,
                    ..UiConstraints::default()
                }),
                UiNode::label(first, "First", "First")
                    .with_layout_hints(UiLayoutHints::fixed_height(40.0)),
                UiNode::label(second, "Second", "Second")
                    .with_layout_hints(UiLayoutHints::fixed_height(40.0)),
            ],
        )
        .expect("layout-mode document");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(400.0, 200.0)));
        let bounds = |id| {
            output
                .layout
                .iter()
                .find(|entry| entry.node == id)
                .map(|entry| entry.bounds)
                .expect("node has geometry")
        };
        let absolute_bounds = bounds(absolute);
        let positioned_bounds = bounds(positioned);
        assert!((positioned_bounds.origin.x - (absolute_bounds.origin.x + 12.0)).abs() < 0.1);
        assert!((positioned_bounds.origin.y - (absolute_bounds.origin.y + 16.0)).abs() < 0.1);
        assert_eq!(positioned_bounds.size, UiSize::new(80.0, 32.0));
        assert!(bounds(first).origin.y < bounds(scroll).origin.y);
        assert_eq!(output.display_list.validate(), Ok(()));
        assert!(matches!(
            output.display_list.primitives.first(),
            Some(
                DisplayPrimitive::PushClip { .. }
                    | DisplayPrimitive::Rect { .. }
                    | DisplayPrimitive::Text { .. }
            )
        ));
    }

    #[test]
    fn rejected_document_update_preserves_runtime_state_and_snapshot_revision() {
        let (document, input) = text_input_document("safe", false);
        let mut runtime = UiRuntime::new(document.clone());
        let first = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("retained".to_owned()),
        ]));
        let invalid = UiNode::container(input, "Cycle", UiLayout::Overlay, vec![input]);
        assert_eq!(
            runtime.try_replace_document(input, vec![invalid]),
            Err(UiDocumentError::Cycle(input))
        );
        assert_eq!(runtime.document(), &document);
        assert_eq!(runtime.text_input_value(input), Some("retained"));
        let second = runtime.reconcile(frame(Vec::new()));
        assert_eq!(second.revision, first.revision + 1);
        assert_eq!(second.focused, Some(input));
        assert_eq!(runtime.last_document_delta(), &UiDocumentDelta::default());
    }

    #[test]
    fn accepted_replacement_reports_incremental_identity_changes() {
        let root = UiNodeId::new(0x510);
        let stable = UiNodeId::new(0x511);
        let initial = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Initial", UiLayout::Overlay, vec![stable]),
                UiNode::label(stable, "Stable", "Before"),
            ],
        )
        .expect("initial document");
        let replacement = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Replacement", UiLayout::Overlay, vec![stable]),
                UiNode::label(stable, "Stable", "After"),
            ],
        )
        .expect("replacement document");
        let mut runtime = UiRuntime::new(initial);
        let delta = runtime.replace_document(replacement);
        assert_eq!(delta.retained, vec![root, stable]);
        assert_eq!(delta.updated, vec![root, stable]);
        assert!(delta.inserted.is_empty());
        assert!(delta.removed.is_empty());
        assert_eq!(runtime.last_document_delta(), &delta);
    }

    #[test]
    fn malformed_frame_rolls_back_to_the_last_accepted_immutable_snapshot() {
        let valid = recovery_panel_document().expect("valid recovery document");
        let mut runtime = UiRuntime::new(valid);
        let accepted = runtime.reconcile(frame(vec![UiEvent::FocusNext]));
        let root = UiNodeId::new(0x520);
        let mut invalid_node = UiNode::label(root, "Invalid pixels", "Invalid pixels");
        invalid_node.style.foreground = UiColor::rgba(f32::NAN, 1.0, 1.0, 1.0);
        let invalid = UiDocument::new(root, vec![invalid_node]).expect("logical tree is valid");
        runtime.replace_document(invalid);

        assert!(matches!(
            runtime.try_reconcile(frame(Vec::new())),
            Err(UiFrameError::InvalidDisplayList(
                DisplayListError::InvalidGeometry { .. }
            ))
        ));
        let fallback = runtime.reconcile(frame(Vec::new()));
        assert_eq!(fallback.revision, accepted.revision);
        assert_eq!(fallback.display_list, accepted.display_list);
        assert!(matches!(
            fallback.diagnostics.last(),
            Some(UiDiagnostic::FrameRejected(
                DisplayListError::InvalidGeometry { .. }
            ))
        ));
    }
}
