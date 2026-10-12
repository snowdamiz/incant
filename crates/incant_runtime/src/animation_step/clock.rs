//! Exact bounded wrap receipts for the same f64 clock expression as playback.
use crate::SceneError;
use std::cmp::Ordering;
const MAX_WRAPS: u64 = 1 << 32;

pub(super) fn advance(
    before: f64,
    duration: f64,
    speed: f64,
    looping: bool,
    playing: bool,
    dt: f64,
) -> Result<(f64, i64, bool), SceneError> {
    if !dt.is_finite()
        || dt <= 0.0
        || !before.is_finite()
        || !duration.is_finite()
        || duration < 0.0
        || !(0.0..=duration).contains(&before)
        || !speed.is_finite()
    {
        return Err(SceneError::Invalid("root-motion clock admission"));
    }
    if !playing {
        return Ok((before, 0, false));
    }
    let mut after = before;
    let completed = crate::animation::advance_clock(&mut after, duration, speed, looping, dt);
    if !after.is_finite() {
        return Err(SceneError::Invalid("root-motion clock overflow"));
    }
    let wraps = if looping && duration > 0.0 && speed != 0.0 {
        // Must match advance_clock, including multiplication/addition order.
        let next = before + dt * speed;
        if !next.is_finite() {
            return Err(SceneError::Invalid("root-motion clock overflow"));
        }
        let mut wraps = floor_quotient(next, duration)?;
        // rem_euclid may round a tiny negative remainder up to the duration;
        // playback selects zero, so the matching cursor advances one cycle.
        if next.rem_euclid(duration) >= duration {
            wraps += 1;
        }
        if wraps.unsigned_abs() > u128::from(MAX_WRAPS) {
            return Err(SceneError::Invalid("root-motion cycle span"));
        }
        i64::try_from(wraps).expect("bounded cycle receipt")
    } else {
        0
    };
    Ok((after, wraps, completed))
}

/// Floating division can round across an integer boundary. Compare the exact
/// binary significands instead, with bounded 33-step integer binary search.
fn floor_quotient(value: f64, divisor: f64) -> Result<i128, SceneError> {
    let (a, ae) = parts(value.abs());
    let (b, be) = parts(divisor);
    let compare = |q: u64| compare_scaled(u128::from(a), ae, u128::from(b) * u128::from(q), be);
    if compare(MAX_WRAPS + 1) != Ordering::Less {
        return Err(SceneError::Invalid("root-motion cycle span"));
    }
    let (mut lo, mut hi) = (0, MAX_WRAPS + 1);
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if compare(mid) == Ordering::Less {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let negative = value < 0.0;
    let magnitude = lo + u64::from(negative && compare(lo) != Ordering::Equal);
    // Permit one negative rounding-correction cycle; advance() checks the final
    // normalized receipt before it can reach a retained integer cursor.
    let signed = i128::from(magnitude);
    Ok(if negative { -signed } else { signed })
}
fn parts(value: f64) -> (u64, i32) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let mantissa = bits & ((1 << 52) - 1);
    if exponent == 0 {
        (mantissa, -1074)
    } else {
        (mantissa | (1 << 52), exponent - 1075)
    }
}
fn compare_scaled(a: u128, ae: i32, b: u128, be: i32) -> Ordering {
    if a == 0 || b == 0 {
        return a.cmp(&b);
    }
    let ab = (128 - a.leading_zeros()) as i32;
    let bb = (128 - b.leading_zeros()) as i32;
    match (ab + ae).cmp(&(bb + be)) {
        Ordering::Equal if ae >= be => (a << (ae - be)).cmp(&b),
        Ordering::Equal => a.cmp(&(b << (be - ae))),
        different => different,
    }
}
