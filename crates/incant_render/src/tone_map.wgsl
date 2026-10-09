// SPDX-License-Identifier: Apache-2.0
// Copyright 2024 The Khronos Group Inc. (Khronos PBR Neutral reference).
// Modified for Incant: ported from GLSL to WGSL, fixed preview exposure added,
// and the commentary rewritten. The curve constants and operations are unchanged.
//
// Source of the curve: Khronos PBR Neutral tone mapper.
//   Reference code: https://github.com/KhronosGroup/ToneMapping/blob/
//     f5dc101149fc5c85c0f9852fe2ba438853e8a7d1/PBR_Neutral/pbrNeutral.glsl
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
//   * Ordinary material values keep their look. While every channel sits in
//     roughly [0.08, 0.8], the output is the input minus a constant 0.04. That
//     offset removes the F0 = 0.04 Fresnel sheen of a face-on glossy dielectric
//     under unit white light, so authored base colors read back as authored.
//   * Hue never shifts. Every step moves the color only within the plane of the
//     input color and the neutral axis, so a red stays the same red and
//     relationships between materials hold as brightness changes.
//   * Bright highlights get a smooth shoulder instead of a hard clip. Above a
//     peak channel of 0.76 the peak is compressed asymptotically toward 1.0,
//     with continuous derivatives so no Mach band appears where the shoulder
//     begins. Very bright colors also desaturate gradually toward white, the
//     way overexposed specular highlights read in photographs, instead of
//     flattening into a saturated slab at the gamut edge.
//   * Deep shadows below 0.08 receive a smaller, quadratic offset that falls to
//     zero at black. Near-black detail is kept and black maps exactly to black.
//
// What this does not do: it cannot supply the missing specular environment.
// Metals and glossy surfaces stay dark outside their direct highlight until the
// next increment adds specular image-based lighting.
fn tone_map(color: vec3f) -> vec3f {
    // Fixed preview exposure. 1.0 keeps the existing studio radiance and fill
    // calibrated in scene units; appearance review may retune it after capture.
    const exposure = 1.0;
    // F90 in the specification: normal-incidence Fresnel of IoR 1.5 dielectrics.
    const f90 = 0.04;
    // Peak value where highlight compression begins (0.8 - F90).
    const start_compression = 0.8 - f90;
    // Speed at which compressed highlights desaturate toward white.
    const desaturation = 0.15;
    const shoulder = 1.0 - start_compression;

    var c = color * exposure;

    // Toe offset: quadratic near black (zero at zero), constant F90 above 2*F90.
    // Both branches are in [0, x], so every channel stays nonnegative.
    let x = min(c.r, min(c.g, c.b));
    let offset = select(f90, x - x * x / (4.0 * f90), x < 2.0 * f90);
    c -= vec3f(offset);

    // Below the shoulder the color passes through unchanged after the offset.
    let peak = max(c.r, max(c.g, c.b));
    if peak < start_compression {
        return c;
    }

    // Shoulder: peak >= 0.76 here, so the denominator is at least 0.24 and the
    // division by peak is safe. new_peak approaches but never reaches 1.0.
    let new_peak = 1.0 - shoulder * shoulder / (peak + shoulder - start_compression);
    c *= new_peak / peak;

    // Blend toward neutral by how much the peak was compressed. The result is a
    // convex mix of values in [0, new_peak], so it stays within [0, 1].
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(c, vec3f(new_peak), g);
}
