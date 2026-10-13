//! Design-only scalar confidence accumulation. Callers supply admitted history;
//! dynamic disocclusion/ownership admission is deliberately not modeled here.
use super::{
    UpscaleConfig,
    kernel_oracle::{Evidence, Kernel},
    oracle,
};
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct History {
    pub color: [f64; 4],
    pub mass: f64,
    pub(super) source_width: f64,
}
pub(super) fn cap(h: f64, ratio: f64) -> Option<f64> {
    if !h.is_finite()
        || !(0. ..=f64::from(0.99_f32)).contains(&h)
        || !ratio.is_finite()
        || !(1. ..=4.).contains(&ratio)
    {
        return None;
    }
    Some(h / (1. - h) * (ratio * ratio).min(4.))
}
impl History {
    pub fn step(
        self,
        evidence: Evidence,
        kernel: Kernel,
        config: UpscaleConfig,
        reset: bool,
    ) -> Option<Self> {
        config.validate().ok()?;
        let max = cap(f64::from(config.history_weight), kernel.ratio)?;
        // At most nine products/additions plus the independently rounded mass
        // comparison. This is arithmetic roundoff, not a reconstruction tolerance.
        let roundoff = 1. + 32. * f64::EPSILON;
        if !evidence.mass.is_finite()
            || evidence.mass <= 0.
            || evidence.mass > 9. * kernel.norm
            || evidence.sum.iter().any(|v| !v.is_finite() || *v < 0.)
            || evidence.sum[3] > evidence.mass * roundoff
            || evidence.sum[..3]
                .iter()
                .any(|v| *v > 65504. * evidence.mass * roundoff)
        {
            return None;
        }
        let mut weight = if reset { 0. } else { self.mass };
        if !weight.is_finite() || weight < 0. || weight > cap(f64::from(0.99_f32), 4.)? {
            return None;
        }
        if self.source_width > 0. && kernel.source[2] > self.source_width {
            weight = weight.min(max) * (self.source_width / kernel.source[2]).powi(2);
        }
        let denominator = weight + evidence.mass;
        let old = oracle::color(self.color);
        let color = oracle::color(std::array::from_fn(|i| {
            (old[i] * weight + evidence.sum[i]) / denominator
        }));
        Some(Self {
            color,
            mass: denominator.min(max),
            source_width: kernel.source[2],
        })
    }
}
