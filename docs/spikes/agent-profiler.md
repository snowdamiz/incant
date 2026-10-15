# Agent-owned native profiler

The default-off `agent-profiler` host feature adds actual bounded CPU measurements
to disposable native `play_run` sessions. This source slice implements the
Appendix A `profiler.read(frames)` capability as the provider-compatible tool name
`profiler_read`. It does not start Tracy, a network listener, a native window, an
OS recording, or an image capture. Ordinary shipping feature selections are unchanged.

## Use and authority

Build an editor or headless host with `agent-profiler`, then call
`play_run({"seconds":1,"profile":true})` and `profiler_read({"frames":60})`.
The requested frame count must be available; a 60 Hz one-second run supplies 60
ticks. Reads select the latest 1–256 retained successful-candidate ticks without
advancing simulation. A host without the feature refuses profiling and omits the
read tool from provider capabilities. `profile` defaults to false.

The accepted native session owns its completed recording. Results bind a fresh
profile ULID, project ULID, original/current authoring revisions, scene, original
script fingerprint and current session tick. Subsequent authoring edits do not
rewrite old results. Project replacement, cancellation, an unavailable session,
unknown fields and out-of-range reads refuse explicitly. The shared host releases
the command-bus lock while formatting and rechecks cancellation/project identity
before returning. No authoring mutation, transaction, source write or filesystem
path is introduced.

A candidate records only its synchronous `tick_recorded` closure. Its successful
world and recording publish together after the existing cook/source validation,
run/observation checks and final cancellation boundary. Runtime failures, failed
snapshot validation and cancellation retain the preceding accepted world and
recording. A completed assertion failure remains an inspectable completed run,
matching existing play behavior. Stop and a successful unprofiled replacement
retire prior measurements. Cooking, replay lookup, observations and report JSON
are outside the measured closure.

## Isolation and interpretation

The existing process-wide `incant_profile::Capture` remains process-wide and is
never returned through this agent tool. A separate non-Send capture activates an
owner-thread TLS slot only around one tick. The existing phase/source writers
can also contribute to this slot, but other threads, same-thread work outside
the closure, and retired scope guards cannot write into its recording. Nested
owner captures refuse before executing their closure. Both collectors can run
concurrently; the session's private Frame scope does not create a process Frame.

The new report format is `incant-session-profile` version 1, not the normal
shipping profiler's version 3. It reports inclusive wall/current-thread CPU
observations. Nested phases and source times must not be summed. No worker-thread,
GPU, display, heap, requested-allocation or RSS counters are available. CPU clock
unavailability, failed/unwound or outstanding scopes, invalid arithmetic, missing
phases, evicted frames and source exhaustion remain explicit. A zone p95 is the
nearest-rank raw wall duration only when every selected tick succeeded and has a
valid sample for that zone. Empty or invalid coverage yields null, never a
fabricated zero. Instrumentation overhead is included; this is editor/headless
behavior evidence, not shipping performance or the 2 ms gate.

## Bounds and qualification scope

The collector preallocates 256 rows before measured ticks and fixes the source
identity table at 64. Each row retains 26 phase and 64 source metrics. A compile-
time size assertion bounds the retained row storage plus storage structure at
2 MiB. The small Rc allocation/control block and stack guards are additional.
No rows or source labels allocate during a measured tick. Report vectors,
serde_json values and serialized copies allocate only during the explicit read;
they are not included in that retained-storage bound. The complete host response
is independently checked at 256 KiB. The bounded input retains the existing
1 MiB play request, 36,000 ticks/600 simulated seconds and host wall deadline.

Source controls cover cross-thread/outside-scope exclusion, concurrent independent
owners, nested admission, eviction, panic/unwind, retired guards, arithmetic
refusal, source exhaustion and worst-case response size. Real native-host
controls exercise ScriptTick/Frame/source observations, coexistence with process
Capture, failed/cancelled/snapshot-rejected replacements, project binding, Stop
and feature-off refusal. Transport fixtures test unlocked reads and post-read
project/cancellation fences; they are not measured game evidence.

## Qualified producing source

CPU generation 001 qualified source `a91cc387fa11fc5f3d60d3287031245713ee6b48`
against base `69e797ab2ccfaeea79eb207a2a09087d7dceebe8`. All 11 planned
commands plus the compiler identity command returned success and their managed
children were reaped: 21 profile-enabled, 2 profile-disabled, 69 agent-library,
11 native-host-enabled and 9 native-host-disabled harness tests (112 total), plus
the allocator executable. Strict all-target lint passed both feature modes;
the editor host compiled with the opt-in feature. Formatting and conventions
also passed. Fourteen executed test executables are retained externally.

The actual native-host controls prove real measured Frame/ScriptTick/source/audio
observations, process Capture coexistence and retained previous data after rejected
replacement. Cargo's recorded target artifacts also confirm the profile feature
is present only in the enabled native/runtime/script/game-runtime units. These
are CPU behavior controls with no latency threshold; they are not a benchmark.

The bounded run used Rust 1.99 arm64, CI profile, locked/offline dependencies,
two build jobs and two Rayon threads. It finished in 296.05 seconds, with
745,986,199 bytes of sampled target growth and a minimum sampled 87,392,329,728
free bytes. Those are qualification supervision observations, not engine timings.
No failed native qualification attempt or retry occurred.

The [portable packet](evidence/agent-profiler/manifest.json) preserves 45 producing
source inputs, exact source delta, plan/grant/supervisor, raw Cargo/test logs,
compiler identity and copied-binary hashes. Replay with
`python3 docs/spikes/evidence/agent-profiler/verify.py`; optional `--originals`,
`--binaries` and `--git-source` recheck locally retained originals, executable
bytes and historical producer Git objects. Binaries are not embedded. Portable
replay verifies the recorded proof; it does not rerun the native controls.

## Main-333 composition

A separate normal merge preserved the producer branch and joined exact main
`50e168940769ca69ddc11e18e6503c96854dfece`. Qualified composition
`a046dbaa5bc9b6e04a29b5f21385670bfcb4ee98` retains all 19 unaffected
implementation paths byte-for-byte. The only Rust overlap is the additive three-
line skill module/catalog/dispatch join in `incant_agent::lib`; removing those
lines reproduces the qualified producer bytes. One bundled profile-optimize
workflow step now explains the advertised CPU tool and its missing-data limits.

The joined CPU run passed 120 agent/playbook tests, 11 feature-enabled native-host
tests and 9 feature-disabled native-host tests, both strict lint modes, actual
editor feature-enabled compilation, formatting and conventions. These ten
managed commands include the compiler identity step; all returned successfully
and their children were reaped. Seventeen additional test executables are retained.
Unchanged profile-only controls rely on the producing-source run above.

The portable packet now contains both generations: 113 bound source-input rows,
31 external executable identities and separate raw results. The joined run took
88.02 seconds with 7,237,603 bytes of sampled target growth; it is CPU correctness
qualification, not performance evidence. Neither run launched a window, browser,
GPU workload, screen capture or listener.

## Remaining availability boundary

The normal editor artifact still leaves `agent-profiler` disabled. This delivery
therefore requires an explicitly enabled build and does not complete normal-user
profiler availability or the entire Phase 3 profiling capability. A later editor-
only feature selection could make `play_run(profile:true)` available without user
rebuilding, but requires matched inactive-instrumentation overhead evidence and
editor-host qualification first. Public game-export/shipping features must remain
unchanged. No performance, native-window, GPU, visual or shipping acceptance is
claimed here.
