# Native TypeScript planar physics queries

Native QuickJS behaviors now use `physics.raycast2D(query)` for ground probes and
`physics.triggerEvents2D()` for sensor enter/exit observations. These are runtime
reads against the active native scene. Movement still uses the existing borrowed
numeric columns and deferred structural commands. No project command, document,
filesystem or additional mutation path is involved.

## Contract and ownership

Rays use XY coordinates and normalized nonzero directions. Distance and coordinate
bounds are the existing planar solver's finite one-million-unit limits. Starting
inside a solid shape returns distance zero. Queries include solid tilemap cells;
empty cells remain misses. `include_sensors`, `exclude_entity`, and both collision
membership/filter masks are honored. Exact-distance ties use stable entity IDs.
Hits have `entity_id`, `distance`, and two-element `point`/`normal` arrays.

Events contain sorted `first`, `second`, and `entered` fields, with canonical ULID
strings. The active scene is implicit. Reads return the last committed planar
step's events without draining them; pending numeric or queued structural changes
become observable after the corresponding synchronization/step. Repeating an
update without stepping reads the same events. Hit/event objects and arrays are
owned, writable VM values; retaining or editing them cannot alter either solver.

The existing numeric loan now moves both solvers into two private revocable
ownership cells. A guard restores both on normal return, error and unwinding;
retained native capabilities see empty cells afterward. No solver or world is
cloned. Numeric validation uses the unsolved ECS/hierarchy path and never reads a
detached solver. This adds one ownership-cell allocation to the previous loan;
it is not a zero-allocation query claim.

Both methods share the existing 1,024-call budget with 3D raycasts, character
queries and events. Every invocation consumes budget before reading arguments.
The 128-character-query sublimit is unchanged. No Rust solver/state borrow spans
guest getters or result creation. Native calls have deadline checks immediately
before and afterward; they cannot be preempted mid-query. A caught invalid call,
quota failure or materialization failure still rejects the complete update,
restores numeric writes, discards queued commands and fails that script session.
Expired functions never reactivate, including during later updates.

## Generated types and packaging

The existing planar `RayQuery`, `RayHit` and `TriggerEvent` are generic over their
ID type with the prior `StableId` default. Rust solver APIs remain binary-ID
based. Schema generation instantiates textual IDs for the three SDK contracts;
`RayHit.entity` serializes as `entity_id`, matching 3D script result casing.
`NativeRayQuery2D`, `NativeRayHit2D`, and `NativeTriggerEvent2D` directly alias those
generated contracts. The SDK includes typed ground/pickup helpers.

The new guest conversion file participates in the cooked-script compatibility
hash. A trusted cook must pair bytecode with the matching engine identity, as for
other host capability changes. No runtime compiler, source loader or export
special case was added here.

## Local verification and limits

On the exact source freeze recorded in
[evidence/native-script-physics-2d.json](evidence/native-script-physics-2d.json),
217 runtime/script/planar-backend tests passed with no ignored tests. Nine new
behavior tests cover solid cells and holes, sensor filters, exclusion, owned
results, committed enter/exit events, mixed 2D/3D continuation, error/unwind
restoration, retained capabilities, shared/recursive budgets, prototype setters,
caught argument failures and allocation-failure rollback. The continuation check
compares 120 steps against an unqueried control with both solvers present.

All-target Clippy passed for those three crates plus `incant_headless`; the
headless build and compiler-disabled script check passed. Sixty-one freshly
produced schema/agent files match the registry, including three new contracts and
58 unchanged prior files. Generated SDK, strict SDK/template TypeScript, bridge,
formatting, conventions and the runtime dependency guard pass. Independent
read-only production review by `/root/runtime_review` found no actionable blocker.
The linked evidence records source/log/review hashes and the exact base.

These are local macOS ARM64 correctness results in the CI profile. This slice
does not add a planar character controller, joints, impulses, physics persistence,
Wasm/static/browser capability parity, editor UI or a complete playable sample.
The existing planar root/unit-scale restrictions remain. It makes ground/sensor
logic available to native TypeScript games; it does not satisfy phone performance,
shipping-device, rendered appearance or phase gates. Complete engine/game release
requirements remain open.
