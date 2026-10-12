# Physics save boundary: exact state is not a pose reconstruction

This bounded feasibility checkpoint leaves native physics saves explicitly
unsupported. No Rapier byte decoder, solver dependency patch, runtime bypass,
world version or ordinary-load policy is adopted. Main163 (`4347bf0f`) remains
the production baseline. Version 12 is reserved for a future accepted physics
world section; session version 3 need not change solely for that addition.

The executable diagnostic in
`tools/probes/physics-snapshot-boundary` compares two concrete paths using pinned
Rapier 0.36.0 / Parry 0.31.1, native enhanced-determinism and a two-thread Rayon
pool. The first copies trusted typed solver state into fresh pipeline objects.
The second reconstructs shapes, current poses, velocities and sleeping flags
through public constructors. Both retain stable overlap identities and the last
entered/exited records. No arbitrary serialized graph is decoded or stepped.

The trusted clone preserves all 960 compared continuation ticks across eight
cases (2D/3D at ticks 0, 30, 150 and 480): complete diagnostic serialized state,
handle index/generation/order, observed body state, current overlaps and subsequent
entered/exited notifications agree. The fixture includes boxes, a sphere/circle,
a capsule, compound rectangular ground, kinematic movement, sensors and recycled
slots. Compound ground uses the same kind of box composition as generated tile
geometry; this is not a tilemap-cooking or full NativeWorld test.

Pose reconstruction begins with equal visible body state in every case, but each
nonzero checkpoint diverges on the next tick. At ticks 150 and 480, both dimensions
also produce different later trigger notifications. Merely retaining the previous
overlap set does not repair physical divergence after contact state is reset.
For the settled fixture, the first differing event is an extra enter for stable
IDs 14/50 at subsequent step 10. The measured maximum translation difference is
0.2002873421 and linear-velocity difference is 1.9619992971 in scene units. These
are fixture observations, not a universal error bound.
Tick-0 reconstructed trajectories match this fixture for 120 ticks, while their
complete state/handle layout already differs. None of these results imply a
safe persisted load boundary: cloning preserves private workspace values that
serde may omit, and uses only trusted already-live objects.

## Why a small public-accessor adapter does not close exact persistent loading

The pinned state is larger than bodies plus contact impulses. The following
families must be admitted together before any imported world can be queried,
stepped, removed from or otherwise mutated:

| Persistent family | Pinned source | Required checked relationship |
| --- | --- | --- |
| Body/collider storage | Rapier `data/arena.rs`, `data/coarena.rs`, body/collider sets | Actual live count, bounded slot storage, complete acyclic free chain, free/live disjointness, handle generations, exact occupied order, no invented side table entries |
| Body state | Rapier `dynamics/rigid_body.rs` and components | Parent/attached collider bijection; body type, enabled/modified flags, finite current/next/CCD/force/mass/inertia/activation state; private IDs consistent with islands and solver mappings |
| Contact and intersection graph | Rapier `data/graph.rs`, `geometry/narrow_phase/mod.rs`, contact pairs | Endpoint validity, reciprocal adjacency chains, exact ordered membership, pair hints and event-list/back-reference consistency, contact feature IDs and supported dispatcher workspaces |
| Warm-start and recycle data | Rapier `geometry/contact_pair.rs`; Parry contact workspace types | Finite/ranged impulses, anchors, normals, manifold points, shape/subshape correspondence, recycle pose/feature correspondence; stale cache fields must be validated or explicitly reconstructed without changing continuation |
| Active and sleeping islands | Rapier `dynamics/island_manager/{manager,island,persistent}.rs` | Awake/sleep partitions, body reverse indices, contact connectivity and link locations, removal journal/split state, free-island identities and epoch relationships |
| Solver contact graph | Rapier `dynamics/solver/solver_contact_graph.rs` | Exact bucket sizes/order, pair/manifold references and reciprocal graph positions, color masks and disjoint writable-body ownership; no duplicate conflict hidden by a valid index |
| Broad-phase state | Rapier `geometry/broad_phase_bvh/mod.rs`; Parry `partitioning/bvh/bvh_tree.rs` and `utils/vec_map.rs` | Node/parent/leaf bijections, no cycles or live/free aliasing, metadata/AABB consistency, pending update lists, collider pair adjacency, frame/counter values |
| Native ownership | Incant `incant_physics{,_2d}_runtime` and `incant_runtime` | Stable-ID/config/handle bijection, retained overlap history and last events, sticky used/failed state, pending motion/query state, planar quaternion echo, exact world/scene/program compatibility |

These types mostly expose read views or derived operations. The internal fields
are private and their serde derives accept structural data without a whole-world
validator. A public body builder cannot recreate arena holes, previous generation
values, island partition/order, contact recycling or persistent solver coloring.
Calling collision detection to regenerate them is an observable state change,
not validation. Current native ray queries deliberately avoid collision-only
pipeline updates; replay must retain this property.

Parry has debugging `assert_well_formed` helpers, but they recurse/index into the
provided tree, panic and allocate a visited set. They are not a bounded error-
returning hostile-data decoder, and do not validate the enclosing Rapier graph.
Catching their panic is not a valid substitute in the shipping abort profile.
Likewise, checksums and a total file-size ceiling do not prove index, numeric or
parallel ownership invariants.

The identity receipt pins 34 source files (17 corresponding families in both
dimensions), totaling 29,095 lines. This is the size of the inspected source
families, not a claim every line was reviewed or an estimate of patch size.
Vendoring all four pinned source trees would bring 1,166 Rust files, about
12 MB and 319,548 lines before metadata/tests. A complete patch can share logical
implementations between dimensions, but it crosses all the above private
families and the Parry boundary. It cannot responsibly be presented as a small
isolated accessor addition. No such patch is introduced in this checkpoint.

## Concrete next implementation choices

**Checked pinned solver format.** Keep the exact same-build continuation target.
One owner would add a private, versioned, inert typed record format and checked
constructors in the pinned Rapier/Parry copies; another independent reviewer
would audit the graph invariants and malformed inputs before integration. Every
variable-size section needs checked lengths and aggregate allocation admission
before allocation. All validation must finish before obtaining a live solver.
Only Incant's supported rigid primitive/compound/tile-box geometry is admitted;
joints, soft bodies and custom dispatchers remain outside the current native
contract. Preserve native config/ID/event state in a separate typed owner section.
Then extend world v12 and the existing pending-session composition, constructing
the exact new world before UI/audio/guest effects. Exact source/feature/numerical
identity is required; universal RSS, mobile timing or final-system-header proofs
are separate claims. This is a multi-family adapter project requiring its own
explicit bounded implementation plan, not a continuation of this small probe.

**Bounded deterministic physics replay.** Avoid importing private graphs: start
from a checked original cooked physics seed and replay the exact accepted native
structural/motion/step operations through the existing solver. Record operation
order, IDs, current numerical mode and exact floating-point inputs. It must cover
all mutation routes, including changes synchronized for queries and root motion,
and preserve current no-collision-refresh query behavior. UI/VM/audio need not
execute during reconstruction; their existing checkpoint data is installed only
after the replayed physics owner is validated. This can preserve warm starts and
events without an untrusted graph constructor, but replay requires O(history)
storage and O(replayed steps × scene work) load work. A finite operation/byte/step
limit and cancellation are essential. Without a checked persistent anchor, a
rolling log cannot discard old history and still recover the exact state.
Periodic anchors return to the checked-format problem. Thus a bounded replay
contract can support explicitly limited scene histories, not arbitrary-duration
full saves; device/load-time budgets need measurement before adoption. No replay
logging or fallback is added here.

**Pose-reset ordinary load.** A named alternative could preserve current body
configuration/pose/velocity/sleep and overlap identities while declaring contact,
island and warm-start reset. The diagnostic shows this changes future state and
sometimes events, so it cannot satisfy the chosen exact continuation contract.
It remains an unadopted alternative, not a silent fallback.

The existing Rapier backend decision is unchanged. Separately, source inspection
still finds the previously identified whole-wide mutable node borrow in Parry's
parallel leaf update before child-field selection. This checkpoint neither
executes malformed states nor claims to close that dependency soundness issue.
Normal production numerical/parallel choices are left untouched.

## Evidence and limits

[Evidence](evidence/native-physics-snapshot-boundary.json) records exact source,
lock, executable, command and result identities, including the initial probe
compile typo and Clippy type-alias correction. The diagnostic's successful
assertions are actual CPU execution, not an engine save/load gate. There is no
new production snapshot API, package/fresh-process restore, GPU, device or
performance acceptance. Independent source and evidence review
[accepted this bounded diagnostic](evidence/native-physics-snapshot-boundary/review.json).
It does not approve a production snapshot format or lift any save guard.
