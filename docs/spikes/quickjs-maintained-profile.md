# Maintained QuickJS counters in opt-in native profiling

Source `9c4ba9f7` plus compile/consumer repair `9de22976` and test-only lint
cleanup `1b20ad5a`, based on main272 (`62a3d09f`). Scoped native qualification
is complete; final independent archive review and integration remain pending.
The earlier isolated getter is producer57897285; review862c8924 accepts that
separate diagnostic only. This composition has its own source review and tests.

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

Root source review `fb5428f0` accepts source002, including the independent
consumer findings and their repairs. Root reviewed the exact source003 test-only
pattern-match delta before the final checks. All commands use Rust1.99, CI,
explicit Apple arm64 target, locked/offline builds, jobs2 and Rayon2. Results:

- Native core same-lock counter/full-scan parity:1 pass; async wrapper compiles.
- Profile feature-off:2 tests; summary:11 tests plus the separate allocation harness.
- Script regressions:214 passed and1 diagnostic ignored per physics mode; feature-off213 passed.
- Player library:21 passed per mode; actual child-process profile parity:1 per mode, covering no capture, maintained and detailed reports with identical gameplay/PCM. The changed test reran on source003.
- Cooked script/attachment compatibility:6 passed per mode; actual source-group rollback profile control:1 per mode.
- Strict all-target Clippy for profile/script/player passed in both modes; Tracy option parsing4 passed and strict Tracy lint passed. Browser compilation and authoring-free normal dependency guard passed.
- Python metric/instrumentation controls:17 passed. Four actual CI child reports replay through the unchanged strict parser; these3-frame fixtures are protocol evidence only.
- Vendor inverse-patch audit:383 upstream files,26 patches and20 bundled ABI declarations. Formatting/conventions passed. The production968-package lock changes only the two explicit vendor source/checksum entries against the baseline.

Counts overlap between focused and broader suites and must not be summed as
unique tests. Executable copies are made immediately after each command, before
later target reuse. Native core unit qualification uses exact frozen core/sys
copies in an isolated test workspace: it appends the local sys patch and CI
profile, removes an unused upstream `trybuild` dev edge absent offline, and
retains a separately reconciled lock. Its five extra optional/dev resolutions
are explicitly listed; no production dependency drift is implied.

Four failed commands remain: initial unresolved heap type, invalid workspace
package invocation, missing unused offline test dependency, and the guarded
`unwrap` Clippy finding in the new player test. Earlier pre-freeze vendor
line-ending/audit and formatting diagnostics also remain. None is reported as
a passing generation. Passing deterministic broad suites from source002 remain
valid because source003 changes only the separately rerun player test.

The portable packet embeds3325 records/615 unique payloads (about4.7MB compressed),
43 commands (39 successful and4 historical failures) and54 immediate executable
copy identities. Native binaries/objects remain hash-bound external evidence.
Run `python3 docs/spikes/evidence/quickjs-maintained-profile/verify.py` to replay
portable assertions; add `--external --git-source` for retained originals and
immutable source objects. The verifier never builds or runs native code.

No performance run, mobile execution, or continuous peak claim is included. A
later shipping experiment needs a reviewed protocol for the changed observation
method and a coordinated quiet lease. Lower profiler overhead must not be
reported as guest-loop optimization.

Read-only vendor source audit:

```sh
python3 tools/probes/quickjs-maintained-profile/audit.py
```
