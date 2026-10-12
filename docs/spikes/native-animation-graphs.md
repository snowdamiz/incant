# Native animation graph foundation

This scope connects typed `AnimationGraph` authoring to cooked native state
machines and flat one-dimensional blend motions. It extends the existing
skeletal playback and two-pose compositor. It does not complete the plan's full
blend-tree, IK, root-motion, rendered appearance or device performance gates.

`AnimationGraph` is an ordinary schema-checked component edited through
`incant_cmd`, with transaction provenance, Undo and journal persistence. It
requires `Animation` and `MeshRenderer`. The entry state's clip and looping flag
must match `Animation`; its authored time must be zero. A blend entry uses
`Animation.clip: null`. Parameter, state and transition keys are canonical stable
ULIDs. The cooker resolves them into immutable numeric tables against the same
captured model fingerprint and complete clip-duration table as playback.

Bounds are 1,024 graph instances per scene, 16 numeric parameters, 32 states,
128 transitions, eight conditions per transition and eight threshold points per
blend. Parameter ranges and thresholds are finite within ±1,000,000; blend cycle
and exit-time limits are 3,600 seconds, and clip fades retain the existing
60-second limit. Unknown kinds, duplicate IDs, missing references, self edges,
unordered thresholds, mismatched clips and unsupported fades fail before world
publication. The shipped loader repeats those checks on the bounded binary.

A state is a clip (including the rest pose) or a flat `blend1d` motion. A blend
selects adjacent threshold clips and interpolates their local TRS poses through
the existing two-input renderer adapter. Outside the thresholds it clamps to an
endpoint; exactly at a threshold it samples one clip. All points share the
state's declared `cycle_seconds`, with each model clip sampled at the same
normalized phase. Translation/scale use linear interpolation and rotation uses
the existing shortest-arc quaternion interpolation. No rendering code or pixel
interpretation changes in this scope.

Automatic edges are ordered by ascending priority, then transition ULID. Their
conditions are ANDed; optional `after_seconds` measures unscaled running state
time, including running at speed zero. Pause freezes it. At most one edge runs
per committed fixed step. State entry resets phase and elapsed time. An active
clip-to-clip fade owns its entire step, including its completion step; automatic
edges cannot interrupt it. Nonzero fades involving a blend state are explicitly
unsupported because they need a four-pose compositor. Zero-duration transitions
and explicit selection between all supported states are implemented. Reverse,
wrap, completion and speed preserve the original clip clock rules. Graph master
pause is retained separately: natural completion freezes the pose while running
state time and conditions continue, so a later parameter or timed edge can
transition. Explicit pause freezes pose, elapsed time and edge evaluation. State
selection starts its new motion whenever the graph master is not paused.

TypeScript uses `commands.setAnimationParameter(entity, parameter, value)` and
`commands.selectAnimationState(entity, state)`. The owned typed columns in
`animations.graphs` report stable IDs, parameter offsets/values, state time,
elapsed time, explicit master pause and the second pose/weight. Queued commands
cannot alter that tick's snapshot. Master `controlAnimation` play/pause and speed remain usable. Its
clip/select/seek/loop/crossfade controls refuse graph instances to avoid competing
clock ownership. Errors poison the existing atomic script batch, rolling back
numeric edits, structural controls and unpublished logs. These graph controls
are not yet part of the Rust/Wasm guest ABI. The existing v1/v2 Wasm animation
snapshots cannot represent parameter-driven blends: both length and read imports
explicitly refuse those snapshots without copying bytes or publishing the atomic
transaction. Graph endpoint poses remain representable, and modules which do not
request those snapshots can continue using the scene. The native snapshot carries
the actual second pose separately from the timed transition record.

Scene v14 appends the graph table after the mandatory v12 text and v13 UI
sections. Scenes without graphs retain their previous version and bytes; an
empty UI table is allowed only as a lower-version suffix of v14. World-save v6
stores each graph's current index, explicit master pause, state time, elapsed time
and all 16 parameter slots, together with the existing animation/fade state. Load checks exact graph
membership, parameter ranges, unused slots, selected sample, and fade identity
before replacing a world. Ordinary session checkpoints admit v6 and retain the
existing explicit script persistence contract. UI-bearing worlds still refuse
checkpoints until UI's own adapter exists. Full static-module state remains
Unsupported.

The final seven-package source suite passed 481 tests, with the unchanged long
10,000-sequence Undo test separately passed before the final rerun. Scoped
all-target Clippy, runtime/types Wasm compilation, strict SDK/editor TypeScript,
generated schemas/bridge checks, 228 Python checks (four skips), 22 native-script
Node checks and the lean dependency audit passed. Tests exercise actual skin TRS
interpolation, atomic authoring and script batches, malformed cold binary/save
inputs, deterministic edge order, master pause versus natural completion,
source/cache deletion and exact active/paused/blended continuation.

The ordinary public exporter produced a fresh shipping player from frozen
`915199f7` source. Three fresh `--package` processes each ran 90 ticks with three
authored actors, 270 logs and 60 independently calculated graph observations.
DevPlay and export observations agree. Active fade, paused fade, completed-and-
paused, and blended-state v6 checkpoints resume to the identical final save, PCM
and log sequence. This includes a nonlooping clip completing before a later
parameter starts its outgoing transition. The driver deleted its private
project/source/cache and export-build storage before execution; the sole removed
debug source map has a recorded hash. Corrupt saves and deliberately wrong
observations produced no PCM/save output. The receipt records compiler/codegen,
package hashes, literal public authoring/export/player commands and limitations.
The later legacy Wasm snapshot refusal (`cc302120`) passes 18 boundary tests,
15 cases on each actual Wasmtime/wasm2c host, 12 native graph tests, 18 script
animation tests and the owned snapshot unit test, plus scoped Clippy and Wasm
checks. Its pinned translator and input identities are recorded separately; the
preserved QuickJS export receipt does not claim a combined later source build.

[Verification receipt](evidence/native-animation-graphs.json),
[public export receipt](evidence/native-animation-graphs-export.json),
[independent source review](evidence/native-animation-graphs-source-review.json),
and [independent export review](evidence/native-animation-graphs-export-review.json).
Local CPU tests do not establish phone or visual acceptance. Nested/2D blend trees, layered animation, blended-state fades,
interruptible transitions, IK and root-motion integration remain outstanding.

## Actual main130 integration

The accepted graph implementation and legacy Wasm guard were merged with the
reviewed public Wasm, steering and CPU UI foundation. The only text conflict was
STATUS; the six automatic source merges retain both sets of contracts. The
actual main130 ancestry merge changed no file in the tested tree.

This union passes 538 affected CPU tests in 92 groups, including the long Undo
sequence; 342 actual Wasmtime/wasm2c tests in 58 groups and three development
processes; all-target Clippy; wasm32; lean runtime graph checks; tooling; strict
SDK types; 68 byte-identical schema exports; generators and formatting. The CPU
and actual-host suites overlap and are not summed as distinct cases. All 97
prepared modules and 486 artifact hashes were verified before private cleanup.

A fresh current-lock ordinary export produced a 9,103,120-byte shipping player
SHA-256 `efb19fb84d387ab45262090bf3e9aaf4527965bac72bef312a59005c5aeddf1b`.
After deleting its private authoring/cache/build inputs and external debug map,
three new processes reproduced all 60 graph observations over 90 ticks. Eight
split/continued processes covered active, paused, completed-paused and blended
v6 checkpoints with exact final save, PCM and logs; corrupt checkpoint and wrong
observation controls refused without publishing output. The same frozen source
also contains the legacy Wasm snapshot refusal. This closes the earlier
combined-build evidence gap without claiming static-module checkpoints, graph
Rust/Wasm controls, rendering, device performance or phase completion.

[Integrated sources, commands, retained logs and fresh export evidence](evidence/native-animation-graphs-main130.json).

The final independent [integration review](evidence/native-animation-graphs-main130-root-review.json)
accepted the exact source, logs, package and continuation identities. Its initial
evidence digest precedes a packaging-only change: the source seam diff is now
stored as gzip with its original uncompressed hash, preserving every reviewed
byte while avoiding patch-context whitespace diagnostics. The final evidence
digest is `c806075c0479228b77f37201ee91af661d59a3d2d85c38b359e3e22a88b04907`.
