# 0015 HDR output: tone-mapping design, curve correction and final visual review — result

## Status

**Final verdict: approved for the corrected curve and the native integration, within
this scoped preview.** The details are in the section "Final corrected headless and
native review" at the end of this file. This approves the preview's appearance only.
No phase gate is approved or claimed, and the specular environment is still missing.

- **First pass (design):** complete. It chose Khronos PBR Neutral, which the
  correction later superseded.
- **Headless review of the initial integration:** complete. It found the color
  regression. Those captures and comparison sheets are historical and describe the
  old curve only.
- **Curve correction:** complete. `tone_map.wgsl` holds the Incant derived preview
  curve.
- **Final review:** complete. I reviewed 24 fresh headless captures in
  `artifacts/hdr-final/` and two native screenshots in `artifacts/native-hdr-final/`.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0015-hdr-output/brief.md`. The first pass had no priority
  revisions. The current packet adds a priority follow-up: review the integrated HDR
  captures, apply the corrected license pin, and treat function-local exposure as
  settled. Those are applied below. A later priority correction gave me ownership of
  the curve itself. That correction was **Astra's routine integration decision**, made
  under the director's standing instruction to continue sensible work. It was not newly
  typed director feedback, and earlier wording in this file that credited the director
  is corrected here. The initial curve selection and the fixed exposure of 1.0 were
  also routine agent choices. The final packet asks for review only, and this file's
  last section records it.

## Changed paths

- `crates/incant_render/src/tone_map.wgsl` (new). It defines only
  `fn tone_map(color: vec3f) -> vec3f`. It has no entry points, bindings, transfer
  function, alpha handling or module-scope names other than `tone_map`.
- `crates/incant_render/src/studio.rs`. Comments only. `EYE`, `LIGHT_DIRECTION`,
  `LIGHT_RADIANCE` and `DIFFUSE_ENVIRONMENT` are unchanged.
- `handoffs/0015-hdr-output/result.md` (this file).
- `handoffs/0015-hdr-output/review/*.png`, follow-up only. Four before-and-after
  comparison sheets built from Astra's readbacks.
- `crates/incant_render/src/studio.rs`, follow-up only. Comments now record the
  headless review. The constants are still unchanged.

- Priority correction: `crates/incant_render/src/tone_map.wgsl` now holds the revised
  curve. `THIRD_PARTY_NOTICES.md` updates the modification notice.
  `crates/incant_render/src/studio.rs` changes comments only.

No other code, UI, fixture, test, render resource or composition math was edited in
any pass.

## Chosen operator: Khronos PBR Neutral (first pass, superseded by the correction below)

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
| Pinned commit | `180b1a7bddec33f73fe41712a2963cc3ad8e5547`, corrected by Astra during integration |
| Code license | Apache-2.0, Copyright 2024 The Khronos Group Inc., per the repository's `.reuse/dep5` |
| Specification | `PBR_Neutral/README.md`, CC-BY-4.0. It is cited as a reference only, and no text was copied |
| Upstream NOTICE file | None exists, so there is no NOTICE text to propagate |

I fetched these directly from the primary repository on 2026-10-09. The first pass
pinned `f5dc101`, the last commit that changed the GLSL file. That commit predates
`.reuse/dep5`, which returns 404 there, so it could not anchor the license declaration.
I re-checked Astra's correction upstream. At `180b1a7`, `dep5` assigns Apache-2.0 to
`pbrNeutral.glsl`, and the GLSL is byte-identical at both commits.

Apache-2.0 obligations and how they are handled:

- **Retain copyright and license identification.** The snippet header carries the SPDX
  identifier, the Khronos copyright line and the pinned source URL.
- **State modifications.** The header says the file was ported to WGSL, gained a fixed
  exposure and had its commentary rewritten. The constants and operations are unchanged.
- **Provide a copy of the license with distributions.** Astra resolved this during
  integration. The full text is in `licenses/Apache-2.0.txt`, and the attribution is in
  `THIRD_PARTY_NOTICES.md`. Astra reports that the development app bundle and the
  desktop CI artifacts include both. I did not inspect a built bundle.

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

## Expected appearance after integration (first-pass predictions)

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

## Pending pixel review (first-pass plan; headless outcome below)

Once Astra's integration lands, I will review headless and native captures for:

1. Exact backdrop color and alpha-edge composition. Partly covered scene edges must
   show no halos, given HDR blending before tone mapping and backdrop composition after.
2. Diffuse parity with the 0014 captures, especially dark albedos on the fill-lit side.
3. Shoulder behavior on specular highlights, with no banding. RGBA16Float precision
   near the shoulder start is worth checking.
4. Whether exposure 1.0 and the current radiance and fill still read well, or need retuning.

No screenshots exist for this pass, and none are claimed.

## First-pass open questions, now resolved

1. **License text distribution.** Resolved by Astra with `licenses/Apache-2.0.txt` and
   `THIRD_PARTY_NOTICES.md`. The project's own license is still undecided, per
   PLAN.md open question 6, but that is separate from this notice.
2. **Exposure ownership.** Resolved by the packet. Exposure stays a function-local
   constant until authored renderer settings exist.
3. **Composition order.** Resolved by the integrated `output.wgsl`, which I read.
   Chrome and backdrop are converted to linear display RGB, mixed with the tone-mapped
   scene by coverage, and sRGB-encoded once at the end. They never pass through
   `tone_map`. The captures confirm this.

## Headless capture review (priority follow-up)

Reviewed on 2026-10-09 by Claude Opus 5.5, model ID `claude-opus-5-5`. I reviewed all
20 PNGs in `artifacts/hdr-review/`, which are Astra's GPU readbacks. I compared them
with the 19 captures in `artifacts/material-before/`. I viewed every image and decoded
every pixel. The textured-cube and default-metal captures come with the native
follow-up, so they are not part of this review.

### Verdict

| Area | Verdict for these headless captures |
|---|---|
| Neutral #141519 backdrop | **Approved.** Exact in all 20 captures |
| Transparency and composition | **Approved.** Exact to linear-space math, with no halos |
| Specular shoulder | **Approved.** Smooth, neutral, unclipped |
| Neutral and grey materials | **Approved.** Neutral, about 6 to 7 code values darker |
| Saturated colors at or below 0.8 | **Approved.** Unchanged |
| Muted and textured color fidelity | **Not approved.** Chroma rises visibly |
| Emissive authored colors | **Not approved.** Same toe effect, and no future fix applies |
| Dark-material legibility | **Not judged.** No dark-albedo fixture exists. The swatch shows strong toe crush |

No tuning was made. `studio.rs` and the exposure constant are unchanged. The reasons
are below.

### Evidence

Each sheet shows the before capture on the left and the HDR capture on the right.
Magenta panels mark new fixtures that have no before capture.

- `handoffs/0015-hdr-output/review/colors-before-after.png`: base-color texture,
  reimported blue texture, metallic-roughness map and emissive map.
- `handoffs/0015-hdr-output/review/neutrals-before-after.png`: lit double-sided back,
  varying normal map, normal map and occlusion map.
- `handoffs/0015-hdr-output/review/transparency-before-after.png`: layered transparency,
  opaque between transparent, opaque alpha zero and the transparent-over-black fixture.
- `handoffs/0015-hdr-output/review/highlight-and-emissive-before-after.png`: glossy
  highlight, linear filter, sixteen-bit color and double-sided back.

The sheets are derived from Astra's readbacks only, by pixel copy. I built them, and
decoded the PNGs, with a pure-Python decoder and encoder in the gitignored
`target/hdr-review-tools/`, which is not committed. I took no screenshots of my own and
used no shell screen capture.

### Findings

**1. The backdrop is preserved exactly.** Every capture contains #141519 as
(20, 21, 25). Its pixel count matches the before capture in every pair, so tone mapping
never touches it. `mask-discarded` is identical to its before capture.

**2. Transparency blends in HDR and composes correctly.** The center of the
transparent-over-black fixture reads 181, which is 0.46 linear as the packet predicts.
A strip where the half-alpha white layer covers only the backdrop reads 177. That
matches an independent computation: tone-map 1.0 to 0.869, then mix it 50% with the
linear backdrop. Where only the opaque black layer shows, pixels read (0, 0, 0). No
other values appear anywhere, so there are no halos or fringes. Layered transparency
keeps its center (188, 6, 138) within one code value of before.

**3. The specular shoulder behaves as designed.** The glossy fixture is neutral in
every pixel. It peaks at 249 with a soft plateau and falls to 237 at the quad edge, in
one-code steps across both axes. There is no banding and no clip to 255. The highlight
reads as a bright, soft hotspot rather than a hard white disc. The quad spans only 12
code values, so the highlight is gentle. That is acceptable for a neutral preview, and
a specular environment will change it anyway.

**4. The reference swatches match the specification.** All 12 columns are uniform and
equal Astra's test values: 0, 2, 105, 181, 226, 240, 250, 253, 255, 255, then
(253, 156, 156) and (255, 255, 255). Saturated red at f16 maximum turning white is the
intended desaturation of extreme highlights.

**5. Neutral materials stay neutral and slightly darker.** Lit greys drop by the
designed 0.04 linear offset. The lit white face moves from 231 to 225. The normal map
moves from 182 to 175, and the occlusion map from 187 to 180. The relative occlusion
and normal-map contrast is unchanged. I judge this loss of brightness too small to
justify retuning.

**6. Muted colors gain chroma, which is a visible regression.** HSL saturation in the
table is computed from the sRGB values.

| Capture | Before | After | Saturation before, after |
|---|---|---|---|
| base-color-texture | (117, 61, 36) | (112, 49, 6) | 0.53, 0.90 |
| reimported-blue-texture | (36, 117, 231) | (7, 112, 228) | 0.80, 0.94 |
| metallic-roughness-map | (167, 132, 98) | (159, 121, 80) | 0.28, 0.33 |
| emissive-map and sixteen-bit-color | (128, 64, 32) | (124, 55, 4) | 0.60, 0.94 |
| opaque-alpha-zero, emissive red 1.0 | (255, 0, 0) | (241, 34, 34) | 1.00, 0.88 |

Hue angle is preserved, as the operator guarantees. Perceptually, though, the brown
texture reads as burnt orange and the soft blue reads as a pure saturated blue. A
material preview that turns authored brown into orange does not meet the brief's
"preserve color relationships in ordinary material values" for these colors. Linear
filter and mip captures, (188, 188, 0), are unchanged, because a zero channel incurs
no offset.

**Cause.** PBR Neutral subtracts 0.04 from every channel, with a quadratic toe below
0.08. The operator assumes that a glossy dielectric under a roughly unit-intensity white
environment always carries about 0.04 of Fresnel reflection, and the offset removes it.
This preview has no specular environment, so the offset removes energy that was never
added. A dark channel such as the brown's blue, at 0.017 linear, collapses to 0.002.
I checked this diagnosis numerically by adding a uniform specular lift before the
operator:

| Color, before HDR | No lift | Lift 0.012, about 0.04 times the 0.30 fill | Lift 0.04, unit environment |
|---|---|---|---|
| brown (117, 61, 36) | (112, 49, 6) | (113, 52, 17) | (118, 63, 40) |
| blue (36, 117, 231) | (7, 112, 229) | (17, 113, 229) | (40, 118, 231) |
| tan (167, 132, 98) | (159, 121, 81) | (161, 124, 86) | (167, 132, 98) |

With a unit-intensity specular environment, the authored colors return to within 4
code values. Emissive and other unlit content never receives that lift, so emissive
colors keep the shift whatever lighting is added.

**Why I did not tune.** None of the permitted controls fixes the cause.

- **Exposure scales colors but cannot undo a constant offset.** At exposure 2.0 the
  brown's saturation is still 0.75, against 0.53 before. Whites sit at 249, deep in the
  shoulder, and emissive red turns visibly pink at (251, 101, 101). Changing exposure
  would also invalidate Astra's fixed emissive and swatch expectations without a visual
  gain.
- **Radiance and fill scale lit colors proportionally,** with the same limitation. A
  lift of about 8% would restore neutral brightness to the 0014 values, but neutrals
  already read well. The specular increment will recalibrate studio levels anyway.

**7. Dark-material legibility cannot be judged from these captures.** No fixture
contains a dark albedo. The only evidence is the 0.01 swatch, which displays as 2,
against 25 without tone mapping, plus the collapsed dark channels in finding 6. The
first-pass prediction still stands: a 0.18 albedo lit only by the fill displays near 37,
against 66 before. I request a dark-albedo fixture, such as albedo 0.04 and 0.1 on a
fill-lit face, for the next capture round.

### Recommendations

**Superseded.** These recommendations applied to the initial curve. The curve
correction below removed the toe, so recommendations 1 and 2 no longer apply. Astra
delivered recommendation 3 as the fill-only dark-material fixtures. No director
decision is needed.

1. **Calibrate the specular-environment increment to the operator.** Dielectrics should
   receive close to unit-intensity F0 reflection, about 0.04, before re-review. The table
   above predicts that this restores textured color fidelity.
2. **Decide whether emissive and unlit fidelity matters.** If authored emissive colors
   must display as authored, PBR Neutral's toe will always shift them. That would argue
   for revisiting the operator or its offset parameter later. The decision belongs to
   the director, not to me.
3. **Add a dark-albedo fixture,** as described in finding 7.

### Native review

Native 1440x900 and 1000x650 captures will follow after Astra's build. Rounded-mask
edges, chrome colors and editor composition at those sizes remain unreviewed and
unapproved. Only Astra captures native UI.

### Follow-up commands

```
python3 -I target/hdr-review-tools/stats.py artifacts/hdr-review artifacts/material-before
python3 -I target/hdr-review-tools/sheet.py artifacts/hdr-review artifacts/material-before <out.png> <names...>
```

These scripts are scratch tools, not committed. No build, cargo test or GPU run was
performed in this follow-up, as the packet allows.

## Priority correction: derived preview curve

Implemented on 2026-10-09 by Claude Opus 5.5, model ID `claude-opus-5-5`. This
section supersedes the first-pass operator choice and the sample table above.

### What changed

`tone_map.wgsl` now implements an **Incant derived preview curve**. It is not Khronos
PBR Neutral and does not conform to that specification. The file header, the comment
block and `THIRD_PARTY_NOTICES.md` all say so.

| Element | Khronos PBR Neutral | Incant derived curve |
|---|---|---|
| Fresnel toe offset | Subtracts 0.04, quadratic below 0.08 | **Removed** |
| Region where output equals input | None. The offset always applies | Peak channel below **0.8** |
| Compression threshold | 0.76 | **0.8** |
| Shoulder formula | 1 − (1−Ks)² / (p + 1 − 2Ks) | Same formula, with Ks = 0.8 |
| Desaturation toward white | Kd = 0.15 | Same |
| Exposure | Not part of the reference | Fixed 1.0, unchanged |

The curve adds no constant to scene radiance and fakes no IBL contribution. Exposure
stays 1.0, because no capture justifies changing it.

### Why this curve and threshold

The headless review showed that the toe offset was the only cause of the regression.
Without a matching specular sheen, it subtracted energy from every color.

- **Removing the offset** makes the curve the identity for ordinary colors. Emissive
  colors, dark fill-lit materials and muted textures now display exactly as rendered.
- **Threshold 0.8** keeps every albedo up to 0.8 under unit white light untouched. That
  matches the top of the base-color range Khronos itself preserves. The studio's
  camera-facing white face, about 0.79 linear, stays exact.
- **0.8 rather than 0.85.** A higher threshold leaves less room for highlights. At 0.85,
  radiance 1 to 4 fits into 246 to 254. At 0.8 it spans 243 to 254, so specular
  highlights keep visible gradation.
- **0.8 rather than 0.76.** The reference threshold would compress ordinary albedos
  between 0.76 and 0.8 for no visible gain in the highlights.
- **The shoulder is still smooth.** At 0.8 its value equals 0.8 and its slope equals 1,
  so there is no visible band. The desaturation blend also starts at zero with zero
  slope.
- **Trade-off at 1.0.** Saturated colors at display maximum take a small hue-preserving
  desaturation. Red [1,0,0] shows as about [243, 30, 30] and still reads clearly red.
  The alternative is clipping, which would lose all highlight gradation above 1.

### Independent expected outputs

I computed these with an independent Python implementation of the revised equations,
in `target/hdr-review-tools/derived.py`, which is gitignored and not committed. They do
not come from GPU execution. The last column models the RGBA16Float input and 32-bit
shader math, and it agrees with the 64-bit result everywhere. The sRGB columns use the
standard piecewise encode with 8-bit rounding.

| Linear input | Linear output | sRGB 8-bit |
|---|---|---|
| 0 | 0 | 0, 0, 0 |
| 0.01 gray | 0.01 | 25, 25, 25 |
| 0.18 gray | 0.18 | 118, 118, 118 |
| 0.5 gray | 0.5 | 188, 188, 188 |
| 0.8 gray | 0.8 | 231, 231, 231 |
| 1 gray | 0.9 | 243, 243, 243 |
| 2 gray | 0.971429 | 252, 252, 252 |
| 4 gray | 0.988235 | 254, 254, 254 |
| 16 gray | 0.997403 | 255, 255, 255 |
| 65504 gray | 0.999999 | 255, 255, 255 |
| [4, 0.2, 0.2] | 0.988235, 0.341558, 0.341558 | 254, 158, 158 |
| [65504, 0, 0] | 0.999999, 0.999898, 0.999898 | 255, 255, 255 |
| [1, 0, 0] | 0.9, 0.0133005, 0.0133005 | 243, 30, 30 |
| [0.5, 0.5, 0] | 0.5, 0.5, 0 | 188, 188, 0 |
| Emissive brown, sRGB 128, 64, 32 decoded | unchanged: 0.215861, 0.0512695, 0.0144438 | 128, 64, 32 |
| Fill-only gray albedo 0.1, 0.03 linear | 0.03 | 48, 48, 48 |
| Fill-only gray albedo 0.18, 0.054 linear | 0.054 | 66, 66, 66 |
| Fill-only gray albedo 0.5, 0.15 linear | 0.15 | 108, 108, 108 |
| Camera-facing white face, 0.7889 | 0.7889 | 230, 230, 230 |
| Fully lit white face, 0.9112 | 0.871465 | 240, 240, 240 |

The fill-only rows assume the 0.30 diffuse fill with no direct light, and they ignore the
very small Fresnel reduction of diffuse. Astra's real fixture is authoritative. The
16-bit emissive texture previously read 127 rather than 128 in the red channel. Any such
input quantization passes straight through the identity region.

Checks in the same script:

- 300,000 random inputs up to 65504, in 64-bit and 32-bit math: no output was
  non-finite or outside [0, 1].
- The same inputs show no hue shift. Each output minus its mean stays parallel to, and
  points the same way as, the input minus its mean.
- A neutral ramp from 1e-6 to 65504 is monotonically nondecreasing.
- At 0.8 the value is continuous, and the slope on the shoulder side is 1.0000 to four
  places.

**WGSL validation.** The revised snippet parses and validates with naga 29.0.4:

```
cd target/tonemap-check
~/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin/cargo run --offline -q -- ../../crates/incant_render/src/tone_map.wgsl
naga 29.0.4: tone_map.wgsl parsed and validated
```

`rustfmt --edition 2024 --check crates/incant_render/src/studio.rs` passed. No cargo
build, test or GPU run was performed, as the packet allows.

### Effect on Astra's existing assertions

These expectations change, and Astra owns updating them:

- `DISPLAY_RED` and `DISPLAY_GREEN` change from 241, 33, 33 to **243, 30, 30**, and
  the equivalent for green.
- The emissive-map and sixteen-bit-color expectations change from 124, 55, 4 back to
  the authored **128, 64, 32**. The sixteen-bit capture previously read 127.
- The swatch table changes to the values above. The changed entries are 0.01 to 25,
  0.18 to 118, 0.5 to 188, 0.8 to 231, 1 to 243, 2 to 252, 4 to 254, and
  [4, 0.2, 0.2] to 254, 158, 158.
- **The transparent-over-black center** now maps 0.5 radiance to 0.5 linear, sRGB 188,
  rather than 0.46. Where the half-alpha white covers only the backdrop, the expected
  value is 179, 180, 180. The same formula reproduces the old capture's 177, 177, 177.
- **The glossy-highlight assertion** expects a value above 243 and below 255. Its comment
  says premature UNORM clipping would yield 240, which was the reference curve's value
  for 1.0. Under the new curve, 1.0 maps to 243, so the clipping detector needs a new
  bound. The asserted range itself may still hold, depending on the fixture's peak.

### Pending

- Fresh headless captures, including the new fill-only dark-material fixture with gray
  albedos 0.1, 0.18 and 0.5. I will review them before anything is approved.
- Fresh native captures at 1440x900 and 1000x650.
- The current `artifacts/hdr-review` captures are historical. They approve nothing
  about the new curve.

No director decision or permission request is needed for this correction. It was a
routine agent design change, not a director action.

## Final corrected headless and native review

Reviewed on 2026-10-09 by Claude Opus 5.5, model ID `claude-opus-5-5`. This is a
review-only pass. I made no builds, new captures, or code, UI, lighting or curve
edits. `tone_map.wgsl` and `studio.rs` are byte-identical to my correction commit
`f1a3364`. Only Astra's captures were inspected. This verdict covers only the
corrected curve. The initial `artifacts/hdr-review` captures and the
`handoffs/0015-hdr-output/review/*.png` comparison sheets show the superseded curve.

### Inspected files

- All 24 PNGs in `artifacts/hdr-final/`: base-color-texture, dark-fill-10,
  dark-fill-18, dark-fill-50, double-sided-back, emissive-map, hdr-glossy-highlight,
  hdr-reference-swatches, hdr-transparent-over-black, layered-transparency,
  linear-filter, lit-double-sided-back, mask-discarded, metallic-roughness-map,
  minified-color-mips, normal-map, occlusion-map, opaque-alpha-zero,
  opaque-between-transparent, reflected-single-sided, reimported-blue-texture,
  sixteen-bit-color, textured-cube and varying-normal-map.
- The 19 overlapping captures in `artifacts/material-before/`, for comparison.
  default-metal is historical and has no new counterpart, so it was not re-reviewed.
- `artifacts/native-hdr-final/07-final-wide.jpg`, 2880x1748 physical pixels.
- `artifacts/native-hdr-final/08-final-minimum.jpg`, 2000x1300 physical pixels.

**Method.** I viewed every image. I decoded every headless pixel with the scratch
pure-Python decoder in the gitignored `target/hdr-review-tools/`. For the native
JPEGs, I converted them to PNG with macOS `sips`, averaged 13x13 pixel patches, and
viewed corner crops. All of this was file processing only. I took no screen capture.

### Verdict

| Area | Verdict |
|---|---|
| Ordinary material colors | **Approved.** Identical to the 0014 captures within one code value |
| Emissive colors | **Approved.** The authored brown is exact |
| Dark fill-lit materials | **Approved.** Exact to prediction and distinct from the backdrop |
| Highlight shoulder | **Approved.** Smooth, neutral, unclipped |
| Transparency and composition | **Approved.** Exact to the linear-space prediction |
| #141519 backdrop | **Approved.** Exact in every headless capture |
| Native integration at both sizes | **Approved.** Matches headless output after color-profile conversion |
| Specular environment | **Still missing.** This is explicit and out of scope |

### Headless findings

**1. Ordinary colors are stable.** These captures are pixel-identical to their 0014
counterparts: emissive-map, sixteen-bit-color, lit-double-sided-back, normal-map,
occlusion-map, linear-filter, minified-color-mips and mask-discarded. In
base-color-texture, metallic-roughness-map, reimported-blue-texture, varying-normal-map
and textured-cube, a small share of pixels changed, and none by more than one code
value. That is 16-bit scene-target quantization. The brown, soft blue, tan and the
orange and blue cube faces read exactly as before. The regression found earlier is
fixed.

| Capture center | 0014 | Initial HDR curve | Corrected curve |
|---|---|---|---|
| base-color-texture | 117, 61, 36 | 112, 49, 6 | **117, 61, 36** |
| reimported-blue-texture | 36, 117, 231 | 7, 112, 228 | **36, 117, 231** |
| emissive-map | 128, 64, 32 | 124, 55, 4 | **128, 64, 32** |
| lit-double-sided-back | 231 gray | 225 gray | **231 gray** |

**2. Dark materials are legible.** The three fill-only fixtures are uniform and exact.

| Fixture | Predicted | Captured |
|---|---|---|
| dark-fill-10 | 48 | 48 |
| dark-fill-18 | 66 | 66 |
| dark-fill-50 | 108 | 108 |

The 0.1 albedo is quiet against the #141519 backdrop, but its edge is clearly
readable. That is the honest look of a 0.1 albedo under a 0.30 fill. It is not crushed
by the output transform.

**3. The highlight shoulder is controlled.** hdr-glossy-highlight is neutral in every
pixel. It rises from 241 at the edge to a 251 peak in one-code steps on both axes, with
no banding and no clip to 255. It reads as a bright, soft hotspot. Compared with the
initial curve's 237 to 249, it has a little more lift and still keeps its gradation.

**4. The swatches match my expected values exactly.** All 12 columns are uniform:
0, 25, 118, 188, 231, 243, 252, 254, 255, 255, then (254, 158, 158) and
(255, 255, 255).

**5. Transparency composes exactly.** The transparent-over-black center is 188, which
is 0.5 linear. The half-alpha strip over the backdrop is (179, 180, 180), and the black
showing through is (0, 0, 0). All three match my predictions, and no other values occur.
In layered-transparency and opaque-between-transparent, the thin edge slivers are
full-strength red and green emission at about 1.0. They now show the shoulder's slight,
hue-preserving desaturation, (179, 26, 28) and (30, 243, 30). That is the intended
behavior at display maximum.

**6. Full-strength red keeps its hue.** opaque-alpha-zero, reflected-single-sided and
double-sided-back read (243, 30, 30), as predicted. The red stays clearly red, and the
roughly 12-code drop from 255 is the deliberate highlight headroom.

**7. The backdrop is preserved.** (20, 21, 25) appears with identical pixel counts in
every capture. mask-discarded is the backdrop alone.

### Native findings

Both screenshots show the rebuilt app in the Attached state with zero problems. The
cube project, four history items and the saved account are restored. The account label
is deliberately not repeated here. The purple badge at the top left is macOS recording
chrome, not app UI.

**Color matches the headless output.** The JPEGs carry the "Color LCD" display profile,
not sRGB, so their raw values are display-space values. I converted the headless sRGB
values to Display P3 as an approximation of that profile. The native samples then match
within two code values, well inside JPEG error.

| Sample | Headless sRGB | Converted to P3 | Native, both sizes |
|---|---|---|---|
| Orange front face | 199, 136, 65 | 190, 139, 78 | 188, 139, 77 |
| Blue front face | 40, 109, 199 | 60, 108, 193 | 59, 107, 192 |
| Orange right face | 156, 106, 48 | 149, 108, 59 | 149, 108, 58 |
| Viewport backdrop | 20, 21, 25 | 20, 21, 25 | 19, 21, 24 |

The face samples are identical in both screenshots, so viewport size does not affect the
output.

**Layout and integration.**
- **Wide, 1440x874 logical:** the cube sits centered in the viewport with comfortable
  margins. The cube reads as the same object as the headless capture. Face shading
  separates three planes clearly, and the top faces are slightly darker than the front.
- **Minimum, 1000x650 logical:** the viewport shrinks and the cube scales down without
  clipping. Panels, tabs, Problems, the Inspector and the Agent area stay legible. The
  Inspector's Identifiers row falls below the fold, consistent with the scroll area that
  0014 verified.
- **Edges:** corner crops of the wide capture show the viewport meeting the panel
  borders cleanly. There is no halo, seam or tone-mapped tint on the chrome. The chrome
  and the panel background stay neutral and distinct from the viewport backdrop.

### Remaining limitations

- **No specular environment.** Metals and glossy dielectrics have only direct
  highlights. The default-metal capture is historical and stays a flat gray until real
  IBL exists, which will be reviewed on its own merits.
- **Scoped preview only.** There is no authored lighting, bloom, antialiasing, shadows
  or clustered forward+. This is not the production render graph.
- **Approximate native color comparison.** JPEG compression and the display profile
  make the native comparison approximate. Display P3 stood in for the "Color LCD"
  profile. Exact numerical checks rest on the headless readbacks and Astra's GPU
  assertions.
- **The wide capture is 874 logical pixels tall,** not 900, because of the available
  desktop height. I reviewed nothing at exactly 1440x900.
- **Static screenshots only.** Resize behavior and Inspector scrolling come from
  Astra's verification and the 0014 review. I did not observe them in motion.
- **Inputs to the GPU-test claims.** The test results cited here, including Clippy,
  the 124 Rust tests, the eleven GPU tests and the 282 UI tests, are Astra's reports. I
  did not rerun them.

No phase gate is approved or claimed by this review.
