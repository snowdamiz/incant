# Bounded script failure diagnostics

Gameplay exceptions previously collapsed into `script execution failed or exceeded
budget`. `ScriptError` now distinguishes an expired wall-clock deadline, a native
QuickJS allocation failure, a JavaScript exception during initialization or a tick,
and a native engine failure for which no JavaScript diagnostic is available.

The VM exception is read while its context is alive. Only primitive string values
and Error name/message fields are reported. Arbitrary thrown objects are never
stringified; stack/source dumps and arbitrary `toString` calls are excluded.
Messages preserve UTF-8, remove control characters except newline/tab, and stop at
1024 bytes plus an ellipsis. Error getters remain untrusted JavaScript under the
same interrupt deadline. Secondary getter exceptions are discarded without
recursively describing them, and deadline expiry takes precedence over text.

Both public `play` and `script` commands include the absolute failing tick. A game
saved at tick 3 and failing after its second resumed update reports tick 5.
Initialization failures identify initialization rather than pretending a tick ran.
A failed update still cannot publish script state, timers, commands or logs, and
the play session cannot publish a new save. Previously committed log entries stay
available. Underlying engine and schema validation errors retain their typed
sources.

At source `b2ca771`, five added behavior tests cover syntax/type exceptions,
failed hot reload, bounded Unicode/control text, a hostile `toString`, throwing
and infinite-loop Error getters, absolute tick reporting across a process restart,
unchanged authoring files and saves, committed-only logs, and initialization versus
timeout CLI output. The existing infinite-loop and native-query expiry tests now
assert the deadline category. All 294 Rust workspace tests and Clippy pass locally;
the steering correction is integrated at `b1dcdd7` and all 294 tests and Clippy
pass again. Public steering and off-mesh save/reopen workflows pass, as does the
updated unsigned native package. The unchanged UI/GPU/contracts also pass at the
initial diagnostics source: 315 UI, 40 GPU and five tool tests. See the
[evidence record](evidence/script-diagnostics-2026-10-10.json). Hosted checks and
reviewed dependency merges remain pending.

The default 50 ms wall-clock budget, 32 MiB VM limit, shared query limits and
sandbox capabilities are unchanged. Host sleep or CPU contention can still expire
a healthy script's deadline. This increment makes that failure distinguishable;
it does not solve scheduling robustness, provide CPU-time accounting, claim
source-mapped stack traces, or classify every JavaScript memory error as a native
allocation failure. No visual surface changes and no phase-gate approval.


At `bb8b597`, the near-head-on steering correction passes all 296 Rust tests and
Clippy, both public steering/off-mesh repeat-and-save workflows and unsigned
native packaging. Its final rendered review and updated hosted checks remain
pending. The diagnostics runtime still uses wall-clock execution limits; the
separate CPU-budget increment changes that accounting.

The subsequent [CPU-budget increment](script-cpu-budget.md) changes execution
accounting. The wall-clock limitations above describe this diagnostics increment
and its original recorded binaries.
