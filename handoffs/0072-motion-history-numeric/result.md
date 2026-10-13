# 0072 result: retained motion-data GPU execution

**Status: PASSED.** I ran the corrected runner once. Both exact compiled tests
ran once each and passed, and the GPU was released with no live process. No
phase or performance gate is claimed.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
- **Director feedback:** I applied priority revision 2 (brief sha256 `a8746458…`).
  - I reviewed root's runner diff. It makes three changes: the
    `models::retained::motion_tests::` filters, a check that each full name
    occurs exactly once in the binary, and a `1 passed; 0 failed` check after
    each child.
  - I checked the new grant and ran the runner once.
  - The earlier refusal is preserved byte-identical in `preflight-result.md`.

## Cases and outcomes

Both tests were run from the binary at
`/Users/andreyyurlov/.codex/incant-evidence/motion-history-checks-001/binaries/incant_render-90f6082e9b891519`.
The arguments were `<exact name> --ignored --exact --nocapture --test-threads=1`.

| Exact test | Exit | Wall time | Libtest result | Raw stdout sha256 |
|---|---|---|---|---|
| `models::retained::motion_tests::accepted_pose_buffers_and_history_swap_only_after_submission` | 0, no timeout | 0.561 s (libtest 0.52 s) | `1 passed; 0 failed; 0 ignored; 0 measured; 118 filtered out` | `790516a2…2c14` |
| `models::retained::motion_tests::skinned_and_ik_history_uses_exact_live_deformed_buffer_and_rejects_rebound_asset` | 0, no timeout | 0.389 s (libtest 0.35 s) | `1 passed; 0 failed; 0 ignored; 0 measured; 118 filtered out` | `a5902818…4811` |

- **Process IDs:** 33007 and 33009.
- **Stderr:** empty for both children.
- **Environment:** `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `HOME`, `TMPDIR=/tmp`,
  `RUST_BACKTRACE=1`.

The passing assertions read back RGBA32F motion, previous-depth and validity
values from tiny targets. There were at most 9 accepted submissions, at 65×33
and 96×64.

**Accepted-pose test.** It covers the following:

- **First frame:** there is no previous frame, and every sample is invalid.
- **Rejected zero-width target:** the sequence and the live instance buffer are
  unchanged.
- **Accepted move:** the old live buffer becomes staging, and more than 8 valid
  samples have x motion within 1e-5 of −33/(65·8) and y within 1e-6.
- **Singular transform:** the submission is rejected, and the sequence stays at 2.
- **Mirrored placement:** it produces valid non-zero motion.
- **Explicit reset:** after a reset there is no previous frame; the next frame
  has one.
- **Replacement owner:** it starts fresh and is not the same owner.

The cooked source directory is deleted before the first render.

**Skinned/IK test.** It covers the following:

- **Pose change:** the exact previous live deformed vertex buffer becomes
  staging, with its sampled vertex preserved, and the live buffer changes.
- **Motion:** valid samples have |motion| above 0.005.
- **Foreign asset generation:** rebinding to it is rejected, and the sequence
  stays at 2.
- **Rest pose plus IK chain:** the submission is accepted at sequence 3, every
  readback value is finite, and valid non-zero motion is present.

## Source and binary identity

| Item | Identity |
|---|---|
| Source | `814717935462e4ee5dbdb1fdae2738d17477f932`. Worktree HEAD differs only in handoff files. |
| Manifest | `docs/spikes/evidence/retained-motion-history.json`, sha256 `1e6dd3f7…3273` |
| Frozen files | 22 sources and 13 unchanged baselines, verified by the runner before each child and after both |
| Binary | `incant_render-90f6082e9b891519`, sha256 `73b32363318cfcd028cccef5513994488a6d4bbbf277bee181523b7b68bc05f5`, 31,515,168 B, default features. Hash checked before each child; each full test name occurs once. |
| Runner | `handoffs/0072-motion-history-numeric/tools/run.py`, sha256 `0053f161…127e` (root-owned revision 2) |
| Grant | `~/.codex/incant-evidence/0072/review-ready.json`, sha256 `2aa3c60b…dc33`. The unused first grant was preserved by root as `superseded-unexecuted-grant.json`. |
| Lifecycle helper | `cluster-grid-readback-002/run-gpu.py`, sha256 `0f890ddb…166a`. Imported unchanged with `-B`. |
| Predecessor | `timestamp-pass-002/gpu-released.json`, sha256 `96837759…be64`. Its ledger process was gone. |

## Process ledger and release

All of these files are under `/Users/andreyyurlov/.codex/incant-evidence/0072/`.

| File | sha256 |
|---|---|
| `gpu-ledger.jsonl` (2 rows, identical to `gpu-results.json`) | `99890e34…f5f9` |
| `gpu-results.json` | `0ebf163b…6900` |
| `gpu-released.json` | `7c036f73…b330` |
| `summary.json` (`passed: 2`) | `928934e5…426a` |
| `<test>-gpu.stdout` / `<test>-gpu.stderr` | as listed above; both stderr files are empty (`e3b0c442…`) |
| `runner-console.stdout` / `.stderr` (my noclobber shell capture) | `685ae8ff…cdd9` / empty |

**Release record:**

- Recorded at 2026-10-13T05:41:25.995559Z with `children: 2`.
- `owned_processes: []` and `ledger_pids_alive: []`.
- `ledger_sha256` matches the ledger file.
- Target: none (precompiled binary only).
- Host: Darwin 27.0.0 arm64, macOS 27.0.1.

**Re-checked after the run:** both PIDs and their process groups were gone, and
no process held `native-gpu.lock`.

## Commands run

- **Reads:** the revised brief, the runner diff, the new grant, and hashes.
- **Pre-run checks:** `lsof` on the lock found no holder, and a names-only
  `PYTHON*` environment check found no variables.
- **The run, executed once from the worktree root** with
  `/opt/homebrew/bin/python3` 3.14.8:
  `python3 -B handoffs/0072-motion-history-numeric/tools/run.py`. Exit code 0.
  Console output: two JSON exit lines, then
  `Both exact retained motion-data tests passed; GPU released.`
- **After the run:** hash and liveness cross-checks, all read-only.
- **Not done:** no build, Cargo target, binary launch outside the runner,
  screenshot, capture or memory write.

## Limitations

- **Adapter and backend unknown.** The tests do not print adapter or backend
  identity. The host is recorded, but which wgpu adapter ran is not. This is
  not a Metal-specific, device or backend benchmark.
- **Pass/fail only.** The receipt is libtest pass/fail on in-process assertions.
  No raw motion or depth values were dumped or retained.
- **Desktop run only.** This is a single local desktop execution. There is no
  appearance, image, phone, performance or memory evidence, and no
  shipping-profile or reference-device gate is claimed.
- **Feature set.** Only the default-feature binary was run; the cooked
  combinations are refused by design.
- **Still unimplemented.** Temporal antialiasing, jitter, color-history resolve
  and disocclusion remain unimplemented, per the spike doc.
- **Screenshots:** none, and none are in scope.
