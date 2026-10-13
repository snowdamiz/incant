# 0072 quality follow-up: declared candidates

**Status: declaration only.** This file is committed before `model.py screen` first
runs. It does not record any candidate result.

All numbers come from the CPU model (`model.py`), not from render evidence. The
model runs the 296e0226 resolve verbatim, with warmup weights and the actual
phase, sign and camera math. `reproduction.txt` checks it against the retained
GPU PNGs.

## What the candidates target

The target is the stationary flicker family: stationary saturated, stationary
edge max, viewport still saturated and viewport still edge max.

In the current shader, every frame-to-frame step of 33 or more on the far-post
and MASK edge patches, frames 9–16, is a *collapse*:

- **Far posts:** 15 of 15 steps land on a refused (R) or clamped (C) frame that
  directly follows an admitted frame.
  - 11 of these are refusals or clamps where the pixel's previous sample saw the
    far ground gap, but none of the current 3×3 samples does.
  - 4 are clamps where the pixel's previous sample was already inside the current
    depth range, but its history still carries the gap from an earlier phase.
- **MASK bars:** 7 of 7 steps are refusals where the pixel's previous sample saw
  the nearer bar, but the current 3×3 sees only holes.

The model reproduces these collapses at the GPU's failing pixels:

| Run | Model f16 against GPU | Edge max, model | Edge max, GPU |
|---|---|---|---|
| Posts | equal to within 1 step | 45 / 69 / 83 | 44 / 68 / 83 |
| Grille | within 5 steps | — | — |

The edge-max triples are baseline / a143 / warmup; the GPU triple is whole-frame
stationary.

The trailing and moving-shadow failures are not candidates here; see the last
section.

## Shared definitions (C1–C3)

All three candidates change only one decision: *when a still pixel holds its
unconditioned history*. Every other path stays verbatim, including:

- reset;
- refusal for invalid or new owners;
- viewport and `far < near` refusal;
- moving pixels;
- clear centres;
- the reaction term and the host weight.

The candidates are defined in `resolve.wgsl` terms, at the pixel `p`:

- **`still`:**
  - The centre is opaque (`c.a > 0`).
  - Every finite opaque 3×3 tap has valid motion with
    `max|m_k.xy*rect.zw - dj| < 1/64` px.
  - Clear taps do not break stillness, but they set `far = far plane` as the
    shader already does.
  - In the model all motion is valid.
- **`near0`, `far0`:** the 3×3 `near`/`far` before the
  `max(options.z, options.y*z)` widening.
- **`pc`:** `view_depth(previous_depth[p])`, the pixel's own previous sample.
  - For a still pixel `old = p`, so this is the one history tap with weight 1.
  - The source loads it at no extra cost when `f == 0`, or with one extra load
    otherwise.
- **`far_g`:** `still && pc > far0*(1 + 0.01) + 0.002`.
- **`near_g`:** `still && pc < near0*(1 - 0.01) - 0.002`.
- **Why the glimpse tests cannot fire on a smooth surface.**
  - The previous centre sample position is `p - jp`. It lies inside the box
    spanned by the current 3×3 sample positions, `p - j ± 1`, because
    `|jp - j| <= 0.75` px per axis.
  - On any plane, `1/z` is affine in screen position, so `pc` lies within
    `[near0, far0]`.
  - A glimpse therefore means the previous sample saw a *different surface* that
    none of the 9 current samples sees.
  - The 1 % + 0.002 margin covers curvature.
  - At clamped viewport borders the hull shrinks; this is recorded as a limitation.
- **Hold path:**
  `h = Σ w_k·clean(prev_color_k) / Σ w_k` over the in-viewport bilinear taps,
  *without* per-tap depth admission, and `target = h` with no clamp.
  - `reaction`, `weight` and `mix(c, target, weight)` are unchanged.
  - The output becomes `mix(c, h, weight)`.

## The three candidates

All three are fixed now and will not be tuned after screening.

| | Rule | Resources |
|---|---|---|
| **C1** — far glimpse | `hold = far_g` | **None.** `resolve.wgsl` only; ABI, bindings and budget unchanged. |
| **C2** — memory-gated near glimpse | `hold = far_g \|\| (near_g && mem_prev > 0)` | One R8Uint pair (below) |
| **C3** — persistent memory | `hold = far_g \|\| (still && mem_prev > 0)` | The same R8Uint pair |

**Memory rule, shared by C2 and C3.** It is written on every resolve path:

```
mem_new = !still           ? 0
        : (far_g || near_g) ? 8
        :                    max(mem_prev - 1, 0)
```

Every reset, refusal, invalid-owner, viewport and clear-centre path writes 0.

**Candidate rationale and expected limits.** These are predictions, not results:

- **C1:**
  - **Handles:** far features missed for a single phase (posts gaps).
  - **Cannot:**
    - a nearer bar (MASK);
    - a second consecutive miss;
    - the 4 "previous inside" collapses.
  - **Safe by construction:**
    - **Disocclusion and despawn:** the previous sample is *nearer*, so the rule
      does not fire.
    - **Spawn:** the new owner's motion is invalid on its first frame, so
      `still` is false.
- **C2:**
  - **Handles:** nearer bars missed for a single phase, at pixels that glimpsed
    within the last 8 frames (one jitter cycle).
  - **Why the memory gate:** the memoryless two-sided rule
    `hold = far_g || near_g` would fire on every still pixel just disoccluded
    behind a moving object. A 21-px strip behind the cube would hold the cube's
    colour. That rule is therefore excluded analytically, not declared.
  - **Expected cost:** a removed static object's aliased edge ring holds for one
    frame.
- **C3:**
  - **Handles:** consecutive misses and the "previous inside" collapses, because
    an aliasing site with memory holds every frame.
  - **Expected costs:**
    - shading changes at aliasing sites reach the output only at the
      exponential rate (about 0.86 per frame), not through the clamp;
    - a removed object's edge ring can hold for up to 8 frames.

**Constants** (`model.py`, fixed): `GLIMPSE_REL 0.01`, `GLIMPSE_ABS 0.002`
(view-depth units), `STILL_EPS 1/64` px, `MEMORY_FRAMES 8`.
- `MEMORY_FRAMES 8`: an aliasing site that glimpses once per 8-phase cycle keeps
  `mem_prev ≥ 1` at the next glimpse.

## Resource accounting for C2/C3

**What the memory is.** A pair of `R8Uint` textures at the internal temporal
size, ping-ponged with the colour history `slot`: 2 B per internal pixel.

**Why no existing channel can hold it:**
- History alpha already carries coverage.
- `previous_depth` is a depth texture.
- The motion target is written by the current frame's motion pass, not by the
  resolve.

**Memory cost:**

| Size | Steady state | During a size transition (old + new pairs) |
|---|---|---|
| 960×540 | 1,036,800 B | — |
| `MAX_MOTION_PIXELS` (4 Mi) | 8 MiB | 16 MiB |

The new `MAX_TEMPORAL_TRANSITION_BYTES` is 448 MiB + 8 KiB + 16 MiB.

**Source plan.** This is not implemented here and needs root's approval.
- **`temporal/targets.rs`:**
  - `Targets` gains `memory: [TextureView; 2]`, format `R8Uint`, usage
    `TEXTURE_BINDING | RENDER_ATTACHMENT`.
  - `bytes` becomes `pixels*18`.
  - New `MAX_TEMPORAL_MEMORY_BYTES = 2*MAX_MOTION_PIXELS`.
  - `MAX_TEMPORAL_TRANSITION_BYTES` gains `2*MAX_TEMPORAL_MEMORY_BYTES`.
  - The preflight term `pixels*40` becomes `pixels*42`, plus the old pair.
- **`temporal/system.rs`:**
  - `layout_entries(false)` gains binding 5 (`TextureSampleType::Uint`).
  - The resolve pipeline gains a second `ColorTargetState` (`R8Uint`, no blend).
  - The resolve pass gains a second attachment, `memory[slot]`, cleared to 0.
  - The bind group binds `memory[1 - slot]`.
- **`temporal/resolve.wgsl`:**
  - Add `@binding(5) previous_memory: texture_2d<u32>`.
  - The fragment returns `struct { @location(0) color: vec4f, @location(1) memory: u32 }`.
  - Every early return writes memory 0.
- **Unchanged:**
  - `copy.wgsl` and the excluded-surface draw are untouched; they run after
    publication.
  - The memory belongs to the same owner as the colour history. It is created,
    reset and released with it, so a reset frame writes 0 everywhere.
- **Dependency:** none new. The design does not assume access to Astra's 0073
  MRT work.

## Screening scenes

The screening uses warmup weights and full 8-phase sequences.

| Scene | What it is | Frames |
|---|---|---|
| `posts` | Far posts and sub-pixel ground gaps (case "far") | 16 |
| `grille` | MASK bars over holes (case "near") | 16 |
| `diag`, `thin`, `aligned` | Static no-motion controls | 16 |
| `cube` | Moving cube; trailing and top bands, and the disoccluded strip | f1–f8 |
| `shadow` | Moving shadow on static ground | f1–f8 |
| `occluder` | Moving occluder crossing the posts; depth rejection after it leaves | 16 |
| `despawn` | Static box removed at frame 9 with no motion; depth rejection | 16 |
| `sweep` | Shading edge sweeping over the static posts at 2.5 px/frame | 16 |
| `checker` | Flat checker with a 50 % BLEND card composited after publication (current-only overlay) | 16 |

## Declared pass criteria

Each criterion is measured against the verbatim shader on the same model. **No
GPU threshold is changed or relabelled by this screening.**

### Primary: flicker

Both `posts` and `grille` must reach **edge max ≤ 50 and saturated (≥ 64) = 0**.
These mirror the GPU stationary limits, applied here to the model patches only.

### Primary: detail kept

Smoothing that erases the feature fails. All of these must hold:

- posts gap retention ≥ verbatim;
- grille bar retention ≥ verbatim − 0.02;
- grille MASK detail ratio ≥ max(0.5, verbatim − 0.01);
- `diag` and `thin` detail ratio ≥ verbatim − 0.01.

Retention is the fraction of the 8×8-supersampled feature contrast kept at f16,
at the GPU's failing pixels.

### Safety

Each must not regress against the verbatim shader:

| Area | Criterion |
|---|---|
| Static controls | `diag` and `thin` edge max ≤ verbatim + 2; aligned-edge f6 bias, as an absolute value, ≤ verbatim + 0.01 px |
| Moving geometry | Cube trailing 1 px ≤ verbatim + 0.5; excess over the top control ≤ verbatim + 0.5; R−B excess in the disoccluded strip ≤ verbatim + 2 |
| Depth rejection | Occluder R−B residue in frames 11–16 ≤ verbatim + 2; despawn R−B residue from the removal frame (frames 9–16) ≤ verbatim + 2 |
| Moving shadow on static ground | Every off-edge bin and the maximum ≤ verbatim |
| Shading change | Sweep stale-shading residue ≤ max(16, verbatim). Measured on pixels that have been fully lit for 3 frames, against the same resolve on the unshaded posts. |
| Current-only overlay | Overlay pixels with frames 2–8 activity ≥ 33 ≤ verbatim |

**Reported but not criteria:** same-phase, halo, convexity, held-pixel counts,
≥ 33 counts and peak.

**Verdict.**
- **Supported:** both primary criteria hold and no safety criterion fails. A
  supported candidate gets an exact source prescription and a focused
  falsification test before any whole comparison.
- **Not supported:** the measured trade-off is reported unchanged.

## Not declared: trailing and the moving shadow

The model cannot screen either of these to the precision their gates need.

**Moving geometry, the trailing band.**
- **What the model gets right:**
  - the pipeline ordering, a143 > warmup > baseline;
  - the per-row shape;
  - the geometry, which matches the GPU off PNG exactly: 33 of 33 rows, and the
    top row.
- **What it does not:** the GPU band is a near-constant **+11.6 to +13.3 R−B**
  higher than the model in every pipeline, including the baseline.
- **The top control is not reproduced either:**
  - the model gives quantised 19–37 against the GPU's 38–44;
  - no single vertical offset fixes this.

**Moving shadow on static ground.** The model's leading-edge lag peaks at 8 steps;
the GPU reaches 10–11. The failing gate bin starts at 9.

**Deliverable for both:** a bounded diagnostic of exact intermediate quantities,
in `quality-followup.md`, rather than a guessed implementation.
