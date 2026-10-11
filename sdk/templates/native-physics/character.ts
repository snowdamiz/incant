import type { NativeContext, NativeId, NativeCharacterMovement } from '../../ts/src/native';

/** Compute one fixed-tick velocity against committed colliders. NativeSession
 * applies this velocity in its next simulation step. Gravity/jumps are supplied
 * by gameplay in desiredDisplacement; there is no hidden second physics step. */
export function driveCharacter(
  context: NativeContext,
  entity: NativeId,
  desiredDisplacement: [number, number, number],
): NativeCharacterMovement {
  const motion = context.physics.computeCharacterMotion({
    entity_id: entity,
    translation: desiredDisplacement,
  });
  context.commands.setVelocity(entity, [
    motion.translation[0] / context.dt,
    motion.translation[1] / context.dt,
    motion.translation[2] / context.dt,
  ]);
  return motion;
}
