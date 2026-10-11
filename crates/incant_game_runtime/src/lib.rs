//! Authoring-free composition of native input/VM/physics and retained scene audio.
//! Input rejection is retryable. Accepted ticks run simulation, script, audio
//! sync, (offline only) render, then the explicitly retained scene host if present.
//! Any post-acceptance failure or unwind destroys the live session; neither the
//! world nor asynchronous device cursors are rolled back. Plain constructors
//! refuse spatial declarations. Hosted variants require a trusted native host.
mod core;
mod hosted;
mod offline;
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
use std::{convert::Infallible, fmt::Debug};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError<E = Infallible> {
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
    #[error("native sample clock exhausted")]
    Clock,
}
pub type Result<T, E = Infallible> = std::result::Result<T, GameError<E>>;

fn require_nonspatial(scene: &CookedScene) -> Result<()> {
    if !scene.spatial().is_empty() || !scene.visual_assets().is_empty() {
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
    pub fn tick(&mut self, events: &[InputEvent]) -> Result<()> {
        self.core.tick(events, |_| Ok(()))
    }
    /// Poll failures between simulation ticks; a backend/decoder error destroys
    /// the session even when no new input packet has been accepted.
    pub fn poll_audio(&mut self) -> Result<()> {
        self.core.poll_audio()
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
    pub fn source_state(&self, id: StableId) -> Result<SourceState> {
        self.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<()> {
        self.core.rebind(actions)
    }
}
