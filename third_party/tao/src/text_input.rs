// Copyright 2026 Incant contributors
// SPDX-License-Identifier: Apache-2.0
//! Opt-in managed desktop text delivery. Session IDs are stamped by native
//! callbacks before events enter Tao's queue, never assigned during app delivery.
use std::{fmt, ops::Range};

/// Maximum UTF-8 payload in one managed native text event.
pub const MAX_TEXT_INPUT_BYTES: usize = 16 * 1024;

/// Native text operations for one explicitly enabled input owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
  /// Cursor/selection offsets are UTF-8 byte boundaries relative to `text`.
  Preedit {
    text: String,
    cursor: Option<Range<usize>>,
  },
  /// Replaces the active composition, or the current text selection.
  Commit(String),
  /// Cancels composition without deleting committed text.
  Cancel,
  /// A native operation cannot be represented without guessing or truncation.
  Refused(TextInputError),
}

/// Typed managed input failure. It contains no user text or OS diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextInputError {
  Session,
  TooLong,
  Range,
  Replacement,
  Unsupported,
}
impl fmt::Display for TextInputError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Session => "text input session must increase monotonically",
      Self::TooLong => "native text exceeds the managed byte limit",
      Self::Range => "native text cursor is not a representable UTF-8 range",
      Self::Replacement => "native replacement range is unsupported",
      Self::Unsupported => "managed text input is unsupported on this platform",
    })
  }
}
impl std::error::Error for TextInputError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Session {
  pub serial: u64,
  pub enabled: bool,
}
#[derive(Debug, Default)]
pub(crate) struct State {
  pub session: Option<Session>, // None preserves the original Tao behavior.
  pub composition: Option<u64>,
}
impl State {
  pub fn check(&self, serial: u64, enabled: bool) -> Result<bool, TextInputError> {
    if serial == 0
      || self
        .session
        .is_some_and(|s| serial < s.serial || (serial == s.serial && enabled != s.enabled))
    {
      return Err(TextInputError::Session);
    }
    Ok(self.session != Some(Session { serial, enabled }))
  }
  pub fn active(&self) -> Option<u64> {
    self.session.filter(|s| s.enabled).map(|s| s.serial)
  }
}

/// Translate an exact UTF-16 scalar boundary, rejecting the middle of a pair.
pub(crate) fn utf16_offset(text: &str, units: usize) -> Option<usize> {
  let mut count = 0usize;
  for (byte, ch) in text.char_indices() {
    if count == units {
      return Some(byte);
    }
    count = count.checked_add(ch.len_utf16())?;
    if count > units {
      return None;
    }
  }
  (count == units).then_some(text.len())
}
pub(crate) fn utf16_range(
  text: &str,
  start: usize,
  length: usize,
) -> Result<Range<usize>, TextInputError> {
  let end = start.checked_add(length).ok_or(TextInputError::Range)?;
  Ok(
    utf16_offset(text, start).ok_or(TextInputError::Range)?
      ..utf16_offset(text, end).ok_or(TextInputError::Range)?,
  )
}
/// Checked native UTF-16 conversion shared by Cocoa and IMM. This allocation
/// is bounded before it happens and invalid surrogate sequences never substitute.
pub(crate) fn utf16_text(units: &[u16]) -> Result<String, TextInputError> {
  if units.len() > MAX_TEXT_INPUT_BYTES {
    return Err(TextInputError::TooLong);
  }
  checked(String::from_utf16(units).map_err(|_| TextInputError::Range)?)
}

pub(crate) fn checked(text: String) -> Result<String, TextInputError> {
  if text.len() > MAX_TEXT_INPUT_BYTES {
    Err(TextInputError::TooLong)
  } else {
    Ok(text)
  }
}

/// Cocoa's unmarkText accepts marked content. Keep this exact state machine
/// separate from Objective-C plumbing so teardown/acceptance sequences are
/// testable without creating a native window or input method.
#[cfg(any(target_os = "macos", test))]
pub(crate) struct CocoaComposition {
  active: bool,
  marked: Option<String>,
}
#[cfg(any(target_os = "macos", test))]
impl CocoaComposition {
  pub fn new() -> Self {
    Self {
      active: true,
      marked: None,
    }
  }
  pub fn active(&self) -> bool {
    self.active
  }
  pub fn marked_units(&self) -> usize {
    self
      .marked
      .as_ref()
      .map_or(0, |text| text.encode_utf16().count())
  }
  pub fn preedit(
    &mut self,
    text: String,
    cursor: Option<Range<usize>>,
  ) -> Result<Option<TextInputEvent>, TextInputError> {
    if !self.active {
      return Ok(None);
    }
    let text = checked(text)?;
    if cursor.as_ref().is_some_and(|r| {
      r.start > r.end
        || r.end > text.len()
        || !text.is_char_boundary(r.start)
        || !text.is_char_boundary(r.end)
    }) {
      return Err(TextInputError::Range);
    }
    // Both retained and queued strings independently respect the byte ceiling.
    self.marked = (!text.is_empty()).then(|| text.clone());
    Ok(Some(TextInputEvent::Preedit { text, cursor }))
  }
  pub fn insert(&mut self, text: String) -> Result<Option<TextInputEvent>, TextInputError> {
    if !self.active {
      return Ok(None);
    }
    let text = checked(text)?;
    self.marked = None;
    Ok(Some(TextInputEvent::Commit(text)))
  }
  pub fn unmark(&mut self) -> Option<TextInputEvent> {
    if !self.active {
      return None;
    }
    self.marked.take().map(TextInputEvent::Commit)
  }
  pub fn retire(&mut self) {
    self.active = false;
    self.marked = None;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn native_utf16_selection_is_exact_and_never_splits_a_surrogate() {
    let text = "a𝄞中文";
    assert_eq!(utf16_range(text, 1, 2), Ok(1..5));
    assert_eq!(utf16_range(text, 3, 2), Ok(5..11));
    assert_eq!(utf16_range(text, 2, 0), Err(TextInputError::Range));
    assert_eq!(utf16_range(text, usize::MAX, 1), Err(TextInputError::Range));
    assert_eq!(utf16_range(text, 6, 0), Err(TextInputError::Range));
  }
  #[test]
  fn mode_is_opt_in_and_session_changes_cannot_alias_an_old_owner() {
    let mut state = State::default();
    assert!(state.session.is_none());
    assert_eq!(state.check(0, true), Err(TextInputError::Session));
    assert_eq!(state.check(1, true), Ok(true));
    state.session = Some(Session {
      serial: 1,
      enabled: true,
    });
    assert_eq!(state.active(), Some(1));
    assert_eq!(state.check(1, true), Ok(false));
    assert_eq!(state.check(1, false), Err(TextInputError::Session));
    assert_eq!(state.check(2, false), Ok(true));
    state.session = Some(Session {
      serial: 2,
      enabled: false,
    });
    assert_eq!(state.active(), None);
    assert_eq!(state.check(1, true), Err(TextInputError::Session));
  }
  #[test]
  fn exact_byte_ceiling_never_truncates_or_substitutes_text() {
    assert!(checked("x".repeat(MAX_TEXT_INPUT_BYTES)).is_ok());
    assert_eq!(
      checked("x".repeat(MAX_TEXT_INPUT_BYTES + 1)),
      Err(TextInputError::TooLong)
    );
  }
  #[test]
  fn native_utf16_decode_is_bounded_and_refuses_incomplete_surrogates() {
    assert_eq!(
      utf16_text(&[0x61, 0xd834, 0xdd1e, 0x4e2d]),
      Ok("a𝄞中".into())
    );
    for bad in [&[0xd800][..], &[0xdc00], &[0xd800, 0x61]] {
      assert_eq!(utf16_text(bad), Err(TextInputError::Range));
    }
    assert!(utf16_text(&vec![0x61; MAX_TEXT_INPUT_BYTES]).is_ok());
    assert_eq!(
      utf16_text(&vec![0x61; MAX_TEXT_INPUT_BYTES + 1]),
      Err(TextInputError::TooLong)
    );
    assert_eq!(
      utf16_text(&vec![0x4e2d; MAX_TEXT_INPUT_BYTES / 3 + 1]),
      Err(TextInputError::TooLong)
    );
  }
}

#[cfg(test)]
mod cocoa_composition_tests {
  use super::*;
  #[test]
  fn active_unmark_accepts_the_latest_marked_text_once() {
    let mut state = CocoaComposition::new();
    state.preedit("first".into(), None).unwrap();
    state.preedit("a𝄞中".into(), Some(1..5)).unwrap();
    assert_eq!(state.marked_units(), 4);
    assert_eq!(state.unmark(), Some(TextInputEvent::Commit("a𝄞中".into())));
    assert_eq!(state.unmark(), None);
    assert_eq!(state.marked_units(), 0);
  }
  #[test]
  fn explicit_insert_then_unmark_does_not_double_commit() {
    let mut state = CocoaComposition::new();
    state.preedit("marked".into(), None).unwrap();
    assert_eq!(
      state.insert("accepted".into()).unwrap(),
      Some(TextInputEvent::Commit("accepted".into()))
    );
    assert_eq!(state.unmark(), None);
  }
  #[test]
  fn retire_before_native_discard_never_commits_or_revives_content() {
    let mut state = CocoaComposition::new();
    state.preedit("abandoned".into(), None).unwrap();
    state.retire();
    assert!(!state.active());
    assert_eq!(state.unmark(), None);
    assert_eq!(state.insert("late".into()).unwrap(), None);
    assert_eq!(state.preedit("late".into(), None).unwrap(), None);
    assert_eq!(state.marked_units(), 0);
    let mut next = CocoaComposition::new();
    assert_eq!(next.unmark(), None);
    assert_eq!(
      next.insert("new".into()).unwrap(),
      Some(TextInputEvent::Commit("new".into()))
    );
  }
  #[test]
  fn rejected_marked_update_preserves_bounded_prior_content() {
    let mut state = CocoaComposition::new();
    state.preedit("中".into(), Some(0..3)).unwrap();
    assert_eq!(
      state.preedit("中".into(), Some(1..2)),
      Err(TextInputError::Range)
    );
    assert_eq!(
      state.preedit("x".repeat(MAX_TEXT_INPUT_BYTES + 1), None),
      Err(TextInputError::TooLong)
    );
    assert_eq!(state.unmark(), Some(TextInputEvent::Commit("中".into())));
    state.preedit(String::new(), Some(0..0)).unwrap();
    // Empty preedit cancels marking; unmark must not delete a selection.
    assert_eq!(state.unmark(), None);
    assert_eq!(
      state.insert(String::new()).unwrap(),
      Some(TextInputEvent::Commit(String::new()))
    );
    assert_eq!(state.unmark(), None);
  }
}
