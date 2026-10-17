# Cooperative native Wasm coroutines

Rust gameplay modules can declare explicit coroutine routines, start or cancel
named tasks, and return Wait or Done from a resumed callback. Both Wasmtime and
wasm2c use the same bounded, transactional host scheduler. No guest stack,
pointer, project document or JSON is retained across ticks. This closes one
native SDK gap; complete SDK and authored shipping qualification remain open.

## Contract

The additive `incant_coroutines_v1` namespace appends operations 72–73 while
keeping ABI 1 and operations 0–71 unchanged. A strict immutable custom section
declares each routine's name, fixed frame length and nonzero layout identity.
Missing, duplicate, malformed or unsupported declarations refuse during cook
and development admission. The trusted native linker emits constant catalog
storage; runtime programs do not load raw Wasm or dynamic native code.

Update precedes due tasks, which resume in ID order. Starts become due on the
next accepted tick. Wait uses a positive u32 delay from the current tick; Done
removes the task. IDs are unique until canceled or completed. Canceling and
restarting the active ID supersedes its old result. Frames are copied, explicitly
encoded bytes. Optional ownership binds a current native entity generation and
is retired on despawn or replacement, including same-public-ID reuse.

There are at most 128 routines and pending tasks, 256 start/cancel actions per
tick, 16 KiB per frame, 256 KiB stored frames and 1 MiB frame transfers per tick.
Current-epoch errors remain fatal even if ignored. Stale epochs stay revoked
without poisoning a different active tick. Incomplete or skipped due callbacks,
invalid results, traps and rejected downstream world operations commit no task,
persistent state or numeric prefix.

Compatible reload requires the same entire catalog, frame lengths and layout
identities. Coroutine-enabled saves use `INCWPS03`; modules without coroutines
still emit `INCWPS02`, and prior timer-free envelopes remain readable under their
existing rules. The schedule is canonical and bounded, contains no native entity
tokens and validates before changing running state. Receiving worlds rebind
saved owners to their own entity generations. Saving requires explicit guest
persistent-state opt-in; arbitrary memory and globals are not captured.

## Local qualification

At `d0346109`, 194 ABI, SDK and runtime test executions pass, and the authored Rust
sample compiles to Wasm. The first actual two-host attempt at `f1f46dd5` fails
before tests: generated catalog arrays borrow temporary storage. `c21ea29a`
corrects the generator with constant storage. That successor passes seven actual
guest/import controls and ten existing TypeScript coroutine controls. Twelve
compiled SDK variants exercise success, changed layout and ten rejection modes.

The actual static and development hosts match TypeScript across eight ticks,
including sorted resumes, update-time cancellation, waits and completion. Fresh
and persisted restore continue the exact saved task state; compatible reload
retains it, and changed layout refuses without replacing it. Ignored unknown
routines, duplicate IDs, bad scratch, owner/frame errors, task/action quotas,
invalid Wait, callback errors and traps refuse without committing world changes.
Both hosts also reject invalid catalog metadata and startup authority. Strict
lints pass for ABI, SDK, runtime, cook, development library and the changed tests.
An independent source review covers transaction/owner/checkpoint boundaries and
the generator correction. These are local correctness checks, not frame timing.

The four bounded native attempts consume 130.196 seconds, including the failed
31.745-second build. Their continuing target account totals 2,264.315 seconds;
peak allocation is 13,548,879,872 bytes under the existing 16,328,871,936-byte cap.
Every wrapper has an actual terminal tool result and freshly confirmed absence
of its owned PID/groups. Prior failures and protected native artifacts remain
retained. [The data manifest](evidence/native-wasm-coroutines/manifest.json)
binds plans, source inventories, raw logs, results, closeouts and source review.
Native executable copies remain outside the repository and hash-bound.

Normal authored export, source-free shipping-player execution and audible PCM
continuation for this new coroutine contract are still to be qualified. The
existing timer proof does not substitute for that work. Full SDK parity,
rendering quality, Core Sample performance and phase approval remain open.
