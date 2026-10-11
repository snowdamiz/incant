import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../../App';
import type {
  AgentSessionSnapshot,
  AgentTurnSnapshot,
  BridgeError,
  BridgeResult,
  BridgeSnapshot,
  EditorBridge,
  EditorCommand,
  HostRequest,
  ProviderState,
  Ulid,
} from '../../bridge/contract';
import { fixtureSnapshot } from '../../bridge/fixture';
import { NativeBridge } from '../../../../bridge/native';
import type { EngineRead, Invoke } from '../../../../bridge/native';
import projectSchema from '../../../../../schemas/Project.schema.json';
import { REFRESH_GRACE_MS } from '../AgentPanel';

/*
 * The token allowance against DETERMINISTIC TEST DOUBLES. Sessions are written in the
 * native snake_case form. Nothing here is a live provider, a native host or an engine.
 */

const MAX = Number.MAX_SAFE_INTEGER;
const SIGNED_IN: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'ada@example.com', accounts: [{ id: 'a', label: 'ada@example.com' }], activeAccount: 'a' };
const SIGNED_OUT: ProviderState = { status: 'not-connected', provider: 'openai' };
const RUNNING = { status: 'running', conversation: 'test' } as const;
const OK: BridgeResult = { ok: true };
const TRANSPORT: BridgeError = { code: 'agent.transport', message: 'Could not contact the native agent. Refresh the session to retry.' };
const REFRESH_REASON = 'Refresh the native agent session after account or project settings changes.';
const EXHAUSTED_REASON = 'The native agent session token allowance is exhausted.';
const ALL = ['agent.send', 'agent.cancel', 'agent.approve', 'provider.connect', 'agent.budget.set'];

function turn(id: number, fields: Partial<AgentTurnSnapshot> = {}): AgentTurnSnapshot {
  return {
    id,
    request: { prompt: `Request ${id}`, model: 'gpt-6-astra', approval: 'destructive' },
    status: 'completed',
    text: '',
    approvals: [],
    pending: null,
    report: null,
    failure: null,
    charged_tokens: 0,
    accounting_uncertain: false,
    ...fields,
  };
}
function session(used: number, max: number, fields: Partial<AgentSessionSnapshot> = {}): AgentSessionSnapshot {
  return { version: 1, budget: { max_tokens: max, used_tokens: used }, accounting_uncertain: false, worker_running: false, turns: [], ...fields };
}
const awaiting = (): AgentTurnSnapshot =>
  turn(3, {
    status: 'awaiting_approval',
    text: 'I will lift the crate.',
    approvals: [{ id: 7, tool: 'doc_patch', bytes: 80, decision: 'pending' }],
    pending: { turn: 3, id: 7, tool: 'doc_patch', arguments: { description: 'Lift the crate', expected_revision: 4, ops: [] } },
  });
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/**
 * TEST DOUBLE, not a bridge implementation: the sample project with an allowance the test
 * controls. It records every command and request and answers as the test says. It never
 * changes its own snapshot, so the pane is never credited with state it invented; the
 * test publishes what a host would.
 */
function allowanceHost(
  initial: { budget?: number; session?: AgentSessionSnapshot; agent?: BridgeSnapshot['agent']; provider?: ProviderState; agentError?: BridgeError },
  options: { capabilities?: string[]; dispatch?: (command: EditorCommand) => Promise<BridgeResult>; refreshAgent?: () => Promise<void> } = {},
) {
  let snapshot: BridgeSnapshot = {
    ...fixtureSnapshot('sample'),
    provider: initial.provider ?? SIGNED_IN,
    agent: initial.agent ?? { status: 'idle' },
    ...(initial.budget === undefined ? {} : { projectAgentBudget: { maxTokens: initial.budget } }),
    ...(initial.session ? { agentSession: initial.session } : {}),
    ...(initial.agentError ? { agentError: initial.agentError } : {}),
  };
  let capabilities = options.capabilities ?? ALL;
  const listeners = new Set<() => void>();
  const commands: EditorCommand[] = [];
  const requests: HostRequest[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    get capabilities() {
      return capabilities;
    },
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dispatch: (command) => {
      commands.push(command);
      return options.dispatch?.(command) ?? Promise.resolve(OK);
    },
    request: (request) => {
      requests.push(request);
      return Promise.resolve(OK);
    },
    ...(options.refreshAgent ? { refreshAgent: options.refreshAgent } : {}),
  };
  /** Publishes fields (and removes the optional ones named in `remove`), as a host would. */
  const update = (patch: Partial<BridgeSnapshot> & { readonly capabilities?: string[] }, remove: readonly ('agentError' | 'agentSession' | 'projectAgentBudget')[] = []) =>
    act(() => {
      const { capabilities: nextCapabilities, ...fields } = patch;
      if (nextCapabilities) capabilities = nextCapabilities;
      const next: Record<string, unknown> = { ...snapshot, ...fields };
      for (const key of remove) delete next[key];
      snapshot = next as unknown as BridgeSnapshot;
      for (const listener of listeners) listener();
    });
  render(<App resolution={{ kind: 'bridge', bridge }} />);
  return { bridge, commands, requests, update };
}

const pane = () => document.querySelector<HTMLElement>('[data-region="agent"]')!;
const line = () => pane().querySelector('.agent-usage');
const header = () => pane().querySelector('.agent__status')!.textContent!.replace(/^,\s*/, '');
const statusLine = () => document.querySelector('.statusbar__message')!.textContent;
const change = () => within(pane()).getByRole('button', { name: 'Change allowance' });
const field = () => within(pane()).getByRole('textbox', { name: 'Allowance in tokens for this project' }) as HTMLInputElement;
const save = () => within(pane()).getByRole('button', { name: /^Sav/ });
const note = () => pane().querySelector('.agent-allowance__note');
const type = (value: string) => fireEvent.change(field(), { target: { value } });
const enter = () => fireEvent.submit(field().form!);

afterEach(() => {
  vi.useRealTimers();
});

describe('Allowance line', () => {
  it('shows the project’s authored allowance and the recorded usage, never the session’s lagging copy', () => {
    allowanceHost({ budget: 250_000, session: session(6_532, 200_000) });
    expect(line()!.textContent).toBe('6,532 of 250,000 tokens used');
    expect(pane().querySelector('.agent-allowance__line')!.textContent).toContain('Project token allowance: ');
    expect(pane().querySelector('.agent-allowance__line')!.textContent).toContain('243,468 tokens left.');
    expect(pane().textContent).not.toContain('200,000');
    expect(change()).toBeTruthy();
  });

  it('shows the default allowance only because the host sent it', async () => {
    const host = allowanceHost({ budget: 1_000_000, session: session(0, 1_000_000) });
    expect(line()!.textContent).toBe('0 of 1,000,000 tokens used');
    fireEvent.click(change());
    expect(field().value).toBe('1000000');
    expect(document.activeElement).toBe(field());
    expect(host.commands).toEqual([]);
  });

  it('gives an older host no editor and keeps its allowance per session', () => {
    allowanceHost(
      { agent: { status: 'unavailable', reason: EXHAUSTED_REASON }, session: session(200_000, 200_000) },
      { capabilities: ['agent.send', 'agent.cancel', 'agent.approve'] },
    );
    expect(within(pane()).queryByRole('button', { name: /Change/ })).toBeNull();
    expect(within(pane()).getByText('This session’s token allowance is used up.')).toBeTruthy();
    expect(header()).toBe('Allowance used up');
    expect(pane().querySelector('.agent-allowance__line')!.textContent).toContain('Token allowance: ');
    expect(pane().querySelector('.agent-allowance__line')!.textContent).not.toContain('Project');
  });

  it('keeps usage above a lowered allowance visible, and invites raising it', () => {
    allowanceHost({ budget: 5_000, agent: { status: 'unavailable', reason: EXHAUSTED_REASON }, session: session(6_532, 5_000) });
    expect(line()!.textContent).toBe('6,532 of 5,000 tokens used');
    expect(line()!.classList.contains('agent-usage--out')).toBe(true);
    expect(pane().querySelector('.agent-allowance__line')!.textContent).toContain('0 tokens left.');
    expect(header()).toBe('Allowance used up');
    expect(within(pane()).getByText('Raise the allowance to send new requests.')).toBeTruthy();
    expect(screen.queryByLabelText('Message to the agent')).toBeNull();
  });

  it('says plainly that an allowance of 0 turns the agent off', () => {
    allowanceHost({ budget: 0, agent: { status: 'unavailable', reason: EXHAUSTED_REASON }, session: session(40, 0) });
    expect(header()).toBe('Off');
    expect(within(pane()).getByText('Raise the allowance to turn the agent on.')).toBeTruthy();
    expect(line()!.textContent).toBe('40 of 0 tokens used');
    expect(line()!.classList.contains('agent-usage--off')).toBe(true);
  });

  it('marks uncertain usage without naming a cause', () => {
    allowanceHost({ budget: 1_000_000, session: session(12_000, 1_000_000, { accounting_uncertain: true }) });
    expect(line()!.textContent).toBe('12,000 of 1,000,000 tokens used (some estimated)');
    expect(line()!.getAttribute('title')).toContain('could not be confirmed or saved exactly');
    expect(pane().querySelector('.agent-allowance__line')!.textContent).toContain('a conservative amount was recorded');
  });

  it('follows an allowance changed by Undo or another edit', () => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    host.update({ projectAgentBudget: { maxTokens: 400_000 }, agentSession: session(6_532, 400_000, { version: 2 }) });
    expect(line()!.textContent).toBe('6,532 of 400,000 tokens used');
  });

  it('shows the change once the host reports the capability, and drops it when the host does', () => {
    const host = allowanceHost({ session: session(10, 200_000) }, { capabilities: ['agent.send'] });
    expect(within(pane()).queryByRole('button', { name: /Change/ })).toBeNull();
    host.update({ projectAgentBudget: { maxTokens: 200_000 }, capabilities: ALL });
    fireEvent.click(change());
    expect(field().value).toBe('200000');
    host.update({ capabilities: ['agent.send'] }, ['projectAgentBudget']);
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(within(pane()).queryByRole('button', { name: /Change/ })).toBeNull();
    expect(line()!.textContent).toBe('10 of 200,000 tokens used');
    expect(host.commands).toEqual([]);
  });
});

describe('Changing the allowance', () => {
  it('saves the exact value once through the shared command, then returns focus to Change', async () => {
    const reply = deferred<BridgeResult>();
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) }, { dispatch: () => reply.promise });
    fireEvent.click(change());
    expect(field().selectionStart).toBe(0);
    expect(field().selectionEnd).toBe(6);
    type('500,000');
    enter();
    enter();
    fireEvent.click(save());
    expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: 500_000 }]);
    expect(save().textContent).toBe('Saving…');
    expect(save().getAttribute('aria-disabled')).toBe('true');
    expect(within(pane()).getByRole('button', { name: 'Cancel' }).getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(field(), { key: 'Escape' });
    expect(field()).toBeTruthy();
    // The host publishes the edit before its command resolves.
    host.update({ projectAgentBudget: { maxTokens: 500_000 } });
    expect(note()?.textContent ?? '').not.toContain('another edit');
    await act(async () => reply.resolve(OK));
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(document.activeElement).toBe(change());
    expect(line()!.textContent).toBe('6,532 of 500,000 tokens used');
    await waitFor(() => expect(statusLine()).toBe('Token allowance set to 500,000.'));
    expect(host.commands).toHaveLength(1);
  });

  it.each([0, MAX])('accepts %d and sends it exactly', async (value) => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    type(String(value));
    expect(field().getAttribute('aria-invalid')).toBeNull();
    enter();
    await waitFor(() => expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: value }]));
  });

  it.each([
    ['-1', 'The allowance can’t be negative.'],
    ['−5', 'The allowance can’t be negative.'],
    ['1.5', 'Enter a whole number of tokens.'],
    ['1,5', 'Enter a whole number of tokens.'],
    ['1e6', 'Enter a whole number of tokens.'],
    ['lots', 'Enter a whole number of tokens.'],
    ['9007199254740992', `The largest allowance is ${MAX.toLocaleString()} tokens.`],
  ])('refuses %j inline without sending', async (text, message) => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    type(text);
    expect(note()!.textContent).toBe(message);
    expect(field().getAttribute('aria-invalid')).toBe('true');
    expect(field().getAttribute('aria-describedby')).toBe(note()!.id);
    expect(save().getAttribute('aria-disabled')).toBe('true');
    enter();
    fireEvent.click(save());
    await waitFor(() => expect(statusLine()).toBe(message));
    expect(document.activeElement).toBe(field());
    expect(host.commands).toEqual([]);
  });

  it('reports an empty field only once a save is tried', () => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    type('');
    expect(field().getAttribute('aria-invalid')).toBeNull();
    expect(note()!.textContent).toBe('6,532 used');
    enter();
    expect(note()!.textContent).toBe('Enter a number of tokens.');
    expect(field().getAttribute('aria-invalid')).toBe('true');
    expect(host.commands).toEqual([]);
  });

  it('keeps what was typed and the host’s safe reason when a save fails', async () => {
    const error: BridgeError = { code: 'engine.revision-mismatch', message: 'Project revision changed.' };
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) }, { dispatch: () => Promise.resolve({ ok: false, error }) });
    fireEvent.click(change());
    type('300000');
    enter();
    await within(pane()).findByText(error.message);
    expect(field().value).toBe('300000');
    expect(note()!.classList.contains('agent-allowance__note--error')).toBe(true);
    expect(field().getAttribute('aria-invalid')).toBeNull();
    expect(field().getAttribute('aria-describedby')).toBe(note()!.id);
    expect(line()).toBeNull();
    type('300001');
    expect(within(pane()).queryByText(error.message)).toBeNull();
    expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: 300_000 }]);
  });

  it('does not clear newer typing when an earlier save succeeds', async () => {
    const reply = deferred<BridgeResult>();
    allowanceHost({ budget: 250_000, session: session(6_532, 250_000) }, { dispatch: () => reply.promise });
    fireEvent.click(change());
    type('300000');
    enter();
    type('350000');
    await act(async () => reply.resolve(OK));
    expect(field().value).toBe('350000');
    expect(save().textContent).toBe('Save');
  });

  it('cancels with Escape or Cancel, and sends nothing for an unchanged value', () => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    type('9');
    fireEvent.keyDown(field(), { key: 'Escape' });
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(document.activeElement).toBe(change());
    fireEvent.click(change());
    expect(field().value).toBe('250000');
    fireEvent.click(within(pane()).getByRole('button', { name: 'Cancel' }));
    expect(document.activeElement).toBe(change());
    fireEvent.click(change());
    type('250,000');
    enter();
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(host.commands).toEqual([]);
  });

  it('says what a value at or under the recorded usage, or 0, would do', () => {
    allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    expect(note()!.textContent).toBe('6,532 used');
    type('6000');
    expect(note()!.textContent).toBe('No new requests: 6,532 already used.');
    type('0');
    expect(note()!.textContent).toBe('0 turns the agent off.');
  });

  it('works without an account, next to the sign-in step', async () => {
    const host = allowanceHost({
      budget: 250_000,
      provider: SIGNED_OUT,
      agent: { status: 'unavailable', reason: 'Connect an OpenAI account to use the agent.' },
      session: session(6_532, 250_000),
    });
    expect(header()).toBe('Not signed in');
    expect(within(pane()).getByRole('button', { name: 'Continue with ChatGPT' })).toBeTruthy();
    fireEvent.click(change());
    type('100000');
    enter();
    await waitFor(() => expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: 100_000 }]));
  });

  it('tells the user when another edit changed the allowance mid-edit, and keeps their value', async () => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    host.update({ projectAgentBudget: { maxTokens: 400_000 } });
    expect(note()!.textContent).toBe('Changed to 400,000 by another edit.');
    expect(field().value).toBe('250000');
    enter();
    await waitFor(() => expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: 250_000 }]));
  });

  it('keeps an edit and a save in flight when the pane is hidden and shown again', async () => {
    const reply = deferred<BridgeResult>();
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) }, { dispatch: () => reply.promise });
    fireEvent.click(change());
    type('320000');
    const toggle = screen.getByRole('button', { name: /inspector/i });
    fireEvent.click(toggle);
    expect(document.querySelector('[data-region="agent"]')).toBeNull();
    fireEvent.click(toggle);
    expect(field().value).toBe('320000');
    enter();
    fireEvent.click(toggle);
    await act(async () => reply.resolve(OK));
    host.update({ projectAgentBudget: { maxTokens: 320_000 } });
    fireEvent.click(toggle);
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(line()!.textContent).toBe('6,532 of 320,000 tokens used');
    expect(host.commands).toHaveLength(1);
  });

  it('never saves one project’s edit into another project', () => {
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) });
    fireEvent.click(change());
    type('999');
    host.update({
      connection: { status: 'ready', project: { id: '01JA45FX0000000000000OTHER' as Ulid, name: 'Other' } },
      projectAgentBudget: { maxTokens: 1_000_000 },
      agentSession: session(0, 1_000_000, { version: 2 }),
    });
    expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull();
    expect(line()!.textContent).toBe('0 of 1,000,000 tokens used');
    fireEvent.click(change());
    expect(field().value).toBe('1000000');
    expect(host.commands).toEqual([]);
  });
});

describe('While the agent applies a change', () => {
  it('warns that saving stops a running request, keeps Stop, and holds decisions until the session catches up', async () => {
    const reply = deferred<BridgeResult>();
    const host = allowanceHost({ budget: 200_000, agent: RUNNING, session: session(1_000, 200_000, { worker_running: true, turns: [awaiting()] }) }, { dispatch: () => reply.promise });
    const allow = within(pane()).getByRole('button', { name: 'Allow' });
    expect(allow.getAttribute('aria-disabled')).toBeNull();
    fireEvent.click(change());
    expect(note()!.textContent).toBe('Saving stops the current request.');
    expect(note()!.classList.contains('agent-allowance__note--warning')).toBe(true);
    type('100000');
    enter();
    const held = 'Waiting for the agent to apply the new allowance. You can still stop the agent.';
    expect(within(pane()).getByText(held)).toBeTruthy();
    expect(within(pane()).getByRole('button', { name: 'Allow' }).getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(within(pane()).getByRole('button', { name: 'Allow' }));
    expect(host.requests).toEqual([]);
    await act(async () => reply.resolve(OK));
    // Saved, but the session still reports the old cap: decisions stay held.
    host.update({ projectAgentBudget: { maxTokens: 100_000 } });
    expect(within(pane()).getByText(held)).toBeTruthy();
    fireEvent.click(within(pane()).getByRole('button', { name: 'Stop' }));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.cancel', turnId: 3 }]));
    host.update({ agentSession: session(1_000, 100_000, { version: 2, worker_running: true, turns: [awaiting()] }) });
    expect(within(pane()).queryByText(held)).toBeNull();
  });

  it('keeps the composer through a session refresh instead of flashing a note, then offers Refresh if it never finishes', async () => {
    vi.useFakeTimers();
    const refresh = vi.fn(() => Promise.resolve());
    const host = allowanceHost({ budget: 300_000, session: session(6_532, 300_000) }, { refreshAgent: refresh });
    const message = screen.getByLabelText('Message to the agent') as HTMLTextAreaElement;
    fireEvent.change(message, { target: { value: 'Add fog' } });
    host.update({ agent: { status: 'unavailable', reason: REFRESH_REASON }, projectAgentBudget: { maxTokens: 400_000 } });
    expect(header()).toBe('Updating…');
    expect(screen.getByLabelText('Message to the agent')).toBe(message);
    expect(message.value).toBe('Add fog');
    const wait = within(pane()).getByRole('button', { name: 'Updating…' });
    expect(wait.getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(message, { key: 'Enter', metaKey: true });
    expect(host.requests).toEqual([]);
    expect(line()!.textContent).toBe('6,532 of 400,000 tokens used');
    await act(async () => {
      vi.advanceTimersByTime(REFRESH_GRACE_MS);
    });
    // Stalled: the host's reason and Refresh, above the same composer and draft.
    expect(within(pane()).getByRole('note').textContent).toContain(REFRESH_REASON);
    expect(header()).toBe('Unavailable');
    expect(screen.getByLabelText('Message to the agent')).toBe(message);
    fireEvent.keyDown(message, { key: 'Enter', metaKey: true });
    expect(host.requests).toEqual([]);
    fireEvent.click(within(pane()).getByRole('button', { name: 'Refresh' }));
    expect(refresh).toHaveBeenCalledTimes(1);
    host.update({ agent: { status: 'idle' }, agentSession: session(6_532, 400_000, { version: 2 }) });
    expect(header()).toBe('Ready');
    expect((screen.getByLabelText('Message to the agent') as HTMLTextAreaElement).value).toBe('Add fog');
    // A later hold starts as Updating…, never as the earlier stall, not even for one commit.
    const observer = new MutationObserver(() => undefined);
    observer.observe(pane(), { childList: true, subtree: true });
    host.update({ agent: { status: 'unavailable', reason: REFRESH_REASON } });
    const flashed = observer
      .takeRecords()
      .some((record) => [...record.addedNodes].some((node) => node instanceof HTMLElement && (node.matches('.agent-notice') || node.querySelector('.agent-notice') !== null)));
    observer.disconnect();
    expect(flashed).toBe(false);
    expect(header()).toBe('Updating…');
    expect(within(pane()).queryByRole('note')).toBeNull();
  });

  it('says what its note would say after a Retry that does not bring the agent back', async () => {
    let host!: ReturnType<typeof allowanceHost>;
    const refresh = vi.fn(async () => {
      host.update({ agent: { status: 'unavailable', reason: EXHAUSTED_REASON }, agentSession: session(250_000, 250_000, { version: 2 }) }, ['agentError']);
    });
    host = allowanceHost({ budget: 250_000, agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message }, session: session(6_532, 250_000) }, { refreshAgent: refresh });
    fireEvent.click(within(pane()).getByRole('button', { name: 'Retry' }));
    await waitFor(() => expect(statusLine()).toBe('Still unavailable. Raise the allowance to send new requests.'));
    expect(within(pane()).getByText('Raise the allowance to send new requests.')).toBeTruthy();
  });

  it('treats a save slower than the grace period as in progress, not stalled', async () => {
    vi.useFakeTimers();
    const reply = deferred<BridgeResult>();
    const host = allowanceHost({ budget: 250_000, session: session(6_532, 250_000) }, { dispatch: () => reply.promise, refreshAgent: () => Promise.resolve() });
    fireEvent.click(change());
    type('300000');
    enter();
    // The native bridge holds new requests while its command runs.
    host.update({ agent: { status: 'unavailable', reason: REFRESH_REASON } });
    await act(async () => {
      vi.advanceTimersByTime(REFRESH_GRACE_MS * 2);
    });
    expect(header()).toBe('Updating…');
    expect(within(pane()).queryByRole('button', { name: 'Refresh' })).toBeNull();
    await act(async () => reply.resolve(OK));
    host.update({ projectAgentBudget: { maxTokens: 300_000 } });
    expect(header()).toBe('Updating…');
    await act(async () => {
      vi.advanceTimersByTime(REFRESH_GRACE_MS);
    });
    expect(within(pane()).getByRole('button', { name: 'Refresh' })).toBeTruthy();
    expect(host.commands).toEqual([{ type: 'agent.budget.set', maxTokens: 300_000 }]);
  });

  it('keeps a successful save when the session refresh after it fails', async () => {
    const refresh = vi.fn(() => Promise.resolve());
    const host = allowanceHost({ budget: 100_000, agent: { status: 'unavailable', reason: EXHAUSTED_REASON }, session: session(100_000, 100_000) }, { refreshAgent: refresh });
    fireEvent.click(change());
    type('200000');
    enter();
    await waitFor(() => expect(host.commands).toHaveLength(1));
    host.update({ projectAgentBudget: { maxTokens: 200_000 }, agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message } });
    await waitFor(() => expect(within(pane()).queryByRole('textbox', { name: /Allowance/ })).toBeNull());
    expect(line()!.textContent).toBe('100,000 of 200,000 tokens used');
    expect(within(pane()).getByText(TRANSPORT.message)).toBeTruthy();
    expect(within(pane()).getByRole('button', { name: 'Retry' })).toBeTruthy();
    expect(pane().textContent).not.toMatch(/not saved|could not save|failed/i);
    await waitFor(() => expect(statusLine()).toBe('Token allowance set to 200,000.'));
  });
});

/* ---- The real native bridge over mocked IPC ---- */

function nativeProject(maxTokens: number, revision: number, applied: number): EngineRead {
  return {
    project: { id: '01JA45FX0000000000000PROJ1', name: 'Harbor', scenes: {}, settings: { tick_rate: 60, agent_budget: { max_tokens: maxTokens } } },
    revision,
    can_redo: applied === 0 && revision > 7,
    history: revision > 7 ? [{ id: '01JA45FX0000000000000EDIT1', description: 'Set project agent token cap', actor: { origin: 'user', actor: 'editor', model: null, conversation_id: null } }] : [],
    applied,
    schemas: { Project: projectSchema },
    console: [],
    viewport_error: null,
    agent_budget: { available: true },
  } as unknown as EngineRead;
}

describe('Allowance on the real native bridge (mocked IPC)', () => {
  it('saves through the shared user command, then Undo restores the cap without touching usage', async () => {
    const calls: { name: string; args: unknown }[] = [];
    let engine = nativeProject(200_000, 7, 0);
    let current = session(150_000, 200_000);
    const invoke: Invoke = async <T,>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      switch (name) {
        case 'engine_read':
          return engine as T;
        case 'engine_execute':
          engine = nativeProject(1_000_000, 8, 1);
          current = session(150_000, 1_000_000, { version: 2 });
          return engine as T;
        case 'engine_history':
          engine = nativeProject(200_000, 9, 0);
          current = session(150_000, 200_000, { version: 3 });
          return engine as T;
        case 'engine_agent_read':
          return { session: args?.afterVersion === current.version ? null : current, project_revision: engine.revision } as T;
        case 'viewport_bounds':
          return null as T;
        default:
          throw new Error(`unexpected IPC ${name}`);
      }
    };
    const bridge = new NativeBridge(invoke);
    await bridge.start();
    bridge.updateProvider(SIGNED_IN);
    await bridge.refreshAgent();
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    expect(header()).toBe('Ready');
    expect(line()!.textContent).toBe('150,000 of 200,000 tokens used');
    calls.length = 0;
    fireEvent.click(change());
    type('1000000');
    enter();
    await waitFor(() => expect(line()?.textContent).toBe('150,000 of 1,000,000 tokens used'));
    expect(calls.filter((call) => call.name !== 'viewport_bounds')).toEqual([
      { name: 'engine_execute', args: { commands: [{ op: 'set_agent_budget', settings: { max_tokens: 1_000_000 } }], description: 'Set project agent token cap', expectedRevision: 7 } },
      { name: 'engine_agent_read', args: { afterVersion: 1 } },
    ]);
    await waitFor(() => expect(header()).toBe('Ready'));
    expect(document.activeElement).toBe(change());
    fireEvent.click(screen.getByRole('button', { name: /^Undo/ }));
    await waitFor(() => expect(line()?.textContent).toBe('150,000 of 200,000 tokens used'));
    expect(calls.some((call) => call.name === 'engine_history')).toBe(true);
    await waitFor(() => expect(header()).toBe('Ready'));
  });
});
