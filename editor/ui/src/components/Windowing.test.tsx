import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeSnapshot, ComponentSchema, EditorBridge, EditorCommand, FieldSchema, Ulid } from '../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../bridge/fixture';
import { indexAt, mountedIndices, revealSpan, rowRange } from '../shell/windowing';
import { FieldView } from './inspector/FieldView';

/**
 * Bounded rendering (PLAN.md revision 4, ADR 0009): long panels mount only the rows near
 * their scroll position, and keyboard focus, selection and rename survive the window
 * moving. jsdom has no layout, so a scroller counts as 640 px tall (FALLBACK_HEIGHT);
 * these tests pin the mounted counts and interaction contracts, not browser timing.
 */

/** The 100,000-entity fixture is built once for the file: it is the expensive part. */
let stress: BridgeSnapshot;
beforeAll(() => {
  stress = fixtureSnapshot('stress');
});
afterAll(() => {
  stress = undefined as unknown as BridgeSnapshot;
});

/** TEST DOUBLE: serves a fixed snapshot, records commands and never changes its data. */
function bridgeFor(snapshot: BridgeSnapshot, capabilities: string[] = []) {
  const commands: EditorCommand[] = [];
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

const treeItems = () => screen.queryAllByRole('treeitem');
const scrollTo = (element: HTMLElement, top: number) => {
  element.scrollTop = top;
  fireEvent.scroll(element);
};

describe('windowing helpers', () => {
  it('cover the viewport plus overscan, clamp to the list, and add pinned rows once', () => {
    expect(rowRange({ top: 0, height: 280, left: 0, width: 300 }, 1000, 28, 4)).toEqual([0, 14]);
    expect(rowRange({ top: 2800, height: 280, left: 0, width: 300 }, 1000, 28, 4)).toEqual([96, 114]);
    expect(rowRange({ top: 27_000, height: 280, left: 0, width: 300 }, 1000, 28, 4)).toEqual([960, 979]);
    expect(rowRange({ top: 27_900, height: 280, left: 0, width: 300 }, 1000, 28, 4)).toEqual([992, 1000]);
    expect(mountedIndices(2, 5, 10, [7, 3, -1, 12])).toEqual([2, 3, 4, 7]);
    expect(indexAt([2, 36, 70, 140], 69)).toBe(1);
    expect(indexAt([2, 36, 70, 140], 1)).toBe(0);
  });

  it('scrolls the least distance that reveals a span', () => {
    const box = document.createElement('div');
    Object.defineProperty(box, 'clientHeight', { value: 100 });
    box.scrollTop = 0;
    revealSpan(box, 250, 28);
    expect(box.scrollTop).toBe(178);
    revealSpan(box, 190, 28);
    expect(box.scrollTop).toBe(178);
    revealSpan(box, 40, 28);
    expect(box.scrollTop).toBe(40);
  });
});

describe('hierarchy with 100,000 entities', () => {
  it('mounts a bounded window of rows and keeps every row’s place in the tree', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: bridgeFor(stress).bridge }} />);
    expect(screen.getByRole('tab', { name: /^Hierarchy/ }).textContent).toContain('100,101');
    // Roots only at first: the scene and its 100 groups, of which a window is mounted.
    expect(treeItems().length).toBeGreaterThan(10);
    expect(treeItems().length).toBeLessThan(60);
    const group = screen.getByRole('treeitem', { name: /^Group 001/ });
    expect(group.getAttribute('aria-level')).toBe('2');
    expect(group.getAttribute('aria-setsize')).toBe('100');
    expect(group.getAttribute('aria-posinset')).toBe('1');

    // A filter that matches every entity still mounts a window, not 100,000 rows.
    fireEvent.change(screen.getByLabelText('Filter hierarchy by name'), { target: { value: 'Entity' } });
    await waitFor(() => expect(screen.getByText('100,000 matches')).toBeTruthy());
    expect(treeItems().length).toBeLessThan(60);
    expect(document.querySelectorAll('input').length).toBeLessThan(10);
  });

  it('moves keyboard focus to rows that were not mounted and keeps the selection while scrolling', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: bridgeFor(stress).bridge }} />);
    fireEvent.change(screen.getByLabelText('Filter hierarchy by name'), { target: { value: 'Entity 100.' } });
    await waitFor(() => expect(screen.getByText('1,000 matches')).toBeTruthy());
    const first = screen.getByRole('treeitem', { name: /^Stress Scene/ });
    act(() => first.focus());
    fireEvent.keyDown(first, { key: 'End' });
    const last = await waitFor(() => screen.getByRole('treeitem', { name: /^Entity 100\.1000/ }));
    await waitFor(() => expect(document.activeElement).toBe(last));
    expect(last.getAttribute('aria-selected')).toBe('true');
    expect(treeItems().length).toBeLessThan(60);

    // Scroll back to the top: the selected row stays mounted as the Tab stop, still selected.
    const tree = screen.getByRole('tree', { name: 'Hierarchy' });
    scrollTo(tree, 0);
    await waitFor(() => expect(screen.getByRole('treeitem', { name: /^Entity 100\.0001/ })).toBeTruthy());
    const pinned = screen.getByRole('treeitem', { name: /^Entity 100\.1000/ });
    expect(pinned.getAttribute('aria-selected')).toBe('true');
    expect(pinned.tabIndex).toBe(0);
    expect(document.querySelector('.inspector__name')?.textContent).toBe('Entity 100.1000');

    // Page Down moves a viewport at a time through rows that were not mounted.
    fireEvent.keyDown(pinned, { key: 'Home' });
    await waitFor(() => expect(document.activeElement?.textContent).toContain('Stress Scene'));
    fireEvent.keyDown(document.activeElement!, { key: 'PageDown' });
    await waitFor(() => expect(document.activeElement?.getAttribute('aria-posinset')).not.toBe('1'));
    expect(document.activeElement?.getAttribute('role')).toBe('treeitem');
  });

  it('keeps a rename open and commits it through the command bus after its row scrolls away', async () => {
    const { bridge, commands } = bridgeFor(stress, ['entity.rename']);
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const group = screen.getByRole('treeitem', { name: /^Group 001/ });
    fireEvent.click(group);
    fireEvent.keyDown(group, { key: 'F2' });
    const input = screen.getByLabelText<HTMLInputElement>('New name');
    // Scroll far past the row: it stays mounted while it is being renamed.
    scrollTo(screen.getByRole('tree', { name: 'Hierarchy' }), 2400);
    await waitFor(() => expect(screen.getByRole('treeitem', { name: /^Group 090/ })).toBeTruthy());
    expect(screen.getByLabelText('New name')).toBe(input);
    fireEvent.change(input, { target: { value: 'District One' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(commands).toEqual([{ type: 'entity.rename', entity: group.id.replace('tree-row-', ''), name: 'District One' }]));
  });

  it('cancels a rename with Escape after window changes, sending nothing', async () => {
    const { bridge, commands } = bridgeFor(stress, ['entity.rename']);
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const group = screen.getByRole('treeitem', { name: /^Group 002/ });
    fireEvent.click(group);
    fireEvent.keyDown(group, { key: 'F2' });
    const input = screen.getByLabelText<HTMLInputElement>('New name');
    scrollTo(screen.getByRole('tree', { name: 'Hierarchy' }), 2000);
    fireEvent.change(input, { target: { value: 'Never sent' } });
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.queryByLabelText('New name')).toBeNull();
    expect(commands).toEqual([]);
  });
});

describe('asset list with 20,000 assets', () => {
  it('mounts a window of labelled folder groups and reaches the last asset from the keyboard', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: bridgeFor(stress).bridge }} />);
    fireEvent.click(screen.getByRole('tab', { name: /^Assets/ }));
    const list = screen.getByRole('listbox', { name: 'Assets' });
    const options = () => within(list).getAllByRole('option');
    expect(options().length).toBeLessThan(60);
    for (const group of within(list).getAllByRole('group')) {
      expect(document.getElementById(group.getAttribute('aria-labelledby')!)?.textContent).toMatch(/^Folder /);
    }
    fireEvent.click(options()[0]!);
    act(() => options()[0]!.focus());
    fireEvent.keyDown(list, { key: 'End' });
    await waitFor(() => expect(document.activeElement?.getAttribute('aria-selected')).toBe('true'));
    // Folders sort by path and names within them by path, so the last row is textures/terrain/surface_9999.
    expect(document.activeElement?.textContent).toMatch(/^surface_9999/);
    expect(options().length).toBeLessThan(60);
    // The Inspector follows the selection.
    expect(document.querySelector('.inspector__name')?.textContent).toBe(document.activeElement?.querySelector('.asset-row__name')?.textContent);
  });
});

describe('a 65,536-value navigation grid', () => {
  const schema = {
    type: 'NavigationGrid',
    version: 1,
    title: 'NavigationGrid',
    properties: {
      costs: { type: 'array', items: { type: 'integer', minimum: 0, maximum: 65535 }, maxItems: 65536, optional: false },
      dimensions: { type: 'array', items: { type: 'integer' }, minItems: 2, maxItems: 2, optional: false },
    },
  } as ComponentSchema;
  const costs = Array.from({ length: 65536 }, (_, i) => (i % 97 === 0 ? 0 : 1 + (i % 5)));
  const value = { costs, dimensions: [256, 256] };
  const show = (diagnostics: BridgeSnapshot['diagnostics'] = []) =>
    render(
      <FieldView
        name="costs"
        schema={schema.properties.costs as FieldSchema}
        value={costs}
        path={['costs']}
        ctx={{ entity: '01JA33RV000000000000000001', component: 'NavigationGrid', diagnostics, value }}
      />,
    );

  it('draws a map and mounts no control per cell; the exact values open as a bounded table', () => {
    const { container } = show();
    expect(container.querySelectorAll('input')).toHaveLength(0);
    expect(container.querySelector('canvas')?.getAttribute('width')).toBe('256');
    expect(container.textContent).toContain('256 × 256');
    expect(container.textContent).toContain('676 blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Show values' }));
    const table = screen.getByRole('group', { name: /Costs/ });
    expect(table.tabIndex).toBe(0);
    const rows = table.querySelectorAll('.number-table__row');
    const cells = table.querySelectorAll('.number-table__cell');
    expect(rows.length).toBeLessThan(50);
    expect(cells.length).toBeLessThan(2000);
    expect(container.querySelectorAll('input')).toHaveLength(0);
    // Scrolling down mounts the rows now in view instead.
    act(() => {
      table.scrollTop = 22 * 200;
      fireEvent.scroll(table);
    });
    return waitFor(() => expect(table.querySelector('.number-table__index')?.textContent).not.toBe('0'));
  });

  it('marks the exact cell an engine diagnostic names', () => {
    const { container } = show([
      { id: 'g1', severity: 'error', message: 'cost 70000 exceeds 65535', entity: '01JA33RV000000000000000001' as Ulid, component: 'NavigationGrid', path: '/costs/3' },
    ]);
    expect(container.textContent).toContain('cost 70000 exceeds 65535');
    fireEvent.click(screen.getByRole('button', { name: 'Show values' }));
    expect(container.querySelectorAll('.number-table__cell.is-error')).toHaveLength(1);
    expect(container.querySelector('.number-table__cell.is-error')?.getAttribute('title')).toBe('x 3, y 0: 4');
  });

  it('shows a size that does not fit the list as a plain table, never a guessed map', () => {
    render(
      <FieldView
        name="costs"
        schema={schema.properties.costs as FieldSchema}
        value={costs.slice(0, 100)}
        path={['costs']}
        ctx={{ entity: '01JA33RV000000000000000001', component: 'NavigationGrid', diagnostics: [], value: { costs: costs.slice(0, 100), dimensions: [256, 256] } }}
      />,
    );
    expect(document.querySelector('canvas')).toBeNull();
    expect(document.querySelector('.object-list__summary')?.textContent).toBe('100 values · 0 to 5');
    expect(screen.getByRole('group', { name: /Costs/ })).toBeTruthy();
  });
});

describe('the sample fixture still mounts everything that fits', () => {
  it('renders all 25 sample entities and all 7 assets', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    expect(treeItems()).toHaveLength(25);
    fireEvent.click(screen.getByRole('tab', { name: /^Assets/ }));
    expect(within(screen.getByRole('listbox', { name: 'Assets' })).getAllByRole('option')).toHaveLength(7);
  });
});
