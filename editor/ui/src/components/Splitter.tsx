import { useRef } from 'react';
import type { KeyboardEvent, PointerEvent } from 'react';

// Focus as a press gives it, with no keyboard focus indicator. TypeScript's DOM types
// do not have HTML's `focusVisible` option yet; engines without it ignore it.
const POINTER_FOCUS = { preventScroll: true, focusVisible: false };

/**
 * Focusable window splitter (WAI-ARIA "window splitter" pattern). `value` is
 * the size of the panel the splitter controls. `invert` means the controlled
 * panel sits after the splitter, so dragging toward it makes it smaller.
 */
export function Splitter({
  label,
  orientation,
  value,
  min,
  max,
  invert = false,
  onChange,
}: {
  label: string;
  /** `vertical` separates columns (drag left/right); `horizontal` separates rows. */
  orientation: 'vertical' | 'horizontal';
  value: number;
  min: number;
  max: number;
  invert?: boolean;
  onChange: (value: number) => void;
}) {
  const drag = useRef<{ pointer: number; start: number; value: number } | null>(null);
  const sign = invert ? -1 : 1;
  const axis = (event: PointerEvent) => (orientation === 'vertical' ? event.clientX : event.clientY);
  // Only the pointer that started the drag moves or ends it.
  const end = (event: PointerEvent) => {
    if (drag.current?.pointer === event.pointerId) drag.current = null;
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? 64 : 16;
    const grow = orientation === 'vertical' ? 'ArrowRight' : 'ArrowDown';
    const shrink = orientation === 'vertical' ? 'ArrowLeft' : 'ArrowUp';
    let next: number | null = null;
    if (event.key === grow) next = value + sign * step;
    else if (event.key === shrink) next = value - sign * step;
    else if (event.key === 'Home') next = min;
    else if (event.key === 'End') next = max;
    if (next === null) return;
    event.preventDefault();
    onChange(Math.min(max, Math.max(min, next)));
  };

  return (
    <div
      className={`splitter splitter--${orientation}`}
      role="separator"
      aria-label={label}
      aria-orientation={orientation}
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onKeyDown={onKeyDown}
      onPointerDown={(event) => {
        // Only the primary button drags. A Control-click is the macOS secondary click.
        if (event.button !== 0 || event.ctrlKey || !event.isPrimary) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { pointer: event.pointerId, start: axis(event), value };
      }}
      onMouseDown={(event) => {
        // A primary press's default action would start a text selection, which a drag
        // then extends across the panels. Cancel it, but keep the focus it would have
        // given, so the arrow keys continue the resize. The press still reaches other
        // listeners, so an open menu closes.
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.focus(POINTER_FOCUS);
      }}
      onPointerMove={(event) => {
        const current = drag.current;
        if (current?.pointer !== event.pointerId) return;
        // The release can be lost, e.g. to a context menu: without the button, stop.
        if ((event.buttons & 1) === 0) {
          drag.current = null;
          return;
        }
        const next = current.value + sign * (axis(event) - current.start);
        onChange(Math.min(max, Math.max(min, next)));
      }}
      onPointerUp={end}
      onPointerCancel={end}
      onLostPointerCapture={end}
    />
  );
}
