import { describe, expect, it } from 'vitest';
import type { Diagnostic, HierarchyTree, Ulid } from '../bridge/contract';
import { defaultExpanded, indexProblems, treeKey, typeAhead, visibleRows } from './tree';

const id = (s: string) => s as Ulid;
//  root
//  ├─ a
//  │  ├─ a1
//  │  └─ a2
//  └─ b
const tree: HierarchyTree = {
  roots: [id('root')],
  nodes: {
    root: { id: id('root'), name: 'Root', kind: 'scene', parent: null, children: [id('a'), id('b')] },
    a: { id: id('a'), name: 'Alpha', kind: 'group', parent: id('root'), children: [id('a1'), id('a2')] },
    a1: { id: id('a1'), name: 'Apple', kind: 'mesh', parent: id('a'), children: [] },
    a2: { id: id('a2'), name: 'Banana', kind: 'mesh', parent: id('a'), children: [] },
    b: { id: id('b'), name: 'Beta', kind: 'light', parent: id('root'), children: [] },
  },
};
const names = (i: Ulid) => tree.nodes[i]!.name;

describe('visibleRows', () => {
  it('shows expanded nodes in document order with ARIA positions', () => {
    const rows = visibleRows(tree, defaultExpanded(tree), '');
    expect(rows.map((r) => r.id)).toEqual(['root', 'a', 'a1', 'a2', 'b']);
    expect(rows[1]).toMatchObject({ depth: 2, posinset: 1, setsize: 2, hasChildren: true, expanded: true });
    expect(rows[4]).toMatchObject({ depth: 2, posinset: 2, setsize: 2 });
  });

  it('hides children of collapsed nodes', () => {
    const rows = visibleRows(tree, new Set(['root']), '');
    expect(rows.map((r) => r.id)).toEqual(['root', 'a', 'b']);
  });

  it('filters by name, keeping ancestors as context and expanding them', () => {
    const rows = visibleRows(tree, new Set(), 'ban');
    expect(rows.map((r) => r.id)).toEqual(['root', 'a', 'a2']);
    expect(rows.map((r) => r.ancestorOfMatch)).toEqual([true, true, false]);
    expect(rows[2]).toMatchObject({ posinset: 1, setsize: 1 });
  });

  it('returns nothing when the filter matches nothing', () => {
    expect(visibleRows(tree, defaultExpanded(tree), 'zzz')).toEqual([]);
  });
});

describe('treeKey (ARIA tree keyboard model)', () => {
  const rows = visibleRows(tree, defaultExpanded(tree), '');
  it('moves down and up', () => {
    expect(treeKey(rows, id('a'), 'ArrowDown')).toEqual({ kind: 'move', to: 'a1' });
    expect(treeKey(rows, id('a'), 'ArrowUp')).toEqual({ kind: 'move', to: 'root' });
    expect(treeKey(rows, id('b'), 'ArrowDown')).toEqual({ kind: 'none' });
  });
  it('collapses an open parent, then moves to the parent from a child', () => {
    expect(treeKey(rows, id('a'), 'ArrowLeft')).toEqual({ kind: 'collapse', id: 'a' });
    expect(treeKey(rows, id('a1'), 'ArrowLeft')).toEqual({ kind: 'move', to: 'a' });
  });
  it('expands a closed parent, then moves into it', () => {
    const collapsed = visibleRows(tree, new Set(['root']), '');
    expect(treeKey(collapsed, id('a'), 'ArrowRight')).toEqual({ kind: 'expand', id: 'a' });
    expect(treeKey(rows, id('a'), 'ArrowRight')).toEqual({ kind: 'move', to: 'a1' });
    expect(treeKey(rows, id('a1'), 'ArrowRight')).toEqual({ kind: 'none' });
  });
  it('jumps with Home and End', () => {
    expect(treeKey(rows, id('a1'), 'Home')).toEqual({ kind: 'move', to: 'root' });
    expect(treeKey(rows, id('a1'), 'End')).toEqual({ kind: 'move', to: 'b' });
  });
});

describe('typeAhead', () => {
  const rows = visibleRows(tree, defaultExpanded(tree), '');
  it('finds the next row starting with the prefix, wrapping', () => {
    expect(typeAhead(rows, names, id('root'), 'a')).toBe('a');
    expect(typeAhead(rows, names, id('a'), 'a')).toBe('a1');
    expect(typeAhead(rows, names, id('a1'), 'a')).toBe('a');
    expect(typeAhead(rows, names, null, 'be')).toBe('b');
    expect(typeAhead(rows, names, null, 'q')).toBeNull();
  });
});

describe('indexProblems', () => {
  const diag = (entity: string | null, severity: Diagnostic['severity']): Diagnostic => ({
    id: `${entity}-${severity}`,
    severity,
    message: 'x',
    entity: entity as Ulid | null,
    component: null,
    path: null,
  });
  it('counts own problems and rolls them up to ancestors, ignoring info', () => {
    const index = indexProblems(tree, [diag('a1', 'error'), diag('a2', 'warning'), diag('b', 'info'), diag(null, 'error')]);
    expect(index.own.get('a1')).toEqual({ errors: 1, warnings: 0 });
    expect(index.descendants.get('a')).toEqual({ errors: 1, warnings: 1 });
    expect(index.descendants.get('root')).toEqual({ errors: 1, warnings: 1 });
    expect(index.own.has('b')).toBe(false);
  });
});
