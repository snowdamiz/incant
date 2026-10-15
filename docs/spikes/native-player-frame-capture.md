# Packaged-player selected-frame capture

The existing source-free CPU composed player can render selected accepted states
to PNG, including its screen-space HUD and UI. CPU qualification and one actual
Metal control passed; shipping Core and locale image collection remains separate.

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

CPU controls cover unique/initial/final selections, limits, missing and
conflicting flags, feature-off early refusal, missing captures, byte/collision
failures and terminal publication. At production source `5b2fa73d`, five
feature-off controls, 30 feature-on player CPU tests and one renderer readback
layout control passed: 36 test executions, including overlapping configurations.
Strict all-target Clippy passed for the ordinary and capture routes; the
native-window route was checked for compilation only. Its existing third-party
Tao warnings remain explicit in the retained log.

The first actual Metal test failed during its ordinary CPU baseline, before
renderer setup: its recording declared version 1 while carrying a viewport
event, which the runtime correctly refuses. Test-only successor `d1194a71`
declares version 2 and adds an ordinary CPU regression that executes the actual
baseline and also checks the version-1 refusal. That regression and affected
strict Clippy passed. Production code and the original Metal assertions stayed
unchanged; the failed attempt remains in the packet.

The successor's exact copied test executable passed one actual Metal control.
Ordinary and sampled player reports and PCM bytes matched. Three 320×192 PNGs
were checked for signature, dimensions and encoded hashes while the logical UI
viewport changed from 160×96 to 80×96. A same-world render without UI produced
different readback bytes. A persistent script counter verified the separate
restart case's selected `(host step, generation, simulation tick)` values:
`(0,1,0)`, `(2,1,2)`, `(3,2,1)`, `(4,2,2)`, followed by generation 3/tick 0.
Fractional-rate PCM remained exactly 1,224 stereo sample frames in that case.
Both native test children were reaped and their PID/groups independently absent.

The test owns and removes its temporary PNG/PCM files. This is sampled rendering,
readback and chronology evidence, with no retained-image appearance, public
shipping export, native-window presentation, continuous submission, temporal
continuity, locale layout or performance acceptance. The CPU observer's existing
`gpu_submitted:false` field still describes that CPU observer; the separate
`frame_capture` result describes selected offscreen submissions.

The [portable packet](evidence/native-player-frame-capture/manifest.json) retains
the source inventories, exact commands, logs, copied-executable identities,
original refusal and cleanup receipts. Executables and unchanged full source
copies remain outside Git. Replay uses saved bytes only:

```sh
python3 docs/spikes/evidence/native-player-frame-capture/verify.py
python3 docs/spikes/evidence/native-player-frame-capture/verify.py \
  --originals --binaries --git-source d1194a71
```

`--source-root` additionally asserts that every producing input is unchanged in
that checkout; it must not be used to relabel later source compositions as reruns.
