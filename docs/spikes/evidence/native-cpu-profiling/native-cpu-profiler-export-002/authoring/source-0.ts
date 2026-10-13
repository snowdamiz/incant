import type { NativeBehavior } from '@incant/runtime/native';

// Trusted numerical fixture, authored through command.execute by the probe.
// Replaced independently for the second attached source before authoring.
const speed = 6;
export default {
  update({ chunks, dt, clock, input, log }) {
    const scratch = new Float64Array(2048);
    let sum = 0;
    for (let i = 0; i < scratch.length; i++) {
      scratch[i] = i * 0.5;
      sum += scratch[i];
    }
    if (sum !== 1048064) throw Error('profile numerical fixture');
    for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++) {
      chunk.transforms[i * chunk.transformStride] += speed * dt;
    }
    if (input.keyboard.pressed.includes('F1')) throw Error('cpu-profiler deliberate trap');
    if (clock.tick % 600 === 0) log('PRIVATE-authored-marker:' + speed + ':' + clock.tick);
  },
} satisfies NativeBehavior;
