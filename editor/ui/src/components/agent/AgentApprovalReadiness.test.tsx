import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from '../../App';
import type { AgentSessionSnapshot, AgentState, AgentTurnSnapshot, BridgeError, BridgeSnapshot, EditorBridge, HostRequest, ProviderState } from '../../bridge/contract';
import { fixtureSnapshot } from '../../bridge/fixture';
import { NativeBridge } from '../../../../bridge/native';
import type { EngineAgentRead, EngineRead, Invoke } from '../../../../bridge/native';
import projectSchema from '../../../../../schemas/Project.schema.json';

/*
 * Allow and Deny wait while the host says it cannot admit a decision yet
 * (`running.approvalReady === false`), and keep working on an older host that does not
 * say. Against the real NativeBridge over deferred, mocked IPC, and against DETERMINISTIC
 * TEST DOUBLES. Nothing here is a live provider, a native host or an engine.
 *
 * Enter or Space on a focused button activates it as a click; jsdom does not synthesize
 * that, so the decisions are clicked while focused here and pressed with real keys in the
 * browser review (handoffs/0049-agent-approval-readiness/tools/capture.mjs).
 */

const SIGNED_IN: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'ada@example.com', accounts: [{ id: 'a', label: 'ada@example.com' }], activeAccount: 'a' };
const SIGNED_OUT: ProviderState = { status: 'not-connected', provider: 'openai' };
const TRANSPORT: BridgeError = { code: 'agent.transport', message: 'Could not contact the native agent. Refresh the session to retry.' };
const CAP = 100_000;

const UPDATING = 'Waiting for the agent to update its session. You can still stop the agent.';
const APPLYING = 'Waiting for the agent to apply the new allowance. You can still stop the agent.';
const SIGN_IN = 'Sign in again to decide. You can still stop the agent.';
const RETRY = 'Retry the connection to decide. You can still stop the agent.';
/** The native guard's refusal, which a press during the hold used to show. */
const REFUSED = 'Connect an OpenAI account and refresh the native session before deciding an approval.';

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
const awaiting = (text = 'I will lift the crate.'): AgentTurnSnapshot =>
  turn(3, {
    status: 'awaiting_approval',
    text,
    approvals: [{ id: 7, tool: 'doc_patch', bytes: 80, decision: 'pending' }],
    pending: { turn: 3, id: 7, tool: 'doc_patch', arguments: { description: 'Lift the crate', expected_revision: 7, ops: [] } },
  });
function session(version: number, last: AgentTurnSnapshot = awaiting()): AgentSessionSnapshot {
  return { version, budget: { max_tokens: CAP, used_tokens: 6_532 }, accounting_uncertain: false, worker_running: true, turns: [last] };
}
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

const pane = () => document.querySelector<HTMLElement>('[data-region="agent"]')!;
const button = (name: string | RegExp) => within(pane()).getByRole('button', { name });
const header = () => pane().querySelector('.agent__status')!.textContent!.replace(/^,\s*/, '');
const note = () => pane().querySelector('.agent-approval__note')?.textContent ?? null;
const input = () => screen.getByLabelText('Message to the agent') as HTMLTextAreaElement;
/** Deny and Allow: focusable and marked unavailable ('true'), or usable (null). */
const decisions = () => ['Deny', 'Allow'].map((name) => button(name).getAttribute('aria-disabled'));

/** Presses Deny and Allow while focused, as a pointer or Enter/Space would. */
async function pressDecisions() {
  for (const name of ['Deny', 'Allow']) {
    const target = button(name);
    target.focus();
    await act(async () => {
      fireEvent.click(target);
    });
  }
}

/* ---- The real native bridge over deferred, mocked IPC ---- */

/** The project at `revision` with its one edit applied (1) or undone (0). */
function nativeEngine(revision: number, applied: 0 | 1): EngineRead {
  return {
    project: { id: '01JA45FX0000000000000PROJ1', name: 'Harbor', scenes: {}, settings: { tick_rate: 60, agent_budget: { max_tokens: CAP } } },
    revision,
    can_redo: applied === 0,
    history: [{ id: '01JA45FX0000000000000EDIT1', description: 'Rename Lantern', actor: { origin: 'user', actor: 'editor', model: null, conversation_id: null } }],
    applied,
    schemas: { Project: projectSchema },
    console: [],
    viewport_error: null,
    agent_budget: { available: true },
  };
}

/**
 * Mocked native IPC for the real NativeBridge, starting with turn 3 awaiting approval 7 and
 * the session's allowance equal to the project's. Undo/Redo waits for `releaseHistory`,
 * session reads wait while `holdReads` is in force, and an allowance edit is never
 * answered, so a test can look at the pane during each barrier. It answers only the calls
 * the bridge makes, records every one, and changes its session as the native host would:
 * Undo or Redo of an ordinary edit leaves the allowance and the turn alone, and the next
 * read reports a decision or a stop.
 */
function nativeHost(applied: 0 | 1) {
  const calls: { name: string; args: Record<string, unknown> | undefined }[] = [];
  let engine = nativeEngine(7, applied);
  let current = session(1);
  const history = deferred();
  let reads: ReturnType<typeof deferred> | null = null;
  const readStarted = deferred();
  const options = { failReads: false };
  const invoke: Invoke = async <T,>(name: string, args?: Record<string, unknown>) => {
    calls.push({ name, args });
    switch (name) {
      case 'engine_read':
        return engine as T;
      case 'engine_history':
        await history.promise;
        engine = nativeEngine(engine.revision + 1, args?.redo === true ? 1 : 0);
        return engine as T;
      case 'engine_execute':
        // Only the pane while its save is in flight matters here.
        return new Promise<T>(() => undefined);
      case 'engine_agent_read': {
        if (reads) {
          readStarted.resolve();
          await reads.promise;
        }
        if (options.failReads) throw 'native transport failure';
        const reply: EngineAgentRead = { session: args?.afterVersion === current.version ? null : current, project_revision: engine.revision };
        return reply as T;
      }
      case 'engine_agent_decide':
        current = session(current.version + 1, turn(3, { status: 'running', text: 'I will lift the crate.', approvals: [{ id: 7, tool: 'doc_patch', bytes: 80, decision: args?.approved === true ? 'allowed' : 'denied' }] }));
        return null as T;
      case 'engine_agent_cancel':
        current = session(current.version + 1, turn(3, { status: 'stopping', text: 'I will lift the crate.', approvals: [{ id: 7, tool: 'doc_patch', bytes: 80, decision: 'cancelled' }] }));
        return null as T;
      case 'viewport_bounds':
        return null as T;
      default:
        throw new Error(`unexpected IPC ${name}`);
    }
  };
  return {
    invoke,
    options,
    readStarted: readStarted.promise,
    holdReads: () => {
      reads = deferred();
    },
    releaseHistory: () => history.resolve(),
    releaseReads: () => {
      const held = reads;
      reads = null;
      held?.resolve();
    },
    called: (name: string) => calls.filter((call) => call.name === name),
  };
}

type NativeHost = ReturnType<typeof nativeHost>;

async function nativePane(applied: 0 | 1) {
  const host = nativeHost(applied);
  const bridge = new NativeBridge(host.invoke);
  await bridge.start();
  bridge.updateProvider(SIGNED_IN);
  await bridge.refreshAgent();
  render(<App resolution={{ kind: 'bridge', bridge }} />);
  expect(header()).toBe('Needs approval');
  expect(bridge.getSnapshot().agent).toMatchObject({ status: 'running', approvalReady: true });
  expect(decisions()).toEqual([null, null]);
  return { host, bridge };
}

/** The hold as the reader sees it: the conversation, draft and Stop stay; the decisions wait. */
function expectHeld(bridge: NativeBridge, reason: string) {
  expect(bridge.getSnapshot().agent).toMatchObject({ status: 'running', approvalReady: false });
  expect(note()).toBe(reason);
  expect(decisions()).toEqual(['true', 'true']);
  expect(button('Stop').getAttribute('aria-disabled')).toBeNull();
  expect(header()).toBe('Needs approval');
  expect(pane().querySelector('.agent-turn__reply')!.textContent).toContain('I will lift the crate.');
}

describe('Approval readiness on the real native bridge (deferred IPC)', () => {
  it.each([
    ['history.undo', 1, /^Undo/],
    ['history.redo', 0, /^Redo/],
  ] as const)('holds Allow and Deny through %s and the fresh session read, then sends the real decision', async (_type, applied, control) => {
    const { host, bridge } = await nativePane(applied);
    fireEvent.change(input(), { target: { value: 'Then add fog' } });
    host.holdReads();
    fireEvent.click(screen.getByRole('button', { name: control }));

    // Waiting for engine_history. Both cached allowances still match, and the pane saved nothing.
    expect(host.called('engine_history')).toHaveLength(1);
    expect(bridge.getSnapshot().projectAgentBudget).toEqual({ maxTokens: CAP });
    expect(bridge.getSnapshot().agentSession?.budget.max_tokens).toBe(CAP);
    expectHeld(bridge, UPDATING);
    await pressDecisions();
    expect(host.called('engine_agent_decide')).toEqual([]);
    expect(within(pane()).queryByText(REFUSED)).toBeNull();

    // The history reply lands; the bridge then reads a fresh session before admitting decisions.
    await act(async () => {
      host.releaseHistory();
      await host.readStarted;
    });
    expect(bridge.getSnapshot().history?.applied).toBe(applied === 1 ? 0 : 1);
    expectHeld(bridge, UPDATING);
    await pressDecisions();
    expect(host.called('engine_agent_decide')).toEqual([]);
    expect(within(pane()).queryByText(REFUSED)).toBeNull();

    // Verified: the decisions come back, and the real one is sent.
    await act(async () => host.releaseReads());
    await waitFor(() => expect(decisions()).toEqual([null, null]));
    expect(bridge.getSnapshot().agent).toMatchObject({ status: 'running', approvalReady: true });
    expect(note()).toBeNull();
    expect(input().value).toBe('Then add fog');
    fireEvent.click(button('Allow'));
    await within(pane()).findByText('Allowed');
    expect(host.called('engine_agent_decide')).toEqual([{ name: 'engine_agent_decide', args: { turnId: 3, approvalId: 7, approved: true } }]);
    expect(header()).toBe('Working…');
    expect(input().value).toBe('Then add fog');
  });

  it('keeps Stop usable while decisions wait, and stops the turn by its ID', async () => {
    const { host, bridge } = await nativePane(1);
    fireEvent.click(screen.getByRole('button', { name: /^Undo/ }));
    expectHeld(bridge, UPDATING);
    fireEvent.click(button('Stop'));
    // Until the host reports the stop, the decisions name it as the reason they wait.
    expect(note()).toBe('The agent is stopping.');
    await waitFor(() => expect(host.called('engine_agent_cancel')).toEqual([{ name: 'engine_agent_cancel', args: { turnId: 3 } }]));
    await waitFor(() => expect(header()).toBe('Stopping…'));
    expect(pane().querySelector('.agent-approval')).toBeNull();
    expect(host.called('engine_agent_decide')).toEqual([]);
  });

  const causes: [string, string, (host: NativeHost, bridge: NativeBridge) => Promise<unknown>][] = [
    [
      'the pane’s own allowance save',
      APPLYING,
      async (host) => {
        fireEvent.click(button('Change allowance'));
        const field = within(pane()).getByRole('textbox', { name: 'Allowance in tokens for this project' }) as HTMLInputElement;
        fireEvent.change(field, { target: { value: '50000' } });
        fireEvent.submit(field.form!);
        expect(host.called('engine_execute')).toHaveLength(1);
      },
    ],
    ['a sign-out', SIGN_IN, async (_host, bridge) => act(() => bridge.updateProvider(SIGNED_OUT))],
    [
      'a failed session read',
      RETRY,
      async (host, bridge) => {
        host.options.failReads = true;
        await act(async () => bridge.refreshAgent());
        expect(within(pane()).getByRole('note').textContent).toContain(TRANSPORT.message);
      },
    ],
  ];
  it.each(causes)('keeps the more precise reason after %s', async (_cause, reason, cause) => {
    const { host, bridge } = await nativePane(1);
    await cause(host, bridge);
    expectHeld(bridge, reason);
    await pressDecisions();
    expect(host.called('engine_agent_decide')).toEqual([]);
    expect(within(pane()).queryByText(REFUSED)).toBeNull();
    fireEvent.click(button('Stop'));
    await waitFor(() => expect(host.called('engine_agent_cancel')).toEqual([{ name: 'engine_agent_cancel', args: { turnId: 3 } }]));
  });
});

/* ---- Hosts that report, or do not report, readiness (test doubles) ---- */

/**
 * TEST DOUBLE, not a bridge implementation: the sample project with a project allowance
 * and an agent session the test controls. It records every request, answers each with
 * success and never changes its own snapshot; the test publishes what a host would.
 */
function testHost(agent: AgentState, fields: Partial<BridgeSnapshot> = {}, capabilities = ['agent.send', 'agent.cancel', 'agent.approve']) {
  let snapshot: BridgeSnapshot = { ...fixtureSnapshot('sample'), provider: SIGNED_IN, agent, projectAgentBudget: { maxTokens: CAP }, agentSession: session(1), ...fields };
  const listeners = new Set<() => void>();
  const requests: HostRequest[] = [];
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities,
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
      return Promise.resolve({ ok: true });
    },
  };
  const update = (patch: Partial<BridgeSnapshot>) =>
    act(() => {
      snapshot = { ...snapshot, ...patch };
      for (const listener of listeners) listener();
    });
  render(<App resolution={{ kind: 'bridge', bridge }} />);
  return { requests, update };
}

const RUNNING = { status: 'running', conversation: '3' } as const;

describe('Approval readiness reported by the host (test double)', () => {
  it('holds Allow and Deny while the host is not ready, even with matching allowances, until it is', async () => {
    const host = testHost({ ...RUNNING, approvalReady: false });
    expect(header()).toBe('Needs approval');
    expect(note()).toBe(UPDATING);
    expect(decisions()).toEqual(['true', 'true']);
    expect(button('Stop').getAttribute('aria-disabled')).toBeNull();
    await pressDecisions();
    expect(host.requests).toEqual([]);
    host.update({ agent: { ...RUNNING, approvalReady: true } });
    expect(note()).toBeNull();
    expect(decisions()).toEqual([null, null]);
    fireEvent.click(button('Allow'));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.approve', turnId: 3, approvalId: 7, approved: true }]));
  });

  it('keeps decisions usable on an older host that does not report readiness', async () => {
    const host = testHost(RUNNING);
    expect(note()).toBeNull();
    expect(decisions()).toEqual([null, null]);
    // A later update that still omits it changes nothing.
    host.update({ agentSession: session(2, awaiting('I will lift the crate, then the lantern.')) });
    expect(decisions()).toEqual([null, null]);
    fireEvent.click(button('Deny'));
    await waitFor(() => expect(host.requests).toEqual([{ type: 'agent.approve', turnId: 3, approvalId: 7, approved: false }]));
  });

  it.each([
    ['signed out', { provider: SIGNED_OUT }, SIGN_IN],
    ['after a transport failure', { agentError: TRANSPORT }, RETRY],
    ['while the session’s allowance lags the project’s', { projectAgentBudget: { maxTokens: 50_000 } }, APPLYING],
  ] as const)('keeps the more precise reason when the host is also not ready: %s', async (_cause, fields, reason) => {
    const host = testHost({ ...RUNNING, approvalReady: false }, fields);
    expect(note()).toBe(reason);
    expect(decisions()).toEqual(['true', 'true']);
    expect(button('Stop').getAttribute('aria-disabled')).toBeNull();
    await pressDecisions();
    expect(host.requests).toEqual([]);
  });

  it('keeps naming a decision this host cannot make at all', () => {
    testHost({ ...RUNNING, approvalReady: false }, {}, ['agent.send', 'agent.cancel']);
    expect(note()).toBe('Decide agent approval is unavailable in this editor build (agent.approve).');
  });
});
