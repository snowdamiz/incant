//! Authoring-free composition of native input/VM/physics and retained scene audio.
//! Input rejection is retryable. Accepted ticks run simulation, script, audio
//! sync, then (offline only) render; any later failure destroys the entire live
//! session. Neither world nor asynchronous device cursors are rolled back.
mod offline;
use incant_audio_runtime::{
    AudioError, AudioManagerSettings, ClipBinding, SceneAudio, SourceState,
};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, NativeSession, SessionError};
use kira::backend::Backend;
pub use offline::OfflineGameSession;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
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
pub type Result<T> = std::result::Result<T, GameError>;

struct Live<B: Backend> {
    audio: SceneAudio<B>,
    native: NativeSession,
}
struct Core<B: Backend> {
    live: Option<Live<B>>,
}
impl<B: Backend> Core<B>
where
    B::Error: Debug,
{
    fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self> {
        if !scene.spatial().is_empty() || !scene.visual_assets().is_empty() {
            return Err(GameError::SpatialHost);
        }
        let native = NativeSession::new(scene, script, actions)?;
        let audio = audio(native.world()?)?;
        Ok(Self {
            live: Some(Live { audio, native }),
        })
    }
    fn ready(&self) -> Result<&Live<B>> {
        self.live.as_ref().ok_or(GameError::Failed)
    }
    fn ready_mut(&mut self) -> Result<&mut Live<B>> {
        self.live.as_mut().ok_or(GameError::Failed)
    }
    fn world(&self) -> Result<&NativeWorld> {
        Ok(self.ready()?.native.world()?)
    }
    fn input(&self) -> Result<&InputFrame> {
        Ok(self.ready()?.native.input()?)
    }
    fn rebind(&mut self, actions: InputActions) -> Result<()> {
        Ok(self.ready_mut()?.native.rebind(actions)?)
    }
    fn source_state(&self, id: StableId) -> Result<SourceState> {
        Ok(self.ready()?.audio.source_state(id)?)
    }
    fn poll_audio(&mut self) -> Result<()> {
        let result = self.ready_mut()?.audio.check_errors();
        self.audio_result(result)
    }
    fn audio_result<T>(&mut self, result: incant_audio_runtime::Result<T>) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.live.take();
                Err(error.into())
            }
        }
    }
    fn advance(&mut self, events: &[InputEvent]) -> Result<()> {
        let live = self.ready_mut()?;
        if let Err(error) = live.native.tick(events) {
            if live.native.is_failed() {
                self.live.take();
            }
            return Err(error.into());
        }
        // Even standalone audio's retryable preflight rejection is terminal here:
        // accepted simulation/input/VM state already advanced and is not undone.
        let result = live.audio.sync(live.native.world()?);
        self.audio_result(result)
    }
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
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        settings: AudioManagerSettings<B>,
    ) -> Result<Self> {
        Ok(Self {
            core: Core::new(scene, script, actions, |world| {
                SceneAudio::new(world, clips, settings)
            })?,
        })
    }
    pub fn tick(&mut self, events: &[InputEvent]) -> Result<()> {
        self.core.advance(events)
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
