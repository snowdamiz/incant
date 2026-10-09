import { useEffect, useRef, useState } from 'react';
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
  const { snapshot, run, accountOpen } = shell;
  const [layout, setLayout] = useState<Layout>(() => defaultLayout(window.innerWidth, window.innerHeight));
  const [panels, setPanels] = useState<PanelVisibility>({ hierarchy: true, dock: true, inspector: true });
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const shortcutsReturn = useRef<HTMLElement | null>(null);

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
        void run({ type: event.shiftKey ? 'history.redo' : 'history.undo' });
      } else if (event.ctrlKey && !event.metaKey && key === 'y') {
        event.preventDefault();
        void run({ type: 'history.redo' });
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
  }, [run, shortcutsOpen, accountOpen]);

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
            <div className="column" style={{ gridTemplateRows: `minmax(0, 1fr) var(--splitter-size) ${layout.agent}px` }}>
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

/** Slim notice under the titlebar, only when the engine is absent, connecting or lost. */
function ConnectionBanner() {
  const { snapshot, bridge } = useShell();
  const connection = snapshot?.connection;
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
  } else if (connection?.status === 'connecting') {
    content = (
      <div className="notice notice--info" role="status">
        <span className="spinner" aria-hidden="true" />
        <span>Connecting to the editor process…</span>
      </div>
    );
  } else if (connection?.status === 'error') {
    content = (
      <div className="notice notice--error" role="alert">
        <Icon name="error" size={14} />
        <span>
          <strong>Lost connection to the editor process.</strong> {connection.error.message} Nothing shown is current.
        </span>
        <span className="notice__code mono">{connection.error.code}</span>
      </div>
    );
  }
  if (!content) return null;
  return (
    <div className="notice-region" role="region" aria-label="Connection status">
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
  return (
    <footer className="statusbar">
      <span className={`statusbar__conn statusbar__conn--${tone}`}>
        <span className="statusbar__dot" aria-hidden="true" />
        {bridge ? bridge.label : 'No engine'}
      </span>
      {snapshot ? (
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
