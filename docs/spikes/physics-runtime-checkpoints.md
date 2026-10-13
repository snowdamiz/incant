# Primitive runtime checkpoints

The default-off `native-checkpoints` feature joins complete native physics
checkpoints to the owners held by `PhysicsRuntime<StableId>` and
`PhysicsWorld2D`. It depends on the production native capture boundary described
in `physics-native-capture-owner.md`; it is not a game-session save format.

## Ownership and publication

Ordinary simulation uses one explicit native owner. With the feature enabled,
real mutations invalidate its completed-step authority before native mutable
access. Immutable queries and unchanged motion echoes preserve that authority.
Simulation is still available when the selected build cannot capture snapshots.
The default feature selection does not enable checkpoint capture.

An opaque `PhysicsCheckpoint` retains the complete native frames, stable entity
and scene IDs, native handle generations, typed declarations, cached motion,
committed overlaps and sorted trigger events. Neither this value nor its native
frame has a public bytes-to-value or clone constructor. Exit events may refer to
an entity or scene removed before the last completed step.

Reconstruction builds new native and outer owners before publishing anything.
The joins check native ownership counts, reverse IDs, collider parents and
attachments, primitive geometry, material and collision settings, body settings,
cached motion and committed trigger observations. `restore_checkpoint` replaces
the destination only after every scene and join succeeds. A late refusal leaves
the destination unchanged. A failed runtime refuses capture before accessing a
possibly poisoned native owner or consuming a supplied budget.

The caller supplies shared payload, logical-work and native-construction
allocation budgets. The outer layer charges its retained payload and native
intersection candidate visits, including false pairs. This uses the pinned
native iterator's constant-time upper bound and refuses a missing or excessive
bound. Payload accounting covers requested stored values, not allocator
bookkeeping or an RSS limit. Native validation and reconstruction consume the
same caller-owned budgets; failure does not refund charges.

## Local evidence

The first combined source used main242, immutable native capture source004 and
the outer runtime draft. Both library suites passed 15 tests, including 96 paired
continuation steps per dimension with complete native encoded-frame equality,
stable IDs, queries, trigger observations, removal/reuse and motion edits.

The subsequent main242 composition uses native source009 (`f19f54ac`) and
outer source004. Both complete runtime test suites pass in deterministic and
fast8 modes: 38 tests per mode, including 17 library cases. Added controls cover
real terminal simulation failure, false intersection candidate charging,
removed-scene exits, exhausted budgets and refusal after a late scene's outer
validation. Both strict Clippy modes, WebAssembly compilation and formatting
pass. The default downstream `incant_runtime` and `incant_physics` suites also
pass 263 tests. These are local CPU checks, not device measurements.

Native source010 (`d6ca9867`) changes only the conditionally used capture-context
argument spelling and its two recorded vendor hashes. The final outer source005
composition with that source passes all 38 runtime tests again in each mode.
The outer implementation is unchanged between these two compositions. The native
producer separately verifies that its serial and WebAssembly libraries now pass
strict Clippy without the earlier unused-argument warning.

Initial error-conversion and test-variable compile failures are retained,
followed by the corrected successful runs. Executed binaries were copied
immediately after each successful test run. The
[portable local record](evidence/physics-runtime-checkpoints/manifest.json)
contains source identities, raw logs and command receipts; its read-only verifier
checks the retained packet. Full local evidence, including executable copies,
is under `~/.codex/incant-evidence/physics-runtime-checkpoints-001/`.
Independent source review closed both observed work-budget/failed-owner issues.
Final producer evidence review is independently accepted under receipt
`bcca0ca82ca0c65dff60c0ffd3d1e6e210481a8a7cf6e47463ba804914e91774`.
Current-main243 source composition is preserved separately in
[`evidence/physics-runtime-checkpoints-main243/source-composition.json`](evidence/physics-runtime-checkpoints-main243/source-composition.json):
all 3,647 listed implementation, build and tool inputs are byte-identical to
the executed source005 prepared tree, including the native capture dependency
and combined lock. This source comparison is not a new build or execution.
Integrated review and publication remain pending.

## Remaining limits

Only primitive geometry is admitted. Compound and tile geometry, retained 3D
character query caches, unsupported native families and states rejected by the
native numerical boundary are refused explicitly. The checkpoint is an in-memory
capability bound to its compiled native identity and retained execution context.
It does not admit untrusted persisted bytes, restore native ECS/script/session
state, implement file saves, prove cross-build compatibility, or satisfy device,
performance or phase gates. Existing public physics-save refusal remains in
force until those owner and admission boundaries are implemented and verified.
