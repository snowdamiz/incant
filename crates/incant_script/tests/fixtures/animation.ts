import type { NativeBehavior } from '@incant/runtime/native';

export default {
  update({ animations, instances, commands, clock }) {
    if (!(animations.clips instanceof Int32Array)
      || !(animations.seconds instanceof Float64Array)
      || !(animations.speeds instanceof Float64Array)
      || !(animations.flags instanceof Uint32Array)) throw Error('numeric arrays');
    for (const instance of instances) {
      const row = animations.ids.indexOf(instance.entity);
      if (row < 0) throw Error('missing animation instance');
      if (clock.tick === 1) {
        const clip = instance.props.clip;
        const speed = instance.props.speed;
        const looping = instance.props.looping;
        if (typeof clip !== 'number' || typeof speed !== 'number'
          || typeof looping !== 'boolean') throw Error('props');
        const old = [animations.clips[row], animations.seconds[row],
          animations.speeds[row], animations.flags[row]];
        commands.controlAnimation(instance.entity, 'select', clip, 0);
        commands.controlAnimation(instance.entity, 'speed', speed);
        commands.controlAnimation(instance.entity, 'looping', looping);
        commands.controlAnimation(instance.entity, 'playing', false);
        if (old.some((value, index) => value !== [animations.clips[row],
          animations.seconds[row], animations.speeds[row], animations.flags[row]][index])) {
          throw Error('pending readback');
        }
      } else {
        if (animations.clips[row] !== instance.props.clip
          || animations.speeds[row] !== instance.props.speed
          || (animations.flags[row] & 1) !== 0) throw Error('committed control');
        commands.controlAnimation(instance.entity, 'seek', 0.5);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
