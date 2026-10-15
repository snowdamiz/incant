# Native Metal creation observation: CPU qualification

This private, default-off source milestone adds `diagnostic-compilation` only to
the pinned Metal HAL. Its source003 CPU qualification passed; no native creation
API or GPU was exercised. Native
player/renderer forwarding, cold binding and final sidecar successor below is
now CPU-qualified; actual native creation/hardware observation remains pending. No
public exporter or default feature selection changes here.
The existing presentation and audio evidence is unchanged.

A diagnostic handle is owned by each Metal `Device`, initialized in both adapter
open and raw-device construction. It retains no native device, queue, window,
drawable or pipeline. The host can clone the handle without retaining those
objects. This is intentionally per device rather than `AdapterShared`, which
may be shared by multiple devices.

The six observed method kinds are Naga-derived source-library compilation,
direct MSL source-library compilation, binary MetalLib loading, standard render
PSO creation, mesh PSO creation and compute PSO creation. The binary hook is at
the actual `newLibraryWithData` call, after dispatch-data construction and before
its release. Source strings and descriptors are constructed before timing.
Completion time is sampled before NSError domain/code observation and before
the original error mapping. Original returned values, error mapping, call count,
cache selection and scheduling remain unchanged. Dispatch/return bookkeeping
surrounding the native call remains part of the measured window.

Library function lookup, Naga translation, descriptors, queue waits, deferred
driver work and other APIs are outside these durations. Cumulative attempts,
successes, errors and unwinds are never decremented on resource destruction.
Existing `HalCounters` remain live counts; they are not used as creation history.
Zero calls at these sites cannot prove a shader-hitch-free game.

The diagnostic feature creates a fixed-size per-device header even when capture
has not started, so pre-binding call counts remain visible. Starting capture is
a one-time cold-path operation. It allocates 1..4096 rows with a compile-time
96-byte upper bound per row (393,216 bytes maximum), plus fixed headers and
allocation bookkeeping. Row overflow does not allocate or block rendering; it
increments a visible omitted-row count while cumulative outcomes continue.
Counter/time saturation is explicit. Entry, completion and snapshot comparisons
use `checked_duration_since`; backward values are absent (`None`), with retained
per-device anomaly counts even when a call has no retained row. Pending and
clock-invalid completion remain distinguishable by outcome; a genuine equal-clock
duration is `Some(0)`. Native success/error classification is independent of the
clock. Snapshot clock anomalies also persist into later snapshots. These counts
measure invalid comparisons, not unique calls. Snapshots clone bounded rows only on a
final/cold path; snapshot and JSON allocations are additional, not included in
the row bound. Feature-off builds contain none of these fields, clocks, locks,
allocations or branches.

A host scope captures phase/session/surface/segment/requested-host-step identity
on its creating thread. Nested scopes refuse; work on other threads is recorded
as unscoped. Each call retains its entry context even if it finishes after that
scope ends. Locks are released before the native call. Acquisition-order rows
and completion-order outcomes remain separate, and a snapshot keeps in-flight
rows pending. The collector does not wait, poll GPU, reset counters or imply
that concurrent spans sum to exclusive frame time. Diagnostic locks, clocks,
Arc work and metadata recording add overhead.

Nineteen source controls cover capacity admission, pre-binding calls completing
after capture starts, independent devices, cross-thread exclusion, nested/invalid
scope refusal, immutable entry context, exact-once result/error propagation,
unwinding, bounded overflow, pending snapshots, destruction/recreation,
saturation, handle lifetime, backward completion preserving native error, valid
zero duration, backward entry/snapshot retention, anomalies without retained rows
anomaly-counter saturation and identity exhaustion. They passed in the exact copied collector under edition2021.
Source milestone `052e032e1eb543e3c177603651d1f7b3d5560d36` and its
13-test freeze are retained; this successor fixes its saturating-clock ambiguity.
The native call-site correspondence received independent source review; actual
feature-on/off HAL compilation and strict lint passed. Subsequent hardware qualification must
retain the original shader options and errors rather than use mocks as proof.

The host successor binds immediately after device creation and before the
finite pipeline catalog prewarm, preserving unobserved prefix counts. It joins
main306 window eligibility without changing that policy. See the authored
contract below; this section records source, not a completed live qualification.

The first CPU admission refused an inherited fixture-only lock edge before any
compilation. The second compiled the actual HAL and found ambiguous NSString
borrows and missing diagnostic mappings for the pinned Naga ray stages; neither
attempt executed these controls. Both failures are retained externally. The
compiler successor explicitly borrows NSString, records every pinned stage, and
uses checked compare-exchange identity allocation without the deprecated atomic
API. No native call, option, return value or scheduling behavior changes.

## Retained CPU qualification

Source `dda197ae907a5d34e7c7a69a4041e1ec4f45aee2` passed 19 pure controls,
host/test strict Clippy, and explicit actual-HAL feature-on and feature-off strict
Clippy under Rust1.99, arm64, edition2021, the CI profile, locked/offline,
jobs2/Rayon2. The copied test executable is SHA256
`1262318b5c9911ff9a88d9280c08b1a0e53b0676ccef2e0f2878a2533e4b68e5`.
The native hooks compile against the real pinned Metal API, but no device,
native creation method, window, GPU or listener was executed. The exact fixture
model executes the same collector/tests bytes without adding upstream example
or dev dependencies to the engine.

The [compact receipt](evidence/native-compilation-observer/cpu-qualification.json)
binds all three source generations, the two retained failures, the successful
commands/logs, compiler feature rows and copied test artifact. External originals
remain at its recorded evidence root. All 80 registry package identities match
the producing engine lock. The final supervisor returned0 and reaped every
managed child; the shared build target was released. These are CPU correctness
and compilation results, not complete-game shader-stall evidence. The authored
host successor and actual live observation require separate qualification.


## Host successor (CPU-qualified; hardware observation pending)

The explicit `native-compilation` player feature enables a renderer forwarding
feature and the HAL observer. Normal native-window/profile/presentation features
do not imply it. Trusted host CLI requires both `--compilation-output FILE` and
`--compilation-capacity 1..4096`, plus `--stop-after 1..36000`, and refuses a
lifecycle probe. Unsupported builds refuse those arguments. Package manifests
and scripts cannot supply output authority. The file is create-only, opened
before scene/device startup; an early error leaves an explicitly abandoned
record rather than an invented successful measurement. Offline package execution
refuses the capture options. No normal exporter admission is widened.

`incant_render::compilation::Capture` owns only the observer handle. The explicit
cold constructor attaches it immediately after `request_device`, before the
ordinary one-sample fixed catalog. The handle survives a partial catalog failure
without retaining the renderer, device, native pipeline, window or surface.
Other constructors preserve existing behavior. The collector's small fixed
header and counts exist with the feature enabled even without an active capture;
feature-off builds omit them entirely.

Host RAII scopes distinguish catalog, game startup/restart, background refresh,
and gameplay work. Gameplay starts before surface reservation; deferred or failed
attempts are abandoned, and commit is recorded only after the existing accepted
host-step increment. Catalog rows do not increment host-attempt counters. Refresh commits are
distinct from gameplay; neither proves
that a drawable displayed. Requested host-step identity is independent of world
tick. Focus, size, suspend, resume, restart, surface reconfiguration/recreation,
deferred reservation and reservation error counters fence subsequent scopes.
All contexts are immutable **scope-entry** identities: a recreation inside one
reservation can change the active surface during that scope, and is separately
recorded; it cannot retroactively rewrite prior rows. Calls outside the listed
scopes and calls on other threads remain explicitly unscoped. Background surface
reservation occurs before its refresh scope. Final haptic/error checks and
presentation-drain work are not attributed to gameplay scopes.

Final output is version1 `incant-native-compilation`, capped at4MiB. It records
all bounded rows, pending/outcome/clock anomalies, unbound prefix and cumulative
counts, fixed host-scope commit/abandon counters, boundary counts and the final
reservation counters. Source/binary/package identity must be bound externally;
the sidecar does not infer it. Snapshot and JSON allocations are additional to
the393216-byte row maximum. Final serialization/flush/sync happens after game
and audio shutdown, before the existing bounded presentation callback drain;
there is no per-frame serialization, polling or GPU wait. A sidecar failure
cannot replace an earlier gameplay/window error, and turns an otherwise
successful result into an error. It cannot claim the final process exit status.

CPU controls cover create-only/no-effect refusal, early-failure output,
explicit commit versus unwind, distinct boundaries, saturated counters, capped
writes, original-error precedence, inert absent capture, trusted CLI/package
forwarding, and cold observer handle ownership. The19 qualified collector
controls are included as exact source; one nested tuple expression and panic
imports were made edition-neutral for the2024 host tests and2021 HAL. These are
format-compatible mechanical changes, not altered collector semantics. The full
upstream patch is regenerated; prior CPU source identities remain retained.
The [host CPU receipt](evidence/native-compilation-observer/host-cpu-qualification.json)
binds source `6fa2ba2540280d7bb088d4dd03ec2426779bd8b1`, both failures, every
managed command/log, actual feature rows and all four retained test binaries.
The142 observer-enabled player tests passed, including19 collector and8 host
controls. Three renderer cold-binding tests passed, including catalog unwind and
handle retention. All97 ordinary-window/profile player tests passed, and strict
all-target player/renderer Clippy passed with both observers enabled and with
both disabled. These are mode-specific executions, not242 unique test cases.
Actual HAL dependencies were compiled in the expected on/off configurations.

The first attempt compiled but found one new test expectation wrong: existing
closed package manifests reject an injected output field instead of ignoring it.
Only that test was corrected. The second passed all142 player controls, then
standalone renderer compilation exposed cached pre-halo dependency metadata
that disagreed with frozen source. The metadata, relative dep-info and all35
fingerprint directories were copied and bound. Only the two affected packages'
fingerprints were invalidated; source and existing artifacts were preserved.
The four remaining commands then passed on unchanged source. No failing input,
assertion or compiler diagnostic was hidden.

All owned children returned after cleanup and the target/CPU slot was released.
Across all attempts the supervisor recorded250.44s, minimum sampled free space
91,935,801,344 bytes, and472,377,454 bytes cumulative target growth, within the
6GiB floor and2GiB growth bound. Immediate executable copies were retained before
release. Source and CPU evidence do not establish a live native API duration,
complete-game shader-stall pass or display performance result. A fresh private
shipping build and separate owned-window observation still require review.
