# 0026 navigation rendered-motion review: result

## Status

Final verdict on binary `0a21e3c1…3f2922` (source `4aee4ce`): **the character
follows real `api.findPath` routes in rendered motion.** Navigation causes no
stalls and there is no wall contact. The character is grounded every tick, stays
within 6.1 mm of the returned polyline and arrives exactly. The room edit
rebuilds three tiles and the next-tick replan reroutes. The pocket query returns
`null` both times.

There is **one actual navigation failure for Astra:** the connected-visibility
repair did not remove the 1.14 m tile-boundary detour. The returned routes are
point-for-point identical to v5. Only the visited-polygon counts grew. Using
only the engine's own queries, I show below a connected route that is 0.59 m
shorter. The visit budget was not the limit.

The other results are as follows:

- **Clearance.** The configured radius is a minimum, with 0.50–0.60 m held at
  convex corners.
- **Doorways.** The doorway sweep reproduces Astra's resolution tradeoff. A
  1.1 m doorway fails at 0.1 m cells and passes at 0.05 m cells.
- **Height.** Quantized heights and step smearing remain, as explicit
  limitations. They are not exact geometry.

This verdict does not approve a phase gate or mark the phase complete. It does
not claim that steering, off-mesh links, 2D navigation, the Navigation Inspector
or native debug draw exist. It certifies no platform budget or cross-device
determinism.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP, on host-supplied evidence. The director states that Astra
invoked the JSON-RPC ACP adapter in `tools/handoff/main.py`. From inside the
session I can see only a Claude Code agent session in this worktree. I cannot
inspect the transport layer myself.

### Priority revision 2 acknowledged (2026-10-10)

The revision asks me to review Astra's bounded line-of-sight corridor repair at
`4aee4ce`, plus the retained erosion safety margin and the finer-cell doorway
fixture. All of it was applied:

- **New binary, new outputs.** I verified the binary against `binary.json`
  before and after every run. I ran the final `v6` and an exact `v6-repeat`
  with the same scene, cameras, marker pools and 600 ticks. `v5` is kept as
  history. No camera was re-authored.
- **Detour.** I inspected whether the 1.14 m detour disappears. It does not, and
  I quantified it with a new no-capture probe that uses only the engine's own
  queries.
- **Doorway sweep.** I added a `--cell-size` option and ran 0.1 m and 0.05 m
  cells with the same 0.4 m radius.
- **Rendered checks.** I reviewed clearance, motion, grounding and arrival in
  every-tick logs and real frames.
- **Unchanged areas.** No Rust, core algorithm, UI layout, other packet or
  mutation routing changed. No native capture was attempted because the Mac is
  locked.

I accept the erosion margin as Astra's radius-safety decision. The doorway
results below describe its cost and do not argue for weakening it.

## Before / after findings

The scene, behavior and settings are the same in all three runs: cell 0.1 m,
cell height 0.05 m, radius 0.4 m.

- **v3** is binary `135b3feff6…57e0d`, source `9833cf6`.
- **v5** is binary `6bacde25…62ce6bc`, source `141a835`.
- **v6** is binary `0a21e3c1…3f2922`, source `4aee4ce`.

| Finding | v3 | v5 | v6 (final) |
|---|---|---|---|
| Route clearance at convex corners (configured 0.4 m) | 0.389 m minimum, a 1.1 cm clip | 0.50–0.60 m | 0.50–0.60 m; radius is a minimum |
| Narrowest passable doorway at 0.1 m cells | Not measured | 1.2 m | 1.2 m (1.1 m passes at 0.05 m cells) |
| Replanned-route detour at (−3.3, −1.8) | None | 1.14 m sideways, +0.60 m | **Unchanged: 1.14 m sideways, +0.59 m against a connected alternative** |
| East-room tile-corner kink at (4.8, −1.8) | 0.24 m | 0.30 m, +3.8 cm | Unchanged |
| Route lengths, first / replanned | 13.45 / 14.56 m | 13.55 / 15.27 m | 13.55 / 15.27 m (same points as v5) |
| Polygons visited, first / replanned | 41 / 36 | 110 / 87 | 376 / 295 |
| Rise before the ridge's near face | 0.23 m | ≤ 0.07 m | ≤ 0.07 m |
| Flat-floor bumps past the ridge's far face (above +5 cm) | Up to 9.1 cm | Up to 10.0 cm | Up to 10.0 cm |
| Marker coverage | Goal dot missing | Complete (pools of 48) | Complete |
| Arrival | Tick 540 | Tick 562 | Tick 562, exact |

## Final evidence (`v6`, identical in `v6-repeat`)

### Scene (unchanged since v5)

The room is 16 × 10 m. A partition at x = 0 has doorway A (z 0.4–2.0) and
doorway B (z −4.4 to −2.8). The remaining pieces are:

- a capsule pillar, r 0.45 m, which is a curved `collider` source
- a 0.15 × 0.6 m ridge across the full depth, a cooked `mesh` source
- a screen wall
- a pocket behind a 0.5 m slit
- a muted-blue barrier that is moved by script

The floor is a cooked `mesh` source with 1 m checker tiles. All geometry is
original mathematical glTF with matching analytic colliders and unit scale.

A kinematic 1.6 m capsule, r 0.3 m, walks the returned polyline at 2 m/s. Each
step goes through `computeCharacterMotion` with autostep, and the result is
applied as a Velocity command. The edit happens at the end of tick 100, and the
character replans at tick 101.

The colour roles are as follows:

- **Amber** marks the live route.
- **Cool grey** marks the superseded route.
- **Blue** marks the edited barrier.
- **Brick** marks the null target.

**Path markers are not native debug draw.** They are 192 pre-authored,
render-only mesh entities, moved onto the returned points with Transform
commands 12 mm above each returned height. The behavior logs a coverage record
for every route. All 3 drawn routes are fully covered: up to 20 points and 19
segments, with pools of 48.

### Queries, edit and rebuild

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 17 points, 13.55 m, via doorway A, generation 1, 376 polygons visited |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.44, 0) | 20 points, 15.27 m, via doorway B, generation 2, 295 polygons visited |
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
| Stalls below 90% of the requested step | Only t287–295, the autostep climb onto the ridge, at 39–88% |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. No wall, pillar, barrier or pocket contact |
| Closest capsule gap | Screen wall 0.200 m (t381), partition 0.215 m, pillar 0.250 m |
| Height on flat floor or ridge top | At rest height; the worst error is the 1 cm spawn settle at t1 |
| Largest vertical move per tick | 3.9 cm (t324, rolling off the ridge) |

The ridge climb is the rounded-capsule autostep behaviour classified in 0023. It
is not a navigation defect.

Compared with v5, per-tick positions differ by at most 3 mm. The first
difference is at t73, in the character-motion vertical component, and both runs
reach the same final pose. The route points are identical, so these differences
come from the new binary's movement or physics arithmetic, not from navigation.

### Navigation failure: the detour is not repaired

The replanned route still goes (−3.44, 0) → (−3.3, −1.8) → (−0.3, −3.3). Its
middle point lies on the z = −1.8 tile-row boundary, 1.14 m to the side of the
direct line.

`detour_probe.ts` checks this with no rendering. On the v6 project it applies the
same barrier edit on tick 1, then issues one `findPath` per tick on the edited
mesh (generation 2):

| Query | Returned | Length |
|---|---|---|
| (−3.44, 0) → (−0.3, −3.3), the doorway-B jamb point | via (−3.3, −1.8); 25 polygons visited | 5.160 m |
| (−3.44, 0) → (−0.6, −3.0) | via (−3.3, −1.8) | 4.760 m |
| (−3.44, 0) → (−0.8, −2.9) | **straight, 2 points** | 3.922 m |
| (−3.44, 0) → (−1.0, −2.6) | straight | 3.566 m |
| (−0.8, −2.9) → (−0.3, −3.3) | via (−0.5, −3.2) | 0.648 m |
| (−3.44, 0) → goal | the scene's route, via (−3.3, −1.8) | 15.268 m |
| (−0.8, −2.9) → goal | through doorway B | 10.757 m |

Joining the engine's own straight leg to (−0.8, −2.9) with its route onward
gives a connected route of **14.679 m**. That is **0.589 m (3.9%) shorter**
than the returned 15.268 m. For the leg to the jamb point alone, the saving is
0.59 m against 5.16 m, or 11%.

Even the 25-polygon query keeps the detour, against a cap of 4000, so the
remaining visit budget is not the cause. The direct segment from (−3.44, 0) to
the jamb point passes 0.45 m from the jamb. That is inside the ~0.5 m erosion,
so a pure vertex-removal shortcut between existing points cannot apply. The
shorter route needs a new bend near the jamb, at about (−0.8, −2.9) here.

It is visible gameplay behavior. In the plan view at t105–150 and the pillar
view at t125, the character walks almost due north before turning toward
doorway B.

The east-room kink at (4.8, −1.8) also remains, at 0.30 m sideways and 3.8 cm.
I did not probe it separately.

### Doorway resolution tradeoff (`doorway_sweep.py`)

Each width is a separate 8 × 6 m room built through the public CLI path. Each
room has a 0.3 m partition with one centred doorway and the same settings,
radius 0.4 m. Only the horizontal cell size differs between the two rows.

| Doorway width (m) | 0.80 | 0.85 | 0.90 | 0.95 | 1.00 | 1.05 | 1.10 | 1.20 | 1.30 | 1.40 | 1.60 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0.10 m cells | null | null | null | null | null | null | null | yes | yes | yes | yes |
| 0.05 m cells | null | null | null | null | null | null | **yes** | yes | yes | yes | yes |

This matches Astra's new Rust fixture: 1.1 m fails at 0.1 m cells and passes at
0.05 m cells.

The configured radius is a minimum clearance, and the cost is conservative
passage loss:

- An agent with a 0.4 m radius physically fits a doorway wider than 0.8 m.
- These settings need at least 1.2 m at 0.1 m cells, or 1.1 m at 0.05 m cells.
- Halving the cell size recovers one 0.05 m step here.

This is the documented tradeoff. It is not a claim that every geometric fit is
found. Exact-clearance construction remains a possible quality improvement.

### Remaining height detail (explicit limitation)

Returned points describe the quantized navigation surface, which is +5 cm on
this aligned geometry. Physics grounds the character, so the excess is visible
only in markers.

- **West (near) face.** The route rises within 0.05–0.07 m of the face.
- **East (far) face.** The ramp runs 0.26 m beyond the face on the replanned
  route, and up to 1.0 m beyond it on the first route.
- **Bumps.** Flat-floor bumps reach +10.0 cm above the quantized level east of
  the ridge, and +2.4 cm just past doorway A.
- **Possible cause.** The far face coincides with the x = 1.6 tile boundary.

None of this is exact geometry, and none of it changed from v5.

### Rendered review

I inspected real frames from five cameras at the start, the pillar pass, the
edit tick, the replan, the detour leg, doorway B, the ridge climb, the ridge top,
the step-down, the screen-wall tip and arrival.

- **Path versus movement.** The capsule stays on the amber strip in every
  inspected frame. Its contact shadow stays attached.
- **Pillar and screen-wall tip.** Clearance is visibly wide, at 0.20–0.25 m of
  capsule gap.
- **Doorway B.** The character goes through centred.
- **Arrival.** The character stands on the goal pad at frame 565.
- **Edit and replan.** Frame 100 shows the one-tick window: the barrier is
  closed while the old route is still drawn. Frame 105 shows the reroute.
- **Path faithfulness.** There is no discrepancy between the returned path and
  the rendered movement. The detour is faithfully followed, and it is a path
  defect, not a motion defect.

The v6 frames differ from v5 only by the sub-millimetre motion changes above.
215 of 605 are byte-identical.

### Repeat stability (local equality only)

`v6` and `v6-repeat` were each built from scratch.

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
shasum -a 256 artifacts/tools/incant_headless        # 0a21e3c1…3f2922 = binary.json (before and after)
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v6 --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v6-repeat --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run ridge:ridge:5 --run profile:profile:5
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v6   # refused, exit 1
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v6 \
  artifacts/0026-navigation/v6/overview.logs.jsonl
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v6-cell0.10 \
  0.8 0.85 0.9 0.95 1.0 1.05 1.1 1.2 1.3 1.4 1.6
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v6-cell0.05 \
  --cell-size 0.05 0.8 0.85 0.9 0.95 1.0 1.05 1.1 1.2 1.3 1.4 1.6
mkdir artifacts/0026-navigation/detour-v6
node tools/build_script.mjs handoffs/0026-navigation-lookdev/tools/detour_probe.ts \
  artifacts/0026-navigation/detour-v6/detour_probe.js
artifacts/tools/incant_headless play artifacts/0026-navigation/v6/navigation.incant.json --ticks 12 \
  --compiled-script artifacts/0026-navigation/detour-v6/detour_probe.js \
  --log-output artifacts/0026-navigation/detour-v6/probe.logs.jsonl
artifacts/tools/incant_headless play artifacts/0026-navigation/v6/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/v6/navigation_course.js     # also --ticks 1 and 99
artifacts/tools/incant_headless script artifacts/0026-navigation/v6/navigation.incant.json \
  artifacts/0026-navigation/v6/navigation_course.js --ticks 600             # ×3, timing
```

The first detour-probe attempt issued all of its queries in one tick. That
exceeded the 256-unit native-query budget, so `play` failed with a script error.
The probe now issues one query per tick. `play` refuses to overwrite its log
output, so I cleared the probe directory by hand between attempts.

Each scene run is 600 ticks with `--capture-every 5`. That gives 121 frames at
960×540, or 250,905,600 bytes (239.3 MiB), under both caps. Every report has
`completed: true` and 889 script commands, adapter `Apple M5 Pro`, 207 model
entities and 14,390 triangles.

Timing on this Mac only; these are not platform budgets. `script --ticks 600`
had a p95 of 2.04–2.09 ms per tick over three runs, with a maximum of 4.0–4.4 ms.
v5 measured a p95 of 2.06–2.86 ms. The repair's extra polygon visits fall on two
query ticks only.

## Screenshots

All 12 are under `handoffs/0026-navigation-lookdev/screenshots/`. The names are
unchanged from the v5 set, and the contents are replaced with `v6` frames. Each
is an unedited `play` frame, byte-identical in `v6-repeat`.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier in doorway A; old route still drawn (one-tick window) |
| `overview-t105-replanned.png` | Superseded route in grey; new amber route via doorway B |
| `overview-t565-arrived.png` | Character on the goal pad |
| `plan-t050-around-pillar.png` | First route around the pillar |
| `plan-t150-tile-boundary-detour.png` | **The unrepaired 1.14 m detour**; character on the northward leg |
| `plan-t380-screen-wall-tip.png` | Rounding the screen-wall tip with 0.2 m capsule gap; east tile-corner kink |
| `pillar-t075-passing-pillar.png` | Clearance while passing the pillar |
| `pillar-t125-walking-detour-leg.png` | Character walking the detour leg toward the camera |
| `ridge-t305-on-ridge-after-doorway-b.png` | On the ridge after doorway B |
| `profile-t105-returned-route-heights.png` | Crisp step at the west face; ramp and bumps past the east face |
| `profile-t320-stepping-down.png` | Grounded step-down over the east face |

In the profile camera, which looks south, east is screen-left. The raw output is
in the ignored `artifacts/0026-navigation/`:

- `v6`, `v6-repeat`, `detour-v6`, `doorway-v6-cell0.10` and
  `doorway-v6-cell0.05` are final.
- `v5`, `v5-repeat`, `doorway-v5`, `v3` and `v3-repeat` are history.
- `review-v6` and the other `review-*` folders hold review sheets made from real
  frames.

## Changed paths (this revision)

- `handoffs/0026-navigation-lookdev/result.md`: rewritten.
- `handoffs/0026-navigation-lookdev/tools/detour_probe.ts`: new no-capture
  probe.
- `handoffs/0026-navigation-lookdev/tools/doorway_sweep.py`: adds the
  `--cell-size` option.
- `handoffs/0026-navigation-lookdev/tools/tsconfig.json`: now includes the probe.
- `handoffs/0026-navigation-lookdev/screenshots/*.png`: replaced with `v6`
  frames. Three are byte-identical to v5: t000, t050 and profile t105.

The scene helper, behavior and `trace_check.py` are unchanged.

## Limitations

- **Course coverage.**
  - The scene covers one flat room, one agent size and one static edit.
  - The doorway sweep covers axis-aligned, centred doorways in one wall and
    two cell sizes.
  - The detour probe establishes that a shorter connected route exists. It does
    not establish the optimal route.
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
  shading is flat. It is fine for motion review but is not shipping look-dev.

## Open questions for Astra

1. **The detour.** The connected-visibility repair keeps a 1.14 m tile-boundary
   detour. The engine's own queries show a connected route 0.59 m shorter, and
   the visit budget is not the limit. Could the repair introduce new bend
   vertices along portal edges, as a funnel does, instead of only removing
   intermediate points? The probe's queries and coordinates above can serve as a
   regression fixture.
2. **East-face smearing.** Is the x = 1.6 tile border involved in the east-face
   bumps of up to 10 cm? They are unchanged since v5.
