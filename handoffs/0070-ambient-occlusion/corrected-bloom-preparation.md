# 0070 corrected bloom: rerun preparation (build004)

## Status

**Preparation is ready for root's review.** No build or GPU authority follows
from it.

- **Nothing was executed.** I ran no Cargo, target, GPU, window or capture.
- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**, as configured and verified by root before this prompt.
- **One helper edit was needed: re-pinning the engine source commit.**
  Everything else that decides pass or fail is unchanged: the runner, the
  analyzer, the Rust helper, every case, threshold, crop, radiometric setting
  and SSAO setting.
- **Old evidence is untouched:** the build002 and build003 directories,
  `result.md`, `corrected-gpu-result.md`, `evidence/execution-00{2,3}.json`
  and the review crops. Build003's failed centroid gate stays failed.

## What the rerun must prove (unchanged contract)

The corrected bloom must pass the **original** analyzer unchanged:

- all 27 gates;
- including `drs_bloom_halo_centroid_on_physical_emitter`, with its original
  whole-viewport centroid and 1.5 / 2.0 / 3.0 px tolerances at 16/16, 12/16
  and 8/16.

My appearance review of the 16 PNGs then follows. A centroid failure remains
a failure; it is not reframed as a different contract.

## Engine change under test

- **Source:** HEAD descends from `d2d2294e6eb3ed9c0c467a436be0f0218006e09f`,
  root's cherry-pick "anchor bloom footprint to physical output space".
- **Matches root's review:** its engine paths are byte-identical to the
  root-reviewed producer `5d02ad6c` (`git diff` empty). Receipt
  `bloom-output-space-final-root-20261013.json`, sha256 `b846384d…`,
  status `accepted_source_cpu`.
- **Delta from the old union `b633f92d`:** exactly ten files:
  - `bloom/{config.rs, mod.rs, filter.wgsl, output.wgsl}`;
  - `bloom/tests{.rs, /device.rs, /output_space.rs, /reference.rs}`;
  - `lib.rs` and `output.rs`.
- **Scope is bloom-only:**
  - `lib.rs` changes only the arguments inside the bloom-enabled
    `bloom_system.map(...)` branch;
  - `output.rs` adds the output size to the optional bloom tuple, and its
    uniform buffer exists only when bloom is on;
  - `bloom/output.wgsl` belongs to the bloom output variant.

  Paths with bloom disabled are behaviorally unchanged.
- **No test or host code outside the bloom module enables bloom.** A
  case-insensitive grep across `crates` finds only the renderer's Option-gated
  plumbing (`graph.rs`, `lib.rs`, `output.rs`). The plain `OutputPass::prepare`
  passes `None`, so with bloom off the bind group keeps its original two
  entries.

## Runner readiness

The existing `tools/runner.py` already takes a fresh evidence directory from
the grant, with its own ledger, locks and latch, and needs no code edit for
that.

**One blocker required a helper edit.** `common.source_freeze` refuses any
engine diff from `common.ENGINE_COMMIT`, which was pinned to `b633f92d`, so it
would refuse the corrected engine outright. Commit `694ebf57` contains the
minimal fix:

- `common.py`: `ENGINE_COMMIT = d2d2294e…`. This is the only functional change.
- `make_plan.py`, `plan.json`: regenerated. The diff is exactly two
  informational lines that the runner never reads:
  - `source_commit`: `b633f92d` → `d2d2294e`;
  - lib `listed_tests`: 127 → 130, from the three new CPU-only tests in
    `bloom/tests/output_space.rs`.

  All other plan content is byte-identical: ignored listings (37 / 50 / 5 / 2
  = 94), frame bounds, budgets, timeouts, numeric cases, appearance views,
  frame rule, grants and environment.

**Unchanged byte for byte:**

| File | sha256 |
|---|---|
| `runner.py` | `a5adf3df…`, root-reviewed |
| `lifecycle.py` | `574db13f…` |
| `analyze.py` | `91245f64…`, root's `fd625299` oracle |
| `tools/effects/**` | Rust helper source unchanged since build003 |
| `tools/effects/Cargo.lock` | `9dcff7a5…`, generated, untracked, preserved |

**CPU checks at `694ebf57`, all passing:**

| Check | Result |
|---|---|
| `selftest.py` | 26 of 26, including the end-to-end runner flows under the new pin |
| `mutations.py` | 4 of 4 caught |
| `analyze.py --selftest` | Pass |
| `test_radiometry.py` | 9 of 9 |
| `ssao_reference.py --selftest` | Pass |
| `listing.py` | 94 |
| `make_plan.py --check` | Exact |
| `rustfmt --check` | Clean |

## Identities to pin

| Item | Value |
|---|---|
| Source commit | **the HEAD at grant time**: `694ebf57…` plus this document's commit, and later the SSAO design commit. None of them touches engine paths, `tools/` or the plan |
| Engine identity | `ENGINE_COMMIT` `d2d2294e6eb3ed9c0c467a436be0f0218006e09f`, engine paths equal to reviewed `5d02ad6c` |
| `plan_sha256` | `fa6067e518204e8cd5b9fdae7223ec7e1eb4981d3dce2e5d38c9a5078729a028` |
| `helper_digest` | `5ec4955232b6e7b41856fa300a3b2776a5ab8b2dc487178d61853196c429cd48` (20 files, including root's `test_radiometry.py`) |
| Evidence directory | `/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004`, which does not exist yet; root creates it |

## Regression selection and bounds

**Reuse rule.** All four regression binaries will be relinked against the
changed `incant_render`, so their hashes will differ. The prior 94 GPU results
(build002, 07:57–07:58Z) stay exact-source evidence for the build002/003
binaries only, and are claimed for nothing newer.

**Relevant regression is the lib suite.** It holds all three bloom device
tests:

| Test | Frames |
|---|---|
| `actual_pyramid_nonzero_halo_resize_and_zero_alpha_control` | 6 |
| `canonical_graph_zero_gain_and_pixel_camera_bypass_and_invalid_replace` | 4 |
| `bloom_output_keeps_emitter_position_at_reduced_internal_resolution` | 3 |

`d2d2294e` changes only signatures and the expected-position formula in these
tests and adds no encode or draw, so the static counts stand. The lib suite
also holds the SSAO device test (7) and 33 non-bloom lib tests (425 frames),
including the queued-graph and output-pass lifecycle tests.

The runner runs whole suites only, so lib (37 tests, 3.06 s in build002) is
the smallest runnable unit containing the bloom tests.

**Not relevant:** `model_gpu` (50), `gpu_host-game` (5) and `gpu_host-ui` (2).
None enables bloom, and the diff leaves their paths behaviorally unchanged.
Their prior results are not claimed as fresh coverage.

**Numeric cases and appearance: all five cases plus appearance.**

- Only `lifecycle_drs`, `bloom_numeric` and the bloom and both renders exercise
  the correction.
- The other three cases are still required because the unchanged analyzer
  demands every declared case in one evidence directory, and the helper binary
  will be new.

**Proposed bounded GPU plan (8 serial foreground children):**

| # | Command | New frames | Existing frames | Watchdog |
|---|---|---|---|---|
| 1 | `list lib` | 0 | 0 | 30 s |
| 2 | `suite lib` | 20 | 425 | 240 s |
| 3–7 | `case ssao_core`, `ssao_materials`, `ssao_geometry`, `lifecycle_drs`, `bloom_numeric` | 20 + 10 + 17 + 18 + 8 = 73 | 0 | 5 × 150 s |
| 8 | `appearance` | 16 (16 PNGs) | 0 | 300 s |
| | **Total** | **109 of 128** | **425 of 8,192** | **1,320 s of 2,700 s** |

- **Children:** 8 of 16, with the 2 reserve retry slots unused.
- **Images:** PNGs 16 of 32, pairs 11 of 16, maximum 960×540.

**Optional extension (root's choice).** If root wants fresh coverage of every
relinked binary despite their unchanged paths, add `list` and `suite` for
`model_gpu`, `gpu_host-game` and `gpu_host-ui`. That is 14 children, existing
frames 8,092 of 8,192, and watchdogs 2,070 s. It still fits the reviewed plan
limits, but model_gpu adds 7,595 frames and about 48 s of GPU time to prove
nothing bloom-related. I don't recommend it.

## Grant templates

**`0070-effects-build-004/build-grant.json`** (root-written):

```json
{"format": "incant-0070-build-grant-v1", "packet": "0070-ambient-occlusion", "kind": "build",
 "source_commit": "<HEAD at grant>", "plan_sha256": "fa6067e518204e8cd5b9fdae7223ec7e1eb4981d3dce2e5d38c9a5078729a028",
 "helper_digest": "5ec4955232b6e7b41856fa300a3b2776a5ab8b2dc487178d61853196c429cd48",
 "evidence_dir": "/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004",
 "cargo_target_dir": "<root-assigned absolute target>", "max_jobs": 2,
 "approved_cargo_configs": {"/Users/andreyyurlov/.codex/worktrees/native-audio-host/incant/.cargo/config.toml": "<sha256 root reviewed, 0d3b42ee… in build003>"}}
```

**`0070-effects-build-004/gpu-grant.json`** (after the build and root's binary
review):

```json
{"format": "incant-0070-gpu-grant-v1", "packet": "0070-ambient-occlusion", "kind": "gpu",
 "source_commit": "<same HEAD>", "plan_sha256": "fa6067e5…", "helper_digest": "5ec49552…",
 "evidence_dir": "/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004",
 "build_record_sha256": "<sha256 of build/build.json>", "expected_backend": "Metal"}
```

## Exact commands

Run from the worktree root. `R` stands for:

```sh
env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B handoffs/0070-ambient-occlusion/tools/runner.py
```

`B` is `…/0070-effects-build-004/build-grant.json` and `G` is
`…/0070-effects-build-004/gpu-grant.json`.

1. **CPU build, root-supervised:**
   1. `R build --grant B`. It compiles engine tests and the helper, runs
      helper Clippy, freezes 5 binaries, and checks feature identity and the
      toolchain pin.
   2. `R release-build --grant B`.
   3. Root reviews the five frozen binary hashes: the four regression binaries
      are expected to differ from build003, and `effects_gpu` too.
2. **GPU children, serial and foreground, each only if the previous exited 0:**
   1. `R list --grant G lib`
   2. `R suite --grant G lib`
   3. `R case --grant G ssao_core`
   4. `R case --grant G ssao_materials`
   5. `R case --grant G ssao_geometry`
   6. `R case --grant G lifecycle_drs`
   7. `R case --grant G bloom_numeric`
   8. `R appearance --grant G`
3. **Release:** always `R release --grant G`, even after a failure. Then check
   that no owned or frozen process remains.
4. **Analysis:**
   `/opt/homebrew/bin/python3 -I -B handoffs/0070-ambient-occlusion/tools/analyze.py ~/.codex/incant-evidence/0070-effects-build-004`.
   Keep its real exit status and report.

## Acceptance and failure handling

- **Pass requires all of these:**
  - all 8 children succeed;
  - the analyzer exits 0 with all 27 gates;
  - my review of the 16 PNGs finds no disqualifying bloom artifact.
- **Any child or validation failure latches.** No retry runs without a precise
  root receipt.
- **A centroid failure stays a failure**, recorded next to build003's.

**Post-run diagnostics, CPU-only and not gates.** I'll repeat my build003
measurements for comparison:

- on `lifecycle_drs`'s single emitter, the bloom-only rms radius, energy and
  left/right and up/down balance at 16/12/8. Build003 measured 32.9 / 39.2 /
  46.1 px, +17% and +31% energy, with 5–10% imbalance;
- the v1/v3 appearance spread: 90 / 97 / 108 px.

An output-anchored footprint should hold these nearly constant.

## Limits

- **Separate leases:** this preparation grants nothing. The build and GPU
  leases remain separate, root-issued and root-supervised.
- **No device or performance claim:** results will be Metal on this Mac only,
  with no phone or performance claim.
- **SSAO repair is separate:** see `ssao-quality-repair.md`. This rerun keeps
  the original SSAO settings, which still carry build003's appearance failure.
