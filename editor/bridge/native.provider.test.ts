import { describe, expect, it } from 'vitest';
import { NativeBridge } from './native';
import type { Invoke } from './native';
import type { ProviderState } from './contract';

const account: ProviderState = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'Current account', activeAccount: 'current' };
type Listen = NonNullable<Parameters<NativeBridge['startProviderUpdates']>[0]>;
function hints() {
  let callback!: (event: { payload: unknown }) => void;
  const listen: Listen = async (event, listener) => {
    expect(event).toBe('incant:provider-changed');
    callback = listener as typeof callback;
    return () => {};
  };
  return { listen, emit: (payload: unknown) => callback({ payload }) };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
async function settle() { for (let index = 0; index < 5; index++) await Promise.resolve(); }

describe('native provider invalidation hints', () => {
  it('re-reads null hints and ignores a delayed connecting payload after terminal account state', async () => {
    const events = hints();
    let current: ProviderState = { status: 'checking', provider: 'openai' };
    const calls: string[] = [];
    const bridge = new NativeBridge(async <T>(name: string) => { calls.push(name); return current as T; });
    await bridge.startProviderUpdates(events.listen);
    current = account;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    events.emit({ status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser', accountLabel: 'Old account' });
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    expect(calls).toEqual(['provider_read', 'provider_read', 'provider_read']);
    expect(JSON.stringify(bridge.getSnapshot().provider)).not.toContain('Old account');
  });

  it.each(['reply', 'failure'] as const)('fences an overlapping old read %s behind the latest hint', async (kind) => {
    const events = hints();
    const old = deferred<ProviderState>();
    const latest = deferred<ProviderState>();
    let reads = 0;
    const bridge = new NativeBridge(async <T>(name: string) => {
      expect(name).toBe('provider_read');
      if (++reads === 1) return { status: 'not-connected', provider: 'openai' } as T;
      return await (reads === 2 ? old.promise : latest.promise) as T;
    });
    await bridge.startProviderUpdates(events.listen);
    events.emit(null);
    events.emit({ status: 'connecting' });
    expect(reads).toBe(3);
    latest.resolve(account);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    if (kind === 'reply') old.resolve({ status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser' });
    else old.reject('secret provider endpoint or token');
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
  });

  it.each(['reply', 'failure'] as const)('fences an obsolete bootstrap %s when an event read finishes first', async (kind) => {
    const events = hints();
    const old = deferred<ProviderState>();
    const started = deferred<void>();
    let reads = 0;
    const bridge = new NativeBridge(async <T>() => {
      if (++reads === 1) { started.resolve(); return await old.promise as T; }
      return account as T;
    });
    const bootstrap = bridge.startProviderUpdates(events.listen);
    await started.promise;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    if (kind === 'reply') old.resolve({ status: 'not-connected', provider: 'openai' });
    else old.reject('obsolete startup failure with private metadata');
    await bootstrap;
    expect(bridge.getSnapshot().provider).toEqual(account);
  });

  it('publishes safe read failures and recovers through a later hint or explicit read', async () => {
    const events = hints();
    let fail = true;
    const invoke: Invoke = async <T>() => { if (fail) throw 'private token or provider endpoint'; return account as T; };
    const bridge = new NativeBridge(invoke);
    await bridge.startProviderUpdates(events.listen);
    expect(bridge.getSnapshot().provider).toMatchObject({ status: 'error', error: { code: 'provider.transport' } });
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('private token');
    fail = false;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    fail = true;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toMatchObject({ status: 'error', error: { code: 'provider.transport' } });
    fail = false;
    await bridge.startProviderUpdates();
    expect(bridge.getSnapshot().provider).toEqual(account);
  });

  it('recovers current terminal state after a provider scheduling failure without replacing the failed receipt', async () => {
    const schedulingError = { code: 'provider.busy', message: 'The previous account operation is still finishing.' };
    const calls: string[] = [];
    const bridge = new NativeBridge(async <T>(name: string) => {
      calls.push(name);
      if (name === 'provider_action') throw schedulingError;
      return account as T;
    });
    bridge.updateProvider({ status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser' });
    expect(await bridge.request({ type: 'provider.connect', method: 'oauth' })).toEqual({ ok: false, error: schedulingError });
    expect(bridge.getSnapshot().provider).toEqual(account);
    expect(calls).toEqual(['provider_action', 'provider_read']);
  });

  it('returns a fixed read error after a scheduled action and recovers on the next hint', async () => {
    const events = hints();
    let fail = false;
    const bridge = new NativeBridge(async <T>(name: string) => {
      if (name === 'provider_action') { fail = true; return null as T; }
      if (fail) throw 'provider_read leaked secret detail';
      return account as T;
    });
    await bridge.startProviderUpdates(events.listen);
    const result = await bridge.request({ type: 'provider.switch', accountId: 'new' });
    expect(result).toMatchObject({ ok: false, error: { code: 'provider.transport' } });
    expect(bridge.getSnapshot().provider).toMatchObject({ status: 'error', error: { code: 'provider.transport' } });
    expect(JSON.stringify([result, bridge.getSnapshot()])).not.toContain('leaked secret');
    fail = false;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
  });

  it('ignores an older action recovery failure after a newer account hint succeeds', async () => {
    const events = hints();
    const old = deferred<ProviderState>();
    const started = deferred<void>();
    let reads = 0;
    const bridge = new NativeBridge(async <T>(name: string) => {
      if (name === 'provider_action') throw { code: 'provider.busy', message: 'Operation is busy.' };
      if (++reads === 2) { started.resolve(); return await old.promise as T; }
      return account as T;
    });
    await bridge.startProviderUpdates(events.listen);
    const pending = bridge.request({ type: 'provider.switch', accountId: 'old' });
    await started.promise;
    events.emit(null);
    await settle();
    expect(bridge.getSnapshot().provider).toEqual(account);
    old.reject('obsolete read failure');
    expect(await pending).toMatchObject({ ok: false, error: { code: 'provider.busy' } });
    expect(bridge.getSnapshot().provider).toEqual(account);
  });

  it('keeps safe previous-account display fields on the actual native connecting response', async () => {
    const pending = { ...account, status: 'connecting', phase: 'browser' };
    const bridge = new NativeBridge(async <T>() => pending as T);
    await bridge.startProviderUpdates();
    expect(bridge.getSnapshot().provider).toEqual(pending);
  });

  it('blocks send against an invalidated provider cache until fresh provider/session reads recover', async () => {
    const events = hints();
    const pending = deferred<ProviderState>();
    const calls: string[] = [];
    let providerReads = 0;
    const bridge = new NativeBridge(async <T>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return { project: { id: '00000000000000000000000001', name: 'Test', scenes: {} },
        revision: 7, can_redo: false, history: [], applied: 0, schemas: {}, console: [], viewport_error: null } as T;
      if (name === 'engine_agent_read') return { session: { version: 0, budget: { max_tokens: 1000, used_tokens: 0 },
        accounting_uncertain: false, worker_running: false, turns: [] }, project_revision: 7 } as T;
      if (++providerReads === 2) return await pending.promise as T;
      return account as T;
    });
    await bridge.startProviderUpdates(events.listen);
    await bridge.start();
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
    events.emit(null);
    expect(bridge.getSnapshot().agent.status).toBe('unavailable');
    calls.length = 0;
    expect(await bridge.request({ type: 'agent.send', text: 'Too early' })).toMatchObject({ ok: false, error: { code: 'agent.unavailable' } });
    expect(calls).toEqual([]);
    pending.resolve(account);
    await settle();
    await bridge.refreshAgent();
    expect(bridge.getSnapshot().agent.status).toBe('idle');
  });

  it.each([null, { status: 'connected', provider: 'openai' }, { ...account, token: 'private value' }])('rejects malformed reads without publishing their payload', async (payload) => {
    const bridge = new NativeBridge(async <T>() => payload as T);
    await bridge.startProviderUpdates();
    expect(bridge.getSnapshot().provider).toMatchObject({ status: 'error', error: { code: 'provider.transport' } });
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('private value');
  });
});
