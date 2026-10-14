# Recorded physical input in a native game window

A packaged native player can replay an explicit physical-input recording while
using its real window, renderer, audio and normal focus/surface eligibility.
Use `--window --stop-after N --input-recording FILE --observe PLAN`, with optional
profiling. The observation plan is optional; both recording and observation
require the explicit host-step bound. Unsupported options and repeated pairs
are rejected. Existing ordinary interactive behavior is retained.

The recording is parsed once under the existing 8 MiB limit. It must start at
zero, match the game tick rate and cover the requested host steps. Input is read
by successfully accepted host step, so redraws, focus pauses and surface retries
do not consume it. A session restart does not rewind the recording; the existing
release fence suppresses held controls until their recorded release. Recorded
UI receipts and viewport changes are explicitly refused before native devices
are created because native layout changes are asynchronous.

Recorded gamepads remain virtual. Replay creates no physical controller backend,
ignores live keyboard/mouse/IME delivery and advertises no haptic targets. Actual
window close, focus, suspension and surface eligibility still apply. An unexpected
nonempty haptic output is refused. This is deterministic input delivery through
the native host, not proof of real controller or captured-mouse behavior.

Observations run at selected accepted host steps, including step zero, and are
reported once at exit. Failed checks and an incomplete recording return a
nonzero exit; the report records the input hash, requested and accepted steps,
completion and device isolation. Restarting simulation ticks do not redefine
the observation host-step timeline.

## Qualification

Source `6a77d64b9e93521cbe2bd794d7db11223f595e0f` passes 85 profiled
native-window library tests and strict all-target Clippy with pinned Rust 1.99,
locked/offline arm64 CI builds and two jobs/threads. Independent source review
covers parser bounds, accepted-step and restart semantics, observation failure,
and physical-device isolation. The source identity and raw results are retained.

A fresh shipping export on Mac17,9 / Apple M5 Pro, macOS 27.0.1, uses fat LTO,
one codegen unit, aborting panics and its existing source-free/symbol/dependency
checks. The 2,156-step Core10k physical recording then completes in an actual
Metal window with seven HUD widgets, no failed runtime frames, no dropped wall
ticks and one surface deferral. All eight checks and all 17 selected entity states exactly match the
retained headless observations. Direct native-window inspection during that run
shows the canyon, moving character/enemies and changing sample-count HUD. The
37.98-second managed run includes startup; it is not a displayed-frame metric.
All owned children were reaped. A proposed exact-PID metrics listener was never
launched because identity verification found the player already exited.

The [manifest](evidence/native-window-replay/manifest.json) and compressed raw
records preserve inputs, commands, source hashes, independent review, CPU logs,
export receipt, actual window report and headless comparison. Replay the retained
evidence with `python3 docs/spikes/evidence/native-window-replay/verify.py`.
Executable/package bytes remain in the named local evidence directory; the
archive carries their identities. Host CPU completion histograms remain distinct
from display timestamps. This qualifies native input/gameplay parity, not 60 fps,
shader-stall, script-time, appearance of every frame or phase gates.
