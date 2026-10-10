# Script execution CPU budgets

Gameplay previously used an `Instant` wall-clock deadline for initialization and
each update. A host pause or descheduling could therefore reject a script even
though the script had not used its execution allowance. The budget now charges
CPU time on the executing thread. The default remains 50 ms. Explicit
`ScriptHost::with_budget` durations use the same CPU-time basis.

The interrupt handler, native-query guards, hostile exception getters and the
final pre-publication check share one budget. Synchronous native query work on
the executing thread counts toward it. A failed update still cannot publish its
commands, state, timers or logs, even if JavaScript catches a native-query error.
Initialization is checked after successful evaluation too, so a zero budget
cannot silently admit a short program.

Pinned `cpu-time` 1.0.0 reads the Unix thread CPU clock or Windows thread times.
Incant uses its fallible clock API. An unavailable clock, backwards reading or
thread change during an execution fails explicitly; there is no silent unbounded
fallback. A new execution resets the clock and owning thread. Saved game data
contains neither OS clock readings nor wall-clock deadlines. Unsupported target
families fail explicitly. This increment does not add WebAssembly QuickJS support.
The dependency's MIT text is retained in source and development packages.

Behavior tests cover a 120 ms host callback wait while another thread burns
100 ms of CPU: the 50 ms script succeeds and publishes exactly one update.
Another callback consumes 80 ms on the executing thread: the script catches its
query exception, but the host still rejects the update without publication.
Existing runaway-loop and hostile-getter tests exercise real VM interruption.
Zero-budget initialization and clock fault/reset cases are covered separately.
The synthetic forced-expiry test remains a guard-placement test, not timing
performance evidence.

CPU time is not frame time. Heavy contention or suspension can make wall elapsed
time much longer than the budget. OS clock resolution and VM interrupt intervals
also permit some overshoot. Native calls cannot be interrupted inside their
solver; bounded input/query limits remain necessary and unchanged. Work executed
by other threads is excluded, so any future parallel query implementation needs
its own bounded work accounting. No new host capability, script sleep function,
filesystem access or process control is exposed.

At source `a9a1814`, 298 Rust tests, all 40 GPU tests, 315 UI tests and five tool
tests pass locally. Full workspace Clippy, generated contracts, strict TypeScript,
unsigned native packaging and both public steering/off-mesh save-reopen workflows
pass. The latter used temporary sleep prevention, so they are integration checks;
the timed callback tests establish the CPU-accounting behavior. Exact binary and
source records are in the [evidence file](evidence/script-cpu-budget-2026-10-10.json).

Cross-platform clock behavior still requires the hosted Windows/Linux workspace
tests. Compilation of the separate core platform probe does not establish a
scripting runtime gate. The final steering correction and dependency merges are
still pending. No phase gate is approved.


At integrated source `735842c`, all 300 Rust tests and Clippy pass, as do the
public steering/off-mesh repeat-and-save workflows and native packaging with
the CPU clock license. The complete unchanged 2700-tick plaza produces exactly
the same script state and runtime state as the reviewed `4c495e9` wall-clock
binary: all 100 remain at goal from 2212, without overlap. The separate rendered
steering review passes. These public runs used temporary sleep prevention.

Three script-only thousand-entity runs at `a9a1814` each issued 120000 commands
over 120 ticks: p95 12.079250, 12.566875 and 12.058417 ms, maximum 13.070250,
13.005375 and 12.501917 ms. They include no rendering and ran during concurrent
development on this Mac. They are workload measurements, not physical-device
release gates or a controlled before/after CPU-clock comparison.
