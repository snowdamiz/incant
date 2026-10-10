# 0027 local-avoidance rendered-motion review: result (revision 3)

## Status

**The near-head-on correction passes the unchanged course.** I reviewed binary
`29ecd16b…8d85f461` (source `4c495e9`).

- **Arrival.** All 100 walkers are at their goals from tick 2212 and stay
  there through tick 2700. A log-only run of the same course to tick 4990 keeps
  all 100 at goal. Every position is frozen to the millimetre from tick 2225.
- **Separation.** No tick has a body overlap. The minimum centre separation is
  0.596 m, and the minimum plinth gap is 4.94 cm.
- **Repeatability.** Two repetitions gave 803 of 803 identical frames and 13 of
  13 identical logs. Every save/load window matched the uninterrupted run. All
  27 plays passed on the first attempt.
- **Revision-2 failures are gone.** No walker orbits its goal, the central mill
  clears on the original schedule, and the longest stall fell to 127 ticks from
  the original 1002.

Two visible issues remain:

1. **Long sweeps.** The roundabout still carries walkers up to 12.8 m sideways
   into other groups' blocks, as the original binary did.
2. **More jostling at goals.** Sharp heading changes rose to 250 from the
   original 175. 240 of them happen within 2 m of a goal, as walkers jostle
   into packed slots, and they stop once the blocks settle.

This result does not approve a phase gate or claim the wider phase complete. It
makes no physics-collision claim, because walkers have no RigidBody or Collider.
It certifies no cross-device determinism or platform budget.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP on the host's evidence. `tools/acp/config.json` pins
`claude-opus-5-5` and the `claude-agent-acp` adapter. This session runs from the
repository's pinned agent SDK binary through the TypeScript SDK entrypoint,
consistent with `tools/handoff/main.py`. From inside the session I cannot
inspect the ACP messages themselves.

### Priority revision 3 acknowledged

The revision asks me to verify the near-head-on correction on an immutable
binary, with the unchanged course, after the unchanged public TypeScript run
has passed.

- **Precondition.** Astra's commit `9a4d99c` records that run passing, with all
  100 at goal from tick 2212 through 2700. My own log-only run of the same
  course reproduces it independently: tick 2212, minimum separation 0.596 m.
- **Binary.** `artifacts/tools/incant_headless` hashes to
  `29ecd16b38a020670b13b49bb26a653b0d5f95d0f78273477bd456488d85f461`, matching
  `binary.json`. Source is `4c495e9`. I verified the hash before and after every
  play and compiled no Rust. The two earlier binaries remain in
  `artifacts/tools/prior-0027/` and `artifacts/tools/prior-0027-r2/`.
- **Change reviewed.** I read the wrapper diff from git without checking it
  out. The 45° passing hand now applies only when two requested directions are
  within about 8° of head-on (dot product below −0.99 times their lengths) and
  their paths predict a conflict. Neighbours with zero requested speed never
  trigger it. Constraints and speed caps are unchanged. The SDK is unchanged.
- **History.** The revision-2 report, frames and evidence are preserved under
  `history/r2-binary-d4b62efd/`, with a one-line superseded banner.
  `history/r1-binary-2c21c317/` is untouched.
- **Same course.** The scene, behavior, goals, cameras, margins, 2700-tick
  course and capture windows are unchanged; `tools/` is byte-identical to
  `2bc125d`. No threshold was changed. The tick-0 frame is identical across all
  three binaries.
- **Sleep prevention.** Every run was wrapped in a temporary
  `caffeinate -s -i`. No play failed, so there is no error to classify.

## Measured results: same course, three binaries

The columns hold the original binary (`20ca453`), the revision-2 binary
(`6df4ef9`) and this binary (`4c495e9`). The first two columns re-measure the
retained logs from earlier reviews with the same checker. The new column is
identical in repetitions A and B.

| Measure, ticks 1–2700 | Original | Revision 2 | **Revision 3** |
|---|---|---|---|
| Ticks with body overlap (centres < 0.50 m) | 0 | 0 | **0** |
| Minimum centre separation | 0.5953 m | 0.5955 m | **0.5960 m** |
| Ticks with a pair inside the 0.60 m margin distance | 2262 | 2457 | 1804 |
| Minimum plinth / post gap | 0.0494 / 0.0499 m | 0.0494 / 0.0496 m | 0.0494 / 0.0500 m |
| All 100 at goal | from tick 2580 | never | **from tick 2212** |
| At goal at tick 2700 | 100 | 26 | **100** |
| Walkers orbiting their goal, last 1000 ticks | 0 | 34 | **0** |
| Walkers within 3 m of the plinth at tick 1200 | 0 | 37 | 0 |
| Walkers with a stall over 60 ticks | 17 | 19 | **4** |
| Longest stall | 1002 ticks | 428 ticks | **127 ticks** |
| Largest sideways sweep | 13.3 m | 5.7 m | 12.8 m |
| Sharp heading changes over 45°, per 0.1 s | 175, in 71 walkers | 282, in 76 | 250, in 88 |
| Sharp changes within 2 m of the goal | 164 | not measured | 240 |
| Reversals over 135° | 44 | 61 | 53 |
| Path length over straight distance, mean / max | 1.20 / 1.81 | 1.63 / 2.53 | 1.22 / 1.84 |
| Mean time from first within 1 m of goal to final arrival | 533 ticks | 784 ticks | 466 ticks |

Motion first differs from the original binary at tick 241, as the blocks
close in.

Every tick from 2212 to 2700 has all 100 walkers within 5 cm of their goals.
After tick 2223 no walker moves faster than 0.1 m/s.

The trace check matches the behavior's own row on separation and plinth gap on
every tick. The arrived count differs by one on 25 ticks, where millimetre
rounding flips walkers at the 5 cm boundary.

### Unchanged course run to tick 4990

This log-only run only observes longer; acceptance stays at 2700 ticks. Its
first 2700 ticks are byte-identical to repetition A.

| Measure, ticks 1–4990 | Result |
|---|---|
| Body overlap ticks | 0 |
| Minimum separation | 0.596 m (tick 1230) |
| All 100 at goal | from tick 2212 to tick 4990, every tick |
| Largest goal offset after tick 2600 | 0.000 m |
| Positions frozen | from tick 2225 |
| Walkers orbiting their goal, last 1000 ticks | 0 |

### Height filter

49 walkers cross under the overhead beam at 1.05 m/s or faster, with a mean of
1.37 m/s. That matches the original binary.

## Observed motion

These observations come from inspecting real frames. Every committed frame is
an unedited copy of engine output, identical in both repetitions.

- **Approach, ticks 0–340.** The blocks march in formation and pass under the
  beam undeflected, as before.
- **Crossing, ticks 340–700.** The groups again move as coherent blocks around
  the plinth, not the interleaved rings of revision 2. At tick 476 the
  telephoto shows distinct footprints with clear gaps, and the plinth edge
  keeps its margin.
- **Dispersal and sweep, ticks 700–1500.** The centre clears by tick 1200. Some
  walkers are still carried with the wrong stream: blue walkers stand in the
  south block, and a few walkers of other groups head home on long diagonal
  paths.
- **Goal blocks, ticks 1400–2212.** Late walkers work into their slots with
  short jostles and no prolonged standoff. At tick 1480 three late walkers
  thread into the south block while an amber walker passes outside it. The
  south block is complete by tick 2216.
- **End, tick 2700.** All four groups stand in complete mirrored grids, as on
  the original binary.

## Findings for Astra

1. **The correction passes and fixes both regressions.** Arrival is earlier
   than on the original binary, at tick 2212 against 2580. Goal retention holds
   to tick 4990. Standoffs are much shorter, at 127 ticks against 1002.
   Separation and obstacle margins are unchanged.
2. **Long sweeps remain a visible quality issue.** Up to 12.8 m of sideways
   carry is close to the original 13.3 m. This is local-avoidance behavior that
   the documentation already assigns to behavior-level route replanning; it is
   not a regression.
3. **Goal-block jostling rose but ends.** Sharp heading changes within 2 m of a
   goal rose from 164 to 240, and 88 walkers turned sharply at least once,
   against 71. In the frames this reads as brief shuffling while slots fill. All
   motion stops by tick 2225. I found no persistent jitter.
4. **No new failures.** No overlap, orbit, failed run or retention loss
   occurred in 7,690 measured ticks.

## Reproduce

Run from the repository root. The output directories must not exist.

```sh
npm ci                                            # if node_modules is absent
shasum -a 256 artifacts/tools/incant_headless     # expect 29ecd16b…8d85f461
for rep in A B; do
  caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py \
    artifacts/0027-steering/rev3-$rep --ticks 2700 \
    --run logs:none:1 --run overview:overview:25 --run plan:plan:25 \
    --window plinth-a:plinth:300:240:2 --window plinth-b:plinth:540:480:4 \
    --window crossing:crossing:300:240:2 --window gantry:gantry:100:240:2 \
    --window endpoint:endpoint:1000:1600:16
done
caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py \
  artifacts/0027-steering/rev3-long --ticks 4990 --run logs:none:1
T=handoffs/0027-steering-lookdev/tools
python3 -I $T/trace_check.py summary artifacts/0027-steering/rev3-A \
  artifacts/0027-steering/rev3-A/logs.logs.jsonl --csv per-tick.csv --json summary.json
python3 -I $T/trace_check.py compare artifacts/0027-steering/rev3-A artifacts/0027-steering/rev3-B
python3 -I $T/trace_check.py window artifacts/0027-steering/rev3-A/logs.logs.jsonl \
  artifacts/0027-steering/rev3-A/endpoint.logs.jsonl     # likewise for each window
# PRIOR_RUN: a log-only run of this course on an earlier binary. The helper always
# uses artifacts/tools/incant_headless; the comparisons used the retained earlier
# outputs artifacts/0027-steering/v4-A (original) and rev2-A (revision 2).
python3 -I $T/binary_compare.py PRIOR_RUN artifacts/0027-steering/rev3-A \
  --long artifacts/0027-steering/rev3-long
```

### Run results

| Check | Result |
|---|---|
| Binary SHA-256 around all 27 plays | `29ecd16b…8d85f461`, unchanged |
| Failed attempts | 0 |
| Frames byte-identical between repetitions | **803 of 803** |
| Logs byte-identical between repetitions | **13 of 13** |
| Window and capture logs equal to the uninterrupted run | 12 of 12 in each repetition |
| Trace summary and per-tick CSV, A versus B | identical |
| Long run's first 2700 ticks versus repetition A | identical |

## Screenshots

These are 14 unedited frames in `screenshots/`, identical in both repetitions.
`evidence/screenshots.sha256` maps each one to its source frame. The earlier
binaries' frames at the same ticks are in the two history folders.

| File | Shows |
|---|---|
| `overview-t0000-start.png` | Start; identical across all three binaries |
| `overview-t0450-roundabout-peak.png` | Block-wise roundabout at peak density |
| `overview-t1200-swept-walkers.png` | Clear centre; swept walkers in other blocks |
| `overview-t2700-all-at-goal.png` | Four complete mirrored grids |
| `plan-t1200-detour-trails.png` | Detour trails around the plinth, top-down |
| `plan-t2700-final-state.png` | Final grids, top-down |
| `plinth-t0346-first-contact.png` | Telephoto at first contact |
| `plinth-t0476-closest-to-plinth.png` | Telephoto at peak; plinth margin held |
| `plinth-t0700-dispersal.png` | Roundabout unwinding |
| `crossing-t0400-roundabout-oblique.png` | The roundabout in 3D |
| `gantry-t0180-under-overhead-beam.png` | Walkers under the height-filtered beam |
| `endpoint-t1480-late-walkers-entering.png` | Late walkers threading into the south block |
| `endpoint-t1800-goal-block-settling.png` | South block nearly filled |
| `endpoint-t2216-block-complete.png` | South block complete |

## Changed paths

- `result.md` is this revision-3 report.
- `history/r2-binary-d4b62efd/` holds the revision-2 `result.md`, `screenshots/`
  and `evidence/`, moved unedited apart from a superseded banner.
- `screenshots/` holds the 14 new frames.
- `evidence/` holds the trace summary, per-tick CSV, A/B comparison, window
  checks, run records with binary hashes, screenshot hashes, comparisons
  against both earlier binaries and the 4990-tick summary.

All paths are under `handoffs/0027-steering-lookdev/`. `history/r1-binary-2c21c317/`
and `tools/` are unchanged. Raw output stays in ignored `artifacts/0027-steering/`
(`rev3-A`, `rev3-B`, `rev3-long`).

## Limitations

- **No physics.** Collision and grounding were not exercised.
- **One Mac.** Equality holds between repetitions on this arm64 Mac only.
- **Sharp-turn measure.** It uses 0.1 s windows of millimetre-rounded positions;
  it counts direction changes, not their visual size.
- **Frame spacing.** Full runs capture every 25 ticks; windows every 2, 4 or 16.
  Single-tick events are judged from per-tick logs, not inferred from frames.
- **One course.** It exercises one dense four-way crossing with packed goal
  grids. It does not cover other densities, layouts or obstacle shapes.

## Open questions

1. Is the remaining 12.8 m sweep acceptable for this increment, given that
   route replanning is documented as behavior-owned?
2. Should the plaza course stay as the rendered regression check for future
   steering changes?
