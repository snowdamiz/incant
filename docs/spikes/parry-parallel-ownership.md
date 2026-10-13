# Parallel BVH ownership in Parry 0.31.1

The native 2D and 3D physics backends now use a narrowly patched copy of the
published Parry 0.31.1 sources. Dependency versions, solver settings and public
tree layout are unchanged. [Provenance and costs](../../third_party/PARRY_PATCH.md)
record the two modified files and original package checksums.

Parallel leaf updates previously created mutable references to a whole wide
node before selecting a child. Sibling writes could therefore borrow overlapping
storage, and duplicate input IDs could target the same child. The patch rejects
duplicates and invalid reverse mappings before any mutation, then projects a raw
pointer directly to the exclusively owned child field. Buffer storage stays fixed
until the parallel operation joins. Parallel refitting now divides output buffers
into disjoint mutable slices, returns child summaries by value, and fills reverse
mappings after joining. That refit path contains no unsafe code.

This is an ownership correction, not a measured speed improvement. Unordered IDs
need a temporary array and sorting; ordered IDs avoid that allocation. Refit adds
a serial pass over output nodes. Phone measurements and the plan's numerical
backend adoption gates remain open.

Both physics modes pass 18 tests each, including eight shared BVH regressions
across 2D and 3D. The cases compare complete trusted tree state against serial
operations with 1/2/4 workers, sibling updates, duplicate and mixed batches,
sparse/recycled slots, resolving/nonresolving passes, removals and the 2048-leaf
parallel threshold. All-target Clippy, script compilation, browser-target physics
compilation, formatting and conventions pass. The source has independent review.
The four independent physics probe workspaces preserve the same source patches;
their locked metadata and the probe-lock guard pass. Only the corresponding
registry source/checksum fields changed in those locks, with no other resolution
updates. The initial missing-patch guard failure is retained.

Two additional Miri experiments remain **unsuccessful process runs**. With the
default experimental Stacked Borrows model, Miri reports a Crossbeam epoch
container-of retag while starting the Rayon pool, before the BVH test. With Tree
Borrows, the single 2D sibling test completes, then Miri reports nine unreclaimed
Crossbeam allocations at process teardown. Integer-to-pointer warnings also limit
the instrumentation. Neither run is reported as a clean Miri result or a complete
dependency soundness audit; the original diagnostics are retained. An independent
safe-Rust Rayon control, with no Parry, Rapier or engine dependency, reproduces the
same startup retag failure and the same nine teardown allocation shapes. This
isolates the instrumentation barrier without disabling any check or clearing the
dependencies of all safety concerns.

Checkpoint compatibility now hashes both local Parry source trees and the native
physics wrappers. This prevents an unchanged path dependency entry in Cargo.lock
from concealing a later runtime change. It does not admit untrusted deserialized
physics graphs or enable exact persisted physics saves. The full solver and those
save/load requirements remain separate work.

[Evidence](evidence/parry-parallel-ownership.json) preserves the source freeze,
ordinary passing checks, independent review and both unsuccessful instrumentation
experiments. No native-window, GPU, device, performance or phase gate is claimed.
