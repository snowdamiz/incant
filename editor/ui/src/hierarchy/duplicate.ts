import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { BridgeResult, EditorBridge, HierarchyTree, Ulid } from '../bridge/contract';
import { unavailableMessage } from '../bridge/resolve';
import type { KeyPlatform } from '../components/ShortcutsDialog';
import { useOptionalPlay } from '../play/PlayContext';
import { useShell } from '../shell/ShellContext';

interface ShortcutEvent {
  readonly key: string;
  readonly code: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

/**
 * ⌘D on macOS, Ctrl+D elsewhere: by the D of the current layout, or by the physical D key
 * on a layout without Latin letters. The other platform's modifier, Shift and Option or
 * Alt are left alone (AltGr types characters).
 */
export function isDuplicateShortcut(event: ShortcutEvent, platform: KeyPlatform): boolean {
  if (event.shiftKey || event.altKey || (platform === 'mac' ? !event.metaKey || event.ctrlKey : !event.ctrlKey || event.metaKey)) return false;
  const key = event.key.toLowerCase();
  return key === 'd' || (event.code === 'KeyD' && !/^[a-z]$/.test(key));
}

const SCENE = 'Scenes cannot be duplicated.';
const PLAYING = 'Stop Play to duplicate.';
const STOPPING = 'Wait for Play to stop.';
const BUSY = 'Wait for the current duplicate to finish.';
const GONE = 'Duplicate failed: the entity no longer exists.';
const NO_COPY = 'Duplicate result unknown: the host reported success but returned no copy.';

export interface Duplicate {
  /** The entity being copied, while its command runs. */
  readonly pending: Ulid | null;
  /** Why Duplicate cannot act on `id` now, or null. */
  readonly blocked: (id: Ulid) => string | null;
  /** Duplicates `id` with its descendants, or announces why it cannot. */
  readonly start: (id: Ulid) => void;
}

/**
 * The hierarchy's Duplicate: one `entity.duplicate` command at a time, for an entity
 * while no Play world exists. Nothing is shown before the engine answers. The returned
 * copy is selected only once it is in the latest snapshot of the bridge and project that
 * asked, and only if the selection did not move on meanwhile; `focus.claim` moves keyboard
 * focus to it while the tree has focus. The Shell announces failures and unknown results,
 * and the selection stays where it was.
 */
export function useDuplicate(
  tree: HierarchyTree,
  focus: { readonly owned: () => boolean; readonly claim: (id: Ulid) => void },
): Duplicate {
  const { bridge, capabilities, dispatch, announce, select, selection } = useShell();
  const play = useOptionalPlay();
  const [pending, setPending] = useState<Ulid | null>(null);
  // A second press in the same task must not send a second command.
  const running = useRef(false);
  const mounted = useRef(false);
  const latest = useRef({ bridge, selection });
  useLayoutEffect(() => {
    latest.current = { bridge, selection };
  });
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const blocked = (id: Ulid): string | null => {
    if (!capabilities.has('entity.duplicate')) return unavailableMessage('entity.duplicate', bridge);
    if (tree.nodes[id]?.kind === 'scene') return SCENE;
    // While a world exists the selection is what Play inspects, and a new revision would
    // refuse a session that is starting.
    if (play?.state === 'stopping') return STOPPING;
    if (play?.stoppable) return PLAYING;
    return pending !== null ? BUSY : null;
  };

  const run = async (host: EditorBridge, id: Ulid) => {
    const before = host.getSnapshot();
    const project = before.connection.status === 'ready' ? before.connection.project.id : null;
    const source = before.hierarchy.status === 'ready' ? before.hierarchy.value.nodes[id] : undefined;
    if (project === null || !source) {
      announce(GONE);
      return;
    }
    const chosen = latest.current.selection;
    running.current = true;
    setPending(id);
    let result: BridgeResult;
    try {
      result = await dispatch({ type: 'entity.duplicate', entity: id });
    } finally {
      running.current = false;
      if (mounted.current) setPending(null);
    }
    if (!result.ok || !mounted.current) return;
    const copy = result.duplicatedEntity;
    if (copy === undefined) {
      announce(NO_COPY);
      return;
    }
    // An answer for a bridge or project that is no longer open says and selects nothing.
    const after = host.getSnapshot();
    if (latest.current.bridge !== host || after.connection.status !== 'ready' || after.connection.project.id !== project) return;
    if (after.hierarchy.status !== 'ready' || !after.hierarchy.value.nodes[copy]) {
      announce(`Duplicated ${source.name}, but the copy is no longer in the hierarchy.`);
      return;
    }
    announce(`Duplicated ${source.name}`);
    const now = latest.current.selection;
    // A selection made while the command ran is the user's newer choice.
    if (now !== chosen && now !== null) return;
    if (focus.owned()) focus.claim(copy);
    select(copy);
  };

  const start = (id: Ulid) => {
    const reason = running.current ? BUSY : blocked(id);
    if (reason !== null) announce(reason);
    else if (bridge) void run(bridge, id);
  };

  return { pending, blocked, start };
}
