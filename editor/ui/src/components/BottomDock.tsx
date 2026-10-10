import { useRef } from 'react';
import type { KeyboardEvent } from 'react';
import type { DockTab } from '../shell/ShellContext';
import { useShell } from '../shell/ShellContext';
import { ConsoleList } from './dock/ConsoleView';
import { HistoryList } from './dock/HistoryView';
import { ProblemsList } from './dock/ProblemsView';

const TABS: { id: DockTab; label: string }[] = [
  { id: 'problems', label: 'Problems' },
  { id: 'console', label: 'Console' },
  { id: 'history', label: 'History' },
];

/**
 * Problems, Console and History share the dock under the viewport (ARIA tabs). Each
 * view is in `dock/`; all three mount at most one page of rows (`dock/paging.tsx`).
 */
export function BottomDock() {
  const { snapshot, dockTab, setDockTab } = useShell();
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const problems = snapshot?.diagnostics ?? [];
  const errors = problems.filter((d) => d.severity === 'error').length;
  const counts: Record<DockTab, string | null> = {
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
    <section className="panel panel--dock" data-region="dock" aria-label="Problems, console and history" tabIndex={-1}>
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
            {tab.label}
            {counts[tab.id] ? (
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
        {dockTab === 'problems' ? <ProblemsList /> : dockTab === 'console' ? <ConsoleList /> : <HistoryList />}
      </div>
    </section>
  );
}
