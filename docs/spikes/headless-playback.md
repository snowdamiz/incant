# Isolated headless playback and GPU frame capture

Phase 1 increment, 2026-10-09. `incant play` runs the existing sandboxed
PlaySession against the saved project checkpoint for a fixed tick count or a
duration rounded up to the next tick. With no script it uses a no-op behavior;
velocity simulation still advances. Compiled TypeScript behaviors use the same
simulation command bus. The authoring project and its journal are never written.

Optional output records initial/final frames and every selected interval using
the shared asset store and renderer. Captures use the current simulation project,
so both velocity and script commands affect pixels. Prior GPU versions remain
alive while the next scene prepares, allowing static buffers to be reused.
No account is needed. Without output no GPU is initialized.

Runs are limited to 10,000 ticks. Capture plans reject more than 128 frames,
2048 pixels per dimension, or 256 MiB of raw pixels before creating output.
Compiled scripts remain bounded by the sandbox's source/memory/deadline limits.
Output must be a new directory. A successful run atomically publishes report.json
with final runtime/script state, geometry counts and capture times. Errors exit
nonzero; partial PNGs may remain, but no completed report is published.

## Verification

- Two subprocess behavior tests verify rounded fixed-tick duration, runtime and
  script movement, script state, authored project/journal isolation, failure
  status, resource limits and preservation of previous output folders.
- One explicitly invoked real GPU test verifies ticks 0/37/60, matching report
  and PNG output, changing simulation pixels, equality of the first frame with
  the authored screenshot, and no completion report after a script failure.
- All 114 workspace Rust behavior tests, all three explicit GPU tests, workspace
  Clippy and formatting pass locally on this branch. No UI change is included.
- A separate CLI run loaded two imported model instances with their sources
  removed and captured distinct geometry positions at ticks 0/30/60 on Apple
  M5 Pro. The authoring document remained byte-identical. A second run used the
  bundled SWC compiler and kinematic TypeScript template: 60 ticks and 120 script
  commands with three GPU frames. Pixel review by Claude and hosted checks are
  pending; numeric PNG differences alone do not certify visual quality.

The appearance is the existing diagnostic renderer with a fixed camera. There
is no production render-graph, PBR, animation, input-script, assertion-script or
game-log claim. Frame times in the report are simulation times, not performance
benchmarks; wall_ms excludes startup. Unsaved editor edits are not replayed.
The current PlaySession projects ECS changes through its simulation command bus
and validates scripts each tick; this is not a game performance-budget result.
