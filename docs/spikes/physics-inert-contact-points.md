# Checked inert contact points and solver rows

This adds the complete tracked-point and solver-contact arrays of one pinned
rigid manifold to the std-only inert format. It preserves the numeric state that
supplies warm starts, including 3D world-space friction and frozen lever arms.
It is not a complete manifold, pair, world snapshot or restore path. There is no
live constructor or runtime consumer; `PhysicsUnsupported` remains unchanged.

## Pinned meaning and admitted records

Rapier 0.36.0 / patched Parry 0.31.1 are the source contract. The record contains:

| Native data | Inert representation |
| --- | --- |
| Tracked point | Two local positions, signed distance, two exact packed feature IDs |
| ContactData | Normal impulse; tangent impulse; retained normal/tangent/twist impulses; retained world-space tangent vector; two frozen world-space solver lever arms |
| Solver contact | Two anchors, signed distance, desired tangent velocity, packed contact index/new bit, explicit zero padding |

All floats are stored as their original `f32` bits. Active components preserve
signed zero and subnormal values. Every active float must be finite. There is no
normalization, coordinate clamp, impulse sign clamp or conversion of frames.
Finite maximum values and negative impulse fields are admitted by this leaf
validator; it does not establish physically consistent values or freedom from
overflow in future solver arithmetic. Anchor-frame interpretation still depends
on the enclosing body's CoM and manifold dominance state.

Both planar native point and solver containers are `ArrayVec<_, 2>`. The spatial
containers are `Vec`; no invented four-point limit is imposed. The checked
format admits at most 262,144 entries in each array, subject also to the aggregate
32 MiB frame limit. These are format limits, not a claim that native vectors have
that capacity. There is no scalar-array flattening across manifolds or clusters.

Planar vectors have a canonical positive-zero unused Z word. Planar tangent
impulses are native one-element nalgebra vectors, stored in element zero of the
two-element inert field. The unused second tangent words, twist impulse and
world-space tangent field are positive zero. Spatial solver padding is the
explicit private native SIMD padding field, observed through trusted one-way
serialization; it must be positive zero. The planar wire slot is also zero.
This is a stricter supported padding domain, never a bit rewrite.

Packed features keep their high-two-bit kind and low-30-bit index. Exact zero is
UNKNOWN; vertex and face encodings are accepted in both dimensions and edge
encodings only in 3D. Every constructor-supported index, including the maximum,
is retained. Native `unpack` also maps malformed nonzero `00` words and planar
edge words to Unknown. This boundary deliberately refuses those words rather
than claiming exact native unpack equivalence. It **does not** check whether a
feature exists on the actual owning shape.

Solver IDs preserve `NEW_CONTACT_BIT` (bit 31) and require the low-31-bit index
to refer to an existing point in this same record. Exact row order, repeated
solver selections, unselected points and empty solver arrays are retained.
Repeated selections are structurally admissible here; this does not approve a
future parallel solver writer alias. A checked lookup returns the original
referenced point or None for an invalid solver row. There is no mutable access
after validation and no reference to a live dependency object.

## Wire and resource bounds

`ICPTS001`, version 1, uses a 56-byte header: common magic/version/dimension/f32
and expected build identity, followed by point and solver counts and a zero
reserved word. Point records are 100 bytes and solver records 48 bytes. Integers
and float bits are little endian. Identity is supplied separately by the caller;
the record does not authenticate an enclosing owner or discover a build.

Count limits, dimensional capacities, aggregate frame length, identity and exact
input length are checked before allocation. The shared `Budget` charges the
requested two Vec payloads before either allocation, and separately charges
encoded output. No charge is refunded after a failed field validation. Already
allocated caller-owned records are outside decoder allocation accounting.
Validation and encoding are O(points + solver), without recursion or validator
scratch. These are requested-payload bounds, not allocator overhead or total RSS
measurements. The normal dependency graph remains the single inert crate.

## Observable checks

The final frozen source002 passes **65 CPU tests in 11 nonempty groups**, with
zero failures or ignored cases. Thirteen are new: nine hostile/format tests and
four trusted native tests. All-target Clippy, wasm32 library compilation, normal
dependency graph, the three-crate runtime dependency guard, formatting,
conventions and diff checks pass. Cargo.lock is unchanged from the attachment
base. Raw commands, exits, logs, frozen source copies and pinned source references
are retained in [the evidence manifest](evidence/physics-inert-contact-points.json).

The two native stepping cases run 360 actual PhysicsWorld steps with a sliding
dynamic cuboid on a fixed cuboid, observing 720 planar and 1,440 spatial points.
Each ordered point/solver array is validated and round-tripped exactly. Both
dimensions observe nonzero normal warm starts, nonzero retained friction, the
new-contact bit, and an unchanged native recycle reference across a changed body
pose. That last observation witnesses retained reference state; it is not a new
instrumented count of internal narrow-phase branches. Five sleeping steps retain
identical rows, and waking with changed velocity changes the captured rows.

The 3D twist solver writes canonical `warmstart_tangent_world`; it does not write
the legacy `tangent_impulse` field on that path. Every stored field is preserved
regardless. The first test run failed to compile because the planar capture
treated a Vector1 as a scalar. The second run passed the codecs and planar case
but its 3D nonzero-friction assertion inspected that legacy field. Both failures
are retained. The third uses the native retained friction fields and passes.
These were fixture corrections, not changes to solver behavior. Source002 also
makes direct record validation enforce the encoder's aggregate frame limit.

Other tests cover every floating word with NaN/infinity, inactive words and
padding, signed zero/subnormal/extreme finite values, constructor/sentinel feature
semantics, packed IDs and invalid references, native planar capacities, aggregate
oversize, every truncation, trailing bytes, identity/version/reserved words,
cumulative exact allocation budget, a 4,096-point ordered table and adversarial
single-byte mutations. Actual public native fields can contain malformed feature
bits, NaN arms or an out-of-range solver ID; one-way capture observes those and
the inert boundary refuses them. No deserialize-to-live, unsafe construction,
panic-catching, save reconstruction or GPU execution is used.

## Still required for exact restoration

Independent source/final producer evidence acceptance is retained in the [main199 integration packet](physics-contact-main199.md). This component does not
bind points to collider geometry, subshapes, normals, material coefficients or
body handles. Manifold/pair metadata, dispatcher/recycle workspaces, solver
clusters and previous cluster state, graph colors/body indices, contact/island
ownership, complete body/collider payloads, removal journals and native owner
composition remain required. Physics-save admission stays refused until those
families are checked together and exact continuation is demonstrated. No full
physics-save, shipping, device or performance gate is claimed.
