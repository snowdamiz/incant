import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { IconName } from '../icons/Icon';
import { Icon } from '../icons/Icon';

export interface MenuItem {
  readonly id: string;
  readonly label: string;
  readonly icon?: IconName;
  /** Shown at the trailing edge, e.g. "F2". */
  readonly shortcut?: string;
  /** Unavailable items stay focusable; choosing one runs `onUnavailable` instead. */
  readonly unavailable?: string | undefined;
  readonly danger?: boolean;
  readonly onSelect: () => void;
}

/**
 * A small context menu (WAI-ARIA menu pattern): arrow keys, Home and End move between
 * items, Enter or Space chooses, Escape or Tab closes and returns focus to the element
 * that opened it. It only offers actions that already exist elsewhere in the editor;
 * an unavailable one explains why when chosen, through `onUnavailable`.
 */
export function ContextMenu({
  label,
  items,
  at,
  onClose,
  onUnavailable,
}: {
  label: string;
  items: readonly MenuItem[];
  /** Viewport coordinates of the pointer, or of the item the keyboard opened it from. */
  at: { readonly x: number; readonly y: number };
  onClose: (restoreFocus: boolean) => void;
  onUnavailable: (reason: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(0);
  const [position, setPosition] = useState(at);

  // Keep the menu inside the window, then focus its first item.
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const { width, height } = menu.getBoundingClientRect();
    setPosition({
      x: Math.max(4, Math.min(at.x, window.innerWidth - width - 4)),
      y: Math.max(4, Math.min(at.y, window.innerHeight - height - 4)),
    });
    menu.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }, [at]);

  // A press anywhere else closes the menu without moving focus back.
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose(false);
    };
    window.addEventListener('mousedown', onDown, true);
    return () => window.removeEventListener('mousedown', onDown, true);
  }, [onClose]);

  const choose = (item: MenuItem) => {
    onClose(true);
    if (item.unavailable) onUnavailable(item.unavailable);
    else item.onSelect();
  };
  const move = (index: number) => {
    const next = (index + items.length) % items.length;
    setActive(next);
    ref.current?.querySelectorAll<HTMLElement>('[role="menuitem"]')[next]?.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowDown') move(active + 1);
    else if (event.key === 'ArrowUp') move(active - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(items.length - 1);
    else if (event.key === 'Escape') onClose(true);
    else if (event.key === 'Tab') onClose(true);
    else if (event.key === 'Enter' || event.key === ' ') {
      const item = items[active];
      if (item) choose(item);
    } else return;
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <div
      ref={ref}
      className="menu"
      role="menu"
      aria-label={label}
      style={{ left: position.x, top: position.y }}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item, index) => (
        <div
          key={item.id}
          role="menuitem"
          tabIndex={index === active ? 0 : -1}
          aria-disabled={item.unavailable ? true : undefined}
          className={`menu__item${item.danger ? ' menu__item--danger' : ''}`}
          title={item.unavailable}
          onMouseEnter={() => setActive(index)}
          onClick={() => choose(item)}
        >
          {item.icon ? <Icon name={item.icon} size={14} className="menu__icon" /> : <span className="menu__icon" aria-hidden="true" />}
          <span className="menu__label">{item.label}</span>
          {item.shortcut ? (
            <kbd className="menu__shortcut" aria-hidden="true">
              {item.shortcut}
            </kbd>
          ) : null}
          {item.unavailable ? <span className="visually-hidden">, unavailable: {item.unavailable}</span> : null}
        </div>
      ))}
    </div>
  );
}
