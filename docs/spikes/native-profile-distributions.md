# Bounded per-frame CPU wall distributions

The native summary report advances to version 2. It keeps the existing inclusive
zone/source totals and adds `frame_wall` protocol version 1. This is opt-in CPU
instrumentation for the PLAN revision 9 Mac gate; it does not itself establish
shipping performance, 10,000-entity coverage, 60 fps, GPU cost or pipeline hitches.
The default feature graph and default no-op instrumentation are unchanged.

`incant_profile::frame` encloses exactly one synchronous player simulation attempt.
The headless and window simulation paths share the existing player wrapper;
event-loop waits, redraw-only presentation, output serialization and restart work
remain outside. Within a successful frame, sequential scopes of each fixed phase
sum into one wall-time sample. `script.tick` is the existing full ScriptHost tick
scope, covering all source groups and command application. Source percentiles are
never summed, and distinct nested phases remain inclusive, not additive frame cost.
An empty or suspended frame with no script scope is missing, not a fabricated zero.

A capture fallibly allocates one flat 2,080,208-byte histogram payload directly on
the heap: 26 phases times 10,001 u64 bins. Collection allocates no dynamic storage;
per-frame counters are fixed arrays. Report creation explicitly copies bounded
rows/bins after sampling. These byte counts describe requested payload only, not
allocator overhead, OS RSS, other profiling storage or Tracy transport storage.
Capture allocation failure returns `Storage` before publishing active collection.

`frame_wall` exports `version`, `unit` (`ns`), `bucket_width_ns` (10,000), `max_ns`
(100,000,000), `attempted_frames`, `successful_frames`, `failed_frames`,
`invalid_frames`, `active_frames`, `saturated`, and `phases`. Each phase row exports
`name`, `samples`, `missing_frames`, `invalid_frames`, `overflow_samples`,
`p95_lower_ns`, `p95_upper_ns`, and `counts` (exactly 10,001 integer counts).
Bin zero means exactly zero nanoseconds. Bin i > 0 is the closed integer interval
`[(i-1)*10,000+1, i*10,000]`. The nearest rank is `ceil(95*N/100)`; both endpoints
of its bin are reported. Exactly 2 ms is a bin boundary. Comparing two upper
bounds as exact observations is invalid: regression assessment must compare
intervals and retain inconclusive outcomes when they overlap a threshold.

Both percentile endpoints are null unless every attempted frame succeeded and
closed, there were no frame collisions or saturated counters, and that phase has
one valid in-range sample per successful frame with no missing, invalid or overflow
samples. Greater than 100 ms is explicit overflow, never clipped into a passing bin.
Same-phase recursion/overlap, an unfinished scope, a scope crossing the frame start,
and other-thread scopes invalidate the affected row. Concurrent/reentrant frame
attempts invalidate the participating frame coverage. Cross-thread system totals
remain in the preexisting process-wide inclusive zone report; these histograms do
not claim parallel critical-path reconstruction. A capture generation fence keeps
outstanding frame guards/scopes from writing into a replacement capture. Failed
results and unwinds retain the original result/unwind while making p95 unavailable.

The standalone runner must validate the exact protocol, histogram arithmetic,
requested tick coverage and shipping/workload/hardware identities. A matched
baseline and actual elapsed measurements are separate evidence; there is no
performance-speedup claim from these correctness tests.

Focused controls cover nearest-rank boundaries, exact zero and 2 ms, two sequential
script costs versus nested source costs, empty/failed/unwound/active frames,
recursion, unfinished and preexisting scopes, other-thread scopes, concurrent frames,
saturation and retired-capture writes. The dedicated CountingAllocator process
checks zero collection allocations after capture initialization. The real two-source
ScriptHost control compares aggregate frame coverage with two distinct source rows
and preserves guest-failure rollback. Qualification results and exact binaries are
retained separately from this contract.

## Scoped qualification

All checks use `./tools/cargo`, `--offline --locked`, explicit
`--target aarch64-apple-darwin`, `--profile ci`, two jobs and `RAYON_NUM_THREADS=2`.
The 17 named CPU cases comprise five histogram units, five summary cases, the
actual two-source ScriptHost test, four player wrapper/option tests, the real
three-tick CLI test, and the default no-op test. The dedicated allocation process
also passes. Four strict Clippy scopes pass: all profiler features/targets, player
library, ScriptHost profile integration test and player CLI integration test.
Summary-feature browser compilation, the empty default normal-dependency graph,
formatting, conventions and whitespace checks pass. No failures occurred in these
retained generations. No full workspace run or duplicated solver suites were needed.

The CLI artifact is a CI-profile, one-entity, three-tick default embedded fixture.
Its actual child report has three script/frame samples and bins containing each
corresponding maximum, as nearest-rank p95 requires for three samples. Inactive
and profiled logical output/PCM match, and guest failure publishes neither success
JSON nor PCM. The JSON and immediate executable copy are retained for the separate
Mac runner's parser check. They are not a shipping-profile or performance gate.

Source001 contains the production implementation and focused controls. Source002
adds only this contract and the CLI protocol/output control; every production byte
is unchanged. Final documentation records outcomes without relabeling the executed
source generations. The [portable evidence manifest](evidence/native-profile-distributions/manifest.json)
binds raw logs, source copies, command arguments, immediate executable copies and
the actual CLI report. Run its adjacent `verify.py --local-binaries` to recheck.
