//! Retained Meridian UI reconciliation and interaction runtime.

mod motion;

pub use meridian_ui_core::UiSpatialMotionKind;
pub use motion::*;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use meridian_ui_core::{
    sanitized_scale_factor, MotionPreference, ThemeId, UiAlignment, UiAxis, UiCollectionCursor,
    UiCollectionNavigation, UiColor, UiConstraints, UiContrast, UiDensity, UiDocument,
    UiDocumentDelta, UiDocumentError, UiDragKind, UiDragPayload, UiDropOperation, UiInputDeviceId,
    UiInputDeviceKind, UiLayout, UiNode, UiNodeId, UiPoint, UiPointerButton, UiPointerEvent,
    UiPointerPhase, UiRect, UiScrollEvent, UiScrollPhase, UiScrollUnit, UiSemanticRelationships,
    UiSharedElementId, UiSize, UiStyle, UiStyleSelector, UiTextValidation, UiTheme, UiVisualState,
    UiWidgetKind, MAX_FRAME_EVENTS, MAX_TEXT_BYTES,
};
use meridian_ui_render::{
    icon_geometry, DisplayList, DisplayListError, DisplayPrimitive, UiClipId, UiCornerRadii,
    UiStroke,
};
use meridian_ui_semantics::{
    SemanticAction, SemanticCollectionItem, SemanticDelta, SemanticLive, SemanticNode,
    SemanticRelationships, SemanticTree, SemanticTreeError,
};
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
    AssistiveSetValue {
        target: UiNodeId,
        text: String,
        replace_selection: bool,
    },
}

impl UiEvent {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::TextCommit(text) | Self::PasteText(text) | Self::CollectionTypeahead(text) => {
                text.len()
            }
            Self::ImePreedit { text, .. }
            | Self::ConfirmClipboardCut { text, .. }
            | Self::AssistiveSetValue { text, .. } => text.len(),
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
    TextFontSubstituted {
        node: UiNodeId,
    },
    TextRasterIncomplete {
        node: UiNodeId,
    },
    IconGeometryRejected {
        node: UiNodeId,
    },
    StyleTokenFallback {
        node: UiNodeId,
    },
    IconThemeTokensFallback {
        node: UiNodeId,
    },
    MotionThemeTokensFallback,
    MotionTrackRejected {
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
    AssistiveEditDenied {
        node: UiNodeId,
    },
    InvalidPointerEvent,
    InvalidScrollEvent,
    /// A defensive layout traversal guard found a repeated retained node.
    /// Validated documents cannot ordinarily reach this state, but rejecting it
    /// keeps a corrupted in-memory document from recursing indefinitely.
    LayoutConstraintCycle {
        chain: Vec<UiNodeId>,
    },
    /// The document passed individual geometry checks but its minimum, maximum,
    /// and aspect requirements could not be satisfied as one rectangle.
    LayoutConstraintsUnsatisfiable {
        node: UiNodeId,
    },
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
    SemanticTreeRejected(SemanticTreeError),
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
    /// Caller-owned monotonic presentation interval for this frame.
    pub presentation_delta_ms: u32,
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
            presentation_delta_ms: 16,
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct RejectedFrameContext {
    theme: ThemeId,
    density: UiDensity,
    contrast: UiContrast,
    motion: MotionPreference,
    scale_factor: f32,
    input_events: usize,
}

/// Accepted logical geometry for one stable retained node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiLayoutSnapshot {
    pub node: UiNodeId,
    pub bounds: UiRect,
}

/// Retained visual state and resolved selector for one immutable frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiVisualStateSnapshot {
    pub node: UiNodeId,
    pub state: UiVisualState,
    pub selector: UiStyleSelector,
}

/// Availability of a measurement owned by a later renderer or host adapter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiMeasurementAvailability {
    #[default]
    Unavailable,
    Available,
}

/// Explicit timing measurement contract for one reconciled frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiFrameTimingDiagnostics {
    pub reconciliation: UiMeasurementAvailability,
    pub layout: UiMeasurementAvailability,
    pub text_shaping: UiMeasurementAvailability,
    pub text_rasterization: UiMeasurementAvailability,
    pub display_validation: UiMeasurementAvailability,
    pub semantic_delta: UiMeasurementAvailability,
}

/// Renderer-independent overdraw reporting for one frame.
///
/// The retained runtime knows primitive declarations but not clipped backend
/// fragments, blend behavior, or target coverage. It therefore leaves the
/// estimate unavailable until a renderer adapter supplies a measured value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiOverdrawDiagnostics {
    pub estimate: UiMeasurementAvailability,
}

/// Input-to-presentation latency reporting for one frame.
///
/// Platform events currently do not carry source timestamps through the public
/// Meridian event contract, so the runtime must not reinterpret the frame
/// presentation interval as event latency.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiEventLatencyDiagnostics {
    pub measurement: UiMeasurementAvailability,
    pub source_timestamped_events: u32,
}

/// Capture activity observed by the retained runtime.
///
/// Frame capture is owned by renderer and platform adapters. The runtime emits
/// this explicit state rather than implying that a display-list snapshot was a
/// captured image.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiCaptureState {
    #[default]
    NotRequested,
}

/// Virtualization evidence the runtime can truthfully derive from a retained
/// document. Collection sources own requested/realized ranges and cache
/// residency, so this runtime only counts declared regions and marks those
/// adapter-owned fields unavailable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiVirtualizationDiagnostics {
    pub declared_regions: u32,
    pub realized_ranges: UiMeasurementAvailability,
    pub cache_state: UiMeasurementAvailability,
}

/// Stable identity reconciliation and layout coverage for one frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiReconciliationDiagnostics {
    pub accepted_nodes: u32,
    pub retained_nodes: u32,
    pub inserted_nodes: u32,
    pub removed_nodes: u32,
    pub updated_nodes: u32,
    pub layout_roots: u32,
    pub reconciled_layout_nodes: u32,
}

/// Display-list primitive coverage emitted by the retained runtime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiPrimitiveDiagnostics {
    pub rects: u32,
    pub borders: u32,
    pub text: u32,
    pub glyph_runs: u32,
    pub focus_indicators: u32,
    pub rounded_rects: u32,
    pub paths: u32,
    pub images: u32,
    pub meshes: u32,
    pub clip_pushes: u32,
    pub clip_pops: u32,
    pub layer_begins: u32,
    pub layer_ends: u32,
    pub shadows: u32,
    pub backdrops: u32,
}

impl UiPrimitiveDiagnostics {
    fn record(&mut self, primitive: &DisplayPrimitive) {
        match primitive {
            DisplayPrimitive::Rect { .. } => self.rects = self.rects.saturating_add(1),
            DisplayPrimitive::Border { .. } => self.borders = self.borders.saturating_add(1),
            DisplayPrimitive::Text { .. } => self.text = self.text.saturating_add(1),
            DisplayPrimitive::GlyphRun { .. } => {
                self.glyph_runs = self.glyph_runs.saturating_add(1);
            }
            DisplayPrimitive::FocusIndicator { .. } => {
                self.focus_indicators = self.focus_indicators.saturating_add(1);
            }
            DisplayPrimitive::RoundedRect { .. } => {
                self.rounded_rects = self.rounded_rects.saturating_add(1);
            }
            DisplayPrimitive::Path { .. } => self.paths = self.paths.saturating_add(1),
            DisplayPrimitive::Image { .. } => self.images = self.images.saturating_add(1),
            DisplayPrimitive::Mesh { .. } => self.meshes = self.meshes.saturating_add(1),
            DisplayPrimitive::PushClip { .. } => {
                self.clip_pushes = self.clip_pushes.saturating_add(1);
            }
            DisplayPrimitive::PopClip { .. } => self.clip_pops = self.clip_pops.saturating_add(1),
            DisplayPrimitive::BeginLayer { .. } => {
                self.layer_begins = self.layer_begins.saturating_add(1);
            }
            DisplayPrimitive::EndLayer { .. } => {
                self.layer_ends = self.layer_ends.saturating_add(1);
            }
            DisplayPrimitive::Shadow { .. } => self.shadows = self.shadows.saturating_add(1),
            DisplayPrimitive::Backdrop { .. } => self.backdrops = self.backdrops.saturating_add(1),
        }
    }
}

/// Text work observed from Meridian-owned layout/raster summaries.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTextWorkDiagnostics {
    pub primitives: u32,
    pub glyphs: u32,
    pub raster_glyphs: u32,
    pub fallback_metrics: u32,
    pub font_substitutions: u32,
    pub unrasterized_primitives: u32,
}

/// Input, routing, focus, capture, virtualization, and motion coverage for a frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiInteractionDiagnostics {
    pub input_events: u32,
    pub event_routes: u32,
    pub focus_entries: u32,
    pub pointer_captures: u32,
    pub scroll_captures: u32,
    pub active_drags: u32,
    pub virtualized_regions: u32,
    pub active_animation_tracks: Option<u32>,
    pub recovery_events: u32,
}

/// Renderer/cache measurements are owned by renderer adapters, not this runtime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiRendererCacheDiagnostics {
    pub renderer_batches: Option<u32>,
    pub renderer_draws: Option<u32>,
    pub cache_hits: Option<u32>,
    pub cache_misses: Option<u32>,
    pub cache_evictions: Option<u32>,
}

/// Bounded frame summary consumed by diagnostics, renderer caches, and recovery evidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiFrameDiagnostics {
    pub reconciliation: UiReconciliationDiagnostics,
    pub timing: UiFrameTimingDiagnostics,
    pub overdraw: UiOverdrawDiagnostics,
    pub event_latency: UiEventLatencyDiagnostics,
    pub capture: UiCaptureState,
    pub primitives: UiPrimitiveDiagnostics,
    pub text: UiTextWorkDiagnostics,
    pub interaction: UiInteractionDiagnostics,
    pub virtualization: UiVirtualizationDiagnostics,
    pub renderer_cache: UiRendererCacheDiagnostics,
    pub layout_nodes: u32,
    pub display_primitives: u32,
    pub semantic_nodes: u32,
    pub event_routes: u32,
    pub commands: u32,
    pub clipboard_requests: u32,
    pub completion_requests: u32,
    pub text_inputs: u32,
    pub scroll_snapshots: u32,
    pub scroll_outcomes: u32,
    pub drops: u32,
    pub diagnostics: u32,
    pub scale_factor: f32,
    pub contrast: UiContrast,
    pub motion: MotionPreference,
    pub recovered_previous_snapshot: bool,
}

impl Default for UiFrameDiagnostics {
    fn default() -> Self {
        Self {
            reconciliation: UiReconciliationDiagnostics::default(),
            timing: UiFrameTimingDiagnostics::default(),
            overdraw: UiOverdrawDiagnostics::default(),
            event_latency: UiEventLatencyDiagnostics::default(),
            capture: UiCaptureState::default(),
            primitives: UiPrimitiveDiagnostics::default(),
            text: UiTextWorkDiagnostics::default(),
            interaction: UiInteractionDiagnostics {
                active_animation_tracks: None,
                ..UiInteractionDiagnostics::default()
            },
            virtualization: UiVirtualizationDiagnostics::default(),
            renderer_cache: UiRendererCacheDiagnostics::default(),
            layout_nodes: 0,
            display_primitives: 0,
            semantic_nodes: 0,
            event_routes: 0,
            commands: 0,
            clipboard_requests: 0,
            completion_requests: 0,
            text_inputs: 0,
            scroll_snapshots: 0,
            scroll_outcomes: 0,
            drops: 0,
            diagnostics: 0,
            scale_factor: 1.0,
            contrast: UiContrast::Standard,
            motion: MotionPreference::Full,
            recovered_previous_snapshot: false,
        }
    }
}

impl UiFrameDiagnostics {
    fn count(value: usize) -> u32 {
        u32::try_from(value).unwrap_or(u32::MAX)
    }

    fn primitives_and_text(
        snapshot: &UiFrameSnapshot,
    ) -> (UiPrimitiveDiagnostics, UiTextWorkDiagnostics) {
        let mut primitives = UiPrimitiveDiagnostics::default();
        let mut text = UiTextWorkDiagnostics::default();
        for primitive in &snapshot.display_list.primitives {
            primitives.record(primitive);
            match primitive {
                DisplayPrimitive::Text { layout, raster, .. }
                | DisplayPrimitive::GlyphRun { layout, raster, .. } => {
                    text.primitives = text.primitives.saturating_add(1);
                    text.glyphs = text.glyphs.saturating_add(Self::count(layout.glyph_count));
                    text.raster_glyphs = text
                        .raster_glyphs
                        .saturating_add(Self::count(raster.glyphs.len()));
                    if layout.used_fallback_metrics {
                        text.fallback_metrics = text.fallback_metrics.saturating_add(1);
                    }
                    if layout.used_fallback_font {
                        text.font_substitutions = text.font_substitutions.saturating_add(1);
                    }
                    if raster.has_unrasterized_glyphs {
                        text.unrasterized_primitives =
                            text.unrasterized_primitives.saturating_add(1);
                    }
                }
                DisplayPrimitive::Rect { .. }
                | DisplayPrimitive::Border { .. }
                | DisplayPrimitive::FocusIndicator { .. }
                | DisplayPrimitive::RoundedRect { .. }
                | DisplayPrimitive::Path { .. }
                | DisplayPrimitive::Image { .. }
                | DisplayPrimitive::Mesh { .. }
                | DisplayPrimitive::PushClip { .. }
                | DisplayPrimitive::PopClip { .. }
                | DisplayPrimitive::BeginLayer { .. }
                | DisplayPrimitive::EndLayer { .. }
                | DisplayPrimitive::Shadow { .. }
                | DisplayPrimitive::Backdrop { .. } => {}
            }
        }
        (primitives, text)
    }

    fn reconciliation(
        runtime: &UiRuntime,
        snapshot: &UiFrameSnapshot,
    ) -> UiReconciliationDiagnostics {
        UiReconciliationDiagnostics {
            accepted_nodes: Self::count(runtime.document.nodes().count()),
            retained_nodes: Self::count(runtime.last_document_delta.retained.len()),
            inserted_nodes: Self::count(runtime.last_document_delta.inserted.len()),
            removed_nodes: Self::count(runtime.last_document_delta.removed.len()),
            updated_nodes: Self::count(runtime.last_document_delta.updated.len()),
            // A rejected frame may expose a prior immutable snapshot while a
            // newer retained document is awaiting remediation. The root count
            // therefore belongs to the snapshot being reported, not the
            // currently retained document.
            layout_roots: u32::from(
                snapshot
                    .semantic_tree
                    .root
                    .is_some_and(|root| snapshot.layout.iter().any(|entry| entry.node == root)),
            ),
            reconciled_layout_nodes: Self::count(snapshot.layout.len()),
        }
    }

    fn declared_virtualized_regions(runtime: &UiRuntime) -> u32 {
        Self::count(
            runtime
                .document
                .nodes()
                .filter(|node| node.kind == UiWidgetKind::VirtualList)
                .count(),
        )
    }

    fn interaction(
        runtime: &UiRuntime,
        snapshot: &UiFrameSnapshot,
        input_events: usize,
        virtualized_regions: u32,
        recovered_previous_snapshot: bool,
    ) -> UiInteractionDiagnostics {
        UiInteractionDiagnostics {
            input_events: Self::count(input_events),
            event_routes: Self::count(snapshot.event_routes.len()),
            focus_entries: u32::from(snapshot.focused.is_some()),
            pointer_captures: u32::from(runtime.pointer_capture.is_some()),
            scroll_captures: u32::from(runtime.scroll_capture.is_some()),
            active_drags: u32::from(snapshot.drag.is_some()),
            virtualized_regions,
            active_animation_tracks: Some(Self::count(runtime.motion_system.active_count())),
            recovery_events: u32::from(recovered_previous_snapshot),
        }
    }

    fn from_runtime(
        runtime: &UiRuntime,
        snapshot: &UiFrameSnapshot,
        recovered_previous_snapshot: bool,
        input_events: usize,
    ) -> Self {
        let (primitives, text) = Self::primitives_and_text(snapshot);
        let virtualized_regions = Self::declared_virtualized_regions(runtime);
        Self {
            reconciliation: Self::reconciliation(runtime, snapshot),
            timing: UiFrameTimingDiagnostics::default(),
            overdraw: UiOverdrawDiagnostics::default(),
            event_latency: UiEventLatencyDiagnostics {
                measurement: UiMeasurementAvailability::Unavailable,
                // UiEvent intentionally carries no platform timestamp. Do not
                // turn `presentation_delta_ms` into an invented latency.
                source_timestamped_events: 0,
            },
            capture: UiCaptureState::NotRequested,
            primitives,
            text,
            interaction: Self::interaction(
                runtime,
                snapshot,
                input_events,
                virtualized_regions,
                recovered_previous_snapshot,
            ),
            virtualization: UiVirtualizationDiagnostics {
                declared_regions: virtualized_regions,
                // The document has only currently retained children. A source
                // collection owns its requested/realized range and cache.
                realized_ranges: UiMeasurementAvailability::Unavailable,
                cache_state: UiMeasurementAvailability::Unavailable,
            },
            renderer_cache: UiRendererCacheDiagnostics::default(),
            layout_nodes: Self::count(snapshot.layout.len()),
            display_primitives: Self::count(snapshot.display_list.primitives.len()),
            semantic_nodes: Self::count(snapshot.semantic_tree.nodes.len()),
            event_routes: Self::count(snapshot.event_routes.len()),
            commands: Self::count(snapshot.commands.len()),
            clipboard_requests: Self::count(snapshot.clipboard_requests.len()),
            completion_requests: Self::count(snapshot.completion_requests.len()),
            text_inputs: Self::count(snapshot.text_inputs.len()),
            scroll_snapshots: Self::count(snapshot.scroll.len()),
            scroll_outcomes: Self::count(snapshot.scroll_outcomes.len()),
            drops: Self::count(snapshot.drops.len()),
            diagnostics: Self::count(snapshot.diagnostics.len()),
            scale_factor: sanitized_scale_factor(snapshot.scale_factor),
            contrast: snapshot.contrast,
            motion: snapshot.motion,
            recovered_previous_snapshot,
        }
    }
}

/// Immutable frame result handed to renderer and semantic adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFrameSnapshot {
    pub revision: u64,
    /// Target geometry used immediately for layout, interaction, and semantics.
    pub layout: Vec<UiLayoutSnapshot>,
    /// Renderer-facing geometry after bounded presentation-only motion.
    pub presentation_layout: Vec<UiLayoutSnapshot>,
    /// Ordered current/target presentation tracks consumed by this frame.
    pub presentation_motion: Vec<UiMotionSnapshot>,
    pub visual_states: Vec<UiVisualStateSnapshot>,
    pub theme: ThemeId,
    pub density: UiDensity,
    pub contrast: UiContrast,
    pub motion: MotionPreference,
    pub scale_factor: f32,
    pub display_list: DisplayList,
    pub semantic_tree: SemanticTree,
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
    pub frame_diagnostics: UiFrameDiagnostics,
    pub focused: Option<UiNodeId>,
    pub preedit: Option<String>,
}

/// Shared immutable compatibility handle retained while callers migrate.
pub type UiFrameOutput = Arc<UiFrameSnapshot>;

/// Typed frame rejection before any mutated interaction state is committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiFrameError {
    TooManyEvents { count: usize, maximum: usize },
    TooManyInputBytes { bytes: usize, maximum: usize },
    TooManyEffects { count: usize, maximum: usize },
    TooManyEffectBytes { bytes: usize, maximum: usize },
    LayoutRejected(UiLayoutError),
    InvalidDisplayList(DisplayListError),
    SemanticTreeRejected(SemanticTreeError),
}

/// Layout failure detected before an immutable frame can be accepted.
///
/// A [`UiDocument`] validates its retained tree before construction, so a
/// normal public document cannot contain a cycle. The runtime retains this
/// guard as recovery protection for corrupted or future incremental sources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiLayoutError {
    ConstraintCycle { chain: Vec<UiNodeId> },
    UnsatisfiableConstraints { node: UiNodeId },
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiAnimatedColorSlot {
    Border,
    Background,
    Foreground,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UiAnimatedColorTarget {
    slot: UiAnimatedColorSlot,
    color: UiColor,
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

fn finite_sum(left: f32, right: f32) -> f32 {
    let result = left + right;
    if result.is_finite() {
        result
    } else if result.is_sign_negative() {
        -f32::MAX
    } else {
        f32::MAX
    }
}

fn finite_product(left: f32, right: f32) -> f32 {
    let result = left * right;
    if result.is_finite() {
        result
    } else if result.is_sign_negative() {
        -f32::MAX
    } else {
        f32::MAX
    }
}

fn finite_quotient(numerator: f32, denominator: f32) -> f32 {
    let result = numerator / denominator;
    if result.is_finite() {
        result
    } else if result.is_sign_negative() {
        -f32::MAX
    } else {
        f32::MAX
    }
}

fn effective_preferences(input: &UiFrameInput) -> (UiContrast, MotionPreference) {
    (
        if input.high_contrast {
            UiContrast::High
        } else {
            input.contrast
        },
        if input.reduced_motion {
            MotionPreference::Reduced
        } else {
            input.motion
        },
    )
}

fn animated_color_target(style: UiStyle) -> UiAnimatedColorTarget {
    if let Some(border) = style.border {
        UiAnimatedColorTarget {
            slot: UiAnimatedColorSlot::Border,
            color: border.color,
        }
    } else if let Some(background) = style.background {
        UiAnimatedColorTarget {
            slot: UiAnimatedColorSlot::Background,
            color: background,
        }
    } else {
        UiAnimatedColorTarget {
            slot: UiAnimatedColorSlot::Foreground,
            color: style.foreground,
        }
    }
}

fn apply_animated_color(style: &mut UiStyle, slot: UiAnimatedColorSlot, color: UiColor) {
    match slot {
        UiAnimatedColorSlot::Border => {
            if let Some(border) = style.border.as_mut() {
                border.color = color;
            }
        }
        UiAnimatedColorSlot::Background => style.background = Some(color),
        UiAnimatedColorSlot::Foreground => style.foreground = color,
    }
}

fn apply_presentation_opacity(style: &mut UiStyle, opacity: f32) {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let apply = |color: UiColor| {
        UiColor::rgba(
            color.red,
            color.green,
            color.blue,
            finite_product(color.alpha, opacity).clamp(0.0, 1.0),
        )
    };
    style.background = style.background.map(apply);
    if let Some(border) = style.border.as_mut() {
        border.color = apply(border.color);
    }
    style.foreground = apply(style.foreground);
}

#[derive(Clone, Copy)]
struct UiPresentationTransform {
    scale_x: f32,
    scale_y: f32,
    translate_x: f32,
    translate_y: f32,
}

impl UiPresentationTransform {
    const fn identity() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    fn from_rects(target: UiRect, current: UiRect) -> Self {
        let scale_x = if target.size.width > f32::EPSILON {
            finite_quotient(current.size.width, target.size.width)
        } else {
            1.0
        };
        let scale_y = if target.size.height > f32::EPSILON {
            finite_quotient(current.size.height, target.size.height)
        } else {
            1.0
        };
        Self {
            scale_x,
            scale_y,
            translate_x: finite_sum(current.origin.x, -finite_product(target.origin.x, scale_x)),
            translate_y: finite_sum(current.origin.y, -finite_product(target.origin.y, scale_y)),
        }
    }

    fn then(self, next: Self) -> Self {
        Self {
            scale_x: finite_product(self.scale_x, next.scale_x),
            scale_y: finite_product(self.scale_y, next.scale_y),
            translate_x: finite_sum(
                finite_product(self.translate_x, next.scale_x),
                next.translate_x,
            ),
            translate_y: finite_sum(
                finite_product(self.translate_y, next.scale_y),
                next.translate_y,
            ),
        }
    }

    fn apply(self, bounds: UiRect) -> UiRect {
        UiRect::new(
            UiPoint {
                x: finite_sum(
                    finite_product(bounds.origin.x, self.scale_x),
                    self.translate_x,
                ),
                y: finite_sum(
                    finite_product(bounds.origin.y, self.scale_y),
                    self.translate_y,
                ),
            },
            UiSize::new(
                finite_nonnegative(finite_product(bounds.size.width, self.scale_x)),
                finite_nonnegative(finite_product(bounds.size.height, self.scale_y)),
            ),
        )
    }
}

fn semantic_actions(
    role: meridian_ui_core::SemanticRole,
    has_command: bool,
    focusable: bool,
    state: meridian_ui_core::UiControlState,
) -> Vec<SemanticAction> {
    if state.disabled {
        return Vec::new();
    }
    let mut actions = Vec::new();
    if focusable {
        actions.push(SemanticAction::Focus);
    }
    if has_command {
        actions.push(SemanticAction::Activate);
    }
    match role {
        meridian_ui_core::SemanticRole::TextInput | meridian_ui_core::SemanticRole::SearchBox => {
            actions.push(SemanticAction::ReplaceSelectedText);
            actions.push(SemanticAction::SetValue);
        }
        _ => {}
    }
    actions
}

fn semantic_relationships(
    relationships: &UiSemanticRelationships,
    invalid: bool,
) -> SemanticRelationships {
    SemanticRelationships {
        labelled_by: relationships.labelled_by.clone(),
        described_by: relationships.described_by.clone(),
        controls: relationships.controls.clone(),
        details: relationships.details.clone(),
        flow_to: relationships.flow_to.clone(),
        // A retained document can declare the durable error target before
        // validation runs. Only an invalid immutable frame exposes it.
        error_message: invalid.then_some(relationships.error_message).flatten(),
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

fn emit_node_surface(
    node: UiNodeId,
    style: UiStyle,
    bounds: UiRect,
    display: &mut DisplayList,
) -> Result<(), DisplayListError> {
    let radius = style.corner_radius;
    match (style.background, style.border, radius > 0.0) {
        (Some(background), Some(border), true) => {
            let width = border.width.max(1);
            display.try_push(DisplayPrimitive::RoundedRect {
                node,
                bounds,
                radii: UiCornerRadii::uniform(radius),
                color: border.color,
            })?;
            display.try_push(DisplayPrimitive::RoundedRect {
                node,
                bounds: inset_bounds(bounds, f32::from(width)),
                radii: UiCornerRadii::uniform((radius - f32::from(width)).max(0.0)),
                color: background,
            })?;
        }
        (Some(background), _, true) => {
            display.try_push(DisplayPrimitive::RoundedRect {
                node,
                bounds,
                radii: UiCornerRadii::uniform(radius),
                color: background,
            })?;
        }
        (Some(background), _, false) => {
            display.try_push(DisplayPrimitive::Rect {
                node,
                bounds,
                color: background,
            })?;
            if let Some(border) = style.border {
                display.try_push(DisplayPrimitive::Border {
                    node,
                    bounds,
                    color: border.color,
                    width: border.width.max(1),
                })?;
            }
        }
        (None, Some(border), _) => {
            display.try_push(DisplayPrimitive::Border {
                node,
                bounds,
                color: border.color,
                width: border.width.max(1),
            })?;
        }
        (None, None, _) => {}
    }
    Ok(())
}

fn emit_state_treatments(
    node: UiNodeId,
    bounds: UiRect,
    style: UiStyle,
    state: UiVisualState,
    display: &mut DisplayList,
) -> Result<(), DisplayListError> {
    let indicator = style.border.map_or(style.foreground, |border| border.color);
    match state.selector() {
        UiStyleSelector::Selected => {
            display.try_push(DisplayPrimitive::Rect {
                node,
                bounds: UiRect::new(
                    bounds.origin,
                    UiSize::new(3.0_f32.min(bounds.size.width), bounds.size.height),
                ),
                color: indicator,
            })?;
        }
        UiStyleSelector::Invalid => {
            display.try_push(DisplayPrimitive::Border {
                node,
                bounds,
                color: indicator,
                width: 2,
            })?;
            let height = 3.0_f32.min(bounds.size.height);
            display.try_push(DisplayPrimitive::Rect {
                node,
                bounds: UiRect::new(
                    UiPoint {
                        x: bounds.origin.x,
                        y: bounds.origin.y + bounds.size.height - height,
                    },
                    UiSize::new(bounds.size.width, height),
                ),
                color: indicator,
            })?;
        }
        UiStyleSelector::Idle
        | UiStyleSelector::Hovered
        | UiStyleSelector::Focused
        | UiStyleSelector::Pressed
        | UiStyleSelector::Disabled => {}
    }
    if state.focused {
        display.try_push(DisplayPrimitive::FocusIndicator {
            node,
            bounds,
            color: indicator,
        })?;
    }
    Ok(())
}

fn resolve_constraints(
    node: UiNodeId,
    bounds: UiRect,
    constraints: UiConstraints,
) -> Result<UiRect, UiLayoutError> {
    let maximum = constraints
        .maximum
        .unwrap_or(UiSize::new(f32::MAX, f32::MAX));
    let mut width =
        finite_nonnegative(bounds.size.width).clamp(constraints.minimum.width, maximum.width);
    let mut height =
        finite_nonnegative(bounds.size.height).clamp(constraints.minimum.height, maximum.height);
    if let Some(aspect) = constraints.aspect_ratio {
        // Resolve the pair as one constraint set. The older independent clamp
        // could satisfy a maximum and then violate a minimum after applying
        // aspect ratio (or vice versa).
        let minimum_width = constraints
            .minimum
            .width
            .max(finite_product(constraints.minimum.height, aspect));
        let maximum_width = maximum.width.min(finite_product(maximum.height, aspect));
        if minimum_width > maximum_width {
            return Err(UiLayoutError::UnsatisfiableConstraints { node });
        }
        let fitted_width = width.min(finite_product(height, aspect));
        width = fitted_width.clamp(minimum_width, maximum_width);
        height = finite_quotient(width, aspect);
    }
    let horizontal_space = (bounds.size.width - width).max(0.0);
    let vertical_space = (bounds.size.height - height).max(0.0);
    let x = finite_sum(
        bounds.origin.x,
        match constraints.horizontal_alignment {
            UiAlignment::Start | UiAlignment::Stretch => 0.0,
            UiAlignment::Center => horizontal_space / 2.0,
            UiAlignment::End => horizontal_space,
        },
    );
    let y = finite_sum(
        bounds.origin.y,
        match constraints.vertical_alignment {
            UiAlignment::Start | UiAlignment::Stretch => 0.0,
            UiAlignment::Center => vertical_space / 2.0,
            UiAlignment::End => vertical_space,
        },
    );
    Ok(UiRect::new(UiPoint { x, y }, UiSize::new(width, height)))
}

/// Resolves a preferred child size before placing it within its parent's slot.
///
/// [`resolve_constraints`] owns constraint validation and the final size.  A
/// preferred child, however, must be aligned against the *parent slot*, not
/// against its own preferred rectangle.  Resolving first also means a maximum
/// or aspect adjustment cannot leave an end- or center-aligned child stranded
/// at the position calculated for its larger preferred size.
fn aligned_preferred_bounds(
    node: UiNodeId,
    slot: UiRect,
    preferred: UiSize,
    constraints: UiConstraints,
) -> Result<UiRect, UiLayoutError> {
    let preferred = UiRect::new(
        UiPoint::default(),
        UiSize::new(
            finite_nonnegative(preferred.width),
            finite_nonnegative(preferred.height),
        ),
    );
    let resolved = resolve_constraints(node, preferred, constraints)?;
    let horizontal_space = (finite_nonnegative(slot.size.width) - resolved.size.width).max(0.0);
    let vertical_space = (finite_nonnegative(slot.size.height) - resolved.size.height).max(0.0);
    let horizontal_offset = match constraints.horizontal_alignment {
        UiAlignment::Start | UiAlignment::Stretch => 0.0,
        UiAlignment::Center => horizontal_space / 2.0,
        UiAlignment::End => horizontal_space,
    };
    let vertical_offset = match constraints.vertical_alignment {
        UiAlignment::Start | UiAlignment::Stretch => 0.0,
        UiAlignment::Center => vertical_space / 2.0,
        UiAlignment::End => vertical_space,
    };
    Ok(UiRect::new(
        UiPoint {
            x: finite_sum(slot.origin.x, horizontal_offset),
            y: finite_sum(slot.origin.y, vertical_offset),
        },
        resolved.size,
    ))
}

struct UiEmission<'a> {
    layout: &'a BTreeMap<UiNodeId, UiRect>,
    presentation_layout: &'a BTreeMap<UiNodeId, UiRect>,
    scale_factor: f32,
    icon_tokens: meridian_ui_core::UiIconTokens,
    icon_tokens_fallback: bool,
    display: &'a mut DisplayList,
    semantic_nodes: &'a mut Vec<SemanticNode>,
    diagnostics: &'a mut Vec<UiDiagnostic>,
    next_scope: u64,
}

struct UiPreparedFrame {
    input_event_count: usize,
    contrast: UiContrast,
    motion: MotionPreference,
    layout: BTreeMap<UiNodeId, UiRect>,
    presentation_layout: BTreeMap<UiNodeId, UiRect>,
    visual_states: Vec<UiVisualStateSnapshot>,
    display_list: DisplayList,
    tree: SemanticTree,
    semantic_delta: SemanticDelta,
    effects: UiFrameEffects,
    text_validation: Vec<UiTextValidationSnapshot>,
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
    hovered: Option<UiNodeId>,
    collection_cursor: UiCollectionCursor,
    pointer_capture: Option<PointerCapture>,
    scroll_offsets: BTreeMap<UiNodeId, f32>,
    scroll_capture: Option<ScrollCapture>,
    drag: Option<ActiveDrag>,
    previous_semantics: Option<SemanticTree>,
    authoritative_layout: BTreeMap<UiNodeId, UiRect>,
    shared_element_layouts: BTreeMap<UiSharedElementId, UiRect>,
    presentation_opacity_targets: BTreeMap<UiNodeId, f32>,
    motion_system: UiMotionSystem,
    resolved_styles: BTreeMap<UiNodeId, UiStyle>,
    animated_color_targets: BTreeMap<UiNodeId, UiAnimatedColorTarget>,
}

/// Retained runtime state.  All mutation is applied between immutable outputs.
#[derive(Debug)]
pub struct UiRuntime {
    document: UiDocument,
    text: UiTextEngine,
    text_inputs: BTreeMap<UiNodeId, UiTextInputState>,
    focused: Option<UiNodeId>,
    hovered: Option<UiNodeId>,
    collection_cursor: UiCollectionCursor,
    pointer_capture: Option<PointerCapture>,
    scroll_offsets: BTreeMap<UiNodeId, f32>,
    scroll_capture: Option<ScrollCapture>,
    drag: Option<ActiveDrag>,
    previous_semantics: Option<SemanticTree>,
    authoritative_layout: BTreeMap<UiNodeId, UiRect>,
    shared_element_layouts: BTreeMap<UiSharedElementId, UiRect>,
    presentation_opacity_targets: BTreeMap<UiNodeId, f32>,
    revision: u64,
    last_document_delta: UiDocumentDelta,
    last_snapshot: Option<Arc<UiFrameSnapshot>>,
    motion_system: UiMotionSystem,
    resolved_styles: BTreeMap<UiNodeId, UiStyle>,
    animated_color_targets: BTreeMap<UiNodeId, UiAnimatedColorTarget>,
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
            hovered: None,
            collection_cursor: UiCollectionCursor::default(),
            pointer_capture: None,
            scroll_offsets,
            scroll_capture: None,
            drag: None,
            previous_semantics: None,
            authoritative_layout: BTreeMap::new(),
            shared_element_layouts: BTreeMap::new(),
            presentation_opacity_targets: BTreeMap::new(),
            revision: 0,
            last_document_delta: UiDocumentDelta::default(),
            last_snapshot: None,
            motion_system: UiMotionSystem::default(),
            resolved_styles: BTreeMap::new(),
            animated_color_targets: BTreeMap::new(),
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
    /// copying private text values into that source; non-password semantic
    /// values remain derived from the current accepted runtime text state.
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
        if self.hovered.is_some_and(|id| {
            document
                .node(id)
                .is_none_or(|node| node.semantics.state.disabled)
        }) {
            self.hovered = None;
        }
        for removed in &delta.removed {
            self.motion_system.remove_node(*removed);
            self.resolved_styles.remove(removed);
            self.animated_color_targets.remove(removed);
            self.authoritative_layout.remove(removed);
            self.presentation_opacity_targets.remove(removed);
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

    /// Moves focus to one enabled retained control after a typed application
    /// command such as Search or pane restoration.
    ///
    /// Returns `false` without changing focus when the identity is absent,
    /// disabled, or not keyboard-focusable.
    pub fn focus_retained_node(&mut self, node: UiNodeId) -> bool {
        if !self
            .document
            .node(node)
            .is_some_and(|candidate| candidate.focusable && !candidate.semantics.state.disabled)
        {
            return false;
        }
        self.set_focus(node);
        true
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
        let (contrast, motion) = effective_preferences(&input);
        let fallback = RejectedFrameContext {
            theme: input.theme.id,
            density: input.density,
            contrast,
            motion,
            scale_factor: sanitized_scale_factor(input.scale_factor),
            input_events: input.events.len(),
        };
        match self.try_reconcile(input) {
            Ok(snapshot) => snapshot,
            Err(UiFrameError::TooManyEvents { count, maximum }) => self.rejected_snapshot(
                RejectedFrameContext {
                    input_events: count,
                    ..fallback
                },
                UiDiagnostic::EventBatchRejected { count, maximum },
            ),
            Err(UiFrameError::TooManyInputBytes { bytes, maximum }) => self.rejected_snapshot(
                fallback,
                UiDiagnostic::InputByteLimitExceeded { bytes, maximum },
            ),
            Err(UiFrameError::TooManyEffects { count, maximum }) => self.rejected_snapshot(
                fallback,
                UiDiagnostic::FrameEffectLimitExceeded { count, maximum },
            ),
            Err(UiFrameError::TooManyEffectBytes { bytes, maximum }) => self.rejected_snapshot(
                fallback,
                UiDiagnostic::FrameEffectByteLimitExceeded { bytes, maximum },
            ),
            Err(UiFrameError::LayoutRejected(UiLayoutError::ConstraintCycle { chain })) => {
                self.rejected_snapshot(fallback, UiDiagnostic::LayoutConstraintCycle { chain })
            }
            Err(UiFrameError::LayoutRejected(UiLayoutError::UnsatisfiableConstraints { node })) => {
                self.rejected_snapshot(
                    fallback,
                    UiDiagnostic::LayoutConstraintsUnsatisfiable { node },
                )
            }
            Err(UiFrameError::InvalidDisplayList(error)) => {
                self.rejected_snapshot(fallback, UiDiagnostic::FrameRejected(error))
            }
            Err(UiFrameError::SemanticTreeRejected(error)) => {
                self.rejected_snapshot(fallback, UiDiagnostic::SemanticTreeRejected(error))
            }
        }
    }

    fn rejected_snapshot(
        &self,
        context: RejectedFrameContext,
        diagnostic: UiDiagnostic,
    ) -> UiFrameOutput {
        let mut fallback = self.last_snapshot.clone().unwrap_or_else(|| {
            Arc::new(UiFrameSnapshot {
                revision: self.revision,
                layout: Vec::new(),
                presentation_layout: Vec::new(),
                presentation_motion: Vec::new(),
                visual_states: Vec::new(),
                theme: context.theme,
                density: context.density,
                contrast: context.contrast,
                motion: context.motion,
                scale_factor: context.scale_factor,
                display_list: DisplayList::default(),
                semantic_tree: SemanticTree::default(),
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
                frame_diagnostics: UiFrameDiagnostics::default(),
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
            snapshot.frame_diagnostics =
                UiFrameDiagnostics::from_runtime(self, snapshot, true, context.input_events);
        }
        fallback
    }

    /// Transactionally computes one immutable frame.
    ///
    /// # Errors
    ///
    /// Invalid display output restores interaction state and preserves the last
    /// accepted snapshot.
    pub fn try_reconcile(
        &mut self,
        mut input: UiFrameInput,
    ) -> Result<UiFrameOutput, UiFrameError> {
        Self::validate_input_bound(&input)?;
        let input_event_count = input.events.len();
        let checkpoint = self.interaction_checkpoint();
        let (contrast, motion) = effective_preferences(&input);
        self.motion_system.advance(input.presentation_delta_ms);
        self.motion_system.apply_preference(motion);
        self.resolve_base_styles(&input.theme, contrast);
        let mut layout = self.resolve_layout_or_restore(input.viewport, &checkpoint)?;
        let mut effects = UiFrameEffects::default();
        let (motion_tokens, motion_tokens_fallback) = input.theme.resolved_motion_tokens();
        if motion_tokens_fallback {
            effects
                .diagnostics
                .push(UiDiagnostic::MotionThemeTokensFallback);
        }
        let (geometry_tokens, _) = input.theme.resolved_geometry_tokens();
        let line_step = geometry_tokens.spacing_base * 4.0;
        self.process_frame_events(
            std::mem::take(&mut input.events),
            input.viewport,
            &mut layout,
            &mut effects,
            &checkpoint,
            line_step,
        )?;
        self.synchronize_presentation_motion(
            &layout,
            motion,
            motion_tokens,
            &mut effects.diagnostics,
        );
        let presentation_layout = self.presentation_layout(&layout);
        let text_validation = self.text_validation_snapshots(&mut effects.diagnostics);
        let mut display_list = DisplayList::default();
        let mut semantic_nodes = Vec::new();
        let visual_states =
            self.resolve_visual_styles(&input.theme, contrast, motion, &mut effects.diagnostics);
        let (icon_tokens, icon_tokens_fallback) = input.theme.resolved_icon_tokens();
        let mut emission = UiEmission {
            layout: &layout,
            presentation_layout: &presentation_layout,
            scale_factor: sanitized_scale_factor(input.scale_factor),
            icon_tokens,
            icon_tokens_fallback,
            display: &mut display_list,
            semantic_nodes: &mut semantic_nodes,
            diagnostics: &mut effects.diagnostics,
            next_scope: 1,
        };
        let emission_result = self.emit_node(self.document.root(), None, 1.0, &mut emission);
        if let Err(error) = emission_result.and_then(|()| display_list.validate()) {
            self.restore_interaction(checkpoint);
            return Err(UiFrameError::InvalidDisplayList(error));
        }
        self.ensure_effect_bound(&effects, &checkpoint)?;
        let tree = SemanticTree {
            root: Some(self.document.root()),
            focus: self.focused,
            nodes: semantic_nodes,
        };
        let semantic_delta = self.semantic_delta_or_restore(&tree, checkpoint)?;
        Ok(self.commit_frame(
            &input,
            UiPreparedFrame {
                input_event_count,
                contrast,
                motion,
                layout,
                presentation_layout,
                visual_states,
                display_list,
                tree,
                semantic_delta,
                effects,
                text_validation,
            },
        ))
    }

    fn commit_frame(&mut self, input: &UiFrameInput, prepared: UiPreparedFrame) -> UiFrameOutput {
        let UiPreparedFrame {
            input_event_count,
            contrast,
            motion,
            layout,
            presentation_layout,
            visual_states,
            display_list,
            tree,
            semantic_delta,
            effects,
            text_validation,
        } = prepared;
        self.previous_semantics = Some(tree.clone());
        self.authoritative_layout.clone_from(&layout);
        self.shared_element_layouts = self.shared_element_targets(&layout);
        self.presentation_opacity_targets = self.retained_presentation_opacity_targets();
        self.revision = self.revision.saturating_add(1);
        let mut snapshot = UiFrameSnapshot {
            revision: self.revision,
            layout: layout
                .iter()
                .map(|(node, bounds)| UiLayoutSnapshot {
                    node: *node,
                    bounds: *bounds,
                })
                .collect(),
            presentation_layout: presentation_layout
                .iter()
                .map(|(node, bounds)| UiLayoutSnapshot {
                    node: *node,
                    bounds: *bounds,
                })
                .collect(),
            presentation_motion: self.motion_system.snapshots(),
            visual_states,
            theme: input.theme.id,
            density: input.density,
            contrast,
            motion,
            scale_factor: sanitized_scale_factor(input.scale_factor),
            display_list,
            semantic_tree: tree,
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
            frame_diagnostics: UiFrameDiagnostics::default(),
            focused: self.focused,
            preedit: self.focused_preedit(),
        };
        snapshot.frame_diagnostics =
            UiFrameDiagnostics::from_runtime(self, &snapshot, false, input_event_count);
        let snapshot = Arc::new(snapshot);
        self.last_snapshot = Some(Arc::clone(&snapshot));
        snapshot
    }

    fn resolve_layout_or_restore(
        &mut self,
        viewport: UiSize,
        checkpoint: &UiInteractionCheckpoint,
    ) -> Result<BTreeMap<UiNodeId, UiRect>, UiFrameError> {
        self.resolved_layout(viewport).map_err(|error| {
            self.restore_interaction(checkpoint.clone());
            UiFrameError::LayoutRejected(error)
        })
    }

    fn process_frame_events(
        &mut self,
        events: Vec<UiEvent>,
        viewport: UiSize,
        layout: &mut BTreeMap<UiNodeId, UiRect>,
        effects: &mut UiFrameEffects,
        checkpoint: &UiInteractionCheckpoint,
        line_step: f32,
    ) -> Result<(), UiFrameError> {
        for event in events {
            let layout_changed = self.process_event(event, layout, effects, line_step);
            self.ensure_effect_bound(effects, checkpoint)?;
            if layout_changed {
                *layout = self.resolve_layout_or_restore(viewport, checkpoint)?;
            }
        }
        Ok(())
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
            hovered: self.hovered,
            collection_cursor: self.collection_cursor,
            pointer_capture: self.pointer_capture,
            scroll_offsets: self.scroll_offsets.clone(),
            scroll_capture: self.scroll_capture,
            drag: self.drag,
            previous_semantics: self.previous_semantics.clone(),
            authoritative_layout: self.authoritative_layout.clone(),
            shared_element_layouts: self.shared_element_layouts.clone(),
            presentation_opacity_targets: self.presentation_opacity_targets.clone(),
            motion_system: self.motion_system.clone(),
            resolved_styles: self.resolved_styles.clone(),
            animated_color_targets: self.animated_color_targets.clone(),
        }
    }

    fn restore_interaction(&mut self, checkpoint: UiInteractionCheckpoint) {
        self.text_inputs = checkpoint.text_inputs;
        self.focused = checkpoint.focused;
        self.hovered = checkpoint.hovered;
        self.collection_cursor = checkpoint.collection_cursor;
        self.pointer_capture = checkpoint.pointer_capture;
        self.scroll_offsets = checkpoint.scroll_offsets;
        self.scroll_capture = checkpoint.scroll_capture;
        self.drag = checkpoint.drag;
        self.previous_semantics = checkpoint.previous_semantics;
        self.authoritative_layout = checkpoint.authoritative_layout;
        self.shared_element_layouts = checkpoint.shared_element_layouts;
        self.presentation_opacity_targets = checkpoint.presentation_opacity_targets;
        self.motion_system = checkpoint.motion_system;
        self.resolved_styles = checkpoint.resolved_styles;
        self.animated_color_targets = checkpoint.animated_color_targets;
    }

    fn semantic_delta_or_restore(
        &mut self,
        tree: &SemanticTree,
        checkpoint: UiInteractionCheckpoint,
    ) -> Result<SemanticDelta, UiFrameError> {
        tree.delta_from(self.previous_semantics.as_ref())
            .map_err(|error| {
                self.restore_interaction(checkpoint);
                UiFrameError::SemanticTreeRejected(error)
            })
    }

    fn resolve_base_styles(&mut self, theme: &UiTheme, contrast: UiContrast) {
        self.resolved_styles = self
            .document
            .nodes()
            .map(|node| {
                (
                    node.id,
                    (*theme)
                        .resolve_style(
                            node.style_reference,
                            node.style,
                            UiVisualState::default(),
                            contrast,
                        )
                        .style,
                )
            })
            .collect();
    }

    fn resolve_visual_styles(
        &mut self,
        theme: &UiTheme,
        contrast: UiContrast,
        motion: MotionPreference,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) -> Vec<UiVisualStateSnapshot> {
        let nodes: Vec<_> = self.document.nodes().cloned().collect();
        let (motion_tokens, motion_tokens_fallback) = (*theme).resolved_motion_tokens();
        if motion_tokens_fallback && !diagnostics.contains(&UiDiagnostic::MotionThemeTokensFallback)
        {
            diagnostics.push(UiDiagnostic::MotionThemeTokensFallback);
        }
        let mut retained_targets = BTreeMap::new();
        let mut resolved_styles = BTreeMap::new();
        let mut visual_states = Vec::with_capacity(nodes.len());
        for node in nodes {
            let state = self.visual_state(&node);
            let resolution =
                (*theme).resolve_style(node.style_reference, node.style, state, contrast);
            if resolution.used_token_fallback {
                diagnostics.push(UiDiagnostic::StyleTokenFallback { node: node.id });
            }
            let mut style = resolution.style;
            let target = animated_color_target(style);
            if let Some(previous) = self.animated_color_targets.get(&node.id).copied() {
                if previous != target {
                    if previous.slot == target.slot {
                        if self
                            .motion_system
                            .retarget_color(
                                node.id,
                                previous.color,
                                target.color,
                                motion_tokens.state_transition_min_ms,
                                motion,
                                motion_tokens,
                            )
                            .is_err()
                        {
                            self.motion_system
                                .remove_channel(node.id, UiMotionChannel::Color);
                            diagnostics.push(UiDiagnostic::MotionTrackRejected { node: node.id });
                        }
                    } else {
                        self.motion_system
                            .remove_channel(node.id, UiMotionChannel::Color);
                    }
                }
            } else {
                self.motion_system
                    .remove_channel(node.id, UiMotionChannel::Color);
            }
            if let Some(UiMotionSnapshot {
                current: UiPresentationValue::Color(color),
                ..
            }) = self.motion_system.snapshot(node.id, UiMotionChannel::Color)
            {
                apply_animated_color(&mut style, target.slot, color);
            }
            retained_targets.insert(node.id, target);
            resolved_styles.insert(node.id, style);
            visual_states.push(UiVisualStateSnapshot {
                node: node.id,
                state,
                selector: resolution.selector,
            });
        }
        self.animated_color_targets = retained_targets;
        self.resolved_styles = resolved_styles;
        visual_states
    }

    fn synchronize_presentation_motion(
        &mut self,
        layout: &BTreeMap<UiNodeId, UiRect>,
        preference: MotionPreference,
        tokens: meridian_ui_core::UiMotionTokens,
        diagnostics: &mut Vec<UiDiagnostic>,
    ) {
        let nodes: Vec<_> = self
            .document
            .nodes()
            .map(|node| (node.id, node.presentation))
            .collect();
        let previous_layout = self.authoritative_layout.clone();
        let previous_shared = self.shared_element_layouts.clone();
        let previous_opacity = self.presentation_opacity_targets.clone();
        for (node, presentation) in nodes {
            let Some(target) = layout.get(&node).copied() else {
                self.motion_system.remove_node(node);
                continue;
            };
            let initial = match presentation.spatial_motion {
                Some(UiSpatialMotionKind::PhysicalPanel) => previous_layout.get(&node).copied(),
                Some(UiSpatialMotionKind::SharedElement) => presentation
                    .shared_element
                    .and_then(|shared_element| previous_shared.get(&shared_element).copied())
                    .or_else(|| previous_layout.get(&node).copied()),
                None => None,
            };
            if let (Some(kind), Some(initial)) = (presentation.spatial_motion, initial) {
                let current_target = self
                    .motion_system
                    .snapshot(node, UiMotionChannel::Spatial)
                    .and_then(|snapshot| match snapshot.target {
                        UiPresentationValue::Rect(bounds) => Some((bounds, snapshot.spatial_kind)),
                        UiPresentationValue::Opacity(_) | UiPresentationValue::Color(_) => None,
                    });
                if current_target.is_none_or(|(current, tracked_kind)| {
                    current != target || tracked_kind != Some(kind)
                }) && self
                    .motion_system
                    .retarget_spatial(node, initial, target, kind, preference, tokens)
                    .is_err()
                {
                    self.motion_system
                        .remove_channel(node, UiMotionChannel::Spatial);
                    diagnostics.push(UiDiagnostic::MotionTrackRejected { node });
                }
            } else {
                self.motion_system
                    .remove_channel(node, UiMotionChannel::Spatial);
            }

            if let Some(initial) = previous_opacity.get(&node).copied() {
                let current_target = self
                    .motion_system
                    .snapshot(node, UiMotionChannel::Opacity)
                    .and_then(|snapshot| match snapshot.target {
                        UiPresentationValue::Opacity(opacity) => Some(opacity),
                        UiPresentationValue::Rect(_) | UiPresentationValue::Color(_) => None,
                    });
                if current_target
                    .is_none_or(|current| current.to_bits() != presentation.opacity.to_bits())
                    && self
                        .motion_system
                        .retarget_opacity(
                            node,
                            initial,
                            presentation.opacity,
                            tokens.state_transition_min_ms,
                            preference,
                            tokens,
                        )
                        .is_err()
                {
                    self.motion_system
                        .remove_channel(node, UiMotionChannel::Opacity);
                    diagnostics.push(UiDiagnostic::MotionTrackRejected { node });
                }
            } else {
                self.motion_system
                    .remove_channel(node, UiMotionChannel::Opacity);
            }
        }
    }

    fn shared_element_targets(
        &self,
        layout: &BTreeMap<UiNodeId, UiRect>,
    ) -> BTreeMap<UiSharedElementId, UiRect> {
        self.document
            .nodes()
            .filter_map(|node| {
                (node.presentation.spatial_motion == Some(UiSpatialMotionKind::SharedElement))
                    .then_some(())
                    .and(node.presentation.shared_element)
                    .zip(layout.get(&node.id).copied())
            })
            .collect()
    }

    fn retained_presentation_opacity_targets(&self) -> BTreeMap<UiNodeId, f32> {
        self.document
            .nodes()
            .map(|node| (node.id, node.presentation.opacity))
            .collect()
    }

    fn presentation_layout(
        &self,
        layout: &BTreeMap<UiNodeId, UiRect>,
    ) -> BTreeMap<UiNodeId, UiRect> {
        let mut presentation = BTreeMap::new();
        self.collect_presentation_layout(
            self.document.root(),
            layout,
            UiPresentationTransform::identity(),
            &mut presentation,
        );
        presentation
    }

    fn collect_presentation_layout(
        &self,
        node: UiNodeId,
        layout: &BTreeMap<UiNodeId, UiRect>,
        inherited: UiPresentationTransform,
        presentation: &mut BTreeMap<UiNodeId, UiRect>,
    ) {
        let Some(bounds) = layout.get(&node).copied() else {
            return;
        };
        let transform = self
            .motion_system
            .snapshot(node, UiMotionChannel::Spatial)
            .and_then(|snapshot| match (snapshot.target, snapshot.current) {
                (UiPresentationValue::Rect(target), UiPresentationValue::Rect(current)) => {
                    Some(inherited.then(UiPresentationTransform::from_rects(target, current)))
                }
                _ => None,
            })
            .unwrap_or(inherited);
        presentation.insert(node, transform.apply(bounds));
        if let Some(document_node) = self.document.node(node) {
            for child in &document_node.children {
                self.collect_presentation_layout(*child, layout, transform, presentation);
            }
        }
    }

    fn presentation_opacity(&self, node: UiNodeId) -> f32 {
        self.motion_system
            .snapshot(node, UiMotionChannel::Opacity)
            .and_then(|snapshot| match snapshot.current {
                UiPresentationValue::Opacity(opacity) => Some(opacity),
                UiPresentationValue::Rect(_) | UiPresentationValue::Color(_) => None,
            })
            .or_else(|| {
                self.document
                    .node(node)
                    .map(|document_node| document_node.presentation.opacity)
            })
            .unwrap_or(1.0)
    }

    fn visual_state(&self, node: &UiNode) -> UiVisualState {
        let invalid = node.semantics.state.invalid
            || node.text_validation.is_some_and(|rule| {
                self.text_inputs
                    .get(&node.id)
                    .is_none_or(|state| !state.is_valid(rule))
            });
        let disabled = node.semantics.state.disabled;
        UiVisualState {
            hovered: !disabled && self.hovered == Some(node.id),
            pressed: !disabled
                && self
                    .pointer_capture
                    .is_some_and(|capture| capture.target == node.id),
            focused: !disabled && self.focused == Some(node.id),
            disabled,
            selected: node.semantics.state.selected,
            invalid,
        }
    }

    fn resolved_style(&self, node: UiNodeId) -> UiStyle {
        self.resolved_styles
            .get(&node)
            .copied()
            .unwrap_or_else(UiStyle::transparent)
    }

    fn resolved_layout(
        &mut self,
        viewport: UiSize,
    ) -> Result<BTreeMap<UiNodeId, UiRect>, UiLayoutError> {
        let root_bounds = UiRect::new(UiPoint::default(), viewport.sanitized());
        let mut layout = UiLayoutResolver::resolve(self, root_bounds)?;
        if self.clamp_scroll_offsets(&layout) {
            layout = UiLayoutResolver::resolve(self, root_bounds)?;
        }
        Ok(layout)
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
            UiEvent::AssistiveSetValue {
                target,
                text,
                replace_selection,
            } => self.assistive_set_value(target, &text, replace_selection, effects),
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

    fn assistive_set_value(
        &mut self,
        target: UiNodeId,
        text: &str,
        replace_selection: bool,
        effects: &mut UiFrameEffects,
    ) {
        let editable = self.document.node(target).is_some_and(|node| {
            node.focusable
                && !node.semantics.state.disabled
                && matches!(
                    node.semantics.role,
                    meridian_ui_core::SemanticRole::TextInput
                        | meridian_ui_core::SemanticRole::SearchBox
                )
        });
        if !editable {
            effects
                .diagnostics
                .push(UiDiagnostic::AssistiveEditDenied { node: target });
            return;
        }
        self.dispatch(target, &mut effects.routes);
        self.set_focus(target);
        let Some(editor) = self.text_inputs.get_mut(&target) else {
            effects
                .diagnostics
                .push(UiDiagnostic::AssistiveEditDenied { node: target });
            return;
        };
        if !replace_selection {
            editor.select_all();
        }
        if !editor.commit(text) {
            effects
                .diagnostics
                .push(UiDiagnostic::TextInputLimitExceeded {
                    node: target,
                    maximum: MAX_TEXT_BYTES,
                });
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
            self.hovered = None;
            effects.diagnostics.push(UiDiagnostic::InvalidPointerEvent);
            return false;
        }
        let hover_capable = matches!(
            event.kind,
            UiInputDeviceKind::Mouse | UiInputDeviceKind::Trackpad | UiInputDeviceKind::Pen
        );
        self.hovered = if event.phase == UiPointerPhase::Cancel || !hover_capable {
            None
        } else {
            self.hit_test(event.position, layout)
        };
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
        let viewport = inset_bounds(bounds, self.resolved_style(id).padding);
        let viewport_extent = if axis == UiAxis::Vertical {
            viewport.size.height
        } else {
            viewport.size.width
        };
        let vertical = axis == UiAxis::Vertical;
        let offset = self.scroll_offsets.get(&id).copied().unwrap_or(0.0);
        let content_origin = finite_sum(
            axis_origin(viewport.origin, vertical),
            -finite_nonnegative(offset),
        );
        let content_end = node.children.iter().fold(content_origin, |end, child| {
            layout
                .get(child)
                .map_or(end, |bounds| end.max(axis_end(*bounds, vertical)))
        });
        let content_extent = (content_end - content_origin).max(0.0);
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
        inherited_opacity: f32,
        emission: &mut UiEmission<'_>,
    ) -> Result<(), DisplayListError> {
        let Some(node) = self.document.node(id) else {
            return Ok(());
        };
        let Some(authoritative_bounds) = emission.layout.get(&id).copied() else {
            return Ok(());
        };
        let bounds = emission
            .presentation_layout
            .get(&id)
            .copied()
            .unwrap_or(authoritative_bounds);
        let opacity =
            finite_product(inherited_opacity, self.presentation_opacity(id)).clamp(0.0, 1.0);
        let mut style = self.resolved_style(id);
        apply_presentation_opacity(&mut style, opacity);
        let clip = node.constraints.clip;
        let mut semantics = node.semantics.clone();
        if let Some(options) = node.text_input {
            // Text state is runtime-owned. Non-password fields publish their
            // current accepted value; password fields never publish any value,
            // even if a malformed source tried to declare one.
            semantics.value = if options.password {
                None
            } else {
                self.text_inputs
                    .get(&id)
                    .and_then(UiTextInputState::value)
                    .map(str::to_owned)
            };
        }
        if let Some(rule) = node.text_validation {
            semantics.state.invalid = self
                .text_inputs
                .get(&id)
                .is_none_or(|state| !state.is_valid(rule));
        }
        let children = node.children.clone();
        let focusable = node.focusable;
        let clip_id = if clip {
            let id = UiClipId(emission.next_scope);
            emission.next_scope = emission.next_scope.saturating_add(1);
            emission.display.try_push(DisplayPrimitive::PushClip {
                id,
                bounds,
                radii: UiCornerRadii::uniform(style.corner_radius),
            })?;
            Some(id)
        } else {
            None
        };
        self.emit_node_visuals(id, bounds, style, emission)?;
        emission.semantic_nodes.push(SemanticNode {
            id,
            parent,
            role: semantics.role,
            name: semantics.name,
            description: semantics.description,
            actions: semantic_actions(
                semantics.role,
                semantics.action.is_some(),
                focusable,
                semantics.state,
            ),
            command: semantics.action,
            value: semantics.value,
            state: semantics.state,
            relationships: semantic_relationships(
                &semantics.relationships,
                semantics.state.invalid,
            ),
            live: match semantics.role {
                meridian_ui_core::SemanticRole::LiveRegion
                | meridian_ui_core::SemanticRole::Status => SemanticLive::Polite,
                _ => SemanticLive::Off,
            },
            collection_item: semantics
                .collection_item
                .map(|item| SemanticCollectionItem {
                    position: item.position,
                    set_size: item.set_size,
                }),
            bounds: authoritative_bounds,
            focused: self.focused == Some(id),
        });
        for child in children {
            self.emit_node(child, Some(id), opacity, emission)?;
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
        style: UiStyle,
        emission: &mut UiEmission<'_>,
    ) -> Result<(), DisplayListError> {
        let Some(node) = self.document.node(id) else {
            return Ok(());
        };
        let visual_state = self.visual_state(node);
        emit_node_surface(id, style, bounds, emission.display)?;
        let rendered_text = self
            .text_inputs
            .get(&id)
            .map(UiTextInputState::rendered_text)
            .or_else(|| node.text.clone());
        let content_width = (bounds.size.width - style.padding * 2.0).max(1.0);
        let content_height = (bounds.size.height - style.padding * 2.0).max(1.0);
        let mut text_origin_x = bounds.origin.x + style.padding;
        let mut text_width = content_width;
        emit_node_icon(
            IconVisualParams {
                id,
                node,
                bounds,
                style,
                has_text: rendered_text.is_some(),
            },
            emission,
            &mut text_origin_x,
            &mut text_width,
        )?;
        if let Some(text) = rendered_text {
            emit_node_text(
                &mut self.text,
                TextVisualParams {
                    id,
                    node,
                    style,
                    text,
                    origin_x: text_origin_x,
                    width: text_width,
                    height: content_height,
                    bounds,
                },
                emission,
            )?;
        }
        emit_state_treatments(id, bounds, style, visual_state, emission.display)?;
        Ok(())
    }
}

/// Per-frame layout resolver. It is deliberately separate from retained
/// interaction state so a rejected geometry pass cannot partially commit a
/// frame. The document validator prevents ordinary cycles; `visiting` remains
/// a defensive guard for corrupted or future incremental document sources.
struct UiLayoutResolver<'a> {
    runtime: &'a UiRuntime,
    layout: BTreeMap<UiNodeId, UiRect>,
    visiting: BTreeSet<UiNodeId>,
    chain: Vec<UiNodeId>,
}

impl<'a> UiLayoutResolver<'a> {
    fn resolve(
        runtime: &'a UiRuntime,
        root_bounds: UiRect,
    ) -> Result<BTreeMap<UiNodeId, UiRect>, UiLayoutError> {
        let mut resolver = Self {
            runtime,
            layout: BTreeMap::new(),
            visiting: BTreeSet::new(),
            chain: Vec::new(),
        };
        resolver.layout_node(runtime.document.root(), root_bounds)?;
        Ok(resolver.layout)
    }

    fn layout_node(&mut self, id: UiNodeId, bounds: UiRect) -> Result<UiRect, UiLayoutError> {
        if !self.visiting.insert(id) {
            let first = self.chain.iter().position(|node| *node == id).unwrap_or(0);
            let mut chain = self.chain[first..].to_vec();
            chain.push(id);
            return Err(UiLayoutError::ConstraintCycle { chain });
        }
        self.chain.push(id);
        let result = self.layout_node_inner(id, bounds);
        self.chain.pop();
        self.visiting.remove(&id);
        result
    }

    fn layout_node_inner(&mut self, id: UiNodeId, bounds: UiRect) -> Result<UiRect, UiLayoutError> {
        let Some(node) = self.runtime.document.node(id).cloned() else {
            // A validated document never reaches this branch. Keeping this
            // bounded fallback avoids an impossible child reference causing a
            // partially emitted geometry map.
            return Ok(bounds);
        };
        let bounds = resolve_constraints(id, bounds, node.constraints)?;
        self.layout.insert(id, bounds);
        if node.children.is_empty() {
            return Ok(bounds);
        }

        let content_bounds = inset_bounds(bounds, self.runtime.resolved_style(id).padding);
        match node.layout {
            UiLayout::Overlay => {
                for child in &node.children {
                    let child_bounds = self.preferred_bounds(*child, content_bounds)?;
                    self.layout_node(*child, child_bounds)?;
                }
            }
            UiLayout::Grid { columns, gap } => {
                self.layout_grid(&node.children, content_bounds, columns, gap)?;
            }
            UiLayout::VerticalStack { gap } => {
                self.layout_stack(&node.children, content_bounds, gap, true)?;
            }
            UiLayout::HorizontalStack { gap } => {
                self.layout_stack(&node.children, content_bounds, gap, false)?;
            }
            UiLayout::Flex { axis, gap } => {
                self.layout_stack(
                    &node.children,
                    content_bounds,
                    gap,
                    axis == UiAxis::Vertical,
                )?;
            }
            UiLayout::Absolute => {
                for child in &node.children {
                    self.layout_node(*child, self.absolute_bounds(*child, content_bounds)?)?;
                }
            }
            UiLayout::Scroll { axis, offset } => {
                let offset = self
                    .runtime
                    .scroll_offsets
                    .get(&id)
                    .copied()
                    .unwrap_or(offset);
                self.layout_scroll(&node.children, content_bounds, axis, offset)?;
            }
        }
        Ok(bounds)
    }

    fn preferred_bounds(&self, child: UiNodeId, slot: UiRect) -> Result<UiRect, UiLayoutError> {
        let Some(node) = self.runtime.document.node(child) else {
            return Ok(slot);
        };
        let size = UiSize::new(
            node.layout_hints
                .preferred_width
                .map_or(slot.size.width, finite_nonnegative),
            node.layout_hints
                .preferred_height
                .map_or(slot.size.height, finite_nonnegative),
        );
        aligned_preferred_bounds(child, slot, size, node.constraints)
    }

    fn preferred_cross_axis_bounds(
        &self,
        child: UiNodeId,
        slot: UiRect,
        vertical: bool,
    ) -> Result<UiRect, UiLayoutError> {
        let Some(node) = self.runtime.document.node(child) else {
            return Ok(slot);
        };
        let mut size = slot.size;
        if vertical {
            size.width = node
                .layout_hints
                .preferred_width
                .map_or(size.width, finite_nonnegative);
        } else {
            size.height = node
                .layout_hints
                .preferred_height
                .map_or(size.height, finite_nonnegative);
        }
        aligned_preferred_bounds(child, slot, size, node.constraints)
    }

    fn absolute_bounds(
        &self,
        child: UiNodeId,
        content_bounds: UiRect,
    ) -> Result<UiRect, UiLayoutError> {
        let preferred = self.preferred_bounds(child, content_bounds)?;
        let Some(position) = self
            .runtime
            .document
            .node(child)
            .and_then(|node| node.absolute_position)
        else {
            return Ok(preferred);
        };
        Ok(UiRect::new(
            UiPoint {
                x: finite_sum(content_bounds.origin.x, position.left),
                y: finite_sum(content_bounds.origin.y, position.top),
            },
            UiSize::new(
                position.width.unwrap_or(preferred.size.width),
                position.height.unwrap_or(preferred.size.height),
            ),
        ))
    }

    fn constrained_axis_extent(
        &self,
        child: UiNodeId,
        main_extent: f32,
        cross_extent: f32,
        vertical: bool,
    ) -> Result<f32, UiLayoutError> {
        let Some(node) = self.runtime.document.node(child) else {
            return Ok(0.0);
        };
        let proposal = if vertical {
            UiSize::new(cross_extent, main_extent)
        } else {
            UiSize::new(main_extent, cross_extent)
        };
        let resolved = resolve_constraints(
            child,
            UiRect::new(UiPoint::default(), proposal),
            node.constraints,
        )?;
        Ok(if vertical {
            resolved.size.height
        } else {
            resolved.size.width
        })
    }

    fn layout_stack(
        &mut self,
        children: &[UiNodeId],
        bounds: UiRect,
        gap: f32,
        vertical: bool,
    ) -> Result<(), UiLayoutError> {
        let gap = finite_nonnegative(gap);
        let total_gap = finite_product(gap, bounded_count_as_f32(children.len().saturating_sub(1)));
        let main_available = (axis_extent(bounds.size, vertical) - total_gap).max(0.0);
        let cross_extent = axis_extent(bounds.size, !vertical);
        let mut extents = Vec::with_capacity(children.len());
        let mut minimums = Vec::with_capacity(children.len());
        let mut grow = Vec::with_capacity(children.len());
        let mut has_preferred_extent = false;

        for child in children {
            let Some(node) = self.runtime.document.node(*child) else {
                continue;
            };
            let minimum = self.constrained_axis_extent(*child, 0.0, cross_extent, vertical)?;
            let preference = if vertical {
                node.layout_hints.preferred_height
            } else {
                node.layout_hints.preferred_width
            };
            has_preferred_extent |= preference.is_some();
            let preferred = preference
                .map(finite_nonnegative)
                .map(|extent| self.constrained_axis_extent(*child, extent, cross_extent, vertical))
                .transpose()?
                .unwrap_or(minimum);
            minimums.push(minimum);
            extents.push(preferred.max(minimum));
            grow.push(finite_nonnegative(node.layout_hints.grow));
        }

        fit_extents_to_available(&mut extents, &minimums, main_available);
        if grow.iter().all(|weight| *weight <= 0.0) && !has_preferred_extent {
            grow.fill(1.0);
        }
        distribute_available_extent(&mut extents, &grow, main_available);

        let mut cursor = axis_origin(bounds.origin, vertical);
        for (index, child) in children.iter().enumerate() {
            let extent = extents.get(index).copied().unwrap_or_default();
            let slot = rect_from_axes(bounds, cursor, extent, vertical);
            let slot = self.preferred_cross_axis_bounds(*child, slot, vertical)?;
            let actual = self.layout_node(*child, slot)?;
            let requested_end = finite_sum(cursor, extent);
            let actual_end = axis_end(actual, vertical);
            cursor = finite_sum(requested_end.max(actual_end), gap);
        }
        Ok(())
    }

    fn layout_grid(
        &mut self,
        children: &[UiNodeId],
        bounds: UiRect,
        columns: u8,
        gap: f32,
    ) -> Result<(), UiLayoutError> {
        let columns = usize::from(columns.max(1)).min(children.len()).max(1);
        let rows = children.len().div_ceil(columns);
        let gap = finite_nonnegative(gap);
        let available_width = (bounds.size.width
            - finite_product(gap, bounded_count_as_f32(columns.saturating_sub(1))))
        .max(0.0);
        let available_height = (bounds.size.height
            - finite_product(gap, bounded_count_as_f32(rows.saturating_sub(1))))
        .max(0.0);
        let mut column_minimums = vec![0.0_f32; columns];
        let mut column_extents = vec![0.0_f32; columns];
        let mut row_minimums = vec![0.0_f32; rows];
        let mut row_extents = vec![0.0_f32; rows];

        for (index, child) in children.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            let Some(node) = self.runtime.document.node(*child) else {
                continue;
            };
            let minimum_width =
                self.constrained_axis_extent(*child, 0.0, bounds.size.height, false)?;
            let preferred_width = node
                .layout_hints
                .preferred_width
                .map(finite_nonnegative)
                .map(|extent| {
                    self.constrained_axis_extent(*child, extent, bounds.size.height, false)
                })
                .transpose()?
                .unwrap_or(minimum_width);
            let minimum_height =
                self.constrained_axis_extent(*child, 0.0, bounds.size.width, true)?;
            let preferred_height = node
                .layout_hints
                .preferred_height
                .map(finite_nonnegative)
                .map(|extent| self.constrained_axis_extent(*child, extent, bounds.size.width, true))
                .transpose()?
                .unwrap_or(minimum_height);
            column_minimums[column] = column_minimums[column].max(minimum_width);
            column_extents[column] = column_extents[column].max(preferred_width);
            row_minimums[row] = row_minimums[row].max(minimum_height);
            row_extents[row] = row_extents[row].max(preferred_height);
        }

        fit_extents_to_available(&mut column_extents, &column_minimums, available_width);
        fit_extents_to_available(&mut row_extents, &row_minimums, available_height);
        distribute_available_extent(&mut column_extents, &vec![1.0; columns], available_width);
        distribute_available_extent(&mut row_extents, &vec![1.0; rows], available_height);

        let mut column_origins = Vec::with_capacity(columns);
        let mut cursor_x = bounds.origin.x;
        for width in &column_extents {
            column_origins.push(cursor_x);
            cursor_x = finite_sum(finite_sum(cursor_x, *width), gap);
        }
        let mut row_origins = Vec::with_capacity(rows);
        let mut cursor_y = bounds.origin.y;
        for height in &row_extents {
            row_origins.push(cursor_y);
            cursor_y = finite_sum(finite_sum(cursor_y, *height), gap);
        }
        for (index, child) in children.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            self.layout_node(
                *child,
                UiRect::new(
                    UiPoint {
                        x: column_origins[column],
                        y: row_origins[row],
                    },
                    UiSize::new(column_extents[column], row_extents[row]),
                ),
            )?;
        }
        Ok(())
    }

    fn layout_scroll(
        &mut self,
        children: &[UiNodeId],
        bounds: UiRect,
        axis: UiAxis,
        offset: f32,
    ) -> Result<(), UiLayoutError> {
        let vertical = axis == UiAxis::Vertical;
        let mut cursor = finite_sum(
            axis_origin(bounds.origin, vertical),
            -finite_nonnegative(offset),
        );
        for child in children {
            let slot = rect_from_axes(bounds, cursor, axis_extent(bounds.size, vertical), vertical);
            let actual = self.layout_node(*child, self.preferred_bounds(*child, slot)?)?;
            cursor = axis_end(actual, vertical);
        }
        Ok(())
    }
}

fn axis_origin(origin: UiPoint, vertical: bool) -> f32 {
    if vertical {
        origin.y
    } else {
        origin.x
    }
}

fn axis_extent(size: UiSize, vertical: bool) -> f32 {
    if vertical {
        size.height
    } else {
        size.width
    }
}

fn axis_end(bounds: UiRect, vertical: bool) -> f32 {
    finite_sum(
        axis_origin(bounds.origin, vertical),
        axis_extent(bounds.size, vertical),
    )
}

fn rect_from_axes(bounds: UiRect, main_origin: f32, main_extent: f32, vertical: bool) -> UiRect {
    if vertical {
        UiRect::new(
            UiPoint {
                x: bounds.origin.x,
                y: main_origin,
            },
            UiSize::new(bounds.size.width, main_extent),
        )
    } else {
        UiRect::new(
            UiPoint {
                x: main_origin,
                y: bounds.origin.y,
            },
            UiSize::new(main_extent, bounds.size.height),
        )
    }
}

fn fit_extents_to_available(extents: &mut [f32], minimums: &[f32], available: f32) {
    let total = extents.iter().copied().fold(0.0, finite_sum);
    if total <= available {
        return;
    }
    let shrinkable = extents
        .iter()
        .zip(minimums)
        .map(|(extent, minimum)| (*extent - *minimum).max(0.0))
        .fold(0.0, finite_sum);
    if shrinkable <= 0.0 {
        return;
    }
    let required = (total - available).min(shrinkable);
    for (extent, minimum) in extents.iter_mut().zip(minimums) {
        let available_shrink = (*extent - *minimum).max(0.0);
        let reduction = required * (available_shrink / shrinkable);
        *extent = (*extent - reduction).max(*minimum);
    }
}

fn distribute_available_extent(extents: &mut [f32], weights: &[f32], available: f32) {
    let allocated = extents.iter().copied().fold(0.0, finite_sum);
    let remaining = (available - allocated).max(0.0);
    let weight_total = weights
        .iter()
        .copied()
        .map(finite_nonnegative)
        .fold(0.0, finite_sum);
    if remaining <= 0.0 || weight_total <= 0.0 {
        return;
    }
    for (extent, weight) in extents.iter_mut().zip(weights) {
        *extent = finite_sum(
            *extent,
            remaining * finite_nonnegative(*weight) / weight_total,
        );
    }
}

struct TextVisualParams<'a> {
    id: UiNodeId,
    node: &'a UiNode,
    style: UiStyle,
    text: String,
    origin_x: f32,
    width: f32,
    height: f32,
    bounds: UiRect,
}

fn emit_node_text(
    text_engine: &mut UiTextEngine,
    params: TextVisualParams<'_>,
    emission: &mut UiEmission<'_>,
) -> Result<(), DisplayListError> {
    let text_bounds = UiRect::new(
        UiPoint {
            x: params.origin_x,
            y: params.bounds.origin.y + params.style.padding,
        },
        UiSize::new(params.width, params.height),
    );
    let text_output = text_engine.layout(
        &params.text,
        text_bounds.size.width,
        params.style.font_size,
        emission.scale_factor,
        params.node.font_role,
    );
    if text_output.layout.used_fallback_metrics {
        emission
            .diagnostics
            .push(UiDiagnostic::TextFallbackMetrics { node: params.id });
    }
    if text_output.layout.used_fallback_font {
        emission
            .diagnostics
            .push(UiDiagnostic::TextFontSubstituted { node: params.id });
    }
    if text_output.raster.has_unrasterized_glyphs {
        emission
            .diagnostics
            .push(UiDiagnostic::TextRasterIncomplete { node: params.id });
    }
    emission.display.try_push(DisplayPrimitive::Text {
        node: params.id,
        bounds: text_bounds,
        text: params.text,
        color: params.style.foreground,
        layout: text_output.layout,
        raster: text_output.raster,
    })
}

#[derive(Clone, Copy)]
struct IconVisualParams<'a> {
    id: UiNodeId,
    node: &'a UiNode,
    bounds: UiRect,
    style: UiStyle,
    has_text: bool,
}

fn emit_node_icon(
    params: IconVisualParams<'_>,
    emission: &mut UiEmission<'_>,
    text_origin_x: &mut f32,
    text_width: &mut f32,
) -> Result<(), DisplayListError> {
    let Some(icon) = params.node.icon else {
        return Ok(());
    };
    let content_width = (params.bounds.size.width - params.style.padding * 2.0).max(1.0);
    let content_height = (params.bounds.size.height - params.style.padding * 2.0).max(1.0);
    let side = emission
        .icon_tokens
        .size
        .min(content_width)
        .min(content_height)
        .max(1.0);
    let icon_x = if params.has_text {
        *text_origin_x
    } else {
        params.bounds.origin.x + (params.bounds.size.width - side) * 0.5
    };
    let icon_bounds = UiRect::new(
        UiPoint {
            x: icon_x,
            y: params.bounds.origin.y + (params.bounds.size.height - side) * 0.5,
        },
        UiSize::new(side, side),
    );
    match icon_geometry(icon, icon_bounds) {
        Ok(geometry) => {
            let stroke = UiStroke::new(params.style.foreground, emission.icon_tokens.stroke_width);
            for commands in geometry.paths {
                emission.display.try_push(DisplayPrimitive::Path {
                    node: params.id,
                    commands,
                    fill: None,
                    stroke: Some(stroke),
                })?;
            }
            if params.has_text {
                let gap = emission.icon_tokens.text_gap;
                *text_origin_x += side + gap;
                *text_width = (*text_width - side - gap).max(1.0);
            }
        }
        Err(_) => emission
            .diagnostics
            .push(UiDiagnostic::IconGeometryRejected { node: params.id }),
    }
    if emission.icon_tokens_fallback {
        emission
            .diagnostics
            .push(UiDiagnostic::IconThemeTokensFallback { node: params.id });
    }
    Ok(())
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
    let mut overlay = UiNode::container(root, "Runtime overlay", UiLayout::Overlay, vec![label])
        .with_style_variant(meridian_ui_core::UiStyleVariant::Transparent);
    overlay.kind = UiWidgetKind::Overlay;
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
        UiAbsolutePosition, UiControlState, UiDragItemId, UiLayoutHints, UiScrollDelta,
        UiSemanticCollectionItem, UiSemanticRelationships, UiSharedElementId, UiStyle,
        UiTextInputOptions,
    };
    use meridian_ui_text::UiTextSelection;

    fn frame(events: Vec<UiEvent>) -> UiFrameInput {
        UiFrameInput {
            events,
            ..UiFrameInput::new(UiSize::new(800.0, 600.0))
        }
    }

    fn presentation_motion_document(
        root: UiNodeId,
        panel: UiNodeId,
        button: UiNodeId,
        left: f32,
        opacity: f32,
        shared: Option<UiSharedElementId>,
    ) -> UiDocument {
        let panel = UiNode::container(panel, "Inspector", UiLayout::Overlay, vec![button])
            .with_absolute_position(UiAbsolutePosition {
                left,
                top: 20.0,
                width: Some(160.0),
                height: Some(100.0),
            })
            .with_presentation_opacity(opacity);
        let panel = match shared {
            Some(shared) => panel.with_shared_element_motion(shared),
            None => panel.with_spatial_motion(UiSpatialMotionKind::PhysicalPanel),
        };
        UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Motion fixture", UiLayout::Absolute, vec![panel.id])
                    .with_style(UiStyle::transparent()),
                panel,
                UiNode::button(button, "Apply inspector", "inspector.apply", "Apply"),
            ],
        )
        .expect("motion fixture is valid")
    }

    #[test]
    fn recovery_panel_emits_display_and_semantics() {
        let document = recovery_panel_document().expect("recovery fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));
        assert!(output.display_list.primitives.len() >= 3);
        assert!(matches!(output.semantic_delta, SemanticDelta::Replace(_)));
        assert_eq!(
            output.frame_diagnostics.display_primitives,
            UiFrameDiagnostics::count(output.display_list.primitives.len())
        );
        assert_eq!(
            output.frame_diagnostics.semantic_nodes,
            UiFrameDiagnostics::count(output.semantic_tree.nodes.len())
        );
        assert!((output.frame_diagnostics.scale_factor - 1.0).abs() < f32::EPSILON);
        assert!(!output.frame_diagnostics.recovered_previous_snapshot);
    }

    #[test]
    fn runtime_emits_declared_descriptions_relations_and_virtual_collection_positions() {
        let root = UiNodeId::new(0x2b0);
        let label = UiNodeId::new(0x2b1);
        let help = UiNodeId::new(0x2b2);
        let error = UiNodeId::new(0x2b3);
        let field = UiNodeId::new(0x2b4);
        let row = UiNodeId::new(0x2b5);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Semantic output",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![label, help, error, field, row],
                ),
                UiNode::label(label, "Project label", "Project name"),
                UiNode::tooltip(help, "Project help", "Shown in the title bar"),
                UiNode::label(error, "Project error", "A project with this name exists"),
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
                .with_semantic_description("Saved display name for this project")
                .with_semantic_relationships(UiSemanticRelationships {
                    labelled_by: vec![label],
                    described_by: vec![help],
                    controls: vec![row],
                    details: vec![help],
                    flow_to: vec![row],
                    error_message: Some(error),
                }),
                UiNode::list_item(row, "World", "world.open", false).with_semantic_collection_item(
                    UiSemanticCollectionItem {
                        position: 4,
                        set_size: 12,
                    },
                ),
            ],
        )
        .expect("semantic metadata document is accepted");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));
        let field_semantics = output
            .semantic_tree
            .nodes
            .iter()
            .find(|node| node.id == field)
            .expect("field semantic node is emitted");
        assert_eq!(
            field_semantics.description.as_deref(),
            Some("Saved display name for this project")
        );
        assert_eq!(field_semantics.relationships.labelled_by, vec![label]);
        assert_eq!(field_semantics.relationships.described_by, vec![help]);
        assert_eq!(field_semantics.relationships.controls, vec![row]);
        assert_eq!(field_semantics.relationships.details, vec![help]);
        assert_eq!(field_semantics.relationships.flow_to, vec![row]);
        assert_eq!(field_semantics.relationships.error_message, Some(error));
        assert_eq!(
            output
                .semantic_tree
                .nodes
                .iter()
                .find(|node| node.id == row)
                .expect("row semantic node is emitted")
                .collection_item,
            Some(SemanticCollectionItem {
                position: 4,
                set_size: 12,
            })
        );
    }

    #[test]
    fn runtime_hides_declared_error_relationship_until_the_frame_is_invalid() {
        let root = UiNodeId::new(0x2c0);
        let error = UiNodeId::new(0x2c1);
        let field = UiNodeId::new(0x2c2);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Semantic error",
                    UiLayout::Overlay,
                    vec![error, field],
                ),
                UiNode::label(error, "Project error", "A project with this name exists"),
                UiNode::text_input(
                    field,
                    "Project name",
                    "Creator Alpha",
                    UiTextInputOptions::default(),
                )
                .with_semantic_relationships(UiSemanticRelationships {
                    error_message: Some(error),
                    ..UiSemanticRelationships::default()
                }),
            ],
        )
        .expect("potential error relationship is retained");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));
        assert_eq!(
            output
                .semantic_tree
                .nodes
                .iter()
                .find(|node| node.id == field)
                .expect("field semantic node is emitted")
                .relationships
                .error_message,
            None
        );
    }

    #[test]
    fn frame_diagnostics_report_runtime_coverage_without_renderer_claims() {
        let root = UiNodeId::new(0x2a0);
        let list = UiNodeId::new(0x2a1);
        let action = UiNodeId::new(0x2a2);
        let icon = UiNodeId::new(0x2a3);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Diagnostics",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![list],
                )
                .with_style(UiStyle::transparent()),
                UiNode::virtual_list(list, "Virtual rows", vec![action, icon]).with_constraints(
                    UiConstraints {
                        clip: true,
                        ..UiConstraints::default()
                    },
                ),
                UiNode::button(action, "Run diagnostics", "diagnostics.run", "Run"),
                UiNode::icon_button(
                    icon,
                    "Play diagnostics",
                    "diagnostics.play",
                    meridian_ui_core::IconId::Play,
                ),
            ],
        )
        .expect("diagnostics fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(vec![UiEvent::FocusNext, UiEvent::Activate]));
        let diagnostics = output.frame_diagnostics;

        assert_eq!(diagnostics.reconciliation.accepted_nodes, 4);
        assert_eq!(diagnostics.reconciliation.layout_roots, 1);
        assert_eq!(
            diagnostics.reconciliation.reconciled_layout_nodes,
            diagnostics.layout_nodes
        );
        assert_eq!(diagnostics.interaction.input_events, 2);
        assert_eq!(
            diagnostics.interaction.event_routes,
            diagnostics.event_routes
        );
        assert_eq!(diagnostics.interaction.focus_entries, 1);
        assert_eq!(diagnostics.interaction.virtualized_regions, 1);
        assert_eq!(diagnostics.interaction.active_animation_tracks, Some(0));
        assert_eq!(diagnostics.primitives.clip_pushes, 1);
        assert_eq!(diagnostics.primitives.clip_pops, 1);
        assert!(diagnostics.primitives.paths >= 1);
        assert!(diagnostics.text.primitives >= 1);
        assert!(diagnostics.text.glyphs >= 1);
        assert_eq!(
            diagnostics.timing,
            UiFrameTimingDiagnostics {
                reconciliation: UiMeasurementAvailability::Unavailable,
                layout: UiMeasurementAvailability::Unavailable,
                text_shaping: UiMeasurementAvailability::Unavailable,
                text_rasterization: UiMeasurementAvailability::Unavailable,
                display_validation: UiMeasurementAvailability::Unavailable,
                semantic_delta: UiMeasurementAvailability::Unavailable,
            }
        );
        assert_eq!(
            diagnostics.renderer_cache,
            UiRendererCacheDiagnostics::default()
        );
        assert_eq!(
            diagnostics.overdraw,
            UiOverdrawDiagnostics {
                estimate: UiMeasurementAvailability::Unavailable,
            }
        );
        assert_eq!(
            diagnostics.event_latency,
            UiEventLatencyDiagnostics {
                measurement: UiMeasurementAvailability::Unavailable,
                source_timestamped_events: 0,
            }
        );
        assert_eq!(diagnostics.capture, UiCaptureState::NotRequested);
        assert_eq!(
            diagnostics.virtualization,
            UiVirtualizationDiagnostics {
                declared_regions: 1,
                realized_ranges: UiMeasurementAvailability::Unavailable,
                cache_state: UiMeasurementAvailability::Unavailable,
            }
        );
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
            SemanticDelta::Update(_) => panic!("first frame cannot be incremental"),
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
            .any(|primitive| matches!(
                primitive,
                DisplayPrimitive::Border { .. } | DisplayPrimitive::RoundedRect { .. }
            )));
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
    fn typed_focus_restoration_accepts_only_enabled_focusable_identity() {
        let root = UiNodeId::new(1);
        let action = UiNodeId::new(2);
        let label = UiNodeId::new(3);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Focus restoration fixture",
                    UiLayout::VerticalStack { gap: 4.0 },
                    vec![action, label],
                ),
                UiNode::button(action, "Search", "fixture.search", "Search"),
                UiNode::label(label, "Read-only status", "Ready"),
            ],
        )
        .expect("focus restoration fixture is valid");
        let mut runtime = UiRuntime::new(document);

        assert!(!runtime.focus_retained_node(UiNodeId::new(99)));
        assert!(!runtime.focus_retained_node(label));
        assert!(runtime.focus_retained_node(action));
        let output = runtime.reconcile(frame(Vec::new()));
        assert_eq!(output.focused, Some(action));
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
    fn icon_button_emits_owned_paths_and_preserves_accessible_name() {
        let button = UiNodeId::new(0x240);
        let document = UiDocument::new(
            button,
            vec![UiNode::icon_button(
                button,
                "Run project",
                "project.play",
                meridian_ui_core::IconId::Play,
            )],
        )
        .expect("icon button fixture is valid");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(frame(Vec::new()));

        assert!(output
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::Path { .. })));
        assert!(!output
            .diagnostics
            .contains(&UiDiagnostic::IconGeometryRejected { node: button }));
        let semantic = output
            .semantic_tree
            .nodes
            .iter()
            .find(|node| node.id == button)
            .expect("icon button semantic node");
        assert_eq!(semantic.name, "Run project");
        assert_eq!(semantic.command.as_deref(), Some("project.play"));
    }

    #[test]
    fn active_theme_icon_tokens_control_size_stroke_and_text_gap() {
        let root = UiNodeId::new(0x77a);
        let mut button = UiNode::button(root, "Run", "run", "Run").with_style(UiStyle {
            font_size: 28.0,
            ..UiStyle::secondary_action()
        });
        button.icon = Some(meridian_ui_core::IconId::Play);
        let document = UiDocument::new(root, vec![button]).expect("icon and text button");
        let mut runtime = UiRuntime::new(document);
        let mut input = frame(Vec::new());
        input.theme.icons.size = 10.0;
        input.theme.icons.stroke_width = 3.0;
        input.theme.icons.text_gap = 5.0;
        let output = runtime.reconcile(input);

        let strokes: Vec<_> = output
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Path { stroke, .. } => *stroke,
                _ => None,
            })
            .collect();
        assert!(!strokes.is_empty());
        assert!(strokes
            .iter()
            .all(|stroke| (stroke.width - 3.0).abs() < f32::EPSILON));
        let text_x = output
            .display_list
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                DisplayPrimitive::Text { bounds, .. } => Some(bounds.origin.x),
                _ => None,
            })
            .expect("text primitive");
        assert!((text_x - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn huge_finite_theme_metrics_emit_registered_fallback_geometry() {
        let root = UiNodeId::new(0x77d);
        let mut button = UiNode::button(root, "Run", "run", "Run");
        button.icon = Some(meridian_ui_core::IconId::Play);
        let document = UiDocument::new(root, vec![button]).expect("icon and text button");
        let mut runtime = UiRuntime::new(document);
        let mut input = frame(Vec::new());
        input.theme.icons.size = f32::MAX;
        input.theme.icons.stroke_width = f32::MAX;
        input.theme.icons.text_gap = f32::MAX;
        input.theme.geometry.spacing_base = f32::MAX;
        input.theme.geometry.radius_control = f32::MAX;
        let output = runtime
            .try_reconcile(input)
            .expect("huge finite theme metrics fall back before emission");

        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::IconThemeTokensFallback { node: root }));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::StyleTokenFallback { node: root }));
        let strokes: Vec<_> = output
            .display_list
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                DisplayPrimitive::Path { stroke, .. } => *stroke,
                _ => None,
            })
            .collect();
        assert!(!strokes.is_empty());
        assert!(strokes.iter().all(|stroke| {
            (stroke.width - UiTheme::meridian_dark().icons.stroke_width).abs() < f32::EPSILON
        }));
        assert!(
            (runtime.resolved_styles[&root].corner_radius
                - UiTheme::meridian_dark().geometry.radius_control)
                .abs()
                < f32::EPSILON
        );
        assert!(
            (runtime.resolved_styles[&root].padding
                - UiTheme::meridian_dark().geometry.spacing_base * 2.5)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn retained_visual_states_resolve_selectors_and_non_color_indicators() {
        let root = UiNodeId::new(0x77b);
        let document = UiDocument::new(root, vec![UiNode::button(root, "Run", "run", "Run")])
            .expect("button document");
        let mut runtime = UiRuntime::new(document);

        let hovered = runtime.reconcile(frame(vec![UiEvent::Pointer(UiPointerEvent {
            device: LEGACY_POINTER_DEVICE,
            kind: UiInputDeviceKind::Mouse,
            phase: UiPointerPhase::Move,
            position: UiPoint { x: 20.0, y: 20.0 },
            button: None,
        })]));
        assert_eq!(hovered.visual_states[0].selector, UiStyleSelector::Hovered);
        assert!(hovered.visual_states[0].state.hovered);

        let pressed = runtime.reconcile(frame(vec![UiEvent::PointerDown(UiPoint {
            x: 20.0,
            y: 20.0,
        })]));
        assert_eq!(pressed.visual_states[0].selector, UiStyleSelector::Pressed);
        assert!(pressed.visual_states[0].state.pressed);
        assert!(pressed.visual_states[0].state.focused);

        let focused = runtime.reconcile(frame(vec![UiEvent::PointerUp(UiPoint {
            x: 20.0,
            y: 20.0,
        })]));
        assert_eq!(focused.visual_states[0].selector, UiStyleSelector::Focused);
        assert!(focused
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::FocusIndicator { node, .. } if *node == root)));

        let selected_document = UiDocument::new(
            root,
            vec![
                UiNode::button(root, "Selected", "select", "Selected").with_control_state(
                    UiControlState {
                        selected: true,
                        ..UiControlState::default()
                    },
                ),
            ],
        )
        .expect("selected button");
        runtime.replace_document(selected_document);
        let selected = runtime.reconcile(frame(Vec::new()));
        assert_eq!(
            selected.visual_states[0].selector,
            UiStyleSelector::Selected
        );
        assert!(selected.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Rect { node, bounds, .. }
                if *node == root && (bounds.size.width - 3.0).abs() < f32::EPSILON)
        }));

        let invalid_document = UiDocument::new(
            root,
            vec![
                UiNode::text_input(root, "Invalid field", "bad", UiTextInputOptions::default())
                    .with_control_state(UiControlState {
                        invalid: true,
                        ..UiControlState::default()
                    }),
            ],
        )
        .expect("invalid field");
        runtime.replace_document(invalid_document);
        let invalid = runtime.reconcile(frame(Vec::new()));
        assert_eq!(invalid.visual_states[0].selector, UiStyleSelector::Invalid);
        assert!(invalid.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Border { node, width: 2, .. } if *node == root)
        }));
    }

    #[test]
    fn disabled_selector_suppresses_selected_and_invalid_structural_treatments() {
        let root = UiNodeId::new(0x77e);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::button(root, "Disabled", "disabled", "Disabled").with_control_state(
                    UiControlState {
                        disabled: true,
                        selected: true,
                        invalid: true,
                        ..UiControlState::default()
                    },
                ),
            ],
        )
        .expect("disabled button");
        let mut runtime = UiRuntime::new(document);
        let mut high_contrast = frame(Vec::new());
        high_contrast.high_contrast = true;
        let disabled = runtime.reconcile(high_contrast);
        assert_eq!(
            disabled.visual_states[0].selector,
            UiStyleSelector::Disabled
        );
        assert_eq!(
            runtime.resolved_styles[&root].foreground,
            UiTheme::meridian_dark().high_contrast_colors.muted
        );
        assert!(!disabled.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Rect { node, bounds, .. }
                if *node == root
                    && ((bounds.size.width - 3.0).abs() < f32::EPSILON
                        || (bounds.size.height - 3.0).abs() < f32::EPSILON))
        }));
        assert!(!disabled.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::Border { node, width: 2, .. } if *node == root)
        }));
    }

    #[test]
    fn frame_motion_animates_state_color_and_reduced_motion_settles_it() {
        let root = UiNodeId::new(0x77c);
        let document = UiDocument::new(root, vec![UiNode::button(root, "Run", "run", "Run")])
            .expect("button document");
        let mut runtime = UiRuntime::new(document);
        let initial = runtime.reconcile(frame(Vec::new()));
        assert_eq!(
            initial
                .frame_diagnostics
                .interaction
                .active_animation_tracks,
            Some(0)
        );

        let hovered = runtime.reconcile(frame(vec![UiEvent::Pointer(UiPointerEvent {
            device: LEGACY_POINTER_DEVICE,
            kind: UiInputDeviceKind::Mouse,
            phase: UiPointerPhase::Move,
            position: UiPoint { x: 20.0, y: 20.0 },
            button: None,
        })]));
        assert_eq!(
            hovered
                .frame_diagnostics
                .interaction
                .active_animation_tracks,
            Some(1)
        );
        assert_eq!(hovered.motion, MotionPreference::Full);

        let mut advanced_input = frame(Vec::new());
        advanced_input.presentation_delta_ms = 50;
        let advanced = runtime.reconcile(advanced_input);
        assert_eq!(advanced.visual_states[0].selector, UiStyleSelector::Hovered);
        assert_eq!(
            advanced
                .frame_diagnostics
                .interaction
                .active_animation_tracks,
            Some(1)
        );

        let mut reduced_input = frame(Vec::new());
        reduced_input.reduced_motion = true;
        let reduced = runtime.reconcile(reduced_input);
        assert_eq!(reduced.motion, MotionPreference::Reduced);
        assert_eq!(
            reduced
                .frame_diagnostics
                .interaction
                .active_animation_tracks,
            Some(0)
        );
        assert_eq!(
            runtime.resolved_styles[&root]
                .border
                .map(|border| border.color),
            Some(UiTheme::meridian_dark().colors.warning)
        );
    }

    #[test]
    fn runtime_consumes_retargeted_physical_and_opacity_presentation_without_moving_hits() {
        let root = UiNodeId::new(0x77f);
        let panel = UiNodeId::new(0x780);
        let button = UiNodeId::new(0x781);
        let mut runtime = UiRuntime::new(presentation_motion_document(
            root, panel, button, 16.0, 1.0, None,
        ));
        let initial = runtime.reconcile(frame(Vec::new()));
        let initial_panel = initial
            .layout
            .iter()
            .find(|entry| entry.node == panel)
            .expect("initial panel layout")
            .bounds;

        runtime.replace_document(presentation_motion_document(
            root, panel, button, 400.0, 0.25, None,
        ));
        let moved = runtime.reconcile(frame(vec![
            UiEvent::PointerDown(UiPoint { x: 440.0, y: 60.0 }),
            UiEvent::PointerUp(UiPoint { x: 440.0, y: 60.0 }),
        ]));
        let target_panel = moved
            .layout
            .iter()
            .find(|entry| entry.node == panel)
            .expect("target panel layout")
            .bounds;
        let presentation_panel = moved
            .presentation_layout
            .iter()
            .find(|entry| entry.node == panel)
            .expect("panel presentation layout")
            .bounds;
        assert_eq!(presentation_panel, initial_panel);
        assert_ne!(presentation_panel, target_panel);
        assert_eq!(
            moved.commands,
            vec![UiCommandRequest {
                source: button,
                action: "inspector.apply".to_owned(),
            }]
        );
        let spatial = moved
            .presentation_motion
            .iter()
            .find(|snapshot| snapshot.node == panel && snapshot.channel == UiMotionChannel::Spatial)
            .copied()
            .expect("physical panel track is snapshotted");
        assert_eq!(
            spatial.spatial_kind,
            Some(UiSpatialMotionKind::PhysicalPanel)
        );
        assert_eq!(spatial.current, UiPresentationValue::Rect(initial_panel));
        assert_eq!(spatial.target, UiPresentationValue::Rect(target_panel));
        assert!(spatial.active);
        let opacity = moved
            .presentation_motion
            .iter()
            .find(|snapshot| snapshot.node == panel && snapshot.channel == UiMotionChannel::Opacity)
            .copied()
            .expect("opacity track is snapshotted");
        assert_eq!(opacity.current, UiPresentationValue::Opacity(1.0));
        assert_eq!(opacity.target, UiPresentationValue::Opacity(0.25));
        assert!(opacity.active);

        let mut advanced_input = frame(Vec::new());
        advanced_input.presentation_delta_ms = 50;
        let advanced = runtime.reconcile(advanced_input);
        let advanced_spatial = advanced
            .presentation_motion
            .iter()
            .find(|snapshot| snapshot.node == panel && snapshot.channel == UiMotionChannel::Spatial)
            .copied()
            .expect("advanced spatial track");
        assert_ne!(
            advanced_spatial.current,
            UiPresentationValue::Rect(initial_panel)
        );
        assert_ne!(
            advanced_spatial.current,
            UiPresentationValue::Rect(target_panel)
        );
        assert!(advanced.display_list.primitives.iter().any(|primitive| {
            matches!(primitive, DisplayPrimitive::RoundedRect { node, color, .. }
                if *node == panel && color.alpha < 1.0 && color.alpha > 0.25)
        }));
        assert!(!advanced
            .display_list
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, DisplayPrimitive::BeginLayer { .. })));
    }

    #[test]
    fn physical_motion_retargeting_preserves_the_current_presentation() {
        let root = UiNodeId::new(0x77f);
        let panel = UiNodeId::new(0x780);
        let button = UiNodeId::new(0x781);
        let mut runtime = UiRuntime::new(presentation_motion_document(
            root, panel, button, 16.0, 1.0, None,
        ));
        runtime.reconcile(frame(Vec::new()));
        runtime.replace_document(presentation_motion_document(
            root, panel, button, 400.0, 0.25, None,
        ));
        runtime.reconcile(frame(Vec::new()));
        let mut advanced_input = frame(Vec::new());
        advanced_input.presentation_delta_ms = 50;
        let advanced = runtime.reconcile(advanced_input);
        let advanced_spatial = advanced
            .presentation_motion
            .iter()
            .find(|snapshot| snapshot.node == panel && snapshot.channel == UiMotionChannel::Spatial)
            .copied()
            .expect("advanced spatial track");

        runtime.replace_document(presentation_motion_document(
            root, panel, button, 220.0, 0.8, None,
        ));
        let mut retarget_input = frame(Vec::new());
        retarget_input.presentation_delta_ms = 0;
        let retargeted = runtime.reconcile(retarget_input);
        let retargeted_spatial = retargeted
            .presentation_motion
            .iter()
            .find(|snapshot| snapshot.node == panel && snapshot.channel == UiMotionChannel::Spatial)
            .copied()
            .expect("retargeted spatial track");
        assert_eq!(retargeted_spatial.current, advanced_spatial.current);
    }

    #[test]
    fn reduced_motion_snaps_physical_presentation_to_authoritative_geometry() {
        let root = UiNodeId::new(0x77f);
        let panel = UiNodeId::new(0x780);
        let button = UiNodeId::new(0x781);
        let mut runtime = UiRuntime::new(presentation_motion_document(
            root, panel, button, 16.0, 1.0, None,
        ));
        runtime.reconcile(frame(Vec::new()));
        runtime.replace_document(presentation_motion_document(
            root, panel, button, 320.0, 0.4, None,
        ));
        let mut input = frame(Vec::new());
        input.reduced_motion = true;
        let reduced = runtime.reconcile(input);
        let reduced_panel = reduced
            .layout
            .iter()
            .find(|entry| entry.node == panel)
            .expect("reduced target layout")
            .bounds;
        assert_eq!(
            reduced
                .presentation_layout
                .iter()
                .find(|entry| entry.node == panel)
                .expect("reduced presentation layout")
                .bounds,
            reduced_panel
        );
        assert!(reduced
            .presentation_motion
            .iter()
            .all(|snapshot| { snapshot.node != panel || !snapshot.active }));
    }

    #[test]
    fn shared_element_motion_handoffs_between_distinct_cross_frame_nodes() {
        let root = UiNodeId::new(0x77f);
        let shared_source = UiNodeId::new(0x782);
        let shared_target = UiNodeId::new(0x783);
        let shared_button = UiNodeId::new(0x784);
        let shared = UiSharedElementId::new(0x55aa);
        let mut shared_runtime = UiRuntime::new(presentation_motion_document(
            root,
            shared_source,
            shared_button,
            24.0,
            1.0,
            Some(shared),
        ));
        let source = shared_runtime.reconcile(frame(Vec::new()));
        let source_bounds = source
            .layout
            .iter()
            .find(|entry| entry.node == shared_source)
            .expect("shared source layout")
            .bounds;
        shared_runtime.replace_document(presentation_motion_document(
            root,
            shared_target,
            shared_button,
            480.0,
            1.0,
            Some(shared),
        ));
        let shared_output = shared_runtime.reconcile(frame(Vec::new()));
        let shared_spatial = shared_output
            .presentation_motion
            .iter()
            .find(|snapshot| {
                snapshot.node == shared_target && snapshot.channel == UiMotionChannel::Spatial
            })
            .copied()
            .expect("shared-element track is snapshotted on its new node");
        assert_eq!(
            shared_spatial.spatial_kind,
            Some(UiSpatialMotionKind::SharedElement)
        );
        assert_eq!(
            shared_spatial.current,
            UiPresentationValue::Rect(source_bounds)
        );
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
            .any(|primitive| matches!(primitive, DisplayPrimitive::FocusIndicator { .. })));
    }

    #[test]
    fn one_to_four_x_frames_are_deterministic_for_identical_inputs() {
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
        let four_x_a = reconcile_at(4.0);
        let four_x_b = reconcile_at(4.0);
        assert_eq!(one_x_a, one_x_b);
        assert_eq!(two_x_a, two_x_b);
        assert_eq!(four_x_a, four_x_b);
        assert_eq!(one_x_a.layout, two_x_a.layout);
        assert_eq!(one_x_a.layout, four_x_a.layout);
        assert!((four_x_a.scale_factor - 4.0).abs() < f32::EPSILON);
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
        assert_eq!(
            output
                .semantic_tree
                .nodes
                .iter()
                .find(|node| node.id == input)
                .and_then(|node| node.value.as_deref()),
            Some("axe\u{301}")
        );

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
            output
                .semantic_tree
                .nodes
                .iter()
                .find(|node| node.id == input)
                .and_then(|node| node.value.as_deref()),
            None
        );
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
            UiEvent::AssistiveSetValue {
                target: input,
                text: "125".to_owned(),
                replace_selection: false,
            },
            UiEvent::SelectAllText,
            UiEvent::AssistiveSetValue {
                target: input,
                text: "250".to_owned(),
                replace_selection: true,
            },
            UiEvent::AssistiveFocus(label),
            UiEvent::AssistiveActivate(label),
            UiEvent::AssistiveSetValue {
                target: label,
                text: "denied".to_owned(),
                replace_selection: false,
            },
        ]));

        assert_eq!(output.focused, Some(input));
        assert_eq!(runtime.text_input_value(input), Some("250"));
        assert_eq!(
            output
                .semantic_tree
                .nodes
                .iter()
                .find(|node| node.id == input)
                .and_then(|node| node.value.as_deref()),
            Some("250")
        );
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveFocusDenied { node: label }));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveActivateDenied { node: label }));
        assert!(output
            .diagnostics
            .contains(&UiDiagnostic::AssistiveEditDenied { node: label }));
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
    fn flex_and_grid_honor_constraints_padding_gap_and_alignment() {
        let flex_root = UiNodeId::new(0x530);
        let flex_first = UiNodeId::new(0x531);
        let flex_second = UiNodeId::new(0x532);
        let flex_document = UiDocument::new(
            flex_root,
            vec![
                UiNode::container(
                    flex_root,
                    "Constrained flex",
                    UiLayout::Flex {
                        axis: UiAxis::Horizontal,
                        gap: 10.0,
                    },
                    vec![flex_first, flex_second],
                )
                .with_style(UiStyle {
                    padding: 8.0,
                    ..UiStyle::transparent()
                }),
                UiNode::label(flex_first, "First", "First")
                    .with_layout_hints(UiLayoutHints::fixed_size(70.0, 32.0))
                    .with_constraints(UiConstraints {
                        minimum: UiSize::new(20.0, 20.0),
                        maximum: Some(UiSize::new(64.0, 80.0)),
                        vertical_alignment: UiAlignment::End,
                        ..UiConstraints::default()
                    }),
                UiNode::label(flex_second, "Second", "Second")
                    .with_layout_hints(UiLayoutHints::fixed_width(70.0)),
            ],
        )
        .expect("flex constraints are valid");
        let mut flex_runtime = UiRuntime::new(flex_document);
        let flex = flex_runtime.reconcile(UiFrameInput::new(UiSize::new(240.0, 100.0)));
        let flex_bounds = |id| {
            flex.layout
                .iter()
                .find(|entry| entry.node == id)
                .map(|entry| entry.bounds)
                .expect("flex node has bounds")
        };
        assert_eq!(flex_bounds(flex_first).size, UiSize::new(64.0, 32.0));
        assert_eq!(flex_bounds(flex_first).origin, UiPoint { x: 8.0, y: 60.0 });
        assert!((flex_bounds(flex_second).origin.x - 82.0).abs() < f32::EPSILON);

        let grid_root = UiNodeId::new(0x540);
        let grid_first = UiNodeId::new(0x541);
        let grid_second = UiNodeId::new(0x542);
        let grid_constraints = UiConstraints {
            minimum: UiSize::new(20.0, 20.0),
            maximum: Some(UiSize::new(80.0, 60.0)),
            aspect_ratio: Some(2.0),
            horizontal_alignment: UiAlignment::Center,
            vertical_alignment: UiAlignment::End,
            ..UiConstraints::default()
        };
        let grid_document = UiDocument::new(
            grid_root,
            vec![
                UiNode::container(
                    grid_root,
                    "Constrained grid",
                    UiLayout::Grid {
                        columns: 2,
                        gap: 10.0,
                    },
                    vec![grid_first, grid_second],
                )
                .with_style(UiStyle {
                    padding: 5.0,
                    ..UiStyle::transparent()
                }),
                UiNode::label(grid_first, "Grid first", "First")
                    .with_layout_hints(UiLayoutHints::fixed_width(60.0))
                    .with_constraints(grid_constraints),
                UiNode::label(grid_second, "Grid second", "Second")
                    .with_layout_hints(UiLayoutHints::fixed_width(60.0))
                    .with_constraints(grid_constraints),
            ],
        )
        .expect("grid constraints are valid");
        let mut grid_runtime = UiRuntime::new(grid_document);
        let grid = grid_runtime.reconcile(UiFrameInput::new(UiSize::new(230.0, 120.0)));
        let grid_bounds = |id| {
            grid.layout
                .iter()
                .find(|entry| entry.node == id)
                .map(|entry| entry.bounds)
                .expect("grid node has bounds")
        };
        assert_eq!(grid_bounds(grid_first).size, UiSize::new(80.0, 40.0));
        // The 10px grid gap belongs inside the padded 220px content width:
        // each 105px cell centers its 80px constrained child without pushing
        // the second column beyond the parent slot.
        assert_eq!(grid_bounds(grid_first).origin, UiPoint { x: 17.5, y: 75.0 });
        assert_eq!(
            grid_bounds(grid_second).origin,
            UiPoint { x: 132.5, y: 75.0 }
        );
    }

    #[test]
    fn overlay_and_absolute_honor_preferred_sizes_padding_and_alignment() {
        let overlay_root = UiNodeId::new(0x550);
        let overlay_child = UiNodeId::new(0x551);
        let overlay_document = UiDocument::new(
            overlay_root,
            vec![
                UiNode::container(
                    overlay_root,
                    "Overlay",
                    UiLayout::Overlay,
                    vec![overlay_child],
                )
                .with_style(UiStyle {
                    padding: 10.0,
                    ..UiStyle::transparent()
                }),
                UiNode::label(overlay_child, "Overlay child", "Overlay")
                    .with_layout_hints(UiLayoutHints::fixed_size(60.0, 30.0))
                    .with_constraints(UiConstraints {
                        horizontal_alignment: UiAlignment::End,
                        vertical_alignment: UiAlignment::Center,
                        ..UiConstraints::default()
                    }),
            ],
        )
        .expect("overlay document");
        let mut overlay_runtime = UiRuntime::new(overlay_document);
        let overlay = overlay_runtime.reconcile(UiFrameInput::new(UiSize::new(200.0, 100.0)));
        assert_eq!(
            overlay
                .layout
                .iter()
                .find(|entry| entry.node == overlay_child)
                .expect("overlay child bounds")
                .bounds,
            UiRect::new(UiPoint { x: 130.0, y: 35.0 }, UiSize::new(60.0, 30.0))
        );

        let absolute_root = UiNodeId::new(0x560);
        let absolute_child = UiNodeId::new(0x561);
        let absolute_document = UiDocument::new(
            absolute_root,
            vec![
                UiNode::container(
                    absolute_root,
                    "Absolute",
                    UiLayout::Absolute,
                    vec![absolute_child],
                )
                .with_style(UiStyle {
                    padding: 10.0,
                    ..UiStyle::transparent()
                }),
                UiNode::label(absolute_child, "Absolute child", "Absolute")
                    .with_layout_hints(UiLayoutHints::fixed_size(60.0, 30.0))
                    .with_absolute_position(UiAbsolutePosition {
                        left: 20.0,
                        top: 5.0,
                        width: None,
                        height: None,
                    }),
            ],
        )
        .expect("absolute document");
        let mut absolute_runtime = UiRuntime::new(absolute_document);
        let absolute = absolute_runtime.reconcile(UiFrameInput::new(UiSize::new(200.0, 100.0)));
        assert_eq!(
            absolute
                .layout
                .iter()
                .find(|entry| entry.node == absolute_child)
                .expect("absolute child bounds")
                .bounds,
            UiRect::new(UiPoint { x: 30.0, y: 15.0 }, UiSize::new(60.0, 30.0))
        );
    }

    #[test]
    fn preferred_bounds_apply_constraints_before_parent_slot_alignment() {
        let root = UiNodeId::new(0x565);
        let child = UiNodeId::new(0x566);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Overlay", UiLayout::Overlay, vec![child])
                    .with_style(UiStyle::transparent()),
                UiNode::label(child, "Constrained preferred child", "Preferred")
                    .with_layout_hints(UiLayoutHints::fixed_size(120.0, 80.0))
                    .with_constraints(UiConstraints {
                        maximum: Some(UiSize::new(90.0, 50.0)),
                        aspect_ratio: Some(2.0),
                        horizontal_alignment: UiAlignment::End,
                        vertical_alignment: UiAlignment::Center,
                        ..UiConstraints::default()
                    }),
            ],
        )
        .expect("preferred child document");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(200.0, 100.0)));
        let bounds = output
            .layout
            .iter()
            .find(|entry| entry.node == child)
            .expect("preferred child bounds")
            .bounds;

        // The preferred 120x80 size resolves to 90x45 under the maximum and
        // aspect constraints, then aligns inside the 200x100 parent slot.
        assert_eq!(bounds.size, UiSize::new(90.0, 45.0));
        assert_eq!(bounds.origin, UiPoint { x: 110.0, y: 27.5 });
    }

    #[test]
    fn scroll_uses_resolved_child_extents_for_clamping() {
        let root = UiNodeId::new(0x570);
        let first = UiNodeId::new(0x571);
        let second = UiNodeId::new(0x572);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(
                    root,
                    "Scroll",
                    UiLayout::Scroll {
                        axis: UiAxis::Vertical,
                        offset: 100.0,
                    },
                    vec![first, second],
                )
                .with_style(UiStyle {
                    padding: 5.0,
                    ..UiStyle::transparent()
                }),
                UiNode::label(first, "First row", "First")
                    .with_layout_hints(UiLayoutHints::fixed_size(40.0, 80.0))
                    .with_constraints(UiConstraints {
                        minimum: UiSize::new(20.0, 20.0),
                        maximum: Some(UiSize::new(100.0, 50.0)),
                        horizontal_alignment: UiAlignment::Center,
                        ..UiConstraints::default()
                    }),
                UiNode::label(second, "Second row", "Second")
                    .with_layout_hints(UiLayoutHints::fixed_height(60.0)),
            ],
        )
        .expect("scroll document");
        let mut runtime = UiRuntime::new(document);
        let output = runtime.reconcile(UiFrameInput::new(UiSize::new(200.0, 100.0)));
        let bounds = |id| {
            output
                .layout
                .iter()
                .find(|entry| entry.node == id)
                .map(|entry| entry.bounds)
                .expect("scroll node has bounds")
        };
        assert_eq!(bounds(first).origin, UiPoint { x: 80.0, y: -15.0 });
        assert_eq!(bounds(first).size, UiSize::new(40.0, 50.0));
        assert!((bounds(second).origin.y - 35.0).abs() < f32::EPSILON);
        assert_eq!(
            output.scroll,
            vec![UiScrollSnapshot {
                node: root,
                offset: 20.0,
                maximum: 20.0,
            }]
        );
    }

    #[test]
    fn unsatisfiable_constraints_recover_last_accepted_geometry_and_cycle_guard_rejects() {
        let root = UiNodeId::new(0x580);
        let child = UiNodeId::new(0x581);
        let document = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Accepted", UiLayout::Overlay, vec![child]),
                UiNode::label(child, "Accepted child", "Accepted"),
            ],
        )
        .expect("accepted document");
        let mut runtime = UiRuntime::new(document);
        let accepted = runtime.reconcile(frame(Vec::new()));
        let impossible = UiDocument::new(
            root,
            vec![
                UiNode::container(root, "Impossible", UiLayout::Overlay, vec![child]),
                UiNode::label(child, "Impossible child", "Impossible")
                    .with_layout_hints(UiLayoutHints::fixed_size(100.0, 100.0))
                    .with_constraints(UiConstraints {
                        minimum: UiSize::new(100.0, 100.0),
                        maximum: Some(UiSize::new(100.0, 100.0)),
                        aspect_ratio: Some(2.0),
                        ..UiConstraints::default()
                    }),
            ],
        )
        .expect("per-axis constraints remain structurally valid");
        runtime.replace_document(impossible);
        let recovered = runtime.reconcile(frame(Vec::new()));
        assert_eq!(recovered.revision, accepted.revision);
        assert_eq!(recovered.layout, accepted.layout);
        assert!(recovered.frame_diagnostics.recovered_previous_snapshot);
        assert_eq!(
            recovered.diagnostics,
            vec![UiDiagnostic::LayoutConstraintsUnsatisfiable { node: child }]
        );

        let mut resolver = UiLayoutResolver {
            runtime: &runtime,
            layout: BTreeMap::new(),
            visiting: BTreeSet::from([root]),
            chain: vec![root],
        };
        assert_eq!(
            resolver.layout_node(root, UiRect::new(UiPoint::default(), UiSize::new(1.0, 1.0))),
            Err(UiLayoutError::ConstraintCycle {
                chain: vec![root, root],
            })
        );
    }

    #[test]
    fn recovered_frame_diagnostics_keep_the_snapshot_layout_root() {
        let accepted_root = UiNodeId::new(0x590);
        let accepted_document = UiDocument::new(
            accepted_root,
            vec![UiNode::label(accepted_root, "Accepted", "Accepted")],
        )
        .expect("accepted root document");
        let mut runtime = UiRuntime::new(accepted_document);
        let accepted = runtime.reconcile(frame(Vec::new()));

        let rejected_root = UiNodeId::new(0x591);
        let rejected_document = UiDocument::new(
            rejected_root,
            vec![
                UiNode::label(rejected_root, "Impossible", "Impossible").with_constraints(
                    UiConstraints {
                        minimum: UiSize::new(100.0, 100.0),
                        maximum: Some(UiSize::new(100.0, 100.0)),
                        aspect_ratio: Some(2.0),
                        ..UiConstraints::default()
                    },
                ),
            ],
        )
        .expect("per-axis constraints remain structurally valid");
        runtime.replace_document(rejected_document);

        let recovered = runtime.reconcile(frame(Vec::new()));
        assert_eq!(recovered.revision, accepted.revision);
        assert_eq!(recovered.semantic_tree.root, Some(accepted_root));
        assert!(recovered.frame_diagnostics.recovered_previous_snapshot);
        assert_eq!(recovered.frame_diagnostics.reconciliation.layout_roots, 1);
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
    fn semantic_rejection_rolls_back_focus_and_private_text_state() {
        let root = UiNodeId::new(0x5a0);
        let input = UiNodeId::new(0x5a1);
        let semantic_name_bytes = 112;
        let semantic_label_count = MAX_TEXT_BYTES / semantic_name_bytes + 1;
        let mut children = Vec::with_capacity(semantic_label_count + 1);
        children.push(input);
        let mut nodes = vec![UiNode::text_input(
            input,
            "Project title",
            "safe",
            UiTextInputOptions { password: false },
        )];
        for index in 0..semantic_label_count {
            let id = UiNodeId::new(0x600 + u128::try_from(index).expect("test index fits"));
            children.push(id);
            nodes.push(UiNode::label(id, "x".repeat(semantic_name_bytes), ""));
        }
        nodes.push(UiNode::container(
            root,
            "Aggregate semantics",
            UiLayout::VerticalStack { gap: 0.0 },
            children,
        ));
        let document = UiDocument::new(root, nodes).expect("per-node semantic text is bounded");
        let mut runtime = UiRuntime::new(document);

        let error = runtime
            .try_reconcile(frame(vec![
                UiEvent::FocusNext,
                UiEvent::SelectAllText,
                UiEvent::TextCommit("mutated".to_owned()),
            ]))
            .expect_err("aggregate semantic text must reject the frame");

        assert!(matches!(
            error,
            UiFrameError::SemanticTreeRejected(SemanticTreeError::TextTooLarge { .. })
        ));
        assert_eq!(runtime.text_input_value(input), Some("safe"));
        assert_eq!(runtime.focused, None);
        assert_eq!(runtime.revision, 0);
        assert!(runtime.previous_semantics.is_none());
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
    fn malformed_legacy_style_falls_back_to_tokens_before_emission() {
        let valid = recovery_panel_document().expect("valid recovery document");
        let mut runtime = UiRuntime::new(valid);
        let accepted = runtime.reconcile(frame(vec![UiEvent::FocusNext]));
        let root = UiNodeId::new(0x520);
        let invalid_node =
            UiNode::label(root, "Invalid pixels", "Invalid pixels").with_style(UiStyle {
                foreground: UiColor::rgba(f32::NAN, 1.0, 1.0, 1.0),
                ..UiStyle::text()
            });
        let invalid = UiDocument::new(root, vec![invalid_node]).expect("logical tree is valid");
        runtime.replace_document(invalid);

        let resolved = runtime
            .try_reconcile(frame(Vec::new()))
            .expect("legacy raw style is token-resolved");
        assert_eq!(resolved.revision, accepted.revision + 1);
        assert!(!resolved.frame_diagnostics.recovered_previous_snapshot);
        assert!(matches!(
            resolved.diagnostics.last(),
            Some(UiDiagnostic::StyleTokenFallback { node }) if *node == root
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
            SemanticDelta::Update(delta) => delta
                .updated
                .iter()
                .find(|node| node.id == input)
                .map(|node| node.state)
                .expect("changed input semantic node"),
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
