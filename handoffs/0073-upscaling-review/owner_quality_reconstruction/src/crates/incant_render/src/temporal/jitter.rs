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
