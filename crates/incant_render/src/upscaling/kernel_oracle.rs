//! Test-only f64 support geometry. Historical callers explicitly select the old
//! fixed-resolution lattice; R4 callers select its new split lattice. There is no
//! production/history admission, resource, LOD or shader connection.
use super::{Plan, oracle};

#[derive(Clone, Copy, Debug)]
pub(super) struct Tap {
    pub texel: [i32; 2],
    pub position: [f64; 2],
    pub weight: f64,
}
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Evidence {
    pub sum: [f64; 4],
    pub mass: f64,
}
#[derive(Clone, Copy)]
pub(super) struct Kernel {
    pub output: [f64; 4],
    pub source: [f64; 4],
    pub jitter: [f64; 2],
    pub ratio: f64,
    pub rho: f64,
    pub norm: f64,
}
impl Kernel {
    pub fn historical(size: [u32; 2], rect: [f32; 4], n: u8, phase: usize) -> Option<Self> {
        Self::with_offset(size, rect, n, crate::temporal::jitter::SAMPLES[phase % 8])
    }
    pub fn current(size: [u32; 2], rect: [f32; 4], n: u8, phase: usize) -> Option<Self> {
        Self::with_offset(
            size,
            rect,
            n,
            crate::temporal::jitter::UPSCALE_SAMPLES[phase % 8],
        )
    }
    fn with_offset(size: [u32; 2], rect: [f32; 4], n: u8, offset: [f32; 2]) -> Option<Self> {
        let scale = crate::resolution::ResolutionScale::new(n).ok()?;
        let frame = crate::resolution::FramePlan::new(size, rect, None, scale).ok()?;
        let plan = Plan::new(size, rect, &frame).ok()?;
        plan.preflight(8192, 0, 0, 0).ok()?;
        let source = plan.source_rect.map(f64::from);
        let output = plan.rect.map(f64::from);
        let offset = offset.map(f64::from);
        let jitter = std::array::from_fn(|i| offset[i] / source[i + 2]);
        let ratio = output[2] / source[2];
        let rho = 0.35 + 0.325 * (ratio - 2.).max(0.);
        Some(Self {
            output,
            source,
            jitter,
            ratio,
            rho,
            norm: ratio * ratio / (2. * std::f64::consts::PI * rho * rho),
        })
    }
    pub fn position(self, texel: [i32; 2]) -> [f64; 2] {
        std::array::from_fn(|i| {
            self.output[i]
                + ((f64::from(texel[i]) + 0.5 - self.source[i]) / self.source[i + 2]
                    - self.jitter[i])
                    * self.output[i + 2]
        })
    }
    pub fn weight(self, point: [f64; 2], sample: [f64; 2]) -> f64 {
        let squared: f64 = (0..2).map(|i| (point[i] - sample[i]).powi(2)).sum();
        self.norm * (-squared / (2. * self.rho * self.rho)).exp()
    }
    pub fn taps(self, point: [f64; 2]) -> Option<[Option<Tap>; 9]> {
        if !(0..2).all(|i| {
            point[i].is_finite()
                && point[i] >= self.output[i]
                && point[i] < self.output[i] + self.output[i + 2]
        }) {
            return None;
        }
        let base =
            oracle::source(point, self.output, self.source, self.jitter).map(|v| v.floor() as i32);
        Some(std::array::from_fn(|i| {
            let texel = [base[0] + (i % 3) as i32 - 1, base[1] + (i / 3) as i32 - 1];
            if !(0..2).all(|j| {
                f64::from(texel[j]) + 0.5 >= self.source[j]
                    && f64::from(texel[j]) + 0.5 < self.source[j] + self.source[j + 2]
            }) {
                return None;
            }
            let position = self.position(texel);
            Some(Tap {
                texel,
                position,
                weight: self.weight(point, position),
            })
        }))
    }
    pub fn evidence(
        self,
        point: [f64; 2],
        sample: impl Fn([f64; 2]) -> [f64; 4],
    ) -> Option<Evidence> {
        let mut out = Evidence::default();
        for tap in self.taps(point)?.into_iter().flatten() {
            let color = oracle::color(sample(tap.position));
            out.mass += tap.weight; // Clear is evidence, not missing support.
            for (s, v) in out.sum.iter_mut().zip(color) {
                *s += tap.weight * v;
            }
        }
        (out.mass.is_finite() && out.mass > 0.).then_some(out)
    }
    /// Negative control: old bilinear current estimate at the same true sample
    /// positions. Existing coordinate/bilinear oracle stays unchanged.
    pub fn bilinear(
        self,
        point: [f64; 2],
        sample: impl Fn([f64; 2]) -> [f64; 4],
    ) -> Option<[f64; 4]> {
        let p = oracle::source(point, self.output, self.source, self.jitter);
        let taps = oracle::taps(p, self.source)?;
        let mut out = [0.; 4];
        let mut mass = 0.;
        for (texel, w) in taps {
            if w > 0. {
                mass += w;
                for (s, v) in out
                    .iter_mut()
                    .zip(oracle::color(sample(self.position(texel))))
                {
                    *s += w * v;
                }
            }
        }
        (mass > 0.).then(|| out.map(|v| v / mass))
    }
}
