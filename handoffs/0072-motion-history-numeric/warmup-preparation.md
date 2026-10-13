# 0072 preparation: reset-ramp (fixed-warmup) comparison

**Status: preparation complete, CPU only. It is ready for root's audit and grants.**

- **Not run:** Cargo, target lease, GPU, screenshots and device execution.
- **Unchanged:** production code.
- **Plan:** the next run evaluates root's reset ramp (`296e0226`) on 110 planned
  submissions against a cap of 120:
  - the existing 105 world/UI submissions (96 world frames + 9 UI, 16 PNGs at
    960×540);
  - the one retained regression's 5 submissions.
- **Unchanged inputs:** the fixture is byte-identical and the thresholds are
  unchanged. It uses a new namespace and new publication directories.
- **Comparison:** against the accepted `e23776df` coverage baseline, and against
  the rejected `a143b5c0` sharpness run as a second exact reference.
- **CPU validation:**
  - every selfcheck section passes;
  - analyzer 22/22, lifecycle 14/14, source 4/4 and linkage 8/8;
  - all 17 guarded commands refuse with no grant, and none creates the
    namespace.
- **Not claimed:** no phase, gate, appearance or performance claim.
- **Visual tree to grant:** `648e32e9d9b1a7b85f2c2e6db2b03e7be1998e25`, from
  preparation commit `c10d01f5`. Later doc-only commits leave it unchanged.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment. I cannot
  inspect the ACP launch settings from inside the session; root's ACP
  verification is root's record.
- **Packet:** "Prepare reset-ramp comparison — 2026-10-13". This is `brief.md`,
  sha256 `899828a1…7b240`, which is root-owned and uncommitted.

## 1. Source identities (verified independently, read-only)

| Item | Identity |
|---|---|
| Candidate | `296e02268458afea21ff90e8aea820a3dec2b314` "Ramp temporal history after owner reset" (Built-by: astra), tree `19e3a512`. It is in this worktree's object store (shared `.git`). |
| Base (rejected run) | `a143b5c0f537bd35111e40e99626f40318a385ac`. `git diff --name-only a143b5c0 296e0226` lists exactly three sources and three docs (see below). |
| Accepted baseline | `e23776df`: coverage run `9927598c`, visual tree `b1327f3d`, manifest `e337bc34…` |
| Changed sources (SHA-256 of the committed blobs) | `temporal/owner.rs` `f9fb45b3…f201`; `temporal/system.rs` `96742ca9…b5ae2`; `temporal_retained_tests.rs` `eed7c13c…4394`. These equal root's `fixed-temporal-warmup-001/source.json`, and `Cargo.lock` `68c187db…` is unchanged. |
| Docs | `docs/spikes/fixed-temporal-warmup.md` `887adfe8…`; `docs/spikes/evidence/fixed-temporal-warmup.json` `62e6d115…`; plus the records archive (`.json.gz`) |
| Root qualification | `~/.codex/incant-evidence/fixed-temporal-warmup-001`: `source.json` `1ebb7ebc…`, `checks.json` `8cd46116…`. See the note below the table. |
| Prior evidence audit | `~/.codex/incant-reviews/fixed-sharpness-final-root-20261013.json` sha256 `1dd3d56f…6c77`, as the brief states. It records 357 submissions, 15 reaped children and `quality_accepted: false` with the 13 failures. |

**Root qualification results:**
- **Lib tests:** 133 passed (default features) and 141 passed
  (`cooked-materials`), with 41 ignored in each run. The log hashes match.
- **Static checks:** strict all-target Clippy, `fmt --check`, the conventions
  check and `git diff --check` all exit 0.

**The ramp (read from the three diffs and the spike doc):**
- **Sample count.** `TemporalFrame.samples` (u8) counts accepted colour samples
  since the latest owner reset. It advances only in `TemporalState::accept`, so
  preparation or failed submissions never publish it.
  - A frame without previous history restarts it at 1.
  - A bypassed frame sets it to 0.
  - It saturates at 100.
- **Weight.** `TemporalSystem::prepare` writes `options.x =
  Pending::history_weight()`. That is 0 for samples ≤ 1, otherwise
  `min(configured, (samples − 1)/samples)`.
- **Unchanged:** shader source, bindings, limits and the configured 0.9.
- **Root's CPU tests:** three new ones (`warmup_tests`).

## 2. The numeric regression, confirmed from source

**The brief's name is the file name, not the libtest path.** The exact path,
traced in `296e0226` (these lines are unchanged from `a143b5c0`), is:

1. `lib.rs:20` `mod models;`
2. `models.rs:42-43` `#[path = "retained.rs"] mod retained;`
3. `retained.rs:640-642` `#[cfg(test)] #[path = "temporal_retained_tests.rs"] mod temporal_tests;`

That gives **`models::retained::temporal_tests::temporal_color_motion_and_geometry_publish_once_and_failures_consume_no_phase`**.
`plan.json` and `gpu.py list lib` use this name; nothing broader is filtered.

| Property | Measured (`selfcheck.py` `numeric_source`) |
|---|---|
| Attributes | `#[test]` `#[ignore = "requires native GPU; Claude owns bounded numeric execution"]`; it is the only test in the file |
| Accepted submit calls | **5**: lines 84, 124, 143 (retained owner), 151 (replacement owner), 176 (after the internal-scale/DRS change) |
| Refused submit calls | **3**: line 97 (zero-width target), 102 (`R32Float` output format, lines 100–111), 139 (singular world after `invalidate_motion_history`) |
| GPU submissions | One `queue.submit` per accepted call (`retained.rs:584/586`, one per feature cfg; default features use 586). Every refusal returns `Err` from validation, motion or temporal preparation, or `draw_scene_scaled` before any `queue.write_buffer` or `queue.submit`. `enable_temporal_accumulation` only creates pipelines; `accepted_color()` returns a view (no readback). So the budget is **5**. |
| Ramp assertions | 7 `accepted_samples()` assertions: 1, 1, 2, 2, 1, 1, 1. That is 1 after the first submission, still 1 after both refused candidates, 2 after the second accepted submission, still 2 after the refused singular world, and 1 after each of the reset, replacement and DRS. |
| Limit | If `crate::motion::admission()` fails, the test returns early and still passes. The integrity check also requires the probe case's `temporal_pipelines_enabled` on the same adapter, so a vacuous pass cannot produce a passing run. |

## 3. The 252 direct shader controls: retained, not rerun

The plan's `numeric.retained_direct_controls` records which earlier results
still cover which unchanged sources:

- **Source identity.** These blob and tree IDs are identical in `a143b5c0` and
  `296e0226`:

  | Path | ID |
  |---|---|
  | `resolve.wgsl` | `fca0f949` |
  | `copy.wgsl` | `70890d11` |
  | `temporal_shader.rs` | `ad72aec9` |
  | `crates/incant_shader_cook` (tree) | `06af86ec` |
  | `device_tests.rs` | `1d871bf3` |
  | `coverage_device.rs` | `ee6c5827` |
  | `jitter.rs` | `5521d0e7` |
  | `targets.rs` | `c8d4597c` |

- **Prior results**, from the `a143b5c0` binary `e62cf78c…`:
  - `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion`:
    102 submissions, PID 16633, exit 0, log `59762f01…`;
  - `temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection`:
    150 submissions, PID 16838, exit 0, log `bf25b40c…`.

  Both report `1 passed; … 170 filtered out`.
- **What they do and don't cover.** Both build their own resolve `Frame`
  uniform: options `[0.5, …]` at `device_tests.rs:171-179`, and
  `[CONFIG.history_weight, …]` at `coverage_device.rs:216-220`. They drive
  `system.resolve` directly and never call `TemporalSystem::prepare` or
  `Pending::history_weight()`; the selfcheck verifies this. **They do not
  exercise the new host policy, and the old binary has no ramp.** Their prior
  results remain evidence for the unchanged kernels only.

## 4. Measured planned counts

| Item | Count | Source of the number |
|---|---|---|
| World frames | 96 | `Report::open` limits in the fixture (selfcheck `helper.observed.frames`), equal to the plan |
| UI submissions | 9 | same (`ui_submissions`) |
| PNGs | 16, at 960×540 | `report.png(...)` names in the fixture, equal to the plan |
| Numeric | 5 | accepted submit calls in the candidate's test source |
| **Planned** | **110** | |

**Caps** (verified by `test_analyze` admission controls):
- **Numeric 5:** a 6th numeric submission is refused.
- **Visual 115:** after the 105 planned, 11 more (116) are refused and exactly
  115 is admitted.
- **Overall 120:** exactly 120 is admitted and 121 is refused.
- **PNGs 16:** a 17th is refused.
- **Reserve:** 10 visual submissions.

Timeouts are unchanged (list 60 s, numeric 120 s, case 120 s).

## 5. Comparison design: what changed, what did not

**The thresholds are unchanged.** The selfcheck checks that `comparison.baseline`,
`sharpness`, `stability`, `ghosts`, `exclusion`, `reset`, `registration` and
`viewport` are identical as JSON values to the `a143b5c0` run's plan (`c874115d`,
visual tree `b11a7d3a`). The 8 review cases and the review config are identical
too, and the fixture is byte-identical (`d4fc4002…`, 76,309 B).

**The prior reference (`comparison.prior`) is new.** The rejected run is
published at `fixed-sharpness-evidence` (manifest `62e89fa2…`) and
`fixed-sharpness-screenshots`. These **exactness** checks gate the comparison:
- the prior manifest SHA-256, and every report, build record, compare record and
  all 16 PNGs against it;
- its identity: `a143b5c0`, tree `b11a7d3a`, review binary `81cb6469…`;
- its compare record (`759ac37c…`) listing exactly the 13 recorded failures;
- every temporal-off PNG and all 12 temporal-off frames identical to the
  candidate's;
- the same adapter, toolchain/host and review config;
- `prior_recomputed_failures_match_record`: the same code run on the prior run
  must reproduce its recorded failures.

The output adds context, which is never a threshold:
- `prior_effect`: every quality check's prior and candidate values side by
  side;
- `failure_history`: resolved, persisting and new failures since the prior, with
  the baseline's own known failures (halo, same-phase).

**History-free frame identity (`comparison.history_free_frames`) is new and
strict.** It measures that reset frames are unchanged rather than assuming it:
- **Rule.** Every frame a report records with `has_previous: false` (not
  bypassed) must have the same `rgba_sha256` as the prior's (11 frames) and, where
  the case exists there, the baseline's (10 frames).
- **Why the ramp cannot change these frames.** A reset frame has samples = 1 and
  weight 0, and the shader returns `registered_current` before any blend.
- **Already measured on the retained runs.** The baseline and prior agree on all
  10 shared history-free frames. These are the first frames of stationary,
  motion object and pan, excluded, skinned and timing, plus the transitions
  `a-1`, `cut-7`, `drs-9` and `full-12`.
- **Unchanged rule alongside it.** The existing reset check (cut f7 and DRS f9
  within 1 step of the baseline) also stays.

**Expectations derived from the ramp** (`comparison.ramp_expectations`). None is
relaxed; all are measured, not assumed:
- **Skinned registration.** The 2-D model predicts 0.204 → 0.060 px at f6
  (`sharpness-result.md` §6).
- **Early frames change more.** Frames 2–9 after each reset blend the current
  sample at 1/2 … 1/9 instead of 1/10, so frame-to-frame changes in early frames
  rise.
  - The **excluded activity map (frames 2–8) lies entirely inside the ramp**.
    Its ≥33-step count was 0 in both earlier runs and may now rise; the threshold
    stays 0.
  - Stationary and viewport activity over frames 9–16 should change little
    (weights 8/9 then 0.9).
- **Same-phase f8/f16.** The first frame's residual weight falls from 0.48 / 0.21
  to 0.125 / 0.053.
- **Short captures** (motion f8 and f6, skinned f6, excluded f8) end inside the
  ramp. Expect less history lag and more current-sample aliasing.
- **Still open, outside the ramp:** the flicker, trailing-band and
  viewport-shadow issues from `sharpness-result.md` §6. The other proposed
  repairs stay separate.

## 6. Changes (current helpers only; old archives and outputs untouched)

| Path | Change |
|---|---|
| `visual/plan.json` | **Stage:** `fixed-warmup-visual`/`-build`, namespace `0072-fixed-warmup-001`. **History:** the rejected sharpness run with root's audit receipt, and the warmup source with root's qualification; root's earlier linkage amendment moves into history. **Source pinning:** `296e0226`, base `a143b5c0`, expected delta. **Numeric:** the one retained regression plus the retained direct-control record. **Budgets:** 120 overall, visual 115, numeric 5, 110 planned. **Comparison:** `candidate_commit`, `prior`, `history_free_frames`, `ramp_expectations` (threshold sections untouched). **Order:** build order corrected so `libtests` runs before `review`, which the tool already enforces. |
| `visual/tools/common.py` | Stage names, namespace, publication directories (`fixed-warmup-screenshots/-evidence`), docstring |
| `visual/tools/gpu.py` | Docstring and one refusal message (one numeric regression) |
| `visual/tools/analyze.py` | `compare` loads the prior run; docstring |
| `visual/tools/sharpness.py` | Prior exactness, `history_free_frames_identical`, prior recomputation with `prior_effect` and `failure_history`; the recursion evaluates exactly the requested quality sections |
| `visual/tools/selfcheck.py` | New counts and identities; comparability with the prior plan and fixture; new `numeric_source` checks from the candidate commit; refusal commands |
| `visual/tools/test_analyze.py` | Counts and ceilings (110; 115/5/120/16); all 8 old grants (`fixed-sharpness-001/-002`, coverage, corrected) refused; old two-control layout fails; three new real-data prior controls |
| `visual/tools/build.py`, `lease.py` | Stage wording only (docstrings, lease release note) |
| `visual/preparation/warmup-*.json` | New receipts (§7) |
| `warmup-preparation.md` | This document. `result.md` points here. |
| Unchanged | The fixture, `pngread.py`, `test_lifecycle.py`, `test_source.py`, `test_library_linkage.py`, every earlier result, preparation, receipt, inspection and evidence directory, and `brief.md` |

## 7. CPU validation (no build, no GPU, no grant)

All run from the worktree root with `/opt/homebrew/bin/python3` 3.14.8, in the
foreground.

| Command | Result |
|---|---|
| `python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/selfcheck.py --write handoffs/0072-motion-history-numeric/visual/preparation/warmup-selfcheck.json` | **`all_pass: true`**, exit 0 (`b75d6de5…`; plan `e587e6f2…`) |
| `… visual/tools/test_analyze.py --receipt …/warmup-analyze.json` | **22/22**, exit 0 (`3ace3146…`) |
| `… visual/tools/test_lifecycle.py --receipt …/warmup-lifecycle.json` | **14/14**, exit 0 (`7ed30c84…`) |
| `… visual/tools/test_source.py --receipt …/warmup-source.json` | **4/4**, exit 0 (`b688b246…`) |
| `… visual/tools/test_library_linkage.py` | `8 linkage controls passed; no build or GPU`, exit 0 |

**`selfcheck` sections:**
- **Plan:** 24 checks, all true.
- **Comparability:** 10, all true. That includes the fixture, cases, review
  config and threshold sections identical to the prior plan, and the baseline
  relationship unchanged.
- **Fixture against plan:** 7, all true. It measures 96 frames, 9 UI and 16
  PNGs.
- **Numeric source:** 9, all true. It covers the path, attributes, 5/3 submit
  calls, the 7 assertions, the `queue.submit` count, the delta, the changed
  hashes, the unchanged kernel/cooker/controls, and the controls bypassing the
  policy.
- **Refusals:** described separately below.

**Refusal verification (missing grants).** With no `0072-fixed-warmup-001`
directory, all 17 guarded commands refuse with their absent-grant message and
exit 1:
- `lease.py acquire|release`;
- `build.py checkout|toolchain|libtests|review|lint`;
- `gpu.py list lib|numeric retained|list review|review probe|review stationary|release|review viewport`;
- `analyze.py check|compare|publish`.

Afterwards the namespace is still absent, `git status` is unchanged, and the
global `native-gpu.lock` (mtime, size, inode) is untouched. I confirmed the
directory's absence again before and after the receipts.

**`test_analyze` real-data controls (read-only):**
- **The prior run as a candidate.** The published `a143b5c0` run, evaluated as a
  candidate, fails exactly `exactness: candidate_source_commit` plus its 13
  recorded failures, in order.
  - Every prior exactness check passes, including
    `prior_recomputed_failures_match_record`.
  - History-free frames: 11, matching 11 prior and 10 baseline.
  - Failure history: resolved [], persisting 13, new [].
- **A tampered prior** (one PNG byte) fails `prior_files_match_manifest`.
- **A history-free frame hash changed** (`transitions/cut-7`) fails
  `history_free_frames_identical` with exactly that frame.
- **Kept from before:** baseline against itself, aliased original, tampered
  baseline, missing outputs.
- **Real old grants refused:**
  - `fixed-sharpness-002` (`d5e50057…`, `0907ee4f…`);
  - `fixed-sharpness-001` build (`14e9f738…`; a stand-in for its review grant,
    which was never written);
  - coverage (`e83f97da…`, `a62dbb56…`);
  - corrected (`4fa5101f…`, `1a1b226d…`).

The comparison's runtime on real data is about 50 s for the analyzer suite, all
in the foreground.

## 8. What root needs to grant (unchanged routine)

1. **`build-ready.json`** in `~/.codex/incant-evidence/0072-fixed-warmup-001/` with:
   - stage `fixed-warmup-build` and the holder;
   - `source_commit` `296e0226…` and root's source freeze (path + SHA-256);
   - the committed visual tree of this preparation commit;
   - a dedicated `~/.codex/*-target`;
   - optional prior pins.

   Then, root-supervised: `lease.py acquire`; `build.py checkout`, `toolchain`,
   `libtests` (unless a frozen same-source library test binary will be pinned),
   `review` and `lint`; `lease.py release`.
2. **`review-ready.json`** with:
   - stage `fixed-warmup-visual`, the holder and the visual tree;
   - the SHA-256 of `build/review.json`;
   - at least one prior release pin;
   - optionally `numeric_binary` for root's frozen same-source (`296e0226`)
     default library test binary.
3. **The GPU run** follows `plan.json` order: `list lib`, `numeric retained`,
   `list review`, the 8 cases, then `release`. Then `analyze.py check`,
   `compare`, personal inspection, and `publish` into `fixed-warmup-*`.

## 9. Limitations

- **Preparation only.** Nothing was compiled or executed on a GPU. The retained
  regression's new assertions have **not executed** for this candidate, and no
  appearance, image or numeric result exists for the ramp yet.
- **Model, not measurement.** The predicted effects (skinned registration,
  early-frame activity, same-phase) come from CPU arithmetic and the 2-D model.
- **Source checks are static.** The submit and assertion counts come from
  parsing the committed Rust source. Each submit is classified by its
  `.unwrap()` or `.is_err()` suffix; a semantic change that keeps those suffixes
  would need review.
- **Old controls are not rerun.** The two direct controls' prior results cover
  the unchanged kernels only; they say nothing about the host policy.
- **Admission.** If motion admission is unavailable on the host, the retained
  regression passes vacuously; the probe case guards this.
- **Scope.** Desktop Metal only; no phone, shipping-profile or performance
  claim, and no phase or gate approval.
