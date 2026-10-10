import { useEffect, useMemo, useRef } from 'react';
import type { CSSProperties } from 'react';
import type { KeyboardEvent } from 'react';
import type { ProjectAsset, Ulid } from '../../bridge/contract';
import { useAssets } from '../../assets/AssetsContext';
import { assetRowId, countFiles, folderLabel, groupByFolder, importAvailability, matchesFilter } from '../../assets/library';
import { ACCEPTED_FORMATS, USAGE_LABEL, isConflict, kindLabel } from '../../assets/paths';
import { Icon, iconForKind } from '../../icons/Icon';
import { projectState, sameError } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { indexAt, revealSpan, useViewport } from '../../shell/windowing';
import { Skeleton, StateView } from '../StateView';
import { focusInspectorTarget } from './AssetInspector';
import { MatchCount } from '../MatchCount';

/**
 * The Assets view of the left column. It lists what the project has imported, grouped by
 * source folder. Details, interpretation, reimport and import all open in the Inspector,
 * the same place entity details live, so this column stays a calm list.
 */
export function AssetBrowser() {
  const { snapshot } = useShell();
  const assets = snapshot?.assets;
  const project = projectState(snapshot);

  let body;
  if (!snapshot) {
    body = (
      <StateView icon="plug" title="No engine connected" compact />
    );
  } else if (!assets) {
    body = (
      <StateView icon="texture" title="Assets are not available" compact />
    );
  } else if (assets.status === 'loading' || project.kind === 'loading') {
    body = <Skeleton rows={6} label="Loading assets" />;
  } else if (assets.status === 'error' && project.kind === 'failed' && sameError(assets.error, project.error)) {
    body = (
      <StateView icon="texture" title="No project loaded" compact />
    );
  } else if (assets.status === 'error') {
    body = (
      <StateView icon="error" tone="error" title="Assets could not be listed" compact>
        <p>{assets.error.message}</p>
        <p className="mono subtle">{assets.error.code}</p>
      </StateView>
    );
  } else {
    body = <Library assets={assets.value} />;
  }
  return <div className="asset-browser">{body}</div>;
}

function ImportButton({ label }: { label: string }) {
  const { snapshot, bridge, capabilities, showPanel } = useShell();
  const { pane, setPane } = useAssets();
  const availability = importAvailability(snapshot, bridge, capabilities);
  return (
    <button
      type="button"
      className={`button button--small asset-browser__import${pane === 'import' ? ' is-current' : ''}`}
      // aria-disabled keeps it focusable: activating it opens the Inspector with the reason.
      aria-disabled={!availability.ok || undefined}
      aria-pressed={pane === 'import'}
      title={availability.ok ? 'Import files from the project folder' : availability.reason}
      onClick={() => {
        showPanel('inspector');
        setPane('import');
        focusInspectorTarget('import');
      }}
    >
      <Icon name="import" size={14} />
      {label}
    </button>
  );
}

/** One line under the toolbar while an import runs or after it fails, wherever the Inspector is. */
function ImportStatus() {
  const { pending, outcome, pane, setPane, selected, setSelected } = useAssets();
  const { panels, showPanel } = useShell();
  if (pending) {
    return (
      <div className="asset-browser__status" role="status">
        <span className="spinner spinner--small" aria-hidden="true" />
        <span className="asset-browser__status-text">
          {pending.kind === 'reimport' ? `Reimporting ${pending.name ?? pending.sources[0]?.source ?? ''}…` : `Importing ${countFiles(pending.sources.length)}…`}
        </span>
      </div>
    );
  }
  if (outcome?.status !== 'failure') return null;
  const target = outcome.job.kind === 'import' ? 'import' : 'details';
  const showing = panels.inspector && pane === target && (target === 'import' || selected === outcome.job.asset);
  return (
    <div className="asset-browser__status asset-browser__status--error">
      <Icon name="error" size={14} />
      <span className="asset-browser__status-text">
        {isConflict(outcome.error.message, outcome.error.code) ? 'Project changed; nothing imported' : 'Last import failed'}
      </span>
      {showing ? null : (
        <button
          type="button"
          className="asset-browser__status-action"
          onClick={() => {
            if (target === 'details' && outcome.job.asset) setSelected(outcome.job.asset);
            showPanel('inspector');
            setPane(target);
            focusInspectorTarget(target);
          }}
        >
          Show
        </button>
      )}
    </div>
  );
}

function Library({ assets }: { assets: readonly ProjectAsset[] }) {
  const { snapshot, bridge, capabilities, showPanel } = useShell();
  const { filter, setFilter, selected, setSelected, setPane, recent } = useAssets();
  const filterRef = useRef<HTMLInputElement>(null);
  const visible = useMemo(() => assets.filter((asset) => matchesFilter(asset, filter)), [assets, filter]);
  const groups = useMemo(() => groupByFolder(visible), [visible]);
  const order = useMemo(() => groups.flatMap((group) => group.assets), [groups]);
  const active: Ulid | null = selected !== null && order.some((asset) => asset.id === selected) ? selected : (order[0]?.id ?? null);
  const availability = importAvailability(snapshot, bridge, capabilities);

  if (assets.length === 0) {
    return (
      <>
        <ImportStatus />
        <StateView icon="texture" title="No assets yet" compact action={<ImportButton label="Import assets" />}>
          <p>{availability.ok ? `Import ${ACCEPTED_FORMATS} files that are already in the project folder.` : availability.reason}</p>
        </StateView>
      </>
    );
  }

  const open = (id: Ulid, focus: boolean) => {
    showPanel('inspector');
    setSelected(id);
    setPane('details');
    if (focus) focusInspectorTarget('details');
  };

  return (
    <>
      <div className="panel__toolbar asset-browser__toolbar">
        <div className="search-field">
          <Icon name="search" />
          <label htmlFor="asset-filter" className="visually-hidden">
            Filter assets by name or path
          </label>
          <input
            id="asset-filter"
            ref={filterRef}
            type="search"
            placeholder="Filter"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape' && filter) {
                event.preventDefault();
                setFilter('');
              } else if (event.key === 'ArrowDown' && active) {
                event.preventDefault();
                document.getElementById(assetRowId(active))?.focus();
              }
            }}
            aria-controls="asset-list"
            aria-describedby="asset-filter-status"
          />
          <span id="asset-filter-status" className="search-field__status" role="status">
            {filter.trim() ? <MatchCount count={visible.length} /> : ''}
          </span>
        </div>
        <ImportButton label="Import" />
      </div>
      <ImportStatus />
      {order.length === 0 ? (
        <StateView icon="search" title="No matching assets" compact />
      ) : (
        <AssetList
          groups={groups}
          order={order}
          active={active}
          selected={selected}
          recent={recent}
          onSelect={(id) => open(id, false)}
          onOpen={(id) => open(id, true)}
          onFind={() => filterRef.current?.focus()}
        />
      )}
    </>
  );
}

/** Row geometry (px): folder heading, asset row, the gap between folders, and padding. */
const FOLDER = 26;
const ROW = 28;
const GAP = 8;
const PAD = 2;

interface PlacedGroup {
  readonly top: number;
  readonly bottom: number;
  /** Absolute top of each asset row. */
  readonly rows: readonly number[];
}

/** Places folders and rows top to bottom; the list mounts only the ones near the scroll position. */
function placeGroups(groups: ReturnType<typeof groupByFolder>): { placed: PlacedGroup[]; total: number } {
  let y = PAD;
  const placed = groups.map((group, index) => {
    if (index > 0) y += GAP;
    const top = y;
    y += FOLDER;
    const rows = group.assets.map(() => {
      const row = y;
      y += ROW;
      return row;
    });
    return { top, bottom: y, rows };
  });
  return { placed, total: y + 12 };
}

/**
 * The asset listbox, grouped by source folder. Each folder is a labelled group; only
 * groups and rows near the scroll position are mounted (every mounted group keeps its
 * folder label), plus the active row, which is the roving Tab stop. Arrow keys move
 * through every asset, mounting the target before it takes focus.
 */
function AssetList({
  groups,
  order,
  active,
  selected,
  recent,
  onSelect,
  onOpen,
  onFind,
}: {
  groups: ReturnType<typeof groupByFolder>;
  order: readonly ProjectAsset[];
  active: Ulid | null;
  selected: Ulid | null;
  recent: ReadonlySet<string>;
  onSelect: (id: Ulid) => void;
  onOpen: (id: Ulid) => void;
  onFind: () => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const pendingFocus = useRef<Ulid | null>(null);
  const { placed, total } = useMemo(() => placeGroups(groups), [groups]);
  const where = useMemo(() => {
    const map = new Map<string, { group: number; row: number }>();
    groups.forEach((group, g) => group.assets.forEach((asset, r) => map.set(asset.id, { group: g, row: r })));
    return map;
  }, [groups]);
  const [viewport, attachViewport, remeasure] = useViewport(listRef, ROW * 4);
  const overscan = ROW * 8;
  const from = viewport.top - overscan;
  const to = viewport.top + viewport.height + overscan;
  const tops = useMemo(() => placed.map((group) => group.top), [placed]);
  // Groups intersecting the window, and the active row's group even when it is far away.
  const visible = new Map<number, Set<number>>();
  for (let g = Math.max(0, indexAt(tops, from)); g < placed.length && placed[g]!.top < to; g += 1) {
    const group = placed[g]!;
    if (group.bottom < from) continue;
    const rows = new Set<number>();
    for (let r = Math.max(0, indexAt(group.rows as number[], from)); r < group.rows.length && group.rows[r]! < to; r += 1) rows.add(r);
    visible.set(g, rows);
  }
  const pinned = active !== null ? where.get(active) : undefined;
  if (pinned) visible.set(pinned.group, new Set([...(visible.get(pinned.group) ?? []), pinned.row]));

  useEffect(() => {
    if (active === null) return;
    const spot = where.get(active);
    if (spot) {
      revealSpan(listRef.current, placed[spot.group]!.rows[spot.row]!, ROW, 4);
      remeasure();
    }
    if (pendingFocus.current === active) {
      pendingFocus.current = null;
      document.getElementById(assetRowId(active))?.focus();
    }
    // Only a change of the active asset scrolls the list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const moveTo = (id: Ulid) => {
    pendingFocus.current = id;
    onSelect(id);
    if (id === active) document.getElementById(assetRowId(id))?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const mod = event.metaKey || event.ctrlKey;
    if (mod && event.key.toLowerCase() === 'f') {
      event.preventDefault();
      onFind();
      return;
    }
    const index = order.findIndex((asset) => asset.id === active);
    const page = Math.max(1, Math.floor((listRef.current?.clientHeight || viewport.height) / ROW) - 1);
    let next = -1;
    if (event.key === 'ArrowDown') next = Math.min(order.length - 1, index + 1);
    else if (event.key === 'ArrowUp') next = Math.max(0, index - 1);
    else if (event.key === 'PageDown') next = Math.min(order.length - 1, index + page);
    else if (event.key === 'PageUp') next = Math.max(0, index - page);
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = order.length - 1;
    else if ((event.key === 'Enter' || event.key === ' ') && active !== null) {
      event.preventDefault();
      onOpen(active);
      return;
    } else return;
    event.preventDefault();
    const target = order[next];
    if (target) moveTo(target.id);
  };

  return (
    <div ref={attachViewport} id="asset-list" className="asset-list" role="listbox" aria-label="Assets" onKeyDown={onKeyDown}>
      <div className="asset-list__canvas" role="none" style={{ height: total }}>
        {[...visible.keys()].sort((a, b) => a - b).map((g) => {
          const group = groups[g]!;
          const layout = placed[g]!;
          const headId = `asset-folder-${g}`;
          const groupStyle = { top: layout.top, height: layout.bottom - layout.top } as CSSProperties;
          return (
            <div key={group.folder} role="group" aria-labelledby={headId} className="asset-list__group" style={groupStyle}>
              <div id={headId} className="asset-list__folder" title={group.folder ? `${group.folder}/` : 'Project folder'}>
                <Icon name="group" size={14} />
                {/* Real paths stay monospace like other paths; the root label is prose. */}
                <span className={`asset-list__folder-name${group.folder ? ' mono' : ''}`}>
                  <span className="visually-hidden">Folder </span>
                  {folderLabel(group.folder)}
                </span>
              </div>
              {[...visible.get(g)!].sort((a, b) => a - b).map((r) => {
                const asset = group.assets[r]!;
                const isActive = asset.id === active;
                const tag = asset.textureUsage ? USAGE_LABEL[asset.textureUsage] : kindLabel(asset.kind);
                return (
                  <div
                    key={asset.id}
                    id={assetRowId(asset.id)}
                    role="option"
                    aria-selected={asset.id === selected}
                    aria-posinset={r + 1}
                    aria-setsize={group.assets.length}
                    tabIndex={isActive ? 0 : -1}
                    data-focus-target={isActive ? '' : undefined}
                    className={`asset-row${asset.id === selected ? ' is-selected' : ''}`}
                    style={{ top: layout.rows[r]! - layout.top }}
                    title={asset.path}
                    onClick={() => onSelect(asset.id)}
                    onDoubleClick={() => onOpen(asset.id)}
                  >
                    <Icon name={iconForKind(asset.kind)} size={14} className={`asset-row__kind kind--${asset.kind}`} />
                    <span className="asset-row__name">{asset.name}</span>{' '}
                    <span className="asset-row__tag">
                      <span className="visually-hidden">
                        {kindLabel(asset.kind)}
                        {asset.textureUsage ? ', ' : ''}
                      </span>
                      {asset.textureUsage ? tag : null}
                    </span>
                    {recent.has(asset.path) ? (
                      <span className="asset-row__recent" title="Imported just now">
                        <span className="visually-hidden">, just imported</span>
                      </span>
                    ) : null}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
