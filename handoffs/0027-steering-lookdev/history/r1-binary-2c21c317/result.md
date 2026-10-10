> **Historical report (superseded).** This reviewed binary `2c21c317…a6374e` (source `20ca453`), now kept in `artifacts/tools/prior-0027/`. It was committed as `25db3a8`. The current review is `../../result.md`.

# 0027 local-avoidance rendered-motion review: result

## Status

**Complete for this packet's scope.** One hundred walkers crossed a plaza with
real `api.steerAgents` proposals. Physical bodies never overlapped. All walkers
arrived and stayed arrived from tick 2580. Two independent repetitions produced
byte-identical frames and logs.

The review found **one engine robustness problem** and **three steering
behaviors** to document:

1. **Wall-clock deadline aborts (engine, robustness).** Identical `play`
   commands aborted at random ticks with "script failed" when the Mac slept or
   another build loaded the CPU. A healthy tick takes about 12–15 ms. The 50 ms
   budget includes parsing the whole project JSON on every tick.
2. **Goal-block standoffs (steering, expected limitation).** Two mirror-image
   standoffs held a late walker 1.3–1.4 m short of its goal for 590 and 1002
   ticks. Both resolved by slow creep without overlap.
3. **Roundabout sweep (steering, expected limitation).** The four-way crossing
   formed a stable counter-clockwise roundabout. It carried several walkers up
   to 13.3 m sideways into other groups' blocks before they walked back.
4. **Margin is honored, not exceeded meaningfully.** The tightest pair came
   4.7 mm inside the 0.60 m margin distance. The plinth gap came 0.6 mm inside
   its 5 cm margin. The physical gap never fell below 9.5 cm between bodies.

This result does not approve a phase gate or claim the wider phase complete.
It makes no physics-collision claim: walkers have no RigidBody or Collider, so
the frames show the engine integrating steering proposals. It certifies no
cross-device determinism and no platform performance budget.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP on the host's evidence. `tools/acp/config.json` pins model
`claude-opus-5-5` and the `claude-agent-acp` adapter. This session runs from the
repository's pinned `@anthropic-ai/claude-agent-sdk-darwin-arm64` binary with
the TypeScript SDK entrypoint. That is consistent with the JSON-RPC adapter in
`tools/handoff/main.py`. From inside the session I cannot inspect the ACP
messages themselves.

### Packet acknowledgement

This packet has no priority-revision section and no earlier director feedback.
No files existed from an interrupted attempt; the worktree was clean. I read
the packet, `docs/spikes/navigation-steering.md`, the generated SDK types,
`tools/probes/game-steering.py`, the 0026 helpers and the relevant engine
sources before changing anything.

No Rust, UI layout, other handoff or mutation routing was changed. Nothing was
compiled from Rust. Nothing was published, merged or signed. No credentials
were read. `npm ci` installed the pinned Node dependencies into this worktree's
ignored `node_modules`.

## Scene

All geometry is original mathematical glTF written by the helper. The project
is built only through public `init`, `import`, `rpc` (`command.execute` then
`project.save`) and `validate`. No project JSON is edited directly.

- **Plaza.** A 28 × 28 m floor with 1 m checker tiles in two neutral greys. The
  tiles double as a measuring grid in the close views.
- **Walkers.** 100 capsules, radius 0.25 m and height 1.8 m, feet at the entity
  origin. Visual and steering dimensions are identical. Each has only
  Transform, MeshRenderer and Velocity.
- **Groups.** Four 5 × 5 blocks at 0.9 m pitch start 10 m west, east, north and
  south of centre. Each block walks 20 m to the opposite block, keeping its
  formation, so goals mirror the start layout.
- **Static obstacle.** A regular octagonal plinth, circumradius 1.0 m and 0.9 m
  tall, sits at the centre of the crossing.
- **Height filter.** An overhead gantry beam at 2.1–2.35 m spans the western
  lane at x = −5.5 m on two posts. The beam is deliberately supplied to the
  solver; its interval does not overlap a 0–1.8 m walker.
- **Markers.** Goal dots and trail dots are ordinary mesh entities. Two walkers
  per group carry a near-black cap and drop a trail dot every 20 ticks, so dot
  spacing shows speed. The engine reports zero diagnostic entities in every
  frame; nothing is native debug draw.

Each group colour is separated by luminance as well as hue:

| Group | Direction | Colour |
|---|---|---|
| W | west to east | amber, light |
| E | east to west | muted blue, mid |
| N | north to south | bone, lightest |
| S | south to north | brick, dark |

The structure stays in neutral greys. The near-black cap marks tracked walkers.

### Cameras

- **Overview.** An elevated oblique view of the whole plaza.
- **Plan.** A top-down view of the whole plaza.
- **Plinth and endpoint telephotos.** Near-orthographic views from 40 m with a
  10° lens. A body top is magnified by only 4.7%, so footprint gaps are not
  visibly understated. A low close camera was tried first. Its wall of 1.8 m
  bodies hid the plinth, so it was replaced.
- **Crossing.** A mid-height oblique view of the roundabout.
- **Gantry.** A low view of walkers passing under the beam.

The sun sits at 70° elevation, so a 1.8 m body casts a 0.65 m shadow. Longer
shadows darkened the gaps in top-down views.

## Behavior

`tools/steering_plaza.ts` is strict TypeScript compiled by
`tools/build_script.mjs`. Each tick it:

1. reads every walker's start-of-tick Transform and Velocity;
2. computes a preferred velocity toward the goal at up to 1.4 m/s, with exact
   arrival;
3. calls `api.steerAgents` once for all 100 walkers, with all four obstacles,
   time horizon 2 s, obstacle horizon 1 s and margin 0.05 m;
4. applies every proposal as an ordinary Velocity command.

The only routing is behavior-owned. While the goal lies beyond the plinth, a
walker aims at a tangent that keeps the plinth on its left. It goes straight
once the goal points away from the plinth.

The first version lacked that last condition and made walkers orbit the plinth
indefinitely. That was my bug, found in the logs and fixed before any capture.

Every tick logs one diagnostic row and one row of all 100 positions in
millimetres. `tools/trace_check.py` recomputes everything from the positions
alone and cross-checks the behavior's own row.

## Measured results

Both repetitions give identical numbers. Positions are rounded to 1 mm in the
log, so distances carry about 1.4 mm of error.

| Measure | Result |
|---|---|
| Ticks with any physical overlap (centres < 0.50 m) | **0 of 2700** |
| Minimum centre separation | 0.5953 m at tick 1284 |
| Ticks with a pair inside the 0.60 m margin distance | 2262 of 2700 |
| Minimum plinth gap from a 0.25 m footprint | 0.0494 m at tick 477 |
| Minimum gantry-post gap | 0.0499 m at tick 1414 |
| Maximum applied speed, from the behavior | 1.400 m/s |
| Maximum displacement speed, from mm positions | 1.476 m/s, within rounding |
| Maximum neighbours of one walker | 99 |
| First arrival (unobstructed walk is 857 ticks) | tick 913 |
| Last first arrival | tick 2128 |
| All 100 at goal, and stay there | **from tick 2580** (43.0 s) |
| Walkers with a stall over 60 ticks | 17, all within 2.4 m of their goals |
| Longest stall | 1002 ticks, Walker S 3-4, 1.39 m short |
| Largest sideways sweep | 13.3 m (E), 12.7 m (W), 10.0 m (N), 7.2 m (S) |

The trace check matches the behavior's own row on separation and plinth gap on
every tick. The arrived count differs by one on 26 ticks. Those are walkers
within about 1 mm of the 5 cm arrival radius, where millimetre rounding flips
the test.

### Height filter

48 walkers crossed the beam's line, inside the posts, at 1.05 m/s or faster.
A solid obstacle at walker height there would have walled off the lane. No
control run with an unfiltered beam was made.

## Observed motion

These observations come from inspecting real frames. Every committed frame is
an unedited copy of engine output, identical in both repetitions.

- **Approach, ticks 0–340.** The blocks march in formation. The western block
  passes under the gantry beam without deflecting.
- **First contact, ticks 343–348.** Contacts between groups happen
  symmetrically, 1.6–2.0 m from the centre. The keep-left tangent turns
  each block's leading rows before they meet.
- **Roundabout, ticks 350–700.** About 59 walkers sit within 3 m of the centre
  at peak, near tick 450. They circulate counter-clockwise around the plinth.
  The telephoto frames show distinct footprints with visible gaps throughout.
  At tick 476 the closest walker stands at the plinth's lower-left edge,
  matching the logged 4.9 cm gap.
- **Sweep, ticks 700–1500.** The circulation carries some walkers out with the
  wrong stream. At tick 1200, blue walkers stand in the south block and amber
  walkers in the north block. They then walk back diagonally at full speed. One
  passes the south gantry post at a 5 cm gap.
- **Goal blocks, ticks 1450–2580.** Late walkers must pass parked walkers. In
  the south block, S 3-4 presses against S 4-4, which stands on the approach to
  S 3-4's goal. Both hold 0.60 m apart while S 3-4 creeps at about 2.5 cm/s,
  pushing S 4-4 off its goal. The north block shows the mirror image with
  N 2-1 and N 1-2. Both resolve, and at tick 2584 the south block is complete.
- **End, tick 2700.** All four blocks stand in mirrored formation with no
  motion.

## Findings for Astra

1. **Wall-clock script deadline makes offline `play` outcomes host-dependent.**
   This is the main issue and it blocked captures for a while. The budget is
   50 ms of wall time per tick. It covers parsing the whole project JSON, which
   is 542 KB for this 1,304-entity scene, plus the behavior and serializing
   the result.

   On a quiet host the behavior's own work is about 2 ms with a 3 ms maximum,
   and a whole tick is 12–15 ms. Identical inputs still aborted:
   - after ticks 1702, 2313, 442 and 252 with no sleep assertion, while the
     locked Mac cycled through system sleep. The power log confirms a sleep
     during the tick-252 failure; the others match its timing and long wall
     times;
   - after tick 698 under an idle-sleep assertion, when the Mac entered
     maintenance sleep anyway;
   - after ticks 1542, 1738, 31, 24 and 15 under a system-sleep assertion, with
     no sleep logged, while another worktree's `rustc` build ran (load 9.5).

   The error text does not say which of syntax, exception, memory or deadline
   failed. Possible directions, for your judgment:
   - measure thread CPU time or an instruction budget for offline `play`;
   - report the tick, the elapsed time and the failing limit in the error;
   - avoid re-parsing the whole project each tick.

   The details are in `evidence/deadline-failures.json`.
2. **Endpoint standoffs are real and long (expected per the contract).**
   Reciprocal avoidance plus exact goals lets a parked walker block a late one.
   The worst case took 1002 ticks to clear. The spike doc already says arrival
   is not promised. Consider adding guidance, such as releasing arrived agents
   from avoidance, lowering their responsibility, or reassigning goal slots.
3. **Dense crossings form roundabouts that sweep walkers far off course.** That
   is reasonable local behavior. Games that need goal-directed crowds will need
   path re-planning on top, as the documentation says.
4. **Obstacle IDs must be ULIDs.** `SteeringObstacle.id` is typed as `string`.
   A readable ID such as `obstacle-plinth` fails with "steering requires
   distinct stable ULIDs", which is clear but undocumented for obstacles.
   Consider stating it in the SDK comment and the spike doc.
5. **No engine rendering defects were found.** The early telephotos showed
   streaks and triangles. They came from my own floor geometry: a slab top
   0.5 mm under the tiles. Removing the hidden face fixed it.

## Reproduce

Run from the repository root. The output directories must not exist.

```sh
npm ci
shasum -a 256 artifacts/tools/incant_headless   # expect 2c21c317…a6374e
for rep in A B; do
  caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py \
    artifacts/0027-steering/v4-$rep --ticks 2700 \
    --run logs:none:1 --run overview:overview:25 --run plan:plan:25 \
    --window plinth-a:plinth:300:240:2 --window plinth-b:plinth:540:480:4 \
    --window crossing:crossing:300:240:2 --window gantry:gantry:100:240:2 \
    --window endpoint:endpoint:1000:1600:16
done
T=handoffs/0027-steering-lookdev/tools/trace_check.py
python3 -I $T summary artifacts/0027-steering/v4-A artifacts/0027-steering/v4-A/logs.logs.jsonl \
  --csv per-tick.csv --json summary.json
python3 -I $T compare artifacts/0027-steering/v4-A artifacts/0027-steering/v4-B
python3 -I $T window artifacts/0027-steering/v4-A/logs.logs.jsonl \
  artifacts/0027-steering/v4-A/plinth-a.logs.jsonl   # likewise for each window
```

The helper verifies the binary hash against `binary.json` before and after
every play. It runs strict `tsc` before compiling the behavior, and checks the
frame budget before each capture. A window first plays its prefix with
`--save-output`, then plays the window with `--load-save`.

A failed play is moved aside as a `.failed-N` file and retried up to four
times. Each attempt is recorded. The `caffeinate` prefix only holds a temporary
sleep assertion while the command runs.

### Run results

| Check | Result |
|---|---|
| Binary SHA-256 around all 26 final plays | `2c21c3171a59…a6374e`, unchanged |
| Failed attempts in the final repetitions | 0 |
| Frames per run, all 960 × 540 | 109 for the two full runs, 121 for four windows, 101 for the endpoint window |
| Frames byte-identical between repetitions | **803 of 803** |
| Logs byte-identical between repetitions | **13 of 13** |
| Window and capture logs equal to the uninterrupted log-only run | 12 of 12 in each repetition |
| Trace summary and per-tick CSV, A versus B | identical |
| Project documents, A versus B | identical except IDs generated by `init` and `import` |

## Screenshots

These are 14 unedited frames in `handoffs/0027-steering-lookdev/screenshots/`.
`evidence/screenshots.sha256` maps each one to its source frame in both runs.

| File | Shows |
|---|---|
| `overview-t0000-start.png` | Four blocks, plinth, gantry, goal dots |
| `overview-t0350-first-contact.png` | Blocks meeting around the plinth |
| `overview-t0450-roundabout-peak.png` | Peak density; trails entering |
| `overview-t1200-swept-walkers.png` | Swept walkers stranded in other groups' blocks |
| `overview-t2700-all-arrived.png` | Mirrored formations, everyone at goal |
| `plan-t1200-detour-trails.png` | Counter-clockwise detour trails around the plinth |
| `plan-t1400-south-post-pass.png` | Returning walkers passing the south post |
| `plinth-t0346-first-contact.png` | Telephoto at first contact |
| `plinth-t0476-closest-to-plinth.png` | Telephoto at the 4.9 cm plinth minimum |
| `plinth-t0700-dispersal.png` | Roundabout unwinding; trail curvature |
| `crossing-t0400-roundabout-oblique.png` | The roundabout in 3D |
| `gantry-t0180-under-overhead-beam.png` | Walkers under the height-filtered beam |
| `endpoint-t1800-goal-block-standoff.png` | South goal-block standoff |
| `endpoint-t2584-standoff-resolved.png` | South block complete |

## Changed paths

- `handoffs/0027-steering-lookdev/result.md`
- `handoffs/0027-steering-lookdev/tools/steering_plaza.py` builds the scene and
  runs the reproduction.
- `handoffs/0027-steering-lookdev/tools/steering_plaza.ts` is the behavior.
- `handoffs/0027-steering-lookdev/tools/trace_check.py` does the independent
  checks and comparisons.
- `handoffs/0027-steering-lookdev/tools/tsconfig.json`
- `handoffs/0027-steering-lookdev/screenshots/*.png`, 14 frames.
- `handoffs/0027-steering-lookdev/evidence/` holds the trace summary, the
  per-tick CSV, the A/B comparison, window checks, run records with binary
  hashes, screenshot hashes and the deadline-failure record.

Raw output stays in ignored `artifacts/0027-steering/`. That includes every
frame, every log and all failed attempts.

## Limitations

- **No physics.** Collision, grounding and Rapier behavior were not exercised.
- **One Mac.** Equality holds between repetitions on this one arm64 Mac only.
  Nothing is claimed for other devices or for performance budgets.
- **Frame spacing.** Full runs capture every 25 ticks. The close windows
  capture every 2, 4 or 16 ticks. Single-tick events are judged from the
  per-tick logs, not inferred from unseen frames.
- **Timing probe.** It ran on a separate timed copy of the behavior, with 1 ms
  clock resolution. Its numbers are indicative only.
- **No native capture.** The Mac was locked; headless capture was sufficient
  for this packet.
- **Behavior-owned choices.** The keep-left tangent and the formation-keeping
  goal assignment are mine. They shape the roundabout and the standoffs. They
  are a fair test, not an optimized crowd.

## Open questions

1. Should offline `play` keep a wall-clock deadline at all? If so, should the
   error identify it?
2. Do you want a documented arrival pattern for crowds that settle into packed
   goal sets?
3. Should obstacle IDs accept any unique string, or should the requirement for
   ULIDs be documented?
