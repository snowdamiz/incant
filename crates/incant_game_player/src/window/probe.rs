//! Opt-in numerical native lifecycle exercise. It requests OS resize/minimize
//! and restores focus; produces no pixels/captures or appearance judgments.
use super::{session, surface::SurfaceState};
use incant_render::Renderer;
use std::time::{Duration, Instant};
use tao::{dpi::PhysicalSize, window::Window};
pub struct Probe {
    start: Instant,
    resized: bool,
    resize_sent: bool,
    minimized: Option<Instant>,
    saw_minimized: bool,
    restored: bool,
    frozen_tick: u64,
    recreated: bool,
}
impl Probe {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            resized: false,
            resize_sent: false,
            minimized: None,
            saw_minimized: false,
            restored: false,
            frozen_tick: 0,
            recreated: false,
        }
    }
    pub fn observe(
        &mut self,
        window: &Window,
        tick: u64,
        surface: &mut SurfaceState,
        renderer: &Renderer,
    ) -> session::Result<()> {
        if self.start.elapsed() > Duration::from_secs(20) {
            return Err("native lifecycle probe deadline exceeded".into());
        }
        if tick >= 8 && !self.resize_sent {
            window.set_inner_size(PhysicalSize::new(512, 320));
            self.resize_sent = true;
        }
        if self.resize_sent && window.inner_size() == PhysicalSize::new(512, 320) {
            self.resized = true;
        }
        if tick >= 16 && self.minimized.is_none() {
            window.set_minimized(true);
            self.minimized = Some(Instant::now());
            self.frozen_tick = tick;
        }
        if let Some(start) = self.minimized
            && !self.restored
        {
            self.saw_minimized |= window.is_minimized();
            if self.saw_minimized && tick != self.frozen_tick {
                return Err("simulation advanced while native window minimized".into());
            }
            if start.elapsed() >= Duration::from_millis(250) {
                if !self.saw_minimized {
                    return Err("platform did not acknowledge requested minimization".into());
                }
                window.set_minimized(false);
                window.set_focus();
                self.restored = true;
            }
        }
        if tick >= 24 && !self.recreated {
            surface.recreate(renderer)?;
            self.recreated = true;
        }
        Ok(())
    }
    pub fn validate(&self) -> session::Result<()> {
        if !(self.resized && self.saw_minimized && self.restored && self.recreated) {
            return Err("native lifecycle probe incomplete".into());
        }
        Ok(())
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"actual_resize":self.resized,"actual_minimize_restore":self.saw_minimized&&self.restored,"suspended_tick":self.frozen_tick,"explicit_surface_recreation":self.recreated,"os_loss_injected":false})
    }
}
