# 0069 GPU-only continuation proposal

CPU source/evidence is independently accepted at `15a59060c06ea249c66740b5940b92721d032970`.
The acceptance receipt is copied beside this document as `cpu-review.json`. Its
scope is CPU correctness and provenance; appearance remains Claude's task.

This is a proposal, **not a GPU grant**. Root alone schedules the next interval
and supplies its authority after the previous holder releases. Claude Opus 5.5
with thinking explicitly verified Max owns execution and pixel review. The
currently active 0073 interval must finish first.

`gpu_only.py` adds a new runtime namespace around the unchanged `tools/lease.py`,
`capture.py` and `regression.py`. It uses their supervision, global GPU lock,
packet lock, prelaunch debit, exact-case checks and final release. It does not
reopen the CPU lease, overwrite its ledger, rebuild, recook, change a font/layout,
write a target, launch a window, or use device audio. The holder comes from the
root authority; there is no invented execution identity.

The source stays generation 5, `acb68a621b4b74acd4c4e14280171451de81f53d2feaa4f8585a9fc6d12d1f1d`.
The wrapper checks every inventoried source, original helper hash, lockfile, host
and toolchain. A packet-only commit is permitted without relabeling any compiled
artifact. CI packages/adapters are separate from the accepted shipping player;
this continuation does not represent CI binaries as shipping-profile evidence.

Inputs are read-only:

- CPU manifest: `cpu-evidence/manifest.json`, SHA-256 `1d5f8a8922e42a1bdda534c598633f7afce1c093bc6ebc615e8d61e7b70b4d27`.
- CPU release: `~/.codex/incant-evidence/0069/release.json`, SHA-256 `bc2cabe26bae17700e300188e3c7f8e68fa06538cb44c7b4197d000c5862696f`.
- Regression copies: `~/.codex/incant-evidence/0069/regression/frozen-build-g5.json`, SHA-256 `dba7bc052da4f333a51fee3e6409d69507f7dfa1fc58f6d188a42b69c7e6a810`.
- Render-lib copy: `0069/frozen-binaries/generation-5/render-lib`, SHA-256 `951e4f18535be547c428ded304b52d70d963da5f72711626e191f2aa608e46bb`.
- GPU-host copy: `0069/frozen-binaries/generation-5/gpu-host-ui`, SHA-256 `8b85ceb1dca228e6ca04e60c55f5592afd8ce44cc37f727a40932092346d3b6e`.
- Four CI packages: `0069/packages/{en,ar,zh-Hans,en-XA}`; every package inventory,
  adapter digest and executable permission is checked before use.
- Three input recordings are copied byte-for-byte from accepted `shipping-002`;
  their exact hashes are pinned in the wrapper. No recording is synthesized.

A fresh sibling directory such as `~/.codex/incant-evidence/0069-gpu-001` receives
all new plans, scratch data, reports, images, ledger, counters and release. It must
not exist. The root authority must be a separate file outside that directory.
The old packages are aliased read-only by usage, and the exact **frozen** test
record is copied to the established `regression/build-g5.json` filename. Its
paths refer to copied executables outside the released target. No old target
path is executed or required. `unused-cargo-target` is a sentinel namespace for
the existing lease schema, not an acquired Cargo cache.

## Exact execution limits

One invocation performs these steps serially:

1. Eight exact ignored regression cases from the pinned `tools/regression.json`,
   each once: 39 conservatively counted tiny frames, under the separate 128-frame
   ceiling, maximum 320 physical pixels on either side. No filter expansion.
2. Only if all eight pass, ten exact capture processes from `tools/plans.json`:
   three review runs (4 frames) and seven final runs (18 frames). All 22 are fresh
   draws; nothing is reused or silently discarded from accounting. Each frame is
   at most 960×540. The appearance ceiling remains 24, but the two spare frames
   do **not** authorize an unlisted run or automatic retry.

At most 18 managed GPU child groups; each has a 120-second timeout, plus bounded
cleanup. Both locks are held through debit, child exit and reaping. A failed
regression prevents all appearance captures; individual regressions retain the
existing once-per-case failure records. Any timeout, interrupt or admission
failure stops execution. All outcomes attempt final release; an unreaped group
or failed release leaves the authority held with the original failure preserved.

The actual child environment is `PATH=/usr/bin:/bin`, a fresh packet `TMPDIR`,
and only the two explicit image-output variables for regression cases. The
wrapper removes the legacy helpers' fabricated `HOME`; no ambient environment,
credentials, `WGPU_*`, `INCANT_*`, Cargo or compiler overrides are inherited.

## Root authority and invocation

The authority must contain the following exact fields. Root supplies the holder,
wrapper digest and preceding GPU release; this document does not create one:

- `packet`: `0069-core-result-presentation`; `phase`: `gpu-only-generation-5`.
- `gpu_only: true`, `cpu_only: false`, `planned_processes: 18`.
- `source_commit`, `freeze_sha256`, `cpu_manifest_sha256`, `cpu_release_sha256`
  equal the identities above.
- `wrapper_sha256`: actual SHA-256 of committed `gpu_only.py`.
- `evidence`: absolute fresh output path; `target`: that path plus `/unused-cargo-target`.
- `ceilings`: `{"appearance":24,"regression":128}`.
- `holder`: actual executing holder, with verified model/effort recorded by root.
- `predecessor_release`: exact absolute `path` and `sha256` of the preceding GPU release.

Read-only preflight (no authority or GPU needed):

```sh
python3 -B handoffs/0069-core-result-presentation/gpu_only.py check
```

Only after root review, grant and Claude session verification:

```sh
python3 -B handoffs/0069-core-result-presentation/gpu_only.py run \
  --evidence /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-001 \
  --authority /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-review-ready.json
```

No source change is allowed during this interval. If visual review calls for
changes, finish/release this interval and prepare a separately reviewed source
and build. These one-frame captures do not establish temporal-history behavior,
mobile/device performance, native-window input, or phase completion.
