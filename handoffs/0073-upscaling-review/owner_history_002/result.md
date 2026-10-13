# 0073 owner history 002: numeric and retained execution result

**Verdict: ACCEPTED by the runner.** All four ordered children passed:
1. numeric WGSL;
2. numeric cooked Metal;
3. retained WGSL;
4. retained cooked Metal.

The release is clean, with zero owned survivors.

**Scope.** This is numeric kernel/backend and retained host-connection evidence
only. It is **not** appearance, general temporal quality, performance, phone or
phase acceptance.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `0073-owner-history-supervision-002/acp-start.json` reads
`"model": "Opus 5.5", "effort": "max", "verified": true, "gpu_authorized": true`.
Its `acp.log` header reads `Claude session verified: Opus 5.5; thinking: max`.

## Authority and identities

Each item was verified before the run and is unchanged after it.

| Item | Value |
|---|---|
| Grant `0073-owner-history-gpu-002/grant.json` | `6fc9ad7b744e01524acf220ea4cf61aeb3d6640f34fdc94fe57e783b552524c8` |
| Root review `temporal-owner-combined-packet-root-20261013.json` | `d49a8393eafa5bdaa35791ba5a55a56814c37b85b68a05fa150ca86d441b6f34` |
| Plan `owner_history_002/plan.json` | `6a618f250e17e50015621ac9c9aef649dd12338fd23f2ac2f55b801962bdb2e0` |
| Runner `owner_history_002/runner.py` | `d89272e66a039d43911a58a04447056ccab26822a3a005429c5edf5c0221cf13` |
| Lifecycle `owner/owner_runner.py` | `b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34` |
| WGSL binary `frozen-binaries/default` | `0401132375f7a146d812e1d75dc2f085911a459f570a9cb978299f6a194ad002`, 32,321,424 B, 0555 |
| Cooked Metal binary `frozen-binaries/cooked-native` | `808243663e214ef421acd4234b3a87f31c21d485dae8dacefce65db83588510b`, 33,736,992 B, 0555 |
| Producer; source freeze | `fd5b86d03a2cabfcc9bd5a8441a137645fb858ae`; `608ad362…` |
| Shader; device test; retained test (all runner-verified) | `0618bd77…`; `e6c2028f…`; `8cd2a178…` |
| Predecessor release (GPU001; its failure stays preserved) | `4d2259a09a7d8731bee81eac4d61c1ed6b621c2a05adc244a4381453ffe05928` |
| Dirty `brief.md`, both copies | `e08f6c74…` |
| Untracked enumeration, plan, prep | `68c5434e…`, `d7da3f1c…`, `0332bad7…` |

**Before the run:**
- the evidence directory held only `grant.json`;
- no frozen binary or runner was running;
- `native-gpu.lock` had no open handle.

## The one invocation

```
cd .../.worktrees/0073-upscaling-review
python3 -B handoffs/0073-upscaling-review/owner_history_002/runner.py run \
  /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/owner_history_002/plan.json
```

- **Interpreter:** `/opt/homebrew/bin/python3` 3.14.8, run in the foreground.
- **Timing:** invoked at 2026-10-13T17:33:43Z and finished at 17:33:48Z.
- **Runner exit:** **0**.
- **Console output:** the runner's stdout (the release JSON) and its empty
  stderr were captured in the sibling `0073-owner-history-gpu-002-console/`
  directory, because the evidence directory had to stay fresh.

## Children: one runner process plus four GPU test processes, in order

| Job | Binary | PID/PGID | Exit | Seconds | Reaped / group empty | Accepted |
|---|---|---|---|---|---|---|
| `numeric-wgsl` | default | 1316 | 0 | 0.590 | yes / yes | yes |
| `numeric-cooked-metal` | cooked-native | 1323 | 0 | 2.050 | yes / yes | yes |
| `retained-wgsl` | default | 1329 | 0 | 0.975 | yes / yes | yes |
| `retained-cooked-metal` | cooked-native | 1372 | 0 | 0.834 | yes / yes | yes |

**Runner checks.**
- **Every child:** `successful_process`, `one_test`, `metal_adapter`,
  `final_marker`, `no_test_failure`, `binary_unchanged`, `no_survivors` and
  `no_images` are all true.
- **Numeric children only:** `exact_case_sequence`, `baseline_cases` (7, 8) and
  `current_only_case` (3) are also true.
- **Retained children only:** `exact_retained_lines` is also true.

**Adapter.** All four children reported the same:
`AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type:
IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend:
Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: true }`.

Each child's stderr is the backend-requirement line only, identical in all four
(`77ab8aa1…`, 282 B).

## Reached markers, verbatim, and budgets

**Numeric (WGSL and cooked).** The case lines are identical on both backends. The
final markers:
- `OWNER_HISTORY cases=12 owner_renders=12 current_only_renders=1 baseline_renders=2 queue_submits=12 readback_maps=12 max_extent=16x16 no_images=true`;
- `test result: ok. 1 passed; 0 failed; …`

Case CENTER tuples, as (colour, depth, record):

| Case | Colour | Depth | Record | Notes |
|---|---|---|---|---|
| 1 | 0.25 | 0.1998002 | `[P|5,0]` | |
| 2 | 0.69970703 | 0.1998002 | `[P|5,7]` | |
| 3 | **4.172325e-7** | 50.05 | `[P|5,7]` | `current_only=true` |
| 4 | 0.25 | 0.1998002 | `[P|5,0]` | |
| 5 | 0.69970703 | 50.05 | `[P|5, P|672]` | |
| 6 | 0.25 | 0.1998002 | `[P|5,0]` | |
| 7 | 0.33374023 | 0.1998002 | `[P|5,0]` | baseline |
| 8 | 14.53125 | 0.1998002 | `[P|5,0]` | baseline |
| 9 | 0.44995117 | 0.1998002 | `[P|5,7]` | |
| 10 | 0.47485352 | 0.1998002 | `[P|5,9]` | |
| 11 | 0.69970703 | 0.1998002 | `[P|5,0]` | probes=7 |
| 12 | 0.0 (alpha 0) | 0 | `[0,0]` | |

**Retained (WGSL and cooked).** Final marker
`RETAINED_OWNER frames=12 readback_submits=12 queue_submits=24 maps=12
max_extent=32x32 no_images=true`, then `test result: ok. 1 passed; …`.

**Budgets.**

| | Queue submissions | Maps | Basis |
|---|---|---|---|
| **Debited** | 12 + 12 + 24 + 24 = 72 | 4 × 12 = 48 | one attempt per child, charged before launch, non-refundable (`debits.jsonl`) |
| **Reached** | 12 + 12 + 24 + 24 = 72 | 4 × 12 = 48 | the children's own final markers |

No stop occurred, so every debited child was attempted and reached its marker.
The 72 are explicit queue submissions, not render passes.

## Case 3, compared with the GPU001 diagnosis

GPU001 failed case 3 with CENTER 0.40478516, and that failure stays preserved.
This run uses the anchor-only changed-owner shader (`0618bd77`) and root's
stronger current-only reference for case 3.

**Observation:** CENTER **4.172325e-7**, depth 50.05, record `[P|5,7]` on both
backends.
- The test compares the colour, depth and record planes byte-for-byte against a
  history-disabled owner reference render. They are identical, so no history was
  reused.
- CENTER also lies inside the test's analytically derived bound: colour within
  [0, 2⁻²⁰], and record `[P|5,0]` or `[P|5,7]`.

**Comparison with the CPU diagnosis oracle (`6d7fa1e4`), patched shader:**

| Class predicted by the oracle | CENTER prediction | Matches this run |
|---|---|---|
| Positive [9,8] weight (class A, about 2⁻²⁰) | `4.172325e-7` and `[P|5,7]` | **exactly** |
| Exact arithmetic | 0.0 and `[P|5,0]` | no |
| Changed-table misread (class T) | 0.405 persisted | no |

So this binary's arithmetic is consistent with class A and inconsistent with class
T. It is not proof of the instruction sequence, and the GPU001 binary was not
re-run.

The value 4.172325e-7 is 7·2⁻²⁴. That is the toward-zero f16 value of about
4.77e-7, so it is again consistent with toward-zero colour storage. This remains
an observation, not established Metal behaviour.

## Release, verified numerically

**The runner's release:** `release.json` has `released: true` (with
`released_at`), `accepted: true` and `exception: null`. Its `owned_processes`,
`process_groups_alive` and `images` are all `[]`.

**My independent checks after the run:**
- for PIDs 1316, 1323, 1329 and 1372, both `kill(pid,0)` and `killpg(pid,0)`
  raise `ProcessLookupError`;
- no process references the frozen binaries, the runner or the evidence
  directory, so no runner or ACP-run descendant remains;
- `lsof` shows no open handle on either lock;
- `tmp/` is empty, and no image exists in the evidence or console directories;
- `release.json`'s ledger, debit and all eight child stdout/stderr hashes equal
  the files;
- the console stdout parses to exactly `release.json`.

| File in `0073-owner-history-gpu-002/` | SHA-256 |
|---|---|
| `release.json` (8,531 B) | `c1bdfed0b833a1e6c275170ce9a39a34f265e22ce8d5969e9e5068239e310899` |
| `ledger.jsonl` (6,532 B) | `f3e576d63d885736ed47e35e68f25993de3c2f89ead6d07755962954f1f8a9e0` |
| `debits.jsonl` (1,050 B) | `fcdbd24c5e4238c45bca9612f250a2480d38f1b86998aa5919f458ba81be95d0` |
| `numeric-wgsl/stdout.txt` | `9b5876c0d876831c9d3b91f3f267cf0ccaa0ab1d16ba732143eca94cba7a564d` |
| `numeric-cooked-metal/stdout.txt` | `3eb245adc0de2d8626462cbdd1c42925e13df02003fbfec1cbc17766bf387d7c` |
| `retained-wgsl/stdout.txt` | `0fd457d37e5ce1e68ca375093535087057861e26d114435652d56c7824da4392` |
| `retained-cooked-metal/stdout.txt` | `eb8bf91facedfbb38a657740b1452d7e457f424c310aab8e8f90987bf5877202` |
| each child's `stderr.txt` | `77ab8aa1c98cb41d2b477cf761985c5fca6ce2881e2976e24f6bd07b5e76c8ef` |
| `grant.json`; `owner.lock` (empty) | `6fc9ad7b…`; `e3b0c442…` |
| Console: `console.stdout.txt`; `console.stderr.txt` (empty); `exit.txt` (`0`) | `a302dbc2…`; `e3b0c442…`; `9a271f2a…` |

## What this establishes, and what it does not

**Established on Apple M5 Pro (Metal), for these two frozen binaries:**
- The anchor-only owner-history resolve passed all twelve numeric fixture cases on
  both WGSL and cooked-native.
  - That includes byte-exact ordinary-resolve equality in cases 7 and 8.
  - It includes case 3's byte-exact match with the history-disabled reference.
- The retained test reached its marker on both backends. It covers twelve 32×32
  renderer frames of submitted colour, depth and record ownership, refused-frame
  isolation, change headers, camera and extent transitions, DRS changes, epoch
  cut and owner toggling.

**Not established:**
- appearance or general stale-colour quality. At the 8×8 internal extent, the
  thin-quad fixture can legitimately produce no reusable records;
- MetalFX or fixed TAA owner consumption and native packing;
- mutable SSAO or effect composition, which must invalidate persistence;
- performance, physical or queued memory, phone and phase gates.

**Preserved:**
- the GPU001 failure (`13540b9c`) and the CPU diagnosis (`6d7fa1e4`);
- every earlier visual rejection and quality failure (edge/sub-pixel, C7, C9, P1).
