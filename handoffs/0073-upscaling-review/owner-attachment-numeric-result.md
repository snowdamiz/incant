# 0073 owner attachment — numeric GPU run result

**Numeric verdict: PASS.** Exactly one root-granted run was made, and it was
accepted on all ten runner checks. This verdict covers only the numeric
owner-attachment test. It makes no appearance, history-consumption,
performance, physical-memory or phone claim.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
- Root's ACP start record,
  `0073-owner-attachment-execution-supervision-001/acp-start.json`
  (`7baab672…`), reads `"gpu_authorized": true, "model": "Opus 5.5",
  "effort": "max", "verified": true`.
- Its `acp.log` header reads `Claude session verified: Opus 5.5; thinking: max`.
- No subagents, memory writes, pushes, builds or source edits.

## Authority and bindings

| Item | Value |
|---|---|
| Grant | `/Users/andreyyurlov/.codex/incant-evidence/0073-owner-attachment-gpu-001/grant.json`, `fb711be759ec19e594a4d080a76a3dbec00edadf5e40c06ec8edc8d5b145371c` (unchanged after the run) |
| Runner, `owner/owner_runner.py` | `b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34`, as reviewed, unchanged |
| Plan, `owner-attachment-plan.json` | `d7da3f1ccbe73b18a0f6d90f7da9d6c4ac586be351b04541afe7ae619c6b6512`, unchanged |
| Predecessor release, `0072-fixed-warmup-001/gpu-released.json` | `7e1a55aca025c3c980761d588bf07136a176e3d69d0afa7d3a65091970a3f091`, verified by the runner |
| Frozen binary `incant_render-288759d68012f619` (cooked-temporal, native-metalfx, profile ci) | `fc3cd75685839d4b2fa465b853925ee2841ef3b28a568cc9c5cf5e8c650693d1`, 33,339,024 B, mode 0755, identical before and after |
| Producer commit, source freeze, manifest, root CPU review, enumeration | `613ee59a…`, `95e4b830…`, `eff50118…`, `d232a224…`, `68c5434e…`, re-verified by the runner |
| Dirty `brief.md` (root's, unstaged) | `e38299ffb5716e284e71687ae02afd36eeeabc3313f03f08ade6ac64ac634165` |

## The one invocation

```
python3 -I -B handoffs/0073-upscaling-review/owner/owner_runner.py run
```

- **Invocation:** from the child checkout, with `/opt/homebrew/bin/python3`
  3.14.8. Invoked at 2026-10-13T14:36:46Z, finished 14:36:47Z; runner exit 0.
- **Locks:** packet lock and `native-gpu.lock` both taken. No refusal; no
  `refusals.jsonl` exists.
- **Predebit:** one row, written before launch and non-refundable: 1 launch,
  24 explicit queue submissions.
- **Child:** PID/PGID **20471**, in its own process group, running the plan's
  exact argv:

  ```
  incant_render-288759d68012f619 --ignored --exact \
    models::retained::motion_owner_tests::exact_integer_owner_matches_visibility_and_leaves_motion_depth_color_unchanged \
    --nocapture --test-threads=1
  ```

  - **Environment:** the plan's `PATH`, a fresh `TMPDIR`, `INCANT_EXPECT_GPU_BACKEND=Metal`
    and `RUST_BACKTRACE=0`, plus HOME inherited unchanged (value not recorded).
  - **cwd:** the fresh `tmp/`, empty afterwards.
- **Outcome:** exit 0 after **0.583 s** of the 90 s timeout. Not timed out, no
  interrupt, no exception.
- **Cleanup:** the group was SIGKILLed and the child reaped. The group was empty
  afterwards and there were no survivors.

**libtest stdout** (`e26b6fb8…`):

```
running 1 test
test models::retained::motion_owner_tests::exact_integer_owner_matches_visibility_and_leaves_motion_depth_color_unchanged ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.56s
```

**stderr** (`342b88ab…`), verbatim:

```
Incant required GPU backend: Metal; actual adapter: AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: true }
MOTION_OWNER adapter=AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: true }
MOTION_OWNER renders=12 readback_submissions=12 max_extent=32x32 source_removed=true no_images=true
```

- **Adapter:** Apple M5 Pro, integrated GPU, Metal.
- **Final marker:** present exactly once, after the adapter line. There is no
  other `MOTION_OWNER` line.

## Runner checks (all true)

1. `exit_zero`
2. `no_timeout_interrupt_exception`
3. `running_one_test`
4. `libtest_ok_1_passed_0_failed_0_ignored`
5. `no_failures_section`
6. `adapter_recorded_with_metal`
7. `adapter_then_exact_final_marker_only`
8. `binary_unchanged`
9. `no_image_files`
10. `reaped_group_empty_no_survivors`

## Release and processes

**`release.json`** (`44cadf67477c235eb9f176a6ad22c76beb857532e6dd53c421553001832c1786`):
- `released: true` and `accepted: true`;
- child and group 20471, `reaped: true`, `group_empty_after: true`;
- `survivors`, `owned_processes` and `ledger_pids_alive` all `[]`;
- it binds the ledger (`31878dfe…`) and debits (`dce5a242…`), which match the
  files.

**Afterwards:**
- `kill(20471, 0)` returns `ProcessLookupError`.
- A process scan by executable name finds nothing running the frozen binary
  and nothing in group 20471.

**Owned processes:**
- **Child:** 20471, the only one, reaped.
- **Runner:** exited 0 and was waited on by its shell.
- **Helpers:** the runner's own `ps` and `git cat-file` and my verification
  `ps` all ran through `subprocess.run`, which always waits.

**Other checks:**
- **Images:** none in the run or console directories.
- **Budget:** 1 launch, 24 explicit submissions (the marker reports 12 renders
  and 12 batched readbacks), 32×32 maximum. There was no retry, build, Cargo,
  target, extra preflight, alternate binary or image.

## Artifacts

**Raw originals, retained outside the checkout:**

| File | Bytes | SHA-256 |
|---|---|---|
| `0073-owner-attachment-gpu-001/grant.json` | 838 | `fb711be759ec19e594a4d080a76a3dbec00edadf5e40c06ec8edc8d5b145371c` |
| `…/debits.jsonl` | 478 | `dce5a242880412a6df13db4075fcf1a39352a6235b6f45b856df97da7ecb5a62` |
| `…/ledger.jsonl` | 3,550 | `31878dfebcd9b295d216985d5f70c66c61fbbbc7eb544ce2e65eb4f002192496` |
| `…/release.json` | 464 | `44cadf67477c235eb9f176a6ad22c76beb857532e6dd53c421553001832c1786` |
| `…/stdout.txt` | 243 | `e26b6fb85f82030cb5eaf52cd678d8aff9f56e3dd0cdf9e1dd34ed719a05e01a` |
| `…/stderr.txt` | 633 | `342b88abfe50be5882512bb09fe1a8a3eb86abc818b6e468afd2aa61303bc557` |
| `0073-owner-attachment-gpu-001-console/console.stdout.txt` | 716 | `2ef80d40938c990b460c27df9aa52f5ad18f641537a175f0e34f05fa8bf77001` |
| `…/console.stderr.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

`owner.lock` is empty, `tmp/` is empty, and there is no `refusals.jsonl`. The
console directory also holds `invoked-at.txt`, `finished-at.txt` and `exit.txt`.

**Committed evidence packet:**
[`owner-attachment-numeric-evidence.json`](owner-attachment-numeric-evidence.json)
(16,509 B, `ea4b9dc399ff8e77223df98bedc45cc0d64f34edff5cc13db4c2aa47407902af`).
- **Contents:** all 14 raw files embedded byte-exact as UTF-8 text, each with
  its original path, size and SHA-256. This includes root's predecessor
  release and ACP start record. It also holds the ledger's verdict, child
  record, adapter and bindings.
- **Verification:** a round trip re-encoded every embedded file to its original
  SHA-256 and size.

## What this establishes, and what stays open

**Established, numerically, for this binary and Apple M5 Pro on Metal:** the
exact owner-attachment test passed. Per root's packet, its assertions cover:
- owner texels against analytic coverage;
- first, unchanged and deformed transforms and skin palettes;
- MASK and BLEND exclusions;
- packing changes;
- failed-attempt rollback;
- exact allocation reuse;
- byte-identical motion, depth and colour with attachment on and off.

I did not inspect pixels; no images exist.

**Still open:**
- history consumption;
- native owner packing;
- temporal appearance;
- physical GPU memory;
- performance;
- reference-phone gates;
- the default-WGSL binary, compiled but deliberately not run;
- the earlier a5ae3179 visual FAIL, C7 and C9 as written, and P1.

Root alone controls any further grant.
