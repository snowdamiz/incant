# Native QuickJS component columns

This is the first functional script host for PLAN.md section 2.6. The canonical
`incant_script` package now depends on the native runtime rather than authoring.
The old document-backed implementation is mechanically relocated to
`incant_play_legacy`; current user-facing headless gameplay still uses that package.
This checkpoint does **not** complete migration of PlaySession or its systems.

## Execution and bytecode boundary

`CompiledScript::compile` accepts SWC CommonJS output, bounded to 1 MB, and invokes
QuickJS's compile-only module declaration and bytecode writer. It executes no
project code. `ScriptHost` initializes a fresh bounded VM from those immutable
compiler-produced bytes, then calls the retained update function on every tick.
The bytecode bytes are private and have no public file/cache deserializer. The
same pinned QuickJS compiler/runtime produces and consumes them; a hash would
not establish executable-code trust. The host retains the source byte allocation
until after all function roots, context and runtime are destroyed because the
loader's ROM-data path may retain references into it.

`update({chunks, dt})` receives one array of actual Bevy table views: stable ULID
strings, Transform f64 stride 10 (translation xyz, rotation xyzw, scale xyz), and
optional Velocity f64 stride 3. A call updates all matching entities in bulk;
there is no project clone, command bus, JSON conversion or per-tick code parsing.
The caller owns native stepping; the script phase does not increment world time.
No filesystem, network, module loader or engine-agent shell capability is installed.

`examples/native_bulk` is explicitly a development runner: it loads a checked
binary scene and compiles CommonJS **once at process startup**. Shipped bytecode
packaging, authenticated loading, version compatibility and source maps remain
open. This development example is not evidence for the no-source-parsing export
gate or the completed cook pipeline.

## Foreign-view lifetime audit

The outer `NumericColumns` scope exclusively owns the native world. The adapter
collects aligned initialized f64 ranges, original values for bounded undo, and
stable IDs. It checks that ranges do not overlap. Safe slice visits end before
the adapter exposes raw pointers. The adapter then retains exclusive access to
the outer handle and performs no more visits while foreign views exist. Table
allocations cannot move, and no native system or derived-state observation can
run until the scope ends.

Each numeric range has exactly one non-owning, fixed external ArrayBuffer; its
reallocation callback is null. A rooted buffer is recorded before constructing
the fallible typed view. QuickJS's same-length transfer can steal a pointer even
from a fixed non-owning buffer, so the fresh context disables every pinned
ArrayBuffer transfer entry point before project code runs, freezes that prototype
and constructor, and refuses additional realms/workers/structuredClone. Ordinary
owned copies remain possible. Typed arrays use the engine intrinsic constructor.

Every exit detaches all roots before native component reads, undo restoration or
release of the outer ECS scope. This includes partial metadata construction:
prototype setters can execute project code during property installation, so the
error path revokes before reading even an exception's name/message getter. Saved
typed arrays, subarrays and DataViews then retain only detached buffer objects.
Failure restores the exact bounded numeric prefix while still inside the world
loan. Outer-scope cleanup publishes change ticks and refreshed hierarchy once.

This deliberately extends raw pointer use beyond a safe slice callback, within
the exclusive outer scope; the native API's documentation now states that
adapter contract. No pointer is cached across ticks or structural changes.
Unsafe operations are confined to external-buffer creation, value ownership,
bounded restoration, trusted bytecode loading and a read-only pending-job query.
Miri/sanitizer and exhaustive engine sandbox proofs are not claimed.

## Failure, budgets and synchronous work

VM memory is capped at 32 MiB and stack at 1 MiB. Each invocation has a thread-CPU
deadline, default 50 ms. Numeric undo has a separate 32 MiB cap; ID strings,
metadata and bookkeeping are additional bounded-by-row-count allocations. The
budget begins before metadata exposure and stays active through diagnostics.
Exception, allocation failure, CPU expiry, unsupported return or deferred work
rolls back numeric writes and permanently fails the host. Recreate the host to
continue. Process abort cannot run rollback destructors.

The synchronous contract rejects **created**, ignored and returned promises;
checking only a returned Promise would silently accumulate never-executed jobs.
A nonallocating native hook counts promise initialization. The pinned
QuickJS source creates two internal promises for one synchronous JS module:
`js_evaluate_module` creates its completion promise, then
`js_execute_sync_module` calls `js_async_function_call`, which creates an execution
promise before the module body runs. Ordinary initialization must observe exactly
two; ticks must observe zero. The counter saturates at three. A read-only queue
check also rejects pending jobs; the host never drains jobs. The two-promise
baseline was confirmed by an unsaturated runtime measurement and pinned-source
review, and must be re-audited when QuickJS or the module strategy changes.

Direct numeric access intentionally does not revalidate components each tick,
as PLAN.md requires. All f64 bit patterns remain memory-valid. Gameplay can write
NaN, overflow or unusable transforms; consumer math policy and optional development
diagnostics are separate work. Invalid arithmetic is not turned into an authoring
validation pass. This adapter's rollback handles execution failure, not all
possible gameplay mistakes.

## Verification and remaining migration

On arm64 macOS, all 363 workspace tests across 114 suites and workspace all-target
Clippy pass after the ownership and async review fixes. Eleven focused script
tests are included. Tests exercise actual multi-table Transform/Velocity
writes for 120 ticks, hierarchy, retained views across 5,000 spawns/archetype
moves and GC, exceptions, memory/CPU failure, transfer routes, copied buffers,
unsupported state/async, ignored unresolved promises, and hostile metadata/error
getters, dynamic imports and deferred work during metadata exposure. All 24
Python tool tests and the generated conventions check also pass.

A development CLI run executed 60 native steps with script writes from a checked
scene after the original project/journal had been removed. Two fresh processes
matched exactly, and zero ticks restored the original scene. Parent translation
was approximately [3, 2, 0], child local translation [1, 4, 3]. The probe was
repeated after the final review fixes on the newly built binary; its hash and
results are in [recorded evidence](evidence/native-script-columns-2026-10-10.json).

The dependency guard now rejects `incant_play_legacy` and checks `incant_script`
alongside `incant_runtime` and `incant_types`. The final resolved graph passes.
Regenerated schemas are byte-for-byte unchanged and the generated SDK check passes.
Hosted CI remains pending. Independent Astra Extra High review found no concrete
memory-safety defect, identified the async gap above, and confirmed it resolved
on re-review.

Persistent behavior state, timers, logs, input/actions, queries/physics,
structural bindings, hot reload, saves, SDK compatibility, real gameplay migration,
browser/Wasm hosts, authenticated cook/export artifacts, profiling and performance
work remain. Each tick currently rebuilds metadata/IDs and copies numeric undo;
these costs need measurements and optimization. No shipping-profile improvement,
10,000-entity phone budget, hosted device run or phase gate is claimed here.

The final local verification commands were:

```sh
./tools/cargo fmt --all --check
./tools/cargo test -j2 --workspace --release --locked
./tools/cargo clippy -j2 --workspace --all-targets --locked -- -D warnings
./tools/cargo build -j2 -p incant_headless -p incant_script --examples --release --locked
./tools/cargo build -j2 -p incant_headless --release --locked
target/release/incant_headless schema schemas
git diff --exit-code -- schemas
node tools/generate_sdk.mjs --check
python3 tools/check_runtime_dependencies.py incant_types incant_runtime incant_script
python3 -m unittest discover -s tools/tests -q
```
