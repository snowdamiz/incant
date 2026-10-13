# 0072 corrected-source temporal review — preparation result

**Status: PREPARED.** This is preparation only. No Cargo, build, target, GPU
process, frame or capture ran, and the new evidence namespace
`~/.codex/incant-evidence/0072-temporal-corrected-001` does not exist yet.

The earlier work is preserved unchanged:
- the original rejection (`temporal-visual-result.md`);
- its 14 captures and retained evidence, including the failed optional lint;
- all old grants and ledgers.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max.
- Both values are as this session reports them.

**Director feedback, applied:**
- The rejection drove an engine fix, and this prepares a fresh, comparable
  review of that exact source.
- I made small edits to the existing helpers and reused 0073's accepted
  source-pin routines read-only.
- I fixed the unused `mut`, preserved the seven cases, and added only the one
  exact numeric regression.
- There is no whole-suite run, no engine edit, no background build, no grant
  polling, no memory change, and no push or PR.

## Corrected source (read from the exact commit)

**Docs read.** I read `docs/spikes/temporal-edge-correctness.md` and
`docs/spikes/temporal-metalfx-composition.md` via
`git show 535003d766bcc1386fd2d2f454c83808a0fcc4c8:…`. They describe:
- valid positive-tap history support with renormalization, instead of
  any-tap rejection;
- unjittered presentation;
- unjittered current-only overlays over a canonical opaque/MASK depth pass;
- the numeric resolve fixture extended to 14 submissions.

**Build freeze.**
- File: `/Users/andreyyurlov/.codex/incant-evidence/temporal-metalfx-union-001/committed-build-source.json`.
- SHA-256 `bd8295b0eb3174b96eb5ff73b0e0e7c464c6c5d620fd66df37bd59d3eafe56a0`.
- 2,449 `{path, sha256, bytes}` rows; `commit` = `535003d7…`.
- `build.py checkout` requires exact build-input coverage against it.

**Fixture compatibility.** I checked by source diff from `e73afe43` to
`535003d7`:
- These are unchanged: `tests/support`, `tests/materials`, `incant_render_data`,
  `incant_types`, `incant_text`, `incant_doc`, `incant_render_authoring`,
  `incant_assets_runtime` and the skeletal fixture.
- `incant_render` gains only additive `pub` items (MetalFX and upscaling).
- `RetainedTarget` and `TemporalConfig` are field-for-field identical.

**Numeric test name.** From the source mounts at `535003d7`:
- `lib.rs:35` has `pub mod temporal`.
- `temporal/mod.rs:8` has `pub(crate) mod system`.
- `system.rs:316-317` has `#[path = "device_tests.rs"] mod device_tests`.
- `device_tests.rs:15` has the `#[ignore]` function.

That gives the libtest name
`temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion`.
It is unchanged, so there is no name difference to report. Under the grant,
`gpu.py list lib` must still list it exactly before `gpu.py numeric` may run,
and the filter is never broadened.

## Changes (small, reviewed-pattern edits)

| File | Change |
|---|---|
| `review/temporal_visual_review.rs` | **Only line 1418**: `let mut report` → `let report` in `probe`, for strict Clippy. `selfcheck` proves this is the single diff from the original fixture at `9009f088`. |
| `plan.json` | **New namespace.** `0072-temporal-corrected-001`. |
| | **Two grants.** Grant schemas; the expected commit and freeze are recorded for review only. |
| | **Seven cases.** Byte-identical frames, UI counts, PNG names and purposes (proven against `9009f088`), plus each case's exact contract-check list. |
| | **One numeric regression.** 14 submissions, one attempt. |
| | **Budget.** Ceiling **128** (planned 79 + 14 = **93**), 14 of 16 PNGs, 960×540. |
| | **No whole-suite run.** The old suites are removed. |
| `tools/common.py` | **0073 two-grant routine.** `build-ready.json` is the CPU grant: holder, 40-hex commit, freeze path and SHA, visual tree and target. `review-ready.json` is the GPU grant: holder, visual tree, build-record SHA, release pin, and optional `numeric_binary`. |
| | **New gates.** `build_gate`/`gate`, `prior_receipts`, and the ceiling-based `debit`. |
| | **Kept unchanged.** `supervise` and `deferred_signals`, the global non-waiting `native-gpu.lock`, the stop latch and the executable check. |
| `tools/build.py` | **Freeze check.** 0073's `freeze_entries`/`freeze_coverage` exact-freeze checkout. |
| | **Holder in receipts.** Every build record names the grant's actual holder. |
| | **Library-only build.** `libtests` builds just the library tests, and only if root pins no same-source binary. |
| | **Review record.** `review` freezes the fixture into `review.json`, the build record the GPU grant binds. |
| `tools/lease.py` | The lease belongs to the build grant and records the holder. It is released after the CPU build; the GPU needs no target. |
| `tools/gpu.py` | **No suites.** The `suite` command is removed. |
| | **Numeric command.** `list lib` must show the exact name; then `numeric` runs once with a 14-submission debit. |
| | **Review gating.** Review cases need the numeric pass and plan order. |
| | **Binary binding.** The review binary is bound through the build-record SHA, and root's library binary through its pinned SHA. |
| | **Release latch.** It records the holder, the debit and ledger hashes, orphan debits and the stop state. |
| `tools/analyze.py` | **Fail-closed verdict, 0073 semantics, narrowed.** A missing, empty, stale or failed grant, build record, lease, latch, listing, numeric run, case, report, check, PNG or hash fails. The verdict is never from observations. |
| | **Publication.** It works only from the latest summary, which must pass and still bind the current evidence. |
| `tools/test_analyze.py` (new) | 9 analyzer controls. |
| `tools/test_source.py` (new, adapted from 0073) | 4 source-pin controls. |
| `tools/selfcheck.py`, `tools/test_lifecycle.py` | Updated for the above. |

No concrete issue required any other fixture change. The original contract
checks remain valid under the corrected source. The corrected overlay depth pass
runs only with overlays, so the timing scene (no overlays) still measures the
same Models composition.

## CPU validation (no build, no GPU)

| Command | Result |
|---|---|
| `python3 -I -B visual/tools/selfcheck.py --write visual/preparation/corrected-selfcheck.json` | `all_pass: true`. Plan budgets and totals hold; comparability holds (same 7 cases and config; fixture diff exactly line 1418); the fixture matches the plan (order, limits, PNGs, size, output variable, config, per-case checks). All **15** guarded commands refuse with both grants absent: no evidence directory, `git status` unchanged, global lock untouched. |
| `test_lifecycle.py --receipt …/corrected-lifecycle.json` | **14/14.** Real children cover kill and reap, timeout, SIGINT/TERM/HUP, the stop latch, the default child mask, non-executable, ceiling and busy-lock refusals before any charge, and the holder in receipts. |
| `test_analyze.py --receipt …/corrected-analyze.json` | **9/9.** A complete synthetic run passes and publishes. Empty evidence, a missing case, a skipped numeric run, a missing report, a false check, a failed child and a stop latch each fail and cannot publish. A stale summary blocks publication. |
| `test_source.py --receipt …/corrected-source.json` | **4/4.** An exact committed freeze exports. A branch name and a freeze-hash mismatch are refused before export. A build input missing from the freeze latches stop. |
| `rustfmt --edition 2024 < fixture` (stdin) | Clean. |

## Identities for root's grants

| Item | Value |
|---|---|
| **Visual tree** (pin in both grants) | **`44b17e865f3923be0ebe8c4569d150a939d91680`** |
| Fixture | `9841c6b1384a9f02db140d58d588f40a262e63ea061127518781e1c799952dde` |
| Plan | `df1d01c245c7cd60573db09b0f086d923a5158d4308c7f69d7cfa923a7948f89` |
| Tools | `common` `9f7ab0ad…`, `build` `3847a0e2…`, `lease` `13acaa49…`, `gpu` `2d97eb72…`, `analyze` `570abe91…`, `selfcheck` `41350bb7…`, `test_lifecycle` `8f33f44d…`, `test_analyze` `c0bc9a87…`, `test_source` `483abef0…` |
| Receipts | `corrected-selfcheck` `20e1d0c7…`, `corrected-lifecycle` `14e2d592…`, `corrected-analyze` `42cb0b13…`, `corrected-source` `b688b246…` |
| Expected source | commit `535003d766bcc1386fd2d2f454c83808a0fcc4c8`, freeze `bd8295b0…` (the grant is authoritative) |
| Original for comparison | commit `9009f088`, tree `ac8a48de…`, evidence `~/.codex/incant-evidence/0072-temporal-visual` |

### What root's grants must contain

**`build-ready.json`:**
- `packet`;
- `"stage": "temporal-corrected-build"`;
- `holder`;
- `source_commit`;
- `source_freeze {path, sha256}`;
- `visual_tree`;
- `target`, as `~/.codex/<name>-target`.

**`review-ready.json`,** written after the build:
- `packet`;
- `"stage": "temporal-corrected-visual"`;
- `holder`;
- `visual_tree`;
- `build_record_sha256`, the SHA-256 of `build/review.json`;
- at least one `prior_*release*_path`/`_sha256` pin;
- optionally `numeric_binary {path, sha256}`, for root's frozen same-source
  default library test binary.

**Execution order:**
1. **CPU, root-supervised in the foreground:**
   1. `lease.py acquire`
   2. `build.py checkout`
   3. `build.py toolchain`
   4. `build.py review`
   5. `build.py libtests`, only if no binary is pinned
   6. `build.py lint`
   7. `lease.py release`
2. **GPU:**
   1. `gpu.py list lib`
   2. `gpu.py numeric`
   3. `gpu.py list review`
   4. the seven `gpu.py review` cases, in order
   5. `gpu.py release`
3. **CPU:**
   1. `analyze.py check`
   2. my personal inspection of every PNG against the original captures
   3. `analyze.py publish`

## Limitations

- **Not compiled yet.** The fixture is not compiled against `535003d7`; that
  happens under the build grant. Its API surface is unchanged, and strict
  Clippy's only original diagnostic is fixed, but compile and lint success are
  not yet proven.
- **No quality claim.** Nothing here says the corrected source looks right.
  Astra's own document notes that phase-dependent coverage can remain at
  subpixel silhouettes. Numeric passes cannot establish appearance.
- **Scope.** This is default portable fixed-mode quality only. 0073 owns
  upscaling and 0074 owns native MetalFX. There is no phone, performance or
  phase-gate claim.
- **Screenshots:** none, because preparation produces no frames.
