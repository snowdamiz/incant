# Inert checked physics records

`incant_physics_snapshot` is the first checked-format component of the exact
physics-save implementation. It encodes and validates inert Parry 0.31.1 BVH and
VecMap fields plus the native rigid geometry subset. It has no normal dependency,
runtime consumer, unsafe block, live Parry/Rapier constructor or serde decoder.
The existing `PhysicsUnsupported` guard and world/session versions are unchanged.
This component does not yet save or restore a physics world.

The base is the accepted Parry ownership correction `396f0602`. Its vendored
types are observed only in tests, through one-way serialization of trusted live
trees. Test data is converted to inert numeric records; no serialized bytes are
ever deserialized into a live dependency object. This checks compatibility with
actual pinned fields without opening the eventual construction boundary.

## BVH admission

The record preserves ordered wide nodes, packed parent references, the complete
sparse leaf VecMap (including trailing empty slots and its stored occupied count),
the free-node reuse stack, and both optimization counters. Active floating-point
fields are exact f32 bits, including signed zero. Unused planar Z words and the
special single-leaf right node are canonical zero.

The iterative validator proves that every node is either reachable once or is
in the unique free partition; root/free/shared-child/cyclic aliases are rejected.
Each internal edge has the exact packed reverse parent, each leaf has one matching
map entry, and every occupied map entry is visited exactly once. Free nodes must
be zeroed. Their stale parent scalars remain bounded but are not live graph edges.
The root parent is zero. Empty and single-leaf storage have explicit admission.

Counts and changed/pending metadata must match child sums and flag ORs. Flags
0, 1 and 3 are admitted; flag 2 is not a defined pinned state. Active AABBs must
be finite, ordered, contained by their parent and bounded to +/-1e9. Loose parent
bounds remain exact; validation does not refit them. Pending trees are admitted
only when ancestor metadata and bounds are already propagated. An unpropagated
intermediate update is rejected rather than repaired. This is a quiescent-section
boundary, not a promise to capture every solver instruction boundary.

`rebuild_frame` retains any u32 value, including the wrapping maximum.
`rebuild_start` is bounded by the admitted historical capacity, not by the current
leaf count: removals can leave that cursor beyond the present count until the
next optimization wraps it. Free-list order and sparse slot length are never
sorted, compacted or recreated.

## Wire format and work limits

Both formats are little-endian version 1. Every decoder receives an independently
selected expected `{dimension, build[32]}` identity. The crate compares identity;
it does not discover, authenticate or authorize a build. A future owner must bind
the identity to exact source, solver features and numeric mode. That owner must
also supply file integrity and complete cross-section identity checks.

| Format | Header | Ordered payload |
| --- | --- | --- |
| `ICBVH001` | 80 bytes: version/dimension/f32 marker, build identity, five lengths/counts, two optimization counters, zero reserved words | 64-byte wide nodes; u32 parents; u32 sparse leaf slots (`u32::MAX` is empty); u32 free stack |
| `ICGEO001` | 56 bytes: version/dimension/f32 marker, build identity, kind, part count, zero reserved word | 44-byte parts: closed shape tag and three parameters, translation, quaternion |

Unknown versions, dimensions, scalar modes, shape tags, nonzero padding and any
length mismatch fail with a typed error. Header/count/identity/total length checks
precede allocation. All vectors use checked requested sizes and fallible reserve.
Decoding produces private immutable validation witnesses; encoding accepts only
those witnesses, so a caller cannot mutate a validated graph behind the encoder.

The hard bounds are 262,144 wide nodes, sparse leaf slots or tile parts; 128 tree
levels; 32 MiB per wire section; and a caller-supplied cumulative requested Vec
payload budget capped at 64 MiB. Record storage, validation scratch and encoded
output each charge that same budget. Charges are not refunded on failure. The
scratch traversal has bounded node/leaf marks and one bounded iterative stack;
there is no recursive walk. This accounts for requested payload, not allocator
rounding, process RSS or caller-owned input/records passed directly to `validate`.
Multiple sections require the same enclosing budget if their aggregate is to be
bounded; callers cannot treat repeated fresh budgets as a world limit.

## Closed geometry boundary

The geometry section admits native boxes, balls and Y capsules in both dimensions;
ordered 3D primitive compounds have at most 64 parts. Generated 2D tile compounds
contain only axis-aligned boxes, with native tile extent and cell-size bounds.
Primitive records have identity local poses. Compound quaternions must already
be finite and unit length within the f32 conversion tolerance; their signs and
bits are preserved, never normalized during decoding. Dimensions, positions and
shape sizes use the existing native admission ranges.

This validates geometry values only. It does not yet establish compound internal
acceleration-tree/part correspondence, cooked source binding, stable native IDs,
collider handles or attached-body relationships. Nested/custom shapes, arbitrary
meshes, heightfields, joints and soft bodies are outside this closed subset.
No geometry witness can construct a live shape in this component.

## Verification and remaining work

Eighteen focused CPU tests currently pass: twelve hostile BVH/wire cases, four
geometry cases and two actual pinned-Parry observation sequences. The sequences
cover sparse inserts, 129 leaves, removals/reuse, refits, forty incremental
optimization/refit cycles, and reduction to single-leaf/empty storage in both
dimensions. Exact records and canonical bytes round-trip. Adversarial cases
cover every truncation, overflow lengths, identity and header failures, malformed
partitions/edges/map entries, numeric/metadata failures, exact depth boundaries,
budget exhaustion and byte mutations. Scoped all-target Clippy passes.

The normal library also compiles for wasm32. The selected normal dependency graph
contains only this crate; the existing 2D/3D physics runtime dependency guard,
formatting and generated conventions checks pass. All 963 pre-existing locked
package identities remain unchanged. Dev-only trusted observation enables serde
edges and adds pinned `serde_arrays 0.2.0`; the normal crate has no serde edge.
Independent source review accepted the eleven frozen inputs with no blocking
finding. Final evidence review remains pending. [Retained evidence](evidence/physics-inert-snapshot.json)
binds that review, source copies, exact commands, raw logs and the lock identity audit.
No new device, GPU, shipping package, performance or exact-save continuation gate
is claimed. The next checked families remain arenas/generations, broad-phase
pending sets and pairs, contact/warm-start graphs, islands, solver conflict
ownership, shape/feature correspondence and complete native owner state. Every
family must be admitted before any future live solver is installed; trusted
clone continuation and this inert validator alone cannot close that boundary.
