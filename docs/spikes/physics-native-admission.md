# Checked native physics admission

This is the first numerical preflight of the joined inert world frame in the
pinned Rapier kernel. It does not construct or install a live physics world.
`PhysicsUnsupported` remains in force. The preceding world-boundary proof used
trusted native clones with fresh scratch, not a decoded restore.

The new `incant-checked-restore` Rapier feature is default off. Its optional
normal edge points to the std-only `incant_physics_snapshot` crate with defaults
disabled; that crate still has no normal dependencies. Its `native-admission`
feature only selects the Rapier development edges for these tests. All external
package versions are unchanged. No runtime or player enables the native feature.

## First numerical stage

A caller supplies the independently expected dimension/build identity and a
shared monotonically consumed logical work budget. The existing joined frame
already bounds decoded bytes, allocation, counts and graph traversals. Native
admission adds checked native integer conversion, base-plus-extra solver counts,
CCD repetition and row/iteration multiplication. Body extras use the pinned
`usize -> u32 -> min(65535)` partition-key order without changing saved values.
This is a conservative logical visit model, not a CPU-time, collision-search or
complete solver-work bound. Failed later checks do not refund charges.

For single box, ball and Y-capsule shapes, the stage evaluates native mass
formulas in the exact saved attachment order, including enabled-state filtering,
zero-density/infinite-mass sentinels, explicit mass properties and additional
mass with the native unit-density fallback. It compares local mass, effective
world mass/inertia, world center and maximum extent bit-for-bit. Attached
collider poses must equal the native body/local-pose product. Quaternion signs
and negative zero are not normalized or folded.

The native full-step spring formulas and force construction/integration must
stay finite. Force preparation uses the actual pinned native helpers: the next
step overwrites effective force from user force plus gravity and effective
axis-locked mass. It does not add gravity again to the saved effective force.
Retained fixed-target CCD AABBs must equal the shape bound at the saved collider
pose loosened by the **historical cached** prediction distance. No current
parameter substitution, contact skin or half-distance rule is used there.

The calculations use stack-native primitives and scalar mass structures. They
do not allocate a live body/collider set, native graph, query tree or world, and
never deserialize into a dependency's unsafe live structures.

## Observable controls and scope

The focused tests capture real completed 2D/3D worlds: box/ball/capsule steps,
additional mass, zero density, explicit mass, locked rotation/translation,
kinematic-to-dynamic changes, user forces and signed-zero gravity settings.
Serialization before/after confirms the source world remains unchanged by
admission. Other controls decode structurally admitted but numerically forged
world-center/local-mass/extent/cache bounds, overflowed iteration arithmetic and
finite scalar forces whose next native formula overflows. Exact budget boundary,
cumulative consumption, unrelated identity and compound refusal are explicit.

Small extra-iteration values are stepped in actual native worlds. Very large
values are checked as inert arithmetic inputs only, never executed. An initial
attempt to inspect a freshly modified native body correctly failed the existing
pending-journal prerequisite; the failure logs remain retained.

## Still required before exact live restoration

This stage deliberately refuses compounds and external physics hooks. It does
not authenticate the capture phase or enclosing owner. Historical pending broad
bounds, shape-feature correspondence, every contact Jacobian and gyroscopic
intermediate, CCD-generated shorter solve intervals, future generation exhaustion,
full allocation/global work and the native event/stable-ID owner are still
separate admission requirements. A valid `NumericFrame` is **not** a restore
capability. The next constructor must remain private and default off, preserve
saved fields exactly, and run only after those requirements are satisfied; no
raw setter or serde-to-live escape path is authorized.

## Qualification

Independent source review accepted frozen source001 (102 inputs). The completed
CPU suites each pass 214 cases in 23 nonempty groups (25 summary rows including
empty library/doctest groups), once with deterministic physics and once with
fast SIMD8. The 12 focused cases overlap these totals. Both snapshot all-target
Clippy and explicit selection of both vendored Rapier libraries plus the trusted
native test pass with warnings denied. The std-only snapshot wasm build, normal
runtime graph, four independent locked probe resolutions, 31 Python dependency
and feature guards, workspace/vendor formatting and conventions pass.

The retained supplemental lint attempts include external-package feature
selection refusal, a pinned Cargo feature-resolver panic for lib-only dev-edge
selection, and upstream all-target tests requiring their omitted bincode dev
dependency. The final precise library plus trusted-test selection passed without
source, dependency or lock changes. Initial focused fixture failures remain as
evidence of pending-journal refusal, not successful continuation.

[Manifest](evidence/physics-native-admission.json) and lossless archive retain
source copies, command receipts and raw logs; two actual trusted-test binaries
are copied with executable mode outside the build target. All 968 locked package
identities are unchanged; the root lock adds only the two default-off optional
local dependency edges. Final independent evidence review is accepted
([retained receipt](evidence/physics-numerics-main227/producer-final-review.json)).
Current-main composition is qualified separately
[in the integration report](physics-numerics-main227.md). These are
CPU/native mathematical checks, not a device or performance gate.
