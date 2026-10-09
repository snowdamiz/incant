# ADR 0003: Rapier behind a Rust-owned physics boundary

Date: 2026-10-08. Updated: 2026-10-09. Status: implemented fallback; Phase 1 remains in progress.

## Decision

Use pinned Rapier 0.36 with `enhanced-determinism` for the first engine physics
integration. This takes PLAN.md's explicit Rapier fallback after measuring the
binding limitations below. It is an implementation decision, not director phase
gate approval. Stable entity IDs cross scripts and documents; solver handles,
contacts and sleeping state belong to disposable play sessions.

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
contacts. Stable insertion ordering, fixed timesteps, serial stepping and the
pinned dependency graph support reproducibility. The probe proves only one
position workload, not universal determinism or a rollback release gate.

See [runtime integration](../spikes/physics-runtime.md) for supported shapes,
queries, script semantics and current limitations. Character controllers, mesh
colliders, hierarchy/scale support, runtime rollback snapshots and physical-device
performance gates remain open. Revisit the backend if representative game/mobile
measurements show a material issue; keep the stable-ID boundary.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
