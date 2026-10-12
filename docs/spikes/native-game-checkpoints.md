# Native game checkpoints and exported offline players

This slice connects the opted-in ordinary-load native session checkpoint to the
existing game composition and public desktop exporter. It does not complete the
Phase 1 full-save requirement. Retained physics, audio cursors, static/Wasm
persistence, window/GPU host loading, and exact physical-input replay remain
explicitly unsupported.

## Construction and publication

`PendingSessionCheckpoint::new` validates the bounded checkpoint against the
actual trusted embedded program and initial cooked scene, including source and
instance membership, immutable localization, origin, scene and tick rate. It
owns the restored world privately before module evaluation. `world()` only lends
that world; `start()` initializes the opted-in bytecode behavior and installs its
persistent data/timers on the same `WorldIdentity`.

`GameSession`, `OfflineGameSession`, `HostedGameSession`, and
`HostedOfflineGameSession` expose `checkpoint_bytes()` and an associated
`from_checkpoint_bytes(...)` factory. Hosts prepare against the exact restored
world before guest initialization or audio backend creation. Plain factories
refuse spatial declarations both in the trusted initial scene and in the restored
world. The hosted factory accepts only what its actual `SceneHost` validates.

These are candidate factories, not in-place device rollback. The caller keeps the
old game until a complete candidate succeeds. Malformed/mismatched data, host
refusal/unwind or backend setup refusal/unwind cannot mutate that old game. A
trusted host/backend remains responsible for its own external preparation side
effects. No mutable world, VM or device escape was introduced.

## Public exported-player route

```sh
python3 tools/export_native.py project.json export-directory --camera CAMERA_ULID
export-directory/player/incant_game_player --package export-directory/player/data \
  --ticks 13 --output first.pcm --save-checkpoint first.incant-checkpoint
export-directory/player/incant_game_player --package export-directory/player/data \
  --ticks 27 --output resumed.pcm --load-checkpoint first.incant-checkpoint \
  --save-checkpoint resumed.incant-checkpoint
```

The same options work on the existing positional plain/CPU-render-input player.
The exported executable contains the trusted program; it never parses project
TypeScript on load. `--window`, `--gpu-offscreen`, `--static-module` and
`--build-info` combined with a checkpoint option refuse before route startup.
The unchanged exporter emits a player containing this route and continues to
validate fresh shipping codegen, authoring-free normal dependencies and absence
of runtime source compiler symbols.

`--ticks N` always means additional ticks. PCM bounds use
`floor((saved_tick + N) * sample_rate / tick_rate) -
floor(saved_tick * sample_rate / tick_rate)`, retaining the fractional phase.
Observation ticks are absolute; observations earlier than the loaded tick refuse.
An input recording supplied with `--input-recording` starts a **new neutral
relative timeline** (start_tick 0), with recording tick 1 applied on the first
resumed native tick. Saved physical controls are not replayed. This is ordinary
load, not exact input replay.

Save creation first verifies capability support, then encodes the final accepted
state only after successful gameplay and any requested assertions. The save is
written and synced to a private same-directory temporary file, then published
with atomic create-only/no-clobber semantics. Existing destinations are preserved;
partial staging failures are cleaned up in the covered normal-error paths. The
pinned tempfile implementation can fall back to hard-link plus unlink: destination
publication exposes the complete file, but a crash or unlink failure can leave a
second staging link. Atomic removal of both names is not promised. Choose a fresh
destination for each save.
This is not a universal power-loss guarantee, and PCM plus checkpoint are **not**
a multifile atomic transaction: PCM may already exist if later save publication
fails. Checksum/origin pairing is compatibility/corruption checking, not
cryptographic authentication of game data.

## Semantics and remaining limits

Every active behavior must explicitly opt into the existing `persistence: {version: 1}`
contract. Persistent typed data, active instance identities, native timer IDs and
committed world state survive. Module globals and closure state are ephemeral and
module initialization runs anew; arbitrary VM heaps/callbacks are never claimed
restored. Diagnostics/log histories and external host resources are not persisted.
The audio/physics-used guards remain sticky and no raw Rapier snapshots were
adopted. A fresh silent mixer does not mean saved audio playback; audio-bearing
worlds refuse.

## Validation

The numerical tests include 144 Hz gameplay with 44,100 Hz output: resume at tick
13 for 27 additional ticks emits exactly 8,269 frames, preserving the rational
phase. Real state/timer ticks yield byte-identical checkpoints to uninterrupted
execution. Tests cover neutral held/released controls, host identity and ordering,
failed candidate retention, audio/physics refusal, a checksum-valid checkpoint
adding spatial data to an originally plain world, option refusal and partial-write
cleanup/no-clobber.

The public probe is `tools/probes/game-checkpoint-export.py`. It authors actual
TypeScript state/timers, a mathematical imported model and a camera only through
public command routes, reopens the journal, then calls the real fresh exporter.
It removes its project/source fixture and all private build storage before its
player controls. No GPU, native window, live provider, device/performance or full
save gate is claimed. Silent PCM equality establishes phase/count consistency,
not audio-cursor continuation.

The final affected run passes **311 tests/doctests in 37 groups** (zero ignored),
including eight new behavior tests. Scoped all-target Clippy, compiler-disabled
static-module checking, optional GPU-feature compilation (no execution), runtime
normal dependency checks, formatting and conventions pass. Cargo.lock is unchanged.

The fresh public export produced an aarch64-apple-darwin shipping player. After
private authoring/build deletion, five successful processes and nine refusal
processes pass. Tick 40 uninterrupted and both tick 13 + 27 ordinary resumes have
identical final checkpoint bytes; CPU retained model X advances from 30 at tick 13
to 93 at tick 40. A separate fresh-input run presses Space at relative tick 1,
observably moving Y to 1 at absolute tick 14 without replaying prior controls.
The 60 Hz / 48,000 Hz exported runs cover 32,000 total frames = 10,400 + 21,600;
the independent unit test above covers a nonintegral sample ratio.

Refusals cover corruption, oversized data, checksum-valid program mismatch,
failed requested-save assertions, a past observation tick, an existing save, and
window/GPU/static mode conflicts. The existing save remains byte-for-byte intact;
normal staging cleanup leaves no temporary save names. Three early fixture compile
or revision errors are preserved in the evidence and corrected before the final
source freeze. They are not counted as passing results.

[Source/check receipt](evidence/native-game-checkpoints.json) and
[public exporter process receipt](evidence/native-game-checkpoints-process.json)
record exact inputs, output identities, limits and historical failures.

[Independent scoped source/evidence review](evidence/native-game-checkpoints-review.json)
accepted the frozen implementation without material findings. This review adds no
physical-device, GPU, physics/audio or full-save acceptance.

## Main103 integration

The reviewed `0d841fcd` source merged actual main103 (`2816ecde`) cleanly at
`4661e254`. All 14 reviewed source hashes and eight build/export input hashes are
unchanged. The integrated player/game runtime passes 34 tests in eight groups and
all-target Clippy with the optional GPU path compiled but never executed. Formatting,
conventions and the CPU player normal/build dependency graph pass. Main103 changed
only renderer portability, its workflow line and documentation; that renderer is
absent from the CPU export graph. The accepted five-success/nine-refusal fresh
shipping proof therefore remains applicable without another cold build.

[Integration receipt](evidence/native-game-checkpoints-main103.json) records the
exact source/log hashes and distinguishes the original 311-test/fresh-process
evidence from the focused integrated checks. The full-save and backend limits
above remain in force.
