import type { AgentApprovalMode, EditorBridge } from '../../bridge/contract';
import type { ApprovalReview } from './agentText';
import { DEFAULT_APPROVAL, DEFAULT_MODEL } from './agentText';

/**
 * An open allowance editor. It belongs to one project: another project's editor is never
 * shown or saved, whatever is typed in it.
 */
export interface AllowanceEdit {
  readonly project: string;
  readonly draft: string;
  /** The authored allowance the edit started from (or last saved here). A different current value came from another edit. */
  readonly base: number;
  /** Why the last save failed: the host's safe message. */
  readonly error: string | null;
  /** A save was attempted, so an empty field is reported rather than left quiet. */
  readonly attempted: boolean;
}

/** A save in flight: its project and exactly what was sent. */
export interface AllowanceSave {
  readonly project: string;
  readonly draft: string;
  readonly value: number;
}

/**
 * The Agent pane's own state for one bridge: the unsent draft, the next message's model
 * and approval mode, the allowance being edited, and the requests this pane has in
 * flight. It lives outside the component (hiding the Inspector column unmounts the
 * pane), so a request that resolves while the pane is away, or after it comes back,
 * lands in the same place. None of it is conversation state or project state: turns come
 * only from the host's session snapshot, and the allowance only from the project.
 */
export interface AgentPaneState {
  readonly draft: string;
  readonly model: string;
  readonly approval: AgentApprovalMode;
  /** From a successful send's `agentTurnId`. Stop can target it before the session lists it. */
  readonly startedTurn: number | null;
  readonly sending: boolean;
  readonly sendError: string | null;
  /** Turns with a stop requested from this pane that the session has not shown ending. */
  readonly cancelling: ReadonlySet<number>;
  /** Decisions in flight by `${turn}:${approval}`: true for Allow, false for Deny. */
  readonly deciding: ReadonlyMap<string, boolean>;
  /** Why a decision failed while its approval was still pending. */
  readonly decisionErrors: ReadonlyMap<string, string>;
  readonly retrying: boolean;
  /** The open allowance editor, or null. Closing it forgets what was typed. */
  readonly allowance: AllowanceEdit | null;
  readonly allowanceSaving: AllowanceSave | null;
}

export interface AgentMemory {
  readonly get: () => AgentPaneState;
  /** Applies a patch synchronously, so duplicate requests are refused at once. */
  readonly set: (patch: Partial<AgentPaneState> | ((state: AgentPaneState) => Partial<AgentPaneState>)) => void;
  readonly subscribe: (listener: () => void) => () => void;
  /** The review each approval showed while pending (the host keeps only a summary once decided). */
  readonly reviews: Map<string, ApprovalReview>;
}

function createMemory(): AgentMemory {
  let state: AgentPaneState = {
    draft: '',
    model: DEFAULT_MODEL,
    approval: DEFAULT_APPROVAL,
    startedTurn: null,
    sending: false,
    sendError: null,
    cancelling: new Set(),
    deciding: new Map(),
    decisionErrors: new Map(),
    retrying: false,
    allowance: null,
    allowanceSaving: null,
  };
  const listeners = new Set<() => void>();
  return {
    get: () => state,
    set: (patch) => {
      state = { ...state, ...(typeof patch === 'function' ? patch(state) : patch) };
      for (const listener of listeners) listener();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    reviews: new Map(),
  };
}

const memories = new WeakMap<EditorBridge, AgentMemory>();

/** Without a bridge nothing can be sent, so nothing is worth keeping. */
export function agentMemory(bridge: EditorBridge | null): AgentMemory {
  if (!bridge) return createMemory();
  let memory = memories.get(bridge);
  if (!memory) {
    memory = createMemory();
    memories.set(bridge, memory);
  }
  return memory;
}

export const approvalKey = (turn: number, approval: number) => `${turn}:${approval}`;

export function withoutKey<K, V>(map: ReadonlyMap<K, V>, key: K): ReadonlyMap<K, V> {
  if (!map.has(key)) return map;
  const next = new Map(map);
  next.delete(key);
  return next;
}

export function withoutItem<T>(set: ReadonlySet<T>, item: T): ReadonlySet<T> {
  if (!set.has(item)) return set;
  const next = new Set(set);
  next.delete(item);
  return next;
}
