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

export type DockTab = 'problems' | 'console' | 'history';

export interface Shell {
  readonly panels: PanelVisibility;
  readonly showPanel: (panel: keyof PanelVisibility) => void;
  readonly togglePanel: (panel: keyof PanelVisibility) => void;
  readonly bridge: EditorBridge | null;
  readonly snapshot: BridgeSnapshot | null;
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
  /** Like `ask`, but returns the host's result so a caller can show the exact error inline. */
  readonly request: (request: HostRequest) => Promise<BridgeResult>;
  readonly explainUnavailable: (capability: Capability) => void;
  /** The ChatGPT account dialog. `from` regains focus when it closes. */
  readonly accountOpen: boolean;
  readonly openAccount: (from: HTMLElement | null) => void;
  readonly closeAccount: () => void;
}

const ShellContext = createContext<Shell | null>(null);

/** Plain-language failure prefixes; other commands keep their command name. */
const FAILURE_LABEL: Partial<Record<EditorCommand['type'], string>> = { 'asset.import': 'Import failed' };

const NO_CAPABILITIES = readCapabilities([]);
const noopSubscribe = () => () => undefined;
const nullSnapshot = () => null;

export function ShellProvider({ bridge, children }: { bridge: EditorBridge | null; children: ReactNode }) {
  // Bound wrappers so a class-based bridge keeps its `this`.
  const subscribe = useMemo(
    () => (bridge ? (listener: () => void) => bridge.subscribe(listener) : noopSubscribe),
    [bridge],
  );
  const getSnapshot = useMemo(() => (bridge ? () => bridge.getSnapshot() : nullSnapshot), [bridge]);
  const snapshot = useSyncExternalStore(subscribe, getSnapshot);
  const capabilities = useMemo(() => (bridge ? readCapabilities(bridge.capabilities) : NO_CAPABILITIES), [bridge]);
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
    async (hostRequest: HostRequest): Promise<BridgeResult> => {
      if (!bridge || !capabilities.has(hostRequest.type)) {
        const message = unavailableMessage(hostRequest.type, bridge);
        announce(message);
        return { ok: false, error: { code: 'unsupported', message } };
      }
      const result = await bridge.request(hostRequest);
      if (!result.ok) announce(`${hostRequest.type} failed: ${result.error.message}`);
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
