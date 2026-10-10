## Priority revision 2: review corrected passing preference

Astra reproduced a Linux CI crowd-arrival failure locally with a 0.0001-radian
rotation of the symmetric crossing fixture. A small fixed lateral nudge was not
robust. The corrected wrapper uses a consistent 45-degree passing preference at
unchanged requested speed when neighboring requested velocities predict entering
the body/margin clearance during the horizon. ORCA still enforces the same
constraints. Twelve rotated/staggered layouts pass unchanged arrival and body
separation requirements; full Rust/Clippy and public 100-agent save/reopen pass.

Review the new immutable binary (source 6df4ef9; SHA in artifacts/tools/binary.json).
The previous source/binary is retained in artifacts/tools/prior-0027/ for provenance.
Do not compile Rust. Preserve the previous result as a historical report before
updating result.md. Reuse the original scene, behavior, camera placements and
2700-tick course initially, so effects of this correction can be compared honestly.
Repeat the same public rendered runs, critical crossing/goal/obstacle windows,
per-tick arrival/clearance/velocity diagnostics and saved-window comparisons.
Report any new jitter, stalls, long detours, overlaps or failed runs. Do not hide
or compensate for defects by changing geometry/goals, inflating margins, changing
body size or relaxing acceptance criteria. If the original course now fails,
report exact evidence and stop rather than silently modifying it.

Use unique ignored output directories. Temporary `caffeinate -s -i` is permitted
and must be reported accurately. One initial Astra public probe failed with the
old generic script error during a 300-tick save prefix while other builds ran;
the same complete probe then passed under temporary sleep prevention. Its exact
cause is unclassified; do not call that earlier error a proven timeout. The new
binary still has the old generic script diagnostic, pending a separate increment.

Commit updated compact evidence and unmodified actual frames with Built-by: claude.
No UI or unrelated scope, and no phase-gate approval.

# 0027: actual local-avoidance motion review

Claude Opus 5.5 through ACP owns visual design, original scene geometry,
camera/look-dev, rendered-pixel inspection and screenshots. Astra owns the Rust
implementation. Use this packet and report exact model/transport evidence.
The host uses the JSON-RPC ACP adapter in tools/handoff/main.py.

Review the new real `api.steerAgents` capability in a clear, polished neutral
look-dev scene. Read docs/spikes/navigation-steering.md and the generated SDK;
tools/probes/game-steering.py is the passing strict TypeScript correctness probe.
Create an original scene demonstrating 100 agents crossing with local avoidance,
plus legible close views of representative encounters and a static obstacle.
Show their actual motion, clearance, endpoint arrival or any stalls/overlap.
Do not hide defects with camera choice or replace outputs with illustrative art.
This is a bounded velocity query, not a crowd planner. A behavior supplies goals,
preferred velocity, height-filtered obstacle polygons and applies ordinary
Velocity/Transform commands. Radius 0.25 m with margin 0.05 m is exercised in
the correctness probes; physical dimensions must match visual dimensions.

Use coherent original geometry and the current neutral palette. Design an
overview and useful close cameras so the scene communicates motion rather than
an unreadable cloud. Markers are real scene geometry, not native debug draw.
Prefer measured actual behavior over ambitious complexity. You may use paths
and physics queries within the shared 256-unit tick budget, but do not claim
physics collision if your scene only applies the steering proposals.

All project changes must use public CLI init/import/RPC and incant_cmd. You may
author source geometry/TypeScript normally and import it. No direct project JSON
mutation and no engine-agent shell access. All art must be original mathematical
geometry or properly sourced; no borrowed credentials or services.

The verified binary is artifacts/tools/incant_headless with metadata in
artifacts/tools/binary.json. Check its SHA before/after every run. Do not compile
unchanged Rust. Use strict TypeScript and tools/build_script.mjs. Run npm ci if
the child worktree lacks dependencies. Repro helpers refuse existing output
directories. Keep raw output in ignored artifacts and commit compact evidence.

Capture actual `play --camera` output at 960x540, no more than 128 frames or
256 MiB raw bytes per run. Keep per-tick numerical diagnostics for separation,
speed, arrival, stalls and obstacle clearance. Repeat independently and compare
actual frames/logs; inspect real frames at the critical crossings. Do not infer
unseen frames or certify cross-device determinism from this one Mac.

Return result.md with scope, exact binary/model, reproducible commands, observed
motion and any correctness findings for Astra. Commit a concise selection of
unedited engine frames, scene/behavior/repro helpers and exact evidence; include
`Built-by: claude`. Do not edit Rust, UI layouts, other handoffs or mutation
routing. No publication, merge or phase approval. This work can use headless
captures while the Mac is locked; native capture is not required for this packet.
