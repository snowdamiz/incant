import type { BridgeSnapshot } from '../../bridge/contract';
import { isActive } from './agentText';

/**
 * What the agent is doing, from the host's snapshot plus this pane's own requests in
 * flight. Nothing is inferred from a send: a turn exists only once the session lists it.
 *
 * - `starting`: a send is in flight, or its receipt reserved a turn the session does not list yet.
 * - `stopping`: a stop was requested (here or natively) and the worker has not finished.
 * - `finishing`: the last turn has ended but the native worker is still running, so a new
 *   turn stays unavailable until it stops.
 * - `running`: a turn is running; with no stoppable turn, the host is busy for a reason
 *   its session does not show (or it has no session, as on older hosts).
 */
export type AgentPhase = 'idle' | 'starting' | 'running' | 'approval' | 'stopping' | 'finishing' | 'unavailable';

export interface AgentLocal {
  readonly sending: boolean;
  /** Turns with a stop request in flight from this pane. */
  readonly cancelling: ReadonlySet<number>;
  /** The turn the last successful send started (its `agentTurnId`). */
  readonly startedTurn: number | null;
}

export interface AgentPhaseView {
  readonly phase: AgentPhase;
  /** The turn Stop cancels, with its explicit ID; null when nothing can be stopped now. */
  readonly stoppable: number | null;
}

export function agentPhase(snapshot: BridgeSnapshot | null, local: AgentLocal): AgentPhaseView {
  const session = snapshot?.agentSession;
  const last = session?.turns.at(-1) ?? null;
  if (last && isActive(last.status)) {
    if (last.status === 'stopping' || local.cancelling.has(last.id)) return { phase: 'stopping', stoppable: null };
    return { phase: last.status === 'awaiting_approval' ? 'approval' : 'running', stoppable: last.id };
  }
  if (session?.worker_running) return { phase: last?.status === 'cancelled' ? 'stopping' : 'finishing', stoppable: null };
  if (local.sending) return { phase: 'starting', stoppable: null };
  if (snapshot?.agent.status === 'running') {
    const started = local.startedTurn;
    if (started !== null && (last?.id ?? 0) < started) {
      // A start receipt reserves this turn until a session read lists it; it can be stopped by ID.
      return local.cancelling.has(started) ? { phase: 'stopping', stoppable: null } : { phase: 'starting', stoppable: started };
    }
    // Busy for a reason the session does not show (or a host without one): no turn to stop.
    return { phase: 'running', stoppable: null };
  }
  if (snapshot?.agent.status === 'idle') return { phase: 'idle', stoppable: null };
  return { phase: 'unavailable', stoppable: null };
}
