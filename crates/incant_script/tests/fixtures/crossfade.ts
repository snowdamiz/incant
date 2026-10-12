import type { NativeBehavior } from '@incant/runtime/native';

export default {
  update({ animations, instances, commands, clock }) {
    for (const instance of instances) {
      const row = animations.ids.indexOf(instance.entity);
      if (row < 0 || animations.transitionFlags.length !== animations.ids.length) throw Error('alignment');
      const duration = instance.props.duration;
      if (typeof duration !== 'number') throw Error('duration');
      if (clock.tick === 1) {
        commands.controlAnimation(instance.entity, 'crossfade', 1, 0.5, duration);
        if (animations.transitionFlags[row] !== 0) throw Error('queued fade');
      } else {
        if (animations.transitionFlags[row] !== 3 || animations.sourceClips[row] !== 0
          || animations.sourceSeconds[row] !== 0.125 || animations.sourceSpeeds[row] !== 1
          || animations.transitionElapsed[row] !== 0.125
          || animations.transitionDuration[row] !== duration) throw Error('captured fade');
        commands.controlAnimation(instance.entity, 'select', null, 0);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
