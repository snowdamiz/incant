# Spawners

## Workflow

1. Define finite spawn zones, deterministic sequence/seed, intervals, total and live-count caps. Use fixed tick time or context.timers, not wall-clock time. Validate spawn roles and supported runtime component capabilities before implementing.
2. Prefer a bounded authored pool when spawning needs render, physics or script components that the available structural API cannot create. Use context.commands.spawn only for its actual supported Transform/Velocity contract; it is not a prefab-instantiation API.
3. Allocate stable, globally valid IDs through a declared deterministic scheme and check collisions before structural submission. Batch supported structural changes once per system, respecting the queue limit. If capacity is exhausted, defer predictably without a retry storm.
4. Persist the schedule/index, live ownership and pending timer state using supported native mechanisms. Despawn/reparent children consistently and reconcile counters with accepted structural commits.

## Verification

Run empty/full capacity, blocked zones, repeated activation, pool exhaustion, removal and checkpoint restore. Assert live and total caps, unique IDs, deterministic accepted spawn order and no same-tick invisible counter drift. Compare a replay with the same input/seed; record any intentionally nondeterministic source.
