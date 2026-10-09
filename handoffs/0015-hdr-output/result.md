# 0015 HDR output: tone-mapping design — first-pass result

## Status

First pass complete: operator chosen, WGSL snippet written, studio commentary updated.
**Pixel review pending.** There are no headless or native captures yet, so this
result approves no rendered output. No phase gate is approved or claimed.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0015-hdr-output/brief.md`. It contained no priority revisions or
  new director feedback beyond the brief itself. No earlier partial attempt existed.

## Changed paths

- `crates/incant_render/src/tone_map.wgsl` (new). It defines only
  `fn tone_map(color: vec3f) -> vec3f`. It has no entry points, bindings, transfer
  function, alpha handling or module-scope names other than `tone_map`.
- `crates/incant_render/src/studio.rs`. Comments only. `EYE`, `LIGHT_DIRECTION`,
  `LIGHT_RADIANCE` and `DIFFUSE_ENVIRONMENT` are unchanged.
- `handoffs/0015-hdr-output/result.md` (this file).

No other code, UI, fixture or test was edited.

## Chosen operator: Khronos PBR Neutral

The operator is the Khronos PBR Neutral tone mapper, ported line-for-line from the
Khronos GLSL reference to WGSL. A fixed exposure multiplier of 1.0 precedes it.

**This is not ACES.** It has no RRT, no ODT, no AP0/AP1 gamut transforms and no
filmic contrast curve. Nothing in the code or comments claims otherwise.

### Why this operator

The brief asks to preserve hue and color relationships in ordinary material values
and to give bright highlights a controlled shoulder. PBR Neutral was designed by
Khronos's 3D Commerce group for exactly that case: product and material viewing,
where authored base colors must read back as authored.

- **Ordinary values are kept.** For every channel in [0.08, 0.8], the output equals
  the input minus 0.04. A face-on glossy dielectric under unit white light therefore
  reproduces its base color exactly. Contrast and saturation are unchanged there.
- **No hue shift anywhere.** All changes stay in the plane of the input color and the
  neutral axis. Per-channel curves like Reinhard or Hable shift hue and desaturate
  mid-tones, and ACES-fitted curves add contrast and hue skew to ordinary values.
- **Controlled shoulder.** Above a peak of 0.76 the peak compresses asymptotically to
  1.0 with continuous derivatives, so there is no visible band where the shoulder
  starts. Very bright colors also desaturate gradually toward white, so specular
  highlights read as bright rather than as flat saturated slabs.
- **Black maps to black.** Near-black values get a quadratic offset that falls to zero.
- **Analytically invertible and specified.** The README gives the equations, which
  makes Astra's mathematical review direct.
- **Minimal code.** It needs no dependencies and no lookup table.

### Fixed exposure

Exposure is 1.0, a function-local constant in the snippet. Handoff 0014 calibrated
the studio radiance and fill in scene units against an untone-mapped display. At 1.0,
those values keep their meaning: the camera-facing white face moves from 0.79 to 0.75
linear. Shadows are the exception, as described under expected appearance. Capture
review may retune exposure, radiance or fill. It will not touch
geometry semantics.

## Source and license

| Item | Value |
|---|---|
| Reference code | `PBR_Neutral/pbrNeutral.glsl` in github.com/KhronosGroup/ToneMapping |
| Pinned commit | `f5dc101149fc5c85c0f9852fe2ba438853e8a7d1`, the latest change to that file |
| Code license | Apache-2.0, Copyright 2024 The Khronos Group Inc., per the repository's `.reuse/dep5` |
| Specification | `PBR_Neutral/README.md`, CC-BY-4.0. It is cited as a reference only, and no text was copied |
| Upstream NOTICE file | None exists, so there is no NOTICE text to propagate |

I fetched these directly from the primary repository on 2026-10-09.

Apache-2.0 obligations and how they are handled:

- **Retain copyright and license identification.** The snippet header carries the SPDX
  identifier, the Khronos copyright line and the pinned source URL.
- **State modifications.** The header says the file was ported to WGSL, gained a fixed
  exposure and had its commentary rewritten. The constants and operations are unchanged.
- **Provide a copy of the license with distributions.** This is not done yet. The repo
  has no project license and no third-party notices file, and creating one is outside
  this scope. See the open questions below.

### Port fidelity notes for Astra's review

- `x - 6.25*x*x` is written as `x - x*x / (4.0*f90)`, which is the spec's form. They are
  equal for f90 = 0.04.
- The GLSL uses `x < 0.08` and `peak < startCompression`, while the README uses `<=`.
  Both functions are continuous at those boundaries, so the results are identical.
- The GLSL ternary becomes WGSL `select`, and the early return is kept.
- Range argument: the offset lies in [0, x], so channels stay nonnegative. When the
  shoulder is active, peak is at least 0.76, so every denominator is at least 0.24.
  The final mix is convex over [0, new_peak], and new_peak is below 1.

## Run commands and results

**WGSL validation (naga 29.0.4, the version pinned in Cargo.lock).** I ran it from a
scratch crate under the gitignored `target/tonemap-check/`, which is not committed. A
stand-in fragment entry point calls `tone_map` so the function is reachable.

```
cd target/tonemap-check
~/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin/cargo run --offline -q -- ../../crates/incant_render/src/tone_map.wgsl
naga 29.0.4: tone_map.wgsl parsed and validated
```

**Formatting.** `rustfmt --edition 2024 --check crates/incant_render/src/studio.rs`
passed.

**Numerical samples.** These come from an independent float64 Python port of the
reference GLSL, not from GPU execution. They are the expected values for Astra's
numerical tests. Hue is the angle around the neutral axis.

| Linear input | Expected output | Hue in, out |
|---|---|---|
| 0, 0, 0 | 0.0000, 0.0000, 0.0000 | n/a |
| 0.01 grey | 0.000625 each | n/a |
| 0.04 grey | 0.0100 each | n/a |
| 0.18 grey | 0.1400 each | n/a |
| 0.5 grey | 0.4600 each | n/a |
| 0.8 grey | 0.7600 each | n/a |
| 0.7889 grey: camera-facing white, studio constants | 0.7489 each | n/a |
| 0.9112 grey: fully lit white, studio constants | 0.8360 each | n/a |
| 1.0 grey | 0.8691 each | n/a |
| 2.0 grey | 0.9600 each | n/a |
| 4.0 grey | 0.9833 each | n/a |
| 16 grey | 0.9963 each | n/a |
| 65504 grey | 0.9999991 each, about 1.0 in f32 | n/a |
| 0.8, 0.1, 0.1 | 0.7600, 0.0600, 0.0600 | 0.00°, 0.00° |
| 0.5, 0, 0 | 0.5000, 0.0000, 0.0000 | 0.00°, 0.00° |
| 0.6, 0.3, 0.05 | 0.5656, 0.2656, 0.0156 | 27.00°, 27.00° |
| 4, 0.2, 0.2 | 0.9833, 0.3310, 0.3310 | 0.00°, 0.00° |
| 65504, 0, 0 | 1.0000, 0.9999, 0.9999 | 0.00°, 0.00° |

Sweep checks in the same Python port:

- 200,000 random inputs, including zeros and values up to 65504: no output was
  nonfinite or outside [0, 1].
- A neutral ramp from 1e-6 to 65504 is monotonically nondecreasing.

**Not run, and not required in this pass:** cargo build, clippy, GPU execution of the
snippet, headless captures and native captures. Integration belongs to Astra.

## Expected appearance after integration, to verify in capture review

These are predictions, not observations.

- Bright and mid-tone diffuse surfaces should look close to the 0014 captures. They
  drop by a constant 0.04 linear, so the cost grows as values get darker.
- **Risk: dark surfaces on the fill-lit side will darken noticeably.** The table below
  shows sRGB code values for surfaces lit only by the 0.30 fill. Dark albedos crush
  toward black, the same failure 0014 fixed by raising the fill. This is the curve's
  designed toe, not a porting error. PBR Neutral assumes scenes lit near unit
  intensity, and this studio's fill is much dimmer than that.

| Albedo, fill side only | Linear before | sRGB before | Linear after | sRGB after |
|---|---|---|---|---|
| 1.0 | 0.300 | 149 | 0.260 | 139 |
| 0.5 | 0.150 | 108 | 0.110 | 93 |
| 0.18 | 0.054 | 66 | 0.018 | 37 |
| 0.1 | 0.030 | 48 | 0.0056 | 17 |
| 0.04 | 0.012 | 29 | 0.0009 | 3 |

  If captures confirm the crush, I expect to recommend a higher fill or exposure rather
  than changing the curve. The brief allows that retuning after capture review, and it
  will not be guessed before then.
- Glossy dielectric and metal highlights that clipped to hard white should now roll off
  smoothly, with saturated highlights fading toward white.
- Metals stay dark outside their direct highlight, because no specular environment
  exists yet. The tone map does not hide this, and it is the next increment.
- The #141519 backdrop and editor chrome should match their current colors exactly.
  I will sample backdrop pixels to confirm this.

## Pending pixel review

Once Astra's integration lands, I will review headless and native captures for:

1. Exact backdrop color and alpha-edge composition. Partly covered scene edges must
   show no halos, given HDR blending before tone mapping and backdrop composition after.
2. Diffuse parity with the 0014 captures, especially dark albedos on the fill-lit side.
3. Shoulder behavior on specular highlights, with no banding. RGBA16Float precision
   near the shoulder start is worth checking.
4. Whether exposure 1.0 and the current radiance and fill still read well, or need retuning.

No screenshots exist for this pass, and none are claimed.

## Open questions

1. **License text distribution.** The shipped engine will embed Apache-2.0 code from
   Khronos. Who should add a third-party notices file containing the Apache-2.0 text,
   and where? The project's own license is also undecided, per PLAN.md open question 6.
2. **Exposure ownership.** Exposure is a constant inside the snippet. If Astra prefers
   a uniform, or a Rust-side constant next to the studio values, that is a binding
   change for Astra to make. The curve itself should stay unchanged.
3. **Composition order.** The brief says the backdrop composes after tone mapping and
   the sRGB encode applies after that. Please confirm the encode is applied once, to
   the tone-mapped scene only, and never to the display-referred backdrop.
