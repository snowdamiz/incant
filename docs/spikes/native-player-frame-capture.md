# Packaged-player selected-frame capture

Source implementation; native compilation and actual GPU execution are pending.
This adds the Phase 1 headless runner's frame-output capability to the existing
source-free CPU composed player, including its screen-space HUD and UI.

```sh
incant_game_player --package DATA --ticks 3056 --input-recording restart.json \
  --observe observations.json --ui-viewport 640x480 --output audio.pcm \
  --capture-frames NEW_DIRECTORY --capture-at 0,900,901,3056 \
  --capture-size 1280x960
```

`--capture-at` contains unique integer **host steps**, not simulation ticks or
PCM sample frames. Step 0 is the initial accepted state; the final requested step
is selectable. Requested steps are sorted and must be within the run. A selected
tick is drawn after simulation, UI, audio output and observations are accepted,
before any restart replaces that world. The receipt preserves its generation,
simulation tick, validated run-local world epoch and cumulative PCM prefix.
The usual final report can describe a newer generation than the last capture
when that frame requested restart. No second world or script execution is added.

The renderer reads exactly `NativeRenderHost::render_input()` and `ui_input()`.
One renderer, physical output texture, padded readback buffer and RGBA vector are
reused. Binding/topology/text or world replacement prepares a fresh retained
scene/UI layer. Motion and temporal history are never enabled. UI uses the same
logical-viewport to physical-target scaling as the ordinary renderer, so a
recorded logical viewport change affects layout without resizing capture files.
World rendering precedes the screen UI and real GPU readback. PNG encoding uses
the renderer's existing pinned encoder; no dependency is added.

The default-off `frame-capture` build feature is included by `native-window`,
making the ordinary desktop shipping player usable in this offline mode. A
standalone capture build needs no native window or device audio feature. A
feature-off invocation refuses before package, GPU or output setup. Capture
requires the existing CPU spatial bytecode route and refuses `--window`,
`--gpu-offscreen`, checkpoints, static modules, material recording and material
prewarming. Existing modes keep their own admission and chronology.

Bounds are 128 frames, 1–4096 physical pixels per dimension, 64 MiB per padded
readback and 256 MiB total selected raw RGBA. Published output is bounded to
320 MiB, each PNG to its raw size plus 1 MiB, and each JSON file to 1 MiB. These
are not aggregate GPU or encoder-memory bounds: retained geometry/UI, frame
attachments and driver overhead are separate. GPU poll and callback waits are
each limited to ten seconds. PNG work and readback are explicit observation
overhead; captured runs are not performance measurements.

The output directory must be absent. Each PNG and receipt uses same-directory
staging, file sync and atomic no-clobber publication. `partial.json` remains as
the initial incomplete marker; a complete `manifest.json` is published only
after every requested frame, the successful simulation and PCM publication.
An error leaves completed partial files for diagnosis and no complete manifest.
PNG, receipt, PCM and final manifest are separate file transactions, not one
multi-file or universal power-loss durability guarantee. Host-selected output
paths are not project data or a private-storage authentication boundary.

Prepared CPU controls cover unique/initial/final selections, limits, missing and
conflicting flags, feature-off early refusal, missing captures, byte/collision
failures and terminal publication. The separately ignored Metal control compares
ordinary CPU and captured player reports/PCM, checks PNG identities and physical
dimensions, falsifies UI omission with a same-world negative oracle, and verifies
pre/post-restart generation/tick/PCM correspondence with a persistent script
counter. It is not yet executed. No window presentation, continuous submission,
temporal continuity, locale appearance or performance acceptance is claimed.
