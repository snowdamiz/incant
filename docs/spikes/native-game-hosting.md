# Explicit retained hosts for the native game session

This change extends the authoring-free game composition with stored, trusted
native `SceneHost` implementations. The existing `GameSession` and
`OfflineGameSession` constructors still reject both spatial declarations and a
manifest-only scene before constructing the VM or audio host. Existing players
remain strict. No renderer, GPU, window or authoring dependency was added.

`HostedGameSession<B, H>` and `HostedOfflineGameSession<H>` take `H::Settings` and
retain the actual `H`, with read-only `host()` access. `H::prepare` must validate
and retain compatible bindings for the exact supplied world and refuse any
unsupported declaration. This is a native implementation contract, like Kira's
backend contract: arbitrary Rust implementations are trusted; this is not an
untrusted capability token or a sandbox. An accepting test host does not prove a
render binding or rendered frame. The real CPU/GPU consumers are separate work.

`PendingNativeSession` owns the validated input bindings and one fresh world. It
exposes no tick or mutable world. Preparation sees that exact world before
`start` creates the VM; moving it into `NativeSession` preserves its opaque world
identity. Audio host construction follows VM construction. Host rejection runs
no guest and issues no session-owned backend setup. These guarantees cannot
undo device effects the caller already performed before supplying its settings.
A later startup failure drops the prepared host and other constructed resources.

Each accepted tick runs native input/simulation/script, audio synchronization,
offline rendering if present, and finally retained host synchronization. Success
publishes observations only after all steps succeed. Invalid input and an
incorrect offline buffer remain retryable; they do not call host synchronization
or modify the output. The rational sample clock is unchanged.

The internal owner takes its live state out during a tick or audio-error poll.
Only success or retryable input rejection puts it back. Post-acceptance errors
and Rust unwinding therefore drop the VM, world, audio and retained host and hide
all session observations. Offline cleanup clears the complete output even when
rendering already produced audible samples before host rejection or unwind.
The guarantee covers ordinary unwinding with non-panicking destructors, not
process abort. Already-submitted device audio and external consumer side effects
are not rolled back; no world, input, VM or cursor save API was added.

## Verification

On the source hashes in the accompanying [evidence](evidence/native-game-hosting-2026-10-10.json):

- `./tools/cargo test -j2 -p incant_game_runtime -p incant_script --release --locked`
  passes 57 tests/doctests: seven new hosting regressions, nine existing game
  regressions, and 41 script tests/doctests.
- `./tools/cargo clippy -j2 -p incant_game_runtime -p incant_script --all-targets --release --locked -- -D warnings`
  passes.
- `./tools/cargo check -j2 -p incant_game_runtime --features device --locked`
  passes on macOS; this compiles the backend feature but opens no audio device.
- Runtime dependency guards for `incant_game_runtime` and `incant_script` pass
  both normally and with `incant_game_runtime/device`; formatting, generated
  conventions and whitespace checks pass.

The new observable tests cover preserved world identity through pending startup,
preparation before top-level guest/backend effects, startup cleanup, stored-host
updates after actual script motion, retryable invalid buffers/packets, rational
frame counts, host error/unwind after audible rendering, script failure skipping
host synchronization, and backend destruction after a late failure/unwind.
All three plain/camera-only/manifest-only scene forms are exercised at the
boundary; the pre-existing strict-refusal test remains unchanged.

The full-workspace and source-free shipping results recorded for `a3327e3` in
[native game composition](native-game-session.md) belong to that earlier source
identity. They are not claimed as fresh shipping validation of this change. The
real prepared render-input adapter, GPU consumer, integrated player proof and
subsequent union checks are separate integration work. No rendered-pixel,
reference-device, performance or phase gate is claimed.
