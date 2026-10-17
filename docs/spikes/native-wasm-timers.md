# Transactional native Wasm timers

Native Wasm modules can now schedule simulation-time callbacks through the Rust
SDK in both Wasmtime and wasm2c. Timers commit with the world transaction, survive
explicit saves and compatible reloads, and roll back on refused callbacks. They
use a bounded native schedule; no wall clock, project documents or JSON enter the
tick path. This closes one SDK parity gap, not the full native SDK requirement.

## Contract

The additive `incant_timers_v1` namespace has operations 69–71: snapshot length,
snapshot read and command submission. Existing operations 0–68 and ABI version 1
are unchanged. `Tick::timers` copies clock/events into caller scratch;
`set_timer` and `cancel_timer` submit fixed-layout commands. IDs contain 1–64 ASCII
letters, digits, underscores, periods, colons or hyphens. Unused bytes must be
zero. Each module has at most 128 pending timers and 256 timer actions per tick.

Delays are positive whole simulation ticks. An optional positive interval repeats
from the prior scheduled deadline, so a late callback does not shift the cadence.
Setting an existing ID replaces its future schedule; cancellation is idempotent.
Due events are sorted by ID. Mutations do not rewrite the current callback's
published events. Invalid current-epoch operations poison the transaction even
if guest code catches their error; stale epochs remain revoked. SDK-local errors
also retain the existing sticky refusal. Unsupported versions, signatures,
startup imports, bad ranges and exceeded budgets fail explicitly.

The accepted clock includes tick and elapsed simulation seconds. Persisted
snapshots use `INCWPS02` with canonical `INCTMS01` timer state, alongside declared
guest state. The old unprefixed snapshot remains readable with an empty schedule
and zero elapsed seconds at its recorded tick. Malformed input is rejected before
mutating the running world. Compatible reloads transfer the committed schedule;
failed reloads preserve the previous VM and state. See the public
[SDK contract](../../sdk/rust/README.md#simulation-timers).

## Qualification

The local CPU qualification includes 93 passing native test executions: 15 shared
runtime/persistence controls, 65 ABI/SDK controls, five actual guest/import
controls and eight existing TypeScript timer controls. Ten Rust SDK variants and
a startup-refusal WAT fixture exercise both actual native hosts. The successful
guest route matches TypeScript across ten ticks and tests ordinary fresh-VM and
persisted restore, cancellation/replacement, periodic callbacks, local error
refusal, pending/action limits and failed/successful reload. Two strict lint
commands pass across ABI, SDK, runtime and the development library/changed tests.
The generator/helper controls and all ten local development checks also pass,
including tools/Core tests, formatting, generated contracts, TS checking, and
editor production build/tests.

Two failed attempts remain failed. CPU001 at `439cdf29` stopped before running
tests because three capability branches omitted explicit returns. `542522cc`
fixes those branches. CPU002 passed all 15 selected runtime/persistence tests,
then its wrapper rejected an unchanged SDK artifact reused from CPU001. CPU003
explicitly binds that prior artifact and preserves the original failed wrapper;
it passes the remaining 65 ABI/SDK tests and strict lint. The `9a9957c5` successor
adds actual guest controls and passes the remaining 13 tests and development
lint. No failed attempt is relabeled or removed.

The four attempts consumed 329.348 seconds; the continuing shared target account,
including earlier work, totals 2,134.119 seconds. Final/sampled peak target
allocation is 13,177,294,848 bytes under the existing 16,328,871,936-byte ceiling.
Each wrapper has an actual terminal tool result, and all observed owned PIDs and
process groups were independently absent before release. These elapsed costs
are build/test costs, not gameplay performance measurements.

[Retained data manifest](evidence/native-wasm-timers/manifest.json) binds compressed
plans, source inventories, raw command output, guest provenance, results,
closeouts and independent source reviews. Each record has compressed and raw
SHA-256 identities. Native executable copies remain external and hash-bound;
the records do not require executing archived code. Source inventories and
protected binaries were checked before/after the original runs.

The separate [authored timer export](native-wasm-timers-export.md) now verifies
normal public authoring, source-free shipping and exact timer/gameplay/audio
continuation. No GPU, window, device timing, full SDK parity or phase completion
is established by these scoped proofs.
