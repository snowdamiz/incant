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

Claude's actual rendered review is the next scoped check. Navigation Inspector,
native debug drawing, off-mesh links, grid navigation, Core Sample integration
and physical-device performance requirements remain open.
