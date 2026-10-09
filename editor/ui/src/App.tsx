import { useCallback, useEffect, useRef, useState } from 'react';
import type { BridgeResolution } from './bridge/resolve';
import { FIXTURE_VARIANTS } from './bridge/fixture';
import { PROVIDER_FIXTURE_NAMES } from './bridge/providerFixture';
import { UI_PROTOCOL_VERSION } from './bridge/contract';
import { AccountDialog } from './components/AccountDialog';
import { AgentPanel } from './components/AgentPanel';
import { BottomDock } from './components/BottomDock';
import { HierarchyPanel } from './components/HierarchyPanel';
import { InspectorPanel } from './components/inspector/InspectorPanel';
import { ShortcutsDialog } from './components/ShortcutsDialog';
import { Splitter } from './components/Splitter';
import { StateView } from './components/StateView';
import { Titlebar } from './components/Titlebar';
import type { PanelVisibility } from './components/Titlebar';
import { ViewportPanel } from './components/ViewportPanel';
import { Icon } from './icons/Icon';
import { ShellProvider, useShell } from './shell/ShellContext';
import { installInputModality } from './shell/inputModality';
import { projectState } from './shell/projectState';
import { focusRegion, nextRegion, regionOf } from './shell/regions';
import { clampLayout, defaultLayout } from './shell/layout';
import type { Layout } from './shell/layout';

export function App({ resolution }: { resolution: BridgeResolution }) {
  if (resolution.kind === 'incompatible') {
    return (
      <FatalScreen title="Engine bridge version mismatch">
        <p>
          This editor UI speaks bridge protocol {UI_PROTOCOL_VERSION}; the engine offered protocol{' '}
          {resolution.protocolVersion}. The UI will not guess at an unknown protocol.
        </p>
        <p className="subtle">Rebuild the editor so the UI and engine come from the same version.</p>
      </FatalScreen>
    );
  }
  if (resolution.kind === 'bad-fixture') {
    return (
      <FatalScreen title="Unknown sample fixture">
        <p>
          No fixture is named <code>{resolution.requested}</code>.
        </p>
        <p className="subtle">Available: {FIXTURE_VARIANTS.join(', ')}.</p>
        <p className="subtle">Account states (provider=): {PROVIDER_FIXTURE_NAMES.join(', ')}.</p>
      </FatalScreen>
    );
  }
  return (
    <ShellProvider bridge={resolution.kind === 'bridge' ? resolution.bridge : null}>
      <Workbench />
    </ShellProvider>
  );
}

function FatalScreen({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <main className="fatal">
      <StateView icon="error" tone="error" title={title}>
        {children}
      </StateView>
    </main>
  );
}

function Workbench() {
  const shell = useShell();
  const { bridge, snapshot, run, accountOpen } = shell;
  const [layout, setLayout] = useState<Layout>(() => defaultLayout(window.innerWidth, window.innerHeight));
  const [panels, setPanels] = useState<PanelVisibility>({ hierarchy: true, dock: true, inspector: true });
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const shortcutsReturn = useRef<HTMLElement | null>(null);

  // Keeps keyboard focus visible where WebKit omits :focus-visible after script focus.
  useEffect(() => installInputModality(document), []);

  const attached = snapshot?.viewport.status === 'attached';
  // With a native surface attached, the page background must not paint over it.
  useEffect(() => {
    document.documentElement.classList.toggle('native-viewport', attached);
    return () => document.documentElement.classList.remove('native-viewport');
  }, [attached]);

  useEffect(() => {
    const onResize = () => setLayout((current) => clampLayout(current, window.innerWidth, window.innerHeight));
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  const runHistory = useCallback((action: 'undo' | 'redo') => {
    const history = snapshot?.history;
    if (!history || (action === 'undo' ? history.applied === 0 : history.applied === history.entries.length)) return;
    void run({ type: action === 'undo' ? 'history.undo' : 'history.redo' });
  }, [run, snapshot?.history]);

  useEffect(() => bridge?.subscribeHistoryRequests?.((action) => {
    if (isTextEntry(document.activeElement)) {
      // WebKit owns the text edit buffer, including React-controlled inputs.
      // No standard API exposes its undo stack. Never fall back to project undo
      // if this draft has nothing to undo or the browser refuses the command.
      document.execCommand(action);
      return;
    }
    if (shortcutsOpen || accountOpen || document.activeElement?.closest('[role="dialog"]')) return;
    runHistory(action);
  }), [bridge, runHistory, shortcutsOpen, accountOpen]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || shortcutsOpen || accountOpen) return;
      const mod = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (event.key === 'F6') {
        event.preventDefault();
        focusRegion(nextRegion(regionOf(document.activeElement), event.shiftKey ? -1 : 1));
        return;
      }
      if (isTextEntry(event.target)) return;
      if (mod && key === 'z') {
        event.preventDefault();
        runHistory(event.shiftKey ? 'redo' : 'undo');
      } else if (event.ctrlKey && !event.metaKey && key === 'y') {
        event.preventDefault();
        runHistory('redo');
      } else if ((event.key === '?' && !mod) || (mod && event.key === '/')) {
        event.preventDefault();
        shortcutsReturn.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        setShortcutsOpen(true);
      } else if (mod && event.altKey && ['1', '2', '3'].includes(event.code.slice(-1)) && event.code.startsWith('Digit')) {
        event.preventDefault();
        const panel = (['hierarchy', 'dock', 'inspector'] as const)[Number(event.code.slice(-1)) - 1]!;
        setPanels((current) => ({ ...current, [panel]: !current[panel] }));
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [runHistory, shortcutsOpen, accountOpen]);

  const openShortcuts = (from: HTMLElement) => {
    shortcutsReturn.current = from;
    setShortcutsOpen(true);
  };
  const resize = (patch: Partial<Layout>) =>
    setLayout((current) => clampLayout({ ...current, ...patch }, window.innerWidth, window.innerHeight));

  const columns = [
    panels.hierarchy ? `${layout.left}px var(--splitter-size)` : '',
    'minmax(0, 1fr)',
    panels.inspector ? `var(--splitter-size) ${layout.right}px` : '',
  ].join(' ');
  return (
    <div className="app">
      <a
        className="skip-link"
        href={panels.hierarchy ? '#hierarchy-title' : '#viewport-title'}
        onClick={(event) => {
          event.preventDefault();
          focusRegion(panels.hierarchy ? 'hierarchy' : 'viewport');
        }}
      >
        Skip to {panels.hierarchy ? 'hierarchy' : 'viewport'}
      </a>
      <Titlebar
        panels={panels}
        onTogglePanel={(panel) => setPanels((current) => ({ ...current, [panel]: !current[panel] }))}
        onShortcuts={openShortcuts}
      />
      <ConnectionBanner />
      <main className="workspace" style={{ gridTemplateColumns: columns }} aria-label="Editor">
        {panels.hierarchy ? (
          <>
            <div className="column">
              <HierarchyPanel />
            </div>
            <Splitter label="Resize hierarchy" orientation="vertical" value={layout.left} min={200} max={480} onChange={(left) => resize({ left })} />
          </>
        ) : null}
        <div
          className="column column--center"
          style={{ gridTemplateRows: panels.dock ? `minmax(0, 1fr) var(--splitter-size) ${layout.dock}px` : 'minmax(0, 1fr)' }}
        >
          <ViewportPanel />
          {panels.dock ? (
            <>
              <Splitter label="Resize output panel" orientation="horizontal" invert value={layout.dock} min={120} max={640} onChange={(dock) => resize({ dock })} />
              <BottomDock />
            </>
          ) : null}
        </div>
        {panels.inspector ? (
          <>
            <Splitter label="Resize inspector and agent" orientation="vertical" invert value={layout.right} min={280} max={560} onChange={(right) => resize({ right })} />
            <div className="column column--side" style={{ gridTemplateRows: `minmax(0, 1fr) var(--splitter-size) ${layout.agent}px` }}>
              <InspectorPanel />
              <Splitter label="Resize agent panel" orientation="horizontal" invert value={layout.agent} min={180} max={560} onChange={(agent) => resize({ agent })} />
              <AgentPanel />
            </div>
          </>
        ) : null}
      </main>
      <StatusBar />
      {shortcutsOpen ? (
        <ShortcutsDialog
          onClose={() => {
            setShortcutsOpen(false);
            shortcutsReturn.current?.focus();
          }}
        />
      ) : null}
      {accountOpen ? <AccountDialog /> : null}
    </div>
  );
}

function isTextEntry(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLTextAreaElement) return !target.readOnly;
  if (target instanceof HTMLInputElement) return !target.readOnly;
  return target.isContentEditable;
}

/**
 * Slim notice under the titlebar, only when the engine is absent, the project is
 * opening, or it failed. This is the one place the full failure text is shown;
 * panels show a short state and Problems lists the engine's diagnostic.
 */
function ConnectionBanner() {
  const { snapshot, bridge } = useShell();
  const project = projectState(snapshot);
  let content = null;
  if (!bridge) {
    content = (
      <div className="notice notice--info" role="note">
        <Icon name="plug" size={14} />
        <span>
          <strong>No engine connected.</strong> You are looking at the editor shell. Open a project from the Incant
          editor to start.
        </span>
      </div>
    );
  } else if (project.kind === 'loading') {
    content = (
      <div className="notice notice--info" role="status">
        <span className="spinner" aria-hidden="true" />
        <span>Opening project…</span>
      </div>
    );
  } else if (project.kind === 'failed') {
    const { error, projectError } = project;
    // Reopening is the retry path. Say so once, unless the engine's message already does.
    const hint = /\breopen\b/i.test(error.message) ? '' : ' Reopen the project to try again.';
    content = (
      <div className="notice notice--error" role="alert">
        <Icon name="error" size={14} />
        <span>
          <strong>{projectError ? 'The project could not be opened.' : 'Lost connection to the editor process.'}</strong>{' '}
          {error.message}
          {hint}
        </span>
        <span className="notice__code mono">{error.code}</span>
      </div>
    );
  }
  if (!content) return null;
  return (
    <div className="notice-region" role="region" aria-label="Project status">
      {content}
    </div>
  );
}

function StatusBar() {
  const { snapshot, bridge, message, capabilities } = useShell();
  const connection = snapshot?.connection.status;
  const tone = !bridge ? 'idle' : bridge.isFixture ? 'fixture' : connection === 'ready' ? 'ok' : connection === 'error' ? 'error' : 'pending';
  const errors = snapshot?.diagnostics.filter((d) => d.severity === 'error').length ?? 0;
  const warnings = snapshot?.diagnostics.filter((d) => d.severity === 'warning').length ?? 0;
  // While the project is opening nothing has been validated, so no counts are claimed.
  const loading = projectState(snapshot).kind === 'loading';
  return (
    <footer className="statusbar">
      <span className={`statusbar__conn statusbar__conn--${tone}`}>
        <span className="statusbar__dot" aria-hidden="true" />
        {bridge ? bridge.label : 'No engine'}
      </span>
      {snapshot && !loading ? (
        <span className="statusbar__item">
          <Icon name="error" size={12} className="sev sev--error" />
          {errors} <span className="visually-hidden">errors,</span>
          <Icon name="warning" size={12} className="sev sev--warning" />
          {warnings} <span className="visually-hidden">warnings</span>
        </span>
      ) : null}
      <span className="statusbar__message" role="status" aria-live="polite">
        {message}
      </span>
      {capabilities.unknown.length > 0 ? (
        <span className="statusbar__item" title={capabilities.unknown.join(', ')}>
          {capabilities.unknown.length} unknown capabilities ignored
        </span>
      ) : null}
      <span className="statusbar__hint">
        <kbd>F6</kbd> next panel · <kbd>?</kbd> shortcuts
      </span>
    </footer>
  );
}
