//! Retained Meridian UI reconciliation and interaction runtime.

use std::collections::BTreeMap;
use std::sync::Arc;

use meridian_ui_core::{
    sanitized_scale_factor, MotionPreference, ThemeId, UiAlignment, UiAxis, UiCollectionCursor,
    UiCollectionNavigation, UiColor, UiConstraints, UiContrast, UiDensity, UiDocument,
    UiDocumentDelta, UiDocumentError, UiDragKind, UiDragPayload, UiDropOperation, UiInputDeviceId,
    UiInputDeviceKind, UiLayout, UiLayoutHints, UiNode, UiNodeId, UiPoint, UiPointerButton,
    UiPointerEvent, UiPointerPhase, UiRect, UiScrollEvent, UiScrollPhase, UiScrollUnit, UiSize,
    UiTextValidation, UiTheme, UiWidgetKind, MAX_FRAME_EVENTS, MAX_TEXT_BYTES,
};
use meridian_ui_render::{
    DisplayList, DisplayListError, DisplayPrimitive, UiClipId, UiCornerRadii,
};
use meridian_ui_semantics::{SemanticDelta, SemanticNode, SemanticTree};
use meridian_ui_text::{
    UiClipboardOperation, UiClipboardRequest, UiCompletionRequest, UiPreeditError,
    UiTextCursorDirection, UiTextEngine, UiTextInputSnapshot, UiTextInputState,
};

const LEGACY_POINTER_DEVICE: UiInputDeviceId = UiInputDeviceId::new(1);

/// Platform-normalized event delivered to the retained interaction model.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    FocusNext,
    FocusPrevious,
    Activate,
    TextCommit(String),
    ImePreedit {
        text: String,
        /// Half-open UTF-8 byte range within `text`; `None` hides the cursor.
        cursor: Option<(usize, usize)>,
    },
    ImeCancel,
    MoveTextCursor {
        direction: UiTextCursorDirection,
        extend_selection: bool,
    },
    DeleteTextBackward,
    DeleteTextForward,
    UndoText,
    RedoText,
    SelectAllText,
    CopySelection,
    CutSelection,
    ConfirmClipboardCut {
        source: UiNodeId,
        text: String,
    },
    PasteText(String),
    RequestCompletion,
    Pointer(UiPointerEvent),
    PointerDown(UiPoint),
    PointerUp(UiPoint),
    PointerCancel,
    Scroll(UiScrollEvent),
    BeginDrag(UiDragPayload),
    BeginKeyboardDrag {
        source: UiNodeId,
        payload: UiDragPayload,
    },
    CompleteDrag,
    CancelDrag,
    NavigateCollection(UiCollectionNavigation),
    CollectionTypeahead(String),
    /// Requests focus for a named focusable control through a semantic adapter.
    ///
    /// This does not activate the control or expose its private text value. It
    /// exists so an accessibility adapter can use the same focus model as
    /// keyboard and pointer input.
    AssistiveFocus(UiNodeId),
    AssistiveActivate(UiNodeId),
}

impl UiEvent {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::TextCommit(text) | Self::PasteText(text) | Self::CollectionTypeahead(text) => {
                text.len()
            }
            Self::ImePreedit { text, .. } | Self::ConfirmClipboardCut { text, .. } => text.len(),
            Self::FocusNext
            | Self::FocusPrevious
            | Self::Activate
            | Self::ImeCancel
            | Self::MoveTextCursor { .. }
            | Self::DeleteTextBackward
            | Self::DeleteTextForward
            | Self::UndoText
            | Self::RedoText
            | Self::SelectAllText
            | Self::CopySelection
            | Self::CutSelection
            | Self::RequestCompletion
            | Self::Pointer(_)
            | Self::PointerDown(_)
            | Self::PointerUp(_)
            | Self::PointerCancel
            | Self::Scroll(_)
            | Self::BeginDrag(_)
            | Self::BeginKeyboardDrag { .. }
            | Self::CompleteDrag
            | Self::CancelDrag
            | Self::NavigateCollection(_)
            | Self::AssistiveFocus(_)
            | Self::AssistiveActivate(_) => 0,
        }
    }
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

/// Accepted scroll position retained by stable container identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScrollSnapshot {
    pub node: UiNodeId,
    pub offset: f32,
    pub maximum: f32,
}

/// Observable consumption and residual handoff for one normalized scroll event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScrollOutcome {
    pub device: UiInputDeviceId,
    pub phase: UiScrollPhase,
    pub target: Option<UiNodeId>,
    pub consumed: UiPoint,
    pub remaining: UiPoint,
}

/// Drag state exposed without granting source or filesystem authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDragSnapshot {
    pub source: UiNodeId,
    pub over: Option<UiNodeId>,
    pub payload: UiDragPayload,
    pub keyboard: bool,
}

/// Validated drop proposal for the host's typed, undoable command adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiDropRequest {
    pub source: UiNodeId,
    pub target: UiNodeId,
    pub payload: UiDragPayload,
}

/// Validation state for a retained text draft.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTextValidationSnapshot {
    pub node: UiNodeId,
    pub rule: UiTextValidation,
    pub valid: bool,
}

/// Non-fatal behavior reported to diagnostics and tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiDiagnostic {
    NoFocusableNode,
    PointerOutsideDocument,
    TextFallbackMetrics {
        node: UiNodeId,
    },
    TextRasterIncomplete {
        node: UiNodeId,
    },
    TextInputNotFocused,
    TextInputLimitExceeded {
        node: UiNodeId,
        maximum: usize,
    },
    TextUndoUnavailable {
        node: UiNodeId,
    },
    TextRedoUnavailable {
        node: UiNodeId,
    },
    ImeCursorInvalid {
        node: UiNodeId,
    },
    TextValidationFailed {
        node: UiNodeId,
    },
    ClipboardDeniedForPassword {
        node: UiNodeId,
    },
    ClipboardCutStale {
        node: UiNodeId,
    },
    CompletionDeniedForPassword {
        node: UiNodeId,
    },
    AssistiveFocusDenied {
        node: UiNodeId,
    },
    AssistiveActivateDenied {
        node: UiNodeId,
    },
    InvalidPointerEvent,
    InvalidScrollEvent,
    ScrollTargetUnavailable,
    DragSourceDenied {
        node: UiNodeId,
        kind: UiDragKind,
    },
    DropTargetDenied {
        node: UiNodeId,
        kind: UiDragKind,
    },
    DropOperationDenied {
        node: UiNodeId,
        operation: UiDropOperation,
    },
    DragUnavailable,
    CollectionQueryTooLong {
        maximum: usize,
    },
    EventBatchRejected {
        count: usize,
        maximum: usize,
    },
    InputByteLimitExceeded {
        bytes: usize,
        maximum: usize,
    },
    FrameEffectLimitExceeded {
        count: usize,
        maximum: usize,
    },
    FrameEffectByteLimitExceeded {
        bytes: usize,
        maximum: usize,
    },
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
    pub completion_requests: Vec<UiCompletionRequest>,
    pub text_inputs: Vec<UiTextInputSnapshot>,
    pub text_validation: Vec<UiTextValidationSnapshot>,
    pub scroll: Vec<UiScrollSnapshot>,
    pub scroll_outcomes: Vec<UiScrollOutcome>,
    pub drag: Option<UiDragSnapshot>,
    pub drops: Vec<UiDropRequest>,
    pub diagnostics: Vec<UiDiagnostic>,
    pub focused: Option<UiNodeId>,
    pub preedit: Option<String>,
}

/// Shared immutable compatibility handle retained while callers migrate.
pub type UiFrameOutput = Arc<UiFrameSnapshot>;

/// Typed frame rejection before any mutated interaction state is committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiFrameError {
    TooManyEvents { count: usize, maximum: usize },
    TooManyInputBytes { bytes: usize, maximum: usize },
    TooManyEffects { count: usize, maximum: usize },
    TooManyEffectBytes { bytes: usize, maximum: usize },
    InvalidDisplayList(DisplayListError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PointerCapture {
    device: UiInputDeviceId,
    target: UiNodeId,
    button: UiPointerButton,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScrollCapture {
    device: UiInputDeviceId,
    target: UiNodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveDrag {
    source: UiNodeId,
    over: Option<UiNodeId>,
    payload: UiDragPayload,
    keyboard: bool,
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

#[derive(Default)]
struct UiFrameEffects {
    routes: Vec<UiEventRoute>,
    commands: Vec<UiCommandRequest>,
    clipboard_requests: Vec<UiClipboardRequest>,
    completion_requests: Vec<UiCompletionRequest>,
    scroll_outcomes: Vec<UiScrollOutcome>,
    drops: Vec<UiDropRequest>,
    diagnostics: Vec<UiDiagnostic>,
}

impl UiFrameEffects {
    fn aggregate_count(&self) -> usize {
        self.routes
            .len()
            .saturating_add(self.commands.len())
            .saturating_add(self.clipboard_requests.len())
            .saturating_add(self.completion_requests.len())
            .saturating_add(self.scroll_outcomes.len())
            .saturating_add(self.drops.len())
            .saturating_add(self.diagnostics.len())
    }

    fn aggregate_bytes(&self) -> usize {
        self.commands
            .iter()
            .map(|command| command.action.len())
            .chain(
                self.clipboard_requests
                    .iter()
                    .map(|request| request.text.len()),
            )
            .chain(
                self.completion_requests
                    .iter()
                    .map(|request| request.prefix.len()),
            )
            .fold(0, usize::saturating_add)
    }
}

#[derive(Clone)]
struct UiInteractionCheckpoint {
    text_inputs: BTreeMap<UiNodeId, UiTextInputState>,
    focused: Option<UiNodeId>,
    collection_cursor: UiCollectionCursor,
    pointer_capture: Option<PointerCapture>,
    scroll_offsets: BTreeMap<UiNodeId, f32>,
    scroll_capture: Option<ScrollCapture>,
    drag: Option<ActiveDrag>,
    previous_semantics: Option<SemanticTree>,
}

/// Retained runtime state.  All mutation is applied between immutable outputs.
#[derive(Debug)]
pub struct UiRuntime {
    document: UiDocument,
    text: UiTextEngine,
    text_inputs: BTreeMap<UiNodeId, UiTextInputState>,
    focused: Option<UiNodeId>,
    collection_cursor: UiCollectionCursor,
    pointer_capture: Option<PointerCapture>,
    scroll_offsets: BTreeMap<UiNodeId, f32>,
    scroll_capture: Option<ScrollCapture>,
    drag: Option<ActiveDrag>,
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
        let scroll_offsets = Self::initial_scroll_offsets(&document);
        Self {
            document,
            text: UiTextEngine::default(),
            text_inputs,
            focused: None,
            collection_cursor: UiCollectionCursor::default(),
            pointer_capture: None,
            scroll_offsets,
            scroll_capture: None,
            drag: None,
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
        let previous_scroll_offsets = std::mem::take(&mut self.scroll_offsets);
        self.scroll_offsets = document
            .nodes()
            .filter_map(|node| match node.layout {
                UiLayout::Scroll { offset, .. } => Some((
                    node.id,
                    previous_scroll_offsets
                        .get(&node.id)
                        .copied()
                        .unwrap_or(offset),
                )),
                _ => None,
            })
            .collect();
        if self.focused.is_some_and(|id| {
            !document
                .node(id)
                .is_some_and(|node| node.focusable && !node.semantics.state.disabled)
        }) {
            self.focused = None;
        }
        if self.focused.is_none()
            && self.collection_cursor.selected.is_some_and(|id| {
                document
                    .node(id)
                    .is_some_and(|node| node.focusable && !node.semantics.state.disabled)
            })
        {
            self.focused = self.collection_cursor.selected;
        }
        self.pointer_capture = None;
        self.scroll_capture = None;
        self.drag = None;
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
            Err(UiFrameError::TooManyEvents { count, maximum }) => self.rejected_snapshot(
                fallback_theme,
                fallback_density,
                fallback_contrast,
                fallback_motion,
                fallback_scale,
                UiDiagnostic::EventBatchRejected { count, maximum },
            ),
            Err(UiFrameError::TooManyInputBytes { bytes, maximum }) => self.rejected_snapshot(
                fallback_theme,
                fallback_density,
                fallback_contrast,
                fallback_motion,
                fallback_scale,
                UiDiagnostic::InputByteLimitExceeded { bytes, maximum },
            ),
            Err(UiFrameError::TooManyEffects { count, maximum }) => self.rejected_snapshot(
                fallback_theme,
                fallback_density,
                fallback_contrast,
                fallback_motion,
                fallback_scale,
                UiDiagnostic::FrameEffectLimitExceeded { count, maximum },
            ),
            Err(UiFrameError::TooManyEffectBytes { bytes, maximum }) => self.rejected_snapshot(
                fallback_theme,
                fallback_density,
                fallback_contrast,
                fallback_motion,
                fallback_scale,
                UiDiagnostic::FrameEffectByteLimitExceeded { bytes, maximum },
            ),
            Err(UiFrameError::InvalidDisplayList(error)) => self.rejected_snapshot(
                fallback_theme,
                fallback_density,
                fallback_contrast,
                fallback_motion,
                fallback_scale,
                UiDiagnostic::FrameRejected(error),
            ),
        }
    }

    fn rejected_snapshot(
        &self,
        theme: ThemeId,
        density: UiDensity,
        contrast: UiContrast,
        motion: MotionPreference,
        scale_factor: f32,
        diagnostic: UiDiagnostic,
    ) -> UiFrameOutput {
        let mut fallback = self.last_snapshot.clone().unwrap_or_else(|| {
            Arc::new(UiFrameSnapshot {
                revision: self.revision,
                layout: Vec::new(),
                theme,
                density,
                contrast,
                motion,
                scale_factor,
                display_list: DisplayList::default(),
                semantic_delta: SemanticDelta::Unchanged,
                event_routes: Vec::new(),
                commands: Vec::new(),
                clipboard_requests: Vec::new(),
                completion_requests: Vec::new(),
                text_inputs: Vec::new(),
                text_validation: Vec::new(),
                scroll: Vec::new(),
                scroll_outcomes: Vec::new(),
                drag: None,
                drops: Vec::new(),
                diagnostics: Vec::new(),
                focused: self.focused,
                preedit: self.focused_preedit(),
            })
        });
        {
            let snapshot = Arc::make_mut(&mut fallback);
            snapshot.semantic_delta = SemanticDelta::Unchanged;
            snapshot.event_routes.clear();
            snapshot.commands.clear();
            snapshot.clipboard_requests.clear();
            snapshot.completion_requests.clear();
            snapshot.scroll_outcomes.clear();
            snapshot.drops.clear();
            snapshot.diagnostics.clear();
            snapshot.diagnostics.push(diagnostic);
            snapshot.drag = self.drag.map(Self::drag_snapshot);
            snapshot.focused = self.focused;
            snapshot.preedit = self.focused_preedit();
        }
        fallback
    }

    /// Transactionally computes one immutable frame.
    ///
    /// # Errors
    ///
    /// Invalid display output restores interaction state and preserves the last
    /// accepted snapshot.
    pub fn try_reconcile(&mut self, input: UiFrameInput) -> Result<UiFrameOutput, UiFrameError> {
        Self::validate_input_bound(&input)?;
        let checkpoint = self.interaction_checkpoint();
        let mut layout = self.resolved_layout(input.viewport);
        let mut effects = UiFrameEffects::default();
        let line_step = finite_nonnegative(input.theme.geometry.spacing_base) * 4.0;
        for event in input.events {
            let layout_changed = self.process_event(event, &layout, &mut effects, line_step);
            self.ensure_effect_bound(&effects, &checkpoint)?;
            if layout_changed {
                layout = self.resolved_layout(input.viewport);
            }
        }
        let text_validation = self.text_validation_snapshots(&mut effects.diagnostics);
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
            diagnostics: &mut effects.diagnostics,
            next_scope: 1,
        };
        let emission_result = self.emit_node(self.document.root(), None, &mut emission);
        if let Err(error) = emission_result.and_then(|()| display_list.validate()) {
            self.restore_interaction(checkpoint);
            return Err(UiFrameError::InvalidDisplayList(error));
        }
        self.ensure_effect_bound(&effects, &checkpoint)?;
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
            event_routes: effects.routes,
            commands: effects.commands,
            clipboard_requests: effects.clipboard_requests,
            completion_requests: effects.completion_requests,
            text_inputs: self
                .text_inputs
                .iter()
                .map(|(node, state)| state.snapshot(*node))
                .collect(),
            text_validation,
            scroll: self.scroll_snapshots(&layout),
            scroll_outcomes: effects.scroll_outcomes,
            drag: self.drag.map(Self::drag_snapshot),
            drops: effects.drops,
            diagnostics: effects.diagnostics,
            focused: self.focused,
            preedit: self.focused_preedit(),
        });
        self.last_snapshot = Some(Arc::clone(&snapshot));
        Ok(snapshot)
    }

    fn validate_input_bound(input: &UiFrameInput) -> Result<(), UiFrameError> {
        if input.events.len() > MAX_FRAME_EVENTS {
            return Err(UiFrameError::TooManyEvents {
                count: input.events.len(),
                maximum: MAX_FRAME_EVENTS,
            });
        }
        let input_bytes = input
            .events
            .iter()
            .map(UiEvent::payload_bytes)
            .fold(0, usize::saturating_add);
        if input_bytes > MAX_TEXT_BYTES {
            return Err(UiFrameError::TooManyInputBytes {
                bytes: input_bytes,
                maximum: MAX_TEXT_BYTES,
            });
        }
        Ok(())
    }

    fn ensure_effect_bound(
        &mut self,
        effects: &UiFrameEffects,
        checkpoint: &UiInteractionCheckpoint,
    ) -> Result<(), UiFrameError> {
        let count = effects.aggregate_count();
        if count > MAX_FRAME_EVENTS {
            self.restore_interaction(checkpoint.clone());
            return Err(UiFrameError::TooManyEffects {
                count,
                maximum: MAX_FRAME_EVENTS,
            });
        }
        let bytes = effects.aggregate_bytes();
        if bytes > MAX_TEXT_BYTES {
            self.restore_interaction(checkpoint.clone());
            return Err(UiFrameError::TooManyEffectBytes {
                bytes,
                maximum: MAX_TEXT_BYTES,
            });
        }
        Ok(())
    }

    fn initial_scroll_offsets(document: &UiDocument) -> BTreeMap<UiNodeId, f32> {
        document
            .nodes()
            .filter_map(|node| match node.layout {
                UiLayout::Scroll { offset, .. } => Some((node.id, offset)),
                _ => None,
            })
            .collect()
    }

    const fn drag_snapshot(drag: ActiveDrag) -> UiDragSnapshot {
        UiDragSnapshot {
            source: drag.source,
            over: drag.over,
            payload: drag.payload,
            keyboard: drag.keyboard,
        }
    }

    fn interaction_checkpoint(&self) -> UiInteractionCheckpoint {
        UiInteractionCheckpoint {
            text_inputs: self.text_inputs.clone(),
            focused: self.focused,
            collection_cursor: self.collection_cursor,
            pointer_capture: self.pointer_capture,
            scroll_offsets: self.scroll_offsets.clone(),
            scroll_capture: self.scroll_capture,
            drag: self.drag,
            previous_semantics: self.previous_semantics.clone(),
        }
    }

    fn restore_interaction(&mut self, checkpoint: UiInteractionCheckpoint) {
        self.text_inputs = checkpoint.text_inputs;
        self.focused = checkpoint.focused;
        self.collection_cursor = checkpoint.collection_cursor;
        self.pointer_capture = checkpoint.pointer_capture;
        self.scroll_offsets = checkpoint.scroll_offsets;
        self.scroll_capture = checkpoint.scroll_capture;
        self.drag = checkpoint.drag;
        self.previous_semantics = checkpoint.previous_semantics;
    }

    fn resolved_layout(&mut self, viewport: UiSize) -> BTreeMap<UiNodeId, UiRect> {
        let mut layout = BTreeMap::new();
        self.layout_node(
            self.document.root(),
            UiRect::new(UiPoint::default(), viewport.sanitized()),
            &mut layout,
        );
        if self.clamp_scroll_offsets(&layout) {
            layout.clear();
            self.layout_node(
                self.document.root(),
                UiRect::new(UiPoint::default(), viewport.sanitized()),
                &mut layout,
            );
        }
        layout
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
                let offset = self.scroll_offsets.get(&id).copied().unwrap_or(offset);
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
        effects: &mut UiFrameEffects,
        line_step: f32,
    ) -> bool {
        match event {
            UiEvent::FocusNext => self.move_focus(true, &mut effects.diagnostics),
            UiEvent::FocusPrevious => self.move_focus(false, &mut effects.diagnostics),
            UiEvent::Activate => {
                if let Some(target) = self.focused {
                    self.dispatch(target, &mut effects.routes);
                    self.activate(target, &mut effects.commands);
                }
            }
            UiEvent::AssistiveFocus(target) => self.assistive_focus(target, effects),
            UiEvent::AssistiveActivate(target) => self.assistive_activate(target, effects),
            UiEvent::TextCommit(text) | UiEvent::PasteText(text) => {
                self.commit_text(&text, &mut effects.routes, &mut effects.diagnostics);
            }
            UiEvent::ImePreedit { text, cursor } => {
                self.set_preedit(text, cursor, &mut effects.routes, &mut effects.diagnostics);
            }
            UiEvent::ImeCancel => self.cancel_preedit(effects),
            UiEvent::MoveTextCursor {
                direction,
                extend_selection,
            } => self.move_text_cursor(
                direction,
                extend_selection,
                &mut effects.routes,
                &mut effects.diagnostics,
            ),
            UiEvent::DeleteTextBackward => {
                self.delete_text(true, &mut effects.routes, &mut effects.diagnostics);
            }
            UiEvent::DeleteTextForward => {
                self.delete_text(false, &mut effects.routes, &mut effects.diagnostics);
            }
            UiEvent::UndoText => self.edit_text_history(false, effects),
            UiEvent::RedoText => self.edit_text_history(true, effects),
            UiEvent::SelectAllText => {
                self.select_all_text(&mut effects.routes, &mut effects.diagnostics);
            }
            UiEvent::CopySelection => {
                self.process_clipboard_selection(UiClipboardOperation::Copy, effects);
            }
            UiEvent::CutSelection => {
                self.process_clipboard_selection(UiClipboardOperation::Cut, effects);
            }
            UiEvent::ConfirmClipboardCut { source, text } => {
                self.confirm_clipboard_cut(source, &text, effects);
            }
            UiEvent::RequestCompletion => {
                self.request_completion(
                    &mut effects.routes,
                    &mut effects.completion_requests,
                    &mut effects.diagnostics,
                );
            }
            UiEvent::Pointer(event) => {
                return self.process_pointer(event, layout, effects, line_step);
            }
            UiEvent::PointerDown(point) => {
                let event = Self::legacy_pointer_event(UiPointerPhase::Press, point);
                return self.process_pointer(event, layout, effects, line_step);
            }
            UiEvent::PointerUp(point) => {
                let event = Self::legacy_pointer_event(UiPointerPhase::Release, point);
                return self.process_pointer(event, layout, effects, line_step);
            }
            UiEvent::PointerCancel => {
                let event = Self::legacy_pointer_event(UiPointerPhase::Cancel, UiPoint::default());
                return self.process_pointer(event, layout, effects, line_step);
            }
            UiEvent::Scroll(event) => {
                return self.process_scroll(event, layout, effects, line_step);
            }
            UiEvent::BeginDrag(payload) => self.begin_pointer_drag(payload, effects),
            UiEvent::BeginKeyboardDrag { source, payload } => {
                self.begin_drag(
                    source,
                    payload,
                    true,
                    &mut effects.routes,
                    &mut effects.diagnostics,
                );
            }
            UiEvent::CompleteDrag => self.finish_active_drag(effects),
            UiEvent::CancelDrag => self.drag = None,
            UiEvent::NavigateCollection(navigation) => {
                self.navigate_collection(navigation, layout, &mut effects.diagnostics);
            }
            UiEvent::CollectionTypeahead(query) => {
                self.collection_typeahead(&query, &mut effects.diagnostics);
            }
        }
        false
    }

    fn assistive_focus(&mut self, target: UiNodeId, effects: &mut UiFrameEffects) {
        if self
            .document
            .node(target)
            .is_some_and(|node| node.focusable && !node.semantics.state.disabled)
        {
            self.dispatch(target, &mut effects.routes);
            self.set_focus(target);
        } else {
            effects
                .diagnostics
                .push(UiDiagnostic::AssistiveFocusDenied { node: target });
        }
    }

    fn assistive_activate(&self, target: UiNodeId, effects: &mut UiFrameEffects) {
        let accepted = self.document.node(target).is_some_and(|node| {
            node.focusable && !node.semantics.state.disabled && node.semantics.action.is_some()
        });
        if accepted {
            self.dispatch(target, &mut effects.routes);
            self.activate(target, &mut effects.commands);
        } else {
            effects
                .diagnostics
                .push(UiDiagnostic::AssistiveActivateDenied { node: target });
        }
    }

    fn process_clipboard_selection(
        &mut self,
        operation: UiClipboardOperation,
        effects: &mut UiFrameEffects,
    ) {
        self.copy_selection(
            operation,
            &mut effects.routes,
            &mut effects.clipboard_requests,
            &mut effects.diagnostics,
        );
    }

    fn cancel_preedit(&mut self, effects: &mut UiFrameEffects) {
        let Some(target) = self.focused_text_input(&mut effects.diagnostics) else {
            return;
        };
        self.dispatch(target, &mut effects.routes);
        self.text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .cancel_preedit();
    }

    fn edit_text_history(&mut self, redo: bool, effects: &mut UiFrameEffects) {
        let Some(target) = self.focused_text_input(&mut effects.diagnostics) else {
            return;
        };
        self.dispatch(target, &mut effects.routes);
        let state = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state");
        let accepted = if redo { state.redo() } else { state.undo() };
        if !accepted {
            effects.diagnostics.push(if redo {
                UiDiagnostic::TextRedoUnavailable { node: target }
            } else {
                UiDiagnostic::TextUndoUnavailable { node: target }
            });
        }
    }

    fn confirm_clipboard_cut(
        &mut self,
        source: UiNodeId,
        expected_text: &str,
        effects: &mut UiFrameEffects,
    ) {
        let accepted = expected_text.len() <= MAX_TEXT_BYTES
            && self.focused == Some(source)
            && self
                .text_inputs
                .get(&source)
                .and_then(UiTextInputState::selected_text)
                == Some(expected_text);
        if !accepted {
            effects
                .diagnostics
                .push(UiDiagnostic::ClipboardCutStale { node: source });
            return;
        }
        self.dispatch(source, &mut effects.routes);
        let _ = self
            .text_inputs
            .get_mut(&source)
            .and_then(UiTextInputState::cut_selected_text);
    }

    fn begin_pointer_drag(&mut self, payload: UiDragPayload, effects: &mut UiFrameEffects) {
        let Some(source) = self.pointer_capture.map(|capture| capture.target) else {
            effects.diagnostics.push(UiDiagnostic::DragUnavailable);
            return;
        };
        self.begin_drag(
            source,
            payload,
            false,
            &mut effects.routes,
            &mut effects.diagnostics,
        );
    }

    fn finish_active_drag(&mut self, effects: &mut UiFrameEffects) {
        let target = self.drag.and_then(|drag| drag.over).or(self.focused);
        self.complete_drag(
            target,
            &mut effects.routes,
            &mut effects.drops,
            &mut effects.diagnostics,
        );
    }

    const fn legacy_pointer_event(phase: UiPointerPhase, position: UiPoint) -> UiPointerEvent {
        UiPointerEvent {
            device: LEGACY_POINTER_DEVICE,
            kind: UiInputDeviceKind::Mouse,
            phase,
            position,
            button: if matches!(phase, UiPointerPhase::Press | UiPointerPhase::Release) {
                Some(UiPointerButton::Primary)
            } else {
                None
            },
        }
    }

    fn process_pointer(
        &mut self,
        event: UiPointerEvent,
        layout: &BTreeMap<UiNodeId, UiRect>,
        effects: &mut UiFrameEffects,
        line_step: f32,
    ) -> bool {
        if !Self::valid_pointer_event(event) {
            if self
                .pointer_capture
                .is_some_and(|capture| capture.device == event.device)
            {
                self.pointer_capture = None;
                self.drag = None;
            }
            effects.diagnostics.push(UiDiagnostic::InvalidPointerEvent);
            return false;
        }
        match event.phase {
            UiPointerPhase::Press => {
                let Some(button) = Self::effective_pointer_button(event) else {
                    effects.diagnostics.push(UiDiagnostic::InvalidPointerEvent);
                    return false;
                };
                let target = self.hit_test(event.position, layout);
                self.pointer_capture = target.map(|target| PointerCapture {
                    device: event.device,
                    target,
                    button,
                });
                if let Some(target) = target {
                    self.dispatch(target, &mut effects.routes);
                    self.set_focus(target);
                } else if !layout
                    .get(&self.document.root())
                    .is_some_and(|bounds| bounds.contains(event.position))
                {
                    effects
                        .diagnostics
                        .push(UiDiagnostic::PointerOutsideDocument);
                }
            }
            UiPointerPhase::Move => {
                if self.drag.is_some() {
                    let over = self.hit_test(event.position, layout);
                    if let Some(drag) = self.drag.as_mut() {
                        drag.over = over;
                    }
                    return self.drag_auto_scroll(event.position, layout, line_step);
                }
            }
            UiPointerPhase::Release => {
                let Some(button) = Self::effective_pointer_button(event) else {
                    effects.diagnostics.push(UiDiagnostic::InvalidPointerEvent);
                    return false;
                };
                let captured = self.pointer_capture;
                if captured.is_some_and(|capture| capture.device != event.device) {
                    effects.diagnostics.push(UiDiagnostic::InvalidPointerEvent);
                    return false;
                }
                self.pointer_capture = None;
                let released_over = self.hit_test(event.position, layout);
                if self.drag.is_some() {
                    if let Some(drag) = self.drag.as_mut() {
                        drag.over = released_over;
                    }
                    self.complete_drag(
                        released_over,
                        &mut effects.routes,
                        &mut effects.drops,
                        &mut effects.diagnostics,
                    );
                } else if let Some(capture) = captured {
                    self.dispatch(capture.target, &mut effects.routes);
                    if capture.button == button
                        && button == UiPointerButton::Primary
                        && released_over == Some(capture.target)
                    {
                        self.activate(capture.target, &mut effects.commands);
                    }
                }
            }
            UiPointerPhase::Cancel => {
                self.pointer_capture = None;
                self.drag = None;
            }
        }
        false
    }

    fn valid_pointer_event(event: UiPointerEvent) -> bool {
        let pointer_kind = matches!(
            event.kind,
            UiInputDeviceKind::Mouse
                | UiInputDeviceKind::Trackpad
                | UiInputDeviceKind::Touch
                | UiInputDeviceKind::Pen
        );
        pointer_kind && event.position.x.is_finite() && event.position.y.is_finite()
    }

    fn effective_pointer_button(event: UiPointerEvent) -> Option<UiPointerButton> {
        event
            .button
            .or((event.kind == UiInputDeviceKind::Touch).then_some(UiPointerButton::Primary))
    }

    fn process_scroll(
        &mut self,
        event: UiScrollEvent,
        layout: &BTreeMap<UiNodeId, UiRect>,
        effects: &mut UiFrameEffects,
        line_step: f32,
    ) -> bool {
        if !Self::valid_scroll_event(event) {
            if self
                .scroll_capture
                .is_some_and(|capture| capture.device == event.device)
            {
                self.scroll_capture = None;
            }
            effects.diagnostics.push(UiDiagnostic::InvalidScrollEvent);
            return false;
        }
        if matches!(event.phase, UiScrollPhase::End | UiScrollPhase::Cancel) {
            let target = self
                .scroll_capture
                .filter(|capture| capture.device == event.device)
                .map(|capture| capture.target);
            if target.is_some() {
                self.scroll_capture = None;
            }
            effects.scroll_outcomes.push(UiScrollOutcome {
                device: event.device,
                phase: event.phase,
                target,
                consumed: UiPoint::default(),
                remaining: UiPoint::default(),
            });
            return false;
        }

        let target = match event.phase {
            UiScrollPhase::Begin => {
                if self
                    .scroll_capture
                    .is_some_and(|capture| capture.device != event.device)
                {
                    effects.diagnostics.push(UiDiagnostic::InvalidScrollEvent);
                    return false;
                }
                let target = self.scroll_hit_test(event.position, layout);
                self.scroll_capture = target.map(|target| ScrollCapture {
                    device: event.device,
                    target,
                });
                target
            }
            UiScrollPhase::Update => {
                let captured = self
                    .scroll_capture
                    .filter(|capture| capture.device == event.device)
                    .map(|capture| capture.target);
                captured.or_else(|| {
                    let target = self.scroll_hit_test(event.position, layout);
                    self.scroll_capture = target.map(|target| ScrollCapture {
                        device: event.device,
                        target,
                    });
                    target
                })
            }
            UiScrollPhase::Momentum => self
                .scroll_capture
                .filter(|capture| capture.device == event.device)
                .map(|capture| capture.target),
            UiScrollPhase::End | UiScrollPhase::Cancel => None,
        };
        let Some(target) = target else {
            effects
                .diagnostics
                .push(UiDiagnostic::ScrollTargetUnavailable);
            effects.scroll_outcomes.push(UiScrollOutcome {
                device: event.device,
                phase: event.phase,
                target: None,
                consumed: UiPoint::default(),
                remaining: Self::normalized_scroll_delta(event, line_step),
            });
            return false;
        };
        let (consumed, remaining, changed) =
            self.apply_scroll_delta(target, event, layout, line_step);
        effects.scroll_outcomes.push(UiScrollOutcome {
            device: event.device,
            phase: event.phase,
            target: Some(target),
            consumed,
            remaining,
        });
        changed
    }

    fn apply_scroll_delta(
        &mut self,
        target: UiNodeId,
        event: UiScrollEvent,
        layout: &BTreeMap<UiNodeId, UiRect>,
        line_step: f32,
    ) -> (UiPoint, UiPoint, bool) {
        let mut remaining = Self::normalized_scroll_delta(event, line_step);
        let mut consumed = UiPoint::default();
        let mut changed = false;
        let scroll_chain: Vec<_> = self
            .document
            .route_to(target)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter_map(|node_id| {
                let node = self.document.node(node_id)?;
                let UiLayout::Scroll { axis, .. } = node.layout else {
                    return None;
                };
                let maximum = self.scroll_limit(node_id, layout);
                Some((node_id, axis, maximum))
            })
            .collect();
        for (node, axis, maximum) in scroll_chain {
            let requested = if axis == UiAxis::Vertical {
                remaining.y
            } else {
                remaining.x
            };
            if requested.abs() <= f32::EPSILON {
                continue;
            }
            let before = self.scroll_offsets.get(&node).copied().unwrap_or(0.0);
            let after = (before + requested).clamp(0.0, maximum);
            let accepted = after - before;
            if accepted.abs() > f32::EPSILON {
                self.scroll_offsets.insert(node, after);
                changed = true;
            }
            if axis == UiAxis::Vertical {
                consumed.y += accepted;
                remaining.y -= accepted;
            } else {
                consumed.x += accepted;
                remaining.x -= accepted;
            }
        }
        (consumed, remaining, changed)
    }

    fn valid_scroll_event(event: UiScrollEvent) -> bool {
        let scroll_kind = matches!(
            event.kind,
            UiInputDeviceKind::Mouse
                | UiInputDeviceKind::Trackpad
                | UiInputDeviceKind::Touch
                | UiInputDeviceKind::Pen
        );
        scroll_kind
            && event.position.x.is_finite()
            && event.position.y.is_finite()
            && event.delta.x.is_finite()
            && event.delta.y.is_finite()
    }

    fn normalized_scroll_delta(event: UiScrollEvent, line_step: f32) -> UiPoint {
        let factor = if event.delta.unit == UiScrollUnit::Lines {
            line_step.max(1.0)
        } else {
            1.0
        };
        UiPoint {
            x: event.delta.x * factor,
            y: event.delta.y * factor,
        }
    }

    fn begin_drag(
        &mut self,
        source: UiNodeId,
        payload: UiDragPayload,
        keyboard: bool,
        routes: &mut Vec<UiEventRoute>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let accepted = self.document.node(source).is_some_and(|node| {
            !node.semantics.state.disabled && node.drag_source == Some(payload.kind)
        });
        if !accepted {
            diagnostics.push(UiDiagnostic::DragSourceDenied {
                node: source,
                kind: payload.kind,
            });
            return;
        }
        self.dispatch(source, routes);
        self.set_focus(source);
        self.drag = Some(ActiveDrag {
            source,
            over: None,
            payload,
            keyboard,
        });
    }

    fn complete_drag(
        &mut self,
        target: Option<UiNodeId>,
        routes: &mut Vec<UiEventRoute>,
        drops: &mut Vec<UiDropRequest>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some(drag) = self.drag.take() else {
            diagnostics.push(UiDiagnostic::DragUnavailable);
            return;
        };
        let Some(target) = target else {
            diagnostics.push(UiDiagnostic::DragUnavailable);
            return;
        };
        let accepted = self.document.node(target).is_some_and(|node| {
            !node.semantics.state.disabled
                && node.drop_accepts.contains(&drag.payload.kind)
                && node.drop_operations.contains(&drag.payload.operation)
        });
        if !accepted {
            let diagnostic = self.document.node(target).map_or(
                UiDiagnostic::DropTargetDenied {
                    node: target,
                    kind: drag.payload.kind,
                },
                |node| {
                    if node.semantics.state.disabled
                        || !node.drop_accepts.contains(&drag.payload.kind)
                    {
                        UiDiagnostic::DropTargetDenied {
                            node: target,
                            kind: drag.payload.kind,
                        }
                    } else {
                        UiDiagnostic::DropOperationDenied {
                            node: target,
                            operation: drag.payload.operation,
                        }
                    }
                },
            );
            diagnostics.push(diagnostic);
            return;
        }
        self.dispatch(target, routes);
        drops.push(UiDropRequest {
            source: drag.source,
            target,
            payload: drag.payload,
        });
    }

    fn drag_auto_scroll(
        &mut self,
        position: UiPoint,
        layout: &BTreeMap<UiNodeId, UiRect>,
        line_step: f32,
    ) -> bool {
        let Some(target) = self.scroll_hit_test(position, layout) else {
            return false;
        };
        let Some(node) = self.document.node(target) else {
            return false;
        };
        let UiLayout::Scroll { axis, .. } = node.layout else {
            return false;
        };
        let Some(bounds) = layout.get(&target).copied() else {
            return false;
        };
        let threshold = line_step.max(1.0).min(if axis == UiAxis::Vertical {
            bounds.size.height / 2.0
        } else {
            bounds.size.width / 2.0
        });
        let delta = if axis == UiAxis::Vertical {
            if position.y <= bounds.origin.y + threshold {
                -line_step
            } else if position.y >= bounds.origin.y + bounds.size.height - threshold {
                line_step
            } else {
                0.0
            }
        } else if position.x <= bounds.origin.x + threshold {
            -line_step
        } else if position.x >= bounds.origin.x + bounds.size.width - threshold {
            line_step
        } else {
            0.0
        };
        if delta.abs() <= f32::EPSILON {
            return false;
        }
        let maximum = self.scroll_limit(target, layout);
        let before = self.scroll_offsets.get(&target).copied().unwrap_or(0.0);
        let after = (before + delta).clamp(0.0, maximum);
        if (after - before).abs() <= f32::EPSILON {
            return false;
        }
        self.scroll_offsets.insert(target, after);
        true
    }

    fn navigate_collection(
        &mut self,
        navigation: UiCollectionNavigation,
        layout: &BTreeMap<UiNodeId, UiRect>,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let Some((container, visible)) = self.collection_context() else {
            diagnostics.push(UiDiagnostic::NoFocusableNode);
            return;
        };
        let visible_on_page = layout.get(&container).map_or(1, |container_bounds| {
            visible
                .iter()
                .filter(|node| {
                    layout
                        .get(node)
                        .is_some_and(|bounds| Self::rects_intersect(*container_bounds, *bounds))
                })
                .count()
                .max(1)
        });
        if visible.contains(&self.focused.unwrap_or(container)) {
            self.collection_cursor.selected = self.focused;
        }
        if let Some(target) = self
            .collection_cursor
            .navigate(&visible, visible_on_page, navigation)
        {
            self.set_focus(target);
        }
    }

    fn collection_typeahead(&mut self, query: &str, diagnostics: &mut Vec<UiDiagnostic>) {
        if query.len() > MAX_TEXT_BYTES {
            diagnostics.push(UiDiagnostic::CollectionQueryTooLong {
                maximum: MAX_TEXT_BYTES,
            });
            return;
        }
        let Some((_, visible)) = self.collection_context() else {
            diagnostics.push(UiDiagnostic::NoFocusableNode);
            return;
        };
        let query = query.trim().to_lowercase();
        if query.is_empty() || visible.is_empty() {
            return;
        }
        let start = self
            .focused
            .and_then(|focused| visible.iter().position(|node| *node == focused))
            .map_or(0, |index| (index + 1) % visible.len());
        let target = visible
            .iter()
            .cycle()
            .skip(start)
            .take(visible.len())
            .find(|node| {
                self.document
                    .node(**node)
                    .is_some_and(|node| node.semantics.name.to_lowercase().starts_with(&query))
            })
            .copied();
        if let Some(target) = target {
            self.set_focus(target);
        }
    }

    fn collection_context(&self) -> Option<(UiNodeId, Vec<UiNodeId>)> {
        let anchor = self
            .focused
            .or(self.collection_cursor.selected)
            .unwrap_or(self.document.root());
        let container = self
            .document
            .route_to(anchor)?
            .into_iter()
            .rev()
            .find(|node| {
                self.document
                    .node(*node)
                    .is_some_and(|node| Self::is_collection_kind(node.kind))
            })?;
        let mut visible = Vec::new();
        self.collect_collection_members(container, container, &mut visible);
        Some((container, visible))
    }

    fn collect_collection_members(
        &self,
        container: UiNodeId,
        current: UiNodeId,
        visible: &mut Vec<UiNodeId>,
    ) {
        let Some(node) = self.document.node(current) else {
            return;
        };
        if current != container && node.focusable && !node.semantics.state.disabled {
            visible.push(current);
        }
        for child in &node.children {
            self.collect_collection_members(container, *child, visible);
        }
    }

    const fn is_collection_kind(kind: UiWidgetKind) -> bool {
        matches!(
            kind,
            UiWidgetKind::ComboBox
                | UiWidgetKind::MenuBar
                | UiWidgetKind::Menu
                | UiWidgetKind::ContextMenu
                | UiWidgetKind::Tabs
                | UiWidgetKind::Tree
                | UiWidgetKind::Table
                | UiWidgetKind::PropertyGrid
                | UiWidgetKind::VirtualList
        )
    }

    fn rects_intersect(first: UiRect, second: UiRect) -> bool {
        first.origin.x < second.origin.x + second.size.width
            && second.origin.x < first.origin.x + first.size.width
            && first.origin.y < second.origin.y + second.size.height
            && second.origin.y < first.origin.y + first.size.height
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
        let result = self
            .text_inputs
            .get_mut(&target)
            .expect("focused text input has retained state")
            .set_preedit(text, cursor);
        match result {
            Ok(()) => {}
            Err(UiPreeditError::TooLong) => {
                diagnostics.push(UiDiagnostic::TextInputLimitExceeded {
                    node: target,
                    maximum: MAX_TEXT_BYTES,
                });
            }
            Err(UiPreeditError::InvalidCursor) => {
                diagnostics.push(UiDiagnostic::ImeCursorInvalid { node: target });
            }
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
        operation: UiClipboardOperation,
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
            .get_mut(&target)
            .expect("focused text input has retained state");
        if state.is_password() {
            diagnostics.push(UiDiagnostic::ClipboardDeniedForPassword { node: target });
            return;
        }
        let text = state.selected_text().map(str::to_owned);
        if let Some(text) = text {
            clipboard_requests.push(UiClipboardRequest {
                source: target,
                operation,
                text,
            });
        }
    }

    fn request_completion(
        &mut self,
        routes: &mut Vec<UiEventRoute>,
        completion_requests: &mut Vec<UiCompletionRequest>,
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
        let Some(prefix) = state.completion_prefix() else {
            diagnostics.push(UiDiagnostic::CompletionDeniedForPassword { node: target });
            return;
        };
        completion_requests.push(UiCompletionRequest {
            source: target,
            prefix,
        });
    }

    fn text_validation_snapshots(
        &self,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) -> Vec<UiTextValidationSnapshot> {
        self.document
            .nodes()
            .filter_map(|node| {
                let rule = node.text_validation?;
                let valid = self
                    .text_inputs
                    .get(&node.id)
                    .is_some_and(|state| state.is_valid(rule));
                if !valid {
                    diagnostics.push(UiDiagnostic::TextValidationFailed { node: node.id });
                }
                Some(UiTextValidationSnapshot {
                    node: node.id,
                    rule,
                    valid,
                })
            })
            .collect()
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
        self.set_focus(order[next]);
    }

    fn set_focus(&mut self, target: UiNodeId) {
        self.focused = Some(target);
        self.collection_cursor.selected = Some(target);
        if let Some(drag) = self.drag.as_mut().filter(|drag| drag.keyboard) {
            drag.over = Some(target);
        }
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
        if node.semantics.state.disabled {
            return;
        }
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
        (node.focusable && !node.semantics.state.disabled).then_some(id)
    }

    fn scroll_hit_test(
        &self,
        point: UiPoint,
        layout: &BTreeMap<UiNodeId, UiRect>,
    ) -> Option<UiNodeId> {
        self.scroll_hit_test_node(self.document.root(), point, layout)
    }

    fn scroll_hit_test_node(
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
            if let Some(target) = self.scroll_hit_test_node(*child, point, layout) {
                return Some(target);
            }
        }
        matches!(node.layout, UiLayout::Scroll { .. }).then_some(id)
    }

    fn scroll_limit(&self, id: UiNodeId, layout: &BTreeMap<UiNodeId, UiRect>) -> f32 {
        let Some(node) = self.document.node(id) else {
            return 0.0;
        };
        let UiLayout::Scroll { axis, .. } = node.layout else {
            return 0.0;
        };
        let Some(bounds) = layout.get(&id).copied() else {
            return 0.0;
        };
        let viewport = inset_bounds(bounds, node.style.padding);
        let viewport_extent = if axis == UiAxis::Vertical {
            viewport.size.height
        } else {
            viewport.size.width
        };
        let content_extent = node.children.iter().fold(0.0, |extent, child| {
            let preferred = self.document.node(*child).and_then(|child| {
                if axis == UiAxis::Vertical {
                    child.layout_hints.preferred_height
                } else {
                    child.layout_hints.preferred_width
                }
            });
            extent + preferred.map_or(viewport_extent, finite_nonnegative)
        });
        (content_extent - viewport_extent).max(0.0)
    }

    fn clamp_scroll_offsets(&mut self, layout: &BTreeMap<UiNodeId, UiRect>) -> bool {
        let limits: Vec<_> = self
            .document
            .nodes()
            .filter(|node| matches!(node.layout, UiLayout::Scroll { .. }))
            .map(|node| (node.id, self.scroll_limit(node.id, layout)))
            .collect();
        let mut changed = false;
        for (node, maximum) in limits {
            let offset = self.scroll_offsets.get(&node).copied().unwrap_or(0.0);
            let clamped = offset.clamp(0.0, maximum);
            if (clamped - offset).abs() > f32::EPSILON {
                self.scroll_offsets.insert(node, clamped);
                changed = true;
            }
        }
        changed
    }

    fn scroll_snapshots(&self, layout: &BTreeMap<UiNodeId, UiRect>) -> Vec<UiScrollSnapshot> {
        self.document
            .nodes()
            .filter(|node| matches!(node.layout, UiLayout::Scroll { .. }))
            .map(|node| {
                let maximum = self.scroll_limit(node.id, layout);
                UiScrollSnapshot {
                    node: node.id,
                    offset: self
                        .scroll_offsets
                        .get(&node.id)
                        .copied()
                        .unwrap_or(0.0)
                        .clamp(0.0, maximum),
                    maximum,
                }
            })
            .collect()
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
        let mut semantics = node.semantics.clone();
        if let Some(rule) = node.text_validation {
            semantics.state.invalid = self
                .text_inputs
                .get(&id)
                .is_none_or(|state| !state.is_valid(rule));
        }
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
            state: semantics.state,
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
    use meridian_ui_core::{
        UiAbsolutePosition, UiDragItemId, UiScrollDelta, UiStyle, UiTextInputOptions,
    };
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
                cursor: Some((2, 2)),
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

        let cancelled = runtime.reconcile(frame(vec![UiEvent::ImeCancel]));
        assert_eq!(cancelled.preedit, None);
        assert!(!cancelled.text_inputs[0].has_preedit);
        let malformed = runtime.reconcile(frame(vec![UiEvent::ImePreedit {
            text: "é".to_owned(),
            cursor: Some((1, 1)),
        }]));
        assert_eq!(malformed.preedit, None);
        assert!(malformed
            .diagnostics
            .contains(&UiDiagnostic::ImeCursorInvalid { node: input }));

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
    fn frame_rejects_over_limit_input_bytes_without_mutating_state() {
        let (document, input) = text_input_document("safe", false);
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::TextCommit("a".repeat(MAX_TEXT_BYTES + 1)),
        ]));

        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::InputByteLimitExceeded {
                bytes: MAX_TEXT_BYTES + 1,
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
    fn assistive_actions_edit_named_controls_and_reject_nonfocusable_targets() {
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
            UiEvent::AssistiveActivate(label),
        ]));

        assert_eq!(output.focused, Some(input));
        assert_eq!(runtime.text_input_value(input), Some("250"));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveFocusDenied { node: label }));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveActivateDenied { node: label }));
        assert!(output.commands.is_empty());
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
        assert!((bounds(first).origin.y - bounds(scroll).origin.y).abs() < 0.1);
        assert_eq!(
            output.scroll,
            vec![UiScrollSnapshot {
                node: scroll,
                offset: 0.0,
                maximum: 0.0,
            }]
        );
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

    #[test]
    fn aggregate_event_rejection_preserves_private_state_and_revision() {
        let (document, input) = text_input_document("safe", false);
        let mut runtime = UiRuntime::new(document);
        let accepted = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("accepted".to_owned()),
        ]));
        let oversized = frame(vec![UiEvent::FocusNext; MAX_FRAME_EVENTS + 1]);

        assert_eq!(
            runtime.try_reconcile(oversized),
            Err(UiFrameError::TooManyEvents {
                count: MAX_FRAME_EVENTS + 1,
                maximum: MAX_FRAME_EVENTS,
            })
        );
        assert_eq!(runtime.text_input_value(input), Some("accepted"));
        assert_eq!(runtime.revision, accepted.revision);
        assert_eq!(runtime.focused, Some(input));
    }

    #[test]
    fn aggregate_effect_bytes_roll_back_repeated_large_commands() {
        let action = "a".repeat(MAX_TEXT_BYTES);
        let button = UiNodeId::new(0x680);
        let document = UiDocument::new(
            button,
            vec![UiNode::button(button, "Bounded action", action, "Run")],
        )
        .expect("maximum-sized action remains a valid document field");
        let mut runtime = UiRuntime::new(document);

        assert_eq!(
            runtime.try_reconcile(frame(vec![
                UiEvent::FocusNext,
                UiEvent::Activate,
                UiEvent::Activate,
            ])),
            Err(UiFrameError::TooManyEffectBytes {
                bytes: MAX_TEXT_BYTES * 2,
                maximum: MAX_TEXT_BYTES,
            })
        );
        assert_eq!(runtime.focused, None);
        assert_eq!(runtime.revision, 0);
    }

    #[test]
    fn aggregate_route_limit_rolls_back_deep_dispatch_state() {
        const DEPTH: usize = 100;
        let ids: Vec<_> = (0..DEPTH)
            .map(|index| UiNodeId::new(0x700 + u128::try_from(index).expect("bounded index")))
            .collect();
        let mut nodes = Vec::with_capacity(DEPTH);
        for pair in ids.windows(2) {
            nodes.push(UiNode::container(
                pair[0],
                "Route",
                UiLayout::Overlay,
                vec![pair[1]],
            ));
        }
        let target = *ids.last().expect("deep route target");
        nodes.push(UiNode::button(
            target,
            "Target",
            "target.activate",
            "Target",
        ));
        let document = UiDocument::new(ids[0], nodes).expect("deep route document");
        let mut runtime = UiRuntime::new(document);
        let mut events = vec![UiEvent::AssistiveFocus(target)];
        events.extend(std::iter::repeat_n(UiEvent::Activate, 27));

        let error = runtime
            .try_reconcile(frame(events))
            .expect_err("aggregate frame effects must remain bounded");
        assert!(matches!(
            error,
            UiFrameError::TooManyEffects { count, maximum }
                if count > maximum && maximum == MAX_FRAME_EVENTS
        ));
        assert_eq!(runtime.focused, None);
        assert_eq!(runtime.revision, 0);
    }

    #[test]
    fn post_event_validation_cannot_bypass_the_aggregate_effect_limit() {
        let input = UiNodeId::new(0x750);
        let document = UiDocument::new(
            input,
            vec![
                UiNode::text_input(input, "Required", "", UiTextInputOptions::default())
                    .with_text_validation(UiTextValidation::NonEmpty),
            ],
        )
        .expect("validated text document");
        let mut runtime = UiRuntime::new(document);
        let events =
            vec![UiEvent::NavigateCollection(UiCollectionNavigation::Next); MAX_FRAME_EVENTS];

        assert_eq!(
            runtime.try_reconcile(frame(events)),
            Err(UiFrameError::TooManyEffects {
                count: MAX_FRAME_EVENTS + 1,
                maximum: MAX_FRAME_EVENTS,
            })
        );
        assert_eq!(runtime.revision, 0);
        assert_eq!(runtime.text_input_value(input), Some(""));
    }

    #[test]
    fn normalized_pointer_capture_rejects_invalid_release_and_never_activates_early() {
        let root = UiNodeId::new(0x600);
        let action = UiNodeId::new(0x601);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Pointer", UiLayout::Overlay, vec![action]),
                UiNode::button(action, "Action", "fixture.action", "Action"),
            ],
        )
        .expect("pointer document");
        let mut runtime = UiRuntime::new(document);
        let device = UiInputDeviceId::new(9);
        let pointer = |phase, position, button| {
            UiEvent::Pointer(UiPointerEvent {
                device,
                kind: UiInputDeviceKind::Mouse,
                phase,
                position,
                button,
            })
        };

        let pressed = runtime.reconcile(frame(vec![pointer(
            UiPointerPhase::Press,
            UiPoint { x: 20.0, y: 20.0 },
            Some(UiPointerButton::Primary),
        )]));
        assert!(pressed.commands.is_empty());

        let invalid_release = runtime.reconcile(frame(vec![pointer(
            UiPointerPhase::Release,
            UiPoint {
                x: f32::NAN,
                y: 20.0,
            },
            Some(UiPointerButton::Primary),
        )]));
        assert!(invalid_release.commands.is_empty());
        assert!(invalid_release
            .diagnostics
            .contains(&UiDiagnostic::InvalidPointerEvent));

        let orphaned_release = runtime.reconcile(frame(vec![pointer(
            UiPointerPhase::Release,
            UiPoint { x: 20.0, y: 20.0 },
            Some(UiPointerButton::Primary),
        )]));
        assert!(orphaned_release.commands.is_empty());

        let activated = runtime.reconcile(frame(vec![
            pointer(
                UiPointerPhase::Press,
                UiPoint { x: 20.0, y: 20.0 },
                Some(UiPointerButton::Primary),
            ),
            pointer(
                UiPointerPhase::Release,
                UiPoint { x: 20.0, y: 20.0 },
                Some(UiPointerButton::Primary),
            ),
        ]));
        assert_eq!(activated.commands.len(), 1);
    }

    fn nested_scroll_document() -> (UiDocument, UiNodeId, UiNodeId) {
        let outer = UiNodeId::new(0x610);
        let inner = UiNodeId::new(0x611);
        let inner_first = UiNodeId::new(0x612);
        let inner_second = UiNodeId::new(0x613);
        let outer_tail = UiNodeId::new(0x614);
        let document = UiDocument::new(
            outer,
            vec![
                UiNode::container(
                    outer,
                    "Outer scroll",
                    UiLayout::Scroll {
                        axis: UiAxis::Vertical,
                        offset: 0.0,
                    },
                    vec![inner, outer_tail],
                )
                .with_style(UiStyle::transparent()),
                UiNode::container(
                    inner,
                    "Inner scroll",
                    UiLayout::Scroll {
                        axis: UiAxis::Vertical,
                        offset: 0.0,
                    },
                    vec![inner_first, inner_second],
                )
                .with_style(UiStyle::transparent())
                .with_layout_hints(UiLayoutHints::fixed_height(60.0)),
                UiNode::label(inner_first, "Inner first", "Inner first")
                    .with_layout_hints(UiLayoutHints::fixed_height(50.0)),
                UiNode::label(inner_second, "Inner second", "Inner second")
                    .with_layout_hints(UiLayoutHints::fixed_height(50.0)),
                UiNode::label(outer_tail, "Outer tail", "Outer tail")
                    .with_layout_hints(UiLayoutHints::fixed_height(100.0)),
            ],
        )
        .expect("nested scroll document");
        (document, outer, inner)
    }

    fn scroll_event(
        device: UiInputDeviceId,
        phase: UiScrollPhase,
        unit: UiScrollUnit,
        y: f32,
    ) -> UiEvent {
        UiEvent::Scroll(UiScrollEvent {
            device,
            kind: UiInputDeviceKind::Trackpad,
            phase,
            position: UiPoint { x: 10.0, y: 10.0 },
            delta: UiScrollDelta { x: 0.0, y, unit },
        })
    }

    #[test]
    fn nested_scroll_hands_residual_to_ancestor_and_preserves_momentum_target() {
        let (document, outer, inner) = nested_scroll_document();
        let mut runtime = UiRuntime::new(document);
        let device = UiInputDeviceId::new(10);
        let mut input = frame(vec![
            scroll_event(device, UiScrollPhase::Begin, UiScrollUnit::Pixels, 70.0),
            scroll_event(device, UiScrollPhase::Momentum, UiScrollUnit::Pixels, 10.5),
            scroll_event(device, UiScrollPhase::End, UiScrollUnit::Pixels, 0.0),
        ]);
        input.viewport = UiSize::new(200.0, 100.0);
        let output = runtime.reconcile(input);
        let offset = |node| {
            output
                .scroll
                .iter()
                .find(|snapshot| snapshot.node == node)
                .map(|snapshot| snapshot.offset)
                .expect("scroll node is reported")
        };
        assert!((offset(inner) - 40.0).abs() < 0.01);
        assert!((offset(outer) - 40.5).abs() < 0.01);
        assert_eq!(output.scroll_outcomes[0].target, Some(inner));
        assert!((output.scroll_outcomes[0].consumed.y - 70.0).abs() < 0.01);
        assert!(output.scroll_outcomes[0].remaining.y.abs() < 0.01);
        assert!((output.scroll_outcomes[1].consumed.y - 10.5).abs() < 0.01);
        assert_eq!(output.scroll_outcomes[2].phase, UiScrollPhase::End);
    }

    #[test]
    fn line_scroll_is_normalized_once_and_pixel_delta_remains_precise() {
        let (document, _, inner) = nested_scroll_document();
        let mut runtime = UiRuntime::new(document);
        let device = UiInputDeviceId::new(11);
        let mut input = frame(vec![scroll_event(
            device,
            UiScrollPhase::Begin,
            UiScrollUnit::Lines,
            1.0,
        )]);
        input.viewport = UiSize::new(200.0, 100.0);
        let lines = runtime.reconcile(input);
        assert!((lines.scroll_outcomes[0].consumed.y - 16.0).abs() < 0.01);
        assert!(
            (lines
                .scroll
                .iter()
                .find(|row| row.node == inner)
                .unwrap()
                .offset
                - 16.0)
                .abs()
                < 0.01
        );
    }

    fn drag_document() -> (UiDocument, UiNodeId, UiNodeId) {
        let root = UiNodeId::new(0x620);
        let source = UiNodeId::new(0x621);
        let target = UiNodeId::new(0x622);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Drag fixture",
                    UiLayout::Absolute,
                    vec![source, target],
                )
                .with_style(UiStyle::transparent()),
                UiNode::button(source, "Asset source", "source.activate", "Asset")
                    .with_drag_source(UiDragKind::Asset)
                    .with_absolute_position(UiAbsolutePosition {
                        left: 0.0,
                        top: 0.0,
                        width: Some(80.0),
                        height: Some(40.0),
                    }),
                UiNode::button(target, "World target", "target.activate", "World")
                    .accepting_drop([UiDragKind::Asset])
                    .with_absolute_position(UiAbsolutePosition {
                        left: 100.0,
                        top: 0.0,
                        width: Some(80.0),
                        height: Some(40.0),
                    }),
            ],
        )
        .expect("drag document");
        (document, source, target)
    }

    #[test]
    fn pointer_and_keyboard_drag_emit_typed_proposals_with_cancel_and_rejection() {
        let payload = UiDragPayload {
            kind: UiDragKind::Asset,
            item: UiDragItemId::new(1),
            operation: UiDropOperation::Move,
        };
        let (document, source, target) = drag_document();
        let mut runtime = UiRuntime::new(document.clone());
        let device = UiInputDeviceId::new(12);
        let pointer = |phase, x| {
            UiEvent::Pointer(UiPointerEvent {
                device,
                kind: UiInputDeviceKind::Mouse,
                phase,
                position: UiPoint { x, y: 20.0 },
                button: matches!(phase, UiPointerPhase::Press | UiPointerPhase::Release)
                    .then_some(UiPointerButton::Primary),
            })
        };
        let pointer_drop = runtime.reconcile(frame(vec![
            pointer(UiPointerPhase::Press, 20.0),
            UiEvent::BeginDrag(payload),
            pointer(UiPointerPhase::Move, 120.0),
            pointer(UiPointerPhase::Release, 120.0),
        ]));
        assert_eq!(
            pointer_drop.drops,
            vec![UiDropRequest {
                source,
                target,
                payload,
            }]
        );
        assert!(pointer_drop.commands.is_empty());

        let mut keyboard_runtime = UiRuntime::new(document.clone());
        let keyboard_drop = keyboard_runtime.reconcile(frame(vec![
            UiEvent::BeginKeyboardDrag { source, payload },
            UiEvent::AssistiveFocus(target),
            UiEvent::CompleteDrag,
        ]));
        assert_eq!(keyboard_drop.drops.len(), 1);

        let mut cancelled_runtime = UiRuntime::new(document.clone());
        let cancelled = cancelled_runtime.reconcile(frame(vec![
            UiEvent::BeginKeyboardDrag { source, payload },
            UiEvent::AssistiveFocus(target),
            UiEvent::CancelDrag,
            UiEvent::CompleteDrag,
        ]));
        assert!(cancelled.drops.is_empty());
        assert!(cancelled
            .diagnostics
            .contains(&UiDiagnostic::DragUnavailable));

        let denied = cancelled_runtime.reconcile(frame(vec![UiEvent::BeginKeyboardDrag {
            source,
            payload: UiDragPayload {
                kind: UiDragKind::Entity,
                item: UiDragItemId::new(2),
                operation: UiDropOperation::Move,
            },
        }]));
        assert!(denied
            .diagnostics
            .contains(&UiDiagnostic::DragSourceDenied {
                node: source,
                kind: UiDragKind::Entity,
            }));

        let mut operation_runtime = UiRuntime::new(document);
        let copy_payload = UiDragPayload {
            operation: UiDropOperation::Copy,
            ..payload
        };
        let operation_denied = operation_runtime.reconcile(frame(vec![
            UiEvent::BeginKeyboardDrag {
                source,
                payload: copy_payload,
            },
            UiEvent::AssistiveFocus(target),
            UiEvent::CompleteDrag,
        ]));
        assert!(operation_denied
            .diagnostics
            .contains(&UiDiagnostic::DropOperationDenied {
                node: target,
                operation: UiDropOperation::Copy,
            }));
    }

    #[test]
    fn text_validation_completion_cut_and_paste_remain_typed_and_password_safe() {
        let input = UiNodeId::new(0x630);
        let document = UiDocument::new(
            input,
            vec![
                UiNode::text_input(input, "Integer", "12", UiTextInputOptions::default())
                    .with_text_validation(UiTextValidation::Integer),
            ],
        )
        .expect("validated text document");
        let mut runtime = UiRuntime::new(document);
        let invalid = runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("bad".to_owned()),
            UiEvent::RequestCompletion,
        ]));
        assert_eq!(invalid.completion_requests[0].prefix, "bad");
        assert_eq!(
            invalid.text_validation,
            vec![UiTextValidationSnapshot {
                node: input,
                rule: UiTextValidation::Integer,
                valid: false,
            }]
        );
        assert!(invalid
            .diagnostics
            .contains(&UiDiagnostic::TextValidationFailed { node: input }));
        let semantic_state = match &invalid.semantic_delta {
            SemanticDelta::Replace(tree) => tree
                .nodes
                .iter()
                .find(|node| node.id == input)
                .map(|node| node.state)
                .expect("input semantic node"),
            SemanticDelta::Unchanged => panic!("first frame publishes semantics"),
        };
        assert!(semantic_state.invalid);

        let cut_requested =
            runtime.reconcile(frame(vec![UiEvent::SelectAllText, UiEvent::CutSelection]));
        assert_eq!(
            cut_requested.clipboard_requests[0].operation,
            UiClipboardOperation::Cut
        );
        assert_eq!(cut_requested.clipboard_requests[0].text, "bad");
        assert_eq!(runtime.text_input_value(input), Some("bad"));

        let repaired = runtime.reconcile(frame(vec![
            UiEvent::ConfirmClipboardCut {
                source: input,
                text: "bad".to_owned(),
            },
            UiEvent::PasteText("42".to_owned()),
        ]));
        assert!(repaired.text_validation[0].valid);
        assert_eq!(runtime.text_input_value(input), Some("42"));

        let (password_document, password) = text_input_document("", true);
        let mut password_runtime = UiRuntime::new(password_document);
        let denied = password_runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::TextCommit("secret".to_owned()),
            UiEvent::RequestCompletion,
        ]));
        assert!(denied.completion_requests.is_empty());
        assert!(denied
            .diagnostics
            .contains(&UiDiagnostic::CompletionDeniedForPassword { node: password }));
    }

    #[test]
    fn text_undo_and_redo_follow_the_focused_private_editor() {
        let (document, input) = text_input_document("one", false);
        let mut runtime = UiRuntime::new(document);
        runtime.reconcile(frame(vec![
            UiEvent::FocusNext,
            UiEvent::SelectAllText,
            UiEvent::TextCommit("two".to_owned()),
            UiEvent::TextCommit("!".to_owned()),
        ]));
        assert_eq!(runtime.text_input_value(input), Some("two!"));

        runtime.reconcile(frame(vec![UiEvent::UndoText, UiEvent::UndoText]));
        assert_eq!(runtime.text_input_value(input), Some("one"));
        runtime.reconcile(frame(vec![UiEvent::RedoText, UiEvent::RedoText]));
        assert_eq!(runtime.text_input_value(input), Some("two!"));
    }

    fn collection_document(include_gamma: bool) -> (UiDocument, UiNodeId, UiNodeId, UiNodeId) {
        let tree = UiNodeId::new(0x640);
        let alpha = UiNodeId::new(0x641);
        let beta = UiNodeId::new(0x642);
        let gamma = UiNodeId::new(0x643);
        let mut children = vec![alpha, beta];
        if include_gamma {
            children.push(gamma);
        }
        let mut nodes = vec![
            UiNode::tree(tree, "Items", children),
            UiNode::tree_item(alpha, "Alpha", "alpha", false, false),
            UiNode::tree_item(beta, "Beta", "beta", false, false),
        ];
        if include_gamma {
            nodes.push(UiNode::tree_item(gamma, "Gamma", "gamma", false, false));
        }
        (
            UiDocument::new(tree, nodes).expect("collection document"),
            alpha,
            beta,
            gamma,
        )
    }

    #[test]
    fn collection_navigation_typeahead_and_filtering_preserve_stable_identity() {
        let (document, alpha, _, gamma) = collection_document(true);
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![
            UiEvent::AssistiveFocus(alpha),
            UiEvent::NavigateCollection(UiCollectionNavigation::End),
            UiEvent::CollectionTypeahead("ga".to_owned()),
        ]));
        assert_eq!(output.focused, Some(gamma));

        let (filtered, _, _, _) = collection_document(false);
        runtime.replace_document(filtered);
        assert_eq!(runtime.focused, None);
        assert_eq!(runtime.collection_cursor.selected, Some(gamma));

        let (restored, _, _, _) = collection_document(true);
        runtime.replace_document(restored);
        assert_eq!(runtime.focused, Some(gamma));
    }
}
