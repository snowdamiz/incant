# 0076 result: current status

**Current stage: full-frame shadow regression coverage, complete. See
[`fullframe-addendum.md`](fullframe-addendum.md).**
- **Scope.** This completes the scoped visual check, with the inherited limits
  below.
- **Who decides.** Root makes the scoped integration decision. Nothing here
  approves a phase gate.

**Model, effort and packet:**
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet:** `brief.md` with root's revision "finish full-frame shadow regression
  coverage", sha256 `1072a326…09b0fbbeb`. It is uncommitted and left so.

**Verdicts:**
- **Original 001 comparison: still FAILED.** It is not exact, and nothing here
  converts it.
- **Determinism (`045bbee0`): same-binary variation demonstrated.**
  - Root accepts it as a separate finding.
  - Root did not adopt a modal-hash pass policy, and I withdraw my earlier
    proposal of one.
- **Full-frame coverage: executed.** Every expectation held, with one
  qualification (addendum §2).
  - **The run.** Two children, `baseline-stationary` then
    `candidate-stationary`:
    - 34 of 34 submissions and 6 of 6 PNGs;
    - every contract check true;
    - no stop and no retry;
    - the GPU released.
  - **Temporal-on.** All 16 frames, the on-f16 PNG and the activity PNG are
    byte-identical across binaries, and to 0072's accepted coverage baseline.
  - **Temporal-off.** The reference differs in 298 pixels.
    - 297 are on the far fence-post shadow lines, in cascade 3 and its blend band,
      with a maximum of 44 steps.
    - One is an invisible one-step change on the ramp, seen through a grille hole
      (cascade 1).
  - **My visual verdict: no newly introduced regression.**

**Evidence:**
- **Full-frame:**
  - helper and pre-registration in `fullframe/` (`c1cdc6f6`);
  - receipts in `fullframe/evidence/`, with `manifest.json`;
  - namespace `~/.codex/incant-evidence/0076-shadow-fullframe-001`.
- **Determinism:** `determinism/run.py` (`93ab2d85`) and `determinism/evidence/`.

**Screenshots:**
- **New:** six PNGs at 960×540 in `fullframe/screenshots/`. For both baseline
  and candidate: `stationary-off`, `stationary-on-f16` and `stationary-activity`.
- **Earlier:** in `screenshots/`.

**Open items and limitations:** see full-frame addendum §5 and determinism
addendum §6.
- **Open:**
  - the 001 strict FAIL;
  - one-frame same-binary variation;
  - thin MASK shadow fidelity;
  - temporal quality.
- **Limits:**
  - one desktop only (Apple M5 Pro, Metal);
  - only `stationary` ran in this slice.

The previous status header (`045bbee0`) follows, unchanged. Its "current stage"
line is now historical.

---

# 0076 result: current status

**Current stage: bounded same-binary determinism diagnosis. See
[`determinism-addendum.md`](determinism-addendum.md).**

**Model, effort and packet:**
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet:** `brief.md` with root's revision, sha256 `494d730c…84cbe56b`. It is
  uncommitted and left so.

**Verdicts:**
- **Original comparison: still FAILED.** It is not exact, and its thresholds are
  unchanged.
- **Determinism diagnosis: same-binary variation demonstrated.**
  - **The run.** Four children under root's grant: baseline-1, candidate-1,
    baseline-2, candidate-2. Together: 104 of 104 submissions and 8 of 8 PNGs,
    with every contract check true, no stop, no retry, and the GPU released.
  - **Baseline.** All 26 frames are identical in all three of its runs (001, -1,
    -2).
  - **Candidate.** Its on-06 differed across its runs; in its two repeats it
    equalled the baseline's.
  - **The two references.** The temporal-off frames and the final on frame are
    stable within each binary.
- **Attribution stays scoped.** It is not proven.

**Corrections.** Two statements in the original text below are corrected in
addendum §1:
- §3's claim that the source change "cannot" produce a single-frame difference;
- the claim that aggregate totals bound the size of that difference.

**Evidence:**
- **Executed wrapper:** `determinism/run.py`, commit `93ab2d85`.
- **Published evidence:** `determinism/evidence/`, with a `manifest.json` and the
  deterministic `analysis/summary.json`.
- **The run's namespace:** `~/.codex/incant-evidence/0076-shadow-determinism-001`.

**Screenshots:** none new. Every repeat PNG is byte-identical to the committed
`screenshots/`, and the varying frame has no retained image.

**Limitations:** addendum §6.

The original `6212f3b3` result follows, unchanged.

---

# 0076 result: frozen shadow receiver guard comparison

**Status: EXECUTED. Integrity PASSED. The frozen comparison FAILED because it is
not exact.**

- **The failure.** One of the 24 temporal-on frames, `on-06`, differs between the
  baseline and candidate runs. My pre-registered rule voids exact attribution, and
  I have not relaxed it.
- **Everything else passed.** That is 24 of 25 checks: every shadow-agreement,
  guard and defect check passes.
- **My visual judgement.** On full-size inspection the candidate's temporal on
  and off grille shadows agree, and I found no additional regression.

- **Run.** It ran exactly as planned and committed. There were 4 GPU children:
  `list` and `review viewport` for baseline (main224, `0aa02559`) and candidate
  (`f636a2d5`). The GPU was then released.
- **Counts.** 52 submissions against a ceiling of 60, 4 PNGs at 960×540, with no
  retry, no extra case and no stop latch.
- **What this is not.** It is not a phase-gate, performance, device or physical
  shadow-fidelity claim.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet.** `brief.md` with root's priority revision ("execute the frozen
  shadow consistency comparison"), sha256 `9a25ced3…9d445b68`. It is uncommitted
  and left so.
- **Director decisions applied:**
  - the one-line `halos: None` fix is accepted;
  - this slice tests shadow-fit consistency only;
  - alpha-tested shadow fidelity stays open;
  - the visual judgement is mine;
  - I sought no further approval, made no retry and ran no other run.
- **Earlier reports kept.**
  - [`preparation-result.md`](preparation-result.md) is the previous `result.md`,
    renamed byte-for-byte.
  - [`preparation.md`](preparation.md) is unchanged.

## 1. Execution

**Grant and frozen inputs.** I verified all of these read-only before the first
child:
- **Root's review:** `~/.codex/incant-reviews/shadow-guard-build-root-20261013.json`
  (`d37ec6e2…`), decision "accepted-frozen-shadow-comparison-builds".
- **The GPU grant:** `review-ready.json` (`18e483b3…`), with holder "Claude0076
  Opus5.5 Max".
- **Visual tree:** `8d9de3ed…`, unchanged and clean.
- **Build records:** baseline `7d38ef43…` and candidate `17e0dfc6…`.
- **Binaries.**
  - Baseline `bd81f74e…` and candidate `e693f872…`. Both are mode 0555 with
    features `default` and `physics-deterministic`.
  - Strict Clippy exited 0 for both.
  - They are byte-identical before and after every child, and again now.
- **Source coverage:** exact for both sources, at 3,141 and 3,142 inputs.
- **Checkouts:** blob-exact, at 19,496 and 19,500 files. The only extra file is
  the committed fixture copy.
- **Predecessor:** the 0073 release pin (`44cadf67…`) verifies and lists no live
  process.
- **Starting state:**
  - no latch and no debit;
  - the lease was released;
  - `native-gpu.lock` was free;
  - no GPU or Cargo process was running.

| # | Child | PID | UTC | Exit | Debit (frames / UI / PNG) | Receipt |
|---|---|---|---|---|---|---|
| 1 | `gpu.py list baseline` | 26437 | 14:49:35.58–35.89 | 0 | — | exactly the fixture's 8 cases |
| 2 | `gpu.py review baseline viewport` | 26513 | 14:49:44.53–46.91 | 0 | 26 / 0 / 2 | 4 of 4 contract checks true; `1 passed; 0 failed; … 8 filtered out` |
| 3 | `gpu.py list candidate` | 26614 | 14:49:59.94–50:00.19 | 0 | — | exactly the 8 cases |
| 4 | `gpu.py review candidate viewport` | 26727 | 14:50:06.84–09.31 | 0 | 26 / 0 / 2 | 4 of 4 true; `1 passed; 0 failed; … 8 filtered out` |
| 5 | `gpu.py release` | — | 14:50:16.11 | 0 | debited 52 / 0 / 4 in total | no stop, no owned or live process, no orphan debit |

**Ledger and latch:**
- **Ledger:** `58b77135…`. It holds 10 rows: root's 6 build rows and my 4 GPU
  children.
- **Debits:** `9e210890…`.
- **Release latch:** `68e22d6d…`.
- **Stderr:** each child printed only the adapter line. That line reads Apple M5
  Pro, Metal, integrated GPU; the host is a Mac17,9 on macOS 27.0.1 with rustc
  1.99.0.

## 2. Verdicts

| Layer | Result | Record |
|---|---|---|
| Integrity (`analyze.py check`) | **PASS**, no problems | `analysis/summary-2026-10-13T145022.235852+0000.json`, sha256 `1783b574…` |
| Comparison (`analyze.py compare`) | **FAIL**, `exact: false`. Exactness 10 of 11; guards 8 of 8; agreement 4 of 4; defect 2 of 2 | `analysis/compare-2026-10-13T145029.149628+0000.json`, sha256 `44bfb050…`, published |
| Failed check, verbatim | `exactness: temporal_on_frames_identical`: 24 frames, differing `["on-06"]` | §3 |
| My visual inspection | Temporal on/off agreement of the grille shadow in the candidate; the expected hatch-to-solid change in the off reference; **no additional regression** | §6 |

## 3. The exactness failure: one temporal-on frame, `on-06`

**What matches and what differs:**
- **Matches.** On-frames 01–05 and 07–24 are byte-identical across the two runs.
  So is the kept `viewport-on-f24.png` (`6b3194c9…` in both), with a maximum step
  of 0.
- **Differs.** Only `on-06` (jitter phase 5) has a different RGBA hash. The fixture
  keeps no image of it.

**How large the difference is.** The report's consecutive-frame tallies depend on
each run's own edge mask, because each run takes its mask from its own temporal-off
reference, and those differ by design. Summing their interior and edge parts
removes that dependence (`inspection/frame-totals.txt`):
- **Pairs not touching on-06.** All 21 such pairs (still and pan) have identical
  totals: the sum of max-channel deltas, the changed count and the maximum.
- **on-05→06:** the summed delta differs by exactly **−1 step**.
- **on-06→07:** it differs by exactly **+1 step**.
- **Other totals.** In both of those pairs the changed counts and maxima are equal.

**What that means:**
- **Size.** This is consistent with a one-step difference in as few as one pixel,
  with no trace in any later frame. Equal sums can still hide cancelling
  differences, so the totals bound the difference from below only.
- **Not the candidate's change.**
  - The candidate's shadow-fit inputs for temporal-on frames are bit-identical to
    the baseline's in every phase.
  - On-14 and on-22 share on-06's jitter phase, and on-14 shares its pose; both
    match exactly.
  - So the source change cannot produce a single-frame difference. This points to
    run-to-run variation in the temporal or GPU path.
- **Not proven.** Proving that needs a same-binary determinism repeat, which is
  not authorized. I am not requesting one here; it is listed in §11.

**Consequence.** Under the frozen rule, the comparison does **not** isolate the fix
exactly. The measurements in §4–§5 are real, but they are reported with that
qualification. **I am not calling this a certified exact fix.**

## 4. Shadow-fit consistency: measured on the final pan pose (f24)

The region is the grille-shadow region derived from 0072's diagnosis: 5,700
pixels, bounding box [440, 271, 699, 310).

| Measure (linear light) | Baseline on vs off | Candidate on vs off | Limit (calibrated before the run) |
|---|---|---|---|
| Mean gap (how much shadow) | **0.1972** | **0.00159** | ≤ 0.020 |
| Blur residual | **0.16885** | **0.00406** | ≤ 0.026 |
| Low-pass max (σ 2 px) | **0.32067** | **0.02018** | ≤ 0.086 |
| Still pose: interior max delta (report) | **96** | **19** | ≤ 66 |
| Region max step (observation) | 95 | 25 | — |
| Pan off-edge pixels > 16 steps inside the region (observation) | 216 | 0 | — |

How to read this:
- **The baseline reproduces the diagnosed mismatch.** Its values sit at the
  calibration's mismatch controls (0.197, 0.164–0.171, 0.320–0.321 and 96).
- **The candidate sits with the consistent controls.**
- **What the candidate changed.** Its temporal-on mean, 0.41347, is identical to
  the baseline's. Only the temporal-off mean moved, from 0.51503 to 0.41413. So
  the improvement comes entirely from the reference's shadow fit, not from any
  blur.

## 5. Guards: all pass

- **Outside the viewport.**
  - The maximum delta is 0 within each run.
  - All 75,015 surround pixels are byte-identical across runs in both PNGs, and
    all equal the canvas (20, 21, 25).
- **Border.**
  - The candidate's 3 px band maximum is not above the inner maximum: 1 against 19
    at the still pose, and 1 against 58 at the pan pose.
  - The off-reference change from baseline to candidate has a band maximum of 0
    against an inner maximum of 95.
- **MASK geometry.**
  - The grille-bar masks in the two off references are identical: 5,047 = 5,047
    pixels, with 0 differing.
  - The bar colours are identical too.
  - The temporal-on frames are byte-identical.

## 6. My visual assessment

**What I looked at:**
- all 4 PNGs at full size;
- every planned pair: the grille shadow, card, border crossings and difference maps;
- true-scale stacks at 4–10×. These are listed in `inspection/commands.txt` and
  kept as scratch viewing aids.

**What I found:**
- **On/off agreement, achieved.**
  - In the candidate, temporal-on and temporal-off show the **same solid grille
    shadow**: the same outline, extent and darkness.
  - The only differences are temporal smoothing of the shadow's slightly notched
    edges, which the off reference shows crisper.
  - **Profile.** Through the core (y 286–296, 8 px bands, x 440–704), every
    band of candidate-off is within 0.006 of temporal-on (0.153–0.158 against
    0.156–0.158).
  - **Difference maps.** The candidate's on/off map shows only geometric-edge
    anti-aliasing and faint banding rings. The bright grille parallelogram is
    gone, while the baseline's map still shows it.
- **The expected hatch-to-solid change, judged honestly.**
  - **Before.** The baseline's off reference showed a faint hatch with a dark top
    line, core 0.35–0.47.
  - **After.** The candidate's off reference shows a solid shadow, core
    0.153–0.158. It is identical in kind to what temporal-on already showed in
    both sources.
  - **Both are aliases.** The cascade-1 shadow map cannot resolve the one-texel
    bars: about 0.36–0.39 texel wide, with vertical bars repeating every 1.45
    texels. The 42.3%-solid grille's true average occlusion is neither the
    near-transparent hatch nor this near-full darkness.
  - **What the guard does.** It makes the alias consistent across temporal modes;
    it does not make it correct. **Alpha-tested shadow fidelity stays open.**
- **No additional regression.** Every off before/after change outside the grille
  shadow is a shadow edge re-quantized by the slightly larger cascade texels
  (`inspection/clusters-off-before-after.txt`):
  - the ramp's shadow triangle and the post's shadow line (up to 34 steps, a few
    hundred pixels);
  - a fence-post shadow line and the wall's ground shadow beside the white cube
    (up to 23);
  - the far posts' thin receding shadow lines, whose dash phase moved (up to 42,
    15 pixels).

  Shape, darkness and extent are preserved, and there is no new artifact. Lit
  ground, objects, grille bars, borders and the canvas are unchanged.
- **One inherited outlier explained.**
  - **What it is.** The candidate's pan-pose interior maximum, 58, is at
    (482, 262) on a grille bar. Temporal-on resolves a sub-pixel green fragment
    there, where the point-sampled reference shows background through a hole.
  - **Not new.** The pixel is identical in both runs' on and off frames
    (`inspection/pixels-card-bar-58.txt`), so the baseline has it too; there it
    is outranked by its 95-step grille mismatch.
  - **What it measures.** It is temporal reconstruction of MASK geometry, not
    shadow fit.

## 7. Inherited temporal-quality results (0072's viewport rules; observations only)

| Rule (limit) | Baseline | Candidate |
|---|---|---|
| Still activity ≥ 33 steps (≤ 318) | 41 | 41 |
| Saturated activity (0) | **1** | **1** |
| Frames 9–16 edge mean (≤ 5) | 2.704 | 2.685 |
| Frames 9–16 edge max (≤ 50) | **71** | **71** |
| Frames 9–16 interior changed (≤ 100) | 66 | 39 |
| Same-phase f8/f16 interior (0) | **32** | **21** |
| Pan f24 off-edge 9–16 (0) | **6** | **1** |
| Pan f24 off-edge > 16 (0) | **216** | 0 |

**Why the two runs differ on some rules:**
- Temporal-on frames 8–16 are byte-identical across runs, and the activity bins
  match exactly.
- The edge mean, interior-changed and same-phase counts differ only because each
  run's edge mask comes from its own off reference.

**The inherited resolve failures**, unchanged by this slice, in both runs:
- one saturated pixel;
- an edge max of 71;
- a same-phase interior count above 0.

**The pan off-edge rule:**
- **Baseline:** fails at > 16 steps, from the grille mismatch.
- **Candidate:** that failure is gone (0 at > 16 steps). One pixel remains at
  exactly 9 steps outside the region, which is inherited resolve behaviour.

## 8. Evidence package (in this worktree)

- **`screenshots/`:** the 4 PNGs, 960×540, copied by `analyze.py publish` and
  hash-checked against the ledger.
  - `baseline-viewport-on-f24.png`: `6b3194c903258a11…`. It is identical to the
    candidate's on PNG.
  - `baseline-viewport-off-f24.png`: `2b762c2041e901a3…`. It is byte-identical
    to 0072's temporal-off PNG from source `a143b5c0`, confirming that
    non-temporal rendering is unchanged between those sources.
  - `candidate-viewport-on-f24.png`: `6b3194c903258a11…`.
  - `candidate-viewport-off-f24.png`: `da58165787a86396…`.
- **`evidence/`:** 40 receipts and `manifest.json` (`7caedc6d…`).
  - **Contents:** the grants; the ledger, debits, latch and lease; the integrity
    summary and the **failing comparison record**; per-role build records; all
    child logs; and each role's `report.json` and `events.jsonl`.
  - **`retention.json`:** two libtest stdout logs that end with a blank line are
    kept byte-exact as deterministic gzip (`.stdout.txt.gz`). They decompress to
    the sha256 values the manifest and ledger record.
- **`inspection/`:** `inspect.py`, which reuses the frozen tools unchanged; its
  text outputs; and `commands.txt` with every command and exit code.
- **Outside the worktree:**
  - the evidence namespace `~/.codex/incant-evidence/0076-shadow-guard-review-001`,
    containing my GPU ledger rows, the latch, `captures/`, `logs/` and
    `analysis/`, including `pairs-2026-10-13T145325.268186+0000/` with the 10
    viewing aids;
  - scratch crops in `/tmp/incant0076-run/crops`. These are not screenshots and
    are not committed.

## 9. Commands

These are the commands in §1, then:

| Command | Exit | Result |
|---|---|---|
| `analyze.py check` | 0 | pass |
| `analyze.py compare` | 1 | fail, kept |
| `analyze.py pairs` | 0 | 10 viewing aids |
| `analyze.py publish` | 0 | 40 files and 4 PNGs |

The retention step and the inspection commands are in `inspection/commands.txt`;
all exited 0.

**Not done:**
- no build, Cargo command or source edit;
- no change to the frozen visual tree or binaries;
- no retry or extra case, and nothing launched after release;
- no screen capture;
- no push, PR, merge or account action.

## 10. Limitations

- **The comparison is not exact.** `on-06` differs, and the cause is inferred,
  not proven (§3).
- **Desktop only.** This is one local desktop run on an Apple M5 Pro (Metal). It
  is not a phone, device, shipping-profile or performance measurement.
- **One case, one kept pose.**
  - Only the viewport case ran.
  - The still pose is judged from report numbers only.
  - The candidate's full-frame temporal-off change in cascade 3 (+0.105%) and the
    UI, sprite and text exclusion paths were not imaged.
- **Thresholds are calibrated proposals, not plan gates.**
- **Neither shadow look is physically correct.** Alpha-tested shadow fidelity is
  open.

## 11. Open for root

These are not approval requests.
1. **Determinism.** Whether the temporal path is reproducible across processes
   needs a same-binary repeat. I have not run one, and none is preauthorized.
2. **Alpha-tested shadow fidelity** remains open, as the director decided.
3. **The full-frame temporal-off change in cascade 3** has not been imaged.
