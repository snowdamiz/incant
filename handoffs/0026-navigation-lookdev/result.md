# 0026 navigation rendered-motion review: result

## Status

Final verdict on binary `f63fc28c…352ef2` (source `afeac5f`): the portal-entry
search **fixes the v6 detour.** Astra's fixed-start diagnostic reproduces
exactly: 13.634 m with no (−3.3, −1.8) kink. On the live course the character
follows real `api.findPath` routes:

- It stays within 1.0 mm of the returned polyline, except for a 3.0 cm
  autostep slip on the diagonal ridge climb.
- It is grounded every tick, never touches a wall and arrives exactly.
- The room edit rebuilds three tiles and the replan reroutes.
- The pocket query returns `null` both times.

Probing the final mesh turned up **two actual navigation failures for Astra.**
Neither is caused by the new search, and both are reproducible with fixed
coordinates:

1. **Missing walkable floor.** The navigation mesh omits a block of open floor
   east of the pillar. It runs from the pillar's eroded disc to the x = −1.6 and
   z = 1.4 tile borders: x −2.3 to −1.7 and z 0.7 to 1.3 on a 0.2 m grid. Nothing
   physical is there. The replanned route bends at the hole's tile corner
   (−1.6, 1.4) and pays about 0.59 m (an analytic estimate). The polygon count is
   the same 145 as in v5 and v6, and the v5/v6 routes bent at the hole's other
   corner, (−1.6, 0.5). So this predates the new search; the earlier tile kinks
   were its symptom.
2. **Valid queries throw.**
   - `findPath` from (4.7, 1.7) to (4.71, 1.7) on open floor throws `invalid
     navigation input: funnel did not cross its navigation corridor`.
   - The reverse query throws `smoothed navigation segment left its polygon`.
   - Shifting either point by 1 cm succeeds, and so does a vertical query.

   The docs reserve throws for bad input or exhausted budgets. An uncaught throw
   ends a gameplay behavior; my own map probe died this way before I added
   `try`/`catch`.

The resolution/clearance tradeoff stays as Astra decided:

- The configured radius is a minimum, and convex corners keep 0.50–0.60 m.
- At 0.1 m cells, doorways need at least 1.2 m; at 0.05 m cells, at least 1.1 m.
  A 0.4 m-radius agent physically fits anything wider than 0.8 m.

Quantized heights (+5 cm) and step smearing also remain. Neither is exact
geometry.

This verdict does not approve a phase gate or mark the phase complete. Portal-
graph optimality is not a global continuous shortest-path claim. Steering,
off-mesh links, 2D navigation, the Navigation Inspector and native debug draw
are not claimed.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP, on host-supplied evidence. The director states that Astra
invoked the JSON-RPC ACP adapter in `tools/handoff/main.py`. From inside the
session I can see only a Claude Code agent session in this worktree. I cannot
inspect the transport layer myself.

### Priority revision 3 acknowledged (2026-10-10)

The revision covers Astra's switch to directed portal-entry A*, with funnel and
connected visibility repair retained. All of it was applied:

- **New binary, new outputs.** I verified the binary against `binary.json`
  before and after every run. The final runs are `v7` and an exact `v7-repeat`,
  with the same scene, cameras, marker pools and 600 ticks. `v3`, `v5` and `v6`
  are kept as history. No camera was re-authored.
- **Fixed-start diagnostic.** I verified it separately from the live course with
  `detour_probe.ts`, as asked.
- **Live course review.** I reviewed motion, wall clearance, grounding, marker
  coverage and arrival on the new route. The new route passes the other side of
  the pillar, so its lengths are not compared with v6's.
- **Residual kink.** I traced the remaining kink to a mesh hole, then mapped the
  whole room's walkable floor. That map exposed the throwing query.
- **Doorway sweep.** I re-ran it on this binary.
- **Unchanged areas.** No Rust, core algorithm, UI layout, other packet or
  mutation routing changed. No native capture was attempted because the Mac is
  locked.

## Before / after findings

| Finding | v3 | v5 | v6 | v7 (final) |
|---|---|---|---|---|
| Search | centroid A* + funnel | same | + visibility repair | **portal-entry A*** + funnel + repair |
| Fixed-start diagnostic, (−3.44, 0) → goal after the edit | — | 15.27 m, bend at (−3.3, −1.8) | 15.27 m, same bend | **13.634 m, no bend** |
| Live replan start / leg bend | (−3.40, 0.1) / none | (−3.44, 0) / (−3.3, −1.8) | same as v5 | (−3.29, 2.0) / **(−1.6, 1.4) at the mesh hole** |
| Live route lengths, first / replanned | 13.45 / 14.56 m | 13.55 / 15.27 m | 13.55 / 15.27 m | 13.29 / 15.85 m (different replan start) |
| Polygons visited, first / replanned | 41 / 36 | 110 / 87 | 376 / 295 | 279 / 421 |
| Walkable hole east of the pillar | not probed | present (bend at (−1.6, 0.5)) | present (same bend) | **mapped: x −2.3 to −1.7, z 0.7 to 1.3** |
| Valid query throws at (4.7, 1.7) | not probed | not probed | not probed | **throws** |
| Route clearance at convex corners (configured 0.4 m) | 0.389 m minimum (clip) | 0.50–0.60 m | 0.50–0.60 m | 0.50–0.60 m |
| Narrowest passable doorway, 0.1 / 0.05 m cells | — | 1.2 m / — | 1.2 / 1.1 m | 1.2 / 1.1 m |
| Arrival | Tick 540 | Tick 562 | Tick 562 | Tick 578, exact |

## Final evidence (`v7`, identical in `v7-repeat`)

### Scene (unchanged since v5)

The room is 16 × 10 m. A partition at x = 0 has doorway A (z 0.4–2.0) and
doorway B (z −4.4 to −2.8). The remaining pieces are:

- a capsule pillar, r 0.45 m, at (−3.6, 1.0), which is a curved `collider`
  source
- a 0.15 × 0.6 m ridge at x 1.0–1.6, a cooked `mesh` source
- a screen wall
- a pocket behind a 0.5 m slit
- a muted-blue barrier that is moved by script

The floor is a cooked `mesh` source with 1 m checker tiles. All geometry is
original mathematical glTF with matching analytic colliders and unit scale. The
navigation tiles are 3.2 m, with borders at x = −4.8, −1.6, 1.6, 4.8 and
z = −1.8, 1.4.

A kinematic 1.6 m capsule, r 0.3 m, walks the returned polyline at 2 m/s. Each
step goes through `computeCharacterMotion` with autostep, and the result is
applied as a Velocity command. The edit happens at the end of tick 100, and the
character replans at tick 101.

**Path markers are not native debug draw.** They are 192 render-only mesh
entities moved onto the returned points, 12 mm above each returned height. The
colour roles are:

- **Amber** marks the live route.
- **Grey** marks the superseded route.
- **Blue** marks the edited barrier.
- **Brick** marks the null target.

### Queries, edit, rebuild and marker coverage

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 15 points, 13.29 m, south of the pillar, via doorway A, generation 1 |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.291, 2.0) | 26 points, 15.85 m, via doorway B, diagonal ridge crossing, around the screen wall's south tip, generation 2 |
| 101 | Route from start to pocket | **`null`** |
| 578 | Arrived | Final position equals the goal to 1 mm |

**Marker coverage.** Every drawn route is fully covered: 15, 15 and 26 points,
with pools of 48.

**Build report.** I stopped playback at tick 100 to see the edit's build:

- Rebuilt tiles (2,1), (2,2) and (2,3); reused the other 17.
- Polygons dropped from 145 to 120; generation went from 1 to 2.

### Motion, every tick (`trace_check.py`)

| Check | Result |
|---|---|
| Deviation from the active returned polyline | max **3.0 cm** (t344, diagonal ridge climb); ≤ 1.0 mm on every other tick before arrival |
| Stalls below 90% of the requested step | Only t341–350, the diagonal ridge climb, at 77–83% |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. No wall, pillar, barrier or pocket contact |
| Closest capsule gap | Partition 0.200 m (t317, doorway B jamb), pillar 0.250 m, screen wall 0.295 m |
| Height on flat floor or ridge top | At rest height; the worst error is the 1 cm spawn settle at t1 |
| Largest vertical move per tick | 2.3 cm (t344) |

**Diagonal ridge crossing.** The route meets the ridge face at about 57° to
the face normal. Autostep resolves the climb over ticks 340–357. It cuts the
component of motion into the face while sideways motion continues, so the
capsule drifts 3 cm along the face before the behavior's next step brings it
back. This is character-controller behaviour, as classified in 0023, not a
navigation error. It is visible in the ridge and profile frames as a short
sideways slip.

### Astra's fixed-start diagnostic, verified separately

`detour_probe.ts` applies the same barrier edit on tick 1. It then issues one
`findPath` per tick on the edited mesh (generation 2):

- **(−3.44, 0) → goal: 13.634 m, 23 points.** It runs straight to (−0.6, −3.1)
  at doorway B, with no (−3.3, −1.8) tile kink. That matches Astra's preliminary
  run exactly.
- **(−3.44, 0) → (−0.3, −3.3): 4.569 m,** against 5.160 m in v6. The 0.59 m
  shortcut that I found in v6 is now returned.

### Navigation failure 1: missing walkable floor east of the pillar

The live replan goes (−3.291, 2.0) → (−3.2, 2.0) → **(−1.6, 1.4)** → (−0.7, −2.9).
(−1.6, 1.4) is a tile corner with no obstacle near it. I probed the edited
mesh:

- Every route from points near the pillar toward doorway B passes through
  (−1.6, 1.4). This holds even from (−2.0, 1.4).
- Endpoints at (−2.0, 1.0) and (−2.0, 0.8) snap away, to (−2.0, 1.4) and
  (−2.0, 0.5).
- Straight crossings of the z = 1.4 border at x = −2.0 to −2.4 detour via
  (−1.6, 1.4), (−1.6, 0.5). At x = −1.0, in the next tile, the same crossing is
  a straight 2-point path.

`walkable_probe.ts` then maps the room. It tests every 0.2 m grid point with a
1 cm query at the quantized surface height and a 6 cm snap distance, then a
second pass at y 0.12 with a 0.1 m snap that tolerates height smearing.
`walkable_check.py` flags grid points that are at least 0.75 m from every
obstacle, floor edge and ridge face but are not walkable. Here 0.75 m is the
0.4 m radius, plus the measured ~0.2 m safety margin, plus one cell and slack.

- **Both passes flag the same 16 points:** x −2.3 to −1.7 by z 0.7 to 1.3.
- The block is bounded by the pillar's eroded disc on the west and by the
  x = −1.6 and z = 1.4 tile borders on the east and north. It reaches about
  z 0.6 on the south.
- The pillar's own eroded disc, radius about 1.0 m, is otherwise correct and
  symmetric.
- No other room area is missing. The tight pass also flagged 3 points at
  (2.5–2.7, 1.5–1.9). They are walkable in the tolerant pass, so they are floor
  raised by height smearing rather than holes.

The fault probably lies in this one tile's region or contour step. The hole ends
exactly on two tile borders, and the navigation polygon count is the same 145 in
v5, v6 and v7. v5 and v6 bent at the hole's south-east corner, (−1.6, 0.5). So
the earlier "tile kinks" near the pillar were symptoms of this hole.

Cost on the live leg: the shortest path around the pillar's eroded disc, radius
1.0 m, from the replan start to (−0.7, −2.9) is 5.60 m. The returned leg is
6.19 m, so the hole costs about **0.59 m**. This is an analytic estimate,
because the engine cannot route through the missing floor.

### Navigation failure 2: valid queries throw

`error_probe.ts` runs one query per tick on the unedited mesh:

| Query | Result |
|---|---|
| (4.7, 0.05, 1.7) → (4.71, 0.05, 1.7) | **throws** `invalid navigation input: funnel did not cross its navigation corridor` |
| (4.7, 0, 1.7) → (4.71, 0, 1.7) | **throws**, same message |
| (4.71, 0, 1.7) → (4.7, 0, 1.7) | **throws** `invalid navigation input: smoothed navigation segment left its polygon` |
| (4.7, 0, 1.7) → (4.7, 0, 1.71) | 2-point path |
| (4.65 or 4.75, 0, 1.7) → +1 cm in x | 2-point path |
| (4.7, 0, 1.6 or 1.8) → +1 cm in x | 2-point path |
| (4.7, 0, 1.7) → (4.8, 0, 1.7), and → goal | 2-point paths |

This is open floor, 1.1 m from the nearest obstacle. It looks like a degenerate
short segment that lies along a polygon edge. It was found by the room map,
which reached this point at tick 676 and died until I wrapped each query in
`try`/`catch`. No live-course query hit it. Older binaries were not retained, so
I cannot say when it began.

### Doorway resolution tradeoff (re-run on v7)

Each width is a separate 8 × 6 m room with a 0.3 m partition and one centred
doorway. The radius is 0.4 m and every other setting matches the scene.

| Doorway width (m) | 1.00 | 1.05 | 1.10 | 1.20 |
|---|---|---|---|---|
| 0.10 m cells | null | null | null | yes |
| 0.05 m cells | null | null | yes | yes |

The thresholds match v6 and Astra's fixture. The full v6 sweep down to 0.8 m
remains in `doorway-v6-*`. The configured radius is a minimum clearance, and the
cost is conservative passage loss. It is not a claim that every geometric fit is
found.

### Remaining height detail (explicit limitation)

Returned points describe the quantized surface, +5 cm on this geometry. Physics
grounds the character, so the excess shows only in the markers.

- **First route, east face.** Points at 0.19–0.17 for 0.2 m beyond the face,
  then a straight segment to (3.251, 0.05). The strip descends over 1.65 m of
  flat floor and floats up to 12 cm above the quantized level near the face.
- **Replanned route, diagonal crossing.** The rise begins 0.11 m before the face,
  with a 4 cm dip, going (0.929, 0.155) → (0.994, 0.113) → (1.055, 0.2). The
  descent reaches floor level 0.29 m past the east face. There is a 1.2 cm bump
  at (0.724, 0.062).

None of this is exact geometry.

### Rendered review

I inspected real frames from five cameras:

- **The search change.** The first route now passes south of the pillar.
  Frames 100 and 105 show the one-tick window and then the reroute.
- **The bend at the hole.** It shows in the plan view at t150 as a knee east of
  the pillar.
- **The ridge.** The diagonal climb and the short sideways slip at t345 are
  visible in the ridge and profile views.
- **Clearance and arrival.** The character goes through doorway B and around the
  screen wall's south tip with clear space, and stands on the goal pad at frame
  580.

**Pillar camera occlusion.** The pillar camera is on the pillar's north side.
The new route runs south of the pillar, so around t125 the character is partly
behind the pillar in that view. I did not move the camera. The plan and overview
views show that stretch fully.

There is no discrepancy between the returned path and the rendered movement,
apart from the logged 3 cm ridge slip.

### Repeat stability (local equality only)

`v7` and `v7-repeat` were each built from scratch.

- All **605/605 frames** across five cameras are byte-identical.
- The logs are identical across both runs and all five cameras.
- Each `report.json` is equal once engine-minted IDs are normalized and
  `wall_ms` is removed.
- The projects are equal once minted IDs and provenance transaction ULIDs are
  normalized.

This was measured on one machine (Apple M5 Pro) with one binary. It is not
evidence of cross-device determinism.

## Commands (repo root)

```sh
shasum -a 256 artifacts/tools/incant_headless        # f63fc28c…352ef2 = binary.json (before and after)
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v7 --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v7-repeat --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v7   # refused, exit 1
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v7 \
  artifacts/0026-navigation/v7/overview.logs.jsonl
# Probes: build each into a NEW directory, then play on the v7 project with --log-output.
node tools/build_script.mjs handoffs/0026-navigation-lookdev/tools/detour_probe.ts artifacts/0026-navigation/detour-v7c/detour_probe.js
artifacts/tools/incant_headless play artifacts/0026-navigation/v7/navigation.incant.json --ticks 25 \
  --compiled-script artifacts/0026-navigation/detour-v7c/detour_probe.js --log-output artifacts/0026-navigation/detour-v7c/probe.logs.jsonl
node tools/build_script.mjs handoffs/0026-navigation-lookdev/tools/walkable_probe.ts artifacts/0026-navigation/walkable-v7-room3/walkable_probe.js
artifacts/tools/incant_headless play artifacts/0026-navigation/v7/navigation.incant.json --ticks 1010 \
  --compiled-script artifacts/0026-navigation/walkable-v7-room3/walkable_probe.js --log-output artifacts/0026-navigation/walkable-v7-room3/probe.logs.jsonl
python3 -I handoffs/0026-navigation-lookdev/tools/walkable_check.py artifacts/0026-navigation/v7 \
  artifacts/0026-navigation/walkable-v7-room3/probe.logs.jsonl
# Tolerant pass: the same probe with PROBE_Y = 0.12 and PROBE_SNAP = 0.1 (artifacts/0026-navigation/walkable-v7-room-tolerant).
node tools/build_script.mjs handoffs/0026-navigation-lookdev/tools/error_probe.ts artifacts/0026-navigation/error-v7/error_probe.js
artifacts/tools/incant_headless play artifacts/0026-navigation/v7/navigation.incant.json --ticks 13 \
  --compiled-script artifacts/0026-navigation/error-v7/error_probe.js --log-output artifacts/0026-navigation/error-v7/probe.logs.jsonl
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v7-cell0.10 1.0 1.05 1.1 1.2
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v7-cell0.05 --cell-size 0.05 1.0 1.05 1.1 1.2
artifacts/tools/incant_headless play artifacts/0026-navigation/v7/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/v7/navigation_course.js     # also --ticks 1 and 99
artifacts/tools/incant_headless script artifacts/0026-navigation/v7/navigation.incant.json \
  artifacts/0026-navigation/v7/navigation_course.js --ticks 600             # ×3, timing
```

Probe attempts that failed are kept in their own directories:

- `detour-v7` and `detour-v7b` are earlier query sets.
- `walkable-v7` used a 5 cm snap from y = 0, which marked everything unwalkable.
- `walkable-v7-room` and `walkable-v7-room2` died on the throwing query.

Each scene run is 600 ticks with `--capture-every 5`. That gives 121 frames at
960×540, or 239.3 MiB, under both caps. Every report has `completed: true` and
889 script commands, adapter `Apple M5 Pro` and 207 model entities.

Timing on this Mac only; these are not platform budgets. `script --ticks 600`
had a p95 of 2.00–2.19 ms per tick over three runs, with a maximum of 3.5 ms.
v6 measured 2.04–2.09 ms.

## Screenshots

All 11 are under `handoffs/0026-navigation-lookdev/screenshots/` (600 KB). They
replace the v6 set. Each is an unedited `play` frame from `v7`, byte-identical
in `v7-repeat`.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier in doorway A; first route (south of the pillar) still drawn |
| `overview-t105-replanned.png` | Superseded route in grey; new amber route via doorway B |
| `overview-t580-arrived.png` | Character on the goal pad |
| `plan-t050-south-of-pillar.png` | New first route around the pillar's south side |
| `plan-t150-tile-corner-bend-at-mesh-hole.png` | The (−1.6, 1.4) bend caused by the missing floor east of the pillar |
| `plan-t460-screen-wall-south-tip.png` | Rounding the screen wall's south tip |
| `pillar-t075-passing-pillar.png` | Passing the pillar (far side from this camera) |
| `ridge-t345-diagonal-ridge-climb.png` | Diagonal autostep climb, at the moment of the 3 cm slip |
| `profile-t105-returned-route-heights.png` | Returned heights at the ridge: early rise with dip; long east ramp of the first route |
| `profile-t360-on-ridge.png` | On the ridge top during the diagonal crossing |

In the profile camera, which looks south, east is screen-left. The raw output is
in the ignored `artifacts/0026-navigation/`:

- `v7`, `v7-repeat`, the `*-v7*` probes and `doorway-v7-*` are final.
- Earlier versions are kept as history.
- `review-v7` holds review sheets made from real frames.

## Changed paths (this revision)

- `handoffs/0026-navigation-lookdev/result.md`: rewritten.
- `handoffs/0026-navigation-lookdev/tools/detour_probe.ts`: now has the
  fixed-start, replan-leg and tile-border queries.
- `handoffs/0026-navigation-lookdev/tools/walkable_probe.ts`: new walkability
  map probe.
- `handoffs/0026-navigation-lookdev/tools/walkable_check.py`: new; compares the
  map with the analytic geometry.
- `handoffs/0026-navigation-lookdev/tools/error_probe.ts`: new exception repro.
- `handoffs/0026-navigation-lookdev/tools/tsconfig.json`: now includes the
  probes.
- `handoffs/0026-navigation-lookdev/screenshots/*.png`: replaced with 11 `v7`
  frames.

The scene helper, behavior, `trace_check.py` and `doorway_sweep.py` are
unchanged.

## Limitations

- **Course coverage.**
  - The scene covers one flat room, one agent size and one static edit.
  - The doorway sweep covers axis-aligned, centred doorways.
  - The walkability map is a 0.2 m grid of the unedited mesh with a 0.75 m
    expectation margin. Holes smaller than that, or within the margin, are not
    detected.
  - The 0.59 m hole cost is an analytic estimate.
- **Untested.**
  - The throwing query was found at one grid point. Other degenerate segments
    may exist.
  - No native screenshot, hosted CI, Rust tests or Clippy were run; no Rust
    changed.
- **Markers.** They show returned points only.
- **Logs.** Captures every 5 ticks cannot show single-tick events, so per-tick
  claims come from the logs. Logged values are rounded to 1 mm.
- **Appearance.** There is no character art or animation, and shading is flat
  `material_preview`.

## Open items for Astra

1. **Missing walkable floor east of the pillar.** The region is x −2.3 to −1.7,
   z 0.7 to 1.3, and is reproducible with `walkable_probe.ts` and
   `walkable_check.py` on this scene. Which bake stage drops it? The hole ends on
   the x = −1.6 and z = 1.4 tile borders. A regression could assert that
   (−2.0, 0.05, 1.0) is walkable and that a query from (−2.0, 1.8) to
   (−2.0, 0.8) is a straight line.
2. **Throwing queries.** (4.7, 1.7) → (4.71, 1.7) and its reverse throw on valid
   input. Should degenerate funnel or repair cases fall back to the raw corridor
   path instead of throwing?
