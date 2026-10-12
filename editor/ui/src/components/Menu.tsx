import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent, RefObject } from 'react';
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
  /** Set on every item of a one-of-many choice: a checked item shows a check mark. */
  readonly checked?: boolean;
  /** Quiet words at the trailing edge, e.g. "No camera". */
  readonly note?: string;
  readonly onSelect: () => void;
}

const ITEM = '[role="menuitem"], [role="menuitemradio"]';

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
  anchor,
}: {
  label: string;
  items: readonly MenuItem[];
  /** Viewport coordinates of the pointer, or of the item the keyboard opened it from. */
  at: { readonly x: number; readonly y: number };
  onClose: (restoreFocus: boolean) => void;
  onUnavailable: (reason: string) => void;
  /** The button that opened the menu: pressing it again is its own toggle, not an outside press. */
  anchor?: RefObject<HTMLElement | null>;
}) {
  const ref = useRef<HTMLDivElement>(null);
  // A choice opens on its current answer; an action menu on its first item.
  const [active, setActive] = useState(() => Math.max(0, items.findIndex((item) => item.checked === true)));
  const [position, setPosition] = useState(at);

  // Keep the menu inside the window, then focus its first (or checked) item.
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const { width, height } = menu.getBoundingClientRect();
    setPosition({
      x: Math.max(4, Math.min(at.x, window.innerWidth - width - 4)),
      y: Math.max(4, Math.min(at.y, window.innerHeight - height - 4)),
    });
    (menu.querySelector<HTMLElement>('[aria-checked="true"]') ?? menu.querySelector<HTMLElement>(ITEM))?.focus();
  }, [at]);

  // A press anywhere else closes the menu without moving focus back.
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!ref.current?.contains(target) && !anchor?.current?.contains(target)) onClose(false);
    };
    window.addEventListener('mousedown', onDown, true);
    return () => window.removeEventListener('mousedown', onDown, true);
  }, [onClose, anchor]);

  const choose = (item: MenuItem) => {
    onClose(true);
    if (item.unavailable) onUnavailable(item.unavailable);
    else item.onSelect();
  };
  const move = (index: number) => {
    if (items.length === 0) return;
    const next = (index + items.length) % items.length;
    setActive(next);
    ref.current?.querySelectorAll<HTMLElement>(ITEM)[next]?.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // Focus is authoritative even if a programmatic focus event and this key
    // arrive in the same React batch, before the roving tab stop has committed.
    const focused = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(ITEM))
      .findIndex((item) => item.contains(document.activeElement));
    const current = focused >= 0 ? focused : active;
    if (event.key === 'ArrowDown') move(current + 1);
    else if (event.key === 'ArrowUp') move(current - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(items.length - 1);
    else if (event.key === 'Escape') onClose(true);
    else if (event.key === 'Tab') onClose(true);
    else if (event.key === 'Enter' || event.key === ' ') {
      const item = items[current];
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
          role={item.checked === undefined ? 'menuitem' : 'menuitemradio'}
          aria-checked={item.checked}
          tabIndex={index === active ? 0 : -1}
          aria-disabled={item.unavailable ? true : undefined}
          className={`menu__item${item.danger ? ' menu__item--danger' : ''}`}
          title={item.unavailable}
          onMouseEnter={() => move(index)}
          onFocus={() => setActive(index)}
          onClick={() => choose(item)}
        >
          {item.checked ? (
            <Icon name="check" size={14} className="menu__icon menu__check" />
          ) : item.icon && item.checked === undefined ? (
            <Icon name={item.icon} size={14} className="menu__icon" />
          ) : (
            <span className="menu__icon" aria-hidden="true" />
          )}
          <span className="menu__label">{item.label}</span>
          {item.shortcut ? (
            <kbd className="menu__shortcut" aria-hidden="true">
              {item.shortcut}
            </kbd>
          ) : item.note ? (
            <span className="menu__note">{item.note}</span>
          ) : null}
          {item.unavailable ? <span className="visually-hidden">, unavailable: {item.unavailable}</span> : null}
        </div>
      ))}
    </div>
  );
}
