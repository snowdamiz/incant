use crate::{
    Core, GameError, Result,
    core::WithHost,
    offline::{OfflineCore, validate_rate},
};
use incant_audio_runtime::{AudioManagerSettings, ClipBinding, SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::CompiledScript;
use kira::backend::Backend;
use std::{error::Error, fmt::Debug};

/// Trusted native hosting implementation, not a project-supplied capability flag.
/// `prepare` must check and retain compatible bindings for this exact world and
/// reject unsupported declarations. `sync` must accept the committed frame or
/// fail; successful CPU preparation does not imply GPU publication or rendering.
///
/// The session calls prepare before its VM/audio setup and retains the actual
/// host. Caller-owned effects (including an already-created backend in Settings)
/// precede this boundary and cannot be undone. Hosts expose no mutable world.
/// Native Rust implementors are trusted, like audio Backend implementations; this
/// contract does not sandbox arbitrary Rust. No unsafe marker or bypass bool exists.
pub trait SceneHost: Sized {
    type Settings;
    type Error: Error;
    fn prepare(
        world: &NativeWorld,
        settings: Self::Settings,
    ) -> std::result::Result<Self, Self::Error>;
    fn sync(&mut self, world: &NativeWorld) -> std::result::Result<(), Self::Error>;
}

/// Explicit scene host + native simulation and backend-driven audio. Each accepted
/// tick synchronizes the retained host last; failure/unwind drops the composite.
/// Submitted device audio and external consumer effects are not rolled back.
pub struct HostedGameSession<B: Backend, H: SceneHost> {
    core: Core<B, WithHost<H>>,
}
impl<B: Backend, H: SceneHost> HostedGameSession<B, H>
where
    B::Error: Debug,
{
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        audio_settings: AudioManagerSettings<B>,
        host_settings: H::Settings,
    ) -> Result<Self, H::Error> {
        Ok(Self {
            core: Core::prepare(
                scene,
                script,
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
    pub fn tick(&mut self, events: &[InputEvent]) -> Result<(), H::Error> {
        self.core.tick(events, |_| Ok(()))
    }
    pub fn host(&self) -> Result<&H, H::Error> {
        Ok(&self.core.ready()?.host.0)
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
    inner: OfflineCore<WithHost<H>>,
}
impl<H: SceneHost> HostedOfflineGameSession<H> {
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
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
                    script,
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
    pub fn rendered_frames(&self) -> Result<u128, H::Error> {
        self.inner.rendered_frames()
    }
    pub fn next_output_frames(&self) -> Result<usize, H::Error> {
        self.inner.next_output_frames()
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<(), H::Error> {
        self.inner.tick(events, output)
    }
    pub fn host(&self) -> Result<&H, H::Error> {
        Ok(&self.inner.core.ready()?.host.0)
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
