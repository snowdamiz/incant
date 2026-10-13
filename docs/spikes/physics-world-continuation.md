# Joined rigid owner state and trusted native continuation

This slice closes the persistent solver/sleep-scan state around the previously
checked body, pair, manifold and island sections. It does not enable physics saves
or install hostile decoded records into Rapier. `PhysicsUnsupported` remains in
force. The native test route serializes only its own constructed owner, creates a
fresh `PhysicsWorld`, installs the joined-record-decoded sleep scan, and steps it
against the uninterrupted owner. This is stronger than cloning or pose rebuilding,
but it is not proof that arbitrary solver serde data is safe to deserialize.

## Joined boundary

`RigidOwnerContext` requires the exact same manifold witness for the checked
islands and pairs; those already share checked body, collider, attachment,
geometry and graph contexts. The record adds all historical sleep-scan slots,
body color masks, pair hints, 130 ordered solver buckets, edge/manifold references,
graph validity/epochs, force-event membership/back-references and pending flags.

The validator checks live pair color conflicts, required body mask bits, selected
plain-versus-cluster manifold ordinals and reciprocal packed graph positions.
When the graph is current it also checks qualified selection, per-body active
indices, first-manifold coloring and overflow. Stale caches retain their exact
bytes and explicitly require rebuilding; they are not treated as current solver
indices. Generic multibody buckets remain refused. Force-event membership is
checked against effective thresholds derived from the exact checked
[collider owner](physics-collider-state.md). Caller scalar threshold arrays are no
longer admitted. The empty-domain/epoch and journal/phase witnesses still require
complete owner composition; these partial records do not authenticate a world.

Native collider-node removal leaves conservative color reservations even after
its edges disappear. Actual 2D and 3D tests exposed this; the validator preserves
the exact mask superset while rejecting missing live bits and live conflicts.
Clearing those reservations would change future coloring. No solver algorithm is
changed here. Completed sleep can invalidate a still-structurally-reciprocal solver
graph, so its validity and epoch are retained rather than fabricated.

The `ICROWR01` version-1 format has a 100-byte header with the existing dimension,
precision and external build identity, six scalar/boolean words and eight bounded
array counts. Exact total length is checked before allocation. Each array is
bounded by `MAX_ITEMS`, the frame by `MAX_BYTES`, and record/scratch allocation by
the shared `Budget`. Booleans, trailing bytes, counts, references, reciprocal
positions, and foreign-context joins fail through typed errors.

## Minimal pinned adapter

Both exact registry Rapier 0.36.0 packages are vendored with licenses and a complete
`INCANT_UPSTREAM.json` file inventory. Five original files per dimension change:
the normalized/original feature declarations, two public re-exports, and an appended
persistent-island adapter implementation. One new module defines its public types.
`incant-checkpoint` is default-off and enabled only by the snapshot crate's dev
edges. Existing algorithms, serde layouts and default behavior are unchanged.
The snapshot dev edges now forward deterministic and optional simd8 modes through
the existing manifest policy; their normal dependency graph remains std-only.
Root plus all four isolated physics probe locks retain package versions while
switching only the Rapier package sources from registry to these exact paths.
The actual-compiler package audit admits only their exact path/version/manifest;
script runtime compatibility includes the newly path-patched solver sources.

The adapter copies or installs the exact historical `(stamp, eligible)` slots,
including unused tails. It refuses pending scans/removal journals, oversized
copies, stamp mismatch and undersized installation before any mutation. Its
stamp check does not authenticate an enclosing world. This is explicitly trusted
same-owner diagnostic infrastructure, not the future hostile restore constructor.

## Observable continuation

The native tests take three completed snapshots per dimension, restore a fresh
native owner, and compare 100 subsequent steps each. At every step they compare
all serialized native persistent state, exact sleep-scan slots, collision events,
and bitwise force-event fields. The route exercises resting contacts, sleep/wake,
impulses, disabling/re-enabling, removal and generation reuse. The joined record
is encoded/decoded and revalidated against the existing owner witnesses; its
resulting slots feed the native adapter. The collider and broad/narrow-phase state
still comes from trusted native serde rather than an inert live constructor.

A separate test advances a real stationary body to the step before sleep, places
only the sleep counter at `u32::MAX` in its trusted fixture, then executes the real
native wrap step. Exact retained slots sleep correctly and continue identically;
otherwise identical serde-only reconstruction with omitted slots remains awake.
This is a synthetic counter placement, not a claim to have run four billion steps.
Afterward both retained worlds are stepped another 32 times, including wake-up.
Atomic adapter refusal and hostile inert frame/ownership controls are also covered.

## Remaining admission work

No engine world/session codec or live restore constructor changes. The fixed rigid
collider configuration and persistent [narrow-phase indices/sensor history](physics-narrow-owner.md)
now have joined checked records; derived collider mass/pose
arithmetic and broad-phase indexing/manager composition, CCD/integration
configuration, complete journal/phase provenance, supported shape/workspace closure,
constructor ordering and resource budgets remain prerequisites. Joint/soft-body
owners remain refused. Near-wrap global-split cooldown arithmetic is a separate
pinned behavior boundary; the wrap test does not exercise every split history.
All results are CPU diagnostics on the recorded build, not phone/performance,
rendering, game shipping, or full physics-save acceptance.
