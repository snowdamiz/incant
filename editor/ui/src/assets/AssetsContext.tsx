import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { AssetImportRequest, BridgeError, ProjectAsset, TextureUsage, Ulid } from '../bridge/contract';
import { useShell } from '../shell/ShellContext';
import { MAX_BATCH, pathProblem, sourceKind } from './paths';

/**
 * UI state for the asset library: the import draft, the one running import and its
 * outcome. It lives above the dock so switching tabs or hiding the dock does not lose
 * typed paths, a pending import or its error. It holds no project data: the asset
 * list always comes from the bridge snapshot, and every change goes through
 * Shell.dispatch as one `asset.import` command.
 */

export interface DraftRow {
  readonly key: number;
  readonly source: string;
  /** Null keeps the default: the saved setting for a known image, otherwise color. */
  readonly usage: TextureUsage | null;
}

export interface ImportJob {
  readonly kind: 'import' | 'reimport';
  readonly sources: readonly AssetImportRequest[];
  /** For a reimport, the asset it was started from and its name when started. */
  readonly asset?: Ulid;
  readonly name?: string;
}

export type ImportOutcome =
  | { readonly status: 'success'; readonly job: ImportJob }
  | { readonly status: 'failure'; readonly job: ImportJob; readonly error: BridgeError };

export type Pane = 'closed' | 'import' | 'details';

export interface AssetWorkspace {
  readonly draft: readonly DraftRow[];
  readonly setSource: (key: number, source: string) => void;
  readonly setUsage: (key: number, usage: TextureUsage) => void;
  /** Inserts rows after `after` (or at the end), up to the batch limit. Returns the keys added. */
  readonly addRows: (sources: readonly string[], after?: number) => number[];
  readonly removeRow: (key: number) => void;
  readonly pending: ImportJob | null;
  readonly outcome: ImportOutcome | null;
  readonly dismissOutcome: () => void;
  /** Source paths written by the last successful import, marked in the list. */
  readonly recent: ReadonlySet<string>;
  readonly pane: Pane;
  readonly setPane: (pane: Pane) => void;
  readonly selected: Ulid | null;
  readonly setSelected: (id: Ulid | null) => void;
  readonly filter: string;
  readonly setFilter: (filter: string) => void;
  /** Interpretation chosen in the details pane, per asset, until it is reimported. */
  readonly reimportUsage: Readonly<Record<string, TextureUsage>>;
  readonly setReimportUsage: (id: Ulid, usage: TextureUsage) => void;
  /** Sends the draft. Resolves false without dispatching if any row is invalid. */
  readonly importDraft: () => Promise<boolean>;
  readonly reimport: (asset: ProjectAsset) => Promise<boolean>;
}

const AssetsContext = createContext<AssetWorkspace | null>(null);

/** The interpretation an image gets when no usage is sent: its saved one, or color. */
export function defaultUsage(source: string, assets: readonly ProjectAsset[]): TextureUsage {
  return assets.find((asset) => asset.path === source)?.textureUsage ?? 'color';
}

/** The request for one draft row. Usage is sent only for images, and only when it changes something. */
export function requestFor(row: DraftRow, assets: readonly ProjectAsset[]): AssetImportRequest {
  if (sourceKind(row.source) !== 'image' || row.usage === null || row.usage === defaultUsage(row.source, assets)) {
    return { source: row.source };
  }
  return { source: row.source, textureUsage: row.usage };
}

/** The rows an import sends: blank rows are ignored unless every row is blank. */
export function filledRows(draft: readonly DraftRow[]): readonly DraftRow[] {
  const filled = draft.filter((row) => row.source !== '');
  return filled.length > 0 ? filled : draft.slice(0, 1);
}

/** Per-row problems for the rows that would be sent, including repeats, keyed by row. */
export function draftProblems(draft: readonly DraftRow[]): Map<number, string> {
  const problems = new Map<number, string>();
  const seen = new Set<string>();
  for (const row of filledRows(draft)) {
    const problem = pathProblem(row.source);
    if (problem) problems.set(row.key, problem);
    else if (seen.has(row.source)) problems.set(row.key, 'This path is already in the list.');
    seen.add(row.source);
  }
  return problems;
}

export function AssetsProvider({ children }: { children: ReactNode }) {
  const { snapshot, dispatch, announce } = useShell();
  const nextKey = useRef(1);
  const [draft, setDraft] = useState<readonly DraftRow[]>(() => [{ key: 0, source: '', usage: null }]);
  const [pending, setPending] = useState<ImportJob | null>(null);
  const [outcome, setOutcome] = useState<ImportOutcome | null>(null);
  const [recent, setRecent] = useState<ReadonlySet<string>>(() => new Set());
  const [pane, setPane] = useState<Pane>('closed');
  const [selected, setSelected] = useState<Ulid | null>(null);
  const [filter, setFilter] = useState('');
  const [reimportUsage, setReimportUsageState] = useState<Readonly<Record<string, TextureUsage>>>({});
  const pendingRef = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value : [];
  const assetsRef = useRef(assets);
  assetsRef.current = assets;

  const clearSuccess = () => setOutcome((current) => (current?.status === 'success' ? null : current));

  const draftRef = useRef(draft);
  draftRef.current = draft;
  // Every draft change derives from the latest rows, so changes made in one event
  // (a paste that edits a row and inserts more) compose instead of overwriting.
  const commit = useCallback((next: readonly DraftRow[]) => {
    draftRef.current = next;
    setDraft(next);
  }, []);
  const setSource = useCallback((key: number, source: string) => {
    clearSuccess();
    commit(draftRef.current.map((row) => (row.key === key ? { ...row, source } : row)));
  }, [commit]);
  const setUsage = useCallback((key: number, usage: TextureUsage) => {
    clearSuccess();
    commit(draftRef.current.map((row) => (row.key === key ? { ...row, usage } : row)));
  }, [commit]);
  const addRows = useCallback((sources: readonly string[], after?: number) => {
    clearSuccess();
    const rows = draftRef.current;
    const fresh = sources.slice(0, Math.max(0, MAX_BATCH - rows.length)).map((source) => ({ key: nextKey.current++, source, usage: null }));
    const index = after === undefined ? rows.length : rows.findIndex((row) => row.key === after) + 1;
    commit([...rows.slice(0, index), ...fresh, ...rows.slice(index)]);
    return fresh.map((row) => row.key);
  }, [commit]);
  const removeRow = useCallback((key: number) => {
    const rows = draftRef.current;
    commit(rows.length > 1 ? rows.filter((row) => row.key !== key) : [{ key: nextKey.current++, source: '', usage: null }]);
  }, [commit]);
  const setReimportUsage = useCallback(
    (id: Ulid, usage: TextureUsage) => setReimportUsageState((current) => ({ ...current, [id]: usage })),
    [],
  );

  const start = useCallback(
    async (job: ImportJob): Promise<boolean> => {
      if (pendingRef.current) {
        announce('An import is already running. Wait for it to finish.');
        return false;
      }
      pendingRef.current = true;
      setPending(job);
      setOutcome(null);
      announce(job.kind === 'reimport' ? `Reimporting ${job.name ?? job.sources[0]?.source ?? 'asset'}…` : `Importing ${job.sources.length} ${job.sources.length === 1 ? 'file' : 'files'}…`);
      const result = await dispatch({ type: 'asset.import', sources: job.sources });
      pendingRef.current = false;
      if (!mounted.current) return result.ok;
      setPending(null);
      if (!result.ok) {
        setOutcome({ status: 'failure', job, error: result.error });
        return false;
      }
      setOutcome({ status: 'success', job });
      setRecent(new Set(job.sources.map((source) => source.source)));
      if (job.kind === 'import') {
        commit([{ key: nextKey.current++, source: '', usage: null }]);
        const count = job.sources.length;
        announce(`Imported ${count} ${count === 1 ? 'file' : 'files'}.`);
      } else {
        if (job.asset) {
          const id = job.asset;
          setReimportUsageState(({ [id]: _done, ...rest }) => rest);
        }
        announce(`Reimported ${job.name ?? job.sources[0]?.source ?? 'asset'}.`);
      }
      return true;
    },
    [announce, commit, dispatch],
  );

  const importDraft = useCallback(async () => {
    if (draftProblems(draft).size > 0) return false;
    return start({ kind: 'import', sources: filledRows(draft).map((row) => requestFor(row, assetsRef.current)) });
  }, [draft, start]);

  const reimport = useCallback(
    async (asset: ProjectAsset) => {
      const chosen = reimportUsage[asset.id];
      // Omitting the usage keeps the saved setting; it is sent only when the user changed it.
      const source: AssetImportRequest =
        asset.kind === 'texture' && chosen !== undefined && chosen !== (asset.textureUsage ?? 'color')
          ? { source: asset.path, textureUsage: chosen }
          : { source: asset.path };
      return start({ kind: 'reimport', sources: [source], asset: asset.id, name: asset.name });
    },
    [reimportUsage, start],
  );

  const value = useMemo<AssetWorkspace>(
    () => ({
      draft,
      setSource,
      setUsage,
      addRows,
      removeRow,
      pending,
      outcome,
      dismissOutcome: () => setOutcome(null),
      recent,
      pane,
      setPane,
      selected,
      setSelected,
      filter,
      setFilter,
      reimportUsage,
      setReimportUsage,
      importDraft,
      reimport,
    }),
    [draft, setSource, setUsage, addRows, removeRow, pending, outcome, recent, pane, selected, filter, reimportUsage, setReimportUsage, importDraft, reimport],
  );
  return <AssetsContext.Provider value={value}>{children}</AssetsContext.Provider>;
}

export function useAssets(): AssetWorkspace {
  const workspace = useContext(AssetsContext);
  if (!workspace) throw new Error('useAssets must be used inside AssetsProvider');
  return workspace;
}
