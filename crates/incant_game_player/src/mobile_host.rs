//! Retained native callback owner for a cooked mobile game. The caller schedules
//! fixed ticks and supplies a real render consumer and PCM sink. CPU observation
//! alone is not a displayed game, device audio or a mobile performance result.
use incant_game_runtime::{GameError, HostedOfflineGameSession};
use incant_input::{InputEvent, MAX_EVENTS_PER_TICK};
use incant_render_game::{
    NativeRenderHost, NativeRenderSettings, RenderHostError, RenderInputConsumer,
};
use incant_runtime::NativeWorld;
use incant_ui::UiEvent;
use std::{path::Path, sync::Arc};
mod game;
pub mod touch;
use game::Game;

#[derive(Debug, thiserror::Error)]
pub enum MobileHostError<E: std::error::Error + 'static> {
    #[error("invalid mobile host settings: {0}")]
    Settings(&'static str),
    #[error("cooked mobile startup failed: {0}")]
    Startup(#[source] Box<dyn std::error::Error>),
    #[error(transparent)]
    Game(#[from] GameError<RenderHostError<E>>),
    #[error("mobile game is suspended")]
    Suspended,
    #[error("mobile game is closed or failed")]
    Failed,
    #[error("mobile input belongs to an obsolete delivery epoch")]
    StaleDelivery,
    #[error("mobile lifecycle neutralization must complete before UI delivery")]
    PendingLifecycle,
    #[error("mobile input packet exceeds its bounded capacity")]
    InputLimit,
    #[error("mobile lifecycle epoch exhausted")]
    EpochLimit,
}

/// Opaque, process-local authority issued when a platform begins input delivery.
/// Keep it with the queued event; do not replace it with a freshly obtained token
/// when that event is eventually dispatched. Never serialize it into game data.
#[derive(Clone, Debug)]
pub struct DeliveryToken {
    owner: Arc<()>,
    epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickReceipt {
    pub simulation_tick: u64,
    pub stereo_frames: usize,
}

/// One native world, VM, render/UI owner and mixer across callbacks.
/// Methods require exclusive access on the platform's game thread. Construction
/// loads only cooked data and the trusted program linked into this build.
pub struct MobileSession<C: RenderInputConsumer> {
    game: Option<Game<C>>,
    owner: Arc<()>,
    epoch: u64,
    active: bool,
    neutralize: bool,
    physical: Vec<InputEvent>,
}
type Result<T, C> = std::result::Result<T, MobileHostError<<C as RenderInputConsumer>::Error>>;

impl<C: RenderInputConsumer> MobileSession<C> {
    /// Starts suspended and does not present a frame. A consumer may allocate
    /// device resources during preparation; its failures retain the normal
    /// composed-host semantics. No output path or input recording is executed.
    pub fn open(
        package: &Path,
        sample_rate: u32,
        ui_viewport: Option<[f32; 2]>,
        consumer: C::Settings,
    ) -> Result<Self, C> {
        if !(8_000..=192_000).contains(&sample_rate) {
            return Err(MobileHostError::Settings("sample rate"));
        }
        if ui_viewport.is_some_and(|v| !valid_viewport(v)) {
            return Err(MobileHostError::Settings("logical viewport"));
        }
        let package =
            crate::package::hosted(package, ui_viewport).map_err(MobileHostError::Startup)?;
        let s = package.settings;
        let mut game = HostedOfflineGameSession::new(
            package.scene,
            crate::program::load(None).map_err(MobileHostError::Startup)?,
            package.actions,
            package.clips,
            sample_rate,
            NativeRenderSettings {
                model_cache: s.model_cache,
                texture_cache: s.texture_cache,
                camera: s.camera,
                text: s.text,
                ui: s.ui,
                consumer,
            },
        )?;
        game.set_suspended(true)?;
        Ok(Self::owned(Game::Offline(Box::new(game))))
    }

    /// Starts the real platform audio backend suspended. No PCM buffer is
    /// returned: `next_output_frames` is zero and `tick` accepts an empty slice.
    /// Audio already submitted to an OS device cannot be rolled back on a late
    /// GPU failure. Device availability is checked, never silently simulated.
    #[cfg(feature = "mobile-device")]
    pub fn open_device(
        package: &Path,
        ui_viewport: Option<[f32; 2]>,
        consumer: C::Settings,
    ) -> Result<Self, C> {
        if ui_viewport.is_some_and(|v| !valid_viewport(v)) {
            return Err(MobileHostError::Settings("logical viewport"));
        }
        let package =
            crate::package::hosted(package, ui_viewport).map_err(MobileHostError::Startup)?;
        let s = package.settings;
        let mut game = incant_game_runtime::HostedGameSession::new_suspended(
            package.scene,
            crate::program::load(None).map_err(MobileHostError::Startup)?,
            package.actions,
            package.clips,
            incant_audio_runtime::AudioManagerSettings::default(),
            NativeRenderSettings {
                model_cache: s.model_cache,
                texture_cache: s.texture_cache,
                camera: s.camera,
                text: s.text,
                ui: s.ui,
                consumer,
            },
        )?;
        game.poll_device()?;
        Ok(Self::owned(Game::Device(Box::new(game))))
    }

    fn owned(game: Game<C>) -> Self {
        Self {
            game: Some(game),
            owner: Arc::new(()),
            epoch: 0,
            active: false,
            neutralize: true,
            physical: Vec::with_capacity(MAX_EVENTS_PER_TICK),
        }
    }

    fn ready(&self) -> Result<&Game<C>, C> {
        self.game
            .as_ref()
            .filter(|g| !g.is_failed())
            .ok_or(MobileHostError::Failed)
    }

    fn next_epoch(&mut self) -> Result<u64, C> {
        match self.epoch.checked_add(1) {
            Some(next) => Ok(next),
            None => {
                self.close();
                Err(MobileHostError::EpochLimit)
            }
        }
    }

    /// Explicitly retire every resource; repeated close is harmless. Dropping
    /// this owner also drops the game. A new owner cannot accept its old tokens.
    pub fn close(&mut self) {
        self.game.take();
        self.active = false;
        self.physical.clear();
    }

    /// Suspension stops ticks without advancing simulation or PCM chronology.
    /// Both transitions revoke queued platform input. Resume first neutralizes
    /// held physical controls and the old UI composition in the next real tick.
    pub fn set_active(&mut self, active: bool) -> Result<(), C> {
        self.ready()?;
        if self.active == active {
            return Ok(());
        }
        let next = self.next_epoch()?;
        let mut game = self.game.take().ok_or(MobileHostError::Failed)?;
        game.set_suspended(!active)?;
        self.game = Some(game);
        self.epoch = next;
        self.active = active;
        self.neutralize = true;
        self.physical.clear();
        Ok(())
    }

    pub fn delivery_token(&self) -> Result<DeliveryToken, C> {
        self.ready()?;
        if !self.active {
            return Err(MobileHostError::Suspended);
        }
        Ok(DeliveryToken {
            owner: self.owner.clone(),
            epoch: self.epoch,
        })
    }

    pub fn world(&self) -> Result<&NativeWorld, C> {
        Ok(self.ready()?.world()?)
    }

    /// Read-only accepted host. Retired focus/pointer tokens are not exposed
    /// during suspension or before the resumed neutralizing tick completes.
    pub fn host(&self) -> Result<&NativeRenderHost<C>, C> {
        self.ready()?;
        if !self.active {
            return Err(MobileHostError::Suspended);
        }
        if self.neutralize {
            return Err(MobileHostError::PendingLifecycle);
        }
        Ok(self.ready()?.host()?)
    }

    pub fn next_output_frames(&self) -> Result<usize, C> {
        Ok(self.ready()?.next_output_frames()?)
    }

    /// Exactly one scheduled fixed tick and its rational stereo PCM slice.
    /// Wrong lifecycle/token/count/buffer inputs are rejected before mutation.
    /// Accepted guest/audio/consumer failure destroys the composite and silences
    /// output; already submitted external GPU effects cannot be rolled back.
    pub fn tick(
        &mut self,
        token: &DeliveryToken,
        events: &[InputEvent],
        ui: &[UiEvent],
        output: &mut [f32],
    ) -> Result<TickReceipt, C> {
        self.ready()?;
        if !self.active {
            return Err(MobileHostError::Suspended);
        }
        if !Arc::ptr_eq(&token.owner, &self.owner) || token.epoch != self.epoch {
            return Err(MobileHostError::StaleDelivery);
        }
        if self.neutralize && !ui.is_empty() {
            return Err(MobileHostError::PendingLifecycle);
        }
        let prefix = if self.neutralize { 2 } else { 0 };
        if events.len() > MAX_EVENTS_PER_TICK - prefix || ui.len() > incant_ui::MAX_UI_EVENTS {
            return Err(MobileHostError::InputLimit);
        }
        let frames = self.next_output_frames()?;
        if output.len() != frames * 2 {
            return Err(MobileHostError::Game(GameError::Buffer {
                expected: frames * 2,
                actual: output.len(),
            }));
        }
        self.physical.clear();
        if self.neutralize {
            self.physical.extend([
                InputEvent::Focus { focused: false },
                InputEvent::Focus { focused: true },
            ]);
        }
        self.physical.extend_from_slice(events);
        let game = self.game.as_mut().ok_or(MobileHostError::Failed)?;
        game.tick_with_ui(&self.physical, ui, output)?;
        self.neutralize = false;
        self.physical.clear();
        Ok(TickReceipt {
            simulation_tick: game.world()?.tick(),
            stereo_frames: frames,
        })
    }

    /// Relayout is separate from simulation. It revokes older async delivery
    /// only after the accepted resize; no hidden tick or PCM is generated.
    pub fn resize_ui(&mut self, viewport: [f32; 2]) -> Result<(), C> {
        self.ready()?;
        if !valid_viewport(viewport) {
            return Err(MobileHostError::Settings("logical viewport"));
        }
        let next = self.next_epoch()?;
        self.game
            .as_mut()
            .ok_or(MobileHostError::Failed)?
            .resize_ui(viewport)?;
        self.epoch = next;
        Ok(())
    }
}

fn valid_viewport(v: [f32; 2]) -> bool {
    v.into_iter()
        .all(|n| n.is_finite() && (1.0..=65_536.0).contains(&n))
}

#[cfg(test)]
mod tests;
