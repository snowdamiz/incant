import type { NativeId, NativePhysics, NativeRayHit } from '../../ts/src/native';

/** Query selected actors as needed; bulk columns remain the default for motion.
 * Results reflect committed physics, before pending numeric/structural writes. */
export function groundBelow(
  physics: NativePhysics,
  origin: [number, number, number],
  excludeEntity: NativeId | null,
): NativeRayHit | null {
  return physics.raycast({
    origin, direction: [0, -1, 0], max_distance: 100,
    include_sensors: false, exclude_entity: excludeEntity,
    memberships: 0xffffffff, filter: 0xffffffff,
  });
}
