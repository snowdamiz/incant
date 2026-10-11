import type { NativeBehavior } from '../../ts/src/native';

// Data events run in the same synchronous update and transaction as movement.
// A local variable is VM state, not a persisted save or hot-reload snapshot.
let ready = true;
export default {
  update({ input, timers, chunks }) {
    for (const event of timers.events) {
      if (event.id === 'dash-ready') ready = true;
    }
    if (ready && input.actions.dash?.pressed) {
      for (const chunk of chunks) {
        if (!chunk.velocities) continue;
        for (let row = 0; row < chunk.ids.length; row++) {
          chunk.velocities[row * chunk.velocityStride] = 8;
        }
      }
      ready = false;
      timers.set('dash-ready', 60);
    }
    return undefined;
  },
} satisfies NativeBehavior;
