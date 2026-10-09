import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it, vi } from 'vitest';
import { App } from '../../App';
import type { BridgeResult, BridgeSnapshot, EditorBridge, EditorCommand, ProjectAsset, Ulid } from '../../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../../bridge/fixture';
import type { FixtureVariant } from '../../bridge/fixture';

const ASSETS: ProjectAsset[] = [
  { id: '01J9ZF1XTR0000000000000001' as Ulid, name: 'crate', path: 'models/crate.glb', kind: 'model', fingerprint: 'f1' },
  { id: '01J9ZF1XTR0000000000000002' as Ulid, name: 'Crate albedo', path: 'textures/crate_albedo.png', kind: 'texture', fingerprint: 'f2', textureUsage: 'color' },
  { id: '01J9ZF1XTR0000000000000003' as Ulid, name: 'crate_normal', path: 'textures/crate_normal.png', kind: 'texture', fingerprint: 'f3', textureUsage: 'normal' },
];

/**
 * TEST DOUBLE, not a bridge implementation. Records every dispatch and resolves it only
 * when the test says so, so busy, failed and successful states are driven explicitly.
 * Its snapshot changes only when a test publishes one; the UI never edits it.
 */
function controlledBridge(overrides: Partial<BridgeSnapshot> = {}, capabilities = ['asset.import', 'history.undo', 'history.redo', 'entity.rename']) {
  let snapshot: BridgeSnapshot = {
    ...fixtureSnapshot('sample'),
    assets: { status: 'ready', value: ASSETS },
    assetImport: { available: true },
    ...overrides,
  };
  const listeners = new Set<() => void>();
  const historyListeners = new Set<(action: 'undo' | 'redo') => void>();
  const commands: EditorCommand[] = [];
  const pending: ((result: BridgeResult) => void)[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities,
    label: 'Controlled test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    subscribeHistoryRequests: (listener) => {
      historyListeners.add(listener);
      return () => historyListeners.delete(listener);
    },
    dispatch: (command) => {
      commands.push(command);
      return new Promise((resolve) => pending.push(resolve));
    },
    request: () => Promise.resolve({ ok: true }),
  };
  return {
    bridge,
    commands,
    resolve: async (result: BridgeResult, next?: Partial<BridgeSnapshot>) => {
      await act(async () => {
        if (next) {
          snapshot = { ...snapshot, ...next };
          listeners.forEach((listener) => listener());
        }
        pending.shift()?.(result);
      });
    },
    menuHistory: (action: 'undo' | 'redo') => historyListeners.forEach((listener) => listener(action)),
  };
}

const renderWith = (bridge: EditorBridge) => render(<App resolution={{ kind: 'bridge', bridge }} />);
const openAssets = () => fireEvent.click(screen.getByRole('tab', { name: /^Assets/ }));
const statusMessage = () => document.querySelector('.statusbar__message')!.textContent;
const pathInput = (n: number) => screen.getByRole('textbox', { name: `Path ${n}` });
const openImport = () => fireEvent.click(screen.getByRole('button', { name: 'Import' }));

async function expectNoAxeViolations(container: HTMLElement) {
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
}

describe('asset library', () => {
  it('lists the sample fixture assets with name, source and type, and keeps it read-only', async () => {
    const { container } = render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    openAssets();
    const list = screen.getByRole('listbox', { name: 'Assets' });
    expect(within(list).getAllByRole('option')).toHaveLength(7);
    expect(within(list).getByRole('option', { name: /^crate_normal textures\/crate_normal\.png Texture · Normal map/ })).toBeTruthy();
    expect(container.textContent).not.toContain('sample-fingerprint');
    await expectNoAxeViolations(container);
    openImport();
    expect(screen.getByText('Importing is unavailable')).toBeTruthy();
    expect(screen.getByText('Import assets is unavailable: the sample fixture is read-only.')).toBeTruthy();
    expect(screen.queryByRole('textbox', { name: 'Path 1' })).toBeNull();
  });

  it.each<[FixtureVariant, RegExp]>([
    ['project-loading', /Loading assets/],
    ['loading', /Loading assets/],
    ['project-error', /No project loaded/],
  ])('%s does not claim an empty library', (variant, expected) => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge(variant) }} />);
    openAssets();
    expect(screen.getByRole('tabpanel').textContent).toMatch(expected);
    expect(screen.queryByText('No assets yet')).toBeNull();
  });

  it('says when an older host does not report assets', () => {
    const { assets: _omitted, ...withoutAssets } = fixtureSnapshot('sample');
    const { bridge } = controlledBridge();
    bridge.getSnapshot = () => withoutAssets;
    renderWith(bridge);
    openAssets();
    expect(screen.getByText('Assets are not available')).toBeTruthy();
    expect(screen.queryByText('No assets yet')).toBeNull();
  });

  it('shows the host reason when importing is unavailable for an unsaved project', () => {
    const host = controlledBridge({ assets: { status: 'ready', value: [] }, assetImport: { available: false, reason: 'Open a saved project to import assets.' } });
    renderWith(host.bridge);
    openAssets();
    expect(screen.getByText('No assets yet')).toBeTruthy();
    expect(screen.getByText('Open a saved project to import assets.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Import from project folder/ }));
    expect(screen.getByText('Importing is unavailable')).toBeTruthy();
    expect(host.commands).toEqual([]);
  });

  it('validates paths before sending and sends nothing while any path is invalid', async () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.change(pathInput(1), { target: { value: '../outside/rock.png' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import 1 file' }));
    expect(screen.getByText(/Paths cannot leave the project folder/)).toBeTruthy();
    expect(pathInput(1).getAttribute('aria-invalid')).toBe('true');
    await waitFor(() => expect(statusMessage()).toMatch(/Fix the highlighted path/));
    expect(host.commands).toEqual([]);
  });

  it('imports a batch as one command, sending usage only for images that change it', async () => {
    const host = controlledBridge();
    const { container } = renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.change(pathInput(1), { target: { value: 'models/lantern.glb' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add path' }));
    fireEvent.change(pathInput(2), { target: { value: 'textures/lantern_n.png' } });
    fireEvent.click(within(screen.getByRole('group', { name: 'Interpret path 2 as' })).getByRole('radio', { name: 'Normal map' }));
    fireEvent.click(screen.getByRole('button', { name: 'Add path' }));
    // Already imported as a normal map: reimporting keeps the saved setting, so no usage is sent.
    fireEvent.change(pathInput(3), { target: { value: 'textures/crate_normal.png' } });
    expect(screen.getByText('Updates “crate_normal”')).toBeTruthy();
    expect(within(screen.getByRole('group', { name: 'Interpret path 3 as' })).getByRole('radio', { name: 'Normal map' })).toHaveProperty('checked', true);
    await expectNoAxeViolations(container);
    fireEvent.click(screen.getByRole('button', { name: 'Import 3 files' }));
    expect(host.commands).toEqual([
      {
        type: 'asset.import',
        sources: [{ source: 'models/lantern.glb' }, { source: 'textures/lantern_n.png', textureUsage: 'normal' }, { source: 'textures/crate_normal.png' }],
      },
    ]);

    // Busy: inputs are kept and read-only, there is no cancel or percentage, and a second submit sends nothing.
    expect(screen.getAllByText(/Importing 3 files…/).length).toBeGreaterThan(0);
    expect(pathInput(1).hasAttribute('readonly')).toBe(true);
    expect(screen.queryByRole('button', { name: /cancel/i })).toBeNull();
    expect(document.body.textContent).not.toMatch(/\d+%/);
    // Navigation stays responsive and the pending import survives a tab switch.
    fireEvent.click(screen.getByRole('tab', { name: /^History/ }));
    expect(screen.getByRole('tab', { name: /^Assets.*importing/ })).toBeTruthy();
    openAssets();
    expect(screen.getByRole('textbox', { name: 'Path 3' })).toBeTruthy();

    await host.resolve({ ok: true }, {
      assets: { status: 'ready', value: [...ASSETS, { id: '01J9ZF1XTR0000000000000009' as Ulid, name: 'lantern', path: 'models/lantern.glb', kind: 'model', fingerprint: 'f9' }] },
    });
    expect(screen.getByText(/Imported 3 files\. Undo reverts the whole batch\./)).toBeTruthy();
    expect(pathInput(1)).toHaveProperty('value', '');
    expect(screen.queryByRole('textbox', { name: 'Path 2' })).toBeNull();
    // With only a blank row left, the submit button claims no file count.
    expect(within(document.querySelector<HTMLElement>('.import-form__footer')!).getByRole('button').textContent).toBe('Import');
    expect(screen.getByRole('option', { name: /^lantern models\/lantern\.glb Model\s*just imported/ })).toBeTruthy();
    await waitFor(() => expect(statusMessage()).toBe('Imported 3 files.'));
    expect(host.commands).toHaveLength(1);
  });

  it('keeps every path and shows the exact error when an import fails, then retries explicitly', async () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.change(pathInput(1), { target: { value: 'models/broken.glb' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import 1 file' }));
    await host.resolve({ ok: false, error: { code: 'engine.request', message: 'could not cook models/broken.glb: missing buffer' } });
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('Nothing was imported');
    expect(alert.textContent).toContain('could not cook models/broken.glb: missing buffer');
    expect(pathInput(1)).toHaveProperty('value', 'models/broken.glb');
    expect(screen.getAllByRole('option')).toHaveLength(3);
    await waitFor(() => expect(statusMessage()).toBe('Import failed: could not cook models/broken.glb: missing buffer'));
    // Leaving the tab marks it; the error is still there when coming back.
    fireEvent.click(screen.getByRole('tab', { name: /^Problems/ }));
    expect(screen.getByRole('tab', { name: /last import failed/ })).toBeTruthy();
    openAssets();
    expect(screen.getByRole('alert').textContent).toContain('missing buffer');
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(host.commands).toHaveLength(2);
    expect(host.commands[1]).toEqual(host.commands[0]);
  });

  it('explains a revision conflict and offers a retry', async () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.change(pathInput(1), { target: { value: 'models/a.glb' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import 1 file' }));
    await host.resolve({ ok: false, error: { code: 'engine.request', message: 'document revision conflict: expected 4, current 5' } });
    expect(screen.getByRole('alert').textContent).toContain('The project changed during the import');
    expect(screen.getByRole('button', { name: 'Try again' })).toBeTruthy();
  });

  it('ignores blank rows when other rows have paths', () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.click(screen.getByRole('button', { name: 'Add path' }));
    fireEvent.change(pathInput(2), { target: { value: 'models/a.glb' } });
    fireEvent.click(screen.getByRole('button', { name: 'Import 1 file' }));
    expect(host.commands).toEqual([{ type: 'asset.import', sources: [{ source: 'models/a.glb' }] }]);
  });

  it('splits a multi-line paste into separate paths', () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    fireEvent.paste(pathInput(1), { clipboardData: { getData: () => 'models/a.glb\ntextures/b.png\n\ntextures/c.exr' } });
    expect(pathInput(1)).toHaveProperty('value', 'models/a.glb');
    expect(pathInput(2)).toHaveProperty('value', 'textures/b.png');
    expect(pathInput(3)).toHaveProperty('value', 'textures/c.exr');
  });

  it('reimports an asset without typing its path, preserving or changing its interpretation', async () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    fireEvent.click(screen.getByRole('option', { name: /^crate models/ }));
    expect(screen.getByRole('heading', { name: 'crate' })).toBeTruthy();
    expect(screen.queryByRole('radiogroup')).toBeNull();
    expect(screen.getByText('Placing models in a scene is not available yet.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Reimport' }));
    expect(host.commands.at(-1)).toEqual({ type: 'asset.import', sources: [{ source: 'models/crate.glb' }] });
    await host.resolve({ ok: true });

    fireEvent.click(screen.getByRole('option', { name: /^Crate albedo/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Reimport' }));
    expect(host.commands.at(-1)).toEqual({ type: 'asset.import', sources: [{ source: 'textures/crate_albedo.png' }] });
    await host.resolve({ ok: true });

    fireEvent.click(screen.getByRole('radio', { name: 'Linear' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reimport as Linear' }));
    expect(host.commands.at(-1)).toEqual({ type: 'asset.import', sources: [{ source: 'textures/crate_albedo.png', textureUsage: 'linear' }] });
    // While it runs, another import cannot start from anywhere.
    fireEvent.click(screen.getByRole('option', { name: /^crate models/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Reimport' }));
    expect(host.commands).toHaveLength(3);
    expect(screen.getByText('Another import is running. Reimport when it finishes.')).toBeTruthy();
    await host.resolve({ ok: true });
  });

  it('moves through the list with the keyboard, opens details with Enter and returns with Escape', async () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    const first = screen.getByRole('option', { name: /^crate models/ });
    act(() => first.focus());
    fireEvent.keyDown(first, { key: 'ArrowDown' });
    const second = screen.getByRole('option', { name: /^Crate albedo/ });
    await waitFor(() => expect(document.activeElement).toBe(second));
    fireEvent.keyDown(second, { key: 'Enter' });
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('heading', { name: 'Crate albedo' })));
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('option', { name: /^Crate albedo/ })));
    expect(screen.queryByRole('heading', { name: 'Crate albedo' })).toBeNull();
  });

  it('keeps text undo in the path field and project undo in the list', () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    openImport();
    const exec = vi.fn(() => false);
    Object.defineProperty(document, 'execCommand', { configurable: true, value: exec });
    try {
      const input = pathInput(1);
      act(() => input.focus());
      act(() => host.menuHistory('undo'));
      expect(fireEvent.keyDown(input, { key: 'z', metaKey: true })).toBe(true);
      expect(exec).toHaveBeenCalledWith('undo');
      expect(host.commands).toEqual([]);
    } finally {
      Reflect.deleteProperty(document, 'execCommand');
    }
    const row = screen.getByRole('option', { name: /^crate models/ });
    act(() => row.focus());
    fireEvent.keyDown(row, { key: 'z', metaKey: true });
    expect(host.commands).toEqual([{ type: 'history.undo' }]);
  });

  it('renders untrusted file names as text', () => {
    const hostile = '<img src=x onerror=alert(1)>';
    const host = controlledBridge({
      assets: { status: 'ready', value: [{ id: '01J9ZF1XTR0000000000000008' as Ulid, name: hostile, path: `textures/${hostile}.png`, kind: 'texture', fingerprint: 'x', textureUsage: 'color' }] },
    });
    renderWith(host.bridge);
    openAssets();
    expect(screen.getByRole('tabpanel').querySelector('img')).toBeNull();
    expect(screen.getByRole('option').textContent).toContain(hostile);
  });

  it('keeps F2 rename in the hierarchy', () => {
    const host = controlledBridge();
    renderWith(host.bridge);
    openAssets();
    const row = screen.getByRole('treeitem', { name: /^Dock Prototype/ });
    act(() => row.focus());
    fireEvent.keyDown(row, { key: 'F2' });
    expect(screen.getByRole('textbox', { name: 'New name' })).toBeTruthy();
  });
});
