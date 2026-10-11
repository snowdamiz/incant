import { Fragment, useCallback, useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode, RefObject } from 'react';
import type { AgentApprovalSummary, AgentPendingApproval, AgentTurnSnapshot } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import { approvalKey } from './agentMemory';
import type { ApprovalReview, Names, ReviewPart } from './agentText';
import {
  DECISION_WORDS,
  approvalModeLabel,
  argumentsText,
  asStatement,
  failureSentence,
  isActive,
  reviewApproval,
  sizeText,
  toolPhrase,
  turnCost,
} from './agentText';

/** What a turn needs from the pane: names, approval state and the actions it can take. */
export interface TurnActions {
  readonly names: Names;
  /** Reviews shown while approvals were pending, by `${turn}:${approval}`. */
  readonly reviews: ReadonlyMap<string, ApprovalReview>;
  /** The review of the session's pending approval (by its key), shared with the status line. */
  readonly pendingReview: { readonly key: string; readonly review: ApprovalReview } | null;
  /** Decisions in flight from this pane: true for Allow, false for Deny. */
  readonly deciding: ReadonlyMap<string, boolean>;
  readonly decisionErrors: ReadonlyMap<string, string>;
  /** Why Allow and Deny cannot be used right now (e.g. signed out), or null. */
  readonly decisionBlocked: string | null;
  readonly decide: (pending: AgentPendingApproval, approved: boolean) => void;
  readonly showHistory: () => void;
  /** The decided row to focus when it appears, after its card's buttons went away. */
  readonly focusDecision: RefObject<string | null>;
  /** Something was expanded to be read: the conversation stops following new output. */
  readonly reading: () => void;
}

/**
 * One request and everything the native session reports about it, in reading order:
 * the prompt, the reply streamed so far, decided approvals, the pending approval, what
 * is happening now, and how it ended. Every string is rendered as text.
 */
export function AgentTurnView({ turn, actions }: { turn: AgentTurnSnapshot; actions: TurnActions }) {
  const decided = turn.approvals.filter((approval) => approval.decision !== 'pending');
  const pendingSummary = turn.pending ? turn.approvals.find((approval) => approval.id === turn.pending?.id) : undefined;
  return (
    <li className="agent-turn">
      <TurnPrompt turn={turn} onReading={actions.reading} />
      {turn.text ? (
        <div className="agent-turn__reply">
          <span className="visually-hidden">Agent: </span>
          {turn.text}
        </div>
      ) : null}
      {decided.length ? (
        <ul className="agent-decisions" aria-label="Approvals">
          {decided.map((approval) => (
            <DecisionRow key={approval.id} turn={turn.id} approval={approval} actions={actions} />
          ))}
        </ul>
      ) : null}
      {turn.pending ? (
        // Keyed by its IDs: a following approval is a new card, never the previous one's
        // focused buttons answering a request that was not read.
        <ApprovalCard key={approvalKey(turn.pending.turn, turn.pending.id)} pending={turn.pending} bytes={pendingSummary?.bytes ?? null} actions={actions} />
      ) : turn.status === 'awaiting_approval' ? (
        <p className="agent-activity">Waiting for approval…</p>
      ) : null}
      {/* The header says "Working…" or "Stopping…"; here a wordless mark shows where output continues. */}
      {turn.status === 'running' || turn.status === 'stopping' ? (
        <span className="agent-turn__live" aria-hidden="true">
          <span className="spinner spinner--small" />
        </span>
      ) : null}
      {isActive(turn.status) ? null : <TurnOutcome turn={turn} actions={actions} />}
    </li>
  );
}

/** Long prompts are clamped to a few lines with a control to show all of it. */
function TurnPrompt({ turn, onReading }: { turn: AgentTurnSnapshot; onReading: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const [clamped, setClamped] = useState(false);
  const text = useRef<HTMLParagraphElement>(null);
  const id = useId();
  useLayoutEffect(() => {
    const element = text.current;
    if (!element || expanded) return;
    const update = () => setClamped(element.scrollHeight > element.clientHeight + 1);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [expanded, turn.request.prompt]);
  const meta = `${turn.request.model} · ${approvalModeLabel(turn.request.approval)}`;
  return (
    <div className="agent-prompt" title={meta}>
      <p ref={text} id={id} className={`agent-prompt__text${expanded ? '' : ' is-clamped'}`}>
        <span className="visually-hidden">You: </span>
        {turn.request.prompt}
      </p>
      {clamped || expanded ? (
        <button
          type="button"
          className="agent-link agent-prompt__more"
          aria-expanded={expanded}
          aria-controls={id}
          onClick={() => {
            onReading();
            setExpanded((value) => !value);
          }}
        >
          {expanded ? 'Show less' : 'Show all'}
        </button>
      ) : null}
    </div>
  );
}

export function ReviewText({ parts }: { parts: readonly ReviewPart[] }) {
  return (
    <>
      {parts.map((part, index) =>
        typeof part === 'string' ? (
          <Fragment key={index}>{part}</Fragment>
        ) : (
          <bdi key={index} className={`agent-name${part.mono ? ' mono' : ''}`}>
            {part.name}
          </bdi>
        ),
      )}
    </>
  );
}

/**
 * A decided approval as one quiet line. The host keeps only the tool and decision, so
 * the wording is the review this pane showed while it was pending, or the tool's name.
 */
function DecisionRow({ turn, approval, actions }: { turn: number; approval: AgentApprovalSummary; actions: TurnActions }) {
  const key = approvalKey(turn, approval.id);
  const review = actions.reviews.get(key);
  const decision = approval.decision === 'pending' ? 'cancelled' : approval.decision;
  const { focusDecision } = actions;
  // After Allow or Deny the card (and the focused button) is replaced by this row.
  const attach = useCallback(
    (element: HTMLLIElement | null) => {
      if (!element || focusDecision.current !== key) return;
      const active = document.activeElement;
      if (active === null || active === document.body) element.focus();
      focusDecision.current = null;
    },
    [focusDecision, key],
  );
  return (
    <li ref={attach} tabIndex={-1} className={`agent-decision agent-decision--${decision}`}>
      <Icon name={decision === 'allowed' ? 'check' : 'close'} size={12} className="agent-decision__icon" />
      <span className="agent-decision__word">{DECISION_WORDS[decision]}</span>
      <span className="agent-decision__what">
        <ReviewText parts={review ? asStatement(review.title) : toolPhrase(approval.tool)} />
      </span>
    </li>
  );
}

/** Arguments beyond this many characters wait for an explicit "Show all". */
const ARGUMENT_PREVIEW = 64 * 1024;

/**
 * The pending approval: what it would do (from its exact arguments), the agent's own
 * description in quotes, the exact arguments on request, and Allow / Deny. Nothing is
 * focused or decided automatically. In a short pane the decision row sticks to the
 * bottom of the conversation while the review scrolls above it.
 */
function ApprovalCard({ pending, bytes, actions }: { pending: AgentPendingApproval; bytes: number | null; actions: TurnActions }) {
  const key = approvalKey(pending.turn, pending.id);
  const shared = actions.pendingReview?.key === key ? actions.pendingReview.review : null;
  const review = useMemo(() => shared ?? reviewApproval(pending.tool, pending.arguments, actions.names), [shared, actions.names, pending]);
  const [open, setOpen] = useState(false);
  const [whole, setWhole] = useState(false);
  const shown = useMemo(() => (open ? argumentsText(pending.arguments, whole ? Number.POSITIVE_INFINITY : ARGUMENT_PREVIEW) : null), [open, whole, pending.arguments]);
  const deciding = actions.deciding.get(key);
  const error = actions.decisionErrors.get(key);
  const blocked = actions.decisionBlocked;
  const unavailable = deciding !== undefined || blocked !== null;
  const ids = useId();
  const titleId = `${ids}-title`;
  const argsId = `${ids}-args`;
  const noteId = `${ids}-note`;
  const choose = (approved: boolean) => {
    if (unavailable) return;
    actions.decide(pending, approved);
  };
  return (
    <section className="agent-approval" aria-labelledby={titleId} aria-describedby={error || blocked ? noteId : undefined}>
      <h3 id={titleId} className="agent-approval__title">
        <ReviewText parts={review.title} />
      </h3>
      {review.description ? (
        <p className="agent-approval__description">
          <span className="visually-hidden">The agent describes it as: </span>“<bdi>{review.description}</bdi>”
        </p>
      ) : null}
      {review.rows.length ? (
        <ul className="agent-approval__rows">
          {review.rows.map((row, index) => (
            <li key={index} className={`agent-approval__row agent-approval__row--${row.kind}`}>
              <ReviewText parts={row.parts} />
            </li>
          ))}
          {review.more ? (
            <li className="agent-approval__row agent-approval__row--more">
              {review.more.toLocaleString()} more {review.more === 1 ? 'change' : 'changes'} in the arguments
            </li>
          ) : null}
        </ul>
      ) : null}
      <button
        type="button"
        className="agent-disclosure"
        aria-expanded={open}
        aria-controls={argsId}
        onClick={() => {
          actions.reading();
          setOpen((value) => !value);
        }}
      >
        <Icon name="chevron" size={10} className={`agent-disclosure__chevron${open ? ' is-open' : ''}`} />
        Exact arguments
        {bytes !== null ? <span className="agent-disclosure__meta">{sizeText(bytes)}</span> : null}
      </button>
      {shown ? (
        <div className="agent-approval__arguments-wrap">
          <pre id={argsId} className="agent-approval__arguments" tabIndex={0} aria-label="Exact arguments">
            {shown.text}
          </pre>
          {shown.truncated ? (
            <button
              type="button"
              className="agent-link"
              onClick={() => {
                actions.reading();
                setWhole(true);
              }}
            >
              Show all {sizeText(shown.length)}
            </button>
          ) : null}
        </div>
      ) : null}
      <div className="agent-approval__decision">
        {error || blocked ? (
          <p id={noteId} className={`agent-approval__note${error ? ' is-error' : ''}`}>
            {error ? <Icon name="error" size={12} className="agent-approval__note-icon" /> : null}
            {error ?? blocked}
          </p>
        ) : null}
        <div className="agent-approval__actions">
          <button type="button" className="button button--small" aria-disabled={unavailable || undefined} onClick={() => choose(false)}>
            {deciding === false ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
            {deciding === false ? 'Denying…' : 'Deny'}
          </button>
          <button type="button" className="button button--small button--primary" aria-disabled={unavailable || undefined} onClick={() => choose(true)}>
            {deciding === true ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
            {deciding === true ? 'Allowing…' : 'Allow'}
          </button>
        </div>
      </div>
    </section>
  );
}

const STATUS_WORDS: Readonly<Record<string, string>> = { cancelled: 'Stopped' };

/** How a finished turn ended: a failure in words, then edits and allowance charged. */
function TurnOutcome({ turn, actions }: { turn: AgentTurnSnapshot; actions: TurnActions }) {
  const known = turn.status === 'completed' || turn.status === 'cancelled' || turn.status === 'failed';
  const edits = turn.report?.transaction_count;
  const parts: ReactNode[] = [];
  const word = known ? STATUS_WORDS[turn.status] : 'Status unknown';
  if (word) parts.push(<span key="status" className="agent-outcome__status">{word}</span>);
  if (edits !== undefined) parts.push(<span key="edits">{edits === 0 ? 'No edits' : `${edits.toLocaleString()} ${edits === 1 ? 'edit' : 'edits'}`}</span>);
  const cost = turnCost(turn);
  parts.push(
    <span key="cost" className="agent-outcome__cost" title={turn.accounting_uncertain ? 'Exact usage could not be confirmed or saved. This is the amount charged to the allowance.' : undefined}>
      {cost}
    </span>,
  );
  return (
    <>
      {turn.status === 'failed' ? (
        <p className="agent-failure">
          <Icon name="error" size={12} className="agent-failure__icon" />
          <span>{failureSentence(turn)}</span>
        </p>
      ) : null}
      <p className="agent-outcome">
        {parts.map((part, index) => (
          <Fragment key={index}>
            {index ? <span className="agent-outcome__sep" aria-hidden="true">·</span> : null}
            {part}
          </Fragment>
        ))}
        {edits ? (
          <button type="button" className="agent-link agent-outcome__history" onClick={actions.showHistory}>
            Show in History
          </button>
        ) : null}
      </p>
    </>
  );
}
