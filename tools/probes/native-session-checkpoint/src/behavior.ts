import type { NativeBehavior } from '@incant/runtime/native';
export default {
  persistence: { version: 1 },
  update(t) {
    let events = Number(t.state.get() ?? 0);
    events += t.timers.events.length;
    t.state.set(events);
    if (t.clock.tick === 1) t.timers.set('same-local-name', 3, 3);
    for (const instance of t.instances) {
      const count = Number(t.state.get(instance.entity) ?? 0) + Number(instance.props.speed);
      t.state.set(count, instance.entity);
      for (const chunk of t.chunks) for (let row = 0; row < chunk.ids.length; row++) {
        if (chunk.ids[row] === instance.entity) chunk.transforms[row * 10] = count + events;
      }
    }
    return undefined;
  }
} satisfies NativeBehavior;
