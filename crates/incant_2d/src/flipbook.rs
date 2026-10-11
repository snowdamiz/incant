//! Bounded sprite-sheet frame declarations and exact fixed-tick playback.
use crate::{SpriteRegion, SpriteRenderer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_FLIPBOOK_FRAMES: usize = 256;
pub const MAX_SCENE_FLIPBOOK_FRAMES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    /// Frames per second = numerator / denominator; no floating-point clock.
    #[schemars(range(min = 1, max = 1000000))]
    pub numerator: u32,
    #[schemars(range(min = 1, max = 1000000))]
    pub denominator: u32,
}
impl FrameRate {
    pub fn validate(self) -> Result<(), FlipbookError> {
        if self.numerator == 0
            || self.denominator == 0
            || self.numerator > 1_000_000
            || self.denominator > 1_000_000
            || u64::from(self.numerator) > u64::from(self.denominator) * 1000
        {
            return Err(FlipbookError::Rate);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpriteFlipbook {
    /// Regions in the SpriteRenderer's single texture; first frame overrides its
    /// static region. Every frame is checked by the retained asset host up front.
    #[schemars(length(min = 1, max = 256))]
    pub frames: Vec<SpriteRegion>,
    pub rate: FrameRate,
    pub looping: bool,
    pub playing: bool,
}
impl SpriteFlipbook {
    pub fn validate(&self, sprite: &SpriteRenderer) -> Result<(), FlipbookError> {
        self.rate.validate()?;
        if self.frames.is_empty() || self.frames.len() > MAX_FLIPBOOK_FRAMES {
            return Err(FlipbookError::Frames);
        }
        let mut candidate = sprite.clone();
        for frame in &self.frames {
            candidate.region = frame.clone();
            candidate.validate().map_err(|_| FlipbookError::Region)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FlipbookError {
    #[error("flipbook requires 1..=256 frames")]
    Frames,
    #[error("flipbook rate requires positive terms <=1000000 and <=1000 frames/second")]
    Rate,
    #[error("flipbook frame is invalid for its sprite")]
    Region,
    #[error("flipbook seek frame is out of range")]
    Seek,
    #[error("flipbook tick rate must be in 1..=1000")]
    TickRate,
}

/// Typed native control. Seek resets the subframe phase and completion, retaining
/// play/pause. Restart resets to frame zero and plays. Play on a completed once
/// sequence restarts it. Pause/resume otherwise preserves the exact phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipbookControl {
    Play,
    Pause,
    Restart,
    Seek(u16),
}
/// Numeric playback state; mutation only through checked native operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlipbookState {
    frame: u16,
    remainder: u64,
    playing: bool,
    completed: bool,
}
impl FlipbookState {
    pub fn new(playing: bool) -> Self {
        Self {
            frame: 0,
            remainder: 0,
            playing,
            completed: false,
        }
    }
    pub fn frame(self) -> u16 {
        self.frame
    }
    pub fn playing(self) -> bool {
        self.playing
    }
    pub fn completed(self) -> bool {
        self.completed
    }
    fn control(&mut self, control: FlipbookControl, frames: usize) -> Result<(), FlipbookError> {
        match control {
            FlipbookControl::Play => {
                if self.completed {
                    *self = Self::new(true);
                } else {
                    self.playing = true;
                }
            }
            FlipbookControl::Pause => self.playing = false,
            FlipbookControl::Restart => *self = Self::new(true),
            FlipbookControl::Seek(frame) => {
                if usize::from(frame) >= frames {
                    return Err(FlipbookError::Seek);
                }
                self.frame = frame;
                self.remainder = 0;
                self.completed = false;
            }
        }
        Ok(())
    }
}

/// Startup-prepared rational clock. Fields are private so stepping cannot accept
/// an unvalidated denominator or allocate/validate authoring data each tick.
#[derive(Debug, Clone, Copy)]
pub struct FlipbookClock {
    numerator: u64,
    denominator: u64,
    frames: u16,
    looping: bool,
}
impl FlipbookClock {
    pub fn new(
        rate: FrameRate,
        frames: usize,
        looping: bool,
        tick_rate: u32,
    ) -> Result<Self, FlipbookError> {
        rate.validate()?;
        if frames == 0 || frames > MAX_FLIPBOOK_FRAMES {
            return Err(FlipbookError::Frames);
        }
        if !(1..=1000).contains(&tick_rate) {
            return Err(FlipbookError::TickRate);
        }
        Ok(Self {
            numerator: u64::from(rate.numerator),
            denominator: u64::from(rate.denominator) * u64::from(tick_rate),
            frames: frames as u16,
            looping,
        })
    }
    pub fn control(
        self,
        state: &mut FlipbookState,
        control: FlipbookControl,
    ) -> Result<(), FlipbookError> {
        state.control(control, usize::from(self.frames))
    }
    /// Constant work even when one tick crosses many frames; no accumulated
    /// floating-point error or absolute-tick multiplication/overflow. State from
    /// another clock is normalized before arithmetic; private fields bound it.
    pub fn advance(self, state: &mut FlipbookState) {
        if !state.playing || state.completed {
            return;
        }
        let phase = state.remainder % self.denominator + self.numerator;
        let next = u64::from(state.frame) + phase / self.denominator;
        state.remainder = phase % self.denominator;
        if self.looping {
            state.frame = (next % u64::from(self.frames)) as u16;
        } else if next >= u64::from(self.frames) {
            state.frame = self.frames - 1;
            state.remainder = 0;
            state.completed = true;
            state.playing = false;
        } else {
            state.frame = next as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_clock_matches_integer_reference_without_drift_or_catch_up_loops() {
        let clock = FlipbookClock::new(
            FrameRate {
                numerator: 30000,
                denominator: 1001,
            },
            7,
            true,
            60,
        )
        .unwrap();
        let mut state = FlipbookState::new(true);
        for tick in 1..=100_000u64 {
            clock.advance(&mut state);
            assert_eq!(u64::from(state.frame()), (tick * 30000 / (1001 * 60)) % 7);
        }
        let fast = FlipbookClock::new(
            FrameRate {
                numerator: 1000,
                denominator: 1,
            },
            7,
            true,
            1,
        )
        .unwrap();
        let mut state = FlipbookState::new(true);
        fast.advance(&mut state);
        assert_eq!(state.frame(), 6);
        let slow = FlipbookClock::new(
            FrameRate {
                numerator: 1,
                denominator: 1_000_000,
            },
            2,
            true,
            240,
        )
        .unwrap();
        for _ in 0..100_000 {
            slow.advance(&mut state);
        }
        assert!(state.frame() < 2); // Foreign clock state is safely normalized.
    }
    #[test]
    fn pause_resume_preserves_phase_and_once_holds_each_frames_full_duration() {
        let clock = FlipbookClock::new(
            FrameRate {
                numerator: 12,
                denominator: 1,
            },
            2,
            false,
            60,
        )
        .unwrap();
        let mut state = FlipbookState::new(true);
        for _ in 0..4 {
            clock.advance(&mut state);
        }
        clock.control(&mut state, FlipbookControl::Pause).unwrap();
        for _ in 0..100 {
            clock.advance(&mut state);
        }
        assert_eq!(state.frame(), 0);
        clock.control(&mut state, FlipbookControl::Play).unwrap();
        clock.advance(&mut state);
        assert_eq!(state.frame(), 1);
        assert!(!state.completed());
        for _ in 0..4 {
            clock.advance(&mut state);
            assert!(!state.completed());
        }
        clock.advance(&mut state);
        assert!(state.completed());
        assert!(!state.playing());
        assert_eq!(state.frame(), 1);
        clock.control(&mut state, FlipbookControl::Play).unwrap();
        assert_eq!(state, FlipbookState::new(true));
        let old = state;
        assert!(clock.control(&mut state, FlipbookControl::Seek(2)).is_err());
        assert_eq!(old, state);
    }
    #[test]
    fn prepared_clock_rejects_unbounded_or_zero_inputs() {
        for rate in [
            FrameRate {
                numerator: 0,
                denominator: 1,
            },
            FrameRate {
                numerator: 1,
                denominator: 0,
            },
            FrameRate {
                numerator: 1_000_001,
                denominator: 1000,
            },
            FrameRate {
                numerator: 1001,
                denominator: 1,
            },
        ] {
            assert!(FlipbookClock::new(rate, 1, true, 60).is_err());
        }
        let rate = FrameRate {
            numerator: 1,
            denominator: 1,
        };
        assert!(FlipbookClock::new(rate, 0, true, 60).is_err());
        assert!(FlipbookClock::new(rate, 257, true, 60).is_err());
        assert!(FlipbookClock::new(rate, 1, true, 0).is_err());
    }
}
