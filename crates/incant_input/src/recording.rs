//! Bounded event clips with fixed-tick seeking. Preparing a clip validates its
//! entire device lifetime before a game can consume even its first event.
use crate::{InputError, InputEvent, InputRuntime, MAX_EVENTS_PER_TICK, RecordedUiEvent};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 256))]
    ui: Vec<RecordedUiEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    viewport: Option<[f32; 2]>,
}
#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("input recording exceeds its byte, tick or event limit")]
    Size,
    #[error("unsupported input-recording format or version")]
    Version,
    #[error("UI input history cannot be seeked without a retained widget checkpoint")]
    UiSeek,
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
        if recording.format != "incant-input" || !matches!(recording.version, 1 | 2) {
            return Err(RecordingError::Version);
        }
        if recording.version == 1
            && recording
                .frames
                .iter()
                .any(|f| !f.ui.is_empty() || f.viewport.is_some())
        {
            return Err(RecordingError::Version);
        }
        if game_tick != recording.start_tick && recording.has_ui() {
            return Err(RecordingError::UiSeek);
        }
        if !(1..=240).contains(&recording.tick_rate) || recording.tick_rate != tick_rate {
            return Err(RecordingError::Rate);
        }
        if recording.ticks > MAX_TICKS
            || recording.start_tick > MAX_SAFE_TICK - recording.ticks
            || recording.frames.len() > MAX_TICKS as usize
            || recording.frames.iter().any(|f| {
                f.events.len() > MAX_EVENTS_PER_TICK || f.ui.len() > crate::MAX_UI_NOTIFICATIONS
            })
            || recording
                .frames
                .iter()
                .map(|f| f.events.len() + f.ui.len())
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
            if frame.viewport.is_some_and(|size| {
                size.iter()
                    .any(|n| !n.is_finite() || !(1.0..=65_536.0).contains(n))
            }) {
                return Err(RecordingError::Input {
                    tick: frame.tick,
                    source: InputError::Ui,
                });
            }
            let mut bytes = 0usize;
            for ui in &frame.ui {
                bytes = bytes
                    .checked_add(ui.validate().map_err(|source| RecordingError::Input {
                        tick: frame.tick,
                        source,
                    })?)
                    .filter(|n| *n <= crate::MAX_UI_NOTIFICATION_BYTES)
                    .ok_or(RecordingError::Size)?;
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
    pub fn has_ui(&self) -> bool {
        self.recording.has_ui()
    }
    pub fn ui_at(&self, tick: u64) -> Result<&[RecordedUiEvent], RecordingError> {
        self.events_at(tick)?;
        Ok(self
            .recording
            .frame(tick - self.recording.start_tick)
            .map_or(&[], |f| f.ui.as_slice()))
    }
    pub fn viewport_at(&self, tick: u64) -> Result<Option<[f32; 2]>, RecordingError> {
        self.events_at(tick)?;
        Ok(self
            .recording
            .frame(tick - self.recording.start_tick)
            .and_then(|f| f.viewport))
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
    fn has_ui(&self) -> bool {
        self.frames
            .iter()
            .any(|f| !f.ui.is_empty() || f.viewport.is_some())
    }
    fn frame(&self, tick: u64) -> Option<&RecordedTick> {
        self.frames
            .binary_search_by_key(&tick, |f| f.tick)
            .ok()
            .map(|i| &self.frames[i])
    }
    fn at_offset(&self, tick: u64) -> &[InputEvent] {
        self.frames
            .binary_search_by_key(&tick, |frame| frame.tick)
            .map_or(&[], |index| self.frames[index].events.as_slice())
    }
}
