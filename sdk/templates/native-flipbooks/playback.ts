import type { NativeBehavior } from '../../ts/src/native';

/** Consume one aligned bulk snapshot per update. Controls are queued until the
 * callback commits, so this snapshot continues to describe pre-command state. */
export default {
  update({ flipbooks, commands, input }) {
    for (let row = 0; row < flipbooks.ids.length; row++) {
      const id = flipbooks.ids[row];
      const playing = (flipbooks.flags[row] & 1) !== 0;
      const completed = (flipbooks.flags[row] & 2) !== 0;
      if (input.actions.restart?.pressed && completed) {
        commands.controlFlipbook(id, 'restart');
      } else if (input.actions.toggle?.pressed) {
        commands.controlFlipbook(id, playing ? 'pause' : 'play');
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
