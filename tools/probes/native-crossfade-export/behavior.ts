import type { NativeBehavior } from '@incant/runtime/native';

export default {
  persistence: { version: 1 },
  update(t) {
    const a = t.animations, tick = t.clock.tick, c = t.commands;
    const arrays = [a.clips, a.seconds, a.speeds, a.flags, a.transitionFlags,
      a.sourceClips, a.sourceSeconds, a.sourceSpeeds, a.transitionElapsed, a.transitionDuration];
    if (a.ids.length !== 2 || t.instances.length !== 2
      || arrays.some(values => values.length !== 2)) throw Error('alignment');
    for (const instance of t.instances) {
      const id = instance.entity, row = a.ids.indexOf(id), first = instance.props.first === true;
      if (row < 0) throw Error('membership');
      const count = Number(t.state.get(id) ?? 0) + 1;
      if (count !== tick) throw Error('instance checkpoint clock');
      t.state.set(count, id);
      const before = arrays.map(values => values[row]);
      for (const chunk of t.chunks) for (let i = 0; i < chunk.ids.length; i++) {
        if (chunk.ids[i] === id) {
          const offset = i * chunk.transformStride;
          chunk.transforms[offset] = before[1];
          chunk.transforms[offset + 1] = before[6];
          chunk.transforms[offset + 2] = count + before[8];
        }
      }
      if (tick === 1) c.controlAnimation(id, 'crossfade', 1, first ? 0.5 : 0.25, 0.5);
      if (tick === 2 && first) c.controlAnimation(id, 'playing', false);
      if (tick === 3 || tick === 15 || tick === 38 || tick === 71) c.controlAnimation(id, 'playing', true);
      if (tick === 4) {
        c.controlAnimation(id, 'speed', first ? -1 : 2);
        if (first) c.controlAnimation(id, 'looping', true);
      }
      if (tick === 12 || tick === 36) c.controlAnimation(id, 'playing', false);
      if (tick === 35) c.controlAnimation(id, 'crossfade', null, 0, 0);
      if (tick === 37) c.controlAnimation(id, 'crossfade', 0, 0.5, 0.25);
      if (tick === 39 && first) c.controlAnimation(id, 'speed', 1);
      if (tick === 44) {
        if (first) c.controlAnimation(id, 'seek', 0.2);
        else c.controlAnimation(id, 'select', 0, 0.1);
      }
      if (tick === 45) {
        c.controlAnimation(id, 'speed', 2);
        c.controlAnimation(id, 'looping', false);
      }
      if (tick === 46) c.controlAnimation(id, 'crossfade', 1, 1.99, 0.25);
      if (tick === 70) c.controlAnimation(id, 'crossfade', null, 0, 0.25);
      if (tick === 90) c.controlAnimation(id, 'crossfade', 0, 0.1, 0);
      if (tick === 91) {
        if (first) c.controlAnimation(id, 'playing', false);
        else c.controlAnimation(id, 'seek', 0.5);
      }
      if (arrays.some((values, i) => values[row] !== before[i])) throw Error('queued control leaked');
      t.log(JSON.stringify([tick, id, ...before]));
    }
  },
} satisfies NativeBehavior;
