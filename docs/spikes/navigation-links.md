# Authored off-mesh navigation links

`NavigationMesh.links` adds up to 128 explicit connections between walkable
surfaces. Each connection has a stable canonical ULID, world-space start/end,
endpoint snap distance, enabled and bidirectional flags, and a nonnegative
extra distance cost. Older documents omit `links` and load an empty list.
All authoring and script edits use normal component commands through incant_cmd.

The bake resolves enabled endpoints to detail triangles on that same mesh.
Missing landings reject the staged rebuild; disabled links may remain unattached
while a room is being assembled. Link-only changes reuse every geometry tile and
advance the mesh generation. Input order does not change route selection. Edits
to geometry re-resolve all enabled endpoints before publication. Endpoint work is
bounded to two million triangle checks; the 128-link cap is not a promise that
128 links fit that budget on a maximum-size mesh.

A* considers each directed connection as a graph transition, charging its snapped
straight-line length plus extra cost. Costs are finite and in [0, 100000], so the
Euclidean heuristic remains admissible. The existing search visit/corridor limits
still apply. The route is optimal for this portal/link graph, not all continuous
paths over a surface.

## Traversal contract

`api.findPath` returns `traversals`, an ordered array of
`{link_id, from_index, to_index, reversed}`. Every traversal identifies consecutive
points crossing an explicit connection. Each walking leg is funneled and height
sampled independently; no smoothing crosses a link. The optional wider visibility
repair currently runs only on routes without links. Heights on walking legs have
the existing quantized-surface limitations.

Gameplay must interpret the link ID and implement its own jump, ladder, teleport
or other transition through ordinary commands. The query does not animate,
teleport, solve a trajectory, reserve capacity, check traversal collisions or
promise that a proposed movement is safe. Scripts must not feed a link segment
straight to ground steering. Link endpoints and returned routes are snapshots;
replan after edits. Save behavior-owned progress and rebuild derived navigation
when reopening a saved game. `generation` is a session-local rebuild counter,
not a persistent content identity. Reopening starts its counter again. Runtime
rebuild reports describe the current synchronization: an unchanged tick moves
previously rebuilt tiles into reused and clears removed tiles. Compare generations
at the edit tick when measuring link-only reuse; later reused counts alone do not
prove that a rebuild happened.

## Atomic runtime publication

The play session now stages simulation and script commands on a disposable
simulation CommandBus. It validates the derived ECS, physics and navigation
before publishing the staged document, script state, timers or logs. A valid
component schema can still describe an impossible enabled landing; that error
now leaves committed project and script state intact. Physics may already have
stepped, so a failed play session remains explicitly unsaveable and must restart
or restore a save; this is not full rollback of the physics step or live input.

## Verification

Library tests exercise two one-way connections across three islands, bidirectional
reverse traversal, stable ordering, alternative route costs, disabled endpoints,
link-only tile reuse and invalid rebuild rollback. Script tests toggle links
through commands, preserve returned traversal data across save/reload, and reject
an invalid landing without committing state, commands, timers or logs.

`python3 tools/probes/navigation-links.py artifacts/navigation-links-cli` uses
public init/RPC commands, durable Undo/Redo, strict TypeScript and ordinary
Transform commands. It scripts a parabolic gap traversal and saves at tick 60,
30.56% through the link. Resuming to tick 180 matches uninterrupted and repeated
runs exactly; authoring files remain unchanged. It makes no physics or rendered
quality claim. The common platform smoke also checks a directed connection,
reverse rejection, tile reuse and failed endpoint rollback.

At source `ccd6d33`, full local verification passes 289 Rust tests, 40 GPU
checks, Clippy, 315 UI tests/build, five tool tests, generated contracts and strict
TypeScript. macOS executes the common platform smoke; WASM/iOS targets compile.
The native unsigned package builds at integrated source `8d92904`. The public
link, navigation and 100-agent steering probes pass. Exact evidence is in
[evidence/navigation-links-2026-10-10.json](evidence/navigation-links-2026-10-10.json).

Claude Opus 5.5's [rendered review](../../handoffs/0028-off-mesh-lookdev/result.md)
uses the same immutable link binary: two independent projects/runs match all 593
frames and all 26 log files. A save at tick 801, 49.30% through a return drop,
reopens with all 49 mid-drop frames identical and identical final entities.
Script state and one later log differ only in the session-local generation
counter, as expected after a rebuild. Link toggles reuse all ten geometry tiles.
Temporary sleep prevention was active; no script deadline failure occurred.

The review exposes a visible 5 cm hover when gameplay places feet directly on
quantized navigation heights. Source-conforming navigation heights remain open;
current gameplay must use physics or raycast grounding. This scene intentionally
has no physics bodies and validates explicit logical traversal only. Centre
endpoints also snap about 5 cm inward; movement uses the returned positions.

The dependency steering PR subsequently exposed a Linux symmetric-crowd stall.
Its correction, updated rendered review and final integrated/hosted checks are
required before merging this increment. Native link editing/debug visualization and
grid navigation remain separate outstanding Phase 1 work; this increment does
not approve a phase gate.
