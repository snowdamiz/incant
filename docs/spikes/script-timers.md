# Fixed-tick script timers

Phase 1's TypeScript SDK now exposes a saved game clock, one-shot timers,
repeating timers and cancellation. All callbacks use the existing sandbox and
simulation command bus. No wall-clock, filesystem, network or shell capability
is added. Revision 3's broader scripting, navigation, 2D, localization and
haptics requirements remain open.

## Observable behavior

`api.clock()` returns a fresh `{tick, elapsed_seconds}` copy. The first callback
sees tick 1, and a restored game continues its saved clock. Deadlines use fixed
ticks, not wall-clock time: `delay_ticks: 1` fires on the next successful tick.
`api.setTimer({id, delay_ticks, interval_ticks?, payload?})` creates or replaces
an ID. Omit `interval_ticks` for a one-shot; a repeating timer reschedules from
its previous deadline. Payloads are copied JSON data. Mutating the request or
one delivered payload does not alter a future delivery.

Implement `onTimer(api, event, state)` alongside `update(api, dt, state)`.
Due callbacks run before update, ordered by `(scheduled_tick, id)`. All see the
same beginning-of-callback-phase world snapshot and share state, queued commands,
logs and the execution budget. Their commands affect the next physics tick.
`cancelTimer(id)` removes a pending timer; cancelling an absent ID succeeds.
Cancellation or replacement in one due callback suppresses a later callback of
that ID in the same tick. It cannot undo a callback already executed. A timer
created by any callback cannot run recursively in the same tick.

The host stages clock/deadlines and validates the entire result before committing
commands, behavior state, timers and logs. A thrown error, expired deadline,
malformed timer or invalid command cannot commit a prefix. A standalone
`ScriptHost` can retry after a fixed hot reload; `PlaySession` instead stays
terminal after a failed tick because physics may have advanced. It cannot save
that failed state. The CLI preserves earlier committed log lines but publishes
neither a completed report nor a new game save.

Behavior callbacks are synchronous. Returned Promises, thenables and generator
iterators now fail explicitly, including synchronous prefixes executed before an
`await`. The strict SDK requires an `undefined` return to catch these signatures.
Detached asynchronous jobs and VM globals are not durable game state. General
coroutines, async job scheduling and serializable continuations are not delivered
by this increment and remain Phase 1 work.

## Saved state and limits

Game-save version 2 includes the schedule, clock, future deadlines, repeat periods
and payloads. It stores no functions or VM handles. Restore validates the clock
against the outer save and requires a handler when pending timers exist. Old
version 1 saves restore the clock with an empty schedule. Removing `onTimer`
during hot reload fails without changing the active host when timers are pending;
compatible hot reload retains their deadlines and payloads.

The schedule is bounded to 128 pending timers, 256 scheduling/cancellation actions
per tick, 16 KiB per JSON payload and 64 KiB total serialized schedule. IDs contain
1–64 ASCII letters/digits or `_.:-`. Delays and repeat periods are positive u32
integers. Deadlines must be future ticks within JavaScript's exact integer range.
The existing behavior-state, command, log, native-query, VM memory and execution
limits still apply. Published schemas and generated TS bindings describe the
wire data; semantic validation remains authoritative.

## Verification

Seven script tests cover ordering, copying, repeat/replace/cancel, clock copying,
callback transaction failure and retry, exact saved continuity, version 1 loading,
malformed schedules, hot reload, bounded requests and explicit async rejection.
A headless subprocess test verifies failed callbacks cannot publish a save or
completed report and retain only prior committed logs.

The public `tools/probes/game-timers.py` probe creates a project and durable
history through RPC and compiles a strictly typechecked TypeScript behavior.
It runs ten ticks, saves pending deadlines, resumes twenty ticks in another
process, saves again and reopens at tick 30. Movement and delayed entity creation
run through commands. Final runtime/behavior state and the continued log suffix
equal uninterrupted play exactly; same-tick cancellation suppresses a later
callback. Both authored file hashes remain unchanged. No GPU or native device
is opened.

The workspace passes 203 tests before the final Rust command-limit guard test;
the script suite passes again with that test, bringing the distinct total to 204.
The guard verifies that untrusted code replacing the JavaScript wrapper cannot
bypass the 10,000-command limit. Workspace Clippy, 315 UI tests/build, five Python
tool tests, generated contracts, strict TypeScript and the native release
build/package pass. Existing public save and input/character/assertion probes
also pass without rendering. Hosted verification is pending.

Machine-readable results are in
[script-timers-2026-10-09.json](evidence/script-timers-2026-10-09.json).
The director's computer-use/capture pause remains active; this
increment needs no visual changes. Source and desktop CI run the same public
probe, with hosted rendering checks unchanged.

## Hosted integration

All three hosted checks passed at `54f85a1`: source run 38021712982 and
Windows/Linux desktop run 38021713409. PR #31 merged as `93bc5ed`; its tree
`80d409788e696a56ecbeab886f2597e7407c2cac` exactly matches the reviewed head.
