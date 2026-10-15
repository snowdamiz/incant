# Native fixed-tick script timers

The native QuickJS context now provides `clock` and `timers`. The first accepted
update has tick 1; elapsed seconds accumulate the supplied simulation delta.
NativeSession advances this clock only after accepting its input packet. A
standalone ScriptHost advances its own update count without stepping the world.
No wall clock, deferred callback or promise is involved.

`timers.set(id, delayTicks, intervalTicks?)` replaces a named timer;
`timers.cancel(id)` removes it. Due events appear in `timers.events` in stable
ASCII ID order. A delay of one fires on the next accepted update. Repeating
timers advance from their previous deadline without dropping or coalescing ticks.
Cancelling an interval while processing an event stops subsequent events.
The [cooldown example](../../sdk/templates/native-timers/cooldown.ts) handles data
events inside the same synchronous update as its movement commands.

IDs contain 1–64 ASCII letters, digits or `_.:-`. Delays/intervals are positive
u32 integers, the pending schedule holds at most 128 timers, and an update accepts
at most 256 set/cancel calls. Clock and deadline integers never exceed JavaScript's
exact integer range. IDs and numbers are strict primitives without coercion or
guest getter execution. UTF-8 conversion remains in VM-managed storage until its
byte limit is checked; only then can an ID be copied into Rust.

Each update stages a bounded candidate schedule. It commits only after the guest,
deadline/deferred-work checks, numeric validation and native structural batch all
succeed. Even a caught timer error rejects the whole update and permanently fails
the host. World writes and the candidate schedule are discarded together.
Functions expire before diagnostics and cannot reactivate on later updates.
Events and clock values are owned snapshots with immutable own data properties;
guest prototype setters cannot intercept them. No project, command bus or JSON
conversion enters scheduling.

Validation passed: 63 script tests/doctests, 570 ordinary workspace release
tests/doctests, scoped and workspace all-target Clippy, strict SDK typechecking,
43 Python tool tests, and the six-root runtime dependency guard. The unchanged
renderer GPU tests were not rerun. Independent review caught and verified the
oversized-ID copy correction, exercised with an 8,000,000-character string.

The existing source-free shipping proof passed 48 process invocations. It retains
all preceding input, raycast, character, trigger and motion checks; adds timer
clock/order/replacement/cancellation/repetition and stale-function assertions to
the same executable; and adds three invalid/capacity refusal processes. Temporary
authoring, compilation and fresh build inputs were removed before execution.
There is no extra shipping build or CI job for timers.

The macOS arm64 player is 4,123,816 bytes, SHA-256
`f118cd5c9bc465f21809fe58131e3d66ad02b58b557345df53171f6c8e471b60`,
with dSYM UUID `c53cbfc3-33e0-3058-9e94-868960e3bda7`.
[Exact sources, logs, review and results](evidence/native-script-timers-2026-10-11.json).

This schedule is host state, not a complete session save or hot-reload snapshot.
Legacy `initialState`/`onTimer`, serialized payloads and coroutine callbacks remain
unsupported by this native API. Bounded schedule copies and VM allocations occur;
there is no allocation-free, mobile performance or full-SDK claim.

Follow-up review found that the pinned binding library's `CString::as_str` assumes
valid UTF-8 even though QuickJS can retain lone UTF-16 surrogates. Timer IDs now
use only CString byte length before checked `String::to_string` conversion. Lone
high/low and mixed surrogates, and non-ASCII valid pairs, reject atomically.
All 63 script tests/doctests and scoped all-target release Clippy pass after this
correction. The 48-process shipping result above records the earlier source
identity; it is not relabeled as a new executable.
[Correction evidence](evidence/native-timer-surrogates.json).

The later [explicit-state coroutine route](native-coroutines.md) adds separately
bounded cooperative tasks and active-task checkpoint restoration alongside these
timers. It uses named synchronous resume functions and typed state; it does not
add generator/async syntax or change the historical shipping evidence above.
