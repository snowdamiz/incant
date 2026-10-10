use crate::{Core, GameError, Result};
use incant_audio_runtime::{ClipBinding, OfflineBackend, SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::CompiledScript;

/// A fixed-tick session with exactly floor(ticks * sample_rate / tick_rate)
/// rendered stereo frames. No rounding accumulator or caller-selected dt exists.
/// Invalid input/buffers leave output untouched; an accepted failing tick clears
/// the entire supplied buffer and destroys all live session state.
pub struct OfflineGameSession {
    core: Core<OfflineBackend>,
    sample_rate: u32,
}
impl OfflineGameSession {
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
    ) -> Result<Self> {
        // Match the supported pull-backend range before VM/backend construction.
        // With cooked tick rates <=240 all legal rates produce nonzero frames.
        if !(8000..=192000).contains(&sample_rate) {
            return Err(GameError::SampleRate);
        }
        Ok(Self {
            core: Core::new(scene, script, actions, |world| {
                SceneAudio::offline(world, clips, sample_rate)
            })?,
            sample_rate,
        })
    }
    fn frames_at(&self, tick: u64) -> Result<u128> {
        Ok(u128::from(tick) * u128::from(self.sample_rate)
            / u128::from(self.core.world()?.tick_rate()))
    }
    pub fn rendered_frames(&self) -> Result<u128> {
        self.frames_at(self.core.world()?.tick())
    }
    pub fn next_output_frames(&self) -> Result<usize> {
        let tick = self.core.world()?.tick();
        let end = tick.checked_add(1).ok_or(GameError::Clock)?;
        usize::try_from(self.frames_at(end)? - self.frames_at(tick)?).map_err(|_| GameError::Clock)
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<()> {
        let expected = self.next_output_frames()? * 2;
        if output.len() != expected {
            return Err(GameError::Buffer {
                expected,
                actual: output.len(),
            });
        }
        if let Err(error) = self.core.advance(events) {
            if self.is_failed() {
                output.fill(0.);
            }
            return Err(error);
        }
        let result = self.core.ready_mut()?.audio.render(output);
        // SceneAudio already clears render failures; keep the composite guarantee
        // explicit if a future backend reports an error before touching output.
        if result.is_err() {
            output.fill(0.);
        }
        self.core.audio_result(result)
    }
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
