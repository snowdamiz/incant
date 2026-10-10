# 0026 navigation rendered-motion review: result

## Status

Final verdict on binary `d4fa8794…7326710` (source `020c640`): **no
navigation query threw anywhere in this review.** The checks were:

- the repro cases from v7 and v8, in both directions
- a new dense probe of 2,904 short queries around both former failure sites, at
  three heights and four directions
- both full-room walkability passes, about 4,000 queries each

The floor east of the pillar is still restored, and the live course is unchanged
from v8. The character follows real `api.findPath` routes:

- It stays within 0.85 mm of the returned polyline, except for a 4.6 cm autostep
  slip on the diagonal ridge climb.
- It is grounded every tick, never touches a wall and arrives exactly at
  tick 563.
- The room edit rebuilds three tiles and the replan reroutes.
- The pocket query returns `null` both times.
- Every returned point and segment has a marker.

v9 and v9-repeat are byte-identical, and no capture attempt failed on this
binary.

Remaining limitations, stated explicitly:

- **Heights.** Returned heights describe a quantized, piecewise-linear surface.
  It smooths step discontinuities and can fall below the source geometry. The
  first route's segments run up to 6.8 cm below the true ridge top over 0.28 m.
  The replanned route's run up to 6.9 cm below it over 0.16 m, which includes
  the point (1.084, 0.081, −2.354). Both are unchanged from v8. The character
  is unaffected: physics holds it at exactly ridge-top rest height there.
  **This is not exact source geometry.** Source-conforming height detail
  remains a navigation quality follow-up.
- **Very short first segment.** A query starting 7 cm above the floor at
  (4.3, 0.12, 2.7) toward the goal begins with a 0.52 mm height-detail segment.
  It does not throw. An earlier version of this report called it a duplicated,
  zero-length point; that was a misreading of 3-decimal rounding. See priority
  revision 6 below.
- **Doorways.** The configured radius is a minimum clearance, and convex corners
  keep 0.50–0.60 m. At 0.1 m cells, doorways need at least 1.2 m; at 0.05 m
  cells, at least 1.1 m. A 0.4 m-radius agent physically fits anything wider
  than 0.8 m.
- **Script failures.** The generic script deadline failure, seen once in v8, is
  an engine robustness issue this binary does not change. It did not recur here.

This verdict does not approve a phase gate, mark the phase complete or claim a
passed full-engine gate. Portal-graph optimality is not a global continuous
shortest-path claim. Steering, off-mesh links, 2D navigation, the Navigation
Inspector and native debug draw are not claimed.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP, on host-supplied evidence. The director states that Astra
invoked the JSON-RPC ACP adapter in `tools/handoff/main.py`. From inside the
session I can see only a Claude Code agent session in this worktree. I cannot
inspect the transport layer myself.

### Priority revision 6 acknowledged: rounded-output correction

This revision is report-only, at Astra's request after numerically validating
the raw log. Astra pointed out that the v9 log contradicts my
"zero-length first segment" finding. I re-read
`artifacts/0026-navigation/error-v9/probe.logs.jsonl` at tick 19 at full
precision:

| Point | x | y | z |
|---|---|---|---|
| first | 4.29685115814209 | 0.05017497390508652 | 2.698483943939209 |
| second | 4.297078609466553 | 0.05000009387731552 | 2.6980462074279785 |

The points are **0.523 mm apart in 3D**: 0.493 mm horizontally and 0.175 mm
vertically. My analysis printed coordinates rounded to 3 decimals, so both
showed as (4.297, 0.05, 2.698).

**This is a very short height-detail segment, not a duplicated point or a
zero-length segment.** I have amended that finding throughout. The v8 log for
the same query shows the same 0.523 mm separation, so the v8 report's
"returned twice" wording was the same misreading.

No engine code or binary changed, and this is not a code fix. Every v9 capture,
probe, frame and screenshot below is unchanged and still valid. No new captures
were run.

### Priority revision 5 acknowledged (2026-10-10)

The revision covers Astra's f64 portal, funnel, visibility and barycentric math,
with ULP-based boundary tolerance. It also clarifies the height scope. All of it
was applied:

- **New binary, new outputs.** I verified the binary against `binary.json`
  before and after every run. The final runs are `v9` and an exact
  `v9-repeat`, with the same scene and all six cameras, including the
  `pillar_south` camera disclosed in v8. Earlier runs are kept as history.
- **Probes re-run.** I re-ran every query probe, both walkability passes,
  clearance, grounding, arrival and the doorway sweep. I added a dense boundary
  probe. Every probe query is wrapped so that any exception would be counted,
  not masked.
- **Height scope adopted.** The point (1.084, 0.081, −2.354) is the documented
  piecewise-linear approximation. I keep the v8 measurement as history,
  re-measured v9, and verified actual character grounding at that point.
- **Sleep assertions.** Long runs used `caffeinate -i`. The lock was not
  bypassed, and there was no native interaction. All attempts are recorded
  below, and none failed.
- **Unchanged areas.** No Rust, core algorithm, UI layout, other packet or
  mutation routing changed.

## Before / after findings

| Finding | v6 | v7 | v8 | v9 (final) |
|---|---|---|---|---|
| Fixed-start diagnostic, (−3.44, 0) → goal | 15.27 m, tile kink | 13.634 m | 13.634 m | 13.634 m |
| Missing floor east of the pillar | present | mapped | none | **none** |
| Live replan leg, (−3.29, 2.0) → doorway-B jamb | — | 6.193 m via tile corner | 5.664 m | 5.664 m |
| Throw at (4.7, 1.7) ↔ (4.71, 1.7) | — | throws | fixed | **fixed** |
| Throw at (4.3, 0.12, 2.7) ↔ (4.31, 0.12, 2.7) | — | not hit | throws | **fixed** (3-point path) |
| Thrown queries in probes | — | 1 site | 1 site | **0** (2,904 dense + 2 room maps + repro set) |
| Deepest returned segment below the true ridge top | — | 2.8 cm over 0.04 m | 6.9 cm over 0.16–0.28 m | 6.9 cm over 0.16–0.28 m (accepted approximation) |
| Character height where returned heights dip | — | — | 0.97 m, grounded | **0.97 m, grounded** |
| Narrowest passable doorway, 0.1 / 0.05 m cells | 1.2 / 1.1 m | 1.2 / 1.1 m | 1.2 / 1.1 m | 1.2 / 1.1 m |
| Navigation polygons before → after the edit | 145 → 120 | 145 → 120 | 147 → 122 | 147 → 122 |
| Captured-run failures | 0 | 0 | 1 of 36 | **0 of 22** |
| Arrival | Tick 562 | Tick 578 | Tick 563 | **Tick 563**, exact |

## Final evidence (`v9`, identical in `v9-repeat`)

### Scene (unchanged since v8)

The room is 16 × 10 m. A partition at x = 0 has doorway A (z 0.4–2.0) and
doorway B (z −4.4 to −2.8). The remaining pieces are:

- a capsule pillar, r 0.45 m, at (−3.6, 1.0), which is a curved `collider`
  source
- a 0.15 × 0.6 m ridge at x 1.0–1.6, a cooked `mesh` source
- a screen wall
- a pocket behind a 0.5 m slit
- a muted-blue barrier that is moved by script

The floor is a cooked `mesh` source. All geometry is original mathematical glTF
with matching analytic colliders.

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

There are six cameras: overview, plan, pillar (north side), `pillar_south`
(added in v8), ridge and profile.

### Queries, edit, rebuild and coverage

| Tick | Event | Result |
|---|---|---|
| 1 | Route from start to goal | 14 points, 13.29 m, south of the pillar, via doorway A |
| 1 | Route from start to pocket | **`null`** |
| 100 | Barrier into doorway A | Committed at the end of the tick |
| 101 | Replan from (−3.291, 2.0) | 28 points, 15.32 m. It hugs the pillar disc, goes through doorway B, crosses the ridge diagonally and rounds the screen wall's south tip |
| 101 | Route from start to pocket | **`null`** |
| 563 | Arrived | Final position equals the goal to 1 mm |

The route points are identical to v8.

- **Marker coverage.** Every drawn route is fully covered: 14, 14 and 28 points,
  with pools of 48.
- **Build report.** I stopped playback at tick 100 to see the edit's build. It
  rebuilt tiles (2,1), (2,2) and (2,3), and reused 17. Polygons dropped from
  147 to 122.

### Motion, every tick (`trace_check.py`)

| Check | Result |
|---|---|
| Deviation from the active returned polyline | max **4.6 cm** (t333, diagonal ridge climb); ≤ 0.85 mm on every other tick before arrival |
| Stalls below 90% of the requested step | Only t325–336, the ridge climb, at 79–88% |
| Grounded | **Every tick** |
| Contacts | Floor and ridge only. No wall, pillar, barrier or pocket contact |
| Closest capsule gap | Partition 0.200 m (doorway B jamb), pillar 0.250 m, screen wall 0.295 m |
| Height on flat floor or ridge top | At rest height; the worst error is the 1 cm spawn settle at t1 |
| Largest vertical move per tick | 2.5 cm (t389, stepping off the ridge) |

The ridge slip is character-controller autostep behaviour, as classified in
0023. Compared with v8, per-tick positions differ by at most 4 mm, during and
after the ridge climb from t326 onward. Both runs reach the same final pose and
arrival tick.

### Query robustness

| Probe | Result |
|---|---|
| `boundary_probe.ts`, new | **2,904 / 2,904 paths, 0 nulls, 0 exceptions** |
| `error_probe.ts` | All 20 cases return paths |
| `walkable_probe.ts`, tight pass: y 0.05, 6 cm snap | No thrown queries; no missing open floor except 3 points of height-smeared floor (2.5–2.7, 1.5–1.9), as in v7 and v8 |
| `walkable_probe.ts`, tolerant pass: y 0.12, 0.1 m snap | No thrown queries, **0 missing points**; v8 threw at (4.3, 2.7) |
| `detour_probe.ts` | Fixed start 13.634 m; replan leg 5.664 m; every former tile-border crossing is a straight 2-point path |

The dense boundary probe covers both former failure sites, (4.7, 1.7) and
(4.3, 2.7). Each site has an 11 × 11 grid at 1 cm spacing, at heights 0, 0.05
and 0.12. Each point issues four 1 cm queries, in ±x and ±z. Every query is
wrapped so that any exception would be counted, and none was.

The `error_probe.ts` cases cover:

- (4.7, 1.7) ↔ (4.71, 1.7) at y 0 and y 0.05
- (4.3, 0.12, 2.7) ↔ (4.31, 0.12, 2.7)
- their neighbours

The v8 throwing pair now returns a 3-point path. The start snaps 3.5 mm
horizontally onto (4.29685, 0.05017, 2.69848), then the path continues to the
end.

**Very short first segment (corrected in revision 6).** (4.3, 0.12, 2.7) → goal
begins at (4.29685, 0.05017, 2.69848) and then goes to (4.29708, 0.05000,
2.69805). That is a **0.523 mm** height-detail segment, not a duplicated point.
It does not throw. The full-precision v8 log shows the same values.

### Heights and grounding (documented approximation)

Returned heights describe the quantized, piecewise-linear navigation surface.
That is +5 cm on flat floor here, and it can fall below the source geometry near
a step.

| Route | Deepest returned segment below the true ridge top | Horizontal length below |
|---|---|---|
| First route | 6.8 cm at (1.0, 1.363) | 0.28 m |
| Replanned route | 6.9 cm at (1.083, −2.356), at the returned point (1.084, 0.081, −2.354) | 0.16 m |

These values are unchanged from v8, and v7's deepest was 2.8 cm. Astra traces
the undershoot to a detail triangle interpolating between a low sample at
x = 0.97 and high samples at x = 1.06 and 1.518.

- **East (far) face.** Points stay high for 0.2 m past the face, then a straight
  segment floats up to 12 cm over the floor on the first route. The replanned
  route reaches floor level 0.29 m past the face.
- **Profile view.** The profile frame at t105 shows the sunken link as an amber
  post partly buried in the ridge.

**Actual grounding is correct.** Over ticks 343–350 the capsule axis passes
x 1.015–1.142, including (1.087, −2.349) next to the dipping point. Its centre
is at exactly 0.97 m on every one of those ticks, which is rest height plus the
0.15 m ridge, and it is grounded. It is never airborne anywhere in the run.
Physics, not the returned height, places the character.

### Doorway resolution tradeoff (re-run on v9)

| Doorway width (m) | 1.00 | 1.05 | 1.10 | 1.20 |
|---|---|---|---|---|
| 0.10 m cells | null | null | null | yes |
| 0.05 m cells | null | null | yes | yes |

The thresholds are unchanged. The configured radius is a minimum clearance, and
the cost is conservative passage loss. It is not a claim that every fit is
found.

### Rendered review

I inspected real frames from all six cameras:

- the start and the edit tick, with its one-tick window
- the replan hugging the pillar, in the plan view and `pillar_south`, with a
  visible gap and an attached shadow
- doorway B, the diagonal ridge climb and its small slip
- the sunken marker post in the profile view
- the screen wall's south tip, and arrival on the goal pad at frame 565

There is no discrepancy between the returned path and the rendered movement,
apart from the logged 4.6 cm ridge slip. The height undershoot is in the
returned data, and the markers show it faithfully.

Compared with v8, 445 of the 726 frames are byte-identical. The other frames
differ only by the ≤ 4 mm motion change after t326. 4 of the 12 committed
screenshots changed bytes.

### Repeat stability (local equality only)

`v9` and `v9-repeat` were each built from scratch.

- All **726/726 frames** across six cameras are byte-identical.
- The logs are identical across both runs and all six cameras.

This was measured on one machine (Apple M5 Pro) with one binary. It is not
evidence of cross-device determinism.

### Capture attempts (all recorded)

| Attempt | Result |
|---|---|
| `v9`, 6 cameras | passed |
| `v9-repeat`, 6 cameras | passed |
| `flake-v9`, ridge camera ×10 | 10 passed |

That is **0 failures in 22 captured runs** on this binary. Every probe and
`play` run exited 0. v8's single generic script failure, 1 of 36, is kept as
history. This binary has no deadline change, so the robustness issue is not
closed by this result.

## Commands (repo root)

```sh
shasum -a 256 artifacts/tools/incant_headless        # d4fa8794…7326710 = binary.json (before and after)
node_modules/.bin/tsc -p handoffs/0026-navigation-lookdev/tools/tsconfig.json   # strict, exit 0
caffeinate -i python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v9 --ticks 600 \
  --run overview:overview:5 --run plan:plan:5 --run pillar:pillar:5 --run pillar_south:pillar_south:5 \
  --run ridge:ridge:5 --run profile:profile:5                     # and v9-repeat
python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py artifacts/0026-navigation/v9   # refused, exit 1
python3 -I handoffs/0026-navigation-lookdev/tools/trace_check.py artifacts/0026-navigation/v9 \
  artifacts/0026-navigation/v9/overview.logs.jsonl
# Probes on the v9 project; each compiled into a NEW directory, then:
#   caffeinate -i artifacts/tools/incant_headless play artifacts/0026-navigation/v9/navigation.incant.json \
#     --ticks N --compiled-script DIR/probe.js --log-output DIR/probe.logs.jsonl
#   boundary_probe.ts N=730 -> boundary-v9; error_probe.ts N=21 -> error-v9; detour_probe.ts N=25 -> detour-v9
#   walkable_probe.ts N=1010 -> walkable-v9; tolerant copy (PROBE_Y 0.12, PROBE_SNAP 0.1) -> walkable-v9-tolerant
python3 -I handoffs/0026-navigation-lookdev/tools/walkable_check.py artifacts/0026-navigation/v9 \
  artifacts/0026-navigation/walkable-v9/probe.logs.jsonl            # and walkable-v9-tolerant
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v9-cell0.10 1.0 1.05 1.1 1.2
python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py artifacts/0026-navigation/doorway-v9-cell0.05 --cell-size 0.05 1.0 1.05 1.1 1.2
artifacts/tools/incant_headless play artifacts/0026-navigation/v9/navigation.incant.json --ticks 100 \
  --compiled-script artifacts/0026-navigation/v9/navigation_course.js     # also --ticks 1 and 99
artifacts/tools/incant_headless script artifacts/0026-navigation/v9/navigation.incant.json \
  artifacts/0026-navigation/v9/navigation_course.js --ticks 600             # ×3, timing
```

Each scene run is 600 ticks with `--capture-every 5`. That gives 121 frames at
960×540, or 239.3 MiB, under both caps. Every report has `completed: true` and
889 script commands, adapter `Apple M5 Pro`.

Timing on this Mac only; these are not platform budgets. `script --ticks 600`
had a p95 of 2.42–2.51 ms per tick over three runs, with a maximum of 4.1 ms.
v8 measured a p95 of 2.05–2.19 ms. These are single-session measurements, so I
am not attributing the 0.3 ms difference.

## Screenshots

All 12 are under `handoffs/0026-navigation-lookdev/screenshots/`. The names and
checkpoints are unchanged from v8, and every file now holds the `v9` frame,
byte-identical in `v9-repeat`. Four changed bytes against v8, all from the
≤ 4 mm motion difference: t330 ridge, t335 profile, t450 plan and t565
overview.

| File | Shows |
|---|---|
| `overview-t000-start.png` | Room, parked barrier, start, goal and pocket target |
| `overview-t100-edit-committed-old-route.png` | Barrier in doorway A; first route still drawn |
| `overview-t105-replanned.png` | Superseded route in grey; replan hugging the pillar |
| `overview-t565-arrived.png` | Character on the goal pad |
| `plan-t050-south-of-pillar.png` | First route around the pillar's south side |
| `plan-t130-replan-hugs-pillar.png` | Replan following the pillar disc |
| `plan-t450-screen-wall-south-tip.png` | Rounding the screen wall's south tip |
| `pillar_south-t100-pillar-clearance.png` | Gap passing south of the pillar |
| `pillar_south-t140-turning-north-past-pillar.png` | Turning north along the pillar's east side |
| `ridge-t330-diagonal-ridge-climb.png` | Diagonal autostep climb |
| `profile-t105-route-dips-below-ridge-top.png` | Documented height approximation: amber link partly below the ridge top |
| `profile-t335-on-ridge.png` | Character on the ridge |

In the profile camera, which looks south, east is screen-left. The raw output is
in the ignored `artifacts/0026-navigation/`:

- `v9`, `v9-repeat`, `flake-v9`, the `*-v9*` probes and `doorway-v9-*` are
  final.
- Earlier versions are history.
- `review-v9` holds review sheets made from real frames.

## Changed paths (this revision)

Revision 6 changed only `handoffs/0026-navigation-lookdev/result.md`. It amends
the short-segment finding in the status, the robustness section and the open
items, and corrects the snap distance from 3.6 mm to 3.5 mm. No tool, binary,
capture or screenshot changed.

Revision 5 changed:

- `handoffs/0026-navigation-lookdev/result.md`: rewritten.
- `handoffs/0026-navigation-lookdev/tools/boundary_probe.ts`: new dense
  short-query probe.
- `handoffs/0026-navigation-lookdev/tools/tsconfig.json`: now includes the probe.
- `handoffs/0026-navigation-lookdev/screenshots/`: 4 PNGs updated to v9 frames.

## Limitations

- **Course coverage.**
  - The scene covers one flat room, one agent size and one static edit.
  - The boundary probe covers two sites densely.
  - The room map uses a 0.2 m grid with a 0.75 m expectation margin. Other
    degenerate geometry may exist elsewhere.
- **Heights.** They remain a documented approximation. Source-conforming height
  detail is a follow-up, not a passed gate.
- **Script-failure cause.** The cause of v8's single generic failure remains
  unknown. The error message does not say whether it was syntax, an exception,
  memory or a deadline.
- **Untested.** No native screenshot, hosted CI, Rust tests or Clippy were run;
  no Rust changed.
- **Markers.** They show returned points only.
- **Logs.** Captures every 5 ticks cannot show single-tick events, so per-tick
  claims come from the logs. Logged values are rounded to 1 mm.
- **Appearance.** There is no character art or animation, and shading is flat
  `material_preview`.

## Open items for Astra

1. **Very short first segment.** (4.3, 0.12, 2.7) → goal starts with a
   0.523 mm height-detail segment. It is not zero-length and does not throw;
   this is noted for completeness only. An earlier version of this report
   misread it as a duplicate; see revision 6.
2. **Source-conforming height detail.** This is the follow-up for the 6.9 cm
   undershoot near the ridge's near face.
3. **Script-failure reason.** `play` reports a generic failure that does not say
   which cause applied. Exposing the cause would let the v8 incident be
   classified.
