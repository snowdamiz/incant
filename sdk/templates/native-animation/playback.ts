import type { NativeBehavior } from '../../ts/src/native';

/** Attachment IDs select actual native instances from the world-wide snapshot.
 * All controls publish together after update; readback stays committed here. */
export default {
  update({ animations, instances, commands, input }) {
    const selected = new Set(instances.map(instance => instance.entity));
    for (let row = 0; row < animations.ids.length; row++) {
      const id = animations.ids[row];
      if (!selected.has(id)) continue;
      const playing = (animations.flags[row] & 1) !== 0;
      const looping = (animations.flags[row] & 2) !== 0;
      if (input.actions.animationSelect?.pressed) {
        commands.controlAnimation(id, 'select', 0, 0);
      }
      if (input.actions.animationToggle?.pressed) {
        commands.controlAnimation(id, 'playing', !playing);
      }
      if (input.actions.animationLoop?.pressed) {
        commands.controlAnimation(id, 'looping', !looping);
      }
      if (input.actions.animationReverse?.pressed) {
        commands.controlAnimation(id, 'speed', -animations.speeds[row]);
      }
      if (input.actions.animationSeek?.pressed) {
        commands.controlAnimation(id, 'seek', 0);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
