use super::TemporalError;
/// Trusted presentation policy. There is intentionally no visual-quality Default.
/// Every field is finite; all settings are constant for an owner lifetime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalConfig {
    /// Maximum old-color contribution [0, 0.99]. Never replaces current evidence.
    pub history_weight: f32,
    /// View-space relative depth tolerance [0, 0.1].
    pub relative_depth: f32,
    /// View-space absolute depth tolerance [0, 1] in world units.
    pub absolute_depth: f32,
    /// Scene-linear RGB max-channel change producing full reaction [1e-6, 65504].
    pub reactive_threshold: f32,
    /// Reduction of history at full reaction [0, 1].
    pub reactive_strength: f32,
}
impl TemporalConfig {
    pub fn validate(self) -> Result<(), TemporalError> {
        if ![
            self.history_weight,
            self.relative_depth,
            self.absolute_depth,
            self.reactive_threshold,
            self.reactive_strength,
        ]
        .iter()
        .all(|x| x.is_finite())
            || !(0. ..=0.99).contains(&self.history_weight)
            || !(0. ..=0.1).contains(&self.relative_depth)
            || !(0. ..=1.).contains(&self.absolute_depth)
            || !(0. ..=1.).contains(&self.reactive_strength)
            || self.reactive_threshold < 1e-6
            || self.reactive_threshold > 65504.
        {
            return Err(TemporalError::Config);
        }
        Ok(())
    }
}
