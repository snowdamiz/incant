//! Concrete Android surface + device-audio host. Platform callbacks stay on the
//! view's owning thread; gameplay uses the retained cooked-session owner.
#[path = "../window/clock.rs"]
#[allow(dead_code)] // The desktop retry helper is not used by display callbacks.
mod clock;
use super::surface;
use crate::mobile_host::touch::{NativeTouchInput, TouchError};
use crate::mobile_host::{MobileHostError, MobileSession};
use incant_input::TouchPhase;
use incant_render::Renderer;
use incant_render_gpu_host::{GpuHostError, GpuRenderConsumer, GpuRenderSettings};
use ndk::native_window::NativeWindow;
use std::{path::Path, sync::Arc, time::Instant};
use surface::{Surface, SurfaceError, Targets};

type Consumer = GpuRenderConsumer<Targets>;
type Game = MobileSession<Consumer>;
type Result<T> = std::result::Result<T, NativeHostError>;
#[derive(Debug, thiserror::Error)]
pub enum NativeHostError {
    #[error(transparent)]
    Surface(#[from] SurfaceError),
    #[error(transparent)]
    Game(#[from] MobileHostError<GpuHostError<SurfaceError>>),
    #[error("native platform settings or input are invalid")]
    Input,
    #[error("native delivery is inactive or stale")]
    Stale,
    #[error(transparent)]
    Touch(#[from] TouchError),
    #[error("native host is closed")]
    Closed,
    #[error("native lifecycle identity exhausted")]
    Epoch,
}
/// Owns actual GPU resources and a real native audio backend. It retains acquired native windows through surface ownership. Failure retires gameplay; a closed host is never reused.
/// Platform keyboard/IME adaptation is separate from this initial touch bridge.
pub struct AndroidMobileHost {
    game: Option<Game>,
    surface: Surface,
    renderer: Arc<Renderer>,
    clock: clock::Clock,
    requested_active: bool,
    delivering: bool,
    epoch: u64,
    input: NativeTouchInput,
}
impl AndroidMobileHost {
    /// Create on the platform view's owning thread. The host starts suspended.
    ///
    /// All callbacks are serialized on the Android view thread.
    pub fn open(
        window: NativeWindow,
        package: &Path,
        physical_size: [u32; 2],
        logical_size: [f32; 2],
    ) -> Result<Self> {
        validate_logical(logical_size)?;
        let (surface, renderer, targets) = Surface::new(window, physical_size)?;
        let game = Game::open_device(
            package,
            Some(logical_size),
            GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets,
            },
        )?;
        let rate = game.world()?.tick_rate();
        Ok(Self {
            game: Some(game),
            surface,
            renderer,
            clock: clock::Clock::new(rate, Instant::now()),
            requested_active: false,
            delivering: false,
            epoch: 1,
            input: NativeTouchInput::default(),
        })
    }
    pub fn close(&mut self) {
        self.game.take();
        self.surface.detach();
        self.requested_active = false;
        self.delivering = false;
        self.clear_input();
    }
    fn clear_input(&mut self) {
        self.input.reset();
    }
    fn revoke(&mut self) -> Result<()> {
        let Some(next) = self.epoch.checked_add(1) else {
            self.close();
            return Err(NativeHostError::Epoch);
        };
        self.epoch = next;
        self.clear_input();
        Ok(())
    }
    fn suspend(&mut self) -> Result<()> {
        self.surface.cancel();
        self.revoke()?;
        self.game
            .as_mut()
            .ok_or(NativeHostError::Closed)?
            .set_active(false)?;
        self.delivering = false;
        self.clock.reset(Instant::now());
        Ok(())
    }
    /// Requested activation does not start device output until a drawable can
    /// actually be reserved. Backgrounding stops delivery and retires input.
    pub fn set_active(&mut self, active: bool) -> Result<()> {
        if self.game.is_none() {
            return Err(NativeHostError::Closed);
        }
        if self.requested_active == active {
            return Ok(());
        }
        self.requested_active = active;
        if let Err(error) = self.suspend() {
            self.close();
            return Err(error);
        }
        Ok(())
    }
    /// Surface destruction suspends output but preserves the exact game/VM.
    pub fn detach(&mut self) -> Result<()> {
        let result = self.suspend();
        self.surface.detach();
        if result.is_err() {
            self.close();
        }
        result
    }
    /// A replacement surface uses the existing device and retires all old input.
    pub fn attach(
        &mut self,
        window: NativeWindow,
        size: [u32; 2],
        logical: [f32; 2],
    ) -> Result<()> {
        surface::validate_size(size)?;
        validate_logical(logical)?;
        let result = (|| {
            self.suspend()?;
            self.surface.attach(window, size, &self.renderer)?;
            self.game
                .as_mut()
                .ok_or(NativeHostError::Closed)?
                .resize_ui(logical)?;
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    /// Opaque delivery identity. Keep it with each gesture, including its final
    /// callback. Zero means no displayed UI currently accepts new input.
    pub fn delivery_epoch(&self) -> u64 {
        if self.delivering && self.game.as_ref().is_some_and(|game| game.host().is_ok()) {
            self.epoch
        } else {
            0
        }
    }
    /// Called by Java Choreographer. It advances at
    /// the cooked fixed rate with at most four catch-up ticks. Unavailable
    /// drawables defer simulation; no texture survives this callback.
    pub fn frame(&mut self) -> Result<usize> {
        let result = self.frame_inner();
        if result.is_err() {
            self.close();
        }
        result
    }
    fn frame_inner(&mut self) -> Result<usize> {
        if self.game.is_none() {
            return Err(NativeHostError::Closed);
        }
        if !self.requested_active {
            return Ok(0);
        }
        let due = self.clock.due(Instant::now());
        let mut presented = 0;
        for _ in 0..due {
            if !self.surface.reserve(&self.renderer)? {
                if self.delivering {
                    self.suspend()?;
                }
                break;
            }
            if !self.delivering {
                self.revoke()?;
                self.game
                    .as_mut()
                    .ok_or(NativeHostError::Closed)?
                    .set_active(true)?;
                self.delivering = true;
            }
            let game = self.game.as_mut().ok_or(NativeHostError::Closed)?;
            let token = game.delivery_token()?;
            game.tick(
                &token,
                self.input.physical(),
                self.input.receipts(),
                &mut [],
            )?;
            if !self.surface.consumed() {
                return Err(SurfaceError::Reservation.into());
            }
            self.input.accepted();
            self.input.presented(game.host()?.ui())?;
            presented += 1;
        }
        Ok(presented)
    }
    /// Logical-point touch input. A UI press owns its complete gesture and does
    /// not leak into gameplay. Additional simultaneous UI presses are suppressed.
    pub fn touch(
        &mut self,
        epoch: u64,
        id: u32,
        phase: TouchPhase,
        position: [f32; 2],
    ) -> Result<()> {
        if epoch == 0 || epoch != self.delivery_epoch() {
            return Err(NativeHostError::Stale);
        }
        let runtime = self
            .game
            .as_ref()
            .ok_or(NativeHostError::Closed)?
            .host()?
            .ui();
        self.input.touch(runtime, id, phase, position)?;
        Ok(())
    }
}
fn validate_logical(size: [f32; 2]) -> Result<()> {
    if size
        .iter()
        .all(|n| n.is_finite() && (1.0..=65536.0).contains(n))
    {
        Ok(())
    } else {
        Err(NativeHostError::Input)
    }
}
