# Opt-in native CPU profiling

This in-progress slice adds connected system/script zones and bounded final player
reports. It does not approve the full PLAN profiler, phone/network capture or any
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
output serialization/file I/O/restart boundary work has separate chronology.
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
for target units. Nonempty Tracy library/C++ override flags are refused before
project I/O. The startup smoke run does not turn on capture. The copied player
still requires its explicit runtime option, and no authored project property can
select either build mode.

## Validation boundary

Initial aggregate/lifetime/quota tests and the dedicated allocation process pass.
The first allocation assertion ran in the parallel Rust test harness and correctly
observed unrelated harness allocations; its failure is retained. Exact allocation
controls now run in a dedicated process, without weakening their expected sizes.
Actual ScriptHost rollback/source/heap and ordinary player report/PCM parity
checks pass. An existing refusal regression exposed dependency-feature unification
accepting a player option without its report feature; explicit player admission
now passes both summary and transport unification controls. CLI, selected graph,
compiler-unit and override refusal tests pass. Clippy, broad affected/default/web
checks, fresh shipping execution and a real local Tracy capture remain pending. Source is frozen for independent review before broad
shipping/capture work. No GPU, visual, device or phone execution is claimed.
