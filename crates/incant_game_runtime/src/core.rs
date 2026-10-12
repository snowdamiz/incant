use crate::{GameError, Result, SceneHost};
use incant_audio_runtime::{SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{
    NativeSession, PendingNativeSession, PendingSessionCheckpoint, SessionProgram,
};
use kira::backend::Backend;
use std::{convert::Infallible, error::Error, fmt::Debug};

pub(crate) trait Hook {
    const SUPPORTS_UI: bool = false;
    #[cfg(feature = "ui")]
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        None
    }
    #[cfg(feature = "ui")]
    fn stage_ui(
        &mut self,
        _world: &NativeWorld,
        _events: &[InputEvent],
        _receipts: &[incant_ui::UiEvent],
    ) -> std::result::Result<Option<crate::RoutedUiInput>, Self::Error> {
        Ok(None)
    }
    #[cfg(feature = "ui")]
    fn cancel_ui(&mut self) {}
    #[cfg(feature = "ui")]
    fn resize_ui(
        &mut self,
        _world: &NativeWorld,
        _viewport: [f32; 2],
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
    type Error: Error;
    fn sync(&mut self, world: &NativeWorld) -> std::result::Result<(), Self::Error>;
}
pub(crate) struct NoHost;
impl Hook for NoHost {
    type Error = Infallible;
    fn sync(&mut self, _: &NativeWorld) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}
pub(crate) struct WithHost<H: SceneHost>(pub H);
impl<H: SceneHost> Hook for WithHost<H> {
    const SUPPORTS_UI: bool = H::SUPPORTS_UI;
    #[cfg(feature = "ui")]
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.0.ui()
    }
    #[cfg(feature = "ui")]
    fn stage_ui(
        &mut self,
        world: &NativeWorld,
        events: &[InputEvent],
        receipts: &[incant_ui::UiEvent],
    ) -> std::result::Result<Option<crate::RoutedUiInput>, Self::Error> {
        self.0.stage_ui(world, events, receipts)
    }
    #[cfg(feature = "ui")]
    fn cancel_ui(&mut self) {
        self.0.cancel_ui();
    }
    #[cfg(feature = "ui")]
    fn resize_ui(
        &mut self,
        world: &NativeWorld,
        viewport: [f32; 2],
    ) -> std::result::Result<(), Self::Error> {
        self.0.resize_ui(world, viewport)
    }
    type Error = H::Error;
    fn sync(&mut self, world: &NativeWorld) -> std::result::Result<(), Self::Error> {
        self.0.sync(world)
    }
}
pub(crate) struct Live<B: Backend, H: Hook> {
    pub audio: SceneAudio<B>,
    pub host: H,
    pub native: NativeSession,
}
pub(crate) struct Core<B: Backend, H: Hook = NoHost> {
    pub live: Option<Live<B, H>>,
    suspended: bool,
}
impl<B: Backend, H: Hook> Core<B, H>
where
    B::Error: Debug,
{
    pub fn prepare(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        prepare: impl FnOnce(&NativeWorld) -> Result<H, H::Error>,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self, H::Error> {
        if scene.has_ui() && (!cfg!(feature = "ui") || !H::SUPPORTS_UI) {
            return Err(GameError::UiHost);
        }
        let pending = PendingNativeSession::new(scene, actions)?;
        let host = prepare(pending.world())?;
        #[cfg(feature = "ui")]
        if pending
            .world()
            .ui_view()
            .map_err(incant_script::SessionError::from)?
            .nodes()
            .len()
            != 0
            && host.ui().is_none()
        {
            return Err(GameError::UiHost);
        }
        let native = pending.start(program)?;
        let audio = audio(native.world()?)?;
        Ok(Self {
            suspended: false,
            live: Some(Live {
                audio,
                host,
                native,
            }),
        })
    }
    pub fn from_checkpoint(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        prepare: impl FnOnce(&NativeWorld) -> Result<H, H::Error>,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self, H::Error> {
        let pending = PendingSessionCheckpoint::new(bytes, initial, program, actions)?;
        let host = prepare(pending.world())?;
        let native = pending.start()?;
        let audio = audio(native.world()?)?;
        Ok(Self {
            suspended: false,
            live: Some(Live {
                audio,
                host,
                native,
            }),
        })
    }
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>, H::Error> {
        Ok(self.ready()?.native.checkpoint_bytes()?)
    }
    pub fn ready(&self) -> Result<&Live<B, H>, H::Error> {
        self.live.as_ref().ok_or(GameError::Failed)
    }
    fn ready_mut(&mut self) -> Result<&mut Live<B, H>, H::Error> {
        self.live.as_mut().ok_or(GameError::Failed)
    }
    pub fn world(&self) -> Result<&NativeWorld, H::Error> {
        Ok(self.ready()?.native.world()?)
    }
    pub fn input(&self) -> Result<&InputFrame, H::Error> {
        Ok(self.ready()?.native.input()?)
    }
    pub fn logs(&self) -> Result<&incant_script::NativeLogs, H::Error> {
        Ok(self.ready()?.native.logs()?)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<(), H::Error> {
        Ok(self.ready_mut()?.native.rebind(actions)?)
    }
    pub fn source_state(&self, id: StableId) -> Result<SourceState, H::Error> {
        Ok(self.ready()?.audio.source_state(id)?)
    }
    pub fn poll_audio(&mut self) -> Result<(), H::Error> {
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        live.audio.check_errors()?;
        self.live = Some(live);
        Ok(())
    }
    pub fn set_suspended(&mut self, suspended: bool) -> Result<(), H::Error> {
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        live.audio.set_suspended(suspended)?;
        self.suspended = suspended;
        self.live = Some(live);
        Ok(())
    }
    pub fn refresh_host(&mut self) -> Result<(), H::Error> {
        // Presentation of the current committed world is independent of the
        // simulation clock and audio transport. External effects still make
        // host failure/unwind terminal, exactly as in an accepted tick.
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        live.host
            .sync(live.native.world()?)
            .map_err(GameError::Host)?;
        self.live = Some(live);
        Ok(())
    }
    pub fn tick(
        &mut self,
        events: &[InputEvent],
        render: impl FnOnce(&mut SceneAudio<B>) -> incant_audio_runtime::Result<()>,
    ) -> Result<(), H::Error> {
        self.tick_packet(
            events,
            #[cfg(feature = "ui")]
            &[],
            render,
        )
    }
    #[cfg(feature = "ui")]
    pub fn resize_ui(&mut self, viewport: [f32; 2]) -> Result<(), H::Error> {
        if self.ready()?.host.ui().is_none() {
            return Err(GameError::UiHost);
        }
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        // Host resize is a prepared transaction; malformed viewports preserve
        // the accepted host and every simulation/input/audio clock.
        if let Err(error) = live.host.resize_ui(live.native.world()?, viewport) {
            self.live = Some(live);
            return Err(GameError::Host(error));
        }
        self.live = Some(live);
        Ok(())
    }
    pub fn tick_packet(
        &mut self,
        events: &[InputEvent],
        #[cfg(feature = "ui")] receipts: &[incant_ui::UiEvent],
        render: impl FnOnce(&mut SceneAudio<B>) -> incant_audio_runtime::Result<()>,
    ) -> Result<(), H::Error> {
        self.ready()?;
        if self.suspended {
            return Err(GameError::Suspended);
        }
        // Keep the public state absent until the complete accepted operation
        // succeeds. Err and Rust unwind drop every live subsystem; no catch or
        // whole-world clone is needed. Abort cannot run cleanup.
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        #[cfg(feature = "ui")]
        let staged = {
            if !receipts.is_empty() && live.host.ui().is_none() {
                self.live = Some(live);
                return Err(GameError::UiHost);
            }
            match live.host.stage_ui(live.native.world()?, events, receipts) {
                Ok(value) => value,
                Err(error) => {
                    live.host.cancel_ui();
                    self.live = Some(live);
                    return Err(GameError::Host(error));
                }
            }
        };
        #[cfg(feature = "ui")]
        let result = if let Some(input) = staged {
            live.native.tick_with_ui(&input.events, input.snapshot)
        } else {
            live.native.tick(events)
        };
        #[cfg(not(feature = "ui"))]
        let result = live.native.tick(events);
        if let Err(error) = result {
            if !live.native.is_failed() {
                #[cfg(feature = "ui")]
                live.host.cancel_ui();
                self.live = Some(live);
            }
            return Err(error.into());
        }
        if live
            .native
            .world()?
            .ui_view()
            .map_err(incant_script::SessionError::from)?
            .nodes()
            .len()
            != 0
            && (!cfg!(feature = "ui") || !H::SUPPORTS_UI)
        {
            return Err(GameError::UiHost);
        }
        live.audio.sync(live.native.world()?)?;
        render(&mut live.audio)?;
        live.host
            .sync(live.native.world()?)
            .map_err(GameError::Host)?;
        self.live = Some(live);
        Ok(())
    }
}

#[cfg(all(feature = "device", not(target_arch = "wasm32")))]
impl<H: Hook> Core<incant_audio_runtime::DeviceBackend, H> {
    pub fn poll_device(&mut self) -> Result<(), H::Error> {
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        live.audio.check_device()?;
        self.live = Some(live);
        Ok(())
    }
}
