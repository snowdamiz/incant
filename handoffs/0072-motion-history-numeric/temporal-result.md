# 0072 stage A result — frozen temporal numeric execution

**Status: PASSED on root's r3 retry.**

- Both exact compiled temporal tests ran once each, exited 0 and reported
  `1 passed; 0 failed`.
- The GPU was released with no live process.
- No phase, appearance, device or performance gate is claimed.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max.
- Both values are as this session reports them; I cannot re-verify the ACP-side
  settings from inside the session.

**Director feedback.** I applied priority revisions 3, 4 and 5:
- I ran each granted runner exactly once and preserved every failed attempt's
  evidence unchanged.
- I never re-ran an old runner or grant.
- `result.md` (the accepted motion result) is unchanged.
- Stage B preparation is reported in `preparation-result.md`.

## Attempts

| Attempt | Runner / grant | Outcome |
|---|---|---|
| original | `tools/temporal_numeric.py`, `0072-temporal/` | **Failed before launch.** The retained copy was mode `0644`, so `Popen` raised `PermissionError` (Errno 13). The release was written with 0 children. |
| r2 | `tools/temporal_numeric_r2.py`, `0072-temporal-r2/` | **Refused before the lock.** Line 22 raised `KeyError: 'pid'` on predecessor ledger row 4, a pre-launch `debit` row; the `0068-r2` ledger mixes 18 debit rows with 21 child rows. There was no attempt latch, no release, no launch and no frames. My preflight had predicted this. |
| **r3** | `tools/temporal_numeric_r3.py` (`e863d1d2…`), `0072-temporal-r3/` | **Passed.** Details below. |

Notes on the failed attempts:
- **Original.** I had missed the execute bit in my preflight. Root later made a
  byte-identical `0700` copy.
- **r2.** Only my console captures were written (stderr `662879fe…`).

## r3 cases and outcomes

Both tests ran from the binary
`/Users/andreyyurlov/.codex/incant-evidence/0072-temporal-r3/incant_render-temporal-tests`,
mode `0700`, sha256
`665e9de2750dca6ed6dcca7bdb5e25561d75a1e7e6d2592ee86fe5c2b51fed94`. It is
byte-identical to the frozen default-feature copy, which is unchanged. The
arguments were `<exact name> --ignored --exact --nocapture --test-threads=1`.

| Exact test | Frames charged before launch | Exit | Wall time | libtest |
|---|---|---|---|---|
| `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion` | 8 | 0, no timeout | 0.645 s (libtest 0.35 s) | `1 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out` |
| `models::retained::temporal_tests::temporal_color_motion_and_geometry_publish_once_and_failures_consume_no_phase` | 5 | 0, no timeout | 0.376 s (libtest 0.36 s) | `1 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out` |

**Run details:**
- **PIDs:** 53296 and 53298.
- **Stderr:** empty for both.
- **Environment:** `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `HOME`, `TMPDIR=/tmp`,
  `RUST_BACKTRACE=1`.
- **Timing:** started at 06:30:16Z.
- **Interpreter:** `/opt/homebrew/bin/python3` 3.14.8 with `-B`.

**What the passing assertions establish.** The checks below are from the test
sources:

**Resolve/copy test:** 8 synthetic 8×8 submissions through the real resolve and
copy pipelines, reading back RGBA16Float values within 0.001. Its eight modes
cover:
- accumulation of `0.375`;
- orthographic depth;
- view-space depth rejection;
- out-of-viewport motion;
- alpha-zero exclusion;
- fractional coverage passing through;
- the reactive radiance bound;
- finite half-float range.

**Retained test:** 5 accepted 65×33 retained submissions. It covers:
- phase 0 and then 1 on success;
- failed zero-width and `R32Float` targets consuming neither phase, history nor
  color;
- the staged instance swap;
- an explicit reset returning to phase 0 with a higher epoch;
- a replacement starting as a new owner;
- a fixed half-scale DRS change resetting both histories (internal 33×17).

## Identity and receipts

**Source.**
- Commit: `e73afe43ffaf78e73b725876411128d092791f5a`.
- Freeze: `06a242fd…`.
- Manifest: `docs/spikes/evidence/temporal-color-accumulation.json`
  (`2040592a…`).
- The 57 frozen source/baseline files were verified before each child and after
  both.

**Grant and chain.**
- Grant: `~/.codex/incant-evidence/0072-temporal-r3/review-ready.json`
  (`067ddab4…`).
- Predecessor: `0068-r2/gpu-released.json` (`c0ed8fe5…`). Its 21 child PIDs were
  all gone.
- Helper: `cluster-grid-readback-002/run-gpu.py` (`0f890ddb…`), unchanged.

All r3 files are under `~/.codex/incant-evidence/0072-temporal-r3/`:

| File | sha256 |
|---|---|
| `attempt-started.json` (one-shot latch, 06:30:16.625229Z) | `d571d318…2604` |
| `attempt-debits.jsonl` (8 + 5 frames, written before each launch) | `cff7ab4a…07fb` |
| `gpu-ledger.jsonl` (2 rows, identical to `gpu-results.json`) | `77cb9873…b1fc` |
| `gpu-results.json` | `0d9836c6…9373` |
| `gpu-released.json` | `0c63e503…d7e2` |
| `summary.json` (`passed: 2`, at most 13 frames) | `5945e3c5…271d` |
| resolve-test `-gpu.stdout` | `861a1e73…7e58` |
| retained-test `-gpu.stdout` | `70ce84d6…195a` |
| both `-gpu.stderr` files | empty |
| `runner-console.stdout` (my noclobber capture) | `e73a43eb…7e8f` |
| `runner-console.stderr` | empty |

**Release record:**
- Recorded at 06:30:17.676766Z with `children: 2`.
- `owned_processes: []` and `ledger_pids_alive: []`.
- The ledger and debit hashes match.
- Target: none.
- Host: Darwin 27.0.0 arm64, macOS 27.0.1.

**Re-checked afterwards:** both PIDs and their process groups were gone, and no
process held `native-gpu.lock`.

**Earlier attempts.** All of their files are preserved:
- `0072-temporal/`: release `97ff1283…`, traceback `dcaca1d9…`.
- `0072-temporal-r2/`: grant `554b5242…`, traceback `662879fe…`.

## Limitations

- **Adapter and backend unknown.** The tests print neither, so this is not a
  Metal-specific, device or backend benchmark.
- **Pass/fail only.** The receipt is libtest pass/fail of in-process assertions.
  No raw values were retained.
- **Desktop run only.** This was one local desktop run. There is no appearance,
  image, phone, performance or memory evidence, and numeric passes cannot
  establish appearance quality.
- **Scope of the implementation.** It is fixed internal-resolution accumulation,
  not temporal upscaling or MetalFX.
- **Screenshots:** none, and none are in scope.
