import { Fragment, useEffect, useId, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';

export type KeyPlatform = 'mac' | 'other';

/** Keys pressed together. `Mod` is ⌘ on macOS and Ctrl elsewhere. */
type Combo = readonly string[];

interface Shortcut {
  readonly action: string;
  /** Alternatives, each pressed as one combination. */
  readonly keys: readonly Combo[];
  /** Replaces `keys` on Windows and Linux when the keys differ there. */
  readonly other?: readonly Combo[];
  /** Where the keys apply, when the group does not already say. */
  readonly where?: string;
  /** The alternatives are the action's two directions ("First or last": Home / End). */
  readonly paired?: boolean;
}

interface ShortcutGroup {
  readonly title: string;
  readonly items: readonly Shortcut[];
}

/**
 * Every shortcut the editor handles, as implemented (App.tsx, HierarchyPanel,
 * AssetBrowser, Splitter, the Inspector lists, Menu, AgentPanel and the game view in
 * src/play/useGameCapture.ts). Nothing is listed that the code does not do.
 */
export const SHORTCUT_GROUPS: readonly ShortcutGroup[] = [
  {
    title: 'General',
    items: [
      { action: 'Next panel', keys: [['F6']] },
      { action: 'Previous panel', keys: [['Shift', 'F6']] },
      { action: 'Undo', keys: [['Mod', 'Z']] },
      { action: 'Redo', keys: [['Shift', 'Mod', 'Z']], other: [['Ctrl', 'Y'], ['Ctrl', 'Shift', 'Z']] },
      { action: 'Keyboard shortcuts', keys: [['?'], ['Mod', '/']] },
      { action: 'Close a dialog or menu', keys: [['Esc']] },
    ],
  },
  {
    title: 'Play',
    items: [
      { action: 'Play or stop', keys: [['Mod', 'P']] },
      { action: 'Pause or resume', keys: [['Shift', 'Mod', 'P']], other: [['Ctrl', 'Shift', 'P']] },
      { action: 'Step one tick', keys: [['Alt', 'Mod', 'P']], other: [['Ctrl', 'Alt', 'P']], where: 'while paused' },
      { action: 'Leave the game view', keys: [['F6'], ['Shift', 'F6']], where: 'while keys go to the game' },
    ],
  },
  {
    title: 'Hierarchy',
    items: [
      { action: 'Move selection', keys: [['↑'], ['↓']], paired: true },
      { action: 'Expand or collapse; go to child or parent', keys: [['→'], ['←']], paired: true },
      { action: 'First or last entity', keys: [['Home'], ['End']], paired: true },
      { action: 'Page up or down', keys: [['Page Up'], ['Page Down']], paired: true },
      { action: 'Jump to an entity by name', keys: [['Type its name']] },
      { action: 'Filter by name', keys: [['Mod', 'F']] },
      { action: 'Rename', keys: [['F2']] },
      { action: 'Delete', keys: [['Mod', 'Backspace'], ['Delete']], other: [['Delete'], ['Ctrl', 'Backspace']] },
      { action: 'Actions menu', keys: [['Shift', 'F10'], ['Menu']] },
    ],
  },
  {
    title: 'Panels and layout',
    items: [
      { action: 'Show or hide the hierarchy', keys: [['Alt', 'Mod', '1']], other: [['Ctrl', 'Alt', '1']] },
      { action: 'Show or hide the output', keys: [['Alt', 'Mod', '2']], other: [['Ctrl', 'Alt', '2']] },
      { action: 'Show or hide the inspector', keys: [['Alt', 'Mod', '3']], other: [['Ctrl', 'Alt', '3']] },
      { action: 'Switch tabs', keys: [['←'], ['→']], where: 'on a tab', paired: true },
      { action: 'Resize a panel', keys: [['←'], ['→']], where: 'on a divider; ↑ ↓ on a horizontal one, Shift for big steps', paired: true },
      { action: 'Smallest or largest size', keys: [['Home'], ['End']], where: 'on a divider', paired: true },
    ],
  },
  {
    title: 'Lists, assets and inspector',
    items: [
      { action: 'Choose an item', keys: [['↑'], ['↓']], paired: true },
      { action: 'First or last item', keys: [['Home'], ['End']], paired: true },
      { action: 'Page up or down', keys: [['Page Up'], ['Page Down']], paired: true },
      { action: 'Open the selected asset', keys: [['Enter'], ['Space']] },
      { action: 'Back to the asset list', keys: [['Esc']], where: 'in asset details' },
      { action: 'Filter assets', keys: [['Mod', 'F']] },
      { action: 'Add an import path below', keys: [['Shift', 'Enter']], where: 'in the import form' },
    ],
  },
  {
    title: 'Agent',
    items: [{ action: 'Send the message', keys: [['Mod', 'Enter']], other: [['Ctrl', 'Enter']] }],
  },
];

const MAC_GLYPHS: Readonly<Record<string, string>> = {
  Mod: '⌘',
  Shift: '⇧',
  Alt: '⌥',
  Ctrl: '⌃',
  Enter: '↩',
  Backspace: '⌫',
  Delete: '⌦',
  Esc: 'Esc',
};
const MAC_WORDS: Readonly<Record<string, string>> = {
  Mod: 'Command',
  Shift: 'Shift',
  Alt: 'Option',
  Ctrl: 'Control',
  Enter: 'Return',
  Backspace: 'Delete',
  Delete: 'Forward Delete',
  Esc: 'Escape',
};
const OTHER_WORDS: Readonly<Record<string, string>> = { Mod: 'Ctrl', Esc: 'Escape' };
const ARROW_WORDS: Readonly<Record<string, string>> = { '↑': 'Up', '↓': 'Down', '←': 'Left', '→': 'Right', '?': 'Question mark', '/': 'Slash' };

/** How a key is written: Mac glyphs on macOS, names elsewhere. */
export function keyLabel(key: string, platform: KeyPlatform): string {
  if (platform === 'mac') return MAC_GLYPHS[key] ?? key;
  return key === 'Mod' ? 'Ctrl' : key;
}

/** A combination as written in menus: "⇧⌘Z" on macOS, "Ctrl+Shift+Z" elsewhere. */
export function comboText(combo: Combo, platform: KeyPlatform): string {
  return combo.map((key) => keyLabel(key, platform)).join(platform === 'mac' ? '' : '+');
}

/** A combination in words, for assistive tech and the filter: "Command Shift Z". */
export function spokenCombo(combo: Combo, platform: KeyPlatform): string {
  return combo.map((key) => ARROW_WORDS[key] ?? (platform === 'mac' ? MAC_WORDS[key] : OTHER_WORDS[key]) ?? key).join(' ');
}

function combosFor(item: Shortcut, platform: KeyPlatform): readonly Combo[] {
  return platform === 'other' && item.other ? item.other : item.keys;
}

/** macOS when the native host says so; in a browser, the user agent decides. */
export function usePlatform(): KeyPlatform {
  const { snapshot } = useShell();
  const host = snapshot?.window?.platform;
  if (host) return host === 'macos' ? 'mac' : 'other';
  const agent = typeof navigator === 'undefined' ? '' : `${navigator.platform} ${navigator.userAgent}`;
  return /Mac|iPhone|iPad/.test(agent) ? 'mac' : 'other';
}

/**
 * Modal reference of keyboard shortcuts: one table, sectioned by where the keys apply,
 * with the current platform's keys written as in its menus. The filter matches actions,
 * sections and key names. Traps focus; Escape clears a filter, then closes, and focus
 * returns to the opener.
 */
export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  const platform = usePlatform();
  const filterRef = useRef<HTMLInputElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState('');
  const statusId = useId();
  const tableId = useId();
  useEffect(() => filterRef.current?.focus(), []);

  const groups = useMemo(() => {
    const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    return SHORTCUT_GROUPS.map((group) => ({
      ...group,
      items: group.items.filter((item) => {
        if (words.length === 0) return true;
        const combos = combosFor(item, platform);
        const haystack = [
          group.title,
          item.action,
          item.where ?? '',
          ...combos.map((combo) => spokenCombo(combo, platform)),
          ...combos.map((combo) => comboText(combo, platform)),
        ]
          .join(' ')
          .toLowerCase();
        return words.every((word) => haystack.includes(word));
      }),
    })).filter((group) => group.items.length > 0);
  }, [query, platform]);
  const total = SHORTCUT_GROUPS.reduce((sum, group) => sum + group.items.length, 0);
  const shown = groups.reduce((sum, group) => sum + group.items.length, 0);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (query && document.activeElement === filterRef.current) setQuery('');
      else onClose();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>('button, input, [tabindex="0"]');
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
        className="dialog dialog--shortcuts"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcuts-title"
        onKeyDown={onKeyDown}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="dialog__header shortcuts__header">
          <h2 id="shortcuts-title" className="dialog__title">
            Keyboard shortcuts
          </h2>
          <div className="search-field">
            <Icon name="search" size={14} />
            <label htmlFor="shortcuts-filter" className="visually-hidden">
              Filter shortcuts
            </label>
            <input
              id="shortcuts-filter"
              ref={filterRef}
              type="search"
              placeholder="Filter"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              aria-describedby={statusId}
              autoComplete="off"
              spellCheck={false}
            />
            <span id={statusId} className="search-field__status" role="status">
              {query.trim() ? `${shown} of ${total}` : ''}
            </span>
          </div>
          <button type="button" className="icon-button" aria-label="Close" onClick={onClose}>
            <Icon name="close" />
          </button>
        </div>
        <div className="dialog__body shortcuts__body" tabIndex={0} aria-label="Shortcut list">
          {groups.length === 0 ? (
            <p className="shortcuts__empty">No shortcut matches “{query.trim()}”.</p>
          ) : (
            <table className="shortcuts__table">
              {groups.map((group, groupIndex) => (
                <tbody key={group.title}>
                  <tr className="shortcuts__section">
                    <th colSpan={2} scope="rowgroup" id={`${tableId}-group-${groupIndex}`}>
                      {group.title}
                    </th>
                  </tr>
                  {group.items.map((item, itemIndex) => (
                    <tr key={item.action} className="shortcuts__row">
                      <td className="shortcuts__action" id={`${tableId}-action-${groupIndex}-${itemIndex}`}>
                        {item.action}
                        {item.where ? <span className="shortcuts__where">{item.where}</span> : null}
                      </td>
                      <td className="shortcuts__keys" aria-labelledby={[
                        `${tableId}-group-${groupIndex}`,
                        `${tableId}-action-${groupIndex}-${itemIndex}`,
                        ...combosFor(item, platform).map((_, index) => `${tableId}-combo-${groupIndex}-${itemIndex}-${index}`),
                      ].join(' ')}>
                        {combosFor(item, platform).map((combo, index) => (
                          <Fragment key={index}>
                            {index > 0 ? (
                              <span className="shortcuts__or" aria-hidden="true">
                                {item.paired ? '/' : 'or'}
                              </span>
                            ) : null}
                            <span className="visually-hidden" id={`${tableId}-combo-${groupIndex}-${itemIndex}-${index}`}>
                              {index > 0 ? ', or ' : ''}
                              {spokenCombo(combo, platform)}
                            </span>
                            <kbd className="shortcuts__combo" aria-hidden="true">
                              {comboText(combo, platform)}
                            </kbd>
                          </Fragment>
                        ))}
                      </td>
                    </tr>
                  ))}
                </tbody>
              ))}
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
