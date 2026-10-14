//! One host-owned relative-pointer capture. Tao displacement has unspecified
//! device units: sensitivity maps them to virtual logical-pixel equivalents.
//! Neither project data nor scripts can acquire OS cursor authority.
use super::{Error, input::PendingInput};
use std::sync::Arc;
use tao::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
    window::Window,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Sensitivity(f64);
impl Sensitivity {
    pub(super) fn new(value: f64) -> Result<Self, Error> {
        if !value.is_finite() || !(1. / 1024. ..=1024.).contains(&value) {
            return Err(Error::PointerSensitivity);
        }
        Ok(Self(value))
    }
}

pub(super) trait Cursor {
    fn grab(&self, grabbed: bool) -> Result<(), Error>;
    fn visible(&self, visible: bool);
}
impl Cursor for Arc<Window> {
    fn grab(&self, grabbed: bool) -> Result<(), Error> {
        self.set_cursor_grab(grabbed)
            .map_err(|error| Error::PointerCapture(error.to_string()))
    }
    fn visible(&self, visible: bool) {
        self.set_cursor_visible(visible);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Delivery {
    Ordinary,
    Gameplay,
    Consume,
}
#[derive(Debug)]
pub(super) struct Route {
    pub delivery: Delivery,
    pub reset: bool,
}
impl Route {
    fn new(delivery: Delivery) -> Self {
        Self {
            delivery,
            reset: false,
        }
    }
}

pub(super) struct Pointer<C: Cursor> {
    cursor: C,
    sensitivity: Option<Sensitivity>,
    captured: bool,
    // Also set before acquisition, so failed acquisition/unwind still cleans up.
    release_required: bool,
    rearm_release: bool,
    captures: u64,
    releases: u64,
    raw_events: u64,
    last_raw: Option<[f64; 2]>,
    last_scaled: Option<[f64; 2]>,
}
impl<C: Cursor> Pointer<C> {
    pub(super) fn new(cursor: C, sensitivity: Option<Sensitivity>) -> Self {
        Self {
            cursor,
            sensitivity,
            captured: false,
            release_required: false,
            rearm_release: false,
            captures: 0,
            releases: 0,
            raw_events: 0,
            last_raw: None,
            last_scaled: None,
        }
    }
    /// Eligibility never acquires capture. Only a new in-window left press can.
    fn button(
        &mut self,
        button: MouseButton,
        state: ElementState,
        eligible: bool,
    ) -> Result<Route, Error> {
        if button == MouseButton::Left && self.rearm_release {
            self.rearm_release = false;
            if state == ElementState::Released {
                return Ok(Route::new(Delivery::Consume));
            }
            // A new press also retires a release lost while unfocused. It is
            // a new gesture, never an automatic focus-return acquisition.
        }
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && eligible
            && !self.captured
            && self.sensitivity.is_some()
        {
            self.release_required = true;
            if let Err(error) = self.cursor.grab(true) {
                let _ = self.release(); // preserve acquisition failure; Drop retries cleanup
                return Err(error);
            }
            self.cursor.visible(false);
            self.captured = true;
            self.rearm_release = true;
            self.captures = self.captures.saturating_add(1);
            return Ok(Route {
                delivery: Delivery::Consume,
                reset: true,
            });
        }
        Ok(Route::new(if self.captured {
            Delivery::Gameplay
        } else {
            Delivery::Ordinary
        }))
    }
    fn escape(&mut self) -> Result<Route, Error> {
        if self.captured {
            self.release()?;
            Ok(Route {
                delivery: Delivery::Consume,
                reset: true,
            })
        } else {
            Ok(Route::new(Delivery::Ordinary))
        }
    }
    pub(super) fn window(
        &mut self,
        event: &WindowEvent<'_>,
        eligible: bool,
    ) -> Result<Route, Error> {
        match event {
            WindowEvent::MouseInput { button, state, .. } => self.button(*button, *state, eligible),
            WindowEvent::KeyboardInput { event, .. }
                if event.physical_key == KeyCode::Escape
                    && event.state == ElementState::Pressed =>
            {
                self.escape()
            }
            WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
                if self.captured =>
            {
                Ok(Route::new(Delivery::Consume))
            }
            WindowEvent::MouseWheel { .. } if self.captured => Ok(Route::new(Delivery::Gameplay)),
            _ => Ok(Route::new(Delivery::Ordinary)),
        }
    }
    pub(super) fn motion(
        &mut self,
        raw: [f64; 2],
        eligible: bool,
        pending: &mut PendingInput,
    ) -> Result<(), Error> {
        // DeviceEvent is global, unlike WindowEvent. No validation, storage or
        // delivery of other windows' motion, nor the duplicate axis events.
        if !eligible || !self.captured {
            return Ok(());
        }
        if !raw.iter().all(|n| n.is_finite() && n.abs() <= 1_000_000.) {
            return Err(Error::InputRange);
        }
        let scale = self.sensitivity.expect("captured host option").0;
        let scaled = raw.map(|n| n * scale);
        pending.relative_motion(scaled)?;
        self.last_raw = Some(raw);
        self.last_scaled = Some(scaled);
        self.raw_events = self.raw_events.saturating_add(1);
        Ok(())
    }
    /// Revoke delivery before OS cleanup, including cleanup failure. Never
    /// reacquire on focus/resize/resume. Preserve only the rearm click's release.
    pub(super) fn release(&mut self) -> Result<bool, Error> {
        let was_captured = std::mem::replace(&mut self.captured, false);
        if self.release_required {
            self.cursor.visible(true);
            self.cursor.grab(false)?;
            self.release_required = false;
            if was_captured {
                self.releases = self.releases.saturating_add(1);
            }
        }
        Ok(was_captured)
    }
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.sensitivity.is_some(),
            "virtual_pixels_per_raw_unit": self.sensitivity.map(|s| s.0),
            "raw_units": "device-dependent Tao MouseMotion displacement; not DPI pixels",
            "captures": self.captures, "releases": self.releases,
            "delivered_raw_events": self.raw_events,
            "last_raw_delta": self.last_raw, "last_scaled_delta": self.last_scaled,
            "captured": self.captured,
        })
    }
}
impl<C: Cursor> Drop for Pointer<C> {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests;
