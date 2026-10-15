import type { NativeBehavior, NativeChunk, NativeContext } from '@incant/runtime/native';

type Row = { chunk: NativeChunk; row: number };
type State = { yaw: number; pitch: number; vertical: number; grounded: boolean };
function row(context: NativeContext, id: string): Row {
  for (const chunk of context.chunks) {
    const index = chunk.ids.indexOf(id);
    if (index >= 0) return { chunk, row: index };
  }
  throw Error(`Template character is missing ${id}`);
}

/** Authored WASD/left-stick movement, Space jump and arrow/right-stick look.
 * Native character queries test committed collision; ordinary bulk velocity
 * writes move the kinematic body on the following fixed simulation step.
 * This source has no project/file/OS authority and needs no editor at runtime. */
export default {
  persistence: { version: 1 },
  update(context: NativeContext) {
    if (context.instances.length !== 1) throw Error('Template requires one controller');
    const props = context.instances[0].props;
    if (Object.keys(props).some(key => !['player', 'camera', 'firstPerson'].includes(key)) ||
        typeof props.player !== 'string' || typeof props.camera !== 'string' ||
        typeof props.firstPerson !== 'boolean' || props.player === props.camera)
      throw Error('Invalid template character roles');
    const player = row(context, props.player), camera = row(context, props.camera);
    const previous = context.state.get();
    const state = previous === null ? { yaw: 0, pitch: props.firstPerson ? 0 : -0.2, vertical: 0, grounded: false }
      : { ...(previous as unknown as State) };
    if (![state.yaw, state.pitch, state.vertical].every(Number.isFinite) || typeof state.grounded !== 'boolean')
      throw Error('Invalid template character state');
    const look = context.input.actions.look?.value ?? [0, 0];
    state.yaw = (state.yaw - look[0] * context.dt * 1.8) % (Math.PI * 2);
    state.pitch = Math.max(-1.15, Math.min(1.15, state.pitch + look[1] * context.dt * 1.4));
    const move = context.input.actions.move?.value ?? [0, 0];
    const speed = 4 * context.dt / Math.max(1, Math.hypot(move[0], move[1]));
    if (context.input.actions.jump?.pressed && state.grounded) state.vertical = 6;
    state.vertical = Math.max(-20, state.vertical - 18 * context.dt);
    const cos = Math.cos(state.yaw), sin = Math.sin(state.yaw);
    const wanted: [number, number, number] = [
      (move[0] * cos + move[1] * sin) * speed, state.vertical * context.dt,
      (-move[0] * sin + move[1] * cos) * speed,
    ];
    const motion = context.physics.computeCharacterMotion({ entity_id: props.player, translation: wanted });
    if (!player.chunk.velocities) throw Error('Template character requires Velocity');
    const velocity = player.row * player.chunk.velocityStride;
    for (let axis = 0; axis < 3; axis++) player.chunk.velocities[velocity + axis] = motion.translation[axis] / context.dt;
    state.grounded = motion.grounded;
    if ((motion.grounded && state.vertical < 0) || (state.vertical > 0 && motion.translation[1] < wanted[1] - 1e-6))
      state.vertical = 0;
    const p = player.row * player.chunk.transformStride, c = camera.row * camera.chunk.transformStride;
    const distance = props.firstPerson ? 0 : 6;
    camera.chunk.transforms[c] = player.chunk.transforms[p] + sin * Math.cos(state.pitch) * distance;
    camera.chunk.transforms[c + 1] = player.chunk.transforms[p + 1] + 0.55 - Math.sin(state.pitch) * distance;
    camera.chunk.transforms[c + 2] = player.chunk.transforms[p + 2] + cos * Math.cos(state.pitch) * distance;
    // Yaw then local pitch. Camera looks along local -Z, matching the renderer.
    const cy = Math.cos(state.yaw / 2), sy = Math.sin(state.yaw / 2);
    const cp = Math.cos(state.pitch / 2), sp = Math.sin(state.pitch / 2);
    camera.chunk.transforms.set([cy * sp, sy * cp, -sy * sp, cy * cp], c + 3);
    context.state.set(state);
  },
} satisfies NativeBehavior;
