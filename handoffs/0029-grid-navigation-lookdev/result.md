# 0029 weighted grid navigation rendered-movement review: result

## Status

**Complete for this packet's scope.** A real-engine scene moves one actor across
a 16 × 10 weighted `NavigationGrid` using only actual `api.findGridPath` routes.
The scene shows four states in order:

1. A cheap route through door A that bends around a mud marsh.
2. A more expensive route through a mud gate, after an ordinary shared
   `set_component NavigationGrid` command closes door A.
3. An unreachable destination with a clear stopped state, after the gate also
   closes.
4. Reopening door A, then a turn and arrival.

An independent plus-shaped corner gauge proves that the blocked-corner rule
holds. One query returns null, and another returns the legal orthogonal detour.

All of these checks pass:

- Every per-tick route cost and every probe matches an independent Python
  Dijkstra oracle using the declared metric.
- Saving during the unreachable wait and reopening reproduces the uninterrupted
  runtime state and script state exactly.
- Frames rendered after the reopen are byte-identical to uninterrupted frames at
  the same ticks.
- Two independently authored projects on the same immutable binary produced
  byte-identical logs and 516 byte-identical frames.

**No engine correctness defect was found**, so there is no reproducer for
Astra. Look notes about my own behavior are listed under Rendered review.

This scene is cell navigation only. It does not claim the future sprite or
tilemap renderer, or the 2D physics milestone. **Rapier collision is not
promised.** The scene has no RigidBody or Collider components. The actor moves
with ordinary Transform commands between cell centres. No phase gate is
approved, and finished-game quality is not claimed.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred. Transport is ACP on the host's evidence. `tools/acp/config.json` pins
model `claude-opus-5-5` and the `claude-agent-acp` adapter. From inside the
session I cannot inspect the ACP messages themselves.

### Packet acknowledgement

The current packet has no priority-revision section and carries no new director
feedback. No files existed from an interrupted attempt; the worktree was clean
at `fd9f46a`. I read `CLAUDE.md`, the packet, `docs/spikes/grid-navigation.md`,
`tools/probes/navigation-grid.py`, the SDK types and the 0026 and 0028 helpers.

No Rust, UI, other handoff, mutation routing, credentials or external accounts
were touched. Nothing was compiled from Rust, and nothing was published, merged
or signed. `npm ci --ignore-scripts` installed the pinned Node dependencies into
the ignored `node_modules`.

**Interpretation of "save while disconnected".** I read this as saving while the
destination is disconnected from the actor, meaning the route is null, as the
public probe does. The save is taken mid-wait, and door A reopens after the
resume. No input-device disconnection is involved.

## Binary and source

| Item | Value |
|---|---|
| Binary | `artifacts/tools/incant_headless` |
| SHA-256 before, during and after every engine call | `3b9e91fe3c0165d5333c5836154f40fa3a711230e57fb1bb5f2a4d955b39702f` |
| `binary.json` source commit | `fd9f46a34f52127e06dfcc529b0e8d5907301b16` |
| Worktree base | `fd9f46a` |
| GPU adapter reported by `play` | Apple M5 Pro |
| Compiled behavior SHA-256, sets A and B | `efb7e5aeb83952eff35beb7979390f1e587dae89d23b9cb11b40c3bdd79de6e0` |
| Node, TypeScript, Python | 22.23.3, 5.9.3, 3.14.8 |

| Committed source | SHA-256 |
|---|---|
| `tools/grid_course.py` | `373cf4fb0872fd387790c88370b6e083cec1d4e757a55221d753b38f12c3ca4f` |
| `tools/grid_course.ts` | `676d2b0056caad54d3cfd8a48ccbcbca40f8ccf2ee8457a438741c979ae25344` |
| `tools/trace_check.py` | `7577a7580bb6863a0e815418ccd8ca02352023a76174dda01b583b8c8f9f5f79` |
| `tools/run_set.sh` | `d74008983acc344104b5b74d2f4f65ba5008e982e9fb55cb40437f10a7b39666` |
| `tools/tsconfig.json` | `d1e9b3e068b70aabe177e4d54d7582280028d37d54cb3fb6b68b56ec2fa16be3` |

The helper verifies the binary hash before and after each `play` call. Each set
makes 17 calls, and all 34 recorded pairs in sets A and B match `binary.json`.

## Scene

The helper writes all geometry as original mathematical glTF. The project is
built only through public `init`, `import`, `rpc` (`command.execute` then
`project.save`) and `validate`. No project JSON is edited directly. Entity IDs
are stable deterministic ULIDs, and every created entity has provenance in the
saved document. The authoring file and journal hashes are identical before and
after all play runs.

- **Grid.** It has 16 columns by 10 rows with 1 m cells. Cell (c, r) is centred
  at world (c − 7.5, 0, r − 4.5), and row 0 is north. A wall fills column 8 with
  two openings. Door A at (8, 2) has weight 1. Gate B at (8, 7) has weight 5.
- **Weights.** Plain floor is 1 and is drawn as pale tiles. Mud is 5 and is drawn
  as ochre tiles. Mud covers the six cells around gate B and a six-cell marsh east
  of door A. Blocked cells (weight 0) are raised dark slate blocks 0.55 m tall
  that exactly fill their cells.
- **Doors.** A closed door is a coral slab filling its cell, slightly lower than
  the wall. The behavior moves the slab in the same tick as its grid command, so
  the slab and the grid publish together.
- **Endpoints.** Start (2, 4) has a white ring, and goal (13, 5) has a green ring.
- **Actor.** It is a white puck of radius 0.30 m and height 0.32 m with a dark
  nose wedge, so facing reads from above.
- **Footprint.** Routes are point routes between cell centres. Since 0.30 m is
  under half a cell, a cardinal move stays inside its row. A legal diagonal only
  touches the four walkable cells around the shared corner. The checker measured
  a minimum clearance of 0.2 m between the actor disc and any blocked or closed
  cell.
- **Markers.** A teal line with dots shows the route returned this tick. Small
  dark dots trail the cells actually visited. An amber ring marks the stop cell
  while the destination is unreachable. The corner gauge uses violet.
- **Cameras.** The plan camera is near-orthographic from 40 m with a 16.5° field
  of view and covers the whole grid. The oblique camera is a three-quarter
  overview. The door, gate and gauge cameras are closer three-quarter views.

## Behavior

`tools/grid_course.ts` passes strict `tsc` and is compiled with
`tools/build_script.mjs`. The fixed step is 1/60 s, confirmed by the logged `dt`.

1. **Every tick** the behavior queries from its anchor to the goal, with
   diagonals on and a 4096-expansion limit. The anchor is the cell the actor
   stands on, or the cell its current step is entering. It verifies the returned
   route independently:
   - endpoints match;
   - steps are adjacent;
   - no cell is blocked;
   - diagonals have both orthogonal cells walkable;
   - the cost recomputed from the start-of-tick costs equals `cost`.
   Any violation throws.
2. **Motion.** A new step always goes to `cells[1]` of the route queried from the
   cell just reached. The actor interpolates linearly between adjacent centres at
   3 cells/s. Facing slews at 540°/s. Heading changes over 100° turn in place
   first.
3. **Close door A.** On reaching its second cell, at tick 40, the behavior sends
   `set_component NavigationGrid` with door A's cost set to 0. That tick still
   planned on the start-of-tick grid. Tick 41 returns the gate route.
4. **Close gate B.** When routed through the gate and on reaching a cell two
   cells from it (Chebyshev distance), the behavior closes the gate at tick 89.
   Tick 90 returns null. The amber ring immediately marks the stop cell. The
   actor finishes its current adjacent step, stops at tick 109 and waits.
5. **Reopen.** After 96 stationary unreachable ticks, at tick 204, the behavior
   reopens door A. Tick 205 returns a route. The actor turns 135° in place over
   15 ticks, crosses door A and arrives at tick 476.
6. **Guard.** Closures refuse to target the actor's anchor or the cell it is
   leaving. The checker confirms that no closure did.

## Measured results

Values come from the committed logs and `evidence/check-A-B.json`.

### Course routes

| Tick | Anchor | Door A / gate B | Engine cost | Oracle cost | Generation |
|---|---|---|---|---|---|
| 1 | (2, 4) | open / open | 14242 | 14242 | 1 |
| 41 | (5, 4) | **closed** / open | 23242 | 23242 | 2 |
| 61 | (6, 5) | closed / open | 21828 | 21828 | 2 |
| 90 | (6, 6) | closed / **closed** | **null** | unreachable | |
| 205 | (6, 6) | **open** / closed | 12828 | 12828 | 4 |

- **Cheap route at tick 1.** It runs (2,4) → (5,4) → (6,3) → (7,2) → door
  (8,2) → (9,2) → (9,3) → (9,4) → (10,5) → (11,5) → (12,5) → (13,5). It steps around the marsh instead of
  crossing it.
- **Gate route at tick 41.** It runs (5,4) → (6,5) → (6,6) → (6,7) → (7,7)
  mud → gate (8,7) mud → (9,7) mud → (10,7) → (11,7) → (12,6) → (13,5).
- **Weights at work.** This route takes floor (6,7) then mud (7,7) for 6000
  rather than the diagonal into mud (7,7) for 5 × 1414 = 7070.

Across all 476 querying ticks, every per-tick cost equals the oracle:

| Measure | Value |
|---|---|
| Per-tick cost mismatches | 0 |
| Steps taken | 17 |
| Steps not equal to `cells[1]` of the fresh route | 0 |
| Corner cuts | 0 |
| Grid queries by the behavior, including probes | 485 |

### Independent probes, ticks 2 to 5

| Probe | Engine | Recomputed | Oracle | Cells |
|---|---|---|---|---|
| Gauge pocket (2,8) → diagonal (1,7), both orthogonals blocked | **null** | | unreachable | |
| Gauge detour (3,7) → (3,9) around the east arm | 4000 | 4000 | 4000 | (3,7) (4,7) (4,8) (4,9) (3,9) |
| Cardinal into mud (9,5) → (9,6) | 5000 | 5000 | 5000 | |
| Start on mud not charged (9,6) → (9,5) | 1000 | 1000 | 1000 | |
| Diagonal into floor (3,3) → (4,2) | 1414 | 1414 | 1414 | |
| Doorway corner (7,6) → gate (8,7) | 10000 | 10000 | 10000 | (7,6) (7,7) (8,7) |
| Four-way start → goal | 16000 | 16000 | 16000 | 17 cells |

- **Corner-cutting comparison.** A corner-cutting planner would return 1414 for
  the pocket query and 2828 for the detour query.
- **Error cases.** These threw distinct errors that the behavior caught. Neither
  is a null route. An out-of-bounds start reported "invalid navigation input:
  grid path endpoint is outside the grid". A limit of one expansion reported
  "navigation resource limit exceeded: grid path exhausted its expansion budget".

### Motion

| Check | Result |
|---|---|
| Position off its from → to centre segment | none, under 1e-12 |
| Max displacement per tick | 0.05000000000000071 m, against a 0.05 m limit |
| Max yaw change per tick | 0.1570796326794898 rad, against a 0.15707963267948966 limit |
| Min footprint clearance to blocked cells | 0.2 m |
| Stationary ticks: waiting | 95 |
| Stationary ticks: turning in place | 15 |
| Stationary ticks: arrived | 64 |

Both overshoots are about 1e-16, which is floating-point rounding. The checker
allows a 1e-9 tolerance. No stationary tick falls outside the wait, the turn or
arrival, so there are no unexplained snaps or stalls.

### Save, resume, repeat and frames

| Check | Result |
|---|---|
| Repeat log-only run versus reference | logs byte-identical, state identical |
| Save at tick 156 while unreachable, reopen, run to 540 | final `state` and `script_state` identical |
| Resumed log messages | identical except session-local `generation` |
| Resumed `elapsed_seconds` | max difference 0.0 |
| Reopen window, saved at 194 during the wait, versus uninterrupted gate camera | 12 of 12 shared frames byte-identical |
| Gate-close window, saved at 77, versus reopen window, saved at 194 | 2 of 2 byte-identical |
| Arrive window, saved at 428, versus uninterrupted oblique camera | 12 of 12 byte-identical |
| Set A versus set B: all logs | byte-identical |
| Set A versus set B: frames | 516 of 516 byte-identical |
| Set A versus set B: final state | identical after mapping 18 generated IDs |

- **Generation.** In uninterrupted play, door A's reopen is generation 4. After
  reopening the save it is generation 2, from tick 205 to 476. This is expected,
  because generation is a session cache revision that restarts on reopen. It is
  excluded from identity.
- **Generated IDs.** `init` and `import` generate the project, scene and asset
  IDs, so they differ between independently authored projects. The 18 mapped IDs
  are one project, one scene and 16 assets.

### Script budget and conditions

The supplied engine enforces a 50 ms executing-thread CPU budget per script
tick. Bounded synchronous grid queries count toward it. No `play` call failed.
That covers 34 calls in final sets A and B, and 126 calls including development
sets. The retry path for failed attempts was never used.

**Sleep prevention.** Every recorded run executed under `caffeinate -s -i`, a
temporary sleep assertion. No sleep assertion appears inside the behavior or
helpers.

I have no per-tick CPU measurement. The `play` report gives wall time for the
whole run, including host synchronization:

| Run | Wall time |
|---|---|
| 540-tick reference, log only | about 755 ms |
| 384-tick resume | about 534 to 554 ms |

These figures are not a frame-time or device guarantee.

## Rendered review

The 14 committed screenshots are unedited engine frames at 960 × 540 from set A.
Set B produced byte-identical copies, as recorded in
`evidence/screenshots.sha256`.

| Screenshot | Shows |
|---|---|
| `plan-t0020-cheap-route-via-door-a.png` | Cheap route through door A, stepping around the marsh |
| `door-t0040-door-a-closed-route-planned-on-start-of-tick-grid.png` | Door slab closed; the drawn route still uses this tick's start-of-tick grid |
| `door-t0041-replanned-through-mud-gate.png` | Next tick: route through the mud gate |
| `plan-t0045-expensive-route-through-gate-b.png` | Changed route from above; (6,7) is preferred over a diagonal into mud |
| `gate-t0089-gate-b-closed-route-planned-on-start-of-tick-grid.png` | Gate slab closed on the edit tick |
| `gate-t0091-null-route-stop-cell-marked.png` | Null route: line cleared, amber ring on the stop cell, actor finishing its step |
| `plan-t0150-stopped-waiting-both-closed.png` | Stopped and waiting; both openings closed |
| `gate-t0204-door-a-reopened-still-waiting.png` | Door A slab removed on the reopen tick |
| `gate-t0205-route-restored-via-door-a.png` | Next tick: route restored, ring cleared |
| `gate-t0212-turning-in-place.png` | 135° turn in place before moving |
| `plan-t0310-entering-door-a-while-turning.png` | Leaving (7,2) for door A while facing slews east |
| `oblique-t0476-arrived.png` | Arrival on the goal ring |
| `plan-t0480-arrived-trail.png` | Whole course trail: toward the gate, stop, back through door A |
| `gauge-t0003-blocked-corner-null-and-detour.png` | Corner gauge: X over the pinch for the null query, violet orthogonal detour |

**Judgement:**

- **Alignment.** Route and obstacles line up. Routes run through cell centres
  and enter both openings cardinally.
- **No visible corner cutting.** This holds both in pixels and in the per-tick
  check.
- **Route changes are explained.** Each one appears exactly one tick after its
  command.
- **Stop and restart are clear.** The ring appears at the first null tick, the
  route clears, and the ring clears when the route returns.
- **No unexplained snaps.**

Look notes about my own behavior, not engine defects:

1. **One-frame stale line on edit ticks.** On ticks 40, 89 and 204 the slab
   changes in the same frame while the drawn route still reflects the
   start-of-tick grid. This is the contract working as documented. The screenshot
   names say so, but a casual viewer could read it as a lag.
2. **Hard corners at cell centres.** Motion is deliberately linear between
   centres with no smoothing. Facing lags by up to five ticks on 45° turns and
   ten ticks on 90° turns. This is visible in `plan-t0310`.
3. **Committed step after a null route.** The actor completes the step it began
   on the previous route, for 19 ticks, before stopping. The amber ring announces
   where it will stop. Reversing mid-step would need a non-cell position as a
   query start, which this point/cell API does not take.
4. **Small markers in the oblique view.** Trail dots and the gauge line are small
   there. The plan view is the reference for routes.
5. **Occlusion in the gate view.** The route line passes behind the wall where it
   crosses door A.

## Reproduce

From the repo root, with `node_modules` installed (`npm ci --ignore-scripts`):

```sh
shasum -a 256 artifacts/tools/incant_headless   # must equal artifacts/tools/binary.json
caffeinate -s -i bash handoffs/0029-grid-navigation-lookdev/tools/run_set.sh artifacts/0029-grid-navigation/A
caffeinate -s -i bash handoffs/0029-grid-navigation-lookdev/tools/run_set.sh artifacts/0029-grid-navigation/B
python3 -I handoffs/0029-grid-navigation-lookdev/tools/trace_check.py artifacts/0029-grid-navigation/A \
    --compare artifacts/0029-grid-navigation/B --json artifacts/0029-grid-navigation/check-A-B.json
```

`run_set.sh` refuses an existing output directory, and `trace_check.py` refuses
an existing JSON output. Each capture stays within 128 frames and 256 MiB of raw
output.

| Capture | Frames | Ticks |
|---|---|---|
| Plan | 109 | full course, every 5 |
| Oblique | 91 | full course, every 6 |
| Gate | 91 | full course, every 6 |
| Gauge | 5 | 2 to 6 |
| Door-close | 49 | 28 to 76 |
| Gate-close | 61 | 77 to 197, every 2 |
| Reopen | 73 | 194 to 266 |
| Arrive | 37 | 428 to 500, every 2 |

Raw evidence stays under the ignored `artifacts/0029-grid-navigation/`. The final
sets are `A` and `B`. The `dev*` sets are earlier look iterations before the
camera and stop-cue changes. They also had no failed attempts.

## Changed paths

- `handoffs/0029-grid-navigation-lookdev/result.md`
- `handoffs/0029-grid-navigation-lookdev/tools/`: `grid_course.py`, `grid_course.ts`, `trace_check.py`, `run_set.sh`, `tsconfig.json`
- `handoffs/0029-grid-navigation-lookdev/screenshots/`: 14 PNG frames
- `handoffs/0029-grid-navigation-lookdev/evidence/`: `check-A-B.json`, `events-A.jsonl`, `geometry-A.json`, `helper-A.json`, `screenshots.sha256`

## Limitations and unavailable tests

- **No physics.** There are no Rapier or physics bodies. Collision, grounding and
  actor clearance are not engine-checked. Clearance is guaranteed only by the
  authored obstacles and the 0.30 m radius, and is measured by the checker.
- **No 2D renderer.** No sprite or tilemap renderer is used, and no 2D physics
  claim is made. The 3D renderer stands in for a top-down view.
- **No device or native-window testing.** Runs used only the supported headless
  renderer on this Mac. The Mac was locked, and no attempt was made to bypass it.
  Native UI is out of scope.
- **Single platform.** There is no cross-device determinism or platform budget
  certification. Byte-identical repeats were on one machine with one binary.
- **No per-tick CPU measurement.** No CPU-budget measurement was available.
- **Mock tests are not gates.** The Python oracle is an independent check of
  logged values. It is not a live gate.

## Verdict

For this scoped increment, weighted grid navigation in a rendered real-engine
scene behaves correctly and reads clearly. That covers routes, the declared
cost metric, blocked-corner handling, door edits publishing next tick, the null
stopped state, save and resume during the null state, and repeatability. No
engine defect was found. Phase gates, the 2D renderer and physics milestones,
and finished-game quality remain unapproved.

## Open questions

- Should a later API accept a fractional start position or a "current step"
  hint? That would let an actor reverse mid-step when a route disappears.
  Today's cell-only queries require finishing the committed step.
- Should the probe's corner-gauge pattern become a standing rendered regression
  scene once the 2D tilemap renderer exists?
