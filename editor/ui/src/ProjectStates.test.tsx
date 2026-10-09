import { act, fireEvent, render, screen, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from './App';
import type { BridgeSnapshot, EditorBridge, ProviderState } from './bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from './bridge/fixture';

const SIGNED_IN: ProviderState = {
  status: 'connected',
  provider: 'openai',
  method: 'oauth',
  accountLabel: 'Sample Account',
  accounts: [{ id: 'a1', label: 'Sample Account' }],
  activeAccount: 'a1',
};

/**
 * TEST DOUBLE, not a bridge implementation: publishes the snapshots the native adapter
 * produces for engine `loading`, `ready` and `error` responses (see fixture.ts, which
 * mirrors snapshotFromEngine). Account metadata is published independently, as the host does.
 */
function projectBridge(initial: BridgeSnapshot) {
  const listeners = new Set<() => void>();
  let snapshot: BridgeSnapshot = { ...initial, provider: SIGNED_IN };
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities: ['entity.rename', 'entity.delete', 'history.undo', 'history.redo', 'provider.connect', 'provider.disconnect'],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: () => Promise.resolve({ ok: false, error: { code: 'engine.busy', message: 'Wait for the current engine operation.' } }),
    request: () => Promise.resolve({ ok: true }),
  };
  const publish = (next: BridgeSnapshot) =>
    act(() => {
      snapshot = { ...next, provider: SIGNED_IN };
      for (const listener of listeners) listener();
    });
  return { bridge, publish };
}

const region = (name: string) => document.querySelector<HTMLElement>(`[data-region="${name}"]`)!;
const dockTab = (name: RegExp) => screen.getByRole('tab', { name });
const timeoutMessage = "Project loading timed out. Check the file's availability and folder permissions, then reopen the project.";

async function expectNoAxeViolations(container: HTMLElement) {
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => v.id)).toEqual([]);
}

describe('project loading and failure states', () => {
  it('while opening, no panel claims validation, history or values it does not have', async () => {
    const { bridge } = projectBridge(fixtureSnapshot('project-loading'));
    const { container } = render(<App resolution={{ kind: 'bridge', bridge }} />);

    const banner = screen.getByRole('region', { name: 'Project status' });
    expect(within(banner).getByRole('status').textContent).toBe('Opening project…');
    expect(screen.getByRole('heading', { level: 1 }).textContent).toContain('Opening project…');
    expect(screen.queryByText('No problems')).toBeNull();
    expect(within(region('dock')).getByText('Opening project…')).toBeTruthy();
    expect(within(region('inspector')).getByText('Opening project…')).toBeTruthy();
    expect(within(region('inspector')).queryByText('Nothing selected')).toBeNull();
    expect(within(region('viewport')).getByText('Opening project…')).toBeTruthy();
    expect(within(region('hierarchy')).getByRole('status', { name: 'Loading hierarchy' })).toBeTruthy();
    // The status bar shows no error/warning counts before anything was validated.
    expect(document.querySelector('.statusbar')!.textContent).not.toMatch(/errors/);

    fireEvent.click(dockTab(/History/));
    expect(screen.queryByText('No history yet')).toBeNull();
    expect(screen.queryByText(/of 0 applied/)).toBeNull();
    expect(within(region('dock')).queryByRole('button', { name: 'Undo' })).toBeNull();

    // Saved account metadata stays visible while the project opens.
    expect(screen.getByRole('button', { name: /^ChatGPT / }).textContent).toContain('Sample Account');
    await expectNoAxeViolations(container);
  });

  it('a failed open is announced once, in full, and listed in Problems without stale data', async () => {
    const { bridge, publish } = projectBridge(fixtureSnapshot('sample'));
    const { container } = render(<App resolution={{ kind: 'bridge', bridge }} />);
    // Select an entity first, so a stale inspector would be visible if it survived.
    fireEvent.click(screen.getByRole('button', { name: /Half extents must be positive/ }));
    expect(within(region('inspector')).queryByText('Nothing selected')).toBeNull();

    publish(fixtureSnapshot('project-error'));

    const alerts = screen.getAllByRole('alert');
    expect(alerts).toHaveLength(1);
    expect(alerts[0]!.textContent).toContain('The project could not be opened.');
    expect(alerts[0]!.textContent).toContain(timeoutMessage);
    expect(alerts[0]!.textContent).toContain('project.timeout');
    // The engine message already says to reopen; the UI does not add a second hint or a retry button.
    expect(alerts[0]!.textContent).not.toContain('Reopen the project to try again.');
    expect(screen.queryByRole('button', { name: /retry|try again/i })).toBeNull();

    // The full text appears in the banner and the Problems list only.
    const occurrences = [...container.querySelectorAll('*')].filter(
      (el) => [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent?.includes(timeoutMessage)),
    );
    expect(occurrences).toHaveLength(2);
    expect(within(region('dock')).getByText(timeoutMessage)).toBeTruthy();
    expect(dockTab(/Problems/).textContent).toContain('1');

    expect(screen.getByRole('heading', { level: 1 }).textContent).toContain('Project not opened');
    expect(within(region('hierarchy')).getByText('No project loaded')).toBeTruthy();
    expect(within(region('hierarchy')).queryByRole('tree')).toBeNull();
    expect(within(region('viewport')).getByText('No project loaded')).toBeTruthy();
    expect(within(region('viewport')).getByText('Error')).toBeTruthy();
    expect(within(region('inspector')).getByText('No project loaded')).toBeTruthy();
    expect(within(region('inspector')).queryByText('Read-only')).toBeNull();
    expect(within(region('agent')).getAllByText(/The agent needs an open project\./).length).toBeGreaterThan(0);

    fireEvent.click(dockTab(/History/));
    expect(within(region('dock')).getByText('No project loaded')).toBeTruthy();
    expect(screen.queryByText(/applied/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Undo' }).getAttribute('aria-disabled')).toBe('true');

    expect(screen.getByRole('button', { name: /^ChatGPT / }).textContent).toContain('Sample Account');
    await expectNoAxeViolations(container);
  });

  it('a failure without a diagnostic does not claim the project has no problems', () => {
    const failed = fixtureSnapshot('project-error');
    const { bridge } = projectBridge({ ...failed, diagnostics: [] });
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    expect(screen.queryByText('No problems')).toBeNull();
    expect(within(region('dock')).getByText('Not validated')).toBeTruthy();
  });

  it('recovers to the ready project after a reopen', () => {
    const { bridge, publish } = projectBridge(fixtureSnapshot('project-error'));
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    publish(fixtureSnapshot('project-loading'));
    expect(screen.queryByRole('alert')).toBeNull();
    publish(fixtureSnapshot('sample'));
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    expect(screen.getByRole('heading', { level: 1 }).textContent).toContain('Dock Prototype');
    expect(screen.getByRole('tree')).toBeTruthy();
    fireEvent.click(dockTab(/History/));
    expect(screen.getByText('4 of 5 applied')).toBeTruthy();
  });

  it('a lost editor process keeps its own wording and adds the reopen hint', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('connection-error') }} />);
    const alerts = screen.getAllByRole('alert');
    expect(alerts).toHaveLength(1);
    expect(alerts[0]!.textContent).toContain('Lost connection to the editor process.');
    expect(alerts[0]!.textContent).toContain('Reopen the project to try again.');
    expect(screen.getByRole('heading', { level: 1 }).textContent).toContain('Disconnected');
  });

  it('a hierarchy-only failure on a ready project still shows its full error in the panel', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('hierarchy-error') }} />);
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    const hierarchy = region('hierarchy');
    expect(within(hierarchy).getByRole('alert').textContent).toContain('expected "}" but found end of file');
  });
});
