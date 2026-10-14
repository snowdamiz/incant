//! Test-only sampling experiment. No production setting, shader or mip change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Sequence {
    #[default]
    Incant8,
    Halton32,
}
impl Sequence {
    pub(crate) fn period(self) -> u32 {
        match self {
            Self::Incant8 => 8,
            Self::Halton32 => 32,
        }
    }
    pub(crate) fn sample(self, phase: u32) -> [f32; 2] {
        assert!(phase < self.period());
        match self {
            Self::Incant8 => crate::temporal::jitter::SAMPLES[phase as usize],
            Self::Halton32 => {
                let inverse = |mut n: u32, base: u32| {
                    let mut value = 0.0_f64;
                    let mut power = 1.0_f64;
                    while n > 0 {
                        power /= f64::from(base);
                        value += f64::from(n % base) * power;
                        n /= base;
                    }
                    (value - 0.5) as f32
                };
                [inverse(phase + 1, 2), inverse(phase + 1, 3)]
            }
        }
    }
    /// Same right/down internal-pixel convention as the existing sequence.
    pub(crate) fn ndc(self, phase: u32, rect: [f32; 4]) -> [f32; 2] {
        let p = self.sample(phase);
        [2. * p[0] / rect[2], -2. * p[1] / rect[3]]
    }
}
#[test]
fn halton_exact_rational_coordinates_and_full_cycle_coverage() {
    // Independent integer digit reversal, one final division per coordinate.
    fn reference(mut n: u32, base: u32) -> f32 {
        let (mut numerator, mut denominator) = (0_u32, 1_u32);
        while n > 0 {
            numerator = numerator * base + n % base;
            denominator *= base;
            n /= base;
        }
        (f64::from(numerator) / f64::from(denominator) - 0.5) as f32
    }
    let mut xs = std::collections::BTreeSet::new();
    let mut ys = std::collections::BTreeSet::new();
    for phase in 0..32 {
        let sample = Sequence::Halton32.sample(phase);
        assert_eq!(sample, [reference(phase + 1, 2), reference(phase + 1, 3)]);
        assert!(sample.iter().all(|p| p.abs() < 0.5));
        xs.insert(sample[0].to_bits());
        ys.insert(sample[1].to_bits());
    }
    assert_eq!((xs.len(), ys.len()), (32, 32));
    assert_eq!(Sequence::Halton32.sample(0), [0., -1. / 6.]);
    for phase in 0..8 {
        assert_eq!(
            Sequence::Incant8.sample(phase),
            crate::temporal::jitter::SAMPLES[phase as usize]
        );
        assert_eq!(
            Sequence::Incant8.ndc(phase, [0., 0., 480., 270.]),
            crate::temporal::jitter::ndc(phase, [0., 0., 480., 270.])
        );
    }
}
