use crate::{AudioError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    pub id: String,
    pub parent: Option<String>,
    pub gain_db: f32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spatial {
    pub position: [f32; 3],
    pub min_distance: f32,
    pub max_distance: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayOptions {
    pub bus: Option<String>,
    pub gain_db: f32,
    pub pan: f32,
    pub rate: f64,
    pub start_seconds: f64,
    pub looping: bool,
    pub streaming: bool,
    pub spatial: Option<Spatial>,
}
impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            bus: None,
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            looping: false,
            streaming: false,
            spatial: None,
        }
    }
}
pub(crate) fn gain(value: f32) -> Result<()> {
    if value.is_finite() && (-80. ..=24.).contains(&value) {
        Ok(())
    } else {
        Err(AudioError::Invalid("gain must be finite in [-80,24] dB"))
    }
}
pub(crate) fn position(value: [f32; 3]) -> Result<()> {
    if value.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.) {
        Ok(())
    } else {
        Err(AudioError::Invalid("position must be finite and bounded"))
    }
}
impl PlayOptions {
    pub(crate) fn validate(&self, duration: f64) -> Result<()> {
        gain(self.gain_db)?;
        if !self.pan.is_finite()
            || !(-1. ..=1.).contains(&self.pan)
            || !self.rate.is_finite()
            || !(0.25..=4.).contains(&self.rate)
            || !self.start_seconds.is_finite()
            || !(0. ..=duration).contains(&self.start_seconds)
        {
            return Err(AudioError::Invalid(
                "pan, rate or start time outside supported range",
            ));
        }
        if let Some(s) = self.spatial {
            position(s.position)?;
            if !s.min_distance.is_finite()
                || !s.max_distance.is_finite()
                || s.min_distance < 0.01
                || s.max_distance <= s.min_distance
                || s.max_distance > 1_000_000.
            {
                return Err(AudioError::Invalid(
                    "spatial distances must be finite, positive and ordered",
                ));
            }
        }
        Ok(())
    }
}
