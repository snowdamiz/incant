//! Native delivery is a bounded queue of receipts from the displayed UI owner.
//! No native callback edits widgets or fabricates text from a physical key.
use super::{
    Error,
    input::{PendingInput, logical_position},
};
use incant_input::{InputEvent, MouseButton};
use incant_ui::{FocusToken, MAX_UI_EVENTS, MAX_UI_PACKET_BYTES, PointerToken, UiEvent, UiRuntime};
use tao::{
    dpi::PhysicalPosition,
    event::{ElementState, WindowEvent},
};

#[derive(Default)]
enum Left {
    #[default]
    Up,
    Gameplay,
    Ui(PointerToken),
    /// A physical press survived cancellation/owner change. Its eventual release
    /// belongs to neither a replacement widget nor a new gameplay press.
    Suppressed,
}
pub struct UiInput {
    receipts: Vec<UiEvent>,
    bytes: usize,
    observed: Option<PointerToken>,
    position: Option<[f32; 2]>,
    left: Left,
    pointer_ready: bool,
    surface_retry: bool,
    ime: super::ime::Ime,
}
impl UiInput {
    pub fn new() -> Self {
        Self {
            receipts: Vec::with_capacity(MAX_UI_EVENTS),
            bytes: 0,
            observed: None,
            position: None,
            left: Left::Up,
            pointer_ready: true,
            surface_retry: false,
            ime: Default::default(),
        }
    }
    pub fn text_session(
        &mut self,
        ui: Option<&UiRuntime>,
        active: bool,
    ) -> Result<Option<(u64, bool)>, Error> {
        self.ime.synchronize(ui, active)
    }
    pub fn stale_text_events(&self) -> u64 {
        self.ime.stale()
    }
    fn native_text(
        &mut self,
        ui: Option<&UiRuntime>,
        serial: u64,
        event: &tao::text_input::TextInputEvent,
    ) -> Result<(), Error> {
        use tao::text_input::TextInputEvent as Native;
        // Retired native clients cannot poison the current owner even with an
        // unrepresentable payload. Resolve authority before scanning or copying.
        let Some(target) = self.ime.target(ui, serial) else {
            return Ok(());
        };
        let (text, cursor) = match event {
            Native::Preedit { text, cursor } => (Some(text), cursor.as_ref()),
            Native::Commit(text) => (Some(text), None),
            Native::Cancel => (None, None),
            Native::Refused(error) => return Err(Error::Ime(*error)),
        };
        if let Some(text) = text {
            if text.len() > incant_types::MAX_TEXT_BYTES {
                return Err(Error::UiCapacity);
            }
            if text.chars().any(char::is_control) {
                return Err(Error::Ime(tao::text_input::TextInputError::Unsupported));
            }
            if cursor.is_some_and(|r| {
                r.start > r.end
                    || r.end > text.len()
                    || !text.is_char_boundary(r.start)
                    || !text.is_char_boundary(r.end)
            }) {
                return Err(Error::Ime(tao::text_input::TextInputError::Range));
            }
        }
        let bytes = text.map_or(0, |s| s.len());
        // Reserve the complete receipt before cloning any native payload.
        if self.receipts.len() == MAX_UI_EVENTS
            || self
                .bytes
                .checked_add(bytes)
                .is_none_or(|n| n > MAX_UI_PACKET_BYTES)
        {
            return Err(Error::UiCapacity);
        }
        let event = match event {
            Native::Preedit { text, cursor } => UiEvent::Preedit {
                target,
                text: text.clone(),
                cursor: cursor.clone(),
            },
            Native::Commit(text) => UiEvent::Commit {
                target,
                text: text.clone(),
            },
            Native::Cancel => UiEvent::Cancel { target },
            Native::Refused(_) => unreachable!("handled refusal"),
        };
        self.push(event, bytes)
    }
    pub fn receipts(&self) -> &[UiEvent] {
        &self.receipts
    }
    /// Only after an accepted composed tick. A retry retains the exact receipts.
    pub fn accepted(&mut self) {
        self.receipts.clear();
        self.bytes = 0;
    }
    pub fn reset_focus(&mut self) {
        self.surface_retry = false;
        self.accepted();
        self.observed = None;
        self.position = None;
        self.left = Left::Up;
    }
    /// A temporary surface acquisition timeout is not an OS focus loss.
    pub fn defer_surface(&mut self) {
        self.surface_retry = true;
    }
    pub fn awaiting_surface(&self) -> bool {
        self.surface_retry
    }
    /// Surface waits preserve bounded delivery; real inactivity never does.
    pub fn accepts_delivery(&self, active: bool, suspended: bool) -> bool {
        active && (!suspended || self.surface_retry)
    }
    pub fn resume(&mut self, physical: &mut PendingInput, focused: bool) {
        if self.surface_retry {
            self.surface_retry = false;
        } else {
            physical.reset_focus(focused);
            self.reset_focus();
        }
    }
    fn push(&mut self, event: UiEvent, bytes: usize) -> Result<(), Error> {
        if self.receipts.len() == MAX_UI_EVENTS
            || bytes > incant_types::MAX_TEXT_BYTES
            || self
                .bytes
                .checked_add(bytes)
                .is_none_or(|sum| sum > MAX_UI_PACKET_BYTES)
        {
            return Err(Error::UiCapacity);
        }
        self.receipts.push(event);
        self.bytes += bytes;
        Ok(())
    }
    /// Viewport changes expire pending pointer receipts, but preserve the actual
    /// text focus/preedit contract. Focused text receipts retain their own token.
    pub fn invalidate_pointer(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        self.receipts
            .retain(|e| !matches!(e, UiEvent::Pointer { .. } | UiEvent::CancelPointer { .. }));
        if matches!(self.left, Left::Gameplay) {
            physical.push(InputEvent::MouseButton {
                button: MouseButton::Left,
                down: false,
            })?;
        }
        if !matches!(self.left, Left::Up) {
            self.left = Left::Suppressed;
        }
        if let Some(target) = ui.and_then(UiRuntime::pointer_token) {
            self.push(UiEvent::CancelPointer { target }, 0)?;
        }
        self.position = None;
        self.observed = ui.and_then(UiRuntime::pointer_token);
        Ok(())
    }
    /// Fence pointer delivery immediately when the native viewport changes.
    /// Only a consumed frame can publish the replacement hit-test coordinates.
    pub fn layout_pending(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        self.invalidate_pointer(ui, physical)?;
        self.pointer_ready = false;
        Ok(())
    }
    pub fn presented(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        self.synchronize(ui, physical)?;
        self.pointer_ready = true;
        Ok(())
    }
    fn cancel_left(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        match self.left {
            Left::Gameplay => physical.push(InputEvent::MouseButton {
                button: MouseButton::Left,
                down: false,
            })?,
            Left::Ui(_) => {
                if let Some(target) = ui.and_then(UiRuntime::pointer_token) {
                    self.push(UiEvent::CancelPointer { target }, 0)?;
                }
            }
            _ => return Ok(()),
        }
        self.left = Left::Suppressed;
        Ok(())
    }
    /// Called after accepted tree/focus/layout publication, and before delivery.
    pub fn synchronize(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        let token = ui.and_then(UiRuntime::pointer_token);
        if self.observed != token {
            self.cancel_left(ui, physical)?;
            self.observed = token;
        }
        if matches!(self.left, Left::Gameplay) && self.hit(ui) {
            self.cancel_left(ui, physical)?;
        }
        Ok(())
    }
    fn hit(&self, ui: Option<&UiRuntime>) -> bool {
        self.position
            .zip(ui)
            .is_some_and(|(p, ui)| ui.pointer_hit(p).is_some())
    }
    fn pointer(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        position: PhysicalPosition<f64>,
        scale: f64,
    ) -> Result<(), Error> {
        let logical = logical_position(position, scale)?;
        if !logical
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
        {
            return Err(Error::InputRange);
        }
        self.position = Some(logical.map(|v| v as f32));
        if !self.pointer_ready {
            return Ok(());
        }
        self.synchronize(ui, physical)?;
        if !self.hit(ui) && !matches!(self.left, Left::Ui(_) | Left::Suppressed) {
            physical.push(InputEvent::PointerPosition { position: logical })?;
        }
        Ok(())
    }
    fn left(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        down: bool,
    ) -> Result<(), Error> {
        if !self.pointer_ready {
            self.left = if down { Left::Suppressed } else { Left::Up };
            return Ok(());
        }
        self.synchronize(ui, physical)?;
        if down {
            if !matches!(self.left, Left::Up) {
                return Ok(());
            }
            if self.position.is_none()
                && ui
                    .and_then(UiRuntime::layout)
                    .is_some_and(|layout| layout.has_focusable())
            {
                self.left = Left::Suppressed;
                return Ok(());
            }
            if self.hit(ui) {
                let target = ui.and_then(UiRuntime::pointer_token).expect("hit layout");
                self.push(
                    UiEvent::Pointer {
                        position: self.position.expect("hit position"),
                        pressed: true,
                        target: target.clone(),
                    },
                    0,
                )?;
                self.left = Left::Ui(target);
            } else {
                physical.push(InputEvent::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                })?;
                self.left = Left::Gameplay;
            }
        } else {
            match &self.left {
                Left::Gameplay => physical.push(InputEvent::MouseButton {
                    button: MouseButton::Left,
                    down: false,
                })?,
                Left::Ui(target) => {
                    if let Some(position) = self.position {
                        self.push(
                            UiEvent::Pointer {
                                position,
                                pressed: false,
                                target: target.clone(),
                            },
                            0,
                        )?;
                    }
                }
                _ => {}
            }
            self.left = Left::Up;
        }
        Ok(())
    }
    fn commit(&mut self, target: Option<FocusToken>, text: &str) -> Result<(), Error> {
        // Enter/Tab/backspace are routed as typed UI operations, not inserted a
        // second time via the platform's ordinary text callback.
        if text.is_empty() || text.chars().any(char::is_control) {
            return Ok(());
        }
        let Some(target) = target else {
            return Ok(());
        };
        if text.len() > incant_types::MAX_TEXT_BYTES
            || self.bytes.saturating_add(text.len()) > MAX_UI_PACKET_BYTES
            || self.receipts.len() == MAX_UI_EVENTS
        {
            return Err(Error::UiCapacity);
        }
        self.push(
            UiEvent::Commit {
                target,
                text: text.to_owned(),
            },
            text.len(),
        )
    }
    fn consumed_key(
        &self,
        physical: &mut PendingInput,
        code: tao::keyboard::KeyCode,
        repeat: bool,
    ) -> Result<(), Error> {
        if !repeat && let Some(code) = super::input::key(code) {
            physical.push(InputEvent::Key { code, down: false })?;
        }
        Ok(())
    }
    pub fn window(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        event: &WindowEvent<'_>,
        scale: f64,
    ) -> Result<(), Error> {
        self.synchronize(ui, physical)?;
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer(ui, physical, *position, scale)
            }
            WindowEvent::CursorLeft { .. } => self.invalidate_pointer(ui, physical),
            WindowEvent::MouseInput {
                button: tao::event::MouseButton::Left,
                state,
                ..
            } => self.left(ui, physical, *state == ElementState::Pressed),
            WindowEvent::MouseWheel { .. }
                if !self.pointer_ready
                    || self.hit(ui)
                    || matches!(self.left, Left::Ui(_) | Left::Suppressed) =>
            {
                Ok(())
            }
            // Tao macOS/GTK send ordinary and IME text here. Their KeyboardInput
            // text overlaps this stream and must not be inserted a second time.
            WindowEvent::TextInput { session, event } => self.native_text(ui, *session, event),
            WindowEvent::ReceivedImeText(_) if self.ime.managed() => Ok(()),
            WindowEvent::ReceivedImeText(text) => {
                self.commit(ui.and_then(UiRuntime::text_focus), text)
            }
            WindowEvent::KeyboardInput { event: key, .. } => {
                if key.text_input_session.is_some() && key.state == ElementState::Pressed {
                    // A native IME-consumed Enter/arrows/space cannot also submit,
                    // navigate or start gameplay. Neutral release also clears any
                    // press that was accepted before text ownership changed.
                    self.consumed_key(physical, key.physical_key, key.repeat)?;
                    return Ok(());
                }
                physical.window(event, scale)?;
                // Windows ordinary WM_CHAR text is attached to KeyEvent; its
                // ReceivedImeText is the separate completed IME composition.
                #[cfg(target_os = "windows")]
                if !self.ime.managed()
                    && key.state == ElementState::Pressed
                    && let Some(text) = key.text
                {
                    self.commit(ui.and_then(UiRuntime::text_focus), text)?;
                }
                #[cfg(not(target_os = "windows"))]
                let _ = key;
                Ok(())
            }
            _ => physical.window(event, scale),
        }
    }
}

#[cfg(test)]
mod tests;
