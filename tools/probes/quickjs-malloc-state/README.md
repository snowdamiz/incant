# QuickJS maintained-allocation diagnostic

This is a source-pinned, native-only correctness experiment. It does not patch
Incant's dependency graph, replace the allocator, change profiler fields, or
establish a shipping speedup. The earlier Float64 store experiment is excluded.
The parent heap assessment is preserved at commit
`11dac35af6049c9ed5d91284022217f0fc11ca30`.

## Small native boundary

`sys.patch` adds `JS_GetMallocState` to the copied QuickJS C implementation and
header, with one matching pre-generated `aarch64-apple-darwin` declaration. It
reads only `malloc_state.malloc_size`, `malloc_count`, and `malloc_limit` into
caller-owned `size_t` outputs. No opaque runtime layout is read from Rust. The
existing allocator, arena, limit checks, GC, interpreter and bytecode remain
unchanged. Native zero still denotes an unlimited memory limit.

`core.patch` adds a plain owned `MallocState` and safe `Runtime::malloc_state()`.
The wrapper holds the existing exclusive runtime guard (`RefCell::borrow_mut`
for these default non-parallel features), calls the native accessor on its own
live runtime, then returns three scalars. No pointer or lifetime escapes. As with
the existing `memory_usage()`, calling it reentrantly inside `Context::with` is
not supported. No custom allocator or new unsafe buffer constructor is used.

The diagnostic-only `diagnostic_malloc_state_with_usage()` holds **one** guard
across getter → unchanged `JS_ComputeMemoryUsage` → getter. It establishes exact
bytes/count/limit parity and lack of getter/full-scan mutation; no guest callback
can intervene. The full scan remains the independent native oracle, including
its distinct estimated `memory_used_size`. Both methods exist only in copied
crates, not a proposed supported Incant API yet.

## Metric meaning

The three values are exact instantaneous native logical-allocation counters and
configured limit. They are not libc arena backing size, Rust requested bytes,
RSS, an exact semantic object graph size, or continuous peaks. Small objects use
the existing arena while logical counters account for individual allocations.
The full scan's used-size estimate also includes externally backed ArrayBuffers.

One safe immutable `Arc<[u8]>` control keeps its backing alive outside the VM,
verifies JS contents, detaches the JS view and verifies external ownership/data.
This is external/shared backing versus native-owned buffer accounting; it is
**not** an unsafe mutable ECS loan, loan rollback, or native-runtime integration
qualification. A transient allocation sampled before/during/after explicitly
shows why the maximum of endpoint samples does not capture a continuous peak.

## Fixed qualification plan

Before any Cargo build, freeze these sources, both original crate inventories,
license metadata, root lockfile, toolchain/compiler identities and the proposed
commands. Use the isolated preparation pattern below after source review.

1. `prepare.py REGISTRY FRESH_EXTERNAL_DIRECTORY` verifies `pins.json`, copies
   exactly rquickjs-core/sys 0.14.0 and applies getter-only patches with zero fuzz.
   It records full copied/original inventories and proves the registry unchanged.
   `getter` and `wrong-count` are separate packages; the latter adds exactly one
   to the returned allocation count and must fail the first parity assertion.
2. Reconcile each copied lock using **full** `cargo metadata --offline`, then
   `check_lock.py ROOT_LOCK COPIED_LOCK`. Retain metadata and assert resolved
   core features are only default/std, with no rust-alloc, parallel or bindgen.
   The repository lockfile must remain unchanged. No unlocked build or downloads.
3. Build each variant once with pinned Rust 1.99.0, repository `./tools/cargo`,
   `--offline --locked --profile ci --target aarch64-apple-darwin -j2` and
   `RAYON_NUM_THREADS=2`. Each variant requires a distinct **fresh target**:
   previous relative-path patched crates demonstrably reused stale artifacts in
   a shared target. Retain verbose compiler commands, actual compiled C/Rust
   copies, reconciled locks and immediately copied executable hashes. Require
   distinct binaries and compiled source agreement before execution.
4. Run the getter binary with a bounded process-group supervisor (120 seconds),
   retaining complete stdout/stderr/exit status. It must emit exactly **32** named
   parity observations followed by one success row. All data assertions remain
   outside any timing interval; this harness has no timing interval.
5. Run wrong-count once under the same supervisor; require nonzero exit and the
   first `count: first/before-context` assertion, with no success row. This tests
   actual C getter branch reachability and the parity oracle, not just source
   mutation. Retain this expected refusal separately from harness failures.
6. Run strict harness Clippy, scoped rustfmt and source/lock/inventory rechecks.
   Retain failures and immutable source successors if an actual fix is needed.
   No broad engine suite, shipping build, benchmark or physical-device test.

The 32 observations cover pre/post context creation in two independent VMs;
10,000 small-object allocation, alternating release/reuse and cyclic GC; native
resizable ArrayBuffer grow/shrink and OOM realloc refusal preserving its old
length/content; ordinary OOM allocation refusal at 32 MiB; external versus
native-owned 1 MiB buffers and detach/free; a 2 MiB transient; independent VM
isolation; explicit zero/unlimited then restored limit without allocations while
unlimited; and context teardown. Stack cap remains 1 MiB, default GC behavior is
unchanged. The OOM controls catch the actual native `out of memory` exception,
not an unrelated max-length error.

## Maintenance and limits

The exact original C source hash is
`3a6b52a225c21709ef1fd314183efd3af862e867ce4ef8caabcae67922a745b3`.
The copied QuickJS MIT license is retained verbatim. The published rquickjs-core
crate declares MIT in its pinned manifests but contains no standalone license
file; this diagnostic does not invent one. All copied files retain original
headers and metadata. Future production vendoring would need normal dependency
notice/provenance review and a supported upstream or pinned patch policy.

Any future adoption must maintain the C declaration, generated bindings on every
supported target, safe wrapper locking, and pinned native semantics at upgrades.
This experiment changes only the native Apple arm64 binding: other targets,
bindgen output, parallel/async runtime wrappers and custom allocators are not
qualified. It cannot itself remove `memory_usage()` from ScriptHost. Such an
integration needs explicit unavailable/detailed-used-size and sampled-peak
semantics, instrumentation identity, separate source review and matched shipping
measurement. Hard memory enforcement remains in native allocation paths and does
not depend on either telemetry method.
