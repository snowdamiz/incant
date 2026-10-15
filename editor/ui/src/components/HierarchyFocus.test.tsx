import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeSnapshot, EditorBridge, EditorCommand, HierarchyNode, Ulid } from '../bridge/contract';
import { fixtureId, fixtureSnapshot } from '../bridge/fixture';

/** TEST DOUBLE: publishes explicit snapshots; commands only record intent. */
function host(snapshot = fixtureSnapshot('sample')) {
  let current = snapshot;
  const listeners = new Set<() => void>();
  const commands: EditorCommand[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities: ['entity.delete', 'history.undo'],
    label: 'Focus test double',
    isFixture: false,
    getSnapshot: () => current,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    dispatch: (command) => {
      commands.push(command);
      return Promise.resolve({ ok: true });
    },
    request: () => Promise.resolve({ ok: true }),
  };
  return {
    bridge,
    commands,
    get snapshot() { return current; },
    publish: (next: BridgeSnapshot) => act(() => {
      current = next;
      listeners.forEach((listener) => listener());
    }),
  };
}

function without(snapshot: BridgeSnapshot, id: Ulid): BridgeSnapshot {
  if (snapshot.hierarchy.status !== 'ready') throw new Error('Expected a ready hierarchy.');
  const { roots, nodes } = snapshot.hierarchy.value;
  const gone = new Set<string>();
  const visit = (id: Ulid) => {
    gone.add(id);
    nodes[id]!.children.forEach(visit);
  };
  visit(id);
  return {
    ...snapshot,
    hierarchy: {
      status: 'ready',
      value: {
        roots: roots.filter((id) => !gone.has(id)),
        nodes: Object.fromEntries(Object.entries(nodes)
          .filter(([id]) => !gone.has(id))
          .map(([id, node]) => [id, { ...node, children: node.children.filter((id) => !gone.has(id)) }])),
      },
    },
  };
}

const open = (bridge: EditorBridge) => render(<App resolution={{ kind: 'bridge', bridge }} />);
const row = (name: string) => screen.getByRole('treeitem', { name: new RegExp(`^${name}`) });
const idOf = (element: HTMLElement) => element.id.replace('tree-row-', '') as Ulid;
const focusRow = (element: HTMLElement) => {
  fireEvent.click(element);
  act(() => element.focus());
};
const selected = () => within(screen.getByRole('tree', { name: 'Hierarchy' })).queryAllByRole('treeitem', { selected: true });

describe('hierarchy focus after entity removal', () => {
  it('keeps arrows working from the next surviving row after keyboard Delete without inventing a selection', () => {
    const double = host();
    open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    fireEvent.keyDown(removed, { key: 'Delete' });
    expect(double.commands).toEqual([{ type: 'entity.delete', entity: idOf(removed) }]);
    double.publish(without(double.snapshot, idOf(removed)));

    const next = row('Crate 03');
    expect(document.activeElement).toBe(next);
    expect(next.tabIndex).toBe(0);
    expect(selected()).toEqual([]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(row('Crate 04'));
    expect(selected()).toEqual([row('Crate 04')]);
  });

  it('uses the previous visible row when the removed row was last', () => {
    const double = host();
    open(double.bridge);
    const all = screen.getAllByRole('treeitem');
    const removed = all.at(-1)!;
    const previous = all.at(-2)!;
    focusRow(removed);
    double.publish(without(double.snapshot, idOf(removed)));
    expect(document.activeElement).toBe(previous);
    expect(previous.tabIndex).toBe(0);
    expect(selected()).toEqual([]);
  });

  it('recovers after reselecting the already focused row and after deleting its unselected replacement', () => {
    const double = host();
    open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    fireEvent.click(removed);
    double.publish(without(double.snapshot, idOf(removed)));
    expect(document.activeElement).toBe(row('Crate 03'));
    expect(selected()).toEqual([]);

    const replacement = row('Crate 03');
    fireEvent.keyDown(replacement, { key: 'Delete' });
    expect(double.commands).toEqual([{ type: 'entity.delete', entity: idOf(replacement) }]);
    double.publish(without(double.snapshot, idOf(replacement)));
    expect(document.activeElement).toBe(row('Crate 04'));
    expect(row('Crate 04').tabIndex).toBe(0);
    expect(selected()).toEqual([]);
  });

  it('skips the whole removed subtree when a focused descendant disappears', () => {
    const double = host();
    open(double.bridge);
    focusRow(row('Crate 02'));
    const group = row('Dock$');
    double.publish(without(double.snapshot, idOf(group)));
    expect(document.activeElement).toBe(row('Harbor Ambience'));
    expect(selected()).toEqual([]);
  });

  it('recovers after Enter reselects the already focused row before Delete', () => {
    const double = host();
    open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    fireEvent.keyDown(removed, { key: 'Enter' });
    expect(document.activeElement).toBe(removed);
    expect(selected()).toEqual([removed]);
    fireEvent.keyDown(removed, { key: 'Delete' });
    expect(double.commands).toEqual([{ type: 'entity.delete', entity: idOf(removed) }]);
    double.publish(without(double.snapshot, idOf(removed)));

    expect(document.activeElement?.id).toBe(row('Crate 03').id);
    expect(row('Crate 03').tabIndex).toBe(0);
    expect(selected()).toEqual([]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(row('Crate 04'));
    expect(selected()).toEqual([row('Crate 04')]);
  });

  it.each(['filter', 'inspector', 'dialog', 'assets', 'body'] as const)('keeps focus that explicitly moved to the %s before removal', (destination) => {
    const double = host();
    open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    if (destination === 'filter') {
      act(() => screen.getByLabelText('Filter hierarchy by name').focus());
    } else if (destination === 'inspector') {
      act(() => document.querySelector<HTMLElement>('[data-region="inspector"]')!.focus());
    } else if (destination === 'dialog') {
      fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    } else if (destination === 'assets') {
      const tab = screen.getByRole('tab', { name: /^Assets/ });
      act(() => tab.focus());
      fireEvent.click(tab);
    } else {
      act(() => removed.blur());
      expect(document.activeElement).toBe(document.body);
    }
    const before = document.activeElement;
    double.publish(without(double.snapshot, idOf(removed)));
    expect(document.activeElement).toBe(before);
  });

  it.each(['project', 'bridge'] as const)('does not recover focus into a switched %s', (changed) => {
    const double = host();
    const view = open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    const next = without(double.snapshot, idOf(removed));
    if (changed === 'project') {
      double.publish({ ...next, connection: { status: 'ready', project: { id: fixtureId(49999), name: 'Other project' } } });
    } else {
      // Same label and project: object identity must still fence recovery.
      view.rerender(<App resolution={{ kind: 'bridge', bridge: host(next).bridge }} />);
    }
    expect(document.activeElement).toBe(document.body);
    expect(selected()).toEqual([]);
  });

  it('preserves a newer explicit selection even if the previously focused row still owned focus', () => {
    const initial = fixtureSnapshot('sample');
    if (initial.hierarchy.status !== 'ready') throw new Error('Expected a ready hierarchy.');
    const newer = Object.values(initial.hierarchy.value.nodes).find((node) => node.name === 'Crate 04')!.id;
    const double = host({ ...initial, diagnostics: [{ id: 'focus-test', severity: 'warning', message: 'Select the newer entity', entity: newer, component: null, path: null }] });
    open(double.bridge);
    const removed = row('Crate 02');
    focusRow(removed);
    // A Problems selection updates intent without requesting keyboard focus in the tree.
    fireEvent.click(within(screen.getByRole('list', { name: 'Problems' })).getByRole('button', { name: /Select the newer entity/ }));
    expect(selected()).toEqual([row('Crate 04')]);
    expect(document.activeElement).toBe(removed);
    double.publish(without(double.snapshot, idOf(removed)));
    expect(document.activeElement).toBe(document.body);
    expect(selected()).toEqual([row('Crate 04')]);
  });

  it('does not recover a row hidden by filtering while the entity still exists', () => {
    const double = host();
    open(double.bridge);
    focusRow(row('Crate 02'));
    fireEvent.change(screen.getByLabelText('Filter hierarchy by name'), { target: { value: 'Sun' } });
    expect(document.activeElement).toBe(document.body);
    expect(document.querySelector('[role="treeitem"][tabindex="0"]')?.textContent).toContain('Dock Prototype');
  });

  it('pins and reveals a surviving neighbour outside the mounted window', async () => {
    const initial = fixtureSnapshot('sample');
    const root = fixtureId(50000);
    const children = Array.from({ length: 150 }, (_, index) => fixtureId(50001 + index));
    const nodes: Record<string, HierarchyNode> = { [root]: { id: root, name: 'Scene', kind: 'scene', parent: null, children } };
    children.forEach((id, index) => { nodes[id] = { id, name: `Item ${index + 1}`, kind: 'mesh', parent: root, children: [] }; });
    const double = host({ ...initial, diagnostics: [], hierarchy: { status: 'ready', value: { roots: [root], nodes } } });
    open(double.bridge);
    const tree = screen.getByRole('tree', { name: 'Hierarchy' });
    Object.defineProperty(tree, 'clientHeight', { value: 140 });
    focusRow(row('Scene'));
    fireEvent.keyDown(document.activeElement!, { key: 'End' });
    const removed = row('Item 150$');
    expect(document.activeElement).toBe(removed);
    tree.scrollTop = 0;
    fireEvent.scroll(tree);
    await waitFor(() => expect(document.getElementById(`tree-row-${children[148]}`)).toBeNull());

    double.publish(without(double.snapshot, idOf(removed)));
    const previous = row('Item 149$');
    expect(document.activeElement).toBe(previous);
    expect(previous.tabIndex).toBe(0);
    expect(tree.scrollTop).toBeGreaterThan(0);
    expect(screen.getAllByRole('treeitem').length).toBeLessThan(60);
    expect(selected()).toEqual([]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(row('Item 148$'));
  });
});
