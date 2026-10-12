import type { NativeNavigation, NativeSteeringAgent, NativeSteeringObstacle } from '../../ts/src/native';

/** Gather one bounded world-space group from the native numeric view, then apply
 * returned XZ proposals using ordinary movement/physics. Entity-local transforms
 * must be converted by the caller before supplying parented positions. */
export function avoid(navigation: NativeNavigation, agents: readonly NativeSteeringAgent[], obstacles: readonly NativeSteeringObstacle[] = []) {
  return navigation.steerAgents({ agents, obstacles, time_horizon: 2, obstacle_time_horizon: 2, margin: 0.05 });
}
