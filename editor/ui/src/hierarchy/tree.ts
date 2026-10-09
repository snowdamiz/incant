import type { Diagnostic, HierarchyTree, Ulid } from '../bridge/contract';

/** One visible row of the hierarchy. Pure view data derived from the bridge snapshot. */
export interface Row {
  readonly id: Ulid;
  readonly depth: number;
  readonly hasChildren: boolean;
  readonly expanded: boolean;
  /** 1-based position among visible siblings, for aria-posinset. */
  readonly posinset: number;
  readonly setsize: number;
  readonly parent: Ulid | null;
  /** True when this row only shows because a descendant matches the filter. */
  readonly ancestorOfMatch: boolean;
}

/** Rows to render, in order. With a filter, matches and their ancestors show, expanded. */
export function visibleRows(tree: HierarchyTree, expanded: ReadonlySet<string>, filter: string): Row[] {
  const query = filter.trim().toLocaleLowerCase();
  const matches = new Set<string>();
  const keep = new Set<string>();
  if (query) {
    for (const node of Object.values(tree.nodes)) {
      if (node.name.toLocaleLowerCase().includes(query)) {
        matches.add(node.id);
        let current: Ulid | null = node.id;
        while (current !== null && !keep.has(current)) {
          keep.add(current);
          current = tree.nodes[current]?.parent ?? null;
        }
      }
    }
  }
  const rows: Row[] = [];
  const walk = (ids: readonly Ulid[], depth: number, parent: Ulid | null) => {
    const shown = query ? ids.filter((id) => keep.has(id)) : ids;
    shown.forEach((id, index) => {
      const node = tree.nodes[id];
      if (!node) return;
      const childIds = query ? node.children.filter((child) => keep.has(child)) : node.children;
      const hasChildren = childIds.length > 0;
      const isExpanded = hasChildren && (query ? true : expanded.has(id));
      rows.push({
        id,
        depth,
        hasChildren,
        expanded: isExpanded,
        posinset: index + 1,
        setsize: shown.length,
        parent,
        ancestorOfMatch: query !== '' && !matches.has(id),
      });
      if (isExpanded) walk(node.children, depth + 1, id);
    });
  };
  walk(tree.roots, 1, null);
  return rows;
}

/** Initial expansion: everything for small trees, otherwise roots only. */
export function defaultExpanded(tree: HierarchyTree, smallTreeLimit = 200): Set<string> {
  const ids = Object.keys(tree.nodes);
  if (ids.length <= smallTreeLimit) {
    return new Set(ids.filter((id) => (tree.nodes[id]?.children.length ?? 0) > 0));
  }
  return new Set(tree.roots);
}

export interface ProblemCounts {
  readonly errors: number;
  readonly warnings: number;
}

export interface ProblemIndex {
  /** Problems reported on the entity itself. */
  readonly own: ReadonlyMap<string, ProblemCounts>;
  /** Problems anywhere beneath the entity (excluding itself). */
  readonly descendants: ReadonlyMap<string, ProblemCounts>;
}

export function indexProblems(tree: HierarchyTree | null, diagnostics: readonly Diagnostic[]): ProblemIndex {
  const own = new Map<string, ProblemCounts>();
  const descendants = new Map<string, ProblemCounts>();
  const add = (map: Map<string, ProblemCounts>, id: string, d: Diagnostic) => {
    const current = map.get(id) ?? { errors: 0, warnings: 0 };
    map.set(id, {
      errors: current.errors + (d.severity === 'error' ? 1 : 0),
      warnings: current.warnings + (d.severity === 'warning' ? 1 : 0),
    });
  };
  for (const d of diagnostics) {
    if (d.entity === null || d.severity === 'info') continue;
    add(own, d.entity, d);
    let parent = tree?.nodes[d.entity]?.parent ?? null;
    while (parent !== null) {
      add(descendants, parent, d);
      parent = tree?.nodes[parent]?.parent ?? null;
    }
  }
  return { own, descendants };
}

export type TreeKeyResult =
  | { readonly kind: 'move'; readonly to: Ulid }
  | { readonly kind: 'expand'; readonly id: Ulid }
  | { readonly kind: 'collapse'; readonly id: Ulid }
  | { readonly kind: 'none' };

/** The ARIA treeview keyboard model for navigation keys. Rename/delete are handled by the panel. */
export function treeKey(rows: readonly Row[], current: Ulid | null, key: string): TreeKeyResult {
  if (rows.length === 0) return { kind: 'none' };
  const index = current === null ? -1 : rows.findIndex((row) => row.id === current);
  const first = rows[0];
  const last = rows[rows.length - 1];
  if (!first || !last) return { kind: 'none' };
  if (index === -1) {
    return key === 'End' ? { kind: 'move', to: last.id } : { kind: 'move', to: first.id };
  }
  const row = rows[index];
  if (!row) return { kind: 'none' };
  switch (key) {
    case 'ArrowDown': {
      const next = rows[index + 1];
      return next ? { kind: 'move', to: next.id } : { kind: 'none' };
    }
    case 'ArrowUp': {
      const previous = rows[index - 1];
      return previous ? { kind: 'move', to: previous.id } : { kind: 'none' };
    }
    case 'ArrowRight': {
      if (!row.hasChildren) return { kind: 'none' };
      if (!row.expanded) return { kind: 'expand', id: row.id };
      const child = rows[index + 1];
      return child ? { kind: 'move', to: child.id } : { kind: 'none' };
    }
    case 'ArrowLeft': {
      if (row.hasChildren && row.expanded) return { kind: 'collapse', id: row.id };
      return row.parent !== null ? { kind: 'move', to: row.parent } : { kind: 'none' };
    }
    case 'Home':
      return { kind: 'move', to: first.id };
    case 'End':
      return { kind: 'move', to: last.id };
    default:
      return { kind: 'none' };
  }
}

/** Type-ahead: the next row after `current` whose name starts with `prefix`, wrapping. */
export function typeAhead(
  rows: readonly Row[],
  names: (id: Ulid) => string,
  current: Ulid | null,
  prefix: string,
): Ulid | null {
  if (!prefix || rows.length === 0) return null;
  const start = current === null ? 0 : rows.findIndex((row) => row.id === current) + 1;
  const needle = prefix.toLocaleLowerCase();
  for (let offset = 0; offset < rows.length; offset += 1) {
    const row = rows[(start + offset) % rows.length];
    if (row && names(row.id).toLocaleLowerCase().startsWith(needle)) return row.id;
  }
  return null;
}
