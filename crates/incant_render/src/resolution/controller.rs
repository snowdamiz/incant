/// Internal linear resolution in exact sixteenths of the output size.
/// The admitted range is 25–100%; super-sampling is not part of this controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolutionScale(u8);
impl ResolutionScale {
    pub const FULL: Self = Self(16);
    pub fn new(sixteenths: u8) -> Result<Self, ResolutionError> {
        (4..=16)
            .contains(&sixteenths)
            .then_some(Self(sixteenths))
            .ok_or(ResolutionError::Config)
    }
    pub fn fraction(self) -> f32 {
        f32::from(self.0) / 16.
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolutionConfig {
    pub target_ms: f64,
    /// Fraction of target on either side with no scale change (0–0.5).
    pub deadband: f64,
    pub minimum: ResolutionScale,
    pub maximum: ResolutionScale,
    pub initial: ResolutionScale,
    /// Consecutive completed samples outside the band (1–240).
    pub overload_samples: u16,
    pub recovery_samples: u16,
    /// Samples ignored after a change (0–240).
    pub cooldown_samples: u16,
    /// Maximum linear change per decision, in sixteenths (1–4).
    pub slew: u8,
}
impl ResolutionConfig {
    pub fn validate(self) -> Result<(), ResolutionError> {
        if !self.target_ms.is_finite()
            || !(1. ..=1000.).contains(&self.target_ms)
            || !self.deadband.is_finite()
            || !(0. ..=0.5).contains(&self.deadband)
            || self.minimum > self.initial
            || self.initial > self.maximum
            || !(1..=240).contains(&self.overload_samples)
            || !(1..=240).contains(&self.recovery_samples)
            || self.cooldown_samples > 240
            || !(1..=4).contains(&self.slew)
        {
            return Err(ResolutionError::Config);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    #[error("invalid dynamic resolution configuration")]
    Config,
    #[error("frame telemetry must be finite and in (0, 10000] milliseconds")]
    Telemetry,
    #[error("dynamic resolution frame sequence exhausted")]
    Sequence,
    #[error("dynamic resolution completion mailbox poisoned")]
    Mailbox,
    #[error(
        "automatic native completion timing is unavailable on this target; supply measured frame telemetry explicitly"
    )]
    ClockUnavailable,
}
/// Pure bounded controller; caller supplies measured completed-frame durations.
/// No averaging allocation, simulation time, synthetic GPU estimate or pipeline work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolutionController {
    config: ResolutionConfig,
    scale: ResolutionScale,
    overload: u16,
    recovery: u16,
    cooldown: u16,
}
impl ResolutionController {
    pub fn new(config: ResolutionConfig) -> Result<Self, ResolutionError> {
        config.validate()?;
        Ok(Self {
            scale: config.initial,
            config,
            overload: 0,
            recovery: 0,
            cooldown: 0,
        })
    }
    pub fn scale(&self) -> ResolutionScale {
        self.scale
    }
    pub(crate) fn reset_history(&mut self) {
        self.overload = 0;
        self.recovery = 0;
        self.cooldown = 0;
    }
    pub fn observe(&mut self, milliseconds: f64) -> Result<ResolutionScale, ResolutionError> {
        if !milliseconds.is_finite() || milliseconds <= 0. || milliseconds > 10_000. {
            return Err(ResolutionError::Telemetry);
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return Ok(self.scale);
        }
        if milliseconds > self.config.target_ms * (1. + self.config.deadband) {
            self.recovery = 0;
            self.overload += 1;
            if self.overload >= self.config.overload_samples {
                self.overload = 0;
                self.change(self.scale.0.saturating_sub(self.config.slew));
            }
        } else if milliseconds < self.config.target_ms * (1. - self.config.deadband) {
            self.overload = 0;
            self.recovery += 1;
            if self.recovery >= self.config.recovery_samples {
                self.recovery = 0;
                self.change(self.scale.0.saturating_add(self.config.slew));
            }
        } else {
            self.reset_history();
        }
        Ok(self.scale)
    }
    fn change(&mut self, candidate: u8) {
        let next = ResolutionScale(candidate.clamp(self.config.minimum.0, self.config.maximum.0));
        if next != self.scale {
            self.scale = next;
            self.cooldown = self.config.cooldown_samples;
        }
    }
}
