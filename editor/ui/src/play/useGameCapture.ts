import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';
import { useShell } from '../shell/ShellContext';
import { GameInput } from './input';
import { usePlay } from './PlayContext';

const MODIFIERS = new Set(['ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight']);

/**
 * Key presses the editor keeps while the game view has input: F6 and Shift+F6 leave the
 * view, the Play shortcuts (Mod+P, with Shift or Alt) control the session, and Command
 * (Windows key) combinations stay app shortcuts. On macOS a key released while Command is
 * held sends no release to the page, so such a press would stay held in the game.
 */
export function isEditorChord(event: KeyboardEvent): boolean {
  if (event.code === 'F6' || event.key === 'F6') return true;
  if ((event.metaKey || event.ctrlKey) && event.code === 'KeyP') return true;
  return event.metaKey && !MODIFIERS.has(event.code);
}

export interface GameCapture {
  /** A session exists and the host forwards input: the game view takes focus. */
  readonly focusable: boolean;
  /** The game view has keyboard focus. */
  readonly focused: boolean;
  /** Keys and the pointer go to the game right now. */
  readonly capturing: boolean;
}

/**
 * Routes desktop input from the focused game view to native Play. Input flows only
 * while the view was deliberately focused (a click or keyboard navigation; the
 * focusing press itself is not sent), the session runs unsuspended, no control
 * request is pending and the window has focus. Losing any of these releases held input.
 */
export function useGameCapture(host: RefObject<HTMLElement | null>): GameCapture {
  const { capabilities, request, explainUnavailable } = useShell();
  const play = usePlay();
  const supported = capabilities.has('play.input');
  const session = play.feed.session;
  const exists = session !== null && (session.phase === 'preparing' || session.phase === 'running' || session.phase === 'paused');
  const generation = exists ? session.generation : null;
  const [focused, setFocused] = useState(false);
  const [windowFocused, setWindowFocused] = useState(() => typeof document === 'undefined' || document.hasFocus());
  const focusable = supported && generation !== null;
  // What the worker does, by the last status: running unsuspended, and readable.
  const accepting = supported && session?.phase === 'running' && !session.suspended && !play.feed.transport;
  // What the editor allows: deliberate focus in a focused window, no control request pending.
  const open = focused && focusable && windowFocused && play.busy === null && play.state !== 'stopping';

  const send = useRef(request);
  const report = useRef(play.reportInputError);
  useLayoutEffect(() => {
    send.current = request;
    report.current = play.reportInputError;
  });
  const [input] = useState(
    () =>
      new GameInput({
        send: (packetGeneration, events) => send.current({ type: 'play.input', generation: packetGeneration, events }, { quiet: true }),
        onError: (message) => report.current(message),
      }),
  );
  useEffect(() => () => input.dispose(), [input]);
  useEffect(() => input.update({ generation, accepting, focused: open }), [input, generation, accepting, open]);

  useEffect(() => {
    const sync = () => setWindowFocused(document.hasFocus() && document.visibilityState !== 'hidden');
    window.addEventListener('focus', sync);
    window.addEventListener('blur', sync);
    document.addEventListener('visibilitychange', sync);
    return () => {
      window.removeEventListener('focus', sync);
      window.removeEventListener('blur', sync);
      document.removeEventListener('visibilitychange', sync);
    };
  }, []);

  // Without play.input, a click on the running game explains why it does not respond.
  const explained = useRef<number | null>(null);
  const unsupportedClick = !supported && session !== null && (session.phase === 'running' || session.phase === 'paused') ? session.generation : null;

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const local = (event: MouseEvent): [number, number] => {
      const rect = element.getBoundingClientRect();
      return [event.clientX - rect.left, event.clientY - rect.top];
    };
    const onFocus = () => {
      setFocused(true);
      setWindowFocused(true);
    };
    const onBlur = () => setFocused(false);
    const onKey = (event: KeyboardEvent) => {
      const down = event.type === 'keydown';
      if (event.isComposing || (down && isEditorChord(event))) return;
      // Releases always go through: a key held in the game is released even with Mod down.
      if (input.key(event.code, down, event.repeat)) event.preventDefault();
    };
    const onPointerDown = (event: PointerEvent) => {
      if (unsupportedClick !== null && explained.current !== unsupportedClick) {
        explained.current = unsupportedClick;
        explainUnavailable('play.input');
      }
      if (event.pointerType === 'touch' || !focusable) return;
      // The press that focuses the game view is not game input.
      if (document.activeElement !== element) {
        element.focus({ preventScroll: true });
        return;
      }
      if (!input.active) return;
      const [x, y] = local(event);
      input.pointer(x, y);
      if (input.button(event.button, true)) {
        event.preventDefault();
        // Releases outside the view still arrive, so no button stays held in the game.
        try {
          element.setPointerCapture(event.pointerId);
        } catch {
          /* Synthetic events have no active pointer to capture. */
        }
      }
    };
    const onPointerUp = (event: PointerEvent) => {
      if (event.pointerType === 'touch' || !input.active) return;
      const [x, y] = local(event);
      input.pointer(x, y);
      if (input.button(event.button, false)) event.preventDefault();
    };
    const onPointerCancel = () => {
      for (const button of [0, 1, 2]) input.button(button, false);
    };
    const onPointerMove = (event: PointerEvent) => {
      if (event.pointerType === 'touch' || !input.active) return;
      const [x, y] = local(event);
      input.pointer(x, y);
    };
    const onWheel = (event: WheelEvent) => {
      if (input.wheel(event.deltaX, event.deltaY, event.deltaMode)) event.preventDefault();
    };
    const onContextMenu = (event: MouseEvent) => {
      // The right button belongs to the game while a session exists.
      if (focusable) event.preventDefault();
    };
    element.addEventListener('focus', onFocus);
    element.addEventListener('blur', onBlur);
    element.addEventListener('keydown', onKey);
    element.addEventListener('keyup', onKey);
    element.addEventListener('pointerdown', onPointerDown);
    element.addEventListener('pointerup', onPointerUp);
    element.addEventListener('pointercancel', onPointerCancel);
    element.addEventListener('pointermove', onPointerMove);
    element.addEventListener('wheel', onWheel, { passive: false });
    element.addEventListener('contextmenu', onContextMenu);
    return () => {
      element.removeEventListener('focus', onFocus);
      element.removeEventListener('blur', onBlur);
      element.removeEventListener('keydown', onKey);
      element.removeEventListener('keyup', onKey);
      element.removeEventListener('pointerdown', onPointerDown);
      element.removeEventListener('pointerup', onPointerUp);
      element.removeEventListener('pointercancel', onPointerCancel);
      element.removeEventListener('pointermove', onPointerMove);
      element.removeEventListener('wheel', onWheel);
      element.removeEventListener('contextmenu', onContextMenu);
    };
  }, [host, input, focusable, unsupportedClick, explainUnavailable]);

  return { focusable, focused: focused && focusable, capturing: open && accepting };
}
