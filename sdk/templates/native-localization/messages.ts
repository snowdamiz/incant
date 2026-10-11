import type { NativeBehavior, NativeId } from '../../ts/src/native';
// Stable table ID from the authored project; messages are cooked with the scene.
const table: NativeId = '00000000000000000000000002';
export default {
  update({localization, commands, input}) {
    const text = localization.localize({table_id: table, key: 'hello', arguments: {name: 'Ada'}});
    if (text.missing) {
      const page = localization.report({offset: 0, limit: 16});
      void page;
    }
    // A settings menu can select a locale through this same buffered command.
    if (input.keyboard.pressed.includes('KeyL')) commands.setLocale({locale: 'fr', fallbacks: ['en']});
    localization.formatNumber(1234.5);
    localization.formatDate({year: 2024, month: 1, day: 2}, 'long');
  },
} satisfies NativeBehavior;
