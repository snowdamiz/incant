//! Authoring-free composition of native input/VM/physics and retained scene audio.
//! Input rejection is retryable. Accepted ticks run simulation, script, audio
//! sync, (offline only) render, then the explicitly retained scene host if present.
//! Any post-acceptance failure or unwind destroys the live session; neither the
//! world nor asynchronous device cursors are rolled back. Plain constructors
//! refuse spatial declarations. Hosted variants require a trusted native host.
mod core;
#[cfg(feature = "native-checkpoints")]
mod native_checkpoint;
#[cfg(feature = "native-checkpoints")]
pub use native_checkpoint::{GameCheckpoint, MAX_PERSISTED_GAME_BYTES, PendingGameCheckpoint};
#[cfg(feature = "ui")]
pub struct RoutedUiInput {
    pub events: Vec<incant_input::InputEvent>,
    pub snapshot: incant_input::UiSnapshot,
}
mod hosted;
mod offline;
mod seed;
use core::{Core, NoHost};
pub use hosted::{HostedGameSession, HostedOfflineGameSession, SceneHost};
use incant_audio_runtime::{
    AudioError, AudioManagerSettings, ClipBinding, SceneAudio, SourceState,
};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{SessionError, SessionProgram};
use kira::backend::Backend;
pub use offline::OfflineGameSession;
pub use seed::GameSeed;
use std::{convert::Infallible, fmt::Debug};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError<E = Infallible> {
    #[error(transparent)]
    Checkpoint(#[from] incant_script::CheckpointError),
    #[error("scene host failed: {0}")]
    Host(#[source] E),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("game session failed; world, input and audio state are unavailable")]
    Failed,
    #[error("offline stereo buffer has {actual} samples; this tick requires exactly {expected}")]
    Buffer { expected: usize, actual: usize },
    #[error("offline sample rate must be within 8000..=192000 Hz")]
    SampleRate,
    #[error("this game session has no spatial host")]
    SpatialHost,
    #[error("this game session has no text host")]
    TextHost,
    #[error("this game session has no compatible retained UI host")]
    UiHost,
    #[error("native sample clock exhausted")]
    Clock,
    #[error("game session is suspended; resume before advancing simulation")]
    Suspended,
}
pub type Result<T, E = Infallible> = std::result::Result<T, GameError<E>>;

fn require_nonspatial(scene: &CookedScene) -> Result<()> {
    if scene.has_ui() {
        return Err(GameError::UiHost);
    }
    if scene.has_text() {
        return Err(GameError::TextHost);
    }
    if !scene.spatial().is_empty() || !scene.visual_assets().is_empty() {
        return Err(GameError::SpatialHost);
    }
    Ok(())
}

fn require_nonspatial_world(world: &NativeWorld) -> Result<()> {
    if world.ui_view().map_err(SessionError::from)?.nodes().len() != 0 {
        return Err(GameError::UiHost);
    }
    let view = world.spatial_view().map_err(SessionError::from)?;
    if view.has_text() {
        return Err(GameError::TextHost);
    }
    if view.nodes().any(|node| node.declaration.is_some()) || !view.assets().is_empty() {
        return Err(GameError::SpatialHost);
    }
    Ok(())
}

/// Backend-driven playback. The backend owns its asynchronous audio clock.
/// For deterministic pulled PCM use OfflineGameSession, whose tick requires an
/// exact rational-clock buffer. This type exposes no mixer or mutable world.
pub struct GameSession<B: Backend> {
    core: Core<B>,
}
impl<B: Backend> GameSession<B>
where
    B::Error: Debug,
{
    pub fn from_seed_suspended(seed: &GameSeed, settings: AudioManagerSettings<B>) -> Result<Self> {
        require_nonspatial(&seed.scene)?;
        let mut game = Self {
            core: Core::prepare(
                seed.scene.clone(),
                seed.program.clone(),
                seed.actions.clone(),
                |_| Ok(NoHost),
                |world| SceneAudio::new_suspended(world, seed.clips.clone(), settings),
            )?
            .initially_suspended(),
        };
        game.enable_restart_requests()?;
        Ok(game)
    }
    pub fn restart_from_suspended(
        &mut self,
        seed: &GameSeed,
        settings: AudioManagerSettings<B>,
    ) -> Result<()> {
        self.core.ready()?;
        self.core.live.take();
        *self = Self::from_seed_suspended(seed, settings)?;
        Ok(())
    }
    pub fn new(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        settings: AudioManagerSettings<B>,
    ) -> Result<Self> {
        require_nonspatial(&scene)?;
        Ok(Self {
            core: Core::prepare(
                scene,
                program,
                actions,
                |_| Ok(NoHost),
                |world| SceneAudio::new(world, clips, settings),
            )?,
        })
    }
    /// Ordinary load of an opted-in, audio/physics-free bytecode session.
    /// Decode and trusted scene validation precede VM/backend creation. A fresh
    /// host is constructed; this never resumes an old asynchronous audio cursor.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        settings: AudioManagerSettings<B>,
    ) -> Result<Self> {
        require_nonspatial(initial)?;
        Ok(Self {
            core: Core::from_checkpoint(
                bytes,
                initial,
                program,
                actions,
                |pending| {
                    require_nonspatial_world(pending.world())?;
                    Ok(NoHost)
                },
                |world| SceneAudio::new(world, clips, settings),
            )?,
        })
    }
    /// No clocks or host state advance. Physics/audio-used and static sessions refuse.
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>> {
        self.core.checkpoint_bytes()
    }
    pub fn tick(&mut self, events: &[InputEvent]) -> Result<()> {
        self.core.tick(events, |_| Ok(()))
    }
    /// Poll failures between simulation ticks; a backend/decoder error destroys
    /// the session even when no new input packet has been accepted.
    pub fn poll_audio(&mut self) -> Result<()> {
        self.core.poll_audio()
    }
    /// Pause/resume master audio and simulation together. Errors destroy the
    /// session. Device commands take effect at the next callback boundary.
    pub fn set_suspended(&mut self, suspended: bool) -> Result<()> {
        self.core.set_suspended(suspended)
    }
    pub fn is_failed(&self) -> bool {
        self.core.live.is_none()
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        self.core.world()
    }
    pub fn input(&self) -> Result<&InputFrame> {
        self.core.input()
    }
    pub fn logs(&self) -> Result<&incant_script::NativeLogs> {
        self.core.logs()
    }
    /// Explicit trusted-owner opt-in for whole-session replacement.
    /// True only for a program with the implemented typed haptic capability.
    pub fn supports_haptics(&self) -> Result<bool> {
        self.core.supports_haptics()
    }
    /// Current host-issued targets; never reconstructed from saved data.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<()> {
        self.core.set_haptic_capabilities(value)
    }
    /// Drain only accepted composed output. Suspension clears pending requests.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests> {
        self.core.take_haptics()
    }

    pub fn enable_restart_requests(&mut self) -> Result<()> {
        self.core.enable_restart_requests()
    }
    /// Only visible after the complete composed tick succeeds.
    pub fn take_session_request(&mut self) -> Result<Option<incant_script::SessionRequest>> {
        self.core.take_session_request()
    }

    pub fn source_state(&self, id: StableId) -> Result<SourceState> {
        self.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<()> {
        self.core.rebind(actions)
    }
}

#[cfg(all(feature = "device", not(target_arch = "wasm32")))]
impl GameSession<incant_audio_runtime::DeviceBackend> {
    /// Poll CPAL device errors separately from generic decoder errors. Reported
    /// or discarded errors destroy the session; device observation is asynchronous.
    pub fn poll_device(&mut self) -> Result<()> {
        self.core.poll_device()
    }
}
