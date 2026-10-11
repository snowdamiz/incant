use crate::{Core, GameError, NoHost, Result, core::Hook};
use incant_audio_runtime::{ClipBinding, OfflineBackend, SceneAudio, SourceState};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::SessionProgram;

pub(crate) fn validate_rate<E>(rate: u32) -> Result<(), E> {
    // Cooked tick rates are <=240: every supported rate emits nonzero frames.
    if !(8000..=192000).contains(&rate) {
        return Err(GameError::SampleRate);
    }
    Ok(())
}
pub(crate) struct OfflineCore<H: Hook = NoHost> {
    pub core: Core<OfflineBackend, H>,
    pub sample_rate: u32,
}
impl<H: Hook> OfflineCore<H> {
    fn frames_at(&self, tick: u64) -> Result<u128, H::Error> {
        Ok(u128::from(tick) * u128::from(self.sample_rate)
            / u128::from(self.core.world()?.tick_rate()))
    }
    pub fn rendered_frames(&self) -> Result<u128, H::Error> {
        self.frames_at(self.core.world()?.tick())
    }
    pub fn next_output_frames(&self) -> Result<usize, H::Error> {
        let tick = self.core.world()?.tick();
        let end = tick.checked_add(1).ok_or(GameError::Clock)?;
        usize::try_from(self.frames_at(end)? - self.frames_at(tick)?).map_err(|_| GameError::Clock)
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<(), H::Error> {
        let expected = self.next_output_frames()? * 2;
        if output.len() != expected {
            return Err(GameError::Buffer {
                expected,
                actual: output.len(),
            });
        }
        let mut guard = SilenceOnFailure {
            output,
            keep: false,
        };
        let result = self.core.tick(events, |audio| audio.render(guard.output));
        // Retryable input rejection restored the original live state. All other
        // errors and unwinds leave it absent and silence the entire tick buffer.
        guard.keep = result.is_ok() || self.core.live.is_some();
        result
    }
}
struct SilenceOnFailure<'a> {
    output: &'a mut [f32],
    keep: bool,
}
impl Drop for SilenceOnFailure<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.output.fill(0.);
        }
    }
}

/// Nonvisual fixed-tick session with exactly floor(ticks * sample_rate / tick_rate)
/// rendered stereo frames. Incorrect buffers/input are retryable and untouched.
/// Any accepted failure/unwind destroys live state and clears the entire buffer.
pub struct OfflineGameSession {
    inner: OfflineCore,
}
impl OfflineGameSession {
    pub fn new(
        scene: CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
    ) -> Result<Self> {
        validate_rate(sample_rate)?;
        crate::require_nonspatial(&scene)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::prepare(
                    scene,
                    program,
                    actions,
                    |_| Ok(NoHost),
                    |world| SceneAudio::offline(world, clips, sample_rate),
                )?,
                sample_rate,
            },
        })
    }
    pub fn rendered_frames(&self) -> Result<u128> {
        self.inner.rendered_frames()
    }
    pub fn next_output_frames(&self) -> Result<usize> {
        self.inner.next_output_frames()
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<()> {
        self.inner.tick(events, output)
    }
    pub fn poll_audio(&mut self) -> Result<()> {
        self.inner.core.poll_audio()
    }
    pub fn is_failed(&self) -> bool {
        self.inner.core.live.is_none()
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        self.inner.core.world()
    }
    pub fn input(&self) -> Result<&InputFrame> {
        self.inner.core.input()
    }
    pub fn source_state(&self, id: StableId) -> Result<SourceState> {
        self.inner.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<()> {
        self.inner.core.rebind(actions)
    }
}
