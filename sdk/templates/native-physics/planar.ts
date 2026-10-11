import type { NativeId, NativePhysics, NativeRayHit2D } from '../../ts/src/native';

/** Ground probe for an existing planar player. Movement remains a bulk write. */
export function groundBelow(physics: NativePhysics, player: NativeId, x: number, y: number): NativeRayHit2D | null {
  return physics.raycast2D({
    origin: [x, y], direction: [0, -1], max_distance: 0.25,
    include_sensors: false, exclude_entity: player,
    memberships: 0xffffffff, filter: 0xffffffff,
  });
}

/** Sensor events are owned, sorted and repeated reads do not drain them. */
export function enteredPickup(physics: NativePhysics, player: NativeId, pickup: NativeId): boolean {
  return physics.triggerEvents2D().some(event => event.entered &&
    ((event.first === player && event.second === pickup) ||
     (event.first === pickup && event.second === player)));
}
