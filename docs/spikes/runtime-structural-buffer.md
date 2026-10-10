# Native runtime structural buffer

The disposable `incant_runtime::NativeWorld` now accepts a bounded `FrameCommands`
batch at an exclusive synchronization point. This implements the native entity
lifecycle part of PLAN.md Section 2.6. It is not an authoring transaction, a foreign
script ABI or a migration of the existing document-backed PlaySession.

## Observable behavior

A batch can spawn and despawn entities, set a parent while retaining the local
transform, and add, replace or remove the Velocity component. Scripts will queue
these operations through their future typed bindings; currently trusted native
systems can fill the buffer while borrowing component values and apply it after
the borrow ends. Rust's exclusive world borrow prevents simultaneous structural
mutation and schedule/view access.

All validation finishes before the first world change. Unknown entities, duplicate
IDs, scene/entity identity collisions, missing parents, cycles, invalid numeric
values, composed transform overflow and capacity violations return typed errors.
Failure retains every entity, the tick counter and the original queue. Success
updates real ECS archetypes and hierarchy caches, leaves the tick counter alone,
and clears the queue while retaining its capacity.

Parent references can point to an entity spawned later in the same batch. Other
commands address entities already present at that point in the command order.
Despawn is noncascading: children must be explicitly removed or reparented in the
same batch. A spawn followed by despawn is cancelled. IDs present at the start,
and IDs already used by another spawn in that batch, cannot be recycled within
the batch. The native caller supplies fresh stable IDs; this buffer does not keep
unbounded historical tombstones for IDs deleted in previous batches.

The maximum is 65,536 commands per batch and one million final entities. These
are explicit limits, not benchmark claims. A topology-changing batch stages only
ID/parent metadata and changed/new component values, then computes a parent-first
order iteratively. It does not copy a Project, journal, serialize or run the
authoring validator. Velocity-only batches touch their addressed entities without
copying hierarchy metadata or scanning the scene. Topology changes still rebuild
the hierarchy in O(N log N); optimization requires measured workloads.

## Verification

```sh
./tools/cargo test -j2 --workspace --release --locked
./tools/cargo clippy -j2 --workspace --all-targets --locked -- -D warnings
python3 tools/check_runtime_dependencies.py incant_types incant_runtime
./tools/cargo check -j2 -p incant_runtime --target wasm32-unknown-unknown --locked
./tools/cargo check -j2 -p incant_runtime --target aarch64-apple-ios-sim --locked
./tools/cargo build -j2 -p incant_cook -p incant_runtime --examples --release --locked
python3 tools/probes/cooked-runtime.py artifacts/cooked-runtime-structural
```

The focused cook/runtime suites pass 21 tests. Workspace verification passes
339 tests across 110 suites, with 44 GPU tests intentionally excluded by the
standard suite. Rendering code is unchanged in this slice; the preceding
foundation's explicit GPU runs passed all 44 (41 renderer plus three
editor/headless integration checks). No new rendered acceptance is claimed.

The nine lifecycle tests cover queued changes from live component access,
rollback of mixed batches, forward parent references, explicit child handling,
queries observing added/removed/re-added components after schedule initialization,
invalid numbers, cancelled spawns, capacity reuse, and 10,000 reverse-ordered
spawns in a deep hierarchy followed by iterative deletion. Loader regression
coverage also rejects an entity sharing its scene ID even after the binary image
has a valid recomputed digest. The standalone source-free cook/run probe passes
after the shared hierarchy refactor, including process repetition, reset and
corrupt-image rejection. Clippy, dependency isolation and both portable targets
pass. Independent Astra Extra High source review found no actionable correctness
defects in atomicity, identity, ordering, hierarchy and live component transitions.
Exact source hashes and probe results are in
[evidence](evidence/runtime-structural-buffer-2026-10-10.json).

## Remaining integration

Foreign TypeScript/Wasm view lifetimes, numeric-fault handling, generated component
bindings, remaining cooked components/resources, physics/navigation/rendering,
binary saves and the play-process migration remain open. Trusted native callbacks
still own the validity of their direct numeric writes. The command buffer does
not constitute a sandbox or an authoring undo/redo history. No shipping-profile,
reference-phone or phase gate is satisfied by these local correctness checks.
