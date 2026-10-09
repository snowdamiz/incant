import { useEffect, useRef } from 'react';
import type { KeyboardEvent } from 'react';
import { Icon } from '../icons/Icon';

const GROUPS: { title: string; items: [keys: string, action: string][] }[] = [
  {
    title: 'Everywhere',
    items: [
      ['F6 / Shift+F6', 'Move to the next / previous panel'],
      ['⌘Z / Ctrl+Z', 'Undo the last transaction'],
      ['⇧⌘Z / Ctrl+Y', 'Redo'],
      ['?', 'Show this list'],
    ],
  },
  {
    title: 'Hierarchy',
    items: [
      ['↑ ↓', 'Move selection'],
      ['→ ←', 'Expand / collapse, or move to child / parent'],
      ['Home / End', 'First / last entity'],
      ['Type letters', 'Jump to an entity by name'],
      ['⌘F / Ctrl+F', 'Filter by name; Escape clears'],
      ['F2', 'Rename'],
      ['Delete', 'Delete'],
    ],
  },
  {
    title: 'Panels',
    items: [
      ['← → on tabs', 'Switch Problems, Console, History'],
      ['Arrows on a divider', 'Resize panels (Shift for larger steps)'],
      ['⌥⌘1 · 2 · 3', 'Show or hide hierarchy, output, inspector'],
      ['⌘Enter / Ctrl+Enter', 'Send a message to the agent'],
    ],
  },
];

/** Modal list of keyboard shortcuts. Traps focus; Escape closes and focus returns to the opener. */
export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  const closeRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  useEffect(() => closeRef.current?.focus(), []);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>('button, [tabindex="0"]');
    if (!focusable || focusable.length === 0) return;
    const first = focusable[0]!;
    const last = focusable[focusable.length - 1]!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="scrim" onMouseDown={onClose}>
      <div
        ref={dialogRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcuts-title"
        onKeyDown={onKeyDown}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="dialog__header">
          <h2 id="shortcuts-title" className="dialog__title">
            Keyboard shortcuts
          </h2>
          <button ref={closeRef} type="button" className="icon-button" aria-label="Close" onClick={onClose}>
            <Icon name="close" />
          </button>
        </div>
        <div className="dialog__body" tabIndex={0} aria-label="Shortcut list">
          {GROUPS.map((group) => (
            <div key={group.title} className="shortcuts__group">
              <h3 className="shortcuts__title">
                {group.title}
              </h3>
              <dl className="shortcuts__list">
                {group.items.map(([keys, action]) => (
                  <div key={keys} className="shortcuts__item">
                    <dt>
                      <kbd>{keys}</kbd>
                    </dt>
                    <dd>{action}</dd>
                  </div>
                ))}
              </dl>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
