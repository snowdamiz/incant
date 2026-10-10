# ADR 0003: Rapier behind a Rust-owned physics boundary

Date: 2026-10-08. Updated: 2026-10-10. Status: per-project modes adopted by
the director in PLAN.md revision 4; feature selection and the backend rule fixed
by the revision 5 decision record. Only the original deterministic variant is
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
instructions on reference phones. Both native variants enable `parallel`; browser builds keep the serial solver until a supported worker pool exists. The fast
variant adds `simd8` where the target benefits and keeps hardware math. The
deterministic variant is `enhanced-determinism` plus `parallel`. Cross-platform deterministic mode remains
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

The pinned rapier3d 0.36.0 source fixes the feature contract. `enhanced-determinism`
enables `simba/libm_force` (software math) and `parry3d/enhanced-determinism`
(insertion-ordered maps). `lib.rs` refuses to compile it together with `simd8`,
so the deterministic variant runs 4-lane SIMD. `parallel` is independent:
Rapier's `parallel_path_parity` test pins one golden hash that builds with and
without `parallel` must both match under `enhanced-determinism`. That is the
upstream contract; Incant's own checksum test must confirm it on its workloads.
The current build enables `enhanced-determinism` without `parallel`, so it pays
for software math and steps serially.

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
on reference phones and desktops, then apply decision 19's rule: keep Rapier
unless Jolt is at least 1.5 times faster in p95 step time on both reference phones
and a maintained C shim builds for all six targets; if both hold, switch behind
this boundary before Phase 6. A binding failure or a microbenchmark alone does
not establish which solver is faster.

First migration step: enable `parallel` in the existing deterministic build and
require exact equality with the current serial regression evidence before
relying on it. Then add the separately built fast variant.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
