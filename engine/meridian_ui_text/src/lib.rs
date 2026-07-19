//! Meridian-owned text contracts and the private Cosmic Text adapter.

use std::collections::VecDeque;

use cosmic_text::{
    fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
};
use meridian_ui_core::{
    sanitized_scale_factor, UiFontRole, UiNodeId, UiPoint, UiTextValidation, MAX_RETAINED_NODES,
    MAX_TEXT_BYTES,
};
use unicode_segmentation::UnicodeSegmentation;

/// Aggregate alpha-mask budget accepted for one immutable UI frame.
pub const MAX_GLYPH_RASTER_BYTES: usize = 1024 * 1024;

const MONA_SANS: &[u8] = include_bytes!("../assets/fonts/MonaSansVF.ttf");
const HUBOT_SANS: &[u8] = include_bytes!("../assets/fonts/HubotSansVF.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoVF.ttf");

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

/// Private text adapter.  The public result is [`UiTextLayout`].
#[derive(Debug)]
pub struct UiTextEngine {
    fonts: FontSystem,
    swash: SwashCache,
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
        }
    }
}

impl UiTextEngine {
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
        if text.len() > MAX_TEXT_BYTES {
            return Err(UiTextLayoutError::TextTooLarge {
                bytes: text.len(),
                maximum: MAX_TEXT_BYTES,
            });
        }
        let scale_factor = sanitized_scale_factor(scale_factor);
        let family = Family::Name(bundled_family(font_role));
        let expected_font = self.fonts.db().query(&fontdb::Query {
            families: &[family],
            ..fontdb::Query::default()
        });
        let metrics = Metrics::relative((font_size * scale_factor).max(1.0), 1.25);
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        buffer.set_size(Some((width * scale_factor).max(1.0)), None);
        buffer.set_text(text, &Attrs::new().family(family), Shaping::Advanced, None);
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
        Ok(UiTextOutput { layout, raster })
    }
}

pub struct UiTextOutput {
    pub layout: UiTextLayout,
    pub raster: UiTextRaster,
}

fn bounded_count_as_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

#[allow(clippy::cast_precision_loss)]
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_locked_fonts_shape_without_platform_substitution() {
        let mut engine = UiTextEngine::default();
        for role in [
            UiFontRole::Interface,
            UiFontRole::Display,
            UiFontRole::Monospace,
        ] {
            let output = engine
                .layout("Meridian 0123", 320.0, 14.0, 2.0, role)
                .expect("bounded fixture shapes");
            assert_eq!(output.layout.font_role, role);
            assert!(!output.layout.used_fallback_metrics);
            assert!(
                !output.layout.used_fallback_font,
                "locked {role:?} font unexpectedly substituted: {:?}",
                output.layout
            );
            assert!(output.layout.glyph_count > 0);
            assert!(!output.raster.glyphs.is_empty());
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
