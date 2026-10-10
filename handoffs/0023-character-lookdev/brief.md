# Character movement look-dev and rendered motion review

## Priority revision: verify the movement corrections

Your first review is integrated. Astra reproduced the flat-floor and wall stalls
in a real PlaySession regression and fixed the controller's numerical handling:
GJK's approximate box-face normals are recovered from the actual face witness;
curved/edge contacts retain their computed normals. The original nudge is retained.
The downhill flag now uses final ground support and actual downward movement,
including ground snapping, instead of the backend's overly broad raw flag.
The new regression runs 300 ticks each across capsule, sphere and box characters,
flat floor/wall slide and autostep on/off. Every tick retains tangential progress,
stable ground height and a false downhill flag. All twelve cases pass.

Use the UPDATED `artifacts/tools/incant_headless` and verify `binary.json` again.
Rerun the complete actual course plus an independent repeat into new directories;
do not reuse old frames as evidence for the corrected engine. Examine every-tick
logs for stalls and flag errors, and review the actual frames for motion quality.
Quantify the remaining step-climb slowdown and whether it is appropriate rounded
capsule traversal or still a defect needing a runtime change. Report limitations
plainly. Replace retained screenshots with useful current frames and rewrite the
result around final evidence, keeping the original bugs and their resolution clear.
No new director feedback beyond the ongoing quality requirement. No UI changes
or native capture are required for this query-only increment. Strict TS, the
existing frame/raw-byte caps and Built-by: claude still apply.

Claude Opus 5.5 over ACP owns all visual choices and pixel judgments. Read
CLAUDE.md. Astra implemented the nonvisual character movement capability on the
existing primitive physics runtime. This is a focused incremental review, not a
phase gate or a production character-art task.

## Actual implementation

`api.computeCharacterMotion({scene_id, entity_id, translation, options?})` is a
read-only primitive shape sweep for a nonsensor velocity-kinematic body with zero
angular velocity. The world uses +Y up. `translation` is desired displacement in
meters for one fixed tick. The result has `translation`, `grounded`,
`sliding_down_slope`, and sorted stable `collisions` entity IDs. Apply the returned
translation divided by `dt` as an ordinary Velocity component command to move on
the next tick. The behavior supplies gravity/jumping; this is not an automatic
gravity system. No solver handles or extra mutation path escape into scripts.

Omitting options uses offset .01m, sliding enabled, max climb and min slide angles
pi/4 radians, snap .2m, autostep disabled. When supplying options, include all
fields: offset, slide, max_slope_climb_angle, min_slope_slide_angle,
snap_to_ground (meters or null), autostep (null or {max_height, min_width,
include_dynamic_bodies}). Both queries and output honor numeric bounds.
The query excludes its own body and sensors and uses its Collider masks. A sweep
costs 16 of the shared 256 physics-query units per tick; a ray costs one. Queries
share the script deadline, including expiry after a native query returns.

Five actual physics tests cover walls/slide, ground, low and high stairs, slope
limits, snap-down, masks, sensors, immediate edits and unchanged solver state.
Script tests cover command application, hot reload, budget sharing and author
isolation. New schemas and SDK types are generated from Rust. Astra will own
further CLI, platform and performance checks.

## Visual task

Create a small, readable real-engine look-dev scene and compiled TypeScript
behavior that demonstrate a capsule/character walking, climbing a low step,
stopping at a taller obstacle, sliding along a wall, and landing after a short
jump. Include a ramp/snap-down demonstration if useful to catch motion defects.
Arrange a clear camera and modest neutral environment with distinguishable test
pieces. Use actual imported glTF/GLB primitives and accurate matching colliders;
physics roots have unit scale, so bake visual dimensions in mesh vertices. Do
not fake movement or use generated frames. All scene edits go through the public
CLI/RPC/incant_cmd. Build geometry source files as needed, then import them.
The prior `handoffs/0022-physics-inspector/tools/physics_lookdev.py` is a reference;
new helpers must refuse to overwrite an existing output directory.

Use `artifacts/tools/incant_headless`, an exact copy of Astra's verified binary,
and `artifacts/tools/binary.json` for its source/hash. This avoids compiling the
same unchanged Rust; do not use another worktree's Cargo target directory.
Run npm ci if SWC needs its local modules. Compile behavior with
`node tools/build_script.mjs source.ts output.js`. Use actual `play --camera`
with 960x540 captures, at most 128 frames and 256 MiB raw data. Choose a capture
interval that keeps the whole test below that cap. Inspect states/logs as well
as frames; report surprising movement rather than hiding it by choosing favorable
angles. Numeric solver fixes belong to Astra. Repeat the same run to check
stable output, while distinguishing local equality from universal determinism.

No Inspector changes are needed: there is no new authored component. Preserve
the left Assets workspace, bottom diagnostics-only dock and neutral shell design.
This handoff does not add native play controls. Native static scene review may
follow; this ACP session has no native CUA, so request specific captures from Astra.

Keep raw project/runtime output in ignored artifacts. Retain a compact useful
set of public screenshots (no credentials/accounts), the reproducible helper and
result.md. Report exact model/transport, commands, motion verdict, real issues,
performance observations and limitations. Commit with Built-by: claude. Do not
publish, merge, change credentials or claim a phase gate.
