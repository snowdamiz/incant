import { useEffect, useMemo, useRef } from 'react';
import type { KeyboardEvent } from 'react';
import type { BridgeSnapshot, EditorBridge, ProjectAsset, Ulid } from '../../bridge/contract';
import { unavailableMessage } from '../../bridge/resolve';
import type { CapabilitySet } from '../../bridge/resolve';
import { useAssets } from '../../assets/AssetsContext';
import { ACCEPTED_FORMATS, USAGE_LABEL, kindLabel, splitPath } from '../../assets/paths';
import { Icon, iconForKind } from '../../icons/Icon';
import { projectState, sameError } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { Skeleton, StateView } from '../StateView';
import { AssetPane, focusPane } from './AssetPane';

export type ImportAvailability = { readonly ok: true } | { readonly ok: false; readonly reason: string };

/** Whether this host can import now, and if not, the host's own reason. */
export function importAvailability(
  snapshot: BridgeSnapshot | null,
  bridge: EditorBridge | null,
  capabilities: CapabilitySet,
): ImportAvailability {
  if (!capabilities.has('asset.import')) return { ok: false, reason: unavailableMessage('asset.import', bridge) };
  const state = snapshot?.assetImport;
  if (!state) return { ok: false, reason: 'The connected engine did not report whether importing is available.' };
  if (!state.available) return { ok: false, reason: state.reason ?? 'The connected engine cannot import assets right now.' };
  return { ok: true };
}

export const assetRowId = (id: string) => `asset-row-${id}`;

/** Sorted by source path so files in the same folder sit together. */
function sortAssets(assets: readonly ProjectAsset[]) {
  return [...assets].sort((a, b) => a.path.localeCompare(b.path) || a.id.localeCompare(b.id));
}

function matches(asset: ProjectAsset, filter: string) {
  const needle = filter.trim().toLowerCase();
  return !needle || asset.name.toLowerCase().includes(needle) || asset.path.toLowerCase().includes(needle);
}

/** The Assets tab: a browsable list of the project's imported assets, plus details and import. */
/** The dock height the details and import panes need to show their main action without scrolling. */
export const PANE_MIN_DOCK = 300;

export function AssetsTab({ onRequestHeight }: { onRequestHeight?: ((min: number) => void) | undefined }) {
  const { snapshot } = useShell();
  const { pane, pending } = useAssets();
  const assets = snapshot?.assets;
  const project = projectState(snapshot);

  let browser;
  if (!snapshot) {
    browser = <StateView icon="plug" title="No engine connected" compact />;
  } else if (!assets) {
    browser = (
      <StateView icon="texture" title="Assets are not available" compact>
        <p>The connected engine does not report project assets.</p>
      </StateView>
    );
  } else if (assets.status === 'loading' || project.kind === 'loading') {
    browser = <Skeleton rows={4} label="Loading assets" />;
  } else if (assets.status === 'error' && project.kind === 'failed' && sameError(assets.error, project.error)) {
    browser = (
      <StateView icon="texture" title="No project loaded" compact>
        <p>Assets appear once the project opens.</p>
      </StateView>
    );
  } else if (assets.status === 'error') {
    browser = (
      <StateView icon="error" tone="error" title="Assets could not be listed" compact>
        <p>{assets.error.message}</p>
        <p className="mono subtle">{assets.error.code}</p>
      </StateView>
    );
  } else {
    browser = <Library assets={assets.value} onRequestHeight={onRequestHeight} />;
  }

  return (
    <div className="assets" data-pane={pane !== 'closed' ? 'open' : undefined}>
      <div className="assets__browser">
        {pending ? (
          <div className="assets__busy" role="status">
            <span className="spinner spinner--small" aria-hidden="true" />
            <span>
              {pending.kind === 'reimport' ? `Reimporting ${pending.name ?? pending.sources[0]?.source ?? ''}…` : `Importing ${countFiles(pending.sources.length)}…`}
            </span>
            <span className="assets__busy-note">You can keep working. Editing now means retrying the import.</span>
          </div>
        ) : null}
        {browser}
      </div>
      {pane !== 'closed' ? <AssetPane /> : null}
    </div>
  );
}

export const countFiles = (count: number) => `${count} ${count === 1 ? 'file' : 'files'}`;

function Library({ assets, onRequestHeight }: { assets: readonly ProjectAsset[]; onRequestHeight?: ((min: number) => void) | undefined }) {
  const { snapshot, bridge, capabilities } = useShell();
  const { filter, setFilter, selected, setSelected, setPane, pane, recent } = useAssets();
  const filterRef = useRef<HTMLInputElement>(null);
  const sorted = useMemo(() => sortAssets(assets), [assets]);
  const rows = useMemo(() => sorted.filter((asset) => matches(asset, filter)), [sorted, filter]);
  const availability = importAvailability(snapshot, bridge, capabilities);
  const active: Ulid | null = selected !== null && rows.some((asset) => asset.id === selected) ? selected : (rows[0]?.id ?? null);

  const openImport = () => {
    onRequestHeight?.(PANE_MIN_DOCK);
    setPane('import');
    focusPane('import');
  };

  if (assets.length === 0) {
    return (
      <StateView
        icon="texture"
        title="No assets yet"
        compact
        action={
          <button type="button" className="button button--small" aria-disabled={!availability.ok || undefined} onClick={openImport}>
            <Icon name="import" size={14} />
            Import from project folder
          </button>
        }
      >
        <p>{availability.ok ? `Import ${ACCEPTED_FORMATS} files that are already inside the project folder.` : availability.reason}</p>
      </StateView>
    );
  }

  const matchCount = filter.trim() ? rows.length : null;
  return (
    <>
      <div className="assets__toolbar">
        <div className="search-field">
          <Icon name="search" />
          <label htmlFor="asset-filter" className="visually-hidden">
            Filter assets by name or path
          </label>
          <input
            id="asset-filter"
            ref={filterRef}
            type="search"
            placeholder="Filter assets"
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
            {matchCount === null ? '' : `${matchCount} ${matchCount === 1 ? 'match' : 'matches'}`}
          </span>
        </div>
        <button
          type="button"
          className={`button button--small${pane === 'import' ? ' is-current' : ''}`}
          aria-disabled={!availability.ok || undefined}
          aria-expanded={pane === 'import'}
          aria-controls="asset-pane"
          title={availability.ok ? 'Import files from the project folder' : availability.reason}
          onClick={openImport}
        >
          <Icon name="import" size={14} />
          Import
        </button>
      </div>
      {rows.length === 0 ? (
        <StateView icon="search" title="No matching assets" compact>
          <p>Nothing named or stored at “{filter.trim()}”. Press Escape to clear the filter.</p>
        </StateView>
      ) : (
        <AssetList
          rows={rows}
          active={active}
          selected={selected}
          recent={recent}
          onSelect={(id) => setSelected(id)}
          onOpen={(id) => {
            setSelected(id);
            onRequestHeight?.(PANE_MIN_DOCK);
            setPane('details');
            focusPane('details');
          }}
          onFind={() => filterRef.current?.focus()}
        />
      )}
    </>
  );
}

function AssetList({
  rows,
  active,
  selected,
  recent,
  onSelect,
  onOpen,
  onFind,
}: {
  rows: readonly ProjectAsset[];
  active: Ulid | null;
  selected: Ulid | null;
  recent: ReadonlySet<string>;
  onSelect: (id: Ulid) => void;
  onOpen: (id: Ulid) => void;
  onFind: () => void;
}) {
  const pendingFocus = useRef<Ulid | null>(null);
  useEffect(() => {
    if (active === null) return;
    const element = document.getElementById(assetRowId(active));
    element?.scrollIntoView?.({ block: 'nearest' });
    if (pendingFocus.current === active) {
      pendingFocus.current = null;
      element?.focus();
    }
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
    const index = rows.findIndex((asset) => asset.id === active);
    let next = -1;
    if (event.key === 'ArrowDown') next = Math.min(rows.length - 1, index + 1);
    else if (event.key === 'ArrowUp') next = Math.max(0, index - 1);
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = rows.length - 1;
    else if ((event.key === 'Enter' || event.key === ' ') && active !== null) {
      event.preventDefault();
      onOpen(active);
      return;
    } else return;
    event.preventDefault();
    const target = rows[next];
    if (target) moveTo(target.id);
  };

  return (
    <div className="asset-list">
      <div className="asset-list__head" aria-hidden="true">
        <span />
        <span>Name</span>
        <span>Source</span>
        <span>Type</span>
      </div>
      <div id="asset-list" className="asset-list__body" role="listbox" aria-label="Assets" onKeyDown={onKeyDown}>
        {rows.map((asset) => {
          const { folder, file } = splitPath(asset.path);
          const isActive = asset.id === active;
          const type = asset.textureUsage ? `${kindLabel(asset.kind)} · ${USAGE_LABEL[asset.textureUsage]}` : kindLabel(asset.kind);
          return (
            <div
              key={asset.id}
              id={assetRowId(asset.id)}
              role="option"
              aria-selected={asset.id === selected}
              tabIndex={isActive ? 0 : -1}
              className={`asset-row${asset.id === selected ? ' is-selected' : ''}`}
              onClick={() => onOpen(asset.id)}
            >
              <span className={`asset-row__tile kind--${asset.kind}`} aria-hidden="true">
                <Icon name={iconForKind(asset.kind)} size={14} />
              </span>
              <span className="asset-row__name" title={asset.name}>
                {asset.name}
              </span>{' '}
              <span className="asset-row__path mono" title={asset.path}>
                <span className="asset-row__folder">{folder}</span>
                <span className="asset-row__file">{file}</span>
              </span>{' '}
              <span className="asset-row__type">
                {type}
                {recent.has(asset.path) ? (
                  <span className="asset-row__recent" title="Imported just now">
                    <span className="visually-hidden"> just imported</span>
                  </span>
                ) : null}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
