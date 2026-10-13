# 0073 revision 1 — fail-closed analyzer and corrected-source readiness

**Status: PREPARED (revision 1), not run.**
- There was no Cargo build, target, compile, GPU child, render or capture.
- `~/.codex/incant-evidence/0073-upscaling-visual` does not exist, and there is
  no grant.
- Prior results are preserved unchanged: `numerical-result.md`,
  `preparation-result.md` and the revision-0 receipts in `visual/preparation/`.

**Model, effort and inputs.**
- Model: Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Root's
  `claude-0073-analyzer-repair-acp.log` header reads `Claude session verified:
  Opus 5.5; thinking: max`.
- Brief: `cd7c45b7…`; its priority revision was committed by root in `15ddda49`.
- Root review: `upscaling-0073-prepared-source-20261013.json`, `d307b665…`,
  status `changes-required-analyzer`. Both hashes were verified.
- No memory writes, accounts, push or PR.

## Feedback applied

1. **The analyzer now fails closed** (root finding P1, `analyze.py`).
   - **One validator.** It requires:
     - the plan's four cases, exactly once each and in order, plus one
       successful listing;
     - one debit per GPU child;
     - no stop latch, a released build lease, and a release latch bound to the
       current ledger and debits;
     - the build record that the GPU grant binds.
   - **Per case** it requires:
     - exit 0, an empty process group, and the frozen binary before and after;
     - ledger hashes for every log, report, events file and PNG that equal the
       files now;
     - exactly the planned output files;
     - `outcome: pass`, with exactly the plan's contract check names, all true;
     - the planned frames, and each PNG at 960×540 with matching file and RGBA
       hashes;
     - every independent comparison: 19 re-cut tiles, UI identity, reproduced
       statistics and chrome identity.
   - **Failure handling.** Any missing, failed, stale or mismatched item is
     recorded in a retained `analysis/summary-*.json`, and `check` exits 1.
   - **Publishing.** `publish` uses only the latest summary. That summary must
     pass, and its recorded bindings must still equal the recomputed ones: grant,
     build record, lease, ledger, debits, latches, plan, visual tree and every
     case file.
   - **Observations stay separate.** Image differences, activity and wire
     presence remain observations and never become thresholds.
2. **Source pinning now comes from root's grants**, replacing the hard-coded
   `0452c5f5` pins.
   - **`build-ready.json`** is the supervised CPU build grant. It names:
     - a 40-hex commit object (a branch name or non-commit object is refused);
     - root's freeze, by path and SHA-256;
     - the visual tree and a `~/.codex/*-target`.
   - **The checkout** is a blob-verified `git archive` export of that commit, so
     no dirty file can enter. Any of the following latches stop:
     - a freeze `head`/`commit` that is not the granted commit;
     - a freeze that does not list exactly the commit's build-input paths
       (`crates`, `Cargo.*`, `third_party`, `tools/cargo`,
       `rust-toolchain.toml`) with equal SHA-256 and size;
     - any other freeze row that is absent or differs.
   - **`review-ready.json`** is the GPU grant. It binds `build/review.json` by
     SHA-256, so it covers the commit, freeze, helper and binary.
   - **Lease.** It now covers only the CPU build, and `gpu.py list` refuses
     while it is held.
   - **Features.** Builds refuse both `cooked-*` (including `cooked-temporal`)
     and `native-metalfx`. This review is the portable path.
3. **Reading the corrected source** (read-only, branch
   `codex/cooked-temporal-metalfx` at `9a65b6c8`, plus root's uncommitted work
   and its composition doc).
   - **What changed for this review:**
     - Upscaled history now renormalizes over supporting previous taps instead
       of refusing when any positive tap is outside or incompatible.
     - BLEND models, sprites and world text draw with the unjittered camera over
       a re-rasterized unjittered opaque depth, and the composite samples them on
       the unjittered internal grid.
   - **What did not change:** the current-coverage edge policy. The output,
     UI, resolution plan, jitter sequence, motion owner and the fixture's test
     modules are byte-identical, and the public calls the fixture makes keep
     their signatures.
   - **Effect on the fixture:** its contract checks never depended on the changed
     math, so its source is unchanged (`f8ec6311…`). Only prose expectations are
     superseded (next section).
4. **Preserved.** The seventeen lifecycle controls (`test_lifecycle.py`,
   byte-identical) still pass. StageA remains historical evidence for the
   `0452c5f5` binary `afa3d035` only, and is not relabeled.

## Expectations superseded

These supersede `preparation-result.md` expectations 5, 7 and 8. They are still
to be verified visually.

- **Overlays.** BLEND, sprite and world-text pixels should be stable across the 8
  phases, apart from translucent pixels over jittering opaque edges. They should
  still be internal-resolution soft. Per-crop activity in `stationary` already
  observes this; there is no new numeric gate.
- **Motion.** Disocclusion trails should be narrower, because partially
  supported pixels keep renormalized history.
- **Viewport edge.** The border band is no longer forced current-only. Only
  points whose reprojection leaves the viewport lose history.
- **Unchanged:** edge and sub-texel-wire shimmer from the current-only edge
  policy, the texture LOD (no bias found), and the scale-change and cut
  behaviour.

## Readiness note for root

Root's current generation-2 freeze is `temporal-metalfx-union-001`, with head
`9a65b6c8`. I checked it read-only against git objects:
- **Coverage:** it covers all 2,448 build inputs of `9a65b6c8` path for path.
- **Problem:** 11 rows are uncommitted edits, and the composition doc is not
  committed. Under the new rule it would be refused.
- **What this means:** the final freeze must be generated from the final commit.

## Frozen identity

| Item | Value |
|---|---|
| Commit / visual tree | `9bcd1e0e` / `09271a620ba4250082c16c4e726b012fa85658d7` |
| Fixture (unchanged) / plan | `f8ec6311…` / `e35367bf…` |
| Tools | `analyze` `cba70dcc…`, `build` `20641c96…`, `common` `d27e8fec…`, `gpu` `66e39aa3…`, `lease` `3ef5812a…`, `selfcheck` `294529dc…`, `test_analyze` `111ad4bd…`, `test_source` `85e5d633…`, `test_lifecycle` `5005ac36…` (unchanged) |
| Budget | Unchanged: 89 of 96 frames, 13 of 16 PNGs, 960×540, REVIEW policy |

## Commands and results (CPU only)

| Command (`python3 -I -B handoffs/0073-upscaling-review/visual/tools/…`) | Result |
|---|---|
| `test_lifecycle.py --receipt …/r1-lifecycle.json` | **17/17**, the unchanged controls |
| `test_analyze.py --receipt …/r1-analyze-controls.json` | **17/17** (below) |
| `test_source.py --receipt …/r1-source-controls.json` | **10/10** (below) |
| `selfcheck.py --write …/r1-selfcheck.json` | `all_pass` (below) |

**Analyzer controls (17).**
- **Passing control:** the complete, current evidence passes, all 19 tiles match,
  and it publishes 13 PNGs.
- **Negative controls (15).** Each one exits 1, retains a failing summary,
  publishes nothing, and names its intended problem:
  - missing: a case, a report, a PNG;
  - mismatched: a bad hash, a replaced build record;
  - false: a contract check, a tile, a statistic, the UI identity, the chrome
    identity;
  - failed: a failed child, a stop latch;
  - zero-verdict evidence: an empty ledger, empty checks;
  - publication refused for no analysis, a stale analysis, or a newer failing
    one.
- **CLI exit codes:** `check` returns 0 when passing and 1 when failing, and
  `publish` returns 1 with "did not pass".

**Source-pinning controls (10).** These run the real `build.checkout()` on a
throwaway git repository.
- An exact committed freeze exports.
- These are refused before any export: a branch name, a tree object, a freeze
  hash mismatch, a freeze head mismatch, and conflicting duplicate rows.
- These latch stop: a dirty build input, an uncommitted freeze row, a missing
  build input, and a size mismatch.

**Selfcheck** (`all_pass`):
- 21 plan checks and 12 fixture checks pass, including the exact contract
  check names per case.
- All 12 guarded commands refuse, each naming the grant it needs.
- No evidence directory was created, `git status` was unchanged, and the
  shared GPU lock was untouched.

## Limits

- **Uncompiled.** The fixture was reviewed against the corrected source but
  never compiled against it.
- **Portable path only.** There is no GPU or appearance evidence. Cooked-temporal
  and MetalFX need their own same-source grants.
- **Not covered:** phones, performance and any phase gate.
- **Root's next actions:** the final commit and freeze, then `build-ready.json`,
  then, after the build, `review-ready.json`.

## Changed paths

- `handoffs/0073-upscaling-review/visual/plan.json`
- `handoffs/0073-upscaling-review/visual/tools/{analyze,build,common,gpu,lease,selfcheck,test_analyze}.py`
  (modified)
- `handoffs/0073-upscaling-review/visual/tools/test_source.py` (new)
- `handoffs/0073-upscaling-review/visual/preparation/r1-{lifecycle,analyze-controls,source-controls,selfcheck}.json`
- `handoffs/0073-upscaling-review/revision-result.md` (this file)
- `handoffs/0073-upscaling-review/result.md` (revision status added; earlier
  content kept)
