# Checked inert regular rigid-body state

The std-only inert format now preserves all fixed pinned regular RigidBody
fields, with its ordered collider list supplied by the existing attachment
witness. This moves body dynamics, activation and integration state into the
checked-format path. It does **not** construct Rapier objects, introduce a normal
solver/runtime dependency, enable physics saves, or prove whole-world continuation.
`PhysicsUnsupported` remains unchanged.

## Payload and ownership

One fixed row corresponds to each live body arena payload row, including fixed,
disabled and collider-less bodies. A checked handle accessor uses the exact
borrowed attachment/body arena witness, so removed, stale, free and sentinel
handles cannot retrieve rows. Payload order is not handle order and is not
sorted. Equal unrelated arena shapes are not enclosing-owner authentication.

The record retains:

- Current and next poses; ordinary and CCD linear/angular velocities.
- Effective force/torque, user force/torque, gravity scale and spatial gyroscopic
  enablement; linear and angular damping.
- Local center/inverse mass/principal inverse inertia/principal frame, effective
  world center/inverse mass/symmetric inertia entries, axis locks, exact optional
  additional mass or full additional mass properties, and cached maximum extent.
- Sleep thresholds, elapsed/required sleep time, sleeping flag and previous pose
  used by the sleep drift check.
- CCD thickness, automatically active flag, full CCD enablement, soft prediction
  distance and fast-rotation permission.
- All four native island/set scalar indices, exact change bits, regular body type,
  dominance, enabled state, both additional iteration counts and all 128 user bits.
- Explicit soft-owner handle/cluster/motion-margin fields; unsupported non-rigid
  ownership is refused rather than silently omitted.

Admitted body types are Dynamic, Fixed, KinematicPositionBased and
KinematicVelocityBased. SoftFrame and unknown wire types return UnsupportedBody.
The native invalid soft handle has both words MAX, the cluster is MAX, and the
motion margin must be numeric zero (either signed zero retained).

The same completed-journal witness as the manifold path is required: nonzero
modified-body, modified-collider or removed-collider counts return PendingMutation
before decoder allocation. Actual native tests derive these counts from the real
sets. A future enclosing decoder must derive them from checked journal payloads;
this independent caller-supplied witness is not phase authentication. Known change
bits are preserved rather than cleared; this slice does not prove their reciprocal
relationship with the enclosing mutation queues.

## Exact numeric domain

Every active float is stored as its f32 bits. No normalization, quaternion sign
folding, clamping or cache recomputation occurs. Negative zero, subnormals and
extreme finite values survive where the field domain permits them. Unused planar
lanes are positive zero: Z for linear vectors, the last two lanes of scalar
angular quantities, all absent principal-frame lanes, the last five inertia
lanes, and the nonexistent gyroscopic flag is false. Spatial symmetric inertia
order is m11,m12,m13,m22,m23,m33; signed off-diagonals are retained.

The supported domain intentionally requires finite translations, velocities,
forces, thresholds and cached scalars, and the existing strict f64 squared-unit
rotation tolerance below 2e-4. Native public setters can accept values outside
this supported domain; unchecked nonfinite configurations and invalid rotations
are explicitly unsupported, not claimed to be impossible native states.

Mass/inertia inverse values, additional mass, effective inverse-mass components,
inertia diagonal entries, damping, times, extent, CCD thickness and prediction
must be nonnegative. Numeric negative zero is permitted. Signed finite gravity
scales and negative sleep thresholds are permitted: negative thresholds are the
native never-sleep convention. The common collider-less CCD thickness sentinel
is **f32::MAX**, not infinity, and is preserved exactly. **Zero inverse mass or
inertia represents infinite mass/inertia** and is accepted. The format does not
replace this convention with positive-infinity floats. Explicit positive-infinity
additional-mass or threshold configurations remain outside the supported domain.

Known axis-lock bits (all six in either dimension) and change bits (all eight)
are preserved. Dominance is the exact native i8 range. Index scalars permit
0..262143 or MAX, matching the aggregate inert capacity; reciprocal island/set
membership is not established here. Additional iteration counts are exact u64
representations of native usize values. They cause no allocation or loop here;
**they do not authorize unbounded solver work**. A future live-owner admission
must impose checked platform conversion and its solver work policy before use.

Scalar checks do not establish positive-semidefinite world inertia, geometry-
derived mass/extent equality, lock/cache consistency, current/next-pose phase
relations, timestep safety or arithmetic overflow safety. Those cross-field and
cross-owner checks remain required before constructing a live body. The full
record intentionally retains independently observed cached values so later
validation cannot silently substitute recomputed state.

## Canonical bounded wire

`ICBODY01`, version 1, uses a 52-byte identity/count/reserved header and fixed
464-byte rows. All words are little endian. Additional mass is an inline tagged
11-word payload: absent data is all zero; scalar mass has zero inactive words;
full mass properties retain every lane. Boolean tags are exactly 0 or 1, and
unknown types/tags/reserved bits refuse. Iteration u64s use low/high words and
user u128 uses four low-to-high words, without float or usize narrowing.

Independent identity, completed-journal witness, count equality, checked frame
size and the 32 MiB byte cap precede allocation. Count is also capped at 262,144;
the fixed row byte cap is tighter for this section. Decode charges the shared
Budget for exactly the requested Vec payload before reserving. Row validation
allocates no scratch. Encoding charges its output separately. Caller-preexisting
records/witnesses belong to the enclosing aggregate budget. These are requested
payload bounds, not allocator overhead or RSS measurements. Work is bounded by
fixed-size rows; there is no recursion, unsafe code, panic-catching or live
native deserialization.

## Observable native and hostile checks

Frozen source001 passes **101 tests in 16 nonempty groups**, with 18 summary rows
including empty library/doc groups, zero failed or ignored. All-target Clippy,
wasm32 library compilation, normal/runtime dependency guards, formatting,
conventions and diff checks pass. Cargo.lock and dependency manifests are
unchanged from parent77910. The normal graph remains the single inert crate.

Actual pinned PhysicsWorld tests in both dimensions perform forces, torque,
linear/angular impulses, damping, gravity changes, optional full/scalar mass
changes, axis locks, kinematic targets, velocity kinematics, sleep/wake, disable/
enable, type and dominance changes, and removal/slot reuse. Every checked native
row round-trips exactly; public position/velocity/mass/force accessors independently
match captured fields. Maximum u128 user data and native negative-zero gravity/
damping are observed without JSON integer truncation. The trusted one-way test
serializer represents u128 as an exact decimal string; it has no deserializer
and is not a normal dependency.

Separate 91-step native cases observe automatically active CCD while full CCD
is disabled, nonzero CCD velocity, changing sleep-history poses and natural
sleep/timer evolution. These are native state observations, **not** restored-world
continuation. No special solver fixture replaces the actual native step.

Hostile tests cover all body types, option/boolean/unknown tags, unsupported soft
owners, nonfinite/negative-domain values, unit rotations, planar padding, stale
handles, reversed dense-row mapping, empty and 4096-body tables, identity/count/
journal preflight, exact cumulative budgets, every truncation, trailing bytes and
single-byte mutations. Every admitted mutation must re-encode byte-for-byte.

Initial test-source compilation errors and the observer's first incorrect tuple
assumption for native Index (actual named index/generation fields) are retained
with their failed logs. They were corrected before source001 freeze. The final
commands, pinned references, source copies and raw output are losslessly retained
in [the evidence manifest](evidence/physics-inert-body-state.json).

Independent producer source/final review is accepted, retained in the [main206 integration packet](evidence/physics-body-main206/producer-final-review.json). Complete collider material/filter/
pose/change payloads, shape and feature ownership, reciprocal island/solver/CCD
manager state, mutation/intersection/soft/joint policy, scratch reconstruction,
enclosing owner identity and safe live construction still need composition and
exact continuation proof. No shipping, phone, GPU or performance gate is claimed.
