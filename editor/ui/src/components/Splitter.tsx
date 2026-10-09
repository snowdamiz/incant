import { useRef } from 'react';
import type { KeyboardEvent, PointerEvent } from 'react';

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
  const drag = useRef<{ start: number; value: number } | null>(null);
  const sign = invert ? -1 : 1;
  const axis = (event: PointerEvent) => (orientation === 'vertical' ? event.clientX : event.clientY);

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
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { start: axis(event), value };
      }}
      onPointerMove={(event) => {
        if (!drag.current) return;
        const next = drag.current.value + sign * (axis(event) - drag.current.start);
        onChange(Math.min(max, Math.max(min, next)));
      }}
      onPointerUp={() => {
        drag.current = null;
      }}
      onPointerCancel={() => {
        drag.current = null;
      }}
    />
  );
}
