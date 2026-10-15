import type { NativeBehavior, NativeChunk, NativeContext } from '@incant/runtime/native';

type Row = { chunk: NativeChunk; row: number };
function row(context: NativeContext, id: string): Row {
  for (const chunk of context.chunks) {
    const index = chunk.ids.indexOf(id);
    if (index >= 0) return { chunk, row: index };
  }
  throw Error(`Template planar role is missing ${id}`);
}

/** Gravity and tile collision belong to the native planar solver. Gameplay
 * changes horizontal velocity/jump, follows committed player position and
 * starts/stops ordinary native sprite animation. No authoring work per tick. */
export default {
  update(context: NativeContext) {
    if (context.instances.length !== 1) throw Error('Template requires one controller');
    const props = context.instances[0].props;
    if (Object.keys(props).some(key => !['player', 'camera', 'platformer'].includes(key)) ||
        typeof props.player !== 'string' || typeof props.camera !== 'string' ||
        typeof props.platformer !== 'boolean' || props.player === props.camera)
      throw Error('Invalid planar template roles');
    const player = row(context, props.player), camera = row(context, props.camera);
    if (!player.chunk.velocities) throw Error('Template player requires Velocity');
    const p = player.row * player.chunk.transformStride, v = player.row * player.chunk.velocityStride;
    const move = context.input.actions.move?.value ?? [0, 0];
    const scale = 4 / Math.max(1, Math.hypot(move[0], props.platformer ? 0 : move[1]));
    player.chunk.velocities[v] = move[0] * scale;
    if (!props.platformer) player.chunk.velocities[v + 1] = move[1] * scale;
    else if (context.input.actions.jump?.pressed && player.chunk.velocities[v + 1] <= 0) {
      const hit = context.physics.raycast2D({
        origin: [player.chunk.transforms[p], player.chunk.transforms[p + 1]], direction: [0, -1],
        max_distance: 0.52, include_sensors: false, exclude_entity: props.player,
        memberships: 0xffffffff, filter: 0xffffffff,
      });
      if (hit) player.chunk.velocities[v + 1] = 6;
    }
    player.chunk.velocities[v + 2] = 0;
    const c = camera.row * camera.chunk.transformStride;
    camera.chunk.transforms[c] = player.chunk.transforms[p];
    camera.chunk.transforms[c + 1] = player.chunk.transforms[p + 1] + (props.platformer ? 1.5 : 0);
    const index = context.flipbooks.ids.indexOf(props.player);
    if (index < 0) throw Error('Template player requires SpriteFlipbook');
    const moving = Math.abs(move[0]) + (props.platformer ? 0 : Math.abs(move[1])) > 0;
    const playing = (context.flipbooks.flags[index] & 1) !== 0;
    if (moving !== playing) context.commands.controlFlipbook(props.player, moving ? 'play' : 'pause');
  },
} satisfies NativeBehavior;
