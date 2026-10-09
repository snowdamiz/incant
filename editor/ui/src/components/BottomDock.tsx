import { useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { ConsoleLevel, Diagnostic, Origin } from '../bridge/contract';
import type { IconName } from '../icons/Icon';
import { Icon } from '../icons/Icon';
import type { DockTab } from '../shell/ShellContext';
import { projectState } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';
import { StateView } from './StateView';
import { AssetsTab } from './assets/AssetsTab';
import { useAssets } from '../assets/AssetsContext';

const TABS: { id: DockTab; label: string; icon: IconName }[] = [
  { id: 'assets', label: 'Assets', icon: 'texture' },
  { id: 'problems', label: 'Problems', icon: 'problems' },
  { id: 'console', label: 'Console', icon: 'console' },
  { id: 'history', label: 'History', icon: 'history' },
];

/** Assets, Problems, Console and History share the dock under the viewport (ARIA tabs). */
/** `onRequestHeight` lets a tab ask for room (never shrinks the dock; the layout clamp still applies). */
export function BottomDock({ onRequestHeight }: { onRequestHeight?: (min: number) => void }) {
  const { snapshot, dockTab, setDockTab } = useShell();
  const { pending, outcome } = useAssets();
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const problems = snapshot?.diagnostics ?? [];
  const errors = problems.filter((d) => d.severity === 'error').length;
  const assets = snapshot?.assets?.status === 'ready' ? snapshot.assets.value.length : 0;
  const counts: Record<DockTab, string | null> = {
    assets: assets ? String(assets) : null,
    problems: problems.length ? String(problems.length) : null,
    console: snapshot?.console.length ? String(snapshot.console.length) : null,
    history: snapshot?.history.entries.length ? String(snapshot.history.entries.length) : null,
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = TABS.findIndex((tab) => tab.id === dockTab);
    let next = index;
    if (event.key === 'ArrowRight') next = (index + 1) % TABS.length;
    else if (event.key === 'ArrowLeft') next = (index - 1 + TABS.length) % TABS.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = TABS.length - 1;
    else return;
    event.preventDefault();
    const tab = TABS[next];
    if (!tab) return;
    setDockTab(tab.id);
    tabRefs.current[tab.id]?.focus();
  };

  return (
    <section className="panel panel--dock" data-region="dock" aria-label="Assets and output" tabIndex={-1}>
      <div className="tabs" role="tablist" aria-label="Output" onKeyDown={onKeyDown}>
        {TABS.map((tab) => (
          <button
            key={tab.id}
            ref={(element) => {
              tabRefs.current[tab.id] = element;
            }}
            type="button"
            role="tab"
            id={`dock-tab-${tab.id}`}
            className="tab"
            aria-selected={dockTab === tab.id}
            aria-controls={`dock-panel-${tab.id}`}
            tabIndex={dockTab === tab.id ? 0 : -1}
            data-focus-target={dockTab === tab.id ? '' : undefined}
            onClick={() => setDockTab(tab.id)}
          >
            <Icon name={tab.icon} size={14} />
            {tab.label}
            {tab.id === 'assets' && pending ? (
              <span className="tab__busy" title="Import running">
                <span className="spinner spinner--small" aria-hidden="true" />
                <span className="visually-hidden">(importing)</span>
              </span>
            ) : tab.id === 'assets' && outcome?.status === 'failure' && dockTab !== 'assets' ? (
              <span className="tab__count tab__count--error" title="The last import failed">
                <span className="visually-hidden">(</span>!<span className="visually-hidden"> last import failed)</span>
              </span>
            ) : counts[tab.id] ? (
              <span className={`tab__count${tab.id === 'problems' && errors ? ' tab__count--error' : ''}`}>
                <span className="visually-hidden">(</span>
                {counts[tab.id]}
                <span className="visually-hidden">)</span>
              </span>
            ) : null}
          </button>
        ))}
      </div>
      <div
        className="tabpanel"
        role="tabpanel"
        id={`dock-panel-${dockTab}`}
        aria-labelledby={`dock-tab-${dockTab}`}
      >
        {dockTab === 'assets' ? (
          <AssetsTab onRequestHeight={onRequestHeight} />
        ) : dockTab === 'problems' ? (
          <ProblemsList />
        ) : dockTab === 'console' ? (
          <ConsoleList />
        ) : (
          <HistoryList />
        )}
      </div>
    </section>
  );
}

const SEVERITY_RANK = { error: 0, warning: 1, info: 2 } as const;
const SEVERITY_ICON = { error: 'error', warning: 'warning', info: 'info' } as const;

function ProblemsList() {
  const { snapshot, select, announce } = useShell();
  const sorted = useMemo(
    () => [...(snapshot?.diagnostics ?? [])].sort((a, b) => SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity]),
    [snapshot?.diagnostics],
  );
  if (!snapshot) return <StateView icon="plug" title="No engine connected" compact />;
  const project = projectState(snapshot);
  // Nothing has been validated while the project opens; never claim "No problems" then.
  if (project.kind === 'loading') {
    return (
      <StateView icon="problems" spinner title="Opening project…" compact>
        <p>Problems appear when validation finishes.</p>
      </StateView>
    );
  }
  if (sorted.length === 0 && project.kind === 'failed') {
    return (
      <StateView icon="problems" title="Not validated" compact>
        <p>The project did not open, so it has not been checked.</p>
      </StateView>
    );
  }
  if (sorted.length === 0) {
    return (
      <StateView icon="problems" title="No problems" compact>
        <p>Validation errors and warnings from the engine appear here.</p>
      </StateView>
    );
  }
  const nodes = snapshot.hierarchy.status === 'ready' ? snapshot.hierarchy.value.nodes : {};
  const reveal = (d: Diagnostic) => {
    if (d.entity === null) return;
    select(d.entity);
    announce(`Selected ${nodes[d.entity]?.name ?? 'entity'}`);
  };
  return (
    <ul className="list" aria-label="Problems">
      {sorted.map((d) => {
        const entity = d.entity !== null ? nodes[d.entity] : undefined;
        const where = [entity?.name, d.component?.replace(/^incant\./, ''), d.path].filter(Boolean).join(' · ');
        const content = (
          <>
            <Icon name={SEVERITY_ICON[d.severity]} size={14} className={`sev sev--${d.severity}`} />
            <span className="visually-hidden">{d.severity}: </span>
            <span className="list__main">{d.message}</span>
            {where ? <span className="list__meta mono">{where}</span> : <span className="list__meta">Project</span>}
          </>
        );
        return (
          <li key={d.id}>
            {d.entity !== null ? (
              <button type="button" className="list__row list__row--action" onClick={() => reveal(d)}>
                {content}
              </button>
            ) : (
              <div className="list__row">{content}</div>
            )}
          </li>
        );
      })}
    </ul>
  );
}

const LEVELS: ConsoleLevel[] = ['error', 'warn', 'info', 'debug'];
const LEVEL_LABEL: Record<ConsoleLevel, string> = { error: 'Errors', warn: 'Warnings', info: 'Info', debug: 'Debug' };

function ConsoleList() {
  const { snapshot } = useShell();
  const [hidden, setHidden] = useState<Set<ConsoleLevel>>(() => new Set(['debug']));
  const entries = snapshot?.console ?? [];
  const visible = entries.filter((entry) => !hidden.has(entry.level));
  return (
    <div className="console">
      <div className="console__filters" role="group" aria-label="Show console levels">
        {LEVELS.map((level) => {
          const count = entries.filter((entry) => entry.level === level).length;
          return (
            <button
              key={level}
              type="button"
              className={`toggle toggle--${level}`}
              aria-pressed={!hidden.has(level)}
              onClick={() =>
                setHidden((current) => {
                  const next = new Set(current);
                  if (next.has(level)) next.delete(level);
                  else next.add(level);
                  return next;
                })
              }
            >
              {LEVEL_LABEL[level]} <span className="toggle__count">{count}</span>
            </button>
          );
        })}
      </div>
      {!snapshot ? (
        <StateView icon="plug" title="No engine connected" compact />
      ) : visible.length === 0 ? (
        <StateView icon="console" title={entries.length ? 'All messages are filtered out' : 'Console is empty'} compact />
      ) : (
        <div className="console__log" role="log" aria-label="Console messages">
        <ol className="list list--console">
          {visible.map((entry) => (
            <li key={entry.id} className={`list__row console__row console__row--${entry.level}`}>
              <time className="mono subtle" dateTime={entry.at}>
                {formatTime(entry.at)}
              </time>
              <span className={`console__level console__level--${entry.level}`}>{entry.level}</span>
              <span className="console__source mono">{entry.source}</span>
              <span className="list__main">{entry.message}</span>
            </li>
          ))}
        </ol>
        </div>
      )}
    </div>
  );
}

function originLabel(origin: Origin): { label: string; detail: string } {
  switch (origin.kind) {
    case 'user':
      return { label: 'User', detail: '' };
    case 'agent':
      return { label: 'Agent', detail: origin.model };
    case 'script':
      return { label: 'Script', detail: origin.script };
    case 'import':
      return { label: 'Import', detail: origin.source };
  }
}

function HistoryList() {
  const { snapshot, run } = useShell();
  if (!snapshot) return <StateView icon="plug" title="No engine connected" compact />;
  const project = projectState(snapshot);
  // Without an open project there is no history to undo; no toolbar or "0 of 0" claim.
  if (project.kind === 'loading') return <StateView icon="history" spinner title="Opening project…" compact />;
  if (project.kind === 'failed') {
    return (
      <StateView icon="history" title="No project loaded" compact>
        <p>Edit history appears once the project opens.</p>
      </StateView>
    );
  }
  const { entries, applied } = snapshot.history;
  return (
    <div className="history">
      <div className="history__toolbar">
        <UnavailableAwareButton
          icon="undo"
          label="Undo"
          shortcut="⌘Z / Ctrl+Z"
          disabled={applied === 0}
          onActivate={() => void run({ type: 'history.undo' })}
          capability="history.undo"
        />
        <UnavailableAwareButton
          icon="redo"
          label="Redo"
          shortcut="⇧⌘Z / Ctrl+Y"
          disabled={applied >= entries.length}
          onActivate={() => void run({ type: 'history.redo' })}
          capability="history.redo"
        />
        <span className="panel__meta">
          {applied} of {entries.length} applied
        </span>
      </div>
      {entries.length === 0 ? (
        <StateView icon="history" title="No history yet" compact>
          <p>Every edit, by you, the agent, a script or an import, is recorded here and can be undone.</p>
        </StateView>
      ) : (
        <ol className="list" aria-label="Edit history, oldest first">
          {entries.map((entry, index) => {
            const { label, detail } = originLabel(entry.origin);
            const undone = index >= applied;
            return (
              <li key={entry.transaction} className={`list__row history__row${undone ? ' is-undone' : ''}`}>
                <span className={`origin origin--${entry.origin.kind}`}>{label}</span>
                <span className="list__main">
                  {entry.description}
                  {undone ? <span className="history__undone">(undone)</span> : null}
                </span>
                {detail ? <span className="list__meta mono">{detail}</span> : null}
                <time className="list__meta mono" dateTime={entry.at}>
                  {formatTime(entry.at)}
                </time>
              </li>
            );
          })}
        </ol>
      )}
    </div>
  );
}

function UnavailableAwareButton({
  icon,
  label,
  shortcut,
  disabled,
  onActivate,
  capability,
}: {
  icon: IconName;
  label: string;
  shortcut: string;
  disabled: boolean;
  onActivate: () => void;
  capability: 'history.undo' | 'history.redo';
}) {
  const { capabilities } = useShell();
  const unavailable = !capabilities.has(capability);
  return (
    <button
      type="button"
      className="button button--ghost"
      // aria-disabled keeps the button focusable so activating it can explain why it is unavailable.
      aria-disabled={disabled || unavailable || undefined}
      aria-keyshortcuts={shortcut.includes('Ctrl+Y') ? 'Meta+Shift+Z Control+Y' : 'Meta+Z Control+Z'}
      title={`${label} (${shortcut})`}
      onClick={() => {
        if (disabled && !unavailable) return;
        onActivate();
      }}
    >
      <Icon name={icon} size={14} />
      {label}
    </button>
  );
}

function formatTime(iso: string) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  // Local wall-clock time; the full ISO instant stays in the <time dateTime> attribute.
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}
