# Core immutable row metadata

Core now caches row indices and configuration-derived coordinates keyed by the
host's frozen ID/instance snapshots. Every update still uses the current numeric
views and strides, writes all 9,999 stress transforms and reads back the actual
written columns for the checksum. Reordered, split or removed rows get different
metadata; a changed instance snapshot invalidates stress membership and its row
cache. Numeric views are never retained. This changes the sample implementation,
not the engine or the workload's intended behavior.

The actual fast-mode shipping comparison improves script p95 in every pair but
still fails the 2 ms gate. It is not an unchanged-script engine comparison and
it does not approve the phase, FPS, display/GPU or complete memory gates.

| Pair | Old script p95, ms | New script p95, ms | 2 ms gate |
| --- | --- | --- | --- |
| 0 | 4.310001–4.320000 | 2.860001–2.870000 | Fail |
| 1 | 4.740001–4.750000 | 2.800001–2.810000 | Fail |
| 2 | 4.770001–4.780000 | 3.070001–3.080000 | Fail |

Each interval is the raw profile histogram's p95 bucket, not a confidence
interval. All three paired 5% comparisons pass for script p95, CPU player-frame
p95 and requested Rust peak allocation. Requested Rust peaks are approximately
76.88 MB. QuickJS peak allocated/used memory increases from 5,364,384/3,962,281
to 6,322,432/4,607,766 bytes with the extra cached metadata. These allocator
figures are distinct from total process memory; no memory improvement is claimed.

## Measured source and functional equivalence

Baseline `031a7173ac865a99964e12538926e02ca64ccd71` contains the accepted engine
caches with the old scripts. Candidate `051e0de7f26c6f38c470765ae86b8bc415d19fcc`
contains the new scripts. All 3,872 recorded engine/tool/SDK/vendor/build inputs
match. The original project and binary runtime data match; only the authored
`gameplay.ts` and `stress.ts` sources change. Undo/redo journals are new authoring
evidence, not additional runtime inputs. Source changes were applied through the
public command API as two separate atomic source transactions; undo, redo, save
and reopen were exercised. The historical authoring binary is explicitly
identified and is not relabeled as a new runtime build.

Both compiler-free shipping packages complete the same 2,156-tick Core win with
10,000 script attachments and two source groups. The non-profile reports match,
including the four pickups, damage, win at tick 2,066, HUD relationships,
population, sampled stress/sentinel observations and PCM hash. Each measured
source executes 2,156 successful calls without unwinds or unavailable CPU samples.
The six selected stress rows and population check are not an all-entity snapshot.
The stress loop and omission controls independently preserve all 9,999 writes.

The predeclared order was B0/C0/C1/B1/B2/C2. Task compilers and GPU work were
released before measurement, and the runner recorded an empty compiler-process
name scan. This does not assert that unrelated user applications were idle.
All match fields other than `script_source_sha256` are identical across each
pair, including machine/OS, compiler/codegen, physics mode, data, input/observation
recordings, tick count and profiling configuration. The machine was Apple M5 Pro
(Mac17,9), macOS 27.0.1 build 26A434, arm64, AC power. No additional measurement
was performed while packaging this evidence.

## Correctness, integration and retained failures

The source controls cover immutable snapshot reuse, reordered/split/removed IDs,
changed strides, detached old buffers, membership replacement, camera/velocity
outputs and omitted-write detection. `npm run test:core` runs strict TypeScript,
Node controls and the ten stress-helper Python tests; `npm run test:tools` now
includes it in the normal development check. Root's complete tool suite passed
at `4e7986aae4e5cfa8ab144860f7052c57baf1b499`, which changes only this CI wiring
and README relative to the executed source051e. Current-main261 integration
replayed the focused Core suite and conventions/diff checks. There is no new
Rust build claim for this composition.

Preparation001 failed while reading the public script envelope before mutation.
Preparation002 tried two source writes in one transaction and was correctly
refused. Preparation003 uses one write per transaction. Both failed attempts,
their raw RPC and the successful public authoring records remain retained. They
are helper errors, not runtime or performance successes. The earlier 2,400-second
old-emitter build timeout remains in the separate
[matched engine-cache packet](mac-core-fast10k.md); it is not this runnable
baseline.

[Manifest](evidence/core-bulk-row-metadata/manifest.json) and the lossless archive
retain compiler streams, raw profiles, functional reports, source inventories,
preparation failures, public RPC and audit sources. Executables, symbols,
binary assets, large authoring journals and PCM are hash/size-bound external
originals. Full source verification reads immutable Git objects rather than
assuming producer checkouts stayed on those branches. The read-only verifier
recomputes profile intervals and compares every required identity; twelve
negative/boundary controls remain active through the prior audit helper.

```sh
python3 docs/spikes/evidence/core-bulk-row-metadata/verify.py --verify-only
python3 docs/spikes/evidence/core-bulk-row-metadata/verify.py --verify-only --external --git-source
```

Independent final integration review is pending. Release and phase decisions
remain separate, and other-device execution remains deferred under the Mac-only
current device policy.
