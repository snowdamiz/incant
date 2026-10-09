import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { EditorBridge, EditorCommand } from './bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from './bridge/fixture';
import type { FixtureVariant } from './bridge/fixture';

/**
 * TEST DOUBLE, not a bridge implementation: serves the sample snapshot, advertises
 * edit capabilities and records commands. It never changes its snapshot, which
 * lets the tests prove the UI does not apply edits locally.
 */
function recordingBridge(capabilities: string[]) {
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

const renderWith = (bridge: EditorBridge | null) =>
  render(<App resolution={bridge ? { kind: 'bridge', bridge } : { kind: 'none' }} />);
const renderFixture = (variant: FixtureVariant) => renderWith(createFixtureBridge(variant));

async function expectNoAxeViolations(container: HTMLElement) {
  // jsdom has no layout or computed colors, so color-contrast is checked by
  // tokens.test.ts and by real-browser axe in scripts/evidence.mjs instead.
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
}

const statusMessage = () => document.querySelector('.statusbar__message')!;
const treeRow = (name: string) => screen.getByRole('treeitem', { name: new RegExp(`^${name}`) });

describe('editor shell', () => {
  it('without a bridge, says no engine is connected and shows no fabricated data', async () => {
    const { container } = renderWith(null);
    expect(screen.getByText('No engine connected.')).toBeTruthy();
    expect(screen.queryByRole('tree')).toBeNull();
    expect(screen.getByText('Native viewport not attached')).toBeTruthy();
    await expectNoAxeViolations(container);
  });

  it('labels the sample fixture as not engine data', () => {
    renderFixture('sample');
    expect(screen.getByText('Sample data')).toBeTruthy();
    expect(screen.getByText(/Sample fixture: sample, not engine state/)).toBeTruthy();
    expect(document.querySelector('.statusbar')?.textContent).toContain('Sample fixture: sample');
  });

  it('renders the hierarchy as an accessible tree with problem counts in names', async () => {
    const { container } = renderFixture('sample');
    const tree = screen.getByRole('tree', { name: 'Hierarchy' });
    expect(within(tree).getAllByRole('treeitem').length).toBe(17);
    expect(treeRow('Crate 03').textContent).toContain('1 error');
    await expectNoAxeViolations(container);
  });

  it('moves selection with the keyboard and shows the selection in the inspector', async () => {
    renderFixture('sample');
    const root = treeRow('Dock Prototype');
    act(() => root.focus());
    fireEvent.keyDown(root, { key: 'ArrowDown' });
    const environment = treeRow('Environment');
    await waitFor(() => expect(environment.getAttribute('aria-selected')).toBe('true'));
    expect(document.activeElement).toBe(environment);
    expect(document.querySelector('.inspector__name')?.textContent).toBe('Environment');

    fireEvent.keyDown(environment, { key: 'ArrowLeft' });
    expect(environment.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('treeitem', { name: /^Sun/ })).toBeNull();

    fireEvent.keyDown(environment, { key: 'End' });
    await waitFor(() => expect(treeRow('Harbor Ambience').getAttribute('aria-selected')).toBe('true'));
  });

  it('filters the tree and reports the match count', async () => {
    renderFixture('sample');
    fireEvent.change(screen.getByLabelText('Filter hierarchy by name'), { target: { value: 'crate' } });
    expect(screen.getByText('4 matches')).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Filter hierarchy by name'), { target: { value: 'zzz' } });
    expect(screen.getByText('No matching entities')).toBeTruthy();
  });

  it('explains instead of acting when the fixture cannot rename', async () => {
    renderFixture('sample');
    const root = treeRow('Dock Prototype');
    act(() => root.focus());
    fireEvent.keyDown(root, { key: 'F2' });
    await waitFor(() => expect(statusMessage().textContent).toMatch(/Rename is unavailable: the sample fixture is read-only/));
    expect(screen.queryByLabelText('New name')).toBeNull();
  });

  it('sends rename, delete and undo through the bridge without mutating local state', async () => {
    const { bridge, commands } = recordingBridge(['entity.rename', 'entity.delete', 'history.undo', 'history.redo']);
    renderWith(bridge);
    const root = treeRow('Dock Prototype');
    act(() => root.focus());
    fireEvent.keyDown(root, { key: 'F2' });
    const input = screen.getByLabelText('New name');
    fireEvent.change(input, { target: { value: 'Harbor' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(commands).toHaveLength(1));
    expect(commands[0]).toMatchObject({ type: 'entity.rename', name: 'Harbor' });
    // The snapshot did not change, so neither does the tree.
    expect(treeRow('Dock Prototype')).toBeTruthy();

    fireEvent.keyDown(treeRow('Dock Prototype'), { key: 'Delete' });
    fireEvent.keyDown(document.body, { key: 'z', ctrlKey: true });
    await waitFor(() => expect(commands.map((c) => c.type)).toEqual(['entity.rename', 'entity.delete', 'history.undo']));
  });

  it('reports unknown capabilities instead of guessing at them', () => {
    renderWith(recordingBridge(['entity.rename', 'mystery.power']).bridge);
    expect(screen.getByText('1 unknown capabilities ignored')).toBeTruthy();
  });

  it('jumps from a problem to the entity and shows the field-level error', async () => {
    renderFixture('sample');
    fireEvent.click(screen.getByRole('button', { name: /Half extents must be positive/ }));
    await waitFor(() => expect(treeRow('Crate 03').getAttribute('aria-selected')).toBe('true'));
    const inspector = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
    expect(within(inspector).getAllByText('Half extents must be positive; y is -0.5.').length).toBeGreaterThan(0);
  });

  it('presents unsupported schema fields and unregistered components explicitly', async () => {
    renderFixture('sample');
    fireEvent.click(screen.getByRole('button', { name: /Script asset not found/ }));
    const inspector = await waitFor(() => {
      const element = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
      expect(element.textContent).toContain('Grapple Controller');
      return element;
    });
    expect(inspector.textContent).toContain('Unsupported field type “curve”');
    expect(inspector.textContent).toContain('Cannot show as a number');
    expect(inspector.textContent).toContain('No schema is registered for incant.ExperimentalRope');
  });

  it('cycles focus between panels with F6 and Shift+F6', () => {
    renderFixture('sample');
    fireEvent.keyDown(document.body, { key: 'F6' });
    expect(document.activeElement?.getAttribute('role')).toBe('treeitem');
    fireEvent.keyDown(document.activeElement!, { key: 'F6' });
    expect(document.activeElement?.closest('[data-region]')?.getAttribute('data-region')).toBe('viewport');
    fireEvent.keyDown(document.activeElement!, { key: 'F6', shiftKey: true });
    expect(document.activeElement?.closest('[data-region]')?.getAttribute('data-region')).toBe('hierarchy');
  });

  it('opens the shortcuts dialog with ? and returns focus on Escape', async () => {
    const { container } = renderFixture('sample');
    const opener = screen.getByRole('button', { name: 'Keyboard shortcuts' });
    act(() => opener.focus());
    fireEvent.keyDown(opener, { key: '?' });
    const dialog = screen.getByRole('dialog', { name: 'Keyboard shortcuts' });
    expect(document.activeElement).toBe(within(dialog).getByRole('button', { name: 'Close' }));
    await expectNoAxeViolations(container);
    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it('resizes panels from the keyboard via separators', () => {
    renderFixture('sample');
    const splitter = screen.getByRole('separator', { name: 'Resize hierarchy' });
    const before = Number(splitter.getAttribute('aria-valuenow'));
    fireEvent.keyDown(splitter, { key: 'ArrowRight' });
    expect(Number(splitter.getAttribute('aria-valuenow'))).toBe(before + 16);
    fireEvent.keyDown(splitter, { key: 'Home' });
    expect(splitter.getAttribute('aria-valuenow')).toBe('200');
  });

  it.each([
    ['empty', 'This project has no entities'],
    ['loading', 'Loading hierarchy'],
    ['hierarchy-error', 'The hierarchy could not be loaded'],
  ] as const)('shows the %s state', async (variant, text) => {
    const { container } = renderFixture(variant);
    expect(screen.getAllByText(text).length).toBeGreaterThan(0);
    await expectNoAxeViolations(container);
  });

  it('announces a lost connection as an alert', () => {
    renderFixture('connection-error');
    const alerts = screen.getAllByRole('alert');
    expect(alerts.some((a) => a.textContent?.includes('Lost connection to the editor process'))).toBe(true);
  });

  it('refuses an incompatible bridge protocol', () => {
    render(<App resolution={{ kind: 'incompatible', protocolVersion: 7 }} />);
    expect(screen.getByText('Engine bridge version mismatch')).toBeTruthy();
  });

  it('agent composer is disabled and explains why without a provider', () => {
    renderFixture('sample');
    const input = screen.getByLabelText('Message to the agent') as HTMLTextAreaElement;
    expect(input.disabled).toBe(true);
    expect(screen.getAllByText('Connect an OpenAI account to use the agent.').length).toBeGreaterThan(0);
    expect(input.getAttribute('aria-describedby')).toBe('agent-reason');
  });

  it('shows no window controls and reserves no inset outside the native host', () => {
    renderFixture('sample');
    expect(screen.queryByRole('group', { name: 'Window' })).toBeNull();
    expect(document.querySelector('.titlebar__inset')).toBeNull();
    expect(document.querySelector('.titlebar')?.getAttribute('data-platform')).toBe('browser');
  });

  it('reserves the macOS traffic-light inset and draws no custom controls for native-overlay chrome', () => {
    const { bridge } = recordingBridge(['window.drag', 'window.maximize']);
    const snapshot = {
      ...bridge.getSnapshot(),
      window: { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 },
    } as const;
    renderWith({ ...bridge, getSnapshot: () => snapshot });
    expect((document.querySelector('.titlebar__inset') as HTMLElement).style.width).toBe('78px');
    expect(screen.queryByRole('group', { name: 'Window' })).toBeNull();
  });

  it('draws only advertised caption buttons and sends typed window requests', async () => {
    const requests: string[] = [];
    const { bridge } = recordingBridge(['window.drag', 'window.minimize', 'window.close']);
    const snapshot = {
      ...bridge.getSnapshot(),
      window: { platform: 'windows', controls: 'custom', maximized: false, fullscreen: false, focused: true, leadingInset: 0, trailingInset: 0 },
    } as const;
    renderWith({
      ...bridge,
      getSnapshot: () => snapshot,
      request: (request) => {
        requests.push(request.type);
        return Promise.resolve({ ok: true });
      },
    });
    const group = screen.getByRole('group', { name: 'Window' });
    expect(within(group).getAllByRole('button').map((b) => b.getAttribute('aria-label'))).toEqual(['Minimize', 'Close']);
    fireEvent.click(within(group).getByRole('button', { name: 'Close' }));
    fireEvent.mouseDown(document.querySelector('.titlebar__spacer')!, { button: 0, detail: 1 });
    // Pressing a titlebar button must not start a window drag.
    fireEvent.mouseDown(screen.getByRole('button', { name: 'Keyboard shortcuts' }), { button: 0, detail: 1 });
    await waitFor(() => expect(requests).toEqual(['window.close', 'window.drag']));
  });

  it('hides and restores panels from the titlebar, and F6 skips hidden panels', () => {
    renderFixture('sample');
    const toggle = screen.getByRole('button', { name: 'Hierarchy panel' });
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    expect(screen.queryByRole('tree')).toBeNull();
    fireEvent.keyDown(document.body, { key: 'F6' });
    expect(document.activeElement?.closest('[data-region]')?.getAttribute('data-region')).toBe('viewport');
    fireEvent.click(toggle);
    expect(screen.getByRole('tree', { name: 'Hierarchy' })).toBeTruthy();
  });
});
