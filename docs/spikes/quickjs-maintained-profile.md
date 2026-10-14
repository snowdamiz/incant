# Maintained QuickJS counters in opt-in native profiling

Source candidate on main272 (`62a3d09f`); **not yet compiled, executed, accepted
for integration or measured**. The earlier isolated getter is producer57897285;
its final independent review862c8924 accepts functional counter parity only.
This production composition needs its own source and execution reviews.

The change reduces profiler observation work. It does not optimize guest loops,
change the Core10k workload, relax the 2ms/5% criteria, or establish a shipping
performance result. Failed cursor/inspector and basis candidates remain excluded.

## Native patch and compatibility

Two exact published rquickjs0.14 crates are path-patched: core and sys. The outer
rquickjs wrapper stays registry-pinned. Each vendor directory retains its upstream
file hashes, archive identity, MIT license and full unified patch. The sys getter
copies three existing native `size_t` fields into caller-owned scalar outputs;
core exposes sync/async safe methods under its existing exclusive runtime guard.
The production API omits the prior diagnostic's combined full-scan helper.

The C allocator, small-block arena, GC, memory limit, bytecode version and execution
backend are unchanged. ScriptHost still enforces32MiB and its existing deadline.
No private struct layout is cast from Rust. All20 bundled declarations use each
binding's actual `size_t`; mobile's existing bindgen route reads the additive
header. Target compilation remains separate from this source coverage.

Script build identity hashes the complete retained vendor input trees using
logical relative paths and bytes, in addition to existing lock/wrapper/host pins.
It is conservative compatibility, not authentication. Existing cooked artifacts
must be recooked after this host/dependency identity changes. No executable
artifact admission is widened. Upgrades must re-audit the accessor's field meaning,
ABI declarations and lock ownership alongside ordinary upstream security updates.

## Report protocol

`profile.version` is3. `quickjs_heap.version` is1, with `mode` equal to
`maintained` or `detailed` and `sampling` equal to `script_tick_end_per_vm`.
Each host tick attempt that reaches the existing telemetry hook observes its VM
once after context/loan cleanup, outside the guest deadline but inside ScriptTick.
Pre-hook refusals produce no heap observation. Multiple VM samples are not summed
into one live-heap estimate. The default non-profile build does not call the hook.

`latest` is null until a valid sample. It contains `allocated_bytes`,
`allocation_count`, `limit_bytes`, and nullable `estimated_used_bytes`.
Maintained mode reads logical native allocator counters without object enumeration;
`estimated_used_bytes` is null. Detailed mode performs the original full scan and
retains its estimated used-size meaning. A zero native limit means unlimited;
Incant's host sets32MiB. Logical counts/bytes are not backing arena reservation,
Rust requested allocation, semantic live heap or RSS. Full-scan estimated size can
include borrowed buffers and has upstream estimation limitations.

`samples`, `invalid_samples` and `detailed_samples` make observed coverage explicit.
Failed integer conversion or a mode/detail mismatch counts as invalid instead of
publishing zero. `sampled_peak_allocated_bytes`, `sampled_peak_allocation_count`
and `sampled_peak_estimated_used_bytes` are nullable maxima of valid observations.
They can miss every transient allocation between tick boundaries; none is a
continuous peak. An observation ticket is bound to its capture generation before
acquiring the VM lock and cannot publish into a replacement capture.

Embedded hosts select `Capture::start_with_heap_mode(HeapMode::Detailed)`.
Standalone summary and Tracy keep their existing options; adding
`--profile-heap-detail` requests detail. The modifier alone or repeated refuses
before capture. New Tracy labels explicitly say sampled logical/estimated bytes.
The existing native-cpu-profile diagnostic requests detail to keep its original
full-scan intent. No process capture transport or shutdown behavior changes.

## Measurement consumers

`mac_performance` reads retained profilev2 as `legacy_full_scan` and profilev3 with
its explicit heap mode. New measurement envelopes are version2 and include
`match.profile_instrumentation`; loading rederives this identity from the hash-bound
raw report. Legacy measurementv1 remains readable only with actual profilev2.
Comparisons refuse different instrumentation generations or modes even when the
frame histogram/workload matches. Equivalent legacyv2 reports can still compare
through old/new envelope formats. This prevents treating lower observer overhead
as a matched improvement in guest execution. Wasm/TS descriptive pairing also
requires the same instrumentation identity. All existing phase selection,
quantile intervals,2ms and5% policies remain unchanged.

## Authored controls and required qualification

Source controls cover safe getter/full-scan equality under one native guard,
independent VMs, allocation/reallocation/GC and unchanged OOM limits; empty/invalid/
retired heap observations; actual successful and rolled-back ScriptHost ticks in
both modes; public player PCM/gameplay equality across no capture, maintained and
detailed capture; malformed CLI modifiers; legacy/new raw consumer binding and
cross-mode/cross-generation refusal. The isolated diagnostic's larger external/
owned-buffer and32-observation matrix remains historical evidence, not a claimed
execution of this composition.

Before integration: source review; exact vendor audit and lock graph comparison;
focused native profile/script/player tests and strict lint, both physics modes
where applicable; default feature-off controls; async wrapper feature compile;
supported available ABI/build checks; consumer Python controls; formatter and
conventions. Retain exact binaries and failures. No new performance run is
currently authorized. A later shipping experiment needs an explicitly reviewed
protocol for its changed observation method and a coordinated quiet lease.

Read-only vendor source audit:

```sh
python3 tools/probes/quickjs-maintained-profile/audit.py
```
