# Checked broad-phase owner and pending tree

This slice retains the pinned broad-phase manager as inert records joined to the
existing checked collider, rigid and narrow owners. It does not construct native
objects, install a BVH, authorize a completed pipeline phase, enable physics saves,
or change the world/session formats. `PhysicsUnsupported` remains. No vendor
source, feature, normal dependency or runtime admission changes are made.

## Owner and exact history

`BroadOwnerContext` borrows the exact checked narrow owner, collider-adjacency
Coarena and pending-tree witness. All identities match. The Coarena must borrow
that narrow owner's exact collider arena, and the tree must borrow its exact
checked collider state; independently validated equal bytes do not authorize a
replacement owner. The tree's occupied leaves exactly match currently enabled
collider slots. Current generations are checked separately through the collider
owner, since native tree leaves themselves store only slot indices.

`BroadOwnerRecord` retains the native sorted pair map and historical insertion
frames, each ordered neighbor list, the previous-updated-leaf sequence, pending
set-AABB handle sequence, wrapping frame, saturated optimizer debt, reinsertion
hysteresis, optimizer strategy and adaptive-margin flag. Adjacency spans preserve
payload row order independently from physical Coarena rows. Every pair has both
reciprocal adjacency entries, with no extra, missing or duplicate neighbors.
Native controls establish the map equals the union of contact and sensor graph
edges at the supported completed boundaries; the validator checks this exact
association, including filtered inactive edges. It never infers current overlap
or event masks from pair presence.

Insertion frames are historical u32 values, not ordered against the wrapping
current frame. Previous leaves can include removed slots and duplicates; they
remain bounded by retained arena capacity. Pending handles require exact current
generations/enabled colliders; duplicate calls and sequence order are preserved.
No sequence is normalized to a set.

`ICBROAD1` has an 84-byte little-endian header: identity, five counts, frame,
optimizer debt, two flag bits, strategy and a zero reserved word. Pair, adjacency,
neighbor, prior leaf and pending handle wire strides are 20/8/8/4/8 bytes. All
counts and their witness relationships, exact input length and enums/flags are
checked before allocation. Every array is bounded by `MAX_ITEMS`; total wire bytes
by `MAX_BYTES`. Actual Rust payload sizes and validation scratch are charged to
the shared cumulative `Budget`. Output buffers are separately charged. These are
requested Vec payload limits, not complete-world work, allocator-overhead or RSS
bounds. Whole-world source authentication remains external and unimplemented.

## A separate pending-tree type is necessary

The strict `ValidatedBvh` remains unchanged in meaning. Native end-step
`BroadPhaseBvh::set_aabb` can set a leaf flag to 3 and apply fat bounds while
`Bvh::insert_with_change_detection` propagates only unpadded bounds and does not
propagate change flags. Ancestors can therefore have flag 1 or bounds that do not
contain the changed leaf's margin in an ordinary completed positive step. The
retained 32-pass/4-failure run and exact native diagnostic preserve this discovery.

`ValidatedPendingBvh` is a distinct inert type, with no conversion to the strict
query-ready witness or native tree. It borrows the independently checked collider
owner and exact pending queue. Each named handle must identify a live enabled
leaf with flag 3. Its actual parent chain defines a bounded ancestor closure.
Every flag-3 leaf must be named; an empty/wrong/stale queue refuses. Only ancestor
flag-OR equality on this closure and parent containment of children on this
closure may be deferred. Unrelated sibling containment remains strict. All
ownership, topology, parent, leaf count, free-slot, depth, finite/range and planar
canonical-zero checks remain strict. Nothing rewrites flags, bounds or parents.

The same `ICBVH001` inert bytes have separate pending encode/decode entry points;
the ordinary strict decoder continues to reject the native counterexample. The
broad record must contain exactly the sequence borrowed by the tree witness.
This is structural admission, not proof of shape-derived leaf bounds, native
mutation provenance, usable spatial queries or arbitrary pipeline boundary.

## Focused native controls and refusals

Actual 2D/3D controls round-trip tree/manager bytes and compare public native
adjacency/query observations. Eight new cases cover three-pair hostile codecs,
reciprocal adjacency and payload permutations; geometry moving after the last pair
scan; sensor/group/enable toggles, removals and reused generations; sparse/bulk
reinsertion, both optimizer strategies, adaptive margins and a trusted synthetic
near-wrap frame placement followed by real native steps. The new cases compare
96 steps of uninterrupted versus trusted native reconstruction, including emitted
events and complete serialized state. The inherited 728-step controls also pass.
These trusted fixtures do not deserialize hostile input into Rapier.

Pending-tree hostile controls reject stale/absent queues, malformed pending
flags/counts/leaves, nonfinite values, invalid parents/free ownership, unrelated
sibling bounds and truncated frames. Broad controls reject wrong joined owners,
missing trees, missing/duplicate/reversed pairs, stale generations, malformed
spans/counts/flags/strategy, trailing bytes and exhausted budgets. Every accepted
single-bit broad-frame mutation re-encodes byte-exactly. Independent native
three-body controls require that the strict tree still rejects actual deferred
flags/bounds accepted by the separate pending witness.

Historical fixture failures remain: an initial move expected 600 m/s integration,
but the pinned default caps velocity at 400 m/s. The final fixture uses 120 m/s,
checks its actual integrated displacement and non-overlapping cuboid extents,
and retains the pair until the next scan. No production rule changed for this.

## Missing owner and remaining admission

Actual zero-dt native controls can schedule deferred optimization, move the tree
into the pipeline task, and return before the positive-step join. The broad owner
then contains an empty tree despite three enabled colliders. Empty tree bytes are
valid in isolation but fail the exact collider-membership join. The slice does
not reconstruct a tree or treat this phase as complete; full pipeline task/phase
ownership remains required. Negative-dt behavior is not separately exercised.

Pinned broad update sites clear updated/current/candidate/update/stale buffers
before filling them, and clear/resize/refill the updated mask. Previous leaves,
pending updates, pair adjacency and regime fields are instead retained explicitly.
The optimizer workspace and deferred task need the full pipeline review; a
`serde(skip)` annotation alone is never evidence that state can be dropped.
Native pipeline persistent/scratch ownership, callbacks/event queues, CCD joins,
shape-derived BVH coherence, aggregate work limits, hostile live construction and
ordinary physics-bearing save/load remain incomplete. No performance, device or
complete physics-continuation gate is claimed.

Independent source review and root final evidence review accepted this scoped
implementation. All 151 CPU tests pass (20 groups, no ignored cases), including
824 designated trusted continuation steps in both dimensions. Strict all-target
Clippy, wasm32 library compilation, normal runtime dependency graph, conventions,
formatting and diff checks also pass. All owned checks are terminal and reaped.

The [reviewed packet](evidence/physics-broad-owner.json) and source001 freeze remain
unchanged. The [final wrapper](evidence/physics-broad-owner-final.json) records this
post-review documentation update. Publisher integration is pending; none of the
remaining live-admission or performance boundaries above is relaxed.
