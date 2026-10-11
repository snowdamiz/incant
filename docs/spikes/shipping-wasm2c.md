# Static native Wasm module execution

The first shipped-native module slice translates a trusted Rust fixture through
validated Wasm and wasm2c into a statically linked standalone executable. The
same copied `incant_v1` numeric/structural capability implementation now serves
the development Wasmtime adapter and the static C adapter. This establishes a
bounded desktop execution path, not complete SDK migration or a device gate.

## Build and runtime boundaries

`incant_wasm_cook` accepts at most 1 MiB of core Wasm. It validates the original
module, exact import names/signatures, export contract, proposals and resource
limits before adding an unaddressable private fuel global. Each original
instruction is charged, including loop bodies and indirect-call paths. The
emitted module is validated again. The initial static capability rejects
`memory.grow` and `table.grow` explicitly and fixes each allocation at its initial
size; development allocation behavior is unchanged.

`tools/wasm2c.py` translates prepared modules using pinned WABT 1.0.42 and emits
a C trap facade plus Rust linked descriptors. The optional `embedded-fixture`
feature compiles this trusted bundle and the pinned upstream C runtime. It is
not a project-controlled build service: Rust compilation, native tools, the C
bundle and build environment are trusted. The manifest records identity, not
authentication. There is no runtime Wasm, native-cache or C-code file loader.

`incant_wasm_runtime` has no compiler/JIT normal dependency. Its unsafe descriptor
constructor is for build-linked functions with an explicit allocation,
exclusive-memory and callback contract. Guest traps remain within C: `setjmp`
appears as a controlling-expression comparison, and Rust import callbacks
return before C raises a trap. Rust panics are contained at the callback boundary.
No longjmp unwinds Rust frames, signal handler recovers guest faults, or guest
pointer names ECS storage. Instances clear callbacks before leaving a C call.

The C build disables memory mapping, uses checked bounds and a 64-call stack
limit, and disables sibling-call optimization and floating-point contraction.
The proof checks expected flags in the build output. This is a global-presence
audit, not validation of every translation unit against contradictory
target-specific flags or a hostile build environment. A real multiply/add
regression additionally checks that the C compiler does not fuse those Wasm
operations. Canonical NaN behavior and cross-platform deterministic execution
remain unproven.

## Native transaction semantics

The shared capability owns an opaque world identity and accepts only one
synchronous exclusive-world invocation at a time. Guest memory receives copied
stable IDs and f64 Transform/Velocity columns; no ECS reference survives a guest
call. Commands use the existing typed `FrameCommands` boundary. Imports provide
no WASI, shell, filesystem, network, clock, randomness or engine-agent capability.

An invalid dt or different world is rejected before acceptance and is retryable.
Once accepted, a trap, quota failure, import error or invalid result permanently
fails that host instance. A development reload explicitly replaces it. Guest
memory is not rolled back. Numeric writes are installed under bounded undo and
checked for component, hierarchy and existing physics constraints; the complete
structural batch is then prevalidated by the native command implementation.
Rejected numeric/domain or structural-graph changes restore numeric values
through fresh safe visits before returning. No whole-world clone, document
projection, JSON tick path or second structural mutation implementation is added.
Conservative native change ticks can still advance during a rejected numeric
visit.

This is not rollback of an entire game tick or a failed solver/backend. An owner
that has already advanced simulation must treat a later module error as terminal.
The example does so by dropping its world and emitting no state on failure.

| Resource | Static limit |
| --- | --- |
| Raw Wasm | 1 MiB |
| Linear memory | One fixed allocation, at most 32 MiB |
| Funcref table | At most one, fixed, at most 4,096 entries |
| Functions / types | 1,024 each |
| Imports | 32, exact supported function capabilities |
| Globals / exports / data / element segments | 128 each |
| Per-function params / results / locals / operand stack | 128 / 16 / 128 / 128 |
| Function body | 256 KiB |
| Native guest call depth | 64 |
| Startup / tick instruction fuel | 1,000,000 / 10,000,000 |
| Captured numeric storage | 32 MiB plus bounded dirty-column undo |
| Numeric chunks / calls / structural commands | 4,096 / 65,536 / 1,024 per invocation |
| Copied bytes | 128 MiB per invocation |

These are bounded conformance limits, not a total-process memory quota or a
wall-clock deadline. Build-time compiler resource isolation is separate work.

## Executed evidence

The [durable evidence](evidence/shipping-wasm2c-2026-10-10.json) records source,
log, generated-bundle and final report hashes. Commands ran against the frozen
source on Apple Silicon, macOS 26.6 (25G72), Rust 1.99.0/LLVM 23.1.1 and Apple
Clang 21.0.0. `INCANT_WASM_BUNDLE` below names the trusted prepared test bundle.

- `INCANT_WASM_BUNDLE=/tmp/incant-wasm2c-bundle-04 ./tools/cargo test -j2 -p incant_wasm_runtime -p incant_wasm_cook -p incant_wasm_dev -p incant_runtime -p incant_script --features incant_wasm_dev/static-conformance --release --locked`: **135 tests/doctests pass**. All original 13 Wasmtime cases remain unchanged and pass; seven static cases, four validator cases and one exact 16-tick static/development state-stream parity test are added.
- The same three Wasm packages with `cargo clippy -j2 --features incant_wasm_dev/static-conformance --all-targets --release --locked -- -D warnings`: pass.
- `python3 tools/probes/shipping-wasm2c.py /tmp/incant-wasm2c-combined-ci-local --rustup /opt/homebrew/bin/rustup --conformance-only`: pass. This executes the new CI helper: 25 tests/doctests, Clippy and the original three-process development Rust guest probe on one development dependency graph.
- `python3 tools/probes/shipping-wasm2c.py /tmp/incant-shipping-wasm2c-02 --rustup /opt/homebrew/bin/rustup`: final fresh shipping proof passes. It builds the executable under opt-level 3, one codegen unit, aborting panic and final fat LTO; three fresh processes match exact dyadic state and 18 runtime invocations are rejected.
- `INCANT_WASM_BUNDLE=/tmp/incant-wasm2c-bundle-04 ./tools/cargo check -j2 -p incant_wasm_runtime --features embedded-fixture --target aarch64-apple-ios --locked`: passes, including actual C compilation. This is not an iOS link, signing, device execution or distribution proof.
- `./tools/cargo check -j2 -p incant_wasm_runtime --no-default-features --target wasm32-unknown-unknown --locked`: passes. No static C/browser execution is claimed.
- The authoring-dependency guard passes for six runtime/player roots with development and static features resolved. The proof separately excludes Wasmtime, Cranelift, wasmparser, wasm-encoder and the cook crate from those normal graphs.
- 40 Python tool tests, formatting, generated conventions, actionlint and diff checks pass.

Tests exercise real traps after writes, NaN/quaternion/scale rejection, finite
hierarchy overflow, a late cyclic structural batch after numeric writes,
out-of-bounds loads/imports, expired epochs, startup imports/fuel, direct and
indirect infinite loops, recursion, call/command quotas, independent concurrent
instances and recovery of the thread's C trap target. The parity case includes
parenting and native **ECS table reallocation**, not Wasm table growth.

The final executable is **2,780,816 bytes**, SHA-256
`ba2f20354371ceb757ed193c572a16eee2a2bd2b921a4e864ad39e548947b4b7`.
Its separately packaged arm64 dSYM matches UUID
`cac646f4-3aa6-3e1b-9568-06cfca4e2172`. The player includes the adversarial fixture
modules as well as the Rust module; this is not a minimal single-module size.
The final report SHA-256 is
`db0057828e7184e6905e0261d5af2fd772d1aeb4c2cab955b6137785f91f2759`.

The proof deletes its temporary Rust/WAT sources, raw Wasm, generated C, downloaded
tools and fresh Cargo build storage before executing the copied player. The
retained player package contains only the executable, binary scene and required
notices/license; symbols are separate. Repository fixtures and the ordinary
validator build directory remain elsewhere. Runtime source/module routes are
rejected; no claim is made that every source file on the machine was deleted.

## Provenance, CI and remaining scope

The translator and six unmodified runtime files are from
[WABT 1.0.42](https://github.com/WebAssembly/wabt/releases/tag/1.0.42), commit
`ff0ef7e0009402740c805a9744c09b05be063e48`. The
[upstream wasm2c documentation](https://github.com/WebAssembly/wabt/blob/1.0.42/wasm2c/README.md)
describes the generated runtime and required compiler semantics.
`third_party/wabt/provenance.json` pins individual source hashes, commit and
Apache-2.0 license. A header-only Git whitespace exception preserves three
upstream comment lines byte-for-byte. The provisioning tool checks official platform archive
SHA-256 values and extracts only the two named executable members. Notices and
the repository Apache-2.0 license are packaged with the player. wasmparser and
wasm-encoder remain exactly 0.258.3, WAT 1.258.3; no external lock resolution
changes in this slice.

The reviewed `development-wasm.yml` keeps Windows's complete development-only
tests/Clippy/Rust guest probe and adds combined static/development conformance
plus a separate fresh shipping audit for macOS/Linux. Existing default-feature
Wasm/iOS/Android isolation remains. Registry-only cache policy is unchanged.
**Hosted execution and cold job timing are pending.** Local execution is macOS
arm64 only; Linux flags and ELF packaging await that matrix. MSVC is explicitly
rejected, and Windows static support is not implemented.

The remaining native platforms, signed mobile builds, browser parity, complete
SDK imports, production module/export selection, deterministic arithmetic and
reference-device performance gates remain open. This copied ABI does not claim
zero-copy speed, TypeScript AOT adoption or full game-session Wasm integration.
