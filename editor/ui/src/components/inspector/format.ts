/**
 * Pure text helpers shared by the Inspector views and presentation profiles.
 * Presentation only: nothing here validates, rounds stored data or repairs a value.
 */

/** JSON pointer for a property path, matching Diagnostic.path. */
export function pointer(path: readonly string[]): string {
  return path.map((part) => `/${part.replace(/~/g, '~0').replace(/\//g, '~1')}`).join('');
}

export function unescapePointer(part: string): string {
  return part.replace(/~1/g, '/').replace(/~0/g, '~');
}

/**
 * Compact, readable numbers that never hide a nonzero value. Integers are shown
 * as is; ordinary magnitudes keep up to four decimals (1.5708); values whose
 * magnitude is below 0.01 or at least 1e9 use four significant digits in
 * exponent form (1e-6, -2.5e-7, 1.235e-3), so a tiny clip distance never reads 0.
 */
export function formatNumber(value: number): string {
  if (Number.isInteger(value) && Math.abs(value) < 1e9) return String(value);
  const magnitude = Math.abs(value);
  if (magnitude !== 0 && (magnitude < 0.01 || magnitude >= 1e9)) {
    return Number(value.toPrecision(4)).toExponential().replace('e+', 'e');
  }
  return String(Number(value.toFixed(4)));
}

/** The exact authored number as a hover title, only when the shown form differs. */
export function exactTitle(value: number): string | undefined {
  return formatNumber(value) === String(value) ? undefined : String(value);
}

/** "translation" -> "Translation", "castShadows" / "cast_shadows" -> "Cast shadows". */
export function humanize(key: string): string {
  const words = key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .trim()
    .toLowerCase();
  return words ? words[0]!.toUpperCase() + words.slice(1) : key;
}

/** "RigidBody" -> "Rigid Body", "ScriptComponent" -> "Script Component". Spaced names are kept. */
export function splitIdentifier(name: string): string {
  if (/\s/.test(name)) return name;
  return name.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2');
}

export function safeJson(value: unknown): string {
  try {
    const text = JSON.stringify(value);
    return text === undefined ? 'undefined' : text.length > 160 ? `${text.slice(0, 157)}…` : text;
  } catch {
    return '[unserializable]';
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** A well-formed ULID (26 Crockford base-32 characters), the shape of every stable ID. */
export function looksLikeUlid(value: string): boolean {
  return /^[0-9A-HJKMNP-TV-Z]{26}$/.test(value);
}

/** "width" and "height" -> "width and height". */
export function listAll(items: readonly string[]): string {
  return items.length < 2 ? (items[0] ?? 'nothing') : `${items.slice(0, -1).join(', ')} and ${items.at(-1)}`;
}

/** "box", "sphere" and "capsule" -> "box, sphere or capsule". */
export function listChoices(choices: readonly string[]): string {
  return choices.length < 2 ? (choices[0] ?? 'nothing') : `${choices.slice(0, -1).join(', ')} or ${choices.at(-1)}`;
}

/** "1 part" / "4 parts" for a noun whose plural adds "s". */
export function count(n: number, noun: string): string {
  return `${n.toLocaleString()} ${n === 1 ? noun : `${noun}s`}`;
}
