# 0076 addendum: full-frame stationary shadow coverage

**Status: COMPLETE.** This completes the scoped visual check of the shadow
receiver guard. The inherited limits are listed in §5.
- **Execution.** Both full-frame children ran exactly once, and each passed every
  lifecycle and contract check. The GPU is released.
- **Expectations.** Every pre-registered expectation held, with one qualification.
  The one changed cascade-1 pixel is isolated, as predicted, but it is not at a
  shadow edge. It is the sunlit ramp seen through a hole in the grille (§3).
- **My visual verdict: no newly introduced regression.** The only visible change is
  the intended re-quantization of the far fence-post shadows in cascade 3.
- **No new gate.** Root makes the scoped integration decision. This addendum
  approves no phase gate. It makes no performance, device or phone claim.

**Model, effort and packet:**
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet:** `brief.md` with root's revision "finish full-frame shadow regression
  coverage", sha256 `1072a326…09b0fbbeb`. It is uncommitted and left so.

**Root feedback applied:**
- **The 001 comparison stays FAILED.** Nothing here converts it. The same-binary
  variation from `045bbee0` stands as a separate finding.
- **No modal-hash pass policy.**
  - My determinism addendum (§5, item 1) proposed judging strict comparisons
    against the modal hash of at least three runs per binary. Root did not adopt
    that, and I withdraw it as a pass rule.
  - Here every frame is compared once. A differing frame would have been reported
    individually.
- **No threshold.** I applied no pass threshold, and I introduced none after seeing
  the results. The metrics are descriptive; the verdict is visual.

## 1. Run (root grant `228b2f20…`)

**Before execution.** I committed `fullframe/run.py` (sha256 `a6c15ede…`) in
`c1cdc6f6`, together with:
- `cascades.py` and `cascades.json`, the derivation;
- `compare.py`, the metrics;
- `expectations.md`, the expectations and inspection triggers.

The helper is a narrow adaptation of the determinism wrapper `93ab2d85`. Neither
that wrapper nor any earlier namespace was modified.

**Inputs, all read-only:**
- **Binaries:** `bd81f74e…` (baseline, main224 `0aa02559`) and `e693f872…`
  (candidate, `f636a2d5`).
- **Fixture and tools:** fixture `5955efa7…` and visual tree `8d9de3ed…`.
- **Checkouts:** the 001 checkouts. Their manifests were the same before each child
  and after release.

| Run | PID | Started (UTC) | Exit | Debit: frames / UI / PNGs | libtest | Fixture checks | Contract |
|---|---|---|---|---|---|---|---|
| `baseline-stationary` | 72350 | 15:24:04.79 | 0 | 17 / 0 / 3 | 1 passed, 0 failed, 8 filtered out | 3 of 3, `pass` | 7 of 7 |
| `candidate-stationary` | 72409 | 15:24:12.68 | 0 | 17 / 0 / 3 | 1 passed, 0 failed, 8 filtered out | 3 of 3, `pass` | 7 of 7 |

**Release** was at 15:24:22.36 UTC:
- 34 of 34 submissions, and 6 of 6 PNGs at 960×540;
- no stop, no retry and no debit without a ledger row;
- no owned or live process.

The hashes are: ledger `cf9ececa…`, debits `86f3f39f…`, release record
`81601b58…`. Nothing used the GPU after release.

## 2. Metrics

**Frames and PNGs** (`analysis/summary.json`, `17a85b6a…`):
- **Temporal-on.**
  - All 16 frames are byte-identical across binaries.
  - So are `stationary-on-f16.png` (`a540c7f5…`) and `stationary-activity.png`
    (`795b9e8a…`).
  - No variation appeared in these two runs. That is an observation, not a
    determinism claim.
- **Temporal-off.** Only the `off` reference differs: baseline `24855127…`,
  candidate `e495397b…`.
- **The accepted 0072 baseline.**
  - **The match.** The baseline's three PNGs are byte-identical to 0072's accepted
    coverage run,
    `48a688c5:handoffs/0072-motion-history-numeric/temporal-coverage-screenshots/`
    (source `e23776df`). So the candidate's temporal-on and activity PNGs match it
    too.
  - **How it was checked.** `compare.json` records the identity of the off PNG as
    `baseline_off_equals_0072_retained_off`. I checked the other two by hash after
    release.
- **Reproducible.** After release I re-ran `run.py summary` and `compare.py`. Both
  analysis files came out identical byte for byte.

**The temporal-off reference by depth class** (`analysis/compare.json`,
`4df3ceb8…`):
- Each pixel is classed by the analytic view depth of the first surface its centre
  ray meets. The grille card counts as solid.
- A step is the largest per-channel difference, in 8-bit sRGB.

| Class (view depth) | Pixels | Differing | > 2 steps | > 8 steps | Max | Pre-registered expectation |
|---|---|---|---|---|---|---|
| c0 (≤ 7.017; fit unchanged) | 143,040 | **0** | 0 | 0 | 0 | identical: held |
| c0c1 blend | 43,132 | 0 | 0 | 0 | 0 | unchanged or isolated: held |
| c1 (far plane 14.940 → 14.959) | 160,776 | **1** | 0 | 0 | 1 | isolated at shadow edges: isolated, but not at an edge (§3) |
| c1c2 blend | 7,089 | 0 | 0 | 0 | 0 | unchanged or isolated: held |
| c2 (far plane 24.726 → 24.761) | 49,043 | 0 | 0 | 0 | 0 | unchanged or isolated: held |
| c2c3 blend | 2,338 | 30 | 0 | 0 | 2 | far shadow edges: held |
| c3 (texel +0.105%, re-snapped) | 19,825 | **267** | 218 | 110 | 44 | far shadow edges: held |
| c3 fade | 521 | 0 | 0 | 0 | 0 | far shadow edges: none occurred |
| beyond 60 | 4,077 | 0 | 0 | 0 | 0 | identical: held |
| no surface | 88,559 | 0 | 0 | 0 | 0 | identical: held |

**Where the 298 differing pixels are.** They form 17 clusters of 8-connected
pixels:
- **16 clusters, 297 pixels:** the far fence-post shadow lines, within x 376–445
  and y 52–108.
  - 12 clusters are in c3 and 4 in the c2c3 band.
  - 110 of these pixels exceed 8 steps, all in c3.
- **1 cluster, 1 pixel:** (597, 253), in c1.
- **Cluster listing.** `compare.json` lists only the 15 largest clusters. I found
  the other two single pixels by calling the same `clusters()` with a longer list,
  in scratch, not committed. They are (402, 108) in c2c3 and (597, 253).
- **Borders.** The nearest differing pixel is 52 px from a frame edge.

**The fixture's own report.**
- **The edge mask moved.** The fixture derives its edge mask from the temporal-off
  reference. The mask grew from 34,327 to 34,345 pixels, and every interior/edge
  split moved with it.
- **Temporal-on pairs, mask-independent.** The 15 consecutive pairs and the
  same-phase f08/f16 pair have identical totals across binaries, as their frames
  are identical. The inherited same-phase interior count stays 17 in both.
- **Comparisons with the reference, mask-independent.**

  | Pair | Summed delta | Pixels over the report's threshold | Max |
  |---|---|---|---|
  | f01 vs off | 613,908 → 612,969 | 16,152 → 16,144 | 158 in both |
  | f16 vs off | 1,009,899 → 1,009,689 | 32,509 → 32,506 | 124 in both |

  Both are slightly smaller for the candidate, whose reference now uses the
  temporal-on fit. I draw no further conclusion from differences this small.

## 3. My visual verdict

**What I inspected:**
- both temporal-off PNGs at full size;
- the far post region at 8×, baseline above candidate, with a ×4 difference map;
- the per-pixel values of the largest clusters and of the c1 pixel.

The temporal-on and activity outputs are byte-identical, so they cannot differ.

**The farthest-cascade edges: the intended receiver re-quantization.**
- **What was already there.** Each far post casts a thin slanted line about two
  pixels thick. In both runs, each line alternates between fully dark spots and
  partly shadowed plateaus. By max channel, the dark spots are about 94–104, the
  plateaus about 130, and the lit ground 184.
- **What the candidate changes.** Every line keeps its place, its overall extent
  and its overall darkness. The alternation along it is redistributed.
- **The largest steps are those swaps:**
  - at (405, 97), dark (84, 87, 94) becomes plateau (128, 128, 130): 44 steps;
  - at (412, 88), plateau (128, 128, 130) becomes dark (86, 89, 95): 42 steps.
- **The c2c3 band.** It changes by at most 2 steps, along the nearest of these
  lines.
- **The likely cause.** The pattern is consistent with the derived cascade-3
  change: the texel grows 0.105% and the centre is re-snapped. That moves the
  shadow-map grid under shadows about two texels wide. I did not isolate the cause
  further.
- **Judgement.** No shadow appears, disappears or detaches.

**The other inspection targets:**
- **Lit surfaces.** Every differing far-ground pixel is darker than the lit ground
  in at least one binary, so it lies in a shadow footprint. The only changed pixel
  that is lit in both binaries is the c1 pixel below.
- **The c1 pixel, which fired my lit-surface trigger.**
  - **What it shows.** (597, 253) lies inside the grille card's outline, at a
    cut-out texel: (63, 109) by nearest-texel lookup. Through it you see the
    ramp, near its lower edge, at view depth 10.7, which is cascade 1.
  - **Lit.** An analytic sun ray from that point is not blocked.
  - **The change.** Only G changed, by one step: (169, 178, 189) became
    (169, 177, 189).
  - **Not at a shadow edge.** Its neighbouring hole pixels are unchanged and
    within two steps of it.
  - **Cause and visibility.** Cascade 1's far plane moved, so a changed depth
    comparison is plausible, but the cause is not isolated. The change is
    invisible.
- **The alpha-tested grille.**
  - No bar pixel changed.
  - Its shadow on the ground is pixel-identical. At this output size cascade 1
    keeps its texel and snapped centre, and only its far plane moves.
  - So the shadow's mixed solid and hatched alias is the same in both runs.
- **Scene borders:** unchanged.
- **Existing aliasing:** unchanged, apart from the far post shadow alternation
  described above.

**Verdict: no newly introduced regression.** The candidate re-quantizes the far,
sub-pixel post shadows, as intended. It changes nothing else visible and adds no
artifact.

## 4. What this completes

- **Coverage.** With this run the scoped visual check is complete. It covered:
  - the offset viewport, in the 001 run and the determinism repeats;
  - the full-frame stationary case, including the farthest cascade.
- **Decision.** I am not creating another approval step. Root makes the scoped
  integration decision.

## 5. Open (inherited, not re-judged)

- **The 001 strict comparison.** It remains FAILED as not exact.
- **One-frame same-binary variation.** It was demonstrated in the viewport runs.
  Its cause and rate are unknown, and one run per binary here cannot address it.
- **Thin MASK shadow fidelity.** The grille's shadow is an alias in both fits.
  Neither form is physically correct.
- **Temporal quality.** Flicker and the other inherited temporal failures are
  unchanged. The temporal-on outputs are byte-identical to the accepted 0072
  baseline.
- **Scope.**
  - **Hardware.** One desktop: Apple M5 Pro, Metal. This is not a phone, device
    or performance result.
  - **Cases.** Only `stationary` ran in this slice. Probe, motion, excluded,
    transitions, skinned and timing did not.

## 6. Evidence

- **Helper and pre-registration:** `fullframe/`, committed in `c1cdc6f6`:
  `run.py`, `cascades.py`, `cascades.json`, `compare.py` and `expectations.md`.
- **Receipts:** `fullframe/evidence/`, indexed by `manifest.json`. The two
  libtest stdout logs are kept as deterministic gzip copies, and the manifest
  records their raw hashes.
- **Screenshots:** `fullframe/screenshots/`, six PNGs at 960×540:
  - `baseline-stationary-{off,on-f16,activity}.png`;
  - `candidate-stationary-{off,on-f16,activity}.png`.
- **The run's namespace:** `~/.codex/incant-evidence/0076-shadow-fullframe-001`.
- **Scratch, not committed:** under `/tmp/incant0076-run/`:
  - `ffcrops/`, the 8× crops and the difference map;
  - three CPU-only listing scripts, for the cluster list, the line values and the
    ramp sun ray.
