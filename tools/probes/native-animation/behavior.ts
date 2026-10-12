import type { NativeBehavior } from '@incant/runtime/native';

const ENTITY = '00000000000000000000000002';
export default {
  persistence: { version: 1 },
  update(t) {
    const a = t.animations, row = a.ids.indexOf(ENTITY), tick = t.clock.tick;
    if (row < 0 || a.ids.length !== 1) throw Error('animation membership');
    const before = [a.clips[row], a.seconds[row], a.speeds[row], a.flags[row]];
    const s = (t.state.get() ?? { count: 0, events: 0 }) as { count: number; events: number };
    const count = s.count + 1, events = s.events + t.timers.events.length;
    if (count !== tick) throw Error('persistent clock');
    t.state.set({ count, events });
    if (tick === 1) {
      t.timers.set('beat', 3, 3);
      t.commands.setLocale({ locale: 'fr', fallbacks: ['en'] });
    } else if (t.localization.locale().locale !== 'fr') {
      throw Error('checkpoint locale');
    }
    const text = t.localization.localize({
      table_id: '00000000000000000000000005', key: 'ready',
    }).text;
    if (text !== (tick === 1 ? 'Ready' : 'Prêt')) throw Error('checkpoint catalog');
    for (const chunk of t.chunks) for (let i = 0; i < chunk.ids.length; i++) {
      if (chunk.ids[i] === ENTITY) {
        const offset = i * chunk.transformStride;
        chunk.transforms[offset] = before[1];
        chunk.transforms[offset + 1] = before[3];
        chunk.transforms[offset + 2] = count + events;
      }
    }
    const c = t.commands;
    if (tick === 1) c.controlAnimation(ENTITY, 'playing', false);
    if (tick === 2) {
      c.controlAnimation(ENTITY, 'select', 1, 0.5);
      c.controlAnimation(ENTITY, 'speed', 2);
      c.controlAnimation(ENTITY, 'looping', true);
    }
    if (tick === 3) c.controlAnimation(ENTITY, 'playing', true);
    if (tick === 6) c.controlAnimation(ENTITY, 'seek', 1.75);
    if (tick === 12) c.controlAnimation(ENTITY, 'speed', -1);
    if (tick === 18) {
      c.controlAnimation(ENTITY, 'looping', false);
      c.controlAnimation(ENTITY, 'seek', 0.02);
    }
    if (tick === 22) c.controlAnimation(ENTITY, 'select', null, 0);
    if (tick === 24) c.controlAnimation(ENTITY, 'playing', true);
    if (tick === 26) {
      c.controlAnimation(ENTITY, 'select', 0, 0.05);
      c.controlAnimation(ENTITY, 'speed', 1);
    }
    if (tick === 30) c.controlAnimation(ENTITY, 'seek', 1.98);
    if (tick === 35) {
      c.controlAnimation(ENTITY, 'looping', true);
      c.controlAnimation(ENTITY, 'playing', true);
    }
    if (tick === 40) {
      c.controlAnimation(ENTITY, 'seek', 0.25);
      c.controlAnimation(ENTITY, 'speed', -0.75);
    }
    if (tick === 65) {
      c.controlAnimation(ENTITY, 'select', 1, 1);
      c.controlAnimation(ENTITY, 'playing', false);
    }
    if (tick === 70) {
      c.controlAnimation(ENTITY, 'looping', false);
      c.controlAnimation(ENTITY, 'speed', 1);
      c.controlAnimation(ENTITY, 'playing', true);
    }
    if (tick === 135) c.controlAnimation(ENTITY, 'select', null, 0);
    if (tick === 137) {
      c.controlAnimation(ENTITY, 'select', 0, 1.9);
      c.controlAnimation(ENTITY, 'playing', true);
    }
    if (tick === 149) {
      c.controlAnimation(ENTITY, 'seek', 0.5);
      c.controlAnimation(ENTITY, 'speed', -0.5);
      c.controlAnimation(ENTITY, 'looping', true);
      c.controlAnimation(ENTITY, 'playing', true);
    }
    if (tick === 157) c.controlAnimation(ENTITY, 'playing', false);
    if ([a.clips[row], a.seconds[row], a.speeds[row], a.flags[row]]
      .some((value, i) => value !== before[i])) throw Error('queued control leaked');
    t.log([tick, before[0], before[1].toFixed(9), before[2], before[3]].join(':'));
  },
} satisfies NativeBehavior;
