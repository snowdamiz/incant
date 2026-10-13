# 0070 SSAO filter comparison (ssao-filter-001): GPU result

## Status

**Halted at child 8 of 9 by a defect in my own helper's guard; GPU released.**

**This run:**

- **Ran:** the three controls and the first four of the five numeric cases,
  each once, serially, in the foreground. All seven succeeded, between
  12:36:15Z and 12:36:57Z, on Metal (Apple M5 Pro).
- **Failed:** `case-ssao_quality` exited 1, with all 10 frames and both stage
  captures retained. Its verdict failed a precondition I wrote into the
  helper: "the sphere covers its analytic silhouette area within 10%".
  - **The guard counted the fractional viewport's 601 partly covered rim
    texels.** It found 3,419 and 1,055 texels against 2,824.6 and 455.2
    expected.
  - **The device is not at fault.** Counting only wholly covered texels gives
    2,818 and 454, off by −0.2% and −0.3%. The analyzer's own sphere gate on
    the same frames records exactly these values and passes.
- **Not run:** under the brief's halt rule, device work stopped, so
  `appearance` (child 9) never ran.
  - **None of the 16 PNGs or 11 pairs exists,** so appearance is not judged.
  - Nothing was retried, and no tool, threshold or source was edited.
- **Released:** I released the GPU through the frozen runner at 12:37:12Z. No
  owned or frozen-binary process remains. The namespace stays latched on
  `case-ssao_quality`, as designed.

**Analysis after release:** `analyze_filter.py` exited 1, which the brief
anticipated.

- **Numeric contract:** all 43 gates pass over the retained frames of all five
  cases.
- **Quality:** fails exactly as the CPU model predicted.
  - **Fail:** N6 authored, 7.00/255 against 1/64; N8 perspective, 0.376
    against 0.25.
  - **Pass:** N7 at 0/0/0 and N8 orthographic at 0.176.
  - **Diagnostic:** N6T is 3.37/255.
  - The device reproduces the model on every quality value.

**Proposed repairs (none applied):**

- **Run contract:** a one-predicate fix in the helper's `sphere_texels`.
- **N8 perspective:** two more bounded CPU candidates also fail (0.439 and
  0.314 at the ROI). The smallest repair that might pass is about four times
  the AO probe loop, and that figure is extrapolated, not measured. The
  decision is root's.
- **N6 authored:** a defect in the metric, not something the renderer can fix.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**, as the wrapper requires. ACP verification is root's.

## Exact identities

| Item | Value |
|---|---|
| Source HEAD | `880d6c6bc02f03563ae334125611685ba444bc83`: my 5f516ec0 helpers plus engine `5dfc522ec164f2bc50d760f12db57eae233f33d8`. Root's `brief.md` stayed modified and uncommitted throughout |
| Plan | `ssao-filter-001/plan.json`, sha256 `5bdceae4…`, equal to the grant's |
| Helper digest | `990dfd7e…`, 26 files |
| GPU grant | `gpu-grant.json`, sha256 `2047f97c…`; Metal; issued 12:34:33Z |
| Root source review | `0070-filter001-helper-root-20261013.json`, `212ceccc…` |
| Build | Grants `build-grant.json` (`6b0ac1af…`) and `build-grant-002.json` (`1929a2bd…`). A Cargo-config refusal was recorded before any child ran (`refusals.jsonl`). Record `build/build.json` (`38c06d0b…`); release `build-released.json` (`de7bedce…`): 5 children, each exit 0 and reaped |
| Root build review | `build-review-root.json`, `59e2096b…`, no findings |
| Frozen binaries | `effects_gpu` `de30ad74…`, `lib` `4614a993…`, `lib-cooked` `932694c6…`. Each was hash-checked at reservation and unchanged at completion |
| Features | `lib` and the helper's `incant_render`: default, physics-deterministic, ssao-stage-readback. `lib-cooked` adds cooked-geometry, cooked-materials and cooked-output |
| Adapter | Apple M5 Pro, Metal, IntegratedGpu: a desktop, not a reference phone |
| Ledger | `ledger.jsonl`, `d040d54a…`, equal to the release record's |
| Release | `released.json`, `ec52f60f…`, 12:37:12Z |
| Analysis | `filter-analysis.json`, `3eb3c878…` |
| Root diagnostic | `sphere-count-diagnostic-root.json`, `4ef86898…`: root's offline count after release |
| Evidence index | `evidence/ssao-filter-001/index.json`. `make_index.py` computes every hash in it from the evidence directory |

The evidence directory is `/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001`.

## Device sequence

**How each child ran:**

- One `filter_runner.py` invocation per child, under the shared GPU lock.
- Exact environment: `INCANT_EXPECT_GPU_BACKEND`, `LC_ALL`, `PATH`,
  `RUST_BACKTRACE` and `TMPDIR`.
- Every child was reaped, with no signal, no timeout and no surviving group
  member.

| # | Child | PID | Submissions | Wall (s) | Exit | Result |
|---|---|---|---|---|---|---|
| 1 | control `filter_device` (lib) | 48157 | 2 | 0.76 | 0 | 1 test passed |
| 2 | control `ao_device` (lib) | 48202 | 7 | 0.70 | 0 | 1 test passed |
| 3 | control `ao_device_cooked` (lib-cooked) | 48248 | 0 | 0.65 | 0 | 1 test passed: typed refusal before any submission |
| 4 | case `ssao_core` | 48305 | 20 | 6.91 | 0 | verdict ok; 20 frames, 4 with stage planes |
| 5 | case `ssao_materials` | 48453 | 10 | 3.35 | 0 | verdict ok; 10 frames |
| 6 | case `ssao_geometry` | 48532 | 17 | 4.47 | 0 | verdict ok; 17 frames, 9 with stage planes |
| 7 | case `lifecycle_drs` | 48658 | 18 | 2.61 | 0 | verdict ok; 17 frames (`l_invalid_viewport` retains none) |
| 8 | case `ssao_quality` | 48736 | 10 | 3.35 | **1** | **verdict failed**; 10 frames, 2 with stage planes, all retained |
| 9 | `appearance` | — | (16) | — | — | **not run**: any child failure halts device work |

**Budgets:**

| Item | Used | Planned or allowed |
|---|---|---|
| Children | 8 | 9 |
| Submissions | 84 | 100 planned, 128 cap |
| PNGs | 0 | 16 |
| Pairs | 0 | 11 |
| Retries | 0 | — |

The children took 22.8 s of wall time in total.

## The run-contract failure

**Exact error**, latched by the runner:

```
case-ssao_quality: exit status 1; no valid verdict:
  perspective: the sphere covers its analytic silhouette area within 10%    texels 3419, expected 2824.635
  orthographic: the sphere covers its analytic silhouette area within 10%   texels 1055, expected 455.221
stderr: verdict failed; every receipt and frame is retained
```

**The case's seven other checks passed:**

- the sphere is byte-identical with AO off and on, in both projections;
- the stripes darken the receiver only: 17,656 texels darker and 0 brighter
  in perspective, 6,079 and 0 in orthographic;
- no wgpu validation error;
- neither renderer created a pipeline after setup.

**Cause: my helper, not the device.** The guard is `sphere_texels`, in
`ssao-filter-001/effects/src/numeric.rs:517`.

- **What it counts:** texels whose **centre** lies inside the viewport
  `(13.25, 17.5, 280.5, 160.5)` and whose colour differs from the background
  probe at (16, 20).
- **The fractional edges cut three lines of texels:**
  - columns x 13 and x 293 are 75% covered;
  - row y 17 is 50% covered;
  - the bottom edge, 178.0, falls on a texel boundary, so no bottom row is cut.
- **Why they count:** their centres lie inside the viewport, but the output
  pass blends them with the canvas (31, 63, 95). Against the scene background
  (20, 21, 25) they read (22, 32, 45) on the columns (320 texels) and
  (26, 47, 70) on row 17 (281 texels, corners included).
- **The arithmetic:** 2 × 161 + 281 − 2 = 601 rim texels, all counted.
  3,419 − 601 = 2,818 and 1,055 − 601 = 454.

**Verified offline** on the retained `p_off_sphere` and `o_off_sphere` RGBA,
with no new render:

- A "wholly covered" test admits exactly 2,818 and 454 texels. Their bounding
  boxes are x 124–182, y 68–127 and x 141–165, y 86–109.
- Root's diagnostic records the same counts and colours.
- The analyzer applies the same precondition over its interior domain, which
  is inset 1.5 px (x 15–291, y 19–175) and so excludes the rim by
  construction. It passes, at 2,818 against 2,824.6 and 454 against 455.2.

**Why no test caught it:** the Rust guard covers the whole viewport instead.
The build compiled and linted it, but no fixture exercised it before the
device run.

## Numeric contract (after release)

`analyze_filter.py` passes all 43 numeric gates across the five cases.

**Filtered interval oracle (15 gates):**

- **Frames:** `ssao_core` perspective, orthographic, edge and strength 0.5;
  the nine `ssao_geometry` frames; and the `ssao_quality` perspective and
  orthographic stripes.
- **Agreement:** each agrees on 43,489 of 43,489 interior texels, with 0
  disagreements, 0 out of range and 0 uninferable.
- **Occlusion counts:** the certainly occluded counts equal the oracle's, for
  example 11,013 for `white_slab` perspective and 17,959 for the perspective
  stripes.

**Stage planes (15 gates):** no texel falls outside the oracle at the raw, H,
V, final, clear or forward stage, over 45,241 viewport texels each.

**R8 rounding, measured:**

- 676,262 raw texels were exactly inferable, with residuals in
  [−0.4375, +0.5].
- All 4,388 ties at 127.5 stored 128. That excludes truncation and
  round-half-down.

**Identity (9 gates):** 0 differing pixels in each.

- three determinism pairs;
- four `ssao_core` identities: far, flat, radius 0.5 and reversed;
- the isolated sphere in both projections. Its facet rise of 0.00128 is below
  the 0.01 bias.

**DRS (3 gates):**

- **Nearest expansion:** 0 of 43,489 texels mismatch at 12/16, and 0 of
  43,056 at 8/16.
- **Bloom halo centroid on the physical emitter: passes.** Expectation and
  tolerances are the same as build003's, where this gate failed.

  | Scale | build003 centroid | This run | Tolerance |
  |---|---|---|---|
  | 16/16 | (186.16, 83.21) | (186.16, 83.21) | 1.5 px |
  | 12/16 | (190.54, 86.47), failed | (186.62, 83.36) | 2.0 px |
  | 8/16 | (181.23, 83.21), failed | (186.77, 83.21) | 3.0 px |

  The expectation is (186.55, 83.06). The engine now includes root's
  d2d2294e ("anchor bloom footprint to physical output space"). Bloom
  appearance was not rendered here.

**Mixed pane (1 gate):**

- median ratio 0.517;
- all 2,454 occluded texels under the pane have scene-radiance intervals that
  contain 1/2;
- outside the pane, the frame is identical to the core render.

These gates show that the device computes R1/R3 exactly as specified. They
say nothing about how the result looks.

## Quality gates

Definitions and bounds are unchanged.

| Gate | Device, this run | CPU model (addendum) | Bound | Result |
|---|---|---|---|---|
| N6 authored, `white_slab` perspective, row 97 | 7.00/255 at x 161 | 7.00 at x 161 | 1/64 (3.98/255) | **Fail** |
| N7: `white_slab` p / o, `edge_slab` p | 0 / 0 / 0 | 0 / 0 / 0 | ≤ 138.2 / 176.7 / 202.7 | Pass |
| N8 perspective, row 124, x 148–158 | 0.376 (12.0 of 31.88) | 0.376 | ≤ 0.25 | **Fail** |
| N8 orthographic, row 110, x 150–156 | 0.176 (14.0 of 79.69) | 0.176 | ≤ 0.25 | Pass |
| N6T (diagnostic, not adopted) | 3.37/255 | 3.37 | 1/64 | Within |

Because the device and the model agree on every value, CPU screening of
further candidates is reliable for these metrics.

## Appearance

**Not rendered, so not judged.** The 16 PNGs and 11 pairs would have come from
the `appearance` child, which the halt rule stopped. With no rendered pairs to
review, I make no appearance claim.

**What I reviewed instead:** seven aids that `review/make_review.py` derived
after release from retained case readbacks.

- They are not renders and not acceptance evidence.
- Each comparison puts build003 on the left and filter-001 on the right, with
  the same fixtures and the same `off` reference.

**What the aids show:**

- **`darkening_white_slab_p_…` and `slab_edge_crop_…`**
  - build003's hard plateaus, its stepped wedge at the lower left and its
    short horizontal hatch are gone.
  - The receiver now darkens in one smooth ramp toward the slab edge.
  - At ×6 gain a faint, soft mottling remains in the outer halo. The ×3
    true-value crop shows it only as a slight unevenness.
- **`darkening_white_slab_o_…`:** the blocky stepped ring around the slab
  becomes a smooth halo with rounded corners.
- **`darkening_edge_slab_p_…`**
  - A smooth ramp replaces the plateaus and hatch.
  - At ×6 gain, soft, low-contrast vertical streaks show in the outer halo,
    without hard edges.
- **`darkening_stripes_p_vs_o_filter001`:** below the stripes, perspective
  keeps a soft vertical comb that fades downward, and orthographic shows a
  weaker one. This matches N8: perspective fails at 0.376 and orthographic
  passes at 0.176.
- **`stripes_p_roi_crop_off_vs_on`:** the uniform white-furnace `off` frame
  against `on`. The comb appears as soft alternating light and dark columns
  under the stripe gaps, around ROI row 124.
- **`sphere_p_off_vs_on`:** byte-identical flat gray disks, as the identity
  gate states.

**My read, from CPU-derived aids only:**

- R3 removes the staircase and hatch artifacts that made build003
  unacceptable as a look.
- The visual issue still open is the soft comb under thin occluders in
  perspective.
- Real rendered pairs are still required before any appearance judgment.

## Proposed repairs

None of these is applied.

### 1. Run contract: the sphere-coverage guard

**Smallest change:** count only texels wholly inside the viewport.

- It changes one predicate and a comment, in the helper only, not in
  production.
- The 10% tolerance and every other check stay the same.

```rust
-/// Viewport texels whose colour differs from the background texel (16, 20).
+/// Texels wholly inside the viewport whose colour differs from the background
+/// texel (16, 20); partly covered rim texels blend with the canvas.
 fn sphere_texels(frame: &Rgba) -> usize {
@@
-            let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
-            cx >= x0 && cy >= y0 && cx < x0 + w && cy < y0 + h && frame.px(x, y) != background
+            let (fx, fy) = (f64::from(x), f64::from(y));
+            fx >= x0 && fy >= y0 && fx + 1. <= x0 + w && fy + 1. <= y0 + h && frame.px(x, y) != background
```

- On the retained frames it gives 2,818 and 454, as shown above.
- A Rust test on a synthetic frame with a blended rim should come with it.

**Rerun:** a new namespace is required.

- This namespace is latched, and the fix changes the helper digest and plan.
  The next device run therefore needs a new root-authorized namespace, build
  and grant.
- **Recommended:** rerun the same frozen sequence: 9 children, 100
  submissions, 16 PNGs and 11 pairs. One build record, one ledger and one
  analyzer pass then cover everything.
- **Not recommended:** rerunning only `ssao_quality` and `appearance`. That
  would need the seven passed children here to count as evidence across two
  builds, and the frozen analyzer reads only one namespace.

### 2. N6 authored: the metric, not the renderer

- **Why it cannot discriminate:** the bound sits at the continuous-sampling
  limit of the same test, which is 3.82/255 after the filter against a bound
  of 3.98. N6 therefore cannot separate banding from the true contact
  gradient, as the addendum showed.
- **The device value is the gradient:** 7.00 at x 161 matches the design's
  converged contact gradient, 7.09 at x 162. It is not banding.
- **Replacement:** N6T, my proposed replacement, reads 3.37 on the device.
  Adopting it is root's decision, and until then N6 stays failed.

### 3. N8 perspective: no small repair found

**Cause** (from the addendum, now confirmed on the device):

- The design's step phase `t = (33 + 2b)/64` probes only the upper half of
  each step stratum.
- Its converged expectation therefore keeps a comb: 15.80/255 at the ROI,
  against 4.81 for the continuous limit.

**Six configurations:** the device-measured design and five bounded candidates
from the CPU model. All keep the 16-direction kernel and the binary test.

| Configuration | Extra cost | ROI row 124 | Rows 125–130 | N6T p | N7 |
|---|---|---|---|---|---|
| Design (device) | — | **0.376** | 0.439 0.502 0.533 0.356 0.335 0.272 (model) | 3.37 | 0/0/0 |
| Full-stratum phase, shared per texel (addendum) | none | 0.471 | — | 3.39 | 0/0/0 |
| 8×8 Bayer, full stratum, 9-tap filter (addendum) | +40 B/px; filter ALU about ×1.8 | 0.314 | — | 2.44 | 0/0/0 |
| 8 probes per ray, upper half (addendum) | AO pass about ×2 | 0.220 | 0.282 0.376 0.408 0.293 0.230 0.146 | 5.37 | 0/0/0 |
| Per-ray full-stratum phase, `t_i = frac((b + 0.5)/16 + i/16)` (this run) | about 16 adds per texel | 0.439 | 0.471 0.471 0.439 0.314 0.272 0.209 | — | 0/0/0 |
| 8 probes per ray, full-stratum phase (this run) | AO pass about ×2 | 0.314 | 0.345 0.345 0.345 0.188 0.209 0.188 | 3.76 | 0/0/0 |

**This run's two checks:**

- **Harness self-check:** `steps8_full_stratum.py` reproduces the addendum's
  design and 8-probe rows exactly.
- **Orthographic N8:** still passes for both candidates, at 0.151 and 0.075.
- **N6 authored:** 6.0 and 7.0/255.

**Math:**

- **What remains:** with a full-stratum phase the probe positions are
  unbiased. What remains is sampling error, which the 5-tap filter only
  partly removes.
- **Excess over the continuous limit (0.151 at the ROI):** it fell from
  0.320 with 4 probes to 0.163 with 8. That is roughly halving with doubled
  probes, as stratified sampling of a step-like visibility function predicts.
  The split is a heuristic, because peak-to-peak values do not add.
- **What passing would need:** an excess of 0.10 or less. Extrapolated, not
  measured, that is about 16 probes per ray. It would cost roughly four times
  the AO probe loop, with robustness across rows unknown.
- **Verdict:** none of this is small. Upper-half 8 probes passes only at the
  ROI row and fails four of the next six.

**Concrete next step, root's choice:**

- **(a)** Keep R1/R3 with N8 perspective recorded as failed, as a documented
  fixed-kernel limitation for thin occluders.
  - The shipped hard comb of 32–48 codes becomes a soft modulation: 12 codes
    at the ROI on the device, and 12–17 on rows 124–130 in the model.
  - The bound does not change.
- **(b)** Authorize one CPU-only screen of 16 probes per ray with the
  full-stratum phase.
  - It would score ROI rows 124–130, N6T and N7 with the same harness, in
    about 20 s.
  - A pass there would still need the reference-phone cost measurement before
    any adoption.

Either way, the next GPU step is the device rerun from repair 1, so that the 16
appearance PNGs can be reviewed.

## Accepted scope

**Accepted here, and only this:** on Apple M5 Pro / Metal, from engine
`5dfc522e` with the frozen binaries above, the R1/R3 device output meets the
numeric contract's 43 gates. Those gates cover:

- interval-oracle agreement and every stage plane;
- the measured R8 round-half-up;
- the identities;
- DRS expansion and the DRS bloom centroid;
- the mixed pane.

**Not accepted:**

- **Appearance:** never rendered.
- **Quality:** N6 authored and N8 perspective fail.
- **This run's contract:** `ssao_quality` is latched failed in the ledger,
  whatever the cause.
- **Gates and claims:** no phase gate, cooked-materials, default-on or
  reference-phone performance claim.

Desktop results do not satisfy phone gates, and test counts grant nothing.

## Screenshots

There are none, because the appearance child did not run. The derived review
aids (not renders) are:

- `evidence/ssao-filter-001/review/slab_edge_crop_build003_vs_filter001.png`
- `evidence/ssao-filter-001/review/darkening_white_slab_p_build003_vs_filter001.png`
- `evidence/ssao-filter-001/review/darkening_white_slab_o_build003_vs_filter001.png`
- `evidence/ssao-filter-001/review/darkening_edge_slab_p_build003_vs_filter001.png`
- `evidence/ssao-filter-001/review/darkening_stripes_p_vs_o_filter001.png`
- `evidence/ssao-filter-001/review/stripes_p_roi_crop_off_vs_on.png`
- `evidence/ssao-filter-001/review/sphere_p_off_vs_on.png`

`review/manifest.json` records their sources and hashes.

## Run commands and results

Every command ran from the worktree root with
`R = env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`.
`E` is the evidence directory.

| Command | Result |
|---|---|
| Preflight: plan sha, helper digest, engine commit, frozen binaries, helper lock and source freeze against the grant | All equal; runner status showed 0 children, not released |
| `R …/ssao-filter-001/filter_runner.py control --grant E/gpu-grant.json filter_device` | exit 0 |
| `… control … ao_device`, then `… control … ao_device_cooked` | exit 0, exit 0 |
| `… case … ssao_core`, `ssao_materials`, `ssao_geometry`, `lifecycle_drs` | exit 0 each |
| `… case … ssao_quality` | **exit 1**, latched (see above) |
| `R …/ssao-filter-001/filter_runner.py release --grant E/gpu-grant.json` | exit 0 at 12:37:12Z; no owned or frozen process |
| `R handoffs/0070-ambient-occlusion/ssao-repair/filter/analyze_filter.py E` | **exit 1** in 103 s: numeric pass; quality fail on N6 authored and N8 perspective |
| `R …/evidence/ssao-filter-001/review/make_review.py` | exit 0; 7 PNGs and the manifest |
| `R …/evidence/ssao-filter-001/next-step/per_ray_jitter.py` | exit 0 in 3.3 s, writing `per-ray-jitter-001.json` |
| `R …/evidence/ssao-filter-001/next-step/steps8_full_stratum.py` | exit 0 in 11.0 s, writing `steps8-full-stratum-001.json` |
| `R …/evidence/ssao-filter-001/make_index.py` | exit 0, writing `index.json` |

**What did not happen:**

- no Cargo, build or target use;
- no retry or new render;
- no GPU work after the release.

The CPU checks and review aids read retained evidence only.

## Changed paths

New files, committed with `Built-by: claude`:

- `handoffs/0070-ambient-occlusion/filter-result.md`
- `handoffs/0070-ambient-occlusion/evidence/ssao-filter-001/`: `index.json`,
  `make_index.py`
- `handoffs/0070-ambient-occlusion/evidence/ssao-filter-001/review/`:
  `make_review.py`, `manifest.json` and the 7 PNGs
- `handoffs/0070-ambient-occlusion/evidence/ssao-filter-001/next-step/`:
  `per_ray_jitter.py`, `per-ray-jitter-001.json`, `steps8_full_stratum.py`,
  `steps8-full-stratum-001.json`

**Unchanged:**

- `result.md` and every earlier report;
- `tools/` (build004);
- `ssao-filter-001/` (the frozen helper and plan) and `ssao-repair/`;
- production source.

Root's `brief.md` stays modified and uncommitted, and the two generated
`Cargo.lock` files stay untracked. Outside the worktree, only the frozen
runner and analyzer wrote anything, and only into the assigned evidence
directory.

## Limitations and open questions

**Limitations:**

- **No appearance evidence exists for R1/R3.** The aids come from 321×193
  readbacks, not from the 959×539 appearance views.
- **One desktop adapter only:** Apple M5 Pro on Metal. No Vulkan, Dx12 or
  phone runs.
- **The N8 candidates are CPU-model results.** The model matched the device
  on every quality value here, but that is not device evidence for the
  candidates.
- **The 16-probe figure is an extrapolation** from two points.

**Open questions for root:**

1. May the guard fix ship in a new namespace, with the full 9-child sequence
   rerun?
2. For N8 perspective, choose (a) a documented limitation or (b) the 16-probe
   CPU screen.
3. Adopt N6T in place of N6 authored, or keep N6 authored failed?
