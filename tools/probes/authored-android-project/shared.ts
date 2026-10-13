import type { NativeBehavior } from '@incant/runtime/native';

function bits(value: number): string {
  const bytes = new ArrayBuffer(8), view = new DataView(bytes);
  view.setFloat64(0, value, true);
  return view.getUint32(4, true).toString(16).padStart(8, '0') +
    view.getUint32(0, true).toString(16).padStart(8, '0');
}

// Two independently attached entry modules share this source dependency. The
// exact source catalog, properties and scene hash must survive the Android cook.
export function behavior(direction: number): NativeBehavior {
  return { update({ instances, chunks, input, clock, dt, log }) {
    for (const compile of [() => eval('1'), () => Function('return 1')]) {
      let denied = false;
      try { compile(); } catch { denied = true; }
      if (!denied) throw Error('authored Android source compiler enabled');
    }
    for (const instance of instances) {
      const { speed, label } = instance.props;
      if (typeof speed !== 'number' || typeof label !== 'string') throw Error('fixture props');
      for (const chunk of chunks) for (let row = 0; row < chunk.ids.length; row++) {
        if (chunk.ids[row] !== instance.entity || !chunk.velocities) continue;
        const velocity = input.actions.right?.active ? direction * speed : 0;
        log([clock.tick, label, bits(chunk.transforms[row * chunk.transformStride]),
          bits(velocity), bits(dt), String((1n << 64n) + 1n), bits(Math.fround(1 / 3))].join(';'));
        chunk.velocities[row * 3] = velocity;
      }
    }
    // Failure follows the first source's staged writes and both groups' logs.
    if (direction < 0 && input.actions.fail?.pressed) throw Error('authored Android late failure');
  } };
}
