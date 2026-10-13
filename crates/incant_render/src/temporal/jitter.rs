//! Fixed stratified, zero-mean subpixel sequence. Units are internal pixels,
//! positive right/down. It advances only with the owning accepted submission.
pub(crate) const SAMPLES: [[f32; 2]; 8] = [
    [-0.375, -0.125],
    [0.125, 0.375],
    [0.375, -0.375],
    [-0.125, 0.125],
    [0.375, 0.125],
    [-0.125, -0.375],
    [-0.375, 0.375],
    [0.125, -0.125],
];
pub(crate) fn ndc(phase: u32, rect: [f32; 4]) -> [f32; 2] {
    let p = SAMPLES[phase as usize % SAMPLES.len()];
    [2. * p[0] / rect[2], -2. * p[1] / rect[3]]
}
/// Conservative receiver/cluster guard includes all phases (at most half pixel).
pub(crate) fn guard(rect: [f32; 4]) -> [f32; 2] {
    [1. / rect[2], 1. / rect[3]]
}

/// Two signed eighth-pixel deltas in one exactly represented f32 byte. The
/// zero-delta code is 136 (not zero); missing phases describe an unjittered view.
pub(crate) fn delta_code(previous: Option<u32>, current: Option<u32>) -> f32 {
    let sample = |phase: Option<u32>| phase.map_or([0.; 2], |p| SAMPLES[p as usize]);
    let a = sample(previous);
    let b = sample(current);
    let x = (8. * (a[0] - b[0]) + 8.) as u32;
    let y = (8. * (a[1] - b[1]) + 8.) as u32;
    (x | (y << 4)) as f32
}
