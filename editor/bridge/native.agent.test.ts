import { describe, expect, it, vi } from 'vitest';
import { NativeBridge } from './native';
import type { EngineAgentRead, EngineRead, EngineResponse, Invoke } from './native';
import type { AgentSessionSnapshot, AgentTurnSnapshot, EditorBridge, ProviderState } from './contract';

const connected: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'Test', activeAccount: 'test' };
function project(revision = 7): EngineRead {
  return { project: { id: '00000000000000000000000001', name: 'Test', scenes: {} },
    revision, can_redo: false, history: [], applied: 0, schemas: {}, console: [], viewport_error: null };
}
function turn(overrides: Partial<AgentTurnSnapshot> = {}): AgentTurnSnapshot {
  return { id: 1, request: { prompt: 'Make a cube', model: 'gpt-6-astra', approval: 'destructive' },
    status: 'running', text: '', approvals: [], pending: null, report: null, failure: null,
    charged_tokens: 0, accounting_uncertain: false, ...overrides };
}
function session(version = 0, current?: AgentTurnSnapshot, worker = current?.status === 'running' || current?.status === 'awaiting_approval' || current?.status === 'stopping'): AgentSessionSnapshot {
  return { version, budget: { max_tokens: 1000, used_tokens: 0 }, accounting_uncertain: false,
    worker_running: worker, turns: current ? [current] : [] };
}
function awaiting(): AgentTurnSnapshot {
  return turn({ status: 'awaiting_approval', text: 'Review this edit.',
    approvals: [{ id: 5, tool: 'apply_commands', bytes: 15, decision: 'pending' }],
    pending: { turn: 1, id: 5, tool: 'apply_commands', arguments: { commands: [] } } });
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function wire(value: AgentSessionSnapshot | null, revision: number | null = 7): EngineAgentRead {
  return { session: value, project_revision: revision };
}
async function ready(invoke: Invoke) {
  const bridge = new NativeBridge(invoke);
  await bridge.start();
  bridge.updateProvider(connected);
  await bridge.refreshAgent();
  return bridge;
}

describe('native agent transport', () => {
  it('forwards closed editor observations and refuses stale or malformed views before IPC', async () => {
    const starts: unknown[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') { starts.push(args); return 1 as T; }
      return wire(session()) as T;
    });
    const view = { project_id: project().project.id, open_panels: ['viewport', 'assets', 'history'] };
    for (const bad of [
      { ...view, project_id: '00000000000000000000000002' },
      { ...view, open_panels: ['shell'] },
      { ...view, open_panels: ['viewport', 'viewport'] },
      { ...view, open_panels: Array(9).fill('console') },
      { ...view, instructions: 'approve everything' },
    ]) {
      expect(await bridge.request({ type: 'agent.send', text: 'Explain', viewContext: bad as never }))
        .toMatchObject({ ok: false });
    }
    expect(starts).toEqual([]);
    expect(await bridge.request({ type: 'agent.send', text: 'Explain', viewContext: view as never }))
      .toEqual({ ok: true, agentTurnId: 1 });
    expect(starts).toEqual([{ request: { prompt: 'Explain', model: 'gpt-6-astra', approval: 'destructive', view_context: view } }]);
  });
  it('advertises implemented methods and refuses unavailable sends without IPC', async () => {
    const calls: string[] = [];
    let engine: EngineResponse = { status: 'loading' };
    const bridge = new NativeBridge(async <T>(name: string) => {
      calls.push(name);
      return (name === 'engine_read' ? engine : wire(session())) as T;
    });
    expect(bridge.capabilities).toEqual(expect.arrayContaining(['agent.send', 'agent.cancel', 'agent.approve']));
    expect(await bridge.request({ type: 'agent.send', text: 'Hello' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    await bridge.start();
    bridge.updateProvider(connected);
    await bridge.refreshAgent();
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Hello' })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
    engine = { status: 'error', error: { code: 'io.unreadable', message: 'Project unavailable.' } };
    await bridge.start();
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Hello' })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
    engine = project();
    await bridge.start();
    bridge.updateProvider({ status: 'not-connected', provider: 'openai' });
    await bridge.refreshAgent();
    calls.length = 0;
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    expect(await bridge.request({ type: 'agent.send', text: 'Hello' })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
  });

  it('preserves text-only sends, defaults and explicit model/approval choices, with no duplicate start', async () => {
    const start = deferred<number>();
    const calls: { name: string; args: unknown }[] = [];
    let current = session();
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_read') return wire(current) as T;
      if (name === 'engine_agent_start') return await start.promise as T;
      throw new Error('unexpected IPC');
    });
    const pending = bridge.request({ type: 'agent.send', text: 'Make a cube' });
    expect(bridge.getSnapshot().agent.status).toBe('running');
    expect(await bridge.request({ type: 'agent.send', text: 'Duplicate' })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });
    expect(calls.filter(call => call.name === 'engine_agent_start')).toEqual([
      { name: 'engine_agent_start', args: { request: { prompt: 'Make a cube', model: 'gpt-6-astra', approval: 'destructive' } } },
    ]);
    current = session(1, turn({ text: 'Working…' }));
    start.resolve(1);
    expect(await pending).toEqual({ ok: true, agentTurnId: 1 });
    expect(bridge.getSnapshot().agentSession?.turns[0]?.text).toBe('Working…');
    expect(await bridge.request({ type: 'agent.send', text: 'Still busy' })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });

    let explicit: unknown;
    const other = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') { explicit = args; return 1 as T; }
      return wire(session()) as T;
    });
    expect(await other.request({ type: 'agent.send', text: 'Inspect', model: 'selected-model', approval: 'always' })).toEqual({ ok: true, agentTurnId: 1 });
    expect(explicit).toEqual({ request: { prompt: 'Inspect', model: 'selected-model', approval: 'always' } });
    // A start receipt reserves its turn until a native read confirms it, even if that read is unchanged.
    expect(await other.request({ type: 'agent.send', text: 'Too early' })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });
  });

  it('checks UTF-8 request limits and measured session allowance before invoking', async () => {
    const calls: string[] = [];
    let current = session();
    const bridge = await ready(async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? project() : wire(current)) as T; });
    calls.length = 0;
    for (const request of [
      { type: 'agent.send', text: ' ' },
      { type: 'agent.send', text: '🙂'.repeat(4097) },
      { type: 'agent.send', text: 'Okay', model: '🙂'.repeat(33) },
      { type: 'agent.send', text: 'Okay', model: '' },
    ] as const) expect(await bridge.request(request)).toMatchObject({ ok: false, error: { code: 'agent.invalid-request' } });
    expect(calls).toEqual([]);
    current = { ...session(1), budget: { max_tokens: 1000, used_tokens: 1000 }, accounting_uncertain: true };
    await bridge.refreshAgent();
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Okay' })).toMatchObject({ ok: false, error: { code: 'agent.budget' } });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    expect(calls).toEqual([]);
  });

  it('polls active streaming versions, preserves unchanged identity and refreshes only changed project revision', async () => {
    vi.useFakeTimers();
    try {
      const calls: { name: string; args: unknown }[] = [];
      let current = session(1, turn({ text: 'First' }));
      let revision = 7;
      const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
        calls.push({ name, args });
        return (name === 'engine_read' ? project(revision) : wire(args?.afterVersion === current.version ? null : current, revision)) as T;
      });
      const stop = bridge.startAgentUpdates();
      await bridge.refreshAgent();
      calls.length = 0;
      let notices = 0;
      bridge.subscribe(() => { notices++; });
      current = session(2, turn({ text: 'First second' }));
      await vi.advanceTimersByTimeAsync(249);
      expect(calls).toEqual([]);
      await vi.advanceTimersByTimeAsync(1);
      expect(calls).toEqual([{ name: 'engine_agent_read', args: { afterVersion: 1 } }]);
      expect(bridge.getSnapshot().agentSession?.turns[0]?.text).toBe('First second');
      expect(Object.isFrozen(bridge.getSnapshot().agentSession?.turns[0])).toBe(true);
      const snapshot = bridge.getSnapshot();
      expect(notices).toBe(1);
      await vi.advanceTimersByTimeAsync(250);
      expect(bridge.getSnapshot()).toBe(snapshot);
      expect(notices).toBe(1);
      revision = 8;
      await vi.advanceTimersByTimeAsync(250);
      expect(calls.filter(call => call.name === 'engine_read')).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(250);
      expect(calls.filter(call => call.name === 'engine_read')).toHaveLength(1);
      current = session(3, turn({ status: 'completed', text: 'Done' }), false);
      await vi.advanceTimersByTimeAsync(250);
      expect(bridge.getSnapshot().agent.status).toBe('idle');
      const count = calls.length;
      await vi.advanceTimersByTimeAsync(1000);
      expect(calls).toHaveLength(count);
      expect(calls.some(call => call.name === 'engine_execute')).toBe(false);
      stop();
    } finally { vi.useRealTimers(); }
  });

  it('coalesces slow reads and stops polling on disposal', async () => {
    vi.useFakeTimers();
    try {
      const reply = deferred<EngineAgentRead>();
      const calls: string[] = [];
      const bridge = new NativeBridge(async <T>(name: string) => { calls.push(name); return await reply.promise as T; });
      const stop = bridge.startAgentUpdates();
      const sameRead = bridge.refreshAgent();
      await vi.advanceTimersByTimeAsync(3000);
      expect(calls).toEqual(['engine_agent_read']);
      stop();
      reply.resolve(wire(session(1, turn())));
      await sameRead;
      await vi.advanceTimersByTimeAsync(1000);
      expect(calls).toEqual(['engine_agent_read']);
    } finally { vi.useRealTimers(); }
  });

  it('routes approvals and cancellation with explicit IDs, refusing duplicate and stale decisions', async () => {
    const decide = deferred<null>();
    const calls: { name: string; args: unknown }[] = [];
    let current = session(2, awaiting());
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_decide') return await decide.promise as T;
      if (name === 'engine_agent_cancel') {
        current = session(3, turn({ status: 'stopping', text: 'Review this edit.', approvals: [{ id: 5, tool: 'apply_commands', bytes: 15, decision: 'cancelled' }] }));
        return null as T;
      }
      return wire(current) as T;
    });
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.approve', turnId: 2, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.stale-turn' } });
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 4, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.stale-approval' } });
    expect(await bridge.request({ type: 'agent.cancel', turnId: Number.MAX_SAFE_INTEGER + 1 })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
    const approval = bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: false });
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.stale-approval' } });
    expect(await bridge.request({ type: 'agent.cancel', turnId: 1 })).toEqual({ ok: true });
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'stopping', text: 'Review this edit.', pending: null });
    decide.resolve(null);
    expect(await approval).toEqual({ ok: true });
    expect(calls).toContainEqual({ name: 'engine_agent_decide', args: { turnId: 1, approvalId: 5, approved: false } });
    expect(calls).toContainEqual({ name: 'engine_agent_cancel', args: { turnId: 1 } });
    expect(bridge.getSnapshot().agentSession?.turns[0]?.status).toBe('stopping');
    expect(await bridge.request({ type: 'agent.cancel', turnId: 1 })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });
    current = session(4, turn({ status: 'cancelled', text: 'Review this edit.' }), false);
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentSession?.turns[0]?.status).toBe('cancelled');
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.stale-turn' } });
  });

  it.each(['reply', 'failure'] as const)('fences an old poll %s after a successful newer start', async (kind) => {
    const old = deferred<EngineAgentRead>();
    let reads = 0;
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') return 1 as T;
      reads++;
      if (reads === 1) return wire(session()) as T;
      if (reads === 2) return await old.promise as T;
      return wire(session(1, turn({ text: 'New turn' }))) as T;
    });
    const pending = bridge.refreshAgent();
    expect(await bridge.request({ type: 'agent.send', text: 'Start' })).toEqual({ ok: true, agentTurnId: 1 });
    if (kind === 'reply') old.resolve(wire(session(), 999));
    else old.reject('obsolete error containing sensitive data');
    await pending;
    expect(bridge.getSnapshot().agentSession?.turns[0]?.text).toBe('New turn');
    expect(bridge.getSnapshot().agentError).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('running');
  });

  it('ignores older session versions and their obsolete project revisions', async () => {
    const calls: string[] = [];
    let response = wire(session(2, turn({ text: 'Latest' })));
    const bridge = await ready(async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? project() : response) as T; });
    const snapshot = bridge.getSnapshot();
    calls.length = 0;
    response = wire(session(1, turn({ text: 'Obsolete' })), 999);
    await bridge.refreshAgent();
    expect(bridge.getSnapshot()).toBe(snapshot);
    expect(calls).toEqual(['engine_agent_read']);
  });

  it('preserves actual stopping/failure history through account errors and keeps cancellation usable', async () => {
    let current = session(2, awaiting());
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return project() as T;
      if (name === 'provider_action' || name === 'engine_agent_cancel') return null as T;
      if (name === 'provider_read') return { status: 'error', provider: 'openai', error: { code: 'provider.account', message: 'Account is unavailable.' } } as T;
      return wire(current) as T;
    });
    expect(await bridge.request({ type: 'provider.switch', accountId: 'different' })).toEqual({ ok: true });
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'awaiting_approval', text: 'Review this edit.' });
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    current = session(3, turn({ status: 'stopping', text: 'Review this edit.', failure: 'provider' }));
    expect(await bridge.request({ type: 'agent.cancel', turnId: 1 })).toEqual({ ok: true });
    expect(bridge.getSnapshot().agentSession?.turns[0]?.status).toBe('stopping');
    current = session(4, turn({ status: 'failed', text: 'Review this edit.', failure: 'provider', charged_tokens: 27 }), false);
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'failed', failure: 'provider', charged_tokens: 27 });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Retry' })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
  });

  it('requires a fresh session after an account change while preserving cancellation of the existing worker', async () => {
    const old = deferred<EngineAgentRead>();
    const calls: string[] = [];
    let reads = 0;
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_cancel') return null as T;
      reads++;
      if (reads === 1) return wire(session(2, awaiting())) as T;
      if (reads === 2) return await old.promise as T;
      return wire(session(3, turn({ status: 'stopping', text: 'Original response', failure: 'provider' }))) as T;
    });
    bridge.updateProvider({ ...connected, activeAccount: 'different', accountLabel: 'Different' });
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).not.toContain('engine_agent_decide');
    expect(await bridge.request({ type: 'agent.cancel', turnId: 1 })).toEqual({ ok: true });
    old.resolve(wire(session(2, awaiting())));
    await old.promise;
    await Promise.resolve();
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'stopping', text: 'Original response', failure: 'provider' });
  });

  it('does not reserve a finished turn forever when its start receipt arrives after an account refresh', async () => {
    const start = deferred<number>();
    let current = session();
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') return await start.promise as T;
      return wire(args?.afterVersion === current.version ? null : current) as T;
    });
    const pending = bridge.request({ type: 'agent.send', text: 'Start' });
    current = session(2, turn({ status: 'cancelled', text: 'Partial response' }), false);
    bridge.updateProvider({ ...connected, activeAccount: 'different', accountLabel: 'Different' });
    await bridge.refreshAgent();
    start.resolve(1);
    expect(await pending).toEqual({ ok: true, agentTurnId: 1 });
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'cancelled', text: 'Partial response' });
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it.each(['reply', 'failure'] as const)('does not let an obsolete provider bootstrap %s replace a newer account event', async (kind) => {
    const old = deferred<ProviderState>();
    const bridge = new NativeBridge(async <T>() => await old.promise as T);
    const pending = bridge.startProviderUpdates();
    bridge.updateProvider(connected);
    if (kind === 'reply') old.resolve({ status: 'not-connected', provider: 'openai' });
    else old.reject('obsolete provider exception');
    await pending;
    expect(bridge.getSnapshot().provider).toEqual(connected);
  });

  it('waits for all provider actions and ignores account replies superseded by newer actions', async () => {
    const first = deferred<ProviderState>();
    const second = deferred<ProviderState>();
    let providerReads = 0;
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push(name);
      if (name === 'engine_read') return project() as T;
      if (name === 'provider_action') return null as T;
      if (name === 'provider_read') return await (++providerReads === 1 ? first.promise : second.promise) as T;
      return wire(args?.afterVersion === 0 ? null : session()) as T;
    });
    const older = bridge.request({ type: 'provider.switch', accountId: 'older' });
    await Promise.resolve();
    const newer = bridge.request({ type: 'provider.switch', accountId: 'newer' });
    await Promise.resolve();
    first.resolve({ ...connected, activeAccount: 'older', accountLabel: 'Older' });
    expect(await older).toEqual({ ok: true });
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Too early' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    second.resolve({ ...connected, activeAccount: 'newer', accountLabel: 'Newer' });
    expect(await newer).toEqual({ ok: true });
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().provider).toMatchObject({ activeAccount: 'newer', accountLabel: 'Newer' });
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it('uses safe native setup errors and requires explicit recovery before another send', async () => {
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push(name);
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') throw { code: 'agent.provider', message: 'The selected OpenAI account is unavailable.' };
      return wire(args?.afterVersion === 0 ? null : session()) as T;
    });
    expect(await bridge.request({ type: 'agent.send', text: 'Start' })).toMatchObject({ ok: false, error: { code: 'agent.provider' } });
    expect(bridge.getSnapshot().agentError?.code).toBe('agent.provider');
    expect(bridge.getSnapshot().agentSession?.turns).toEqual([]);
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Retry' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    const typed: EditorBridge = bridge;
    expect(typeof typed.refreshAgent).toBe('function');
    await typed.refreshAgent?.();
    expect(bridge.getSnapshot().agentError).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it.each(['send', 'cancel', 'approve'] as const)('does not publish an old %s failure after its recovery read is superseded by an account change', async (action) => {
    const old = deferred<EngineAgentRead>();
    const recoveryStarted = deferred<void>();
    const initial = action === 'send' ? session() : session(2, awaiting());
    let current = initial;
    let reads = 0;
    const error = { code: 'agent.provider', message: 'Old account unavailable.' };
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_read') {
        if (++reads === 2) { recoveryStarted.resolve(); return await old.promise as T; }
        return wire(current) as T;
      }
      if (['engine_agent_start', 'engine_agent_cancel', 'engine_agent_decide'].includes(name)) throw error;
      throw new Error('unexpected IPC');
    });
    const pending = bridge.request(action === 'send' ? { type: 'agent.send', text: 'Start' }
      : action === 'cancel' ? { type: 'agent.cancel', turnId: 1 }
        : { type: 'agent.approve', turnId: 1, approvalId: 5, approved: true });
    await recoveryStarted.promise;
    current = action === 'send' ? session(1)
      : session(3, turn({ status: 'cancelled', text: 'Native state after account change' }), false);
    bridge.updateProvider({ ...connected, activeAccount: 'new-account', accountLabel: 'New account' });
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentError).toBeUndefined();
    old.resolve(wire(initial));
    expect(await pending).toEqual({ ok: false, error });
    expect(bridge.getSnapshot().provider).toMatchObject({ activeAccount: 'new-account', accountLabel: 'New account' });
    expect(bridge.getSnapshot().agentSession).toEqual(current);
    expect(bridge.getSnapshot().agentError).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it('retains a start reservation after read failure and recovers the actual terminal state explicitly', async () => {
    let readsFail = false;
    let current = session();
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return project() as T;
      if (name === 'engine_agent_start') { readsFail = true; return 1 as T; }
      if (readsFail) throw 'secret API-key value or provider endpoint';
      return wire(current) as T;
    });
    expect(await bridge.request({ type: 'agent.send', text: 'Start' })).toEqual({ ok: true, agentTurnId: 1 });
    expect(bridge.getSnapshot().agentError?.code).toBe('agent.transport');
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('secret API-key');
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Duplicate' })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });
    expect(calls).toEqual([]);
    readsFail = false;
    current = session(2, turn({ status: 'failed', failure: 'context_limit', text: 'Partial response' }), false);
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentError).toBeUndefined();
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'failed', failure: 'context_limit', text: 'Partial response' });
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it('keeps the worker busy until native reap confirms it has stopped', async () => {
    let current = session(1, turn({ status: 'completed' }), true);
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? project() : wire(current)) as T);
    expect(bridge.getSnapshot().agent.status).toBe('running');
    expect(await bridge.request({ type: 'agent.send', text: 'Next' })).toMatchObject({ ok: false, error: { code: 'agent.busy' } });
    current = session(2, turn({ status: 'completed' }), false);
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it('keeps terminal usage/accounting/report values from native metadata without inventing a split', async () => {
    const current = { ...session(1, turn({ status: 'failed', failure: 'budget_accounting', charged_tokens: 900, accounting_uncertain: true,
      report: { steps: 3, usage: { input_tokens: 42, output_tokens: 8 }, transaction_count: 2, tool_call_count: 1,
        transaction_ids: ['00000000000000000000000009'], tool_calls: [{ name: null, succeeded: false, selected_camera: null }], truncated: true } }), false),
      budget: { max_tokens: 1000, used_tokens: 1000 }, accounting_uncertain: true };
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? project() : wire(current)) as T);
    expect(bridge.getSnapshot().agentSession).toEqual(current);
    expect(bridge.getSnapshot().agentSession?.turns[0]?.report?.usage).toEqual({ input_tokens: 42, output_tokens: 8 });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
  });

  it.each([
    ['null bootstrap', wire(null)],
    ['unknown metadata', wire({ ...session(), token: 'must not enter snapshot' } as AgentSessionSnapshot)],
    ['unknown failure', wire(session(1, turn({ failure: 'unknown' as AgentTurnSnapshot['failure'] })))],
    ['unsafe version', wire(session(Number.MAX_SAFE_INTEGER + 1))],
    ['negative revision', wire(session(), -1)],
    ['missing approval', wire(session(1, turn({ status: 'awaiting_approval' })))],
    ['duplicate turn ID', wire({ ...session(1, turn()), turns: [turn({ status: 'completed' }), turn()] })],
    ['malformed report', wire(session(1, turn({ report: { steps: 2 } as AgentTurnSnapshot['report'] })))],
    ['oversized approval', wire(session(1, { ...awaiting(), pending: { ...awaiting().pending!, arguments: 'x'.repeat(2 * 1024 * 1024 + 1) } }))],
    ['excessive approval nodes', wire(session(1, { ...awaiting(), pending: { ...awaiting().pending!, arguments: Array(4096).fill(null) } }))],
  ])('rejects %s visibly without publishing its payload', async (_label, response) => {
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? project() : response) as T);
    expect(bridge.getSnapshot().agentError?.code).toBe('agent.invalid-response');
    expect(bridge.getSnapshot().agentSession).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('must not enter snapshot');
  });
});
