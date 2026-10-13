//! Independent CPU reference used by the numeric shader tests (not runtime work).
use super::TemporalConfig;
pub(super) fn clean(color: [f32; 4]) -> [f32; 4] {
    if !color.iter().all(|x| x.is_finite()) || color[3] <= 0. {
        return [0.; 4];
    }
    std::array::from_fn(|i| color[i].clamp(0., if i == 3 { 1. } else { 65504. }))
}
pub(super) fn linear_depth(depth: f32, near: f32, far: f32, ortho: bool) -> Option<f32> {
    if !depth.is_finite() || !(0. ..=1.).contains(&depth) {
        return None;
    }
    Some(if ortho {
        (1. - depth) * near + depth * far
    } else {
        1. / ((1. - depth) / near + depth / far)
    })
}
pub(super) fn depth_matches(
    a: f32,
    b: f32,
    near: f32,
    far: f32,
    ortho: bool,
    c: TemporalConfig,
) -> bool {
    if a >= 1. {
        return false;
    }
    let (Some(a), Some(b)) = (
        linear_depth(a, near, far, ortho),
        linear_depth(b, near, far, ortho),
    ) else {
        return false;
    };
    (a - b).abs() <= c.absolute_depth.max(c.relative_depth * b)
}
pub(super) fn blend(
    current: [f32; 4],
    old: [f32; 4],
    lo: [f32; 4],
    hi: [f32; 4],
    c: TemporalConfig,
) -> [f32; 4] {
    if !current.iter().chain(old.iter()).all(|v| v.is_finite())
        || !(0. ..=1.).contains(&current[3])
        || !(0. ..=1.).contains(&old[3])
    {
        return clean(current);
    }
    let current = clean(current);
    let old = clean(old);
    let difference = (0..3)
        .map(|i| (old[i] - current[i]).abs())
        .fold(0., f32::max);
    let weight = c.history_weight
        * (1. - c.reactive_strength * (difference / c.reactive_threshold).clamp(0., 1.));
    std::array::from_fn(|i| {
        ((1. - weight) * current[i] + weight * old[i].clamp(lo[i], hi[i]))
            .clamp(0., if i == 3 { 1. } else { 65504. })
    })
}

/// Scalar independent history gather. Invalid or zero-weight support is omitted;
/// no finite positive accepted support means disocclusion/current-only.
pub(super) fn gather(taps: [([f64; 4], f64, f64, bool); 4], expected: f64) -> Option<[f64; 4]> {
    if !expected.is_finite() || expected <= 0. {
        return None;
    }
    let valid: Vec<_> = taps
        .into_iter()
        .filter(|(c, z, w, inside)| {
            *inside
                && w.is_finite()
                && *w > 0.
                && *w <= 1.
                && z.is_finite()
                && *z > 0.
                && (*z - expected).abs() <= 0.001
                && c.iter().all(|x| x.is_finite() && *x >= 0. && *x <= 65504.)
                && c[3] == 1.
        })
        .collect();
    let mass: f64 = valid.iter().map(|t| t.2).sum();
    (mass > 0.).then(|| std::array::from_fn(|i| valid.iter().map(|t| t.0[i] * t.2 / mass).sum()))
}
