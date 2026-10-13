# Opt-in native CPU profiling

This slice adds connected system/script zones and bounded final player reports.
Source and execution reviews are accepted; [main194 integration checks](native-cpu-profiler-main194.md)
pass with independent integration review pending. It does not approve the full PLAN profiler, phone/network capture or any
performance gate. Renderer GPU timestamps remain a separately owned feature.

A default build has zero-sized no-op scopes and no profiler clock, allocator
wrapper, client initialization or network service. Build the ordinary player with
`--features profile`, then pass `--profile` to append a typed `profile` member to
its existing final JSON report. Unprofiled invocations preserve the prior report.
The option applies to the ordinary offline route; window and build-info requests
refuse it. Embedded hosts may use that same summary route. There is no tick-time
JSON/world projection and no gameplay/project configuration or mutation path.
Failed simulation still emits no success report or PCM. Failed observation reports
retain their existing diagnostic/publication contract.

One process capture may be active. Fixed aggregates cover 25 compile-time phase
labels and at most 64 numeric source identities; excess source samples increment a
dropped counter. Sources with multiple instances run once per existing source
group and receive one profile row per source, not per entity. A generation fence
prevents scopes outstanding at retirement from writing into a new capture.
This is process-wide instrumentation, including worker-thread system execution;
it does not pretend to isolate simultaneous worlds in the same process.
Scope durations are inclusive wall and current-thread CPU measurements. Nested
zones overlap and cannot be summed as frame time. Missing CPU samples are counted,
not fabricated. The player.frame zone covers accepted game tick work, while
output serialization/file I/O are outside this frame zone; the restart boundary
has its own static zone.
Instrumentation overhead is real and not a reference-device budget result.

The executable's feature-gated CountingAllocator delegates every allocation,
reallocation and deallocation to the existing System allocator with unchanged
pointer/layout/null semantics. It retains fixed atomics, no pointer table. Counts
and requested byte totals are interval deltas; live bytes and the monotonic peak
are process-lifetime requested Rust sizes. Samples are not a transactional view
of other allocating threads, and overflow is marked by a saturation flag. Failed
reallocation preserves the original allocation/accounting. An embedded host that
does not install the wrapper reports installed=false; zero counters do not imply
zero allocations. C/native heaps, allocator overhead, GPU memory and RSS are not
included. There is no allocation call-stack capture in this slice.

QuickJS's own memory_usage is sampled only when profiling is active, outside JS
loans and after guest budget checks. Its latest/peak measurements describe an
individual sampled VM, not the sum of concurrently alive VMs and not the Rust
allocator. The existing 32 MiB VM cap and execution budget remain unchanged. Heap
walking adds diagnostic overhead. Static Wasm and development modules receive
separate execution zones; their private linear heaps are not labeled QuickJS.

## Explicit standalone Tracy transport

`--features profile-tracy` adds exactly pinned tracy-client 0.18.5 and
tracy-client-sys 0.29.0 (Tracy 0.14.0 protocol). Only explicit standalone CLI
`--profile-tracy` starts the client. Embedded/library/mobile `run` rejects that
transport option. The adapter enables manual lifetime, on-demand and localhost;
broadcast, code transfer, sampling, system tracing and crash handlers stay off.
It uses fixed source locations with no call stacks or authored strings, and
numeric source slots associated with the bounded final identity table.

The pinned client deliberately has no safe shutdown/restart API. Its loopback
transport therefore lasts until the opted-in standalone process exits. Summary
retirement does not pretend to stop that service. No unsafe shutdown is called.
Tracy's own C transport buffers are not covered by the finite summary-storage
claim. A local capture requires a matching explicitly launched capture process;
no listener starts in a default build or from an ordinary --profile summary.

The ordinary trusted desktop exporter accepts `--profiling summary|tracy` only
for its headless route. Omission preserves the existing build and receipt shape.
The receipt binds the selected normal dependency graph and actual target compiler
units, including exact Tracy versions/features; build-host units cannot stand in
for target units. Ambient CARGO_FEATURE_* and C++ compiler aliases are refused by presence,
including empty values; nonempty Tracy library/C++ flag overrides are refused
before project I/O. The startup smoke run does not turn on capture. The copied player
still requires its explicit runtime option, and no authored project property can
select either build mode.

## Validation and retained failures

The broad affected suite passes 495 tests in 62 groups; the default player passes
19 tests. All-target Clippy, default/summary wasm compilation, 23 focused Python
export/graph tests, four actual small process-lifecycle tests, formatting,
conventions and the lean runtime graph checks pass. Counts overlap focused Rust
controls and are not added into a misleading unique-test total.

Two fresh ordinary exports compile summary and Tracy headless players from the
same two-source mathematical project, authored through two public atomic
transactions. Both exports independently pass selected normal-graph and actual
target compiler-unit audits, and startup after private build storage removal.
Source authoring and exported JavaScript debug maps are deleted before gameplay.
The same source groups see the shared native bulk world: both apply their speed,
so both owners move by (6 + 12) * ticks / 60. This fixture has no sound source;
its exact finite silence tests PCM output preservation, not sound-content load.

Five fresh 3,600-tick runs compare summary/Tracy-capable players with no profiling,
summary reporting, and explicit local capture. All logical reports, logs, final
observations and PCM bytes agree; each active summary contains 3,600 calls for
each of the two numeric source identities, 3,600 QuickJS samples and installed
Rust requested-allocation counters. The default feature graph/tests are checked
separately: the shipped inactive controls use those same capable binaries without
a runtime profiling option. No inactive/summary run creates a network endpoint.

The explicit Tracy 0.14.0 capture contains 20 actual CPU zone types and 3,447
samples for each of four QuickJS/Rust memory/allocation plots. Capture starts only
after lsof sees the exact owned player's localhost listener. It is partial and
on-demand: early frames are not captured, while the bounded final summary still
covers the complete run. The trace and summary contain no fixture-authored log
strings. Five exact option/guest refusals produce no success/PCM publication;
failed guest diagnostics agree with profiling disabled and enabled. All recorded
player/capture/authoring child processes are reaped; no listener remains.

Failures are preserved rather than overwritten: the first allocation assertion
included unrelated parallel test-harness allocations and moved to a dedicated
process; feature-unification admission and a default-build lint were corrected;
an inherited Cargo feature environment could change pinned Tracy C++ flags and
is now refused. Probe authoring initially attempted two source writes in one
transaction, then its oracle initially assumed source-local bulk chunks. Both
failed before export. The first completed export/capture run passed all game
comparisons but the short player exited during the receiver's initial reconnect
handshake. The retained same-package retry launches the receiver on the observed
owned listener, without extending or pausing gameplay. Harness lifecycle review
also replaced unread PIPEs with retained files and the outer killable exporter
wrapper with the public in-process exporter, preserving its managed-child cleanup.

[Independent review](evidence/native-cpu-profiling-review.json) accepts this scope.
The [manifest](evidence/native-cpu-profiling.json) retains exact source freezes,
reviews, raw/lossless logs, process receipts, trace/CSV, package identities and an
assertion-only re-audit script. Original failures remain separately identified.
No GUI, GPU, native-window, phone/device capture, allocation call stacks, RSS,
network-device profiler, performance gain or full PLAN profiler gate is claimed.
