import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { HierarchyTree, Ulid } from '../bridge/contract';
import type { ProblemCounts, Row } from '../hierarchy/tree';
import { defaultExpanded, indexProblems, treeKey, typeAhead, visibleRows } from '../hierarchy/tree';
import { Icon, iconForKind } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';
import { Skeleton, StateView } from './StateView';

export function HierarchyPanel() {
  const { snapshot, bridge } = useShell();
  const hierarchy = snapshot?.hierarchy;
  let body;
  if (!snapshot) {
    body = (
      <StateView icon="plug" title="No engine connected">
        <p>The hierarchy appears when the editor process attaches a project.</p>
      </StateView>
    );
  } else if (!hierarchy || hierarchy.status === 'loading') {
    body = <Skeleton label="Loading hierarchy" />;
  } else if (hierarchy.status === 'error') {
    body = (
      <StateView icon="error" tone="error" title="The hierarchy could not be loaded">
        <p>{hierarchy.error.message}</p>
        <p className="mono subtle">{hierarchy.error.code}</p>
      </StateView>
    );
  } else if (hierarchy.value.roots.length === 0) {
    body = (
      <StateView icon="scene" title="This project has no entities">
        <p>Entities created by any client, including the agent, appear here.</p>
      </StateView>
    );
  } else {
    body = <Tree tree={hierarchy.value} key={bridge?.label ?? 'tree'} />;
  }
  const count = hierarchy?.status === 'ready' ? Object.keys(hierarchy.value.nodes).length : null;
  return (
    <section className="panel" data-region="hierarchy" aria-labelledby="hierarchy-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="hierarchy-title" className="panel__title">
          Hierarchy
        </h2>
        {count !== null ? (
          <span className="panel__count" title={`${count.toLocaleString()} entities`}>
            {count.toLocaleString()}
            <span className="visually-hidden"> entities</span>
          </span>
        ) : null}
      </header>
      {body}
    </section>
  );
}

function Tree({ tree }: { tree: HierarchyTree }) {
  const { snapshot, selection, select, capabilities, explainUnavailable, run, announce } = useShell();
  const [expanded, setExpanded] = useState<Set<string>>(() => defaultExpanded(tree));
  const [filter, setFilter] = useState('');
  const [renaming, setRenaming] = useState<Ulid | null>(null);
  const pendingFocus = useRef<Ulid | null>(null);
  const typed = useRef({ text: '', at: 0 });
  const listRef = useRef<HTMLDivElement>(null);
  const filterRef = useRef<HTMLInputElement>(null);

  const rows = useMemo(() => visibleRows(tree, expanded, filter), [tree, expanded, filter]);
  const problems = useMemo(() => indexProblems(tree, snapshot?.diagnostics ?? []), [tree, snapshot?.diagnostics]);
  const active = selection !== null && rows.some((row) => row.id === selection) ? selection : (rows[0]?.id ?? null);
  const name = (id: Ulid) => tree.nodes[id]?.name ?? '';

  // Reveal a selection made elsewhere (e.g. the Problems list) by expanding its ancestors.
  // Runs only when the selection changes, so collapsing an ancestor afterwards sticks.
  const revealed = useRef<Ulid | null>(null);
  useEffect(() => {
    if (selection === null || revealed.current === selection) return;
    revealed.current = selection;
    const ancestors: string[] = [];
    let parent = tree.nodes[selection]?.parent ?? null;
    while (parent !== null) {
      ancestors.push(parent);
      parent = tree.nodes[parent]?.parent ?? null;
    }
    if (ancestors.length === 0) return;
    setExpanded((current) => (ancestors.every((id) => current.has(id)) ? current : new Set([...current, ...ancestors])));
  }, [selection, tree]);

  useEffect(() => {
    if (selection === null) return;
    const element = document.getElementById(rowDomId(selection));
    element?.scrollIntoView({ block: 'nearest' });
    if (pendingFocus.current === selection) {
      pendingFocus.current = null;
      element?.focus();
    }
  }, [selection, rows]);

  const moveTo = (id: Ulid) => {
    pendingFocus.current = id;
    select(id);
    if (id === selection) document.getElementById(rowDomId(id))?.focus();
  };

  const toggle = (id: string, open: boolean) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (open) next.add(id);
      else next.delete(id);
      return next;
    });

  const beginRename = (id: Ulid) => {
    if (!capabilities.has('entity.rename')) explainUnavailable('entity.rename');
    else setRenaming(id);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (renaming !== null || event.target instanceof HTMLInputElement) return;
    const mod = event.metaKey || event.ctrlKey;
    if (mod && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      filterRef.current?.focus();
      return;
    }
    if (event.key === 'F2' && active !== null) {
      event.preventDefault();
      beginRename(active);
      return;
    }
    if ((event.key === 'Delete' || (event.key === 'Backspace' && mod)) && active !== null) {
      event.preventDefault();
      void run({ type: 'entity.delete', entity: active });
      return;
    }
    if (event.key === 'Enter' && active !== null) {
      event.preventDefault();
      select(active);
      return;
    }
    const result = treeKey(rows, active, event.key);
    if (result.kind !== 'none') {
      event.preventDefault();
      if (result.kind === 'move') moveTo(result.to);
      else toggle(result.id, result.kind === 'expand');
      return;
    }
    if (event.key.length === 1 && !mod && !event.altKey && event.key !== ' ') {
      const now = event.timeStamp;
      typed.current = { text: now - typed.current.at < 700 ? typed.current.text + event.key : event.key, at: now };
      const from = typed.current.text.length > 1 ? rows[rows.findIndex((row) => row.id === active) - 1]?.id ?? null : active;
      const match = typeAhead(rows, name, from, typed.current.text);
      if (match) moveTo(match);
    }
  };

  const onFilterKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      setFilter('');
      const target = active;
      if (target) moveTo(target);
    } else if (event.key === 'ArrowDown' && rows[0]) {
      event.preventDefault();
      moveTo(active ?? rows[0].id);
    }
  };

  const matchCount = filter.trim() ? rows.filter((row) => !row.ancestorOfMatch).length : null;

  return (
    <>
      <div className="panel__toolbar">
        <div className="search-field">
          <Icon name="search" />
          <label htmlFor="hierarchy-filter" className="visually-hidden">
            Filter hierarchy by name
          </label>
          <input
            id="hierarchy-filter"
            ref={filterRef}
            type="search"
            placeholder="Filter"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            onKeyDown={onFilterKey}
            aria-controls="hierarchy-tree"
            aria-describedby="hierarchy-filter-status"
          />
          <span id="hierarchy-filter-status" className="search-field__status" role="status">
            {matchCount === null ? '' : `${matchCount} ${matchCount === 1 ? 'match' : 'matches'}`}
          </span>
        </div>
        <button
          type="button"
          className="tool-button"
          aria-label="Collapse all"
          title="Collapse all"
          onClick={() => setExpanded(new Set(tree.roots))}
        >
          <Icon name="collapseAll" />
        </button>
      </div>
      {rows.length === 0 ? (
        <StateView icon="search" title="No matching entities" compact>
          <p>Nothing named “{filter.trim()}”. Press Escape to clear the filter.</p>
        </StateView>
      ) : (
        <div
          ref={listRef}
          id="hierarchy-tree"
          className="tree"
          role="tree"
          aria-labelledby="hierarchy-title"
          onKeyDown={onKeyDown}
        >
          {rows.map((row) => (
            <TreeRow
              key={row.id}
              row={row}
              tree={tree}
              active={row.id === active}
              selected={row.id === selection}
              own={problems.own.get(row.id)}
              hidden={!row.expanded ? problems.descendants.get(row.id) : undefined}
              renaming={renaming === row.id}
              onActivate={() => moveTo(row.id)}
              onToggle={() => toggle(row.id, !row.expanded)}
              onRename={() => beginRename(row.id)}
              onRenameDone={async (value) => {
                setRenaming(null);
                const trimmed = value.trim();
                if (trimmed && trimmed !== name(row.id)) {
                  const ok = await run({ type: 'entity.rename', entity: row.id, name: trimmed });
                  if (ok) announce(`Renamed to ${trimmed}`);
                }
                moveTo(row.id);
              }}
            />
          ))}
        </div>
      )}
    </>
  );
}

export function rowDomId(id: string) {
  return `tree-row-${id}`;
}

function problemText(counts: ProblemCounts | undefined) {
  if (!counts) return '';
  const parts = [];
  if (counts.errors) parts.push(`${counts.errors} ${counts.errors === 1 ? 'error' : 'errors'}`);
  if (counts.warnings) parts.push(`${counts.warnings} ${counts.warnings === 1 ? 'warning' : 'warnings'}`);
  return parts.join(', ');
}

function TreeRow({
  row,
  tree,
  active,
  selected,
  own,
  hidden,
  renaming,
  onActivate,
  onToggle,
  onRename,
  onRenameDone,
}: {
  row: Row;
  tree: HierarchyTree;
  active: boolean;
  selected: boolean;
  own: ProblemCounts | undefined;
  hidden: ProblemCounts | undefined;
  renaming: boolean;
  onActivate: () => void;
  onToggle: () => void;
  onRename: () => void;
  onRenameDone: (value: string) => void;
}) {
  const node = tree.nodes[row.id];
  if (!node) return null;
  const ownText = problemText(own);
  const hiddenText = problemText(hidden);
  return (
    <div
      id={rowDomId(row.id)}
      role="treeitem"
      className={`tree-row${selected ? ' is-selected' : ''}${row.ancestorOfMatch ? ' is-context' : ''}`}
      aria-level={row.depth}
      aria-posinset={row.posinset}
      aria-setsize={row.setsize}
      aria-expanded={row.hasChildren ? row.expanded : undefined}
      aria-selected={selected}
      tabIndex={active ? 0 : -1}
      data-focus-target={active ? '' : undefined}
      style={{ paddingInlineStart: `calc(${row.depth - 1} * var(--indent) + var(--space-2))` }}
      data-depth={row.depth}
      onClick={onActivate}
      onDoubleClick={onRename}
    >
      {row.depth > 1 ? (
        <span className="tree-row__guides" aria-hidden="true" style={{ width: `calc(${row.depth - 1} * var(--indent))` }} />
      ) : null}
      <span
        className={`tree-row__twisty${row.hasChildren ? '' : ' is-leaf'}${row.expanded ? ' is-open' : ''}`}
        aria-hidden="true"
        onClick={(event) => {
          if (!row.hasChildren) return;
          event.stopPropagation();
          onToggle();
        }}
      >
        {row.hasChildren ? <Icon name="chevron" size={12} /> : null}
      </span>
      <Icon name={iconForKind(node.kind)} className={`tree-row__kind kind--${node.kind}`} />
      {renaming ? (
        <RenameInput initial={node.name} onDone={onRenameDone} />
      ) : (
        <span className="tree-row__name" title={node.name}>
          {node.name}
        </span>
      )}
      {own?.errors ? (
        <span className="badge badge--error" aria-hidden="true">
          <Icon name="error" size={12} />
          {own.errors}
        </span>
      ) : null}
      {own?.warnings ? (
        <span className="badge badge--warning" aria-hidden="true">
          <Icon name="warning" size={12} />
          {own.warnings}
        </span>
      ) : null}
      {hidden && (hidden.errors || hidden.warnings) ? (
        <span
          className={`tree-row__nested ${hidden.errors ? 'is-error' : 'is-warning'}`}
          aria-hidden="true"
          title={`Contains ${hiddenText}`}
        />
      ) : null}
      {ownText ? <span className="visually-hidden">, {ownText}</span> : null}
      {hiddenText ? <span className="visually-hidden">, contains {hiddenText}</span> : null}
    </div>
  );
}

function RenameInput({ initial, onDone }: { initial: string; onDone: (value: string) => void }) {
  const [value, setValue] = useState(initial);
  const done = useRef(false);
  const finish = (next: string) => {
    if (done.current) return;
    done.current = true;
    onDone(next);
  };
  return (
    <input
      className="tree-row__rename"
      aria-label="New name"
      autoFocus
      value={value}
      onChange={(event) => setValue(event.target.value)}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === 'Enter') finish(value);
        if (event.key === 'Escape') finish(initial);
      }}
      onBlur={() => finish(initial)}
    />
  );
}
