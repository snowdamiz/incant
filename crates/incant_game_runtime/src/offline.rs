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
        self.tick_packet(
            events,
            #[cfg(feature = "ui")]
            &[],
            output,
        )
    }
    pub fn tick_packet(
        &mut self,
        events: &[InputEvent],
        #[cfg(feature = "ui")] receipts: &[incant_ui::UiEvent],
        output: &mut [f32],
    ) -> Result<(), H::Error> {
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
        let result = self.core.tick_packet(
            events,
            #[cfg(feature = "ui")]
            receipts,
            |audio| audio.render(guard.output),
        );
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
    pub(crate) inner: OfflineCore,
}
impl OfflineGameSession {
    /// Create a fresh native session with the restart-request contract enabled.
    pub fn from_seed(seed: &crate::GameSeed, sample_rate: u32) -> Result<Self> {
        let mut game = Self::new(
            seed.scene.clone(),
            seed.program.clone(),
            seed.actions.clone(),
            seed.clips.clone(),
            sample_rate,
        )?;
        game.enable_restart_requests()?;
        Ok(game)
    }
    /// Drop all live state before constructing the replacement. Constructor
    /// failure/unwind is terminal; no previous/new mixed session is observable.
    /// Output chronology belongs to the host, not the reset simulation clock.
    pub fn restart_from(&mut self, seed: &crate::GameSeed) -> Result<()> {
        self.inner.core.ready()?;
        self.inner.core.live.take();
        *self = Self::from_seed(seed, self.inner.sample_rate)?;
        Ok(())
    }
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
    /// Restore ordinary gameplay data and checked offline audio transports.
    /// Static resampler history, streamed cursors and rational sample phase are
    /// preserved; device output and arbitrary VM heap state are not restored.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
    ) -> Result<Self> {
        validate_rate(sample_rate)?;
        crate::require_nonspatial(initial)?;
        Ok(Self {
            inner: OfflineCore {
                core: Core::from_offline_checkpoint(
                    bytes,
                    initial,
                    program,
                    actions,
                    |pending| {
                        crate::require_nonspatial_world(pending.world())?;
                        Ok(NoHost)
                    },
                    clips,
                    sample_rate,
                )?,
                sample_rate,
            },
        })
    }
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>> {
        self.inner.core.checkpoint_bytes()
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
    /// Suspend transport and reject ticks until resumed; failure is terminal.
    pub fn set_suspended(&mut self, suspended: bool) -> Result<()> {
        self.inner.core.set_suspended(suspended)
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
    pub fn logs(&self) -> Result<&incant_script::NativeLogs> {
        self.inner.core.logs()
    }
    /// Explicit trusted-owner opt-in for whole-session replacement.
    /// True only for a program with the implemented typed haptic capability.
    pub fn supports_haptics(&self) -> Result<bool> {
        self.inner.core.supports_haptics()
    }
    /// Current host-issued targets; never reconstructed from saved data.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<()> {
        self.inner.core.set_haptic_capabilities(value)
    }
    /// Drain only accepted composed output. Suspension clears pending requests.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests> {
        self.inner.core.take_haptics()
    }

    pub fn enable_restart_requests(&mut self) -> Result<()> {
        self.inner.core.enable_restart_requests()
    }
    /// Only visible after the complete composed tick succeeds.
    pub fn take_session_request(&mut self) -> Result<Option<incant_script::SessionRequest>> {
        self.inner.core.take_session_request()
    }

    pub fn source_state(&self, id: StableId) -> Result<SourceState> {
        self.inner.core.source_state(id)
    }
    pub fn rebind(&mut self, actions: InputActions) -> Result<()> {
        self.inner.core.rebind(actions)
    }
}
