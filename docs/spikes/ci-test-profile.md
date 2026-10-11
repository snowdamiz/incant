# Functional CI profile measurement and adoption

The ordinary/GPU test graph and the subsequent custom-protocol tool build spend
substantial time compiling optimized workspace code after dependency restoration.
A local matched experiment used a separate named profile inheriting `release`
with only `opt-level = 1`. Shipping settings and fresh shipping storage were never
changed. This is a functional CI configuration, not a performance configuration
for players, games, reference devices or frame budgets.

## Matched local experiment

All four runs used frozen PR61 `c620868`, Rust 1.99.0, Apple M5 Pro/macOS 26.6,
`-j2`, disabled incremental compilation and distinct target storage per profile.
Each cold/warm pair executed the same commands and exact identity multisets of
451 ordinary tests/doctests and 56 ignored GPU tests. The measured chain includes
dev Clippy/audio compilation, the ordinary/GPU graph, all six then-current
custom-protocol tools, KTX/schema/save/gameplay/SDK checks and the two unchanged
independent fresh shipping proofs. Frontend setup/build was excluded equally.
Later main62+ audio/GPU composition and static-Wasm additions are outside this
comparison; current-tree validation is recorded separately.

| Local seconds | Release cold | CI cold | Release warm | CI warm |
| --- | ---: | ---: | ---: | ---: |
| Whole measured chain | 995.48 | 903.91 | 539.22 | 454.01 |
| Ordinary test command | 529.87 | 431.03 | 250.83 | 149.82 |
| Ignored/GPU command | 37.20 | 48.38 | 34.12 | 46.59 |
| Custom-protocol tools | 28.46 | 20.36 | 25.82 | 19.48 |
| Fresh runtime shipping proof | 86.51 | 79.41 | 71.99 | 82.30 |
| Fresh scripted-physics proof | 124.84 | 111.63 | 116.52 | 114.79 |

The measured chain was 9.2% shorter cold and 15.8% shorter after dependency-only
reuse (8m59s to 7m34s). GPU tests were slower at lower optimization, and unchanged
shipping steps varied. These are single local samples, not a Windows estimate or
causal attribution of every whole-job difference. Other engineering CPU work ran
concurrently. A 345.28s explicit native-review pause in candidate cold was excluded
from command-chain time and retained in raw elapsed evidence.

Warm preparation explicitly cleaned **both dev and the tested profile** for all
26 checkout packages, including the patched vendor package. It checked that no
owned fingerprints/build outputs remained and required the editor/runtime to
compile again. The first attempted warm run cleaned dev only, accidentally kept
release workspace artifacts, and was stopped/rejected; none of its timings enter
this table. Valid warm ordinary compilation spans were 198.78s and 91.10s. Each
compiled 170 Cargo units. The longest default editor unit was 41.36s versus 28.24s;
its report exposes no frontend/codegen/link split. Durations may overlap and are
not link-only attribution.

After removing owned artifacts, local tar+zstd level 3 projections retained
1,205,186,596 bytes for release and 1,158,758,951 for CI (~3.85% less), each with
15,007 files. These are local target-only projections, not actual hosted cache
uploads; registry content was unchanged/separate. No workspace artifact caching
was added. Exact test identities, all 128 retained command-log hashes, proof
identities, method hashes and timing summaries accompany the evidence.

## Production scope

The named `ci` profile is used only for functional ordinary/ignored tests, the
optional GPU-player all-target check and their existing executable tools in the
general/desktop workflows. Target lists, feature unification, assertions, GPU
serialization, credential workflows and check identities are preserved. Every
probe receives the exact `target/ci` executable explicitly; standalone probe
commands keep their prior release defaults. The renderer probe's optional binary
argument fails on a missing file rather than falling back or creating output.

The existing `release` and `shipping` profiles are unchanged. All four current
general shipping proofs continue to create independent fresh storage, verify
actual shipping codegen and compiler exclusions, package matched symbols, delete
source/build inputs and execute their existing positive/negative controls.
Lower-optimization tools only prepare fixtures; their output is not a shipping
performance measurement. No environment-wide RUSTFLAGS/profile override is used.

After restoring a native cache, `cargo clean --profile release` removes only the
obsolete target/release graph before building CI. This avoids retaining both old
release and new CI dependencies in one archive. Dev/registry inputs remain,
shipping storage is separate, and PRs remain restore-only. Current default-branch
producers and exact-family replacement retention continue; no extra producer or
cache namespace is introduced. Actual hosted cache size and Windows execution
must be observed on the new head before speed or retention claims are expanded.

## Current-tree functional validation

The candidate on main64 `77d86b8` passed 543 ordinary tests/doctests and all 66
ignored tests, optional GPU-player Clippy, all eight current custom-protocol
executables, KTX/schema/native-save validation, four independent fresh shipping
proofs (runtime, native audio, GPU player and scripted physics), and every selected
existing CLI/renderer probe. All 32 command steps passed and all 2,631 frozen
tracked/input hashes stayed unchanged. This was functional validation with a
separately cloned dependency target and CPU prefill during a native-review GPU
hold; its wall time is **not** another matched performance sample.

The later main/import union is validated separately before publication. The
selector's failure-after-output hardening changes only workflow predicates and
has its own failure-matrix test. Evidence retains the full shipping reports,
command/log hashes, exact tested profile inputs and safe cleanup-boundary check.
[Measurement and current-tree proof](evidence/ci-test-profile-2026-10-11.json).

Completed same-family default-branch cache replacement cleanup retained
8,594,322,567 bytes in 17 entries at the recorded snapshot. No unrelated or
active-PR entries were removed. Natural PR72 Linux restored the existing opt3
cache and completed in 9m50s execution after 3m55s queue; it does not measure this
new profile. Current iOS target-cache absence is recorded without attributing its
cause. [Retention and consumer evidence](evidence/ci-current-cache-retention-2026-10-11.json).
