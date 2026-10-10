# Native binary world saves

The first native save slice serializes the currently migrated world: stable
scene/entity identities, Transform, optional Velocity, hierarchy, fixed tick rate
and tick counter. It runs only when explicitly requested. Normal simulation does
not clone project data, serialize JSON, touch a journal or validate the world.

This is not yet the game-session save system. Script VM state, timers, input,
physics, navigation, audio and the remaining components are not owned by this
native world and have no save representation here. External FrameCommands are
also excluded; the caller saves a committed boundary. The legacy PlaySession and
its save format are unchanged. Adding another native system must extend the
versioned state contract rather than silently dropping its state.

## Format and publication

Version 1 begins with INCANTSV, a u32 version, reserved zero flags, the u64 tick,
a u64 payload length and a SHA-256 checksum. The little-endian 64-byte envelope
covers both its metadata and the nested cooked scene. The existing scene format
supplies component masks, their schema version, canonical archetype/ID ordering,
reference/bounds checking and its own integrity check. Unknown versions, flags,
component masks, malformed values, cycles, missing parents and trailing data
fail before a world is returned. Hashes detect corruption; they do not authenticate
save authors or authorize executable content. No executable content is loaded.

The maximum is 128,000,136 bytes, corresponding to the existing million-entity
scene bound plus the envelope. Loaders check this before parsing/allocating.
Save construction validates the current numeric/hierarchy state before emitting
bytes, because trusted direct native writes can temporarily hold invalid values.
Float bits and the integer clock are preserved; derived global transforms are
rebuilt. Reordered ECS tables produce the same canonical save. Tick overflow
fails before simulation changes anything.

NativeWorld::from_save_bytes creates a checked disposable world without a source
scene/project/journal. restore_bytes constructs the whole candidate first and
only replaces a running world if scene identity and tick rate match. Any failure
leaves the prior world and clock untouched. It does not reset or restore external
script hosts. Script/session orchestration must integrate that lifecycle before
exposing full-session restore in the editor or game SDK.

The save_world example bounds input acquisition, caps requested ticks and refuses
to overwrite an existing output file. It is a process-proof tool, not a finished
save-slot host: crash-safe durable file replacement, backups, cloud conflicts,
package compatibility and migration policy remain work for that host.

## Verification

Nine new observable tests cover exact save/resume against every uninterrupted
tick at 1/60/120/240 Hz; spawn/despawn/reparent/Velocity changes and direct column
writes; canonical bytes independent of insertion order; every truncation and
single-byte corruption; rehashed malformed payloads; wrong-scene/rate rejection;
invalid live values; tick overflow; empty worlds and explicit legacy rejection.
All 382 workspace tests/doctests and 44 serialized GPU tests pass, together with
workspace all-targets Clippy. The runtime compiles for WASM, iOS simulator and
Android. The actual runtime dependency guard, generated conventions, Python syntax
and workflow lint pass. No hosted or device result is claimed yet.

The public probe authors through incant_cmd, cooks without modifying authoring
files, deletes project/journal, then runs a copied standalone executable. After
saving at tick 25 it also deletes the cooked scene. Fresh processes restoring
that save to tick 60 produce exactly the same 368 bytes as uninterrupted execution;
repeat and zero-tick restores are byte-identical. A corrupt save creates no output,
and attempting to overwrite the input save leaves it unchanged. The local
release-profile proof is recorded in
[evidence](evidence/native-world-saves-2026-10-10.json).

macOS CI builds save_world in the existing workspace-selected tool invocation
and runs the process probe directly. It adds no second package graph, full build
or separate job; all existing checks remain. This branch also carries the reviewed
Khronos texture-probe reuse follow-up. Its hosted timing remains unmeasured.

Built-by: astra
