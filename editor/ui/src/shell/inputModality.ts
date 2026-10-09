/**
 * Keyboard focus marker, independent of the engine's `:focus-visible` heuristic.
 *
 * WebKit (the macOS native host) does not match `:focus-visible` when script moves
 * focus after a key press, e.g. a dialog opened with Enter focusing its Close button.
 * Chrome does. To keep keyboard focus visible everywhere, this tracks the last input
 * modality and sets `data-focus-visible` on the focused element while the user is on
 * the keyboard. CSS draws the same ring for `:focus-visible` and `[data-focus-visible]`.
 *
 * A pointer press switches back to pointer modality and clears the marker, so a
 * pointer-opened dialog keeps the engine's own behavior (no ring on Close).
 */
export const FOCUS_VISIBLE_ATTR = 'data-focus-visible';

type Modality = 'keyboard' | 'pointer';

const MODIFIER_KEYS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Fn', 'FnLock', 'Hyper', 'Super', 'OS']);

/** Installs document-level listeners. Returns an uninstall function. */
export function installInputModality(doc: Document = document): () => void {
  let modality: Modality = 'pointer';
  let marked: Element | null = null;

  const mark = (element: Element | null) => {
    if (marked && marked !== element) marked.removeAttribute(FOCUS_VISIBLE_ATTR);
    marked = null;
    if (!element || element === doc.body || element === doc.documentElement) return;
    element.setAttribute(FOCUS_VISIBLE_ATTR, '');
    marked = element;
  };
  const clear = () => mark(null);

  const onKeyDown = (event: KeyboardEvent) => {
    // Shortcuts with a command modifier (⌘Z, Ctrl+Y, ⌘Tab) are not focus navigation.
    if (event.metaKey || event.ctrlKey || event.altKey || MODIFIER_KEYS.has(event.key)) return;
    modality = 'keyboard';
    // The element already focused (e.g. by an earlier click) now shows its ring too.
    mark(doc.activeElement);
  };
  const onPointerDown = () => {
    modality = 'pointer';
    clear();
  };
  const onFocusIn = (event: FocusEvent) => {
    if (modality === 'keyboard') mark(event.target instanceof Element ? event.target : null);
    else clear();
  };
  const onFocusOut = (event: FocusEvent) => {
    if (event.target === marked) clear();
  };

  // Capture phase: modality must be known before any handler moves focus for this event.
  doc.addEventListener('keydown', onKeyDown, true);
  doc.addEventListener('pointerdown', onPointerDown, true);
  doc.addEventListener('mousedown', onPointerDown, true);
  doc.addEventListener('focusin', onFocusIn, true);
  doc.addEventListener('focusout', onFocusOut, true);
  return () => {
    clear();
    doc.removeEventListener('keydown', onKeyDown, true);
    doc.removeEventListener('pointerdown', onPointerDown, true);
    doc.removeEventListener('mousedown', onPointerDown, true);
    doc.removeEventListener('focusin', onFocusIn, true);
    doc.removeEventListener('focusout', onFocusOut, true);
  };
}
