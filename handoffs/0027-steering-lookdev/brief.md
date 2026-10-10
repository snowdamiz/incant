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
