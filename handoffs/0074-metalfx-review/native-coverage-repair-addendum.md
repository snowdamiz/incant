# Native coverage repair addendum: numerical follow-up (design only)

**Status: design correction, CPU only.** No renderer source, GPU, Cargo or capture
was used for this addendum. `native-coverage-repair.md` (committed in `9998e60c`)
is left unchanged; this addendum replaces the parts listed in §6. Root owns the
implementation and the math.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
  Line 2 of root's ACP log `claude-0074-native-late-execution-acp.log` reads
  "Claude session verified: Opus 5.5; thinking: max".
- **Evidence:** every number below comes from
  [`coverage-model/model.py`](coverage-model/model.py) (`c1ed1a77…`), a
  deterministic, standard-library model of the engine-owned coverage history only.
  It does not model MetalFX colour. Its output is
  [`coverage-model/results.json`](coverage-model/results.json) (`432c1e29…`).
  Reproduce with `python3 -I -B coverage-model/model.py`.
- **Comparisons are matched resets.** The same rule runs on the scene without the
  departed owner, with history reset in the same frame. Only stale history can then
  differ, which follows 0073's C7 lesson.

## 1. Conventions verified in 077f7c39 source, not inferred from names

| Fact | Source |
|---|---|
| `SAMPLES` is an image shift in internal px, positive right/down. `ndc()` adds `[2·jx/W, −2·jy/H]` to the projection | `temporal/jitter.rs`, `motion/owner.rs` |
| `Frame.jitter = [J_cur/W, J_prev/W_prev]` in UV. The previous value uses the old extent | `metalfx/system.rs`, `motion/retained.rs` |
| `source_position(p) = source.xy + ((p − output.xy)/output.zw + jitter.xy)·source.zw`. Texel t at phase k samples the scene point `(t + 0.5 − J)·r` | `upscaling/common.wgsl` |
| The motion target holds `previous_uv − current_uv`. The current value is the jittered raster position; the previous one goes through the previous jittered matrix | `motion/shader.wgsl` |
| `pack` computes `delta = (m.xy + J_cur/W − J_prev/W_prev)·W`. That is the true current→previous displacement in current-content px, with both jitters removed | `metalfx/prepare.wgsl` (matches `numerics.rs`) |
| `m.z` is the point's previous NDC depth; `m.w` is validity (0 when off-screen, clipped or invalid) | `motion/shader.wgsl` |
| `Frame.depth = [near, far, orthographic, has_previous]`. `pack` writes reactive 0 only when the motion is valid **and** `has_previous` is set | `metalfx/system.rs`, `prepare.wgsl` |
| `compatible(a, b)`: both in (0, far], and `|a − b| ≤ max(options.z, options.y·b)`. `options = [history_weight, relative_depth, absolute_depth, reactive_threshold]` from `TemporalConfig` (0.05 relative, 0.02 absolute in REVIEW) | `common.wgsl`, `system.rs` |
| `linear_depth(d)` is 0 for d outside [0, 1), so clear is invalid. `admitted` means finite, all values ≥ 0 and a ≤ 1. `clean` returns zero when the value is non-finite or a ≤ 0 | `common.wgsl` |

**Correction to the committed design.** Its tolerance, max(0.001, 0.01·z), was the
`numerics.rs` oracle's test constant. The runtime test uses the `TemporalConfig`
tolerances above.

**Available without new memory:** the expected previous depth (`m.z`) of every
source sample. The motion owner also already retains the previous frame's depth
target (`temporal_input` `previous_depth`), although `normalize` does not bind it
today.

## 2. Root's points, with numbers

**(a) The uniform fractional class was missing.** When `lo = hi = a` with
0 < a < 1, the result is α = a exactly, colour `a·clamp(native, L, U)`, and no
history read, with `H_next = a`. This is the existing oracle's uniform-fractional
row; the class table now lists it explicitly (§5).

**(b) The stale-opacity claim fails when a new edge is nearby.** Model case B1:
- **Setup:** a nearer bar A (depth 3) is static for 24 frames, then leaves at
  6 px per frame. A static bar B (depth 5) has an edge inside the vacated pixels'
  3×3.
- **What R0 does:** the committed rule R0 takes B's motion (valid, zero) from the
  depth-dilated 3×3. The clamp range is [0, 1], so A's history passes:

| Excess opacity at vacated pixels | 1st frame after vacating | +4 frames | +12 frames |
|---|---|---|---|
| R0, 16/16 | 0.709 | 0.465 | 0.200 |
| R0, 8/16 | 0.810 | 0.332 | 0.143 |
| R1a (§3), 16/16 and 8/16 | 0.000 | 0.000 | 0.000 |

- **Fully clear neighbourhoods:** with no new edge (B1c), R0 is already exact (0),
  because the clamp is [0, 0]. The committed claim was true only for that case.

**(c) Depth-dilated motion can select a different owner.**
- **B1 is the counterexample.** The admitted motion and owner (B) differ from the
  owner that wrote the history (A), and "valid motion + clamped coverage" admits it.
- **B2, a moving variant.** A nearer bar D approaches a static bar C's edge and
  stops. After the stop, the contamination at C's edge pixels is:
  - 16/16: 0 for every rule;
  - 8/16: one frame of **deficit**, −0.417 for R0 and −0.157 for R1a, then 0.

  No excess opacity remains.
- **Not modelled:** lateral 2D passes, where an owner slides along another owner's
  edge.
- **Conclusion:** no rule here is "ghost-free"; §3 gives bounds instead.

**(d) Per-frame coverage clamping repeats 0073's V3.** A static 0.8 px wire over
clear at 8/16, at the wire pixel over one full cycle:
- R0 gives [0, 0.0875, 0.166, 0, 0.0875, 0, 0, 0.0875]. The clamp erases it in 4
  of 8 phases.
- The clamp is withdrawn (§3).

**(e) A fixed history weight of 0.9 is not the uniform 8-phase coverage.**
- **Steady state** is periodic, with weights `w_m = (1−h)·h^m/(1−h^8)` running from
  0.176 down to 0.084, against 0.125 for a uniform average.
- **Over every edge case** (1,318–1,642 per scale; 24 orientations × sub-pixel
  offsets):

| Scale | Max steady deviation from the uniform mean | Steady peak-to-peak | Max error, frame 8 after reset | Max error, frame 16 |
|---|---|---|---|---|
| 16/16 | 0.042 | 0.085 | 0.167 | 0.074 |
| 12/16 | 0.042 | 0.085 | 0.165 | 0.074 |
| 8/16 | 0.042 | 0.078 | 0.153 | 0.068 |

- **Worst retained trajectory** (16/16, edge normal 82.5°, offset −0.03 px): the
  per-phase coverage is [0.125, 0.625, 0.375, 0.875, 0.875, 0.375, 0.625, 0.125],
  with uniform mean 0.5. EMA frames 1–16: 0.125, 0.175, 0.195, 0.263, 0.324, 0.329,
  0.359, 0.336, 0.314, 0.346, 0.348, 0.401, 0.449, 0.441, 0.460, 0.426.
- **Weight trade-off at 16/16** (steady deviation / frame-16 error):

| Weight | Steady deviation | Frame-16 error |
|---|---|---|
| 0.875 | 0.055 | 0.052 |
| 0.9 | 0.042 | 0.074 |
| 0.95 | 0.020 | 0.170 |
| 0.98 | 0.008 | 0.281 |

  No single weight gives both small oscillation and fast convergence.
- **Partial fix:** a cumulative mean over the first 8 frames after a reset is exact
  at frame 8 for static content (0 error). Continuing at a fixed 7/8 weight brings
  back a 0.055 steady oscillation.
- **Exact steady state:** this needs per-phase storage (the last 8 coverage
  samples), for example 8×R8 (8 B/px).

## 3. Proposed bounded safeguard: owner-depth admission (R1a)

For each output pixel, after the coverage class (§5):

1. **Candidate owner.**
   - The primary bilinear tap, if it is covered.
   - Otherwise the nearest-depth covered tap of the 2×2.
   - Otherwise the nearest-depth covered tap of the 3×3.

   The 3×3 is a last resort. B1 shows that it is admissible only with step 3.
2. **Reproject** to `p_prev = P + delta·(output/source)`, using the candidate's
   packed motion.
3. **Admit per history tap, not per bilinear result:**
   - each of the 4 bilinear taps of H_prev at p_prev is admitted only if its stored
     owner depth is `compatible` with the candidate's expected previous depth (its
     `m.z`, linearised);
   - renormalise over the admitted taps;
   - with no admitted tap, there is no history and α = α_cur;
   - with no candidate, only history whose stored owner is clear is admitted.

   This follows e237's accepted per-tap gather rule, applied to coverage.
4. **Update:** `α = (1−h)·α_cur + h·α_prev`, with no per-frame coverage clamp.
   Store the candidate's depth (0 for clear) as the owner depth.

**Results:**
- B1 and B1c: 0 excess.
- B2: one frame at −0.157.
- B3: the wire at α_cur, with no erasure from the clamp.

**Residual bounds, stated rather than claimed away:**
- **R-tol (B4):** a departing surface within `max(0.02, 0.05·z)` of the replacing
  surface's depth is indistinguishable by depth. Its ghost behaves like R0:
  0.709 → 0.465 → 0.200 over 1, 5 and 13 frames, decaying as `h^n`. This happens
  only when two owners lie within the tolerance, for example an object sliding
  against a coplanar wall.
- **R-miss (B3, miss-run table):** a static feature that a phase misses entirely
  leaves no candidate in the 3×3, so it is not persisted. It blinks at current-only
  contrast: R1a gives [0, 0.875, 0.875, 0, 0.875, 0, 0, 0.875]. That is today's
  behaviour, so not a regression, but it is not repaired either.

  Longest consecutive-miss runs for a static vertical feature:

| Scale | 0.25 px | 0.4 px | 0.5 px | 0.8 px | 1.0 px | 1.6 px |
|---|---|---|---|---|---|---|
| 16/16 | 5 | 5 | 2 | 1 | 0 | 0 |
| 12/16 | 8 (never hit) | 5 | 5 | 2 | 1 | 0 |
| 8/16 | 8 (never hit) | 8 (never hit) | 5 | 5 | 2 | 1 |

**Rejected variant R1b: R1a plus persistence for up to 5 consecutive misses.**
- It keeps the wire present in every phase (0.41–0.52).
- But every fully vacated pixel ghosts for up to 5 frames (0.900, 0.729, 0.590, then
  dropped), because depth alone cannot tell "a phase missed a static owner" from "the
  owner left".
- This is the same flaw root found in 0073's draft.

**What exact separation needs.** To get neither ghosts nor blinking, history must
record explicit owner identity, and the engine must supply a per-frame
owner-changed set, as 0073's corrected admission does:
- an owner-ID channel from the opaque and MASK pass at source resolution;
- stored owner IDs in H;
- persistence only for owners that are unchanged this frame.

**Root decides between:**
- **R1a now:** no ghosts beyond the R-tol bound; thin sub-sample features stay at
  today's behaviour.
- **The owner-ID variant:** shared plumbing with the portable 0073 lane, at extra
  memory cost.

## 4. Cost and memory of the revised contract (estimates; no performance claim)

- **History:** α (R16F) plus owner depth (R16F) as RG16Float, ping-pong, is 8 B per
  output pixel, against 4 B in the committed design.
- **Per allocation:** `Plan::bytes()` goes from 25·px to 33·px.
- **Transition budget:** at the 4,194,304-px admitted maximum, two allocations need
  2·(33·4,194,304 + 2048) = 276,828,160 B. That is 64 MiB over today's 200 MiB +
  4096. Alternatively, cap the admitted extent at 3,177,503 px.
- **Exact per-phase accumulation (§2e):** +8 B/px for 8×R8.
- **Owner-ID variant:** + an R16Uint owner target per source pixel, plus stored
  owner IDs per output pixel (at least 2 B per stored owner).
- **Per-pixel work:** the per-tap gather reads 4 RG16F history taps instead of one
  bilinear R16F sample, plus 4 depth compares. The ring for N's class and colour
  range is unchanged from the committed design. The per-frame clamp is removed.

## 5. Revised contract (replaces §3.2's coverage rows of the committed design)

| Coverage class (3×3 N) | Depth class (2×2 F) | Output (premultiplied), α | H_next (α, owner depth) |
|---|---|---|---|
| interior: `lo = 1` | A: all compatible | native RGB, α 1 | (1, front depth) |
| interior | B: opaque depth step | `clamp(native, Lo − ε, Uo + ε)`, α 1 | (1, front depth) |
| uniform fractional: `lo = hi = a`, 0 < a < 1 | — | `a·clamp(native, L, U)`, α a, no history | (a, candidate depth or 0) |
| clear: `hi = 0` | — | with no admitted clear history: 0, α 0 | (0, 0) |
| boundary: `lo < hi` | — | `α·clamp(native, L, U)`, α from §3 steps 1–4 | (α, candidate depth or 0) |
| native RGB not finite or negative | any | today's `current` | (α_cur, candidate depth or 0) |

**Unchanged from the committed design:**
- the pack fill with unpremultiplied colour;
- depth output (z only for F opaque and compatible);
- native alpha ignored;
- overlays outside `pack` and H;
- the oracle-continuity table.

**Accumulation is specified explicitly.** Root chooses one:
- **(i)** a fixed weight, with a contract tolerance of 0.042 steady and 0.074 at
  frame 16 (h = 0.9; the §2e table for other h);
- **(ii)** a cumulative mean for frames 1–8 after a reset (exact at frame 8), then a
  fixed weight;
- **(iii)** per-phase storage, exact for static content at +8 B/px.

## 6. Superseded parts of `native-coverage-repair.md`

- **§3.2:**
  - the H update clamp to [lo, hi];
  - depth-dilated motion with any-owner bilinear admission;
  - the class table without the uniform fractional row.
- **§3, G2 and G3 claims:** "stale opacity … forces α = 0", which holds only for
  fully clear neighbourhoods, and "no leakage". These are replaced by the bounds in
  §3.
- **§6:** the history memory (4 → 8 B/px) and the transition-budget figures.
- **§4:** new oracle rows are required:
  - B1 / B1c with 0 excess under R1a;
  - B4 as a documented residual;
  - B3's R0 erasure as an expected failure and R1a's current-only behaviour as
    documented;
  - the §2e accumulation tolerance for the chosen option.

**0073 mistakes this avoids:**
- the per-frame clamp that erases what one phase misses (V3);
- admission inferred from motion and clamp signals instead of explicit owner
  metadata;
- clip boxes so wide they admit anything;
- dilated motion as the default motion source;
- convergence counted as ghosting (matched-reset references here);
- claims without a model and expected numbers.

## 7. Limitations

- **What the model covers:** y-independent bars with true 2D jitter sampling.
  - It does not model MetalFX colour, MASK texture coverage, lateral 2D passes or
    depth that changes along the motion. The expected previous depth equals the
    current depth here; the engine has `m.z`.
  - Numbers are model outputs for the design, not engine or GPU evidence.
- **Pixel acceptance is not offered.** That needs root's implementation, the
  existing `numerics.rs` oracle extended per §6, and a late-frame review on the
  repaired source.
