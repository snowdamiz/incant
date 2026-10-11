# Native script physics queries

Native QuickJS behaviors now receive `physics.raycast(query)` alongside bulk
columns, structural commands and input. Types derive from the existing generated
RayQuery/RayHit schema, with the active native scene implicit. The
[ground-query example](../../sdk/templates/native-physics/raycast.ts) runs through
the same build-produced bytecode path as other native behaviors. No document,
command-bus transaction, JSON conversion or project validation enters the query.

## Ownership and tick behavior

Queries read the last committed solver state, including previously accepted
structural bindings. Pending numeric writes and queued structural commands are
not visible until their normal synchronization points. Results contain copied
IDs/scalars and mutable VM-owned vectors; changing them cannot alter physics.
Sensors, collision masks, exclusion and stable-ID tie behavior use the existing
Rapier boundary. Empty worlds still validate requests before returning null.

A private RAII guard temporarily moves the actual solver into a revocable
read-only capability while exclusive numeric ECS views are exposed. It allocates
the slot before moving ownership, and restores the solver after numeric cleanup
on success, error or Rust unwinding. Retained functions never retain the solver
after that scope, and an old tick cannot reactivate them. No raw world pointer,
cloned solver or unsynchronized ECS view is used. The trusted numeric cleanup
path remains independent of the temporarily detached solver.

Each update allows at most 1,024 raycasts. An invocation reserves its slot before
reading guest properties; no Rust/RefCell borrow spans getters. Recursive getters
therefore share the same bound. Invalid input, limit/deadline failure and result
allocation errors latch failure even if the guest catches the JS exception,
rolling back numeric writes and discarding the structural batch. Query and column
capabilities revoke before diagnostic getters execute. The host fails permanently
after a rejected update, consistent with the existing accepted-tick contract.

QuickJS cannot preempt an in-progress native Rapier call. The thread CPU deadline
is checked immediately before and after that work; phone p95 budgets still need
device measurement. The capability slots and guest results allocate; this is not
an allocation-free or full physics-SDK claim. Shape casts, character controllers,
trigger bindings, timers/state and the remaining SDK migration are separate work.

## Verification

The runtime/script/physics packages pass **124 release tests/doctests**, all-target
release Clippy and SDK typechecking. Observable tests cover real hits/filters,
committed versus speculative poses, retained capability revocation, exception
getters, bounded reentrancy, next-step solver equivalence, Rust unwinding, and
complete numeric/structural rollback after a swallowed query error. A
production-absent one-shot allocation fault exercises the real guest-catch path;
independent review found that missing latch and approved its correction.

The existing source-free script probe now runs **34 native processes** from one
initially empty shipping build: all 32 prior repeated structural/input/physics
controls plus two invalid/over-budget query refusals. The physics runs execute
ground/exclusion queries for up to 600 ticks; nonphysical worlds require null.
Every subsequent tick confirms that its previous function has expired. Both
refusals catch the guest error and still produce no successful world output.
Authoring sources, TypeScript/CommonJS and fresh Cargo storage are removed before
the first process starts. Existing parser-symbol, source-sentinel, compiler-flag
and matching crash-symbol checks remain intact; CI gets no additional cold build.

Actual local player: **4,074,184 bytes**, aarch64 macOS, SHA-256
`99223e51c2ac67d3125f9bd222891860698b4db7438affff51172c1d8db0b934`,
arm64 dSYM UUID `9bae028c-2630-3e7d-a638-3a979eb84bd6`.
Raw package, symbols and logs: `/tmp/incant-native-raycast-shipping/`.
[Source identities, review and complete observations](evidence/native-script-queries-2026-10-11.json).

The integrated branch also passes **553 ordinary workspace release tests and
doctests**, workspace and GPU-player all-target Clippy, the six-root normal
dependency guard, and 43 Python tool tests. The 66 ignored GPU tests were not
rerun for this query-only change; the renderer implementation is unchanged.
