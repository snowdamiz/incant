# Correlated full-Core script attribution (source proposal)

This private diagnostic starts from main281
`dfac86cb17e9a1dd71d63207d779b3836e7ad6ae`. Main282 changes documentation only.
The rejected ordered-query implementation is absent. No native build, execution,
timing, performance improvement, or gate result is claimed for this source.

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
ticks and is additional storage; a future collector will bound stdout at 32 MiB.

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
numeric rollback and detached retained views. These controls have **not run**.

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
