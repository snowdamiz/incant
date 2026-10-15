# Simple AI

## Workflow

1. Define patrol, investigate/chase and disengage states, perception ranges and stable target IDs. Author real navigation sources/bounds and validate a committed navmesh or grid before moving agents.
2. Use context.navigation for path requests and supported local steering. Batch agents in one native script system. Replan on meaningful target movement, invalid path or timer expiry, under the native query budget; do not request one full path per entity every tick.
3. Follow waypoints using native collision/movement. Handle unreachable targets and off-mesh transitions explicitly. Local avoidance adjusts preferred velocities but does not replace collision or prove global crowd simulation.
4. Keep state transitions and replanning timers deterministic and persistent where required. Bounded retries, stuck detection and missing/dead-target handling prevent busy loops.

## Verification

Test patrol completion, chase, target loss, blocked/unreachable goals, moving obstacles, tile rebuild and reset. Assert no wall crossing, bounded query count and stable state transitions. Compare large-entity CPU cost separately; a few successful agents do not establish the Core10k script budget.
