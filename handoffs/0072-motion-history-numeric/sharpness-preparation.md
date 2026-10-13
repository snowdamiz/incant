# 0072 C-reg sharpness comparison — preparation

**Status: PREPARED.** This is preparation only.
- No GPU, capture, visual rendering, screen interaction, Cargo, build or target
  access happened.
- The evidence namespace `~/.codex/incant-evidence/0072-fixed-sharpness-001`
  does not exist.
- The earlier 229-submission / 14-PNG coverage evidence and every older result
  are unchanged.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max, as this session reports them.

**Director feedback applied:**
- **The packet.** "prepare C-reg sharpness comparison (2026-10-13)".
- **Root's mid-session update.** Candidate `a143b5c0` supersedes `0c80df4e`.
  - Its production bytes are identical; I verified this with `git diff` on the
    production paths.
  - It adds 16 numeric controls only, so the numeric total is 102 + 150 = 252.
- **The two independent reviews**, both read-only:
  - `eb5ff0a1` (production `0c80df4e`: accepted, no findings);
  - `bc1bece3` (the `a143b5c0` test-only delta: accepted).
- **What changed and what didn't.** I made bounded edits to the existing fixture
  and helpers only. There is no new framework, no production or evidence edit,
  and the brief stays uncommitted.

## 1. Does C-reg match my design (`fixed-sharpness-design.md`, `e2b499e7`)? Yes.

I read the 10-path diff `e23776df..0c80df4e` and the test-only delta
`0c80df4e..a143b5c0`.

| Design element | Implementation (identical production bytes in `0c80df4e` and `a143b5c0`) |
|---|---|
| Packed jitter delta in the free uniform slot | `jitter::delta_code`: `(8·Δx + 8) \| (8·Δy + 8) << 4`, an exact f32. **Zero is 136**, as designed (Δ = 0, not code 0). `owner.rs` derives it from the **last accepted** frame's phase, and uses 136 without previous history. That is stronger than the design's wording and correct for abandoned or unaccepted frames. |
| Reprojection subtracts the delta | `resolve.wgsl`: `old = p + m·size − dj` |
| Clear centre reprojects statically | `var m = vec4f(dj/frame.rect.zw, 1, 1)` |
| Registered history-free output | `registered_current` (`clean` per tap, bilinear at `p + j`, taps clamped to [lo, hi]) on every refusal/reset path: no history, invalid opaque motion, empty range, outside rect, zero mass. Outside-rect, non-finite and degenerate-rect returns are unchanged, as designed. |
| Identity copy | `copy.wgsl` loads `p` directly and canonicalises (finite, a > 0). |
| Unchanged ABI and admission | Same 64-byte frame, bindings, admission, clamp and reaction. The resolve and copy cooker hashes were updated. |
| Deviation | The registered fallback uses **4 fresh loads** instead of reusing the 3×3 array I suggested. It is acceptable (only refusal/reset paths pay), and no cost claim is made. |
| My six proposed numeric controls | (1) code round-trip: `tests.rs`, all 64 pairs. (2) static identity: the 64 phase-pair modes expect exactly texel (3,3). (3) registered reset: 8 modes. (4) Δ-not-subtracted negative: implied by (2). (6) raw-jittered-fallback negative: implied by (3). Plus `a143b5c0`'s 16 nonzero-jitter depth controls (one of four taps admitted, or all refused). Not explicit: (5), a double-un-jitter negative through the copy. |

## 2. Concrete design concerns

1. **My halo threshold cannot be met against an aliased reference.** The rule is
   "no pixel more than 2 steps above the temporal-off reference's local 3×3
   maximum".
   - The **accepted baseline itself fails it**: 1,048 stationary pixels and
     1,276 excluded.
   - The aliased original fails it less: 181 and 198.
   - It flags legitimate anti-aliasing of sub-pixel features that the
     point-sampled reference misses, such as green grille coverage over a grey
     wall where the reference's 3×3 has no grille texel.
   - As the brief requires, I keep it **unrelaxed**, so it will fail whatever the
     sharpness. Root should decide whether to amend it, for example to a
     baseline-relative rule. I have not changed it.
2. **Same-phase f8/f16 interior = 0** (my earlier proposal) is not met by the
   baseline (17). It is kept, and is likely to fail too.
3. **Stability risk (CPU-derived, not measured).** C-reg removes the recursive
   bilinear blur that also hid the jittered-sample residual. The predicted
   largest per-frame change at edges rises, as a fraction of contrast:

   | Feature | Baseline | C-reg |
   |---|---|---|
   | Axis-aligned steps (1-D) | 0.088 | 0.111–0.115 |
   | 45° diagonal | 0.029 | 0.130 |
   | Thin diagonal lines | 0.053–0.087 | 0.130 |

   - The GPU thresholds (33+-step activity at most 318, edge max at most 50,
     edge mean at most 5) are at risk.
   - If they fail, the next single controlled change should be a non-recursive
     current-sample reconstruction weight, not a return to the recursive blur.
4. **Colour/depth pairing (CPU-derived indicator, not measured).** In the 2-D
   derivation:
   - 0.6–1.9% of the history taps C-reg admits pair an unjittered colour texel
     with a raster depth sample from the other surface (baseline 0–0.4%);
   - no scenario leaves residue 2 px or more behind a moving diagonal bar.

   `a143b5c0`'s synthetic controls cover the gather/refusal seam. Real moving,
   rasterized silhouettes are covered only by this run's explicit pairing gate
   (§6), as review `eb5ff0a1` asks.
5. **Thin 45° lines.** The derivation gives a detail ratio of 1.11 (above the
   box ideal), a slight staircase residue from the 4 distinct sample offsets per
   axis.
6. **Offset viewports.** No numeric control covers the identity copy or
   `registered_current` at offset/odd viewport borders. The new `viewport` case
   covers them visually.

## 3. Bounded preparation changes (this commit)

| Path | Change |
|---|---|
| `visual/review/temporal_visual_review.rs` | **Baseline fixture plus one appended block.** The first 69,578 bytes are byte-identical to the baseline fixture (`9841c6b1…`, tree `b1327f3d`); the full file is `d4fc4002…`. **Added:** the `viewport` case and its helpers. It is rustfmt-clean (via stdin), but not compiled here; root's build and lint decide that. |
| `visual/plan.json` | **New names.** Namespace `0072-fixed-sharpness-001`; stages `fixed-sharpness-build`/`-visual`; candidate `a143b5c0`. **Numeric.** Two controls (102 + 150). **Cases.** 8 (the original seven, verbatim, plus `viewport`). **Budgets.** Numeric 252, visual 120, overall 372, 16 PNGs. **Comparison.** Baseline identity and thresholds. |
| `visual/tools/common.py` | Names, plus separate numeric and visual caps in the prelaunch debit |
| `visual/tools/gpu.py` | `list lib` must list both exact names; `numeric <resolve\|coverage>` runs in plan order; review cases need both numeric passes |
| `visual/tools/analyze.py` | **Integrity.** It checks both numeric children (exact test, debit) and the visual cap. **`compare` (new).** It runs only after a current passing check, writes a create-only `analysis/compare-*.json` and exits 1 on any threshold failure. **Publish.** It carries the latest comparison. |
| `visual/tools/sharpness.py`, `pngread.py` (new) | The comparison metrics (as in `temporal-coverage-inspection/`) and the exactness rules. A stdlib PNG reader adapted read-only from 0067. |
| `visual/tools/selfcheck.py`, `test_analyze.py` | New counts, the fixture-prefix proof, and comparison controls on real retained runs |
| `visual/preparation/sharpness-*.json` | Receipts (§8) |
| `fixed-sharpness/model2d.py`, `model2d-output.txt` | The 2-D CPU derivation (§7) |
| Unchanged | `build.py`, `lease.py`, `test_lifecycle.py`, `test_source.py`, all evidence, results and `brief.md` |

## 4. The `viewport` case (new)

**Scene and geometry:**
- The stationary look-dev stage, which already has axis edges, diagonals, the
  sphere, 19 thin posts and the one-texel MASK grille.
- It is drawn in an **integer-offset, odd-sized viewport `[37, 19, 885, 501]`**
  inside the unchanged 960×540 output. At full scale `FramePlan` keeps this
  placement, so it is also the temporal rect.

**Frames:**
1. Temporal-off at the still pose.
2. One owner accumulates 16 still frames (two jitter cycles).
3. Then 8 frames of lateral pan with yaw carry content across the viewport
   border on every frame.
4. Temporal-off at the final pan pose.

That is **26 submissions** and **2 PNGs** (`viewport-on-f24.png`,
`viewport-off-f24.png`).

**Contract checks:** `viewport_run_uninterrupted`,
`references_have_no_temporal_state`, `outside_viewport_matches_reference` (every
on frame equals the reference outside the rect) and `frames_match_plan`.

**Observations in the report:**
- still and pan consecutive statistics;
- same-phase f08/f16;
- border band (3 px) against inner differences;
- activity bands for still frames 9–16;
- the outside-viewport maximum.

The other required coverage comes from the existing cases:
- moving disocclusion: `motion` (object and pan);
- phase transitions: `transitions` (cut, half-scale DRS, restore);
- current-only overlays: `excluded`;
- diagonal and thin features: `stationary` regions (horizon diagonal, sphere,
  posts, grille).

## 5. Proposed bounded future run (root grants only after its audit and freezes)

**CPU, root, in the foreground:**
1. `lease.py acquire`
2. `build.py checkout` (`a143b5c0` and root's freeze)
3. `toolchain`
4. `review`
5. `lint`
6. `libtests`, only if root pins no frozen `a143b5c0` default library test
   binary
7. `lease.py release`

**GPU** (serial, foreground, one attempt each, global non-waiting
`native-gpu.lock`, 120 s per child):

| Child | Exact compiled test | Submissions | PNGs |
|---|---|---|---|
| `gpu.py list lib` | `--list --ignored` on the library test binary | 0 | 0 |
| `gpu.py numeric resolve` | `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion` (`for mode in 0..102`; one submit at line 328; 8×8) | **102** | 0 |
| `gpu.py numeric coverage` | `temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection` (2 × 5 × 15; one submit at line 266; 12×8) | **150** | 0 |
| `gpu.py list review` | `--list --ignored` on `temporal_visual_review` | 0 | 0 |
| `review probe` | `probe` | 0 | 0 |
| `review stationary` | `stationary` | 17 | 3 |
| `review motion` | `motion` | 16 | 4 |
| `review excluded` | `excluded` | 9 + 9 UI | 3 |
| `review transitions` | `transitions` | 13 | 2 |
| `review skinned` | `skinned` | 7 | 2 |
| `review timing` | `timing` | 8 | 0 |
| `review viewport` | `viewport` | 26 | 2 |
| `gpu.py release` | — | — | — |

- **Totals:** numeric **252**; visual world + UI **105** (cap 120); overall
  **357**; ceiling **372** (= 252 + 120, including failed attempts); PNGs
  **16 of 16** at 960×540.
- **Review arguments:** `--ignored --exact --nocapture --test-threads=1`.

**CPU, after the run:**
1. `analyze.py check` (fail-closed integrity)
2. `analyze.py compare` (exact comparison and thresholds)
3. my personal inspection of every PNG against the baseline and earlier captures
4. `analyze.py publish`, into `fixed-sharpness-screenshots/` and
   `fixed-sharpness-evidence/`

**Failing honestly:**
- **A failing GPU child.** It fails on a contract assert after its
  `report.json` and PNGs are written, or on an exit, timeout or interrupt.
  - This latches `stop.json`, and no further GPU child runs.
  - Outputs are retained, and the process group is killed and reaped.
  - `native-gpu.lock` is held only inside `run_child`'s `with` block, and
    `gpu.py release` still writes the latch. The lifecycle controls cover this.
- **A quality threshold.** It fails in `analyze.py compare`, which exits 1 and
  keeps its JSON. A failing comparison is still published with the evidence.

## 6. Exact baseline reuse and thresholds

**What is reused.** The accepted run's retained outputs for the seven unchanged
cases: 14 PNGs, reports and build records. Each file is verified against the
committed manifest `e337bc34…`, together with the baseline identity
(`e23776df`, tree `b1327f3d`, review binary `27756c0d`).

**Exactness**, all required; otherwise the comparison fails and nothing is
reused silently:
- the candidate commit is `a143b5c0`;
- the five temporal-off references are byte-identical (RGBA hash);
- the same adapter;
- the same toolchain and host;
- every report's review config equals the plan's (f32);
- the self-check proves the seven case definitions and the fixture prefix
  unchanged.

**What is not reused.** The `viewport` case is new and has no baseline, so it
uses absolute candidate-only bounds. Candidate outputs live only in the new
namespace and publication directories; the baseline's stay in
`temporal-coverage-*`.

**Thresholds** (design proposals plus every earlier constraint, unchanged and
not relaxed). The baseline value is context only.

| Section | Check | Baseline (accepted run) |
|---|---|---|
| Sharpness | Grille detail at least 0.50 of reference | 0.182 (fails) |
| | Effective σ at most 0.60 px in each of the 5 regions | 0.757–0.900 (fails) |
| Stability (stationary) | 33+-step activity at most 318 (2 × 159) | 159 |
| | None saturated | 0 |
| | Frames 9–16 edge mean at most 5 | 3.32 |
| | Edge max at most 50 | 44 |
| | Interior changes at most 100 | 54 |
| | Same-phase interior 0 | 17 (fails) |
| | Halo 0 (stationary and excluded) | 1,048 / 1,276 (fails; concern 1) |
| Ghosts / pairing gate | Off-edge bins no worse than the baseline: object (830, 30, 0), pan (391, 0, 0), skinned (0, 0, 0) | equal |
| | Trailing 1 px band at most 1.1 × the top-edge control | 47.7 against 44.3 |
| | **Added (stricter):** trailing band no worse than the baseline's | 47.7 |
| Exclusion | UI byte-identical in 8 phases; overlay 33+-step activity 0 | true; 0 |
| Reset | Cut f7 and DRS f9 within 1 step of the baseline | 0 / 0 |
| Registration | Global linear-light Lucas–Kanade at most 0.1 px (stationary, object, skinned, excluded) | at most 0.04 |
| Viewport (candidate only) | 33+ at most 318 and none saturated; edge mean at most 5, max at most 50, interior at most 100 (still frames 9–16); same-phase interior 0; border band no worse than inner (still and pan); pan off-edge: nothing at 9 steps or more; outside identical | — |

## 7. CPU derivations (not measurements)

**1-D model** (`fixed-sharpness/model.py`; output `model-output.txt`). Its C-reg
rules match the implementation. For axis-aligned steps:
- width 2.20 → 1.36 px (box ideal 1.32);
- grille detail 0.38 → 0.96;
- no ghost residue;
- post-reset registration unchanged.

**2-D model, new** (`fixed-sharpness/model2d.py`; output
`model2d-output.txt`; 40×40, last 8 of 64 frames). Fractions of contrast:

| Scenario | Baseline: rms / flicker / detail | C-reg: rms / flicker / detail | Pairing indicator (baseline → C-reg) |
|---|---|---|---|
| Diagonal step, 26.6° | 0.048 / 0.069 / 0.54 | **0.009** / 0.109 / **0.94** | 0.00% → 0.63% |
| Diagonal step, 45° | 0.062 / 0.029 / 0.50 | **0.015** / 0.130 / **0.93** | 0.00% → 0.63% |
| Thin line, 26.6° | 0.074 / 0.087 / 0.35 | **0.019** / 0.130 / **0.89** | 0.37% → 1.55% |
| Thin line, 45° | 0.087 / 0.053 / 0.41 | **0.033** / 0.130 / **1.11** | 0.00% → 1.88% |
| Bar at 0.5 px/frame (disocclusion) | 0.063 / 0.332; residue behind 0 | 0.068 / 0.431; residue behind **0** | 0.18% → 1.73% |
| Bar at 0.3 px/frame | 0.076 / 0.305; residue 0 | 0.064 / 0.305; residue **0** | 0.00% → 1.79% |

**Reading the table.** C-reg improves error and detail on diagonals and thin
lines by 4–5×, but raises the per-frame change (concern 3). The pairing
indicator rises without producing ghost residue (concern 4). The GPU run decides
both.

## 8. CPU validation (no build, no GPU, no grant)

| Command | Result |
|---|---|
| `python3 -I -B visual/tools/selfcheck.py --write visual/preparation/sharpness-selfcheck.json` | **`all_pass: true`** (`0e031205…`) |
| `test_analyze.py --receipt …/sharpness-analyze.json` | **19/19** (`a87f9681…`) |
| `test_lifecycle.py --receipt …/sharpness-lifecycle.json` | **14/14** (`c53c1f3d…`); re-run because the debit gained caps |
| `test_source.py --receipt …/sharpness-source.json` | **4/4** (`b688b246…`) |
| `rustfmt --edition 2024 --emit stdout < fixture` | Identical to the file |
| `python3 -I -B fixed-sharpness/model2d.py` | Reproducible output (`5c86d134…`) |

**`selfcheck` covered:**
- **Counts:** 105 within 120; 102 + 150 = 252; 357 within 372; 16 of 16 PNGs;
  GPU children at most 120 s.
- **Identities:** both exact numeric names; the namespace, stages, publication
  directories, baseline tree and candidate commit.
- **Fixture:** it extends the baseline byte for byte and adds only `viewport`;
  the seven cases are unchanged; the baseline differs from the original only at
  line 1418.
- **Fixture against plan:** it agrees for all 8 cases.
- **Refusals:** all 18 guarded commands refuse with no grant; no evidence
  directory, `git status` unchanged, lock untouched.

**`test_analyze` covered:**
- **Existing integrity controls**, at the new counts: complete runs pass and
  publish (357 and 16); empty, missing case or report, false check, failed
  child, stop latch and stale summary all fail; missing numeric fails.
- **New admission controls:** refusals at numeric 253 > 252, visual 121 > 120,
  17 PNGs, and overall 373 > 372; exactly 120 and 372 are admitted.
- **New integrity controls:** the real coverage grants (`e83f97da…`,
  `a62dbb56…`) and corrected grants are refused; the old single-numeric layout
  and a wrong numeric test fail.
- **New `compare` CLI control:** it refuses without a check, then fails honestly
  (exit 1, JSON kept and published).
- **Real-data comparison controls:**
  - the baseline against itself passes exactness except the commit, and fails
    sharpness;
  - the aliased original passes sharpness but fails stability (11,143 / 10,246
    saturated / edge max 169);
  - a one-byte-tampered baseline is not exact;
  - missing candidate outputs fail.

The comparison takes about 14 s.

## 9. Identities for root's grants

| Item | Value |
|---|---|
| **Visual tree** (pin in both grants) | **`861478d46e05287a371ee1713ea7fe9562514064`** |
| Plan | `60e8342ae3f697e28e1c7a7c876c1fcd6c1ab6faea61aed54724322ec0521f54` |
| Fixture | `d4fc40029f7e7995719bf058806f9a79936f11c8ac63e7905ac79414eb281338` (prefix = baseline `9841c6b1…`) |
| Changed or new tools | `analyze` `d666995c…`, `common` `fe4b3e71…`, `gpu` `b956b8ee…`, `selfcheck` `8ab07e8d…`, `test_analyze` `c3e44d03…`, `sharpness` `b84be748…`, `pngread` `b627e402…` |
| Unchanged tools | `build` `319430e2…`, `lease` `13acaa49…`, `test_lifecycle` `8f33f44d…`, `test_source` `483abef0…` |
| Candidate | `a143b5c0f537bd35111e40e99626f40318a385ac`. Reviews: `eb5ff0a1…` (production), `bc1bece3…` (test delta). |
| Baseline | `e23776df`; evidence manifest `e337bc34…`; root acceptance `ed99f05b…` |

**`build-ready.json`:**
- `"stage": "fixed-sharpness-build"`;
- `source_commit` `a143b5c0…`;
- root's `source_freeze {path, sha256}`;
- `visual_tree`;
- `target`;
- `holder`.

**`review-ready.json`:**
- `"stage": "fixed-sharpness-visual"`;
- `holder`;
- `visual_tree`;
- `build_record_sha256`;
- prior release pins;
- `numeric_binary {path, sha256}` for root's frozen `a143b5c0` default library
  test binary (otherwise `build.py libtests` builds it).

## Limitations

- **Not compiled or executed here.** Fixture compilation, numeric controls and
  appearance are all unproven until root's build and a future grant.
- **No measurements.** The CPU derivations are predictions from 1-D and 2-D
  models of the shader rules: axis-aligned or straight features, one scalar
  channel, synthetic scenes.
- **Two thresholds already fail on the baseline.** The halo and same-phase
  thresholds fail on the accepted baseline (concerns 1–2). Kept unrelaxed, they
  will make the comparison fail regardless of sharpness, unless root amends them.
- **Scope.** Fixed mode only: no upscaling, MetalFX, phone, performance or
  phase claim, and no approval of anything.
