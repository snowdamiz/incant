// Copyright 2026 Incant contributors
// SPDX-License-Identifier: Apache-2.0
//! Managed IMM contexts. Only the window thread calls native context APIs; no
//! WindowState mutex is held across APIs which can synchronously reenter WndProc.
use super::window_state::WindowState;
use crate::text_input::{
  utf16_range, utf16_text, Session, State, TextInputError as Error, TextInputEvent as Event,
  MAX_TEXT_INPUT_BYTES,
};
use parking_lot::Mutex;
use windows::Win32::{
  Foundation::HWND,
  System::Threading::GetCurrentThreadId,
  UI::{Input::Ime::*, WindowsAndMessaging::*},
};
#[derive(Default)]
pub(super) struct Native {
  pub state: State,
  pub owned: Option<HIMC>,
  original: Option<HIMC>,
}
/// Opt-in API: synchronous and restricted to the owning native window thread.
pub fn set(
  hwnd: HWND,
  shared: &Mutex<WindowState>,
  serial: u64,
  enabled: bool,
) -> Result<(), Error> {
  // SAFETY: HWND belongs to this Window; native APIs below require this thread.
  unsafe {
    if GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId() {
      return Err(Error::Session);
    }
  }
  let old = {
    let mut window = shared.lock();
    if !window.text_input.state.check(serial, enabled)? {
      return Ok(());
    }
    window.text_input.state.session = Some(Session {
      serial,
      enabled: false,
    });
    window.text_input.state.composition = None;
    window.text_input.owned.take()
  };
  // Retire before installing new data. Old callback events already queued by
  // Tao retain the old serial. WM_IME messages are only change hints; any later
  // hint must pass the current HIMC and start-composition checks before reading.
  unsafe {
    let previous = ImmAssociateContext(hwnd, HIMC::default());
    {
      let mut window = shared.lock();
      if window.text_input.original.is_none() {
        window.text_input.original = Some(previous);
      }
    }
    if let Some(old) = old {
      let _ = ImmNotifyIME(old, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
      let _ = ImmDestroyContext(old);
    }
    // Retiring a context also retires its not-yet-dispatched change hints and
    // translated characters. Bound work; overload fails closed, not partially on.
    let mut message = MSG::default();
    for (first, last) in [
      (WM_IME_STARTCOMPOSITION, WM_IME_COMPOSITION),
      (WM_CHAR, WM_DEADCHAR),
      (WM_SYSCHAR, WM_SYSDEADCHAR),
      (WM_IME_CHAR, WM_IME_CHAR),
    ] {
      let mut count = 0;
      while PeekMessageW(&mut message, Some(hwnd), first, last, PM_REMOVE).as_bool() {
        count += 1;
        if count > 1024 {
          return Err(Error::TooLong);
        }
      }
    }
    if enabled {
      let context = ImmCreateContext();
      if context.0.is_null() {
        return Err(Error::Unsupported);
      }
      // Publish native ownership before association callbacks. The context has
      // no old composition, and no callback can see the previous context's data.
      {
        let mut window = shared.lock();
        window.text_input.owned = Some(context);
        window.text_input.state.session = Some(Session {
          serial,
          enabled: true,
        });
      }
      let _ = ImmAssociateContext(hwnd, context);
      let actual = ImmGetContext(hwnd);
      let valid = actual == context;
      let _ = ImmReleaseContext(hwnd, actual);
      if !valid {
        retire(hwnd, shared);
        return Err(Error::Unsupported);
      }
    } else {
      shared.lock().text_input.state.session = Some(Session {
        serial,
        enabled: false,
      });
    }
  }
  Ok(())
}
pub fn retire(hwnd: HWND, shared: &Mutex<WindowState>) {
  let (owned, original) = {
    let mut window = shared.lock();
    if let Some(s) = &mut window.text_input.state.session {
      s.enabled = false;
    }
    window.text_input.state.composition = None;
    (
      window.text_input.owned.take(),
      window.text_input.original.take(),
    )
  };
  // SAFETY: called on the window thread before destruction, no mutex held.
  unsafe {
    if let Some(context) = owned {
      let _ = ImmAssociateContext(hwnd, original.unwrap_or_default());
      let _ = ImmNotifyIME(context, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
      let _ = ImmDestroyContext(context);
    } else if let Some(original) = original {
      let _ = ImmAssociateContext(hwnd, original);
    }
  }
}
/// Validate the exact currently associated context before reading OS memory.
pub fn composition(hwnd: HWND, expected: HIMC, flags: u32) -> Result<[Option<Event>; 2], Error> {
  if flags & CS_INSERTCHAR != 0 {
    return Err(Error::Replacement);
  }
  unsafe {
    let current = ImmGetContext(hwnd);
    if current != expected {
      let _ = ImmReleaseContext(hwnd, current);
      return Err(Error::Session);
    }
    let result = (|| {
      let committed = if flags & GCS_RESULTSTR.0 != 0 {
        Some(Event::Commit(read(current, GCS_RESULTSTR)?))
      } else {
        None
      };
      let preedit = if flags & (GCS_COMPSTR.0 | GCS_CURSORPOS.0) != 0 {
        let text = read(current, GCS_COMPSTR)?;
        let cursor = ImmGetCompositionStringW(current, GCS_CURSORPOS, None, 0);
        let byte = usize::try_from(cursor)
          .ok()
          .and_then(|n| utf16_range(&text, n, 0).ok().map(|range| range.start))
          .ok_or(Error::Range)?;
        Some(Event::Preedit {
          text,
          cursor: Some(byte..byte),
        })
      } else if flags == 0 {
        Some(Event::Cancel)
      } else {
        None
      };
      Ok([committed, preedit])
    })();
    let _ = ImmReleaseContext(hwnd, current);
    result
  }
}
unsafe fn read(context: HIMC, field: IME_COMPOSITION_STRING) -> Result<String, Error> {
  let bytes = unsafe { ImmGetCompositionStringW(context, field, None, 0) };
  let bytes = usize::try_from(bytes).map_err(|_| Error::Range)?;
  if bytes > MAX_TEXT_INPUT_BYTES * 2 {
    return Err(Error::TooLong);
  }
  if bytes % 2 != 0 {
    return Err(Error::Range);
  }
  let mut units = vec![0u16; bytes / 2];
  let copied = unsafe {
    ImmGetCompositionStringW(
      context,
      field,
      Some(units.as_mut_ptr().cast()),
      bytes as u32,
    )
  };
  // Never expose uninitialized/oversized results, including native length races.
  if copied < 0 || copied as usize != bytes {
    return Err(Error::Range);
  }
  utf16_text(&units)
}
