# Retained native world checkpoints (save version 2)

This slice extends the explicit `NativeWorld` save boundary to the currently
retained nonphysics scene state. It does not save a game session or a script VM.
Normal ticks and queries do not serialize, copy documents, or touch these codecs.

## Preserved state and checked restore

New saves preserve stable entity IDs, current hierarchy, continuous f64 local
transforms and velocities, the fixed tick, all current spatial declarations and
binary asset recipe bindings, nonsolid tilemaps, flipbook configuration plus exact
frame/rational remainder/play/completion state, and navigation declarations with
embedded checked mesh geometry/grid costs. The original cooked-scene fingerprint
is preserved separately from the mutable checkpoint scene. In-place restore
requires matching origin, scene ID and tick rate; the origin remains available to
program-binding checks. These hashes detect accidental mismatch/corruption, not
authorship or authorization, and the checkpoint loads no executable code.

Global transforms, tile placements and navigation query caches are derived anew
from checked data before publishing a candidate. No source files, authoring
project, JSON, asset cache, renderer or GPU are needed for this world operation.
The eventual asset consumer must still resolve/check the saved binary descriptors.
Old path results, rebuild reports, dirty-tile history and derived cache allocations
are not persistent game state. Queries after loading use the newly built geometry;
this does not claim to continue an externally retained path/crowd/agent object.

Saving dirty navigation refuses until an accepted `apply` or `step` publishes the
rebuild. A save never publishes pending changes on the caller's behalf. External
`FrameCommands` remain outside the checkpoint: hosts must choose a committed sync
boundary and discard pending commands when replacing the world. Commands rejected
by an atomic batch are absent from the save. Ordinary native loans cannot overlap
save/restore borrows; trusted immediate numeric values are checked during saving.

An in-place restore first decodes and validates the complete candidate. Failure
preserves the destination, its queries, clock and identities. Successful restore
preserves `WorldIdentity` for the retained host but replaces `WorldIncarnation`
and every entity identity. Spatial topology/binding generations and navigation
path generations advance beyond those of the replaced instance. Borrowed views
cannot span the exclusive restore; hosts must discard previously owned results.
These generations are process/instance cache validity, not persistent path IDs.

## Wire format and resource bounds

The existing 64-byte `INCANTSV` envelope remains: little-endian version, zero
reserved flags, u64 tick, u64 payload length, and SHA-256 over header metadata and
payload. Version 2 adds a 40-byte section header: original scene SHA-256 (32),
u32 nested-scene length, and u32 flipbook row count. It then stores the checked
canonical scene and strictly increasing stable-ID playback rows. Each 28-byte row
has a 16-byte ID, u16 frame, two separate canonical boolean bytes, and u64 rational
remainder. The owning checked clock validates the frame, fractional denominator
and possible completion state before private playback state can be constructed.
Every flipbook has exactly one row; missing, extra, duplicate and unordered rows
refuse. Counts and exact aggregate length are checked before scene allocation.

Nested scene bytes retain their 128 MiB cap and current versioned component
validation. At most 32,768 playback rows add 917,504 bytes. The total save cap is
135,135,336 bytes (64 + 40 + 128 MiB + 917,504). This is a serialized-data cap, not a
claim about peak reconstruction heap usage. Existing geometry/derived navigation
caps also apply at load. Saving is an explicit potentially allocating operation.
It is not the frame path, and no save-time/performance claim is made.

Existing version-1 plain Transform/Velocity/hierarchy saves remain readable and
new writes use version 2. V1 has no original fingerprint field; a fresh v1 load can
identify only its saved plain scene. A matching plain live destination preserves
its own known origin on v1 restore. V1 cannot clear a destination's spatial or
navigation capability, nor smuggle nested spatial/navigation/physics/audio state.

## Explicit refusals and next dependencies

Any world that used 3D/2D physics or audio still refuses saves/restores, including
a world whose declarations were later removed. Solid tilemaps require the planar
solver and also refuse. A rehashed valid scene cannot bypass this boundary.
Reconstructing bodies from transforms would silently discard contacts, sleep,
warm-start impulses and overlap/event history; rebuilding audio declarations
would discard sample cursors/resampling and host state. Those are not implemented
by this slice.

The next sequential work is:

1. Add a pinned backend checkpoint contract for complete 3D/2D solver state and
   native ID/config/event mappings, with bounded decode before allocation and
   actual contact/sleep/trigger continuation across fresh processes. Document
   version, target and determinism compatibility; do not substitute body poses.
2. Add explicit scene-audio cursor/mixer, input, native timer/schedule and script
   attachment/state contracts, coordinating their owning crates. QuickJS closure
   and heap persistence needs an explicit supported contract; simply starting a
   new VM or replaying callbacks is not continuation. Static/Wasmtime module state
   likewise requires its own versioned trusted-module pairing and memory policy.
3. Compose those accepted pieces into one atomic session checkpoint/restore API,
   with terminal host/consumer failure behavior, then provide durable save slots
   and export/SDK/editor integration. No session/guest API is exposed prematurely.

## Local verification

The [scoped evidence record](evidence/native-retained-saves.json) lists exact
source/log identities and counts. The [independent source review](evidence/native-retained-saves-review.md)
found no actionable blocker.
Behavior tests compare complete saved bytes and subsequent world/flipbook/query
results after pause/resume, reparenting and stable-ID reuse. Five separate test
processes cover uninterrupted, partial, resumed, repeated and zero-tick playback;
the binary source scene is deleted before resume. Both paths include a fractional
flipbook pause and actual retained grid/navmesh queries. These are local native
process correctness checks, not a shipped game, visual review, VM checkpoint,
Windows/Linux/mobile execution, physical-device performance or phase gate.
