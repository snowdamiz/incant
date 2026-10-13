# Checked inert rigid-pair persistence

This extends the inert manifold witness with fixed rigid-pair persistence:
event bookkeeping, pair color/mask indices, recycling reference configuration
and a closed set of persistent composite query workspaces. The persistent
workspace maps contain native indices into prior manifold arrays; their keys,
indices and timestamps are now checked together before exposure. No live
constructor, normal dependency, runtime consumer or physics-save admission is
introduced. `PhysicsUnsupported` remains unchanged.

## Admitted persistent state

Each pair row matches the existing checked graph/manifold payload order and
inherits its exact build/dimension identity, collider generations, shape/child
witnesses and completed-narrow-phase restriction. Nonzero mutation journals were
already refused by that witness. An inert witness is not authentication of an
enclosing live world, and zero journal counts still require derivation from
checked journal payloads in the eventual whole-owner decoder.

Pair event status preserves the two pinned bits for a prior collision-start and
initial force-threshold event. Pair color permits native 0..=128 or uncolored
255. Body-mask slots permit bounded indices or MAX. No reciprocal island/mask or
parallel writer-ownership claim follows from these scalar-domain checks.

Optional ContactRecycleState contains the exact second-relative-to-first pose,
both absolute reference rotations, maximum extent and maximum drift. The pose
and rotations reuse the existing strict finite/unit-rotation admission, without
normalizing signs or bits. Extent/drift are finite and nonnegative, retaining
negative zero, subnormal and extreme finite values. These are explicit supported
domain restrictions; geometry-derived extent equality, configuration-derived
drift equality and arithmetic safety still need the complete owner state.

Supported workspace kinds are exactly:

- None, with canonical false timestamp and empty detector span. Primitive pairs
  may have zero or one ordinary manifold; composite pairs with no workspace must
  have no ordinary manifolds.
- CompositeShapeShape, requiring exactly one composite collider. Each native u32
  key occupies word zero; unused word one is zero, even when the composite is
  the second collider.
- CompositeShapeCompositeShape, requiring two composite colliders. Both key
  words preserve the original ordered collider subshape indices, including when
  native traversal chooses the opposite larger tree first.

All other wire kinds, including custom/trimesh/heightfield/voxel/soft workspace
representations, return typed Workspace refusal. No unknown kind becomes None.
The closed existing geometry witness independently excludes those shape types.
The trusted test observer also explicitly refuses any unexpected native variant;
it is not a production capture adapter.

Exact native map iteration order is stored as flat detector rows. Every pair's
range must be a contiguous complete partition. Detector timestamps must equal
the owning workspace's current timestamp. Keys must be unique, and detector
manifold IDs form a bijection onto that pair's ordinary manifold rows. IDs never
address solver clusters or another pair. Each key equals the owning manifold's
checked subshape indices, so existing child bounds/poses are reused. Duplicate
keys, duplicate/out-of-range IDs, stale timestamps, orphan rows and wrong shape
kinds are refused. Scratch keys are sorted for uniqueness; record order is never
sorted or rewritten. A checked accessor returns the referenced ordinary manifold.

## Wire and bounds

`ICPAIR01`, version 1, has a 56-byte common identity/count/reserved header. Pair
rows are 104 bytes, including an inline tagged 68-byte recycle payload; detector
rows are 16 bytes. Boolean and optional tags are exactly 0 or 1, absent recycle
payloads are all zero, and all words are little endian. Pair/detector counts are
at most 262,144 and complete frames at most 32 MiB. Identity, exact frame length
and matching pair count precede allocation.

The shared Budget charges both requested Vec payloads before allocating, then
one byte per checked manifold plus eight bytes per detector for scratch. Encoding
charges output separately. Failed validation does not refund charges. Preexisting
caller records and witnesses belong to the enclosing caller's aggregate budget.
These are requested payload bounds rather than allocator overhead/RSS claims.
Validation is bounded linear work plus per-pair detector-key sorting; no recursion,
unsafe code, panic-catching or deserialize-to-live is used.

The only dependency change is a direct **dev-only serde** edge to the already
locked version, for trusted one-way observation of private pinned fields. No
package identity/resolution changes. JSON object serialization cannot represent
native tuple map keys; the test-only serializer instead retains ordered key/value
rows, while keeping named struct fields. Its own control proves reversed key
order, tuple keys, negative zero and a positive subnormal survive. It implements
no deserializer and is absent from the normal dependency graph.

## Actual native and adversarial evidence

Frozen source002 passes **88 CPU tests in 14 nonempty groups**, with 16 summary
rows including empty library/doc groups, zero failed or ignored. All-target
Clippy, wasm32 library compilation, normal/runtime dependency guards, formatting,
conventions and diff checks pass. Source001 passed 87 cases; source002 changes
only tests: excludes timestamp-only differences from the churn oracle, exercises
both two-composite traversal-size conditions, checks ordinary cluster outer
allocation state and verifies the ordered observer. No failed test/build attempts
occurred. Commands, raw logs, frozen copies, exact lock delta and pinned references
are retained in [the evidence manifest](evidence/physics-inert-pair-state.json).

All prior actual native manifold observations now also validate and round-trip
pair states byte-for-byte. Additional public PhysicsWorld tests in both dimensions
observe nonzero event status and a retained recycle reference across changing
body position, then force a larger move and observe a changed full-update
reference. This witnesses reference-state reuse/change, not an instrumented
count of internal branch executions. Actual shape changes swap which ordered
collider is composite, then produce a two-composite tuple-key workspace. Moving
that footprint changes detector keys/manifold references independent of timestamp
toggles. Changing actual composite extents exercises both sides of the pinned
BVH traversal-size comparison, with exact key/manifold correspondence retained.

Hostile cases cover each detector reference/key/timestamp/partition, workspace
shape mismatches, field domains and unsupported variants, optional inactive data,
identity, every truncation, canonical tags, exact shared allocation and per-byte
mutations. Every admitted mutation must re-encode byte-for-byte. Shared manifold
fixture extraction is test-only and preserves the existing ten manifold tests.

## Cluster scratch audit and remaining restore boundary

Pinned pair_update swaps current solver_clusters into solver_clusters_prev before
rebuilding. The old previous storage becomes output. cluster_manifolds_for_solver
only searches the already initialized prefix `out[..num_out]`; each newly reached
output slot clears points, overwrites normals/subshape IDs, clears child poses,
resets all ContactManifoldData while retaining an empty solver-vector allocation,
and fills points before later reads. It truncates unused slots. Warm starts are
read from the swapped **current** clusters, which the manifold component records.

This establishes numeric overwrite-before-read for that rebuilding path, but is
not a blanket exact-reconstruction proof. Native set_subshape_pos(None) retains
an existing outer Some([None, None]) allocation. Ordinary clusters in the actual
test have outer None; arbitrary externally mutated previous storage may not.
Current option-state bits remain preserved by the manifold format. This slice
neither captures nor reconstructs previous-cluster storage and makes **no claim**
that omitting it preserves every observable allocation-state distinction for all
admitted native states. A future full-owner policy/constructor must close that
boundary before live restoration, rather than silently normalizing it.

Independent source/final review is accepted; the receipt is retained in the [main205 integration packet](evidence/physics-pair-main205/producer-final-review.json). Complete body/collider payloads,
shape acceleration and feature ownership, reciprocal solver graph/island state,
intersection/soft/journal state and enclosing owner authentication remain required.
Safe live construction and exact continuation must be proven before enabling
physics saves. No shipping, reference-phone, performance or GPU gate is claimed.
