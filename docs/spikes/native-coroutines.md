# Native explicit-state coroutines

Native TypeScript behaviors can now start bounded cooperative tasks, wait for
accepted simulation ticks, and save/restore every active task's continuation.
The continuation is explicit typed data plus a captured routine name. This is
not JavaScript generator, `async` or Promise syntax. It runs in the existing
native world and tick transaction, without a document/command-bus tick path or
per-tick JSON conversion.

## Authoring contract

Declare `coroutines: { name: { resume(context, task) { ... } } }` on a
`NativeBehavior`. The engine captures those functions at initialization; later
changes to the declaration do not replace them. `update` remains mandatory,
synchronous and returns `undefined`.

`context.coroutines.start(id, routine, state, entity?)` schedules a new task for
the next accepted update. `cancel(id)` removes it; cancelling an absent ID is
allowed and charged. Starting an existing ID refuses; cancel then start replaces
it. Names and IDs contain 1–64 ASCII letters, digits or `_.:-`.

`resume` receives fresh tick capabilities and an owned, deeply frozen task
`{ id, state, entity? }`. Return `{ kind: 'done' }`, or
`{ kind: 'wait', delayTicks: positiveU32, state: nextState }`. A wait is measured
from the current accepted tick. Delays and clocks must remain within JavaScript's
exact integer range. State supports null, booleans, finite numbers, strings,
arrays and string-keyed objects; functions, VM objects and borrowed views are
not continuations. Runtime validation supplements the generated structural types.

Ordinary `NativeContext.coroutines` is optional because programs without a
catalog do not expose it. Narrow that capability before starting a task.
`NativeCoroutineContext` guarantees it within `resume`. The
[strict pulse template](../../sdk/templates/native-coroutines/pulse.ts) starts
at tick 1, adds one unit at tick 2, waits two ticks, adds two more at tick 4 and
finishes. The corresponding cooked native control verifies the resulting state.

Sources execute in canonical source-ID order. Each source's update runs before
its due tasks, which resume in task-ID order, at most once per accepted update.
A new or replaced task cannot run again in the same tick. Cancellation or
replacement during a callback takes precedence over that callback's return;
the return must still be valid. There is no cancellation callback or generator
cleanup execution.

An optional owner must be a live authored attachment of that source. Owner
retirement cancels its tasks; losing the source's last attachment cancels its
remaining tasks. Reusing an entity ID does not restore an old attachment. A
queued despawn commits after the source callbacks, so a task already due in that
tick may run before retirement. Saving immediately after commit prunes retired
tasks against the committed world, including deletion from either update or
resume; loading cannot resurrect them.

## Failure, costs and restoration

Task changes stage alongside numeric/structural writes, timers, persistent state
and logs. A thrown error, invalid result, caught invalid capability call,
deadline or deferred work rejects the tick and retains terminal failure behavior.
Retained capabilities expire and old bulk views stay detached. Task processing
does not get a second script deadline or bypass existing capability checks.

| Resource | Limit |
| --- | --- |
| Captured routines | 32 per source, 128 per program |
| Active tasks / resumes | 128 globally / at most 128 per accepted update |
| Start/cancel calls | 256 per whole update |
| One task state | 16 KiB, 1,024 nodes, depth 16 |
| Retained task states | 256 KiB, 16,384 nodes |
| Whole-tick task state transfer | 1 MiB, 65,536 nodes |

State accounting occurs during bounded typed traversal, before recursive
allocation can exceed the limits. Catalog-enabled ticks copy a bounded task
map, share immutable task records and allocate fresh VM data/capabilities.
Programs without a catalog retain the previous 16-argument update call and do
not allocate a coroutine queue or tick API. Startup catalog inspection and the
runtime branch still exist; no latency or allocation-free claim is established.

Behaviors use the existing `persistence: { version: 1 }` opt-in. Saves capture
source/task/routine identity, owner, next deadline and typed state. Ordinary,
in-memory native and persisted native checkpoints restore active waits in a
fresh VM without replaying earlier side effects. Source initialization still
runs; module globals and closures remain ephemeral. All state needed after load
must live in the task, explicit persistent state or native world.

Checkpoint versions 5/6 add the bounded task section. Task-free writers retain
the earlier section format, and readers still understand formats 1–4. Program
and runtime identity checks remain mandatory: this is not cross-build save or
hot-reload migration. The compiler/build identity includes the new wrappers,
catalog and task implementation, so old executable caches refuse before
QuickJS bytecode loading. Existing physics/audio/UI/static-module checkpoint
admission rules are unchanged; this feature does not make unsupported sessions
saveable.

## Executed qualification

Producer `9c72358987ca4e2bbc307c541569b66773ce4760` on macOS arm64, pinned Rust
1.99.0, passed **179 script library + 19 project-binding + 3 actual TypeScript
cook tests**, with no ignored or filtered controls, then three scoped strict
Clippy commands. The tests cover ordering/waits, cancel/replacement, global
limits, hostile result inspection, atomic failure, revocation, immediate-save
retirement, malformed checkpoint refusal and exact future state/log/checkpoint
parity. The cook test compiles the strict TS fixture, deletes its source
directory, then resumes and restores native state mid-wait. It is a test-process
source-free proof, not a separately packaged player.

The actual Rust/Schemars producer generated both new contracts byte for byte.
All 78 earlier registry files remain unchanged, and the headless generator's
only source change adds those same two `schema_for` registrations (80 files
total). The SDK generator check, strict SDK types, three coroutine TS controls
and seven isolated registry-checker controls pass. A full fresh headless registry
execution was not repeated; the source/byte-identity check is recorded separately.

Two compile failures remain retained: the first exposed an argument-tuple arity
limit and shadowed test helper; the second exposed a QuickJS lifetime annotation.
Two other preparations were superseded before execution by source review, which
corrected the result inspector and optional SDK capability contract. The final
attempt ran once, stopped on no failure, exited 0 and independently confirmed
144 observed PIDs and all seven owned groups absent. Original cumulative costs
remain charged: 801/3,600 seconds and 17,014,541,622 target bytes after the
explicit 2 GiB ceiling increase. Resource observations are sampled bounds.

[Machine-readable evidence](evidence/native-coroutines.json) binds exact source,
tools, commands, artifact hashes and the
[63 portable records](evidence/native-coroutines-records.json.gz), including both
failures and raw successful test/schema/lint output. The compressed archive is
data-only JSON containing UTF-8 records and per-record hashes; it contains no
executable artifacts or executable verifier.

No shipping-profile/player export, phone/GPU/window run, full-SDK completion,
2 ms script-budget result or phase gate is claimed. Web/Wasm task APIs,
implicit generator lowering and cross-version continuation migration remain
outside this scoped native explicit-state route.
