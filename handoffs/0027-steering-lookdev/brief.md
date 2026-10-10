## Priority revision 3: verify the near-head-on correction

Your revision-2 review correctly blocked the broad passing rule: 26/100 at goal
at tick 2700 and 11 never arrived by 4990. Preserve that report, evidence and
frames under a separate historical directory before replacing current results.
The original revision-1 history must remain intact too.

Astra first tried limiting the preference to any opposing traffic. The unchanged
public course still failed (43/100 at tick 2700). The current correction applies
the 45-degree preference only when desired directions are within about eight
degrees of antiparallel AND their paths predict body/margin conflict during the
horizon. Other crossing, stationary and co-directed traffic retains its original
objective. All LP constraints and speed caps remain unchanged. A stable tiny
ID-based jitter was tried and rejected after it failed the crossing regression.

Review the immutable binary sourced from 4c495e9 (exact SHA/source metadata in
artifacts/tools/binary.json). A new numerical Rust regression ports your existing
plaza routing and initial positions/goals; it fails the previous opposing rule
and passes the current rule with all 100 settled, as do all 12 rotated/staggered
crossing layouts. That port is numerical coverage, not an exact public replay.
The unchanged public TypeScript run must also pass before this review begins.

Do not compile Rust. Reuse exactly the original scene, behavior, goals, camera
placements, margins and 2700-tick course. Repeat both public rendered runs and
critical crossing/goal/obstacle windows, then compare actual frames/logs and
saved-window results. Inspect jitter, stalls, detours, overlap and final goal
retention. Keep genuine failures and never compensate for the implementation by
altering the course or weakening thresholds. Report remaining visible issues.
No engine changes, unrelated UI work or phase-gate approval.

Use fresh ignored output directories. Prior binaries/metadata remain in
artifacts/tools/prior-0027/ and artifacts/tools/prior-0027-r2/. Verify the current
binary hash before and after runs. Temporary `caffeinate -s -i` is permitted;
report it accurately. This binary still has the old generic wall-clock script
failure, pending separate diagnostics/CPU-budget increments. Do not attribute
an unclassified error to a proven timeout.

Commit updated compact evidence and unedited actual frames with Built-by: claude.

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
