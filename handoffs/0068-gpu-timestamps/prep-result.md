# 0068 revision 2 prep-result — corrected pass timestamps on current-main200

## Status

**Prepared; execution pending a fresh root grant in `0068-r2`.**

- **Grant absent.** I checked once: `/Users/andreyyurlov/.codex/incant-evidence/0068-r2/`
  does not exist. I did not poll.
- **Nothing executed.** I ran no Cargo build, owned no target, did no device or GPU
  work, made no screen capture and inspected no new pixels.
- **Previous run superseded.** The previous 0068 run is complete, and its all-zero
  timestamps are **not accepted** as a measurement.
- **Historic files intact.**
  - The previous `result.md` is now `previous-result.md`, byte-identical (SHA-256
    `0d94e74cb5c0dce55478bfa2fb880159149df0d71fca6262d8e2b16ed7d9e501`).
  - `evidence/`, `screenshots/`, `preparation/` and
    `/Users/andreyyurlov/.codex/incant-evidence/0068` are untouched.
  - The new tools write only under `0068-r2` and publish only into new
    `evidence-r2/` and `screenshots-r2/` folders.
- **For Astra's review:** tools commit `1548670fde30162b3b63f39a7ceb7537c4d833e1`. The
  exact diff and the proposed execution sequence are below.

## Brief and director feedback (acknowledged and applied)

- **Brief.** Root's uncommitted priority revision, SHA-256
  `26d8f9d3bfd53e0562478ac6ce3f961c8f507705be11126cfc5e973f88a82347`. I left it
  unchanged and uncommitted. Applied:
  - preparation only, with a new evidence directory and grant;
  - the previous result kept as `previous-result.md`;
  - the existing reviewed tools and fixture adapted, with no new framework and no
    broad survey;
  - the plan binds the immutable source and the full build-input freeze, using the
    existing archive approach;
  - the ignored-case split taken from source;
  - the numeric test chosen by its full compiled name, and it must prove that
    exactly one test ran;
  - the four off/on pairs and the bounded `owner_drop` reused;
  - a nonzero-measurement rule;
  - a four-cascade case;
  - the 24/48 bounds, including failures;
  - the global lock, debit, cleanup and environment requirements;
  - only CPU checks.
- **Not done, as instructed:** no personal or auto-memory edits, no push, PR, merge or
  credentials, no unrelated helpers and no visual-design change.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max.
- **ACP verification.** Line 2 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0068-r2-preparation-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. I read only that line.
- No subagent was used.

## Source read and bound

- **Docs read:** `docs/spikes/gpu-timestamps.md` and `docs/spikes/gpu-timestamps-main200.md`.
- **Root CPU and source acceptance:**
  `docs/spikes/evidence/gpu-timestamps-main200/final-cpu-review.json`, SHA-256
  `c9d1419a8b5ea122766f4789b9c104e37d60b6554d19f85cce52319ca3aa2e19`. This matches the
  brief.
- **Bound source.**
  - Commit `f33495b2211ac37f172cd83e2552b2041df2357f`, tree
    `bb4d232286376c85b7f34595b402df2694fdf309`.
  - Its tested head is `ed341fa5`. Since then only docs and evidence changed: there is
    no diff across the 12 build-input roots.
- **Full build-input freeze.** `docs/spikes/evidence/gpu-timestamps-main200/freeze.json`,
  SHA-256 `12f94454…67e45`, with 2,879 inputs. All match this worktree: 0 missing,
  0 differing.
- **What the checkout step will do after the grant.** It will:
  1. export the bound commit with `git archive`;
  2. verify every file against its blob, and the tree against the pin;
  3. verify the freeze file hash and count;
  4. verify every one of the 2,879 freeze entries by SHA-256.

  The grant must name exactly this source; any other commit is refused.
- **Corrected production behaviour the fixture now encodes**, all read from source:
  - Timing needs only `TIMESTAMP_QUERY`. Render and compute pass descriptors write
    each group's endpoints. There are 14 queries per slot.
  - The frame interval runs from the first instrumented beginning to the last end.
  - `Shadows` is encoded only when shadow layers exist, meaning models plus at least
    one shadowing directional light. Each shadowing light has exactly 4 cascades.
    The first cascade writes the beginning, the last writes the end, and interior
    passes have no descriptor (`range_edges`).
  - `Text` needs both glyph quads and an atlas.
  - The resolve is a second submission from the completion callback.
  - An all-zero payload returns `GpuTimingError::Unwritten`.
  - The numeric test keeps its full name and now asserts `frame.nanoseconds > 0`.
- **Compiled GPU cases on macOS default features: 92.**

  | Test binary | Ignored cases |
  |---|---|
  | Renderer library | 34 (33 + the numeric test, which the suite skips) |
  | `model_gpu` | 50 |
  | `timestamp_backend_probe` (new binary) | 1 |
  | GPU-host game | 5 |
  | GPU-host UI | 2 |

  The source has 93 `#[ignore]` attributes. `tests/persistent_cache.rs` is gated
  `#![cfg(all(unix, feature = "persistent-pipeline-cache"))]`, and its test is
  Linux-only, so it doesn't compile here. That leaves root's 92.

## Tools commit for Astra

**Commit `1548670fde30162b3b63f39a7ceb7537c4d833e1`** ("Prepare 0068 r2 timestamp review tools
for main200 pass timestamps"). It changes 9 files: 852 insertions and 331 deletions,
with only tools, plan and fixture paths. Review it with `git show 1548670f`.

```
 handoffs/0068-gpu-timestamps/plan.json             | 100 ++++++---
 handoffs/0068-gpu-timestamps/tools/analyze.py      | 145 ++++++++-----
 handoffs/0068-gpu-timestamps/tools/build.py        | 152 ++++++++------
 handoffs/0068-gpu-timestamps/tools/common.py       |  85 +++++++-
 handoffs/0068-gpu-timestamps/tools/gpu.py          | 230 ++++++++++++---------
 handoffs/0068-gpu-timestamps/tools/lease.py        |  29 ++-
 .../tools/review/gpu_timestamp_review.rs           | 222 +++++++++++++++++---
 handoffs/0068-gpu-timestamps/tools/selfcheck.py    | 134 +++++++++---
 .../0068-gpu-timestamps/tools/test_lifecycle.py    |  86 ++++++--
```

**Committed file hashes (SHA-256):**

| File | SHA-256 |
|---|---|
| `plan.json` | `8183d41e6cb5b21f8cff6d875393eea14a6a104f14b878e79e872646be3859be` |
| `review/gpu_timestamp_review.rs` | `563a3fc58bbc9c71e5455c54808a79fcdf975d5a5c376ac2b33de94bf59968ca` |
| `analyze.py` | `bd20541ec76f9d4d735e68b7b9998dfcb2312c88bcade7e0479c71dac7158aab` |
| `build.py` | `9f020a476f7a781e47d5575f653a799c1942aa38a4f80c7031ac03b44d4f8261` |
| `common.py` | `04c9b5cf80f2a81c757d423bdb87f46cc06528c288ca73b121b002a88c0ee300` |
| `gpu.py` | `852b66fee511392a027323735e36a8f2a9a2c3f0c67062264c4da9a9489d4ab5` |
| `lease.py` | `d3332e1ca326a7dc45d8a6c37a362ad6881cf2f95ff47e1c414a90036fcd838c` |
| `selfcheck.py` | `a3b1b4b15ae60152aba0d0c4a348c986cafa4199d8ca56d4fa7481bec7582e04` |
| `test_lifecycle.py` | `69d85c067d8a5f4bc0492dd9672cdc98c2e745d94ca058a0aa1b2c044e3ee0b7` |

### What changed

- **`common.py`**
  - Paths: evidence moves to `0068-r2`, with the historic `0068` path kept as a
    read-only constant. The freeze path points to the main200 freeze.
  - `grant()` accepts packet `0068-gpu-timestamps` or `0068-gpu-timestamps-r2`. It
    refuses any `source_commit` other than the plan's bound commit.
  - `exclusive()` takes `/Users/andreyyurlov/.codex/incant-evidence/native-gpu.lock`
    with a bounded 60 s non-blocking wait, never an indefinite one, then the packet
    lock. It refuses if the global lock stays held.
  - Signals: `terminations_raise()` turns SIGTERM and SIGHUP during a child into an
    `Interrupted` exception, which takes the same path as Ctrl-C.
    `signals_ignored()` protects kill and reap from a second SIGINT, SIGTERM or SIGHUP.
  - `durable_append()` does an fsync append.
- **`gpu.py`**
  - **Grant first.** It checks the grant before taking any lock or creating any file.
  - **Pre-launch debit.** The frames and submissions debit is a fsynced ledger row
    written while both locks are held. Budgets, run limits and reruns are counted from
    these debits, so a killed runner still stays charged.
  - **Process creation.** `Popen` sits inside the guarded `try`. Kill and reap are
    unconditional, and stay uninterruptible, on timeout, signals and parser or ledger
    exceptions. The rest is Astra's audited structure.
  - **Numeric test.** It is selected by its full compiled name, which must appear in
    the library listing. Only one attempt is ever allowed. A run counts only if the
    test summary shows `running 1`, exactly 1 passed, 0 ignored, and the test's own
    `test <full name> ...` line.
  - **New suite and case.** It adds the `timestamp_backend_probe` suite and the
    `cascades` review case.
  - **Retries.** Suites get one attempt and never retry.
- **`build.py`**
  - **Run group.** The same grant-first, global-lock, signal and Popen-in-`try` cleanup
    as `gpu.py`.
  - **Checkout.** It verifies the tree pin, the freeze hash and count, and all 2,879
    freeze entries, plus the build-input diff from the tested head.
  - **Builds.** `tests-render` also builds and freezes `timestamp_backend_probe`.
- **`lease.py`**
  - **Labels.** Its records carry the r2 packet and holder names.
  - **Global lock check.** At acquisition the global lock must be free.
  - **Prior receipts.** The grant's `prior_*_path`/`*_sha256` verification is
    unchanged.
- **`analyze.py`**
  - **Acceptance.** A sample is an accepted measurement only if all of these hold:
    - its labels equal the content-derived set (`Shadows` needs a shadowing light);
    - every present value is finite and every present endpoint is a nonzero counter;
    - every span is ordered;
    - the conversion matches the period;
    - the aggregate frame interval is positive.

    A group with equal nonzero endpoints is still valid.
  - **Never accepted:** typed errors (including `Unwritten`), zero payloads and
    skipped or not-exactly-one tests.
  - **Overall gate.** `measurement_accepted` requires an accepted numeric test plus
    every measured review case.
  - **Publishing.** It goes only into `evidence-r2/` and `screenshots-r2/`.
- **`review/gpu_timestamp_review.rs`** (small fixture updates)
  - **Probe:** checks `TIMESTAMP_QUERY` and records the other flags.
  - **Expectations:** `shadow_lights()` drives the `Shadows` expectation, and scene
    receipts record `shadow_lights`.
  - **Sample checks:** these add `endpoints_nonzero` and `aggregate_nonzero` to the
    acceptance.
  - **Submission counting:** each timing-on frame counts 2 submissions, the frame plus
    the renderer's resolve.
  - **`owner_drop`:** also drains the resolve chain with a second wait for all work.
  - **`Lit` fixture:** `Lit::with(shadows, local_lights)`; `Lit::new()` keeps the
    reviewed scene unchanged.
  - **New `cascades` case:** a four-cascade sun with no local lights, timing off then
    on, plus a timing-on unshadowed control.
  - **Lint:** the four style findings from 0068's lint are fixed (an unneeded `mut`,
    and `as_chunks` replacing `chunks_exact`). Behaviour is unchanged.
  - **Unchanged:** scene content, sizes, sampling and the four reused pairs.
- **`plan.json`**
  - **Source binding:** commit, tree, tested head, freeze path/hash/count, root
    acceptance hash, the 12 build-input roots and the method.
  - **Suites:** the 92-case split with its derivation.
  - **Numeric test:** its acceptance rule.
  - **Measurement:** the acceptance rule.
  - **Review cases:** per-case frames and submissions, including the new `cascades`.
  - **Budgets and retained pairs:** first-run totals, the reserve and 5 retained
    pairs (3–6).
  - **Process:** an empty amendments list, the debit and counting rules, and the
    publish targets.
- **`selfcheck.py`, `test_lifecycle.py`:** see the checks below.

## Proposed execution sequence (only after root writes `0068-r2/review-ready.json`)

All commands run as `/opt/homebrew/bin/python3 -I -B` from the worktree root.

```
handoffs/0068-gpu-timestamps/tools/lease.py acquire       # verifies grant, prior pins, idle target, free global lock
handoffs/0068-gpu-timestamps/tools/build.py checkout      # git archive f33495b2 -> 0068-r2/checkout; blob + tree + 2,879-entry freeze check
handoffs/0068-gpu-timestamps/tools/build.py toolchain
handoffs/0068-gpu-timestamps/tools/build.py tests         # --locked --offline --profile ci -j2; default features
handoffs/0068-gpu-timestamps/tools/build.py review        # helper copied only now; must reuse the identical library artifact
handoffs/0068-gpu-timestamps/tools/gpu.py list lib|model_gpu|timestamp_backend_probe|game|ui|review   # expect 34/50/1/5/2 and 6 cases
handoffs/0068-gpu-timestamps/tools/gpu.py numeric         # full compiled name; exactly one test must run; debit 6
handoffs/0068-gpu-timestamps/tools/gpu.py suite lib|model_gpu|timestamp_backend_probe|game|ui   # 33/50/1/5/2, one attempt each
handoffs/0068-gpu-timestamps/tools/gpu.py review probe|lit|sprites_text|viewport|cascades|owner_drop
handoffs/0068-gpu-timestamps/tools/analyze.py check       # CPU only; then personal inspection of every retained frame
handoffs/0068-gpu-timestamps/tools/gpu.py release
handoffs/0068-gpu-timestamps/tools/build.py lint          # optional, CPU only, still leased
handoffs/0068-gpu-timestamps/tools/lease.py release
handoffs/0068-gpu-timestamps/tools/analyze.py publish     # evidence-r2/ and screenshots-r2/ only
```

### Bounds

- **Process groups.** One bounded process group at a time, under the global
  `native-gpu.lock` (bounded wait) and the packet lock.
- **Timeouts.** Listings 60 s, existing suites 300 s, the numeric test and review cases
  120 s, builds 2,700 s.
- **Compilation.** Two compiler jobs on `incant-core-presentation-target`, used only
  after the grant.
- **Debited budgets.**

  | Item | Appearance frames | Numeric submissions |
  |---|---|---|
  | Numeric test | 0 | 6 (actual 5) |
  | `probe` | 0 | 0 |
  | `lit` | 8 | 10 |
  | `sprites_text` | 2 | 2 |
  | `viewport` | 2 | 2 |
  | `cascades` | 3 | 4 |
  | `owner_drop` | 0 | 2 |
  | **First runs** | **15 of 24** | **26 of 48** |
  | Reserve | 9 | 22 |

  The reserve covers one rerun of the largest case (`lit`), and only under the plan's
  rerun rule. Existing regression-suite frames are counted separately and never
  retried.
- **Frames and PNGs.** Every frame is 640×360, under the 960×540 maximum. There are
  10 PNGs: the four reused pairs plus `cascades-off`, `cascades-on` and
  `cascades-on-unshadowed`.

### Grant fields the tools require

- **Packet:** `"packet": "0068-gpu-timestamps"` (or `"0068-gpu-timestamps-r2"`).
- **Target:** `"target": "/Users/andreyyurlov/.codex/incant-core-presentation-target"`.
- **Source:** `"source_commit": "f33495b2211ac37f172cd83e2552b2041df2357f"`.
- **Prior receipts:** `prior_<name>_path` plus `prior_<name>_sha256` pairs, including
  one whose name contains `release`. For example, 0068's own latch
  `0068/gpu-released.json` (`eba4f536…`) and lease `0068/target-lease.json`
  (`bb32a241…`), or whichever owner released last.
- **Extra case:** `"approved_extra_cases": ["owner_drop"]` if the smoke check stays
  approved.

## CPU-only checks run this session (no Cargo, device or GPU)

- **`tools/test_lifecycle.py`** runs real `/usr/bin/true` and `/bin/sleep` children
  against a temporary lock and ledger. It never touches the real `native-gpu.lock`.
  - **Result:** **10/10 pass** in 1.5 s, run twice, before and after commit.
  - **Astra's 7 controls, adapted:** build and GPU success, timeout and SIGINT, plus
    GPU post-processing interruption. The GPU-runner controls now check one debit row
    plus one result row each.
  - **3 new controls:** build SIGTERM, GPU SIGTERM, and a GPU ledger-append failure.
    In the last one the result row can't be written, yet the exact child PID and its
    group are gone (killed with SIGKILL) and its debit stays charged.
  - No `/bin/sleep` child remained afterwards.
- **`tools/selfcheck.py`: `all_pass: true`**, run twice, before and after commit.
  - **Plan:** 17 checks pass. They cover the first runs within caps, the reserve
    arithmetic, sizes, unique PNG names, the 5 retained pairs (including the 4 reused
    ones), the library skip, the 92 total, the backend probe build and the r2
    evidence directory.
  - **Source:** 7 checks pass against git at the bound commit: tree, freeze hash, 2,879
    inputs, the freeze path, the root acceptance hash, tested-head ancestry and no
    build-input diff.
  - **Helper:** the tests, Report limits, PNG names, size, output variable and
    two-per-measured-frame counting all match the plan.
  - **Acceptance:** 11 synthetic CPU checks behave as intended.
    - Accepted: a good sample, and a group with equal nonzero endpoints.
    - Rejected: an all-zero payload, a zero aggregate, a missing group, an unexpected
      group and a NaN value.
    - The numeric test is accepted only when exactly one test ran, and rejected when
      skipped or when its samples are zero.
    - `Shadows` is expected only with a shadowing light.
  - **Refusals:** all 17 guarded commands refuse with "review-ready.json is absent".
    `0068-r2` was never created and `git status` was unchanged.
- **rustfmt, formatting only (stdin, edition 2024):** the fixture parses and is
  formatting-stable. There was no compilation; compiling has to wait for the grant.

## Limitations and open questions

- **Fixture not compiled.** It mirrors only APIs already used in the repository, and
  the previous version compiled. Any compile fix found by `build.py review` would be
  recorded and reported before any GPU child runs.
- **What the four-cascade case can prove.** From public data it can show:
  - the four-pass shadow group encodes with interior passes accepted by wgpu (an
    empty interior descriptor would fail validation);
  - pixels are identical between timing off and on;
  - both `Shadows` endpoints are written, nonzero and ordered;
  - `Shadows` is absent without a shadowing light.

  It **cannot** prove that no interior pass overwrote an endpoint, because no
  per-cascade value is public. Root's CPU `range_edges` tests cover one, two, four and
  eight passes. A `cfg(test)` per-pass observer would be needed for GPU proof. Should
  Astra add one?
- **Group ordering.** Ordering across groups, or containment within the frame, is
  recorded as an observation only. On tile-based GPUs the passes may overlap, and the
  brief forbids assuming strict barriers.
- **Feature set.** Only default features are planned, so cooked shader paths stay
  default-off. Root's CPU checks also covered a cooked configuration. Is a cooked
  appearance variant wanted? It would need its own build and a share of the 24/48
  budget.
- **Submission counting.** It is deliberately conservative: a measured frame counts
  2, so the numeric test is debited 6 although its actual count is 5.
- **Cleanup limits.** A signal during process creation itself, before `Popen`
  returns, is outside the cleanup guarantee, as is SIGKILL of the runner. The pre-launch
  debit still keeps such an attempt charged.
- **`owner_drop` scope.** It remains a no-panic/no-hang smoke check.
  Callback-after-drop ordering is not established.
- **Out of scope.** Nothing here is phone, performance, other-backend, profiler or
  phase-gate evidence.

## Changed paths (owned)

- Tools commit `1548670f`: `handoffs/0068-gpu-timestamps/{plan.json,tools/*.py,tools/review/gpu_timestamp_review.rs}`.
- Second commit:
  - `handoffs/0068-gpu-timestamps/result.md` moved to `previous-result.md`, byte-identical;
  - `handoffs/0068-gpu-timestamps/prep-result.md`, this file.
- Not committed: root's edit to `brief.md`.
