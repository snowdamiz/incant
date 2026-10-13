use crate::{
    Core, GameError, Result,
    core::WithHost,
    offline::{OfflineCore, validate_rate},
};
use incant_audio_runtime::{AudioManagerSettings, ClipBinding, SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::SessionProgram;
use kira::backend::Backend;
use std::{error::Error, fmt::Debug};

/// Trusted native hosting implementation, not a project-supplied capability flag.
/// `prepare` must check and retain compatible bindings for this exact world and
/// reject unsupported declarations. `sync` must accept the committed frame or
/// fail. It may run repeatedly at the same tick for presentation-only refreshes;
/// it must not advance gameplay or audio. Successful CPU preparation does not
/// imply GPU publication or rendering.
///
/// The session calls prepare before its VM/audio setup and retains the actual
/// host. Caller-owned effects (including an already-created backend in Settings)
/// precede this boundary and cannot be undone. Hosts expose no mutable world.
/// Native Rust implementors are trusted, like audio Backend implementations; this
/// contract does not sandbox arbitrary Rust. No unsafe marker or bypass bool exists.
pub trait SceneHost: Sized {
    /// Trusted compiled consumer admission, checked before host/VM/audio setup.
    const SUPPORTS_UI: bool = false;
    #[cfg(feature = "ui")]
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        None
    }
    /// Stage only: returned input and private host candidate share the next
    /// native tick. Err must preserve the accepted host.
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
    type Settings;
    type Error: Error;
    fn prepare(
        world: &NativeWorld,
        settings: Self::Settings,
    ) -> std::result::Result<Self, Self::Error>;
    /// Restore the typed widget owner and its drawable before preparing external
    /// consumers. Default hosts refuse UI data rather than silently resetting it.
    #[cfg(feature = "ui")]
    fn prepare_checkpoint(
        world: &NativeWorld,
        settings: Self::Settings,
        ui: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> Result<Self, Self::Error> {
        if ui.is_some() {
            return Err(GameError::UiHost);
        }
        Self::prepare(world, settings).map_err(GameError::Host)
    }
    fn sync(&mut self, world: &NativeWorld) -> std::result::Result<(), Self::Error>;
}

/// Explicit scene host + native simulation and backend-driven audio. Each accepted
/// tick synchronizes the retained host last; failure/unwind drops the composite.
/// Submitted device audio and external consumer effects are not rolled back.
pub struct HostedGameSession<B: Backend, H: SceneHost> {
    pub(crate) core: Core<B, WithHost<H>>,
}
impl<B: Backend, H: SceneHost> HostedGameSession<B, H>
where
    B::Error: Debug,
{
    /// Destroy every live owner immediately. Further access fails closed. Native
    /// hosts use this before acquiring resources for a complete replacement.
    pub fn stop(&mut self) {
        self.core.live.take();
    }
    /// Fresh restart-capable composition. Audio is born suspended; no asynchronous
    /// pause command follows an already-playing startup voice.
    pub fn from_seed_suspended(
        seed: &crate::GameSeed,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        let mut game = Self {
            core: Core::prepare(
                seed.scene.clone(),
                seed.program.clone(),
                seed.actions.clone(),
                |world| {
                    H::prepare(world, host_settings)
                        .map(WithHost)
                        .map_err(GameError::Host)
                },
                |world| SceneAudio::new_suspended(world, seed.clips.clone(), audio_settings),
            )?
            .initially_suspended(),
        };
        game.enable_restart_requests()?;
        Ok(game)
    }
    /// Destroy all live state before replacement. Failure leaves this owner
    /// terminal. The fresh session stays paused until explicitly resumed.
    pub fn restart_from_suspended(
        &mut self,
        seed: &crate::GameSeed,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<(), H::Error> {
        self.core.ready()?;
        self.core.live.take();
        *self = Self::from_seed_suspended(seed, audio_settings, host_settings)?;
        Ok(())
    }
    pub fn new(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        Ok(Self {
            core: Core::prepare(
                scene,
                program,
                actions,
                |world| {
                    H::prepare(world, host_settings)
                        .map(WithHost)
                        .map_err(GameError::Host)
                },
                |world| SceneAudio::new(world, clips, audio_settings),
            )?,
        })
    }
    /// Prepare a new retained host on the exact restored world before VM/audio
    /// setup. Old hosts are never reused. External prepare/backend effects are
    /// not rollbackable; this is a candidate factory, not in-place device restore.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        Ok(Self {
            core: Core::from_checkpoint(
                bytes,
                initial,
                program,
                actions,
                |pending| {
                    #[cfg(feature = "ui")]
                    {
                        H::prepare_checkpoint(pending.world(), host_settings, pending.ui())
                            .map(WithHost)
                    }
                    #[cfg(not(feature = "ui"))]
                    {
                        H::prepare(pending.world(), host_settings)
                            .map(WithHost)
                            .map_err(GameError::Host)
                    }
                },
                |world| SceneAudio::new(world, clips, audio_settings),
            )?,
        })
    }
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>, H::Error> {
        self.core.checkpoint_bytes()
    }
    pub fn tick(&mut self, events: &[InputEvent]) -> Result<(), H::Error> {
        self.core.tick(events, |_| Ok(()))
    }
    #[cfg(feature = "ui")]
    pub fn tick_with_ui(
        &mut self,
        events: &[InputEvent],
        receipts: &[incant_ui::UiEvent],
    ) -> Result<(), H::Error> {
        self.core.tick_packet(events, receipts, |_| Ok(()))
    }
    #[cfg(feature = "ui")]
    pub fn resize_ui(&mut self, viewport: [f32; 2]) -> Result<(), H::Error> {
        self.core.resize_ui(viewport)
    }
    pub fn host(&self) -> Result<&H, H::Error> {
        Ok(&self.core.ready()?.host.0)
    }
    /// Re-present the committed world, including while suspended. No script,
    /// physics, input application or audio advancement occurs. Error/unwind
    /// destroys the complete session; submitted device effects cannot rewind.
    pub fn refresh_host(&mut self) -> Result<(), H::Error> {
        self.core.refresh_host()
    }
    pub fn poll_audio(&mut self) -> Result<(), H::Error> {
        self.core.poll_audio()
    }
    /// Suspend transport and reject ticks until resumed; failure is terminal.
    pub fn set_suspended(&mut self, suspended: bool) -> Result<(), H::Error> {
        self.core.set_suspended(suspended)
    }
    pub fn is_failed(&self) -> bool {
        self.core.live.is_none()
    }
    pub fn world(&self) -> Result<&NativeWorld, H::Error> {
        self.core.world()
    }
    pub fn input(&self) -> Result<&InputFrame, H::Error> {
        self.core.input()
    }
    pub fn logs(&self) -> Result<&incant_script::NativeLogs, H::Error> {
        self.core.logs()
    }
    /// Explicit trusted-owner opt-in for whole-session replacement.
    /// True only for a program with the implemented typed haptic capability.
    pub fn supports_haptics(&self) -> Result<bool, H::Error> {
        self.core.supports_haptics()
    }
    /// Current host-issued targets; never reconstructed from saved data.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), H::Error> {
        self.core.set_haptic_capabilities(value)
    }
    /// Drain only accepted composed output. Suspension clears pending requests.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, H::Error> {
        self.core.take_haptics()
    }

    pub fn enable_restart_requests(&mut self) -> Result<(), H::Error> {
        self.core.enable_restart_requests()
    }
    /// Only visible after the complete composed tick succeeds.
    pub fn take_session_request(
        &mut self,
    ) -> Result<Option<incant_script::SessionRequest>, H::Error> {
        self.core.take_session_request()
    }

    pub fn source_state(&self, id: StableId) -> Result<SourceState, H::Error> {
        self.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<(), H::Error> {
        self.core.rebind(actions)
    }
}

/// Explicit retained scene host plus the same rational offline sample clock as
/// OfflineGameSession. Host sync follows native/script/audio sync and rendering;
/// any accepted failure/unwind clears output and hides every live observation.
pub struct HostedOfflineGameSession<H: SceneHost> {
    pub(crate) inner: OfflineCore<WithHost<H>>,
}
impl<H: SceneHost> HostedOfflineGameSession<H> {
    /// Reuse only immutable startup data. The retained host is prepared on the
    /// fresh exact world before guest/audio initialization as on ordinary start.
    pub fn from_seed(
        seed: &crate::GameSeed,
        sample_rate: u32,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        let mut game = Self::new(
            seed.scene.clone(),
            seed.program.clone(),
            seed.actions.clone(),
            seed.clips.clone(),
            sample_rate,
            host_settings,
        )?;
        game.enable_restart_requests()?;
        Ok(game)
    }
    /// Replacement is destructive at an accepted host boundary. Preparation
    /// failure leaves the old composition dropped and this session terminal.
    pub fn restart_from(
        &mut self,
        seed: &crate::GameSeed,
        host_settings: H::Settings,
    ) -> Result<(), H::Error> {
        self.inner.core.ready()?;
        self.inner.core.live.take();
        *self = Self::from_seed(seed, self.inner.sample_rate, host_settings)?;
        Ok(())
    }
    pub fn new(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        validate_rate(sample_rate)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::prepare(
                    scene,
                    program,
                    actions,
                    |world| {
                        H::prepare(world, host_settings)
                            .map(WithHost)
                            .map_err(GameError::Host)
                    },
                    |world| SceneAudio::offline(world, clips, sample_rate),
                )?,
                sample_rate,
            },
        })
    }
    /// Ordinary load. Audio data and its trusted PCM are validated first; a fresh
    /// retained host is then prepared on the exact restored world before guest
    /// initialization. Offline mixer construction creates no physical device.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        validate_rate(sample_rate)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::from_offline_checkpoint(
                    bytes,
                    initial,
                    program,
                    actions,
                    |pending| {
                        #[cfg(feature = "ui")]
                        {
                            H::prepare_checkpoint(pending.world(), host_settings, pending.ui())
                                .map(WithHost)
                        }
                        #[cfg(not(feature = "ui"))]
                        {
                            H::prepare(pending.world(), host_settings)
                                .map(WithHost)
                                .map_err(GameError::Host)
                        }
                    },
                    clips,
                    sample_rate,
                )?,
                sample_rate,
            },
        })
    }
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>, H::Error> {
        self.inner.core.checkpoint_bytes()
    }
    pub fn rendered_frames(&self) -> Result<u128, H::Error> {
        self.inner.rendered_frames()
    }
    pub fn next_output_frames(&self) -> Result<usize, H::Error> {
        self.inner.next_output_frames()
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<(), H::Error> {
        self.inner.tick(events, output)
    }
    #[cfg(feature = "ui")]
    pub fn tick_with_ui(
        &mut self,
        events: &[InputEvent],
        receipts: &[incant_ui::UiEvent],
        output: &mut [f32],
    ) -> Result<(), H::Error> {
        self.inner.tick_packet(events, receipts, output)
    }
    #[cfg(feature = "ui")]
    pub fn resize_ui(&mut self, viewport: [f32; 2]) -> Result<(), H::Error> {
        self.inner.core.resize_ui(viewport)
    }
    pub fn host(&self) -> Result<&H, H::Error> {
        Ok(&self.inner.core.ready()?.host.0)
    }
    /// Re-present committed state without advancing the offline sample clock,
    /// transport, input or simulation. Host error/unwind is terminal.
    pub fn refresh_host(&mut self) -> Result<(), H::Error> {
        self.inner.core.refresh_host()
    }
    pub fn poll_audio(&mut self) -> Result<(), H::Error> {
        self.inner.core.poll_audio()
    }
    /// Suspend transport and reject ticks until resumed; failure is terminal.
    pub fn set_suspended(&mut self, suspended: bool) -> Result<(), H::Error> {
        self.inner.core.set_suspended(suspended)
    }
    pub fn is_failed(&self) -> bool {
        self.inner.core.live.is_none()
    }
    pub fn world(&self) -> Result<&NativeWorld, H::Error> {
        self.inner.core.world()
    }
    pub fn input(&self) -> Result<&InputFrame, H::Error> {
        self.inner.core.input()
    }
    pub fn logs(&self) -> Result<&incant_script::NativeLogs, H::Error> {
        self.inner.core.logs()
    }
    /// Explicit trusted-owner opt-in for whole-session replacement.
    /// True only for a program with the implemented typed haptic capability.
    pub fn supports_haptics(&self) -> Result<bool, H::Error> {
        self.inner.core.supports_haptics()
    }
    /// Current host-issued targets; never reconstructed from saved data.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), H::Error> {
        self.inner.core.set_haptic_capabilities(value)
    }
    /// Drain only accepted composed output. Suspension clears pending requests.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, H::Error> {
        self.inner.core.take_haptics()
    }

    pub fn enable_restart_requests(&mut self) -> Result<(), H::Error> {
        self.inner.core.enable_restart_requests()
    }
    /// Only visible after the complete composed tick succeeds.
    pub fn take_session_request(
        &mut self,
    ) -> Result<Option<incant_script::SessionRequest>, H::Error> {
        self.inner.core.take_session_request()
    }

    pub fn source_state(&self, id: StableId) -> Result<SourceState, H::Error> {
        self.inner.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<(), H::Error> {
        self.inner.core.rebind(actions)
    }
}

#[cfg(all(feature = "device", not(target_arch = "wasm32")))]
impl<H: SceneHost> HostedGameSession<incant_audio_runtime::DeviceBackend, H> {
    /// Poll real device errors. Any reported or discarded error is terminal.
    pub fn poll_device(&mut self) -> Result<(), H::Error> {
        self.core.poll_device()
    }
}
