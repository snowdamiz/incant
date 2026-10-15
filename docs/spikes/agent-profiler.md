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

At this source milestone native compilation and execution are pending review.
No performance, native-window, GPU, visual or shipping acceptance is claimed.
