import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { AgentSessionSnapshot } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import type { TurnActions } from './AgentTurn';
import { AgentTurnView } from './AgentTurn';

/** The native session keeps at most this many turns (editor/bridge/native.ts validAgentSession). */
const SESSION_TURN_LIMIT = 16;
/** Within this many pixels of the end, the transcript keeps following new output. */
const FOLLOW_SLACK = 24;
/** Space kept above a pending review aligned to the top: inside the 8 px turn gap. */
const REVIEW_INSET = 6;

/**
 * The session's turns, oldest first. It is a log for assistive tech but not a live
 * region: streamed text would otherwise be read out chunk by chunk. The pane announces
 * discrete events (approval needed, finished) instead. The allowance is the pane's last
 * line, outside the log, so editing it never moves with the conversation.
 */
export function AgentTranscript({ session, actions }: { session: AgentSessionSnapshot | undefined; actions: Omit<TurnActions, 'reading'> }) {
  const turns = session?.turns ?? [];
  const lastTurn = turns.at(-1)?.id ?? null;
  const follow = useFollow(session?.version ?? null, lastTurn);
  const turnActions = useMemo<TurnActions>(() => ({ ...actions, reading: follow.hold }), [actions, follow.hold]);
  // The session holds a bounded tail of turns. The contract has no "trimmed" flag, so the
  // bound is stated as the policy once it is reached, without claiming what was dropped.
  const atBound = turns.length >= SESSION_TURN_LIMIT;
  return (
    <div className="agent__transcript-frame">
      <div
        ref={follow.attach}
        className="agent__transcript"
        role="log"
        aria-live="off"
        aria-label="Agent conversation"
        // A keyboard stop only while its content overflows, so it can be scrolled with keys.
        tabIndex={follow.scrollable ? 0 : undefined}
        onScroll={follow.onScroll}
      >
        <div className="agent__transcript-content">
          {atBound ? <p className="agent-older">Only the latest {SESSION_TURN_LIMIT} requests are kept.</p> : null}
          {turns.length ? (
            <ol className="agent-turns">
              {turns.map((turn) => (
                <AgentTurnView key={turn.id} turn={turn} actions={turnActions} />
              ))}
            </ol>
          ) : (
            <p className="agent-empty">Describe a change. Every edit lands in History and can be undone.</p>
          )}
        </div>
      </div>
      {follow.behind ? (
        <button type="button" className="agent-jump" onClick={follow.jump}>
          <Icon name="chevron" size={10} className="agent-jump__icon" />
          Latest
        </button>
      ) : null}
    </div>
  );
}

/**
 * Where a following reader is kept: the end of the conversation, or, while a decision is
 * pending and its review is taller than the view, the top of that review (its decision
 * row sticks to the bottom), so the question is never scrolled away above its buttons.
 */
function liveEdge(element: HTMLElement): number {
  const end = Math.max(0, element.scrollHeight - element.clientHeight);
  const card = element.querySelector<HTMLElement>('.agent-approval');
  if (!card) return end;
  const box = card.getBoundingClientRect();
  if (box.height + 2 * FOLLOW_SLACK <= element.clientHeight) return end;
  const top = box.top - element.getBoundingClientRect().top + element.scrollTop - REVIEW_INSET;
  return Math.max(0, Math.min(end, top));
}

/**
 * Keeps the newest output in view while the reader is at the live edge, and leaves the
 * position alone (offering a jump back) while they read earlier turns. A new turn means
 * the user just sent something, so it is always shown.
 */
function useFollow(version: number | null, lastTurn: number | null) {
  const [element, setElement] = useState<HTMLDivElement | null>(null);
  const following = useRef(true);
  const [behind, setBehind] = useState(false);
  const [scrollable, setScrollable] = useState(false);
  const seenTurn = useRef(lastTurn);
  const reviewShown = useRef(false);
  // The last position this hook or the reader left the view at. A scroll event that is not
  // a move by the reader (our own jump, or text reflowing after fonts load) keeps following.
  const lastTop = useRef(0);
  const toEdge = useCallback((target: HTMLElement) => {
    target.scrollTop = liveEdge(target);
    lastTop.current = target.scrollTop;
  }, []);

  // Observe the actual mount: readiness can replace this element without the pane
  // closing. Content growth and pane resizes keep a following reader at the edge.
  useLayoutEffect(() => {
    if (!element) return;
    const update = () => {
      setScrollable(element.scrollHeight > element.clientHeight + 1);
      if (following.current) toEdge(element);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    for (const child of element.children) observer.observe(child);
    return () => observer.disconnect();
  }, [element, toEdge]);

  useLayoutEffect(() => {
    if (!element) return;
    // A new request was sent, or a decision was recorded: the agent has the turn again.
    const review = element.querySelector('.agent-approval') !== null;
    if (lastTurn !== seenTurn.current || (reviewShown.current && !review)) following.current = true;
    seenTurn.current = lastTurn;
    reviewShown.current = review;
    if (following.current) {
      toEdge(element);
      setBehind(false);
    } else {
      setBehind(true);
    }
  }, [element, version, lastTurn, toEdge]);

  const onScroll = useCallback(() => {
    if (!element) return;
    const top = element.scrollTop;
    const edge = liveEdge(element);
    if (Math.abs(top - edge) <= FOLLOW_SLACK) {
      following.current = true;
      setBehind(false);
    } else if (top !== lastTop.current) {
      // The reader moved: back up, or further into a pending review than its question.
      following.current = false;
    }
    lastTop.current = top;
  }, [element]);

  // Expanding something to read it (a long prompt, the exact arguments) holds the view.
  const hold = useCallback(() => {
    following.current = false;
    if (element) lastTop.current = element.scrollTop;
  }, [element]);

  const jump = useCallback(() => {
    if (!element) return;
    following.current = true;
    toEdge(element);
    setBehind(false);
    // The button goes away; the conversation keeps the focus so reading can continue.
    element.focus();
  }, [element, toEdge]);

  return { attach: setElement, scrollable, behind, onScroll, jump, hold };
}
