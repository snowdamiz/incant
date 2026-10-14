# Native Metal creation observation: source milestone

This private, default-off source milestone adds `diagnostic-compilation` only to
the pinned Metal HAL. It has not been compiled or exercised on a GPU. Native
player/renderer feature forwarding, cold binding and final sidecar output are
still pending; no public exporter or default feature selection changes here.
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

Eighteen source controls cover capacity admission, pre-binding calls completing
after capture starts, independent devices, cross-thread exclusion, nested/invalid
scope refusal, immutable entry context, exact-once result/error propagation,
unwinding, bounded overflow, pending snapshots, destruction/recreation,
saturation, handle lifetime, backward completion preserving native error, valid
zero duration, backward entry/snapshot retention, anomalies without retained rows
and anomaly-counter saturation. They are authored only, not passing evidence.
Source milestone `052e032e1eb543e3c177603651d1f7b3d5560d36` and its
13-test freeze are retained; this successor fixes its saturating-clock ambiguity.
The native call-site correspondence still needs independent source review and
an actual feature-on CPU compile/lint. Subsequent hardware qualification must
retain the original shader options and errors rather than use mocks as proof.

The next host step is to bind immediately after device creation and before the
finite pipeline catalog prewarm, preserving unobserved prefix counts. Host
wiring will follow the separate window-eligibility integration to avoid a shared
`window/mod.rs` conflict. It will use an explicit trusted host option with a
create-only output path and finite capture bound. No project/script capability,
per-frame snapshot, new rendering wait or performance verdict is introduced.
