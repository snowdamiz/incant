# Native Rapier parallel stepping

Phase 1 now enables Rapier 0.36.0's `parallel` feature for non-WASM targets while
retaining `enhanced-determinism` everywhere. WASM stays serial. The product
change is confined to the target-specific dependency declaration and lockfile;
PhysicsRuntime's public API and synchronization behavior are unchanged. This
does not extend the legacy document tick path or migrate physics into the new
native runtime world. Both that migration and the project's future fast versus
deterministic export variants remain separate work.

The pinned Rapier source was inspected: `PhysicsWorld::configure_thread_pool`
installs an explicit pool, and `num_threads` reports it. Without that override,
the native product uses Rapier's calling/global Rayon pool. In
`pipeline/physics_pipeline/solve.rs`, the staged solver uses the calling pool's
thread count with `parallel`; the serial build passes one and runs inline.
No engine-side task scheduler, alternate solver, unsafe code or public worker
configuration is added here.

## Executed parity checks

The independent [probe](../../tools/probes/physics-parity/README.md) separately
compiles serial and parallel builds from identical source and pinned shared
dependency versions. Three independent process trials compare serial, explicit
one-worker and explicit four-worker pools. Each trial covers a 273-body contact
pile and a 217-body scene containing 192 spherical joints. Every body has a
stable spawn-order ID. Each initial and stepped frame records all position,
quaternion, linear/angular velocity and sleep fields, with no tolerance or
rounding. The inputs, dt 1/64 and gravity -8 are dyadic.

On **Apple M5 Pro, macOS 26.6 (25G72), Rust 1.99.0**, all 18 native state streams
match byte-for-byte. Three fresh **Chromium 156.0.8078.4** processes actually
instantiate the serial WASM module and execute the same 640-step worlds. Their
six streams also match the native reference exactly. A compile-only WASM result
is not being substituted for this execution.

| Scenario | Bodies / frames | Peak active contact pairs | Peak sleeping bodies | Wake transitions | Stream bytes |
| --- | --- | --- | --- | --- | --- |
| Contact pile | 273 / 641 | 503 | 272 | 560 | 10,499,644 |
| Spherical chains | 217 / 641 | 192 | 192 | 240 | 8,345,884 |

[Machine evidence](evidence/physics-parallel-2026-10-10.json) records the exact
source hashes, stream hashes, per-process results, commands and log hashes.
Raw local streams remain in `artifacts/physics-parity-final`. An initial run
matched all 24 streams but failed while collecting a `rustc` version from PATH;
the runner now resolves the toolchain through rustup, and the complete final run
exits successfully. Its earlier output remains separate.

The product feature-graph guard passes desktop macOS/Linux/Windows, iOS,
iOS simulator, Android and WASM: every native graph has both requested features,
and WASM has only enhanced determinism. Direct product `incant_physics` checks
also pass for iOS simulator, Android and WASM. These three builds are compile
checks, not mobile execution. Workspace Clippy and **373 normal workspace tests**
pass with native parallel enabled, as do probe Clippy in both modes, both probe
test modes, **28 Python tool tests**, format and generated conventions/SDK/bridge
checks. The 44 pre-existing ignored GPU tests were not rerun for this dependency
change. No shared schema, SDK or authored scene changes are introduced.

## Hosted coverage and limits

The existing path-filtered physics workflow retains its Jolt candidate builds,
behavior tests, separate position/timing reports and cross-host reference check.
The old candidate workspace and its recorded medians are unchanged. New jobs
add serial/one/four-worker full-state parity on all three hosted desktop OSes,
plus actual Chromium/WASM on Linux, with a strict cross-host report comparator.
The original candidate build parallelism is preserved. New helper builds stay
isolated from the engine/editor workspace, and add no Rust dependency cache.

Linux obtains the [official wasm-bindgen 0.2.126 release](https://github.com/wasm-bindgen/wasm-bindgen/releases/tag/0.2.126)
as a prebuilt CLI instead of compiling it on every PR. The 11,484,249-byte Linux
musl archive was downloaded and independently checked locally against the
official asset digest:
`064948d58e2d6c0a745216477a639ba696216d6309aaa902939d1b865b1d869d`.
CI checks that hardcoded SHA256 before extracting only the required executable.
Executing that Linux binary remains part of hosted CI, not a macOS claim.

Hosted Linux/Windows parity is **pending**. This evidence covers two synthetic
workloads and public rigid-body state; it does not certify hidden solver state,
all shapes/joints, soft bodies, complete game determinism, Rapier 2D, the fast
variant, speedup, shipping-profile performance, hosted phones or a phase gate.
The PLAN Section 6.14 backend adoption rule and old Jolt candidate evidence stay
in force. Real device performance measurement and native physics migration
remain outstanding.
