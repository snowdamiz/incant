# 0072 result — retained motion-data GPU execution

**Status: BLOCKED before execution.** I did not invoke the runner and no GPU
child ran. The grant is still unconsumed. Reason: the supplied runner's test
filter cannot match either compiled test, so running it would consume the
one-shot grant and produce no numeric evidence.

- Model: Claude Opus 5.5 (`claude-opus-5-5`). Thinking effort: **max**. Both
  values come from this session's environment; I cannot re-verify the ACP-side
  settings from inside the session.
- Director feedback: the current brief has no appended priority revisions or new
  director feedback. I re-checked it before committing; its sha256 is
  `ebf31dd9…`, unchanged from commit `928a4ae8`.

## The defect (report for Astra)

1. `tools/run.py:27` sets `m.TEST='motion_device_tests::'+name`.
2. The audited helper passes that value with `--exact` (`run-gpu.py:34`).
3. libtest names a test by its module path, not by its file name, and this file
   is mounted as `mod motion_tests`:
   - `crates/incant_render/src/lib.rs:19`: `mod models;`
   - `crates/incant_render/src/models.rs:42-43`:
     `#[path = "retained.rs"] mod retained;`
   - `crates/incant_render/src/retained.rs:450-451`:
     `#[path = "motion_device_tests.rs"] mod motion_tests;`
4. The compiled names are therefore
   `models::retained::motion_tests::<fn>`, and neither `--exact` nor substring
   matching on `motion_device_tests::<fn>` can select them.

The frozen binary confirms this. I only read it; I did not execute it.

- **Its own earlier CPU run log:**
  `docs/spikes/evidence/retained-motion-history/motion-history-checks-001/cpu-all.log.gz`.
  The raw sha256 is `53cf1cb1…`, matching the manifest.
  - Line 4 runs `.../incant_render-90f6082e9b891519`.
  - Lines 51–52 list both tests as
    `models::retained::motion_tests::… ... ignored`.
- **The retained binary itself** (sha256 `73b32363…`):
  - It contains each full `models::retained::motion_tests::<fn>` string exactly
    once.
  - It contains no `motion_device_tests::<fn>` string.
  - `motion_device_tests` appears only as the source filename
    `motion_device_tests.rs`.

**Predicted outcome of running as-is.** This comes from the static evidence
above; I did not observe it.

- Each child selects 0 tests and exits 0 without creating a GPU device.
- The runner then writes `gpu-results.json` and `gpu-released.json`, which
  permanently blocks reuse (see `run.py:8`).
- It then fails `run.py:40`, because `'1 passed; 0 failed'` never appears.

That outcome cannot be undone, and stopping keeps the grant reusable. So I
followed the brief's "report the exact failure and stop" at the earliest point.

I did not:

- edit the hash-bound runner,
- launch the binary (not even with `--list`),
- open or take the global GPU lock,
- create a replacement grant,
- write anything under `~/.codex/incant-evidence`.

**Requested fix (root/Astra):** change `run.py:27` to
`m.TEST='models::retained::motion_tests::'+name`. Then re-hash the runner and
update or reissue the grant. Everything else in the chain below passes as-is.

## Source and binary identity; precondition audit

These checks were read-only and mirror the runner's pre-execution assertions.

| Check | Result |
|---|---|
| Grant `~/.codex/incant-evidence/0072/review-ready.json` (sha256 `247e1bc8…`) | packet `0072-motion-history-numeric` ✔ |
| Runner sha256 `00dbc8bcba33…c8e4` vs grant | match ✔ |
| `gpu-released.json` | absent ✔ |
| Manifest `retained-motion-history.json` sha256 `1e6dd3f7…3273` | ✔ |
| 22 sources + 13 unchanged baselines vs manifest | 35/35 match ✔ |
| Worktree HEAD `928a4ae8` vs source `814717935462e4ee5dbdb1fdae2738d17477f932` | differs only in the two handoff files |
| Binary `~/.codex/incant-evidence/motion-history-checks-001/binaries/incant_render-90f6082e9b891519` | sha256 `73b32363318cfcd028cccef5513994488a6d4bbbf277bee181523b7b68bc05f5`, 31,515,168 B ✔ |
| Binary features | default: `cpu-all` argv has no `--features`, and log line 4 maps this binary |
| Predecessor `timestamp-pass-002/gpu-released.json` (sha256 `96837759…`) | ✔; `owned_processes: []`, `ledger_pids_alive: []` |
| Predecessor ledger (sha256 `37cd86da…`) | ✔; 1 row, pid 24220 gone (kill/killpg signal 0) |
| Helper `cluster-grid-readback-002/run-gpu.py` sha256 `0f890ddb…` | ✔ (read, not imported or changed) |
| `~/.codex/incant-evidence/native-gpu.lock` | exists, no current holder (`lsof`) |
| `motion::admission()` with default features | `Ok`: it errors at compile time only for cooked features. A corrected run therefore cannot pass vacuously through the tests' early return. |

## Cases and outcomes

| Test (compiled name) | Outcome |
|---|---|
| `models::retained::motion_tests::accepted_pose_buffers_and_history_swap_only_after_submission` | not executed |
| `models::retained::motion_tests::skinned_and_ik_history_uses_exact_live_deformed_buffer_and_rejects_rebound_asset` | not executed |

- GPU children: 0. Frames rendered: 0.
- Process ledger and release paths: none were created.
  `~/.codex/incant-evidence/0072/` still holds only `review-ready.json`.

## Commands run (all read-only)

- `cat`/`sed`/`grep` on the brief, runner, grant, helper, spike doc, manifest,
  `motion_device_tests.rs`, and the module-chain lines.
- `git diff --stat 814717935462e4ee5dbdb1fdae2738d17477f932 HEAD`: only the two
  handoff files changed.
- `gzip -dc …/cpu-all.log.gz | grep`, plus `shasum -a 256` on the gz and raw
  streams.
- `LC_ALL=C grep -aoc <test name> <retained binary>`.
- `python3 -I -B -` (script on stdin, no files written): hashes, JSON reads, and
  signal-0 liveness probes.
- `lsof` on the lock file.
- **Not run:** `python3 -B handoffs/0072-motion-history-numeric/tools/run.py`.

## Limitations

- This packet produced no GPU, numeric-readback, adapter, appearance, phone or
  performance evidence. No gate is claimed.
- The failure prediction rests on static evidence: the binary's bytes and its
  own earlier log. It was intentionally not confirmed by spending the grant.
- Even after the fix, these tests print no adapter identity and no raw values.
  The only receipt is libtest's pass/fail on assertions over the RGBA32F
  motion, previous-depth and validity readbacks.
- Screenshots: none, and none are in scope. Model and effort are as reported by
  the session.

## Open question

After the fix, should the run reuse this evidence directory (still clean) or
use a fresh one? That is root's choice. If root prefers that the unmodified
runner be executed once anyway, the grant is intact for that.
