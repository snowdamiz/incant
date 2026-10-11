use crate::{GameError, Result, SceneHost};
use incant_audio_runtime::{SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, NativeSession, PendingNativeSession};
use kira::backend::Backend;
use std::{convert::Infallible, error::Error, fmt::Debug};

pub(crate) trait Hook {
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
}
impl<B: Backend, H: Hook> Core<B, H>
where
    B::Error: Debug,
{
    pub fn prepare(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        prepare: impl FnOnce(&NativeWorld) -> Result<H, H::Error>,
        audio: impl FnOnce(&NativeWorld) -> incant_audio_runtime::Result<SceneAudio<B>>,
    ) -> Result<Self, H::Error> {
        let pending = PendingNativeSession::new(scene, actions)?;
        let host = prepare(pending.world())?;
        let native = pending.start(script)?;
        let audio = audio(native.world()?)?;
        Ok(Self {
            live: Some(Live {
                audio,
                host,
                native,
            }),
        })
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
    pub fn tick(
        &mut self,
        events: &[InputEvent],
        render: impl FnOnce(&mut SceneAudio<B>) -> incant_audio_runtime::Result<()>,
    ) -> Result<(), H::Error> {
        // Keep the public state absent until the complete accepted operation
        // succeeds. Err and Rust unwind drop every live subsystem; no catch or
        // whole-world clone is needed. Abort cannot run cleanup.
        let mut live = self.live.take().ok_or(GameError::Failed)?;
        if let Err(error) = live.native.tick(events) {
            if !live.native.is_failed() {
                self.live = Some(live);
            }
            return Err(error.into());
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
