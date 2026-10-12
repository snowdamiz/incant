import { useCallback, useId, useLayoutEffect, useRef } from 'react';
import type { KeyboardEvent } from 'react';
import { unavailableMessage } from '../../bridge/resolve';
import { Icon } from '../../icons/Icon';
import { useShell } from '../../shell/ShellContext';
import type { Allowance, AllowanceInput } from './tokenAllowance';
import { allowanceSpoken, allowanceText, allowanceTitle, formatCount, parseAllowance, problemText } from './tokenAllowance';
import type { AgentMemory, AgentPaneState, AllowanceEdit } from './agentMemory';

interface Feedback {
  readonly text: string;
  readonly tone: 'error' | 'warning' | 'quiet';
}

/** A typed value's problem is reported once it is wrong, or, for an empty field, once a save was tried. */
const reportable = (edit: AllowanceEdit, parsed: AllowanceInput) => !parsed.ok && (parsed.problem !== 'empty' || edit.attempted);

/**
 * One line under the edit: why it cannot be saved, what saving it would do, or the usage
 * it is weighed against. Only the most important applies.
 */
function feedbackFor(edit: AllowanceEdit, parsed: AllowanceInput, allowance: Allowance, saving: boolean, activeTurn: boolean): Feedback | null {
  if (edit.error) return { text: edit.error, tone: 'error' };
  if (!parsed.ok && reportable(edit, parsed)) return { text: problemText(parsed.problem), tone: 'error' };
  if (saving) return null;
  // Native hosts end the running request when the allowance changes (docs/spikes/agent-project-budget.md).
  if (activeTurn) return { text: 'Saving stops the current request.', tone: 'warning' };
  if (allowance.cap !== edit.base) return { text: `Changed to ${formatCount(allowance.cap)} by another edit.`, tone: 'quiet' };
  const used = allowance.used;
  if (parsed.ok && parsed.value === 0) return { text: '0 turns the agent off.', tone: 'quiet' };
  if (parsed.ok && used !== null && parsed.value <= used) return { text: `No new requests: ${formatCount(used)} already used.`, tone: 'quiet' };
  if (used !== null) return { text: `${formatCount(used)} used${allowance.uncertain ? ' (some estimated)' : ''}`, tone: 'quiet' };
  return null;
}

/**
 * The pane's last line: how much of the token allowance is used and, where the project
 * has an authored allowance, Change, which edits it in place. A save is a project edit
 * through the shared command bus (`agent.budget.set`), journaled and undoable like any
 * other; recorded usage is never reset or edited here. Older hosts keep a per-session
 * allowance, shown without Change.
 */
export function AgentAllowance({
  allowance,
  project,
  memory,
  pane,
  activeTurn,
}: {
  allowance: Allowance;
  /** The open project's ID. An edit started in another project is not shown or saved. */
  project: string;
  memory: AgentMemory;
  pane: Pick<AgentPaneState, 'allowance' | 'allowanceSaving'>;
  /** A request is running, which a saved change would stop. */
  activeTurn: boolean;
}) {
  const { capabilities, dispatch, announce, explainUnavailable, bridge } = useShell();
  const supported = capabilities.has('agent.budget.set');
  const editable = allowance.scope === 'project';
  const edit = editable && supported && pane.allowance?.project === project ? pane.allowance : null;
  const saving = pane.allowanceSaving?.project === project;
  const ids = useId();
  const inputId = `${ids}-input`;
  const noteId = `${ids}-note`;
  const input = useRef<HTMLInputElement>(null);
  // Focus moves only after the user's own open, cancel or save, never on a remount.
  const focusInput = useRef(false);
  const focusChange = useRef(false);

  useLayoutEffect(() => {
    if (!edit || !focusInput.current) return;
    focusInput.current = false;
    input.current?.focus();
    // Typing replaces the whole value, as in a rename.
    input.current?.select();
  }, [edit]);

  // After the editor closes, its Change takes focus, unless focus has gone elsewhere since.
  const attachChange = useCallback((element: HTMLButtonElement | null) => {
    if (!element || !focusChange.current) return;
    focusChange.current = false;
    const active = document.activeElement;
    if (active === null || active === document.body) element.focus();
  }, []);

  const begin = () => {
    if (!supported) {
      explainUnavailable('agent.budget.set');
      return;
    }
    focusInput.current = true;
    memory.set({ allowance: { project, draft: String(allowance.cap), base: allowance.cap, error: null, attempted: false } });
  };

  const close = () => {
    focusChange.current = true;
    memory.set({ allowance: null });
  };

  const cancel = () => {
    // A save in flight finishes first; its result decides what the editor shows.
    if (memory.get().allowanceSaving?.project === project) return;
    close();
  };

  const save = async () => {
    const state = memory.get();
    const current = state.allowance;
    if (state.allowanceSaving || !current || current.project !== project) return;
    const parsed = parseAllowance(current.draft);
    if (!parsed.ok) {
      memory.set({ allowance: { ...current, attempted: true } });
      announce(problemText(parsed.problem));
      input.current?.focus();
      return;
    }
    if (!capabilities.has('agent.budget.set')) {
      explainUnavailable('agent.budget.set');
      return;
    }
    // Nothing to change: no command, so History gains no empty entry.
    if (parsed.value === allowance.cap) {
      close();
      return;
    }
    memory.set({ allowanceSaving: { project, draft: current.draft, value: parsed.value } });
    const result = await dispatch({ type: 'agent.budget.set', maxTokens: parsed.value });
    // One decision, read and applied together (the store is synchronous). The editor may
    // be gone or belong to another project by now. A success closes it only if nothing new
    // was typed meanwhile; a failure keeps what was typed, with the host's reason.
    const open = memory.get().allowance;
    if (!open || open.project !== project) memory.set({ allowanceSaving: null });
    else if (!result.ok) memory.set({ allowanceSaving: null, allowance: { ...open, error: result.error.message } });
    else if (open.draft === current.draft) {
      focusChange.current = true;
      memory.set({ allowanceSaving: null, allowance: null });
    } else memory.set({ allowanceSaving: null, allowance: { ...open, base: parsed.value, error: null } });
    // Saved, even if refreshing the agent session afterwards fails: that is the session's own error.
    if (result.ok) announce(`Token allowance set to ${formatCount(parsed.value)}.`);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLFormElement>) => {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    cancel();
  };

  if (edit) {
    const parsed = parseAllowance(edit.draft);
    const invalid = reportable(edit, parsed);
    const feedback = feedbackFor(edit, parsed, allowance, saving, activeTurn);
    return (
      <div className="agent-allowance">
        <form
          // A very long value takes the field's whole row, so every digit stays in view.
          className={`agent-allowance__form${edit.draft.length > 12 ? ' is-long' : ''}`}
          noValidate
          onKeyDown={onKeyDown}
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <label className="agent-allowance__label" htmlFor={inputId}>
            Allowance
          </label>
          {/* The field is as wide as its digits, so the unit follows the number; a press anywhere in the box puts the caret at its end. */}
          <span
            className="agent-allowance__field"
            onMouseDown={(event) => {
              const element = input.current;
              if (!element || event.target === element) return;
              event.preventDefault();
              element.focus();
              element.setSelectionRange(element.value.length, element.value.length);
            }}
          >
            <input
              ref={input}
              id={inputId}
              className="agent-allowance__input"
              // The visible label plus what it is an allowance of; the unit is drawn after the digits.
              aria-label="Allowance in tokens for this project"
              style={{ width: `${Math.min(Math.max(edit.draft.length, 1), 24) + 1}ch` }}
              value={edit.draft}
              inputMode="numeric"
              autoComplete="off"
              spellCheck={false}
              aria-invalid={invalid || undefined}
              aria-describedby={feedback ? noteId : undefined}
              onChange={(event) => {
                const draft = event.target.value;
                memory.set((state) => (state.allowance ? { allowance: { ...state.allowance, draft, error: null } } : {}));
              }}
            />
            <span className="agent-allowance__unit" aria-hidden="true">
              tokens
            </span>
          </span>
          <span className="agent-allowance__actions">
            <button type="button" className="button button--small button--ghost" aria-disabled={saving || undefined} onClick={cancel}>
              Cancel
            </button>
            {/* Raised, not the accent: Send or Stop sits just above it and keeps the one accent action. */}
            <button type="submit" className="button button--small" aria-disabled={saving || !parsed.ok || undefined}>
              {saving ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
              {saving ? 'Saving…' : 'Save'}
            </button>
          </span>
          {feedback ? (
            <p id={noteId} className={`agent-allowance__note agent-allowance__note--${feedback.tone}`}>
              {feedback.tone === 'quiet' ? null : <Icon name={feedback.tone} size={12} className="agent-allowance__note-icon" />}
              <span>{feedback.text}</span>
            </p>
          ) : null}
        </form>
      </div>
    );
  }

  const spoken = allowanceSpoken(allowance);
  return (
    <div className="agent-allowance">
      <p className="agent-allowance__line">
        {allowance.used !== null ? <span className="visually-hidden">{allowance.scope === 'project' ? 'Project token allowance: ' : 'Token allowance: '}</span> : null}
        <span className={`agent-usage agent-usage--${allowance.level}`} title={allowanceTitle(allowance)}>
          {allowanceText(allowance)}
        </span>
        {spoken ? <span className="visually-hidden"> {spoken}</span> : null}
        {editable ? (
          <button
            ref={attachChange}
            type="button"
            className="agent-link agent-allowance__change"
            aria-label="Change allowance"
            aria-disabled={supported ? undefined : true}
            title={supported ? 'Change the project’s token allowance' : unavailableMessage('agent.budget.set', bridge)}
            onClick={begin}
          >
            Change
          </button>
        ) : null}
      </p>
    </div>
  );
}
