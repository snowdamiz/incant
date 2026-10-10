/**
 * Copying text to the system clipboard, with every outcome said in words. The async
 * Clipboard API can be absent (an older or locked-down web view) or refuse a write
 * (permission); neither may fail silently.
 */

/** Said when the runtime offers no clipboard writes; the ID stays selectable in the Inspector. */
export const COPY_ID_UNAVAILABLE = 'Copy ID is unavailable: this editor has no clipboard access. Select the ID in the Inspector to copy it.';

/** True when this runtime can write text to the clipboard. */
export function canWriteClipboard(): boolean {
  return typeof navigator !== 'undefined' && typeof navigator.clipboard?.writeText === 'function';
}

/**
 * Writes `text` and announces the result: "Entity ID copied", the unavailable reason,
 * or the refusal. Returns whether the text was copied.
 */
export async function copyText(text: string, what: string, announce: (message: string) => void): Promise<boolean> {
  if (!canWriteClipboard()) {
    announce(COPY_ID_UNAVAILABLE);
    return false;
  }
  try {
    await navigator.clipboard.writeText(text);
    announce(`${what} copied`);
    return true;
  } catch {
    announce('Could not copy: clipboard access was denied.');
    return false;
  }
}
