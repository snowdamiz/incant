# 0072 fixed-mode sharpness — design packet

**Status: DESIGN ONLY.**
- No GPU, capture, Cargo, target use or production edit happened.
- The executed coverage report, its 49 evidence records and 14 PNGs are
  unchanged.
- Astra owns the numerical implementation. Every quality number below is a CPU
  model prediction or a labelled proposal. There are no GPU results here.

**Model and effort.** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max,
as this session reports them.

**Director feedback applied.** Root accepted `9927598c` (receipt
`0072-fixed-coverage-execution-root-20261013.json` `ed99f05b…`). The opt-in
acceptance and the softness limitation stand, with no default or phase
approval. This packet designs the smallest change that addresses the measured
softness and the 18% MASK detail:
- in scope: fixed-mode accumulation and presentation only;
- preserved: the accepted depth/coverage/motion admission.

## Inputs read (exact bytes at `e23776df`, via `git show`)

| File | SHA-256 (matches root's source review) | What it contributes |
|---|---|---|
| `temporal/resolve.wgsl` | `4e7f1ee5…` | Every filter inside the accumulation |
| `temporal/copy.wgsl` | `096ee89e…` | The presentation un-jitter |
| `motion/shader.wgsl` | `1ebe19d0…` | Motion = previous UV − current UV; "temporal projections include jitter" |
| `temporal/system.rs` | — | `reactive = [strength, jitter.x, jitter.y, 0]`; the resolve pass, then the copy into the HDR target before excluded surfaces |
| `output.wgsl` | — | Nearest `textureLoad` |
| `temporal/jitter.rs` | — | The 8 `SAMPLES` |

No pass is multisampled (`sample_count: 1`).

## How many spatial filters touch one displayed sample

In fixed mode, for static content:
1. **Rasterization.** One point sample at the jittered position (MASK alpha test
   at that sample). This is not a filter.
2. **History fetch.** A 2×2 bilinear history fetch at `p + m·size`
   (`resolve.wgsl:64–84`). For static content `m = j_prev − j_curr`
   (`motion/shader.wgsl:38–39`), so its fraction is 0.25, 0.5 or 0.75 on every
   frame. It is recursive: a sample of age k has been bilinearly resampled k
   times. Mean age is w/(1−w) = 9 frames at history weight 0.9, before the
   reaction lowers it.
3. **Clamp and blend.** The 3×3 per-channel min/max clamp (`:40–60`, `:90`) is
   nonlinear and bounds values, but it does not sharpen. The reactive temporal
   blend follows (`:87–90`).
4. **Presentation un-jitter.** One 2×2 bilinear at `p + j_curr`
   (`copy.wgsl:16–27`). It runs once, but on every displayed frame.
5. **Output.** Nearest `textureLoad`, tone map and sRGB (`output.wgsl:21`). This
   is pointwise, with no spatial filter.

So two linear resamplers act: a recursive one (about 10 bilinear passes on
average) and a single one. The anti-aliasing itself comes from these
re-interpolations between grids shifted by the jitter.

**Model check against the GPU.** The 1-D model below reproduces the measured
softness. Its baseline static edge is 2.20 px wide (10–90%), the width of a
Gaussian with σ ≈ 0.86 px. The GPU run measured an equivalent σ of 0.82–0.90 px.

## CPU predictions (`fixed-sharpness/model.py`, output `model-output.txt`)

**Method.** A deterministic 1-D model of the exact shader rules:
- jitter `SAMPLES` (both axes), geometry shifted by +j, motion including jitter;
- 3-tap neighbourhood, which is exact for straight axis-aligned edges and lines;
- view-depth range admission with the review tolerances 0.05/0.02, clear as the
  far plane, closest-depth motion with centre priority;
- the positive-tap renormalized bilinear fetch, `clean()`, per-channel clamp,
  reaction (0.9, 4.0, 0.5);
- the premultiplied, positive-alpha bilinear copy.

It runs 96 frames and measures the last full 8-phase cycle against the
pixel-area (box) ideal. It takes about 6 s in the foreground.

**Meaning of the columns:**
- "flicker" is the largest frame-to-frame change beyond true motion, as a
  fraction of edge contrast;
- "detail" and "modulation" are ratios to the box ideal;
- "residue" is error at least 2 px behind or ahead of a moving bar.

| Case and metric | Box ideal | **Baseline** | A: CR history | B: CR present | A+B | **C-reg** | C+A | N1 current-only | N3 unsharp 0.8 |
|---|---|---|---|---|---|---|---|---|---|
| Static depth/clear step: 10–90% width (px) | 1.32 | **2.20** | 2.08 | 2.06 | 1.91 | **1.36** | 1.36 | 1.52 | 1.68 |
| Static step: rms error vs ideal | 0 | **0.071** | 0.063 | 0.062 | 0.053 | **0.016** | 0.016 | 0.077 | 0.052 |
| Static step: flicker max | — | **0.088** | 0.091 | 0.084 | 0.097 | **0.111–0.115** | 0.111 | 0.625 | 0.118 |
| Static step: per-phase edge wobble (px) | — | **0.27** | 0.20 | 0.25 | 0.18 | **0.17** | 0.17 | 0.75 | 0.27 |
| Frames 0–3 after reset: max edge offset (px) | — | **0.43** | 0.43 | 0.38 | 0.38 | **0.43** | 0.70 | 0.43 | 0.43 |
| Static step: overshoot | 0 | **0** | 0 | 0 | 0 | **0** | 0 | 0 | 0.098 |
| 1-px MASK grille, period 4: modulation / detail | 1 / 1 | **0.44 / 0.38** | 0.57 / 0.49 | 0.51 / 0.44 | 0.65 / 0.57 | **0.95 / 0.96** | 0.95 / 0.96 | 0.88 / 0.84 | 0.67 / 0.58 |
| 1-px grille, period 2 (Nyquist stress): modulation | 1 | **0.08** | 0.08 | 0.10 | 0.10 | **0.69** | 0.69 | 0.52 | 0.16 |
| Grille: flicker max | — | **0.126** | 0.132 | 0.127 | 0.130 | **0.111** | 0.111 | 0.625 | 0.213 |
| Bar at 0.25 / 0.5 / 0.75 px/frame: rms | 0 | **0.066 / 0.070 / 0.066** | 0.059 / 0.065 / 0.059 | 0.059 / 0.063 / 0.059 | 0.052 / 0.058 / 0.052 | **0.055 / 0.068 / 0.055** | 0.046 / 0.057 / 0.046 | 0.058 / 0.084 / 0.058 | 0.050 / 0.058 / 0.050 |
| Moving bar: flicker max | — | **0.19 / 0.35 / 0.43** | 0.20 / 0.33 / 0.38 | 0.18 / 0.34 / 0.41 | 0.18 / 0.31 / 0.36 | **0.21 / 0.32 / 0.43** | 0.20 / 0.28 / 0.37 | 0.63 / 0.75 / 0.38 | 0.21 / 0.34 / 0.37 |
| Moving bar: residue behind / ahead | 0 | **0 / 0** | 0 / 0 | 0 / 0 | 0 / 0 | **0 / 0** | 0 / 0 | 0 / 0 | 0.012 / 0.012 |

The "C+A" column's reset row (0.70 px) is plain C without the registered
history-free fallback. That fallback is C-reg's only difference from plain C,
which is why plain C is not the candidate. With the fallback, C+A has C-reg's
static values. Two further negative controls appear in the model output:
- N2 (A without the clamp) overshoots by 1–3% and widens edges;
- N4 (A renormalized over admitted taps instead of falling back) matches A here,
  but keeps the risk of dividing by a small mixed-sign mass.

**Reading the table:**
- **Sharper kernels on the existing grid help only at the margins.** A, B and
  A+B narrow the static edge from 2.20 px to 1.91 px at best, and the grille's
  detail from 0.38 to 0.57. The cause is structural: every frame re-interpolates
  the whole static history between grids shifted by the jitter delta.
- **C-reg removes that re-interpolation for static content.** Static history is
  fetched at integer positions, and the copy becomes an identity load. The
  predictions:
  - edge width 1.36 px, essentially the box ideal's 1.32;
  - grille detail 0.38 → 0.96;
  - Nyquist grille modulation 0.08 → 0.69;
  - rms error down 4×;
  - per-phase wobble and grille flicker lower;
  - ghost residue still 0;
  - post-reset registration unchanged.
- **Its cost is slightly higher maximum flicker at static steps:** 0.088 → about
  0.11 of contrast. That is still a fifth of current-only aliasing (0.625).
- **Negative controls:**
  - current-only (N1) shows that "sharp" can simply mean aliased;
  - unsharp masking (N3) narrows edges, but adds halos (overshoot 0.10), raises
    flicker, and leaves a halo residue around motion.

## Change 1 (recommended, evaluate alone): C-reg — accumulate on the unjittered grid

**Summary:**
- history texel *i* holds the unjittered pixel *i*, instead of the current
  jittered grid;
- static history is fetched exactly;
- the copy stops resampling;
- history-free pixels keep today's registered reconstruction of the current
  frame. Whole-frame resets (first frame, cut, DRS) reproduce today's frames.

**Unchanged:** admission, neighbourhood, clamp, reaction, the motion pass,
targets, bindings, the 64-byte frame layout, MetalFX and upscaling.

### 1. Uniform: the jitter delta (`system.rs`)

**Encoding.** `reactive.w` is padding (0) today. It becomes an exact small
integer code for Δj = SAMPLES[prev] − SAMPLES[curr], in pixels:
- each component is a multiple of 0.125 in [−0.75, 0.75];
- `code = (Δx/0.125 + 8) + 16·(Δy/0.125 + 8)`, an integer in 0..255, exact in
  f32;
- use 0 for Δ when there is no previous frame. That frame is history-free
  anyway.

**Unchanged.** `reactive.yz` stays the current jitter.

**Shader decode:**
```
let code = u32(frame.reactive.w);
let dj = (vec2f(f32(code & 15u), f32(code >> 4u)) - 8.0) * 0.125;
```

**Why not pass two floats.** It would need a new uniform field, changing the
64-byte ABI, or an f16 bit-pack (NaN and denormal risk), so neither is
proposed.

### 2. `resolve.wgsl`

**Reprojection** (line 64):
```
let old = p.xy + m.xy*frame.rect.zw - dj;
```
- static content gives `old == p.xy`, so `f == 0` and exactly one tap of weight
  1 (zero-weight taps are already skipped);
- object motion gives `p − v`.

**Clear centre** (line 42). Start `m` at the static motion so an all-clear 3×3
also reprojects to `p`:
```
var m = vec4f(dj/frame.rect.zw, 1.0, 1.0);
```
Covered centres and closest-depth selection are unchanged.

**Registered history-free output.** At every place that today returns `c` for a
pixel inside the viewport (lines 30–31 when there is no history, 35, 61, 65 and
83), return `reg` instead:
- `reg` is today's `copy.wgsl` reconstruction applied to the current frame: a
  bilinear sample at `q = p.xy + frame.reactive.yz − 0.5`, `base = floor(q)`,
  `f = fract(q)`;
- weights per axis are (1−f, f), product in 2-D;
- taps are clamped to [lo, hi], and only finite taps with `a > 0` contribute
  `(max(rgb, 0), clamp(a, 0, 1))·w`, with no renormalization; then `clean()`;
- for |j| ≤ 0.375 its 2×2 taps are inside the already loaded 3×3 current
  window, so it needs no new loads. Keep the 3×3 values in a local array.

The outside-viewport, non-finite and degenerate-rect (line 39) returns stay as
they are.

**Everything else is unchanged:**
- `c` (the raw jittered sample) in the blend;
- the 3×3 min/max over raw current samples;
- depth range, closest motion, per-tap admission, renormalization, reaction and
  `clean()`.

### 3. `copy.wgsl`

**Identity, in place of the un-jitter:**
```
if inside rect: v = textureLoad(color, vec2i(p.xy), 0);
return finite and v.a > 0 ? vec4f(max(v.rgb, 0), clamp(v.a, 0, 1)) : vec4f(0);
```
Outside the rect it behaves as today.

### 4. Cooker

The trusted source hashes of the resolve and copy programs change. The ABI,
bindings and layouts do not.

### Premultiplied RGBA and edge cases

- **Overshoot.** All Change-1 weights are convex and non-negative: one integer
  tap, the existing renormalized bilinear, the registered bilinear and the
  identity. So no value leaves the hull of its inputs, and there is no new
  overshoot path.
- **Alpha bounds.** Alpha stays in [0, 1] through `clean()`.
- **Zero-alpha RGB.** This stays canonical zero: `clean()` maps a ≤ 0 to zero
  before and after every combination, and the copy and registered paths skip
  a ≤ 0 taps exactly as today.
- **Depth discontinuities.** The admission rule and its 3×3 view-depth range are
  unchanged. History colour now sits on the unjittered grid, while
  `previous_depth` is still the previous jittered depth, up to 0.375 px away.
  - Range admission is a one-pixel policy, so this matters only within about
    0.4 px of a depth edge.
  - The model's moving bars show no residue at 2+ px.
  - This is the main pairing risk, so it gets a dedicated control (test 6
    below).
- **Refused history.** It produces the registered current reconstruction.
  Whole-frame resets (cut, DRS) therefore match today's frames: post-reset
  offset 0.43 px, the same as the baseline. A disoccluded pixel among
  accumulated neighbours uses the same filter, but over current samples only.
  Plain C returns the raw jittered sample and reaches 0.70 px, which is why
  C-reg is the candidate.

**Cost per pixel** (from the shader; no measurement):
- **Resolve, static content:** history drops from up to 4 colour + 4 depth loads
  to 1 + 1, so 22 loads instead of up to 28. Moving content is unchanged.
- **Copy:** 4 loads become 1.
- **Registers:** the registered path reuses the 3×3 values (9 vec4 kept live).
- **Resources:** no new texture, binding or memory, and the uniform size is
  unchanged.
- Net: cheaper than today.

## Change 2 (only if moving content is still too soft after Change 1): C+A

Use a Catmull-Rom (Keys a = −½) history fetch for fractional object motion only,
when `f ≠ 0`:
- **Support.** 4×4 taps per pixel, starting at `base − 1`.
- **Per-axis weights:**
  - w(−1) = −½f³ + f² − ½f
  - w(0) = 3/2 f³ − 5/2 f² + 1
  - w(1) = −3/2 f³ + 2f² + ½f
  - w(2) = ½f³ − ½f²

  The 2-D weights are their product, and they sum to 1.
- **A 12-tap variant.** Dropping the four corner taps (each |w| ≤ 0.0039) and
  renormalizing is an acceptable cost variant.
- **Guard.** Use it only if every nonzero-weight tap passes the unchanged
  per-tap admission (inside the viewport, finite, non-negative, a ≤ 1, depth in
  range). Otherwise use the existing bilinear path. Never renormalize
  mixed-sign weights (N4).
- **Bounds.** `h = clean(Σ w·clean(tap))`, then the existing per-channel 3×3
  clamp bounds the negative-lobe overshoot to the current neighbourhood. N2
  shows the clamp is required.
- **Predicted effect.** Moving-bar rms falls from 0.055/0.068/0.055 (C) to
  0.046/0.057/0.046, and flicker from 0.21/0.32/0.43 to 0.20/0.28/0.37. Static
  content is unchanged.
- **Cost.** Up to 16 (or 12) colour and depth loads each, on moving pixels only.

## Controls for Astra's numerical implementation (CPU oracle and the ignored device test)

Retain the existing coverage, disocclusion, trailing-rejection, malformed-clear,
projection and reaction controls, re-derived for the unjittered grid. Add these
six:
1. **Code round-trip.** All 64 (prev, curr) phase pairs encode, decode and equal
   SAMPLES[prev] − SAMPLES[curr].
2. **Static identity.** Static content fetches exactly one history tap at `p`
   (f = 0). An all-clear 3×3 also reprojects to `p`.
3. **Registered history-free.** With no history, the resolve output equals
   today's copy reconstruction of the current frame (bit-exact, or within f16
   rounding).
4. **Negative: Δj not subtracted.** The oracle must fail on static drift.
5. **Negative: double un-jitter.** Keeping today's copy with the new resolve
   must fail registration.
6. **Negative: raw jittered sample for history-free pixels** (plain C). The
   reset-registration check must fail. Also add a depth-pairing case: a
   foreground edge exactly 0.25–0.375 px from a texel centre must admit and
   reject as the 3×3 range dictates.

## Proposed quality targets (proposals, not results)

**CPU model** (axis-aligned; `model.py`):
- static edge 10–90% width ≤ 1.5 px (box ideal 1.32, baseline 2.20);
- rms ≤ 0.03;
- flicker max ≤ 0.12 of contrast (baseline 0.088, current-only 0.625);
- per-phase wobble ≤ 0.2 px, and post-reset offset ≤ baseline;
- no overshoot;
- period-4 grille detail ≥ 0.9;
- moving-bar residue 0 and rms ≤ baseline.

**Future GPU re-review** (same frozen fixture, seven cases and policy; new
namespace and grant; old criteria and controls unchanged, with these sharpness
proposals added):
- **Sharpness:** grille detail (`detail.py`) ≥ 50% of reference (currently 18%);
  effective σ ≤ 0.6 px in the five regions (currently 0.82–0.90).
- **Stability:**
  - 33+-step activity pixels ≤ 2× the coverage run's 159, with none saturated;
  - edge mean (frames 9–16) ≤ 5;
  - no halo: no pixel more than 2 steps above the off reference's local 3×3
    maximum.
- **Ghosts:** off-edge counts no worse than the coverage run (830/30/0;
  391/0/0; 0); trailing band within 10% of the top-edge control.
- **Exclusion:** UI byte-identical; overlay 33+ activity 0.
- **History-free frames:** the cut and DRS captures stay byte-identical to the
  coverage run (or within 1 step).
- **Registration:** linear-light registration within 0.1 px.

## Risks and open questions

- **Design change.** Change 1 alters the "history on the jittered raster grid"
  choice in `temporal-coverage.md`. Admission rules are unchanged, but colour
  and depth pairing shift by up to 0.375 px (see Change 1, depth
  discontinuities, and test 6).
- **Model limits.** The model is 1-D and axis-aligned with one colour channel.
  Diagonal edges, 2-D grille lattices, HDR highlights and real MASK raster need
  the GPU review. The 0.11 maximum flicker may surface as more 17–32-step
  activity.
- **Reaction.** The reactive term is unchanged. Whether a lower effective blur
  changes how often it engages is untested on the GPU.
- **Scope.** Upscaling (0073) and MetalFX (0074) keep their own resolves. The
  shared motion pass is untouched.

## Files (this commit)

- `handoffs/0072-motion-history-numeric/fixed-sharpness-design.md`: this packet.
- `handoffs/0072-motion-history-numeric/fixed-sharpness/model.py`: the CPU
  model (standard library only). Run it with `python3 -I -B model.py` from its
  directory, in about 6 s.
- `handoffs/0072-motion-history-numeric/fixed-sharpness/model-output.txt`: the
  output quoted above.

Unchanged: `brief.md` (root's, uncommitted), every earlier result, report and
evidence, and `visual/`.

## Limitations

- **No GPU result.** Every number is a model prediction or a labelled proposal.
- **Bounded model.** It covers straight axis-aligned features over the actual 8
  jitter phases, one scalar channel plus alpha, and the review policy.
- **Scope.** Fixed-mode accumulation and presentation only. There is no
  upscaling, MetalFX, phone, performance or phase claim.
- **Unmeasured cost.** The load counts are derived from the shader text, not
  measured.
