import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useSyncExternalStore } from 'react';
import type { FocusEvent, MouseEvent } from 'react';
import type { AgentApprovalMode, AgentPendingApproval, AgentTurnSnapshot } from '../bridge/contract';
import { unavailableMessage } from '../bridge/resolve';
import { Icon } from '../icons/Icon';
import { WispMark } from '../icons/WispMark';
import { projectState } from '../shell/projectState';
import { focusRegion } from '../shell/regions';
import { useShell } from '../shell/ShellContext';
import { ChatGPTButton } from './AccountDialog';
import { comboText, usePlatform } from './ShortcutsDialog';
import { AgentComposer } from './agent/AgentComposer';
import type { ComposerAction } from './agent/AgentComposer';
import { AgentTranscript } from './agent/AgentTranscript';
import type { TurnActions } from './agent/AgentTurn';
import { agentMemory, approvalKey, withoutItem, withoutKey } from './agent/agentMemory';
import { agentPhase } from './agent/agentPhase';
import {
  DEFAULT_MODEL,
  MODEL_LIMIT_BYTES,
  PROMPT_LIMIT_BYTES,
  asStatement,
  failureSentence,
  isActive,
  namesFrom,
  plainText,
  reviewApproval,
  sizeText,
  utf8Bytes,
} from './agent/agentText';

/** True when the agent can take or is working on a request, so the composer is useful. */
export function agentReady(status: string | undefined): boolean {
  return status === 'idle' || status === 'running';
}

/**
 * The native bridge reports that the agent is not part of the editor yet: today in
 * plain words, in earlier builds by naming an internal milestone (a headless command, a
 * "Phase 0 provider spike"). The contract has no reason code, so the sentence is
 * recognised here. Presentation only: the note gets its own title instead of inviting a
 * sign-in that would not enable the agent, and the state stays "unavailable".
 */
export function isNotInEditorReason(reason: string): boolean {
  return /not (?:available in|part of) the editor|headless agent command|provider spike|\bphase \d/i.test(reason);
}

type Tone = 'ok' | 'busy' | 'attention' | 'pending' | 'error' | 'idle';

/** The words the pane says (in the status line) when a turn reaches this state. */
function outcomeAnnouncement(turn: AgentTurnSnapshot): string {
  const edits = turn.report?.transaction_count;
  switch (turn.status) {
    case 'completed':
      return edits ? `The agent finished: ${edits.toLocaleString()} ${edits === 1 ? 'edit' : 'edits'}.` : 'The agent finished.';
    case 'cancelled':
      return 'The agent stopped.';
    case 'failed':
      return `The agent request failed. ${failureSentence(turn)}`;
    default:
      return 'The agent request ended.';
  }
}

/**
 * Agent area under the Inspector: the native session's turns, oldest first, and the
 * composer. Conversation state comes only from the host's `agentSession`; a send never
 * fabricates a turn. While the agent cannot run, the composer gives way to one short
 * note with its single next step, and any turns the session still holds stay readable
 * above it (account changes and transport recovery keep the history). The header
 * collapses the pane to its status line.
 */
export function AgentPanel({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { snapshot, capabilities, bridge, request, announce, explainUnavailable, openAccount, ask, showPanel, setDockTab } = useShell();
  const platform = usePlatform();
  // The draft, the next message's settings and this pane's requests in flight live per
  // bridge, outside the component, so they survive the pane unmounting mid-request.
  const memory = useMemo(() => agentMemory(bridge), [bridge]);
  const pane = useSyncExternalStore(memory.subscribe, memory.get);
  const { draft, model, approval, startedTurn, sending, sendError, cancelling, deciding, decisionErrors, retrying } = pane;
  const focusDecision = useRef<string | null>(null);

  const agent = snapshot?.agent;
  const provider = snapshot?.provider;
  const agentError = snapshot?.agentError;
  const session = snapshot?.agentSession;
  const turns = useMemo(() => session?.turns ?? [], [session]);
  const last = turns.at(-1) ?? null;
  const pending = last?.pending ?? null;
  const project = projectState(snapshot);
  const signedIn = provider?.status === 'connected';
  const signingIn = provider?.status === 'connecting' || provider?.status === 'checking';
  const notInEditor = agent?.status === 'unavailable' && isNotInEditorReason(agent.reason);
  const exhausted = !!session && session.budget.used_tokens >= session.budget.max_tokens;
  const { phase, stoppable } = agentPhase(snapshot, { sending, cancelling, startedTurn });

  const hierarchy = snapshot?.hierarchy;
  const assets = snapshot?.assets;
  const names = useMemo(() => namesFrom(hierarchy, assets), [hierarchy, assets]);

  // One review per pending approval: the card shows it, the status line says it, and its
  // decided row keeps its wording.
  const pendingReview = useMemo(() => (pending ? reviewApproval(pending.tool, pending.arguments, names) : null), [pending, names]);
  useEffect(() => {
    const key = pending ? approvalKey(pending.turn, pending.id) : null;
    if (key && pendingReview) memory.reviews.set(key, pendingReview);
    if (!session) return;
    const held = new Set(session.turns.map((turn) => turn.id));
    for (const stored of memory.reviews.keys()) if (!held.has(Number(stored.split(':')[0]))) memory.reviews.delete(stored);
    // A failed decision's message matters only while that approval is still pending.
    const errors = memory.get().decisionErrors;
    if ([...errors.keys()].some((stored) => stored !== key)) memory.set({ decisionErrors: new Map([...errors].filter(([stored]) => stored === key)) });
  }, [memory, pending, pendingReview, session]);

  // A start receipt matters only until the session lists its turn or the host is idle again.
  const lastId = last?.id ?? 0;
  const agentStatus = agent?.status;
  useEffect(() => {
    if (startedTurn !== null && !sending && (lastId >= startedTurn || agentStatus === 'idle')) memory.set({ startedTurn: null });
  }, [memory, startedTurn, sending, lastId, agentStatus]);

  // A stop the host accepted stays "stopping" here until the session shows the turn ended.
  useEffect(() => {
    const ended = [...memory.get().cancelling].filter((id) => {
      const turn = turns.find((candidate) => candidate.id === id);
      return turn ? turn.status !== 'running' && turn.status !== 'awaiting_approval' : id !== startedTurn;
    });
    if (ended.length) memory.set((state) => ({ cancelling: new Set([...state.cancelling].filter((id) => !ended.includes(id))) }));
  }, [memory, turns, startedTurn]);

  // Discrete announcements in the status line: approval needed, and how each turn ended.
  // Streamed text is never announced. History present on first sight is not re-announced.
  const seen = useRef<Map<number, string> | null>(null);
  useEffect(() => {
    if (!session) return;
    const previous = seen.current;
    const next = new Map<number, string>();
    for (const turn of session.turns) {
      const state = `${turn.status}:${turn.pending?.id ?? ''}`;
      next.set(turn.id, state);
      const before = previous?.get(turn.id);
      if (!previous || before === state) continue;
      if (turn.pending && turn.status === 'awaiting_approval') {
        const review = turn.pending === pending && pendingReview ? pendingReview : reviewApproval(turn.pending.tool, turn.pending.arguments, names);
        announce(`The agent needs approval: ${plainText(asStatement(review.title))}.`);
      } else if (!isActive(turn.status) && (before === undefined || isActive(before.split(':')[0]!))) {
        announce(outcomeAnnouncement(turn));
      }
    }
    seen.current = next;
  }, [session, pending, pendingReview, names, announce]);

  // A transport failure noticed by polling (not by a request of ours, which the shell
  // already announces) is said once.
  const errorKey = agentError ? `${agentError.code}\n${agentError.message}` : null;
  const lastErrorKey = useRef(errorKey);
  useEffect(() => {
    if (errorKey === lastErrorKey.current) return;
    lastErrorKey.current = errorKey;
    const now = memory.get();
    const ours = now.sending || now.cancelling.size > 0 || now.deciding.size > 0 || now.retrying;
    if (agentError && !ours) announce(`Agent: ${agentError.message}`);
  }, [memory, errorKey, agentError, announce]);

  const draftBytes = utf8Bytes(draft.trim());
  const modelBytes = utf8Bytes(model.trim());
  const overLimit = draftBytes > PROMPT_LIMIT_BYTES || modelBytes > MODEL_LIMIT_BYTES;

  const submit = async () => {
    const current = memory.get();
    if (current.sending) return;
    if (phase !== 'idle') {
      if (phase !== 'unavailable') announce('The agent is still working. Send the message when it finishes.');
      return;
    }
    if (!capabilities.has('agent.send')) {
      explainUnavailable('agent.send');
      return;
    }
    const sent = current.draft;
    const text = sent.trim();
    if (!text || overLimit) return;
    memory.set({ sending: true, sendError: null });
    const result = await request({ type: 'agent.send', text, model: current.model.trim() || DEFAULT_MODEL, approval: current.approval });
    // Clear only what was sent: anything typed while the request was in flight stays.
    memory.set((state) =>
      result.ok
        ? { sending: false, draft: state.draft === sent ? '' : state.draft, startedTurn: result.agentTurnId ?? state.startedTurn }
        : { sending: false, sendError: result.error.message },
    );
    if (result.ok && !bridge?.getSnapshot().agentSession) announce('Sent. This editor host does not show the agent’s replies; its edits appear in History.');
  };

  const stop = async (turnId: number) => {
    if (!capabilities.has('agent.cancel')) {
      explainUnavailable('agent.cancel');
      return;
    }
    if (memory.get().cancelling.has(turnId)) return;
    memory.set((state) => ({ cancelling: new Set(state.cancelling).add(turnId) }));
    // A failure is announced by the shell; the turn then shows what the host reports.
    const result = await request({ type: 'agent.cancel', turnId });
    if (!result.ok) memory.set((state) => ({ cancelling: withoutItem(state.cancelling, turnId) }));
  };

  const decide = useCallback(
    async (approvalRequest: AgentPendingApproval, approved: boolean) => {
      if (!capabilities.has('agent.approve')) {
        explainUnavailable('agent.approve');
        return;
      }
      const key = approvalKey(approvalRequest.turn, approvalRequest.id);
      if (memory.get().deciding.has(key)) return;
      memory.set((state) => ({ deciding: new Map(state.deciding).set(key, approved), decisionErrors: withoutKey(state.decisionErrors, key) }));
      // The card's buttons go away once the host records the decision; its row takes focus.
      focusDecision.current = key;
      const result = await request({ type: 'agent.approve', turnId: approvalRequest.turn, approvalId: approvalRequest.id, approved });
      memory.set((state) => ({
        deciding: withoutKey(state.deciding, key),
        ...(result.ok ? {} : { decisionErrors: new Map(state.decisionErrors).set(key, result.error.message) }),
      }));
      if (!result.ok && focusDecision.current === key) focusDecision.current = null;
    },
    [memory, capabilities, explainUnavailable, request],
  );

  const retry = async () => {
    if (!bridge?.refreshAgent || memory.get().retrying) return;
    memory.set({ retrying: true });
    try {
      await bridge.refreshAgent();
    } catch {
      // The bridge publishes its own safe error; nothing raw is shown here.
    }
    memory.set({ retrying: false });
    const after = bridge.getSnapshot().agentError;
    announce(after ? `Still unavailable. ${after.message}` : 'Agent session refreshed.');
  };

  const showHistory = useCallback(() => {
    showPanel('dock');
    setDockTab('history');
    // After the dock renders its History tab.
    window.setTimeout(() => focusRegion('dock'), 0);
  }, [showPanel, setDockTab]);

  // Starts sign-in and opens the account dialog, which shows browser and validation progress.
  const connect = (event: MouseEvent<HTMLButtonElement>) => {
    openAccount(event.currentTarget);
    if (capabilities.has('provider.connect')) void ask({ type: 'provider.connect', method: 'oauth' });
  };

  const decisionBlocked = !capabilities.has('agent.approve')
    ? unavailableMessage('agent.approve', bridge)
    : phase === 'stopping'
      ? 'The agent is stopping.'
      : !signedIn
        ? 'Sign in again to decide. You can still stop the agent.'
        : agentError
          ? 'Retry the connection to decide. You can still stop the agent.'
          : null;
  const turnActions = useMemo<Omit<TurnActions, 'reading'>>(
    () => ({
      names,
      reviews: memory.reviews,
      pendingReview: pending && pendingReview ? { key: approvalKey(pending.turn, pending.id), review: pendingReview } : null,
      deciding,
      decisionErrors,
      decisionBlocked,
      decide: (approvalRequest, approved) => void decide(approvalRequest, approved),
      showHistory,
      focusDecision,
    }),
    [names, memory, pending, pendingReview, deciding, decisionErrors, decisionBlocked, decide, showHistory],
  );

  // Header status: the agent's own state first, then why it cannot run.
  const status: { text: string; tone: Tone } = !snapshot
    ? { text: 'No engine', tone: 'idle' }
    : phase === 'idle'
      ? { text: 'Ready', tone: 'ok' }
      : phase === 'starting'
        ? { text: 'Starting…', tone: 'busy' }
        : phase === 'running'
          ? { text: 'Working…', tone: 'busy' }
          : phase === 'approval'
            ? { text: 'Needs approval', tone: 'attention' }
            : phase === 'stopping'
              ? { text: 'Stopping…', tone: 'busy' }
              : phase === 'finishing'
                ? { text: 'Finishing…', tone: 'busy' }
                : provider?.status === 'error'
                  ? { text: 'Sign-in problem', tone: 'error' }
                  : signingIn
                    ? { text: 'Signing in…', tone: 'pending' }
                    : agentError && signedIn
                      ? { text: 'Unavailable', tone: 'error' }
                      : signedIn || notInEditor
                        ? { text: 'Unavailable', tone: 'idle' }
                        : { text: 'Not signed in', tone: 'idle' };

  const reason = !snapshot
    ? 'No engine is connected.'
    : project.kind === 'failed'
      ? // The banner shows the failure itself; repeating it here adds nothing.
        'The agent needs an open project.'
      : agent?.status === 'unavailable'
        ? notInEditor
          ? 'The agent is not available in the editor yet.'
          : exhausted && signedIn && !agentError
            ? 'This session’s token allowance is used up.'
            : agent.reason
        : '';

  const composerShown = phase !== 'unavailable';
  const transcriptShown = composerShown || turns.length > 0;
  // Mirrors App: only an open, ready agent is a resizable pane. Otherwise the pane is as
  // tall as its content, so held turns above a note scroll within a bounded height.
  const capped = transcriptShown && !(open && agentReady(agent?.status));

  const sendCombo = comboText(['Mod', 'Enter'], platform);
  const action: ComposerAction = (() => {
    const wait = (label: string, title: string): ComposerAction => ({ kind: 'wait', label, title, unavailable: true, onActivate: () => announce(title) });
    const stopAction = (turnId: number): ComposerAction => ({
      kind: 'stop',
      label: 'Stop',
      title: 'Stop the agent',
      unavailable: !capabilities.has('agent.cancel'),
      onActivate: () => void stop(turnId),
    });
    switch (phase) {
      case 'idle': {
        const supported = capabilities.has('agent.send');
        return {
          kind: 'send',
          label: 'Send',
          title: `Send (${sendCombo})`,
          unavailable: !supported || !draft.trim() || overLimit,
          onActivate: () => (supported ? void submit() : explainUnavailable('agent.send')),
        };
      }
      case 'starting':
        return stoppable === null ? wait('Sending…', 'Sending the message to the agent.') : stopAction(stoppable);
      case 'running':
      case 'approval':
        return stoppable === null ? wait('Working…', 'The agent is busy. You can send when it finishes.') : stopAction(stoppable);
      case 'stopping':
        return wait('Stopping…', 'The agent is stopping. A model call in progress can take a moment to end.');
      default:
        return wait('Finishing…', 'The agent is finishing its last request. You can send when it stops.');
    }
  })();

  const notes: { id: string; text: string; tone: 'error' | 'quiet' }[] = [];
  if (sendError) notes.push({ id: 'agent-send-error', text: sendError, tone: 'error' });
  if (draftBytes > PROMPT_LIMIT_BYTES) notes.push({ id: 'agent-send-limit', text: `This message is ${sizeText(draftBytes)}; the limit is 16 KB.`, tone: 'quiet' });
  if (modelBytes > MODEL_LIMIT_BYTES) notes.push({ id: 'agent-model-limit', text: `The model name is ${modelBytes} bytes; the limit is ${MODEL_LIMIT_BYTES}.`, tone: 'quiet' });
  if (!capabilities.has('agent.send')) notes.push({ id: 'agent-reason', text: 'This bridge does not support sending messages to the agent.', tone: 'quiet' });

  // Keep keyboard focus in the pane when the element holding it goes away (the composer
  // replaced by a note, a retry that succeeded), instead of dropping it on the page. Only
  // while focus is still ours: leaving the pane forgets the element.
  const section = useRef<HTMLElement>(null);
  const lastFocused = useRef<HTMLElement | null>(null);
  const trackFocus = (event: FocusEvent<HTMLElement>) => {
    lastFocused.current = event.target instanceof HTMLElement ? event.target : null;
  };
  const trackBlur = (event: FocusEvent<HTMLElement>) => {
    const leaving = event.target;
    // Decide after the event: a removed element (still ours to rescue) is disconnected by then.
    queueMicrotask(() => {
      if (lastFocused.current !== leaving || !leaving.isConnected) return;
      if (!section.current?.contains(document.activeElement)) lastFocused.current = null;
    });
  };
  useLayoutEffect(() => {
    const previous = lastFocused.current;
    if (!previous || previous.isConnected) return;
    const active = document.activeElement;
    if (active !== null && active !== document.body) return;
    const root = section.current;
    const target = root?.querySelector<HTMLElement>('[data-agent-rescue]:not(:disabled)') ?? root?.querySelector<HTMLElement>('.agent__toggle');
    lastFocused.current = null;
    target?.focus();
  });

  return (
    <section
      ref={section}
      className={`panel panel--agent${open ? ' is-open' : ''}${capped ? ' is-capped' : ''}`}
      data-region="agent"
      aria-labelledby="agent-title"
      tabIndex={-1}
      onFocus={trackFocus}
      onBlur={trackBlur}
    >
      <header className="panel__header agent__header">
        <h2 className="agent__heading">
          <button
            type="button"
            className="agent__toggle"
            aria-expanded={open}
            aria-controls="agent-body"
            data-focus-target=""
            title={open ? 'Collapse the agent panel' : 'Expand the agent panel'}
            onClick={onToggle}
          >
            <Icon name="chevron" size={12} className={`agent__chevron${open ? ' is-open' : ''}`} />
            <span className="agent__mark" aria-hidden="true">
              <WispMark size={14} />
            </span>
            <span id="agent-title" className="agent__title">
              Agent
            </span>
            <span className={`agent__status agent__status--${status.tone}`}>
              {status.tone === 'busy' ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
              <span className="visually-hidden">, </span>
              {status.text}
            </span>
          </button>
        </h2>
      </header>
      {open ? (
        <div id="agent-body" className="agent__body">
          {transcriptShown ? <AgentTranscript session={session} actions={turnActions} /> : null}
          {composerShown ? (
            <>
              {agentError ? (
                <div className="agent-notice" role="note">
                  <Icon name="warning" size={12} className="agent-notice__icon" />
                  <span className="agent-notice__text">{agentError.message}</span>
                  {bridge?.refreshAgent ? (
                    <button type="button" className="button button--ghost button--small agent-notice__action" aria-disabled={retrying || undefined} onClick={() => void retry()}>
                      {retrying ? 'Retrying…' : 'Retry'}
                    </button>
                  ) : null}
                </div>
              ) : null}
              <AgentComposer
                draft={draft}
                onDraft={(value) => memory.set({ draft: value, sendError: null })}
                model={model}
                onModel={(value) => memory.set({ model: value })}
                approval={approval}
                onApproval={(value: AgentApprovalMode) => memory.set({ approval: value })}
                action={action}
                onSubmit={() => void submit()}
                inputDisabled={!capabilities.has('agent.send')}
                notes={notes}
              />
            </>
          ) : (
            // One sentence: why the agent cannot run, and the single next step when one
            // exists. The header already carries the state; no title repeats it.
            <div className="agent-note">
              <p className="agent-note__body" id="agent-reason" tabIndex={-1} data-agent-rescue="">
                {reason}
              </p>
              {notInEditor ? null : snapshot && provider?.status === 'not-connected' ? (
                <div className="agent-note__actions">
                  <ChatGPTButton small available={capabilities.has('provider.connect')} onClick={connect} />
                </div>
              ) : snapshot && provider && provider.status !== 'connected' ? (
                <div className="agent-note__actions">
                  <button
                    type="button"
                    className={`button button--small${provider.status === 'error' ? ' button--danger' : ''}`}
                    aria-haspopup="dialog"
                    onClick={(event) => openAccount(event.currentTarget)}
                  >
                    {provider.status === 'error' ? (
                      <Icon name="error" size={12} />
                    ) : (
                      <span className="spinner spinner--small" aria-hidden="true" />
                    )}
                    {provider.status === 'error' ? 'Sign-in problem…' : provider.status === 'checking' ? 'Checking sign-in…' : 'Signing in…'}
                  </button>
                </div>
              ) : agentError && bridge?.refreshAgent ? (
                <div className="agent-note__actions">
                  <button type="button" className="button button--small" aria-disabled={retrying || undefined} onClick={() => void retry()}>
                    {retrying ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
                    {retrying ? 'Retrying…' : 'Retry'}
                  </button>
                </div>
              ) : null}
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}
