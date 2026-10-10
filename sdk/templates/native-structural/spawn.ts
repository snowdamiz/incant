import type { NativeBehavior } from '../../ts/src/native';

// First native migration example. Feed a cooked scene to the native_bulk runner;
// this is not a behavior for the legacy PlaySession API.
const spawned = '00000000000000000000000003';
let tick = 0;
export default {
  update({ chunks, commands }) {
    if (tick === 0) {
      const parent = chunks[0]?.ids[0] ?? null;
      commands.spawn(spawned, [1, 0, 0, 0, 0, 0, 1, 1, 1, 1], parent, null);
      commands.setVelocity(spawned, [2, 0, 0]);
    } else if (tick === 2) {
      commands.setParent(spawned, null);
      commands.setVelocity(spawned, null);
    } else if (tick === 3) {
      commands.despawn(spawned);
    }
    for (const chunk of chunks) {
      for (let i = 0; i < chunk.ids.length; i++) chunk.transforms[i * chunk.transformStride + 1] += 1;
    }
    tick++;
    return undefined;
  },
} satisfies NativeBehavior;
