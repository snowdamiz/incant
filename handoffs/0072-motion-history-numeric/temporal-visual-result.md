# 0072 stage B result — temporal accumulation visual review (executed)

## Verdict

The behaviour is correct, but quality is not yet acceptable for default use.

**Execution:**
- **Regression:** 96 of 96 existing ignored cases pass, each suite once from a
  fresh build. That includes both temporal numeric tests and both motion
  tests.
- **Review:** all 7 cases pass their contract checks.
- **Rendering spend:** 70 world frames, 9 UI overlay submissions and 14 PNGs,
  out of budgets of 96 and 16, with no failure and no retry.

**What is correct (actual Apple M5 Pro / Metal pixels):**
- **History ownership.** Phase, epoch, extent, owner, explicit cut, DRS reset and
  return to output scale all behave as specified.
- **No ghosting.** I found none in disocclusion, camera pan, skinned
  deformation or the cut.
- **Stable UI.** The UI is byte-identical across every jitter phase.

**The main defect.** Every silhouette, thin opaque feature and MASK cut-out stays
current-only and **shimmers on every frame** after convergence. The temporal-off
image has none of this shimmer. Transparent, sprite and world-text overlays also
flicker at full jitter amplitude.

**Recommendation:** keep temporal accumulation opt-in. Fix the depth-edge
rejection and the overlay jitter (below) before any default enablement or
appearance acceptance.

There is also one retained failure: the optional CPU-only fixture lint, after
all GPU work (details below).

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max, preserved on this resumed session as reported by the
  session.

**Brief revisions applied:**
- **Revision 6:** CPU build grant.
- **Revision 7:** root's supervised build recovery, then this one bounded
  execution. I made no rebuild, re-acquisition or repeat of the revision 6
  commands, and no production or fixture edits.
- I made no memory writes, push, PR or phase approval.

## Identity

| Item | Value |
|---|---|
| Source | `9009f088ac230e31185e8ce64183ca93878e9444`. Build inputs are identical to the reviewed `e73afe43`, and the 57-file freeze `06a242fd…` was verified at checkout. |
| Grants | `review-ready.json` `992ddf0b…`; `execution-grant.json` `3121cda3…`; root build recovery `728d2bc9…` (original logs retained) |
| Visual tree | `ac8a48de3a4df9314bee210c6e3c050172ec4862` (re-verified before every child) |
| Review binary | `temporal_visual_review` `48557af4894734528ec39d929646af548de46fe55089fa6308944d1dcaf5ba10`, mode 0555. It links the identical renderer library the regression build produced (rlib `b0e8bd61…`, rmeta `3d9152b7…`). |
| Regression binaries | `incant_render-lib-tests` `21fa85e4…`, `model_gpu` `c638a76b…`, `timestamp_backend_probe` `a3d76ebe…`, `gpu_host-game` `f8544212…`, `gpu_host-ui` `d54e171d…`; features `default`, `physics-deterministic` |
| Toolchain and host | rustc/cargo 1.99.0 (LLVM 23.1.1); macOS 27.0.1 (26A434), Mac17,9, Apple M5 Pro |
| Adapter (recorded by the fixture) | **Apple M5 Pro, Metal, IntegratedGpu**; `INCANT_EXPECT_GPU_BACKEND=Metal` was enforced |
| Config (explicit, validated) | history 0.9, relative depth 0.05, absolute depth 0.02, reactive threshold 4.0, strength 0.5 |
| Target | `incant-core-presentation-target`, leased 06:50:01Z and released 07:10:43Z with no process on it. Its contents are left as warm cache. |

## Execution (all children foreground, serial, under the global `native-gpu.lock`)

| Step | Result |
|---|---|
| Listings | lib 38, model_gpu 50, timestamp probe 1, game 5, ui 2, review 7 — each matches the plan |
| Suites (once) | **lib 38/38** (4 s), **model_gpu 50/50** (49 s), **timestamp probe 1/1**, **game 5/5**, **ui 2/2** |
| Review cases (once, in plan order) | probe, stationary (17 frames), motion (16), excluded (9 + 9 UI), transitions (13), skinned (7), timing (8): **all pass** |
| GPU release (07:09:55Z) | 21 children (3 builds and 18 GPU); debits = use = 70 frames + 9 UI + 14 PNGs; no stop; no owned process |
| `analyze.py check` | No problems. Every log, report and PNG matches its ledger hash. |

One command was mistyped: zsh passed `"list review"` as a single argument, so
`gpu.py` printed its usage before any launch. It left no ledger row, refusal or
latch, and I re-ran it with explicit arguments.

### Retained failure: optional fixture lint

`build.py lint` ran after the GPU release, under the still-held lease. It ran
`clippy -D warnings` and exited 101:

```
error: variable does not need to be mutable
    --> crates/incant_render/tests/temporal_visual_review.rs:1418:9
1418 |     let mut report = Report::open("probe", 0, 0);
     = note: `-D unused-mut` implied by `-D warnings`
```

**Effect:**
- It latched `stop.json` at 07:10:20Z, by design.
- No GPU or build work remained, and the target and publication steps don't
  check the latch.

**Cause.** `probe` never calls a `&mut self` method on its report, and
`finish(mut self)` takes the report by value. The ordinary build only warns.

**Fix for the next fixture revision.** Drop the `mut` on line 1418. Under this
grant I made no edit.

## Quality findings (personally inspected: all 14 PNGs, every report and the pixel statistics)

The screenshots are in
`handoffs/0072-motion-history-numeric/temporal-visual-screenshots/`
(14 PNGs, plus `manifest.json` `c688e726…`). All are 960×540 Rgba8UnormSrgb.

### 1. Stationary convergence — interiors converge; edges shimmer permanently (defect)

Screenshots: `stationary-off.png`, `stationary-on-f16.png` and
`stationary-activity.png` (the largest change between consecutive frames over
frames 9–16, ×4).

**Interiors converge to a phase-locked state:**
- Frame 8 against frame 16, the same jitter phase one cycle later, differs on 1
  interior pixel (max 3 steps) and 614 edge pixels (max 6).
- Frame 16 against temporal-off: interior mean 0.21 steps.
- Ground shading, the sphere body and the faces of the cube and wall are stable.

**Edges never settle:**
- In every frame of both cycles, 7,389–14,606 of the 34,327 reference-edge
  pixels change by more than 2 steps. Most transitions reach 13,000–14,600; the
  ones out of jitter phase 3 are lower.
- The edge mean is 13–25 sRGB steps per frame, with a maximum of 169.
- The activity map shows a bright one-pixel rim on every silhouette: all 19
  fence posts, the horizon, the cube, the wall, the full sphere outline and the
  thin ramp post.
- **The whole one-texel MASK grille is solid noise.**
- Shadow edges keep a faint residual flicker.
- The temporal-off image has none of this, because it is not jittered.

**Cause (source, now confirmed by pixels).** `resolve.wgsl:43-47` returns
current colour for a pixel if any of its four bilinear history taps fails the
depth test, including taps with zero weight. With zero motion, the tap set is
{p, p+x, p+y, p+xy}. So every pixel whose right, lower or diagonal neighbour lies
across a depth discontinuity is permanently current-only. Silhouettes, 1–2 px
geometry and MASK cut-outs therefore show the raw ±0.375 px jitter every frame.

**Consequence.** Accumulation as implemented adds edge shimmer to exactly the
detail that temporal anti-aliasing should stabilise.

### 2. Object motion and disocclusion — no ghost trail; slight moving-shadow lag

Screenshots: `motion-object-on-f8.png` and `motion-object-off-f8.png`. The cube
moves about 21 px/frame in front of the thin wall and passes behind the sphere.

**No ghost in uncovered or occluded areas.** The wall and ground the cube
uncovered are clean, and nothing ghosts where the sphere occludes it. Against
temporal-off at the same pose, the differences away from any edge (3 px or more,
edge threshold 12) are:

| Image | 3–8 steps | 9–16 steps | Above 16 |
|---|---|---|---|
| Stationary baseline | 781 | 0 | 0 |
| This frame | 1,080 | 47 | 0 |

**Minor defect: shadow lag.** The 47 pixels form one 31×16 patch (x 570–600,
y 290–305), where the cube's moving sun shadow lands in the cool point-light
pool. The temporal-on image is uniformly up to 11 steps brighter there.
- The cause: lighting changes on static receivers have zero motion and are
  bounded only by the 3×3 clamp.
- The reactive term (threshold 4.0 scene-linear) barely engages for shadow-sized
  changes.
- It is not visible at 1:1, but it is measurable.

### 3. Camera pan — no smear

Screenshots: `motion-pan-on-f6.png` and `motion-pan-off-f6.png`, a lateral
pan with yaw (parallax).

Off-edge differences are 1,044 pixels at 3–8 steps, 6 at 9–16 and none above.
There is no trailing on the posts, the sphere or the grille. The edge shimmer
from finding 1 persists.

### 4. Thin opaque and MASK detail — worse with temporal on, under jitter

The one-texel MASK grille and the receding posts show finding 1 at its worst.
- **Grille:** every texel edge flickers every frame.
- **DRS half scale** (`transition-drs-f9.png`, 480×270 internal): the grille
  breaks into an aliased pattern, and silhouettes show the expected
  nearest-upscale blocks. That is the DRS output path, not temporal.

### 5. Skinned motion — clean

Screenshots: `skinned-on-f6.png` and `skinned-off-f6.png`. The skeletal
fixture's clip 1 rotates two vertices about 40 px/frame over a checker.

- 99.8% of the 12,638 differing pixels lie on an edge. The remaining 21 sit on
  the moving hypotenuse where its contrast against a light square is below the
  edge threshold.
- No pixel 3 px or more from an edge (threshold 12) differs at all.
- I found no ghost or lag from the deformed vertex buffers.

### 6. Reset, cut and DRS transitions — correct

**Epoch and phase sequence:**

| Event | Epoch | Phase | History |
|---|---|---|---|
| View A | 1 | 0–5 | builds normally |
| Explicit cut to view B | 2 | 0 | none |
| Half-scale DRS | 3 | 0 | none (internal 480×270) |
| Return to output scale | 4 | 0 | none |

One owner holds throughout.

**Visual check.** `transition-cut-f7.png` shows view B with no trace of view A.
Against the temporal-off view B its interior mean is 0.115, max 24, and those
are jitter-only edges. The two phase-0, no-history frames before and after DRS
produced identical statistics.

### 7. Excluded surfaces — current-only overlay jitter is visible; UI is stable

Screenshots: `excluded-on-f8.png`, `excluded-off.png` and
`excluded-activity.png` (frames 2–8).

**UI:** the pixels are **byte-identical** to temporal-off in all 8 phases (max
difference 0). Screen-space UI is never jittered.

**Overlays:** BLEND glass outline, all sprite texel edges and every world-text
glyph outline (including the uncovered Hebrew `.notdef`) flicker at full
amplitude.
- Edge mean is about 10.7 steps per frame, with a maximum of 243.
- They are jittered with the opaque raster but never accumulated, so they cannot
  converge. This is the documented current-only limit, and here it is plainly
  visible.

**Opaque checker:** a ~10% residual on nearest-sampled texel edges, the expected
exponential residual at weight 0.9. It is acceptable.

### 8. Timing — the Models span covers the temporal passes (inference)

Each frame produced exactly one sample, with none skipped or abandoned. Median
group spans, temporal off against on:

| Group | Off | On | Change |
|---|---|---|---|
| Models | 0.825 ms | 1.557 ms | **×1.89** |
| Shadows | 0.180 ms | 0.175 ms | unchanged |
| AssignLights | 0.077 ms | 0.076 ms | unchanged |
| InitializeScene | 0.024 ms | 0.023 ms | unchanged |

The Models end moves out by about 0.75 ms, consistent with motion, resolve and
copy at 960×540 being encoded inside Models.

**Caveats:**
- **Overlapping groups.** On this backend the group intervals overlap. Display
  starts at about 166 µs while Models runs to about 950 µs, so Display's growth
  (0.85 → 1.53 ms) is overlap, not Display cost.
- **Out-of-frame start.** InitializeScene can begin up to about 19 µs before the
  frame timestamp (`groups_within_frame: false`).
- **Not a performance measurement.** The timestamp period is 1.0.

## Recommendations for Astra (implementation owner)

These are concrete findings only. I changed no production shader or source.

1. **Depth-edge rejection (priority).**
   - Depth-test only taps with non-zero bilinear weight, or renormalise over the
     passing taps instead of rejecting all four.
   - Then rely on the neighbourhood clamp at silhouettes rather than
     current-only output.
   - Re-run `stationary` and compare its activity map. The edge rim should
     disappear while `motion`, `skinned` and the cut stay ghost-free.
2. **Overlay jitter.** BLEND models, sprites and world text receive the jitter
   but never accumulate, which guarantees shimmer. Evaluate drawing excluded
   overlays unjittered over the resolved image, or giving them their own
   accumulation path.
3. **Lighting-only changes (minor).** Moving shadows on static receivers lag by
   up to about 11/255. Consider a radiance-change or reactive signal for
   lighting changes, or a lower reactive threshold. The trade-off is texture-edge
   accumulation.
4. **Default enablement.** Keep it opt-in until 1 and 2 are addressed and
   re-reviewed.
5. **Fixture.** Remove `mut` at `temporal_visual_review.rs:1418` so the
   optional clippy lint passes.

## Raw outputs and receipts

`handoffs/0072-motion-history-numeric/temporal-visual-evidence/` holds 64 files
(about 300 KB) copied unchanged from
`~/.codex/incant-evidence/0072-temporal-visual`. Its `manifest.json` gives each
file's source path and SHA-256. Included:
- the 7 case reports and event logs;
- the stdout and stderr of all 18 GPU children;
- the lint stderr (`e2d08049…`);
- the ledgers: `gpu-ledger.jsonl` `c66b02e6…`, `post-release-ledger.jsonl`
  `6a66ac4c…`, `budget.jsonl` `c92f4e37…`;
- the latches: `gpu-released.json` `d2342da2…`, `stop.json` `080e4850…`,
  released `target-lease.json` `72576ada…`;
- the grants and recovery receipt;
- the build records (`tests.json`, `review.json`, `toolchain.json`,
  `source.json`);
- the analysis summary.

## Limits

- **One host and backend.** One local desktop on one adapter (Apple M5 Pro,
  Metal). No phone, shipping profile, other backend or performance gate is
  claimed. The timings are interval observations with a fixed period of 1.0.
- **Not upscaling.** This is fixed internal-resolution accumulation only, not
  temporal upscaling or MetalFX.
- **Edge-mask method.** The metrics use a temporal-off edge mask (threshold 24,
  dilated 1 px) and are observations. Low-contrast edges can fall into
  "interior".
- **Inspection scale.** I judged the PNGs at full frame, without magnified
  crops. Shimmer was judged from the activity maps and statistics, because a
  single PNG cannot show temporal flicker.
- **Fixed scenes.** Each case used one config and one scene composition. Other
  content (foliage density, HDR highlights, fast camera cuts without a host
  signal) is untested.
- **Root's decisions.** Root decides what follows: the re-review grant after
  fixes, retention of the evidence directory, and any phase gate.
