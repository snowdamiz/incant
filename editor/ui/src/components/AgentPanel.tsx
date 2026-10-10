import { useCallback, useState } from 'react';
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
 * The native bridge reports that the agent is not part of the editor yet: today in
 * plain words, in earlier builds by naming an internal milestone (a headless command, a
 * "Phase 0 provider spike"). The contract has no reason code, so the sentence is
 * recognised here. Presentation only: the note gets its own title instead of inviting a
 * sign-in that would not enable the agent, and the state stays "unavailable".
 */
export function isNotInEditorReason(reason: string): boolean {
  return /not (?:available in|part of) the editor|headless agent command|provider spike|\bphase \d/i.test(reason);
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
  const [transcript, scrollable] = useOverflow();
  const agent = snapshot?.agent;
  const provider = snapshot?.provider;
  const canSend = agent?.status === 'idle' && capabilities.has('agent.send');
  const project = projectState(snapshot);
  const notInEditor = agent?.status === 'unavailable' && isNotInEditorReason(agent.reason);
  const reason = !snapshot
    ? 'No engine is connected.'
    : project.kind === 'failed'
      ? // The banner shows the failure itself; repeating it here adds nothing.
        'The agent needs an open project.'
      : agent?.status === 'unavailable'
        ? notInEditor
          ? 'The agent is not available in the editor yet.'
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
          : signedIn || notInEditor
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
                <p className="agent-empty">Describe a change. Every edit lands in History and can be undone.</p>
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
            // One sentence: why the agent cannot run, and the single next step when one
            // exists. The header already carries the state; no title repeats it.
            <div className="agent-note">
              <p className="agent-note__body" id="agent-reason">
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
              ) : null}
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}

/** True while the element's content is taller than its box; tracks resizes of both. */
function useOverflow() {
  const [overflowing, setOverflowing] = useState(false);
  // Observe the actual mount: readiness can replace this element without changing
  // whether the surrounding panel is open. React runs this cleanup on replacement.
  const attach = useCallback((element: HTMLDivElement | null) => {
    if (!element) return;
    const update = () => setOverflowing(element.scrollHeight > element.clientHeight + 1);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    for (const child of element.children) observer.observe(child);
    return () => observer.disconnect();
  }, []);
  return [attach, overflowing] as const;
}
