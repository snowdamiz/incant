import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeSnapshot, Diagnostic, EditorBridge } from '../bridge/contract';
import { fixtureSnapshot } from '../bridge/fixture';

/** TEST DOUBLE, not a bridge implementation: serves fixed diagnostics and refuses commands. */
function bridgeWith(diagnostics: (snapshot: BridgeSnapshot) => Diagnostic[]): EditorBridge {
  const base = fixtureSnapshot('sample');
  const snapshot: BridgeSnapshot = { ...base, diagnostics: diagnostics(base) };
  return {
    protocolVersion: 1,
    capabilities: [],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: () => () => {},
    dispatch: () => Promise.resolve({ ok: false, error: { code: 'engine.busy', message: 'Test double.' } }),
    request: () => Promise.resolve({ ok: true }),
  };
}

const LONG_PATH = 'models/environment/harbor/district_02/props/weathered_pier_lantern_cluster_variant_b.gltf';
const SOURCE_ERROR = 'could not cook triangle.gltf: invalid glTF: expected value at line 1 column 1';

describe('Problems rows', () => {
  it('show the full message and keep the source path in its own location column', () => {
    const bridge = bridgeWith(() => [
      { id: 'asset-source:1', severity: 'error', message: SOURCE_ERROR, entity: null, component: null, path: 'triangle.gltf' },
      {
        id: 'asset-source:2',
        severity: 'error',
        message: `could not cook ${LONG_PATH}: missing field \`asset\` at line 14 column 3`,
        entity: null,
        component: null,
        path: LONG_PATH,
      },
    ]);
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const rows = within(screen.getByRole('list', { name: 'Problems' })).getAllByRole('listitem');

    // The precise error, including line and column, is the row's own text, not a tooltip.
    const first = rows[0]!;
    expect(first.querySelector('.list__main')!.textContent).toBe(SOURCE_ERROR);
    expect(first.querySelector('.list__meta')!.textContent).toBe('triangle.gltf');
    // Message and location sit in a wrapping row, not a single truncated line.
    expect(first.querySelector('.problems__row .problems__text .list__main')).toBeTruthy();

    // Long paths get a break opportunity after each folder, without changing their text.
    const meta = rows[1]!.querySelector('.list__meta')!;
    expect(meta.textContent).toBe(LONG_PATH);
    expect(meta.querySelectorAll('wbr')).toHaveLength(LONG_PATH.split('/').length - 1);
    expect(rows[1]!.querySelector('.list__main')!.textContent).toContain('at line 14 column 3');
    // Project-level problems are not actions.
    expect(within(rows[0]!).queryByRole('button')).toBeNull();
  });

  it('keeps entity problems as buttons whose name carries the full message and reveals the entity', () => {
    let name = '';
    const bridge = bridgeWith((snapshot) => {
      const tree = snapshot.hierarchy.status === 'ready' ? snapshot.hierarchy.value : null;
      const node = Object.values(tree!.nodes).find((candidate) => candidate.parent !== null)!;
      name = node.name;
      return [
        {
          id: 'entity:1',
          severity: 'warning',
          message: 'Intensity is negative and is clamped to 0 at line 3 column 18',
          entity: node.id,
          component: 'incant.light',
          path: 'intensity',
        },
      ];
    });
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const button = within(screen.getByRole('list', { name: 'Problems' })).getByRole('button');
    expect(button.textContent).toContain('warning: Intensity is negative and is clamped to 0 at line 3 column 18');
    expect(button.querySelector('.list__meta')!.textContent).toBe(`${name} · light · intensity`);

    fireEvent.click(button);
    expect(screen.getByRole('treeitem', { selected: true }).textContent).toContain(name);
  });
});
