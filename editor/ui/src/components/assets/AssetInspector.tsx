import { useState } from 'react';
import type { ClipboardEvent, FormEvent, KeyboardEvent, ReactNode } from 'react';
import type { BridgeError, ProjectAsset, TextureUsage } from '../../bridge/contract';
import { defaultUsage, draftProblems, filledRows, reimportRequest, useAssets } from '../../assets/AssetsContext';
import type { DraftRow, ImportOutcome } from '../../assets/AssetsContext';
import { ACCEPTED_FORMATS, MAX_BATCH, USAGE_HINT, USAGE_LABEL, isConflict, kindLabel, sourceKind, sourceTypeLabel, splitPaste, textureTag } from '../../assets/paths';
import type { SourceKind } from '../../assets/paths';
import { Icon, iconForKind } from '../../icons/Icon';
import type { IconName } from '../../icons/Icon';
import { useShell } from '../../shell/ShellContext';
import { StateView } from '../StateView';
import { assetRowId, countFiles, importAvailability } from '../../assets/library';
import { IdentityHeader } from '../inspector/IdentityHeader';
import type { ImportAvailability } from '../../assets/library';

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
const TITLE_ID = 'asset-inspector-title';

/** The glyph and kind tint of an import source; a sprite atlas cooks to a texture. */
const SOURCE_GLYPH: Readonly<Record<SourceKind, { readonly icon: IconName; readonly kind: string }>> = {
  model: { icon: 'model', kind: 'model' },
  image: { icon: 'texture', kind: 'texture' },
  atlas: { icon: 'texture', kind: 'texture' },
  audio: { icon: 'audio', kind: 'audio' },
};

/**
 * Moves focus into the Inspector after it renders: the asset name for details, the first
 * empty path for the import form. Only called from a user action, so re-rendering never
 * steals focus from the list.
 */
export function focusInspectorTarget(mode: 'details' | 'import') {
  window.setTimeout(() => {
    const root = document.getElementById('asset-inspector');
    if (!root) return;
    const inputs = mode === 'import' ? [...root.querySelectorAll<HTMLInputElement>('.import-row__input')] : [];
    const target = inputs.find((input) => input.value === '') ?? inputs[0] ?? null;
    (target ?? document.getElementById(TITLE_ID))?.focus();
  }, 0);
}

/** Escape in the Inspector returns to the asset list, to the row it came from. */
function returnToList(selected: string | null) {
  window.setTimeout(() => {
    const row = selected !== null ? document.getElementById(assetRowId(selected)) : null;
    (row ?? document.querySelector<HTMLElement>('#asset-list [tabindex="0"]') ?? document.querySelector<HTMLElement>('.asset-browser__import'))?.focus();
  }, 0);
}

/** In-pane progress: no percentage or cancel exists, only what is true while cooking. */
function PendingStatus({ label }: { label: string }) {
  return (
    <span className="asset-pending" role="status">
      <span className="spinner spinner--small" aria-hidden="true" />
      <span className="asset-pending__text">
        {label}
        <span className="asset-pending__note">You can keep working. Editing now means retrying the import.</span>
      </span>
    </span>
  );
}

function scrollPaneTop() {
  document.getElementById('asset-inspector')?.closest('.panel__scroll')?.scrollTo?.({ top: 0 });
}

/** Recover a removed action/path only while its pane still owns the lost focus. */
function recoverImportFocus(origin: Element | null, scope: Element | null, mode: 'details' | 'import') {
  if (!origin || !scope?.contains(origin)) return () => {};
  return () => window.setTimeout(() => {
    if (!scope.isConnected || origin.isConnected || document.activeElement !== document.body) return;
    const target = mode === 'import'
      ? scope.querySelector<HTMLElement>('.import-row__input')
      : document.getElementById(TITLE_ID);
    target?.focus();
  }, 0);
}

/** Escape anywhere in the Inspector (its header included) returns to the asset list row. */
export function useAssetEscape() {
  const { showPanel } = useShell();
  const { selected } = useAssets();
  return (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === 'Escape' && !event.defaultPrevented) {
      event.preventDefault();
      showPanel('hierarchy');
      returnToList(selected);
    }
  };
}

/**
 * The Inspector's header line while the Assets view is active: the selected asset's
 * kind and name (the focus target when details open), or the import form's title.
 * Null when nothing is shown, so the panel keeps its plain "Inspector" title.
 */
export function useAssetInspectorHeader(): ReactNode {
  const { snapshot } = useShell();
  const { pane, selected } = useAssets();
  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value : [];
  if (pane === 'import') return <IdentityHeader icon="import" name="Import assets" titleId={TITLE_ID} focusable />;
  const asset = pane === 'details' ? assets.find((candidate) => candidate.id === selected) : undefined;
  if (!asset) return null;
  // Kind and usage (or "Atlas") are spoken with the name; the Interpretation section shows the usage.
  const description = [kindLabel(asset.kind), textureTag(asset)].filter(Boolean).join(', ');
  return <IdentityHeader icon={iconForKind(asset.kind)} kind={asset.kind} name={asset.name} titleId={TITLE_ID} focusable description={description} />;
}

/**
 * The Inspector body while the Assets view is active: the selected asset's details and
 * reimport, or the import form. Entity details stay in the same column for the
 * Hierarchy view, so "select on the left, inspect on the right" holds for both.
 */
export function AssetInspector() {
  const { snapshot, bridge, capabilities } = useShell();
  const { pane, selected } = useAssets();
  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value : [];
  const availability = importAvailability(snapshot, bridge, capabilities);
  const asset = pane === 'details' ? assets.find((candidate) => candidate.id === selected) : undefined;

  let content: ReactNode;
  if (pane === 'import') {
    content = availability.ok ? (
      <ImportForm assets={assets} />
    ) : (
      <StateView icon="lock" title="Importing is unavailable" compact>
        <p>{availability.reason}</p>
      </StateView>
    );
  } else if (asset) {
    content = <AssetDetails asset={asset} availability={availability} />;
  } else if (pane === 'details') {
    content = (
      <StateView icon="texture" title="This asset is no longer in the project" compact />
    );
  } else {
    content = (
      <StateView icon="texture" title="No asset selected" compact />
    );
  }
  return (
    <div id="asset-inspector" className="asset-inspector">
      {content}
    </div>
  );
}

function OutcomeNotice({ outcome, onDismiss }: { outcome: ImportOutcome; onDismiss: () => void }) {
  const { snapshot } = useShell();
  if (outcome.status === 'success') {
    const count = outcome.job.sources.length;
    const transaction = outcome.commit?.transaction;
    const history = snapshot?.history;
    const canUndoImport = transaction != null && history?.entries[history.applied - 1]?.transaction === transaction;
    return (
      <div className="asset-note asset-note--success" role="status">
        <Icon name="check" size={14} />
        <p>
          {transaction === null ? 'Already up to date. Nothing changed.'
            : transaction === undefined ? 'Import completed.'
            : outcome.job.kind === 'reimport' ? 'Reimported.' : `Imported ${countFiles(count)}.`}
          {canUndoImport ? ` Undo reverts the whole ${outcome.job.kind === 'reimport' ? 'reimport' : 'batch'}.` : null}
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
  // Only an ordinary image can be read another way; the button names a change only when
  // the reimport request actually carries one.
  const source = asset.kind === 'texture' ? sourceKind(asset.path) : null;
  const changed = reimportRequest(asset, reimportUsage[asset.id]).textureUsage !== undefined;
  const mine = pending?.asset === asset.id;
  const busy = pending !== null;
  const ownOutcome = outcome && outcome.job.kind === 'reimport' && outcome.job.asset === asset.id ? outcome : null;
  const blocked = !availability.ok || busy;

  return (
    <div className="asset-details">
      {ownOutcome ? (
        <div className="asset-section asset-section--notice">
          <OutcomeNotice outcome={ownOutcome} onDismiss={dismissOutcome} />
        </div>
      ) : null}
      <section className="asset-section" aria-labelledby={`source-${asset.id}`}>
        <h4 className="asset-section__title" id={`source-${asset.id}`}>
          Source
        </h4>
        <p className="asset-source mono">
          <PathText path={asset.path} />
        </p>
        {asset.kind === 'model' ? <p className="asset-section__hint">Placing models in a scene is not available yet.</p> : null}
      </section>
      {asset.kind === 'texture' ? (
        <section className="asset-section" aria-labelledby={`usage-${asset.id}`}>
          <h4 className="asset-section__title" id={`usage-${asset.id}`}>
            Interpretation
          </h4>
          {source === 'image' ? (
            <>
              <UsageChoice
                name={`reimport-usage-${asset.id}`}
                legend={`Interpret ${asset.name} as`}
                value={chosen}
                saved={saved}
                disabled={busy}
                onChange={(usage) => setReimportUsage(asset.id, usage)}
              />
              <p className="asset-section__hint">
                {USAGE_HINT[chosen]}
                {changed ? <span className="asset-section__changed"> Saved as {USAGE_LABEL[saved]}; reimport to apply.</span> : null}
              </p>
            </>
          ) : (
            // A sprite atlas is one color page; no other reading exists, so the saved
            // interpretation is shown as a value instead of being offered as a choice.
            <>
              <p className="asset-value">{USAGE_LABEL[saved]}</p>
              <p className="asset-section__hint">{source === 'atlas' ? 'Sprite atlases are always color.' : USAGE_HINT[saved]}</p>
            </>
          )}
        </section>
      ) : null}
      <section className="asset-section" aria-label="Reimport">
        <div className="asset-section__actions">
          {mine ? (
            <PendingStatus label="Reimporting…" />
          ) : (
            <button
              type="button"
              className="button button--primary button--small"
              aria-disabled={blocked || undefined}
              aria-describedby={`reimport-note-${asset.id}`}
              onClick={(event) => {
                if (blocked) return;
                const recover = recoverImportFocus(document.activeElement, event.currentTarget.closest('.asset-details'), 'details');
                // The result note is at the top of the Inspector; bring it into view when done.
                void reimport(asset).then(() => { scrollPaneTop(); recover(); });
              }}
            >
              <Icon name="reimport" size={14} />
              {ownOutcome?.status === 'failure' ? 'Try again' : changed ? `Reimport as ${USAGE_LABEL[chosen]}` : 'Reimport'}
            </button>
          )}
        </div>
        <p className="asset-section__hint" id={`reimport-note-${asset.id}`}>
          {!availability.ok
            ? availability.reason
            : busy && !mine
              ? 'Another import is running. Reimport when it finishes.'
              : 'Reads the source file again. The name and ID stay the same.'}
        </p>
      </section>
      <details className="asset-section asset-ids">
        <summary className="asset-section__title">
          <Icon name="chevron" size={12} className="asset-ids__chevron" />
          Identifiers
        </summary>
        <dl className="asset-ids__list">
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
    const recover = recoverImportFocus(document.activeElement, event.currentTarget, 'import');
    const ok = await importDraft();
    // The result is shown at the top of the form; bring it into view.
    scrollPaneTop();
    if (ok) {
      setAttempted(false);
      setVisited(new Set());
    }
    recover();
  };

  const onPaste = (row: DraftRow) => (event: ClipboardEvent<HTMLInputElement>) => {
    const lines = splitPaste(event.clipboardData.getData('text'));
    if (lines.length < 2) return;
    event.preventDefault();
    // Several lines become several rows. The first replaces this path when the field is
    // empty or wholly selected (select all, then paste); otherwise this path is kept and
    // every pasted line goes into new rows below it.
    const input = event.currentTarget;
    const replaces = row.source === '' || (input.selectionStart === 0 && input.selectionEnd === row.source.length);
    const [first, ...rest] = lines as [string, ...string[]];
    if (replaces) setSource(row.key, first);
    const extra = replaces ? rest : lines;
    const added = addRows(extra, row.key);
    const dropped = extra.length - added.length;
    if (dropped > 0) announce(`Only ${MAX_BATCH} paths fit in one import; ${dropped} were not added.`);
  };

  return (
    <form className="import-form" onSubmit={submit} noValidate aria-busy={mine || undefined} aria-labelledby={TITLE_ID}>
      {ownOutcome ? <OutcomeNotice outcome={ownOutcome} onDismiss={dismissOutcome} /> : null}
      <p className="import-form__lede">
        Enter paths relative to the project file, such as <code>models/crate.glb</code>. Files must already be in the
        project folder.
      </p>
      <ol className="import-rows" aria-label="Paths to import">
        {draft.map((row, index) => {
          const kind = row.source ? sourceKind(row.source) : null;
          const glyph = kind ? SOURCE_GLYPH[kind] : null;
          const type = kind ? sourceTypeLabel(kind, row.source) : null;
          const existing = assets.find((asset) => asset.path === row.source);
          const problem = problems.get(row.key);
          const show = problem !== undefined && (attempted || (visited.has(row.key) && row.source !== ''));
          const problemId = `import-row-${row.key}-problem`;
          return (
            <li key={row.key} className={`import-row${show ? ' is-invalid' : ''}`}>
              <div className="import-row__line">
                <span className={`import-row__kind${glyph ? ` kind--${glyph.kind}` : ''}`} aria-hidden="true">
                  <Icon name={glyph?.icon ?? 'prefab'} size={14} />
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
              ) : type ? (
                <div className="import-row__meta">
                  <span className="import-row__type">{type}</span>
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
          <PendingStatus label={`Importing ${countFiles(pending.sources.length)}…`} />
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
