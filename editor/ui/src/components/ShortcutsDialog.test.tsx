import { fireEvent, render, screen, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { EditorBridge } from '../bridge/contract';
import { fixtureSnapshot } from '../bridge/fixture';
import { SHORTCUT_GROUPS, keyLabel, spokenCombo } from './ShortcutsDialog';

/** TEST DOUBLE: the sample snapshot inside a native window of the given platform; refuses every command. */
function hostBridge(platform: 'macos' | 'windows'): EditorBridge {
  const snapshot = {
    ...fixtureSnapshot('sample'),
    window: { platform, controls: platform === 'macos' ? 'native' : 'custom', leadingInset: 0, maximized: false, focused: true },
  } as ReturnType<typeof fixtureSnapshot>;
  const refused = Promise.resolve({ ok: false as const, error: { code: 'test-double', message: 'Test double.' } });
  return {
    protocolVersion: 1,
    capabilities: [],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
    dispatch: () => refused,
    request: () => refused,
  };
}

const open = (platform: 'macos' | 'windows') => {
  const view = render(<App resolution={{ kind: 'bridge', bridge: hostBridge(platform) }} />);
  fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
  return { ...view, dialog: screen.getByRole('dialog', { name: 'Keyboard shortcuts' }) };
};
const row = (dialog: HTMLElement, action: string) =>
  [...dialog.querySelectorAll<HTMLElement>('.shortcuts__row')].find((item) => item.querySelector('.shortcuts__action')?.firstChild?.textContent === action)!;
/** The keys of each alternative, as written: "⇧⌘Z", "Ctrl+Shift+Z". */
const combos = (item: HTMLElement) => [...item.querySelectorAll('.shortcuts__combo')].map((combo) => combo.textContent);
const actions = (dialog: HTMLElement) => [...dialog.querySelectorAll('.shortcuts__row .shortcuts__action')].map((cell) => cell.firstChild?.textContent);

describe('keyboard shortcuts dialog', () => {
  it('writes macOS keys as in its menus, with spoken names', async () => {
    const { container, dialog } = open('macos');
    expect(combos(row(dialog, 'Redo'))).toEqual(['⇧⌘Z']);
    expect(combos(row(dialog, 'Show or hide the hierarchy'))).toEqual(['⌥⌘1']);
    expect(combos(row(dialog, 'Keyboard shortcuts'))).toEqual(['?', '⌘/']);
    expect(row(dialog, 'Redo').querySelector('.shortcuts__keys')?.textContent).toContain('Shift Command Z');
    // One table of sections, no cards: each section caption heads its rows.
    expect([...dialog.querySelectorAll('.shortcuts__section th')].map((th) => th.textContent)).toEqual([
      'General',
      'Hierarchy',
      'Panels and layout',
      'Lists, assets and inspector',
      'Agent',
    ]);
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });

  it('writes Windows and Linux keys by name, with their own alternatives', () => {
    const { dialog } = open('windows');
    expect(combos(row(dialog, 'Undo'))).toEqual(['Ctrl+Z']);
    expect(combos(row(dialog, 'Redo'))).toEqual(['Ctrl+Y', 'Ctrl+Shift+Z']);
    expect(combos(row(dialog, 'Send the message'))).toEqual(['Ctrl+Enter']);
  });

  it('filters by action, group or key, says when nothing matches, and clears with Escape before closing', () => {
    const { dialog } = open('macos');
    const filter = within(dialog).getByRole('searchbox', { name: 'Filter shortcuts' });
    expect(document.activeElement).toBe(filter);
    fireEvent.change(filter, { target: { value: 'rename' } });
    expect(actions(dialog)).toEqual(['Rename']);
    expect(within(dialog).getByRole('status').textContent).toMatch(/^1 of \d+$/);
    fireEvent.change(filter, { target: { value: 'command z' } });
    expect(actions(dialog)).toEqual(['Undo', 'Redo']);
    fireEvent.change(filter, { target: { value: 'teleport' } });
    expect(within(dialog).getByText('No shortcut matches “teleport”.')).toBeTruthy();
    fireEvent.keyDown(filter, { key: 'Escape' });
    expect((filter as HTMLInputElement).value).toBe('');
    expect(screen.getByRole('dialog', { name: 'Keyboard shortcuts' })).toBeTruthy();
    fireEvent.keyDown(filter, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('names every key it shows', () => {
    for (const group of SHORTCUT_GROUPS) {
      for (const item of group.items) {
        for (const combo of [...item.keys, ...(item.other ?? [])]) {
          for (const platform of ['mac', 'other'] as const) {
            expect(spokenCombo(combo, platform)).not.toMatch(/[⌘⇧⌥⌃↩⌫⌦↑↓←→]/);
            expect(combo.map((key) => keyLabel(key, platform)).join('')).not.toBe('');
          }
        }
      }
    }
  });
});
