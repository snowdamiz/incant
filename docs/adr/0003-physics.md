# ADR 0003: Rapier behind a Rust-owned physics boundary

Date: 2026-10-08. Updated: 2026-10-10. Status: per-project modes adopted by
the director in PLAN.md revision 4; only the original deterministic variant is
implemented. Phase 1 remains in progress.

## Decision

Use pinned Rapier 0.36 behind the stable Rust boundary with a per-project physics
mode. The default fast variant uses supported vector instructions and parallel
stepping; the rollback/exact-replay variant uses `enhanced-determinism`. Export
and play-test processes select a separately built compatible runtime. Cargo
feature unification means this cannot be a runtime toggle in one linked Rapier
build. The current unconditional `enhanced-determinism` dependency is migration
work, not evidence that both variants already exist.

Select features against the pinned version and target capabilities: 0.36 exposes
`parallel` and `simd8`; do not assume older `simd-stable` flags or force x86-only
instructions on reference phones. Cross-platform deterministic mode remains
conditional on measured checksums for the complete simulation, including scripts.
Stable entity IDs cross scripts and documents; solver handles, contacts and
sleeping state belong to disposable native runtime worlds.

## Evidence

The [candidate probe](../spikes/physics-candidates.md) passes native Jolt and
Rapier on macOS, Windows and Linux, including matching per-backend position
streams across those hosts. The first Jolt binding (`oxijolt` 1.0.1) refuses the
iOS simulator and wasm32 targets. Rapier builds for both. A second published
binding, `joltc-sys` 0.3.1, reaches its iOS C++ build but fails at bindgen's invalid
`arm64-apple-ios-sim` Clang target. Its wasm32 attempt fails in CMake's compiler
link check with the installed Apple Clang; no WASM C++ toolchain was configured.
The third published binding (`jolt-sys` 0.1.5) hardcodes Visual Studio 2019,
Windows runtime libraries and SSE flags in its build script (source inspection,
not a claimed cross-platform build). These findings concern current bindings and
toolchains, not a claim that Jolt's C++ core cannot run on these platforms.

Maintaining a new portable C++ shim is possible. The plan already permits a pure
Rust fallback, and the measured Rapier workload fits the available local budget.
Adopt it explicitly instead of making native-only physics a runtime dependency.
No SDK accounts, signing identities or development credentials are needed.

## Consequences and remaining work

One independently simulated world per scene prevents accidental cross-scene
contacts. The original implementation uses stable insertion ordering, fixed
timesteps and serial stepping. Preserve its regression evidence for that build;
do not require fast-mode cross-platform bit equality or imply those measurements
cover parallel stepping. The probe proves only one position workload, not
universal determinism or a rollback release gate.

See [runtime integration](../spikes/physics-runtime.md) for supported shapes,
queries, script semantics and current limitations. Read-only character movement
queries are implemented in the [character increment](../spikes/character-movement.md),
including actual platform execution and rendered review. Mesh colliders,
hierarchy/scale support, runtime rollback snapshots and physical-device
performance gates remain open. Before Phase 6, compare the fast and deterministic
variants with Jolt through a maintained portable shim on the Section 6.14 workload:
500 dynamic bodies, destructible props, four characters and enemy crowds. Measure
on reference phones and desktops, then resolve open decision 19; a binding failure
or a microbenchmark alone does not establish which solver is faster.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
