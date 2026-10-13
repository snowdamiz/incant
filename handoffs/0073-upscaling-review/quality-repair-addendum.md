# 0073 quality repair addendum: static-history disocclusion

**Status: design correction, CPU only.** No renderer source, GPU, Cargo, target
or capture was used. `quality-repair-brief.md` (e1b26907) and all earlier
evidence are unchanged. This addendum replaces the parts listed in section 9.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-disocclusion-design-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs:**
- root's design review `review.json` `9649659c…`;
- its proof `static_disocclusion.py` `b8677710…` and `static_disocclusion.json`
  `27df57b3…`;
- the brief (root's uncommitted revision at session start).

**Evidence.** Every number below comes from the model in
[`repair-model/model.py`](repair-model/model.py). Its output is in
[`repair-model/results.json`](repair-model/results.json), with 50 cases, each
with an expected outcome. Retained failures are expected to fail. See section 10
for how to reproduce it.

## 1. The counterexample holds

Root's case: a white square, stationary for several frames, starts ordinary
continuous motion over a static black floor or over clear. Camera, lights and the
instance list are unchanged.

At a pixel the square vacates, everything the draft looked at says "keep":
- the motion source is the static floor;
- the stored history-static flag was written while the square was still static;
- the shading key is 1.

The draft therefore takes the persistence path, with no depth test and no clip.
The stale white then decays only through accumulation, until the miss guard drops
it after more than 8 frames.

The model reproduces this (largest residual white at vacated pixels at least 2 px
behind the trailing edge):

| Case | After 1 frame | After 3 | After 8 |
|---|---|---|---|
| F1 root's recurrence (cap 9) | 0.9 | 0.729 | 0.430 |
| F2 draft, over black, 16/16 | 0.841 | 0.729 | 0.431 |
| F2 draft, over black, 8/16 (cap 36) | 0.962 | 0.912 | 0.631 |
| F2 draft, over clear, 16/16 | 0.841 | 0.728 | 0.431 |
| C12 corrected, all four of these | ≤ 0.0002 | ≤ 0.0002 | ≤ 0.0002 |

The draft over clear at 8/16 happens not to ghost. Its dilated motion source
picks up the moved square inside the ±3 px window, so the pixel takes the moving
path. That is luck, not a rule.

**Root cause.** The draft asked whether the pixel was static when its history was
written. The right question is whether every surface that contributed to the
history is still where it was. A previously static occluder that starts moving
changes visibility without changing any signal the draft read.

**Two further draft defects found by the model:**
- **Frozen depth.** The static path kept the motion source's depth from the first
  frame. An edge pixel froze the floor's depth, and later admitted the square's
  stale coverage on the moving path.
- **Clip box too wide.** At 8/16 the box over all 9 taps spans ±3 output px, and
  it still contains the square's white 3 px later. On a black-and-white checker
  any colour fits inside the box, so clipping cannot remove a ghost there (C7,
  draft: 0.362).

## 2. Corrected admission (replaces the static path in brief 2.4)

The principle: a jitter phase that misses a static sub-pixel feature changes no
owner, while a genuine visibility change at a static pixel always does. It is
detected through explicit owner metadata, written when samples are accumulated,
never inferred from the history value.

**The three ways visibility can change at a static pixel:**
- **An instance moved or deformed.** Its owner is marked changed this frame.
- **The lighting changed.** The shading key or the frame-global flag catches it.
- **The topology changed.** That is a scene replacement. A retained scene fixes
  its topology (`retained.rs` L1–2), and `MotionError::Topology` refuses the
  frame, so history resets as today.

**Near taps.** These are the window taps within `R = max(r, 4ρ)` output px of P:
1.4 px at 16/16, 2 px at 8/16 and 4 px at 4/16.
- **Contributors:** the owners of near geometry taps with coverage greater than 0.
- **Front contributor:** the nearest-depth near geometry tap, with its owner and
  depth.
- **Rectification box:** the min/max over near taps, clear included.
- **Fallback:** if no tap is near, the primary tap alone.
- **What is left out:** taps beyond `R` carry at most e⁻⁸ ≈ 3.4·10⁻⁴ of the peak
  weight. That is the bound on contributions that owners do not track.

**Stored per output pixel:**
- up to two contributing owners, or an overflow mark for three or more;
- the front owner;
- the front depth, in the existing R32Float history depth;
- `W`;
- the shading key.

**Per pixel, after the reconstruction of brief 2.1–2.3:**
1. **Primary and motion source.** The primary is the tap with the largest
   weight. The motion source is the primary if it is geometry; otherwise the
   front contributor; otherwise none. P is static if the motion source moved
   less than 1/256 output px; with no motion source, P is static if the camera
   is static.
2. **Static pixel with stored history.**
   - **Persist (keep unclipped)** when all of these hold:
     - the stored owners have not overflowed;
     - none of them is in this frame's changed set;
     - the frame-global flag is clear.

     This is no depth test and no clip, so sub-sample coverage and super-resolved
     texture survive missed phases. The shading key still applies, as described
     below. Then `owners ← stored ∪ contributors` (overflow above two), and the
     front contributor becomes the nearer of the stored and current fronts.
   - **Drop** when the stored front owner changed and no near tap lies at its
     depth. The front contributor has left, so its history is stale.
   - **Otherwise, clip in place**: clip the history at P to the near box. If the
     frame-global flag is set, also apply `W ← min(W, 0.125)`. Then
     `owners ← contributors` and the front contributor is the current one.
3. **Moving pixel.**
   - **Fetch:** reproject along the primary's motion, using bilinear history at
     `P_prev`.
   - **Admit** a history tap when its front depth matches the expected previous
     depth of some near surface moving with the primary (screen motion equal
     within 1/256 px).
   - **Rectify:** renormalise, clip to the near box, and apply the reactive term.
   - **Store:** `owners ← contributors`, and the front contributor is the current
     one.

   Using the primary's own motion avoids the leading-edge halo that a dilated
   motion would cause. Admitting same-motion near surfaces keeps depth edges
   under camera motion.
4. **No motion source while the camera moves:** no history, as today.
5. **Shading key, on the persist path only.** The definition is unchanged from
   brief 2.5. On a change of more than 1/16, clip to the near box and set
   `W ← min(W, 0.125)`. This was 0.25 in the brief; see section 5.
6. **Frame-global flag.** Set it when the light list changes, or when any light's
   kind, shadow slot, direction, radiance or range changes, or when the
   environment intensity or rotation changes. Instance motion no longer needs it.

The draft's history-moving flag and its miss guard are removed. Owners subsume
both.

## 3. What the corrected rules do in the model

| Case | Result |
|---|---|
| C12 root's case, over black and clear, 8/16 and 16/16 | Residual ≤ 0.0002 after 1, 3 and 8 frames |
| C17 partial-coverage edge band, over black and clear | 0.000 (draft 0.936 and 0.060) |
| C14 start, move, stop over texture | Worst column 0.019 against a never-moved square |
| C15 nearer bar sweeps over a floor and a static 0.8 px wire | Wire present in every phase (0.21–0.42); floor worst column 0.025 |
| C16 camera pans 16 px, then holds | Floor worst column 0.015; activity 3.0 codes |
| C19 three static owners meet; a bar passes | Largest residual 0.014; overflow pixels clip in place |
| C5 static wires over clear and over a floor | 1.6 px ≥ 0.749; 0.8 px 0.33–0.43 in every phase (same as draft) |
| C3 static chart at 8/16 | Contrast 0.38 / 0.48 / 0.74 of FULL; activity 1.0 / 2.5 / 2.3 codes (535003d7: 0.0 / 1.7 / 2.1) |
| C8 moving shadow, exact key | Signed lag 0.019; no key: 0.144 (F5) |
| C6 light change with the frame-global flag | Lag 0.006; without the flag: 0.231 (F9) |
| C9 4/16 → 16/16 | 0.68 then 0.87 of steady, by the third and fourth frames; without the rescale 0.23 and 0.37 (F6) |
| C9 12/16 → 4/16 | First 4/16 frame keeps 0.96 |
| C10 cut | Bit-identical restart |
| C2 interior cycle mass, n = 4..16 | 7.927–8.073, inside [7.9, 8.1] |
| C21 fractional viewport | Interior mass 7.998; edge mass down to 0.0018 (no bound claimed) |

C0 checks the model itself: the 535003d7 resolve in the model gives 0.00, 0.05
and 0.42 of FULL on the chart, against 0.03, 0.06 and 0.43 measured in a5ae3179.

## 4. C7: a correction to my control, not a relaxation

C7 as written in the brief asked that, beyond two frames of motion behind the
trailing edge, the mean |moving − never moved| per column be at most 0.06. The
corrected design measures 0.092 (worst column), against 0.362 for the draft. The
decomposition shows the 0.092 is not a ghost:

| Case | Value |
|---|---|
| C7a signed bias toward the white occluder, per column | 0.000 (draft 0.362) |
| C7b against a never-moved floor reset at the same pixels and frames | 0.000 at every pixel (255 drops) |
| C7c a current-frame-only floor against the converged floor | 0.179 |
| C7d residue against frames since vacated | 0.028 at 5, 0.026 at 6, 0.009 at 7, 0.017 at 8 |
| F7 a wider kernel for fresh history (rejected) | Worst column 0.069, mean worse (0.054 vs 0.047) |

The vacated pixels hold exactly what a floor reset at that moment would hold. C7
as written therefore measured how fast texture converges after a disocclusion,
not ghosting. Two frames after vacating, a design that honestly drops stale
history still differs from a converged floor by this kind of amount on a
black-and-white checker: a single current frame differs by 0.179, and the
wider-kernel alternative still leaves 0.069.

**Replacement controls.** All are stricter on ghosting than C7 was:
- **C7a** signed bias ≤ 0.01;
- **C7b** ≤ 0.02 per pixel against the matched-reset floor;
- **C7d** residue ≤ 0.06 within 6 frames of vacating;
- **C12** root's ≤ 0.06 at 1, 3 and 8 frames (measured: 0.000);
- **C17** ≤ 0.02 in the partial-coverage band.

C7 as written stays in the model as an expected failure for both rule sets, so
the change is visible.

## 5. Other corrections to e1b26907 numbers

- **C9.** The brief's 0.77/0.98 were measured against an under-converged 16/16
  reference. Against that same kind of reference (the run's own frame 39), this
  model gives 0.74/0.94. Against the converged steady state it gives 0.68/0.87,
  and without the rescale 0.23/0.37. The control is now "≥ 0.65 by the third
  16/16 frame and ≥ 0.85 by the fourth".
- **Reset value (R1).** A key change or the frame-global flag now resets to
  `W ≤ 0.125`, not 0.25. On signed lag, 0.25 gives 0.031 and 0.125 gives 0.019,
  for 0.064 vs 0.060 absolute difference. A full drop (0) removes the lag but
  raises the absolute difference to 0.093.
- **C2.** The [7.9, 8.1] bound holds only where all 9 taps are valid in every
  phase, which matches root's independent 7.927..8.073. No bound is claimed at
  clipped viewport edges.
- **Shading key cost.** The bound is now stated (section 7), and gating is added:
  - **Bound:** up to 4 shadowed lights × 2 cascades × 9 comparisons = 72
    comparisons per persisting pixel, plus 9 exponentials per pixel for the
    kernel.
  - **Gating:** the host supplies, per shadowed light, up to 8 light-space
    rectangles. Each covers the previous and current bounds of a changed shadow
    caster, plus the PCF radius and normal offset. Overflow merges rectangles.
    Outside every rectangle the key is reused, because cascade xy does not depend
    on casters (`shadows/cascade.rs` L128–137). Caster bounds change only the
    cascades' depth range, which remaps stored and compared depths alike.
  - **K2:** the gated key equals the ungated key bit for bit, with 4,608 instead
    of 25,600 evaluations in that scene. The worst case stays at 72 comparisons
    when every pixel lies inside a rectangle.
  - **Cheaper alternative rejected (K1):** one 2×2 gather per cascade
    (worst case 32 comparisons) leaves a lag of 0.040 against 0.019, so it is not
    the same quality.

## 6. Coordinate rounding and fractional viewport edges

- **Output pixels.** Pixel `p` has its centre at `p + 0.5`. It is processed only
  if that centre is in `[ox, ox + ow) × [oy, oy + oh)`. Outside pixels write zero
  colour, depth and meta (no owners, `W = 0`).
- **Source viewport.** It is the output rect × n/16 in f32, as today. Texel `t`
  is valid when `lo ≤ t ≤ hi`, with `lo = ceil(origin − 0.5)` and
  `hi = ceil(origin + extent − 0.5) − 1` (the existing `lo`/`hi`).
- **Window.** `s = source_position(P)` and the window is `floor(s) + {−1,0,1}²`.
  Invalid texels are skipped and never clamped.
- **Tap position.**
  `x_t = output.xy + ((t + 0.5 − source.xy) / source.zw − jitter.xy) · output.zw`,
  unrounded.
- **Ties.** Primary and nearest-depth ties take the first tap in row-major window
  order (y then x, each −1 to 1).
- **Comparisons.** The near test is `d² ≤ R²`, inclusive. The static test is
  strictly less than 1/256 px.
- **No valid tap.** The output is the persisted or clipped history if there is
  one, else zero.
- **History fetch.** On the moving path, `P_prev` must lie inside the output rect
  (half-open); the bilinear base is `floor(P_prev − 0.5)`, and taps outside
  `lo..hi` are skipped and renormalised. Static pixels read their own index.
- **Edge pixels.** Their cycle mass falls below 8 (to 0.0018 at a fractional
  corner in C21). Normalisation keeps the estimate a weighted mean of the valid
  taps, and `W` grows more slowly there.

## 7. Data path and exact accounting (portable path only)

**Owner ids.**
- Each motion row's owner id is `1 + its packed row index`. The order is stable
  while the retained topology is fixed.
- It is stored as f32 in `Previous.valid[1]` (`motion/retained.rs` L18–21).
  Lanes 1–3 are written as zero today. The value is exact because rows are at
  most `2 × MAX_INSTANCES` = 200,000 < 2²⁴.
- The row stays 80 bytes.

**Changed set.**
- The host sets bit `i` when row `i`'s uploaded previous and current world
  matrices differ, and conservatively for animated batches every frame.
- It is a storage buffer of `4 · ceil(rows / 32)` bytes, at most 25,000. Row `i`
  is bit `i % 32` (least significant first) of word `i / 32`.

**Owner target.**
- The jittered motion pass gets, only in the portable upscaling pipeline variant,
  a second colour target: `R32Uint` at internal size, holding the owner id.
- It is cleared to 0, and MASK discards leave it 0.
- The overlay-depth pass is unchanged.

**History meta.**
- A pair of `Rgba32Uint` textures at output size:
  - x: front owner;
  - y: second owner (0 = none, `0xFFFFFFFF` = overflow);
  - z: `pack2x16float(W, key)`;
  - w: reserved (0).
- The existing R32Float history depth now stores the front depth.

**Portable resolve uniform.**
- Unjittered inverse view-projection (64 B).
- Flags: camera static, frame-global, scale factor, `R` (16 B).
- Kernel: ρ, norm, `W_max`, reset (16 B).
- Up to 4 × 8 gate rectangles of 32 B.
- Total: 1,120 B.

**Isolation.**
- MetalFX and fixed temporal allocate none of this. They keep their pipelines and
  accounting, and MetalFX's `Output` stays 32 B/px.
- `common.wgsl` and the shared 96-byte `Frame` are unchanged.

**Worst case per output pixel (no timing claim):**

| Item | 535003d7 | Corrected, persisting pixel | Corrected, moving pixel |
|---|---|---|---|
| Current colour, depth, owner loads | 13 + 17 + 0 | 9 + 9 + 9 | 9 + 9 + 9 |
| Motion loads | 1 | 1 | Up to 9 (same-motion test) |
| History loads | 8 | 3 (colour, depth, meta at P) | 12 (bilinear colour, depth, meta) |
| Changed-set reads | 0 | 2 | 0 |
| Shadow comparisons | 0 | 0 outside the gate; up to 72 inside | 0 |
| Gate tests | 0 | Up to 32 point-in-rectangle | 0 |
| Kernel exponentials | 0 | 9 | 9 |
| Bytes written | 12 | 28 | 28 |

**Exact logical ceiling for the portable path** (old plus candidate sets):

| Term | Bytes |
|---|---|
| Output: 2 × (16 colour + 8 depth + 8 composite + 32 meta) B/px | |
| Internal: 2 × (8 overlay + 24 motion + 4 owner) B/px | |
| Pixels: 2 × 100 B/px × `MAX_MOTION_PIXELS` (4,194,304) | 838,860,800 |
| Motion instance payloads: 2 × 67,108,864 (unchanged rows) | 134,217,728 |
| Changed sets: 2 × 25,000 | 50,000 |
| Portable resolve uniforms: 2 × 1,120 | 2,240 |
| Existing fixed metadata allowance | 16,384 |
| **Total** | **973,147,152** |

**Compared with today.** The portable ceiling is 671,105,024 B today
(640 MiB + 16 KiB); MetalFX's stays there.

**How to encode it.** Astra should compute the constant from these named terms,
and assert both totals in `upscaling/tests.rs`, not rename the old constant.

## 8. Tests for Astra

This list replaces brief section 6 where it overlaps. Each item names the model
case it mirrors. CPU cases go in `upscaling/oracle.rs` and `upscaling/tests.rs`.
GPU raw cases follow the existing `actual_output_reconstruction…` pattern in
`upscaling/device_tests.rs`. Real-path cases go in `upscaling_retained_tests.rs`.

**The cases root required:**

| Root's case | Controls |
|---|---|
| Moving-to-clear | C12 over clear, C17 over clear |
| Moving-away foreground | C12, C7a, C7b, C7d, C14 |
| New occluders | C15 (sweeping bar over a floor and a static wire). Spawns change topology, so they reset history (one retained test) |
| Camera motion | C16 |
| Partial-coverage history | C17, plus C19 for owner overflow |

**Other controls:**
- **Unchanged or renumbered:**
  - C1 mapping;
  - C2 interior mass in [7.9, 8.1], with edges reported but not bounded;
  - C3 static chart;
  - C5 static wires over clear and over a floor;
  - V5 (formerly part of C5): the 0.4 px information limit;
  - C10 cut.
- **New:**
  - C6 light change with and without the frame-global flag;
  - K2 gated key equals the ungated key.
- **Revised:**
  - C8 signed lag ≤ 0.03 at a 0.125 reset (R1 reports the alternatives);
  - C9 with the corrected figures.
- **Retained failures, which must keep failing:**
  - F1–F2 draft persistence;
  - C7 as written;
  - F3 flat `W` cap of 9 (9.1 codes of shimmer);
  - F4 bias with a per-frame clamp (activity 0.5);
  - F5 no shading key;
  - F6 no rescale;
  - F7 wider fresh kernel;
  - F8 colour-ratio test (5th and 95th percentiles 0.061 and 2.87, overlapping a
    0.2 shadow);
  - F9 no frame-global flag;
  - K1 gather key.
- **GPU additions:**
  - meta encode/decode round trip, including overflow;
  - the owner target written only by the portable variant;
  - changed-set bit order;
  - the gate applied in light space per cascade.

## 9. What changes in the e1b26907 brief

- **Section 2.4, static path:** replaced by section 2 here, together with the
  history-moving flag, the miss guard, the frozen-depth rule and the 9-tap box.
  Moving-path admission now uses same-motion near surfaces.
- **Section 2.4, frame-global triggers:** now lights and environment only.
- **Section 2.5:** meta layout per section 7. The key is unchanged, plus gating
  and the stated bound. The reset is 0.125.
- **Section 4:** data paths and accounting per section 7.
- **Sections 5–6:** risks and controls per sections 2, 4 and 8.
- **Unchanged:**
  - 2.1–2.3 reconstruction;
  - the `W` cap and rescale;
  - the 2.6 bias rule and landing order (2.6 still waits for this admission);
  - sections 3 and 7, except the C9 figures;
  - section 8, the label fix.

**Decisions taken, as root gave them:**
- one combined review after the full algorithm passes correctness;
- the label fixture stays unchanged for that comparison and is fixed afterwards;
- extra meta and the owner target exist only on the portable path;
- the ceiling uses the exact accounting above;
- root implements the numerical mechanics and budgets.

## 10. The model, and its limits

**Run:**

```
python3 -I -B handoffs/0073-upscaling-review/repair-model/model.py \
    --out handoffs/0073-upscaling-review/repair-model/results.json
```

- Standard library only.
- Deterministic: two runs give byte-identical output.
- Python 3.9.6 and 3.14.8 give identical values.
- About 10 s of CPU.
- It exits nonzero if any case departs from its expected outcome.

**Not modelled:**
- **Scene:** grey scene-linear values on 32×32 or 48×48 outputs; textures at one
  texel per output pixel without perspective; one depth per surface; camera pans
  without parallax.
- **Shadow:** a vertical shadow band with 1.5 px texels and 3-tap PCF.
- **Rendering features:** no MASK, HDR, overlays, UI or half-float quantisation;
  the reactive term is not exercised.

It checks the design's logic, not GPU numerics. The engine's oracle and device
tests must establish every result again. No appearance, performance or phone
claim is made.
