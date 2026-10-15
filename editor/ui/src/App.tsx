import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { BridgeResolution } from './bridge/resolve';
import { FIXTURE_VARIANTS } from './bridge/fixture';
import { PROVIDER_FIXTURE_NAMES } from './bridge/providerFixture';
import { UI_PROTOCOL_VERSION } from './bridge/contract';
import { AssetsProvider, useAssets } from './assets/AssetsContext';
import { AccountDialog } from './components/AccountDialog';
import { AgentPanel, agentPaneResizable } from './components/AgentPanel';
import { BottomDock } from './components/BottomDock';
import { HierarchyPanel } from './components/HierarchyPanel';
import { InspectorPanel } from './components/inspector/InspectorPanel';
import { PinDialog } from './components/PinDialog';
import { ShortcutsDialog } from './components/ShortcutsDialog';
import { Splitter } from './components/Splitter';
import { Titlebar } from './components/Titlebar';
import { ViewportPanel } from './components/ViewportPanel';
import { Icon } from './icons/Icon';
import { PlayProvider, usePlay } from './play/PlayContext';
import { ProjectFileProvider, useProjectFile } from './projectFile/ProjectFileContext';
import { ShellProvider, useShell } from './shell/ShellContext';
import { installInputModality } from './shell/inputModality';
import { projectState } from './shell/projectState';
import { focusRegion, nextRegion, regionOf } from './shell/regions';
import { clampLayout, defaultLayout } from './shell/layout';
import type { Layout } from './shell/layout';
import { agentViewContext } from './shell/agentContext';

/** Below this window height the agent pane starts collapsed (its status line stays visible). */
export const AGENT_OPEN_MIN_HEIGHT = 720;

export function App({ resolution }: { resolution: BridgeResolution }) {
  if (resolution.kind === 'incompatible') {
    return (
      <FatalScreen title="Engine bridge version mismatch">
        <p>
          This editor UI speaks bridge protocol {UI_PROTOCOL_VERSION}; the engine offered protocol{' '}
          {resolution.protocolVersion}.
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
        <FatalChoices title="Sample fixtures (fixture=)" names={FIXTURE_VARIANTS} />
        <FatalChoices title="Account states (provider=)" names={PROVIDER_FIXTURE_NAMES} />
      </FatalScreen>
    );
  }
  return (
    <ShellProvider bridge={resolution.kind === 'bridge' ? resolution.bridge : null}>
      <ProjectFileProvider>
        <AssetsProvider>
          <PlayProvider>
            <Workbench />
          </PlayProvider>
        </AssetsProvider>
      </ProjectFileProvider>
    </ShellProvider>
  );
}

/**
 * The whole window when the shell cannot start: the problem as the page's heading and
 * what to do next, as plain text. Announced as an alert.
 */
function FatalScreen({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <main className="fatal">
      <div className="fatal__content" role="alert">
        <h1 className="fatal__title">
          <Icon name="error" size={16} className="fatal__icon" />
          {title}
        </h1>
        <div className="fatal__body">{children}</div>
      </div>
    </main>
  );
}

/** Valid names for a URL parameter, as a labelled list. */
function FatalChoices({ title, names }: { title: string; names: readonly string[] }) {
  return (
    <div className="fatal__choices">
      <p className="fatal__choices-title">{title}</p>
      <ul className="fatal__chips" aria-label={title}>
        {names.map((name) => (
          <li key={name}>
            <code>{name}</code>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Workbench() {
  const shell = useShell();
  const assets = useAssets();
  const { bridge, snapshot, run, accountOpen, panels, togglePanel } = shell;
  // The Play actions of the latest commit: a shortcut pressed right after a state change
  // must not reach the previous render's action (Step while it still thinks "running").
  const play = usePlay();
  const projectFile = useProjectFile();
  const pinOpen = projectFile.pin.status !== 'closed';
  const mac = snapshot?.window ? snapshot.window.platform === 'macos' : /Mac|iPhone|iPad/.test(`${navigator.platform} ${navigator.userAgent}`);
  const playActions = useRef(play);
  const save = useRef(projectFile.save);
  useLayoutEffect(() => {
    playActions.current = play;
    save.current = projectFile.save;
  });
  const [layout, setLayout] = useState<Layout>(() => defaultLayout(window.innerWidth, window.innerHeight));
  // The agent pane's disclosure; collapsing leaves its one-line status. A short window
  // starts with it collapsed, so the Inspector keeps the column; the choice then sticks.
  const [agentOpen, setAgentOpen] = useState(() => window.innerHeight >= AGENT_OPEN_MIN_HEIGHT);
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
      // if this draft has nothing to undo or the browser refuses the command
      // (or no longer offers execCommand at all).
      if (typeof document.execCommand === 'function') document.execCommand(action);
      return;
    }
    if (shortcutsOpen || accountOpen || pinOpen || document.activeElement?.closest('[role="dialog"]')) return;
    runHistory(action);
  }), [bridge, runHistory, shortcutsOpen, accountOpen, pinOpen]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // A key the focused game view sent to the game arrives here already handled.
      if (event.defaultPrevented) return;
      if (isSaveShortcut(event, mac)) {
        // The browser's own Save Page never applies to the editor, so the keys are always
        // taken. Inside a dialog they do nothing; elsewhere, text fields included, they are
        // the project menu's Save, sent to the host like it.
        event.preventDefault();
        const inDialog = shortcutsOpen || accountOpen || pinOpen || document.activeElement?.closest('[role="dialog"]');
        if (!event.repeat && !inDialog) save.current();
        return;
      }
      if (shortcutsOpen || accountOpen || pinOpen) return;
      const mod = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (event.key === 'F6') {
        event.preventDefault();
        focusRegion(nextRegion(regionOf(document.activeElement), event.shiftKey ? -1 : 1));
        return;
      }
      // Play shortcuts by physical key, as the panel shortcuts are: Mod+P plays or stops,
      // with Shift pauses or resumes, with Alt steps. They work from the game view too.
      // In a text field only the platform's own modifier counts: Ctrl+P moves up a line
      // there on macOS, and Ctrl+Alt elsewhere may be AltGr typing a character.
      const playKeys = !isTextEntry(event.target) || (mac ? event.metaKey : event.ctrlKey && !event.altKey);
      if (mod && event.code === 'KeyP' && playKeys) {
        event.preventDefault();
        if (event.repeat) return;
        if (event.altKey) playActions.current.step();
        else if (event.shiftKey) playActions.current.togglePause();
        else playActions.current.playOrStop();
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
        togglePanel(panel);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [runHistory, shortcutsOpen, accountOpen, pinOpen, togglePanel, mac]);

  const openShortcuts = (from: HTMLElement) => {
    shortcutsReturn.current = from;
    setShortcutsOpen(true);
  };
  const resize = (patch: Partial<Layout>) =>
    setLayout((current) => clampLayout({ ...current, ...patch }, window.innerWidth, window.innerHeight));

  // An open agent that is ready, or that holds turns from its session, is a resizable pane.
  // Otherwise the pane is as tall as its content: one status line, or a short note.
  const agentResizable = agentOpen && agentPaneResizable(snapshot);

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
        onTogglePanel={togglePanel}
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
            <div
              className="column column--side"
              style={{ gridTemplateRows: agentResizable ? `minmax(0, 1fr) var(--splitter-size) ${layout.agent}px` : 'minmax(0, 1fr) auto' }}
            >
              <InspectorPanel />
              {agentResizable ? (
                <Splitter label="Resize agent panel" orientation="horizontal" invert value={layout.agent} min={180} max={560} onChange={(agent) => resize({ agent })} />
              ) : null}
              <AgentPanel open={agentOpen} onToggle={() => setAgentOpen((open) => !open)}
                viewContext={agentViewContext(snapshot?.connection.status === 'ready' ? snapshot.connection.project.id : undefined,
                  panels, assets.view, shell.dockTab, agentOpen)} />
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
      {pinOpen ? <PinDialog /> : null}
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
 * ⌘S on macOS, Ctrl+S elsewhere: by the S of the current layout, or by the physical S key
 * on a layout without Latin letters. Shift, Option and AltGr combinations are left alone:
 * there is no Save As, and AltGr types characters.
 */
function isSaveShortcut(event: KeyboardEvent, mac: boolean): boolean {
  if (event.shiftKey || event.altKey || (mac ? !event.metaKey || event.ctrlKey : !event.ctrlKey || event.metaKey)) return false;
  const key = event.key.toLowerCase();
  return key === 's' || (event.code === 'KeyS' && !/^[a-z]$/.test(key));
}

/**
 * Slim notice under the titlebar, only when the engine is absent, the project is
 * opening, or it failed, and after a project file request failed outside a dialog. This is
 * the one place the full failure text is shown; panels show a short state and Problems
 * lists the engine's diagnostic.
 */
function ConnectionBanner() {
  const { snapshot, bridge } = useShell();
  const { notice, dismissNotice } = useProjectFile();
  const project = projectState(snapshot);
  let content = null;
  if (!bridge) {
    content = (
      <div className="notice notice--info" role="note">
        <Icon name="plug" size={14} />
        <span>
          <strong>No engine connected.</strong> Open a project from the Incant editor to start.
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
  if (!content && !notice) return null;
  return (
    <div className="notice-region" role="region" aria-label="Project status">
      {content}
      {notice ? (
        // A failed save, or a pin that failed after its dialog closed, stays in view until
        // dismissed, superseded or another project opens. The host's words are shown as
        // they are, with no added explanation of the cause.
        <div className="notice notice--error" role="alert">
          <Icon name="error" size={14} />
          <span className="notice__text">
            <strong>{notice.title}</strong> {notice.failure.message}
          </span>
          {notice.failure.code !== null ? <span className="notice__code mono">{notice.failure.code}</span> : null}
          <button
            type="button"
            className="button button--ghost button--small notice__dismiss"
            onClick={(event) => {
              // The notice goes with its button: keyboard focus continues at the project menu.
              if (document.activeElement === event.currentTarget) document.querySelector<HTMLElement>('.titlebar__project-button')?.focus();
              dismissNotice();
            }}
          >
            Dismiss
          </button>
        </div>
      ) : null}
    </div>
  );
}

function StatusBar() {
  const { snapshot, bridge, message, capabilities, showPanel, setDockTab } = useShell();
  const connection = snapshot?.connection.status;
  const tone = !bridge ? 'idle' : bridge.isFixture ? 'fixture' : connection === 'ready' ? 'ok' : connection === 'error' ? 'error' : 'pending';
  const errors = snapshot?.diagnostics.filter((d) => d.severity === 'error').length ?? 0;
  const warnings = snapshot?.diagnostics.filter((d) => d.severity === 'warning').length ?? 0;
  // While the project is opening nothing has been validated, so no counts are claimed;
  // a project that failed to open is not "clear" either, only its reported problems count.
  const project = projectState(snapshot).kind;
  const unvalidated = project === 'loading' || (project === 'failed' && errors === 0 && warnings === 0);
  return (
    <footer className="statusbar">
      <span className={`statusbar__conn statusbar__conn--${tone}`}>
        <span className="statusbar__dot" aria-hidden="true" />
        {bridge ? bridge.label : 'No engine'}
      </span>
      {snapshot && !unvalidated ? (
        <button
          type="button"
          className={`statusbar__problems${errors ? ' has-errors' : warnings ? ' has-warnings' : ' is-clear'}`}
          title="Show problems"
          onClick={() => {
            showPanel('dock');
            setDockTab('problems');
          }}
        >
          {errors === 0 && warnings === 0 ? (
            <>
              <Icon name="check" size={12} className="sev sev--clear" />
              No problems
            </>
          ) : (
            <>
              {errors ? (
                <span className="statusbar__count">
                  <Icon name="error" size={12} className="sev sev--error" />
                  {errors} <span className="visually-hidden">{errors === 1 ? 'error' : 'errors'}{warnings ? ', ' : ''}</span>
                </span>
              ) : null}
              {warnings ? (
                <span className="statusbar__count">
                  <Icon name="warning" size={12} className="sev sev--warning" />
                  {warnings} <span className="visually-hidden">{warnings === 1 ? 'warning' : 'warnings'}</span>
                </span>
              ) : null}
            </>
          )}
        </button>
      ) : null}
      <span className="statusbar__message" role="status" aria-live="polite">
        {message}
      </span>
      {capabilities.unknown.length > 0 ? (
        <span className="statusbar__item" title={capabilities.unknown.join(', ')}>
          {capabilities.unknown.length} unknown capabilities ignored
        </span>
      ) : null}
    </footer>
  );
}
