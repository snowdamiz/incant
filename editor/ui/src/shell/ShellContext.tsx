import { createContext, useCallback, useContext, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import type { ReactNode } from 'react';
import type {
  BridgeSnapshot,
  Capability,
  EditorBridge,
  EditorCommand,
  HostRequest,
  Ulid,
} from '../bridge/contract';
import type { CapabilitySet } from '../bridge/resolve';
import { readCapabilities, unavailableMessage } from '../bridge/resolve';

export type DockTab = 'problems' | 'console' | 'history';

export interface Shell {
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
  readonly ask: (request: HostRequest) => Promise<boolean>;
  readonly explainUnavailable: (capability: Capability) => void;
}

const ShellContext = createContext<Shell | null>(null);

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

  const run = useCallback(
    async (command: EditorCommand) => {
      if (!bridge || !capabilities.has(command.type)) {
        explainUnavailable(command.type);
        return false;
      }
      const result = await bridge.dispatch(command);
      if (!result.ok) announce(`${command.type} failed: ${result.error.message}`);
      return result.ok;
    },
    [announce, bridge, capabilities, explainUnavailable],
  );

  const ask = useCallback(
    async (request: HostRequest) => {
      if (!bridge || !capabilities.has(request.type)) {
        explainUnavailable(request.type);
        return false;
      }
      const result = await bridge.request(request);
      if (!result.ok) announce(`${request.type} failed: ${result.error.message}`);
      return result.ok;
    },
    [announce, bridge, capabilities, explainUnavailable],
  );

  // A selection that no longer exists in the document is dropped.
  const liveSelection =
    selection !== null && snapshot?.hierarchy.status === 'ready' && snapshot.hierarchy.value.nodes[selection]
      ? selection
      : null;

  const value = useMemo<Shell>(
    () => ({
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
      ask,
      explainUnavailable,
    }),
    [bridge, snapshot, capabilities, liveSelection, dockTab, message, announce, run, ask, explainUnavailable],
  );
  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>;
}

export function useShell(): Shell {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error('useShell must be used inside ShellProvider');
  return shell;
}
