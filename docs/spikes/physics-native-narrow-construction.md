# Private checked narrow-phase construction

This slice stages an exact primitive-only `NarrowPhase` from a borrowed immutable
`NumericFrame` and the existing cumulative non-Clone allocation permit. It does
not install a physics world, accept raw native serialization, authenticate a
completed runtime owner, or change `PhysicsUnsupported`.

Both dimensions preserve graph node/edge order, collider generations and stale
Coarena rows; contact and sensor event state; tracked-point and solver-contact
bits; manifold/subshape metadata; current solver clusters and recycle references;
solver colors, bucket order, hint/mask tails and epochs; and exact force-event
lists/reverse positions/flags. Unknown event/solver flags remain typed errors.
The constructor uses the pinned native default dispatcher within a rigid single-primitive domain. A custom dispatcher is not represented or authenticated by the frame and remains unsupported at the future enclosing-owner boundary.
A non-None composite workspace is refused, never rewritten as absent. Soft body
state and compound shape provenance remain outside this constructor.

Allocation accounting uses the same pre-admitted shared permit as other private
owners. Every owned vector payload is charged before a fallible reservation;
2D two-point/two-solver-contact ArrayVec storage is inline, while 3D payloads are
charged separately. The native subshape Box and default-dispatcher Arc payload
plus control block are charged explicitly. The actual native test allocator
checks request bytes/call counts. These are logical allocator requests, not
allocator metadata/RSS bounds; stable Box/Arc allocation still retains process
OOM behavior. The pre-existing test-only allocation observer was mechanically
moved from broad-map tests to one shared cfg(test) module, retaining its exact
System delegation and thread-local counters.

## Fresh scratch, with native overwrite sites

The paths below are identical in both pinned Rapier dimensions. No omission is
justified merely by `serde(skip)`.

| Field | Native behavior before use |
| --- | --- |
| `update_candidates` | `narrow_phase/mod.rs::collect_pairs_to_update` clears before collecting. |
| `awake_body_mask` | `mod.rs` clears, resizes false and fills the current awake set. |
| `body_qualify_info` | `solver_graph.rs::rebuild_body_qualify_info` clears/resizes; maintenance calls it for an empty vector even when the graph is otherwise valid. |
| `solver_graph_dirty` | `contacts.rs` takes and clears serial storage; parallel collection creates the current dirty list before maintenance. |
| `solver_color_todo` | `contacts.rs` takes then clears before creating this update's recoloring work. |
| `retired_pairs` | `pair_management.rs` pops then `ContactPair::reset_for_reuse` resets owners, event status, all rigid history, color and body slots before insertion; only reusable capacity survives. |
| `solver_clusters_prev` | `pair_update.rs` swaps the current solved clusters into the previous source first. `contact_clustering.rs` clears/rebuilds the old output slots, truncates and transfers warm starts from that current source. A separate native compound control exercises populated previous storage; it is not compound-constructor admission. |
| `soft_self` | Fresh empty storage only within explicitly rigid-only admission; no soft continuation claim. |

Current solver clusters, warm starts, force-event status and recycle state are
preserved, not classified as scratch. The compound scratch control is limited to
its observed native history, not arbitrary historical compound/subshape reuse.

## Observable scope

Actual native captures are one-way encodings through the complete checked inert
join. In-crate tests decode those bytes and construct only the narrow phase; all
other native owners are explicit trusted clones. The original and reconstructed
worlds receive identical public mutations. They compare serialized persistent
native state, copied pipeline verdict/deferred/quarantine metadata and complete
collision/force event records. Event records preserve every handle/generation,
flag, start/stop distinction and scalar/vector float bit; they are sorted as a
multiset because parallel callback scheduling is not an event ordering promise.

The fixture exercises nonzero warm starts, sleep/wake, removal/generation reuse,
sensor conversion, event enable/disable and force-threshold crossings. A negative
control clears the retained force-start bit and invokes the actual post-solve
native emitter on the same saved impulses. This is an event-identity distinction,
not a claim of changed trajectories. Historical compile and fixture-oracle
failures remain retained with their original source copies.

Independent source review accepted source001 (receipt4953bdcc), and scoped CPU qualification passed. Final independent producer review is accepted (`1c702483`), retained in [the current intake](evidence/physics-native-narrow-main237/producer-final-root.json). Current-main qualification remains separate. The remaining
whole-world constructor, completed-step runtime authentication, shared stable IDs
and event queues, full allocation/work composition and public save entry are
separate requirements; this private owner does not complete exact restoration.

## Retained execution

Deterministic and fast8 each pass 227 snapshot tests (27 summaries, 26 nonempty)
and 45 private native tests (two groups), including the nine new focused controls.
The new controls run 312 decoded paired continuation steps per dimension and
mode, plus 32 trusted 3D compound scratch steps per mode. Eight native owner
constructions per dimension/mode have exact measured logical request bytes and
call counts. Subshape payload requests are 40 bytes in 2D, 72 bytes in deterministic
3D and 96 bytes in fast8 3D; each is one allocation.

The event fixture reports 25 collision and 102 force records per dimension/mode,
including five started and 97 ongoing force records. Threshold, sensor and event
policy changes happen through native public setters; the omission control invokes
the native post-solve emitter, with no fabricated event expectation.

Strict native-library Clippy passes in both modes; snapshot all-target Clippy,
default/construction wasm checks, std-only/ordinary runtime dependency graphs,
31 feature/probe guards, scoped Rust formatting and conventions pass. This is
scoped native-library lint, not blanket all-target vendored lint. Cargo.lock and
all 968 root package identities are unchanged. Private native unit-test harnesses
retain only the previously pinned bincode/oorandom dev extras.

Six historical focused failures are retained: import/lifetime, test graph API,
test graph index type, premature next-tick event expectation, inaccessible graph
witness, and empty previous-cluster fixture under native recycling. The first
cluster fixture did not reach the intended precondition; disabling recycling in
that trusted compound diagnostic produces a populated previous buffer and passes.
The actual constructor and primitive settings remain unchanged by that diagnostic.

[Evidence manifest](evidence/physics-native-narrow-construction.json) and lossless
archive retain exact source generations, all command streams, mode-specific
capture bindings and four executable native test binaries. The deterministic
event fixture is checked in. The private fast8 harness substitutes fresh native
fast8 event captures and accepted prior fast8 island captures, with exact byte
bindings; it does not reinterpret deterministic whole-world bytes as fast8.
[Independent source receipt](evidence/physics-native-narrow-source-review.json).
