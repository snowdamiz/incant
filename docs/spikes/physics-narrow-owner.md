# Checked narrow-phase owner and sensor history

This slice adds an inert, bounded representation of the persistent narrow-phase
owner around the existing checked rigid solver and collider records. It does not
construct a live Rapier owner, enable physics saves, alter a world/session format,
or remove `PhysicsUnsupported`. Normal dependencies remain std-only and unsafe
Rust is forbidden. Neither pinned Rapier dimension is modified.

## Exact joined representation

`NarrowOwnerContext` joins the checked rigid owner, a checked collider-index
Coarena and a checked intersection graph. The Coarena must borrow the exact
collider arena already used by the rigid owner's collider/body/attachment chain;
equal bytes from a separately validated arena refuse. Build/dimension identities
and payload counts must match. Contact graph and manifold ownership come from the
existing exact joined rigid context, not a second caller-supplied graph.

`NarrowOwnerRecord` retains both native graph indices per Coarena payload row,
each intersection node's collider handle, and both intersection payload booleans.
Graph physical indices are distinct from payload rows. Both directions of every
graph-index/collider-handle association are checked, including exact generations.
A node may remain isolated after edge removal or a solid/sensor type transfer.
Missing graph indices use `u32::MAX`; stale/invalidated Coarena rows can remain
with these invalid indices, but cannot reference a live replacement node.
The lookup API requires a live checked collider before using the Coarena getter.

Completed contact edges must connect solid colliders. Intersection edges require
at least one sensor and reject self-pairs and duplicate unordered pairs. Existing
checked graph adjacency, edge orientation, node order and payload order remain
unchanged. The validator sorts only its budget-charged duplicate-check scratch.
This is a completed-owner structural boundary; the prior journal witnesses do
not authenticate all pending phases or arbitrary callbacks.

`ICNARR01` is a version-1 little-endian frame: a 60-byte header contains the existing
identity, three array counts and a zero reserved word. Each index pair, handle,
and intersection payload occupies eight wire bytes. Booleans admit exactly zero
or one. Counts must match checked witnesses before allocation, each array is
bounded by `MAX_ITEMS`, the aggregate frame by `MAX_BYTES`, and record plus scratch
allocations by the shared `Budget`. Encode charges output bytes separately.
These limits are not a complete world-wide memory admission policy.

## History and actual native behavior

`intersecting` and `start_event_emitted` are independent history, not an inferred
current-mask invariant. Both dimensions demonstrate the following through real
native steps, checked records and byte-exact codec round trips:

- Enter, stay and exit retain the exact native query values and event order.
- Disabling collision events before a filter-induced exit can leave
  `intersecting=false, start_event_emitted=true`.
- Enabling events on an already intersecting pair does not invent an enter.
  Its later filtered exit can emit a stop even though no enter was emitted.
- Removing a pair with retained emitted history still sends the native sensor
  removal stop when the current event mask is disabled.
- Solid/sensor transfers keep isolated nodes, while removal repairs swapped
  node/edge indices. Reusing the collider slot changes generation and does not
  alias the removed collider's query.

The observer compares the decoded graph's ordered adjacency/query results with
Rapier's public intersection iterator and checks exact collider-to-node indices.
Hostile tests operate only on inert records: wrong owner/build/generation,
missing/invalid reciprocal indices, self/duplicate edges, over-limit counts,
invalid booleans, truncation/trailing data and exhausted budgets. Node/edge and
Coarena payload permutations are tested independently. Accepted single-bit frame
mutations must re-encode exactly; historical booleans are never normalized away.

## Skipped scratch is not automatically reconstructible

The source audit distinguishes each native skipped field. `update_candidates`
is cleared then collected, sorted and deduplicated before processing;
`awake_body_mask` is cleared/resized/refilled for each update. Sequential contact
updates clear `solver_graph_dirty`; the parallel path produces a fresh collected
list. `solver_color_todo` is cleared before transitions. `body_qualify_info` is
rebuilt when empty or when the solver graph/epochs demand a rebuild. These exact
pinned reset/read paths are retained with the source freeze. Their annotations
alone are not evidence.

The retired contact pool retains allocations, but `add_pair` calls
`ContactPair::reset_for_reuse` before consuming one: new handles, empty event
status, cleared manifolds/clusters/workspace/recycle/soft state, uncolored solver
status and invalid color bodies. An actual spatial-separation test retires a
contact, takes a trusted native snapshot, then compares reused-pool and fresh-pool
paths for 64 steps per dimension. It alternates contact, sensor transfer and
spatial exit; all serialized native state, exact emitted events and sleep scan
state must match. This supplements the existing 600-step trusted continuation
route. It is not a performance or arbitrary scratch reconstruction claim.

The custom query dispatcher is not identified or admitted by this record.
Soft-self state and soft-body owners remain outside the supported rigid domain.
Native restoration in these tests remains trusted serde of their own constructed
fixtures, with the already checked sleep-scan adapter. Hostile decoded data never
enters a live constructor. An omitted external event queue cannot be recovered
from pair history alone; that enclosing lifecycle remains future admission work.

## Evidence status

The eight new focused native tests pass in actual 2D/3D, including 128 continued
steps and ordered event controls. The first compile attempt failed because the
native event type lacks `PartialEq`; its unchanged production source, failed log
and test correction are retained. Independent source and final evidence review have accepted the unchanged
implementation: 143 CPU tests (zero failures/ignored), strict all-target Clippy,
wasm32 library compilation, normal dependency graph, conventions, fmt and diff
checks all pass. All owned processes are terminal and reaped; publisher integration
is pending. No GPU, device, shipping-game or performance claim follows.

The [final receipt wrapper](evidence/physics-narrow-owner-final.json) preserves the
unchanged [audited packet](evidence/physics-narrow-owner.json), its 108 lossless
records/52 source and pinned inputs, and independently accepted receipts. The
audited source001 documentation retains its historical pending wording; only this
closeout and STATUS changed after acceptance. Raw evidence and the copied test
executable remain under `~/.codex/incant-evidence/physics-narrow-owner-001`.

Derived physical arithmetic, broad-phase spatial/pair managers, CCD/integration
configuration, complete journal/phase/dispatcher ownership, constructor ordering
and aggregate world budgets remain required before live save admission. Joint and
soft-body domains remain refused.
