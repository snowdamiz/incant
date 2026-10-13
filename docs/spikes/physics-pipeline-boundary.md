# Physics pipeline capture boundary and retained CCD verdict

This slice is read-only native observation and a checked inert record. It does
not enable physics saves, install pipeline state, join a deferred worker during
validation, or establish a complete/restorable native owner.

The default-off pinned Rapier `incant-checkpoint` observer returns the exact
`post_solve_ccd_active: Option<bool>`, two independent deferred-BVH ownership
flags, three quarantine report counts and two quarantine workspace counts. Its
exclusive receiver permits `Mutex::get_mut`; it never locks, receives, takes,
clears poison, executes or joins a task. Poison returns a typed error. No pointer,
channel, worker pool or scratch allocation is exposed. These copied observations
are not an authenticated step-phase witness. The native pipeline does not retain
last-step `dt`; empty counts do not prove all scratch is reconstructible.

## Actual native workspace replacement counterexample

In both pinned 2D and 3D, two worlds undergo the same real public mutations and
steps. After one fast dynamic body has produced `Some(true)`, the experiment
replaces only one world's `PhysicsPipeline` with `PhysicsPipeline::new()`. All
other native owners and the CCD cache are retained; no serde-to-live path or
snapshot installer participates. The last dynamic body is then removed from
both worlds.

For three subsequent positive steps, the reused pipeline keeps `Some(true)` and
repopulates its fixed-target CCD cache, while the fresh pipeline keeps `None` and
leaves its cache uninitialized. The reported CCD substep count is respectively
1 and 0. The serialized body, collider, island, broad/narrow, joint, soft and
integration owners remain byte-equal. Adding another awake fast body refreshes
both verdicts and the compared owner/cache state reconverges. This establishes a
retained cache/counter difference, not a demonstrated trajectory divergence or a
requirement to persist diagnostic wall-clock counters.

The cause is exact pinned control flow: `solve.rs` only invokes the staged solver
when an awake island exists; `init.rs` only replaces its verdict after that solve;
`substep.rs` consumes a retained `Some` instead of recomputing CCD flags; and
`ccd_solver.rs` prepares the fixed-target cache even when its active body lists
are empty. `serde(skip)` and the upstream workspace description therefore do not
justify dropping this value during exact restoration.

Other actual controls retain `None`, `Some(false)` and `Some(true)` distinctly,
exercise CCD-disabled steps, preserve quarantine reports across repeated reads,
and observe zero-`dt` deferred work without completing it. A later actual native
positive step performs the join. Ten controls pass with nonparallel native code;
ten pass with two Rayon workers. The latter records a retained receiver, the
former an inline task. These counts overlap the same five scenarios in both
dimensions; they are not twenty independent physical scenarios.

## Admission and remaining composition

The fixed 112-byte inert section retains exact finite gravity bits (including
negative zero), the three-valued verdict and explicit refused boundary states.
Decoding uses stack storage; output allocation charges the existing aggregate
Budget. Unknown tags, flags, padding, truncated and trailing bytes refuse.
Only a claimed healthy positive step may be structurally admitted, with exact
integration/CCD witness association. This is a stricter capture scope, not proof
that nonpositive native steps are invalid. Deferred work, quarantine and
unverified phases remain explicit refusals. A future enclosing owner must
independently authenticate the phase and unchanged configuration.

Remaining whole-owner work includes joining the accepted broad/narrow, body,
collider, shape, island, pair, integration, CCD and pipeline witnesses under one
bounded owner envelope; derived shape/mass/AABB and global solver-work checks;
then a separately reviewed whole-owner native constructor with continuation
proof. Native stable-ID maps, overlap/event state and runtime query/character
state must also be composed. No generic live deserializer or partial installer
is supplied here. `PhysicsUnsupported` remains unchanged.

## Scoped validation

Independent source reviews accepted the read-only getter and inert codec. The
final qualification ran 176 tests in each deterministic and fast-SIMD8 mode
(23 nonempty groups, 25 including empty unit/doc groups), plus the 20 focused
two-worker controls. All passed without ignored cases. Strict all-target Clippy,
wasm32 inert-library compilation, the std-only normal dependency graph, runtime
dependency guard, formatting, conventions and diff checks passed. Numerical-mode
and focused totals overlap; this is not shipping/device/performance evidence.
The six retained focused test binaries preserve executable modes and exact bytes.

The first joined quarantine fixture failed at the existing `PendingMutation`
prerequisite: the actual quarantined world retains a modified-body journal. Its
raw failure is retained. The final control preserves that refusal and separately
checks the actual copied nonzero quarantine counts against a healthy context,
explicitly without claiming that context belongs to the quarantined world. A
private test getter compile mistake and repeated failed edit attempt are also
retained. No production validation was weakened to admit that fixture.

The observer poison branch is implemented without clearing or inspecting poisoned
contents and is compiled on the parallel path; no artificial mutex-poison
execution is claimed. Neither this observer nor the successful inert decode can
prove an authenticated completion phase or a complete scratch-reconstruction
contract. Final evidence review remains pending.

[Lossless execution/source evidence](evidence/physics-pipeline-boundary.json)
binds each generation, failed attempts, exact command streams and independent
source receipts. The isolated vendor observer is commit `40adf519`; no solver
algorithm, lock resolution, default feature or live restore path changed.
