import { useEffect, useRef, useState } from 'react';
import type { MouseEvent } from 'react';
import { Icon } from '../icons/Icon';
import { WispMark } from '../icons/WispMark';
import { projectState } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';
import { ChatGPTButton } from './AccountDialog';

/**
 * Agent chat area. It never fabricates a transcript or a reply: without a
 * running agent and the `agent.send` capability, the composer is disabled and
 * the panel explains why. Transcript rendering is a follow-up packet once the
 * bridge exposes conversation events.
 */
export function AgentPanel() {
  const { snapshot, capabilities, ask, explainUnavailable, openAccount } = useShell();
  const [draft, setDraft] = useState('');
  const transcript = useRef<HTMLDivElement>(null);
  const scrollable = useOverflow(transcript);
  const agent = snapshot?.agent;
  const provider = snapshot?.provider;
  const canSend = agent?.status === 'idle' && capabilities.has('agent.send');
  const project = projectState(snapshot);
  const reason = !snapshot
    ? 'No engine is connected.'
    : project.kind === 'failed'
      ? // The banner shows the failure itself; repeating it here adds nothing.
        'The agent needs an open project.'
      : agent?.status === 'unavailable'
      ? agent.reason
      : !capabilities.has('agent.send')
        ? 'This bridge does not support sending messages to the agent.'
        : agent?.status === 'running'
          ? 'The agent is working on the current request.'
          : '';
  const ready = agent?.status === 'idle' || agent?.status === 'running';

  // Starts sign-in and opens the account dialog, which shows browser and validation progress.
  const connect = (event: MouseEvent<HTMLButtonElement>) => {
    openAccount(event.currentTarget);
    if (capabilities.has('provider.connect')) void ask({ type: 'provider.connect', method: 'oauth' });
  };
  const signedIn = provider?.status === 'connected';

  return (
    <section className="panel panel--agent" data-region="agent" aria-labelledby="agent-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="agent-title" className="panel__title">
          Agent
        </h2>
      </header>
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
            <WispMark size={28} />
          </span>
          <p className="agent-empty__title">
            {ready ? 'Describe a change' : signedIn ? 'Agent not ready' : 'Use your ChatGPT account'}
          </p>
          <p className="agent-empty__body">
            {ready
              ? 'Every edit the agent makes lands in History and can be undone.'
              : `${reason} Everything else works offline, no account needed.`}
          </p>
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
          rows={canSend ? 2 : 1}
          placeholder={canSend ? 'Ask the agent to change the scene…' : 'Agent unavailable'}
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
          {snapshot && !ready && provider?.status === 'not-connected' ? (
            <ChatGPTButton small available={capabilities.has('provider.connect')} onClick={connect} />
          ) : snapshot && !ready && provider && provider.status !== 'connected' ? (
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
          ) : (
            <span className="composer__hint">{canSend ? '⌘↵ to send' : ''}</span>
          )}
          <button type="submit" className="send-button" aria-label="Send" aria-disabled={!canSend || undefined} title="Send (⌘Enter)">
            <Icon name="send" size={14} />
          </button>
        </div>
      </form>
    </section>
  );
}

/** True while the element's content is taller than its box; tracks resizes of both. */
function useOverflow(ref: React.RefObject<HTMLElement | null>): boolean {
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
  }, [ref]);
  return overflowing;
}
