import { useDeferredValue, useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import type { HierarchyTree, Ulid } from '../bridge/contract';
import type { ProblemCounts, Row } from '../hierarchy/tree';
import { defaultExpanded, indexProblems, treeKey, typeAhead, visibleRows } from '../hierarchy/tree';
import { Icon, iconForKind } from '../icons/Icon';
import { projectState, sameError } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';
import { mountedIndices, revealSpan, rowRange, useViewport } from '../shell/windowing';
import { Skeleton, StateView } from './StateView';
import { AssetBrowser } from './assets/AssetBrowser';
import { MatchCount } from './MatchCount';
import { ContextMenu } from './Menu';
import { unavailableMessage } from '../bridge/resolve';
import { COPY_ID_UNAVAILABLE, canWriteClipboard, copyText } from '../shell/clipboard';
import { useAssets } from '../assets/AssetsContext';
import type { NavigatorView } from '../assets/AssetsContext';

export function HierarchyPanel() {
  const { snapshot, bridge } = useShell();
  const { view } = useAssets();
  const hierarchy = snapshot?.hierarchy;
  const project = projectState(snapshot);
  let body;
  if (!snapshot) {
    body = (
      <StateView icon="plug" title="No engine connected">
        <p>The hierarchy appears when the editor process attaches a project.</p>
      </StateView>
    );
  } else if (!hierarchy || hierarchy.status === 'loading') {
    body = <Skeleton label="Loading hierarchy" />;
  } else if (hierarchy.status === 'error' && project.kind === 'failed' && sameError(hierarchy.error, project.error)) {
    // The banner carries the full failure; this panel only says what it cannot show.
    body = (
      <StateView icon="error" tone="error" alert={false} title="No project loaded">
        <p>Entities appear here once the project opens.</p>
      </StateView>
    );
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
  const count = useMemo(() => (hierarchy?.status === 'ready' ? Object.keys(hierarchy.value.nodes).length : null), [hierarchy]);
  return (
    <section className="panel" data-region="hierarchy" aria-label="Hierarchy and assets" tabIndex={-1}>
      <NavigatorTabs entityCount={count} />
      <div
        className="navigator__panel"
        role="tabpanel"
        id={`navigator-panel-${view}`}
        aria-labelledby={`navigator-tab-${view}`}
      >
        {view === 'assets' ? <AssetBrowser /> : body}
      </div>
    </section>
  );
}

const VIEWS: readonly NavigatorView[] = ['hierarchy', 'assets'];

/**
 * The left column shows either the scene hierarchy or the project's assets. Both are
 * "pick something, inspect it on the right" lists, so they share one column and one
 * Inspector instead of putting a browser in the output dock.
 */
function NavigatorTabs({ entityCount }: { entityCount: number | null }) {
  const { snapshot } = useShell();
  const { view, setView, pending, outcome } = useAssets();
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});
  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value.length : null;
  const select = (next: NavigatorView, focus: boolean) => {
    setView(next);
    if (focus) refs.current[next]?.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = VIEWS.indexOf(view);
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % VIEWS.length;
    else if (event.key === 'ArrowLeft') next = (index - 1 + VIEWS.length) % VIEWS.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = VIEWS.length - 1;
    else return;
    event.preventDefault();
    select(VIEWS[next]!, true);
  };
  const tab = (id: NavigatorView, label: ReactNode, extra: ReactNode) => (
    <button
      ref={(element) => {
        refs.current[id] = element;
      }}
      type="button"
      role="tab"
      id={`navigator-tab-${id}`}
      className="navigator__tab"
      aria-selected={view === id}
      aria-controls={`navigator-panel-${id}`}
      tabIndex={view === id ? 0 : -1}
      onClick={() => select(id, false)}
    >
      {label}
      {extra}
    </button>
  );
  return (
    <header className="panel__header navigator__header">
      <div className="navigator__tabs" role="tablist" aria-label="Left panel" onKeyDown={onKeyDown}>
        {tab(
          'hierarchy',
          <span id="hierarchy-title">Hierarchy</span>,
          entityCount !== null ? (
            <span className="panel__count">
              {entityCount.toLocaleString()}
              <span className="visually-hidden"> entities</span>
            </span>
          ) : null,
        )}
        {tab(
          'assets',
          <span id="assets-title">Assets</span>,
          pending ? (
            <span className="navigator__busy" title="Import running">
              <span className="spinner spinner--small" aria-hidden="true" />
              <span className="visually-hidden">, importing</span>
            </span>
          ) : outcome?.status === 'failure' ? (
            <span className="navigator__alert" title="The last import failed">
              <span className="visually-hidden">, last import failed</span>
            </span>
          ) : assets !== null ? (
            <span className="panel__count">
              {assets.toLocaleString()}
              <span className="visually-hidden"> assets</span>
            </span>
          ) : null,
        )}
      </div>
    </header>
  );
}

/** Height of one tree row (--row-height) and the tree's top padding (--space-1). */
const ROW = 28;
const TOP = 2;

/**
 * The scene tree. Rows are a flat ARIA tree (level, position and set size on each
 * row), so it can mount only the rows near the scroll position: a 100,000-entity
 * project mounts a few dozen rows. The active row (the roving Tab stop) and a row
 * being renamed always stay mounted, so focus, keyboard moves and rename survive
 * scrolling them out of view.
 */
function Tree({ tree }: { tree: HierarchyTree }) {
  const { snapshot, bridge, selection, select, capabilities, explainUnavailable, run, announce } = useShell();
  const [menu, setMenu] = useState<{ id: Ulid; at: { x: number; y: number } } | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(() => defaultExpanded(tree));
  const [filter, setFilter] = useState('');
  // Typing stays responsive in a large tree: the filter applies at a lower priority.
  const query = useDeferredValue(filter);
  const [renaming, setRenaming] = useState<Ulid | null>(null);
  const pendingFocus = useRef<Ulid | null>(null);
  const revealPending = useRef<Ulid | null>(null);
  const typed = useRef({ text: '', at: 0 });
  const listRef = useRef<HTMLDivElement>(null);
  const filterRef = useRef<HTMLInputElement>(null);

  const rows = useMemo(() => visibleRows(tree, expanded, query), [tree, expanded, query]);
  const indexOf = useMemo(() => new Map(rows.map((row, index) => [row.id as string, index])), [rows]);
  const problems = useMemo(() => indexProblems(tree, snapshot?.diagnostics ?? []), [tree, snapshot?.diagnostics]);
  const active = selection !== null && indexOf.has(selection) ? selection : (rows[0]?.id ?? null);
  const name = (id: Ulid) => tree.nodes[id]?.name ?? '';
  const [viewport, attachViewport, remeasure] = useViewport(listRef, ROW * 4);
  const [first, last] = rowRange(viewport, rows.length, ROW, 8, TOP);
  const mounted = mountedIndices(first, last, rows.length, [
    active !== null ? (indexOf.get(active) ?? -1) : -1,
    renaming !== null ? (indexOf.get(renaming) ?? -1) : -1,
  ]);

  // Reveal a selection made elsewhere (e.g. the Problems list) by expanding its ancestors.
  // Runs only when the selection changes, so collapsing an ancestor afterwards sticks.
  const revealed = useRef<Ulid | null>(null);
  useEffect(() => {
    if (selection === null || revealed.current === selection) return;
    revealed.current = selection;
    revealPending.current = selection;
    const ancestors: string[] = [];
    let parent = tree.nodes[selection]?.parent ?? null;
    while (parent !== null) {
      ancestors.push(parent);
      parent = tree.nodes[parent]?.parent ?? null;
    }
    if (ancestors.length === 0) return;
    setExpanded((current) => (ancestors.every((id) => current.has(id)) ? current : new Set([...current, ...ancestors])));
  }, [selection, tree]);

  // Once a new selection's row exists, scroll it into view (it is always mounted) and focus it if asked.
  useEffect(() => {
    if (selection === null) return;
    const index = indexOf.get(selection);
    if (index === undefined) return;
    if (revealPending.current === selection || pendingFocus.current === selection) {
      revealPending.current = null;
      revealSpan(listRef.current, TOP + index * ROW, ROW, 4);
      remeasure();
    }
    if (pendingFocus.current === selection) {
      pendingFocus.current = null;
      document.getElementById(rowDomId(selection))?.focus();
    }
  }, [selection, indexOf]);

  const moveTo = (id: Ulid) => {
    pendingFocus.current = id;
    select(id);
    if (id === selection) {
      const index = indexOf.get(id);
      if (index !== undefined) {
        revealSpan(listRef.current, TOP + index * ROW, ROW, 4);
        remeasure();
      }
      document.getElementById(rowDomId(id))?.focus();
    }
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
    if ((event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) && active !== null) {
      event.preventDefault();
      const rect = document.getElementById(rowDomId(active))?.getBoundingClientRect();
      setMenu({ id: active, at: { x: (rect?.left ?? 0) + 28, y: (rect?.bottom ?? 0) + 2 } });
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
    if ((event.key === 'PageDown' || event.key === 'PageUp') && active !== null) {
      // A viewport of rows at a time, like a native list.
      event.preventDefault();
      const page = Math.max(1, Math.floor((listRef.current?.clientHeight || viewport.height) / ROW) - 1);
      const index = indexOf.get(active) ?? 0;
      const target = rows[Math.max(0, Math.min(rows.length - 1, index + (event.key === 'PageDown' ? page : -page)))];
      if (target) moveTo(target.id);
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
      const from = typed.current.text.length > 1 ? rows[(indexOf.get(active ?? '') ?? 0) - 1]?.id ?? null : active;
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

  const matchCount = query.trim() ? rows.filter((row) => !row.ancestorOfMatch).length : null;

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
            {matchCount === null ? '' : <MatchCount count={matchCount} />}
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
          <p>Nothing named “{query.trim()}”. Press Escape to clear the filter.</p>
        </StateView>
      ) : (
        <div
          ref={attachViewport}
          id="hierarchy-tree"
          className="tree"
          role="tree"
          aria-labelledby="hierarchy-title"
          onKeyDown={onKeyDown}
        >
          <div className="tree__canvas" role="none" style={{ height: TOP + rows.length * ROW }}>
            {mounted.map((index) => {
              const row = rows[index]!;
              return (
                <TreeRow
                  key={row.id}
                  row={row}
                  top={TOP + index * ROW}
                  tree={tree}
                  active={row.id === active}
                  selected={row.id === selection}
                  own={problems.own.get(row.id)}
                  hidden={!row.expanded ? problems.descendants.get(row.id) : undefined}
                  renaming={renaming === row.id}
                  onActivate={() => moveTo(row.id)}
                  onToggle={() => toggle(row.id, !row.expanded)}
                  onRename={() => beginRename(row.id)}
                  onMenu={(at) => {
                    // Select without moving focus: the menu takes focus, and returns it on close.
                    select(row.id);
                    setMenu({ id: row.id, at });
                  }}
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
              );
            })}
          </div>
        </div>
      )}
      {menu ? (
        <ContextMenu
          label={`Actions for ${name(menu.id)}`}
          at={menu.at}
          onClose={(restore) => {
            const id = menu.id;
            setMenu(null);
            if (restore) document.getElementById(rowDomId(id))?.focus();
          }}
          onUnavailable={announce}
          items={[
            {
              id: 'rename',
              label: 'Rename',
              icon: 'pencil',
              shortcut: 'F2',
              unavailable: capabilities.has('entity.rename') ? undefined : unavailableMessage('entity.rename', bridge),
              onSelect: () => beginRename(menu.id),
            },
            {
              id: 'copy-id',
              label: 'Copy ID',
              icon: 'copy',
              // Without a clipboard the item says so instead of doing nothing.
              unavailable: canWriteClipboard() ? undefined : COPY_ID_UNAVAILABLE,
              onSelect: () => void copyText(menu.id, 'Entity ID', announce),
            },
            {
              id: 'delete',
              label: 'Delete',
              icon: 'trash',
              shortcut: 'Del',
              danger: true,
              unavailable: capabilities.has('entity.delete') ? undefined : unavailableMessage('entity.delete', bridge),
              onSelect: () => void run({ type: 'entity.delete', entity: menu.id }),
            },
          ]}
        />
      ) : null}
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
  top,
  tree,
  active,
  selected,
  own,
  hidden,
  renaming,
  onActivate,
  onToggle,
  onRename,
  onMenu,
  onRenameDone,
}: {
  row: Row;
  top: number;
  tree: HierarchyTree;
  active: boolean;
  selected: boolean;
  own: ProblemCounts | undefined;
  hidden: ProblemCounts | undefined;
  renaming: boolean;
  onActivate: () => void;
  onToggle: () => void;
  onRename: () => void;
  /** Opens the row's context menu at viewport coordinates. */
  onMenu: (at: { x: number; y: number }) => void;
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
      className={`tree-row${selected ? ' is-selected' : ''}${row.ancestorOfMatch ? ' is-context' : ''}${node.kind === 'scene' && row.depth === 1 ? ' is-scene' : ''}`}
      aria-level={row.depth}
      aria-posinset={row.posinset}
      aria-setsize={row.setsize}
      aria-expanded={row.hasChildren ? row.expanded : undefined}
      aria-selected={selected}
      tabIndex={active ? 0 : -1}
      data-focus-target={active ? '' : undefined}
      style={{ top, paddingInlineStart: `calc(${row.depth - 1} * var(--indent) + var(--space-2))` }}
      data-depth={row.depth}
      onClick={onActivate}
      onDoubleClick={onRename}
      onContextMenu={(event) => {
        event.preventDefault();
        onMenu({ x: event.clientX, y: event.clientY });
      }}
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
      // Start with the whole name selected, so typing replaces it (as in Finder and code editors).
      onFocus={(event) => event.currentTarget.select()}
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
