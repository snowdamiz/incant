// Copyright 2026 Incant contributors
// SPDX-License-Identifier: Apache-2.0
//! One Cocoa client/context per owner. A retired client can never retag an
//! asynchronous callback as belonging to a newly focused widget.
use super::{app_state::AppState, event::EventWrapper, util::EMPTY_RANGE, window::get_window_id};
use crate::{
  event::{Event, WindowEvent},
  text_input::{
    utf16_range, utf16_text, CocoaComposition, TextInputError, TextInputEvent, MAX_TEXT_INPUT_BYTES,
  },
  window::WindowId,
};
use objc2::{
  define_class, msg_send,
  rc::{Retained, Weak},
  runtime::{AnyObject, ProtocolObject, Sel},
  DefinedClass, MainThreadOnly,
};
use objc2_app_kit::{NSEvent, NSTextInputClient, NSTextInputContext, NSView, NSWindow};
use objc2_foundation::{
  MainThreadMarker, NSArray, NSAttributedString, NSAttributedStringKey, NSObjectProtocol, NSPoint,
  NSRange, NSRangePointer, NSRect, NSSize, NSString, NSUInteger,
};
use std::cell::{Cell, RefCell};

struct ClientState {
  serial: u64,
  window: Weak<NSWindow>,
  composition: RefCell<CocoaComposition>,
  selected: Cell<NSRange>,
  position: Cell<NSPoint>,
  delivered: Cell<bool>,
  command: Cell<bool>,
}

define_class!(
  // SAFETY: NSView permits subclassing; all access stays on the main thread.
  #[unsafe(super = NSView)]
  #[thread_kind = MainThreadOnly]
  #[ivars = ClientState]
  struct ManagedClient;
  impl ManagedClient {
    #[unsafe(method_id(window))]
    fn owning_window(&self) -> Option<Retained<NSWindow>> { self.ivars().window.load() }
  }
  unsafe impl NSObjectProtocol for ManagedClient {}
  // SAFETY: Each selector uses its SDK signature and owns no untraced client
  // references. Caller-supplied out pointers are touched only in unsafe methods.
  unsafe impl NSTextInputClient for ManagedClient {
    #[unsafe(method(insertText:replacementRange:))]
    unsafe fn insert(&self, value: &AnyObject, replacement: NSRange) {
      if !self.ivars().composition.borrow().active() { return; }
      let result = self.text(value).and_then(|text| {
        self.replacement(replacement)?;
        self.ivars().composition.borrow_mut().insert(text)
      });
      if result.is_ok() { self.ivars().selected.set(EMPTY_RANGE); }
      self.deliver_optional(result);
    }
    #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
    unsafe fn preedit(&self, value: &AnyObject, selected: NSRange, replacement: NSRange) {
      if !self.ivars().composition.borrow().active() { return; }
      let result = self.text(value).and_then(|text| {
        self.replacement(replacement)?;
        let cursor = if selected.location == EMPTY_RANGE.location { None }
          else { Some(utf16_range(&text, selected.location, selected.length)?) };
        let event = self.ivars().composition.borrow_mut().preedit(text, cursor)?;
        self.ivars().selected.set(selected);
        Ok(event)
      });
      self.deliver_optional(result);
    }
    #[unsafe(method(unmarkText))]
    fn unmark(&self) {
      let event = self.ivars().composition.borrow_mut().unmark();
      self.ivars().selected.set(EMPTY_RANGE);
      self.deliver_optional(Ok(event));
    }
    #[unsafe(method(doCommandBySelector:))]
    unsafe fn command(&self, _selector: Sel) { self.ivars().command.set(true); }
    #[unsafe(method(selectedRange))]
    fn selection(&self) -> NSRange { self.ivars().selected.get() }
    #[unsafe(method(markedRange))]
    fn marked(&self) -> NSRange {
      let count = self.ivars().composition.borrow().marked_units();
      if count == 0 { EMPTY_RANGE } else { NSRange::new(0, count) }
    }
    #[unsafe(method(hasMarkedText))]
    fn has_marked(&self) -> bool { self.ivars().composition.borrow().marked_units() != 0 }
    #[unsafe(method_id(validAttributesForMarkedText))]
    fn attributes(&self) -> Retained<NSArray<NSAttributedStringKey>> { NSArray::new() }
    #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
    unsafe fn substring(&self, _range: NSRange, actual: NSRangePointer) -> Option<Retained<NSAttributedString>> {
      if !actual.is_null() { unsafe { *actual = EMPTY_RANGE; } }
      None
    }
    #[unsafe(method(firstRectForCharacterRange:actualRange:))]
    unsafe fn rect(&self, range: NSRange, actual: NSRangePointer) -> NSRect {
      if !actual.is_null() { unsafe { *actual = range; } }
      NSRect::new(self.ivars().position.get(), NSSize::new(0., 0.))
    }
    #[unsafe(method(characterIndexForPoint:))]
    fn index(&self, _point: NSPoint) -> NSUInteger { EMPTY_RANGE.location }
  }
);
impl ManagedClient {
  fn deliver_optional(&self, result: Result<Option<TextInputEvent>, TextInputError>) {
    match result {
      Ok(Some(event)) => self.deliver(Ok(event)),
      Err(error) => self.deliver(Err(error)),
      Ok(None) => {}
    }
  }
  fn deliver(&self, result: Result<TextInputEvent, TextInputError>) {
    if !self.ivars().composition.borrow().active() {
      return;
    }
    if let Some(window) = self.ivars().window.load() {
      self.ivars().delivered.set(true);
      AppState::queue_event(EventWrapper::StaticEvent(Event::WindowEvent {
        window_id: WindowId(get_window_id(&window)),
        event: WindowEvent::TextInput {
          session: self.ivars().serial,
          event: result.unwrap_or_else(TextInputEvent::Refused),
        },
      }));
    }
  }
  fn replacement(&self, range: NSRange) -> Result<(), TextInputError> {
    // The engine owns committed selection. Cocoa may replace the whole marked
    // range, but reconversion/replacement of unknown committed surroundings is
    // explicitly unsupported instead of silently changing the wrong substring.
    if range.location == EMPTY_RANGE.location
      || (self.ivars().composition.borrow().marked_units() != 0
        && range.location == 0
        && range.length == self.ivars().composition.borrow().marked_units())
    {
      Ok(())
    } else {
      Err(TextInputError::Replacement)
    }
  }
  fn text(&self, value: &AnyObject) -> Result<String, TextInputError> {
    let attributed;
    let text = if let Some(text) = value.downcast_ref::<NSString>() {
      text
    } else if let Some(text) = value.downcast_ref::<NSAttributedString>() {
      attributed = text.string();
      &attributed
    } else {
      return Err(TextInputError::Unsupported);
    };
    // UTF-16 length bounds the first Rust allocation; checked UTF-8 conversion
    // then applies the exact public payload ceiling. No lossy/truncated text.
    if text.length() > MAX_TEXT_INPUT_BYTES {
      return Err(TextInputError::TooLong);
    }
    let mut units = vec![0u16; text.length()];
    // SAFETY: initialized buffer is exactly the checked NSString UTF-16 length.
    unsafe {
      text.getCharacters_range(
        std::ptr::NonNull::new(units.as_mut_ptr()).expect("Vec pointer"),
        NSRange::new(0, units.len()),
      );
    }
    utf16_text(&units)
  }
}

#[derive(Clone)]
pub(super) struct Managed {
  context: Retained<NSTextInputContext>,
  client: Retained<ManagedClient>,
}
impl Managed {
  pub fn new(serial: u64, window: &NSWindow, position: Option<(f64, f64)>) -> Self {
    let mtm = MainThreadMarker::new().expect("managed input is main-thread only");
    let (x, y) = position.unwrap_or_else(|| {
      let frame = window.frame();
      (frame.origin.x, frame.origin.y + frame.size.height)
    });
    let this = ManagedClient::alloc(mtm).set_ivars(ClientState {
      serial,
      window: Weak::from(window),
      composition: RefCell::new(CocoaComposition::new()),
      selected: Cell::new(EMPTY_RANGE),
      position: Cell::new(NSPoint::new(x, y)),
      delivered: Cell::new(false),
      command: Cell::new(false),
    });
    // SAFETY: NSView's designated initializer, initialized Rust ivars; the
    // context retains the client, which never retains the context or window.
    let client: Retained<ManagedClient> = unsafe {
      msg_send![super(this), initWithFrame: NSRect::new(NSPoint::new(0.,0.), NSSize::new(0.,0.))]
    };
    let context = NSTextInputContext::initWithClient(
      NSTextInputContext::alloc(mtm),
      ProtocolObject::from_ref(&*client),
    );
    context.activate();
    Self { context, client }
  }
  pub fn raw_context(&self) -> *mut AnyObject {
    Retained::as_ptr(&self.context).cast_mut().cast()
  }
  pub fn retire(&self) {
    // Clear active before native calls; synchronous/reentrant or delayed old
    // callbacks can only target the old immutable serial and are discarded.
    self.client.ivars().composition.borrow_mut().retire();
    self.context.discardMarkedText();
    self.context.deactivate();
  }
  pub fn position(&self, x: f64, y: f64) {
    self.client.ivars().position.set(NSPoint::new(x, y));
    self.context.invalidateCharacterCoordinates();
  }
  pub fn handle(&self, event: &NSEvent) -> Option<u64> {
    let state = self.client.ivars();
    if !state.composition.borrow().active() {
      return None;
    }
    let composing = state.composition.borrow().marked_units() != 0;
    state.delivered.set(false);
    state.command.set(false);
    let handled = self.context.handleEvent(event);
    (composing || state.delivered.get() || (handled && !state.command.get()))
      .then_some(state.serial)
  }
}
