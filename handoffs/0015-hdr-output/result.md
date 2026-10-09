# 0015 HDR output: tone-mapping design and headless capture review — result

## Status

- **First pass (design):** complete. The operator is chosen, the WGSL snippet is written
  and the studio commentary is updated.
- **Priority follow-up (headless review):** complete. The verdict is in the section
  "Headless capture review" at the end of this file. Output structure is approved for
  these headless captures. Material color fidelity is **not** approved.
- **Native review:** pending. The 1440x900 and 1000x650 native captures do not exist
  yet, and no native output is approved. No phase gate is approved or claimed.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0015-hdr-output/brief.md`. The first pass had no priority
  revisions. The current packet adds a priority follow-up: review the integrated HDR
  captures, apply the corrected license pin, and treat function-local exposure as
  settled. Those are applied below.

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

No other code, UI, fixture or test was edited in either pass.

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

These are for Astra and the director. Changing the operator is outside this packet.

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
