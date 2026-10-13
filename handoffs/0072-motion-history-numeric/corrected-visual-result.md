# 0072 corrected-source temporal review — result (executed)

## Verdicts

**Numerical: PASS.**

- **Regression.** The exact fixed-temporal regression ran once on root's frozen
  same-source library binary and passed: 1 passed, 162 filtered out, 0.79 s,
  14 submissions. Its full name is
  `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion`.
- **Review cases.** All seven pass every contract check.
- **Integrity.** `analyze.py check` reports no problems. Every log, report and
  PNG matches its ledger hash, and the summary binds the current grants, build
  record, lease, ledger, debits and release latch.
- **Spend.** 93 submissions (14 numeric + 70 world frames + 9 UI) against the
  ceiling of 128, and 14 PNGs against 16. That is exactly the plan, with no
  failure, retry or stop.

**Appearance: FAIL for default fixed-mode quality, but partly fixed.** I compared
every image with the original rejection (`temporal-visual-result.md`).

*Fixed:*
- **Overlays.** BLEND glass, sprites and world text no longer flicker. Pixels
  with an edge peak change of 33 steps or more dropped from 4,568 to 505, and
  the per-frame maximum from 243 to 33.
- **Interiors and shadow edges.** Both now converge: interior pixels changed per
  frame dropped from 2,706–7,021 to 31–97.
- **Registration.** Presentation is now within 0.1 px of the unjittered
  reference.

*Still failing:*
- **Silhouettes, thin geometry and MASK.** Every silhouette, thin opaque
  feature and the one-texel MASK grille still presents the current phase's
  single sample, as aliased as temporal-off, and changes every frame.
- **Peaks lower, but not fewer.** Pixels with a peak change of 33 steps or more
  are no fewer (11,143 → 12,061). Only the amplitude fell (per-frame maximum
  169 → 99–121).

*Still correct:*
- no ghosting in disocclusion, pan, skinned motion or the cut;
- epochs, phases and ownership through the cut and DRS;
- byte-identical UI.

**Recommendation.** Keep temporal accumulation opt-in. The cause is a design
property of the resolve's history admission, not a tuning value. A repair brief
follows; this frozen run was not changed.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max, as this session reports them.
- Grant holder: "Claude Opus 5.5 Max, explicitly verified through ACP,
  handoff0072".

**Brief applied:** the priority revision "execute corrected fixed-temporal
review now". Specifically:
- There was no build or target use. Only the frozen binaries ran, including
  root's named library binary; no libtests were built.
- The run was foreground, serial and once, in the prescribed order, with
  `python3 -I -B`.
- I made no retry and no policy, source or fixture change.
- There was no desktop capture, window, device audio or memory write.
- I made no push or PR. The previous failed run is untouched.

## Identity

| Item | Value |
|---|---|
| Source | Commit `535003d766bcc1386fd2d2f454c83808a0fcc4c8`. Freeze `bd8295b0eb3174b96eb5ff73b0e0e7c464c6c5d620fd66df37bd59d3eafe56a0` (2,449 rows). Exact coverage of **2,448 build inputs**: `build/source.json` `ed56d186…`, tree `e98b2484…`, `Cargo.lock` `68c187db…`. |
| Visual tree | `53fd64b6ec2ddaef1cd1802e04411d7672d06c5e`. Bound in the analyzer summary; unchanged at commit time. |
| Grants | `build-ready.json` `1a1b226d…`; `review-ready.json` `4fa5101f64cadaf4e16b402afc3165e9af4080c5c7a214993e57a112f6e2c7c0` |
| Build record and root review | `build/review.json` `eebd5b3be9e1233bc6ba9273323e67408e8cb9f051b33e45267f3e30151d99a9`; root build review `cc962b22a9c5c46908235bb7256614dd0cb8c94a27e7a5f52943a1ae6eb53832` |
| Release pins | 0069 `release.json` `a3b090e2…` (released 08:42:26Z, 8 children reaped); build lease `ef5886c0…` (released 08:37:06Z) |
| Review binary | `temporal_visual_review` `3c28ec3ee04c60969f13c772a98a854e7c4d4e5ebab662d0a406ecf53e03b5d8`. 18,214,488 bytes, mode 0555; features `default`, `physics-deterministic`. |
| Numeric binary (root-frozen) | `incant_render-90f6082e9b891519` `865b886352d9ec131d951e08a6a62d2b09ada4b0d666b23aa7540845adc80755`, mode 0755 |
| Strict Clippy (root) | Exit 0 (`build/lint.json` `13742464…`). The original unused `mut` is gone. |
| Toolchain and host | rustc/cargo 1.99.0; macOS 27.0.1 (26A434), Mac17,9, Apple M5 Pro |
| Adapter (recorded by the binaries) | **Apple M5 Pro, Metal, IntegratedGpu**. The required backend, Metal, was enforced. |
| Config (explicit, validated) | history 0.9, relative depth 0.05, absolute depth 0.02, reactive threshold 4.0, strength 0.5. These match the original review. |
| Comparability | The fixture differs from the original `9009f088` only at line 1418 (`let report`). The cases, frames, UI counts and PNG names are identical. **All five temporal-off reference PNGs are byte-identical** between the runs. |

## Execution

The GPU children ran from 08:44:42.777Z to 08:46:02.293Z, all in the
foreground, serially, under the global `native-gpu.lock`.

| Step (pid) | Result |
|---|---|
| `gpu.py list lib` (5603) | 40 tests listed; the exact numeric name is present once |
| `gpu.py numeric` (5639) | **1 passed, 0 failed, 162 filtered out, 0.79 s**; debit 14; contract `exact_test_ran_once_and_passed` |
| `gpu.py list review` (5660) | 7 cases, matching the plan |
| `review probe` (5667) | Pass: 0 frames |
| `review stationary` (5710) | Pass: 17 frames, 3 PNGs |
| `review motion` (5809) | Pass: 16 frames, 4 PNGs |
| `review excluded` (5936) | Pass: 9 frames + 9 UI, 3 PNGs |
| `review transitions` (6005) | Pass: 13 frames, 2 PNGs |
| `review skinned` (6039) | Pass: 7 frames, 2 PNGs |
| `review timing` (6046) | Pass: 8 frames, 0 PNGs |
| `gpu.py release` (08:46:07.447Z) | 12 children (root's 2 builds + 10 GPU). Debited 84 frames (14 numeric + 70) + 9 UI = **93**, and **14 PNGs** at 960×540. No stop latched; no owned or live process; no debit without a ledger row. Ledger `32601b51…`, debits `fa353361…`, latch `3c54be77…`. |
| `analyze.py check` | **pass**, no problems. Summary `90d8ef0c…` (2026-10-13T08:46:13Z). |
| `analyze.py publish` | Published 14 screenshots and 49 evidence files plus `manifest.json` `84093bbf…` |

Every contract check is true in every report:
- **probe:** invalid configs rejected; review config validates; second enable
  idempotent; pipelines enabled.
- **stationary:** one uninterrupted run over two cycles; reference has no
  temporal state.
- **motion:** object and pan runs uninterrupted; references have no temporal
  state.
- **excluded:** blend, sprites and text present; accumulation uninterrupted; UI
  stable across phases and off.
- **transitions:** the cut advances the epoch and restarts at phase 0; half
  scale and output scale restart history; one owner throughout.
- **skinned:** run uninterrupted.
- **timing:** one sample per frame; the Models span grows.
- **All cases:** `frames_match_plan`.

## Appearance findings

I personally inspected all 14 PNGs at full frame, plus 3× crops of the grille,
posts, sphere and skinned junction. I also read every report.

**Method.** The tables below use two kinds of figures:
- The fixture's own statistics: max-channel sRGB steps, with an edge mask from
  the temporal-off reference (threshold 24, dilated one pixel).
- Three CPU-only scripts I ran on the published PNGs of both runs:
  `temporal-corrected-inspection/`, with their outputs alongside.

### 1. Stationary convergence — FAIL: silhouettes, thin geometry and MASK still shimmer

Screenshots: `stationary-off.png`, `stationary-on-f16.png` and
`stationary-activity.png` (peak consecutive change over frames 9–16, ×4).

**Fixture statistics** (34,327 edge pixels; consecutive frames, two cycles):

| Metric | Original | Corrected |
|---|---|---|
| Edge pixels changing more than 2 steps per frame | 7,389–14,606 | 13,490–20,051 |
| Edge mean per frame | 13.3–24.9 | 10.6–16.3 |
| Edge max per frame | 169 | 99–121 |
| Interior pixels changing per frame | 2,706–7,021 | **31–97** (665 on the first frame with history) |
| Interior max per frame | 103–123 | 74–94 |
| Same phase f8 vs f16: edge / interior changed | 614 (max 6) / 1 (max 3) | 889 (max 10) / 0 (max 2) |
| f16 vs off: edge mean / interior mean | 9.15 / 0.213 | 13.68 / 0.226 |

**Activity map** (frames 9–16; pixels by peak consecutive change, edge /
interior):

| Peak change (steps) | Original | Corrected |
|---|---|---|
| 3–8 | 2,666 / 9,546 | 4,275 / 90 |
| 9–16 | 3,222 / 1,367 | 2,089 / 14 |
| 17–32 | 2,684 / 7 | 2,815 / 3 |
| 33–63 | 897 / 0 | 8,419 / 11 |
| 64 or more (saturated) | 10,234 / 12 | 3,620 / 11 |
| **33 or more, total** | **11,143** | **12,061** |

**What the fix achieved:**
- The original map showed grey bands on every cast shadow, plus faint activity
  on the cube faces, the sphere body and the ground. All of that is gone.
- Interiors converge into a stable, phase-locked state.

**What remains:**
- Every silhouette keeps a bright one-pixel rim: the horizon, all 19 fence
  posts, both cubes, the wall, the full sphere outline and the thin ramp post.
- **The whole MASK grille is still saturated noise.**
- At 3× (off | original | corrected), the sphere and post silhouettes in
  corrected f16 are the same single-sample stair-steps as temporal-off. The
  grille is aliased in every frame.
- So the per-frame change is not convergence of anti-aliasing. It is the raw
  per-phase sample, now displaced by the presentation's bilinear un-jitter.
  That bilinear blend of neighbouring samples likely explains the lower peak,
  from 169 to about 100–121 steps (inference).
- The number of pixels flickering by 33 steps or more did not fall.

**Cause.** Read from `crates/incant_render/src/temporal/resolve.wgsl` at
`535003d7`, and consistent with these pixels:
- **Same-surface admission.** Line 50 admits a history tap only when
  `compatible(previous_depth[tap], m.z)` holds. `m.z` is the previous depth
  expected for *the current sample's own surface*.
- **The current surface alternates.** At a static silhouette, the surface under
  a pixel centre alternates with the jitter phase. History from the other
  surface is always rejected.
- **History stays single-surface.** The history that is admitted was itself
  built only from samples of that same surface. Shading accumulates, but
  coverage never blends.
- **Clear pixels are excluded.** Line 31 returns current-only for clear
  (`a < 1`) pixels, and line 50 rejects history with `a != 1`. So opaque-to-clear
  edges (the horizon) cannot accumulate fractional coverage either.
- **Already anticipated.** Astra's `docs/spikes/temporal-edge-correctness.md`
  (lines 24–26 at `535003d7`) says this conservative policy "can still leave
  phase-dependent coverage at subpixel silhouettes". These pixels show that it
  does, at full frame.

### 2. Excluded surfaces — PASS: overlay jitter fixed; UI stable

Screenshots: `excluded-on-f8.png`, `excluded-off.png` and
`excluded-activity.png` (frames 2–8).

| Metric (32,592 edge pixels) | Original | Corrected |
|---|---|---|
| Edge mean / max per frame | 5.6–10.8 / 243 | **0.8–2.0 / 33** |
| f8 vs off: edge mean / max | 5.39 / 243 | 3.69 / 41 |
| Activity: edge pixels with a peak of 33 steps or more | 4,568 (2,010 saturated) | **505** (none saturated) |
| Activity: edge pixels with a peak of 17–32 steps | 4,124 | 49 |
| UI vs temporal-off, all 8 phases | byte-identical | byte-identical |

**What changed:**
- The glass outline, every sprite texel edge and every world-text glyph
  (including the uncovered `.notdef` box) now show no activity at all.
- At 3× they are crisp, unjittered and stable.

**What remains** (low amplitude): dim nearest-texel checker edges, and the
backdrop's outer silhouette against the clear background (max 33). The second
is the same mechanism as finding 1.

### 3. Object motion and disocclusion — PASS: no ghost; the small moving-shadow lag is reduced

Screenshots: `motion-object-on-f8.png` and `motion-object-off-f8.png`.

- **No ghosting.** The wall and ground the cube uncovers are clean, and nothing
  ghosts behind the sphere.
- **Same-method re-measurement.** These are differences from temporal-off at
  pixels at least 3 px (8-connected) from any edge of either image, with edge
  threshold 12:

| Image | Run | 3–8 steps | 9–16 steps | More than 16 | Max |
|---|---|---|---|---|---|
| Stationary f16 (baseline) | original / corrected | 600 / 559 | 0 / 0 | 0 / 0 | 7 / 7 |
| Object f8 | original / corrected | 918 / 823 | 45 / **30** | 0 / 0 | 11 / 10 |
| Pan f6 | original / corrected | 843 / **240** | 4 / 0 | 0 / 0 | 11 / 7 |
| Skinned f6 | original / corrected | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |

These counts differ from the original document's table, because that used only
the off image's edges. Here both runs are measured the same way.

**Shadow lag.** All 30 of the 9–16-step pixels lie in x 579–600, y 291–305. That
is the same patch as before: the cube's moving sun shadow inside the cool
point-light pool. It is up to 10 steps, so not visible at 1:1, but measurable.

**Edge differences.** Differences right at edges against the aliased reference
grew (object f8 vs off: edge pixels changed 8,622 → 20,274; mean 9.1 → 13.7).
That is the softer, registered reconstruction, not a trail.

### 4. Camera pan — PASS: no smear

Screenshots: `motion-pan-on-f6.png` and `motion-pan-off-f6.png`.
- There is no trailing on the posts, the sphere or the grille.
- Off-edge differences fell to 240 small pixels, with none at 9 steps or more
  (table above).
- The silhouette shimmer from finding 1 persists.

### 5. Skinned motion — PASS: clean

Screenshots: `skinned-on-f6.png` and `skinned-off-f6.png`.
- **No ghost or lag.** No pixel 3 px or more from an edge differs, and I found
  no ghost or lag from the deformed vertices.
- **Near-edge differences.** The fixture's interior count is 235 pixels, max 22
  (originally 24, max 23). All of these lie within 2 px of edges.
- **A one-pixel step, in both runs.** At 4× there is a one-pixel step in the
  checker's horizontal boundary, just right of the moving hypotenuse (about
  x 480–565, y 269–270). The original capture shows the same step. It is a
  texture-edge registration difference where current-only pixels meet
  accumulated ones, not a trail of the triangle.

### 6. Reset, cut and DRS transitions — PASS

**Sequence.** The epochs are 1 / 2 / 3 / 4 for view A, the cut, half-scale DRS
(internal 480×270) and the return to output scale. Phase restarts at 0 after
each reset, and one owner holds throughout.

**Cut frame.** `transition-cut-f7.png` shows view B with no trace of view A.
Against temporal-off view B:

| Metric | Original | Corrected |
|---|---|---|
| Interior pixels changed | 2,358 | 862 |
| Edge mean | 11.6 | 14.0 |

**Interior maximum of 49 steps.** It sits within 2 px of an edge: comparing the
two runs' cut frames, no pixel away from an edge differs by 25 steps or more. The
returned frame (f12) has identical statistics.

**DRS frame.** `transition-drs-f9.png` is blocky with nearest upscale, and the
grille breaks into an aliased pattern. That is expected of the DRS output path,
not temporal. Consecutive DRS edge means fell from 33.0/32.7 to 22.9/15.7.

### 7. Registration — PASS (new measurement)

I used a global Lucas–Kanade translation against the unjittered reference, over
about 14,000–18,300 strong-gradient pixels:

| Frame (jitter phase, offset) | Original | Corrected |
|---|---|---|
| stationary f16 (7, +0.125/−0.125) | (+0.12, −0.14) | (−0.02, −0.09) |
| object f8 (7) | (+0.12, −0.14) | (−0.03, −0.09) |
| skinned f6 (5, −0.125/−0.375) | (−0.10, −0.28) | (+0.01, +0.10) |
| excluded f8 (7) | (+0.10, −0.09) | (−0.01, +0.00) |

- **Original:** the output sat about one current-jitter offset away from the
  reference.
- **Corrected:** within 0.10 px. The presentation un-jitter works.

This corrects a doubt in my working notes. A residual-offset defect I suspected
from a cruder edge-crossing measurement is not supported, so I report no
registration defect.

### 8. Timing — observation only

Each frame produced exactly one sample, with none skipped, abandoned or pending.
Median group spans, temporal off against on:

| Group | Off | On | Change | Original change |
|---|---|---|---|---|
| Models | 0.816 ms | 1.661 ms | **×2.04** | ×1.89 |
| Shadows | 0.176 ms | 0.175 ms | unchanged | unchanged |
| AssignLights | 0.077 ms | 0.076 ms | unchanged | unchanged |
| InitializeScene | 0.024 ms | 0.023 ms | unchanged | unchanged |

**Caveats:**
- **Display.** Display's growth (0.84 → 1.66 ms) is group overlap, not Display
  cost.
- **Out-of-frame start.** `groups_within_frame` is false, as before.
- **Not a performance measurement.** The timestamp period is 1.0, and there is
  no performance claim.

## Repair brief (fixed mode; for Astra, the implementation owner)

These are concrete findings only. I changed no engine source, policy value or
fixture, and this frozen run stays as recorded.

1. **Accumulate coverage at silhouettes (priority).** This is in
   `temporal/resolve.wgsl` lines 31 and 41–55.
   - **Admit history by neighbourhood depth range, not per-surface identity.**
     Compute the min and max view depth of the expected previous depths
     (`m.z`) over the current 3×3, widened by the existing absolute and
     relative tolerances. Admit a positive-weight tap when its previous depth
     lies in that range. At a silhouette the 3×3 holds both surfaces, so both
     surfaces' history survives and coverage blends. At a true disocclusion
     the occluder's depth lies outside the range and is rejected, as now.
     Keep renormalization over admitted taps.
   - **Treat clear as a surface at the far plane.** Accumulate premultiplied
     RGBA, including alpha. Stop returning current-only for `a < 1` (line 31)
     and stop rejecting history with `a != 1` (line 50). A pixel whose whole 3×3
     is clear still rejects geometry-depth history, so newly empty pixels are
     not filled. `copy.wgsl` already reconstructs premultiplied coverage.
   - **Fetch history with the closest-depth 3×3 motion (motion dilation).**
     Then a moving silhouette reprojects with its foreground motion.
   - **Keep the colour clamp as the ghost guard.** It already spans both
     surfaces at an edge. Consider variance clipping in YCoCg (mean ± γσ) if
     residual edge flicker stays high.
   - **Expected result.** A converged edge pixel presents its coverage blend.
     It moves per frame by at most about (1 − history weight), 10%, of its
     linear contrast, instead of the full per-phase sample.
   - **The same pattern exists in upscaling.** The upscaling resolve applies the
     same per-tap admission (`upscaling/resolve.wgsl:52` at `535003d7`). That
     appearance belongs to 0073 and was not reviewed here.
2. **Add numeric guards before the next GPU review.** The current numeric
   regression passed while silhouettes failed, so add CPU oracle and device
   cases for:
   - **A static depth-discontinuity edge** with a gap far beyond the tolerance.
     Over 16 frames (two cycles), edge pixels must end strictly between the
     foreground and background. Their per-frame change over frames 9–16 must be
     at most (1 − w)·|F − B| + ε in scene-linear.
   - **An opaque-to-clear edge** with the same assertion on premultiplied RGBA.
   - **A one-texel MASK grille** with the same assertion.
   - **A moving occluder.** Uncovered pixels reach the background within 2
     frames, and pixels 2 px or more behind the trailing edge are never
     filled.
   - **The existing regression** must still pass unchanged.
3. **Moving-shadow lag (minor).** The lag is up to 10 steps over 30 pixels.
   - The cause: the reaction uses an absolute scene-linear threshold of 4.0, so
     shadow-sized changes barely engage it.
   - A luminance-relative change measure, `|h − c| / (luma(c) + k)`, would
     react to shadow-sized changes. It must not bring back texture-edge
     flicker.
4. **Re-review on the same fixture, plan, policy and seven cases**, with a new
   evidence namespace and only the source changed. These are proposed
   acceptance criteria, as reviewer judgement and not plan gates:
   - **Stationary:**
     - pixels with a peak change of 33 steps or more over frames 9–16: at most
       3,000 (from 12,061);
     - saturated pixels: at most 100 (from 3,620);
     - consecutive-frame edge mean at most 5 (from 10.6–16.3) and edge max at
       most 50 (from 99–121);
     - no saturated activity inside the grille;
     - silhouettes at 3× show intermediate coverage values rather than the
       temporal-off stair-steps;
     - interiors stay at 100 or fewer changed pixels per frame, and same-phase
       f8/f16 interior stays at 0.
   - **Everything else no worse than this run:**
     - object off-edge: at most 30 pixels at 9–16 steps and none above 16;
     - pan: none at 9 steps or more;
     - skinned: 0;
     - excluded: at most 505 edge pixels with a peak of 33 steps or more, and
       UI byte-identical;
     - transitions: epochs, phases and owner as now;
     - registration within 0.1 px.

## Files

**New in this commit** (all in `handoffs/0072-motion-history-numeric/`):
- `corrected-visual-result.md`: this document.
- `temporal-corrected-screenshots/`: 14 PNGs, 960×540, published by
  `analyze.py publish`:

  | Screenshot | SHA-256 prefix |
  |---|---|
  | `stationary-off` | `2485512777e5` |
  | `stationary-on-f16` | `ee1f84cca949` |
  | `stationary-activity` | `525b2388c73b` |
  | `motion-object-off-f8` | `212ac1a0e0af` |
  | `motion-object-on-f8` | `70166cf2ed3d` |
  | `motion-pan-off-f6` | `30aa3e196bca` |
  | `motion-pan-on-f6` | `9312305a053a` |
  | `excluded-off` | `d1592ae5ed1f` |
  | `excluded-on-f8` | `3f44a6612f39` |
  | `excluded-activity` | `47cd1f9e3736` |
  | `transition-cut-f7` | `5bc0618bc5a5` |
  | `transition-drs-f9` | `89097514f9ef` |
  | `skinned-off-f6` | `9dc53d04deeb` |
  | `skinned-on-f6` | `99613fd3a9e0` |

  The five `-off` references are byte-identical to the original run's.
- `temporal-corrected-evidence/`: 50 files, about 0.9 MB, copied unchanged from
  `~/.codex/incant-evidence/0072-temporal-corrected-001`. `manifest.json`
  (`84093bbf…`) gives each file's source path and SHA-256. Included:
  - both grants and the released lease;
  - the GPU ledger, the debits and the release latch;
  - the analysis summary;
  - the build records (`review.json`, `source.json`, `toolchain.json`,
    `lint.json`);
  - the seven reports and event logs;
  - the stdout and stderr of all 10 GPU children and root's 2 build children.
- `temporal-corrected-inspection/`: my three CPU-only inspection scripts and
  their outputs (`activity_histogram`, `off_edge_differences`,
  `registration`, each `.py` and `.txt`). They read only published PNGs, using
  the 0067 `pngio.py` reader read-only.

**Unchanged:**
- `result.md`, `preflight-result.md`, `temporal-result.md`,
  `preparation-result.md`, `temporal-visual-result.md` and
  `corrected-preparation-result.md`;
- `temporal-visual-screenshots/` and `temporal-visual-evidence/`;
- `visual/` (tree `53fd64b6…`), `tools/` and `brief.md`.

The original evidence directory, `~/.codex/incant-evidence/0072-temporal-visual`,
has no file newer than this run's grant.

## Commands run (this session; all exit 0)

```
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py list lib
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py numeric
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py list review
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py review probe
  ... review stationary | motion | excluded | transitions | skinned | timing   (one each, in that order)
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py release
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/analyze.py check
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/analyze.py publish
/opt/homebrew/bin/python3 -I -B handoffs/0072-motion-history-numeric/temporal-corrected-inspection/<script>.py \
    handoffs/0067-dynamic-resolution/tools handoffs/0072-motion-history-numeric   (CPU only, three scripts)
```

Before the first child, I verified:
- the grant and its SHA;
- the holder;
- the build record, root review and release pins;
- the review and numeric binaries (SHA, mode, executable bit, exact name);
- exact coverage, lint exit 0, the visual tree, and a free global lock.

Unavailable tests: none were required. No other test suite ran, by design.

## Limitations

- **One host and backend.** One local desktop on one adapter (Apple M5 Pro,
  Metal, macOS 27.0.1). No phone, shipping profile, other backend,
  performance or phase-gate claim is made. Timings are interval observations
  with a fixed period of 1.0.
- **Fixed mode only.** This is the default portable internal-resolution
  accumulation. Temporal upscaling belongs to 0073 and native MetalFX to 0074;
  neither is judged here.
- **Metrics are observations.** The fixture's figures come from its own edge
  mask; my figures come from my scripts. Low-contrast edges can fall into
  "interior". Flicker was judged from the activity maps and statistics, because
  a single PNG cannot show it. Crops were magnified only for inspection and are
  not committed.
- **Fixed scenes.** Each case used one config and one scene composition. Other
  content (dense foliage, HDR highlights, fast cuts without a host signal) is
  untested.
- **The acceptance criteria above are proposals.** Root decides what follows:
  the repair, a re-review grant, retention of the evidence directory, and any
  phase gate. This result approves nothing.
