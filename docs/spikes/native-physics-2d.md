# Native planar physics

The native runtime now retains Rapier2D bodies and solid tilemap collision through
cooking, typed numeric writes, structural commands and fixed ticks. This is a
bounded Phase 1 slice, not a device, performance, sample-release, rollback-save,
complete character/joint or backend-adoption gate. The local evidence is recorded
in [native-physics-2d-local.json](evidence/native-physics-2d-local.json).

## Data and ownership

Rapier2D is pinned to 0.36.0, matching the existing 3D boundary; Parry2D and all
transitive resolutions are locked. `enhanced-determinism` is enabled, with
`parallel` only on non-Wasm targets. WebAssembly stays serial. The separate fast
export variant and project variant selector remain open. Upstream definitions:
[Rapier source](https://github.com/dimforge/rapier/tree/v0.36.0) and
[crate documentation](https://docs.rs/rapier2d/0.36.0/rapier2d/).

Shared `RigidBody2D`, `Collider2D` and `AngularVelocity2D` authoring contracts use
the same typed validation as cooking and native structural preflight. Generated
JSON schemas and TypeScript declarations are included. A collider supports a box,
circle or Y capsule, material, collision masks and sensor behavior. Its body is
optional (fixed collider); dynamic and kinematic bodies retain linear/angular
motion and solver-owned sleep. These entities must be roots with unit scale and
planar rotation/velocity. Render Z is preserved and does not enter 2D collision.
A single entity cannot combine 2D, 3D or generated solid-map collision; independent
2D and 3D bodies can run in the same world, each integrated once.

Cooked binary v7 carries a bounded, canonical table keyed by existing stable IDs.
Unknown versions/tags, invalid booleans/counts, duplicate or unknown IDs, truncated
records and invalid numeric domains fail explicitly. Versions 1–6 retain their
wire branches and scenes without explicit 2D declarations retain their earlier
encoding version. A compatibility qualification: solid v6 maps that previously
loaded but always refused simulation now receive the root/unit-scale/fixed-motion
checks at cook/load. Unsupported parented, scaled or moving solid maps are not
silently accepted by the new solver.

One native world owns one retained solver per dimension. Rapier handles stay
private; external IDs and trigger pair ordering remain stable. Geometry is
prepared at cook/structural boundaries, never rebuilt from authoring data on a
tick. Solid maps consume the retained integer-cell union, including holes and
overlapping-layer deduplication, with friction 0.5, restitution 0, all masks and
no sensor. Atlas/frame pivots do not affect collision. Exact integer union does
not imply arbitrary subpixel f32 contact precision, especially near the
±1,000,000 position limit. Limits are 65,536 bindings, 262,144 primitive shapes,
65,536 simultaneous sensor pairs and at most 131,072 entered/exited records.
These bound stored data, not the time for an adversarial overlapping workload.

## Mutation and failure semantics

`SetPhysics2D` and `SetAngularVelocity2D` join the existing typed structural
buffer. Complete final-state/capacity preflight precedes publication. Angular
reset still requires a body. Invalid batches preserve components and the queued
commands. Numeric Transform/Velocity loans use the existing checked native
boundary and bounded undo: invalid planar or final structural state rolls back
before script success, and retained guest views stay detached. There is no
per-tick JSON, authoring validation, project clone or solver reconstruction.

Every native step validates all inputs and candidate hierarchy before solver
motion writes. Once either solver advances, numerical/capacity failure or unwind
terminalizes the world; ECS values and the public clock are published only after
all outputs and candidate globals validate. This is terminal failure, not solver
rollback. Existing owning sessions also discard accepted failed ticks. Trusted
native immediate writes keep their documented contract; callers needing a
recoverable transaction must use the checked boundary.

Unchanged numeric echoes and no-op structural reinstalls preserve the exact last
solver angle rather than recomputing it from the quaternion, so sleepers remain
asleep. Pre-step/dirty raycasts inspect retained shapes without a collision-only
step that could consume new-body flags. Stepped queries use the solver BVH;
exact-distance ties use stable ID order. Invalid queries do not advance time.
Physics save/restore is explicitly refused once 2D physics has been used, even
if every binding is later removed; solver/contact state is not serialized.

## Observable verification

The final local union passes 585 release workspace tests/doctests, all 66 ignored
GPU tests, full all-target release Clippy with warnings denied, schema/SDK
regeneration checks and SDK TypeScript checking. The nine solver tests and eleven
new native-world tests cover real collision/sleep, tile holes, retained storage,
queries/filtering/ties, sorted sensors, replacement/despawn, mixed dimensions,
no double integration, invalid-batch atomicity and terminal solver failure.
Cook tests exercise the real authoring boundary; a QuickJS regression exercises
five invalid 2D numeric/structural transactions and detached retained views.
Refreshed-checksum malformed v7 records exercise decoder branches beyond checksum
rejection. A 64-angle no-op regression failed before the angle-preservation fix.
An older legacy-refusal test fixture was corrected to keep its solid map fixed;
its original refusal/unchanged-state assertions remain. Legacy Play was not
extended. A pinned f32 angular integration uses an explicit 1e-6-radian tolerance
against the ideal value; full cross-runtime streams below compare exact bytes.

The shared scenario runs 640 dyadic ticks with 35 IDs, box/circle/capsule bodies,
a retained floor with holes, kinematics and a sensor. Numeric wake and structural
replacement commands exercise the native boundary. On local macOS ARM, three
fresh trials at both 1 and 4 workers plus three real Chromium/Wasm processes
produce 18 matching streams across the separate existing 3D and new 2D scenarios.
The 2D stream includes all public solver fields plus native Transform/Velocity,
641 frames, 32 sleepers, 32 wake transitions and 40 trigger events. The unchanged
3D bytes also match retained PR #57 artifacts from all three desktop hosts.
That historical comparison does not establish new 2D cross-host parity.

Resolved feature checks cover 49 root/solver/target rows across native desktop,
iOS, Android and Wasm. Native parallel and Wasm serial selection pass, as do the
runtime normal-dependency guard and actual iOS-simulator/Wasm compilation. The
browser actually steps both worlds; mobile checks remain compile/graph evidence.
The existing physics workflow retains Jolt/candidate timing and serial/joint
controls, adds both native-world streams, strict cross-host comparison and one
fresh planar shipping proof per desktop. New hosted 2D results are pending.

## Fresh shipping executable

`tools/probes/native-physics-2d.py` builds the real `native_physics_2d` example in
fresh storage, verifies opt-level 3, fat LTO on the executable, one codegen unit,
abort-on-panic, debug/split settings and a normal graph without authoring, VM or
renderer crates. A trusted typed fixture producer encodes the checked v7 scene;
authoring/cook APIs are tested separately. Its temporary copy and all fresh build
storage are removed before three copied-player runs and four refusal controls.
The package retains only the executable and cooked scene; no texture cache is
needed. This is explicitly a nonrendering conformance player, not a complete game
or a bypass of existing visual-host refusals. Simulation failure produces no
stream; output I/O itself is not atomic and can leave a partial file.

The local macOS ARM shipping build took 1m55s in the recorded fresh Cargo log
(not a runtime benchmark). The stripped executable is 3,994,224 bytes, SHA-256
`c6104759251293f9e172a9af180b382a652af22d7f45ca069606a11d26677ae1`,
with matching arm64 dSYM UUID `dedba3ad-13b3-3d79-8883-254aa2ce5d83`.
All seven processes pass and the entire 3,791,547-byte 2D stream matches ordinary
release and browser execution. Linux ELF and Windows PDB packaging are wired to
the existing checked tool but await this slice's hosted execution. This binary
size is specific to the lean conformance example, not a complete-player budget.

2D character/joint/navigation support, a full script query/event SDK, physics
save/rollback, project-selectable fast builds and reference-phone performance
remain open. No renderer appearance, backend adoption or phase gate is approved.
