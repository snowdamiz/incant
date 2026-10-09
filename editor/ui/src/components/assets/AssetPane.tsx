import { useState } from 'react';
import type { ClipboardEvent, FormEvent, ReactNode } from 'react';
import type { BridgeError, ProjectAsset, TextureUsage } from '../../bridge/contract';
import { defaultUsage, draftProblems, filledRows, useAssets } from '../../assets/AssetsContext';
import type { DraftRow, ImportOutcome } from '../../assets/AssetsContext';
import { ACCEPTED_FORMATS, MAX_BATCH, USAGE_HINT, USAGE_LABEL, isConflict, kindLabel, sourceKind, splitPaste } from '../../assets/paths';
import { Icon, iconForKind } from '../../icons/Icon';
import { useShell } from '../../shell/ShellContext';
import { StateView } from '../StateView';
import { assetRowId, countFiles, importAvailability } from './AssetsTab';
import type { ImportAvailability } from './AssetsTab';

/** A path that wraps after each "/" rather than mid-name. Text only; never parsed as markup. */
export function PathText({ path }: { path: string }) {
  const parts = path.split('/');
  return (
    <>
      {parts.map((part, index) => (
        <span key={index}>
          {part}
          {index < parts.length - 1 ? (
            <>
              /<wbr />
            </>
          ) : null}
        </span>
      ))}
    </>
  );
}

const USAGES: readonly TextureUsage[] = ['color', 'linear', 'normal'];
const PANE_TITLE = 'asset-pane-title';

/**
 * Moves focus into the pane after it renders. Only called from a user action (opening
 * details or the import form), so re-mounting the dock never steals focus.
 */
export function focusPane(mode: 'details' | 'import') {
  window.setTimeout(() => {
    const pane = document.getElementById('asset-pane');
    if (!pane) return;
    const inputs = mode === 'import' ? [...pane.querySelectorAll<HTMLInputElement>('.import-row__input')] : [];
    const target = inputs.find((input) => input.value === '') ?? inputs[0] ?? null;
    (target ?? document.getElementById(PANE_TITLE))?.focus();
  }, 0);
}

/** Details/reimport for the selected asset, or the import form. */
export function AssetPane() {
  const { snapshot, bridge, capabilities } = useShell();
  const { pane, setPane, selected } = useAssets();
  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value : [];
  const availability = importAvailability(snapshot, bridge, capabilities);
  const asset = pane === 'details' ? assets.find((candidate) => candidate.id === selected) : undefined;

  const close = () => {
    setPane('closed');
    window.setTimeout(() => {
      const row = selected !== null ? document.getElementById(assetRowId(selected)) : null;
      (row ?? document.querySelector<HTMLElement>('#asset-list [tabindex="0"]') ?? document.getElementById('dock-tab-assets'))?.focus();
    }, 0);
  };

  let title: ReactNode;
  let body: ReactNode;
  if (pane === 'import') {
    title = 'Import from project folder';
    body = availability.ok ? (
      <ImportForm assets={assets} />
    ) : (
      <StateView icon="lock" title="Importing is unavailable" compact>
        <p>{availability.reason}</p>
      </StateView>
    );
  } else if (asset) {
    title = asset.name;
    body = <AssetDetails asset={asset} availability={availability} />;
  } else {
    title = 'Asset';
    body = (
      <StateView icon="texture" title="This asset is no longer in the project" compact>
        <p>It may have been removed by an undo. Choose another asset from the list.</p>
      </StateView>
    );
  }

  return (
    <aside
      id="asset-pane"
      className="asset-pane"
      aria-labelledby={PANE_TITLE}
      onKeyDown={(event) => {
        if (event.key === 'Escape' && !event.defaultPrevented) {
          event.preventDefault();
          close();
        }
      }}
    >
      <header className="asset-pane__header">
        <button type="button" className="tool-button asset-pane__back" aria-label="Back to assets" title="Back to assets" onClick={close}>
          <Icon name="back" />
        </button>
        {asset ? (
          <span className={`asset-pane__tile kind--${asset.kind}`} aria-hidden="true">
            <Icon name={iconForKind(asset.kind)} size={14} />
          </span>
        ) : pane === 'import' ? (
          <span className="asset-pane__tile" aria-hidden="true">
            <Icon name="import" size={14} />
          </span>
        ) : null}
        <h3 id={PANE_TITLE} className="asset-pane__title" tabIndex={-1} title={typeof title === 'string' ? title : undefined}>
          {title}
        </h3>
        <button type="button" className="tool-button asset-pane__close" aria-label="Close panel" title="Close (Esc)" onClick={close}>
          <Icon name="close" />
        </button>
      </header>
      <div className="asset-pane__body">{body}</div>
    </aside>
  );
}

function OutcomeNotice({ outcome, onDismiss }: { outcome: ImportOutcome; onDismiss: () => void }) {
  if (outcome.status === 'success') {
    const count = outcome.job.sources.length;
    return (
      <div className="asset-note asset-note--success" role="status">
        <Icon name="check" size={14} />
        <p>
          {outcome.job.kind === 'reimport' ? 'Reimported.' : `Imported ${countFiles(count)}.`} Undo reverts the whole{' '}
          {outcome.job.kind === 'reimport' ? 'reimport' : 'batch'}.
        </p>
      </div>
    );
  }
  return <FailureNotice error={outcome.error} onDismiss={onDismiss} />;
}

function FailureNotice({ error, onDismiss }: { error: BridgeError; onDismiss: () => void }) {
  const conflict = isConflict(error.message, error.code);
  return (
    <div className="asset-note asset-note--error" role="alert">
      <Icon name="error" size={14} />
      <div className="asset-note__text">
        <p className="asset-note__title">{conflict ? 'The project changed during the import' : 'Nothing was imported'}</p>
        <p className="asset-note__message">{error.message}</p>
        <p className="asset-note__hint">
          {conflict ? 'Nothing was imported. Try again to import into the current project.' : 'The asset list is unchanged. Your paths are kept below.'}
        </p>
      </div>
      <button type="button" className="tool-button asset-note__dismiss" aria-label="Dismiss error" title="Dismiss" onClick={onDismiss}>
        <Icon name="close" size={12} />
      </button>
    </div>
  );
}

function UsageChoice({
  name,
  legend,
  value,
  saved,
  disabled,
  onChange,
}: {
  name: string;
  legend: string;
  value: TextureUsage;
  saved?: TextureUsage | undefined;
  disabled: boolean;
  onChange: (usage: TextureUsage) => void;
}) {
  return (
    <fieldset className="segmented" disabled={disabled}>
      <legend className="visually-hidden">{legend}</legend>
      {USAGES.map((usage) => (
        <label key={usage} className="segmented__option" title={USAGE_HINT[usage]}>
          <input type="radio" name={name} value={usage} checked={value === usage} onChange={() => onChange(usage)} />
          <span>{USAGE_LABEL[usage]}</span>
          {saved === usage && value !== usage ? <span className="visually-hidden"> (saved)</span> : null}
        </label>
      ))}
    </fieldset>
  );
}

function AssetDetails({ asset, availability }: { asset: ProjectAsset; availability: ImportAvailability }) {
  const { pending, outcome, dismissOutcome, reimport, reimportUsage, setReimportUsage } = useAssets();
  const saved = asset.textureUsage ?? 'color';
  const chosen = reimportUsage[asset.id] ?? saved;
  const changed = asset.kind === 'texture' && chosen !== saved;
  const mine = pending?.asset === asset.id;
  const busy = pending !== null;
  const ownOutcome = outcome && outcome.job.kind === 'reimport' && outcome.job.asset === asset.id ? outcome : null;
  const blocked = !availability.ok || busy;

  return (
    <div className="asset-details">
      {ownOutcome ? <OutcomeNotice outcome={ownOutcome} onDismiss={dismissOutcome} /> : null}
      <dl className="asset-facts">
        <div>
          <dt>Type</dt>
          <dd>{kindLabel(asset.kind)}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd className="mono asset-facts__path">
            <PathText path={asset.path} />
          </dd>
        </div>
      </dl>
      {asset.kind === 'texture' ? (
        <div className="asset-details__section">
          <p className="asset-details__label" aria-hidden="true">
            Interpret as
          </p>
          <UsageChoice
            name={`reimport-usage-${asset.id}`}
            legend={`Interpret ${asset.name} as`}
            value={chosen}
            saved={saved}
            disabled={busy}
            onChange={(usage) => setReimportUsage(asset.id, usage)}
          />
          <p className="asset-details__hint">
            {USAGE_HINT[chosen]}
            {changed ? <span className="asset-details__changed"> Saved as {USAGE_LABEL[saved]}; reimport to apply.</span> : null}
          </p>
        </div>
      ) : null}
      <div className="asset-details__actions">
        {mine ? (
          <span className="asset-pending" role="status">
            <span className="spinner spinner--small" aria-hidden="true" />
            Reimporting…
          </span>
        ) : (
          <button
            type="button"
            className="button button--primary button--small"
            aria-disabled={blocked || undefined}
            aria-describedby={`reimport-note-${asset.id}`}
            onClick={() => {
              if (blocked) return;
              void reimport(asset);
            }}
          >
            <Icon name="reimport" size={14} />
            {ownOutcome?.status === 'failure' ? 'Try again' : changed ? `Reimport as ${USAGE_LABEL[chosen]}` : 'Reimport'}
          </button>
        )}
      </div>
      <p className="asset-details__hint" id={`reimport-note-${asset.id}`}>
        {!availability.ok
          ? availability.reason
          : busy && !mine
            ? 'Another import is running. Reimport when it finishes.'
            : 'Reads the source file again. The name and ID stay the same.'}
      </p>
      {asset.kind === 'model' ? <p className="asset-details__note">Placing models in a scene is not available yet.</p> : null}
      <details className="asset-details__ids">
        <summary>Identifiers</summary>
        <dl className="asset-facts asset-facts--ids">
          <div>
            <dt>ID</dt>
            <dd className="mono">{asset.id}</dd>
          </div>
          <div>
            <dt>Fingerprint</dt>
            <dd className="mono">{asset.fingerprint}</dd>
          </div>
        </dl>
      </details>
    </div>
  );
}

function ImportForm({ assets }: { assets: readonly ProjectAsset[] }) {
  const { draft, setSource, setUsage, addRows, removeRow, pending, outcome, dismissOutcome, importDraft } = useAssets();
  const { announce } = useShell();
  const [attempted, setAttempted] = useState(false);
  const [visited, setVisited] = useState<ReadonlySet<number>>(() => new Set());
  const problems = draftProblems(draft);
  const busy = pending !== null;
  const mine = pending?.kind === 'import';
  const ownOutcome = outcome && outcome.job.kind === 'import' ? outcome : null;
  const full = draft.length >= MAX_BATCH;
  const filled = draft.some((row) => row.source !== '') ? filledRows(draft).length : 0;

  const focusRow = (key: number) => window.setTimeout(() => document.getElementById(`import-row-${key}`)?.focus(), 0);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (busy) return;
    setAttempted(true);
    const first = draft.find((row) => problems.has(row.key));
    if (first) {
      announce(`Fix ${problems.size === 1 ? 'the highlighted path' : `${problems.size} highlighted paths`} before importing.`);
      focusRow(first.key);
      return;
    }
    const ok = await importDraft();
    // The result is shown at the top of the form; bring it into view.
    document.getElementById('asset-pane')?.querySelector('.asset-pane__body')?.scrollTo?.({ top: 0 });
    if (ok) {
      setAttempted(false);
      setVisited(new Set());
    }
  };

  const onPaste = (row: DraftRow) => (event: ClipboardEvent<HTMLInputElement>) => {
    const lines = splitPaste(event.clipboardData.getData('text'));
    if (lines.length < 2) return;
    // Several lines become several rows; the first replaces this row only if it is empty.
    event.preventDefault();
    const [first, ...rest] = lines as [string, ...string[]];
    if (row.source === '') setSource(row.key, first);
    const added = addRows(row.source === '' ? rest : lines, row.key);
    const dropped = (row.source === '' ? rest : lines).length - added.length;
    if (dropped > 0) announce(`Only ${MAX_BATCH} paths fit in one import; ${dropped} were not added.`);
  };

  return (
    <form className="import-form" onSubmit={submit} noValidate aria-busy={mine || undefined}>
      {ownOutcome ? <OutcomeNotice outcome={ownOutcome} onDismiss={dismissOutcome} /> : null}
      <p className="import-form__lede">
        Paths are relative to the project file, for example <code>models/crate.glb</code>. The files must already be in the
        project folder.
      </p>
      <ol className="import-rows" aria-label="Paths to import">
        {draft.map((row, index) => {
          const kind = row.source ? sourceKind(row.source) : null;
          const existing = assets.find((asset) => asset.path === row.source);
          const problem = problems.get(row.key);
          const show = problem !== undefined && (attempted || (visited.has(row.key) && row.source !== ''));
          const problemId = `import-row-${row.key}-problem`;
          return (
            <li key={row.key} className={`import-row${show ? ' is-invalid' : ''}`}>
              <div className="import-row__line">
                <span className={`import-row__kind${kind ? ` kind--${kind === 'image' ? 'texture' : 'model'}` : ''}`} aria-hidden="true">
                  <Icon name={kind === 'model' ? 'model' : kind === 'image' ? 'texture' : 'prefab'} size={14} />
                </span>
                <input
                  id={`import-row-${row.key}`}
                  className="import-row__input mono"
                  aria-label={`Path ${index + 1}`}
                  aria-invalid={show || undefined}
                  aria-describedby={show ? problemId : undefined}
                  placeholder={index === 0 ? 'models/crate.glb' : 'textures/crate_albedo.png'}
                  value={row.source}
                  readOnly={busy}
                  spellCheck={false}
                  autoCapitalize="off"
                  autoComplete="off"
                  autoCorrect="off"
                  onChange={(event) => setSource(row.key, event.target.value)}
                  onBlur={() => setVisited((current) => (current.has(row.key) ? current : new Set([...current, row.key])))}
                  onPaste={onPaste(row)}
                  onKeyDown={(event) => {
                    // Shift+Enter adds a path below this one; Enter submits the form.
                    if (event.key === 'Enter' && event.shiftKey && !busy && !full) {
                      event.preventDefault();
                      const [key] = addRows([''], row.key);
                      if (key !== undefined) focusRow(key);
                    }
                  }}
                />
                <button
                  type="button"
                  className="tool-button import-row__remove"
                  aria-label={`Remove path ${index + 1}`}
                  title="Remove"
                  aria-disabled={busy || (draft.length === 1 && row.source === '') || undefined}
                  onClick={() => {
                    if (busy || (draft.length === 1 && row.source === '')) return;
                    const neighbour = draft[index + 1] ?? draft[index - 1];
                    removeRow(row.key);
                    if (neighbour) focusRow(neighbour.key);
                    else focusRow(row.key);
                  }}
                >
                  <Icon name="close" size={12} />
                </button>
              </div>
              {show ? (
                <p className="import-row__problem" id={problemId}>
                  {problem}
                </p>
              ) : kind === 'image' ? (
                <div className="import-row__meta">
                  <UsageChoice
                    name={`import-usage-${row.key}`}
                    legend={`Interpret path ${index + 1} as`}
                    value={row.usage ?? defaultUsage(row.source, assets)}
                    saved={existing?.textureUsage}
                    disabled={busy}
                    onChange={(usage) => setUsage(row.key, usage)}
                  />
                  {existing ? <span className="import-row__existing">Updates “{existing.name}”</span> : null}
                </div>
              ) : kind === 'model' ? (
                <div className="import-row__meta">
                  <span className="import-row__type">Model</span>
                  {existing ? <span className="import-row__existing">Updates “{existing.name}”</span> : null}
                </div>
              ) : null}
            </li>
          );
        })}
      </ol>
      <div className="import-form__add">
        <button
          type="button"
          className="button button--ghost button--small"
          aria-disabled={busy || full || undefined}
          onClick={() => {
            if (busy || full) return;
            const [key] = addRows(['']);
            if (key !== undefined) focusRow(key);
          }}
        >
          <Icon name="plus" size={14} />
          Add path
        </button>
        <span className="import-form__count">
          {draft.length} of {MAX_BATCH}
          <span className="visually-hidden"> paths</span>
        </span>
      </div>
      <p className="import-form__formats">Accepts {ACCEPTED_FORMATS}. Paste several lines to add several paths.</p>
      <div className="import-form__footer">
        {mine ? (
          <span className="asset-pending" role="status">
            <span className="spinner spinner--small" aria-hidden="true" />
            Importing {countFiles(pending.sources.length)}…
          </span>
        ) : (
          <button type="submit" className="button button--primary button--small" aria-disabled={busy || undefined}>
            <Icon name="import" size={14} />
            {ownOutcome?.status === 'failure' ? 'Try again' : filled > 0 ? `Import ${countFiles(filled)}` : 'Import'}
          </button>
        )}
        {busy && !mine ? <span className="import-form__wait">Another import is running.</span> : null}
      </div>
    </form>
  );
}
