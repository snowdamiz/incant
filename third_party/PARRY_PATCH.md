# Parry 0.31.1 parallel BVH ownership correction

The two directories contain the complete published `parry2d` and `parry3d`
0.31.1 crate archives. `parry-upstream.json` records both registry package
checksums and every original file hash. The Apache license is copied from the
recorded upstream revision; the published archives omitted that file. The crate
manifests, versions, dependency constraints and default features are unchanged.
Separate directories preserve normal Cargo packaging and source-discovery paths.

Only `src/partitioning/bvh/bvh_insert.rs` and `bvh_refit.rs` differ from the
published source, identically in both dimensions:

- Batch updates reject duplicate IDs before touching either the tree or output
  statuses. Safe bounds and reverse-identity checks precede the parallel writes.
  Each worker then borrows only its selected left or right node through raw field
  projection. It never creates a mutable reference to the enclosing wide node.
- Parallel refitting partitions its writable node and parent buffers into disjoint
  slices before joining child tasks. It returns each child summary by value and
  updates the leaf reverse map after joining. The parallel refit contains no unsafe
  code and never shares whole mutable vectors between tasks.

The sorted-ID preflight allocates nothing. Unsorted IDs use one temporary `u32`
array and an O(m log m) uniqueness check. Refit adds one sequential pass over its
output nodes to update reverse mappings. These are explicit costs; no throughput
or reference-phone performance improvement is claimed. The solver, numerical
mode selection, hierarchy layout and flag math remain the same.

Both runtime physics crates compile the shared public regression suite in
`tools/probes/parry-parallel-tests.rs`. It compares full trusted BVH state with
the serial implementation across 1/2/4-worker pools, the 2048-leaf split boundary,
sparse/recycled slots, repeated resolving/nonresolving passes, sibling updates,
mixed insertion/update batches and subsequent removals. Duplicate existing and
new IDs must refuse without any tree or status mutation.

Runtime checkpoint identity includes both patched source trees and native physics
owners, because local Cargo patches no longer have registry checksums in the lock.
This does not enable persisted physics loads, validate arbitrary deserialized
Rapier/Parry graphs, or audit every unsafe operation in these dependencies.

## Default-off typed BVH construction layer

`INCANT_CHECKED_BVH.json` in each Parry/Rapier vendor records the layered
`incant-checked-restore` delta after the accepted island producer. It adds only
immutable validated-witness tree constructors and an optional std-only shared
allocation utility. All original patch inventories remain historical. Pending
ancestor bounds/flags stay exact; this is not a query-ready promotion or world
installer. No unsafe code or raw/serde live conversion is added. VecMap capacity
allocation is precharged but remains infallible under process OOM.
