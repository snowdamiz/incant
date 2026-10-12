// Copyright 2026 Incant contributors
// SPDX-License-Identifier: Apache-2.0
//! GTK contexts and their callbacks are never reused across text owners.
use crate::text_input::{Session, State, TextInputError, TextInputEvent, MAX_TEXT_INPUT_BYTES};
use gtk::{gdk, prelude::*};
use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};
struct Context {
  ime: gtk::IMMulticontext,
  live: Rc<Cell<bool>>,
}
impl Context {
  fn retire(self) {
    self.live.set(false); // retirement precedes callbacks triggered by reset
    self.ime.reset();
    self.ime.focus_out();
    self.ime.set_client_window(None);
  }
}
pub(super) struct Managed {
  legacy: gtk::IMContextSimple,
  state: RefCell<State>,
  context: RefCell<Option<Context>>,
  emit: Rc<dyn Fn(u64, TextInputEvent)>,
}
impl Managed {
  pub fn new(legacy: gtk::IMContextSimple, emit: impl Fn(u64, TextInputEvent) + 'static) -> Self {
    Self {
      legacy,
      state: Default::default(),
      context: Default::default(),
      emit: Rc::new(emit),
    }
  }
  pub fn is_managed(&self) -> bool {
    self.state.borrow().session.is_some()
  }
  pub fn set(&self, window: &gtk::Window, serial: u64, enabled: bool) {
    match self.state.borrow().check(serial, enabled) {
      Ok(false) => return,
      Err(error) => {
        (self.emit)(serial, TextInputEvent::Refused(error));
        return;
      }
      Ok(true) => {}
    }
    if !self.is_managed() {
      self.legacy.reset();
      self.legacy.focus_out();
    }
    let old = self.context.borrow_mut().take();
    self.state.borrow_mut().session = Some(Session { serial, enabled });
    if let Some(old) = old {
      old.retire();
    }
    if !enabled {
      return;
    }
    let ime = gtk::IMMulticontext::new();
    let live = Rc::new(Cell::new(true));
    ime.set_client_window(window.window().as_ref());
    ime.set_use_preedit(true);
    let emit = self.emit.clone();
    let active = live.clone();
    ime.connect_commit(move |_, text| {
      if !active.get() {
        return;
      }
      emit(
        serial,
        if text.len() > MAX_TEXT_INPUT_BYTES {
          TextInputEvent::Refused(TextInputError::TooLong)
        } else {
          TextInputEvent::Commit(text.to_owned())
        },
      );
    });
    let emit = self.emit.clone();
    let active = live.clone();
    ime.connect_preedit_changed(move |context| {
      if !active.get() {
        return;
      }
      // GTK owns its returned temporary string. Bound the engine-owned copy
      // before cloning; this does not purport to bound input-method OS memory.
      let (text, _, cursor) = context.preedit_string();
      if text.len() > MAX_TEXT_INPUT_BYTES {
        emit(serial, TextInputEvent::Refused(TextInputError::TooLong));
        return;
      }
      let byte = usize::try_from(cursor).ok().and_then(|n| {
        text
          .char_indices()
          .map(|(i, _)| i)
          .chain(std::iter::once(text.len()))
          .nth(n)
      });
      let Some(byte) = byte else {
        emit(serial, TextInputEvent::Refused(TextInputError::Range));
        return;
      };
      emit(
        serial,
        TextInputEvent::Preedit {
          text: text.to_string(),
          cursor: Some(byte..byte),
        },
      );
    });
    let emit = self.emit.clone();
    let active = live.clone();
    ime.connect_preedit_end(move |_| {
      if active.get() {
        emit(serial, TextInputEvent::Cancel);
      }
    });
    let emit = self.emit.clone();
    let active = live.clone();
    ime.connect_delete_surrounding(move |_, _, _| {
      if active.get() {
        emit(serial, TextInputEvent::Refused(TextInputError::Replacement));
      }
      false // no unvalidated mutation of committed surrounding text
    });
    ime.focus_in();
    *self.context.borrow_mut() = Some(Context { ime, live });
  }
  pub fn filter(&self, event: &gdk::EventKey) -> Option<u64> {
    let serial = self.state.borrow().active()?;
    let context = self
      .context
      .borrow()
      .as_ref()
      .map(|context| context.ime.clone())?;
    // No RefCell loan spans a signal callback.
    context.filter_keypress(event).then_some(serial)
  }
  pub fn position(&self, x: i32, y: i32) {
    let context = self
      .context
      .borrow()
      .as_ref()
      .map(|context| context.ime.clone());
    if let Some(context) = context {
      context.set_cursor_location(&gdk::Rectangle::new(x, y, 0, 0));
    }
  }
  pub fn retire(&self) {
    let old = self.context.borrow_mut().take();
    if let Some(old) = old {
      old.retire();
    }
  }
}
impl Drop for Managed {
  fn drop(&mut self) {
    self.retire();
  }
}
