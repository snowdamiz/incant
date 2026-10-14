# Correlated full-Core script attribution (private diagnostic)

This private diagnostic starts from main281
`dfac86cb17e9a1dd71d63207d779b3836e7ad6ae`. Main282 changes documentation only.
The rejected ordered-query implementation is absent. Focused CI qualification,
one private shipping build, one functional child and one correlated diagnostic
child are now complete. The results identify a navigation investigation; they
do not establish a performance improvement or change any normal shipping gate.

The retained main278-player B0/B1/B2 runs from the ordered-query comparison have
mean whole ScriptTick 1.4715–1.4805 ms, actual source callbacks 1.1255–1.1345 ms,
and a same-run mean remainder 0.3430–0.3460 ms. Stress accounts for about
0.910–0.919 ms and the controller about 0.215 ms. These are additive totals divided
by the same 2,156 calls. The marginal ScriptSource p95 is 1.67–1.69 ms and
ScriptTick p95 2.08–2.10 ms; those percentiles cannot be subtracted or assumed to
refer to the same ticks. Their whole-tick mean wall-minus-thread-CPU difference
is only 69–172 ns. That does not identify a scheduler-wait cause for the tail.

The old `core-script-phase` CI probe omitted the actual controller, physics,
assets, UI, audio and input route; its heap observer also predates maintained
counters. Its 253 us validation and 76 us cleanup means identify broad regions,
not a new proven query, matrix, allocation, or VM optimization. Matrix-cache,
loop, typed-store, Loan-capacity and ordered-query outcomes remain preserved.
This diagnostic supplies the missing same-frame attribution before another
production optimization is chosen.

## Hooks and interpretation

The new `diagnostic-script-phases` Cargo feature is default-off. Player forwards
it to script/runtime/profile; existing `profile`, summary, normal dependencies
and public exporter selections do not enable it. The feature emits profile
**version 4**, with `diagnostic_only: true` and
`eligible_for_performance_gate: false`. Feature-off reporting remains version 3.
The existing strict v3 shipping measurement reader refuses v4. Heap collection
remains the existing maintained mode. There is no altered guest source, VM,
deadline, numeric admission, foreign view, undo, structural command, or simulation
algorithm.

`ScriptHost::tick_with_input` starts a diagnostic guard before its ordinary
ScriptTick scope. Fourteen contiguous regions cover initial checks; snapshots;
group setup; API setup; capability setup; numeric undo capture; view exposure;
callbacks; revocation/checks; numeric validation; transaction extraction;
context/scope cleanup; structural publication; and heap observation. The usual
ScriptTick scope supplies its actual duration to that same row when it closes.
The row binds the existing player frame ID, capture generation and ordinal.

Each source's call is observed separately around argument construction and the
existing synchronous `Function::call`, before its ordinary source-profile guard
closes. This includes native capabilities invoked by the callback, not just JS
interpreter execution. Source slots join the existing exact source-ID table.
No extra solver/navigation instrumentation is included in this first slice.

Three nested runtime spans observe `prepare_numeric_globals`, changed-tick
stamping in `NumericColumns::drop`, and `publish_numeric_globals`. They add no
per-row clock reads. Preparation still validates all components and composes
parent-first matrices; publication and change ticks retain their original order.
Major regions partition `trace_ns`. Callback/numeric durations are **subsets**,
not more exclusive time. Analysis may show same-row callback-loop, validation,
and cleanup residuals only after validating the nonoverlap and containment
relations. It must not add nested durations or subtract marginal percentiles.
The trace includes observer work; guard-entry/append and normal profiler boundary
cost also leave a difference from the existing `whole_tick_ns`. No overhead
estimate is subtracted and no diagnostic result replaces unmodified shipping
measurements.

## Bounds and invalidation

Capture reserves at most 4,096 raw rows before ticks begin. Each row stores 14
major durations, three nested durations/counts, 64 source aggregate durations and
counts, identity and validity fields. Compile-time assertions require both raw
and report rows to be at most 1,024 bytes. Raw reservation is at most 4 MiB; the
final typed report copy is separately at most 4 MiB. Source spans retain only
one live start time each, not 64 timestamp pairs per row. The active TLS record
is one bounded row plus timestamps/flags. Final JSON/value construction is outside
ticks and is additional storage; the private collector bounds stdout at 32 MiB.

There is no trace allocation, string formatting, guest value, pointer, ECS
identity, or numeric view retained per tick. Missing/repeated/reversed regions,
wrong thread, nesting, stale capture, counter overflow, missing/duplicate
ScriptTick closure and absent player-frame association invalidate coverage.
The 4,097th trace is counted and makes the report incomplete without growing
storage. It does not alter gameplay or turn a refused diagnostic into a valid
sample. Failure/unwind rows remain visible; a valid proposed run requires every
frame and source call, zero invalid/failed/overflow rows, and exact histogram
reconstruction from the recorded whole-tick durations.

Authored controls cover exact same-frame duration correspondence, complete
partition/nested containment, missing/nested/retired/foreign-thread traces,
unwind, storage overflow and checked arithmetic. An actual ScriptHost control
observes successful writes plus view detachment, then NaN refusal with complete
numeric rollback and detached retained views. These observer controls passed the
focused qualification below. The separately authorized private adapter execution
is recorded after that historical qualification.

## Proposed qualification and one-process protocol

Source review precedes any native command. Proposed focused qualification uses
Rust 1.99, explicit `aarch64-apple-darwin`, locked/offline CI, jobs 2/Rayon 2:
profile diagnostic unit controls; the exact new ScriptHost attribution control;
existing script/runtime correctness checks with the diagnostic feature disabled;
strict scoped Clippy with it enabled and disabled; formatting; and a selected
normal-export feature-graph check proving no diagnostic feature. Existing normal
v3 Python measurement controls plus an explicit v4 refusal are required.

The subsequent diagnostic artifact is **not a public export or distribution**.
A separately reviewed private builder must use the ordinary exporter helpers for
fresh cook, package data, dependency/codegen/symbol checks, with an explicitly
admitted extra diagnostic feature and a diagnostic-only artifact format. The
public exporter and its default graph/report remain unchanged. Match Rust 1.99,
arm64 shipping opt-level 3/fat LTO/abort/one codegen unit, fast4 physics, and
`project,render-input,profile`; add only `diagnostic-script-phases`, never
native-checkpoints or a runtime source compiler. Audit actual target compiler
features and codegen rather than relying on requested flags.

Marker source changes conservative script/runtime compatibility fingerprints.
Fresh cook must retain those actual fingerprints and produce matching host/data;
old envelopes must not be forced to load. Verify all 74 authored inputs, generated
TypeScript/compiler output, exact bytecode payload and cooked scene/assets against
the retained Core package; record any envelope-only identity delta explicitly.

After build and fresh functional qualification, propose **one** maintained-mode
2,156-tick full Core win-route child, with the retained exact input recording,
observation file, 640x480 UI viewport, original 10,000 script attachments and both
source groups. One 45-second child timeout, a bounded supervisor, owned-tree
cleanup, stop on failure, no retries or warmup exclusion. No GPU. Exact complete
logical output and PCM must match the historical control. Startup and functional
work are separate from the sole diagnostic process and must be explicitly listed
in the final reviewed execution plan; this note does not authorize any of them.

Retain all rows and reconstruct the original ScriptTick histogram. Report the
major/nested durations of the *same* tail frames (with deterministic frame-ID tie
breaking), callback source identities, counts and residuals. This identifies a
candidate cost center; it is not a 2 ms, 5%, speedup, or adoption test. Any later
optimization needs correctness review and the original unmodified shipping gate
protocol. The normal route keeps the failed 2 ms result until such evidence exists.

## Focused observer qualification

Reviewed source `e0569895` was followed by an explicit normal-reader refusal
control (`20072ef5`), a test-only redundant-borrow correction (`60ff5538`), and
formatter-only module ordering (`a4c4cc43`). The first strict Clippy failure and
subsequent formatting failure remain retained; neither was a runtime failure.
All production hook logic is unchanged from the reviewed source.

Retained CI controls passed: 22 Python reader tests; 8 diagnostic profile unit
tests; one actual ScriptHost success/NaN rollback/view-detachment test; ordinary
profile 11 tests plus its dedicated allocation process; disabled profile 2 tests;
ordinary runtime 27 and script 151 tests, with one pre-existing diagnostic ignored.
Capture-owning tests ran single-threaded. Both feature-on/off strict all-target
Clippy, final fmt and diff checks pass. Ordinary runtime/script controls were not
repeated after the test-only borrow and module-order corrections.

Actual target compiler records confirm normal player/script/runtime/profile
units omit the diagnostic feature and the script runtime omits its source
compiler and native checkpoints. This CI-only check selected SIMD8 explicitly;
it is **not** shipping fast4 parity evidence. The private adapter instead requires
the retained fast4 exporter selection and actual physics/codegen evidence.

Final observer source freeze is
`~/.codex/incant-evidence/core-script-attribution-main281-001/source-004/freeze.json`
(SHA256 `72e706b1037316c23497f114d82794baf70510c4a637c296f78ce7c21a1b498c`).
The compact qualification receipt is `qualification-final.json` in that directory's
parent (SHA256 `19bc1d7649a8291f76d23b7cf91b645e55addf02364a47a400c70dcb00b70162`),
binding 183 records and 13 immediate test-binary copies. Supervisors 26389 and
19890 retained their initial failures; final 11499 exited zero and was reaped.
The target and builder slot were released. No exporter or player was run during
that focused CI qualification.

The private adapter and its bounded two-stage protocol live in
`tools/probes/core-script-attribution/README.md`. The original proposed plan stays
immutable, including its then-unexecuted status; later supervision receipts
record the distinct grants and actual executions.

## Completed private execution

Final adapter source is `3ed059957b1cb9bcd407a488e8394bda4af8083e`.
The compact [manifest](evidence/core-script-attribution/manifest.json) binds the
source/plan, earlier qualification, all stage receipts, raw diagnostic JSON and
analysis. Binaries, full raw rows and PCM remain in the external directory above;
this is not a self-contained raw archive. Read-only replay, without compiling or
launching native code:

```sh
python3 -B docs/spikes/evidence/core-script-attribution/verify.py --binaries
```

Build001 failed before Rust compilation: the graph parser incorrectly rejected
distinct feature rows for the same package/version (`indexmap 2.14.2` itself has
both `default,std` and `std` rows in the retained baseline). Its attempt, stderr
and terminal supervision are preserved. Source003 compares sets of **distinct
feature rows**, never a feature union; positive duplicate-unit and negative
merged/missing/extra-unit controls passed. The exact four diagnostic receivers
remain the only permitted feature additions. Original attempts were not erased
or rerun under the same output identity.

Separately authorized build002 completed in 261.47 seconds, with Rust 1.99.0,
arm64, opt3/fat LTO/abort/one codegen unit, fast4 physics and the original
`project,render-input,profile` selection plus the diagnostic feature. Actual
target lib/bin compiler records confirm the features and codegen. No source
compiler, native checkpoints or SIMD8 entered the runtime. All 4,353 tracked
source inputs and 74 authored files were checked. Script bytecode, hardening and
map bytes matched the baseline, as did all 28 packaged scene/asset files
(35,365,370 bytes). The real conservative compatibility fingerprint changed;
it was not replaced with the old identity. The copied player SHA256 is
`f40f5a5c4698e243efb050b6ff4240c006c1c302a36c4c0f46616101f058509e`.
The owned temporary target was removed; minimum observed free space was 10.93 GB.

One unprofiled functional child and then one maintained v4 diagnostic child each
completed the full 2,156-tick route. Each had a 45-second child limit and a
90-second stage execution deadline, with separately bounded host probes/cleanup.
Both exited zero and were reaped; neither had a retry, excluded warmup or extra
startup child. Their complete logical gameplay results and 13,798,400 PCM bytes
matched the historical baseline. The diagnostic stdout is 1,788,250 bytes;
stderr is empty. Every row is valid/successful, all source calls and nested regions
are present, and the original whole-tick histogram reconstructs exactly.
Actual raw rows are 832 bytes, for a 3,407,872-byte maximum reservation.

The host was Mac17,9 / Apple M5 Pro, 15 physical CPUs, 51,539,607,552 bytes memory,
arm64 macOS 27.0.1 (26A434), AC power; low-power status was unavailable. Identity
was unchanged before/after both children. Diagnostic load averages were
`[1.3027, 1.5229, 1.4546]` before and `[1.5986, 1.5806, 1.4751]` after. The quiet
receipt records root's attestation that its, texture's and the renderer auditor's
owned workloads were idle; it does not claim independent observation of every
user application. Build71579, functional42313 and diagnostic4165 are terminal
and reaped. No GPU ran in this lane.

## Same-frame observations and source interpretation

The [exact integer observations](evidence/core-script-attribution/observations.json)
retain all totals. The tail is the same 108 largest **whole** ScriptTicks, with
frame-ID tie-breaking. These means are totals divided by their own frame count;
they are not marginal p95 values.

| Region | All 2,156 total (ms) | All mean (us) | Same 108 tail total (ms) | Tail mean (us) |
| --- | ---: | ---: | ---: | ---: |
| Callbacks | 2,435.773 | 1,129.765 | 206.631 | 1,913.247 |
| Numeric validation | 422.103 | 195.780 | 21.112 | 195.481 |
| Scope cleanup | 130.272 | 60.423 | 6.595 | 61.063 |
| Structural/logical publication | 25.138 | 11.659 | 20.614 | 190.868 |
| Undo capture | 26.114 | 12.112 | 1.362 | 12.608 |

Inside callbacks, the actual controller source `01CSSC00000000000000000001`
accounts for 461.904 ms overall and 104.278 ms on those tail frames. The stress
source `01CSSC00000000000000000002` accounts for 1,971.581 ms overall and
102.234 ms on the same frames. These spans surround argument construction and
the real synchronous `Function::call`, including its native capabilities. They
cannot be labeled interpreter-only time. Callbacks are 78.19% of the tail's
264.274 ms whole-tick total. The trace exceeds that same whole total by 37,167 ns;
observer overhead remains included and is not subtracted.

Nested numeric preparation is 422.002 ms overall / 21.107 ms in the tail; it
already belongs to validation. `NumericColumns::validate` calls
`NativeWorld::prepare_numeric_globals`, which still checks all components and
composes parent-first matrices. Nested global publication is 116.787 ms overall /
5.881 ms in the tail; it belongs to scope cleanup, alongside changed-tick stamping
and view/object destruction. These nested amounts must not be added again.
The earlier ordered-query experiment did not establish a gain; this observation
does not reverse that decision or justify weakening numeric validation/rollback.

Of the tail frames, 102 have `tick % 15` equal to 1, 2 or 3. That matches the
unchanged controller's three-enemy staggered `findPath` refresh phases. It is
correlation with the authored schedule, **not** a measured navigation subphase.
`crates/incant_script/src/queries.rs` still admits/meters the query, parses the
request, checks the deadline and materializes its result inside the callback.
The stress source still performs the actual 9,999-row writes and checksums.

The six largest tail publications are frames 1, 508, 2066, 1266, 1842 and 127.
Their 19.434 ms is 94.28% of the tail's publication time; they remain included.
They correspond to initial publication and the route's pickup/win events.
`Undo::apply`/`NativeWorld::apply` stages structural/UI work there; ordinary empty
command frames take the separate retained navigation-motion path. This is not
evidence of a steady 190.868 us publication cost on every tick. No structural
event is hidden or removed from the workload.

## Proposed next change, not implemented here

The smallest concrete candidate is a generation-owned immutable directed search
graph in `incant_nav`, built once by `NavigationMesh::rebuild_with_links` after
the staged geometry and resolved off-mesh links succeed. Currently
`search.rs::search_portals` reconstructs every portal/link state, midpoint and
per-polygon outgoing list for each path query. `NavigationSnapshot` already
copies at most eight mesh/grid **Arc handles**, not their geometry, so replacing
snapshot copying or blaming repeated mesh cloning would target the wrong path.

The cache must belong to the actual mesh value/generation, shared across its
clones rather than keyed globally by a generation number. Preserve portal order,
sorted link order and forward/reverse enumeration, the virtual per-query start
at index zero, exact arithmetic, heap tie-breaking, route/visit counts and
query-specific endpoint/snap/filter semantics. Query costs, parents and heap
remain fresh. No previous path or endpoint result is reused. Unchanged rebuilds
retain the same graph; changed tiles or links replace it only with the complete
staged mesh; failed rebuilds retain both the old graph and old query results.

One admission trap is explicit: the 262,144 directed-portal check currently
occurs **after** the same-polygon shortcut and endpoint lookup. Build-time cache
preparation must retain an over-limit sentinel, not introduce a new earlier
rebuild refusal. The cache is bounded by those 262,144 portals, at most 256
directed links and 32,768 polygons. Exact retained bytes and construction peak
must be charged from the chosen representation; it must not silently increase
navigation limits. Differential controls should cover exact routes/ties/visited
counts, invalid inputs and visit budgets, one-way links, changed links/tiles,
failed staged rebuilds, old retained snapshots and that over-limit shortcut.

Secondary candidates are full-triangle endpoint searches and repeated shortcut
nearest/line-walk scratch allocation. They need independent attribution before
an acceleration structure or reusable scratch changes ordering/lifetimes. Do not
combine them with the first graph cache. A navigation-only probe may diagnose
that candidate, but adoption still requires the unchanged full-Core shipping
route and whole-tick gates. No optimization, speedup, 2 ms, 5%, distribution or
phase-gate result is established by this diagnostic.
