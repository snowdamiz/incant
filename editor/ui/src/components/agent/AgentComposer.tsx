import { useId, useLayoutEffect, useRef } from 'react';
import type { AgentApprovalMode } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import { APPROVAL_MODES, DEFAULT_MODEL, MODEL_LIMIT_BYTES } from './agentText';

/**
 * The one action button: Send while the agent is idle, Stop while a turn can be
 * cancelled, and a quiet wait (Sending…, Stopping…, Finishing…) in between. It is one
 * element throughout, so keyboard focus stays on it as the state changes.
 */
export interface ComposerAction {
  readonly kind: 'send' | 'stop' | 'wait';
  readonly label: string;
  readonly title: string;
  readonly unavailable: boolean;
  readonly onActivate: () => void;
}

/** The message field grows with its text up to this height, then scrolls. */
const MAX_INPUT_HEIGHT = 132;

export function AgentComposer({
  draft,
  onDraft,
  model,
  onModel,
  approval,
  onApproval,
  action,
  onSubmit,
  inputDisabled,
  notes,
}: {
  draft: string;
  onDraft: (value: string) => void;
  model: string;
  onModel: (value: string) => void;
  approval: AgentApprovalMode;
  onApproval: (mode: AgentApprovalMode) => void;
  action: ComposerAction;
  /** Mod+Enter in the message field. */
  onSubmit: () => void;
  /** True only when the host cannot take messages at all. */
  inputDisabled: boolean;
  /** One line under the field, e.g. why the last send failed, with its tone. */
  notes: readonly { readonly id: string; readonly text: string; readonly tone: 'error' | 'quiet' }[];
}) {
  const input = useRef<HTMLTextAreaElement>(null);
  const ids = useId();
  const modeId = `${ids}-mode`;
  const mode = APPROVAL_MODES.find((entry) => entry.value === approval) ?? APPROVAL_MODES[0]!;

  useLayoutEffect(() => {
    const element = input.current;
    if (!element) return;
    element.style.height = '';
    // jsdom has no layout; leave its height alone there.
    if (element.scrollHeight > 0) element.style.height = `${Math.min(element.scrollHeight, MAX_INPUT_HEIGHT)}px`;
  }, [draft]);

  const commitModel = () => {
    const trimmed = model.trim();
    if (trimmed !== model || !trimmed) onModel(trimmed || DEFAULT_MODEL);
  };

  return (
    <div className="composer">
      <label htmlFor="agent-input" className="visually-hidden">
        Message to the agent
      </label>
      <textarea
        ref={input}
        id="agent-input"
        className="composer__input"
        rows={1}
        placeholder="Ask for a change…"
        disabled={inputDisabled}
        aria-describedby={notes.length ? notes.map((note) => note.id).join(' ') : undefined}
        data-agent-rescue=""
        value={draft}
        onChange={(event) => onDraft(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            onSubmit();
          }
        }}
      />
      {notes.map((note) => (
        <p key={note.id} id={note.id} className={`composer__note composer__note--${note.tone}`}>
          {note.tone === 'error' ? <Icon name="error" size={12} className="composer__note-icon" /> : null}
          {note.text}
        </p>
      ))}
      <div className="composer__bar">
        <input
          className="composer__model"
          aria-label="Model"
          title={`Model for the next message. Clear it to use the default, ${DEFAULT_MODEL}. Incant cannot list models yet.`}
          value={model}
          maxLength={MODEL_LIMIT_BYTES}
          spellCheck={false}
          autoComplete="off"
          autoCapitalize="off"
          style={{ width: `calc(${Math.min(Math.max(model.length, 4), 32)}ch + 10px)` }}
          onChange={(event) => onModel(event.target.value)}
          onBlur={commitModel}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              commitModel();
              input.current?.focus();
            }
          }}
        />
        {/* The native select keeps its keyboard, menu and spoken value; it is transparent over
            the visible label so the control is as wide as the chosen mode, not the longest. */}
        <span className={`composer__mode${approval === 'auto' ? ' is-unguarded' : ''}`} title={mode.detail}>
          <span className="composer__mode-label" aria-hidden="true">
            {mode.label}
          </span>
          <Icon name="chevron" size={10} className="composer__mode-chevron" />
          <select aria-label="Approval" aria-describedby={modeId} value={approval} onChange={(event) => onApproval(event.target.value as AgentApprovalMode)}>
            {APPROVAL_MODES.map((entry) => (
              <option key={entry.value} value={entry.value}>
                {entry.label}
              </option>
            ))}
          </select>
          <span id={modeId} className="visually-hidden">
            {mode.detail}
          </span>
        </span>
        <button
          type="button"
          className={`send-button send-button--${action.kind}`}
          aria-label={action.label}
          aria-disabled={action.unavailable || undefined}
          aria-keyshortcuts={action.kind === 'send' ? 'Meta+Enter Control+Enter' : undefined}
          title={action.title}
          onClick={action.onActivate}
        >
          {action.kind === 'stop' ? (
            <span className="stop-glyph" aria-hidden="true" />
          ) : action.kind === 'wait' ? (
            <span className="spinner spinner--small" aria-hidden="true" />
          ) : (
            <Icon name="send" size={14} />
          )}
        </button>
      </div>
    </div>
  );
}
