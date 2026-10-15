import type { NativeBehavior } from '../../ts/src/native';

// Explicit continuation state is saved even while this task is waiting. Load
// starts the named resume function with that state, without replaying the pulse.
export default {
  persistence: { version: 1 },
  coroutines: {
    pulse: {
      resume({ chunks, clock, timers, log }, task) {
        const phase = task.state;
        if (typeof phase !== 'number') throw Error('invalid pulse continuation');
        for (const chunk of chunks) {
          for (let row = 0; row < chunk.ids.length; row++) {
            if (task.entity === undefined || chunk.ids[row] === task.entity) {
              chunk.transforms[row * 10] += phase + 1;
            }
          }
        }
        log(`pulse:${task.id}:${phase}:${clock.tick}`);
        if (phase === 0) {
          timers.set(`pulse.${task.id}`, 1);
          return { kind: 'wait', delayTicks: 2, state: 1 };
        }
        return { kind: 'done' };
      },
    },
  },
  update({ clock, coroutines, instances }) {
    if (clock.tick === 1) {
      coroutines.start('main', 'pulse', 0, instances[0]?.entity);
    }
    return undefined;
  },
} satisfies NativeBehavior;
