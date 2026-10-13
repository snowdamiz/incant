# 0072 result: frozen reset-ramp (fixed-warmup) comparison

**Status: the run is complete and integrity PASSED; the quality comparison FAILED.
Two earlier failures are resolved and there are no new ones.**

- **Run:**
  - 11 GPU children ran in plan order, once each, all exit 0.
  - The GPU was released at 13:29:52Z.
  - 110 of 110 planned submissions (cap 120), plus 16 PNGs.
- **Numeric guard:** the probe genuinely enabled temporal pipelines, so the
  retained regression's pass is not vacuous.
- **Comparison:** exact. All 17 exactness checks pass against the accepted
  `e23776df` baseline and the rejected `a143b5c0` prior. Quality: 27 of 38
  checks pass and 11 fail.
  - **Resolved since a143:** skinned registration and pan off-edge.
  - **Persisting:** 11 of the 13.
  - **New:** none.
  - **Reset frames:** all 11 history-free frames are byte-identical to the prior,
    and the 10 the baseline also has match it too.
- **What the ramp did:**
  - It fixed what it was designed for: registration after a reset (0.207 →
    0.067 px).
  - It did not solve the unrelated failures.
  - It made steady-state flicker peaks on sub-pixel features worse.
- **Not claimed:** passing integrity is not a quality pass. No phase,
  performance, device or gate approval is claimed.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment. I cannot
  inspect the ACP launch settings from inside the session.
- **Packet:** "Execute frozen reset-ramp comparison — 2026-10-13". This is
  `brief.md`, sha256 `b60aa8b7…3163`, which is root-owned, uncommitted and
  unmodified.
- **Unchanged:** production, helpers, plan, fixture and thresholds. I used no
  Cargo or target, made no retries, and did not rerun the 252 direct controls.

## 1. Verdicts

| Layer | Result | Evidence |
|---|---|---|
| GPU run | 11 children in plan order, all exit 0; no refusal, stop latch or post-release row | `gpu-ledger.jsonl`, `gpu-released.json` |
| Submissions | 110 = 5 numeric + 96 world + 9 UI (cap 120); 16 PNGs, all 960×540 | `budget.jsonl` (9 debits) |
| Retained regression | **PASS**, see §3 | `logs/gpu-numeric-retained.*` |
| Temporal enablement | **PASS**, see §3 | `captures/probe/report.json` |
| Case contracts | **PASS**, 36 of 36 checks in 8 cases | `captures/*/report.json` |
| Integrity (`analyze.py check`) | **PASS** | `analysis/summary-2026-10-13T132958.314439+0000.json`, sha256 `671ee5a7…43b7` |
| Exactness (baseline and prior) | **PASS**, 17 of 17 | compare JSON |
| Quality (thresholds unchanged) | **FAIL**: 27 pass, 11 fail | `analysis/compare-2026-10-13T133029.430314+0000.json`, sha256 `eeba0f35…8ed6` |

**Exactness, by reference:**
- **Baseline (8 checks):** manifest, files, identity, candidate commit, the 5
  off references, adapter, toolchain and config.
- **Prior (9 checks):**
  - manifest `62e89fa2…`, its files and identity;
  - its recorded 13 failures;
  - temporal-off identity (6 PNGs, 12 frames);
  - adapter, toolchain and config;
  - recomputed failures equal to the record.

## 2. Identity

| Item | Identity |
|---|---|
| Candidate | `296e02268458afea21ff90e8aea820a3dec2b314` (reset ramp, Built-by: astra, base `a143b5c0`) |
| Visual tree | `648e32e9d9b1a7b85f2c2e6db2b03e7be1998e25` (preparation `c10d01f5` / `8dfeb207`); clean at every child |
| Grants | `review-ready.json` `a0611325…4050` (stage `fixed-warmup-visual`); `build-ready.json` `46c1d3ef…f97d` |
| Grant pins (verified before the first child) | 0070 GPU release `ec52f60f…`, released 12:37:12Z, `owned_processes_alive: []`. This build's lease `0e4fe21a…`, released 13:25:28Z, no target processes. |
| Build record | `build/review.json` `afb3f68b…f476`, as the brief states; `library_linkage` valid (same package and features) |
| Root build review | `~/.codex/incant-reviews/fixed-warmup-build-root-20261013.json` `c7abafec…7365`, as the brief states (`accepted_frozen_build_for_bounded_Claude_GPU`, `gpu_executed: false`) |
| Source | Freeze `05daf1a3…`; 2,452 build inputs exact; `Cargo.lock` unchanged; Rust 1.99.0; strict lint exit 0 |
| Root's builds | `libtests` PID 95324, `review` 95800, `lint` 95855: all exit 0 and reaped |
| Review binary | `temporal_visual_review` `924028c951e0bad4…1f06c87`, 18,249,384 B, mode 0555, features `default, physics-deterministic` |
| Library test binary | `incant_render-lib-tests` `cb86e678f00bd3b1…d768181`, 32,017,760 B, mode 0555, same features (from `build/tests.json`; the grant pins no separate binary) |
| Adapter and host | Apple M5 Pro, Metal, integrated GPU; Mac17,9, macOS 27.0.1; the same as both reference runs |

## 3. Observed counts

| # | Child | PID | UTC start → end | Exit | Debit (frames / UI / PNG) | Receipt |
|---|---|---|---|---|---|---|
| 1 | `gpu.py list lib` | 97022 | 13:28:44.58 → 44.96 | 0 | — | 41 ignored tests; the exact retained name listed once |
| 2 | `gpu.py numeric retained` | 97063 | 13:28:53.95 → 54.63 | 0 | 5 / 0 / 0 | `models::retained::temporal_tests::temporal_color_motion_and_geometry_publish_once_and_failures_consume_no_phase ... ok`; `1 passed; 0 failed; 0 ignored; 0 measured; 173 filtered out` |
| 3 | `gpu.py list review` | 97122 | 13:29:02.30 → 02.56 | 0 | — | 8 cases, matching the plan |
| 4 | `review probe` | 97144 | 13:29:09.58 → 09.99 | 0 | 0 / 0 / 0 | pass (4 checks) |
| 5 | `review stationary` | 97169 | 13:29:18.76 → 19.71 | 0 | 17 / 0 / 3 | pass (3) |
| 6 | `review motion` | 97224 | 13:29:23.75 → 25.30 | 0 | 16 / 0 / 4 | pass (4) |
| 7 | `review excluded` | 97260 | 13:29:29.36 → 30.20 | 0 | 9 / 9 / 3 | pass (5) |
| 8 | `review transitions` | 97319 | 13:29:35.16 → 36.06 | 0 | 13 / 0 / 2 | pass (8) |
| 9 | `review skinned` | 97368 | 13:29:40.15 → 40.93 | 0 | 7 / 0 / 2 | pass (3) |
| 10 | `review timing` | 97416 | 13:29:44.85 → 45.37 | 0 | 8 / 0 / 0 | pass (5); Models median ratio on/off 2.20 (inference only) |
| 11 | `review viewport` | 97451 | 13:29:50.64 → 51.91 | 0 | 26 / 0 / 2 | pass (4); outside-viewport delta 0 |

**Numeric child details:**
- Its stderr names the adapter: `Apple M5 Pro … backend: Metal`.
- The binary holds 174 tests, 3 more than `a143b5c0`'s; those are root's
  `warmup_tests`.

**The numeric pass is not vacuous.** The probe reports:
- `temporal_pipelines_enabled: true`, `enable: Ok(())`;
- all 5 invalid configs rejected;
- the second enable idempotent.

`enable_temporal_accumulation` calls `crate::motion::admission()?` first, so
motion admission holds on this adapter. The retained test therefore could not
have taken its early-return branch.

**Release** (12th command, `gpu.py release`, 13:29:52.008874Z):
- `children: 14` (root's 3 builds plus my 11);
- debited 101 frames (5 numeric + 96 world), 9 UI submissions and 16 PNGs, with
  no unmatched debits;
- `stop_latched: false`, `owned_processes: []`, `ledger_pids_alive: []`;
- ledger `6c3f008c…`, debits `94ce7d42…`, grant unchanged at release.

Afterwards no review or test binary was running and `native-gpu.lock` had no
holder.

## 4. Results against unchanged thresholds

Thresholds are byte-identical to the a143 plan; the preparation selfcheck
verifies this. The baseline is `e23776df`, the prior is `a143b5c0`, and the
warmup is this run.

| Section | Check | Baseline | Prior | **Warmup** | Limit | Result |
|---|---|---|---|---|---|---|
| Sharpness | Grille (MASK) detail ratio | 0.182 | 0.566 | **0.587** | ≥ 0.5 | pass |
| | Effective σ: grille / posts+shadows / far posts / horizon diagonal / sphere (px) | 0.887 / 0.900 / 0.874 / 0.879 / 0.757 | 0.533 / 0.531 / 0.567 / 0.533 / 0.457 | **0.521 / 0.519 / 0.557 / 0.531 / 0.451** | ≤ 0.6 | pass ×5 |
| Stability | Activity ≥ 33 (frames 9–16) | 159 | 216 | **139** | ≤ 318 | pass |
| | Saturated (≥ 64) | 0 | 4 | **6** | ≤ 0 | **FAIL** |
| | Edge mean | 3.316 | 2.419 | **2.368** | ≤ 5 | pass |
| | Edge max | 44 | 68 | **83** | ≤ 50 | **FAIL** |
| | Interior changed | 54 | 40 | **40** | ≤ 100 | pass |
| | Same-phase f8/f16 interior | 17 | 10 | **9** | ≤ 0 | **FAIL** (baseline fails too) |
| | Halo, stationary | 1,048 | 100 | **101** | ≤ 0 | **FAIL** (baseline fails too) |
| | Halo, excluded | 1,276 | 0 | **0** | ≤ 0 | pass |
| Ghosts | Object off-edge 3–8 / 9–16 / > 16 | 830 / 30 / 0 | 403 / 39 / 0 | **349 / 36 / 0** | each ≤ baseline | **FAIL** |
| | Pan off-edge | 391 / 0 / 0 | 285 / 2 / 0 | **96 / 0 / 0** | each ≤ baseline | pass (resolved) |
| | Skinned off-edge | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** | each ≤ baseline | pass |
| | Trailing 1 px vs top control (R−B) | 47.73 vs 44.29 | 54.00 vs 42.42 | **51.45 vs 38.07** | ≤ 1.1 × top (41.9) | **FAIL** |
| | Trailing vs baseline | — | 54.00 | **51.45** | ≤ 47.73 | **FAIL** |
| Exclusion | UI identical in 8 phases and off | true | 8 × true | **8 × true** | 8 × true | pass |
| | Overlay activity ≥ 33 | 0 | 0 | **0** | ≤ 0 | pass |
| Reset | Cut f7 / DRS f9 max step vs baseline | — | 0 / 0 | **0 / 0** | ≤ 1 | pass ×2 |
| | History-free frames identical (new, strict) | — | — | **11 frames: 11/11 prior, 10/10 baseline** | all identical | pass |
| Registration (px) | Stationary | 0.001, −0.033 | 0.007, −0.027 | **0.008, −0.027** | \|d\| ≤ 0.1 | pass |
| | Motion object | 0.001, −0.038 | 0.022, −0.028 | **0.034, −0.023** | | pass |
| | Skinned (87 points) | 0.004, 0.023 | 0.207, 0.042 | **0.067, −0.005** | | pass (resolved) |
| | Excluded | −0.000, −0.001 | −0.000, −0.001 | **−0.000, −0.001** | | pass |
| Viewport | Still activity ≥ 33 / saturated | — | 280 / 10 | **198 / 23** | ≤ 318 / ≤ 0 | pass / **FAIL** |
| | Still edge mean / max | — | 2.015 / 67 | **1.930 / 91** | ≤ 5 / ≤ 50 | pass / **FAIL** |
| | Still interior / same-phase | — | 33 / 7 | **39 / 2** | ≤ 100 / ≤ 0 | pass / **FAIL** |
| | Border band ≤ inner (still; pan) | — | 1 ≤ 96; 1 ≤ 95 | **1 ≤ 96; 1 ≤ 95** | band ≤ inner | pass ×2 |
| | Outside the viewport | — | 0 | **0** | = 0 | pass |
| | Pan f24 off-edge 9–16 / > 16 | — | 3 / 211 (max 86) | **2 / 211 (max 86)** | 0 / 0 | **FAIL** |

**Failure history** (`compare … baseline_context.failure_history`):
- **Resolved:**
  - `ghosts: off_edge_motion-pan-on-f6.png` (partly by metric reclassification;
    see §5);
  - `registration: skinned-on-f6.png`.
- **Persisting:**
  - stationary saturated, edge max, same-phase and halo;
  - object off-edge;
  - both trailing gates;
  - viewport saturated, edge max, same-phase and pan off-edge.
- **New:** none.
- **Recomputed against the record:** the prior's recomputed failures equal its
  13 recorded ones.
- **Also failing on the baseline:** halo and same-phase.

## 5. The ramp's actual effect

I personally inspected:
- **Every warmup PNG:** all 16.
- **Pairs and crops:** magnified prior / warmup / off crops (and the baseline
  where useful) of the grille and its activity, the skinned corner and
  triangle, the moving cube, the panned far posts, the excluded checker and the
  viewport shadow.
- **Exact pixel windows.**

These cover every captured run (baseline, prior, warmup); no extra GPU capture
was made. Every number below is reproducible from `fixed-warmup-inspection/`.

### Where the ramp changed the output

Compared with the prior, per PNG (`png_vs_prior.txt`):
- **Byte-identical:** every off reference and both transition captures.
- **Within 2 steps:** `viewport-on-f24`, which converged after 24 frames.
- **The short captures:**

  | Capture | Largest change (steps) | Pixels changed by more than 8 steps |
  |---|---|---|
  | object f8 | 43 | 4,708 |
  | pan f6 | 37 | 3,308 |
  | skinned f6 | 18 | 1,816 |
  | stationary f16 | 15 | 612 |
  | excluded f8 | 13 | 586 |

### By area

- **Registration: fixed as derived.**
  - **Skinned.** The global registration is 0.067 px (prior 0.207). The static
    checker corners away from the triangle are at 0.055 px (prior 0.205).
  - **Edge bleed.** At the pixel-aligned corner (column 210), the one-sided
    bleed is 5.4–5.9% in linear light (G 81 and 31 against the reference's 83
    and 24), down from the prior's 22% (75 and 45). The predicted residual
    after five ramped accumulations is 0.375 × 1/6 = 6.25%.
  - **Edge error.** Skinned f6-vs-off edge mean is 1.29 (prior 3.04).
  - **Other cases** stay within 0.012 px of the prior.
- **Flicker: moderate flicker down, peaks up.** For stationary frames 9–16:

  | Band | Prior | Warmup |
  |---|---|---|
  | 17–32 steps | 1,012 | 1,098 |
  | 33–63 steps | 212 | 133 |
  | ≥ 64 (saturated) | 4 | 6 |

  - **Diagonals.** The 35-step clusters on the long right diagonal's staircase
    fell below 33.
  - **The 6 saturated pixels:** the same 4 far-post gap pixels, plus 2 at the
    MASK card's bottom edge, (556, 242) and (542, 243).
  - **Edge max** rose to 83, at pairs 8→9 and 10→11; the prior peaked at 65 and
    64 there.
  - **Viewport** still frames: ≥ 33 280 → 198, saturated 10 → 23, edge max
    67 → 91 (pair 8→9; the prior had 53 there).
  - **Why the peaks rose.** The ramp's frame weights in frames 9–16 differ from
    the prior's by at most 0.011 (8/9 against 0.9). The larger peaks come from
    the history state instead:
    - After the ramp, the frame-8 history is the even mean of 8 phases, rather
      than being 48% the registered first frame.
    - So sub-pixel gaps and MASK bars carry more contrast.
    - The per-frame neighbourhood clamp/refusal then jumps further whenever a
      phase's 3×3 misses the feature. This matches the CPU-model mechanism in
      `sharpness-result.md` §6.
  - **Same-phase** improved as predicted: stationary 10 → 9 (max step 8 → 4),
    viewport 7 → 2.
- **Trailing: slightly lower in absolute terms, still failing.**

  | Measure (R−B) | Prior | Warmup |
  |---|---|---|
  | 1 px outside the trailing edge | 54.00 | 51.45 |
  | 2 px outside | 0.70 | 0.61 |
  | First pixel inside | −19.58 | −18.30 |
  | No-motion top control | 42.42 | 38.07 |
  | Excess over the top control | 12.3 | 14.0 (baseline 9.6) |

  Because the top control fell further, the excess grew. The object off-edge
  9–16 bin (39 → 36) is the same soft edge of the moving cube's shadow on static
  ground, (579–600, 304–305).
- **MASK detail: slightly sharper.** Grille detail 0.566 → 0.587, σ 0.533 →
  0.521. In the crops the lattice is indistinguishable from the prior; the
  activity map shows the same y%6 bar flicker.
- **Halo: unchanged.** 101 against 100, on the same one-pixel bright sliver at
  the wall base (y = 243); excluded stays 0.
- **Excluded overlays: still pass.**
  - The UI is identical across all 8 phases.
  - Sprites, the BLEND card and text stay current-only.
  - Overlay ≥ 33 stays 0.
  - **Prediction realised, still inside the threshold.** The backdrop's frames
    2–8 activity rose as the preparation predicted: the 9–16 band 413 → 1,922
    pixels and the 17–32 band 0 → 402. Nothing reaches 33 steps.
  - `excluded-on-f8` differs from the prior by at most 13 steps.
- **Viewport shadows: unchanged.** Pan off-edge above 16 steps is still 211
  (max 86), inside the grille card's shadow quad (`off_edge_clusters-viewport-pan-warmup.txt`).
  The crops show the same solid temporal-on shadow against the hatched off
  shadow. This is the cascade-guard difference from `sharpness-result.md` §5,
  outside the resolve.
- **Pan: resolved, partly by reclassification.**
  - **Real improvement.** Off-edge pixels at ≥ 3 steps fell from 287 to 96: the
    prior's largest cluster (108 pixels, 4–5 steps, at (436–455, 240–285)) is
    gone. Unmasked whole-frame differences above 16 steps fell from 16,072 to
    14,493.
  - **Reclassification.** The prior's 2 counted 9–16 pixels at (385–386, 58)
    lie on a sub-pixel far-post shadow line. The off reference barely samples
    it, but the ramp's even phase mix renders it darker (168 against the prior's
    173 and the reference's 183). Its 14-step neighbour step now exceeds the
    metric's 12-step edge threshold, so it is masked as an edge rather than
    counted.
- **Skinned disocclusion residue: lighter.** In the triangle box, differences
  above 16 steps fell from 413 to 225 pixels (3–8 steps: 191 → 431). The ramp
  has no per-pixel age, so this comes only from the lower global weight in
  frames 2–6.
- **Reset and first frames: identical.**
  - All 11 history-free frames are byte-identical to the prior.
  - The 10 the baseline also has are identical to it too.
  - Both transition PNGs are identical.
  - All 12 temporal-off frames and the 6 off PNGs are identical to the prior.

## 6. Next source prescription (only where the observations support one)

1. **Keep the ramp (`296e0226`).**
   - It resolves the reset-registration failure exactly as derived, within a
     fraction of a percent of the predicted residual.
   - It adds no threshold failure, and leaves every reset and off frame
     byte-identical.
2. **No resolve source change is prescribed for the 11 persisting failures.**
   - **Flicker peaks.** The observations localize them to features narrower than
     the jitter footprint (far-post gaps, MASK bar edges). They show the peaks
     grow when history represents those features more faithfully.
   - **What a fix must do.** It has to stop the per-frame neighbourhood
     clamp/refusal from toggling on such features.
   - **No measured candidate yet.** The earlier CPU model showed that removing
     refusals alone moves the same jump to the clamp. Nothing in this run
     identifies a specific change.
   - **Trailing gates.** Not isolated.
3. **Viewport pan off-edge: a fix in the shadow path, not the resolve.**
   - It is unchanged here (211 pixels).
   - The source-derived prescription from `sharpness-result.md` §5–6 stands, in
     `crates/incant_render/src/shadows/cascade.rs::fit`: use the same receiver
     guard in the temporal-on and temporal-off fits, so their shadow maps are
     identical.
   - The guard there is (1/width, 1/height). It currently changes cascades 0
     and 1 at 885×501.
   - This is a production change in the shadow path; root's call.
4. **Halo and same-phase stay threshold decisions.** Both fail on the accepted
   baseline too.

## 7. Evidence package

**In this worktree:**
- **`fixed-warmup-screenshots/`**, 16 PNGs at 960×540, copied by
  `analyze.py publish` and hash-checked against the ledger:

  | PNG | sha256 (first 16) |
  |---|---|
  | `stationary-off` | `2485512777e50989` |
  | `stationary-on-f16` | `10082f5aed1438d2` |
  | `stationary-activity` | `e599eec847957339` |
  | `motion-object-on-f8` | `541871d817450c78` |
  | `motion-object-off-f8` | `212ac1a0e0af8307` |
  | `motion-pan-on-f6` | `55784f565f71e5c1` |
  | `motion-pan-off-f6` | `30aa3e196bca8145` |
  | `excluded-on-f8` | `85e630e7cdf99726` |
  | `excluded-off` | `d1592ae5ed1f392e` |
  | `excluded-activity` | `a2aaf8f2c75a7188` |
  | `transition-cut-f7` | `5bc0618bc5a5f44f` |
  | `transition-drs-f9` | `89097514f9ef2b96` |
  | `skinned-on-f6` | `c9db68887a0408a2` |
  | `skinned-off-f6` | `9dc53d04deebc66b` |
  | `viewport-on-f24` | `fe22b8849e8f5f99` |
  | `viewport-off-f24` | `2b762c2041e901a3` |

- **`fixed-warmup-evidence/`**, 57 files plus `manifest.json` (`f6d3edb5…13fa`):
  - both grants, the ledger, debits, release latch and target lease;
  - the integrity summary and the **failing compare record**;
  - `build/` (source, toolchain, review, tests, lint);
  - the stdout and stderr of all 14 ledgered children;
  - each case's `report.json` and `events.jsonl`.
- **`fixed-warmup-inspection/`**, `commands.txt` (exact commands and exits) and
  the text outputs:
  - one new tiny aid, `png_vs_prior.py`;
  - the other scripts are the committed `fixed-sharpness-inspection/` aids,
    reused read-only.
  - The magnified crops were viewing aids outside the worktree, not committed;
    their commands are listed.
- **This document;** `result.md` now points here.

**Outside the worktree:** `~/.codex/incant-evidence/0072-fixed-warmup-001/`.
Only the guarded tools wrote there:
- the ledger rows, debits and release latch;
- `captures/`, `logs/`, `tmp/`, and `analysis/summary-…json` and
  `compare-…json`.

Root's files there are untouched.

## 8. Commands run

All were run from the worktree root with `/opt/homebrew/bin/python3` 3.14.8,
in the foreground.

1. **Read-only verification:**
   - the brief, the grants and their pins;
   - root's build review, build/test/source/lint records and ledger;
   - frozen binary hashes and modes;
   - `lsof` on `native-gpu.lock`, and `ps` for GPU children.
2. **GPU commands**, each `python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py …`, once each:
   - `list lib`;
   - `numeric retained`;
   - `list review`;
   - `review probe`, `stationary`, `motion`, `excluded`, `transitions`,
     `skinned`, `timing`, `viewport`;
   - `release`.

   All exited 0.
3. **Analysis:**
   - `analyze.py check`: exit 0, pass.
   - `analyze.py compare`: exit 1, the honest threshold failure; the record was
     retained and published.
   - `analyze.py publish`: exit 0; 57 files and 16 screenshots.
4. **Inspection:** the commands in `fixed-warmup-inspection/commands.txt`, all
   exit 0, plus the listed crops.
5. **Not done:**
   - any Cargo or target access, or retry;
   - any change to production, helpers, plan, fixture or thresholds;
   - a rerun of the 252 direct controls;
   - any extra capture;
   - any push, PR, merge, account change, subagent, memory write or gate
     approval.

## 9. Limitations

- **Desktop only.** One local desktop run on an Apple M5 Pro (Metal). It is not
  a phone, shipping-profile, device or performance measurement; timing is a
  pass-coverage inference only.
- **Thresholds are proposals.** They come from `plan.json`, not plan gates.
  The viewport case has no baseline.
- **What the fixture keeps.** It keeps final-frame PNGs, activity peaks (capped
  at 64 steps) and report statistics. So:
  - the exact pixel behind each edge-max value cannot be located;
  - the clamp/refusal explanation for the larger peaks rests on these
    observations plus the earlier CPU model.
- **The pan resolution is partly the metric.** The edge mask now covers the
  stronger sub-pixel shadow line (§5).
- **The old direct controls were not rerun.** Their earlier results cover the
  unchanged kernels only.
- **The ramp has no per-pixel age.** Pixels uncovered mid-sequence still
  restart from the one-sided registered reconstruction.
