import type { NativeBehavior } from '@incant/runtime/native';

export default {
  update({ chunks, dt, input, clock }) {
    const right = input.actions.right;
    const dx = right?.active ? 6 * dt : 0;
    const edge = right?.pressed ? 1 : 0;
    for (const chunk of chunks) {
      for (let row = 0; row < chunk.ids.length; row++) {
        const offset = row * chunk.transformStride;
        chunk.transforms[offset] += dx;
        chunk.transforms[offset + 1] = clock.tick;
        chunk.transforms[offset + 2] += edge;
      }
    }
  },
} satisfies NativeBehavior;
