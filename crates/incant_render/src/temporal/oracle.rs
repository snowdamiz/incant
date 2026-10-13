//! Independent CPU reference used by the numeric shader tests (not runtime work).
use super::TemporalConfig;
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
    if !current.iter().chain(old.iter()).all(|v| v.is_finite()) || current[3] < 1. || old[3] < 1. {
        return current;
    }
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
