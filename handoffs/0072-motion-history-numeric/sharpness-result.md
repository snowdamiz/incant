# 0072 result: frozen C-reg sharpness comparison

**Status: the run is complete and integrity PASSED; the quality comparison FAILED.**

- **Run and integrity:**
  - All 12 GPU children ran once each, in plan order, and exited 0. The GPU was
    released.
  - Both numeric controls passed.
  - All 36 case-contract checks and all 8 exactness checks passed.
- **Quality:** C-reg (`a143b5c0`) fails **13 of the 37 frozen quality
  thresholds**. Compared with the accepted baseline:
  - it is much sharper;
  - it is less stable on edges and sub-pixel features;
  - it misregisters pixel-aligned content after a history reset.
- **Failures shared with the baseline:** halo and same-phase also fail on the
  baseline, and are kept visible below.
- **Not claimed:** a passing integrity check is not a quality pass. No phase,
  performance, device or gate approval is claimed.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet:** "Execute frozen sharpness comparison — 2026-10-13". This is
  `brief.md`, sha256 `42777138…6fae`, which is root-owned, uncommitted and
  unmodified.
- **Before the run:** I read `sharpness-build-repair.md`.
  - Attempt001 is retained, and no GPU ran in it.
  - Root changed only the nonvisual build linkage helper (`cd4f4cd7`).

## 1. Verdicts

| Layer | Result | Evidence |
|---|---|---|
| GPU run | 12 GPU children in plan order, once each, all exit 0; released 12:01:17Z | `gpu-ledger.jsonl`, `gpu-released.json` |
| Submissions | 357 of 357 planned (252 numeric + 96 world + 9 UI), cap 372; 16 PNGs, all 960×540, cap 16 | `budget.jsonl`, release record |
| Numeric controls | **PASS**, both exact tests, see §3 | `logs/gpu-numeric-*.stdout.txt` |
| Case contracts | **PASS**, 36 of 36 checks in 8 cases | `captures/*/report.json` |
| Integrity (`analyze.py check`) | **PASS** | `analysis/summary-2026-10-13T120122.643644+0000.json`, sha256 `ca659d29…962a` |
| Exactness against the baseline | **PASS**, 8 of 8 (see below) | compare JSON |
| Quality thresholds (unchanged) | **FAIL**: 24 pass, 13 fail | `analysis/compare-2026-10-13T120142.507964+0000.json`, sha256 `759ac37c…8e33` |

The 8 exactness checks cover:
- the baseline manifest, its files and its identity;
- the candidate commit;
- the 5 temporal-off references, which are byte-identical;
- the same adapter, the same toolchain and host, and the same review config.

## 2. Identity

| Item | Identity |
|---|---|
| Candidate (production) | `a143b5c0f537bd35111e40e99626f40318a385ac` (tree `fd3b43e2`). Production bytes are identical to `0c80df4e`; it adds 16 numeric depth controls. Unchanged by me. |
| Baseline | Accepted fixed-coverage run `9927598c`: source `e23776df`, visual tree `b1327f3d`, review binary `27756c0d…`, manifest `e337bc34…`. |
| Visual tools | Tree `b11a7d3aad4228eab0fee10024a87d46bb109560` at root helper commit `cd4f4cd7`; `plan.json` sha256 `c874115d…b9c3` |
| Fixture | `temporal_visual_review.rs` sha256 `d4fc4002…1338`. It is the baseline fixture (`9841c6b1…`) plus the appended `viewport` case, unchanged from my `1f4bbcb0`. |
| GPU grant | `~/.codex/incant-evidence/0072-fixed-sharpness-002/review-ready.json`, sha256 `d5e50057…dc0db9`. Matches the brief; unmodified. |
| Grant pins | Bloom release `17d62049…`, released with all children reaped. Build lease `aaa8f2eb…`, released 11:52:57Z with no target processes. `tests.json` `ed6ef37f…`. All verified before the first child. |
| Build record | `build/review.json` sha256 `9719632b…f17fa`. `library_linkage` is valid: same package features, and the unit-test build emits no ordinary library. |
| Review binary | `temporal_visual_review` sha256 `81cb646924957929…1a49d36980`, 18,249,272 B, mode 0555, features `default, physics-deterministic` |
| Numeric binary | `incant_render-lib-tests` sha256 `e62cf78c618220b0…1fbb631e85`, 32,021,872 B, mode 0555, same features |
| Source export | Exact: 2,452 build inputs, 0 mismatched or missing. `Cargo.lock` `68c187db…`; Rust 1.99.0. |
| Lint | Strict Clippy (`-D warnings`) on the fixture: exit 0 |
| Adapter and host | Apple M5 Pro, Metal, integrated GPU; Mac17,9, macOS 27.0.1; rustc 1.99.0 (`b940084d7`) |

**Preflight, before any GPU child:**
- the grant and its pins were verified;
- both frozen binaries matched their hashes and were executable;
- the ledger held only root's 3 build rows, all exit 0;
- there was no stop or release latch, `native-gpu.lock` was free, and no GPU
  process was running.

**Not touched:**
- I did no build;
- I did not touch the Cargo target (root was compiling SSAO in the core
  target);
- I made no threshold, fixture or tool edit;
- I made no retry.

## 3. Observed counts

| # | Child | PID | UTC start → end | Exit | Debit (frames / UI / PNG) | Receipt |
|---|---|---|---|---|---|---|
| 1 | `gpu.py list lib` | 16574 | 12:00:18.96 → 19.31 | 0 | — | `--list --ignored`: 41 ignored tests; both exact names present once |
| 2 | `gpu.py numeric resolve` | 16633 | 12:00:23.87 → 24.49 | 0 | 102 / 0 / 0 | `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion ... ok`; `1 passed; 0 failed; 0 ignored; 0 measured; 170 filtered out` |
| 3 | `gpu.py numeric coverage` | 16838 | 12:00:29.44 → 29.84 | 0 | 150 / 0 / 0 | `temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection ... ok`; `1 passed; … 170 filtered out` |
| 4 | `gpu.py list review` | 16868 | 12:00:34.52 → 34.74 | 0 | — | `--list --ignored`: 8 cases, matching the plan |
| 5 | `review probe` | 16927 | 12:00:38.52 → 38.80 | 0 | 0 / 0 / 0 | pass (4 checks) |
| 6 | `review stationary` | 17003 | 12:00:42.82 → 43.82 | 0 | 17 / 0 / 3 | pass (3) |
| 7 | `review motion` | 17058 | 12:00:47.88 → 49.57 | 0 | 16 / 0 / 4 | pass (4) |
| 8 | `review excluded` | 17090 | 12:00:53.82 → 54.74 | 0 | 9 / 9 / 3 | pass (5) |
| 9 | `review transitions` | 17100 | 12:00:58.62 → 59.53 | 0 | 13 / 0 / 2 | pass (8) |
| 10 | `review skinned` | 17158 | 12:01:03.26 → 04.06 | 0 | 7 / 0 / 2 | pass (3) |
| 11 | `review timing` | 17354 | 12:01:07.85 → 08.41 | 0 | 8 / 0 / 0 | pass (5); Models span median ratio on/off 2.24 (pass-coverage inference, not performance) |
| 12 | `review viewport` | 17387 | 12:01:12.34 → 13.72 | 0 | 26 / 0 / 2 | pass (4); outside-viewport max delta 0 |

- **Review receipts:** each review child printed `1 passed; 0 failed; 0 ignored;
  0 measured; 8 filtered out`. The binary holds the 8 ignored cases plus one
  other test, and the baseline run printed 7 for its 7 cases.
- **Release** (`gpu.py release`, 12:01:17.481Z):
  - `children: 15` (root's 3 builds plus my 12);
  - debited 348 frames, 9 UI submissions and 16 PNGs, with no unmatched
    debits;
  - `stop_latched: false`; `owned_processes: []`; `ledger_pids_alive: []`;
  - ledger `39219906…37c`, debits `66ac3a47…aa`, grant unchanged at release.
- **Totals against the plan:** 252 numeric + 105 visual = 357, against a cap
  of 372. The 16 PNGs equal the cap.

## 4. Quality comparison (thresholds unchanged, from `plan.json`)

Baseline values come from the accepted coverage run, computed with the same
rules.

| Section | Check | C-reg | Limit | Baseline | Result |
|---|---|---|---|---|---|
| Sharpness | Grille detail ratio | 0.566 | ≥ 0.5 | 0.182 | pass |
| | Effective σ: grille / posts+shadows / far posts / horizon diagonal / sphere edge (px) | 0.533 / 0.531 / 0.567 / 0.533 / 0.457 | ≤ 0.6 each | 0.887 / 0.900 / 0.874 / 0.879 / 0.757 | pass ×5 |
| Stability | Activity ≥ 33 steps (frames 9–16) | 216 | ≤ 318 | 159 | pass |
| | Saturated activity (≥ 64 steps) | **4** | ≤ 0 | 0 | **FAIL** |
| | Edge mean, frames 9–16 | 2.419 | ≤ 5 | 3.316 | pass |
| | Edge max, frames 9–16 | **68** | ≤ 50 | 44 | **FAIL** |
| | Interior changed, frames 9–16 | 40 | ≤ 100 | 54 | pass |
| | Same-phase f8/f16 interior changed | **10** | ≤ 0 | 17 | **FAIL** (baseline fails too) |
| | Halo, stationary (above the off 3×3 max by > 2) | **100** | ≤ 0 | 1,048 | **FAIL** (baseline fails too) |
| | Halo, excluded | 0 | ≤ 0 | 1,276 | pass |
| Ghosts | Object off-edge bins 3–8 / 9–16 / > 16 | 403 / **39** / 0 | each ≤ baseline | 830 / 30 / 0 | **FAIL** |
| | Pan off-edge bins | 285 / **2** / 0 | each ≤ baseline | 391 / 0 / 0 | **FAIL** |
| | Skinned off-edge bins | 0 / 0 / 0 | each ≤ baseline | 0 / 0 / 0 | pass |
| | Cube trailing 1 px band vs top control (R−B) | **54.0** vs 42.42 | ≤ 1.1 × top (46.7) | 47.73 vs 44.29 | **FAIL** |
| | Trailing band vs baseline | **54.0** | ≤ 47.73 | — | **FAIL** |
| Exclusion | UI identical in all 8 phases and off | 8 × true | 8 × true | true | pass |
| | Overlay activity ≥ 33 | 0 | ≤ 0 | 0 | pass |
| Reset | Cut f7 and DRS f9 max step vs baseline | 0 and 0 (byte-identical) | ≤ 1 | — | pass ×2 |
| Registration (linear LK, px) | Stationary | 0.007, −0.027 | \|d\| ≤ 0.1 | 0.001, −0.033 | pass |
| | Motion object | 0.022, −0.028 | | 0.001, −0.038 | pass |
| | Skinned (87 points) | **0.207**, 0.042 | | 0.004, 0.023 | **FAIL** |
| | Excluded | −0.000, −0.001 | | −0.000, −0.001 | pass |
| Viewport (new; absolute bounds) | Still activity ≥ 33 / saturated | 280 / **10** | ≤ 318 / ≤ 0 | — | pass / **FAIL** |
| | Still edge mean / edge max | 2.015 / **67** | ≤ 5 / ≤ 50 | — | pass / **FAIL** |
| | Still interior changed / same-phase | 33 / **7** | ≤ 100 / ≤ 0 | — | pass / **FAIL** |
| | Border band max not above inner (still, pan) | 1 vs 96; 1 vs 95 | band ≤ inner | — | pass ×2 |
| | Outside the viewport | 0 | = 0 | — | pass |
| | Pan f24 off-edge 9–16 / > 16 | **3 / 211** (max 86) | 0 / 0 | — | **FAIL** |

**Failed, verbatim from the compare record:**
- `stability: stationary_activity_saturated`, `frames_9_16_edge_max`,
  `same_phase_f08_f16_interior_changed`, `halo_stationary-on-f16.png`;
- `ghosts: off_edge_motion-object-on-f8.png`, `off_edge_motion-pan-on-f6.png`,
  `trailing_band_vs_top_control`, `trailing_band_no_worse_than_baseline`;
- `registration: skinned-on-f6.png`;
- `viewport: still_activity_saturated`, `still_frames_9_16_edge_max`,
  `still_same_phase_interior_changed`, `pan_off_edge_ge9`.

### Original failures that stay visible

- **Halo.** The baseline itself fails: 1,048 stationary and 1,276 excluded.
  - C-reg reduces these to 100 and 0, but stationary still fails.
  - 78 of the 100 pixels lie on one row (y = 243) at the bottom edge of the
    dark wall. There the reference has a one-pixel bright sliver (130 at
    y = 244), and C-reg shows it over two rows (137 and 128).
  - Most of the rest are at a far post (434, 42–45, up to 23 steps) and on two
    short rows at (579–610, 182–183); a few are scattered.
  - As `sharpness-preparation.md` concern 1 records, this rule also flags real
    anti-aliasing of sub-pixel features that the point-sampled reference misses.
- **Same-phase f8/f16 interior.** The baseline fails at 17 pixels (max step 4).
  C-reg gives 10 (max step 8), and the viewport gives 7. It still fails.

Neither threshold is relaxed here. Whether to amend them is root's decision.

## 5. Assessment

I personally inspected:
- **Every generated PNG:** all 16, at full size.
- **Every exact baseline pair:** the 14 baseline PNGs against the
  same-named candidate PNGs. The 5 temporal-off references and both reset
  captures are byte-identical to the baseline.
- **Magnified crops** of every failing region: the grille, far posts, right
  diagonal, wall base, moving cube and its shadow, pan post shadow, skinned
  checker corners, excluded checker edges, the viewport shadow and both
  viewport borders.

Every number below is reproducible from `fixed-sharpness-inspection/`.

### Sharpness and thin diagonals: large improvement

- **Grille.** The one-texel MASK grille is resolved:
  - its vertical, horizontal and diagonal bars are distinct;
  - it is slightly softer than the aliased reference;
  - the baseline shows a green wash.
- **Far posts.** These are separate posts. The baseline merges them into one
  blur.
- **Diagonal edges.** The long ground/sky diagonals are crisp and smoothly
  anti-aliased (the baseline has a 2–3 px ramp), with no visible staircase in
  the still frame.
- **Measured:**
  - grille detail rises from 0.182 to 0.566;
  - effective σ falls from 0.757–0.900 to 0.457–0.567 px;
  - the stationary f16-vs-off interior changes in 195 pixels (max 46),
    against the baseline's 4,133 (max 24).
- **Excluded backdrop.** Checker edges converge to the true area coverage (for
  example, about 63% at column 360) instead of a two-pixel blur.
- **Under motion.** The pan keeps the grille lattice where the baseline turns it
  to moiré.

### Stability: less low-level shimmer, more high-amplitude edge flicker

**Activity bands in the stationary case, frames 9–16 (all pixels):**

| Steps | Baseline | C-reg |
|---|---|---|
| 1–2 | 48,469 | 35,507 |
| 3–8 | 18,919 | 6,500 |
| 9–16 | 4,738 | 5,055 |
| 17–32 | 711 | 1,012 |
| ≥ 33 | 159 | 216 |
| saturated | 0 | 4 |

- **Excluded case:** activity is lower in every band, for example 8,273 → 2,794
  at 3–8 steps.
- **Viewport still case:** it repeats the stationary pattern (280 / 10 / 67).

**Where the high-amplitude activity sits:**
- **The 4 saturated pixels:** (428, 54–57) and (431, 53). These are the
  one-pixel ground gaps between the most distant fence posts.
  - In the reference those columns are 183 (ground).
  - C-reg mostly closes them, at about 106.
  - The activity map reads 255 in those gap columns.
- **The 33–35-step band:** regular 6–8-pixel groups on the staircase steps of
  the long right diagonal (602–904, 52–154).
- **One more cluster:** (553–571, 226–237) at the grille card's lower corner,
  up to 49 steps.

### Motion and depth pairing

- **Trailing band (the pairing gate) fails.**

  | Band | Baseline (R−B) | C-reg (R−B) |
  |---|---|---|
  | 1 px outside the trailing edge | +47.73 | +54.00 |
  | 2 px outside | +6.03 | +0.70 |
  | 3 px outside | +0.09 | +0.03 |
  | First pixel inside | −19.06 | −19.58 |
  | Top control, 1 px outside | +44.29 | +42.42 |
  | Top control, first pixel inside | −22.89 | −0.69 |

  - **Total outside smear is unchanged:** 54.7 for C-reg against 53.9 for the
    baseline.
  - **The difference is concentration:** C-reg puts it all in the first pixel
    and removes the second-pixel residue. Its no-motion top edge stays sharp
    inside.
  - The excess over the top control is +12.3 for C-reg against +9.6 for the
    baseline.
- **Object off-edge 9–16 (39 vs 30).**
  - The 39 pixels are two rows at (579–600, 304–305) on the soft edge of the
    moving cube's shadow, which falls on static ground (no receiver motion).
  - Both runs lag that edge by about one row and differ from each other by at
    most 8 steps.
  - In the box (560–640, 295–315), C-reg has fewer 9–16-step pixels than the
    baseline: 40 against 64.
  - The bin fails on pixels sitting just above the 9-step threshold. The 3–8
    bin improved from 830 to 403.
- **Pan off-edge 9–16 (2 vs 0).**
  - (385–386, 58) is a 10-step ghost of a one-pixel far-post shadow line. The
    baseline's version is 7 steps.
  - The 3–8 bin improved from 391 to 285.
- **Thin-line residue in both runs.** Both runs leave faint orange dots where
  the cube crossed the wall's one-pixel bright base line, about 21 px apart.
  These are within 3 px of an edge, so no bin counts them.

### Registration: one real regression (skinned)

- **The skinned case is shifted.** Every static checker corner, far from the
  deforming triangle, is offset by about (+0.19 to +0.22, +0.09 to +0.22) px.
  - This holds across all 40 point clusters at the 20 static corners.
  - The baseline's same corners measure −0.04 to +0.005.
  - This is a global shift, not residue from the moving triangle.
- **Cause, reproduced exactly:**
  - The orthographic checker's edges lie on pixel boundaries (x = 210,
    y = 135).
  - Frame 1 is a history reset. Its `registered_current` bilinear un-jitter at
    phase 0, j = (−0.375, −0.125), blends 37.5% of the left neighbour into
    column 210 and 12.5% into row 135, on one side only.
  - The later raw jittered samples are exact for pixel-aligned edges, so that
    one-sided blend decays only as 0.9ⁿ.
  - After five accumulations, 0.375 × 0.9⁵ = 0.22 of it remains. In linear
    light that predicts G = 75 (light square) and 45 (dark square) at column
    210, which is exactly what was measured (`pixels-skinned-corner.txt`).
- **Other cases.** Stationary, object and excluded register within 0.03 px.
  Excluded is dominated by its identical current-only sprites and text.

### Viewport: borders and exclusions are clean; the pan failure is not the resolve

**The viewport-specific rules pass:**
- the odd, offset rect `[37, 19, 885, 501]` keeps the outside byte-identical;
- the 3 px border band never exceeds the inner maximum (1 against 95–96);
- the crops show clean anti-aliased diagonals up to both borders, with no
  smear across them.

**The pan off-edge failure is a shadow-map difference, not a resolve defect:**
- **Location.** 212 of the 214 pixels at 9 steps or more lie inside one
  parallelogram. That includes all 211 above 16 steps (worst: (582, 288),
  on = (100, 112, 130), off = (186, 189, 194)).
  - The parallelogram is the grille card's shadow on the ground, which I
    derived at (444–695, 274–306).
  - The other two pixels are at exactly 9 steps.
- **Appearance.** There the temporal-on frame shows a **solid** card shadow,
  and the temporal-off reference shows the hatched grille shadow.
- **It precedes the pan.** At the still pose, on-f16 against off-still already
  differs in 4,180 interior pixels (max 96).
- **Cause.** `shadows/cascade.rs::fit` widens the frustum by the temporal
  receiver guard (1/width, 1/height) before quantizing the cascade radius to
  1/16.
  - In the 885×501 viewport this crosses a quantization step in cascades 0 and
    1: the texel grows by 0.752% and 0.389%. Cascade 3 changes slightly at
    both sizes.
  - The grille's shadow falls entirely in cascade 1, at view depth 8.8–10.1.
  - The grille texel (0.0156 world units) is half a cascade-1 shadow texel
    (0.0315). So the alpha-tested one-texel bars alias differently in the on
    run and the off run.
  - At 960×540 the guard leaves cascades 0–2 unchanged. That is why the
    stationary and pan shadows match their references.
- **Shared with the baseline.** `shadows/cascade.rs`, `camera.rs`,
  `shadow_draw.rs` and `jitter::guard` are byte-identical between `e23776df`
  and `a143b5c0`. All 8 files that change are in the temporal
  resolve/copy/delta path. So the baseline would show the same difference;
  there is no baseline viewport run to show it.
- **Evidence:** `cascade_texels.txt` and
  `off_edge_clusters-viewport-pan-creg.txt`.

### Exclusions and resets: pass

- **Exclusions:**
  - the UI button is byte-identical across all 8 jitter phases and off;
  - the sprites, BLEND card and world text stay current-only;
  - overlay activity ≥ 33 is 0;
  - excluded halo falls from 1,276 to 0.
- **Resets:** the cut f7 and DRS f9 captures are byte-identical to the baseline
  (both are history-free frames), and all 8 transition contract checks pass.

## 6. Remaining design failures and the smallest concrete repairs

The CPU derivation is `fixed-sharpness-inspection/repair_model.py` (output
`repair_model.txt`).
- It reuses `model2d.py`'s shader rules unchanged.
- It adds three candidate switches and one attribution-only switch.
- It is a model of the rules, **not a measurement**. No candidate below has
  been compiled or run on a GPU.

| Failure | Cause | Smallest concrete repair | Derived effect (model) | Leaves open |
|---|---|---|---|---|
| Skinned registration 0.207 px | **Confirmed** (§5). The history-free first frame's one-sided bilinear un-jitter on pixel-aligned edges decays at 0.9 per frame. | **Reset ramp.** Use history weight `min(0.9, n/(n+1))` for the n-th accumulation after a reset. This is a CPU-side value in the existing uniform (`options.x`): no ABI, binding or shader change. Reset frames are unchanged, so cut/DRS identity holds. Numeric controls that assume a constant weight after a reset would need matching updates. | Pixel-aligned edge bias at f6: 0.204 → **0.060 px** (the GPU measured 0.207 against the model's 0.204); at f16, 0.066 → 0.019. Steady state unchanged. | Per-pixel disocclusions after the reset: there is no per-pixel age state. |
| Diagonal and thin-line flicker (≥ 33 band, part of edge max) | **Consistent** with the prediction. Each frame blends a raw jittered sample, displaced by up to 0.375 px, into the unjittered history. | **Registered blend.** In the accumulation path, blend and measure reaction against the registered current (the bilinear estimate the refusal path already computes) instead of the raw centre sample. Its 2×2 footprint lies inside the 3×3 already loaded, or the existing 4-load function can be reused. | Flicker on the diagonal / thin line: 0.109 / 0.130 → **0.042 / 0.034** (baseline 0.069 / 0.052). Detail: 0.94 / 1.11 → 0.88 / 0.99 (baseline 0.54 / 0.42). | Costs some sharpness; grille detail could land near the 0.5 limit (unmeasured). |
| Sub-pixel gap saturation (edge max 68 / 67, saturated 4 / 10) | **Mechanism shown in the model.** See the notes below the table. | **No small repair is proven.** | Max change at a 0.7 px gap: C-reg 0.524, registered blend 0.392, baseline 0.301. | Root decision: accept far-geometry flicker, use content level of detail, or pursue a larger change (for example, a temporal history of the clamp range). |
| Trailing 1 px band | **Not isolated.** In the model, C-reg leaves zero trailing residue at 2.3 and 6.3 px/frame, with raster or colour-grid depth pairing alike. So the model gives no evidence either way that pairing causes the GPU excess. | **None proposed** from this evidence. | — | The real cube's per-row velocity (20.9–21.7 px/frame), its shading, and the wall about 16% behind it are not modelled. |
| Object (39 vs 30) and pan (2 vs 0) 9–16 bins | Threshold-edge effects of lags both runs share (a moving shadow on a static receiver; a one-pixel far shadow line). | No C-reg-specific repair. The registered blend may lower the pan pixels. | — | Shared-lag behaviour. |
| Viewport pan (211 above 16 steps) | **Confirmed** as a shadow-map difference (§5), not the resolve. | **Production, outside C-reg:** fit cascades with the same receiver guard whether or not jitter is active, so the on and off shadow maps match. Simply dropping the guard from the fit looks unsafe: by my reading of the margin comment, about half a texel (0.0157) is left after snapping and filtering, less than the 0.0197 radius growth the guard causes. The existing guard test would decide. | — | Root/production decision. The packet does not change production. |
| Halo, same-phase | Baseline metric limits (§4). | The reset ramp should shrink the same-phase f8/f16 difference, since the first-frame residual drops from 0.48 / 0.21 to about 0.125 / 0.05. Unmeasured. | — | The halo rule against a point-sampled reference. |

**Sub-pixel gap mechanism (third row), as shown in the model:**
- **Refusals.** At 0.5–0.7 px gaps between dark posts, C-reg refuses the whole
  history in 26–48% of frames; the baseline refuses in 0%.
  - C-reg's static reprojection lands exactly on a texel centre, so one
    toggling raster depth sample decides admission.
- **The clamp takes over.** Admitting on the four nearest raster depths
  (`depth2x2`) removes the refusals, but the clamp then makes the same jump,
  0.524 of contrast.
  - In the phases that miss the gap, the current 3×3 has no background sample.
- **Why the baseline is lower.** The baseline's presentation blur hides it, at
  0.301.

**Recommended next bounded change:**
- **Reset ramp first.** It fixes the only C-reg regression with a confirmed
  mechanism, and it touches no shader. If a further shader change is wanted,
  evaluate the registered blend after it.
- **Separate decisions.** The far-post gap flicker, the trailing band and the
  viewport cascade issue need separate decisions; neither change above
  resolves them.
- **No rerun proposed.** No GPU rerun of the unchanged candidate is proposed.

## 7. Evidence package

**In this worktree:**
- **`fixed-sharpness-screenshots/`**, 16 PNGs at 960×540, copied by
  `analyze.py publish` and hash-checked against the ledger:

  | PNG | sha256 (first 16) |
  |---|---|
  | `stationary-off` | `2485512777e50989` |
  | `stationary-on-f16` | `8d7b20a90014a50e` |
  | `stationary-activity` | `576979e65df4ff7b` |
  | `motion-object-on-f8` | `c39f80b8e8f8233b` |
  | `motion-object-off-f8` | `212ac1a0e0af8307` |
  | `motion-pan-on-f6` | `2f18c032e9a4e383` |
  | `motion-pan-off-f6` | `30aa3e196bca8145` |
  | `excluded-on-f8` | `6a4e116cc5261d75` |
  | `excluded-off` | `d1592ae5ed1f392e` |
  | `excluded-activity` | `3d29ab09c65fa7bc` |
  | `transition-cut-f7` | `5bc0618bc5a5f44f` |
  | `transition-drs-f9` | `89097514f9ef2b96` |
  | `skinned-on-f6` | `ae4ab390d309314f` |
  | `skinned-off-f6` | `9dc53d04deebc66b` |
  | `viewport-on-f24` | `e120753993f2ad09` |
  | `viewport-off-f24` | `2b762c2041e901a3` |

- **`fixed-sharpness-evidence/`**, 59 files plus `manifest.json` (sha256
  `62e89fa2…6b87`, published 12:04:45Z):
  - the grant and build grant;
  - the ledger, debits, release latch and target lease;
  - the integrity summary and the **failing compare record**;
  - the build records (source, toolchain, review, tests, lint);
  - all 15 children's stdout and stderr;
  - each case's `report.json` and `events.jsonl`.
- **`fixed-sharpness-inspection/`**, my CPU inspection scripts and their
  outputs, with exact commands and exit codes in `commands.txt`:
  - `cascade_texels`, `off_edge_clusters`, `registration_split`, `pixels`,
    `localize`, `activity_bands`, `cube_profile`, `cube_velocity` and
    `region_compare`;
  - the `repair_model` derivation.
  - Magnified crops were made with
    `temporal-coverage-inspection/crops.py` into a scratch directory outside
    the worktree. They are viewing aids, not committed, and their commands are
    listed.
- **This document;** `result.md` now points here.

**Outside the worktree:**
- `~/.codex/incant-evidence/0072-fixed-sharpness-002/` is the run's own
  namespace.
- The only files I added there are what the guarded tools write:
  - the ledger and debit rows;
  - the release latch;
  - `captures/`, `logs/`, and `analysis/summary-…json` and `compare-…json`.
- Nothing there was edited or removed.

**Unchanged:** production, the grant, every earlier evidence directory and
result, and `brief.md`.

## 8. Commands run

All were run from the worktree root with `/opt/homebrew/bin/python3` 3.14.8,
in the foreground, once each.

1. **Read-only preflight:**
   - the brief and `sharpness-build-repair.md`;
   - the grant and pin hashes;
   - the build records and binary hashes;
   - ledger and latch checks;
   - `native-gpu.lock` holder check and a GPU-process check.
2. **GPU children** (each `python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py …`):
   - `list lib`;
   - `numeric resolve`;
   - `numeric coverage`;
   - `list review`;
   - `review probe`, `stationary`, `motion`, `excluded`, `transitions`,
     `skinned`, `timing`, `viewport`;
   - `release`.

   All exited 0.
3. **Analysis:**
   - `analyze.py check`: exit 0, pass.
   - `analyze.py compare`: exit 1, the honest threshold failure; the record was
     retained.
   - `analyze.py publish`: exit 0; 59 files and 16 screenshots.
4. **Inspection:** the commands in `fixed-sharpness-inspection/commands.txt`,
   all exit 0, plus the crops listed there.
5. **Not done:** any build, Cargo command, retry, threshold or tool edit,
   ad hoc renderer, push, PR, merge, account change, memory write, subagent,
   or phase approval.

## 9. Limitations

- **Desktop only.** This is one local desktop run on an Apple M5 Pro (Metal).
  It is not a phone, shipping-profile, device or performance measurement, and
  timing is a pass-coverage inference only.
- **Thresholds are proposals.** They come from `plan.json`, not plan gates.
  The viewport case has no baseline and uses absolute bounds.
- **What the fixture keeps.** It keeps final-frame PNGs, activity peaks and
  report statistics, not per-frame images. So:
  - the same-phase pixels and the path taken at each saturated pixel cannot be
    located on the GPU;
  - §6 relies on the CPU model for mechanism.
- **The model is a model.** It is 2-D, single-channel and limited to synthetic
  scenes. It reproduces the registration bias and the gap behaviour but not the
  cube's trailing band. Its predicted repair effects are unmeasured.
- **The cascade explanation is derived, not toggled on the GPU.** It comes from
  the frozen source and fixture constants; the guard was not toggled in a run.
  Its location, shape and still-pose precursor match the observation.
- **Not in scope:** MetalFX, upscaling and phone work.

## 10. Open questions for root

1. **Cascade guard.** Should cascade fitting be made independent of the
   temporal receiver guard? That would make shadows identical with temporal on
   and off. It is a production change.
2. **Metric amendments.** Should the halo and same-phase rules become
   baseline-relative? Both fail on the accepted baseline.
3. **Far-geometry flicker.** Is flicker on sub-pixel far geometry acceptable
   (with level of detail handled in content), or must the resolve address it?
   No small change does.
4. **Next change.** Should the reset ramp (CPU-only) be the next bounded change,
   with the registered blend evaluated separately afterwards?
