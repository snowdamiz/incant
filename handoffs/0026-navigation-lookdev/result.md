# 0026 navigation rendered-motion review: result

## Status

Final verdict on binary `6aa98123…26445e` (source `6962fdb`): **both v7
failures are fixed.**

- **Floor restored.** The missing floor east of the pillar is back. Both
  walkability passes are clean, and the polygon count rose from 145 to 147.
  Every tile-border crossing that bent in v7 is now a straight two-point path.
  The live replan hugs the pillar's eroded disc instead of the tile corner,
  which saves 0.53 m.
- **Original throw fixed.** The 4.7/4.71 query no longer throws, in either
  direction.

On the live course the character follows real `api.findPath` routes:

- It stays within 0.85 mm of the returned polyline, except for a 4.6 cm autostep
  slip on the diagonal ridge climb.
- It is grounded every tick, never touches a wall and arrives exactly at
  tick 563.
- The room edit rebuilds three tiles and the replan reroutes.
- The pocket query returns `null` both times.
- Every returned point and segment has a marker.

This review found **two new issues and one incident:**

1. **Navigation failure: another valid query throws.** From (4.3, 0.12, 2.7) to
   (4.31, 0.12, 2.7), 7 cm above open floor with the documented snap, it throws
   `smoothed navigation segment left its polygon`. The reverse throws `funnel
   did not cross its navigation corridor`. These are the same two errors as the
   v7 case, at a new point. The same query at surface or floor height succeeds.
2. **Height regression at the ridge's near face.** Returned segments now pass
   **below the true ridge top by up to 6.9 cm**: over 0.28 m on the first route
   and 0.16 m on the replanned one. In v7 it was 2.8 cm over 0.04 m. One
   replanned point, (1.084, 0.081, −2.354), sits inside the ridge. The marker
   strip renders as a post sinking into the ridge.
3. **One transient capture failure.** One captured `play` run failed with a
   generic script failure at tick 584, after arrival. That was 1 of 36 captured
   runs on this binary. Its frames up to that point are byte-identical to the
   passing runs. It did not recur in 10 dedicated retries.

The resolution/clearance tradeoff is unchanged:

- The configured radius is a minimum, and convex corners keep 0.50–0.60 m.
- At 0.1 m cells, doorways need at least 1.2 m; at 0.05 m cells, at least 1.1 m.
  A 0.4 m-radius agent physically fits anything wider than 0.8 m.

Quantized heights (+5 cm) and step smearing remain. None of this is exact
geometry. This verdict does not approve a phase gate or mark the phase complete.
Portal-graph optimality is not a global continuous shortest-path claim.
Steering, off-mesh links, 2D navigation, the Navigation Inspector and native
debug draw are not claimed.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP, on host-supplied evidence. The director states that Astra
invoked the JSON-RPC ACP adapter in `tools/handoff/main.py`. From inside the
session I can see only a Claude Code agent session in this worktree. I cannot
inspect the transport layer myself.

### Priority revision 4 acknowledged (2026-10-10)

The revision covers Astra's fixes for both v7 failures:

- the patched rerecast triangle clipping, for the phantom raster spans
- f64 closest-point ranking, for the centimetre query

All of it was applied:

- **New binary, new outputs.** I verified the binary against `binary.json`
  before and after every run. The final runs are `v8b` and an exact
  `v8b-repeat`, 600 ticks each.
- **Failed first pair.** `v8` was the first attempt. Its ridge capture failed
  (incident 3), so it is kept as evidence. `v8a` and `v8a-repeat` then passed
  with the original five cameras.
- **History.** `v3` to `v7` are kept as history.
- **Probes re-run.** I re-ran the hole map in both passes, the query-error
  probe, the fixed-start diagnostic, live motion and clearance, and the doorway
  sweep.
- **Actual checkpoints.** The raster fix changed the route and arrival, so the
  checkpoints are taken from this binary's run.
- **One camera added, disclosed.** Since v7 the route has passed south of the
  pillar, where the original north-side pillar camera sees it occluded. The
  revision permits an adjustment, so I added `pillar_south`. The overview,
  plan, pillar, ridge and profile cameras are unchanged. All 605 frames and every
  log of the five-camera `v8a` are byte-identical in `v8b`, so the addition
  changed nothing else.
- **Sleep assertions.** Long runs used `caffeinate -i`. The lock was not
  bypassed, and there was no native interaction.
- **Unchanged areas.** No Rust, core algorithm, UI layout, other packet or
  mutation routing changed.

## Before / after findings

| Finding | v5 | v6 | v7 | v8 (final) |
|---|---|---|---|---|
| Search | centroid A* + funnel | + visibility repair | portal-entry A* | same + raster/precision fixes |
| Fixed-start diagnostic, (−3.44, 0) → goal | 15.27 m, bend at (−3.3, −1.8) | same | 13.634 m | **13.634 m** |
| Missing floor east of the pillar | present | present | mapped, 16 grid points | **none** (both passes) |
| Tile-border crossings near the pillar | bent | bent | bent via (−1.6, 1.4) | **straight** |
| Live replan leg, (−3.29, 2.0) → doorway-B jamb | — | — | 6.193 m via the tile corner | **5.664 m** around the pillar disc |
| Live route lengths, first / replanned | 13.55 / 15.27 m | 13.55 / 15.27 m | 13.29 / 15.85 m | 13.29 / **15.32 m** |
| Throw at (4.7, 1.7) ↔ (4.71, 1.7) | — | — | throws | **fixed** |
| Throw at (4.3, 0.12, 2.7) ↔ (4.31, 0.12, 2.7) | — | — | not hit | **throws** (new) |
| Deepest returned segment below the true ridge top | — | — | 2.8 cm over 0.04 m | **6.9 cm over 0.16–0.28 m** |
| Navigation polygons before → after the edit | 145 → 120 | 145 → 120 | 145 → 120 | 147 → 122 |
| Route clearance at convex corners (configured 0.4 m) | 0.50–0.60 m | same | same | same |
| Narrowest passable doorway, 0.1 / 0.05 m cells | 1.2 m / — | 1.2 / 1.1 m | 1.2 / 1.1 m | 1.2 / 1.1 m |
| Arrival | Tick 562 | Tick 562 | Tick 578 | **Tick 563**, exact |

## Final evidence (`v8b`, identical in `v8b-repeat`)

### Scene (unchanged apart from the added camera)

The room is 16 × 10 m. A partition at x = 0 has doorway A (z 0.4–2.0) and
doorway B (z −4.4 to −2.8). The remaining pieces are:

- a capsule pillar, r 0.45 m, at (−3.6, 1.0), which is a curved `collider`
  source
- a 0.15 × 0.6 m ridge at x 1.0–1.6, a cooked `mesh` source
- a screen wall
- a pocket behind a 0.5 m slit
- a muted-blue barrier that is moved by script

The floor is a cooked `mesh` source with 1 m checker tiles. All geometry is
original mathematical glTF with matching analytic colliders and unit scale.

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

### Queries, edit, rebuild and coverage

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 14 points, 13.29 m, south of the pillar, via doorway A, generation 1 |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.291, 2.0) | 28 points, 15.32 m. It hugs the pillar disc via (−3.0, 1.9), (−2.7, 1.6) and (−2.6, 1.4), goes through doorway B, crosses the ridge diagonally and rounds the screen wall's south tip |
| 101 | Route from start to pocket | **`null`** |
| 563 | Arrived | Final position equals the goal to 1 mm |

**Marker coverage.** Every drawn route is fully covered: 14, 14 and 28 points,
with pools of 48.

**Build report.** I stopped playback at tick 100 to see the edit's build:

- Rebuilt tiles (2,1), (2,2) and (2,3); reused the other 17.
- Polygons dropped from 147 to 122; generation went from 1 to 2.

### Motion, every tick (`trace_check.py`)

| Check | Result |
|---|---|
| Deviation from the active returned polyline | max **4.6 cm** (t333, diagonal ridge climb); ≤ 0.85 mm on every other tick before arrival |
| Stalls below 90% of the requested step | Only t325–336, the diagonal ridge climb, at 79–88% |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. No wall, pillar, barrier or pocket contact |
| Closest capsule gap | Partition 0.200 m (t302, doorway B jamb), pillar 0.250 m (t91), screen wall 0.296 m |
| Height on flat floor or ridge top | At rest height; the worst error is the 1 cm spawn settle at t1 |
| Largest vertical move per tick | 2.5 cm (t389, stepping off the ridge) |

The diagonal ridge slip is character-controller behaviour, as classified in
0023. Autostep cuts the component of motion into the face while sideways motion
continues. It is 4.6 cm here against 3.0 cm in v7, because the crossing angle
changed with the route.

### Fixed v7 failures, verified

**Hole map.** `walkable_probe.ts` tests every 0.2 m grid point with a 1 cm
query. `walkable_check.py` flags open floor, at least 0.75 m from everything,
that is not walkable.

| Pass | Missing points | Thrown queries |
|---|---|---|
| Tight: y 0.05, 6 cm snap | 3, at (2.5–2.7, 1.5–1.9). These are floor raised by height smearing; v7 had the same 3 | none |
| Tolerant: y 0.12, 0.1 m snap | **0** | **1, at (4.3, 2.7)**; see below |

**The v7 hole is gone in both passes.** The pillar's eroded disc is otherwise
unchanged.

**Tile-border crossings.** `detour_probe.ts` re-ran the v7 queries:

- From (−2.0, 1.8) to (−2.0, 0.8), from (−2.2, 1.8) to (−2.2, 0.8) and from
  (−2.4, 1.8) to (−1.8, 0.6) are now **straight two-point paths**.
- (−2.0, 1.0) is walkable.
- The replan leg to the doorway-B jamb is 5.664 m, against 6.193 m in v7. That
  is within 6 cm (1%) of the analytic taut path around a 1.0 m eroded disc,
  5.60 m. The gap is the disc's polygon facets. The leg's last bend at
  (−2.6, 1.4) costs under 1 mm compared with the straight line.

**Fixed-start diagnostic.** (−3.44, 0) → goal is 13.634 m, unchanged from v7,
with no (−3.3, −1.8) kink.

**Original throwing query.** `error_probe.ts` shows every v7 case now returns a
two-point path. That includes (4.7, 1.7) ↔ (4.71, 1.7) at y 0 and y 0.05.

### New navigation failure: query 7 cm above the surface throws

| Query (`snap_distance` 1) | Result |
|---|---|
| (4.3, 0.12, 2.7) → (4.31, 0.12, 2.7) | **throws** `invalid navigation input: smoothed navigation segment left its polygon` |
| (4.31, 0.12, 2.7) → (4.3, 0.12, 2.7) | **throws** `invalid navigation input: funnel did not cross its navigation corridor` |
| the same query at y 0.05 or y 0 | 2-point path |
| (4.3, 0.12, 2.7) → (4.3, 0.12, 2.71) | 3-point path; the start snaps 3.6 mm sideways to (4.297, 0.05, 2.698) |
| (4.29, 0.12, 2.7) → (4.3, 0.12, 2.7) | 2-point path |
| (4.3, 0.12, 2.7) → goal | succeeds, but the first point is returned twice |

- This is open floor about 0.57 m from the pocket-wall corner, near the eroded
  boundary.
- Endpoints given 7 cm above the surface are valid input under the documented
  3D snap.
- The failing direction pairs and messages match the v7 case. That suggests a
  remaining degenerate case when a snapped endpoint lands on or near a polygon
  edge.
- The duplicated first point in the goal query is a smaller output defect.
- Older binaries were not retained for this point. It was not hit in v7's
  tolerant pass, which reported only (4.7, 1.7).

### Remaining bends and height artifacts (quantified)

**Bends.** The replanned route has three groups of short bends, and none of them
are tile-border kinks:

- around the pillar's eroded disc: (−3.2, 2.0), (−3.0, 1.9), (−2.7, 1.6) and
  (−2.6, 1.4)
- around the doorway-B jamb ends: (−0.6, −3.1) to (0.6, −3.1)
- around the screen wall's south tip: (2.88, 0.42) to (3.3, 0.8)

All of them follow eroded obstacle outlines. The facets of the pillar disc add
about 6 cm to that leg.

**Heights.** Returned points describe the quantized surface, which is +5 cm on
flat floor.

- **West (near) face of the ridge (new regression).**
  - The first route goes from (0.95, 0.07) straight to (1.486, 0.2). Its line
    crosses the ridge's 0.15 m top face at x ≈ 1.28. It is 6.8 cm below the
    top at the face, and below the top for 0.28 m.
  - The replanned route goes (0.994, 0.113) → **(1.084, 0.081)** →
    (1.089, 0.2). That point lies 8 cm inside the ridge footprint and 6.9 cm
    below its top. The segments are below the top for 0.16 m.
  - In v7 the deepest was 2.8 cm, over 0.04 m.
- **Profile camera, t105.** It shows the result: the near-vertical link from
  0.081 to 0.2 renders as an amber post, with its lower part buried in the
  ridge.
- **East (far) face.** Unchanged from v7. The first route keeps 0.19–0.17 m
  points for 0.2 m past the face, then a straight segment down to (3.251, 0.05)
  that floats up to 12 cm over the floor. The replanned route reaches floor
  level 0.29 m past the face.
- **Flat-floor bump.** There is a 1.2 cm bump at (0.724, 0.062).

Physics grounds the character, so these artifacts show only in the markers. None
of this is exact geometry.

### Doorway resolution tradeoff (re-run on v8)

Each width is a separate 8 × 6 m room with a 0.3 m partition and one centred
doorway, radius 0.4 m.

| Doorway width (m) | 1.00 | 1.05 | 1.10 | 1.20 |
|---|---|---|---|---|
| 0.10 m cells | null | null | null | yes |
| 0.05 m cells | null | null | yes | yes |

The thresholds are unchanged. The configured radius is a minimum clearance, and
the cost is conservative passage loss. It is not a claim that every fit is
found.

### Rendered review

I inspected real frames from six cameras. The first five match the v5 to v7
setup, and the sixth is the added `pillar_south` camera. It shows the pass
south of the pillar and the replan turning north along its east side, with a
visible gap and an attached shadow.

The plan view shows the replan hugging the pillar disc, with no knee at the old
tile corner. The character crosses the ridge diagonally, rounds the screen
wall's south tip with clear space and stands on the goal pad at frame 565.
Frames 100 and 105 show the one-tick window and then the reroute.

There is no discrepancy between the returned path and the rendered movement,
apart from the logged 4.6 cm ridge slip. The sunken-segment artifact is in the
returned data, and the markers show it faithfully.

### Incident: one transient script failure during capture

`v8`'s ridge run, its fourth camera, exited with this error at tick 584, about
20 ticks after arrival:

```
Error: "script failed (syntax, exception, memory limit or execution deadline)"
```

- Its 117 frames and 592 log lines are byte-identical prefixes of the passing
  runs.
- The same project runs all 600 ticks without capture.
- 10 more captured ridge runs of the `v8a` project all passed.

That is 1 failure in 36 captured runs on this binary: 4 in `v8`, which stopped
at the failure, 10 in `v8a` and its repeat, 12 in `v8b` and its repeat, and the
10 retries. The behavior issues no navigation query on that
tick. A wall-clock execution deadline during GPU capture is the likeliest cause,
but the message does not say which of the four causes applied. A reliability
gate would need that reason exposed.

### Repeat stability (local equality only)

`v8b` and `v8b-repeat` were each built from scratch.

- All **726/726 frames** across six cameras are byte-identical.
- The logs are identical across both runs and all six cameras.
- Each `report.json` is equal once engine-minted IDs are normalized and
  `wall_ms` is removed.
- The projects are equal once minted IDs and provenance transaction ULIDs are
  normalized.

This was measured on one machine (Apple M5 Pro) with one binary. It is not
evidence of cross-device determinism.

## Commands (repo root)

```sh
shasum -a 256 artifacts/tools/incant_headless        # 6aa98123…26445e = binary.json (before and after)
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
caffeinate -i python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v8b --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run pillar_south:pillar_south:5 \
  --run ridge:ridge:5 --run profile:profile:5                     # and v8b-repeat; v8, v8a, v8a-repeat had 5 cameras
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v8   # refused, exit 1
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v8b \
  artifacts/0026-navigation/v8b/overview.logs.jsonl
# Probes on the v8a project (same scene); each built into a NEW directory:
node tools/build_script.mjs handoffs/0026-navigation-lookdev/tools/walkable_probe.ts artifacts/0026-navigation/walkable-v8/walkable_probe.js
caffeinate -i artifacts/tools/incant_headless play artifacts/0026-navigation/v8a/navigation.incant.json --ticks 1010 \
  --compiled-script artifacts/0026-navigation/walkable-v8/walkable_probe.js --log-output artifacts/0026-navigation/walkable-v8/probe.logs.jsonl
python3 -I handoffs/0026-navigation-lookdev/tools/walkable_check.py artifacts/0026-navigation/v8a \
  artifacts/0026-navigation/walkable-v8/probe.logs.jsonl
# Tolerant pass: the same probe with PROBE_Y = 0.12, PROBE_SNAP = 0.1 (artifacts/0026-navigation/walkable-v8-tolerant).
# detour_probe.ts --ticks 25 -> detour-v8; error_probe.ts --ticks 21 -> error-v8 (same pattern)
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v8-cell0.10 1.0 1.05 1.1 1.2
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v8-cell0.05 --cell-size 0.05 1.0 1.05 1.1 1.2
artifacts/tools/incant_headless play artifacts/0026-navigation/v8a/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/v8a/navigation_course.js     # also --ticks 1 and 99
artifacts/tools/incant_headless play artifacts/0026-navigation/v8a/navigation.incant.json --ticks 600 \
  --compiled-script artifacts/0026-navigation/v8a/navigation_course.js --output artifacts/0026-navigation/flake-v8/ridge-N \
  --camera <ridge camera id> --capture-every 5 --width 960 --height 540   # N = 1..10, all passed
artifacts/tools/incant_headless script artifacts/0026-navigation/v8a/navigation.incant.json \
  artifacts/0026-navigation/v8a/navigation_course.js --ticks 600            # ×3, timing
```

Each scene run is 600 ticks with `--capture-every 5`. That gives 121 frames at
960×540, or 239.3 MiB, under both caps. Every report has `completed: true` and
889 script commands, adapter `Apple M5 Pro`.

Timing on this Mac only; these are not platform budgets. `script --ticks 600`
had a p95 of 2.05–2.19 ms per tick over three runs, with a maximum of 3.5 ms.

## Screenshots

All 12 are under `handoffs/0026-navigation-lookdev/screenshots/` (656 KB). They
replace the v7 set. Each is an unedited `play` frame from `v8b`, byte-identical
in `v8b-repeat`.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier in doorway A; first route still drawn |
| `overview-t105-replanned.png` | Superseded route in grey; replan hugging the pillar toward doorway B |
| `overview-t565-arrived.png` | Character on the goal pad |
| `plan-t050-south-of-pillar.png` | First route around the pillar's south side |
| `plan-t130-replan-hugs-pillar.png` | Replan following the pillar disc; no tile-corner knee |
| `plan-t450-screen-wall-south-tip.png` | Rounding the screen wall's south tip |
| `pillar_south-t100-pillar-clearance.png` | **New camera.** Visible gap passing south of the pillar |
| `pillar_south-t140-turning-north-past-pillar.png` | **New camera.** Turning north along the pillar's east side |
| `ridge-t330-diagonal-ridge-climb.png` | Diagonal autostep climb at the time of the slip |
| `profile-t105-route-dips-below-ridge-top.png` | **The sunken segment**: amber post buried in the ridge at its west face |
| `profile-t335-on-ridge.png` | On the ridge during the diagonal crossing |

In the profile camera, which looks south, east is screen-left. The raw output is
in the ignored `artifacts/0026-navigation/`:

- `v8b`, `v8b-repeat`, `v8a`, `v8a-repeat` and the `*-v8*` probes are final.
- `v8` holds the incident and `flake-v8` holds the retries.
- Earlier versions are history, and `review-v8` holds review sheets.

## Changed paths (this revision)

- `handoffs/0026-navigation-lookdev/result.md`: rewritten.
- `handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py`: adds the
  `pillar_south` camera; the others are unchanged.
- `handoffs/0026-navigation-lookdev/tools/error_probe.ts`: adds the (4.3, 2.7)
  cases and keeps the v7 cases as a regression.
- `handoffs/0026-navigation-lookdev/screenshots/*.png`: replaced with 12 `v8b`
  frames.

## Limitations

- **Course coverage.**
  - The scene covers one flat room, one agent size and one static edit.
  - The walkability map is a 0.2 m grid of the unedited mesh with a 0.75 m
    expectation margin.
  - Thrown queries were found at grid points only. Other degenerate segments may
    exist.
  - The incident's cause is inferred, not confirmed.
- **Untested.** No native screenshot, hosted CI, Rust tests or Clippy were run;
  no Rust changed.
- **Markers.** They show returned points only.
- **Logs.** Captures every 5 ticks cannot show single-tick events, so per-tick
  claims come from the logs. Logged values are rounded to 1 mm.
- **Appearance.** There is no character art or animation, and shading is flat
  `material_preview`.

## Open items for Astra

1. **Throwing query at (4.3, 0.12, 2.7).** It and its reverse throw on valid
   input, with the same messages as the fixed v7 case. Should a degenerate
   funnel or repair case fall back to the raw corridor path instead of throwing?
   The duplicated first point in the goal query may share a cause.
2. **Sunken height detail at the ridge's near face.** Returned points and
   segments fall up to 6.9 cm below the source surface, against 2.8 cm in v7.
   This conflicts with the stated intent that query heights not fall below
   walkable spans.
3. **Script-failure reason.** Could `play` report which of syntax, exception,
   memory or deadline caused a failure? One captured run failed after arrival
   with no navigation work on that tick.
