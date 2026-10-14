# QuickJS heap telemetry: source assessment

A custom allocator can provide constant-time, exact counters for its **backing
allocations**, including their lifetime high-water mark. It cannot replace
`memory_used_size`, `malloc_size` and `malloc_count` under the same names or
semantics. The pinned QuickJS allocator pools small objects in arenas, and its
heap enumeration computes another, partly estimated object-graph metric.

This is a source-only assessment against main
`ed3999b02d2a747dee93d278a2c1c50d43f275b7` and registry rquickjs 0.14.0.
The relevant production behavior also matches shipping reference `03bc4b98`.
Nineteen source copies/hashes are retained in
`~/.codex/incant-evidence/quickjs-heap-accounting-001/inputs.json`
(SHA recorded with the review handoff). No allocator code, dependencies, runtime
limits or measurements changed.

## Actual cost and semantics

`incant_script/src/lib.rs:619` queries `Runtime::memory_usage` only while an
Incant profile capture is enabled, after guest/context loans close and after
native publication or refusal. It is outside the guest execution deadline but
inside actual ScriptTick profiling. Nonprofiled gameplay already skips this scan.
The earlier actual-host diagnostic measured about 0.104 ms mean for this region;
that is diagnostic, not a guaranteed saving or shipping p95 prediction.

`rquickjs-core/src/runtime/raw.rs:331` invokes `JS_ComputeMemoryUsage`.
In `quickjs.c:7574`, that function first reads three maintained scalar counters,
then traverses contexts, GC objects, properties, arrays, shapes, atoms and values.
Its `memory_used_size` is an object-size accounting estimate: several internal
classes have TODO handling, some shared values use reference-count division and
rounding, and ArrayBuffer data contributes byte length even for externally owned
backing memory. It is neither process RSS nor exact retained allocator bytes.

`incant_profile/src/enabled.rs:281` takes the maximum of these **end-of-tick
samples**. `peak_used_bytes` and `peak_allocated_bytes` do not capture an allocation
that is created and freed between samples. They also take a maximum across sampled
VMs, not the concurrent aggregate of every VM. Replacing this with fewer samples
and calling the result an exact peak would be incorrect.

## What a custom allocator sees

The published `Runtime::new_with_alloc` API accepts an `unsafe Allocator` trait
implementation. The default graph selects `std`, not `rust-alloc`:
`Runtime::new` currently uses QuickJS's libc allocator. The alternative supplied
`RustAllocator` stores a size header, rounds to eight-byte alignment and calls
Rust's global allocator.

QuickJS `arena_new` (`quickjs.c:1751`) obtains approximately 4 KiB backing blocks.
`js_arena_malloc`/`js_arena_free` carve/recycle small objects without invoking the
backing allocator for each object; the last free releases the arena. Larger
allocations go directly through the callbacks with an internal header. Therefore:

| Counter | What it means | Constant-time source |
| --- | --- | --- |
| Current `memory_used_size` | Enumerated/estimated object memory, including some external buffer bytes | None exposed; full scan required |
| Current `malloc_size`/`malloc_count` | Maintained logical QuickJS allocation bytes/count, including individual arena blocks | Native private `malloc_state`; no public getter |
| New backing usable bytes/count | Successful live allocator blocks, including arena slack and large block/header capacity | Custom callback counters |
| New backing lifetime peak | Maximum backing counter after successful allocation/reallocation | Update counter at every callback |

A backing-block count must never be labeled object count. A backing-byte high-water
mark is not semantic live heap, malloc's internal transient realloc peak, operating
system commit, GPU memory or RSS.

## Hard-limit finding and allocator hazards

The rquickjs `set_memory_limit` documentation says custom allocators make it a
no-op. That prose disagrees with this pinned implementation. The Rust method
unconditionally calls `JS_SetMemoryLimit`; C stores `malloc_state.malloc_limit`.
`js_malloc_rt`, `js_calloc_rt` and `js_realloc_rt` check that limit before calling
any backing allocator, and update logical sizes afterward. This path also applies
to `JS_NewRuntime2` custom allocators. Preserve the existing 32 MiB limit,
1 MiB stack limit, GC policy and interrupt handler; do not infer enforcement from
API prose alone. A real custom-allocator exhaustion control is still required.

Switching to RustAllocator is not transparent: large-allocation usable sizes and
padding can differ, which affects logical charges, GC thresholds and precise OOM
points. It also moves QuickJS backing allocations into the existing process-wide
Rust requested-byte profiler. Old/new Rust memory numbers would no longer have
the same meaning; combining both fields would double-count. Such a change needs
an explicit allocator/metric identity, not the current matched-baseline label.

A wrapper must preserve null/failure/reallocation ownership exactly. Read old
usable size before realloc, update counters only after success, and retain old
live bytes on realloc failure. Zero/null cases need explicit delegation matching
the native contract; overflow must refuse before invoking helpers that panic.
Do not copy the pinned allocator's test wrapper: its example subtracts the old
allocation before knowing whether realloc succeeds. No logging, allocation,
locking or unwinding may occur inside allocator callbacks. Per-runtime counters
need to outlive VM teardown, with an invalid/saturation flag if invariants fail.

The existing allocator trait requires a small, audited unsafe delegation layer;
there is no fully safe callback implementation hook. Platform libc wrapping could
preserve the current backing allocator, but needs portable usable-size bindings
and their own review. Delegating to supplied RustAllocator is smaller but changes
the backend as described. Neither is an unconditional production recommendation.

## Smallest next implementable choice

A bounded allocator diagnostic is technically possible without a normal dependency
fork: a per-runtime counter wrapper around supplied RustAllocator, fixed shared
counter storage allocated before VM creation, and `Runtime::new_with_alloc`.
Read counters only after exclusive VM/context activity closes. Proposed distinct
fields are `backing_usable_bytes`, `backing_allocation_count`,
`runtime_peak_backing_usable_bytes`, `allocation_failures` and `accounting_valid`,
with `allocator_kind` and `metric_version`. Count every successful callback from
runtime creation, including initialization; do not reset a lifetime peak when
profiling starts. A capture-interval peak would require a separate, explicit
baseline/reset protocol. Existing scan-derived used bytes must become explicitly
unavailable or remain a separately requested diagnostic; never populate them with
backing bytes or pretend zero means measured.

Expected tradeoff: constant-time tick telemetry removes the object scan, while
alloc/free/realloc acquire a few counter/size reads and writes. Startup requires
fixed counter ownership plus custom allocator boxes. Use per-runtime state, not a
pointer map or heap allocation on each callback. Timing benefit remains unknown,
and a wrapper can make allocation-heavy guest execution slower.

An alternative is a tiny upstream-style C getter of the **existing** malloc_state
counters, exposed through rquickjs under its runtime lock. This preserves libc,
logical allocation semantics and refusal behavior, with no per-allocation work.
It supplies exact current logical counters and sampled peaks, not a continuous
peak or object used-size. Continuous logical peak would need a native high-water
counter updated at every successful malloc/calloc/realloc. Both require a reviewed
pinned dependency patch, so they must be weighed against interpreter maintenance;
private struct layout casts are not an acceptable shortcut.

Before either option reaches production, focused observable controls must cover:

- allocation/free and growing/shrinking/failed realloc accounting, zero/null and
  overflow behavior, independent runtimes, teardown returning live counts to zero;
- transient allocate-then-free peaks distinguishable from end-of-tick samples,
  arena recycling distinguishable from backing allocation count, external borrowed
  buffers distinguished from owned allocations;
- real VM memory exhaustion under the unchanged 32 MiB native limit, exact failed
  tick rollback/poison/deadline behavior and no usable capability after failure;
- bytecode/compiler and selected physics modes, feature-off default graph, honest
  profiler serialization/Tracy labels and refusal to match unlike metric identities;
- matched actual-host/shipping measurements only after correctness, including
  startup and allocation-heavy cases to expose moved overhead.

No extra region measurement is needed to decide this semantic boundary. A direct
custom-allocator substitution preserving the old report fields should be rejected.
A distinct backing-accounting experiment or narrowly reviewed native scalar getter
is the next concrete design choice; neither has been implemented here.
