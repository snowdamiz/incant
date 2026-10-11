import { describe, expect, it } from 'vitest';
import { NativeBridge } from './native';
import type { EngineAgentRead, EngineRead, Invoke } from './native';
import type { AgentSessionSnapshot, AgentTurnSnapshot, ProviderState } from './contract';
import projectSchema from '../../schemas/Project.schema.json';

const connected: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'Test', activeAccount: 'test' };
const MAX = Number.MAX_SAFE_INTEGER;
function project(maxTokens: number | undefined = 100, revision = 7): EngineRead {
  return { project: { id: '00000000000000000000000001', name: 'Test', scenes: {},
    settings: { tick_rate: 60, ...(maxTokens === undefined ? {} : { agent_budget: { max_tokens: maxTokens } }) } },
    revision, can_redo: false, history: [], applied: 0, schemas: { Project: projectSchema },
    console: [], viewport_error: null, agent_budget: { available: true } };
}
function session(maxTokens = 100, used = 40, version = 1, turn?: AgentTurnSnapshot): AgentSessionSnapshot {
  return { version, budget: { max_tokens: maxTokens, used_tokens: used }, accounting_uncertain: false,
    worker_running: turn !== undefined && ['running', 'awaiting_approval', 'stopping'].includes(turn.status), turns: turn ? [turn] : [] };
}
function approval(): AgentTurnSnapshot {
  return { id: 1, request: { prompt: 'Edit', model: 'gpt-6-astra', approval: 'always' }, status: 'awaiting_approval',
    text: 'Review the edit.', approvals: [{ id: 5, tool: 'apply_commands', bytes: 15, decision: 'pending' }],
    pending: { turn: 1, id: 5, tool: 'apply_commands', arguments: { commands: [] } }, report: null, failure: null,
    charged_tokens: 0, accounting_uncertain: false };
}
function wire(current: AgentSessionSnapshot | null, revision: number | null = 7): EngineAgentRead {
  return { session: current, project_revision: revision };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
async function ready(invoke: Invoke) {
  const bridge = new NativeBridge(invoke);
  await bridge.start();
  bridge.updateProvider(connected);
  await bridge.refreshAgent();
  return bridge;
}

describe('native project agent budget', () => {
  it('reserves a cap mutation before synchronous subscribers can dispatch another write', async () => {
    let engine = project();
    let current = session();
    let writes = 0;
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_execute') { writes += 1; engine = project(200, 8); current = session(200, 40, 2); }
      return (name === 'engine_read' || name === 'engine_execute' ? engine : wire(current, engine.revision)) as T;
    });
    let nested: ReturnType<NativeBridge['dispatch']> | undefined;
    const unsubscribe = bridge.subscribe(() => {
      unsubscribe();
      nested = bridge.dispatch({ type: 'agent.budget.set', maxTokens: 300 });
    });
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toEqual({ ok: true });
    expect(await nested).toMatchObject({ ok: false, error: { code: 'engine.busy' } });
    expect(writes).toBe(1);
    expect(bridge.getSnapshot().projectAgentBudget).toEqual({ maxTokens: 200 });
  });

  it.each(['old host', 'schema only', 'unavailable adapter'] as const)('does not fabricate writable support for %s', async (kind) => {
    const engine = project();
    if (kind === 'old host') { delete engine.agent_budget; engine.schemas = {}; delete engine.project.settings; }
    if (kind === 'schema only') delete engine.agent_budget;
    if (kind === 'unavailable adapter') engine.agent_budget = { available: false };
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? engine : wire(session())) as T; });
    expect(bridge.capabilities).not.toContain('agent.budget.set');
    expect(bridge.getSnapshot().projectAgentBudget).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
    calls.length = 0;
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toMatchObject({ ok: false, error: { code: 'unsupported' } });
    expect(calls).toEqual([]);
  });

  it.each([1_000_000, 0, MAX])('reads and freezes valid cap %s without resetting session usage', async (cap) => {
    const engine = project(cap);
    if (cap === 1_000_000) engine.project.settings = { tick_rate: 60 };
    const current = session(cap, 40);
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? engine : wire(current)) as T);
    expect(bridge.capabilities).toContain('agent.budget.set');
    expect(bridge.getSnapshot().projectAgentBudget).toEqual({ maxTokens: cap });
    expect(Object.isFrozen(bridge.getSnapshot().projectAgentBudget)).toBe(true);
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(40);
    expect(bridge.getSnapshot().agent.status).toBe(cap === 0 ? 'unavailable' : 'idle');
  });

  it.each([
    ['missing settings', undefined], ['null settings', null], ['array settings', []], ['missing tick', {}],
    ['unknown setting', { tick_rate: 60, invented: true }], ['missing cap', { tick_rate: 60, agent_budget: {} }],
    ['null cap', { tick_rate: 60, agent_budget: null }], ['negative cap', { tick_rate: 60, agent_budget: { max_tokens: -1 } }],
    ['fractional cap', { tick_rate: 60, agent_budget: { max_tokens: 1.5 } }],
    ['unsafe cap', { tick_rate: 60, agent_budget: { max_tokens: MAX + 1 } }],
    ['unknown budget key', { tick_rate: 60, agent_budget: { max_tokens: 100, secret: 'must not enter snapshot' } }],
  ])('refuses malformed %s without defaulting or invoking edits', async (_label, settings) => {
    const engine = project(); engine.project.settings = settings;
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? engine : wire(session())) as T; });
    expect(bridge.capabilities).not.toContain('agent.budget.set');
    expect(bridge.getSnapshot().projectAgentBudget).toBeUndefined();
    expect(bridge.getSnapshot().diagnostics).toContainEqual(expect.objectContaining({ id: 'agent-budget', path: 'settings.agent_budget' }));
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('must not enter snapshot');
    calls.length = 0;
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toMatchObject({ ok: false });
    expect(await bridge.request({ type: 'agent.send', text: 'Edit' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
  });

  it.each(['missing schema', 'wrong default', 'unbounded cap', 'malformed availability'] as const)('requires host/schema evidence: %s', async (kind) => {
    const engine = project();
    const schema = structuredClone(projectSchema);
    engine.schemas.Project = schema;
    if (kind === 'missing schema') engine.schemas = {};
    if (kind === 'wrong default') schema.$defs.ProjectSettings.properties.agent_budget.default.max_tokens = 2_000_000;
    if (kind === 'unbounded cap') schema.$defs.AgentBudgetSettings.properties.max_tokens.maximum = MAX + 1;
    if (kind === 'malformed availability') engine.agent_budget = { available: 'yes' } as unknown as NonNullable<EngineRead['agent_budget']>;
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? engine : wire(session())) as T);
    expect(bridge.capabilities).not.toContain('agent.budget.set');
    expect(bridge.getSnapshot().projectAgentBudget).toBeUndefined();
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
  });

  it.each([-1, 1.5, MAX + 1, NaN, Infinity, '100', null])('rejects invalid command cap %s before IPC', async (cap) => {
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? project() : wire(session())) as T; });
    calls.length = 0;
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: cap as number })).toMatchObject({ ok: false, error: { code: 'schema.invalid' } });
    expect(calls).toEqual([]);
  });

  it.each([0, MAX])('uses the shared user command and exact revision for cap %s, then reads actual usage', async (cap) => {
    let engine = project();
    let current = session();
    const calls: { name: string; args: unknown }[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') { engine = project(cap, 8); current = session(cap, 51, 2); return engine as T; }
      return wire(current, engine.revision) as T;
    });
    calls.length = 0;
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: cap })).toEqual({ ok: true });
    expect(calls).toEqual([
      { name: 'engine_execute', args: { commands: [{ op: 'set_agent_budget', settings: { max_tokens: cap } }], description: 'Set project agent token cap', expectedRevision: 7 } },
      { name: 'engine_agent_read', args: { afterVersion: 1 } },
    ]);
    expect(bridge.getSnapshot().projectAgentBudget).toEqual({ maxTokens: cap });
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: cap, used_tokens: 51 });
  });

  it('allows offline authoring and verifies an unchanged session on a no-op cap edit', async () => {
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push(name);
      return (name === 'engine_read' || name === 'engine_execute' ? project() : wire(args?.afterVersion === 1 ? null : session())) as T;
    });
    bridge.updateProvider({ status: 'not-connected', provider: 'openai' });
    await bridge.refreshAgent();
    calls.length = 0;
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 100 })).toEqual({ ok: true });
    expect(calls).toEqual(['engine_execute', 'engine_agent_read']);
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(40);
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
  });

  it.each(['reply', 'failure'] as const)('increases an exhausted cap and fences the older session %s', async (kind) => {
    let engine = project();
    let current = session(100, 100);
    let reads = 0;
    const old = deferred<EngineAgentRead>();
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') { engine = project(200, 8); current = session(200, 100, 2); return engine as T; }
      if (name === 'engine_agent_start') { current = session(200, 100, 3, { ...approval(), status: 'running', approvals: [], pending: null }); return 1 as T; }
      if (++reads === 2) return await old.promise as T;
      return wire(current, engine.revision) as T;
    });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    const obsolete = bridge.refreshAgent();
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toEqual({ ok: true });
    expect(bridge.getSnapshot().agent.status).toBe('idle');
    const snapshot = bridge.getSnapshot();
    calls.length = 0;
    if (kind === 'reply') old.resolve(wire(session(100, 0, 999), 999));
    else old.reject('obsolete credential-like exception');
    await obsolete;
    expect(bridge.getSnapshot()).toBe(snapshot);
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 200, used_tokens: 100 });
    expect(calls).toEqual([]);
    expect(await bridge.request({ type: 'agent.send', text: 'Use the increased allowance' })).toEqual({ ok: true, agentTurnId: 1 });
    expect(calls).toEqual(['engine_agent_start', 'engine_agent_read']);
  });

  it('keeps Send and approvals blocked through cap mutation and fresh mismatched session reads, preserving worker usage', async () => {
    let engine = project();
    let current = session(100, 40, 1, approval());
    const write = deferred<EngineRead>();
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') return await write.promise as T;
      return wire(current, engine.revision) as T;
    });
    const editing = bridge.dispatch({ type: 'agent.budget.set', maxTokens: 0 });
    await bridge.refreshAgent();
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(await bridge.request({ type: 'agent.send', text: 'Too early' })).toMatchObject({ ok: false });
    expect(calls).toEqual([]);
    engine = project(0, 8);
    write.resolve(engine);
    expect(await editing).toEqual({ ok: true });
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(0);
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 100, used_tokens: 40 });
    expect(bridge.getSnapshot().agentSession?.turns[0]?.text).toBe('Review the edit.');
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.approve', turnId: 1, approvalId: 5, approved: true })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    current = session(0, 51, 2, { ...approval(), status: 'cancelled', pending: null,
      approvals: [{ id: 5, tool: 'apply_commands', bytes: 15, decision: 'cancelled' }], charged_tokens: 11 });
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 0, used_tokens: 51 });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
  });

  it.each(['history.undo', 'history.redo'] as const)('%s refreshes cap and session without restoring old consumption, fencing an old read', async (type) => {
    let engine = project(200);
    let current = session(200, 175);
    let reads = 0;
    const old = deferred<EngineAgentRead>();
    const history = deferred<EngineRead>();
    const recovered = deferred<EngineAgentRead>();
    const recoveryStarted = deferred<void>();
    const calls: { name: string; args: unknown }[] = [];
    const bridge = await ready(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, args });
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_history') return await history.promise as T;
      if (++reads === 2) return await old.promise as T;
      if (reads === 4) { recoveryStarted.resolve(); return await recovered.promise as T; }
      return wire(current, engine.revision) as T;
    });
    const obsolete = bridge.refreshAgent();
    const editing = bridge.dispatch({ type });
    await bridge.refreshAgent(); // A newer read while history is in flight still cannot verify admission.
    expect(await bridge.request({ type: 'agent.send', text: 'Too early' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    engine = project(100, 8);
    current = session(100, 175, 2);
    history.resolve(engine);
    await recoveryStarted.promise;
    old.resolve(wire(session(200, 0, 999), 999));
    await obsolete;
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(175);
    expect(await bridge.request({ type: 'agent.send', text: 'Still early' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    recovered.resolve(wire(current, 8));
    expect(await editing).toEqual({ ok: true });
    expect(calls).toContainEqual({ name: 'engine_history', args: { redo: type === 'history.redo' } });
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(100);
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 100, used_tokens: 175 });
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
  });

  it.each(['rejection', 'stale response', 'unchanged cap response', 'malformed response', 'unknown exception'] as const)('preserves the failed edit receipt and recovers current native cap after %s', async (kind) => {
    let engine = project();
    let current = session();
    const original = { code: 'engine.revision-mismatch', message: 'Project revision changed.' };
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') {
        if (kind === 'rejection') throw original;
        if (kind === 'unknown exception') throw 'API-key secret and endpoint';
        engine = project(50, 9); current = session(50, 40, 2);
        if (kind === 'stale response') return project(200, 6) as T;
        if (kind === 'unchanged cap response') return project(100, 8) as T;
        const malformed = project(200, 8); malformed.project.settings = { tick_rate: 60, agent_budget: {} }; return malformed as T;
      }
      return wire(current, engine.revision) as T;
    });
    const result = await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 });
    expect(result).toMatchObject({ ok: false, error: { code: kind === 'rejection' ? original.code : kind === 'stale response' || kind === 'unchanged cap response' ? 'engine.stale' : kind === 'malformed response' ? 'engine.invalid-response' : 'engine.request' } });
    if (kind === 'rejection') expect(result).toEqual({ ok: false, error: original });
    expect(JSON.stringify(result)).not.toContain('API-key secret');
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(kind === 'rejection' || kind === 'unknown exception' ? 100 : 50);
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(40);
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it('keeps a successful cap receipt when the session refresh fails and recovers explicitly', async () => {
    let engine = project(100);
    let fail = false;
    let current = session(100, 100);
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') { engine = project(200, 8); current = session(200, 100, 2); fail = true; return engine as T; }
      if (fail) throw 'native worker credential-like exception';
      return wire(current, engine.revision) as T;
    });
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toEqual({ ok: true });
    expect(bridge.getSnapshot().agentError?.code).toBe('agent.transport');
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('credential-like');
    fail = false;
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(100);
  });

  it('blocks admission while an observed newer document revision is being read', async () => {
    let engine = project();
    let current = session();
    const refresh = deferred<EngineRead>();
    let reads = 0;
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return (++reads === 1 ? engine : await refresh.promise) as T;
      return wire(current, current.version === 1 ? 7 : 8) as T;
    });
    current = session(100, 40, 2);
    const pending = bridge.refreshAgent();
    await Promise.resolve();
    expect(await bridge.request({ type: 'agent.send', text: 'Cached cap is stale' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    engine = project(0, 8); current = session(0, 40, 3);
    refresh.resolve(engine);
    await pending;
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(0);
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(40);
  });

  it('rejects a command reply older than a newer published cap without regressing state', async () => {
    let engine = project();
    let current = session();
    const write = deferred<EngineRead>();
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') return await write.promise as T;
      return wire(current, engine.revision) as T;
    });
    const editing = bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 });
    engine = project(50, 9); current = session(50, 40, 2);
    await bridge.start();
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(50);
    write.resolve(project(200, 8));
    expect(await editing).toMatchObject({ ok: false, error: { code: 'engine.stale' } });
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(50);
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 50, used_tokens: 40 });
  });

  it('retains cancellation while a cap edit is pending without replacing worker accounting', async () => {
    let engine = project();
    let current = session(100, 40, 1, approval());
    const write = deferred<EngineRead>();
    const calls: string[] = [];
    const bridge = await ready(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return engine as T;
      if (name === 'engine_execute') return await write.promise as T;
      if (name === 'engine_agent_cancel') {
        current = session(100, 40, 2, { ...approval(), status: 'stopping', pending: null,
          approvals: [{ id: 5, tool: 'apply_commands', bytes: 15, decision: 'cancelled' }] });
        return null as T;
      }
      return wire(current, engine.revision) as T;
    });
    const editing = bridge.dispatch({ type: 'agent.budget.set', maxTokens: 0 });
    expect(await bridge.request({ type: 'agent.cancel', turnId: 1 })).toEqual({ ok: true });
    expect(bridge.getSnapshot().agentSession?.turns[0]).toMatchObject({ status: 'stopping', text: 'Review the edit.' });
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 100, used_tokens: 40 });
    engine = project(0, 8);
    write.resolve(engine);
    expect(await editing).toEqual({ ok: true });
    expect(calls.filter(name => name === 'engine_agent_cancel')).toHaveLength(1);
    expect(bridge.getSnapshot().projectAgentBudget?.maxTokens).toBe(0);
    expect(bridge.getSnapshot().agentSession?.budget).toEqual({ max_tokens: 100, used_tokens: 40 });
  });

  it('keeps project recovery errors credential-safe after an edit failure', async () => {
    let fail = false;
    const bridge = await ready(async <T>(name: string) => {
      if (name === 'engine_execute') { fail = true; throw 'secret provider endpoint'; }
      if (name === 'engine_read') { if (fail) throw 'secret API-key value'; return project() as T; }
      return wire(session()) as T;
    });
    expect(await bridge.dispatch({ type: 'agent.budget.set', maxTokens: 200 })).toMatchObject({ ok: false, error: { code: 'engine.request' } });
    expect(bridge.getSnapshot().connection.status).toBe('error');
    expect(bridge.getSnapshot().projectAgentBudget).toBeUndefined();
    expect(bridge.capabilities).not.toContain('agent.budget.set');
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('secret');
  });

  it('preserves independently measured usage through account changes', async () => {
    let current = session(100, 75);
    const bridge = await ready(async <T>(name: string) => (name === 'engine_read' ? project() : wire(current)) as T);
    current = session(100, 75, 2);
    bridge.updateProvider({ ...connected, activeAccount: 'new-account', accountLabel: 'New account' });
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().projectAgentBudget).toEqual({ maxTokens: 100 });
    expect(bridge.getSnapshot().agentSession?.budget.used_tokens).toBe(75);
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });
});
