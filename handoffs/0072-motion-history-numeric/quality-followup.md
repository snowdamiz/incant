# 0072 quality follow-up: bounded CPU investigation of resolve flicker and trailing

**Status: CPU investigation complete.** No GPU, screenshots, captures, Cargo or
target were used, and no production file changed. No gate is claimed.

| Defect | Outcome |
|---|---|
| Stationary flicker | One of three pre-declared candidates, **C3**, meets every declared criterion in the CPU model. §4 gives its exact source prescription and a focused falsification test. It is a **model result, not render evidence**. |
| Trailing and moving shadow | The existing evidence cannot place the cause, so these get a bounded diagnostic, not an implementation (§5). |
| Halo and same-phase | Assessed in §6. Both stay failing. |

**Session details:**
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max. Root verified Max through ACP, per the brief.
- **Brief:** "Resolve quality follow-up: bounded CPU investigation", sha256
  `1eb02bc9…`. It was unchanged through the session and has no priority
  revisions; it stays uncommitted.

**Inputs:**
- the 296e0226 shader (`git show`);
- the three published runs:
  - `temporal-coverage-screenshots/` (e23776df);
  - `fixed-sharpness-screenshots/` (a143b5c0);
  - `fixed-warmup-screenshots/` (296e0226);
- the committed inspection outputs.

## 1. Model versus render evidence

**Render evidence:** only the retained GPU PNGs and the inspection files derived
from them.

**The CPU model, `quality-followup/model.py`:**
- **What it is:**
  - Standard library only, deterministic.
  - Exact fixture geometry, from `geometry.py`.
  - The 296e0226 resolve, verbatim:
    - per-tap `clean`;
    - the 3×3 box;
    - nearest-expected-depth motion;
    - range admission widened by `max(0.02, 0.05·z)`;
    - `old = p + m − dj`;
    - per-tap previous-depth admission;
    - `registered_current` on refusal;
    - reaction before the clamp.
  - Owner weights `0, min(0.9, (n−1)/n)`.
  - The `jitter.rs` SAMPLES and `delta_code` sign convention.
- **Colours:** taken from the temporal-off PNGs.
- **Penumbra:** least-squares fitted, at 0.07 ground units.
- **What its numbers are:** numerical predictions, never captures.

## 2. The model reproduces the failure mechanism (`reproduction.txt`)

**Agreement with the GPU at f16:**

| Pixel set | Model against the three GPU runs |
|---|---|
| Far posts, 6 GPU failing pixels | Equal to within 1 sRGB step in every run, e.g. (428,54): model 105/106/106, GPU 106/106/106 |
| Far-posts edge max, frames 9–16 | Model 45 / 69 / 83; GPU (whole frame) 44 / 68 / 83 |
| MASK grille, 5 pixels | Within 5 steps |

The run order is baseline e23776df / a143 / warmup 296e0226.

**Coverage of the GPU failures.** All 6 GPU saturated stationary pixels lie inside
the two modelled patches:
- (428,54), (428,55), (428,57) and (431,53) on the far posts;
- (556,242) and (542,243) on the grille.

**Mechanism.** Every step of 33 or more in frames 9–16 on edge pixels is a
**collapse**:
- **Far posts:** 15 of 15 collapses land on a refused or clamped frame right
  after an admitted frame. The transitions are refused 9 and clamped 6.
  - 11 occur where the pixel's previous sample saw the far ground gap but no
    current 3×3 sample does.
  - 4 are clamps where only older history carries the gap.
- **MASK grille:** 7 of 7 are refusals where the previous sample saw the nearer
  bar but the 3×3 sees only holes.

**Trace at (428,54).** The gap's true coverage is 0.75. The output climbs
84 → 106 → … → 146 over admitted frames, then a refusal resets it to the post
colour (84). Across the five gap pixels, f16 keeps on average only 35 % of the
true gap contrast (gap retention 0.354), against an achievable 1.022.

**Why the ramp raised the peaks.** The ramp makes the history the even phase
mean, so each collapse falls further.

**Moving content in the model:**
- **Cube trailing band:**
  - The pipeline order (a143 > warmup > baseline) and the per-row shape
    reproduce.
  - The GPU is **+11.6 to +13.3 R−B higher** in every pipeline.
- **Moving shadow:** the model lags by 8 steps; the GPU by 10–11.

See §5.

## 3. Declared candidates and screening

**Declaration.** `quality-followup/candidates.md` fixed three candidates,
their constants, the R8Uint accounting and the pass criteria. It was committed as
`9f4a6018` **before** `model.py screen` first ran.
- `screening.txt` is that run's unedited output.
- Rerunning `model.py` and `model.py screen` reproduces both files
  byte-for-byte.
- Since then `model.py` has gained only additions: checks and viewing aids.

**What all three candidates change.** Only when a *still* pixel holds its
unconditioned history: no depth admission and no clamp. They differ in the
trigger:
- **C1** — far glimpse;
- **C2** — far glimpse, or a memory-gated near glimpse;
- **C3** — far glimpse, or any still pixel whose 8-frame glimpse memory is
  nonzero.

Here a glimpse means the pixel's own previous sample lies outside the current
3×3 depth range by more than 1 % + 0.002.

**Results** (CPU model, warmup weights, full 8-phase sequences):

| Measure | Verbatim 296e0226 | C1 | C2 | **C3** |
|---|---|---|---|---|
| Posts: edge max / ≥ 64 / ≥ 33 | 83 / 5 / 14 | 69 / 1 / 9 | 69 / 1 / 9 | **34 / 0 / 3** |
| Posts: gap retention (ideal 1.022) / detail ratio | 0.354 / 0.517 | 0.956 / 0.657 | 0.956 / 0.657 | **1.039 / 0.678** |
| Grille: edge max / ≥ 64 / ≥ 33 | 68 / 1 / 7 | 68 / 1 / 7 | 28 / 0 / 0 | **28 / 0 / 0** |
| Grille: bar retention (ideal 0.912) / MASK detail ratio | 0.811 / 0.589 | 0.811 / 0.589 | 0.873 / 0.609 | **0.873 / 0.609** |
| Diag / thin / aligned controls | 13 / 7 edge max, bias 0.06 px | same | same | same |
| Cube trailing 1 px / excess / disocclusion | 38.73 / 11.21 / 0 | same | same | same |
| Moving shadow off-edge bins | 267 / 0 / 0, max 8 | same | same | same |
| Occluder residue / box-despawn residue (R−B) | 38 / 38 | 38 / 38 | 38 / **52** | 38 / 38 |
| Sweep stale shading | 33 | 33 | 33 | 33 |
| Overlay: differing backdrop values / ≥ 33 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| **Verdict** | — | flicker not met | flicker not met; depth rejection fails | **supported** |

The ideal retention is the plain mean of the last 8 raw samples.

**Post-screening checks** (`checks.txt`). These were added after screening ran;
none changes a verdict.

- **Thin-wire despawn stress test.**
  - **Setup:** a 0.6-px static wire at depth 40, removed without motion. This is
    not a declared criterion. The declared box was too large for its edges ever
    to glimpse, so it could not exercise C3's predicted cost.
  - **Verbatim:** removes the wire at once, but flickers it every phase while it
    is present (R−B 56, −9, 29, −9, …).
  - **C3:** keeps the wire stable while present (R−B 22–44 from f4), but leaves
    a decaying ghost after removal:

    | Frame | f9 | f10 | f11 | f12 | f13 | f14 | f15 | f16 |
    |---|---|---|---|---|---|---|---|---|
    | C3 residue (R−B) | 35 | 30 | 25 | 22 | 19 | 15 | 13 | 10 |

  - **C2:** 35, then 22 → 7.
  - **This is a real C3 regression.** Its size matches the verbatim shader's
    *existing* box-despawn residue, 38 → 9.
  - **Where that existing residue comes from.** The traced pixel (428,57) never
    samples the box itself. While the box is present, its 3×3 spans orange box
    samples and dark posts.
    - The per-channel clamp turns its grey history (173,173,170) into the box
      corner (164,108,91), so the output is (153,104,91).
    - That orange was never a sample.
    - After removal it still lies inside the posts' per-channel box, which
      allows R−B up to 93, so it decays only exponentially.
- **Sweep.**
  - **Verbatim:** already shows up to 33 steps of stale shading. The maxima are at
    ground pixels beside post edges, whose box spans post to ground, so a dark
    history is not clamped.
  - **C3:** held pixels stay within that, at up to 29.
- **Cube scene.**
  - C3 holds 25,678 pixel-frames:
    - 25,666 after the cube's disocclusion glimpse, holding clean background
      history;
    - 12 at never-covered static pixels.
  - f8 differs from verbatim at 23 of 20,500 pixels, max 10 steps, none moving.
- **C3 remaining peaks.** Three 34-step clamps at (427, 47–49). These pixels
  never glimpse: their neighbour (428) did.
- **C3 halo.** 2 pixels at (436, 47–48), outputs 121 and 112.
  - The point-sampled reference's 3×3 sees only posts (84).
  - The box-filtered truth is 103, and the jittered samples hit the gap in 2 and
    1 of 8 phases.
  - C3 therefore converges to the sample mean of real sub-pixel content. It does
    not overshoot those samples.

**Viewing aids.** `quality-followup/cpu-model-posts-truth-verbatim-c3.png` and
`quality-followup/cpu-model-grille-truth-verbatim-c3.png` are **CPU-model viewing
aids, not screenshots**. Each is magnified ×8:
- **Top row:** box-filtered truth | verbatim f16 | C3 f16.
- **Bottom row:** blank | verbatim activity | C3 activity, where grey = 4 × the
  largest step in frames 9–16.

They show the verbatim far-post gaps closed to post colour and the C3 gaps
restored. The isolated saturated activity is gone.

## 4. Prescription: C3

The only declared candidate within the existing ABI is C1, and it fails
flicker: posts 69 / 1, grille unchanged. A memoryless near-side rule would hold
freshly disoccluded pixels (C2's rationale in `candidates.md`). C3 therefore
needs one new resource, accounted below.

### Equations

These apply to `crates/incant_render/src/temporal/resolve.wgsl` at 296e0226 and
are not applied here.

**New declarations:**
- `@group(0) @binding(5) var previous_memory:texture_2d<u32>;`
- `struct Resolved { @location(0) color:vec4f, @location(1) memory:u32 }`

**Returns.** The fragment returns `Resolved`. Every existing early `return X;`
becomes `return Resolved(X, 0u);`. These cover:
- non-finite or out-of-rect;
- an empty viewport;
- reset (`frame.depth.w`);
- an invalid owner;
- `far < near`;
- `old` outside the viewport.

**3×3 loop:**

```wgsl
var still=c.a>0.0;
// if !finite4(value) { still=false; continue; }
// after `if !valid_motion(candidate)`: still=false; continue;
still=still && all(abs(candidate.xy*frame.rect.zw-dj)<vec2f(1.0/64.0));
```

**After the `far<near` check:** add `let near0=near; let far0=far;` before the
widening.

**History loop.** Before the `z<near || z>far` admission test, accumulate the
same tap into `loose += clean(color)*weight; loose_mass += weight;`.

**After the history loop:**

```wgsl
let dp=textureLoad(previous_depth,pixel,0);
let seen=still && dp>=0.0 && dp<=1.0;
let pc=view_depth(dp);
let far_g=seen && pc>far0*1.01+0.002;
let near_g=seen && pc<near0*0.99-0.002;
let mem_prev=textureLoad(previous_memory,pixel,0).x;
let mem=select(0u, select(max(mem_prev,1u)-1u, 8u, far_g||near_g), still);
let hold=(far_g || (still && mem_prev>0u)) && loose_mass>0.0;
var target:vec4f;
if hold { h=clean(loose/loose_mass); target=h; }
else {
    if !(mass>0.0) { return Resolved(registered_current(p.xy,lo,hi), mem); }
    h=clean(h/mass); target=clamp(h,minimum,maximum);
}
// change / reaction / weight unchanged, computed from h
return Resolved(clean(mix(c,target,weight)), mem);
```

**Clarification to `candidates.md`.** The no-admitted-tap refusal writes the
computed `mem`, as in the screened model. Only the listed early returns write 0.

**Additions beyond the model.** The model has neither invalid motion nor
non-finite taps. Making those break stillness is conservative: it only removes
holds.

### Resources and source plan

**Memory.** Two `R8Uint` textures at the internal temporal size, ping-ponged with
the colour `slot`.

| Size | Steady state | Size transition (old + new) |
|---|---|---|
| 960×540 | 1,036,800 B | — |
| `MAX_MOTION_PIXELS` (4 Mi) | 8 MiB | 16 MiB |

**Per-pixel cost:**
- one R8 load and one R8 colour-attachment write;
- one explicit `previous_depth` load at `p`. For a still pixel this is the texel
  its single history tap has just read.

GPU time is **unmeasured**.

**`temporal/targets.rs`:**
- `Targets.memory: [TextureView; 2]`.
- `bytes = pixels*18`.
- `MAX_TEMPORAL_MEMORY_BYTES = 2*MAX_MOTION_PIXELS`.
- `MAX_TEMPORAL_TRANSITION_BYTES += 2*MAX_TEMPORAL_MEMORY_BYTES`.
- Preflight `pixels*40` becomes `pixels*42`, plus the old pair.

**`temporal/system.rs`:**
- binding 5, `TextureSampleType::Uint`;
- a second `ColorTargetState`, `R8Uint`, with no blend;
- a second attachment, `memory[slot]`, cleared to 0;
- the bind group binds `memory[1-slot]`.

**Unchanged:**
- `copy.wgsl` and the excluded-surface draw: both run after publication.
- Lifecycle: the memory is created, reset and released with the colour owner, so
  a reset writes 0.
- Dependencies: none new. Astra's 0073 MRT work is not assumed.

### Expected limits (model)

**Fixes** depth-glimpse collapses at still sub-pixel features, without erasing
them: gaps and bars are retained *better* (above).

**Does not fix:**
- collapses with no glimpse at the pixel itself, such as the three 34-step clamps;
- colour-only aliasing on a single surface (texture or shadow-map edges), where no
  glimpse is possible;
- thin features against clear sky (no stillness evidence);
- any moving content, including trailing and moving shadows;
- whole-frame sites the model does not cover.

**Unmodelled grille-card sites.** The card's other activity clusters, max 52–60
at (542–640, 187–252), are not modelled. The holes there show shadowed ground
that the model does not shade: only 1,313 of 5,104 off-reference pixels in that
wider area match the GPU to within 8 steps.

**Costs:**
- thin-object despawn ghost, about 8 frames (above);
- shading changes at held sites follow the 0.86–0.9 exponential;
- stationary halo may rise: real sub-pixel content, but the gate already fails
  and stays failing;
- +2 B per pixel.

### Focused falsification test, before any whole comparison

**A. Numeric unit cases** in the existing numeric resolve fixture: five frame
pairs, comparing output and memory against 296e0226.

1. **Far glimpse.** The output equals `mix(c, h_loose, weight)` unclamped, and
   memory is 8.
2. **Memory hold without glimpse** (`mem_prev` 3). The pixel holds, and memory
   is 2.
3. **Disocclusion** (`pc` nearer, `mem_prev` 0). The output is byte-identical to
   296e0226, and memory is 8.
4. **Moving tap** (|Δ| = 1/32 px, `mem_prev` 8). The output is byte-identical,
   and memory is 0.
5. **Reset frame.** The output is byte-identical, and memory is 0 everywhere.

**B. One GPU run** of existing fixtures and harness only:
- stationary;
- viewport;
- motion-object;
- excluded;
- the cut and DRS resets.

It uses no new fixture and no more submissions than the warmup run spent on
those cases.

**Each item below falsifies C3 as prescribed:**

- **F1 — mechanism.** Stationary f16, sRGB per channel, must match within ±3 on
  the posts and ±6 on the grille. The verbatim model matched within 1 and 5.

  | Pixel | Predicted f16 |
  |---|---|
  | (428,54) | (164,163,162) |
  | (428,56) | (164,163,162) |
  | (431,53) | (164,164,162) |
  | (431,50) | (147,147,147) |
  | (434,49) | (124,125,126) |
  | (425,52) | (85,87,94) |
  | (556,242) | (117,170,132) |

  The current values are 106/107 G at the gaps.
- **F2 — the modelled sites.**
  - None of the 6 current saturated pixels saturates.
  - Edge steps in frames 9–16 stay ≤ 37 at the far posts (x 418–441,
    y 44–61) and ≤ 31 at the grille (x 532–563, y 230–251): the model's 34
    and 28, plus 3.
- **F3 — nothing else moves:**
  - **Motion-object:**
    - trailing 1 px within ±0.5 of 51.45;
    - top control within ±0.5 of 38.07;
    - off-edge bins equal.
  - **Skinned and motion registration:** within 0.005 px.
  - **Excluded:**
    - the UI is identical in all 8 phases;
    - overlay ≥ 33 stays 0;
    - excluded halo stays 0.
  - **Reset and DRS:** history-free frames are byte-identical.
- **F4 — detail kept.**
  - MASK detail ≥ 0.577.
  - Every effective σ ≤ 0.6.

**Recorded, not falsifiers:**
- whole-frame stationary and viewport edge max and saturated (the gates),
  including the unmodelled card clusters;
- halo;
- same-phase.

## 5. Trailing: moving geometry versus its moving shadow, and the diagnostic

### Isolation

**Moving geometry.** Both trailing gates measure the cube's own left silhouette:
- 33 rows, y 190–239, at x 436–438;
- the no-motion top control on row 178.

**Moving shadow.**
- **Geometry:** the cube's f8 shadow lies at x 471–606 and y 264–306 (CPU
  geometry). That region is disjoint from both bands.
- **The off-edge failure:** 35 of the 36 object off-edge pixels at ≥ 9 steps are
  the moving shadow's leading edge on static ground, at (579–600, 304–305), max
  11 (`off_edge_clusters-object-warmup.txt`).
- **The remaining pixel:** (445,58), on the static far posts.

So the two failing trailing gates are moving geometry only, and the failing 9–16
off-edge bin is the moving shadow only.

**C3 changes neither** in the model: these pixels are not still, or the ground
is a single surface.

### Why there is no candidate

**What the model does reproduce:**
- the cube geometry (33 of 33 staircase rows, the top row and the colours);
- the pipeline order of the trailing band.

**What it does not:**
- a pipeline-independent +11.6 to +13.3 R−B offset. It appears in the
  e23776df baseline too, whose resolve is very different.
- The top control: the model's values are quantised at 19–37 against the GPU's
  38–44, and no vertical offset of the samples fixes this.
- The shadow lag: 8 in the model against 10–11 on the GPU.

**What remains unknown:** whether the offset arises inside the resolve, or in its
inputs (edge coverage and shading in `current`, the motion at the band, previous
depth). Any trailing or shadow prescription would therefore be a guess.

### Smallest diagnostic

**Scene and frames.** motion-object, the existing f1–f8 sequence, with 296e0226
unchanged.

**What to read back.** The resolve's inputs and output at **f7 and f8**, in two
crops:

| Crop | Area | Covers |
|---|---|---|
| A | x 400–520, y 170–250 | trailing rows 190–239, the top row 178, and the 21-px-left history sources |
| B | x 560–611, y 290–315 | the shadow cluster |

**Inputs:**
- `current` (RGBA16F);
- `motion` (RGBA32F: xy, expected previous depth, validity);
- `previous_depth` (D32F raw);
- `previous_color` (RGBA16F).

**Outputs:**
- the resolve output (RGBA16F);
- the 64-B `Frame` uniform.

**Budget.**
- 11,153 px × 44 B, about 0.49 MB per frame, so about 0.98 MB in total.
- One fixture.
- No new scene.
- A test-only readback hook, which is root's or Astra's call: it needs
  `COPY_SRC` on the motion and depth targets; colour history already has it.

**Replay and decision.** Replay `model.py`'s resolve on the captured inputs and
record, per pixel:
- the chosen motion tap;
- `near0` / `far0`, raw and widened;
- `old`;
- the four tap weights, depths and admission;
- `h`, `mass`, the box min/max, reaction, weight and output.

Then decide:

| Finding | Conclusion |
|---|---|
| Replay equals the GPU output (≤ 1 step), but differs from the analytic model | The offset is in the inputs. Compare `current`, motion and depth with the model at the band. |
| Replay differs from the GPU output | The shader's arithmetic differs from the model. |
| Shadow, crop B | Measure the jittered `current` box max against the penumbra. A wider box from shadow-map texel noise would explain the 2–3 missing steps. |

**Hypothesis for the band.** Admitted dilated-cube history, plus per-channel
box-corner clamping (§3), on inputs that differ from the analytic model. Both
are checkable from the replay.

## 6. Halo and same-phase

Both fail on the accepted baseline as well. **Both are retained as failing.**
Whether to amend either rule is root's decision.

### Halo

**What the shader guarantees.** In C-reg (a143 and 296e0226), every output lies
within the current 3×3 jittered sample range:
- refusal returns a bilinear mix of current samples;
- otherwise the output is `mix(c, clamp(h,min,max))`.

**Model confirmation.** The model finds 0 outputs outside the current 3×3 range
in posts, grille and penumbra, against 1,470, 1,183 and 380 for the e23776df
baseline. These counts include outputs above and below the range.

**Where the GPU halo pixels are.** Most of the GPU's 101 pixels lie on row 243,
along the wall base. There the point-sampled reference has a one-pixel sliver,
which C-reg spreads over two rows.

**C3's halo pixels.** They sit where the jittered samples see a sub-pixel gap that
the reference's 3×3 misses entirely.

**Assessment.** These indicate a metric limitation: the reference is
point-sampled, so its 3×3 maximum is not an upper bound on correctly
antialiased content. They are not resolve overshoot.

### Same-phase

**The recursion.** The model evaluates the exact recursion for a pixel that is
never clamped and whose samples take two values. It uses warmup weights and the
reaction, over all 254 mixed 8-phase patterns.

| Sample pair | Largest f8/f16 change | Patterns above 2 steps |
|---|---|---|
| 130/150 | 1 | 0 |
| 120/160 | 2 | 0 |
| 100/180 | 5 | 30 |

**Why f8 and f16 differ.**
- f8 is the ramp's even mean.
- f16 is the w = 0.9 exponential's phase-weighted mean, with weights 0.176 down
  to 0.084.
- A soft static penumbra converges, with 0 changes.

**Where this happens.** A same-phase failure needs jittered samples that vary by
more than 40 steps. No 40-step pattern exceeds 2 steps, while 30 of the 254
80-step patterns do. On a pixel the point-sampled reference marks as non-edge
(≤ 24-step neighbours), that means sub-pixel content the reference misses.

**Assessment.** This is real convergence behaviour of the accumulator, and it is
bounded. It is neither overshoot nor noise.

## 7. Limitations

- **Patches only.** The model covers two static patches and controls, not the
  whole frame.
  - Grille pixel agreement is within 5 steps.
  - The wider card area is not modelled (§4).
  - The viewport fixture is not modelled.
- **Not modelled:**
  - invalid or new-owner motion (spawn);
  - clear-sky neighbourhoods;
  - fp16 storage;
  - viewport-border hull shrinkage.
- **The safety controls are synthetic,** apart from the cube and shadow: diag,
  thin, aligned, occluder, despawn, wire, sweep and checker.
  - The wire test was added after screening.
  - It shows a real C3 cost that the declared box despawn could not exercise.
- **Prescription status:**
  - The C3 prescription is unimplemented and its performance unmeasured.
  - Any phone or performance claim needs the plan's shipping-profile
    measurements on real devices.
  - The falsification test must pass before any whole comparison.
- **Unresolved model-level observations.** The verbatim shader's per-channel
  RGB clamp can manufacture box-corner hues (§3). It leaves:
  - occluder-exit residue: 46 and 38 for 2 frames;
  - box-despawn residue: 38 → 9.

  These are model findings with no GPU evidence. They are not a declared
  candidate and were not pursued.

## 8. Files and commands

**Files in `quality-followup/`:**

| File | sha256 (first 16) |
|---|---|
| `candidates.md` | `84d2ec69ee95bece` |
| `geometry.py` | `835f18933ef4d650` |
| `model.py` | `2738a88dc9e04573` |
| `reproduction.txt` | `124f68ba7c072dab` |
| `screening.txt` | `1fb9cbc93d2b5ff0` |
| `checks.txt` | `5e1077f4ea76702b` |
| `cpu-model-posts-truth-verbatim-c3.png` | `3c87d510b00f2c2d` |
| `cpu-model-grille-truth-verbatim-c3.png` | `cbb52259e5e69a85` |

**Commands.** All were run from `handoffs/0072-motion-history-numeric/` with
`/opt/homebrew/bin/python3` 3.14.8. Each was run in the foreground on the CPU and
exited 0.

| Command | Time |
|---|---|
| `python3 -I -B quality-followup/model.py > quality-followup/reproduction.txt` | 28 s |
| `python3 -I -B quality-followup/model.py screen > quality-followup/screening.txt` | 21 s |
| `python3 -I -B quality-followup/model.py checks > quality-followup/checks.txt` | — |
| `python3 -I -B quality-followup/model.py viewaid` | — |

- **Determinism:** reruns of the first two are byte-identical (`cmp`).
- **One failed run:** a `checks` run exited 1 on an indexing bug I introduced
  while adding the cube attribution. It was fixed and rerun; the retained file
  is from the exit-0 rerun.

**Not done:**
- no GPU, screenshots, captures, Cargo, target, production edits, pushes, PRs,
  merges or memory writes;
- no subagents or account changes;
- no gate approval.
