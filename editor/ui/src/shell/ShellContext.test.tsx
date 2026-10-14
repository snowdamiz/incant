import { act, render, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NativeBridge } from '../../../bridge/native';
import type { BridgeResult, EditorBridge } from '../bridge/contract';
import { createFixtureBridge } from '../bridge/fixture';
import { createNativePlayDouble, doubleRead } from '../test/nativePlayDouble';
import type { AuthoringSnapshot, Shell } from './ShellContext';
import type { EngineRead } from '../../../bridge/native';
import projectSchema from '../../../../schemas/Project.schema.json';
import { ShellProvider, capabilityOf, useShell } from './ShellContext';
import type { RequestResult } from './requests';

describe('shell snapshot', () => {
  it('updates native budget capabilities after loading and preserves the authored cap', async () => {
    let engine: EngineRead = {
      project: { id: '00000000000000000000000001', name: 'Test', scenes: {},
        settings: { tick_rate: 60, agent_budget: { max_tokens: 100 } } },
      revision: 1, can_redo: false, history: [], applied: 0, schemas: { Project: projectSchema },
      console: [], viewport_error: null, agent_budget: { available: true },
    };
    const calls: string[] = [];
    const bridge = new NativeBridge(async <T,>(name: string) => {
      calls.push(name);
      if (name === 'engine_execute') engine = { ...engine, revision: 2,
        project: { ...engine.project, settings: { tick_rate: 60, agent_budget: { max_tokens: 0 } } } };
      if (name === 'engine_agent_read') return { session: { version: 1,
        budget: { max_tokens: 0, used_tokens: 40 }, accounting_uncertain: false,
        worker_running: false, turns: [] }, project_revision: 2 } as T;
      return engine as T;
    });
    let shell!: Shell;
    function Panel() { shell = useShell(); return null; }
    render(<ShellProvider bridge={bridge}><Panel /></ShellProvider>);
    expect(shell.capabilities.has('agent.budget.set')).toBe(false);
    await act(async () => { await bridge.start(); });
    expect(shell.capabilities.has('agent.budget.set')).toBe(true);
    expect(shell.snapshot?.projectAgentBudget).toEqual({ maxTokens: 100 });
    const supported = shell.capabilities;
    await act(async () => {
      expect(await shell.dispatch({ type: 'agent.budget.set', maxTokens: 0 })).toEqual({ ok: true });
    });
    expect(shell.snapshot?.projectAgentBudget).toEqual({ maxTokens: 0 });
    expect(shell.snapshot?.agentSession?.budget.used_tokens).toBe(40);
    expect(shell.capabilities).toBe(supported);
    engine = { ...engine, agent_budget: { available: false } };
    await act(async () => { await bridge.start(); });
    expect(shell.capabilities.has('agent.budget.set')).toBe(false);
    expect(shell.snapshot?.projectAgentBudget).toBeUndefined();
    calls.length = 0;
    await act(async () => {
      expect(await shell.dispatch({ type: 'agent.budget.set', maxTokens: 200 }))
        .toMatchObject({ ok: false, error: { code: 'unsupported' } });
    });
    expect(calls).toEqual([]);
  });

  it('does not re-render authoring panels for Play status reads, and keeps Play status out of them', async () => {
    const double = createNativePlayDouble({ read: doubleRead('single', {}) });
    const bridge = new NativeBridge(double.invoke);
    await bridge.startEngineUpdates(double.listen);
    await bridge.refreshPlay();
    const seen: (AuthoringSnapshot | null)[] = [];
    function Panel() {
      seen.push(useShell().snapshot);
      return null;
    }
    render(
      <ShellProvider bridge={bridge}>
        <Panel />
      </ShellProvider>,
    );
    const before = seen.length;
    const shown = seen.at(-1)!;
    expect(shown).not.toHaveProperty('play');
    // Status reads while a session runs: a new status object four times a second.
    for (let tick = 1; tick <= 12; tick += 1) {
      act(() => bridge.updatePlay({ ...double.status(), phase: 'running', sequence: 100 + tick, tick }));
    }
    expect(seen).toHaveLength(before);
    expect(bridge.getSnapshot().play?.tick).toBe(12);
    // An authoring change still reaches them.
    await act(async () => {
      double.bumpRevision();
      await bridge.start();
    });
    expect(seen.length).toBeGreaterThan(before);
    expect(seen.at(-1)).not.toBe(shown);
    expect(seen.at(-1)).not.toHaveProperty('play');
  });

  it('checks Play operations against the play.control capability', () => {
    expect(capabilityOf({ type: 'play.step' })).toBe('play.control');
    expect(capabilityOf({ type: 'play.inspect', entity: 'X' as never })).toBe('play.inspect');
    expect(capabilityOf({ type: 'play.input', generation: 1, events: [] })).toBe('play.input');
    expect(capabilityOf({ type: 'window.close' })).toBe('window.close');
  });

  it('an old revocation cannot erase a newer queued or visible announcement', () => {
    vi.useFakeTimers();
    let view: ReturnType<typeof render> | undefined;
    try {
      let shell!: Shell;
      function Panel() { shell = useShell(); return <div role="status">{shell.message}</div>; }
      view = render(<ShellProvider bridge={null}><Panel /></ShellProvider>);
      let revokeOld!: () => void;
      act(() => { revokeOld = shell.announce('Earlier result'); });
      act(() => { vi.advanceTimersByTime(10); });
      act(() => {
        shell.announce('New unrelated result');
        revokeOld();
      });
      // The earlier queued message never appears, even before the newer one is due.
      act(() => { vi.advanceTimersByTime(20); });
      expect(view.getByRole('status').textContent).toBe('');
      act(() => { vi.advanceTimersByTime(10); });
      expect(view.getByRole('status').textContent).toBe('New unrelated result');
      act(() => { revokeOld(); });
      expect(view.getByRole('status').textContent).toBe('New unrelated result');
    } finally {
      view?.unmount();
      vi.useRealTimers();
    }
  });

  it('revokes its own queued and visible messages and cancels superseded and unmounted timers', () => {
    vi.useFakeTimers();
    let view: ReturnType<typeof render> | undefined;
    try {
      let shell!: Shell;
      function Panel() { shell = useShell(); return <div role="status">{shell.message}</div>; }
      view = render(<ShellProvider bridge={null}><Panel /></ShellProvider>);
      let revoke!: () => void;
      act(() => { revoke = shell.announce('Never shown'); revoke(); });
      act(() => { vi.advanceTimersByTime(30); });
      expect(view.getByRole('status').textContent).toBe('');
      act(() => { revoke = shell.announce('Visible result'); });
      act(() => { vi.advanceTimersByTime(30); });
      expect(view.getByRole('status').textContent).toBe('Visible result');
      act(() => { revoke(); });
      expect(view.getByRole('status').textContent).toBe('');

      act(() => { shell.announce('Older result'); vi.advanceTimersByTime(4000); });
      act(() => { shell.announce('Longer-lived result'); });
      // The older message's expiry must not clear its replacement.
      act(() => { vi.advanceTimersByTime(4050); });
      expect(view.getByRole('status').textContent).toBe('Longer-lived result');
      view.unmount();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      view?.unmount();
      vi.useRealTimers();
    }
  });

  it('resolves a rejected bridge request as a failure marked rejected, keeping the thrown code and message', async () => {
    // TEST DOUBLE: a bridge whose request promise rejects instead of resolving to a result,
    // and finally resolves to a host failure carrying the editor's own rejection code.
    const answers: (() => Promise<BridgeResult>)[] = [
      () => Promise.reject(new Error('IPC channel closed')),
      () => Promise.reject({ code: 'host.closed', message: 'The editor process exited.' }),
      () => Promise.reject(42),
      () => Promise.resolve({ ok: false, error: { code: 'request.rejected', message: 'A host refusal.' } }),
    ];
    const bridge: EditorBridge = {
      ...createFixtureBridge('sample'),
      isFixture: false,
      label: 'Test double',
      capabilities: ['window.close'],
      request: () => answers.shift()!(),
    };
    let shell!: Shell;
    function Panel() {
      shell = useShell();
      return null;
    }
    render(<ShellProvider bridge={bridge}><Panel /></ShellProvider>);
    const results: RequestResult[] = [];
    await act(async () => {
      results.push(await shell.request({ type: 'window.close' }, { quiet: true }));
      results.push(await shell.request({ type: 'window.close' }, { quiet: true }));
      expect(await shell.ask({ type: 'window.close' })).toBe(false);
      results.push(await shell.request({ type: 'window.close' }, { quiet: true }));
    });
    expect(results.map((result) => (result.ok ? null : result.error))).toEqual([
      { code: 'request.rejected', message: 'IPC channel closed' },
      { code: 'host.closed', message: 'The editor process exited.' },
      { code: 'request.rejected', message: 'A host refusal.' },
    ]);
    // Codes cannot establish whether completion arrived; the local marker does.
    expect(results.map((result) => !result.ok && result.outcomeUnknown === true)).toEqual([true, true, false]);
    await waitFor(() => expect(shell.message).toBe('window.close failed: The request ended without a result.'));
  });
});
