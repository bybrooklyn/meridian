//! Meridian-owned text contracts and the private Cosmic Text adapter.

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use meridian_ui_core::{sanitized_scale_factor, UiNodeId, UiPoint, MAX_TEXT_BYTES};
use unicode_segmentation::UnicodeSegmentation;

/// Aggregate alpha-mask budget accepted for one immutable UI frame.
pub const MAX_GLYPH_RASTER_BYTES: usize = 1024 * 1024;

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
}

impl UiTextInputState {
    #[must_use]
    pub fn new(initial_value: impl Into<String>, password: bool) -> Self {
        Self {
            value: if password {
                String::new()
            } else {
                initial_value.into()
            },
            selection: UiTextSelection::default(),
            preedit: None,
            password,
        }
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

    /// Stores a bounded IME pre-edit without committing it to the value.
    pub fn set_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) -> bool {
        if text.len() > MAX_TEXT_BYTES {
            return false;
        }
        self.preedit = Some((text, cursor));
        true
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
            let _ = self.commit("");
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
        true
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
        fonts.db_mut().load_system_fonts();
        Self {
            fonts,
            swash: SwashCache::new(),
        }
    }
}

impl UiTextEngine {
    pub fn layout(
        &mut self,
        text: &str,
        width: f32,
        font_size: f32,
        scale_factor: f32,
    ) -> UiTextOutput {
        let scale_factor = sanitized_scale_factor(scale_factor);
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
