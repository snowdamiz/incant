import type { NativeBehavior } from '../../ts/src/native';

/** Named actions and raw devices share one fixed-tick, VM-owned snapshot. */
export default {
  update({ chunks, dt, input }) {
    const move = input.actions.move?.value[0] ?? 0;
    const jump = input.actions.jump?.pressed ? 1 : 0;
    const use = input.actions.use?.pressed ? 2 : 0;
    for (const chunk of chunks) {
      for (let i = 0; i < chunk.ids.length; i++) {
        const offset = i * chunk.transformStride;
        chunk.transforms[offset] += move * dt;
        chunk.transforms[offset + 2] += jump + use;
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
