import { useState } from 'react';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';

/**
 * Agent chat area. It never fabricates a transcript or a reply: without a
 * running agent and the `agent.send` capability, the composer is disabled and
 * the panel explains why. Transcript rendering is a follow-up packet once the
 * bridge exposes conversation events.
 */
export function AgentPanel() {
  const { snapshot, capabilities, ask, explainUnavailable } = useShell();
  const [draft, setDraft] = useState('');
  const agent = snapshot?.agent;
  const provider = snapshot?.provider;
  const canSend = agent?.status === 'idle' && capabilities.has('agent.send');
  const reason = !snapshot
    ? 'No engine is connected.'
    : agent?.status === 'unavailable'
      ? agent.reason
      : !capabilities.has('agent.send')
        ? 'This bridge does not support sending messages to the agent.'
        : agent?.status === 'running'
          ? 'The agent is working on the current request.'
          : '';
  const ready = agent?.status === 'idle' || agent?.status === 'running';

  const connect = () => {
    if (!capabilities.has('provider.connect')) explainUnavailable('provider.connect');
    else void ask({ type: 'provider.connect', method: 'oauth' });
  };

  return (
    <section className="panel panel--agent" data-region="agent" aria-labelledby="agent-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="agent-title" className="panel__title">
          Agent
        </h2>
      </header>
      <div className="agent__transcript" role="log" aria-label="Agent conversation">
        <div className="agent-empty">
          <span className="agent-empty__mark" aria-hidden="true">
            <Icon name="spark" size={18} />
          </span>
          <p className="agent-empty__title">{ready ? 'Describe a change' : 'Bring your own AI'}</p>
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
          {snapshot && !ready && provider?.status !== 'connected' ? (
            <button
              type="button"
              className="button button--accent button--small"
              aria-disabled={!capabilities.has('provider.connect') || undefined}
              onClick={connect}
            >
              <Icon name="plug" size={12} />
              Connect OpenAI…
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
