import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { App } from '../../App';
import type {
  AgentFailure,
  AgentPendingApproval,
  AgentReportSummary,
  AgentSessionSnapshot,
  AgentTurnSnapshot,
  AgentTurnStatus,
  BridgeError,
  BridgeResult,
  BridgeSnapshot,
  EditorBridge,
  HostRequest,
  ProviderState,
} from '../../bridge/contract';
import { fixtureSnapshot } from '../../bridge/fixture';
import { NativeBridge } from '../../../../bridge/native';
import type { EngineRead, Invoke } from '../../../../bridge/native';

/*
 * Agent pane behaviour against DETERMINISTIC TEST DOUBLES. Sessions are written in the
 * native snake_case form; nothing here is a live provider, a native host or an engine.
 */

const SIGNED_IN: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'ada@example.com', accounts: [{ id: 'a', label: 'ada@example.com' }], activeAccount: 'a' };
const SIGNED_OUT: ProviderState = { status: 'not-connected', provider: 'openai' };
const RUNNING = { status: 'running', conversation: 'test' } as const;
const OK: BridgeResult = { ok: true };
const TRANSPORT: BridgeError = { code: 'agent.transport', message: 'Could not contact the native agent. Refresh the session to retry.' };

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
const report = (edits: number): AgentReportSummary => ({
  steps: 2,
  usage: { input_tokens: 100, output_tokens: 20 },
  transaction_count: edits,
  tool_call_count: edits + 1,
  transaction_ids: [],
  tool_calls: [],
  truncated: false,
});
function session(turns: AgentTurnSnapshot[], fields: Partial<AgentSessionSnapshot> = {}): AgentSessionSnapshot {
  return {
    version: 1,
    budget: { max_tokens: 200000, used_tokens: turns.reduce((sum, item) => sum + item.charged_tokens, 0) },
    accounting_uncertain: false,
    worker_running: false,
    turns,
    ...fields,
  };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/**
 * TEST DOUBLE, not a bridge implementation: the sample project with an agent session the
 * test controls. It records every host request and answers as the test says; it never
 * changes its own snapshot, so the pane cannot be credited with state it invented.
 */
function agentHost(
  initial: { agent?: BridgeSnapshot['agent']; provider?: ProviderState; session?: AgentSessionSnapshot; agentError?: BridgeError },
  options: { capabilities?: string[]; respond?: (request: HostRequest) => Promise<BridgeResult>; refreshAgent?: () => Promise<void> } = {},
) {
  let snapshot: BridgeSnapshot = {
    ...fixtureSnapshot('sample'),
    provider: initial.provider ?? SIGNED_IN,
    agent: initial.agent ?? { status: 'idle' },
    ...(initial.session ? { agentSession: initial.session } : {}),
    ...(initial.agentError ? { agentError: initial.agentError } : {}),
  };
  const listeners = new Set<() => void>();
  const requests: HostRequest[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities: options.capabilities ?? ['agent.send', 'agent.cancel', 'agent.approve', 'provider.connect'],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dispatch: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Test double.' } }),
    request: (request) => {
      requests.push(request);
      return options.respond?.(request) ?? Promise.resolve(OK);
    },
    ...(options.refreshAgent ? { refreshAgent: options.refreshAgent } : {}),
  };
  /** Replaces fields, and removes the optional ones named in `remove` (as a host that recovered). */
  const update = (patch: Partial<BridgeSnapshot>, remove: readonly ('agentError' | 'agentSession')[] = []) =>
    act(() => {
      const next: Record<string, unknown> = { ...snapshot, ...patch };
      for (const key of remove) delete next[key];
      snapshot = next as unknown as BridgeSnapshot;
      for (const listener of listeners) listener();
    });
  render(<App resolution={{ kind: 'bridge', bridge }} />);
  return { bridge, requests, update };
}

const pane = () => document.querySelector<HTMLElement>('[data-region="agent"]')!;
const input = () => screen.getByLabelText('Message to the agent') as HTMLTextAreaElement;
const action = (name: string | RegExp) => within(pane()).getByRole('button', { name });
const header = () => pane().querySelector('.agent__status')!.textContent!.replace(/^,\s*/, '');
const statusLine = () => document.querySelector('.statusbar__message')!.textContent;
const nodeId = (name: string) => {
  const hierarchy = fixtureSnapshot('sample').hierarchy;
  if (hierarchy.status !== 'ready') throw new Error('sample hierarchy');
  return Object.values(hierarchy.value.nodes).find((node) => node.name === name)!.id;
};

function awaiting(fields: { id?: number; approval?: number; ops?: unknown[]; tool?: string; args?: AgentPendingApproval['arguments'] } = {}): AgentTurnSnapshot {
  const id = fields.id ?? 3;
  const approval = fields.approval ?? 7;
  const tool = fields.tool ?? 'doc_patch';
  const args = fields.args ?? ({ description: 'Lift the crate', expected_revision: 4, ops: fields.ops ?? [] } as AgentPendingApproval['arguments']);
  return turn(id, {
    status: 'awaiting_approval',
    text: 'I will lift the crate and add a lantern.',
    approvals: [{ id: approval, tool, bytes: 321, decision: 'pending' }],
    pending: { turn: id, id: approval, tool, arguments: args },
  });
}
const crateOps = () => [
  { op: 'set_component', scene_id: 'S', entity_id: nodeId('Crate 03'), component: 'Transform', value: { translation: [0, 1, 0] } },
  { op: 'delete_entity', scene_id: 'S', entity_id: 'MISSING' },
  { op: 'create_entity', scene_id: 'S', entity: { id: 'N', name: 'Lantern', components: { PointLight: {} } } },
];

describe('Agent conversation from the native session', () => {
  it('renders each prompt and streamed reply as text, never as markup, without a live region for the stream', () => {
    agentHost({
      session: session([
        turn(1, {
          request: { prompt: '<b>Make</b> it <script>bad()</script>', model: 'gpt-6-astra', approval: 'destructive' },
          text: 'Done. <img src=x onerror="alert(1)"> **not markdown**',
          report: report(0),
          charged_tokens: 12,
        }),
      ]),
    });
    expect(pane().querySelector('img, b, script')).toBeNull();
    expect(pane().textContent).toContain('<img src=x onerror="alert(1)"> **not markdown**');
    expect(pane().textContent).toContain('<b>Make</b> it <script>bad()</script>');
    const log = within(pane()).getByRole('log', { name: 'Agent conversation' });
    expect(log.getAttribute('aria-live')).toBe('off');
    expect(header()).toBe('Ready');
  });

  it('states how each turn ended truthfully, including unknown codes and uncertain usage', () => {
    agentHost({
      session: session(
        [
          turn(1, { text: 'Moved both.', report: report(2), charged_tokens: 1200 }),
          turn(2, { status: 'cancelled', text: 'Renaming', charged_tokens: 300, accounting_uncertain: true }),
          turn(3, { status: 'failed', failure: 'provider', accounting_uncertain: true }),
          turn(4, { status: 'failed', failure: 'not_a_known_code' as AgentFailure, charged_tokens: 5 }),
          turn(5, { status: 'paused_by_host' as AgentTurnStatus, charged_tokens: 7 }),
        ],
        { accounting_uncertain: true },
      ),
    });
    const turns = pane().querySelectorAll('.agent-turn');
    const outcome = (index: number) => turns[index]!.querySelector('.agent-outcome')!.textContent;
    expect(outcome(0)).toBe('2 edits·1,200 tokensShow in History');
    expect(outcome(1)).toBe('Stopped·300 tokens (estimated)');
    expect(turns[2]!.querySelector('.agent-failure')!.textContent).toBe('The model provider did not complete the request.');
    expect(outcome(2)).toBe('Token use unknown');
    expect(turns[3]!.querySelector('.agent-failure')!.textContent).toBe('The request failed.');
    expect(outcome(4)).toBe('Status unknown·7 tokens');
    expect(pane().querySelector('.agent-usage')!.textContent).toBe('1,512 of 200,000 tokens used (some estimated)');
    fireEvent.click(within(pane()).getByRole('button', { name: 'Show in History' }));
    expect(screen.getByRole('tab', { name: /^History/ }).getAttribute('aria-selected')).toBe('true');
  });

  it('states the session’s bound once it holds that many requests', () => {
    agentHost({ session: session(Array.from({ length: 16 }, (_, n) => turn(n + 5, { report: report(0) }))) });
    expect(within(pane()).getByText('Only the latest 16 requests are kept.')).toBeTruthy();
  });
});

describe('Sending', () => {
  it('sends the trimmed text once, invents no turn, and keeps what was typed while the request was in flight', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ session: session([]) }, { respond: () => reply.promise });
    fireEvent.change(input(), { target: { value: '  Hang a lantern  ' } });
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    fireEvent.keyDown(input(), { key: 'Enter', ctrlKey: true });
    expect(host.requests).toEqual([{ type: 'agent.send', text: 'Hang a lantern', model: 'gpt-6-astra', approval: 'destructive' }]);
    expect(action('Sending…').getAttribute('aria-disabled')).toBe('true');
    expect(header()).toBe('Starting…');
    // The field stays editable: the next message can be drafted meanwhile.
    fireEvent.change(input(), { target: { value: '  Hang a lantern  and a rope' } });
    await act(async () => reply.resolve({ ok: true, agentTurnId: 1 }));
    expect(input().value).toBe('  Hang a lantern  and a rope');
    expect(pane().querySelectorAll('.agent-turn')).toHaveLength(0);
    expect(host.requests).toHaveLength(1);
  });

  it('clears the draft after a successful send when nothing new was typed', async () => {
    const host = agentHost({ session: session([]) });
    fireEvent.change(input(), { target: { value: 'Add fog' } });
    fireEvent.click(action('Send'));
    await waitFor(() => expect(input().value).toBe(''));
    expect(host.requests).toEqual([{ type: 'agent.send', text: 'Add fog', model: 'gpt-6-astra', approval: 'destructive' }]);
  });

  it('keeps the draft and says why when a send fails, until the text changes', async () => {
    const message = 'Enter a nonempty prompt up to 16 KiB and a model name up to 128 bytes.';
    agentHost({ session: session([]) }, { respond: () => Promise.resolve({ ok: false, error: { code: 'agent.invalid-request', message } }) });
    fireEvent.change(input(), { target: { value: 'Add fog' } });
    fireEvent.click(action('Send'));
    const error = await within(pane()).findByText(message);
    expect(input().value).toBe('Add fog');
    expect(input().getAttribute('aria-describedby')).toContain(error.closest('p')!.id);
    fireEvent.change(input(), { target: { value: 'Add light fog' } });
    expect(within(pane()).queryByText(message)).toBeNull();
  });

  it('sends the chosen model and approval mode, and returns a cleared model to the default', async () => {
    const host = agentHost({ session: session([]) });
    const mode = within(pane()).getByRole('combobox', { name: 'Approval' }) as HTMLSelectElement;
    expect([...mode.options].map((option) => [option.value, option.text])).toEqual([
      ['destructive', 'Ask before edits'],
      ['always', 'Ask every time'],
      ['auto', 'Never ask'],
    ]);
    fireEvent.change(mode, { target: { value: 'auto' } });
    expect(pane().querySelector('.composer__mode.is-unguarded')).not.toBeNull();
    fireEvent.change(mode, { target: { value: 'always' } });
    const model = within(pane()).getByRole('textbox', { name: 'Model' }) as HTMLInputElement;
    expect(model.value).toBe('gpt-6-astra');
    fireEvent.change(model, { target: { value: ' gpt-6-astra-mini ' } });
    fireEvent.change(input(), { target: { value: 'Check the lights' } });
    fireEvent.click(action('Send'));
    await waitFor(() => expect(host.requests).toHaveLength(1));
    expect(host.requests[0]).toEqual({ type: 'agent.send', text: 'Check the lights', model: 'gpt-6-astra-mini', approval: 'always' });
    fireEvent.change(model, { target: { value: '   ' } });
    fireEvent.blur(model);
    expect(model.value).toBe('gpt-6-astra');
  });

  it('checks the model name against the host’s byte limit before sending', () => {
    const host = agentHost({ session: session([]) });
    fireEvent.change(within(pane()).getByRole('textbox', { name: 'Model' }), { target: { value: '模型'.repeat(22) } });
    fireEvent.change(input(), { target: { value: 'Add fog' } });
    expect(within(pane()).getByText('The model name is 132 bytes; the limit is 128.')).toBeTruthy();
    expect(action('Send').getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    expect(host.requests).toEqual([]);
  });

  it('lands a send that resolves after the pane was hidden and shown again', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ session: session([turn(3, { report: report(0) })]) }, { respond: () => reply.promise });
    fireEvent.change(input(), { target: { value: 'Add fog' } });
    fireEvent.click(action('Send'));
    host.update({ agent: RUNNING });
    // Hiding the Inspector column unmounts the pane; the request is still in flight.
    const toggle = screen.getByRole('button', { name: /inspector/i });
    fireEvent.click(toggle);
    expect(document.querySelector('[data-region="agent"]')).toBeNull();
    fireEvent.click(toggle);
    expect(action('Sending…')).toBeTruthy();
    expect(input().value).toBe('Add fog');
    await act(async () => reply.resolve({ ok: true, agentTurnId: 4 }));
    expect(input().value).toBe('');
    fireEvent.click(action('Stop'));
    await waitFor(() => expect(host.requests.at(-1)).toEqual({ type: 'agent.cancel', turnId: 4 }));
  });

  it('says plainly when the message is over the host’s limit and does not send it', () => {
    const host = agentHost({ session: session([]) });
    fireEvent.change(input(), { target: { value: 'é'.repeat(9000) } });
    expect(within(pane()).getByText('This message is 17.6 KB; the limit is 16 KB.')).toBeTruthy();
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    expect(host.requests).toEqual([]);
  });

  it('is honest about an older host that reports no session', async () => {
    const host = agentHost({});
    expect(within(pane()).getByText(/Describe a change/)).toBeTruthy();
    fireEvent.change(input(), { target: { value: 'Add fog' } });
    fireEvent.click(action('Send'));
    await waitFor(() => expect(statusLine()).toBe('Sent. This editor host does not show the agent’s replies; its edits appear in History.'));
    expect(host.requests).toHaveLength(1);
    expect(pane().querySelectorAll('.agent-turn')).toHaveLength(0);
  });
});

describe('Approvals', () => {
  it('summarises the exact operations, shows the arguments on request and never decides or focuses by itself', () => {
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) });
    expect(header()).toBe('Needs approval');
    const card = within(pane()).getByRole('region', { name: 'Apply 3 changes to the project?' });
    expect(card.textContent).toContain('“Lift the crate”');
    expect([...card.querySelectorAll('.agent-approval__row')].map((row) => row.textContent)).toEqual([
      'Set Transform on Crate 03',
      'Delete MISSING',
      'Create Lantern with PointLight',
    ]);
    const disclosure = within(card).getByRole('button', { name: /Exact arguments/ });
    expect(disclosure.getAttribute('aria-expanded')).toBe('false');
    expect(card.querySelector('pre')).toBeNull();
    fireEvent.click(disclosure);
    expect(within(card).getByLabelText('Exact arguments').textContent).toContain('"op": "set_component"');
    expect(document.activeElement).not.toBe(within(card).getByRole('button', { name: 'Allow' }));
    expect(host.requests).toEqual([]);
  });

  it('sends Allow once with the explicit turn and approval IDs, and focuses the decision when the host records it', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) }, { respond: () => reply.promise });
    const allow = action('Allow');
    allow.focus();
    fireEvent.click(allow);
    fireEvent.click(allow);
    expect(host.requests).toEqual([{ type: 'agent.approve', turnId: 3, approvalId: 7, approved: true }]);
    expect(allow.textContent).toBe('Allowing…');
    expect(action('Deny').getAttribute('aria-disabled')).toBe('true');
    await act(async () => reply.resolve(OK));
    host.update({
      agentSession: session([turn(3, { status: 'running', text: 'Lifting.', approvals: [{ id: 7, tool: 'doc_patch', bytes: 321, decision: 'allowed' }] })], { worker_running: true, version: 2 }),
    });
    const row = pane().querySelector('.agent-decision--allowed')!;
    expect(row.textContent).toBe('AllowedApply 3 changes to the project');
    expect(document.activeElement).toBe(row);
    expect(within(pane()).queryByRole('button', { name: 'Allow' })).toBeNull();
  });

  it('sends Deny as approved false, and says the denial ended the turn', async () => {
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) });
    action('Deny').focus();
    fireEvent.click(action('Deny'));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.approve', turnId: 3, approvalId: 7, approved: false }]));
    host.update({
      agent: { status: 'idle' },
      agentSession: session([turn(3, { status: 'failed', failure: 'tool', approvals: [{ id: 7, tool: 'doc_patch', bytes: 321, decision: 'denied' }], charged_tokens: 40 })], { version: 2 }),
    });
    expect(document.activeElement).toBe(pane().querySelector('.agent-decision--denied'));
    expect(pane().querySelector('.agent-failure')!.textContent).toBe('Ended after you denied a request.');
  });

  it('keeps an approval that is still pending when the decision fails, and says why', async () => {
    const message = 'This agent approval is no longer pending.';
    agentHost(
      { agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) },
      { respond: () => Promise.resolve({ ok: false, error: { code: 'agent.stale-approval', message } }) },
    );
    fireEvent.click(action('Allow'));
    await within(pane()).findByText(message);
    expect(action('Allow').getAttribute('aria-disabled')).toBeNull();
  });

  it('shows a following approval in the same turn as a new card that has not been focused or decided', async () => {
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) });
    const first = action('Allow');
    first.focus();
    fireEvent.click(first);
    await waitFor(() => expect(host.requests).toHaveLength(1));
    // The host records the first decision and asks the next question in one update.
    const next = { turn: 3, id: 8, tool: 'doc_patch', arguments: { description: 'Then remove it', expected_revision: 5, ops: [crateOps()[1]!] } } as AgentPendingApproval;
    host.update({
      agentSession: session(
        [turn(3, { status: 'awaiting_approval', text: 'Next.', approvals: [{ id: 7, tool: 'doc_patch', bytes: 321, decision: 'allowed' }, { id: 8, tool: 'doc_patch', bytes: 90, decision: 'pending' }], pending: next })],
        { worker_running: true, version: 2 },
      ),
    });
    const second = action('Allow');
    expect(second).not.toBe(first);
    expect(second.getAttribute('aria-disabled')).toBeNull();
    expect(document.activeElement).not.toBe(second);
    expect(document.activeElement).toBe(pane().querySelector('.agent-decision--allowed'));
    expect(within(pane()).getByRole('region', { name: 'Apply 1 change to the project?' })).toBeTruthy();
    expect(host.requests).toHaveLength(1);
  });

  it('ignores a decision reply that arrives after the host already cancelled that approval', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) }, { respond: () => reply.promise });
    action('Allow').focus();
    fireEvent.click(action('Allow'));
    // The host stopped the turn meanwhile: the approval is cancelled, not allowed.
    host.update({
      agent: { status: 'idle' },
      agentSession: session([turn(3, { status: 'cancelled', text: 'I will lift the crate.', approvals: [{ id: 7, tool: 'doc_patch', bytes: 321, decision: 'cancelled' }] })], { version: 2 }),
    });
    await act(async () => reply.resolve({ ok: false, error: { code: 'agent.stale-approval', message: 'This agent approval is no longer pending.' } }));
    expect(within(pane()).queryByRole('button', { name: /Allow/ })).toBeNull();
    expect(within(pane()).queryByText('This agent approval is no longer pending.')).toBeNull();
    const row = pane().querySelector('.agent-decision--cancelled')!;
    expect(row.textContent).toBe('CancelledApply 3 changes to the project');
    expect(document.activeElement).toBe(row);
    expect(pane().querySelector('.agent-outcome')!.textContent).toBe('Stopped·0 tokens');
    expect(host.requests).toHaveLength(1);
  });

  it('blocks decisions while signed out but keeps Stop usable for the running worker', async () => {
    const host = agentHost({ agent: RUNNING, provider: SIGNED_OUT, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) });
    expect(within(pane()).getByText('Sign in again to decide. You can still stop the agent.')).toBeTruthy();
    expect(action('Allow').getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(action('Allow'));
    expect(host.requests).toEqual([]);
    fireEvent.click(action('Stop'));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.cancel', turnId: 3 }]));
  });

  it('keeps Stop usable when the project fails while an existing worker still runs', async () => {
    const host = agentHost({ agent: RUNNING, session: session([turn(6, { status: 'running', text: 'Partial' })], { worker_running: true }) });
    host.update({ connection: { status: 'error', error: { code: 'project.unreadable', message: 'The project file could not be read.' } } });
    expect(header()).toBe('Working…');
    expect(pane().textContent).toContain('Partial');
    fireEvent.click(action('Stop'));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.cancel', turnId: 6 }]));
  });

  it('explains approvals the host does not support instead of sending them', () => {
    const host = agentHost({ agent: RUNNING, session: session([awaiting({ ops: crateOps() })], { worker_running: true }) }, { capabilities: ['agent.send', 'agent.cancel'] });
    expect(within(pane()).getByText('Decide agent approval is unavailable in this editor build (agent.approve).')).toBeTruthy();
    fireEvent.click(action('Allow'));
    expect(host.requests).toEqual([]);
  });

  it('bounds very long arguments until all of them are asked for', () => {
    agentHost({
      agent: RUNNING,
      session: session([awaiting({ tool: 'script_write', args: { description: 'Swap', expected_revision: 1, script_id: 'S', expected_source: { state: 'missing' }, source: 'z'.repeat(100_000) } })], { worker_running: true }),
    });
    fireEvent.click(action(/Exact arguments/));
    const shown = () => within(pane()).getByLabelText('Exact arguments').textContent!.length;
    expect(shown()).toBe(64 * 1024);
    fireEvent.click(action(/Show all/));
    expect(shown()).toBeGreaterThan(100_000);
  });
});

describe('Stopping and recovery', () => {
  it('stops the running turn by ID, keeping focus on the one action through Stopping… and back to Send', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ agent: RUNNING, session: session([turn(4, { status: 'running', text: 'Checking' })], { worker_running: true }) }, { respond: () => reply.promise });
    expect(header()).toBe('Working…');
    const button = action('Stop');
    button.focus();
    fireEvent.click(button);
    fireEvent.click(button);
    expect(host.requests).toEqual([{ type: 'agent.cancel', turnId: 4 }]);
    expect(button.getAttribute('aria-label')).toBe('Stopping…');
    await act(async () => reply.resolve(OK));
    host.update({ agentSession: session([turn(4, { status: 'stopping', text: 'Checking' })], { worker_running: true, version: 2 }) });
    expect(header()).toBe('Stopping…');
    // Cancelled, but the native worker has not exited: still no new turn.
    host.update({ agentSession: session([turn(4, { status: 'cancelled', text: 'Checking', charged_tokens: 30, accounting_uncertain: true })], { worker_running: true, version: 3 }) });
    expect(button.getAttribute('aria-label')).toBe('Stopping…');
    host.update({ agent: { status: 'idle' }, agentSession: session([turn(4, { status: 'cancelled', text: 'Checking', charged_tokens: 30, accounting_uncertain: true })], { version: 4 }) });
    expect(button.getAttribute('aria-label')).toBe('Send');
    expect(document.activeElement).toBe(button);
    expect(host.requests).toHaveLength(1);
  });

  it('tells a finishing worker apart from a stoppable turn and sends nothing meanwhile', () => {
    const host = agentHost({ agent: RUNNING, session: session([turn(1, { text: 'Done.', report: report(0), charged_tokens: 10 })], { worker_running: true }) });
    expect(header()).toBe('Finishing…');
    expect(action('Finishing…').getAttribute('aria-disabled')).toBe('true');
    expect(within(pane()).queryByRole('button', { name: 'Stop' })).toBeNull();
    fireEvent.change(input(), { target: { value: 'Next' } });
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    expect(host.requests).toEqual([]);
  });

  it('stops a turn by the ID its send returned before the session lists it', async () => {
    const reply = deferred<BridgeResult>();
    const host = agentHost({ session: session([turn(3, { report: report(0) })]) }, { respond: (request) => (request.type === 'agent.send' ? reply.promise : Promise.resolve(OK)) });
    fireEvent.change(input(), { target: { value: 'Go' } });
    fireEvent.click(action('Send'));
    // As the native bridge does: busy while starting and until a read lists the turn.
    host.update({ agent: RUNNING });
    await act(async () => reply.resolve({ ok: true, agentTurnId: 9 }));
    fireEvent.click(action('Stop'));
    await waitFor(() => expect(host.requests.at(-1)).toEqual({ type: 'agent.cancel', turnId: 9 }));
  });

  it('explains a Stop the host does not support', async () => {
    const host = agentHost({ agent: RUNNING, session: session([turn(1, { status: 'running' })], { worker_running: true }) }, { capabilities: ['agent.send'] });
    const stop = action('Stop');
    expect(stop.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(stop);
    expect(host.requests).toEqual([]);
    await waitFor(() => expect(statusLine()).toBe('Stop agent is unavailable in this editor build (agent.cancel).'));
  });

  it('keeps the history readable through sign-out and transport failure, and retries only when asked', async () => {
    const refresh = vi.fn(() => Promise.resolve());
    const host = agentHost({ session: session([turn(1, { text: 'Moved it.', report: report(1), charged_tokens: 50 })]) }, { refreshAgent: refresh });
    host.update({ provider: SIGNED_OUT, agent: { status: 'unavailable', reason: 'Connect an OpenAI account to use the agent.' } });
    expect(pane().textContent).toContain('Moved it.');
    expect(within(pane()).getByText('Connect an OpenAI account to use the agent.')).toBeTruthy();
    expect(within(pane()).getByRole('button', { name: 'Continue with ChatGPT' })).toBeTruthy();
    expect(screen.queryByLabelText('Message to the agent')).toBeNull();
    host.update({ provider: SIGNED_IN, agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message } });
    expect(header()).toBe('Unavailable');
    expect(pane().textContent).toContain('Moved it.');
    expect(refresh).not.toHaveBeenCalled();
    fireEvent.click(action('Retry'));
    await waitFor(() => expect(refresh).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(statusLine()).toBe(`Still unavailable. ${TRANSPORT.message}`));
  });

  it('does not pull focus back into the pane after the user has left it', async () => {
    const host = agentHost({ agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message }, session: session([turn(1, { report: report(0) })]) }, { refreshAgent: () => Promise.resolve() });
    action('Retry').focus();
    const elsewhere = screen.getByRole('tab', { name: /^History/ });
    elsewhere.focus();
    await act(async () => Promise.resolve());
    // The agent recovers by itself and the Retry button goes away.
    host.update({ agent: { status: 'idle' } }, ['agentError']);
    expect(document.activeElement).toBe(elsewhere);
  });

  it('keeps focus in the pane when the focused control goes away', () => {
    const host = agentHost({ agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message }, session: session([turn(1, { report: report(0) })]) }, { refreshAgent: () => Promise.resolve() });
    action('Retry').focus();
    host.update({ agent: { status: 'idle' } }, ['agentError']);
    expect(document.activeElement).toBe(input());
  });

  it('offers no Retry on a host without the recovery hook', () => {
    agentHost({ agentError: TRANSPORT, agent: { status: 'unavailable', reason: TRANSPORT.message }, session: session([]) });
    expect(within(pane()).getByText(TRANSPORT.message)).toBeTruthy();
    expect(within(pane()).queryByRole('button', { name: 'Retry' })).toBeNull();
  });

  it('shows a transport failure above the composer while an existing worker still runs, with Stop usable', () => {
    agentHost({ agent: RUNNING, agentError: TRANSPORT, session: session([turn(2, { status: 'running', text: 'Partial' })], { worker_running: true }) }, { refreshAgent: () => Promise.resolve() });
    const notice = within(pane()).getByRole('note');
    expect(notice.textContent).toContain(TRANSPORT.message);
    expect(within(notice).getByRole('button', { name: 'Retry' })).toBeTruthy();
    expect(action('Stop').getAttribute('aria-disabled')).toBeNull();
  });

  it('says when the session allowance is used up instead of offering a composer', () => {
    agentHost({
      agent: { status: 'unavailable', reason: 'The native agent session token allowance is exhausted.' },
      session: session([turn(1, { report: report(0), charged_tokens: 200000 })]),
    });
    expect(within(pane()).getByText('This session’s token allowance is used up.')).toBeTruthy();
    expect(pane().querySelector('.agent-usage--out')!.textContent).toBe('200,000 of 200,000 tokens used');
  });
});

describe('Announcements', () => {
  it('announces approval requests and outcomes once, and never the streamed text', async () => {
    const host = agentHost({ agent: RUNNING, session: session([turn(1, { status: 'running', text: 'A' })], { worker_running: true }) });
    host.update({ agentSession: session([turn(1, { status: 'running', text: 'A B' })], { worker_running: true, version: 2 }) });
    host.update({ agentSession: session([turn(1, { status: 'running', text: 'A B C' })], { worker_running: true, version: 3 }) });
    await new Promise((done) => setTimeout(done, 60));
    expect(statusLine()).toBe('');
    host.update({ agentSession: session([awaiting({ id: 1, approval: 2, ops: [crateOps()[1]!] })], { worker_running: true, version: 4 }) });
    await waitFor(() => expect(statusLine()).toBe('The agent needs approval: Apply 1 change to the project.'));
    host.update({ agent: { status: 'idle' }, agentSession: session([turn(1, { text: 'Deleted.', approvals: [{ id: 2, tool: 'doc_patch', bytes: 9, decision: 'allowed' }], report: report(1), charged_tokens: 70 })], { version: 5 }) });
    await waitFor(() => expect(statusLine()).toBe('The agent finished: 1 edit.'));
  });
});

/* ---- The real native bridge over mocked IPC ---- */

function nativeProject(): EngineRead {
  return { project: { id: '00000000000000000000000001', name: 'Agent', scenes: {} }, revision: 7, can_redo: false, history: [], applied: 0, schemas: {}, console: [], viewport_error: null } as unknown as EngineRead;
}

describe('Agent pane on the real native bridge (mocked IPC)', () => {
  it('sends, approves and stops through native IPC with the explicit turn and approval IDs', async () => {
    const calls: { name: string; args: unknown }[] = [];
    let current = session([], { version: 0 });
    const pending: AgentPendingApproval = { turn: 1, id: 2, tool: 'doc_patch', arguments: { description: 'Lift', expected_revision: 7, ops: [] } };
    const invoke: Invoke = async <T,>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      switch (name) {
        case 'engine_read':
          return nativeProject() as T;
        case 'engine_agent_read':
          return { session: args?.afterVersion === current.version ? null : current, project_revision: 7 } as T;
        case 'engine_agent_start':
          current = session([turn(1, { request: { prompt: 'Lift the crate', model: 'gpt-6-astra', approval: 'destructive' }, status: 'awaiting_approval', text: 'Review.', approvals: [{ id: 2, tool: 'doc_patch', bytes: 70, decision: 'pending' }], pending })], { version: 1, worker_running: true });
          return 1 as T;
        case 'engine_agent_decide':
          current = session([turn(1, { status: 'running', text: 'Review. Lifting.', approvals: [{ id: 2, tool: 'doc_patch', bytes: 70, decision: 'allowed' }] })], { version: 2, worker_running: true });
          return null as T;
        case 'engine_agent_cancel':
          current = session([turn(1, { status: 'stopping', text: 'Review. Lifting.', approvals: [{ id: 2, tool: 'doc_patch', bytes: 70, decision: 'allowed' }] })], { version: 3, worker_running: true });
          return null as T;
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
    fireEvent.change(input(), { target: { value: 'Lift the crate' } });
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    fireEvent.keyDown(input(), { key: 'Enter', metaKey: true });
    await within(pane()).findByRole('button', { name: 'Allow' });
    const agentCalls = () => calls.filter((call) => call.name.startsWith('engine_agent_') && call.name !== 'engine_agent_read');
    expect(agentCalls()).toEqual([{ name: 'engine_agent_start', args: { request: { prompt: 'Lift the crate', model: 'gpt-6-astra', approval: 'destructive' } } }]);
    expect(input().value).toBe('');
    fireEvent.click(action('Allow'));
    await within(pane()).findByText('Allowed');
    expect(agentCalls()[1]).toEqual({ name: 'engine_agent_decide', args: { turnId: 1, approvalId: 2, approved: true } });
    fireEvent.click(action('Stop'));
    await waitFor(() => expect(header()).toBe('Stopping…'));
    expect(agentCalls()[2]).toEqual({ name: 'engine_agent_cancel', args: { turnId: 1 } });
    expect(agentCalls()).toHaveLength(3);
  });
});
