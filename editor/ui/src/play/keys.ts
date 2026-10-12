import type { PlayInput } from '../bridge/contract';

/** The engine's physical key names: the generated InputEvent `KeyCode` union. */
export type KeyCode = Extract<PlayInput, { type: 'key' }>['code'];

/**
 * Runtime copy of the generated KeyCode list, in schema order. `satisfies` rejects a
 * name the engine does not define, and `KeyCodesComplete` fails the typecheck when the
 * generated union gains a name this list lacks. keys.test.ts also compares it with
 * schemas/InputEvent.schema.json. Browser `KeyboardEvent.code` values use these names.
 */
export const KEY_CODES = [
  'KeyA', 'KeyB', 'KeyC', 'KeyD', 'KeyE', 'KeyF', 'KeyG', 'KeyH', 'KeyI', 'KeyJ', 'KeyK', 'KeyL', 'KeyM',
  'KeyN', 'KeyO', 'KeyP', 'KeyQ', 'KeyR', 'KeyS', 'KeyT', 'KeyU', 'KeyV', 'KeyW', 'KeyX', 'KeyY', 'KeyZ',
  'Digit0', 'Digit1', 'Digit2', 'Digit3', 'Digit4', 'Digit5', 'Digit6', 'Digit7', 'Digit8', 'Digit9',
  'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight',
  'Space', 'Enter', 'Escape', 'Tab', 'Backspace', 'Delete', 'Insert', 'Home', 'End', 'PageUp', 'PageDown',
  'ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight',
  'CapsLock', 'NumLock', 'ScrollLock', 'Pause', 'PrintScreen', 'ContextMenu',
  'Backquote', 'Minus', 'Equal', 'BracketLeft', 'BracketRight', 'Backslash', 'Semicolon', 'Quote', 'Comma',
  'Period', 'Slash',
  'Numpad0', 'Numpad1', 'Numpad2', 'Numpad3', 'Numpad4', 'Numpad5', 'Numpad6', 'Numpad7', 'Numpad8', 'Numpad9',
  'NumpadAdd', 'NumpadSubtract', 'NumpadMultiply', 'NumpadDivide', 'NumpadDecimal', 'NumpadEnter',
  'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12',
] as const satisfies readonly KeyCode[];

type AssertNever<T extends never> = T;
/** Compile-time check: every generated KeyCode is listed above. */
export type KeyCodesComplete = AssertNever<Exclude<KeyCode, (typeof KEY_CODES)[number]>>;

const KNOWN: ReadonlySet<string> = new Set(KEY_CODES);

/**
 * The engine key for a browser `KeyboardEvent.code`, or null for a key the engine does
 * not define (media keys, IntlBackslash, F13 and above, Fn). Callers drop those.
 */
export function keyCodeOf(code: string): KeyCode | null {
  return KNOWN.has(code) ? (code as KeyCode) : null;
}
