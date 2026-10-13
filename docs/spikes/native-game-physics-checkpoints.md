# Process-local game checkpoints

The default-off `incant_game_runtime/native-checkpoints` feature composes the
opaque native physics/script checkpoint with the game owner's audio clock,
suspension and retained UI. It is an in-memory continuation API, not a persisted
physics save format. Public byte-save refusal remains in force for stepped
physics, and this work does not approve the Phase 1 save or device gates.

## Ownership and startup

`GameCheckpoint::prepare` reconstructs one private native world and binds its
captured UI/audio sections to that world's fresh identity. It runs no guest
initialization and creates no external scene host. Each `from_checkpoint`
factory consumes this pending candidate and returns a complete new game. The
previous game remains available until the caller explicitly replaces it. Old
physical input and externally queued host work must be discarded on replacement.

Offline checkpoints retain the original sample rate. Their factories accept no
replacement sample rate, so a continuation cannot silently change the rational
output clock. Backend-driven and offline checkpoint families cannot be
interchanged. Backend-driven audio transport restoration remains unsupported;
the existing typed refusal applies before host or guest startup.

For an offline audio checkpoint, the actual clip catalog and audio transports
are validated against the fresh world before the trusted host is prepared.
Hosted UI is then restored through `SceneHost::prepare_checkpoint`, and the
pending session validates the actual retained owners before starting its VM.
Suspension is retained even for a silent scene. Restart authority is not copied
to a fresh game; the receiving owner must opt in explicitly.

Module initialization runs again. Only opted-in persistent script data and
timers survive; arbitrary guest globals, closures, logs and Wasm memory do not.
Budgets cover the existing physics payload/work/allocation boundaries. Script,
ECS, UI and audio retain their individual bounds; these are not a process-wide
memory limit. Trusted Rust host preparation remains responsible for external
side effects it performs before returning an error or unwinding.

## Verification scope

Focused controls exercise mixed 2D/3D continuation with pending ECS input,
opted-in script state/timers, real static and streamed PCM, the captured 44.1 kHz
clock at 144 simulation ticks per second, silent suspension, host setup
error/unwind, missing clip data, incompatible clock families and failed-session
capture. The UI control uses typed text/focus state and a deterministic CPU
measurement fixture; it makes no rendered-pixel or font-rendering claim.

Final source003 passes all 39 game tests in both deterministic and fast8 modes,
including the six focused controls. Strict Clippy passes both modes and the
headless feature combination; static-module compatibility and wasm compilation,
feature-policy, formatting and convention checks pass. The feature-off suite
passes 33 tests on source002; the later capture change is feature-gated.

Source001 retained a UI fixture failure because it attempted input before initial
layout. Source002 fixes that fixture. Independent review then caught a silent
mixer clock reset, fixed by retaining the actual audio owner even without
audio declarations. Source003 tests repeated capture after silent restoration.
Logs, frozen sources, review receipts and executable identities are in the
[portable packet](evidence/native-game-physics-checkpoints/manifest.json).
