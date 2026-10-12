import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { KEY_CODES, keyCodeOf } from './keys';

describe('game keys', () => {
  it('are exactly the generated InputEvent KeyCode list, in schema order', () => {
    const schema = JSON.parse(readFileSync(resolve(__dirname, '../../../../schemas/InputEvent.schema.json'), 'utf8'));
    expect([...KEY_CODES]).toEqual(schema.$defs.KeyCode.enum);
  });

  it('pass browser codes the engine defines and drop the rest explicitly', () => {
    expect(keyCodeOf('KeyW')).toBe('KeyW');
    expect(keyCodeOf('ShiftLeft')).toBe('ShiftLeft');
    expect(keyCodeOf('NumpadEnter')).toBe('NumpadEnter');
    for (const unsupported of ['IntlBackslash', 'F13', 'MediaPlayPause', 'AudioVolumeUp', 'Fn', 'OSLeft', 'Unidentified', '', 'keyw', 'toString', '__proto__']) {
      expect(keyCodeOf(unsupported)).toBeNull();
    }
  });
});
