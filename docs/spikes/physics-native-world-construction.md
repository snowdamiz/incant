# Private checked primitive-world assembly

This slice composes all previously checked owners into one local PhysicsWorld candidate. It accepts no public bytes or caller authenticity flag. The only CaptureContext issuer is the in-crate test harness, which owns the native world, its default dispatcher, the actual step, and all preceding mutations. Public numeric admission is still not a restore capability. Runtime scene/ECS/stable-ID/event-queue/poison composition and PhysicsUnsupported remain closed.

The private `ICNATIV1` envelope carries the unchanged `ICWORLD1` byte frame plus the exact impulse-joint assembly epoch. This extra epoch is required even for empty joint sets: map_attached_joints_mut increments it before looking for attached joints. Initial zero-only refusal was preserved in focused002. Native wrapping behavior is retained; empty-but-used joint arenas, Coarena rows, graph storage, journals, multibody epoch and soft-body arena/attachment history are independently refused. No empty length or serde equality grants admission.

One plan covers body/collider/shape, island, broad, narrow, CCD, all native empty-family slots/graphs, and pipeline scratch allocations. The actual allocator caught 7,376 uncharged bytes in initial 2D construction: 4,096 bytes for the fixed SIMD ContactRef bucket box and 3,280 bytes for the two default empty graphs' ten-node/ten-edge capacities. Both are now explicitly charged. Stable Box/Arc retain ordinary process-OOM semantics; Vec reservations are fallible and exact. Budget refusal precedes all candidate allocations; late internal-permit refusal drops every prior candidate owner.

Ordinary new() APIs and default behavior are unchanged. The new private child literals retain all native defaults but route nonempty allocations through the same non-Clone permit. The pool Arc is an explicit trusted execution policy shared from the capture owner; no pool is decoded or manufactured, and no reconstructed physical owner is a trusted clone.

Fresh scratch is grounded in native sites: CollisionPipeline::detect_collisions clears broad_phase_events; PhysicsPipeline::solve clears sleep observations and parallel active handles. A fresh pipeline sets joint_selection_primed=false, and solve invalidates joint selection memo before select_active_interactions. A negative control proves an equal memo otherwise leaves arbitrary prior output unchanged. Staged init clears/rebuilds groups, constraints and chunk workspaces; staged_joints_valid=false prevents cached joint builders being consumed. Contact generator paths reset accumulators and warm-start from reconstructed manifold fields; buffer capacity is not accepted as semantic history. Staged post_solve_ccd_active is explicitly persistent and is reconstructed as None/false/true.

Native continuation includes an intentionally refused intermediate wake-up boundary. The original and reconstructed worlds can both have modified-body journals after a positive step. Capture admission must remain PendingMutation; exact native field/event/query/cache/observer comparison continues without erasing the journal, and full encoded frame equality is required whenever both worlds admit. Expected refused tick lists are pinned by dimension rather than allowing arbitrary unexpected skips.

Focused attempts remain immutable. 001 and003 were explicit std imports in the test capture adapter;005 used private module paths for the allocation diagnostic;009 had a test Result alias collision. 004 and006 were real incomplete allocation plans. 007 also exposed a test sequence that advanced only the source before adding a follower (fixed without changing native code). 008 locates actual pending modified-body journals at wake-up.

## Exact binding and admission limits

The construction entry receives one private `AdmittedAssembly`. Its numeric
witness is derived from the envelope's own inner bytes inside the bounded,
higher-ranked decoding callback; its epoch is taken from that same envelope.
A separate frame and epoch cannot be supplied to the constructor. This is data
binding, not authentication: the private format has no external provenance or
public compatibility guarantee. The only native context issuer remains the
exclusive test harness. Inert whole-frame and numeric admission still reject
unsupported geometry, custom hooks/dispatch, journals and incoherent derived
values before the candidate is constructed.

The empty-family gates inspect arena generation, native four-slot free chain,
Coarena rows, graph nodes/edges, pending journals and multibody topology epoch.
The impulse assembly epoch is preserved even when nonzero or wrapping. Native
body changes can advance it without creating a joint. Removed joint/soft-body
history still refuses even when length and the impulse epoch are zero.

An explicit pool Arc is process execution policy, not serialized world state;
sharing it does not clone any reconstructed physical owner. The pool is retained
under the same native configuration and thread count for the paired evidence.
Counters and capacity choices are not exposed as exact persistent timing state.
The next actual CCD substep count is checked explicitly.

## Native scratch audit

All paths below are under the pinned `third_party/rapier{2d,3d}/src` trees.

- `pipeline/collision_pipeline.rs:75` clears collision-only broad events before
  update. Its constructor holds no other persistent owner.
- `pipeline/physics_pipeline/solve.rs:70,240,265` clears broad events, sleep
  observations and parallel active handles before producing new values.
- `pipeline/physics_pipeline/solve.rs:351` invalidates the impulse-joint selection
  memo on the first solve of a new pipeline. The equal-memo negative control
  proves why simply assuming a fresh output is already initialized is wrong.
- `dynamics/solver/staged_island_solver/init.rs:74,201,310` rebuilds groups,
  interaction lists and chunk/color partitions. Its later reset/generation paths
  initialize constraints and load warm starts from the retained manifold data.
- `dynamics/solver/staged_island_solver/init.rs:569` resets/reuses the 3D twist
  buffers; the Coulomb path has its own corresponding reset. These buffers are
  not a replacement for retained manifold impulse history.
- `dynamics/solver/staged_island_solver/joints.rs:49` only reuses cached joint
  assembly when `staged_joints_valid` is true. A new checked pipeline sets false,
  so the rebuild path initializes the empty-joint scratch instead.
- `post_solve_ccd_active` is retained as exact `None`, `Some(false)` or
  `Some(true)`. The omitted-verdict negative control changes the actual native
  cache and substep counter. It makes no trajectory-divergence claim.

Deferred optimization and quarantine refuse capture without consuming or
clearing them. Empty scratch counts do not authenticate a completed step.
The supported capture domain still requires empty native mutation journals;
the paired continuation explicitly compares the same refused wake-up boundary.

## Retained execution

Source001 and its source002 lint-only correction were independently reviewed.
Each deterministic and fast8 broad run passed **138 private native tests** and
**227 snapshot tests** (the inherited controls overlap earlier accepted slices).
All 24 commands in the final broad chain exited zero: both selected solver
feature graphs, scoped native library and snapshot all-target Clippy, wasm
compilation, lean/default runtime dependency guards, 31 tooling controls,
formatting, conventions and whitespace checks. The unchanged inherited vendor
checkpoint re-export ordering is excluded from the scoped owned-file rustfmt
check; the root workspace formatting check passed.

The last source003 delta changes only five test assertions per dimension, plus
current pin metadata. Simple CCD/cache/epoch continuations must now explicitly
admit every recapture. Only the churn fixture permits a refusal, pinned at tick 7
on both worlds. Both current focused runs pass **21 tests** (10 planar, 11 spatial)
with no ignored cases. Production constructor/admission bytes are unchanged from
the reviewed and broadly qualified source002.

Whole-world controls perform 198 paired steps in 2D and 262 in 3D per mode:
156 churn/event/query steps, 20 retained-CCD steps, 12 cache-shape steps, 9 verdict
steps and the epoch-wrap step, plus 64 3D warm multipoint friction steps. These
counts exclude the omitted-verdict negative control and overlapping inherited
owner tests. Collision/force records include identities, generations, flags and
all scalar/vector bits; parallel callback ordering is intentionally compared as
a sorted multiset. All non-refused recaptures compare exact full inert bytes.

Thirty-two constructed candidates per focused mode measure actual allocation
requests against the cumulative permit. The conservative map charge can exceed
the allocator request by eight bytes; calls match exactly. No allocator metadata,
RSS, hard process-memory ceiling, phone or performance conclusion follows.

Ten earlier failed command receipts are preserved: nine focused development
attempts and the first strict library Clippy attempt. They include the real epoch
assumption, missing allocation charges and native wake-journal discovery as well
as fixture/import errors. The deterministic source002 test repeat had already
started when the instruction to avoid unnecessary repeated passing suites
arrived; it is recorded separately, without multiplying coverage.

The final lossless packet and executable copies retain the source generations,
actual argv/exit/log identities and accepted review receipts. Final independent
packet review remains pending. Public `PhysicsUnsupported`, outer SceneWorld
maps/ECS/stable IDs/event queues, character/query phase admission and the actual
runtime capture capability are still separate unfinished boundaries.
