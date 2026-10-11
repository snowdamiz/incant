# Native TypeScript navigation queries

This bounded Phase 1 slice exposes the reviewed retained native navigation cache
through `context.navigation.findPath(entity, request)` and
`findGridPath(entity, request)`. It builds on reviewed native navigation/save
`15a73702` and the reviewed native audio/2D-query parent `cfaa4420`; local merge
`19bae87a` preserves both audio and navigation preflight before state publication.
It introduces no authored project mutation or additional movement command.

Mesh results contain owned world-space points, polygon corridor, visited count,
cache generation and explicit off-mesh traversal records. Each traversal keeps
its canonical link ID, consecutive point indices and direction. Gameplay must
handle those segments explicitly rather than treating them as walkable mesh.
Grid results contain owned adjacent cell indices, exact planner cost and cache
generation. Diagonals cannot cut blocked corners. Disconnected/blocked/snap misses
return null; absent owners, malformed requests and exhausted searches reject the
whole script update, including when JavaScript catches the exception.

The numeric callback's existing private RAII guard now moves the retained
navigation cache alongside both physics solvers. All three empty Rc containers
are allocated before moving any state. Before the loan begins, native freshness
validation refuses uncommitted source poses. During the callback, queries see
only the last committed cache, including when numeric writes or structural
commands are pending. The query loan clones no cache, world, geometry or solver;
owned query result/frontier allocations remain bounded by the existing native
algorithms. Numeric validation and cleanup touch only ECS/hierarchy while the
native caches are detached. Success, error and unwinding restore ownership and
empty the retained capabilities. Old functions cannot reactivate in another tick.

The existing shared query state permits 1,024 total physics/navigation calls per
update, at most 32 navigation calls, and a sum of at most 65,536 requested search
expansions across all mesh/grid requests and all authored source callbacks.
Each request consumes its full requested allowance, including misses. The grid
default is 65,536, so callers making multiple queries must choose explicit lower
budgets. Calls are charged before guest getters; requested search work is charged
before native execution, with recursive-getter failures rechecked. Finite double
bounds are checked before f32 narrowing; grid cells/limits must be exact unsigned
integers. Generations and costs cannot silently round beyond JavaScript's safe
integer limit. Materialization uses owned data properties, bypasses inherited
setters, and allocation failure poisons the transaction.

CPU deadlines bracket synchronous searches; they cannot preempt native work.
The expansion quota is not a count of every nearest-triangle, detail or smoothing
operation. Those are separately bounded by the existing cooked geometry/path
caps and native algorithm work bounds. This is correctness evidence, not a
reference-phone frame-time or 100-agent navigation performance result.

Generations are scoped to one native world and navigation owner. Save restore
advances the retained generation without reviving old capabilities. Owned paths
must not be compared across sessions/worlds. This API does not persist a script
VM or path objects, supply an asynchronous query, map grid cells to world space,
move actors, steer/avoid crowds, traverse links automatically, expose runtime
geometry/grid/link editing, or add a static-Wasm navigation ABI. Those capabilities
remain unsupported; existing native movement commands remain the control path.

Validation on local aarch64 macOS in the `ci` profile: 298 tests in the complete
runtime/script/game-runtime/navigation suite, then the added restore-generation
case and focused repeats: 299 distinct tests, zero failures/ignored, including
12 new behavior tests. The actual native tests exercise detours, grid costs and
movement, disconnected/blocked misses, off-mesh identity/direction, committed
visibility, source freshness refusal, RAII unwind, owned outputs, stale handles,
recursive getters, shared multi-source budgets, caught-error/materialization and
late-command rollback, exact generation conversion, and retained-save invalidation.
Five-package all-target Clippy, headless build, compiler-disabled script check,
strict SDK/templates, generated SDK/bridge, formatting/conventions and normal
runtime dependency checks pass. All 61 freshly generated schema/agent files are
byte-identical; native SDK aliases reuse the existing Rust-derived path schemas.
The independent read-only review found no actionable blocker. Machine-readable
commands, source/log hashes and limitations are in
[evidence](evidence/native-script-navigation.json).

## Integration with repaired main91

The original reviewed `179d384` history is preserved by merge `bdddb0b` onto
actual main91 `c8b05fb`. Only STATUS conflicted; all thirteen reviewed source, test
and SDK file hashes remain exact. Current audio/navigation atomic preparation,
Cargo inputs and workflow coverage are unchanged.

The integrated six-package runtime/script/game/navigation/Play/headless suite
passes **351 tests/doctests**, with the two existing headless GPU cases ignored;
this query-only integration does not claim a new GPU run. All-target Clippy, the
compiler-disabled script check, headless build, strict SDK/templates, generated
bridge, conventions and formatting pass. All **62** freshly generated contract
files match. The four affected runtime graphs remain authoring-free.

The rebuilt public headless host also passes **32 actual RPC requests across two
fresh processes**, including source authoring, register/attach, native motion,
Undo/Redo and journal recovery. Authoring stays unchanged by Play. This is local
macOS correctness evidence; comprehensive Windows/mobile/browser validation stays
with the next frozen release checkpoint, and none of the limitations above is
removed. The [ledger](evidence/native-script-navigation.json) retains the exact
source, proof and validation-log identities.
