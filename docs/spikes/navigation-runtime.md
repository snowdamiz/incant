# Tiled runtime navigation

This increment implements runtime navigation from selected static colliders and
cooked model meshes, incremental Recast-style tile builds, A* path queries and
funnel smoothing. It is exercised through real TypeScript character movement and
saved games. Local avoidance, off-mesh links, 2D grids, navigation Inspector/debug
draw, production Core Sample integration and device gates remain open.

## Shared authoring and runtime

An entity's typed `NavigationMesh` component contains world-space `settings` and
an explicit `sources` list of `{entity, geometry: "collider" | "mesh"}`. Source
entities must belong to the same scene. There may be eight meshes per project
and 1024 distinct source/entity pairs per mesh. Empty sources produce an empty
mesh. Every edit uses ordinary component commands in `incant_cmd`; malformed
settings, missing references, sensors, moving bodies and moving ancestors reject
the entire command transaction. Scripts may move or assemble static rooms with
Transform/component commands; navigation updates after the transaction.

Model sources use the renderer's default glTF scene, instantiated primitives and
parent-first transforms. Mirroring preserves outward winding. The host supplies
immutable cooked resources with matching asset fingerprints; sources are not
read during baking. Conversion is lazy and memoized, so unused high-detail art
is not forced into the navigation budget. A missing/stale model fails explicitly.
Boxes are exact; spheres/capsules use enclosing 24x12 faceted approximations.
The enclosure test checks every facet against the analytic shape. Voxelization
and contour simplification add their own resolution-dependent approximation.

All changed tiles and graph connections are prepared before publication. Failure
preserves the previous navigation, physics and ECS state. Unchanged source
stamps share the mesh without copying its graph. Within a changed source set,
padded per-tile geometry/configuration hashes retain unaffected tiles. Each
runtime mesh report includes rebuilt/reused/removed tile IDs and a session-local
generation; removing/readding a mesh cannot accidentally reuse an old generation.
Rendering uses an immutable entity projection, so opening or drawing a scene
does not start simulation or bake navigation.

## Script API

```ts
const path = api.findPath({
  scene_id: room.scene_id,
  mesh_entity: navigationEntity.id,
  path: {
    start: [-8, 0, 0], end: [8, 0, 0],
    snap_distance: 1, max_visited: 1000
  }
});
```

`null` means no nearby/reachable path. Bad input, missing meshes and exhausted
budgets throw. Positions are in world space with +Y up. Endpoints snap to the
nearest detail triangle within a bounded 3D distance; stacked floors stay
separate. A* minimizes centroid-edge cost in the polygon adjacency graph. Funnel
smoothing shortens the route within that corridor; height-detail intersections
retain terrain ridges and steps. This is not an exact global surface-geodesic
solver. A separate Dijkstra oracle checks the graph cost on competing routes.

Results include points, corridor indices, visited polygons and generation.
They are snapshots: behavior must replan after source changes. Generations are
not durable save identifiers. Each path query costs 64 of the shared 256 native
query units per tick and observes the script deadline before/after native work.
No mutation, solver handle, filesystem or shell capability enters the sandbox.
The character example applies movement through `computeCharacterMotion` and
ordinary Velocity commands. Saving preserves its behavior-owned route/progress;
loading rebuilds derived navigation from saved scene data and cooked resources.
Reloading identical script source now preserves grown JSON arrays and dynamic
keys instead of treating their initial empty values as a new schema.

## Limits and precision

Pure Rust `rerecast = 0.4.0` supplies voxelization, clearance filters, erosion,
watershed regions, contours, polygons and height detail. `glam = 0.32.1` uses
libm; workspace feature unification is covered by the renderer/physics regressions.
No Bevy navigation plugin or alternate physics engine is introduced.

Each mesh is bounded to 256 tiles, 32768 source triangles/98304 vertices, 32768
polygons and 262144 detail triangles. Tile sides are 16/32/64/128 cells; horizontal
cells are 0.05–2 m and vertical cells 0.025–1 m. Radius, height, climb, slope,
vertical extent and distant-coordinate precision are checked before allocation.
The bake caps projected raster work at eight million cell visits, tile spans at
262144, tile polygons at 4096 and border comparisons at four million. Paths cap
visited polygons at 32768, corridor/output points at 4096 and height-detail work
at two million units. Rebuilds are synchronous. These are explicit initial limits,
not a streaming-world or crowd simulation implementation.

## Verification and measured scope

The [machine-readable evidence](evidence/navigation-runtime-2026-10-10.json)
records the actual binary hashes and public probe. Run:

```sh
./tools/cargo test -p incant_nav -p incant_core -p incant_physics -p incant_assets -p incant_script --release
python3 tools/probes/game-navigation.py artifacts/navigation-example
./tools/cargo run -p incant_nav --example budget --release
```

Tests cover tile reuse/removal, rollback, circle clearance, narrow corridors,
low ceilings, terrain heights, slope/climb restrictions, stacked floors, graph
cost optimality, scene isolation, cooked model hierarchy, stale resources,
script budget failures, hot reload and saved paths. The public workflow imports
a mathematical glTF floor, authors the scene through RPC, exercises durable
Undo/Redo and invalid rollback, deletes the source file, then runs a strict TS
character around a wall and replans after a room edit. Repeated gameplay and
save/resume agree exactly in this local fixture.

The measured small desktop fixture has one-tile rebuild p95 0.168 ms and path
p95 0.004 ms. A sixteen-tile fixture rebuilding four affected tiles has rebuild
p95 0.726 ms and path p95 0.007 ms. Each uses 100 samples after 20 warmups on this
arm64 Mac; these are workload measurements, not maximum-scene guarantees. The
one-tile-under-2-ms mobile gate and hundred-agent steering gate remain open.
The six-platform probe now performs a real collider bake and incremental rebuild;
local macOS execution passes, while WASM/iOS compilation is recorded separately.
Claude's actual rendered motion review and new hosted checks are pending.
