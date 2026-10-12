import type { NativeBehavior } from '@incant/runtime/native';

const ENTITY = '00000000000000000000000002';
export default {
  update({ animations, commands, clock, log }) {
    const row = animations.ids.indexOf(ENTITY);
    if (row < 0) throw Error('animation missing');
    // Every new Play session starts the same repeating numerical timeline.
    // The camera and entity transform stay stationary; the joint moves/rotates.
    const phase = (clock.tick - 1) % 240;
    if (phase === 0 || phase === 120) {
      commands.controlAnimation(ENTITY, 'select', phase === 0 ? 0 : 1, 0);
      commands.controlAnimation(ENTITY, 'speed', 1);
      commands.controlAnimation(ENTITY, 'looping', true);
      commands.controlAnimation(ENTITY, 'playing', true);
    }
    if (clock.tick % 30 === 0) {
      log(['skeletal', clock.tick, animations.clips[row],
        animations.seconds[row].toFixed(6)].join(':'));
    }
  },
} satisfies NativeBehavior;
