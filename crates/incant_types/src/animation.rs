//! Numeric animation observation shared by simulation and render consumers.

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
