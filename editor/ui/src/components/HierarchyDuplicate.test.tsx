import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import type { EngineRead, Invoke } from '../../../bridge/native';
import { NativeBridge } from '../../../bridge/native';
import { App } from '../App';
import type { BridgeResult, BridgeSnapshot, EditorBridge, EditorCommand, HierarchyNode, HierarchyTree, Ulid, WindowChrome } from '../bridge/contract';
import { createFixtureBridge, fixtureId, fixtureSnapshot } from '../bridge/fixture';
import { isDuplicateShortcut } from '../hierarchy/duplicate';
import { IDS, createNativePlayDouble, doubleId, doubleRead } from '../test/nativePlayDouble';

/**
 * The hierarchy's Duplicate against test doubles. These are UI tests: they show what the
 * editor sends, shows and selects for each answer. They do not prove the command bus,
 * the native host or a real copy, which belong to incant_cmd and editor/app.
 */

const EDITS = ['entity.rename', 'entity.delete', 'entity.duplicate', 'history.undo', 'history.redo'];
const MAC: WindowChrome = { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 };
const LINKED = 'invalid command: duplicate subtree: detach linked instances explicitly before duplication';
const SINGLETON = 'invalid command: duplicate subtree: a subtree containing a singleton cannot be duplicated';
const UNVERIFIED = 'The duplicate result could not be verified. Check the hierarchy before retrying.';

interface Pending {
  readonly command: EditorCommand;
  readonly resolve: (result: BridgeResult) => void;
  readonly reject: (error: unknown) => void;
}

/**
 * TEST DOUBLE, not a bridge: serves a snapshot, records every command and leaves it
 * pending until the test answers it. An answer can publish a snapshot first, as the
 * native bridge does before it resolves. It never copies or changes anything by itself.
 */
function host({ capabilities = EDITS, snapshot = fixtureSnapshot('sample'), mac = false }: { capabilities?: readonly string[]; snapshot?: BridgeSnapshot; mac?: boolean } = {}) {
  const shown = (next: BridgeSnapshot) => (mac ? { ...next, window: MAC } : next);
  let current = shown(snapshot);
  const listeners = new Set<() => void>();
  const commands: EditorCommand[] = [];
  const pending: Pending[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities,
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => current,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dispatch: (command) => {
      commands.push(command);
      return new Promise<BridgeResult>((resolve, reject) => pending.push({ command, resolve, reject }));
    },
    request: () => Promise.resolve({ ok: true }),
  };
  const set = (next: BridgeSnapshot) => {
    current = shown(next);
    listeners.forEach((listener) => listener());
  };
  const take = () => {
    const call = pending.shift();
    if (!call) throw new Error('No command is pending.');
    return call;
  };
  return {
    bridge,
    commands,
    get snapshot() {
      return current;
    },
    /** Another edit, a reload or another project, published while nothing is answered. */
    publish: (next: BridgeSnapshot) => act(() => set(next)),
    /** Answers the oldest pending command; `next` is published first. */
    answer: async (result: BridgeResult, next?: BridgeSnapshot) => {
      const call = take();
      await act(async () => {
        if (next) set(next);
        call.resolve(result);
      });
    },
    /** The oldest pending command's promise rejects instead of resolving. */
    reject: async (error: unknown) => {
      const call = take();
      await act(async () => call.reject(error));
    },
  };
}

const ready = (snapshot: BridgeSnapshot): HierarchyTree => {
  if (snapshot.hierarchy.status !== 'ready') throw new Error('The hierarchy is not ready.');
  return snapshot.hierarchy.value;
};
const idOf = (snapshot: BridgeSnapshot, name: string) => Object.values(ready(snapshot).nodes).find((node) => node.name === name)!.id;

/**
 * TEST DATA, not engine output: `source`'s subtree copied to fresh ids from `first` on,
 * with the same names, kinds and components, the copied root last under the same parent
 * (as newer ULIDs order in the engine's scene map).
 */
function withCopy(snapshot: BridgeSnapshot, source: Ulid, first: number) {
  const { roots, nodes } = ready(snapshot);
  const ids = new Map<string, Ulid>();
  let next = first;
  const visit = (id: Ulid) => {
    ids.set(id, fixtureId(next++));
    nodes[id]!.children.forEach(visit);
  };
  visit(source);
  const copied: Record<string, HierarchyNode> = { ...nodes };
  const entities = { ...snapshot.entities };
  for (const [from, to] of ids) {
    const node = nodes[from]!;
    copied[to] = { ...node, id: to, parent: from === source ? node.parent : ids.get(node.parent!)!, children: node.children.map((child) => ids.get(child)!) };
    const detail = snapshot.entities[from];
    if (detail) entities[to] = { ...detail, id: to };
  }
  const parent = nodes[source]!.parent;
  const copy = ids.get(source)!;
  if (parent !== null) copied[parent] = { ...nodes[parent]!, children: [...nodes[parent]!.children, copy] };
  const value: HierarchyTree = { roots: parent === null ? [...roots, copy] : roots, nodes: copied };
  return { next: { ...snapshot, hierarchy: { status: 'ready' as const, value }, entities }, copy, ids };
}

/** TEST DATA: `id` and its descendants removed, as another edit or an Undo would leave it. */
function without(snapshot: BridgeSnapshot, id: Ulid): BridgeSnapshot {
  const { roots, nodes } = ready(snapshot);
  const gone = new Set<string>();
  const visit = (node: Ulid) => {
    gone.add(node);
    nodes[node]!.children.forEach(visit);
  };
  visit(id);
  const kept = Object.fromEntries(
    Object.entries(nodes)
      .filter(([key]) => !gone.has(key))
      .map(([key, node]) => [key, { ...node, children: node.children.filter((child) => !gone.has(child)) }]),
  );
  return { ...snapshot, hierarchy: { status: 'ready', value: { roots: roots.filter((root) => !gone.has(root)), nodes: kept } } };
}

const open = (bridge: EditorBridge) => render(<App resolution={{ kind: 'bridge', bridge }} />);
const tree = () => screen.getByRole('tree', { name: 'Hierarchy' });
const rows = () => within(tree()).getAllByRole('treeitem');
const rowOf = (id: string) => {
  const element = document.getElementById(`tree-row-${id}`);
  if (!element) throw new Error(`Row ${id} is not mounted.`);
  return element;
};
const selected = () => within(tree()).queryAllByRole('treeitem', { selected: true });
const message = () => document.querySelector('.statusbar__message')?.textContent ?? '';
const settle = () => act(() => new Promise((done) => setTimeout(done, 60)));
const focusRow = (id: string) => {
  fireEvent.click(rowOf(id));
  act(() => rowOf(id).focus());
};
/** ⌘D or Ctrl+D on `target`; false when the key was taken. */
const pressDuplicate = (target: Element, mac = false, extra: Partial<KeyboardEventInit> = {}) =>
  fireEvent.keyDown(target, { key: 'd', code: 'KeyD', metaKey: mac, ctrlKey: !mac, ...extra });
const menuItem = (name: RegExp) => within(screen.getByRole('menu')).getByRole('menuitem', { name });

async function expectNoAxeViolations(container: HTMLElement) {
  // jsdom has no layout or colours; contrast is checked by tokens.test.ts and the browser review.
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
}

describe('duplicate shortcut', () => {
  const press = (fields: Partial<Parameters<typeof isDuplicateShortcut>[0]>) => ({ key: 'd', code: 'KeyD', metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...fields });

  it('is ⌘D on macOS and Ctrl+D elsewhere, by layout letter or by the physical key without Latin letters', () => {
    expect(isDuplicateShortcut(press({ metaKey: true }), 'mac')).toBe(true);
    expect(isDuplicateShortcut(press({ metaKey: true, key: 'D' }), 'mac')).toBe(true);
    expect(isDuplicateShortcut(press({ ctrlKey: true }), 'other')).toBe(true);
    // A layout without Latin letters: the physical D key.
    expect(isDuplicateShortcut(press({ ctrlKey: true, key: 'в' }), 'other')).toBe(true);
    // A Latin layout's D on another physical key counts; another letter on the D key does not.
    expect(isDuplicateShortcut(press({ metaKey: true, code: 'KeyE' }), 'mac')).toBe(true);
    expect(isDuplicateShortcut(press({ metaKey: true, key: 'e' }), 'mac')).toBe(false);
  });

  it('leaves the other platform’s modifier, Shift, Option and Alt alone', () => {
    expect(isDuplicateShortcut(press({ ctrlKey: true }), 'mac')).toBe(false);
    expect(isDuplicateShortcut(press({ metaKey: true, ctrlKey: true }), 'mac')).toBe(false);
    expect(isDuplicateShortcut(press({ metaKey: true }), 'other')).toBe(false);
    expect(isDuplicateShortcut(press({ metaKey: true, shiftKey: true, key: 'D' }), 'mac')).toBe(false);
    expect(isDuplicateShortcut(press({ metaKey: true, altKey: true, key: '∂' }), 'mac')).toBe(false);
    // AltGr arrives as Ctrl+Alt on Windows.
    expect(isDuplicateShortcut(press({ ctrlKey: true, altKey: true }), 'other')).toBe(false);
    expect(isDuplicateShortcut(press({}), 'other')).toBe(false);
  });
});

describe('hierarchy duplicate', () => {
  it('duplicates the focused entity with ⌘D on macOS and selects, reveals and focuses the returned copy', async () => {
    const double = host({ mac: true });
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    const before = rows().length;
    // Control+D is not the macOS shortcut.
    expect(pressDuplicate(rowOf(crate), false)).toBe(true);
    expect(double.commands).toEqual([]);
    expect(pressDuplicate(rowOf(crate), true)).toBe(false);
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
    // Nothing is shown before the engine answers.
    expect(tree().getAttribute('aria-busy')).toBe('true');
    expect(rows()).toHaveLength(before);
    expect(selected()).toEqual([rowOf(crate)]);

    const { next, copy } = withCopy(double.snapshot, crate, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    expect(selected()).toEqual([rowOf(copy)]);
    expect(rowOf(copy).querySelector('.tree-row__name')?.textContent).toBe('Crate 01');
    expect(rowOf(copy).getAttribute('aria-level')).toBe(rowOf(crate).getAttribute('aria-level'));
    expect(tree().hasAttribute('aria-busy')).toBe(false);
    expect(document.querySelector('.inspector__name')?.textContent).toBe('Crate 01');
    await waitFor(() => expect(message()).toBe('Duplicated Crate 01'));

    // One command and no rename; Undo is the existing history command.
    fireEvent.keyDown(rowOf(copy), { key: 'z', code: 'KeyZ', metaKey: true });
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }, { type: 'history.undo' }]);
  });

  it('uses Ctrl+D on Windows and Linux and leaves other combinations to the page', () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 02');
    focusRow(crate);
    expect(pressDuplicate(rowOf(crate), true)).toBe(true);
    expect(pressDuplicate(rowOf(crate), false, { key: 'D', shiftKey: true })).toBe(true);
    expect(pressDuplicate(rowOf(crate), false, { altKey: true })).toBe(true);
    expect(double.commands).toEqual([]);
    expect(pressDuplicate(rowOf(crate))).toBe(false);
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
  });

  it('offers Duplicate after Rename in the context menu and duplicates through the command bus', async () => {
    const double = host({ mac: true });
    const { container } = open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    fireEvent.contextMenu(rowOf(crate), { clientX: 120, clientY: 300 });
    const menu = screen.getByRole('menu', { name: 'Actions for Crate 01' });
    expect(within(menu).getAllByRole('menuitem').map((item) => item.querySelector('.menu__label')?.textContent)).toEqual(['Rename', 'Duplicate', 'Copy ID', 'Delete']);
    const item = menuItem(/^Duplicate/);
    expect(item.querySelector('.menu__shortcut')?.textContent).toBe('⌘D');
    expect(item.hasAttribute('aria-disabled')).toBe(false);
    await expectNoAxeViolations(container);

    fireEvent.click(item);
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(rowOf(crate));
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
    const { next, copy } = withCopy(double.snapshot, crate, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    expect(selected()).toEqual([rowOf(copy)]);
  }, 20_000);

  it('writes the menu shortcut as Ctrl+D on Windows and Linux and opens the menu from the keyboard', () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    fireEvent.keyDown(rowOf(crate), { key: 'F10', shiftKey: true });
    const menu = screen.getByRole('menu', { name: 'Actions for Crate 01' });
    fireEvent.keyDown(menu, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(menuItem(/^Duplicate/));
    expect(menuItem(/^Duplicate/).querySelector('.menu__shortcut')?.textContent).toBe('Ctrl+D');
    fireEvent.keyDown(menu, { key: 'Enter' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(rowOf(crate));
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
  });

  it('copies a nested entity with its descendants under the same parent and selects the copied root', async () => {
    const double = host();
    open(double.bridge);
    const player = idOf(double.snapshot, 'Player');
    const spawn = idOf(double.snapshot, 'Player Spawn');
    focusRow(player);
    pressDuplicate(rowOf(player));
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: player }]);
    const { next, copy } = withCopy(double.snapshot, player, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));

    // Same parent and level, last of the parent's children, which stay shown.
    expect(rowOf(copy).getAttribute('aria-level')).toBe(rowOf(player).getAttribute('aria-level'));
    expect([rowOf(copy).getAttribute('aria-posinset'), rowOf(copy).getAttribute('aria-setsize')]).toEqual(['2', '2']);
    expect(rowOf(spawn).getAttribute('aria-expanded')).toBe('true');
    // The copied root is selected, not one of its copied descendants.
    expect(selected()).toEqual([rowOf(copy)]);
    expect(rowOf(copy).getAttribute('aria-expanded')).toBe('false');
    fireEvent.keyDown(rowOf(copy), { key: 'ArrowRight' });
    const names = (id: Ulid) => ready(next).nodes[id]!.children.map((child) => rowOf(child).querySelector('.tree-row__name')?.textContent);
    expect(names(copy)).toEqual(['Camera Rig', 'Map Camera', 'Close-up Camera', 'Grapple Controller']);
    expect(names(copy)).toEqual(names(player));
  });

  it('reveals and focuses a copy that lands far outside the mounted rows of a long list', async () => {
    const double = host({ snapshot: fixtureSnapshot('large') });
    open(double.bridge);
    const group = idOf(double.snapshot, 'Group 001');
    const first = idOf(double.snapshot, 'Entity 001.001');
    focusRow(group);
    fireEvent.keyDown(rowOf(group), { key: 'ArrowRight' });
    fireEvent.keyDown(rowOf(group), { key: 'ArrowRight' });
    await waitFor(() => expect(document.activeElement).toBe(rowOf(first)));
    pressDuplicate(rowOf(first));
    const { next, copy } = withCopy(double.snapshot, first, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    expect([rowOf(copy).getAttribute('aria-posinset'), rowOf(copy).getAttribute('aria-setsize')]).toEqual(['51', '51']);
    expect(rowOf(group).getAttribute('aria-expanded')).toBe('true');
    // 92 rows are visible; bounded rendering mounts the copy as the active row, not the list.
    expect(rows().length).toBeLessThan(45);
  });

  it.each([
    ['first', 'last'],
    ['last', 'first'],
  ])('selects exactly the returned entity when it is the %s of two new same-named rows', async (returned) => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    // Another edit added an entity with the same name and parent in the same snapshot.
    const one = withCopy(double.snapshot, crate, 50000);
    const two = withCopy(one.next, crate, 50100);
    const [copy, other] = returned === 'first' ? [one.copy, two.copy] : [two.copy, one.copy];
    await double.answer({ ok: true, duplicatedEntity: copy }, two.next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    expect(selected()).toEqual([rowOf(copy)]);
    expect(rowOf(other).getAttribute('aria-selected')).toBe('false');
  });

  it('keeps the read-only sample fixture read-only and explains why', async () => {
    const fixture = createFixtureBridge('sample');
    const dispatch = vi.spyOn(fixture, 'dispatch');
    open(fixture);
    const crate = idOf(fixture.getSnapshot(), 'Crate 01');
    focusRow(crate);
    expect(pressDuplicate(rowOf(crate))).toBe(false);
    const reason = 'Duplicate is unavailable: the sample fixture is read-only.';
    await waitFor(() => expect(message()).toBe(reason));
    fireEvent.contextMenu(rowOf(crate), { clientX: 100, clientY: 200 });
    const item = menuItem(/^Duplicate/);
    expect(item.getAttribute('aria-disabled')).toBe('true');
    expect(item.getAttribute('title')).toBe(reason);
    fireEvent.click(item);
    expect(dispatch).not.toHaveBeenCalled();
    expect(rows()).toHaveLength(25);
  });

  it('treats a host that does not advertise entity.duplicate as unable to duplicate', async () => {
    const double = host({ capabilities: ['entity.rename', 'entity.delete', 'history.undo', 'history.redo'] });
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    await waitFor(() => expect(message()).toBe('Duplicate is unavailable in this editor build (entity.duplicate).'));
    fireEvent.contextMenu(rowOf(crate), { clientX: 100, clientY: 200 });
    expect(menuItem(/^Duplicate/).getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(menuItem(/^Duplicate/));
    expect(double.commands).toEqual([]);
  });

  it('sends one command while one runs: held keys repeat nothing, and another press or choice is refused', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    const before = rows().length;
    pressDuplicate(rowOf(crate));
    // Holding the keys: taken, but silent.
    expect(pressDuplicate(rowOf(crate), false, { repeat: true })).toBe(false);
    await settle();
    expect(message()).toBe('');
    pressDuplicate(rowOf(crate));
    await waitFor(() => expect(message()).toBe('Wait for the current duplicate to finish.'));
    fireEvent.keyDown(rowOf(crate), { key: 'F10', shiftKey: true });
    const item = menuItem(/^Duplicate/);
    expect(item.getAttribute('aria-disabled')).toBe('true');
    expect(item.getAttribute('title')).toBe('Wait for the current duplicate to finish.');
    fireEvent.click(item);
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
    // No row and no selection change until the engine answers.
    expect(rows()).toHaveLength(before);
    expect(selected()).toEqual([rowOf(crate)]);

    const { next, copy } = withCopy(double.snapshot, crate, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    pressDuplicate(rowOf(copy));
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }, { type: 'entity.duplicate', entity: copy }]);
  });

  it('sends one command for two presses in the same task, before the pending state renders', () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    act(() => {
      pressDuplicate(rowOf(crate));
      pressDuplicate(rowOf(crate));
    });
    expect(double.commands).toEqual([{ type: 'entity.duplicate', entity: crate }]);
  });

  it('does nothing more when the hierarchy that asked is gone by the time the engine answers', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    fireEvent.click(screen.getByRole('tab', { name: /^Assets/ }));
    const { next, copy } = withCopy(double.snapshot, crate, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, next);
    await settle();
    expect(message()).toBe('');
    fireEvent.click(screen.getByRole('tab', { name: /^Hierarchy/ }));
    expect(rowOf(copy).getAttribute('aria-selected')).toBe('false');
    expect(selected()).toEqual([rowOf(crate)]);
  });

  it.each([LINKED, SINGLETON])('announces the refusal "%s" in the host’s words and keeps selection and focus on the source', async (refusal) => {
    const double = host();
    open(double.bridge);
    const player = idOf(double.snapshot, 'Player');
    focusRow(player);
    const before = rows().length;
    pressDuplicate(rowOf(player));
    await double.answer({ ok: false, error: { code: 'engine.request', message: refusal } });
    await waitFor(() => expect(message()).toBe(`Duplicate failed: ${refusal}`));
    expect(selected()).toEqual([rowOf(player)]);
    expect(document.activeElement).toBe(rowOf(player));
    expect(rows()).toHaveLength(before);
    expect(tree().hasAttribute('aria-busy')).toBe(false);
  });

  it.each([
    {
      name: 'an unverified completion',
      answer: (double: ReturnType<typeof host>) => double.answer({ ok: false, outcomeUnknown: true, error: { code: 'engine.response', message: UNVERIFIED } }),
      said: `Duplicate result unknown: ${UNVERIFIED}`,
    },
    {
      name: 'a success without the copy, even with a new row',
      answer: (double: ReturnType<typeof host>, crate: Ulid) => double.answer({ ok: true }, withCopy(double.snapshot, crate, 50000).next),
      said: 'Duplicate result unknown: the host reported success but returned no copy.',
    },
    {
      name: 'a rejected dispatch',
      answer: (double: ReturnType<typeof host>) => double.reject(new Error('IPC channel closed')),
      said: 'Duplicate result unknown: IPC channel closed',
    },
    {
      name: 'a copy that is not in the latest snapshot',
      answer: (double: ReturnType<typeof host>) => double.answer({ ok: true, duplicatedEntity: fixtureId(50000) }),
      said: 'Duplicated Crate 01, but the copy is no longer in the hierarchy.',
    },
  ])('never selects a copy after $name, says so, and frees Duplicate', async ({ answer, said }) => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    await answer(double, crate);
    await waitFor(() => expect(message()).toBe(said));
    expect(selected()).toEqual([rowOf(crate)]);
    expect(document.activeElement).toBe(rowOf(crate));
    expect(tree().hasAttribute('aria-busy')).toBe(false);
    pressDuplicate(rowOf(crate));
    expect(double.commands).toHaveLength(2);
  });

  it('does not send a command for an entity that disappeared while its menu was open', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    fireEvent.contextMenu(rowOf(crate), { clientX: 120, clientY: 300 });
    double.publish(without(double.snapshot, crate));
    fireEvent.click(menuItem(/^Duplicate/));
    await waitFor(() => expect(message()).toBe('Duplicate failed: the entity no longer exists.'));
    expect(double.commands).toEqual([]);
    expect(selected()).toEqual([]);
  });

  it('selects nothing when the source disappears and the engine refuses, and the copy when it succeeds', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 02');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    // Another edit removes the source while the command runs.
    const gone = without(double.snapshot, crate);
    double.publish(gone);
    await double.answer({ ok: false, error: { code: 'engine.request', message: 'document revision conflict: expected 4, current 5' } });
    await waitFor(() => expect(message()).toBe('Duplicate failed: document revision conflict: expected 4, current 5'));
    expect(selected()).toEqual([]);
    expect(document.getElementById(`tree-row-${crate}`)).toBeNull();

    const third = idOf(gone, 'Crate 03');
    focusRow(third);
    pressDuplicate(rowOf(third));
    // The copy outlives its source when a later edit removes the source.
    const { next, copy } = withCopy(double.snapshot, third, 50000);
    await double.answer({ ok: true, duplicatedEntity: copy }, without(next, third));
    await waitFor(() => expect(selected()).toEqual([rowOf(copy)]));
  });

  it('does not take ⌘D from typing, dialogs or other panels', async () => {
    const double = host({ mac: true });
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    const filter = screen.getByLabelText<HTMLInputElement>('Filter hierarchy by name');
    fireEvent.change(filter, { target: { value: 'cr' } });
    act(() => filter.focus());
    expect(pressDuplicate(filter, true)).toBe(true);
    expect(filter.value).toBe('cr');
    fireEvent.change(filter, { target: { value: '' } });

    focusRow(crate);
    fireEvent.keyDown(rowOf(crate), { key: 'F2' });
    const rename = screen.getByLabelText<HTMLInputElement>('New name');
    expect(pressDuplicate(rename, true)).toBe(true);
    expect(screen.getByLabelText('New name')).toBe(rename);
    fireEvent.keyDown(rename, { key: 'Escape' });

    for (const region of ['viewport', 'dock', 'inspector'] as const) {
      const panel = document.querySelector<HTMLElement>(`[data-region="${region}"]`)!;
      act(() => panel.focus());
      expect(pressDuplicate(panel, true)).toBe(true);
    }
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    const shortcuts = screen.getByRole('dialog', { name: 'Keyboard shortcuts' });
    expect(pressDuplicate(within(shortcuts).getByRole('searchbox'), true)).toBe(true);
    expect(pressDuplicate(shortcuts, true)).toBe(true);
    await settle();
    expect(double.commands).toEqual([]);
    expect(message()).toBe('');
  });

  it('does not duplicate a scene, and says why', async () => {
    const double = host();
    open(double.bridge);
    const scene = idOf(double.snapshot, 'Dock Prototype');
    focusRow(scene);
    expect(pressDuplicate(rowOf(scene))).toBe(false);
    await waitFor(() => expect(message()).toBe('Scenes cannot be duplicated.'));
    fireEvent.contextMenu(rowOf(scene), { clientX: 100, clientY: 100 });
    expect(menuItem(/^Duplicate/).getAttribute('aria-disabled')).toBe('true');
    expect(menuItem(/^Duplicate/).getAttribute('title')).toBe('Scenes cannot be duplicated.');
    fireEvent.click(menuItem(/^Duplicate/));
    expect(double.commands).toEqual([]);
    expect(selected()).toEqual([rowOf(scene)]);
  });

  it('does not select or announce a copy for a project that is no longer open', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    // Another project opened in this window while the command ran; its tree even has the id.
    const { next, copy } = withCopy(double.snapshot, crate, 50000);
    const other: BridgeSnapshot = { ...next, connection: { status: 'ready', project: { id: fixtureId(49999), name: 'Lighthouse' } } };
    await double.answer({ ok: true, duplicatedEntity: copy }, other);
    await settle();
    expect(rowOf(copy).getAttribute('aria-selected')).toBe('false');
    expect(message()).toBe('');
  });

  it('does not select or announce a copy for a bridge that was replaced', async () => {
    const first = host();
    const view = open(first.bridge);
    const crate = idOf(first.snapshot, 'Crate 01');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    const { next, copy } = withCopy(first.snapshot, crate, 50000);
    // The editor reconnected to the same project through a new bridge, which shows the copy.
    const second = host({ snapshot: next });
    view.rerender(<App resolution={{ kind: 'bridge', bridge: second.bridge }} />);
    await first.answer({ ok: true, duplicatedEntity: copy }, next);
    await settle();
    expect(rowOf(copy).getAttribute('aria-selected')).toBe('false');
    expect(message()).toBe('');
    expect(second.commands).toEqual([]);
  });

  it('leaves a selection made while the command ran, and keeps focus that left the tree', async () => {
    const double = host();
    open(double.bridge);
    const crate = idOf(double.snapshot, 'Crate 01');
    const next02 = idOf(double.snapshot, 'Crate 02');
    focusRow(crate);
    pressDuplicate(rowOf(crate));
    fireEvent.keyDown(rowOf(crate), { key: 'ArrowDown' });
    await waitFor(() => expect(document.activeElement).toBe(rowOf(next02)));
    const first = withCopy(double.snapshot, crate, 50000);
    await double.answer({ ok: true, duplicatedEntity: first.copy }, first.next);
    await waitFor(() => expect(message()).toBe('Duplicated Crate 01'));
    expect(selected()).toEqual([rowOf(next02)]);
    expect(document.activeElement).toBe(rowOf(next02));

    // Focus moved to another panel, the selection did not: the copy is selected there.
    pressDuplicate(rowOf(next02));
    act(() => document.querySelector<HTMLElement>('[data-region="inspector"]')!.focus());
    const second = withCopy(double.snapshot, next02, 50100);
    await double.answer({ ok: true, duplicatedEntity: second.copy }, second.next);
    await waitFor(() => expect(selected()).toEqual([rowOf(second.copy)]));
    expect(document.activeElement).toBe(document.querySelector('[data-region="inspector"]'));
  });
});

/**
 * The same interaction through the REAL NativeBridge (editor/bridge/native.ts), whose
 * Tauri commands a scripted host answers. The host stands in for editor/app/src/entity.rs
 * at its IPC shape only; the copy and its transaction belong to incant_cmd.
 */
const SCHEMAS = Object.fromEntries(
  ['Transform', 'Camera', 'RigidBody', 'Collider', 'Velocity', 'PointLight'].map((name) => [
    name,
    JSON.parse(readFileSync(resolve(__dirname, `../../../../schemas/${name}.schema.json`), 'utf8')),
  ]),
);

function scriptedHost(receipt: (revision: number, copy: string) => Record<string, unknown> | undefined = (revision, copy) => ({ project: IDS.project, entity: copy, revision, transaction: doubleId(901) })) {
  let read: EngineRead = { ...doubleRead('single', SCHEMAS), entity_duplicate: { available: true } };
  const calls: { name: string; args: Record<string, unknown> | undefined }[] = [];
  let serial = 800;
  const invoke: Invoke = async <T,>(name: string, args?: Record<string, unknown>): Promise<T> => {
    calls.push({ name, args });
    switch (name) {
      case 'engine_read':
        return read as T;
      case 'engine_duplicate_subtree': {
        const scene = read.project.scenes[args!.sceneId as string]!;
        const entities = Object.values(scene.entities);
        const ids = new Map<string, string>();
        const visit = (id: string) => {
          ids.set(id, doubleId(serial++));
          entities.filter((entity) => entity.parent === id).forEach((entity) => visit(entity.id));
        };
        visit(args!.entityId as string);
        const copies = Object.fromEntries([...ids].map(([from, to]) => {
          const source = scene.entities[from]!;
          return [to, { ...source, id: to, parent: from === args!.entityId ? source.parent : ids.get(source.parent!)! }];
        }));
        read = {
          ...read,
          revision: read.revision + 1,
          project: { ...read.project, scenes: { ...read.project.scenes, [scene.id]: { ...scene, entities: { ...scene.entities, ...copies } } } },
        };
        return { snapshot: read, receipt: receipt(read.revision, ids.get(args!.entityId as string)!) } as T;
      }
      case 'viewport_bounds':
        return null as T;
      default:
        throw `The scripted host does not implement ${name}.`;
    }
  };
  return { invoke, calls };
}

describe('hierarchy duplicate through the native bridge', () => {
  it('sends the scene, entity and revision, and selects the copy the host returns', async () => {
    const scripted = scriptedHost();
    const native = new NativeBridge(scripted.invoke);
    await native.start();
    expect(native.capabilities).toContain('entity.duplicate');
    open(native);
    focusRow(IDS.rig);
    pressDuplicate(rowOf(IDS.rig));
    await waitFor(() => expect(message()).toBe('Duplicated Camera Rig'));
    expect(scripted.calls.filter((call) => call.name === 'engine_duplicate_subtree').map((call) => call.args)).toEqual([
      { sceneId: IDS.harbor, entityId: IDS.rig, expectedRevision: 41 },
    ]);
    const copy = doubleId(800);
    await waitFor(() => expect(document.activeElement).toBe(rowOf(copy)));
    expect(selected()).toEqual([rowOf(copy)]);
    expect(rowOf(copy).getAttribute('aria-level')).toBe(rowOf(IDS.rig).getAttribute('aria-level'));
  });

  it('reports a completion the bridge cannot verify as unknown and keeps the selection', async () => {
    const scripted = scriptedHost((revision, copy) => ({ project: IDS.project, entity: copy, revision: revision - 1, transaction: doubleId(901) }));
    const native = new NativeBridge(scripted.invoke);
    await native.start();
    open(native);
    focusRow(IDS.crate1);
    pressDuplicate(rowOf(IDS.crate1));
    await waitFor(() => expect(message()).toBe(`Duplicate result unknown: ${UNVERIFIED}`));
    expect(selected()).toEqual([rowOf(IDS.crate1)]);
    expect(document.activeElement).toBe(rowOf(IDS.crate1));
  });

  it('reads an older native host without entity_duplicate as unable to duplicate', async () => {
    const scripted = scriptedHost();
    const native = new NativeBridge(async <T,>(name: string, args?: Record<string, unknown>) => {
      const value = await scripted.invoke<T>(name, args);
      if (name !== 'engine_read') return value;
      const { entity_duplicate: _absent, ...older } = value as EngineRead;
      return older as T;
    });
    await native.start();
    open(native);
    focusRow(IDS.crate1);
    pressDuplicate(rowOf(IDS.crate1));
    await waitFor(() => expect(message()).toBe('Duplicate is unavailable in this editor build (entity.duplicate).'));
    expect(scripted.calls.map((call) => call.name)).not.toContain('engine_duplicate_subtree');
  });

  it('leaves live Play inspection alone: no copy while a world exists, and Duplicate returns after Stop', async () => {
    const double = createNativePlayDouble({ read: doubleRead('single', SCHEMAS), holdStart: true });
    const duplicates: unknown[] = [];
    // Stop waits until the test releases it, so the stopping state can be seen.
    const stop: { release?: () => void } = {};
    const native = new NativeBridge(async <T,>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_duplicate_subtree') {
        duplicates.push(args);
        throw 'The Play test host does not duplicate.';
      }
      if (name === 'play_command' && (args?.command as { op?: string } | undefined)?.op === 'stop') {
        await new Promise<void>((resolve) => {
          stop.release = resolve;
        });
      }
      const value = await double.invoke<T>(name, args);
      return name === 'engine_read' ? ({ ...(value as EngineRead), entity_duplicate: { available: true } } as T) : value;
    });
    await native.startEngineUpdates(double.listen);
    await native.refreshPlay();
    open(native);
    const header = () => document.querySelector<HTMLElement>('.panel--viewport > .panel__header')!;
    fireEvent.click(within(header()).getByRole('button', { name: 'Play' }));
    await act(async () => double.finishStart());
    double.frame();
    await act(async () => {
      await native.refreshPlay();
    });
    await waitFor(() => expect(header().querySelector('.status-pill')?.textContent).toContain('Playing'));

    focusRow(IDS.crate1);
    await waitFor(() => expect(double.commands.some((command) => command.op === 'inspect' && command.entity === IDS.crate1)).toBe(true));
    expect(pressDuplicate(rowOf(IDS.crate1))).toBe(false);
    await waitFor(() => expect(message()).toBe('Stop Play to duplicate.'));
    fireEvent.contextMenu(rowOf(IDS.crate1), { clientX: 100, clientY: 200 });
    expect(menuItem(/^Duplicate/).getAttribute('title')).toBe('Stop Play to duplicate.');
    fireEvent.click(menuItem(/^Duplicate/));
    expect(duplicates).toEqual([]);
    expect(selected()).toEqual([rowOf(IDS.crate1)]);
    expect(document.querySelector('.runtime')).not.toBeNull();

    fireEvent.click(within(header()).getByRole('button', { name: 'Stop' }));
    await waitFor(() => expect(stop.release).toBeDefined());
    focusRow(IDS.crate1);
    pressDuplicate(rowOf(IDS.crate1));
    await waitFor(() => expect(message()).toBe('Wait for Play to stop.'));
    await act(async () => stop.release?.());
    await waitFor(() => expect(header().querySelector('.status-pill')?.textContent).not.toContain('Playing'));
    fireEvent.contextMenu(rowOf(IDS.crate1), { clientX: 100, clientY: 200 });
    expect(menuItem(/^Duplicate/).hasAttribute('aria-disabled')).toBe(false);
    expect(duplicates).toEqual([]);
  });
});
