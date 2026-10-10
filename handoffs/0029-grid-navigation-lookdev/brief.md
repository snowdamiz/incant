# Weighted grid navigation: actual rendered movement review

Claude Opus 5.5 through ACP owns all scene look-dev, visual geometry, cameras,
rendered judgment and screenshots. Read CLAUDE.md, docs/spikes/grid-navigation.md
and tools/probes/navigation-grid.py. This scoped increment is cell navigation;
it does not claim the future sprite/tilemap renderer or 2D physics milestone.

## Scope and acceptance

Author a compact, professional, clearly legible real-engine scene demonstrating
`api.findGridPath` with weighted cells and a door changing route availability.
Use the existing 3D renderer for a top-down or similarly clear grid demonstration;
you choose the visual design and cameras. Distinguish cheaper and more expensive
walkable areas, blocked cells, destination and a moving actor without making the
scene visually noisy. This is a correctness look-dev scene, not a mock editor.

Drive actual motion from the returned adjacent `cells`; never substitute a
handwritten path or infer a diagonal through blocked corners. The actor may
interpolate between cell centers with ordinary Transform commands. Queries are
point/cell routes: account for the actor's visual footprint in your authored
obstacles, and state explicitly that Rapier collision is not being promised.
Keep behavior logically small and bounded. Verify the declared cost metric:
entering-cell weight * 1000 cardinal or * 1414 diagonal, no start-cell charge.

Show a cheap route, a changed more expensive route after a normal shared
NavigationGrid component command, a temporarily unreachable destination with a
clear stopped/waiting state, then reopening and arrival. Include an independent
blocked-corner query proving it returns null or the legal orthogonal detour.
Edits publish on the next tick; do not expect the same update to query its staged
commands. Do not close a door on an actor and then treat that as a planner defect.

Save while disconnected, resume after reopening and compare the final behavior
and runtime state against uninterrupted playback. Repeat the authored course and
compare actual frames and logs. Capture useful approach, changed-route, stopped,
restart and arrival moments. Review route/obstacle alignment, visible corner
cutting, unexplained snaps, path changes and stop/restart clarity. If you find a
correctness defect, preserve a small reproducer with exact values for Astra;
do not modify Rust to hide it or approve an unverified result.

## Interfaces and execution

Use only the supplied immutable `artifacts/tools/incant_headless`, verifying it
against `artifacts/tools/binary.json` before and after. No Rust compilation is
needed. Install pinned node dependencies with npm ci if absent. Strict-check
TypeScript and compile with tools/build_script.mjs. Existing handoff 0026 helpers
show real `play --camera` capture mechanics; the public grid probe shows the
query/command/save contract. Helpers must refuse existing output directories.

Every authored entity, component or asset change goes through public incant_cmd
RPC/import commands, with stable ULIDs and provenance. Original mathematical
source glTF may be authored and imported. No direct project JSON edits, runtime
shell access, credentials or accounts. Do not edit UI, other handoff packets,
mutation routing or engine code. Do not publish, merge, sign or use external art.

Each grid axis is 1..1024, at most 65536 total cells and eight grids per project.
Costs are 0 blocked, 1..1000 walkable; diagonals require both adjacent orthogonal
cells walkable. Each query costs 64 of 256 shared units per update. Search limits
are 1..65536 expansions and 4096 returned cells. Out-of-bounds, exhausted budget
or oversized route errors are distinct from an unreachable null route. Generation
is a session cache revision, can reset on save reopen and is not durable identity.

Keep scene small and use 960x540 real-engine frames. Each capture invocation is
limited to 128 frames and 256 MiB raw output. Store raw evidence under ignored
artifacts/0029-grid-navigation. The supplied engine uses a 50 ms executing-thread CPU budget. Bounded synchronous
native queries count toward it, while host pauses do not. Record any failures
and any temporary sleep assertions used; CPU time is not a frame-time guarantee.
Computer use is authorized, but the Mac is currently locked: use the supported
headless renderer and never attempt to bypass the lock. Native UI is out of scope.

Return result.md with exact model/ACP transport, source and binary hashes,
reproducible commands, numerical and rendered results, explicit limitations and
a clear scoped verdict. Commit a concise useful set of unedited engine frames,
helper scripts and compact evidence. All visual decisions and screenshots must
be yours; no mock or generated picture substitutes for engine output. Commit with
Built-by: claude. Phase gates and finished-game quality remain unapproved.
