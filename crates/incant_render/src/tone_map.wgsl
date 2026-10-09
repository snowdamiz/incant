// SPDX-License-Identifier: Apache-2.0
// Copyright 2024 The Khronos Group, Inc. (Khronos PBR Neutral reference).
// Modified by Incant (2026): ported from GLSL to WGSL; the Fresnel toe offset
// removed; highlight compression threshold changed from 0.76 to 0.8; fixed
// preview exposure added; commentary rewritten. The shoulder formula and the
// 0.15 desaturation constant are kept from the reference.
//
// Incant derived preview curve. This is NOT Khronos PBR Neutral and does not
// conform to that specification: the reference subtracts a 0.04 Fresnel offset
// from every color, and this curve does not. It derives its shoulder from:
//   Reference code: https://github.com/KhronosGroup/ToneMapping/blob/
//     180b1a7bddec33f73fe41712a2963cc3ad8e5547/PBR_Neutral/pbrNeutral.glsl
//     (Apache-2.0 per the repository's .reuse/dep5).
//   Specification: PBR_Neutral/README.md in the same repository (CC-BY-4.0).
//
// This is not ACES and makes no ACES claim. There is no RRT, ODT, AP0/AP1 gamut
// transform or filmic contrast curve here. It is one neutral, hue-preserving
// highlight compressor for linear Rec. 709 input, chosen for material preview.
//
// Contract with the postprocessing pass that appends this snippet:
//   input:  finite, nonnegative linear Rec. 709 scene RGB, up to f16 max 65504.
//   output: finite linear Rec. 709 display RGB in [0, 1]. Black stays black.
// The caller owns alpha, the sRGB transfer function, bindings and entry points.
// The display-referred #141519 viewport backdrop and editor chrome never pass
// through here; the caller composes them after this transform using scene alpha.
//
// Visual intent:
//   * Ordinary scene colors are shown exactly as rendered. Any color whose
//     brightest channel is below 0.8 passes through unchanged: lit and
//     fill-lit materials, dark albedos and emissive colors keep their value,
//     saturation and hue. Albedos up to 0.8 under unit white light fall in
//     this range.
//   * Why no toe offset: Khronos PBR Neutral subtracts 0.04 to cancel the
//     Fresnel sheen a unit white environment adds to every dielectric. Scenes
//     without that exact sheen, including all emissive and unlit content, lose
//     energy they never had: muted colors over-saturate and dark channels
//     crush toward black. Headless review in handoff 0015 measured this, so the
//     offset is gone. Future image-based lighting is judged on its own merits.
//   * Bright highlights get a smooth shoulder instead of a hard clip. Above a
//     peak channel of 0.8 the peak compresses asymptotically toward 1.0. The
//     value and slope match the identity at 0.8, so no Mach band appears where
//     the shoulder begins. Radiance 1 shows near sRGB 243, 2 near 252, and 4
//     near 254, leaving visible gradation inside specular highlights.
//   * Hue never shifts. Scaling by the peak and blending toward the neutral
//     axis keep the color within the plane of the input and the neutral axis.
//   * Very bright colors desaturate gradually toward white, the way overexposed
//     highlights read in photographs. The blend starts at zero with zero slope
//     at the threshold, so a color at 1.0 stays clearly saturated; red [1,0,0]
//     shows near sRGB [243, 30, 30].
//
// What this does not do: it cannot supply the missing specular environment.
// Metals and glossy surfaces stay dark outside their direct highlight until a
// later increment adds specular image-based lighting.
fn tone_map(color: vec3f) -> vec3f {
    // Fixed preview exposure. 1.0 keeps studio radiance and fill in scene units.
    const exposure = 1.0;
    // Peak channel value where highlight compression begins. Below it the curve
    // is the identity.
    const start_compression = 0.8;
    // Speed at which compressed highlights desaturate toward white (reference).
    const desaturation = 0.15;
    const shoulder = 1.0 - start_compression;

    let c = color * exposure;

    // Identity below the shoulder. Black and every ordinary color are untouched.
    let peak = max(c.r, max(c.g, c.b));
    if peak < start_compression {
        return c;
    }

    // Shoulder: peak >= 0.8 here, so the denominator is at least 0.2 and the
    // division by peak is safe. new_peak equals 0.8 with slope 1 at the
    // threshold, then approaches but never reaches 1.0.
    let new_peak = 1.0 - shoulder * shoulder / (peak + shoulder - start_compression);
    let compressed = c * (new_peak / peak);

    // Blend toward neutral by how much the peak was compressed. The result is a
    // convex mix of values in [0, new_peak], so it stays within [0, 1].
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(compressed, vec3f(new_peak), g);
}
