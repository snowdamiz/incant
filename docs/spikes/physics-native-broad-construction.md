# Private broad-phase manager construction

This default-off Rapier child reconstructs only `BroadPhaseBvh` from an immutable
validated broad-owner witness and the same cumulative allocation permit. The
private NumericFrame facade supplies the exact joined witness and expected
identity. Raw bytes never reach a live constructor. Native phase authentication
still belongs to the exclusive enclosing owner; the trusted test harness has
actually completed a positive step and checks both deferred flags and quarantine.
An inert phase tag, empty scratch or matching identity alone authenticates nothing.

Persistent reconstruction preserves pending-tree bits, pair membership and exact
insertion stamps, Coarena generations (including empty stale/invalidated rows),
neighbor order, prior leaf order, pending AABB order, frame/debt counters,
optimization strategy, adaptive margin and reinsert hysteresis. No refit, sorting
of adjacency, shape recreation or geometry normalization occurs during staging.
The pending tree remains pending. Native update processing resolves it normally.

The selected map has a separate, pinned allocation model. Hashbrown 0.17.1 uses
power-of-two buckets with its actual small-table rules and 7/8 load bound; its
request includes bucket payload, control bytes, alignment and trailing group.
Every supported control group is at most 16 bytes. The constructor charges that
conservative size before `try_reserve`. IndexMap 2.14.2 adds an index table plus
its separate `{HashValue(usize), key, value}` entries; each field padded to their
maximum alignment bounds any Rust field order. `try_reserve_exact` avoids rounded
entry growth. Both paths check returned capacity and reject insertion duplicates.
Neither plan treats a hash table as just a vector of pairs. Logical requests do
not include allocator bookkeeping/RSS, and native VecMap/Box OOM limits inherited
from earlier layers remain unchanged. Failed staging never refunds the permit.

Canonical pair order is not an invented equivalence: pinned broad update probes,
inserts and removes map entries, while adjacency vectors drive traversal order.
`serialize_pairs` sorts only the pair set/stamps. Stale deletions are explicitly
sorted before application (`update.rs:572`) and adjacency deletion uses native
`swap_remove`. No map iteration controls live pair-event order in this revision.
Tests retain native collision-event variant, both full handles and all flag bits.

Fresh scratch is admitted only for this completed-step subset:

| Field | Pinned overwrite or completed-operation invariant |
| --- | --- |
| updated_colliders, curr_updated_leaves | `update.rs:61–62` clears before use |
| update_workspace | `update.rs:98–99` takes and clears, then fills before iteration |
| update_batch_workspace/statuses | update clears/fills batch at line152; Parry `bvh_insert.rs:326–327` clears/resizes every status before reads |
| candidates_workspace (sequential) | `update.rs:325–326` clears before traversal |
| stale_pairs | `update.rs:451` clears before removed/changed adjacency scan |
| updated_mask | `update.rs:474–484` clears, resizes false, writes the current membership |
| BvhWorkspace.refit_tmp | `bvh_refit.rs:370+` clears trivial paths, otherwise resizes, overwrites each retained node and truncates before swap |
| rebuild_leaves | `bvh_optimize.rs:242/281/308` clears before each collection |
| optimization_roots | consumed and cleared at `bvh_optimize.rs:326` before a completed optimizer returns |
| queue/dequeue | `bvh_optimize.rs:380` drains deque; line440 clears heap after complete collection |
| traversal_stack | `bvh_traverse_bvtt.rs:30` clears before traversal |
| deferred_optimize_pending | false only for the separately checked completed phase; not inferred from serde(skip) |

The original workspace doc mentions retained optimization state; actual progress
counters live in the preserved tree, while the scratch containers above have the
cited lifecycle. Interrupted/deferred/quarantined states remain unsupported.
Tests exercise native completed mutations, not a panic-caught substitute boundary.

Focused controls compare four decoded frames and native continuation through
sleep/wake, removal/reuse and body/solver changes, with other owners explicitly
remaining trusted clones. Additional controls cover both optimization policies,
adaptive margin on/off, u32 frame wrap, collision events, queue/generation/identity
mismatches and allocation failure after the tree was staged. A test-only System
allocator observer measures actual map requests at capacity thresholds; it does
not claim to measure all parallel solver allocations.

This is not a whole-world installer. Narrow contact/manifold/workspace graph,
retained CCD/pipeline installation, full derived arithmetic/global work,
authenticated stable-ID/event composition and public save admission remain open.
No `PhysicsUnsupported` change, new normal runtime edge, GPU or device claim.

Focused deterministic and fast8 runs each passed ten tests in two dimensions.
Each dimension measured thirteen map capacities and compared 156 decoded-frame
continuation steps plus 64 policy/frame-wrap steps with eight collision events.
Those are 220 paired steps per dimension per mode; other native owners were
trusted clones. Fast8 used the exact previously accepted fast8 captures, with a
hash-bound copied-fixture receipt, rather than claiming a fresh capture.

Two initial focused failures are retained: Rust's collision-event enum lacks
PartialEq, so the test now compares a complete explicit event record; and frame
zero's valid pending queue is empty, so the hostile mismatch control now handles
both empty and nonempty queues. Neither correction changed production code.
Root source review accepted source001 (`9100d429`). Full deterministic and fast8
qualification each passed 225 snapshot tests and 36 private native tests; the
new ten broad tests are included in those 36, not additional cases. Both native
library Clippy runs, snapshot all-target Clippy, wasm default/construction builds,
std-only and ordinary-runtime graphs, 31 Python guards, scoped formatting and
convention checks passed. All source001 compiled inputs stayed unchanged. Four
executed native binaries are retained with executable mode; no blanket vendored
all-target lint or GPU claim is made. The original producer final review is now accepted; the unchanged receipt is
retained in [the current intake](evidence/physics-native-broad-main236/producer-final-root.json).
This acceptance applies to the original producer; current-main qualification is separate.

The [lossless evidence manifest](evidence/physics-native-broad-construction.json)
retains raw commands, logs, source copies, map-layout references, mode/fixture
identities and the two initial failures. The verifier independently recomputes
hashes, counts, selected features and the exact continuation/allocation markers.
