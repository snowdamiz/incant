import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { EditorBridge, EditorCommand } from '../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../bridge/fixture';

/** TEST DOUBLE: the sample snapshot; records commands and never changes its data. */
function recording(capabilities: string[]) {
  const commands: EditorCommand[] = [];
  const snapshot = fixtureSnapshot('sample');
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities,
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
    dispatch: (command) => {
      commands.push(command);
      return Promise.resolve({ ok: true });
    },
    request: () => Promise.resolve({ ok: true }),
  };
  return { bridge, commands };
}

const row = (name: string) => screen.getByRole('treeitem', { name: new RegExp(`^${name}`) });

describe('hierarchy context menu', () => {
  // This case includes a full-App axe scan and exceeded 5s on hosted Windows.
  // Keep its behavior assertions; the local budget is not a performance gate.
  it('opens from the pointer, offers only existing actions, and deletes through the command bus', async () => {
    const { bridge, commands } = recording(['entity.rename', 'entity.delete']);
    const { container } = render(<App resolution={{ kind: 'bridge', bridge }} />);
    fireEvent.contextMenu(row('Crate 01'), { clientX: 120, clientY: 300 });
    const menu = screen.getByRole('menu', { name: 'Actions for Crate 01' });
    expect(within(menu).getAllByRole('menuitem').map((item) => item.querySelector('.menu__label')?.textContent)).toEqual(['Rename', 'Duplicate', 'Copy ID', 'Delete']);
    expect(row('Crate 01').getAttribute('aria-selected')).toBe('true');
    expect(document.activeElement).toBe(within(menu).getAllByRole('menuitem')[0]);
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
    fireEvent.click(within(menu).getByRole('menuitem', { name: /^Delete/ }));
    expect(screen.queryByRole('menu')).toBeNull();
    await waitFor(() => expect(commands).toEqual([{ type: 'entity.delete', entity: row('Crate 01').id.replace('tree-row-', '') }]));
  }, 20_000);

  it('opens from the keyboard, moves with arrows, starts a rename, and returns focus on Escape', () => {
    const { bridge } = recording(['entity.rename', 'entity.delete']);
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const crate = row('Crate 02');
    fireEvent.click(crate);
    act(() => crate.focus());
    fireEvent.keyDown(crate, { key: 'F10', shiftKey: true });
    const menu = screen.getByRole('menu', { name: 'Actions for Crate 02' });
    fireEvent.keyDown(menu, { key: 'ArrowDown' });
    expect(document.activeElement?.textContent).toContain('Duplicate');
    fireEvent.keyDown(menu, { key: 'ArrowDown' });
    expect(document.activeElement?.textContent).toContain('Copy ID');
    fireEvent.keyDown(menu, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(row('Crate 02'));

    fireEvent.keyDown(row('Crate 02'), { key: 'ContextMenu' });
    fireEvent.keyDown(screen.getByRole('menu'), { key: 'Enter' });
    expect(screen.getByLabelText('New name')).toBeTruthy();
  });

  it('keeps unavailable actions focusable and explains them instead of acting', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.contextMenu(row('Crate 03'), { clientX: 100, clientY: 200 });
    const menu = screen.getByRole('menu');
    const remove = within(menu).getByRole('menuitem', { name: /^Delete/ });
    expect(remove.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(remove);
    await waitFor(() => expect(document.querySelector('.statusbar__message')?.textContent).toMatch(/Delete is unavailable: the sample fixture is read-only/));
    expect(row('Crate 03')).toBeTruthy();
  });
});
