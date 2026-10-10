> **Historical report (superseded).** This reviewed binary `d4b62efd…cd991` (source `6df4ef9`), now kept in `artifacts/tools/prior-0027-r2/`. It was committed as `2bc125d`. The current review is `../../result.md`; revision 1 is in `../r1-binary-2c21c317/`.

# 0027 local-avoidance rendered-motion review: result (revision 2)

## Status

**The original course now fails on binary `d4b62efd…cd991` (source `6df4ef9`).**
Per the packet, I report the evidence and stop. I did not change the course,
geometry, goals, margins, body size or acceptance criteria.

- **Arrival regressed.** The prior binary had all 100 walkers at goal from
  tick 2580. On the new binary only 26 of 100 are at goal at tick 2700, and 33
  never reach their goal once.
- **It is a livelock, not slow convergence.** An unchanged log-only run to tick
  4990 still ends with 26 at goal, and 11 walkers never arrive at all.
- **Walkers orbit their goals.** In the last 1000 ticks, 34 walkers wind at
  least one full turn around their own goal, all in the same sense. The prior
  binary had none. The sense matches the wrapper's +45° passing preference.
- **The crossing mills for longer.** Walkers keep circling the plinth until
  about tick 2400; the prior binary cleared the centre by tick 1200.
- **Separation still holds.** No tick has a body overlap, through 4990 ticks.
  The minimum centre separation is 0.5955 m by tick 2700 and 0.5924 m by tick
  4990.
- **Runs are clean and repeatable.** Two repetitions gave 803 of 803 identical
  frames and 13 of 13 identical logs. All 26 plays passed on the first attempt.

The sideways sweep shrank from 13.3 m to 5.7 m, which is the one improvement.
Jitter rose: sharp heading changes went from 175 to 282 and mean path length
from 1.20 to 1.63 times the straight distance.

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

### Priority revision 2 acknowledged

The revision asks for a review of the corrected passing preference on the new
immutable binary. It arrived in the session packet; the worktree's `brief.md`
still holds the earlier text, and I did not edit it.

- **Binary.** `artifacts/tools/incant_headless` hashes to
  `d4b62efd369074f130ef059ca535b94827ea7acda15675ece687f4f9a56cd991`, matching
  `binary.json`. Source is `6df4ef9`. The prior binary, `2c21c317…a6374e`, is
  retained in `artifacts/tools/prior-0027/`. I verified the hash before and
  after every play and compiled no Rust.
- **Change reviewed.** I read the wrapper diff from git without checking it
  out. When any neighbour's requested velocity predicts entering the combined
  body-plus-margin clearance within the horizon, the preferred velocity turns
  45° counter-clockwise in (x, z) at unchanged speed.
- **SDK.** Between the two sources the SDK gained only a comment saying agent
  and obstacle IDs must be ULIDs. That resolves my earlier finding. The types
  are unchanged, so compiling against this worktree's SDK is equivalent.
- **History.** The previous report, frames and evidence are preserved unedited
  under `history/r1-binary-2c21c317/`, with a one-line superseded banner.
- **Same course.** The scene, behavior, cameras, 2700-tick course and capture
  windows are byte-identical to the previous review; the tools are unchanged
  since `25db3a8`. The tick-0 frame is identical across binaries. I only added
  metrics to `trace_check.py` and a new `binary_compare.py`, and applied both
  to the prior binary's retained logs as well.
- **Sleep prevention.** Every run of this revision was wrapped in a temporary
  `caffeinate -s -i`. Another process already held a system-sleep assertion
  when I started. No play failed.
- **Earlier failures.** The engine never classified the aborts in my first
  review; it printed only the generic "script failed" message. Their timing
  matched system sleep and CPU load, but that is correlation, not proof of a
  timeout. I make no claim about Astra's failed probe.

## Measured results: same course, both binaries

The prior column comes from the retained repetition-A logs of the previous
review, re-measured with the updated checker. The new column is identical in
repetitions A and B.

| Measure, ticks 1–2700 | Prior binary | New binary |
|---|---|---|
| Ticks with body overlap (centres < 0.50 m) | 0 | **0** |
| Minimum centre separation | 0.5953 m | 0.5955 m (tick 563) |
| Ticks with a pair inside the 0.60 m margin distance | 2262 | 2457 |
| Minimum plinth gap | 0.0494 m | 0.0494 m (tick 788) |
| Minimum post gap | 0.0499 m | 0.0496 m (tick 799) |
| Maximum displacement speed, from mm positions | 1.476 m/s | 1.465 m/s |
| All 100 at goal | from tick 2580 | **never** |
| At goal at tick 2700 | 100 | **26** |
| Walkers that never reach their goal | 0 | **33** |
| Last first arrival | tick 2128 | tick 2699 |
| Walkers orbiting their goal, last 1000 ticks | 0 | **34**, all one sense |
| Walkers within 3 m of the plinth at tick 1200 | 0 | 37 |
| Walkers with a stall over 60 ticks | 17 | 19 |
| Longest stall | 1002 ticks | 428 ticks |
| Largest sideways sweep | 13.3 m | 5.7 m |
| Sharp heading changes over 45°, per 0.1 s | 175, in 71 walkers | **282, in 76 walkers** |
| Reversals over 135° | 44 | **61** |
| Path length over straight distance, mean / max | 1.20 / 1.81 | **1.63 / 2.53** |

Motion first differs between the binaries at tick 155, as the blocks close in.

The arrived count rises and falls on the new binary: 18 at tick 1900, 12 at
tick 2500, 26 at tick 2700. Walkers that had arrived are pushed off again.
At tick 2700, 42 walkers are short of their goal but within 1.5 m of it, and
32 are 1.5–5 m away. The unarrived walkers still move at a mean 0.65 m/s.

### Unchanged course run to tick 4990

I ran the identical course, log-only, for 4990 ticks, the most the log budget
allows. This only observes longer; acceptance stays at 2700 ticks. Its first
2700 ticks are byte-identical to repetition A.

| Measure, ticks 1–4990 | Result |
|---|---|
| Body overlap ticks | 0 |
| Minimum separation | 0.5924 m (tick 4361) |
| At goal, by tick | 2900: 23 · 3700: 29 · 3900: 15 · 4500: 30 · 4990: **26** |
| Walkers that never arrive | **11** |
| Walkers orbiting their goal, last 1000 ticks | 28, all one sense |
| Walkers with a stall over 60 ticks | 46 |
| Path ratio, mean / max | 2.39 / 3.54 |

## Observed motion

These observations come from inspecting real frames. Every committed frame is
an unedited copy of engine output, identical in both repetitions.

- **Approach, ticks 0–340.** The motion matches the prior binary until tick
  155. The beam still never walls off the lane: 49 walkers cross under it at
  a mean 1.36 m/s. The slowest crossing is 0.63 m/s, versus 1.05 m/s on the
  prior binary, which I attribute to crowding nearby rather than the beam.
- **Crossing, ticks 340–700.** The groups no longer move as coherent blocks.
  They interleave in mixed-colour rings spiralling around the plinth. The
  telephoto frames still show distinct footprints with clear gaps. At tick 476
  the plinth edge keeps its margin.
- **Central mill, ticks 700–2400.** At tick 1200, about 30 walkers still circle
  tightly around the plinth, where the prior binary had a clear centre. Thirteen
  remain at tick 2000. The mill is gone by tick 2400.
- **Goal blocks, ticks 1500–2700.** No block forms its grid. In the south block
  the goal dots stay uncovered while walkers mill around them. A tracked
  walker's trail closes into a tight loop near its goal. Walkers that arrived
  get pushed off as others circle through.
- **End, tick 2700.** All four groups sit roughly at their destinations,
  disordered and still moving. The southbound group spills past the north edge
  of its zone. The prior binary showed four complete mirrored grids here.

## Findings for Astra

1. **Regression: the passing preference causes goal-block livelock on this
   course.** The evidence is listed above and in `evidence/prior-vs-new.json`.

   My hypothesis, from reading the code, is not a measured cause. The trigger
   predicts closest approach from the requested velocity over the full horizon.
   This behavior, like `tools/probes/game-steering.py`, requests full speed until
   one tick from the goal. So a walker 6 cm from its goal, facing a parked
   neighbour 0.9 m away, is treated as if it will walk 2.8 m toward it. The
   trigger fires and turns it 45°. Avoidance then blocks the inward part,
   leaving a steady tangential drift, which is the observed orbit. The same
   hand for every walker turns local milling into consistent rotation.

   Directions to consider, for your judgment:
   - stop the prediction at the requested goal distance, which needs the goal
     or a stopping distance in the query;
   - do not trigger on neighbours whose requested velocity is zero;
   - fade the rotation as the requested speed or the remaining distance falls.

   Your twelve fixtures place goals in the open, not in a packed 0.9 m grid
   beside parked agents, which may be why they pass.
2. **The crossing improved in drift but worsened in jitter.** Sideways sweep
   fell from 13.3 m to 5.7 m. Sharp turns, reversals and path length all rose,
   and a mill persisted around the obstacle for about 1,200 extra ticks.
3. **Separation and obstacle margins are unchanged.** No overlap occurred in
   7,690 measured ticks across both runs. The minima stay within a few
   millimetres of the requested distances.
4. **Run robustness was clean this time.** All 27 plays in this revision
   passed on the first attempt, under temporary sleep prevention on a quiet
   host. That does not settle the earlier unclassified script failures.

## Reproduce

Run from the repository root. The output directories must not exist.

```sh
npm ci                                            # if node_modules is absent
shasum -a 256 artifacts/tools/incant_headless     # expect d4b62efd…f9a56cd991
for rep in A B; do
  caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py \
    artifacts/0027-steering/rev2-$rep --ticks 2700 \
    --run logs:none:1 --run overview:overview:25 --run plan:plan:25 \
    --window plinth-a:plinth:300:240:2 --window plinth-b:plinth:540:480:4 \
    --window crossing:crossing:300:240:2 --window gantry:gantry:100:240:2 \
    --window endpoint:endpoint:1000:1600:16
done
caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py \
  artifacts/0027-steering/rev2-long --ticks 4990 --run logs:none:1
T=handoffs/0027-steering-lookdev/tools
python3 -I $T/trace_check.py summary artifacts/0027-steering/rev2-A \
  artifacts/0027-steering/rev2-A/logs.logs.jsonl --csv per-tick.csv --json summary.json
python3 -I $T/trace_check.py compare artifacts/0027-steering/rev2-A artifacts/0027-steering/rev2-B
python3 -I $T/trace_check.py window artifacts/0027-steering/rev2-A/logs.logs.jsonl \
  artifacts/0027-steering/rev2-A/endpoint.logs.jsonl     # likewise for each window
# PRIOR_RUN: a log-only run of the same course on the prior binary. The committed
# helper always uses artifacts/tools/incant_headless, so the prior column came
# from the retained previous-review output artifacts/0027-steering/v4-A.
python3 -I $T/binary_compare.py PRIOR_RUN artifacts/0027-steering/rev2-A \
  --long artifacts/0027-steering/rev2-long
```

### Run results

| Check | Result |
|---|---|
| Binary SHA-256 around all 27 plays | `d4b62efd…cd991`, unchanged |
| Failed attempts | 0 |
| Frames byte-identical between repetitions | **803 of 803** |
| Logs byte-identical between repetitions | **13 of 13** |
| Window and capture logs equal to the uninterrupted run | 12 of 12 in each repetition |
| Trace summary and per-tick CSV, A versus B | identical |
| Long run's first 2700 ticks versus repetition A | identical |

## Screenshots

These are 14 unedited frames in `screenshots/`, identical in both repetitions.
`evidence/screenshots.sha256` maps each one to its source frame. The prior
binary's frames at the same ticks are in `history/r1-binary-2c21c317/screenshots/`.

| File | Shows |
|---|---|
| `overview-t0000-start.png` | Start; identical to the prior binary |
| `overview-t0450-interleaved-rings.png` | Groups interleaving around the plinth |
| `overview-t1200-plinth-mill-persists.png` | Central mill still present |
| `overview-t2700-goal-blocks-unformed.png` | No group has formed its grid |
| `plan-t1200-plinth-mill.png` | Top-down view of the mill and trails |
| `plan-t2700-final-state.png` | Top-down view of the unformed blocks |
| `plinth-t0346-first-contact.png` | Telephoto at first contact |
| `plinth-t0476-closest-to-plinth.png` | Mixed-colour rings; plinth margin held |
| `plinth-t0700-mill.png` | Mill continuing past the prior dispersal |
| `crossing-t0400-interleaved-oblique.png` | Interleaved crossing in 3D |
| `gantry-t0180-under-overhead-beam.png` | Walkers under the height-filtered beam |
| `endpoint-t1800-goal-orbits.png` | South goal block milling |
| `endpoint-t2200-orbit-trail-loop.png` | A tracked walker's trail looping near its goal |
| `endpoint-t2600-block-unformed.png` | South block still unformed at the end of the window |

## Changed paths

- `result.md` is this revision-2 report.
- `history/r1-binary-2c21c317/` holds the previous `result.md`, `screenshots/`
  and `evidence/`, moved unedited apart from a superseded banner.
- `screenshots/` holds the 14 new frames.
- `evidence/` holds the trace summary, per-tick CSV, A/B comparison, window
  checks, run records with binary hashes, screenshot hashes, the prior-binary
  summary, the prior-versus-new comparison and the 4990-tick summary.
- `tools/trace_check.py` gained motion metrics: sharp turns, reversals, path
  ratio and settling time.
- `tools/binary_compare.py` is new; it compares arrival, plinth mill, goal
  orbits and divergence between two runs.

All paths are under `handoffs/0027-steering-lookdev/`. Raw output stays in
ignored `artifacts/0027-steering/` (`rev2-A`, `rev2-B`, `rev2-long`).

## Limitations

- **No physics.** Collision and grounding were not exercised.
- **One Mac.** Equality holds between repetitions on this arm64 Mac only.
- **Hypothesis only.** The cause in finding 1 comes from reading the code. I
  ran no modified course or behavior to test it, as the packet requires.
- **Mixed tool versions in the comparison.** The prior column re-measures logs
  from the previous review with the updated checker. Those logs came from the
  same course, built by the unchanged helper on the prior binary.
- **Frame spacing.** Full runs capture every 25 ticks; windows every 2, 4 or 16.
  Single-tick events are judged from per-tick logs, not inferred from frames.

## Open questions

1. Should the passing-preference trigger account for goal distance or stopped
   neighbours, or should the documentation require behaviors to slow
   preferred velocity near goals?
2. After a fix, should this unchanged course be re-run as the rendered
   regression check?
