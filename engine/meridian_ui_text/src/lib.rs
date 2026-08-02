//! Meridian-owned text contracts and the private Cosmic Text adapter.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use cosmic_text::{
    fontdb, Attrs, Buffer, Family, FontSystem, Metrics, PhysicalGlyph, Shaping, SwashCache,
    SwashContent,
};
use meridian_ui_core::{
    sanitized_scale_factor, UiFontRole, UiFontWeight, UiNodeId, UiPoint, UiTextValidation,
    MAX_RETAINED_NODES, MAX_TEXT_BYTES,
};
use unicode_segmentation::UnicodeSegmentation;

/// Aggregate alpha-mask budget accepted for one immutable UI frame.
pub const MAX_GLYPH_RASTER_BYTES: usize = 1024 * 1024;

const MONA_SANS: &[u8] = include_bytes!("../assets/fonts/MonaSansVF.ttf");
const HUBOT_SANS: &[u8] = include_bytes!("../assets/fonts/HubotSansVF.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoVF.ttf");
const DISPLAY_ELLIPSIS: &str = "…";

const fn cosmic_font_weight(weight: UiFontWeight) -> fontdb::Weight {
    match weight {
        UiFontWeight::Normal => fontdb::Weight::NORMAL,
        UiFontWeight::Medium => fontdb::Weight::MEDIUM,
        UiFontWeight::Semibold => fontdb::Weight::SEMIBOLD,
    }
}

const fn bundled_family(font_role: UiFontRole) -> &'static str {
    match font_role {
        UiFontRole::Interface => "Mona Sans VF",
        UiFontRole::Display => "Hubot Sans",
        UiFontRole::Monospace => "JetBrains Mono",
    }
}

/// A cursor movement expressed in extended-grapheme positions, not UTF-8 bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextCursorDirection {
    Backward,
    Forward,
    Start,
    End,
}

/// Rejected IME composition data before private editor state changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiPreeditError {
    TooLong,
    InvalidCursor,
}

/// Rejection returned when a public text-input constructor exceeds the shared
/// retained-value bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextInputError {
    InitialValueTooLong,
}

/// Rejection returned before untrusted text reaches the private shaping
/// adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTextLayoutError {
    TextTooLarge { bytes: usize, maximum: usize },
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
    pub operation: UiClipboardOperation,
    pub text: String,
}

/// Clipboard mutation requested from a capability-gated platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiClipboardOperation {
    Copy,
    Cut,
}

/// Completion query containing only a non-password retained prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCompletionRequest {
    pub source: UiNodeId,
    pub prefix: String,
}

/// Private-value text editing state owned by the text boundary.
///
/// Password values are never exposed by a public accessor. Runtime frames may
/// observe only redacted snapshots and rendered masks.
#[derive(Clone, Debug)]
pub struct UiTextInputState {
    value: String,
    selection: UiTextSelection,
    preedit: Option<(String, Option<(usize, usize)>)>,
    password: bool,
    undo: UiTextHistory,
    redo: UiTextHistory,
}

#[derive(Clone, Debug)]
struct UiTextEditState {
    value: String,
    selection: UiTextSelection,
}

#[derive(Clone, Debug, Default)]
struct UiTextHistory {
    states: VecDeque<UiTextEditState>,
    bytes: usize,
}

impl UiTextHistory {
    fn push(&mut self, state: UiTextEditState) {
        let state_bytes = state.value.len();
        while self.states.len() >= MAX_RETAINED_NODES
            || self.bytes.saturating_add(state_bytes) > MAX_TEXT_BYTES
        {
            let Some(removed) = self.states.pop_front() else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(removed.value.len());
        }
        self.bytes = self.bytes.saturating_add(state_bytes);
        self.states.push_back(state);
    }

    fn pop(&mut self) -> Option<UiTextEditState> {
        let state = self.states.pop_back()?;
        self.bytes = self.bytes.saturating_sub(state.value.len());
        Some(state)
    }

    fn clear(&mut self) {
        self.states.clear();
        self.bytes = 0;
    }
}

impl UiTextInputState {
    /// Creates an empty retained editing state for a control whose authoritative
    /// initial value is intentionally absent.
    #[must_use]
    pub fn empty(password: bool) -> Self {
        Self {
            value: String::new(),
            selection: UiTextSelection::default(),
            preedit: None,
            password,
            undo: UiTextHistory::default(),
            redo: UiTextHistory::default(),
        }
    }

    /// Creates a bounded text-input state without silently retaining or
    /// discarding an oversized initial value.
    ///
    /// Password initial values retain the existing redaction policy and are
    /// discarded rather than stored.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextInputError::InitialValueTooLong`] when a non-password
    /// initial value exceeds [`MAX_TEXT_BYTES`].
    pub fn new(initial_value: impl AsRef<str>, password: bool) -> Result<Self, UiTextInputError> {
        let initial_value = initial_value.as_ref();
        if password {
            return Ok(Self::empty(true));
        }
        if initial_value.len() > MAX_TEXT_BYTES {
            return Err(UiTextInputError::InitialValueTooLong);
        }
        Ok(Self {
            value: initial_value.to_owned(),
            selection: UiTextSelection::default(),
            preedit: None,
            password,
            undo: UiTextHistory::default(),
            redo: UiTextHistory::default(),
        })
    }

    #[must_use]
    pub const fn is_password(&self) -> bool {
        self.password
    }

    #[must_use]
    pub fn value(&self) -> Option<&str> {
        (!self.password).then_some(self.value.as_str())
    }

    #[must_use]
    pub fn rendered_text(&self) -> String {
        if self.password {
            "•".repeat(grapheme_count(&self.value))
        } else {
            self.value.clone()
        }
    }

    #[must_use]
    pub fn snapshot(&self, node: UiNodeId) -> UiTextInputSnapshot {
        UiTextInputSnapshot {
            node,
            selection: self.selection,
            grapheme_count: grapheme_count(&self.value),
            password: self.password,
            has_preedit: self.preedit.is_some(),
        }
    }

    /// Replaces the current grapheme selection within the shared text bound.
    pub fn commit(&mut self, replacement: &str) -> bool {
        let before = self.edit_state();
        if !self.replace_selection(replacement) {
            return false;
        }
        if self.value != before.value {
            self.record_edit(before);
        }
        true
    }

    fn replace_selection(&mut self, replacement: &str) -> bool {
        let selection = clamp_selection(self.selection, grapheme_count(&self.value));
        let start = byte_index_at_grapheme(&self.value, selection.start());
        let end = byte_index_at_grapheme(&self.value, selection.end());
        let retained_bytes = self.value.len().saturating_sub(end.saturating_sub(start));
        if replacement.len() > MAX_TEXT_BYTES.saturating_sub(retained_bytes) {
            return false;
        }
        self.value.replace_range(start..end, replacement);
        self.selection = UiTextSelection::cursor(selection.start() + grapheme_count(replacement));
        self.preedit = None;
        true
    }

    /// Stores bounded IME composition and a UTF-8 byte-range cursor.
    ///
    /// # Errors
    ///
    /// Rejects composition text above the shared text bound or a cursor range
    /// that is reversed or outside the supplied composition.
    pub fn set_preedit(
        &mut self,
        text: String,
        cursor: Option<(usize, usize)>,
    ) -> Result<(), UiPreeditError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(UiPreeditError::TooLong);
        }
        if cursor.is_some_and(|(start, end)| {
            start > end
                || end > text.len()
                || !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
        }) {
            return Err(UiPreeditError::InvalidCursor);
        }
        self.preedit = Some((text, cursor));
        Ok(())
    }

    pub fn cancel_preedit(&mut self) {
        self.preedit = None;
    }

    pub fn move_cursor(&mut self, direction: UiTextCursorDirection, extend_selection: bool) {
        let count = grapheme_count(&self.value);
        let selection = clamp_selection(self.selection, count);
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
        self.selection = if extend_selection {
            UiTextSelection {
                anchor: selection.anchor,
                focus: destination,
            }
        } else {
            UiTextSelection::cursor(destination)
        };
    }

    pub fn delete(&mut self, backward: bool) {
        let before = self.edit_state();
        let count = grapheme_count(&self.value);
        let selection = clamp_selection(self.selection, count);
        if selection.is_collapsed() {
            self.selection = if backward {
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
            self.selection = selection;
        }
        if !self.selection.is_collapsed() {
            let _ = self.replace_selection("");
            if self.value != before.value {
                self.record_edit(before);
            }
        }
        self.preedit = None;
    }

    pub fn select_all(&mut self) {
        self.selection = UiTextSelection {
            anchor: 0,
            focus: grapheme_count(&self.value),
        };
    }

    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        if self.password {
            return None;
        }
        let selection = clamp_selection(self.selection, grapheme_count(&self.value));
        if selection.is_collapsed() {
            return None;
        }
        let start = byte_index_at_grapheme(&self.value, selection.start());
        let end = byte_index_at_grapheme(&self.value, selection.end());
        self.value.get(start..end)
    }

    /// Returns and removes a non-password selection for a capability-gated cut.
    pub fn cut_selected_text(&mut self) -> Option<String> {
        let selected = self.selected_text()?.to_owned();
        let before = self.edit_state();
        let _ = self.replace_selection("");
        self.record_edit(before);
        Some(selected)
    }

    /// Restores the preceding non-password edit state within the shared history bound.
    pub fn undo(&mut self) -> bool {
        if self.password {
            return false;
        }
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.edit_state());
        self.restore_edit(previous);
        true
    }

    /// Reapplies the next non-password edit state within the shared history bound.
    pub fn redo(&mut self) -> bool {
        if self.password {
            return false;
        }
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.edit_state());
        self.restore_edit(next);
        true
    }

    #[must_use]
    pub fn completion_prefix(&self) -> Option<String> {
        if self.password {
            return None;
        }
        let selection = clamp_selection(self.selection, grapheme_count(&self.value));
        let end = byte_index_at_grapheme(&self.value, selection.focus);
        self.value.get(..end).map(str::to_owned)
    }

    #[must_use]
    pub fn is_valid(&self, validation: UiTextValidation) -> bool {
        match validation {
            UiTextValidation::NonEmpty => !self.value.trim().is_empty(),
            UiTextValidation::Integer => self.value.parse::<i64>().is_ok(),
            UiTextValidation::Decimal => self.value.parse::<f64>().is_ok_and(f64::is_finite),
            UiTextValidation::MaximumGraphemes(maximum) => {
                grapheme_count(&self.value) <= usize::from(maximum)
            }
        }
    }

    #[must_use]
    pub fn preedit_text(&self) -> Option<&str> {
        (!self.password)
            .then_some(())
            .and_then(|()| self.preedit.as_ref().map(|(text, _)| text.as_str()))
    }

    pub fn reset_from_document(&mut self, value: &str) -> bool {
        if self.password || value.len() > MAX_TEXT_BYTES {
            return false;
        }
        value.clone_into(&mut self.value);
        self.selection = UiTextSelection::default();
        self.preedit = None;
        self.undo.clear();
        self.redo.clear();
        true
    }

    fn edit_state(&self) -> UiTextEditState {
        UiTextEditState {
            value: self.value.clone(),
            selection: self.selection,
        }
    }

    fn record_edit(&mut self, before: UiTextEditState) {
        if self.password {
            return;
        }
        self.undo.push(before);
        self.redo.clear();
    }

    fn restore_edit(&mut self, state: UiTextEditState) {
        self.value = state.value;
        self.selection = state.selection;
        self.preedit = None;
    }
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

/// Owned layout statistics, not glyphs or adapter structures.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiTextLayout {
    pub line_count: usize,
    pub glyph_count: usize,
    pub width: f32,
    pub height: f32,
    pub used_fallback_metrics: bool,
    pub used_fallback_font: bool,
    pub font_role: UiFontRole,
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

/// Bounded shaping-cache activity since the preceding activity drain.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTextCacheActivity {
    pub hits: u32,
    pub misses: u32,
    pub evictions: u32,
    /// Aggregate wall time spent resolving bounded shaping requests.
    pub shaping_nanoseconds: u64,
    /// Aggregate wall time spent rasterizing shaped glyphs.
    pub rasterization_nanoseconds: u64,
}

/// Current bounded shaping-cache occupancy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiTextCacheOccupancy {
    pub entries: usize,
    pub key_bytes: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct UiTextCacheKey {
    text: String,
    width_bits: u32,
    font_size_bits: u32,
    scale_factor_bits: u32,
    font_role: UiFontRole,
    font_weight: UiFontWeight,
}

#[derive(Debug, Default)]
struct UiTextShapeCache {
    entries: VecDeque<(UiTextCacheKey, UiShapedText)>,
    key_bytes: usize,
    activity: UiTextCacheActivity,
}

impl UiTextShapeCache {
    fn find(&mut self, key: &UiTextCacheKey) -> Option<UiShapedText> {
        let shaped = self
            .entries
            .iter()
            .find_map(|(candidate, shaped)| (candidate == key).then(|| shaped.clone()));
        if shaped.is_some() {
            self.activity.hits = self.activity.hits.saturating_add(1);
        } else {
            self.activity.misses = self.activity.misses.saturating_add(1);
        }
        shaped
    }

    fn insert(&mut self, key: UiTextCacheKey, shaped: UiShapedText) {
        let key_bytes = key.text.len();
        while self.entries.len() >= MAX_RETAINED_NODES
            || self.key_bytes.saturating_add(key_bytes) > MAX_TEXT_BYTES
        {
            let Some((removed, _)) = self.entries.pop_front() else {
                break;
            };
            self.key_bytes = self.key_bytes.saturating_sub(removed.text.len());
            self.activity.evictions = self.activity.evictions.saturating_add(1);
        }
        self.key_bytes = self.key_bytes.saturating_add(key_bytes);
        self.entries.push_back((key, shaped));
    }

    fn occupancy(&self) -> UiTextCacheOccupancy {
        UiTextCacheOccupancy {
            entries: self.entries.len(),
            key_bytes: self.key_bytes,
        }
    }

    fn take_activity(&mut self) -> UiTextCacheActivity {
        std::mem::take(&mut self.activity)
    }

    fn record_shaping(&mut self, duration: Duration) {
        self.activity.shaping_nanoseconds = self
            .activity
            .shaping_nanoseconds
            .saturating_add(duration_nanoseconds(duration));
    }

    fn record_rasterization(&mut self, duration: Duration) {
        self.activity.rasterization_nanoseconds = self
            .activity
            .rasterization_nanoseconds
            .saturating_add(duration_nanoseconds(duration));
    }
}

/// Private text adapter.  The public result is [`UiTextLayout`].
#[derive(Debug)]
pub struct UiTextEngine {
    fonts: FontSystem,
    swash: SwashCache,
    shape_cache: UiTextShapeCache,
}

#[derive(Clone, Debug)]
struct UiShapedText {
    layout: UiTextLayout,
    physical_glyphs: Vec<PhysicalGlyph>,
}

impl Default for UiTextEngine {
    fn default() -> Self {
        let mut fonts = FontSystem::new();
        fonts.db_mut().load_font_data(MONA_SANS.to_vec());
        fonts.db_mut().load_font_data(HUBOT_SANS.to_vec());
        fonts.db_mut().load_font_data(JETBRAINS_MONO.to_vec());
        fonts.db_mut().load_system_fonts();
        fonts.db_mut().set_sans_serif_family("Mona Sans VF");
        fonts.db_mut().set_monospace_family("JetBrains Mono");
        Self {
            fonts,
            swash: SwashCache::new(),
            shape_cache: UiTextShapeCache::default(),
        }
    }
}

impl UiTextEngine {
    /// Shapes and measures bounded text without constructing glyph bitmaps.
    ///
    /// Layout calls this before geometry resolution so intrinsic size can
    /// participate in reflow without paying a duplicate rasterization cost.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextLayoutError::TextTooLarge`] before invoking Cosmic Text
    /// when `text` exceeds [`MAX_TEXT_BYTES`].
    pub fn measure(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
    ) -> Result<UiTextLayout, UiTextLayoutError> {
        self.measure_with_weight(
            text,
            width,
            font_size,
            scale_factor,
            font_role,
            UiFontWeight::Normal,
        )
    }

    /// Shapes and measures text with a bounded authored emphasis weight.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextLayoutError::TextTooLarge`] before invoking the private
    /// text adapter when `text` exceeds [`MAX_TEXT_BYTES`].
    pub fn measure_with_weight(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
        font_weight: UiFontWeight,
    ) -> Result<UiTextLayout, UiTextLayoutError> {
        self.prepare(text, width, font_size, scale_factor, font_role, font_weight)
            .map(|shaped| shaped.layout)
    }

    /// Returns and clears shaping-cache activity accumulated since the previous
    /// call. Occupancy remains available separately and the cache is retained.
    pub fn take_cache_activity(&mut self) -> UiTextCacheActivity {
        self.shape_cache.take_activity()
    }

    /// Reports current shaping-cache occupancy without exposing adapter types.
    #[must_use]
    pub fn cache_occupancy(&self) -> UiTextCacheOccupancy {
        self.shape_cache.occupancy()
    }

    /// Shapes and rasterizes one bounded text value.
    ///
    /// The retained-document boundary already enforces this limit for normal
    /// frames. This check remains at the direct adapter boundary so callers
    /// cannot bypass that contract and allocate in the shaping engine first.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextLayoutError::TextTooLarge`] before invoking Cosmic Text
    /// when `text` exceeds [`MAX_TEXT_BYTES`].
    pub fn layout(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
    ) -> Result<UiTextOutput, UiTextLayoutError> {
        self.layout_with_weight(
            text,
            width,
            font_size,
            scale_factor,
            font_role,
            UiFontWeight::Normal,
        )
    }

    /// Shapes and rasterizes text with a bounded authored emphasis weight.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextLayoutError::TextTooLarge`] before invoking the private
    /// text adapter when `text` exceeds [`MAX_TEXT_BYTES`].
    pub fn layout_with_weight(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
        font_weight: UiFontWeight,
    ) -> Result<UiTextOutput, UiTextLayoutError> {
        let shaped = self.prepare(text, width, font_size, scale_factor, font_role, font_weight)?;
        let rasterization_started = Instant::now();
        let mut raster = UiTextRaster::default();
        let mut raster_bytes = 0_usize;
        for glyph in shaped.physical_glyphs {
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
        self.shape_cache
            .record_rasterization(rasterization_started.elapsed());
        Ok(UiTextOutput {
            layout: shaped.layout,
            raster,
        })
    }

    /// Shapes one display-only value as exactly one line, adding an ellipsis
    /// at an extended-grapheme boundary when the available width cannot hold
    /// the full value.
    ///
    /// This is deliberately a presentation operation. Callers retain the full
    /// source and semantic value; the fitted text is only what a compact
    /// control draws in its bounded visual slot.
    ///
    /// # Errors
    ///
    /// Returns the same bounded text-layout error as [`Self::layout`] before
    /// shaping untrusted text.
    pub fn layout_single_line(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
    ) -> Result<UiFittedText, UiTextLayoutError> {
        self.layout_single_line_with_weight(
            text,
            width,
            font_size,
            scale_factor,
            font_role,
            UiFontWeight::Normal,
        )
    }

    /// Fits and rasterizes one bounded line with a bounded authored emphasis weight.
    ///
    /// # Errors
    ///
    /// Returns [`UiTextLayoutError::TextTooLarge`] before invoking the private
    /// text adapter when `text` exceeds [`MAX_TEXT_BYTES`].
    pub fn layout_single_line_with_weight(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
        font_weight: UiFontWeight,
    ) -> Result<UiFittedText, UiTextLayoutError> {
        let output =
            self.layout_with_weight(text, width, font_size, scale_factor, font_role, font_weight)?;
        if output.layout.line_count <= 1 {
            return Ok(UiFittedText {
                text: text.to_owned(),
                output,
            });
        }

        let ellipsis = self.layout_with_weight(
            DISPLAY_ELLIPSIS,
            width,
            font_size,
            scale_factor,
            font_role,
            font_weight,
        )?;
        if ellipsis.layout.line_count > 1 || ellipsis.layout.width > width.max(1.0) {
            return Ok(UiFittedText {
                text: String::new(),
                output: self.layout_with_weight(
                    "",
                    width,
                    font_size,
                    scale_factor,
                    font_role,
                    font_weight,
                )?,
            });
        }

        let graphemes = text.graphemes(true).collect::<Vec<_>>();
        let mut low = 0_usize;
        let mut high = graphemes.len();
        let mut best = UiFittedText {
            text: DISPLAY_ELLIPSIS.to_owned(),
            output: ellipsis,
        };
        while low < high {
            let midpoint = low + (high - low).div_ceil(2);
            let mut candidate = graphemes[..midpoint].concat();
            candidate.push_str(DISPLAY_ELLIPSIS);
            let candidate_output = self.layout_with_weight(
                &candidate,
                width,
                font_size,
                scale_factor,
                font_role,
                font_weight,
            )?;
            if candidate_output.layout.line_count <= 1 {
                low = midpoint;
                best = UiFittedText {
                    text: candidate,
                    output: candidate_output,
                };
            } else {
                high = midpoint.saturating_sub(1);
            }
        }
        Ok(best)
    }

    fn prepare(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
        font_weight: UiFontWeight,
    ) -> Result<UiShapedText, UiTextLayoutError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(UiTextLayoutError::TextTooLarge {
                bytes: text.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        let shaping_started = Instant::now();
        let scale_factor = sanitized_scale_factor(scale_factor);
        let key = UiTextCacheKey {
            text: text.to_owned(),
            width_bits: width.to_bits(),
            font_size_bits: font_size.to_bits(),
            scale_factor_bits: scale_factor.to_bits(),
            font_role,
            font_weight,
        };
        if let Some(shaped) = self.shape_cache.find(&key) {
            self.shape_cache.record_shaping(shaping_started.elapsed());
            return Ok(shaped);
        }
        let shaped = self.shape(text, width, font_size, scale_factor, font_role, font_weight);
        self.shape_cache.insert(key, shaped.clone());
        self.shape_cache.record_shaping(shaping_started.elapsed());
        Ok(shaped)
    }

    fn shape(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
        font_role: UiFontRole,
        font_weight: UiFontWeight,
    ) -> UiShapedText {
        let scale_factor = sanitized_scale_factor(scale_factor);
        let family = Family::Name(bundled_family(font_role));
        let expected_font = self.fonts.db().query(&fontdb::Query {
            families: &[family],
            ..fontdb::Query::default()
        });
        let metrics = Metrics::relative((font_size * scale_factor).max(1.0), 1.25);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        buffer.set_size(Some((width * scale_factor).max(1.0)), None);
        buffer.set_text(
            text,
            &Attrs::new()
                .family(family)
                .weight(cosmic_font_weight(font_weight)),
            Shaping::Advanced,
            None,
        );
        let mut line_count = 0;
        let mut glyph_count = 0;
        let mut observed_width = 0.0_f32;
        let mut height = 0.0_f32;
        let mut used_fallback_font = false;
        let mut physical_glyphs = Vec::new();
        {
            let mut borrowed = buffer.borrow_with(&mut self.fonts);
            for run in borrowed.layout_runs() {
                line_count += 1;
                glyph_count += run.glyphs.len();
                observed_width = observed_width.max(run.line_w);
                height += run.line_height;
                used_fallback_font |= run
                    .glyphs
                    .iter()
                    .any(|glyph| Some(glyph.font_id) != expected_font);
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
            used_fallback_font,
            font_role,
        };
        UiShapedText {
            layout,
            physical_glyphs,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiTextOutput {
    pub layout: UiTextLayout,
    pub raster: UiTextRaster,
}

/// The visual text and raster output accepted for one compact display slot.
/// The original source value remains outside this display-only result.
#[derive(Clone, Debug, PartialEq)]
pub struct UiFittedText {
    pub text: String,
    pub output: UiTextOutput,
}

fn bounded_count_as_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_locked_fonts_and_weights_shape_without_platform_substitution_at_1x_and_2x() {
        let mut engine = UiTextEngine::default();
        for scale_factor in [1.0, 2.0] {
            for role in [
                UiFontRole::Interface,
                UiFontRole::Display,
                UiFontRole::Monospace,
            ] {
                for weight in [
                    UiFontWeight::Normal,
                    UiFontWeight::Medium,
                    UiFontWeight::Semibold,
                ] {
                    let output = engine
                        .layout_with_weight(
                            "Meridian 0123",
                            320.0,
                            14.0,
                            scale_factor,
                            role,
                            weight,
                        )
                        .expect("bounded fixture shapes");
                    assert_eq!(output.layout.font_role, role);
                    assert!(!output.layout.used_fallback_metrics);
                    assert!(
                        !output.layout.used_fallback_font,
                        "locked {role:?}/{weight:?} font unexpectedly substituted at {scale_factor}x: {:?}",
                        output.layout
                    );
                    assert!(output.layout.glyph_count > 0);
                    assert!(!output.raster.glyphs.is_empty());
                    assert!(
                        output
                            .raster
                            .glyphs
                            .iter()
                            .flat_map(|glyph| glyph.alpha.iter())
                            .any(|alpha| *alpha > 0 && *alpha < u8::MAX),
                        "locked {role:?}/{weight:?} font must retain antialiased glyph coverage at {scale_factor}x"
                    );
                }
            }
        }
    }

    #[test]
    fn layout_rejects_oversized_text_before_shaping() {
        let mut engine = UiTextEngine::default();
        let bytes = MAX_TEXT_BYTES + 1;
        assert!(matches!(
            engine.layout(&"x".repeat(bytes), 320.0, 14.0, 1.0, UiFontRole::Interface),
            Err(UiTextLayoutError::TextTooLarge {
                bytes: observed,
                maximum: MAX_TEXT_BYTES,
            }) if observed == bytes
        ));
    }

    #[test]
    fn single_line_layout_truncates_at_a_grapheme_boundary() {
        let mut engine = UiTextEngine::default();
        let fitted = engine
            .layout_single_line(
                "Meridian 👩‍🔬 workspace control label",
                92.0,
                14.0,
                1.0,
                UiFontRole::Interface,
            )
            .expect("bounded compact label shapes");

        assert_eq!(fitted.output.layout.line_count, 1);
        assert!(fitted.text.ends_with('…'));
        assert!(fitted.text.is_char_boundary(fitted.text.len()));
        assert!(fitted.text.len() <= "Meridian 👩‍🔬 workspace control label…".len());
    }

    #[test]
    fn measurement_matches_layout_without_constructing_a_second_raster() {
        let mut engine = UiTextEngine::default();
        let measured = engine
            .measure(
                "Intrinsic Meridian text",
                180.0,
                28.0,
                2.0,
                UiFontRole::Interface,
            )
            .expect("bounded fixture measures");
        let output = engine
            .layout(
                "Intrinsic Meridian text",
                180.0,
                28.0,
                2.0,
                UiFontRole::Interface,
            )
            .expect("bounded fixture lays out");
        assert_eq!(measured, output.layout);
        assert!(!output.raster.glyphs.is_empty());
        let activity = engine.take_cache_activity();
        assert_eq!(
            (activity.hits, activity.misses, activity.evictions),
            (1, 1, 0)
        );
        assert_eq!(
            engine.cache_occupancy(),
            UiTextCacheOccupancy {
                entries: 1,
                key_bytes: "Intrinsic Meridian text".len(),
            }
        );
    }

    #[test]
    fn shaping_cache_keys_every_geometry_font_and_weight_input() {
        let mut engine = UiTextEngine::default();
        let base = ("Meridian", 180.0, 28.0, 1.0, UiFontRole::Interface);
        engine
            .measure(base.0, base.1, base.2, base.3, base.4)
            .expect("base fixture shapes");
        let _ = engine.take_cache_activity();
        for (text, width, font_size, scale, role) in [
            ("Changed", base.1, base.2, base.3, base.4),
            (base.0, 181.0, base.2, base.3, base.4),
            (base.0, base.1, 29.0, base.3, base.4),
            (base.0, base.1, base.2, 2.0, base.4),
            (base.0, base.1, base.2, base.3, UiFontRole::Display),
        ] {
            engine
                .measure(text, width, font_size, scale, role)
                .expect("changed fixture shapes");
        }
        engine
            .measure_with_weight(
                base.0,
                base.1,
                base.2,
                base.3,
                base.4,
                UiFontWeight::Semibold,
            )
            .expect("weighted fixture shapes");
        let activity = engine.take_cache_activity();
        assert_eq!(
            (activity.hits, activity.misses, activity.evictions),
            (0, 6, 0)
        );
        assert_eq!(activity.rasterization_nanoseconds, 0);
    }

    #[test]
    fn cache_activity_accumulates_saturating_phase_durations() {
        let mut cache = UiTextShapeCache::default();
        cache.record_shaping(Duration::from_nanos(7));
        cache.record_shaping(Duration::from_nanos(11));
        cache.record_rasterization(Duration::from_nanos(13));

        assert_eq!(
            cache.take_activity(),
            UiTextCacheActivity {
                hits: 0,
                misses: 0,
                evictions: 0,
                shaping_nanoseconds: 18,
                rasterization_nanoseconds: 13,
            }
        );
        assert_eq!(cache.take_activity(), UiTextCacheActivity::default());
    }

    #[test]
    fn shaping_cache_evicts_fifo_within_shared_count_and_byte_bounds() {
        let mut cache = UiTextShapeCache::default();
        let shaped = UiShapedText {
            layout: UiTextLayout::default(),
            physical_glyphs: Vec::new(),
        };
        let key = |text: String, width: f32| UiTextCacheKey {
            text,
            width_bits: width.to_bits(),
            font_size_bits: 14.0_f32.to_bits(),
            scale_factor_bits: 1.0_f32.to_bits(),
            font_role: UiFontRole::Interface,
            font_weight: UiFontWeight::Normal,
        };
        let first_bytes = MAX_TEXT_BYTES / 2 + 1;
        cache.insert(key("a".repeat(first_bytes), 100.0), shaped.clone());
        cache.insert(key("b".repeat(first_bytes), 100.0), shaped.clone());
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.key_bytes, first_bytes);
        assert_eq!(cache.activity.evictions, 1);

        cache.entries.clear();
        cache.key_bytes = 0;
        cache.activity = UiTextCacheActivity::default();
        for index in 0..=MAX_RETAINED_NODES {
            let width = f32::from(u16::try_from(index).expect("retained-node bound fits u16"));
            cache.insert(key(String::new(), width), shaped.clone());
        }
        assert_eq!(cache.entries.len(), MAX_RETAINED_NODES);
        assert_eq!(cache.key_bytes, 0);
        assert_eq!(cache.activity.evictions, 1);
    }

    #[test]
    fn oversized_text_rejection_preserves_shaping_cache_state() {
        let mut engine = UiTextEngine::default();
        engine
            .measure("accepted", 180.0, 28.0, 1.0, UiFontRole::Interface)
            .expect("bounded fixture shapes");
        let _ = engine.take_cache_activity();
        let occupancy = engine.cache_occupancy();
        let text = "x".repeat(MAX_TEXT_BYTES + 1);
        assert!(matches!(
            engine.measure(&text, 180.0, 28.0, 1.0, UiFontRole::Interface),
            Err(UiTextLayoutError::TextTooLarge { .. })
        ));
        assert_eq!(engine.cache_occupancy(), occupancy);
        assert_eq!(engine.take_cache_activity(), UiTextCacheActivity::default());
    }

    #[test]
    fn completion_prefix_stops_at_the_grapheme_cursor() {
        let mut state = UiTextInputState::new("alpha beta", false).expect("bounded fixture");
        state.move_cursor(UiTextCursorDirection::End, false);
        state.move_cursor(UiTextCursorDirection::Backward, false);
        state.move_cursor(UiTextCursorDirection::Backward, false);
        state.move_cursor(UiTextCursorDirection::Backward, false);
        state.move_cursor(UiTextCursorDirection::Backward, false);
        assert_eq!(state.completion_prefix().as_deref(), Some("alpha "));
    }

    #[test]
    fn preedit_cursor_uses_valid_utf8_byte_boundaries() {
        let mut state = UiTextInputState::new("", false).expect("bounded fixture");
        assert_eq!(state.set_preedit("啊b".to_owned(), Some((3, 3))), Ok(()));
        assert_eq!(
            state.set_preedit("啊b".to_owned(), Some((1, 1))),
            Err(UiPreeditError::InvalidCursor)
        );
        assert_eq!(state.preedit_text(), Some("啊b"));
    }

    #[test]
    fn public_constructor_rejects_oversized_initial_values_and_redacts_passwords() {
        assert!(matches!(
            UiTextInputState::new("x".repeat(MAX_TEXT_BYTES + 1), false),
            Err(UiTextInputError::InitialValueTooLong)
        ));
        assert_eq!(
            UiTextInputState::new("secret", true)
                .expect("password initial values are discarded")
                .value(),
            None
        );
    }

    #[test]
    fn cut_and_validation_remain_grapheme_safe_and_password_private() {
        let mut state = UiTextInputState::new("12👩‍🔬", false).expect("bounded fixture");
        state.move_cursor(UiTextCursorDirection::End, false);
        state.move_cursor(UiTextCursorDirection::Backward, true);
        assert_eq!(state.cut_selected_text().as_deref(), Some("👩‍🔬"));
        assert_eq!(state.value(), Some("12"));
        assert!(state.is_valid(UiTextValidation::Integer));
        assert!(!state.is_valid(UiTextValidation::MaximumGraphemes(1)));

        let mut password = UiTextInputState::new("ignored", true).expect("bounded fixture");
        assert!(password.commit("secret"));
        password.select_all();
        assert_eq!(password.selected_text(), None);
        assert_eq!(password.cut_selected_text(), None);
        assert_eq!(password.completion_prefix(), None);
    }

    #[test]
    fn bounded_text_history_restores_value_and_selection_without_password_snapshots() {
        let mut state = UiTextInputState::new("one", false).expect("bounded fixture");
        state.select_all();
        assert!(state.commit("two"));
        assert_eq!(state.value(), Some("two"));
        assert!(state.undo());
        assert_eq!(state.value(), Some("one"));
        assert_eq!(state.snapshot(UiNodeId::new(1)).selection.end(), 3);
        assert!(state.redo());
        assert_eq!(state.value(), Some("two"));

        let mut password = UiTextInputState::new("", true).expect("bounded fixture");
        assert!(password.commit("secret"));
        assert!(!password.undo());
        assert!(!password.redo());
    }

    #[test]
    fn text_history_never_exceeds_shared_count_or_byte_bounds() {
        let mut state = UiTextInputState::new("", false).expect("bounded fixture");
        for _ in 0..1_000 {
            assert!(state.commit("x"));
        }
        assert!(state.undo.states.len() <= MAX_RETAINED_NODES);
        assert!(state.undo.bytes <= MAX_TEXT_BYTES);
    }
}
