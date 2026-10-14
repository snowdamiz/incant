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
/// Portable output upscaling only: the same lattice with frames 0-3 moved by
/// (-1/16, -1/16) and frames 4-7 by (+1/16, +1/16). Eight distinct offsets per
/// axis (thin features of 1/8 internal pixel are sampled every cycle), zero mean,
/// and each half cycle still covers all four 2x quadrants. The fixed path keeps
/// SAMPLES; `guard` covers both (at most half a pixel).
pub(crate) const UPSCALE_SAMPLES: [[f32; 2]; 8] = [
    [-0.4375, -0.1875],
    [0.0625, 0.3125],
    [0.3125, -0.4375],
    [-0.1875, 0.0625],
    [0.4375, 0.1875],
    [-0.0625, -0.3125],
    [-0.3125, 0.4375],
    [0.1875, -0.0625],
];
pub(crate) fn offset(phase: u32, upscaled: bool) -> [f32; 2] {
    #[cfg(test)]
    if upscaled && DIAGNOSTIC_HALTON_32.get() {
        // Opposite sign to the external reference, matching Incant's projection
        // convention. This override is reachable only in the ignored diagnostic.
        let halton = |mut index: u32, base: u32| {
            let mut value = 0.;
            let mut fraction = 1. / base as f32;
            while index > 0 {
                value += (index % base) as f32 * fraction;
                index /= base;
                fraction /= base as f32;
            }
            value
        };
        let index = phase % 32 + 1;
        return [0.5 - halton(index, 2), 0.5 - halton(index, 3)];
    }
    let samples = if upscaled { &UPSCALE_SAMPLES } else { &SAMPLES };
    samples[phase as usize % samples.len()]
}
pub(crate) fn phase_count(_upscaled: bool) -> u32 {
    #[cfg(test)]
    if _upscaled && DIAGNOSTIC_HALTON_32.get() {
        return 32;
    }
    8
}
#[cfg(test)]
thread_local! {
    static DIAGNOSTIC_HALTON_32: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static DIAGNOSTIC_INTERNAL_FOOTPRINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Test-only input bandwidth control. Both material color and MASK motion
/// coverage must use the same footprint. This is not a shipping quality setting.
#[cfg(test)]
pub(crate) fn diagnostic_lod_bias(ordinary: f32) -> f32 {
    if DIAGNOSTIC_INTERNAL_FOOTPRINT.get() {
        0.
    } else {
        ordinary
    }
}
#[cfg(test)]
pub(crate) struct DiagnosticInternalFootprint(std::marker::PhantomData<std::rc::Rc<()>>);
#[cfg(test)]
impl DiagnosticInternalFootprint {
    pub(crate) fn begin() -> Self {
        assert!(!DIAGNOSTIC_INTERNAL_FOOTPRINT.replace(true));
        Self(std::marker::PhantomData)
    }
}
#[cfg(test)]
impl Drop for DiagnosticInternalFootprint {
    fn drop(&mut self) {
        DIAGNOSTIC_INTERNAL_FOOTPRINT.set(false);
    }
}
#[cfg(test)]
pub(crate) struct DiagnosticHalton32;
#[cfg(test)]
impl DiagnosticHalton32 {
    pub(crate) fn begin() -> Self {
        assert!(!DIAGNOSTIC_HALTON_32.replace(true));
        Self
    }
}
#[cfg(test)]
impl Drop for DiagnosticHalton32 {
    fn drop(&mut self) {
        DIAGNOSTIC_HALTON_32.set(false);
    }
}
pub(crate) fn ndc_offset(p: [f32; 2], rect: [f32; 4]) -> [f32; 2] {
    [2. * p[0] / rect[2], -2. * p[1] / rect[3]]
}
/// The fixed sequence's NDC jitter; production frames use `ndc_offset(offset(..))`.
#[cfg(test)]
pub(crate) fn ndc(phase: u32, rect: [f32; 4]) -> [f32; 2] {
    ndc_offset(SAMPLES[phase as usize % SAMPLES.len()], rect)
}
/// Conservative receiver/cluster guard includes all phases (at most half pixel).
pub(crate) fn guard(rect: [f32; 4]) -> [f32; 2] {
    [1. / rect[2], 1. / rect[3]]
}
/// Smallest admitted DRS scale, in sixteenths (`ResolutionScale::new` admits 4..=16).
pub(crate) const MIN_SIXTEENTHS: f32 = 4.;
/// Most negative texture LOD bias for internal draws under upscaling. An empirical
/// visual candidate chosen after the R4 4/16 entry artefacts, not a proven
/// alias-free or optimal band limit (eight correlated jitter positions imply no
/// sampling theorem). It binds only below scale 0.354 (4/16, 5/16); pixels pending.
pub(crate) const MIN_LOD_BIAS: f32 = -1.5;
/// DRS-stable guard for an upscaled frame: the guard of the smallest admitted
/// internal rect (4/16 of the physical output rect). Internal rects are exactly
/// output * n/16, so it bounds `guard(rect)` at every admitted scale, and it stays
/// constant across DRS steps: the shadow cascade fit (and its texel quantization)
/// then depends only on the camera and the output rect, not on the internal scale.
pub(crate) fn drs_guard(output: [f32; 4]) -> [f32; 2] {
    guard([
        0.,
        0.,
        output[2] * MIN_SIXTEENTHS / 16.,
        output[3] * MIN_SIXTEENTHS / 16.,
    ])
}
