# 0026 navigation rendered-motion review: result

## Status

Final verdict on binary `6bacde25…62ce6bc` (source `141a835`): **the character
follows real `api.findPath` routes in rendered motion.** Navigation causes no
stalls and there is no wall contact. The character stays within 6.1 mm of the
returned polyline and arrives exactly. The room edit rebuilds three tiles, and
the next-tick replan reroutes. The pocket query returns `null` both times.

The clearance refinement removed the earlier corner clip. The ridge's near face
is now crisp. The re-review found **two new problems**:

1. **Doorway false negatives (correctness concern).** Every doorway up to
   1.1 m wide returns `null`, with a 0.4 m agent radius. That geometry should
   be passable: a 0.4 m radius needs 0.8 m. The first passable width is
   1.2 m. Corners now keep 0.50–0.60 m of route clearance, not 0.4 m.
2. **A large tile-boundary detour (quality).** The replanned route bends
   1.14 m sideways to a tile-row boundary and adds 0.60 m (13%) to that leg.

Two issues remain:

- Height smearing past the ridge's far face is not reduced, and floor bumps
  reach 10 cm.
- The 5 cm quantized height is now a documented scope decision.

This verdict does not approve a phase gate or mark the wider phase complete. It
does not claim that steering, off-mesh links, 2D navigation, the Navigation
Inspector or native debug draw exist. It certifies no platform budget or
cross-device determinism.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP, on host-supplied evidence. The director's revision states that
Astra invoked the JSON-RPC ACP adapter in `tools/handoff/main.py`. From inside
the session I can see only a Claude Code agent session in this worktree. I
cannot inspect the transport layer myself.

### Priority revision acknowledged (2026-10-10)

The revision asks for a re-review after Astra's correctness changes at `141a835`.
Those changes are:

- erosion compensation for chamfer-distance error
- 0.25-cell contour simplification
- one-cell detail sampling
- 0.25-cell height error

All of it was applied:

- **New binary, new outputs.** I verified the new binary against `binary.json`
  before and after every run. The new outputs are `v5` and `v5-repeat`; `v3` is
  preserved untouched. `v4` and `v4-repeat` were a first pass at 560 ticks. The
  longer route did not arrive in time, so I raised the run to 600 ticks.
- **Re-checked behavior.** I re-ran the scene, the every-tick diagnostics and the
  exact repeat. I then re-inspected real frames, focusing on wall clearance,
  ridge profile, doorway traversal and arrival.
- **Marker pools.** The pools did not cover every point. In `v3` the replanned
  route had 17 points but there were only 16 dot markers, so the final goal dot
  was never drawn. The pools are now 48 dots and 48 links each. The behavior
  logs a coverage record for every drawn route, at `error` level if anything is
  uncovered. All 3 drawn routes in `v5` are fully covered: up to 20 points and
  19 segments. The frame budget is unchanged.
- **Astra's scope decisions accepted.** Query heights describe the quantized
  surface, and physics owns grounding. Corridor/tile kinks are an explicit
  limitation. Last-build reports include no-ops, so I kept the truncated-run
  method for the edit's rebuild evidence. I measured the remaining error rather
  than asking for a universal one-cell subtraction.
- **Doorway traversal.** I added a no-capture doorway-width sweep through the
  same public CLI path to check it directly.

No Rust, core algorithm, UI layout, other packet or mutation routing changed.
No native capture was attempted because the Mac is locked.

## Before / after findings

The "before" column is `v3`: binary `135b3feff6…57e0d`, source `9833cf6`. The
"after" column is `v5`: binary `6bacde25…62ce6bc`, source `141a835`. Both use the
same scene, behavior and settings: cell 0.1 m, cell height 0.05 m, radius 0.4 m.

| Finding | v3 (before) | v5 (final) |
|---|---|---|
| Route clearance at convex corners (configured 0.4 m) | 0.389 m minimum (1.1 cm inside the radius). Pillar 0.45, partition 0.46–0.51 | **0.50 m minimum**. Pillar 0.55–0.56, partition 0.51–0.60. No clip; 0.1–0.2 m conservative |
| Closest capsule-to-wall gap | 0.089 m | 0.200 m |
| Narrowest passable doorway (0.4 m radius) | Not measured; the old binary was not retained | **1.2 m. Every doorway up to 1.1 m returns `null`** (new concern) |
| Height of returned points | +5 cm over the true surface | +5 cm. Astra's scope decision: quantized surface, physics grounds |
| Rise before the ridge's near (west) face | Starts 0.23 m early | **Starts ≤ 0.07 m early** (improved) |
| Ramp past the ridge's far (east) face | 0.35 m | 0.26–0.29 m |
| Flat-floor bumps east of the ridge, above the +5 cm level | +1.5 to +9.1 cm | **+2.4 to +10.0 cm** (not improved) |
| Tile-boundary kinks | 0.24 m sideways at (4.8, −1.8) | 0.30 m at (4.8, −1.8), plus **a 1.14 m sideways detour at (−3.3, −1.8) that adds 0.60 m** (new) |
| Marker coverage | Goal dot missing on the replanned route (pool of 16 for 17 points) | All points and segments drawn (pools of 48) |
| Polygons before → after the edit | 56 → 46 | 145 → 120 |
| Arrival | Tick 540 | Tick 562 (longer route), exact |

## Final evidence (`v5`, identical in `v5-repeat`)

### Scene (unchanged from v3 apart from the marker pools)

The room is 16 × 10 m. A partition at x = 0 has doorway A (z 0.4–2.0) and
doorway B (z −4.4 to −2.8). The remaining pieces are:

- a capsule pillar, r 0.45 m, which is a curved `collider` source
- a 0.15 × 0.6 m ridge across the full depth, a cooked `mesh` source
- a screen wall
- a pocket behind a 0.5 m slit
- a muted-blue barrier that is moved by script

The floor is a cooked `mesh` source with 1 m checker tiles. All geometry is
original mathematical glTF with matching analytic colliders and unit scale.

The character is a kinematic 1.6 m capsule, r 0.3 m. It walks the returned
polyline at 2 m/s. Each step goes through `computeCharacterMotion` with
autostep, and the result is applied as a Velocity command. The edit happens at
the end of tick 100, and the character replans at tick 101.

The colour roles are as follows:

- **Amber** marks the live route.
- **Cool grey** marks the superseded route.
- **Blue** marks the edited barrier.
- **Brick** marks the null target.

**Path markers are not native debug draw.** They are 192 pre-authored,
render-only mesh entities that the behavior moves onto the returned points with
Transform commands, 12 mm above each returned height. They show returned points
only, not polygons, tiles or corridors.

### Queries, edit and rebuild

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 17 points, 13.55 m in plan view, via doorway A, generation 1, 110 polygons visited |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.44, 0) | 20 points, 15.27 m in plan view, via doorway B, generation 2 |
| 101 | Route from start to pocket | **`null`** |
| 562 | Arrived | Final position equals the goal to 1 mm |

Stopping playback at tick 100 shows the edit's build report:

- Rebuilt tiles (2,1), (2,2) and (2,3); reused the other 17.
- Polygons dropped from 145 to 120; generation went from 1 to 2.

The reports at ticks 1 and 99 are no-op updates that reuse all 20 tiles.

### Motion, every tick (`trace_check.py`)

| Check | Result |
|---|---|
| Deviation from the active returned polyline | max **6.1 mm** (t291, ridge climb) |
| Stalls below 90% of the requested step | Only t287–295, the autostep climb onto the ridge |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. No wall, pillar, barrier or pocket contact |
| Closest capsule gap | Screen wall 0.200 m (t381), partition 0.214 m, pillar 0.250 m |
| Height on flat floor or ridge top | At rest height; the worst error is the 1 cm spawn settle at t1 |
| Largest vertical move per tick | 4.0 cm (t324, rolling off the ridge) |

The ridge is crossed over ticks 287–326:

- Contact is at t287.
- The capsule rides the edge arc at 39–100% of horizontal input.
- It sits at exactly 0.97 on top.
- It rolls off the far edge grounded every tick.

This is the rounded-capsule autostep behaviour classified in 0023, not a
navigation defect.

### New finding 1: doorway false negatives (correctness concern)

`doorway_sweep.py` builds a separate 8 × 6 m room for each width, through the
public CLI path. Each room has a box floor and a 0.3 m partition with one centred
doorway. It uses the same navigation settings as the scene and asks for a route
from (−3, 0, 0) to (3, 0, 0) through the doorway.

| Doorway width (m) | 0.80 | 0.85 | 0.90 | 0.95 | 1.00 | 1.05 | 1.10 | 1.20 | 1.30 | 1.40 | 1.60 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Route | null | null | null | null | null | null | null | yes | yes | yes | yes |

- An agent with a 0.4 m radius needs 0.8 m. One extra voxel per side would
  explain a threshold near 1.0 m. A threshold between 1.1 and 1.2 m implies an
  effective jamb erosion of about 0.55–0.6 m. That matches the 0.50–0.60 m
  corner clearances in the main scene.
- The radius checks in Astra's tests confirm that clearance never falls *below*
  the radius. They do not confirm that passages wider than 2 × radius stay open.
- A 1.0 m doorway that a 0.8 m-wide agent fits through physically is reported
  unreachable. That is a false negative a game would hit with ordinary door
  sizes.
- The 1.2–1.6 m routes run down the doorway centreline, so their jamb clearance
  only reflects the doorway width.
- The old binary was not retained, so I cannot say whether v3 passed 1.0 m.
- This is a correctness question for Astra, not a visual one.

### New finding 2: tile-boundary detour

The replanned route goes (−3.44, 0) → (−3.3, −1.8) → (−0.3, −3.3).

- The middle point sits on the z = −1.8 tile-row boundary, 1.14 m to the side of
  the direct line.
- The direct segment passes 0.45 m from the doorway-B jamb, which is inside the
  new ~0.5 m erosion. A small bend near the jamb is therefore expected, but not a
  1.14 m excursion.
- The leg is 0.60 m (13%) longer.
- It reads clearly in the frames. In the plan view at t150 and the pillar view
  at t125, the character walks nearly due north before turning toward
  doorway B.
- The east-room kink at the (4.8, −1.8) tile corner is still there: 0.30 m
  sideways, adding 3.8 cm.
- The first route has a 5 cm kink at the x = −1.6 tile boundary.

I accept Astra's statement that centroid-graph A* plus funnel is not a global
shortest-path solver. I report this size of detour because it is visible
gameplay behavior.

### Remaining height detail, measured

All returned points keep the deliberate +5 cm quantized-surface offset. The
excess beyond that offset is:

- **West (near) face.** The route rises within 0.05 m (second route) and 0.07 m
  (first route) of the face, down from 0.23 m. This is improved.
- **East (far) face.** The ramp reaches floor level 0.26 m (second route) and
  1.0 m (first route) beyond the face. The first route has bumps of +6.4, +10.0
  and +8.8 cm at x 1.771, 2.032 and 2.174. The second route has bumps of +4.5
  and +2.4 cm at x 2.512 and 2.775.
- **Doorway A.** There is a new +2.4 cm bump on flat floor at x 0.412, just past
  doorway A.
- **Asymmetry.** All the larger excess lies east of the ridge. The ridge's east
  face coincides with the x = 1.6 tile boundary, which may be worth checking for
  tile-border detail sampling.

The character is unaffected because physics grounds it. The profile screenshots
show the markers stepping cleanly at the west face, and the strip sloping off
the east face.

### Rendered review

I inspected real frames from five cameras at the start, the pillar pass, the
edit tick, the replan, the detour leg, doorway B, the ridge climb, the ridge top,
the step-down, the screen-wall tip and arrival. Clearance at the pillar and the
screen-wall tip is visibly wider than in v3.

- **Edit and replan.** Frame 100 shows the barrier filling doorway A while the
  old route is still drawn, which is the expected one-tick window. By frame 105
  the old route is grey and the new amber route is shown.
- **Doorway B.** The character goes through doorway B centred, with no contact.
- **Pocket.** The brick target stays unvisited.
- **Arrival.** The character stands on the goal pad at frame 565.
- **Path versus movement.** Every inspected frame shows the capsule on the amber
  strip. There is no discrepancy between the returned path and the rendered
  movement.

### Repeat stability (local equality only)

`v5` and `v5-repeat` were each built from scratch.

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
shasum -a 256 artifacts/tools/incant_headless        # 6bacde25…62ce6bc = binary.json (before and after)
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v5 --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v5-repeat --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v4   # refused, exit 1
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v5 \
  artifacts/0026-navigation/v5/overview.logs.jsonl
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v5 \
  0.8 0.85 0.9 0.95 1.0 1.05 1.1 1.2 1.3 1.4 1.6
artifacts/tools/incant_headless play artifacts/0026-navigation/v5/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/v5/navigation_course.js     # also --ticks 1 and 99
artifacts/tools/incant_headless script artifacts/0026-navigation/v5/navigation.incant.json \
  artifacts/0026-navigation/v5/navigation_course.js --ticks 600             # ×3, timing
```

`node_modules` from the first session's `npm ci` was still present: TypeScript
5.9.3 and SWC 1.16.13.

Each play run is 600 ticks with `--capture-every 5`. That gives 121 frames at
960×540, or 250,905,600 bytes (239.3 MiB), under both caps. Every report has
`completed: true` and 889 script commands, which are:

- 600 Velocity commands
- 1 edit
- 288 marker moves

Each report also has adapter `Apple M5 Pro`, 207 model entities and 14,390
triangles.

Timing on this Mac only; these are not platform budgets:

- `script --ticks 600` had a p95 of 2.06–2.86 ms per tick over three runs, with
  a maximum of 3.3–3.8 ms.
- v3 measured 0.94–1.01 ms. The behavior's per-tick `query()` results now carry
  three times as many marker entities, so this difference is **not** attributable
  to navigation.

## Screenshots

All 12 are under `handoffs/0026-navigation-lookdev/screenshots/` (696 KB). They
replace the v3 set. Each is an unedited `play` frame from `v5`, byte-identical in
`v5-repeat`.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier in doorway A; old route still drawn (one-tick window) |
| `overview-t105-replanned.png` | Superseded route in grey; new amber route via doorway B |
| `overview-t565-arrived.png` | Character on the goal pad |
| `plan-t050-around-pillar.png` | First route around the pillar, with wider clearance |
| `plan-t150-tile-boundary-detour.png` | Replanned route's 1.14 m sideways detour; character on the northward leg |
| `plan-t380-screen-wall-tip.png` | Rounding the screen-wall tip; east tile-corner kink |
| `pillar-t075-passing-pillar.png` | Clearance while passing the pillar |
| `pillar-t125-walking-detour-leg.png` | Character walking the detour leg toward the camera |
| `ridge-t305-on-ridge-after-doorway-b.png` | On the ridge after doorway B |
| `profile-t105-returned-route-heights.png` | Crisp step at the west face; ramp off the east face |
| `profile-t320-stepping-down.png` | Grounded step-down over the east face |

In the profile camera, which looks south, east is screen-left. The raw output is
in the ignored `artifacts/0026-navigation/`:

- `v5`, `v5-repeat` and `doorway-v5` are final.
- `v3` and `v3-repeat` are the historical before-runs.
- `v4` and `v4-repeat` are the 560-tick runs that did not arrive.
- `try1`–`try3`, `v1` and `v2` are earlier iterations.
- `review-*` holds review sheets and zoomed crops made from real frames.

## Changed paths (this revision)

- `handoffs/0026-navigation-lookdev/result.md`: rewritten.
- `handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py`: marker pools
  raised from 16 to 48.
- `handoffs/0026-navigation-lookdev/tools/navigation_course.ts`: logs a coverage
  record for every drawn route.
- `handoffs/0026-navigation-lookdev/tools/doorway_sweep.py` and
  `doorway_probe.ts`: new doorway diagnostic, which refuses existing output.
- `handoffs/0026-navigation-lookdev/tools/tsconfig.json`: now includes the probe.
- `handoffs/0026-navigation-lookdev/screenshots/*.png`: 12 v3 frames replaced by
  12 v5 frames.

`trace_check.py` is unchanged.

## Limitations

- **Course coverage.**
  - The scene covers one flat room, one agent size and one static edit.
  - The doorway sweep covers axis-aligned, centred doorways in one 0.3 m wall.
  - Slopes, stacked floors, rotated doorways, mirrored or hierarchical models,
    many-agent steering and moving obstacles were not exercised.
- **Markers.** They show returned points only.
- **Logs.** Captures every 5 ticks cannot show single-tick events, so per-tick
  claims come from the logs. Logged values are rounded to 1 mm.
- **Unchecked approaches.** The pocket `null` is tested from the start position
  only.
- **Untested.** No native screenshot, hosted CI, Rust tests or Clippy were run;
  no Rust changed.
- **Appearance.** There is no character art or animation. `material_preview`
  shading without an environment light is flat. It is fine for motion review but
  is not shipping look-dev.

## Open questions for Astra and the director

1. **Doorway false negatives.** Should erosion compensation keep passages of
   about 2 × radius plus one cell open? Any doorway up to 1.1 m now returns `null`
   for a 0.4 m radius. A test asserting that a just-wider-than-2r doorway stays
   reachable would pin this.
2. **Detour size.** Is a 1.14 m sideways, 13% tile-boundary detour within the
   accepted corridor-kink limitation? Or should the corridor search avoid it?
3. **East-face smearing.** It did not improve with one-cell detail sampling,
   with bumps up to 10 cm. Is the x = 1.6 tile border involved?
