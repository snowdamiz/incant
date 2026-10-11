import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { NativeBridge } from '../../../bridge/native';
import { createNativePlayDouble, doubleRead } from '../test/nativePlayDouble';
import type { AuthoringSnapshot, Shell } from './ShellContext';
import type { EngineRead } from '../../../bridge/native';
import projectSchema from '../../../../schemas/Project.schema.json';
import { ShellProvider, capabilityOf, useShell } from './ShellContext';

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
});
