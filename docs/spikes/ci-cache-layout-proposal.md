# Cache capacity and retention proposal

**Proposal only.** No workflow, key, permission, cache contents, test selection,
profile, runner or billing setting is changed. PR56 remains fixed while its
required checks run. Shipping dependency seeds are deferred until the ordinary
cache set has demonstrated sustainable capacity.

## Evidence and current limit

After the one authorized main53 nightly dispatch, all six platform jobs succeeded.
Main macOS/Windows engine producers succeeded; Linux failed with the copied-player
`ETXTBSY` test race, so its successful prior-generation cache was retained. The
reviewed test isolation is in the next PR union; no producer retry was dispatched.
Nine exact obsolete generations were deleted only after independently verifying
successful replacement jobs, completed saves, matching family/version and intact
cache entries, reclaiming 6,709,767,909 bytes.

The final observed inventory contains 24 entries / **10,030,209,780 bytes
(9.341 GiB)**. It covers 11 of 12 Rust families. The newly published Linux
credential cache `8780777374` (566,356,126 bytes) disappeared during overlapping
uploads and was never selected for our deletion. This is consistent with quota
churn; the API does not identify an eviction cause. Adding that missing family
back would yield **10,596,565,906 bytes (9.869 GiB)**, leaving only **140,852,334
bytes** below 10 GiB. Current wasm-bindgen/cargo-ndk archives add only 3,306,644
bytes combined. They are not the capacity problem.

[Retention evidence](evidence/ci-main53-retention-2026-10-10.json) records exact
IDs, keys, bytes, job timings, the failure, disappearance and final inventory.
Producer timing is not PR merge latency: main53 Windows took 14m43s plus 2s queue,
restoring the newer main52 prefix and compiling 21 workspace/one other package
event. Its ordinary build was 5m31s, GPU build 0.59s with zero compilation, and
explicit tools 48.01s. All 44 GPU identities passed. That contrasts with PR53's
older-prefix observation without constituting a controlled benchmark.

## Read-only component measurement

The pinned rust-cache action stores registry/git data together with the target
in each family. Its cleanup retains matching downloaded crates, sparse-index
entries and `-sys` source timestamps; it prunes targets by external package/target
names. It exposes no input to omit registry paths from an otherwise normal target
cache. A split therefore needs reviewed packaging logic rather than an assumed
configuration flag.

A local macOS measurement applied those selection rules without deleting files,
then streamed each selected group through tar/zstd level 3. It used the reviewed
numeric/audio/Tauri union and its isolated debug/release target, not a hosted
archive. [Machine evidence](evidence/ci-cache-parts-local-2026-10-10.json) records
the exact source head, cleanup-source hash and limitations.

| Selected local component | Raw file bytes | Compressed tar bytes |
| --- | ---: | ---: |
| Registry/index/downloads and required sys sources | 393,409,603 | 150,782,417 |
| Third-party debug/release target artifacts | 5,184,418,254 | 1,487,336,402 |

The registry is about 9.2% of this local compressed pair. It is not valid to
multiply this sample into a claimed hosted saving: families, platform graphs,
compression headers and historical feature variants differ. Hosted archives
currently report only their combined size. Before a layout implementation, take
separate compressed measurements on successful representative native, credential
and platform producers, at least once per host OS. Record registry versus target,
old unused variants, largest new archive and total repository footprint.

## Candidate layout and projected range

The candidate to validate first is **one registry layer per supported host family,
plus the existing 12 independent third-party target families**. Keep all dependency
artifacts currently eligible for each family, keep workspace artifacts excluded,
and preserve every check and command. Do not silently make auxiliary jobs
registry-only, trim required variants or turn on workspace-artifact caching.

Assuming three compatible registry host groups and 75–200 MB of duplicated
registry data per current family, eliminating nine copies projects **0.675–1.8 GB**
(0.629–1.676 GiB) saved. The local 150.8 MB sample suggests a central estimate of
1.357 GB. Applied to the reconstructed complete inventory, that would put steady
storage around **8.19–9.24 GiB**, approximately 8.60 GiB centrally. These are
planning ranges, not measured hosted outcomes. Different compatibility groups
or little shared registry data reduce the saving.

Splitting alone does **not** establish safe replacement capacity: the current
largest archive is about 1.85 GB, and the predicted steady footprint may not leave
room for its immutable replacement. Adopt a measured admission rule:

```
retained inventory + measured candidate upload + safety reserve <= 10 GiB
```

A 256 MiB initial safety reserve is a proposed floor, not a guarantee against
uncoordinated npm/Gradle uploads. Count every repository cache in the inventory.
If admission fails, retain the previous usable generation, skip only the cache
write, emit explicit capacity evidence, and still run every test. Do not delete a
current family before its replacement exists. A skipped write can leave future
dependency deltas cold; this is a visible fallback, not a claim of complete seeding.

The target steady-state goal is at most 8 GiB with room for the measured largest
replacement plus reserve. If the split cannot achieve it, pause expansion and
measure overlapping same-host target artifacts across families before proposing
a producer union/deduplication. Do not spend the remaining space on broad shipping
seeds or silently change the cache quota.

## Invalidation and trust boundaries

Proposed new namespaces must not collide with legacy monolithic caches.

- Registry keys: format version, registry-source/index format identity, host OS /
  architecture / Linux distro, pinned Cargo/rustc identity, and resolved lock/
  manifest hash. Preserve sys-source timestamps; do not introduce cross-OS archive
  reuse in the first version. Prefix restore stays within the verified host and
  compiler group; Cargo's locked checksums remain authoritative.
- Target keys: format version, existing family, target triple, host/image ABI,
  exact rustc identity, profile/codegen/RUSTFLAGS inputs, toolchain/config and
  manifest/lock hash. Preserve current invalidation at minimum, including static
  Windows CRT choices. Source-only changes may reuse dependencies, but no workspace
  crate or its generated outputs can enter the archive.
- Packaging is an explicit registry/third-party-target allowlist. Exclude Cargo
  credentials/config secrets, bin/proxies, application keyrings/account files,
  project content, distribution keys and packaged players/symbols. Treat cache
  contents as build inputs, not instructions; no compiler-wrapper bypass.
- PRs remain restore-only. Publication and retention mutations run only from
  successful trusted default-branch producers. Any retention token with
  `actions:write` belongs to a dedicated trusted job; it must not be exposed to PR
  code or a privileged checkout of an untrusted head. No `pull_request_target` path.

## Bounded retention mechanism

Automate the same replacement-only proof currently performed manually. For each
successful published component, re-read its exact default-branch ref, key, cache
version, ID and bytes. Match a strict format/family schema and the producer's
successful upload evidence. Delete only older same-family/version default-branch
generations that the replacement supersedes. Re-read immediately before each
mutation, record the exact before/after IDs and bytes, and verify every other
current family and all active PR caches survive. Unknown keys, mismatched versions,
pagination failures or ambiguous state fail closed.

Coordinate trusted publications so admission is assessed against pending uploads,
not just an earlier inventory snapshot. GitHub job-level concurrency would
serialize whole producer jobs if applied naively, increasing queue/latency;
therefore a publishing-stage design and its transfer/storage cost need review.
An after-the-fact cleanup alone is insufficient—the observed current-family loss
occurred before the next manual cleanup. Include npm/Gradle producer growth in
capacity accounting without broadening deletion to unrelated keys.

No retention helper or new trigger is enabled by this proposal. Existing producer
paths and finite recovery remain; a source-only PR after eviction can be cold.

## Validation and decision gates for the CI change

1. Capture hosted component sizes without changing restore/save behavior. Preserve
   exact heads, package/unit graphs, queue/execution times and all check identities.
2. Test packaging/invalidation and retention as pure local fixtures: different
   targets/features/CRT/toolchains must separate; mismatched versions, forged refs,
   active PR caches and unrelated keys must never become deletion candidates.
3. Dry-run the proposed layout/retention against a frozen real inventory. Require
   complete family coverage, a projected steady set within the budget goal and
   safe largest-generation overlap before enabling default-branch writes.
4. In one reviewed rollout, seed only missing components via existing producer
   entry points, keep legacy caches as a bounded fallback, and verify every new
   component's upload and later restore. Remove legacy copies only after an exact
   same-family replacement is independently verified. No parallel duplicate seeds.
5. Compare a subsequent PR with the same dependency/feature graph: all native,
   exact 44 GPU, credential, platform and fresh shipping proofs still execute.
   Record actual restored keys, zero PR writes, job queue separately, compilation
   units, longest merge-gating execution and retained footprint after replacement.
   Roll back the new restore path if correctness or latency regresses; cache misses
   must fall back to normal installs/builds, never skip coverage.

Only after this holds through a replacement cycle should a third-party-only
shipping seed be evaluated. The current shipping tool requires fresh actual
`incant_runtime`/`run_scene` rustc commands and fresh symbols in a temporary target.
Any future seed must exclude **all** owned workspace outputs, preserve those
invocation checks, flags, abort behavior and source-free execution, and first
measure whether saved dependency compilation outweighs storage/restore cost.
