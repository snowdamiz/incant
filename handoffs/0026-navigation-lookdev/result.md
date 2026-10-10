# 0026 navigation rendered-motion review: result

## Status

Scoped verdict: **the character follows real `api.findPath` routes in rendered
motion without stalls caused by navigation, without wall contact, and without
leaving the returned polyline by more than 5.1 mm.** The room edit rebuilds
three tiles and the next-tick replan reroutes cleanly. The disconnected pocket
query returns `null` both times.

I found four navigation-quality issues for Astra. None of them breaks this
course, but all of them are visible in the rendered path markers:

1. Every returned point is 5 cm above the true surface.
2. The 0.15 m ridge is returned as ramps that extend 0.23–0.35 m beyond its
   vertical faces, with extra 1.5–9 cm bumps on flat floor.
3. The route bends at a tile corner in open floor, 0.24 m off the straight line.
4. At one wall corner the route runs 1.1 cm inside the agent radius.

This verdict does not approve a phase gate. It does not claim that steering,
off-mesh links, 2D navigation, the Navigation Inspector or native debug draw
exist. It certifies no platform budget or cross-device determinism.

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred. This ran as a Claude Code agent session in this worktree under the
director's packet. The packet routes Claude work through ACP. I cannot verify the
transport layer from inside the session.

### Packet and feedback acknowledgement

This is the first session for 0026. The worktree held no earlier files for this
handoff, and the brief has no priority-revision section or new director feedback.
I applied the brief as written. No Rust, core algorithm, UI layout, other
handoff packet or mutation routing was changed. No native capture was attempted
because the Mac is locked.

Binary: `artifacts/tools/incant_headless`, sha256
`135b3feff61a53953fddb280b026a895c077b3f5d820a82a6ec1edf0f7757e0d`. That
matches `artifacts/tools/binary.json` (source `9833cf6`). The helper checks the
hash before and after every build, and every run matched. No Rust was compiled.

## The scene

The room is 16 × 10 m, with x pointing east and z pointing south toward the
overview camera. All geometry is original mathematical glTF with the dimensions
baked into the vertices. Each physics piece has unit scale and an analytic
collider that equals its mesh.

| Piece | Navigation source | Purpose |
|---|---|---|
| Checkered floor slab, 1 m tiles | `mesh` (cooked model) | Walkable surface and distance scale |
| Partition at x = 0, 1.0 m tall | `collider` (boxes) | Splits the room. Doorway A (z 0.4–2.0) and doorway B (z −4.4 to −2.8) are each 1.6 m wide |
| Pillar, capsule r 0.45 m | `collider` (faceted for navigation) | Curved obstacle on the first route |
| Ridge, 0.15 × 0.6 m, full depth | `mesh` (cooked model) | Climbable height detail that every route crosses |
| Screen wall, east room | `collider` | Corner on the second route |
| Pocket, 0.5 m slit | `collider` | Too narrow for the 0.4 m agent radius; its target must be unreachable |
| Barrier, muted blue | `collider` | The scripted room edit |

The character is a 1.6 m capsule with radius 0.3 m. It is kinematic and is not a
navigation source. Navigation settings:

- cell size 0.1 m, cell height 0.05 m, tile 32 cells (20 tiles)
- agent radius 0.4 m, height 1.7 m, climb 0.25 m, slope 45°

The behavior runs as follows:

- **Tick 1.** It queries the start-to-goal route and the start-to-pocket route.
- **Every tick.** It walks the returned polyline at 2 m/s. Corners are consumed
  within the tick, so the requested step never loses speed. It resolves each
  step with `computeCharacterMotion`, using the same options as handoff 0023,
  autostep included. It applies the result as an ordinary Velocity command.
- **End of tick 100.** It moves the barrier into doorway A with a Transform
  command.
- **Tick 101.** It replans from its current position and repeats the pocket
  query.

Colour roles stay within the neutral Incant direction:

- Structure is warm greys at different values. The character is the lightest
  object, and the pillar is darker so the two never merge.
- The one warm accent, amber, marks the live route and the goal pad.
- A cool light grey marks the superseded route.
- Muted blue marks the edited barrier.
- Brick marks the unreachable pocket target.

**Path markers are not native debug draw.** They are 64 pre-authored, render-only
mesh entities: discs and unit strips scaled to segment length. The behavior moves
them onto the returned points with ordinary Transform commands, 12 mm above each
returned height. They show exactly what `findPath` returned. They do not show
navmesh polygons, tiles or corridors. The native debug-draw design is the next
increment.

## Evidence (final run `v3`, identical in `v3-repeat`)

### Queries, edit and rebuild (logs)

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 13 points, 13.45 m in plan view, via doorway A, generation 1, 41 polygons visited |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier moved into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.398, 0.1) | 17 points, 14.56 m in plan view, via doorway B, generation 2 |
| 101 | Route from start to pocket | **`null`** |
| 540 | Arrived | Final position equals the goal to 1 mm |

Stopping playback at tick 100 shows the edit's build report:

- Rebuilt tiles (2,1), (2,2) and (2,3). Those are the tiles under the barrier's
  old and new positions.
- Reused the other 17 tiles.
- Polygons dropped from 56 to 46; generation went from 1 to 2.

The full run's final report only shows the last no-op reconcile, with all 20
tiles reused.

### Motion, every tick (`trace_check.py`)

| Check | Result |
|---|---|
| Deviation of the capsule axis from the active returned polyline | max **5.1 mm** (t277, on the ridge climb) |
| Stalls (below 90% of the requested step) | Only t270–278, the autostep climb onto the ridge (see below). None anywhere else |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. **No wall, pillar, barrier or pocket contact** |
| Closest capsule-to-obstacle gap | Screen wall 0.089 m (t374), pillar 0.150 m, partition 0.160 m |
| Height on flat floor or ridge top | At rest height; the worst error is 1 cm, from the spawn settle at t1 |
| Largest vertical move per tick | 3.7 cm (t307, rolling off the ridge) |

**Ridge crossing.** The ridge is 0.15 m tall and is crossed through doorway B
over ticks 270–308.

- Contact is at t270, x = 0.723.
- The capsule rides up the edge arc over 11 ticks at 40–100% of horizontal input.
- t272 has a single 3.3 cm vertical pop.
- It sits at exactly 0.97 on top (rest height 0.82 + 0.15).
- It rolls off the far edge grounded every tick, at full horizontal speed.
- It loses about 0.1 m, roughly 3 ticks.

This is the rounded-capsule autostep behaviour already classified in 0023. It is
not a navigation defect.

### Findings for Astra (navigation quality)

1. **Returned heights are biased by +5 cm, exactly one cell height.**
   - Every floor point is y = 0.05 over a floor at y = 0.
   - Ridge-top points are 0.20 over a top at 0.15.
   - Characters are unaffected because physics grounds them. Anything placed
     from path heights floats 5 cm, for example markers, foot IK or effects.
     The screenshots show this as discs riding visibly above the floor.
   - Question: should query output subtract the voxel-top offset, or should the
     docs state it?
2. **The step is returned as ramps, with bumps on flat floor.** The ridge has
   vertical faces at x = 1.0 and x = 1.6.
   - The second route rises from y 0.05 at x 0.60 to 0.117 at 0.773, then 0.20
     at 0.998. The climb starts 0.23 m before the face.
   - It descends from 0.20 at 1.60 to 0.065 at 1.952. That ramp hangs 0.35 m out
     over the floor.
   - The first route has flat-floor points 7.3 cm and 9.1 cm above the biased
     floor, at x 1.803 and 2.151, which is 0.2–0.55 m past the face.
   - The second route has 1.5 cm and 3.0 cm bumps at x 1.952 and 2.721.
   - The profile screenshots show the amber strip sloping off the ridge into the
     air.
   - This matches coarse detail-mesh sampling. The docs promise that
     "intersections retain terrain ridges and steps": the step is retained, but
     smeared.
3. **There is a bend at a tile corner in open floor.**
   - The second route has a point at exactly (4.8, −1.8), which is a 3.2 m tile
     corner, with nothing nearby.
   - It sits 0.24 m to the side of the straight line from the screen-wall
     corner to the goal, and costs 2.5 cm of length.
   - It is visible in the plan screenshot at t375.
   - It looks like a corridor that touches a tile-corner vertex, so the funnel
     cannot straighten across it. This is consistent with the documented
     "not an exact geodesic". It is still a visible kink in a straight open run.
4. **The route clips the agent radius slightly at the screen-wall tip.**
   - The second route passes 0.389 m from the wall corner, against a configured
     radius of 0.4 m, so it is 1.1 cm inside. That is within one cell.
   - The capsule's real gap is 0.089 m, with no contact.
   - Elsewhere the clearance is conservative: pillar 0.45 m, which is the
     faceting margin, and partition 0.46–0.51 m.

There is no discrepancy between the returned path and the rendered movement.
The logged positions follow the polyline to within 5 mm. In every inspected
frame the capsule sits on the amber strip. It never cuts through marker corners
and never moves without a route. The only rendered offsets are the marker
heights described above, which faithfully show the returned data.

### Rendered review

I inspected actual frames from all five cameras at the start, the pillar pass,
the edit tick, the replan, the ridge climb, the top of the ridge, the step-down,
the screen-wall tip and arrival. I used tiled review sheets and zoomed crops.
These views make the route and the room change obvious:

- **The edit and replan.** Frame 100 shows the barrier already filling doorway A
  while the old amber route still runs through it. That is the expected
  one-tick window, because commands become visible next tick. By frame 105 the
  old route is cool grey and the new amber route turns toward doorway B.
- **The pillar pass.** The capsule passes in front of the pillar with a visible
  gap. Its shadow stays attached and it does not jitter.
- **The ridge.** The capsule rides the ridge edge, sits level on top and steps
  down. There is no penetration or floating.
- **The pocket.** The brick disc stays behind the slit. Nothing travels toward it.

In the first pillar-camera iteration, the capsule appeared to lean. The engine
state reported identity rotation, so the lean was 3-point perspective from a
steep downward tilt. I levelled that camera rather than report a false artifact.

### Repeat stability (local equality only)

`v3` and `v3-repeat` were each built from scratch by the helper.

- All **565/565 frames** across five cameras are byte-identical PNGs.
- The logs are identical across the two runs and across all five cameras.
- Each `report.json` is equal once engine-minted IDs are normalized and
  `wall_ms` is removed.
- The project files are equal once minted IDs and provenance transaction ULIDs
  are normalized.

This was measured on one machine (Apple M5 Pro) with one binary. It is not
evidence of cross-platform or cross-GPU determinism.

## Commands (repo root)

```sh
shasum -a 256 artifacts/tools/incant_headless        # 135b3feff6…57e0d = binary.json
npm ci                                                # node_modules was absent; exit 0, 0 vulnerabilities
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v3 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v3-repeat \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v1   # refused, exit 1
(cd artifacts/0026-navigation/v3 && python3 -I ../../../handoffs/0026-navigation-lookdev/tools/trace_check.py . overview.logs.jsonl)
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v3 \
  artifacts/0026-navigation/v3/overview.logs.jsonl --rows 264 312
artifacts/tools/incant_headless play artifacts/0026-navigation/try3/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/try3/navigation_course.js     # also --ticks 1 and 99: build reports
artifacts/tools/incant_headless script artifacts/0026-navigation/v3/navigation.incant.json \
  artifacts/0026-navigation/v3/navigation_course.js --ticks 560             # ×3, timing
```

Each helper run takes about 8 s wall time and includes:

1. `init`
2. 19 `import`s
3. One `rpc` `command.execute` that creates 86 entities, then `project.save`
4. `validate`
5. `build_script.mjs`
6. Five `play --camera --log-output` runs

Each play run is 560 ticks with `--capture-every 5`. That gives 113 frames at
960×540, or 234,316,800 bytes (223.5 MiB) of raw capture data, under both caps.
Every report has `completed: true`, 657 script commands, adapter `Apple M5 Pro`,
79 model entities and 7,478 triangles. The command count is 560 Velocity
commands, 1 edit and 96 marker moves.

Performance on this Mac only; these are not platform budgets:

- `script --ticks 560` (behavior, physics and navigation queries, with one JSON
  log per tick) had a p95 of 0.94–1.01 ms per tick over three runs.
- The maximum was 1.76–2.02 ms per tick.
- A captured `play` run took 1.5–2.5 s wall time.

## Screenshots

All 12 are under `handoffs/0026-navigation-lookdev/screenshots/` (728 KB). Each
is an unedited `play` frame from `v3`, byte-identical to the same tick in
`v3-repeat`.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier closed in doorway A; the old route is still drawn (one-tick window) |
| `overview-t105-replanned.png` | Superseded route in grey; new amber route via doorway B |
| `overview-t540-arrived.png` | Character on the goal pad |
| `plan-t050-around-pillar.png` | Top view of the first route bending around the pillar |
| `plan-t375-screen-wall-tip-tile-corner-bend.png` | Rounding the screen-wall tip; the tile-corner bend on the diagonal |
| `pillar-t075-passing-pillar.png` | Clearance while passing the pillar |
| `pillar-t110-turn-onto-new-route.png` | Turning off the old route onto the replanned one |
| `ridge-t275-climbing-ridge.png` | Autostep climb onto the ridge at doorway B |
| `profile-t105-returned-route-heights.png` | Returned heights in profile: rise before the face, ramp off the far face |
| `profile-t290-on-ridge.png` | Level on the ridge top |
| `profile-t305-stepping-down.png` | Grounded step-down past the marker ramp |

In the profile camera, which looks south, east is screen-left. All raw output is
in the ignored `artifacts/0026-navigation/`:

- `v3` and `v3-repeat` are the final runs.
- `try1`–`try3`, `v1`, `v1-repeat`, `v2` and `v2-repeat` are layout and camera
  iterations. They have identical behavior from `try3` onward.
- `review-v1` and `review-v2` hold review sheets and zoomed crops. These are
  review aids made from real frames, not evidence substitutes.

## Changed paths

- `handoffs/0026-navigation-lookdev/result.md`
- `handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py`: the helper. It
  refuses an existing output directory and checks the binary hash.
- `handoffs/0026-navigation-lookdev/tools/navigation_course.ts`: the strict TS
  behavior.
- `handoffs/0026-navigation-lookdev/tools/trace_check.py`: the every-tick checker.
- `handoffs/0026-navigation-lookdev/tools/tsconfig.json`
- `handoffs/0026-navigation-lookdev/screenshots/*.png` (12 files)

## Limitations

- **Course coverage.** The course covers one flat room:
  - box walls, one capsule pillar, one 0.15 m ridge and one disconnected pocket
  - one agent size and one static edit

  Slopes, stacked floors, mirrored or hierarchical models, large worlds,
  many-agent steering and moving obstacles were not exercised.
- **Markers.** They show returned points only, not polygons, tiles or corridors.
- **Logs.** Captures every 5 ticks cannot show single-tick events, so per-tick
  claims come from the logs. Logged values are rounded to 1 mm.
- **Unchecked approaches.** The pocket's `null` is tested from the start position
  only.
- **Build reports.** The edit's rebuilt-tile report came from a separate run
  stopped at tick 100. The full run's final report shows only the last no-op
  reconcile.
- **Untested.** No native screenshot, hosted CI, Rust tests or Clippy were run;
  no Rust changed.
- **Appearance.** There is no character art, facing indicator or animation.
  `material_preview` shading without an environment light is flat. It is fine
  for motion review but is not shipping look-dev.

## Open questions for Astra and the director

1. Should `findPath` heights be corrected to the true surface, removing the
   one-cell bias? Or should the docs tell callers to raycast for placement?
2. Is the detail-mesh smearing at a 0.15 m vertical step acceptable? It shows
   0.23–0.35 m ramps and 1.5–9 cm bumps on flat floor. Or should detail sampling
   or edge retention be tightened?
3. Should the funnel or corridor avoid forced tile-corner vertices in open floor?
4. Should erosion guarantee the configured radius at convex corners? It is 1.1 cm
   short here.
5. Should `play` reports keep the last *changing* build report, or a build
   history, so the rebuilt tiles of a mid-run edit are visible without
   truncating the run?
