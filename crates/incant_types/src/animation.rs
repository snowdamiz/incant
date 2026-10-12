//! Numeric animation observation shared by simulation and render consumers.

pub const MAX_ANIMATION_CLIPS: usize = 256;
pub const MAX_ANIMATION_SPEED: f64 = 16.0;

/// Authored starting playback for a MeshRenderer's immutable cooked model.
/// Clip indices are checked against that exact asset during scene cooking.
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    #[schemars(range(min = 0, max = 255))]
    pub clip: Option<u16>,
    #[schemars(range(min = 0))]
    pub seconds: f64,
    #[schemars(range(min = -16, max = 16))]
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
}

impl Animation {
    /// Asset-independent checks shared by authoring and native boundaries.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self
            .clip
            .is_some_and(|clip| usize::from(clip) >= MAX_ANIMATION_CLIPS)
            || !self.seconds.is_finite()
            || self.seconds < 0.0
            || (self.clip.is_none() && self.seconds != 0.0)
            || !self.speed.is_finite()
            || self.speed.abs() > MAX_ANIMATION_SPEED
        {
            return Err("invalid animation clip, time or speed");
        }
        Ok(())
    }
}

/// One committed sample of an immutable cooked model's animation.
///
/// Simulation owns the clock. A renderer samples this value without advancing
/// time or consulting authoring data. `None` selects the authored rest pose;
/// `Some` indexes the model's cooked clip table. At publication boundaries,
/// seconds must be finite and nonnegative and the selected clip must exist.
/// Model-specific duration and fingerprint validation belongs to that binding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSample {
    pub clip: Option<u16>,
    pub seconds: f64,
}

impl AnimationSample {
    pub const REST: Self = Self {
        clip: None,
        seconds: 0.0,
    };
}
