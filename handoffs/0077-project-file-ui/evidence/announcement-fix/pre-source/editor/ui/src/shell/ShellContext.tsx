import { createContext, useCallback, useContext, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import type { ReactNode } from 'react';
import type {
  BridgeResult,
  BridgeSnapshot,
  Capability,
  EditorBridge,
  EditorCommand,
  HostRequest,
  Ulid,
} from '../bridge/contract';
import type { CapabilitySet } from '../bridge/resolve';
import { readCapabilities, unavailableMessage } from '../bridge/resolve';
import type { PanelVisibility } from '../components/Titlebar';
import type { RequestResult } from './requests';
import { rejectedRequest } from './requests';

export type DockTab = 'problems' | 'console' | 'history';

/**
 * The bridge snapshot without Play status. Play status is read at up to 4 Hz while a
 * session runs; it is selected separately (src/play/PlayContext.tsx), so its reads do
 * not re-render every panel.
 */
export type AuthoringSnapshot = Omit<BridgeSnapshot, 'play' | 'playError'>;

export interface RequestOptions {
  /** The caller reports the outcome itself; a failure is not announced. */
  readonly quiet?: boolean;
}

export interface Shell {
  readonly panels: PanelVisibility;
  readonly showPanel: (panel: keyof PanelVisibility) => void;
  readonly togglePanel: (panel: keyof PanelVisibility) => void;
  readonly bridge: EditorBridge | null;
  readonly snapshot: AuthoringSnapshot | null;
  readonly capabilities: CapabilitySet;
  readonly selection: Ulid | null;
  readonly select: (id: Ulid | null) => void;
  readonly dockTab: DockTab;
  readonly setDockTab: (tab: DockTab) => void;
  readonly message: string;
  readonly announce: (message: string) => void;
  /** Checks the capability, dispatches, and announces any failure. Resolves true on success. */
  readonly run: (command: EditorCommand) => Promise<boolean>;
  /** Like `run`, but returns the bridge's result so a caller can show the exact error inline. */
  readonly dispatch: (command: EditorCommand) => Promise<BridgeResult>;
  readonly ask: (request: HostRequest) => Promise<boolean>;
  /**
   * Like `ask`, but returns the host's result so a caller can show the exact error inline.
   * Always resolves: a rejected bridge request carries outcomeUnknown. The same marker
   * is preserved when the native bridge cannot verify a file completion receipt.
   */
  readonly request: (request: HostRequest, options?: RequestOptions) => Promise<RequestResult>;
  readonly explainUnavailable: (capability: Capability) => void;
  /** The ChatGPT account dialog. `from` regains focus when it closes. */
  readonly accountOpen: boolean;
  readonly openAccount: (from: HTMLElement | null) => void;
  readonly closeAccount: () => void;
}

const ShellContext = createContext<Shell | null>(null);

/** Plain-language failure prefixes; other commands keep their command name. */
const FAILURE_LABEL: Partial<Record<EditorCommand['type'], string>> = { 'asset.import': 'Import failed', 'agent.budget.set': 'Token allowance not saved' };

const NO_CAPABILITIES = readCapabilities([]);
const noopSubscribe = () => () => undefined;
const nullSnapshot = () => null;

/** Play and project-file operations each share their host capability. */
export function capabilityOf(request: HostRequest): Capability {
  switch (request.type) {
    case 'project.pin.review':
    case 'project.pin.apply':
    case 'project.save':
      return 'project.file';
    case 'play.start':
    case 'play.pause':
    case 'play.resume':
    case 'play.step':
    case 'play.stop':
      return 'play.control';
    default:
      return request.type;
  }
}

/**
 * Reads the bridge snapshot without its Play status, returning the previous object when
 * only Play status changed, so a status read does not re-render the panels.
 */
function authoringReader(bridge: EditorBridge): () => AuthoringSnapshot {
  let raw: BridgeSnapshot | null = null;
  let shown: AuthoringSnapshot | null = null;
  return () => {
    const next = bridge.getSnapshot();
    if (next === raw && shown) return shown;
    const previous = raw;
    raw = next;
    const keys = Object.keys(next).filter((key) => key !== 'play' && key !== 'playError') as (keyof AuthoringSnapshot)[];
    const unchanged =
      shown !== null &&
      previous !== null &&
      keys.length === Object.keys(shown).length &&
      keys.every((key) => Object.hasOwn(previous, key) && next[key] === previous[key]);
    if (!unchanged || !shown) {
      const { play: _play, playError: _playError, ...rest } = next;
      shown = rest;
    }
    return shown;
  };
}

/** Capabilities can arrive with native metadata; retain identity between changes. */
function capabilityReader(bridge: EditorBridge | null): () => CapabilitySet {
  let previous: readonly string[] = [];
  let current = NO_CAPABILITIES;
  return () => {
    const next = bridge?.capabilities ?? [];
    if (next.length !== previous.length || next.some((value, index) => value !== previous[index])) {
      previous = [...next];
      current = readCapabilities(previous);
    }
    return current;
  };
}

export function ShellProvider({ bridge, children }: { bridge: EditorBridge | null; children: ReactNode }) {
  // Bound wrappers so a class-based bridge keeps its `this`.
  const subscribe = useMemo(
    () => (bridge ? (listener: () => void) => bridge.subscribe(listener) : noopSubscribe),
    [bridge],
  );
  const getSnapshot = useMemo(() => (bridge ? authoringReader(bridge) : nullSnapshot), [bridge]);
  const snapshot = useSyncExternalStore(subscribe, getSnapshot);
  const getCapabilities = useMemo(() => capabilityReader(bridge), [bridge]);
  const capabilities = getCapabilities();
  const [selection, setSelection] = useState<Ulid | null>(null);
  const [dockTab, setDockTab] = useState<DockTab>('problems');
  const [panels, setPanels] = useState<PanelVisibility>({ hierarchy: true, dock: true, inspector: true });
  const showPanel = useCallback((panel: keyof PanelVisibility) => {
    setPanels((current) => current[panel] ? current : { ...current, [panel]: true });
  }, []);
  const togglePanel = useCallback((panel: keyof PanelVisibility) => {
    setPanels((current) => ({ ...current, [panel]: !current[panel] }));
  }, []);
  const [message, setMessage] = useState('');
  const clearTimer = useRef<number | undefined>(undefined);

  const announce = useCallback((text: string) => {
    window.clearTimeout(clearTimer.current);
    // Clear first so repeating the same message is announced again.
    setMessage('');
    window.setTimeout(() => setMessage(text), 30);
    clearTimer.current = window.setTimeout(() => setMessage(''), 8000);
  }, []);

  const explainUnavailable = useCallback(
    (capability: Capability) => announce(unavailableMessage(capability, bridge)),
    [announce, bridge],
  );

  const dispatch = useCallback(
    async (command: EditorCommand): Promise<BridgeResult> => {
      if (!bridge || !capabilities.has(command.type)) {
        const message = unavailableMessage(command.type, bridge);
        announce(message);
        return { ok: false, error: { code: 'unsupported', message } };
      }
      const result = await bridge.dispatch(command);
      if (!result.ok) announce(`${FAILURE_LABEL[command.type] ?? `${command.type} failed`}: ${result.error.message}`);
      return result;
    },
    [announce, bridge, capabilities],
  );
  const run = useCallback(async (command: EditorCommand) => (await dispatch(command)).ok, [dispatch]);

  const request = useCallback(
    async (hostRequest: HostRequest, options?: RequestOptions): Promise<RequestResult> => {
      const capability = capabilityOf(hostRequest);
      if (!bridge || !capabilities.has(capability)) {
        const message = unavailableMessage(capability, bridge);
        if (!options?.quiet) announce(message);
        return { ok: false, error: { code: 'unsupported', message } };
      }
      let result: RequestResult;
      try {
        result = await bridge.request(hostRequest);
      } catch (error) {
        // A rejection is not a result: the caller gets a failure it can show, never a
        // pending state that cannot end, and nothing is claimed about what the host did.
        result = rejectedRequest(error);
      }
      if (!result.ok && !options?.quiet) announce(`${hostRequest.type} failed: ${result.error.message}`);
      return result;
    },
    [announce, bridge, capabilities],
  );
  const ask = useCallback(async (hostRequest: HostRequest) => (await request(hostRequest)).ok, [request]);

  const [accountOpen, setAccountOpen] = useState(false);
  const accountReturn = useRef<HTMLElement | null>(null);
  const openAccount = useCallback((from: HTMLElement | null) => {
    accountReturn.current = from;
    setAccountOpen(true);
  }, []);
  const closeAccount = useCallback(() => {
    setAccountOpen(false);
    const target = accountReturn.current;
    // The opener may have unmounted (e.g. the agent panel's button after sign-in).
    if (target?.isConnected) target.focus();
    else document.querySelector<HTMLElement>('.provider-chip')?.focus();
  }, []);

  // A selection that no longer exists in the document is dropped.
  const liveSelection =
    selection !== null && snapshot?.hierarchy.status === 'ready' && snapshot.hierarchy.value.nodes[selection]
      ? selection
      : null;

  const value = useMemo<Shell>(
    () => ({
      panels,
      showPanel,
      togglePanel,
      bridge,
      snapshot,
      capabilities,
      selection: liveSelection,
      select: setSelection,
      dockTab,
      setDockTab,
      message,
      announce,
      run,
      dispatch,
      ask,
      request,
      explainUnavailable,
      accountOpen,
      openAccount,
      closeAccount,
    }),
    [panels, showPanel, togglePanel, bridge, snapshot, capabilities, liveSelection, dockTab, message, announce, run, dispatch, ask, request, explainUnavailable, accountOpen, openAccount, closeAccount],
  );
  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>;
}

export function useShell(): Shell {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error('useShell must be used inside ShellProvider');
  return shell;
}
