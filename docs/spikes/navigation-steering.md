# Bounded local avoidance

`api.steerAgents` proposes horizontal velocities for a snapshot of up to 128
agents. It uses the unmodified pure Rust `dodgy_2d = 0.5.5` ORCA implementation.
It neither moves entities nor writes project data. Games combine it with path
following and apply ordinary Velocity/Transform commands through `incant_cmd`.
Physics remains responsible for actual collision and grounding. This is local
avoidance, not a crowd planner or a promise of arrival in every arrangement.

## Contract

Agents have stable ULIDs, world-space feet positions, XZ current/preferred
velocities, radius, height, maximum speed and a positive avoidance responsibility.
The solver reads all agents at once; output order is by ID and independent of
input ordering. Nonoverlapping vertical intervals do not interact, so separate
floors do not repel each other. Positions are bounded to 10 km, speed to 30 m/s,
radius to 0.05–4 m and height to 0.1–10 m. Responsibility is 0.1–10; a larger
value takes a larger share of avoidance when constraints are feasible.

Optional static obstacles have height intervals and either a strictly convex
closed polygon (3–32 vertices) or a directed two-point segment. Solid lies to
the left of each edge: counterclockwise polygons block their interiors,
clockwise polygons enclose the agent, and open segments are one-sided. At most
32 obstacles and 128 total edges are accepted. Duplicate IDs, nonfinite values,
degenerate/concave/self-crossing polygons and unknown fields fail explicitly.
Obstacle geometry is supplied by the behavior; navmesh boundary extraction and
moving-obstacle prediction are not provided by this API.

The session fixes the time step. Both horizons must be between that step and
10 seconds. `margin` adds 0–1 m to each avoidance radius, defaulting to 5 cm.
It leaves space for approximation and physical corrections. Initial overlap,
infeasible constraints and symmetry can still cause overlap or deadlock; the
returned velocity is a proposal. This increment does not smooth acceleration.

One batch costs 128 of the shared 256 native-query units per tick, accepts at
most 64 KiB of JSON, and checks the script deadline before and after solving.
Raycast, character motion, paths and localization keep the same shared budget.
Failed batches cannot publish a prefix of results or commit script commands,
state, timers or logs. The capability survives hot reload and restored saves.

## Determinism and numerical behavior

Agents and obstacles are sorted by ID. Each solve uses local XZ coordinates.
The backend's zero-length overlap case normally chooses a random normal; the
wrapper detects that singularity and supplies a tiny, antisymmetric pair offset
from stable IDs, then verifies the corrected vector is normalizable. No random
branch is used in those accepted singular cases. Returned values must be finite;
the hard speed cap is reapplied after floating-point linear-program intersections.
The backend's glam version enables libm alongside the navigation library's
separate pinned glam version. Exact cross-device equality is not claimed.

## Evidence

The [evidence record](evidence/navigation-steering-2026-10-10.json) distinguishes
native library checks, public TypeScript gameplay, rendered review and CI.

- Two 100-agent crossing fixtures run for 1500 steps. One starts on a common
  circle, the other on staggered radii. All reach their opposite goals, with
  minimum separations of 0.568 m and 0.591 m for 0.25 m radii in the final local
  library run. ID-order reversals produce equal proposals.
- Other tests cover stacked floors, walls, clockwise enclosures, one-sided
  segments, zero maximum speed, coincident agents at large world coordinates,
  noncoincident overlap singularities, malformed inputs and resource limits.
- Script tests verify atomic rejection, fixed-step binding, movement through
  shared commands, hot reload and saved continuation.
- `tools/probes/game-steering.py` authors 100 walkers through public RPC, compiles
  strict TypeScript and runs 600 ticks. Whole/repeated and 300+300 saved/reopened
  states match exactly; all walkers arrive and minimum separation is 0.600 m.
  This probe has no rendering or Rapier collision claim.
- The six-platform smoke test executes a real 100-agent batch, including
  coincident agents, and checks input-order independence and bounded speeds.
  Hosted results are pending; local macOS execution and WASM/iOS compilation pass.

On this arm64 Mac, 100 sampled batches after 20 warmups measured p95 0.295 ms
for 100 crossing agents, 0.590 ms for 100 coincident agents, and 1.198 ms for the
128-agent/128-obstacle-edge case. These are fixed desktop workloads, not device
or maximum-scene guarantees. Reproduce with
`cargo run -p incant_nav --example steering_budget --release --locked`.

During development, the perfectly converging fixture with only a 1 cm margin
showed a minimum separation of 0.476 m (overlap). The explicit 5 cm default
passes both final fixtures; this does not establish collision-free behavior for
arbitrary crowds. The limitation remains part of the API contract.

Claude's actual rendered review is complete for this increment, as recorded
below. Navigation Inspector, native debug drawing, off-mesh links, grid
navigation, Core Sample integration and physical-device performance
requirements remain separate work.

## Rendered verification and remaining behavior limits

Claude Opus 5.5 reviewed a 100-agent, four-direction plaza crossing using the
actual API and ordinary Velocity commands. Two 2,700-tick runs produced 803/803
identical frames and 13/13 identical logs, including save/load capture windows.
Every walker remained at its destination from tick 2580; no 0.5 m body diameter
overlap occurred. Minimum separation was 0.5953 m with the requested 0.60 m
combined body/margin distance, and minimum obstacle clearance was 0.0494 m.
Millimetre log rounding limits those measurements. The 5 cm extra margin is a
solver input, not a strict numerical clearance guarantee. The review used no
RigidBody/Collider and makes no physics-collision guarantee.

Local avoidance produced long goal-block standoffs (up to 1,002 ticks) and a
roundabout with up to 13.3 m sideways drift. These require behavior-level route
replanning/goal assignment where they are undesirable. Agent and obstacle IDs
are distinct canonical ULIDs within their respective lists.

The existing 50 ms script wall-clock deadline caused intermittent aborts when
the Mac slept or compilation loaded the CPU. Final repetitions passed under
a temporary sleep assertion on a quiet host. This is an open scripting-runtime
robustness/diagnostic issue, not successful determinism evidence for failed runs.
The [review packet](../../handoffs/0027-steering-lookdev/result.md) records the
exact binary, failed attempts, observations and scene reproduction.


## Linux convergence regression (2026-10-10)

PR #36's Linux desktop run at `084065a` retained separation but failed arrival:
one common-radius crowd still had an agent 16.081753 m from its goal after 1500
steps. Eleven other hosted checks passed. Rotating that same fixture by 0.0001
radians reproduced the failure on this Mac (22.316338 m remaining). This is a
local-avoidance symmetry/progress failure, not a compiler failure or a test to
waive. Fixed small lateral preferences of 5% and 20% did not reliably solve it.

The wrapper now predicts closest approach from the requested velocities. If a
neighbor would enter the combined body/margin clearance during the horizon,
the objective turns 45 degrees toward a consistent passing hand without changing
its requested speed. ORCA still constrains this objective against the same agents
and obstacles; no constraint or speed limit is relaxed. Stopped preferences remain
zero. This reduces reciprocal crowd stalls, but is not a guarantee of global
arrival or feasible separation in every arrangement.

All twelve common/staggered-radius cases at rotations 0, 0.0001, 0.03, 0.17, 0.7
and 1.2 radians pass the original 1500-step arrival and separation requirements
locally. The smallest observed separation is 0.5568335 m; every final goal distance
is below 0.00001 m. The earlier rendered results above describe the prior binary.
Full checks, the updated public workflow and a new rendered review are required
for this correction before merging.


The first passing preference failed the unchanged rendered plaza course:
only 26/100 were at their goals after 2700 ticks, compared with all 100 from
tick 2580 before this correction. Even the 4990-tick diagnostic extension left
11 walkers that had never arrived. Repeat frames/logs were identical and there
were no overlaps. PR #36 is held despite all twelve checks passing at `6df4ef9`.
See the current 0027 report and its retained prior-binary history.

The next candidate limits the preference to opposing requested velocities, so
parked and co-directed neighbors do not trigger it. All twelve radial/staggered
library cases and a simple approach beside a parked agent pass locally; the
simple case also passes the prior rule and is not a reproduction of the plaza
regression. The unchanged full plaza course remains the acceptance test, with
updated rendering and integration checks required before merge.


The opposing-only candidate `8d56f0c` also failed the public course: 43/100 at
2700, maximum remaining distance 9.890 m, minimum measured separation
0.59465769 m and zero overlaps. The new fast numerical plaza regression fails
that candidate too (31/100 at tick 2640); its Rust floating-point path is not an
exact replay of the TypeScript host. A fixed 0.0001-scale ID-derived perturbation
was also rejected after it failed a radial crossing case.

Candidate `4c495e9` restricts the shared passing hand to near-antiparallel desired
directions (dot product below -0.99 times their lengths, about eight degrees of
head-on) with a predicted conflict during the horizon. This leaves ordinary
crossing and goal-seeking objectives unchanged. All twelve radial layouts pass
with minimum separation 0.5368895 m and final distances below 0.00001 m. The new
numerical plaza test has all 100 settled throughout its last second, minimum
separation 0.59696561 m and no body/obstacle overlap. Full public playback and
Claude's unchanged rendered course must still verify this candidate before merge.


The unchanged public TypeScript plaza run also passes candidate `4c495e9`: all
100 remain at their goals from tick 2212 through 2700, zero overlap ticks,
minimum separation 0.59603359 m and minimum plinth clearance
0.05000004 m. All 286 Rust tests and Clippy pass. The independent
100-agent public workflow repeats and resumes exactly with unchanged authoring
files. Claude's updated rendered review and final hosted checks remain pending.
