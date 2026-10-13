# 0073 quality repair brief: portable output reconstruction

**Status: design only.** No source, fixture, helper, build, GPU or capture
changed in this session. Production stays at `535003d7` until root schedules the
repair and a new review.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-quality-repair-prep-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Ownership.**

| Owner | Responsibility |
|---|---|
| Astra | Algorithm correctness and integration |
| Claude | Appearance direction now, and pixel acceptance in the next review |

**Basis.**
- Engine source `535003d766bcc1386fd2d2f454c83808a0fcc4c8`, read from git
  objects. Line numbers below are at that commit.
- Review `a5ae3179`:
  - evidence in `evidence/visual/` (manifest `c0552e9a…`);
  - figures in `screenshots/`;
  - findings in `visual-result.md`.

**Scope.**
- **In scope:** the portable WGSL output reconstruction (`Kind::Upscale`), and
  the texture LOD of the passes that feed its history.
- **Unchanged:**
  - MetalFX;
  - cooked/native selection;
  - the fixed-resolution temporal path (0072);
  - DRS nearest expansion;
  - the overlay composite;
  - physical UI.

**Held fixed for the repair and its review.**
- The 0073 fixture, its four cases, crops and figures.
- The review policy: history 0.9, depth 0.05/0.02, reactive 4.0/0.5.
- The FULL and DRS references.
- The output contract checks.
- The a5ae3179 evidence.

No score may improve by changing a reference, a crop or a mask.

## 1. Causes

**Calibration.** I built a throwaway CPU model of the `535003d7` resolve that
uses the real jitter table. It reproduces the measured 8/16 chart contrast as a
fraction of FULL:

| Pattern | Model | Measured |
|---|---|---|
| 1-texel lines | 0.00 | 0.03 |
| 2-texel checker | 0.05 | 0.06 |
| 4-texel checker | 0.42 | 0.43 |

The same model, run with the proposed design, produces every expectation in
sections 3 and 7. They are expectations, not results. The model was not retained
or committed; the oracle controls in section 6 replace it.

| ID | a5ae3179 finding | Cause | Status |
|---|---|---|---|
| V1 | Chart texture below DRS (2-texel 3.3 vs 25.2, 4-texel 30.6 vs 60.8). Error vs FULL flat from frame 1 to 24. Contrast drains after the cut (8.3 → 2.6) | Each frame contributes a 2×2 bilinear interpolation of the jittered internal frame at the de-jittered position (`upscaling/resolve.wgsl` L11–26), blended at a fixed 0.9 (L65–68). The output averages internal-resolution interpolations; where a sample actually landed never decides where detail goes | Verified: source, measurement and model |
| V2 | Edges never converge (43% of edge pixels > 2 codes, 17% > 32) | Any footprint containing clear, partial-coverage, inadmissible or depth-incompatible taps returns current-only (L24, L27, L31–33). History taps with alpha < 1 are refused (L52) | Verified |
| V3 | Wires blink (0.8 px: 0, 62, 158 …). Detail is lost at once on 12→4 (6.2 → 0.1) | History is clamped every frame to the min/max of the current 3×3 internal taps (L57–64, L68), so anything one frame misses is erased | Verified |
| V4 | 1-texel lines gone in both DRS and UP (2.5 and 1.4 vs 43.9) | Material and MASK textures use implicit internal-resolution derivatives with no bias (`model_material.wgsl` L39–43, `motion/shader.wgsl` L26). There is no `textureSampleBias` anywhere in `incant_render` | Verified |
| V5 | 0.4 px wire never present at 8/16 | The 8-phase table (`temporal/jitter.rs` L3–12) has four offsets per axis, so sample rows are 0.25·r output px apart. At 8/16 they sit at pixel centre ±0.25 px, and a static feature under 0.5 px can miss every row | Verified. An information limit of the table, not a reconstruction defect |
| V6 | Detail returns slowly after 4/16 (0.1 → 3.2 → 8.0 over three 16/16 frames) | Fixed 10% per-frame rate (L67), regardless of the denser new samples | Verified |
| D1 | — | A negative LOD bias combined with V3's per-frame clamp aliases near-Nyquist texture into phase-coherent flicker. In the model, 1-texel lines at 8/16 flicker at full amplitude | Derived. This is why section 2.6 must not land without 2.4 |
| H1 | Mover-crop error above DRS (19.3 vs 15.5) | Bilinear history resampling under motion (L44–54) adds cumulative blur | Hypothesis; 0073 does not isolate it |
| H2 | Horizon-band shimmer | Relative-depth incompatibility at grazing ground angles makes interior pixels current-only (L31) | Hypothesis |
| H3 | Flat patch inside the moving shadow (contrast 7.2 vs FULL 27.8) | V1 and V3 acting on a static surface whose shading changes | Hypothesis, consistent with V1 and V3 |

## 2. Repair design

**Shape of the change.** It is one change set: the portable resolve, one new
history target, and LOD plumbing.

**Landing order.**
1. Sections 2.1–2.3.
2. Sections 2.4–2.5.
3. Section 2.6.

Section 2.6 must not land before 2.4 (see D1).

### 2.1 Reconstruct from true sample positions

For each output pixel centre P (in output px):

1. **Taps.** Compute `s = source_position(P)`. The taps are the 3×3 internal
   texels around `floor(s)`. Skip texels outside the source viewport; do not
   clamp them, which would duplicate edge texels.
2. **Sample position.** Each tap's scene position in output px is
   `x_t = output.xy + ((t + 0.5 - source.xy) / source.zw - jitter.xy) * output.zw`.
   This is the exact inverse of `source_position`.
3. **Weight.** `w_t = norm · exp(-|x_t - P|² / (2ρ²))`, where:
   - `r = output.z / source.z`;
   - `ρ = 0.35` px for `r ≤ 2`, and `0.35 + 0.325·(r - 2)` for `2 < r ≤ 4`;
   - `norm = r² / (2πρ²)`, so the mean mass per frame is one "average frame".
4. **Current estimate.**
   - `S = Σ w_t c_t`, with premultiplied RGBA and clear = 0.
   - `Wc = Σ w_t`.

**Why these choices hold.** Computed from the jitter table:
- **Uniform coverage.** Every output pixel at every scale from 4/16 to 16/16
  receives 8.00 ± 0.07 kernel mass per 8-phase cycle.
- **Negligible truncation.** The 3×3 window contains every sample within 1.5
  internal px. Any sample outside it weighs less than 10⁻⁴ of the kernel peak.

### 2.2 Confidence instead of a fixed blend

**New channel.** Each pixel gets a history confidence `W`, measured in average
frames.

**The cap.** `W_max = (h / (1 - h)) · min(r², 4)`, where `h` is the owner's
`history_weight`. At the review policy:

| Internal scale | `W_max` |
|---|---|
| 16/16 | 9 |
| 12/16 | 16 |
| 8/16 and below | 36 |

**What the cap means.**
- At 16/16, a frame of average evidence keeps the old 0.9 meaning.
- At lower scales, history holds the same number of native-density samples.
- Frames whose samples land near P weigh more; frames whose samples land far
  away weigh less.

**Why scale the cap.** A flat cap of 9 gives each 8/16 pixel only about two
centred hits per horizon, and the model's static chart then shimmers at 4–9 sRGB
codes. The scaled cap gives 1.0–2.5 codes, against 1.8–2.4 for the `535003d7`
model and 3.1 measured, with the same detail.

**Update.**
- `C = (H·W + S) / (W + Wc)`.
- `W' = min(W + Wc, W_max)`.

**Start.** At the first frame, a cut, or `has_previous = 0`, set `W = 0`. The
output is then the normalised kernel interpolation.

**Internal scale change.** Let `s` be the internal viewport width.
- More samples (`s_cur > s_prev`):
  `W ← min(W, W_max(r_cur)) · (s_prev / s_cur)²`.
- Fewer samples: keep `W`.

**Reactive term.** On the moving path (2.4) the existing reactive term lowers
`W`: `W ← W · (1 - strength · clamp(max|H - S/Wc| / threshold, 0, 1))`.

**Documentation.** The `TemporalConfig::history_weight` doc comment
(`temporal/config.rs` L6) and the spike must state this upscaling meaning. The
fixed-resolution path keeps its own meaning.

### 2.3 Edges and coverage

- **Accumulate everything.** Accumulate premultiplied RGBA from every tap,
  including clear and partially covered ones.
- **Remove:**
  - the current-only rule (L24, L27, L31–33);
  - the alpha < 1 history refusal (L52).
- **Keep:** the finite, nonnegative and alpha-zero cleanup (`clean`, `admitted`).
- **Downstream:** no change needed. The output shader already un-premultiplies
  and mixes with the backdrop by coverage (`output.wgsl` L22–27), so fractional
  silhouette coverage composites correctly.

### 2.4 History admission and rectification

**Definitions.**
- **Primary tap:** the tap with the largest `w_t`.
- **Motion source:**
  - the primary tap, if it is geometry;
  - otherwise the nearest-depth geometry tap in the window;
  - otherwise none.
- **Static pixel:** either of these holds:
  - the motion source's `P_prev` (the formula at L41–42) is within 1/256 px of P;
  - the window has no geometry and the host's camera-static flag is set
    (section 4).
- **History-static:** the flag the previous frame wrote to meta (2.5).

**Static and history-static pixels (persistence).**
- **Take history at P exactly:** no bilinear, no depth test, no box clip. This is
  what lets sub-sample coverage and super-resolved texture survive phases that
  miss them.
- **Shading change:** if the key (2.5) differs from the stored key by more than
  1/16:
  - clip H to the min/max of the 9 taps;
  - set `W ← min(W, 0.25)`.

  In the model, a reset to 0.25 leaves a 0.041 trailing excess behind a moving
  shadow; 0.5 leaves about 0.056.
- **Removal guard:** if no tap is depth-compatible with the stored history depth
  for more than 8 consecutive frames, drop the history. A feature that covers at
  least one sub-row goes at most 6 consecutive frames without a hit.
- **Depth:** keep the stored history depth; it fixes the world point the key
  uses.

**All other pixels (moving path).**
- **Fetch:** bilinear history at `P_prev`.
- **Admit taps** that are all of:
  - finite;
  - inside the output viewport;
  - depth-compatible with the motion source's expected previous depth (existing
    `compatible` and tolerances).

  Any coverage is allowed.
- **Renormalise.** If no tap is admitted, there is no history, as today.
- **Rectify:**
  - clip H (RGBA) to the min/max of the 9 current taps;
  - apply the reactive term.
- **Depth:** write the motion source's depth.

**Frame-global suspension (host flag).**
- **Triggers:**
  - the light list changes;
  - any light's kind, shadow slot, direction, radiance or range changes;
  - the environment intensity or rotation changes;
  - the motion owner reports an instance removal, a visibility change or a
    discontinuity.
- **Applies only** when the event does not already reset history.
- **Effect:** for that frame, every pixel takes the moving path, and
  `W ← min(W, 0.25)`.

### 2.5 Shading-change key and meta target

**Why one key suffices.**
- Directional lights are the only shadowed lights: `lighting/shade.wgsl` L36–41
  multiplies only the directional loop by `directional_shadow`.
- Cascades are jitter-stable: `shadows/cascade.rs` L115–116 uses the constant
  `jitter_guard`.

So with a static camera, a static point's visibility changes only when a shadow
caster moves.

**Key definition.**
- `key = Σ lum(radiance_l) · vis_l / Σ lum(radiance_l)` over shadowed
  directional lights; 1 if there are none.
- `vis_l` comes from the unchanged `directional_shadow`, evaluated at the stored
  world point: the unjittered ray through P at the stored history depth.
- A zero normal offset is acceptable, because both frames use the same method.

**Plumbing.** The resolve receives the frame's lighting bind group, which uses
the material group-3 layout.

**Meta contents.** A new history pair stores, per output pixel:
- `W`;
- the key (−1 = not yet valid; moving pixels write −1);
- the consecutive-miss count;
- the history-moving flag.

### 2.6 Mip selection

**The bias.** `b = −min(log2 r, 1)`:

| Internal scale | Bias |
|---|---|
| 16/16 | 0 |
| 12/16 | −0.415 |
| 8/16 and below | −1 |

**Why the −1 cap.** −1 is where the model stays stable. At 4/16, a bias of −1
doubles the detail of 0, with activity ≤ 0.054. Biases of −1.5 and −2 flicker on
content that 0.5 samples per pixel per cycle cannot resolve.

**Where it applies.** Use `textureSampleBias` for:
- the five material samples (`model_material.wgsl` L39–43);
- the MASK sample (`motion/shader.wgsl` L26).

Apply it in the jittered opaque/MASK passes and in the unjittered overlay-depth
re-raster, so MASK coverage matches everywhere history and overlays meet.

**Bias 0 for:**
- BLEND (the `transparent_camera` frame, `model_draw.rs` L63–74);
- sprites and text (already explicit LOD 0);
- FULL;
- DRS;
- fixed temporal;
- MetalFX;
- pixel cameras.

**References stay byte-identical.** FULL and DRS output must not change. If a
backend's `textureSampleBias(…, 0)` differs from `textureSample`, give the biased
path its own entry point instead.

### 2.7 Unchanged

- The overlay composite and the current-only overlays.
- Physical UI.
- `common.wgsl` and the shared 96-byte `Frame`. MetalFX's `prepare.wgsl` and
  `composite.wgsl` include both.
- The jitter table.
- The cut, epoch and owner contract.
- The review policy values.

## 3. Behaviour by scale

All values below are model expectations with the full design.

| Internal | r | Samples per px per cycle | Sub-row spacing (px) | ρ (px) | Bias | `W_max` |
|---|---|---|---|---|---|---|
| 16/16 | 1 | 8 | 0.25 | 0.35 | 0 | 9 |
| 12/16 | 1.33 | 4.5 | 0.33 | 0.35 | −0.415 | 16 |
| 8/16 | 2 | 2, each 0.354 px from the pixel centre | 0.5 | 0.35 | −1 | 36 |
| 6/16–5/16 | 2.67–3.2 | 1.1–0.8 | 0.67–0.8 | 0.57–0.74 | −1 | 36 |
| 4/16 | 4 | 0.5, a checkerboard of pixel centres | 1.0 | 1.0 | −1 | 36 |

Expected appearance at each scale:

- **16/16.** A stable, antialiased image after one cycle.
  - Chart vs FULL: lines 0.45, 2-texel 0.52, 4-texel 0.76.
  - Static activity 3–4 sRGB codes.
  - All three wires are present.
- **12/16.** Between 16/16 and 8/16.
- **8/16.**
  - Chart vs FULL: lines 0.38, 2-texel 0.48, 4-texel 0.74.
  - Static activity 1.0–2.5 codes.
  - Wires: the 1.6 px wire is steady. The 0.8 px wire is present in every phase.
    Its model presence is 0.33–0.37 per phase, against a steady 0.75 for the
    1.6 px wire. The 0.4 px wire may be absent (V5).
- **6/16–5/16.** Softer, and stable.
- **4/16.**
  - Soft, with no parity pattern: model error is equal on both parities.
  - The 4-texel checker reaches 0.34 of FULL, with 1.3–2.5 codes of activity.
  - On a downward change, detail fades over about one cycle instead of in one
    frame.

## 4. Data paths and budget

**Resolve and shader assembly.**
- `upscaling/resolve.wgsl`: rewrite per 2.1–2.5. New portable-only helpers stay
  out of `common.wgsl`.
- `temporal_shader.rs` L27–32: the `Kind::Upscale` source also includes the
  lighting and shadow declarations that `directional_shadow` needs (and `unit`).
- Cooked-temporal digests regenerate. MetalFX programs do not change.

**`upscaling/system.rs`.**
- **Bindings:** the resolve layout gains:
  - binding 6: previous meta, as a non-filterable float;
  - binding 7: a portable-only resolve uniform.
- **Portable-only uniform fields:**
  - unjittered inverse view-projection;
  - camera-static flag;
  - frame-global flag;
  - scale factor;
  - ρ;
  - `norm`;
  - `W_max`.
- **Pipeline:**
  - the pipeline gains a third colour target (meta);
  - the lighting group sits at group 3, with groups 1–2 empty.
- **Encoding:** `Draw::resolve` takes the lighting group. `graph.rs` L287–289
  passes it from `frame.models`.

**Targets and accounting.**
- `upscaling/targets.rs`: `Output` gains `meta: [TextureView; 2]`
  (`Rgba16Float`) for the portable path, and `bytes()` goes from 32 to 48 per
  pixel.
- `plan.rs` L5–7 and `tests.rs` L68:
  - `MAX_OUTPUT_BYTES` becomes 48·px;
  - the logical ceiling goes from 640 MiB + 16 KiB to 768 MiB + 16 KiB.
- MetalFX shares `Output` (`owner.rs` L123–134). Either allocate meta only for
  the portable resolve, or charge it to MetalFX explicitly.

**Host flags.** These come from the owners (`upscaling/owner.rs`,
`motion/retained.rs`):
- **Camera static:** the previous and current unjittered view and projection are
  equal.
- **Frame-global:** per section 2.4.
- **Scale factor:** `(s_prev / s_cur)²`, capped at 1.

**LOD plumbing.**
- **Material frame:** `model_draw.rs` L15–44 and `model_material.wgsl` L2. The
  `Frame` gains `lod: vec4f`, going from 96 to 112 bytes; update the size and
  offset asserts. The `transparent_camera` copy sets it to 0.
- **Motion frame:** `motion/retained.rs` L29–33 and `motion/shader.wgsl` L2 gain
  the same field. `overlay_frame` (L373) copies it.
- **Cooked materials:** digests regenerate.

**Per-output-pixel budget.**

| Item | 535003d7 | Repair |
|---|---|---|
| Current colour loads | 13 (4 bilinear + 9 clamp) | 9 |
| Current depth loads | 17 (4 + 4 + 9) | 9 |
| Motion loads | 1 | 1–2 |
| History loads | 8 (colour 4, depth 4) | Static path: 3 (colour, depth and meta at P). Moving path: 12 (bilinear colour, depth and meta) |
| Shadow comparisons | 0 | Static pixels only: `directional_shadow` per shadowed directional light, 9 per cascade, at most 2 cascades |
| Kernel | Bilinear weights | 9 exponentials |
| Targets written | Colour, depth | Colour, depth, meta |
| Output history payload | 32 B/px | 48 B/px |
| Logical transition ceiling | 640 MiB + 16 KiB | 768 MiB + 16 KiB |

No timing, residency or phone claim is made. Cost needs the plan's device
measurements.

## 5. Failure risks

| Risk | Path | Handling | Control |
|---|---|---|---|
| A removed, hidden or teleported static object ghosts under persistence | Static | Host frame-global suspension; otherwise the miss count drops history after 8 frames | C6 |
| Halo behind moving objects from dilated motion | Moving | Motion and admission come from the primary tap; dilation only when the primary is clear | C7 |
| A moving shadow lags on a static receiver | Static | Visibility key and confidence reset | C8, G4 |
| Animated lights or environment | All | Frame-global suspension. Continuous animation runs at low confidence, which looks like current-only aliasing | C6 |
| Unshadowed moving punctual light | Static | Handled only by frame-global suspension, since any light change triggers it | C6 |
| Animated materials | — | None exist at `535003d7`. Any future material animation must mark its pixels reactive (for example `motion.w = 2`) before persistence applies | — |
| Features thinner than 0.25·r | All | Information limit (V5); acceptance does not require them | C5 |
| Near-Nyquist texture while moving | Moving | The per-frame clip shows FULL-like aliasing during motion, with no smear | Motion review |
| HDR highlights dominating linear accumulation | All | Not addressed. Luminance-weighted accumulation is the follow-up if the review shows it | — |
| Fractional viewport edges | All | Outside taps are skipped, not clamped; outside history is refused, as now | C1, G2, border band |
| Cut and reset determinism | All | `W = 0` path; bit-identical restart | C10 |

## 6. Numerical controls for Astra

**Where they go.** Extend `upscaling/oracle.rs`, `upscaling/tests.rs`,
`upscaling/device_tests.rs` and `upscaling_retained_tests.rs`. No new harness.

**Thresholds.** Astra pins them from its f64 oracle. My model values are for
orientation only. A result below about 80% of them indicates a defect, not a
tuning opportunity.

**CPU controls (f64 oracle).**
- **C1 Mapping.** Tap output positions invert `source()` to within 1e-9 for:
  - n = 4..16;
  - every phase;
  - rects `[0,0,65,33]` and `[3.25,5.5,65,33]`.
- **C2 Coverage.** For n = 4..16:
  - per-pixel cycle mass is in [7.9, 8.1];
  - the nearest sample per cycle is at most 0.40 px for n ≥ 8 (n = 8: 0.354;
    n = 9: 0.393);
  - for n < 8 it is at most 1.0 px (n = 4: exactly 1.0 on the unsampled
    parity).
- **C3 Static convergence.** Run lines, 2-texel and 4-texel patterns for six
  cycles.
  - Contrast fraction near the model: 8/16 gives 0.38, 0.48, 0.74; 16/16 gives
    0.45, 0.52, 0.76.
  - The `535003d7` oracle on the same input gives 0.00, 0.05, 0.42; use it as
    the regression reference.
  - Last-cycle activity at 8/16 is no higher than the `535003d7` oracle's on the
    same input. The model gives 1.0–2.5 sRGB codes against 1.8–2.4, and 4–9 with
    a flat `W_max` of 9.
- **C4 No phase flicker.** Grid-aligned 1-texel lines at 8/16 with `b = −1`:
  activity ≤ 0.05. A per-frame clamp gives 0.5.
- **C5 Thin coverage.** Static lines over clear:
  - At 8/16, 1.6 px and 0.8 px lines are present in all 8 phases. Their spread is
    at most 0.05 and 0.2 of the mean respectively (model: under 0.01 and 0.11).
  - A 0.4 px line placed between the 8/16 sub-rows receives zero samples (this
    documents V5).
  - At 16/16 the 0.4 px line is present in every phase.
- **C6 Removal and global changes.**
  - A static square removed with the host flag set disappears in that frame.
  - Removed without the flag, it disappears within 9 frames.
  - A light change suspends persistence for one frame.
- **C7 Disocclusion.** A square moving 3 px/frame over a static textured
  background. Beyond 2 frames of motion behind the trailing edge, the mean
  absolute difference per column from a never-moved converged reference is at
  most 0.06.
- **C8 Moving shadow.** A static textured plane with a visibility band moving
  3 px/frame.
  - Trailing-region error ≤ static baseline + 0.05 (model: 0.041).
  - A static scene produces zero key triggers over 3 cycles.
  - Triggers occur only where the band changed.
- **C9 Scale.**
  - 4/16 → 16/16 reaches ≥ 75% of the 16/16 steady contrast by the third 16/16
    frame and ≥ 90% by the fourth. The model gives 0.77 and 0.98; without the
    `W` rescale it gives 0.38 and 0.51.
  - On a static scene, 12/16 → 4/16 keeps contrast in the first 4/16 frame at
    ≥ 90% of the last 12/16 frame (model: 96%). It then settles toward the 4/16
    level over about one cycle, never in one step.
- **C10 Reset.** A cut and `has_previous = 0` both reproduce the first-frame
  output bit for bit, with `W = 0`.
- **C11 Policy.**
  - `W_max = (h / (1 - h)) · min(r², 4)`.
  - On a scale increase, `W` is clamped to the new cap before the density
    factor.
  - Reactive attenuation as in 2.2.
  - `TemporalConfig::validate` unchanged.

**GPU raw controls.** These use the existing
`actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay`
pattern.
- **G1.** Multi-frame raw cases (one-hot texel, gradient, coverage edge, clear)
  through 8 phases, a scale change and a cut. Every output pixel, not only (3,3),
  matches the oracle within the existing half-float tolerance.
- **G2.** Keep the refusal semantics of the 19 modes, with expectations
  re-derived from the oracle:
  - NaN, inf or negative values;
  - alpha-zero residuals;
  - outside-viewport taps;
  - invalid motion;
  - overlays never entering history;
  - moving-path disocclusion.
- **G3.** Meta encode/decode round trip for W, key, miss count and flag.
- **G4.** Key with the real lighting and shadow group:
  - a static caster gives identical keys over 8 frames;
  - a moved caster changes keys only inside its shadow footprint.

**Retained controls (`upscaling_retained_tests.rs`).**
- **R1 Mip selection.** A mip-tinted texture on a plane at about one texel per
  output pixel.
  - UP 8/16 shows FULL's mip.
  - UP 4/16 shows FULL's mip + 1 (the cap).
  - FULL, DRS and fixed temporal are unchanged.
- **R2 References.** FULL and DRS outputs are byte-identical to the pre-repair
  build.
- **R3 MASK.** Cutout coverage is identical between the motion/depth and colour
  passes under bias.

## 7. Appearance acceptance (Claude, next review)

**Fixed inputs.** The unchanged 0073 fixture, cases, crops, policy and frame
budget.

**Precondition.** FULL and DRS control RGBA hashes equal those from a5ae3179:

| Case | FULL | DRS |
|---|---|---|
| Stationary | `full` `4c69efe2…` | `drs50` `6611c41f…` |
| Motion, frame 14 | `full-14` `ecd60300…` | `drs50-14` `650a2e0d…` |
| Motion, frame 20 | `full-20` `47c3bee7…` | `drs50-20` `473530b0…` |
| Transitions | `full` `49d0f741…` | `drs50` `a8471f8d…` |

If the section 8 correction rides along, compare stationary hashes only outside
the existing `in_ui` label rectangle.

**How I judge.** I judge pixels; numbers stay observations. Against a5ae3179, the
model expects the following at 8/16.

**Chart** (smooth rather than blocky):

| Pattern | Model expectation | DRS | a5ae3179 UP |
|---|---|---|---|
| 1-texel lines | ≈ 17 | 2.5 | 1.4 |
| 2-texel checker | ≈ 26 | 25.2 | 3.3 |
| 4-texel checker | ≈ 52 | 60.8 | 30.6 |

These are the model's contrast fractions times FULL's measured luminance
standard deviation in the same regions (see `visual-result.md`).

**Elsewhere:**
- **Edges:** silhouettes accumulate and antialias, with edge activity far below
  a5ae3179's 19.4 mean and 17% above 32.
- **Wires:**
  - the 1.6 px wire is steady;
  - the 0.8 px wire is present in every phase;
  - the 0.4 px wire may stay absent (V5).
- **Static interiors:** as quiet as before.
  - Interior activity is about 0.4.
  - Chart-crop activity is not materially above 3.1.
- **Motion:**
  - no trail or halo behind the cube;
  - its shadow carries the chart texture;
  - the pan may alias like FULL, but must not smear.
- **Transitions:**
  - no instant detail loss on 12→4;
  - most detail back by the third 16/16 frame on 4→16 (model: about 80% of
    steady);
  - the cut stays exact;
  - border band and chrome unchanged.
- **Overlays and UI:** unchanged.

**What fails the review:**
- the chart's 2-texel checker visibly weaker than DRS;
- the rings or the 4-texel checker not clearly better than a5ae3179;
- any phase where the 0.8 px wire is at zero;
- new blinking or shimmer on static content;
- visible trails, halos or shadow lag;
- a parity pattern at 4/16;
- any change to FULL, DRS, overlays or UI.

## 8. Separate fixture correction: clipped UI label (not applied)

**The problem.** The label widget clips its own text.
- **Clip height:** `UI_LABEL = [252, 488, 600, 520]`
  (`visual/review/upscaling_review.rs` L91), with the shared 8 px padding
  (L1164), gives a 16 px clip.
- **Text height:** 18 px text gets a 23 px line (L1176 sets the line height to
  `round(1.25 × size)`; the call is L1202).

**The correction.** Change one literal, in a new visual tree:

```diff
-        widget(2, UiRole::Label, UI_LABEL, "physical UI 960x540", 18.),
+        widget(2, UiRole::Label, UI_LABEL, "physical UI 960x540", 12.),
```

At 12 px the line is 15 px, and the NotoSans ink stays inside the 16 px clip.

**Unchanged by it:**
- the button's bounds, text, size and padding;
- `UI_INTERIOR`;
- `ui_button_interior_identical_on_all_ten_ui_frames`;
- `UI_LABEL`, and therefore the `in_ui` exclusion and every statistics mask;
- crops and budgets.

**Effect.** The label's glyph pixels change equally in all three stationary
variants. a5ae3179, its tree `7e6d4b08` and its evidence stay as they are.

**Sequencing.** Root picks one:
- apply it in the visual tree for the repaired-source review, with the stationary
  FULL/DRS hash check made outside the `in_ui` label rectangle; or
- apply it after that review, to keep a fully byte-identical stationary control.

## 9. Not in this repair

- **Jitter-table changes.** These would lift V5, but they change the 0072 path.
- **Catmull-Rom history (H1).** Only if the next review shows motion blur from
  resampling.
- **Overlay resolution or composite filter.** Overlays stay current-only and
  mildly soft.
- **Luminance-weighted accumulation and sharpening.**
- **MetalFX, cooked/native, phones and performance.**
