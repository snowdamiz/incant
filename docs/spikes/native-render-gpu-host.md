# Native game GPU host

`incant_render_gpu_host` supplies the real retained GPU consumer for
`NativeRenderHost`. One `HostedGameSession` now advances typed input, QuickJS,
physics, retained scene audio, native render input and GPU output through the
same committed tick. Preparation allocates GPU resources before guest/audio
startup succeeds but presents nothing; the first frame follows tick one.

The host owns its renderer and target provider. Every accepted sync acquires a
frame, submits retained geometry, and presents it. A structural change prepares
a replacement before publishing it. Only read-only metrics escape. Target
acquisition, GPU preparation/submission or presentation failure drops the live
consumer; the composite drops its world/VM/audio/host, refuses subsequent ticks,
and clears the caller's offline PCM buffer. A presentation failure can follow
successful GPU submission, which cannot be undone. `accepted_frames` counts only
frames whose submission and presentation both succeeded.

The renderer installs a device-loss callback at device creation. Nonblocking
polls before target acquisition and after submission reject observed loss. This
is not a synchronous guarantee about when a driver reports loss, nor rollback
of GPU work. Trusted callers of the existing public device must not replace the
renderer-owned callback. Surface resize/reacquisition policy belongs to the
native target provider; returning an error after a game tick is terminal.

## Verification

The combined release suite passes 102 ordinary tests/doctests plus all 61 native
GPU tests (58 renderer, three game GPU host). Scoped all-target release Clippy
passes. Resolved normal dependency checks include renderer, native input, game
host, GPU host, game runtime and player with scene audio enabled; authoring and
compiler dependencies remain excluded.

On Apple M5 Pro / Metal, actual cooked model and PCM fixtures feed the binary
native world, QuickJS input logic and kinematic physics. Source files, authoring
store and cooked cache are deleted before eight ticks. Tests verify changing GPU
readback, expected native position, 6,000 audible PCM frames at 64 Hz/48 kHz,
and matching CPU tick, accepted GPU frame and presentation counts. Additional
tests inject acquisition and presentation errors, presentation unwind and
observed device destruction; they verify terminal cleanup and silent offline
output. The device-loss test waits for prior submissions before checking the
callback; production checks remain nonblocking.

This is in-process numerical integration, not appearance approval or proof of a
compiler-free rendered executable. That standalone player proof is the next
integration slice. Native surface/window ownership, sprite and tilemap support,
complete runtime restoration, per-frame allocation optimization and the plan's
real-device/performance requirements remain open.
