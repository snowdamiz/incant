//! Bounded runtime text editing, independent of widget appearance and OS input.
//!
//! Offsets are UTF-8 bytes. Committed selections use extended grapheme boundaries;
//! IME cursors use scalar boundaries within the preedit string, as desktop input
//! methods can select part of a grapheme. Movement is in logical string order,
//! not visual bidi order. A platform adapter must route IME-owned navigation to
//! the input method and cancel composition before changing ordinary selection.
use crate::{MAX_TEXT_BYTES, TextError};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub focus: usize,
}

impl Selection {
    pub const fn caret(byte: usize) -> Self {
        Self {
            anchor: byte,
            focus: byte,
        }
    }

    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.focus)..self.anchor.max(self.focus)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    PreviousGrapheme,
    NextGrapheme,
    Start,
    End,
}

/// All ranges except `cursor` refer to their named complete string.
pub struct Preedit<'a> {
    pub text: &'a str,
    /// UTF-8 byte range relative to `text`; None means hide the IME caret.
    pub cursor: Option<Range<usize>>,
    /// Range replaced in the unchanged committed text.
    pub replacement: Range<usize>,
    /// Preedit span within TextBuffer::display_text().
    pub display_range: Range<usize>,
}

struct Composition {
    display: String,
    replacement: Range<usize>,
    preedit: Range<usize>,
    cursor: Option<Range<usize>>,
}

/// Disposable game-widget state. This never edits an authoring document.
///
/// Committed and composed display strings each have the shaping stack's 16 KiB
/// byte bound. Reads borrow retained storage; only accepted text/preedit updates
/// allocate. Failed admission preserves text, selection, focus and composition.
/// Focus loss cancels preedit without implicitly committing it. OS activation,
/// keyboard delivery, clipboard and widget focus ownership belong to the caller.
pub struct TextBuffer {
    text: String,
    selection: Selection,
    composition: Option<Composition>,
    focused: bool,
}

impl TextBuffer {
    pub fn new(text: &str) -> Result<Self, TextError> {
        check_length(text.len())?;
        Ok(Self {
            text: text.into(),
            selection: Selection::caret(text.len()),
            composition: None,
            focused: false,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Shape this borrowed string to display an active composition. Persist only
    /// `text()`: preedit remains transient until an accepted commit.
    pub fn display_text(&self) -> &str {
        self.composition
            .as_ref()
            .map_or(&self.text, |c| c.display.as_str())
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn display_selection(&self) -> Option<Selection> {
        self.composition.as_ref().map_or(Some(self.selection), |c| {
            c.cursor.as_ref().map(|range| Selection {
                anchor: c.preedit.start + range.start,
                focus: c.preedit.start + range.end,
            })
        })
    }

    pub fn preedit(&self) -> Option<Preedit<'_>> {
        self.composition.as_ref().map(|c| Preedit {
            text: &c.display[c.preedit.clone()],
            cursor: c.cursor.clone(),
            replacement: c.replacement.clone(),
            display_range: c.preedit.clone(),
        })
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn set_focused(&mut self, focused: bool) {
        if !focused {
            self.cancel_preedit();
        }
        self.focused = focused;
    }

    /// Programmatic replacement also cancels composition and moves the caret to
    /// the end. It is allowed while unfocused; invalid replacement changes nothing.
    pub fn set_text(&mut self, text: &str) -> Result<(), TextError> {
        check_length(text.len())?;
        let candidate = text.to_owned();
        self.text = candidate;
        self.selection = Selection::caret(text.len());
        self.composition = None;
        Ok(())
    }

    /// Programmatic selection is allowed while unfocused and cancels composition.
    /// Byte offsets inside a grapheme are refused, never silently rounded.
    pub fn select(&mut self, selection: Selection) -> Result<(), TextError> {
        if !boundary(&self.text, selection.anchor) || !boundary(&self.text, selection.focus) {
            return Err(TextError::Invalid("selection must use grapheme boundaries"));
        }
        self.selection = selection;
        self.composition = None;
        Ok(())
    }

    /// Replace the selected committed range with transient IME text. Repeated
    /// preedits keep the same replacement range. Empty preedit clears it; a
    /// subsequent commit still replaces the unchanged ordinary selection.
    pub fn set_preedit(
        &mut self,
        text: &str,
        cursor: Option<Range<usize>>,
    ) -> Result<(), TextError> {
        self.require_focus()?;
        check_length(text.len())?;
        if cursor.as_ref().is_some_and(|c| {
            c.start > c.end || !text.is_char_boundary(c.start) || !text.is_char_boundary(c.end)
        }) {
            return Err(TextError::Invalid(
                "preedit cursor must use UTF-8 boundaries",
            ));
        }
        if text.is_empty() {
            self.cancel_preedit();
            return Ok(());
        }
        if self
            .preedit()
            .is_some_and(|old| old.text == text && old.cursor == cursor)
        {
            return Ok(());
        }
        let replacement = self.selection.range();
        let display = replace(&self.text, replacement.clone(), text)?;
        let preedit = replacement.start..replacement.start + text.len();
        self.composition = Some(Composition {
            display,
            replacement,
            preedit,
            cursor,
        });
        Ok(())
    }

    pub fn cancel_preedit(&mut self) {
        self.composition = None;
    }

    /// Ordinary text insertion and IME commit share one replacement path. Empty
    /// commit deletes a selection; it differs from cancelling an empty preedit.
    /// No normalization/filtering is implicit, including for line endings.
    pub fn commit(&mut self, text: &str) -> Result<(), TextError> {
        self.require_focus()?;
        self.replace_selection(text)
    }

    pub fn backspace(&mut self) -> Result<(), TextError> {
        self.require_ordinary_input()?;
        let mut range = self.selection.range();
        if range.is_empty() {
            range.start = previous(&self.text, range.start);
        }
        self.replace_range(range, "")
    }

    pub fn delete_forward(&mut self) -> Result<(), TextError> {
        self.require_ordinary_input()?;
        let mut range = self.selection.range();
        if range.is_empty() {
            range.end = next(&self.text, range.end);
        }
        self.replace_range(range, "")
    }

    /// Previous/next collapse a nonempty selection toward that logical edge
    /// unless extending. Start/end refer to the entire buffer, not a visual line.
    pub fn move_selection(&mut self, movement: Movement, extend: bool) -> Result<(), TextError> {
        self.require_ordinary_input()?;
        let selection = self.selection;
        let selected = selection.anchor != selection.focus;
        let focus = match movement {
            Movement::PreviousGrapheme if selected && !extend => selection.range().start,
            Movement::NextGrapheme if selected && !extend => selection.range().end,
            Movement::PreviousGrapheme => previous(&self.text, selection.focus),
            Movement::NextGrapheme => next(&self.text, selection.focus),
            Movement::Start => 0,
            Movement::End => self.text.len(),
        };
        self.selection = Selection {
            anchor: if extend { selection.anchor } else { focus },
            focus,
        };
        Ok(())
    }

    fn require_focus(&self) -> Result<(), TextError> {
        if self.focused {
            Ok(())
        } else {
            Err(TextError::Invalid("text input requires focus"))
        }
    }

    fn require_ordinary_input(&self) -> Result<(), TextError> {
        self.require_focus()?;
        if self.composition.is_some() {
            Err(TextError::Invalid("input method owns active composition"))
        } else {
            Ok(())
        }
    }

    fn replace_selection(&mut self, text: &str) -> Result<(), TextError> {
        self.replace_range(self.selection.range(), text)
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) -> Result<(), TextError> {
        let candidate = replace(&self.text, range.clone(), text)?;
        // Inserting a combining mark/ZWJ or joining formerly separated text can
        // merge surrounding graphemes. Keep the caret valid in the new string.
        let intended = range.start + text.len();
        let caret = candidate
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|&i| i >= intended)
            .unwrap_or(candidate.len());
        self.text = candidate;
        self.selection = Selection::caret(caret);
        self.composition = None;
        Ok(())
    }
}

fn check_length(length: usize) -> Result<(), TextError> {
    if length > MAX_TEXT_BYTES {
        Err(TextError::Limit("text bytes"))
    } else {
        Ok(())
    }
}

fn replace(text: &str, range: Range<usize>, replacement: &str) -> Result<String, TextError> {
    // Validate before allocating/copying, including externally supplied input.
    check_length(replacement.len())?;
    let length = text.len() - range.len() + replacement.len();
    check_length(length)?;
    let mut candidate = String::with_capacity(length);
    candidate.push_str(&text[..range.start]);
    candidate.push_str(replacement);
    candidate.push_str(&text[range.end..]);
    Ok(candidate)
}

fn boundary(text: &str, byte: usize) -> bool {
    byte == text.len() || text.grapheme_indices(true).any(|(i, _)| i == byte)
}

fn previous(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|&i| i < byte)
        .last()
        .unwrap_or(0)
}

fn next(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|&i| i > byte)
        .unwrap_or(text.len())
}
