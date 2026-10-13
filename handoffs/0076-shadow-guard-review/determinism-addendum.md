# 0076 addendum: wording correction and same-binary determinism diagnosis

**Status:**
- **The original comparison stays FAILED.** The strict comparison of `6212f3b3`
  is unchanged: it is not exact, and its thresholds are unchanged.
- **Same-binary variation is demonstrated.** The candidate binary produced two
  different `on-06` frames across three runs.
- **Attribution stays scoped.** Any conclusion about the source stays limited to
  what §4 states.

**Model, effort and packet:**
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Packet:** `brief.md` with root's revision "bounded same-binary determinism
  diagnosis", sha256 `494d730c…84cbe56b`. It is uncommitted and left so.

## 1. Corrections to the `6212f3b3` result

Root is right that two of my statements went beyond the evidence.

**The "cannot" claim.**
- **What I wrote.** In `result.md` §3: "the source change cannot produce a
  single-frame difference".
- **Why that overreached.** Matching frames at the same phase and pose (on-14,
  on-22) is suggestive, not a proof.
- **The correct statement.** The source analysis predicted identical temporal-on
  frames, and I found no mechanism for a one-frame difference. That does not
  establish attribution.

**The "bound" claim.**
- **What I wrote.** The `6212f3b3` commit message said the frame totals "bound it
  to a one-step difference".
- **Why that overreached.** Aggregate totals cannot bound any pixel's difference.
  Near-equal sums can hide larger, cancelling differences.
- **The correct statement.** The totals show only that `on-06` differed and that
  the two sums touching it moved by exactly one step each.

The original text is preserved unchanged in `result.md`, below its current-status
header.

## 2. What ran (root grant `78a5d7df…`, 2026-10-13 UTC)

**Frozen inputs, reused read-only.** Nothing was written to the 001 namespace.
- the baseline binary `bd81f74e…` and the candidate binary `e693f872…`;
- their build records;
- fixture `5955efa7…` and visual tree `8d9de3ed…`;
- the 001 exported checkouts, which stayed blob-exact before and after.

**The wrapper.** It is `determinism/run.py`, committed as `93ab2d85` before
running, with sha256 `1d6157f9…`. It reuses the frozen `common.py` lock, child
environment and supervise/reap primitives.

| Run | PID | UTC | Exit | Debit | Contract (7 checks) |
|---|---|---|---|---|---|
| `baseline-1` | 45616 | 15:08:25.95–27.39 | 0 | 26 / 0 UI / 2 PNG | all true; `1 passed; 0 failed` |
| `candidate-1` | 45665 | 15:08:35.32–36.73 | 0 | 26 / 0 / 2 | all true |
| `baseline-2` | 45726 | 15:08:43.13–44.47 | 0 | 26 / 0 / 2 | all true |
| `candidate-2` | 45774 | 15:08:50.32–51.70 | 0 | 26 / 0 / 2 | all true |

**Release** was at 15:09:00.27:
- 104 of 104 submissions and 8 of 8 PNGs;
- no stop, no retry and no orphan debit;
- no owned or live process.

The ledger is `da8da449…` and the debits `46c6b8de…`.

## 3. Findings

These come from `analysis/summary.json`, sha256 `d6c749c6…`. The summary is
deterministic: two runs of it produced the same bytes.

| Binary | Runs compared | Frames varying across runs | Temporal-off (off-still, off-pan) | Final on (on-24) and both PNGs |
|---|---|---|---|---|
| Baseline | 001, 1, 2 | **none**. All 26 frames identical; the three `report.json` files are byte-identical (`5b173e96…`) | stable | stable |
| Candidate | 001, 1, 2 | **`on-06` only**: run 001 gave `6bb02e4c…`, runs 1 and 2 gave `962159b2…` | stable | stable |

**Across binaries:**
- **Frames on-01 to on-24.** All are identical across binaries, except that
  candidate-001's on-06 matches no baseline run.
- **on-06 in the repeats.** Candidate-1 and candidate-2 reproduce the baseline's
  on-06 (`962159b2…`) exactly.
- **The temporal-off references.** off-still and off-pan differ in every pairing,
  and each is stable within its binary. This is the intended change.

**How candidate-001 differs.** Its report differs from the repeats in exactly
three fields:
- `frames[6].rgba_sha256`, which is on-06;
- the on-05→06 interior mean delta;
- the on-06→07 interior mean delta.

| Mask-independent total | Candidate-001 | Candidate-1 and candidate-2 |
|---|---|---|
| on-05→06 sum | 129,564 | 129,563 |
| on-06→07 sum | 84,655 | 84,656 |

- The changed counts (15,375 and 10,261) and the maxima (50 and 43) are equal.
- Every other pair is equal.
- These totals are aggregates. They bound no pixel's difference.

**The images.**
- **What is identical.** The on-f24 PNG is byte-identical across all six runs.
  The off-f24 PNG is identical within each binary: baseline `2b762c20…`,
  candidate `da581657…`.
- **Visual judgement.** Every repeat PNG is identical to the committed
  `screenshots/`, so my visual judgement from `6212f3b3` applies unchanged.
- **on-06.** The fixture keeps no image of it, so I could not inspect it.

## 4. Diagnosis (scoped)

**Demonstrated:**
- **Same-binary variation.** The identical candidate binary rendered two
  different on-06 frames in three runs. In two of the three, that frame equalled
  the baseline's.
- **What follows.** The original exactness failure can arise with no source
  difference at all.

**Not established:**
- **The cause.** I found nothing about the cause.
- **The rate.** It was 1 in 144 temporal-on frames across six runs, all in the
  candidate binary. Three runs per binary cannot tell whether the variation is
  binary-specific, rare or timing-related.

**Scoped attribution.**
- **What the repeats show.** Across the repeat pairs, the candidate's temporal-on
  output equals the baseline's. The only systematic difference is the two
  temporal-off references.
- **What that is consistent with.** The candidate changing only the temporal-off
  shadow fit.
- **What it is not.** It is an independent finding. It does not convert the
  frozen FAIL, and it does not prove attribution.

## 5. Next actionable step (root decides)

1. **Comparison protocol.** Judge any further strict comparison of temporal frames
   against a same-binary envelope: at least three runs per binary, compared per
   frame against the modal hash. A single run's byte identity is not reliable on
   this path.
2. **Localizing the variation** needs one of these:
   - retained on-05 to on-07 images from repeated same-binary runs, which is a
     fixture change outside this packet;
   - a GPU determinism test of the temporal resolve and copy path, which is
     Astra's correctness lane.

## 6. Limitations

- **Small sample.** Three runs per binary, on one desktop: Apple M5 Pro, Metal.
  It is not a phone, device or performance measurement.
- **No image of the varying frame.** The difference is known only through hashes
  and aggregate totals. Neither gives its location or per-pixel size.
- **Other lanes.** A CPU-only `cargo check` from another lane (in a different
  target) was running at preflight. It had ended before the first child launched,
  and I left it alone.
- **No retries.** No further runs were made, and none are requested here.
