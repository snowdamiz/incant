import { useEffect, useRef, useState } from 'react';
import type { MouseEvent } from 'react';
import { Icon } from '../icons/Icon';
import { WispMark } from '../icons/WispMark';
import { projectState } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';
import { ChatGPTButton } from './AccountDialog';

/** True when the agent can take or is working on a request, so the composer is useful. */
export function agentReady(status: string | undefined): boolean {
  return status === 'idle' || status === 'running';
}

/**
 * The native bridge explains the missing in-editor agent with internal milestone
 * wording (a headless command, a "Phase 0 provider spike"). Presentation only: such
 * a reason is shown in plain words and the state stays "unavailable".
 */
export function isMilestoneReason(reason: string): boolean {
  return /headless agent command|provider spike|\bphase \d/i.test(reason);
}

/**
 * Agent area under the Inspector. It never fabricates a transcript or a reply.
 * While the agent cannot run, the pane is one short note with its single next
 * step (sign in, or why it is unavailable) instead of a disabled composer, so the
 * Inspector keeps the column. The header collapses the pane to one line either way.
 * Transcript rendering is a follow-up once the bridge exposes conversation events.
 */
export function AgentPanel({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { snapshot, capabilities, ask, explainUnavailable, openAccount } = useShell();
  const [draft, setDraft] = useState('');
  const transcript = useRef<HTMLDivElement>(null);
  const scrollable = useOverflow(transcript, open);
  const agent = snapshot?.agent;
  const provider = snapshot?.provider;
  const canSend = agent?.status === 'idle' && capabilities.has('agent.send');
  const project = projectState(snapshot);
  const milestone = agent?.status === 'unavailable' && isMilestoneReason(agent.reason);
  const reason = !snapshot
    ? 'No engine is connected.'
    : project.kind === 'failed'
      ? // The banner shows the failure itself; repeating it here adds nothing.
        'The agent needs an open project.'
      : agent?.status === 'unavailable'
        ? milestone
          ? 'The agent is not part of the editor yet.'
          : agent.reason
        : !capabilities.has('agent.send')
          ? 'This bridge does not support sending messages to the agent.'
          : agent?.status === 'running'
            ? 'The agent is working on the current request.'
            : '';
  const ready = agentReady(agent?.status);
  const signedIn = provider?.status === 'connected';
  const signingIn = provider?.status === 'connecting' || provider?.status === 'checking';
  const status = !snapshot
    ? 'No engine'
    : ready
      ? agent?.status === 'running'
        ? 'Working…'
        : 'Ready'
      : provider?.status === 'error'
        ? 'Sign-in problem'
        : signingIn
          ? 'Signing in…'
          : signedIn || milestone
            ? 'Unavailable'
            : 'Not signed in';
  const tone = ready ? 'ok' : provider?.status === 'error' ? 'error' : signingIn ? 'pending' : 'idle';

  // Starts sign-in and opens the account dialog, which shows browser and validation progress.
  const connect = (event: MouseEvent<HTMLButtonElement>) => {
    openAccount(event.currentTarget);
    if (capabilities.has('provider.connect')) void ask({ type: 'provider.connect', method: 'oauth' });
  };

  return (
    <section className={`panel panel--agent${open ? ' is-open' : ''}`} data-region="agent" aria-labelledby="agent-title" tabIndex={-1}>
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
            <span className={`agent__status agent__status--${tone}`}>
              <span className="visually-hidden">, </span>
              {status}
            </span>
          </button>
        </h2>
      </header>
      {open ? (
        <div id="agent-body" className="agent__body">
          {ready ? (
            <>
              {/* Focusable only while its content overflows (a short panel), so keyboard users can scroll it. */}
              <div
                ref={transcript}
                className="agent__transcript"
                role="log"
                aria-label="Agent conversation"
                tabIndex={scrollable ? 0 : undefined}
              >
                <div className="agent-empty">
                  <span className="agent-empty__mark" aria-hidden="true">
                    <WispMark size={24} />
                  </span>
                  <p className="agent-empty__title">Describe a change</p>
                  <p className="agent-empty__body">Every edit the agent makes lands in History and can be undone.</p>
                </div>
              </div>
              <form
                className={`composer${canSend ? '' : ' is-disabled'}`}
                onSubmit={(event) => {
                  event.preventDefault();
                  if (!canSend) {
                    explainUnavailable('agent.send');
                    return;
                  }
                  const text = draft.trim();
                  if (!text) return;
                  void ask({ type: 'agent.send', text }).then((ok) => ok && setDraft(''));
                }}
              >
                <label htmlFor="agent-input" className="visually-hidden">
                  Message to the agent
                </label>
                <textarea
                  id="agent-input"
                  className="composer__input"
                  rows={2}
                  placeholder={canSend ? 'Ask the agent to change the scene…' : 'The agent is busy'}
                  disabled={!canSend}
                  aria-describedby={canSend ? undefined : 'agent-reason'}
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) event.currentTarget.form?.requestSubmit();
                  }}
                />
                <span id="agent-reason" className="visually-hidden">
                  {reason}
                </span>
                <div className="composer__bar">
                  <span className="composer__hint">{canSend ? '⌘↵ to send' : ''}</span>
                  <button type="submit" className="send-button" aria-label="Send" aria-disabled={!canSend || undefined} title="Send (⌘Enter)">
                    <Icon name="send" size={14} />
                  </button>
                </div>
              </form>
            </>
          ) : (
            <div className="agent-note">
              <p className="agent-note__title">
                {!snapshot ? 'No engine connected' : milestone ? 'Not in the editor yet' : signedIn ? 'Agent not ready' : 'Use your ChatGPT account'}
              </p>
              <p className="agent-note__body" id="agent-reason">
                {/* The title already says the agent is not in the editor; the body adds what still works. */}
                {milestone || !snapshot ? '' : `${reason} `}Everything else works offline, no account needed.
              </p>
              {snapshot && provider?.status === 'not-connected' ? (
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
              ) : null}
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}

/** True while the element's content is taller than its box; tracks resizes of both. */
function useOverflow(ref: React.RefObject<HTMLElement | null>, active: boolean): boolean {
  const [overflowing, setOverflowing] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const update = () => setOverflowing(element.scrollHeight > element.clientHeight + 1);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    for (const child of element.children) observer.observe(child);
    return () => observer.disconnect();
  }, [ref, active]);
  return overflowing;
}
