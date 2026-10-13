# Checked rigid collider properties

This extends `incant_physics_snapshot`'s inert boundary with the complete fixed
configuration of the supported rigid collider owner. It does not construct native
colliders, restore a live world, change a world/session format, or remove
`PhysicsUnsupported`. The normal dependency graph remains std-only and forbids
unsafe Rust. The exact pinned Rapier 0.36.0 sources are unchanged.

## Record and joins

`ColliderStateTable` uses the exact checked collider arena payload-row order.
Each row retains world pose, solid/sensor kind, density/mass/explicit local mass
properties, material coefficients and combine rules, collision and solver groups
(including their And/Or test modes), active collision types, hooks/events, native
enabled tri-state, contact skin, force threshold, change flags and all 128 bits of
user data. Soft/deformable associations and the DEFORMED change bit refuse.
Shapes and parent-local poses remain in the existing checked geometry and
attachment records; no lossy shape reconstruction or second shape format is added.

The validator takes a checked body owner and an exact collider-row geometry slice.
Dimension/build identity and row counts must match. The joined rigid owner now
requires the same body and geometry witnesses already used by its manifold graph,
not merely equal cloned records. Its force-event membership derives each effective
threshold from the checked collider's event bit and threshold. The former caller
scalar threshold array is removed. Native event-disabled thresholds use finite
`f32::MAX`, matching Rapier.

Every floating field is stored as exact f32 bits. Finite/nonnegative physical
scalars, pose rotations, dimension-specific padding and explicit local mass fields
are checked. Negative zero and positive subnormals remain exact; restitution is
not arbitrarily clamped to one. Enums and bitmasks reject unknown values, including
the holes in the native active-collision mask. No callback is authorized by storing
known hook flags. Any nonempty supplied mutation journal or nonzero rigid collider
change flags refuse before returning a validated record. The supplied journal is
still a partial boundary witness, not authenticated phase provenance.

`ICCOLL01` has a 52-byte header and exactly 168 bytes (42 little-endian words) per
row. The header retains the existing dimension/precision/build identity, count
and reserved word. Density/mass use a fixed mass payload whose inactive words
must be positive zero; explicit properties use all eleven words. Unknown tags,
noncanonical booleans, padding, truncation, trailing bytes and incorrect exact
length refuse. `MAX_ITEMS`, the 32 MiB encoded-frame limit, and the shared bounded
allocation budget apply before record allocation; encode separately charges its
output bytes. The frame limit can be stricter than the global item limit.

## Actual native semantics

Enabled status is history, not a derived parent invariant. In both native 2D and
3D, explicitly disabling then enabling a collider under an already-disabled body
leaves that collider `Enabled`. Detaching a `DisabledByParent` collider retains
that enum even when no parent remains. The initial overstrict parent-coherence
validator rejected these two real completed-step states. That failed run and its
source are retained; the corrected validator preserves all three states exactly.
Recomputing them from the current parent would change subsequent behavior.

Focused tests cover byte-exact round trips, every scalar enum, all mass modes,
malformed finite/flag/padding cases, all truncations, accepted single-bit mutations,
wrong build/row counts, quota exhaustion and reversed arena payload order through
4,096 standalone rows. Actual native observers compare decoded values with public
getters and exercise material/group/hook/event settings, u128 data, mass changes,
sensors, explicit and parent-driven disabling, standalone objects, removal and
handle reuse. Sensor intersections independently demonstrate asymmetric Or,
mixed And/Or refusal, and reciprocal And admission. Primitive boxes, planar tile
compounds and spatial compounds are captured from actual native shapes.

All existing trusted native continuation diagnostics now join these collider
records, including three snapshots and 300 subsequent steps per dimension,
bitwise native state/sleep scan/events, plus the near-wrap control. Their native
world reconstruction remains explicitly trusted serde of their own fixture;
no hostile decoded collider data enters Rapier. Equal-but-distinct body/geometry
witnesses and a changed event flag incompatible with an actual force-event cache
are rejected by the joined owner.

## Evidence and limits

Final source and CPU evidence have independent root acceptance. The full snapshot
suite passes 135 tests (zero failures/ignored); strict all-target Clippy, wasm32
library compile, normal runtime dependency graph, conventions, fmt and diff checks
all pass. All owned processes are terminal and reaped. Integration remains pending.

The [final receipt wrapper](evidence/physics-collider-state-final.json) joins the
[source review](evidence/physics-collider-source-review.json),
[final audit](evidence/physics-collider-final-review.json), and unchanged
[audited packet](evidence/physics-collider-state.json). Its 73 lossless records,
48 source/baseline/native inputs and two copied executable test binaries remain
hash-bound. The audited packet retains its original pre-final-review status; this
paragraph and the wrapper record subsequent acceptance without rewriting evidence.
Original logs and the first false-invariant regression remain under
`~/.codex/incant-evidence/physics-collider-state-001`.

Derived mass/tensor/transform arithmetic, broad/narrow-phase managers and
intersection graphs, CCD/integration configuration, full pending-journal and phase
provenance, joint/soft-body domain closure, live constructor ordering and aggregate
world budgets remain prerequisites to engine save admission. The new row's checked
fields do not authenticate a world or prove every derived solver invariant. No
shipping game, GPU, device or performance claim is made.
