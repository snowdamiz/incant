# Retained cooked-game mobile owner

The default-off `incant_game_player/mobile-host` feature adds a safe Rust
`MobileSession<C>` over the existing native game, render/UI owner and offline
mixer. It loads the cooked package and the trusted program linked into the build
once. A platform supplies its render consumer and receives each accepted stereo
PCM slice. Construction is suspended and presents no frame.

Each `tick` is exactly one scheduled fixed simulation tick, not an arbitrary
display callback. The caller asks `next_output_frames` for the rational PCM
slice and supplies that exact output size. No recording, output file, project
document, per-tick JSON projection, or command-bus simulation is introduced.
The ordinary package argument route retains its shared startup verifier.

Delivery tokens bind an owner and a checked lifecycle epoch. Platforms must
retain the token with queued input rather than relabeling an old event when it
arrives. Suspension and resume revoke queued deliveries; accepted UI resizing
revokes older deliveries without advancing simulation or audio. Foreign owners,
stale tokens, malformed packets and wrong buffers reject before advancement.
The first resumed tick clears held controls and old UI composition; explicit UI
events wait until that neutralizing tick succeeds. Retryable input refusal does
not consume that transition. Accepted guest, audio or mandatory consumer failure
retires the composed game and silences PCM under its existing failure contract.

## Local qualification

The seven focused tests use an actual cooked scene, build-linked gameplay and
native render-input host with an instrumented consumer. They cover 104 callbacks
at 144 Hz with a 44,117 Hz mixer, stable world/consumer identity, exact fractional
sample count, suspension, stale and foreign deliveries, resize, malformed input,
UI admission, epoch exhaustion, package corruption and terminal guest/consumer
errors. They do not execute a GPU or an audio device. All 28 player library tests
pass in the final generation; the earlier five-test generation is retained.

Strict all-target Clippy passes for deterministic mobile-host plus mobile-ffi,
fast8 mobile-host, and ordinary feature-off builds. The runtime dependency guard,
physics forwarding guard, conventions, formatting and diff checks pass. The
`aarch64-apple-ios` library check passes; that is compilation, not a linked app or
device execution. An initial evidence-driver invocation misspelled the physics
guard subcommand; its exit 2 is retained and the corrected invocation passes.

## Remaining platform work

This is the retained Rust owner, not a complete iOS/Android app or a new C ABI.
Platform surface acquisition/presentation, real-time PCM delivery, scheduling,
touch/gesture mapping, IME/on-screen keyboard and lifecycle callbacks still need
concrete adapters and their own evidence. No restart/persisted restore or haptic
capability is exposed by this owner yet. A platform must stop external output on
suspension/failure and reserve its render target before accepting a tick. The
consumer contract cannot roll back GPU work already submitted before a late
failure. Existing immutable-package startup assumptions apply; this does not add
protection against another process changing package files during startup.

No mobile visual review, physical-device run, shipping-profile measurement or
phase gate is claimed. [Retained evidence](evidence/mobile-retained-host/manifest.json)
records exact source generations, commands and copied executable identities.
