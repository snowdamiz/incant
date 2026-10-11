# Native planar physics

This is an in-progress Phase 1 migration. The first internal checkpoint introduces
`incant_physics_2d_runtime`; the existing cooked/native solid-tilemap refusal remains
until the next integration checkpoint. No device, performance, sample-release,
rollback-save, complete character, joint or backend-adoption gate is claimed.

## Solver checkpoint

Rapier2D is pinned to 0.36.0, matching the existing 3D boundary. The checked lock
pins Parry2D and all transitive dependencies. This first path enables
`enhanced-determinism`, plus `parallel` on non-Wasm targets. WebAssembly stays
serial. It does not implement the separate fast export variant or a project
variant selector. Upstream feature definitions are in the pinned
[Rapier source](https://github.com/dimforge/rapier/tree/v0.36.0) and
[crate documentation](https://docs.rs/rapier2d/0.36.0/rapier2d/).

One native owner retains one solver. Stable 128-bit IDs identify declarations,
queries and sorted trigger pairs; Rapier handles remain private. Primitive box,
circle and Y-capsule declarations reuse the dimension-independent rigid-body
configuration. Typed state includes planar translation/angle, linear/angular
velocity and observed sleeping. Sleep is solver-owned; an unchanged echoed state
does not wake a body. Raycasts before a real step visit retained collider shapes
at current poses, avoiding a collision-only pipeline that would consume new-body
flags before island registration. Stepped queries use the solver BVH. Exact
distance ties use stable ID order.

Tile collision consumes the prepared solid-cell union, including its holes and
overlapping-layer deduplication. Exact integer-cell coverage does not imply
arbitrary subpixel f32 contact precision, especially near the ±1,000,000-meter
position bound. The geometry is built at a structural boundary,
never regenerated during ticks. It is fixed, with the explicit default
Collider2D friction 0.5, restitution 0, all collision masks and no sensor; visual
atlas/frame pivots and render Z are not collision inputs. Capacity is limited to
65,536 entity bindings, 262,144 primitive shapes and 65,536 simultaneous sensor
pairs (at most 131,072 entered/exited records). These bound stored data, not the
wall-clock cost of an adversarial all-overlapping solver workload.

Prepared structural batches validate duplicate IDs and final collection capacity
before any solver mutation. Borrowed motion slices make a complete validation pass before
writing values, with no whole-world clone or JSON/authoring validation. Invalid
packets leave solver state available for retry. Invalid dt also preserves state.
A numerical/range/event-capacity error after stepping permanently fails the
solver (also on a caught Rust unwind after step begins); queries and mutations refuse it, and state/events are hidden. This is not
solver rollback. Native/session integration must preserve the same distinction.

Local boundary checks: nine actual release solver tests and all-target release
Clippy with warnings denied. They exercise dynamic collisions and sleep, exact
tile holes, pre-step query followed by real falling, sorted sensor enter/exit,
filter/exclusion/ties on both query paths, atomic invalid motion/structural
batches, capsule linear/angular motion, malformed input and terminal numerical
failure. A single 1/64-second angular integration at 2 rad/s yields
0.031249364838004112 rad in pinned f32 Rapier, compared with the ideal 0.03125;
the test uses an explicit 1e-6-radian tolerance while checking dyadic linear
motion and angular velocity exactly. This is correctness evidence, not a timing
or cross-platform parity result. Browser, full native integration and hosted
parity evidence remain pending at this checkpoint.
