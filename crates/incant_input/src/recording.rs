//! Bounded event clips with fixed-tick seeking. Preparing a clip validates its
//! entire device lifetime before a game can consume even its first event.
use crate::{InputError, InputEvent, InputRuntime, MAX_EVENTS_PER_TICK};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_RECORDING_BYTES: usize = 8 * 1024 * 1024;
const MAX_TICKS: u64 = 10_000;
const MAX_EVENTS: usize = 100_000;
const MAX_SAFE_TICK: u64 = (1 << 53) - 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputRecording {
    format: String,
    version: u32,
    #[schemars(range(min = 1, max = 240))]
    tick_rate: u32,
    /// Absolute game tick immediately before this clip begins.
    start_tick: u64,
    /// Number of additional ticks covered by this clip, including empty ticks.
    #[schemars(range(min = 0, max = 10000))]
    ticks: u64,
    /// Sparse, strictly increasing, one-based offsets within the clip.
    #[schemars(length(max = 10000))]
    frames: Vec<RecordedTick>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordedTick {
    #[schemars(range(min = 1, max = 10000))]
    tick: u64,
    #[schemars(length(max = 1024))]
    events: Vec<InputEvent>,
}
#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("input recording exceeds its byte, tick or event limit")]
    Size,
    #[error("unsupported input-recording format or version")]
    Version,
    #[error("input-recording tick rate must match the game")]
    Rate,
    #[error("input-recording frames must have strictly increasing offsets within the clip")]
    Order,
    #[error("requested game tick is outside the input clip")]
    Range,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("input recording is invalid at offset {tick}: {source}")]
    Input { tick: u64, source: InputError },
}

pub struct InputReplay {
    recording: InputRecording,
    initial: InputRuntime,
}
impl InputReplay {
    /// Seek input history only; this never advances or reruns the game itself.
    /// Logical saves omit physical devices. Supplying the same clip on restore
    /// reconstructs held controls and gesture history at the saved tick.
    pub fn from_text(text: &str, game_tick: u64, tick_rate: u32) -> Result<Self, RecordingError> {
        if text.len() > MAX_RECORDING_BYTES {
            return Err(RecordingError::Size);
        }
        let recording: InputRecording = serde_json::from_str(text)?;
        if recording.format != "incant-input" || recording.version != 1 {
            return Err(RecordingError::Version);
        }
        if !(1..=240).contains(&recording.tick_rate) || recording.tick_rate != tick_rate {
            return Err(RecordingError::Rate);
        }
        if recording.ticks > MAX_TICKS
            || recording.start_tick > MAX_SAFE_TICK - recording.ticks
            || recording.frames.len() > MAX_TICKS as usize
            || recording
                .frames
                .iter()
                .any(|f| f.events.len() > MAX_EVENTS_PER_TICK)
            || recording
                .frames
                .iter()
                .map(|f| f.events.len())
                .sum::<usize>()
                > MAX_EVENTS
        {
            return Err(RecordingError::Size);
        }
        if game_tick < recording.start_tick || game_tick > recording.start_tick + recording.ticks {
            return Err(RecordingError::Range);
        }
        let mut previous = 0;
        for frame in &recording.frames {
            if frame.tick <= previous || frame.tick > recording.ticks {
                return Err(RecordingError::Order);
            }
            previous = frame.tick;
        }
        let offset = game_tick - recording.start_tick;
        let mut initial = InputRuntime::default();
        let mut state = InputRuntime::default();
        for tick in 1..=recording.ticks {
            state
                .advance(recording.at_offset(tick), 1. / f64::from(tick_rate))
                .map_err(|source| RecordingError::Input { tick, source })?;
            if tick == offset {
                initial = state.clone();
            }
        }
        Ok(Self { recording, initial })
    }
    pub fn initial_state(&self) -> InputRuntime {
        self.initial.clone()
    }
    pub fn end_tick(&self) -> u64 {
        self.recording.start_tick + self.recording.ticks
    }
    pub fn events_at(&self, tick: u64) -> Result<&[InputEvent], RecordingError> {
        if tick <= self.recording.start_tick || tick > self.end_tick() {
            return Err(RecordingError::Range);
        }
        Ok(self.recording.at_offset(tick - self.recording.start_tick))
    }
}
impl InputRecording {
    fn at_offset(&self, tick: u64) -> &[InputEvent] {
        self.frames
            .binary_search_by_key(&tick, |frame| frame.tick)
            .map_or(&[], |index| self.frames[index].events.as_slice())
    }
}
