//! Test-only propagation across two actual RGBA16Float render attachments.
//! The existing per-pass allowance is one binary16 spacing plus 2^-18 relative
//! arithmetic error. We do not change it or assert a hardware rounding mode.
//! RNE is used only to locate inward representable endpoints; the first pass's
//! decoded values, never observed GPU output, form the second pass's input set.
use half::f16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Interval {
    first: u16,
    last: u16,
}
impl Interval {
    fn values(self) -> [f64; 2] {
        [self.first, self.last].map(|bits| f16::from_bits(bits).to_f64())
    }
    pub(crate) fn contains(self, value: f64) -> bool {
        let value = if value == 0. { 0. } else { value };
        let encoded = f16::from_f64(value);
        value.is_finite()
            && value >= 0.
            && encoded.to_f64() == value
            && (self.first..=self.last).contains(&encoded.to_bits())
    }
}

fn representable(lower: f64, upper: f64, maximum: f64) -> Interval {
    assert!(lower.is_finite() && upper.is_finite() && lower <= upper);
    let lower = lower.clamp(0., maximum);
    let upper = upper.clamp(0., maximum);
    // The numeric contract admits either zero sign. Ordered positive half bits
    // require canonical zero, unlike f64::clamp which can retain -0.0.
    let lower = if lower == 0. { 0. } else { lower };
    let upper = if upper == 0. { 0. } else { upper };
    let mut first = f16::from_f64(lower).to_bits();
    let mut last = f16::from_f64(upper).to_bits();
    if f16::from_bits(first).to_f64() < lower {
        first += 1;
    }
    if f16::from_bits(last).to_f64() > upper {
        last -= 1;
    }
    assert!(first <= last && last <= 0x7bff);
    Interval { first, last }
}

fn stored(value: f64, maximum: f64) -> Interval {
    let error = super::half_bound(value);
    representable(value - error, value + error, maximum)
}

/// `over` is the fixture's independently reconstructed, sanitized overlay (its
/// uniform or affine bilinear field). Sanitization matches the composite's
/// finite/nonnegative HDR and alpha contract. This is not an arbitrary rasterizer.
pub(crate) fn composite(history: [f64; 4], over: [f64; 4]) -> [Interval; 4] {
    let history = super::oracle::color(history);
    let over = super::oracle::color(over);
    let maxima = [65504., 65504., 65504., 1.];
    let first: [Interval; 4] = std::array::from_fn(|i| stored(history[i], maxima[i]));
    let alpha = first[3].values();
    if over == [0.; 4] {
        // Loading and storing an already representable value is exact here;
        // do not invent a second error allowance for the identity operation.
        // clean() can clear RGB if a subnormal alpha rounds to zero.
        return std::array::from_fn(|i| Interval {
            first: if i < 3 && alpha[0] == 0. {
                0
            } else {
                first[i].first
            },
            last: first[i].last,
        });
    }
    std::array::from_fn(|i| {
        let mut under = first[i].values();
        if alpha[0] == 0. && i < 3 {
            under[0] = 0.; // clean() clears RGB at a possible zero alpha.
        }
        if alpha[1] == 0. {
            under = [0.; 2];
        }
        let [lower, upper] = under.map(|v| over[i] + v * (1. - over[3]));
        let lower = lower.clamp(0., maxima[i]);
        let upper = upper.clamp(0., maxima[i]);
        // This existing per-pass bound is nondecreasing on the nonnegative
        // domain, including power-of-two binade boundaries. Use its maximum
        // over the interval, then keep only representable final attachment values.
        let error = super::half_bound(upper);
        representable(lower - error, upper + error, maxima[i])
    })
}

#[cfg(test)]
#[path = "attachment_reference_tests.rs"]
mod tests;
