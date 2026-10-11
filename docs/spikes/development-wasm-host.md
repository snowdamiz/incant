# Development Wasm module host

`incant_wasm_dev` implements a desktop development Wasmtime host behind the
explicit `wasmtime-host` feature. It executes real core Wasm against the native
ECS numeric/structural boundary. Default shipping runtime and player dependency
graphs do not reach Wasmtime or Cranelift, including with this workspace feature
enabled. The settled shipping implementation remains wasm2c; this work does not
implement it or satisfy the phone, AOT, SDK-migration or deterministic-Wasm gates.

## Boundary and ordering

`DevModuleHost::new` accepts bounded raw core Wasm, validates it and compiles it
locally. It never deserializes an executable Wasmtime cache and provides no WASI,
filesystem, network, shell or agent capability. Startup and the ABI handshake
have fuel but no active ECS capability. Unknown imports/signatures fail.
`tick(&mut NativeWorld, dt)` performs only the module phase; its owner schedules
native simulation separately. Invalid dt and a different world identity are
retryable preflight errors. The first accepted call binds the exact world.

Wasm linear memory does not alias ECS tables. An exclusive `NumericColumns`
scope copies bounded native IDs/columns, lets the guest read/write those copies,
then installs dirty columns and checks component domains and composed hierarchy.
A small undo guard restores invalid numeric writes using fresh safe borrows. The
host retains bounded numeric undo until `FrameCommands::apply` has validated and
committed the entire final structural graph. Rejected commands restore numeric
values and leave topology unchanged. No whole-world clone, JSON serialization,
authoring validation or second project mutation path is involved.

Any failure after acceptance permanently fails the instance; guest memory is
not rolled back. Component values, derived transforms, topology and simulation
tick remain unchanged on tested guest/numeric/structural rejection; conservative
Bevy change ticks can still advance during native column visits. Successful
`reload` explicitly resets guest memory/globals while preserving the bound world;
a failed reload candidate leaves the old instance unchanged. VM state migration
and hot reload of arbitrary guest state are not implemented.

## Version 1 transfer ABI

Exports are `memory` (unshared 32-bit), `incant_abi() -> i32` returning 1 and
`update(epoch: i64, dt: f64) -> ()`. Each import below belongs to `incant_v1`,
requires the exact active epoch and counts against the host-call quota.

| Import | Arguments after epoch | Result / transfer |
| --- | --- | --- |
| `chunk_count` | none | i32 count |
| `chunk_len`, `chunk_flags` | chunk i32 | rows / bit 0 indicates Velocity |
| `read_ids` | chunk, destination i32 | rows × 16 canonical stable-ID bytes |
| `read_transforms`, `write_transforms` | chunk, memory offset i32 | rows × 10 little-endian f64 |
| `read_velocities`, `write_velocities` | chunk, memory offset i32 | rows × 3 little-endian f64; missing column traps |
| `spawn` | ID, parent, Transform, Velocity offsets | queues typed spawn |
| `despawn` | ID offset | queues typed despawn |
| `set_velocity` | ID, Velocity offsets | queues typed set/remove |
| `set_parent` | ID, parent offsets | queues typed reparent |

Transform order is position xyz, quaternion xyzw, scale xyz. Optional parent or
Velocity uses only offset -1 as absence. Other negative offsets and checked
out-of-range spans trap. Whole chunks transfer; no IDs, padding or native pointers
are writable through numeric arrays. Chunk/row order is unspecified across
structural edits; modules identify entities by stable IDs. Epochs prevent retained
indices from addressing later ticks. Physics configuration, input, audio, spatial
configuration and other unimplemented imports fail explicitly.

## Fixed quotas and limits

| Resource | Limit |
| --- | --- |
| Raw module | 1 MiB, core Wasm version 1 |
| Types / defined functions / actual imports | 1024 / 1024 / 32 |
| Globals / exports / data segments / element segments | 128 each |
| Tables / memories | 1 each |
| Locals per function / function body | 4096 / 256 KiB |
| Linear memory / table elements / instances | 32 MiB / 4096 / 1 |
| Wasm stack | 256 KiB |
| Startup including handshake / tick fuel | 1,000,000 / 10,000,000 |
| Captured numeric storage including IDs / chunks | 32 MiB / 4096 |
| Additional dirty numeric undo | bounded by captured storage, at most another 32 MiB plus bounded vector metadata |
| Cumulative transfer / host calls / commands per tick | 128 MiB / 65,536 / 1024 |

Growth beyond memory/table limits traps. Threads, shared memory, memory64,
multi-memory, SIMD/relaxed SIMD, function references, GC, tail calls, stack
switching, exceptions and compact imports are disabled. NaNs are canonicalized.
These limits are not a hard compiler wall-time or total-process/JIT allocation
cap. Stable cross-platform iteration/floating-point determinism is not established.

## Executed evidence

Exact source and log hashes plus the fresh-process report are recorded in
[evidence JSON](evidence/development-wasm-host-2026-10-10.json). Local host:
aarch64 macOS, Rust 1.99.0 (LLVM 23.1.1).

- `./tools/cargo test -j2 -p incant_wasm_dev -p incant_runtime -p incant_script --features incant_wasm_dev/wasmtime-host --release --locked`: **123 tests/doctests pass**, including 13 actual Wasm conformance tests.
- `./tools/cargo clippy -j2 -p incant_wasm_dev --features wasmtime-host --all-targets --release --locked -- -D warnings`: pass.
- Feature-disabled `cargo check -j2 -p incant_wasm_dev --no-default-features --locked` on the local host, `wasm32-unknown-unknown`, `aarch64-apple-ios` and `aarch64-linux-android`: pass. These establish inactive-feature isolation, not mobile/browser Wasmtime execution.
- Runtime dependency guard for `incant_wasm_dev`, `incant_game_player`, `incant_script_player` with `incant_wasm_dev/wasmtime-host`: pass. Probe separately traverses normal graphs for five shipping roots and rejects Wasmtime/Cranelift.
- `python3 tools/probes/development-wasm.py /tmp/incant-development-wasm-proof-02 --rustup /opt/homebrew/bin/rustup`: pass. Pinned Rust compiles the trusted no_std fixture directly to Wasm; three fresh processes execute it after deleting the temporary Rust input. Every process matches exact dyadic state: IDs 1/3 at x=0.25 and IDs 2/4 at x=0.125 after eight 64-Hz native/module ticks.
- Formatting, generated conventions, diff and actionlint checks pass. A separate development-Wasm workflow explicitly runs the optional-feature tests, Clippy and Rust guest probe on three desktops, sharing the reviewed registry-only cache. Hosted results are pending; no global workspace/player feature was enabled.

The tests compare actual Wasm, QuickJS and direct native writes; cover
Transform-only/multiple tables, exact ID bytes, hierarchy, structural growth and
reacquisition, traps after writes, NaN/quaternion/scale/finite-hierarchy failures,
late structural-cycle atomicity, stale epochs, OOB access, missing columns,
inactive startup capability, unknown/WASI imports, wrong signatures/versions,
source/cache rejection, reload and identity, and actual fuel/stack/memory/table/
transfer/command/call quotas.

The proof deletes its temporary guest compiler input, not the repository fixture
or host build storage. This is development JIT execution of raw Wasm, not a
source-free shipped player or trusted arbitrary-source compiler service. It does
not run project build scripts/procedural macros. Linux/Windows execution, shipping
wasm2c, browser parity, all SDK capabilities, arbitrary-source sandboxed builds,
full guest persistence and device/performance gates remain open.

## Pinned upstream basis

Wasmtime **49.0.2** is pinned exactly, with std/runtime/cranelift only.
Its [release](https://github.com/bytecodealliance/wasmtime/releases/tag/v49.0.2)
and [tagged workspace manifest](https://github.com/bytecodealliance/wasmtime/blob/v49.0.2/Cargo.toml)
identify the matching wasmparser **0.258.3** and test WAT **1.258.3** resolutions.
The host API was checked against the downloaded 49.0.2 source, including
[Config](https://github.com/bytecodealliance/wasmtime/blob/v49.0.2/crates/wasmtime/src/config.rs)
and [Store limits](https://github.com/bytecodealliance/wasmtime/blob/v49.0.2/crates/wasmtime/src/runtime/limits.rs).
The upstream [security model](https://docs.wasmtime.dev/security.html) motivates
raw validated Wasm and capability-limited imports; it does not authenticate native
serialized caches or supply our process-wide compiler quotas.
