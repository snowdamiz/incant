# Opaque native session checkpoints

The opt-in `incant_script/native-checkpoints` feature joins an opaque
`WorldCheckpoint` with the existing explicit script persistence version 1
contract. `NativeSession::checkpoint` captures a healthy bytecode session;
`NativeSessionCheckpoint::reconstruct` creates a new session;
`NativeSession::restore_checkpoint` replaces an existing session only after the
entire world and fresh VM initialize successfully.

This is a process-local headless session capability. There is no byte importer,
arbitrary JavaScript heap or closure snapshot, Wasm state admission, or change to
public physics byte-save refusal. UI and audio host obligations, including prior
use after declaration removal, refuse explicitly. Composed host/session exports
remain separate work.

## State and publication

The checkpoint retains the actual trusted `CompiledScript`, its active authored
instance set, immutable bounded persistent entries, bounded timer schedule, and
validated action definitions. The same filtering and opt-in validation serve the
existing byte checkpoint. Retired per-entity state is removed at capture; reuse
of a stable ID cannot revive an attachment. The world checkpoint preserves
completed native physics separately from accepted pending ECS numeric inputs.
Capture performs no extra solver step or pending-input flush.

A fresh VM evaluates the captured startup code and persistence declarations
again. Module globals, closures and diagnostic logs start fresh. Physical input
and its input clock restart neutral, while script simulation time and timers
continue from the saved tick. Captured action definitions replace later rebinds.
The host must discard queued OS events and commands and reconnect devices;
this is not exact input replay.

Pending restart requests refuse both capture and in-place restore. A newly
reconstructed session has restart capability disabled until its host opts in.
In-place restore retains only the receiving host's current restart capability.
It does not import a capability from the checkpoint. Failed sessions refuse.

In-place restore requires exact trusted program compatibility and stages a
world using `WorldCheckpoint::reconstruct_for`, preserving immutable scene and
catalog bindings and sticky host/save obligations. Fresh VM initialization and
explicit-state installation precede one session assignment. Failure leaves the
live world, VM, input and persistent values unchanged; supplied budgets may have
been consumed. Success creates a fresh world identity and entity incarnations,
so retained external handles and hosts must be discarded.

## Bounds and verification scope

The existing script bounds remain: 1 MiB / 65,536 nodes / depth 16 for explicit
persistent values, bounded authored instances, 128 timers, validated bounded
input bindings and the normal 32 MiB VM limit. Physics capture/work/allocation
budgets cover native physics and its outer owners only. ECS serialization uses
its existing save-byte/scene limits. These are logical component limits, not a
claim to cap all process allocations or allocator overhead.

Source001 was independently reviewed before broad qualification. Its first
focused run passed eight controls and exposed a fixture-oracle error: public
`physics_state` intentionally exposes pending ECS pose/motion rather than raw
solver fields. The corrected fixture records the freshly published Y inside
the script callback before adding 0.75, then checks exact pending ECS bits.
The initial failed result/binary and a subsequent format-brace compile failure
remain retained. All nine corrected focused controls passed. Final numerical
and feature-build results are recorded in the accompanying evidence manifest.

The controls cover mixed planar/spatial continuation with pending inputs and
angular state, timers and per-instance retirement, fresh-VM ephemeral reset,
neutral physical input/captured bindings, restart authority and pending requests,
UI/audio obligations, opt-in and failed-session refusal, immutable saved entries,
short budgets, wrong program/world, and a test-only late initialization trap.
The late-trap control mutates private test state to reach the failure stage; it
does not introduce a production code setter or checkpoint importer.

## Source004 execution

The full script suite passed 195 tests in each of deterministic and fast8 modes
(136 library, 58 integration, one compile-fail documentation test); feature-off
passed 186. Counts overlap the nine focused checkpoint controls. Strict all-target
Clippy passed both modes. The combined UI/audio/static-module/Wasmtime option
check, lean library check, wasm32 check, normal runtime dependency guard, scoped
formatting, conventions and diff checks passed. Optional compilation does not
claim hosted native checkpoint support in this headless slice.

The conservative all-path physics manifest guard failed: twelve diagnostics
already exist on the inherited runtime API revision, and the new script feature
propagates one of them. Three inherited optional snapshot edges enable defaults
and omit weak numerical-mode forwarding. The snapshot crate's normal dependency
graph is std-only; actual scoped normal deterministic and fast8 graphs pass all
five solver mode assertions. The original failed guard and baseline comparison
are retained. Root owns the separate manifest repair; this packet does not claim
the conservative guard passed.

Immediate copies preserve the executed library test binaries; integration and
documentation results are retained as exact command logs, without a claim that
their executables were copied. No ordinary exported package, device, GPU, or
hosted UI/audio continuation was exercised by this slice.

## Repaired generation and current main246 composition

The preceding failed manifest guard remains historical source004 evidence. The
three inherited snapshot defaults/weak forwarding omissions were repaired and
merged in PR246. Accepted repaired producer `7a900ece` passed the guard, both
selected solver graphs and nine focused checkpoint tests in each mode; the broad
195/195/186 totals are not relabelled as a repaired-generation rerun.

The curated main246 implementation is byte-identical to that repaired producer
across all 3,656 tracked implementation/build/tool files. Ten read-only checks
pass, including exact normalized graph comparisons and ordinary snapshot
exclusion. No new compile or session execution was necessary. The current
composition, portable receipts and limitations are bound in
[native-session-main246.md](native-session-main246.md). Final current-main
independent review is accepted (`12de683d`); publication awaits its exact-head fast check. Hosted UI/audio continuation stays separate.
