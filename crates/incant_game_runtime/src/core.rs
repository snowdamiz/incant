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
        prepare: impl FnOnce(&PendingSessionCheckpoint) -> Result<H, H::Error>,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self, H::Error> {
        let pending = PendingSessionCheckpoint::new(bytes, initial, program, actions)?;
        Self::from_pending_checkpoint(pending, prepare, audio)
    }
    pub(crate) fn from_pending_checkpoint(
        pending: PendingSessionCheckpoint,
        prepare: impl FnOnce(&PendingSessionCheckpoint) -> Result<H, H::Error>,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self, H::Error> {
        if pending.audio().is_some() {
            return Err(GameError::Audio(
                incant_audio_runtime::AudioError::Unsupported(
                    "backend-driven audio restore; use the offline composed session",
                ),
            ));
        }
        if pending.world().requires_ui_checkpoint() && (!cfg!(feature = "ui") || !H::SUPPORTS_UI) {
            return Err(GameError::UiHost);
        }
        let host = prepare(&pending)?;
        #[cfg(feature = "ui")]
        let native = if pending.ui().is_some() {
            pending.start_with_ui(host.ui().ok_or(GameError::UiHost)?)?
        } else {
            pending.start()?
        };
        #[cfg(not(feature = "ui"))]
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
        let live = self.ready()?;
        if live.native.world()?.requires_audio_checkpoint() {
            return Ok(live.native.checkpoint_bytes_with_audio(
                &live.audio,
                #[cfg(feature = "ui")]
                live.host.ui(),
            )?);
        }
        #[cfg(feature = "ui")]
        if live.native.world()?.requires_ui_checkpoint() {
            return Ok(live
                .native
                .checkpoint_bytes_with_ui(live.host.ui().ok_or(GameError::UiHost)?)?);
        }
        Ok(live.native.checkpoint_bytes()?)
    }
    #[cfg(feature = "native-checkpoints")]
    pub(crate) fn checkpoint(
        &mut self,
        sample_rate: Option<u32>,
        payload: &mut incant_runtime::CheckpointPayloadBudget,
        planar_work: &mut incant_runtime::PlanarCheckpointWorkBudget,
        spatial_work: &mut incant_runtime::SpatialCheckpointWorkBudget,
    ) -> Result<crate::GameCheckpoint, H::Error> {
        let suspended = self.suspended;
        let live = self.ready_mut()?;
        // Even a silent offline mixer owns the absolute rendered-sample clock.
        // Retain it before a later audio declaration can make it observable.
        let native = if sample_rate.is_some() || live.native.world()?.requires_audio_checkpoint() {
            live.native.checkpoint_with_audio(
                &live.audio,
                #[cfg(feature = "ui")]
                live.host.ui(),
                payload,
                planar_work,
                spatial_work,
            )?
        } else {
            #[cfg(feature = "ui")]
            if live.native.world()?.requires_ui_checkpoint() {
                live.native.checkpoint_with_ui(
                    live.host.ui().ok_or(GameError::UiHost)?,
                    payload,
                    planar_work,
                    spatial_work,
                )?
            } else {
                live.native.checkpoint(payload, planar_work, spatial_work)?
            }
            #[cfg(not(feature = "ui"))]
            live.native.checkpoint(payload, planar_work, spatial_work)?
        };
        Ok(crate::GameCheckpoint {
            native,
            sample_rate,
            suspended,
        })
    }
    #[cfg(feature = "native-checkpoints")]
    pub(crate) fn with_checkpoint_suspension(mut self, suspended: bool) -> Result<Self, H::Error> {
        if self.suspended != suspended {
            self.set_suspended(suspended)?;
        }
        Ok(self)
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
    pub fn supports_haptics(&self) -> Result<bool, H::Error> {
        Ok(self.ready()?.native.supports_haptics()?)
    }
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), H::Error> {
        Ok(self.ready_mut()?.native.set_haptic_capabilities(value)?)
    }
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, H::Error> {
        // Audio and the mandatory host have accepted before Live is published.
        Ok(self.ready_mut()?.native.take_haptics()?)
    }
    pub fn enable_restart_requests(&mut self) -> Result<(), H::Error> {
        Ok(self.ready_mut()?.native.enable_restart_requests()?)
    }
    pub fn take_session_request(
        &mut self,
    ) -> Result<Option<incant_script::SessionRequest>, H::Error> {
        // The entire live composition is absent throughout the accepted operation.
        // A later audio/host failure drops its native request together with it.
        Ok(self.ready_mut()?.native.take_session_request()?)
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
        if suspended {
            live.native.clear_haptics();
        }
        self.suspended = suspended;
        self.live = Some(live);
        Ok(())
    }
    /// Only for constructors that created SceneAudio with its private master
    /// initially paused. No backend command is issued by this bookkeeping.
    pub(crate) fn initially_suspended(mut self) -> Self {
        self.suspended = true;
        self
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
            let _profile = incant_profile::scope(incant_profile::Phase::UiStage);
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
        {
            let _profile = incant_profile::scope(incant_profile::Phase::AudioSync);
            live.audio.sync(live.native.world()?)?;
        }
        {
            let _profile = incant_profile::scope(incant_profile::Phase::AudioRender);
            render(&mut live.audio)?;
        }
        {
            let _profile = incant_profile::scope(incant_profile::Phase::HostSync);
            live.host
                .sync(live.native.world()?)
                .map_err(GameError::Host)?;
        }
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

impl<H: Hook> Core<incant_audio_runtime::OfflineBackend, H> {
    pub fn from_offline_checkpoint(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        prepare: impl FnOnce(&PendingSessionCheckpoint) -> Result<H, H::Error>,
        clips: Vec<incant_audio_runtime::ClipBinding>,
        sample_rate: u32,
    ) -> Result<Self, H::Error> {
        let pending = PendingSessionCheckpoint::new(bytes, initial, program, actions)?;
        Self::from_pending_offline_checkpoint(pending, prepare, clips, sample_rate)
    }
    pub(crate) fn from_pending_offline_checkpoint(
        pending: PendingSessionCheckpoint,
        prepare: impl FnOnce(&PendingSessionCheckpoint) -> Result<H, H::Error>,
        clips: Vec<incant_audio_runtime::ClipBinding>,
        sample_rate: u32,
    ) -> Result<Self, H::Error> {
        if pending.world().requires_ui_checkpoint() && (!cfg!(feature = "ui") || !H::SUPPORTS_UI) {
            return Err(GameError::UiHost);
        }
        if let Some(state) = pending.audio() {
            // Catalog, every transport, and this exact world are checked before
            // any guest initialization or external render consumer preparation.
            let suspended = state.suspended();
            let audio = state.restore(pending.world(), clips, sample_rate)?;
            let host = prepare(&pending)?;
            let native = pending.start_with_audio(
                &audio,
                #[cfg(feature = "ui")]
                host.ui(),
            )?;
            Ok(Self {
                suspended,
                live: Some(Live {
                    audio,
                    host,
                    native,
                }),
            })
        } else {
            // Preserve established host-before-VM/backend ordering for old
            // audio-free checkpoints, including byte-for-byte v1/v2 framing.
            let host = prepare(&pending)?;
            #[cfg(feature = "ui")]
            let native = if pending.ui().is_some() {
                pending.start_with_ui(host.ui().ok_or(GameError::UiHost)?)?
            } else {
                pending.start()?
            };
            #[cfg(not(feature = "ui"))]
            let native = pending.start()?;
            let audio = SceneAudio::offline(native.world()?, clips, sample_rate)?;
            Ok(Self {
                suspended: false,
                live: Some(Live {
                    audio,
                    host,
                    native,
                }),
            })
        }
    }
}
