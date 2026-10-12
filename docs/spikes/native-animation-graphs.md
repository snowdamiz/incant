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
elapsed time, explicit master pause and the second pose/weight. Queued commands cannot alter that tick's
snapshot. Master `controlAnimation` play/pause and speed remain usable. Its
clip/select/seek/loop/crossfade controls refuse graph instances to avoid competing
clock ownership. Errors poison the existing atomic script batch, rolling back
numeric edits, structural controls and unpublished logs. These graph controls
are not yet part of the Rust/Wasm guest ABI.

Scene v14 appends the graph table after the mandatory v12 text and v13 UI
sections. Scenes without graphs retain their previous version and bytes; an
empty UI table is allowed only as a lower-version suffix of v14. World-save v6
stores each graph's current index, explicit master pause, state time, elapsed time
and all 16 parameter
slots, together with the existing animation/fade state. Load checks exact graph
membership, parameter ranges, unused slots, selected sample, and fade identity
before replacing a world. Ordinary session checkpoints admit v6 and retain the
existing explicit script persistence contract. UI-bearing worlds still refuse
checkpoints until UI's own adapter exists. Full static-module state remains
Unsupported.

Verification and public process receipts will be recorded in the companion
JSON after final source checks. Local CPU tests do not establish phone or visual
acceptance. Nested/2D blend trees, layered animation, blended-state fades,
interruptible transitions, IK and root-motion integration remain outstanding.
