# 0073 owner-history numerical kernel run: result

**Verdict: FAILED, not accepted.** The one granted invocation launched the WGSL
child, which failed its test at case 3 (exit 101).
- The runner stopped as designed; the cooked-native Metal child was **not
  launched**.
- The release is clean, with no survivors. No retry, rerun, direct executable
  call, probe, build, source edit, image or appearance judgement was made.
- Root decides the next action.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `0073-owner-history-supervision-001/acp-start.json` reads
`"model": "Opus 5.5", "effort": "max", "verified": true, "gpu_authorized": true`.
Its `acp.log` header reads `Claude session verified: Opus 5.5; thinking: max`.

## Authority and identities

All of these were verified before the run and are unchanged after it.

| Item | SHA-256 |
|---|---|
| Grant `0073-owner-history-gpu-001/grant.json` | `dc75b4fed57ee536baffdf1afd0efb798b4be4125b0a5d7536ea82f7e69369b6` |
| Root preparation review `temporal-owner-gpu-preparation-root-20261013.json` | `205ead03f4526129b126272e6a6c5b73f495fa1c3f6f603d016e1a2a2457b1fa` |
| `owner_history/plan.json` | `19cdc72ce68564f24e4ec88eb732aa8a17b412d964556b5ae838b21b992168c7` |
| `owner_history/runner.py` | `bf9acf839628bf48e34af08367c5c27275551a2dd895e088fddd3c76ffcdddda` |
| Lifecycle `owner/owner_runner.py` | `b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34` |
| Binary `frozen-binaries/default` (WGSL; 32,280,400 B, 0755) | `ca9ae76e301913293c38e6d2b6d1529a523e1ca3342e44cf807d6a7c843792b4` |
| Binary `frozen-binaries/cooked-native` (33,346,448 B, 0755) | `fd965d4bef936bec79c8acac1ef2926df1b09e3749db67d7fc01ab85cbb40828` |
| Predecessor release `ssao-main229-001/gpu/released.json` | `09f03b24611984c46343f819ce54fcd816a2db5e099c88f85527c7c1ce5488a5` |
| Dirty `brief.md`, both copies | `e08f6c74…` |
| Untracked inputs: enumeration, plan, prep | `68c5434e…`, `d7da3f1c…`, `0332bad7…` |

**Before the run:**
- the evidence directory held only `grant.json`;
- no frozen binary was running;
- no `__pycache__` was written, since the runner sets `dont_write_bytecode`.

## The one invocation

```
cd .../.worktrees/0073-upscaling-review
python3 handoffs/0073-upscaling-review/owner_history/runner.py run \
  /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/owner_history/plan.json
```

**Invocation.**
- **Interpreter:** `/opt/homebrew/bin/python3` 3.14.8, run in the foreground.
- **Console output:** redirected to the sibling directory
  `0073-owner-history-gpu-001-console/`. The evidence directory itself had to
  stay fresh for the runner.
- **Timing:** invoked at 2026-10-13T16:49:35Z and finished at 16:49:36Z.
- **Runner exit:** **1**, meaning not accepted. Its stderr was empty, and its
  stdout is the release JSON.

**Processes.**
- **Runner:** one process; it exited and its shell waited for it.
- **GPU test children:** one of the maximum two.
  - **WGSL child:** PID/PGID **38551**, the plan's exact argv (`--ignored --exact
    <test> --nocapture --test-threads=1`) and the fixed four-variable environment
    with HOME passed through unchanged.
  - **Outcome:** **exit 101** after **1.262 s** of 90; not timed out or
    interrupted; reaped, with its group empty.
  - **Cooked-native Metal child:** not launched, because the runner stops on the
    first failure.
- **Helpers:** the lifecycle's `ps` calls run through `subprocess.run`, which
  waits for them.

**Adapter** (stderr and the first marker):
`AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type:
IntegratedGpu, ..., backend: Metal, ... }`.

**Markers in the child's stdout:**
- the adapter line;
- `OWNER_HISTORY case=1 center=([0.25, 0.25, 0.25, 1.0], 0.1998002, [2147483653, 0]) probes=1 baseline=false`;
- `OWNER_HISTORY case=2 center=([0.69970703, 0.69970703, 0.69970703, 1.0], 0.1998002, [2147483653, 7]) probes=1 baseline=false`;
- then `FAILED`;
- `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 193 filtered out; finished in 1.25s`.

There was **no final `OWNER_HISTORY cases=12 ...` marker**.

**Panic** (stderr, verbatim):
`panicked at crates/incant_render/src/upscaling/owner_device_tests.rs:510:17:`
`case 3 probe Probe { pixel: [8, 8], color: [0.0, 0.0, 0.0, 1.0], depth: 50.05, record: [2147483653, 0] }: [0.40478516, 0.40478516, 0.40478516, 1.0]`.

**Runner checks for the WGSL child:**

| Check | Result |
|---|---|
| `metal_adapter` | true |
| `binary_unchanged` | true |
| `no_survivors` | true |
| `no_images` | true |
| `successful_process` | false |
| `one_test` | false |
| `exact_case_sequence` | false |
| `baseline_cases` | false |
| `final_marker` | false |
| `no_test_failure` | false |

**Submissions and maps.**
- **Debited:** 1 attempt, 12 queue submissions and 12 maps, non-refundable. That
  is 12 of the 24 budgeted; the cooked-native attempt was never debited.
- **Actually executed:** not counter-verified, because the final counter marker
  never printed.
  - Case markers 1 and 2 print after their read-back assertions.
  - Case 3's probe value proves its read-back completed.
  - That evidences at least 3 submissions and 3 maps in this process. The
    fixture batches one submission and one map per case.

## Release, verified numerically

**The runner's release:** `release.json` has `released: true` (with
`released_at`), `accepted: false`, and one child. Its `owned_processes`,
`process_groups_alive`, `images` are all `[]`, and its `exception` is `null`.

**My independent checks after the run:**
- `kill(38551, 0)` and `killpg(38551, 0)` both raise `ProcessLookupError`;
- no process runs either frozen binary or `owner_history/runner.py`;
- `lsof` shows no open handle on `owner.lock` or the global `native-gpu.lock`;
- `tmp/` is empty, and no image file exists in the evidence or console
  directories;
- `release.json`'s embedded ledger, debit, child stdout and child stderr hashes
  equal the files' SHA-256s;
- the console stdout parses to exactly the `release.json` content.

| File in `0073-owner-history-gpu-001/` | Bytes | SHA-256 |
|---|---|---|
| `release.json` | 2,989 | `4d2259a09a7d8731bee81eac4d61c1ed6b621c2a05adc244a4381453ffe05928` |
| `ledger.jsonl` | 1,621 | `0e6afdd8bf0ffa9d75a72d5807c1e85cff0761b458931bdf743538be35249a64` |
| `debits.jsonl` | 231 | `6168341100661a7876ee214768f0b6d2ade09661e1563a16387d23523d7df57f` |
| `wgsl/stdout.txt` | 860 | `bf7e639747a3702f0647a928ab06d3ced68ee501aec89d2306d033620577442c` |
| `wgsl/stderr.txt` | 714 | `21c22e6b7f042bfa94944bec374d90aa218832618ed2a54d3033c09a89b70640` |
| `grant.json` (unchanged) | 1,250 | `dc75b4fed57ee536baffdf1afd0efb798b4be4125b0a5d7536ea82f7e69369b6` |
| `owner.lock` (empty) | 0 | `e3b0c442…b855` |

| File in `…-console/` | SHA-256 |
|---|---|
| `console.stdout.txt` (3,228 B) | `f36898e2296efc2b1f13364b6bdf734c289c148243a89bd002fd59b36c7e63ba` |
| `console.stderr.txt` (empty) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `exit.txt` (`1`) | `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865` |
| `invoked-at.txt` / `finished-at.txt` | `8ea1d5f4…` / `988e36c9…` |

All raw diagnostics are kept unmodified.

## What the failure shows, and what it does not

**Facts from the frozen test** (`owner_device_tests.rs`, `07b168aa…`, read as
text only):

*Case 3's fixture.*
- It is an orthographic camera at 1:1 scale: `source = output = [1,1,14,14]`,
  with zero jitter.
- Current colour is 0 everywhere. History is 0.45, and every record is
  `{5,7}` (`[P|5, 7]`).
- Owner 7 changed: class mask `classes([7])` and table `words[0][0]` bit 7.
- Owner 7 appears only at source texel [9,8], with colour 0.5. The fixture
  comment says it sits in a zero-weight tap at CENTER, so "wrong support
  counting would reuse 0.45 and the ring would allow it."
- The expected CENTER colour is 0.0, a changed-absent drop.

*The observation.*
- CENTER read back 0.40478516, which is the stale-history value 0.9 × f16(0.45)
  stored round-toward-zero.
- Case 2's 0.69970703 against the exact 0.7 shows the same truncation.
- The colour assertion (tolerance 0.001) runs first, so the depth and record at
  that probe were **not** asserted. Cases 1 and 2 passed all their assertions.

**Two hypotheses, neither verified.** I ran nothing further; root decides the next
step.
- **H1:** on Metal, `source_position()` at 1:1 scale gives a tiny positive
  `fract`, not exactly 0.
  - It computes `(8.5-1)/14*14+1`. An f32 emulation gives exactly q = 8.5 for
    IEEE division with or without FMA, and for reciprocal division with separate
    rounding.
  - Only reciprocal-multiply division combined with a fused multiply-add gives
    q = 8.500000954 and fract = 9.54e-7. Both are permitted under fast math.
  - Tap [9,8] would then have weight of about 1e-6 > 0, so owner 7 would join the
    positive-weight support.
  - The shader's rule would then treat 7 as present, admit the `{5,7}` record,
    and let the clamp ring (which includes 7's 0.5) pass 0.45. That gives 0.405.
- **H2:** the persistence or changed-table path misread the table, which also
  gives 0.405.

Both paths would write the same record `[P|5,7]`, so this run's outputs cannot
tell them apart.

**Established, for this binary and Apple M5 Pro on Metal (WGSL backend):**
- the test's case 1 (fresh/outside) and case 2 (static persistence without an
  anchor match) assertions passed;
- case 3 failed.

**Not established:** cases 4–12, and nothing at all about the cooked-native
backend.

**Still open:**
- the earlier 0072/0073 quality failures and C7, C9 and P1;
- full retained history, appearance, performance, MetalFX and fixed TAA
  ownership, mutable SSAO/effect integration, and reference-phone gates.

This is kernel/backend evidence only.
