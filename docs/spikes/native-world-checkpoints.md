# Opaque native world checkpoints

Phase 1 requires game-state saves on the native runtime data path. This slice
joins the already checked primitive physics owners with the ECS world and its
fixed-step clock. It is an opt-in, in-memory capability; persisted physics
admission and complete script/game-session saves remain separate work.

## Ownership and continuation

`incant_runtime/native-checkpoints` exposes `NativeWorld::checkpoint`,
`WorldCheckpoint::reconstruct`, `WorldCheckpoint::reconstruct_for` and
`NativeWorld::restore_checkpoint`. The opaque
checkpoint has no byte constructor, public mutable fields or `Clone` capability.
It retains the versioned component projection, both completed physics owners,
their stable IDs and trigger observations, immutable animation catalogs and
capability history. No project document, command bus, JSON or serialization is
introduced on the simulation path.

Scripts may edit typed numeric components after a physics tick. The checkpoint
therefore retains pending ECS poses, linear/angular motion and complete native
observations separately, without flushing or stepping during capture. Exact
body/collider membership and configuration are joined through read-only native
configuration views. Invalid pending values refuse. The planar binding's last
published quaternion is retained independently of a pending rotation, preserving
the existing exact-angle/sleep behavior on the next synchronization.

Both solvers advance whenever a world has used either physics dimension. A
3D-only world thus retains its stepped empty planar owner, even before its first
2D declaration. Saved declaration-history flags remain distinct from native
owner presence. Rewinding to a time before physics was used restores that actual
stepping history; a separate sticky save obligation keeps ordinary byte saves
closed without inventing extra native ticks.

## Atomic replacement

Reconstruction stages both complete native owners before ECS hydration, which
accepts those actual owners and never constructs temporary replacement solvers
from pending components. Original scene/rate/catalog checks, planar cache joins
and destination generation checks precede final publication. A late failure
leaves the destination world, physics, events and identities unchanged.

A fresh reconstruction receives a new host identity and ECS incarnation.
In-place restore keeps its destination host identity while changing ECS/entity
incarnations and advancing retained-view generations. Navigation caches rebuild;
old path results and script bindings must not attach to the new incarnation.
`reconstruct_for` applies the destination compatibility and historical-obligation
checks to a fresh candidate without publishing it. A session host can then bind
and validate a fresh VM before replacing both owners together. The runtime
reexports the four checkpoint budget types for this composition.
Failed worlds remain terminal. Dirty native owners, unsupported compound/tile
geometry, retained character query caches and other native admission refusals
remain explicit.

## Limits and qualification

Physics payload/work/allocation budgets retain their existing documented scope.
They do not claim to measure all ECS serializer allocations, allocator overhead
or process RSS. Component serialization keeps the existing `MAX_SAVE_BYTES` and
scene bounds. Dirty navigation refuses until committed.

Public `save_bytes`, `from_save_bytes`, scene-bound saves and byte restore keep
their physics refusal and legacy error precedence. The checkpoint contains no
script heap, callbacks, external command buffer, physical input, mixer cursors,
widget buffers or IME state. Hosts must separately coordinate those owners; this
is not a complete game-session save, cross-build format or performance gate.

Source003 has independent source acceptance after three review corrections:
avoid duplicate native construction, retain a stepped empty planar owner, and
separate historical save obligations from restored stepping history. The initial
test compile error and subsequent legacy-navigation error-precedence regression
are retained as failed historical runs. Source004 adds the compatible staging
API and its no-publication control; its independent delta and execution review
is accepted. It passes 261 runtime tests in each of the deterministic and fast8
modes, including eleven aggregate checkpoint controls. The 205 default
physics/script consumer tests passed on source003; the source004 changes are
feature-only and do not alter that ordinary execution path. Strict all-target
Clippy passes in both modes; wasm compilation, the authoring-free dependency
guard, formatting and generated conventions also pass. The compiled test
executables were copied immediately before target reuse. The portable
[manifest](evidence/native-world-checkpoints/manifest.json) retains distinct
source003/source004 identities and historical failures. Run
`python3 docs/spikes/evidence/native-world-checkpoints/verify.py` for a read-only
source and receipt check; `--local-binaries` also hashes the retained executables.
Current-main composition and publication are reviewed separately. Local binary
payloads remain under `~/.codex/incant-evidence/native-world-checkpoints-001/`.
