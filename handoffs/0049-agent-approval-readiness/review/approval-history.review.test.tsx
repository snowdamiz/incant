import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import { NativeBridge } from '../../../bridge/native';
import type { EngineRead, Invoke } from '../../../bridge/native';
import type { AgentSessionSnapshot } from '../../../bridge/contract';
import schema from '../../../../schemas/Project.schema.json';

function deferred<T>() {
  let resolve!: (value: T) => void;
  return { promise: new Promise<T>(yes => { resolve = yes; }), resolve: (value: T) => resolve(value) };
}
function engine(cap: number, revision: number): EngineRead {
  return {
    project: { id: '00000000000000000000000001', name: 'Test', scenes: {}, settings: { tick_rate: 60, agent_budget: { max_tokens: cap } } },
    revision, can_redo: false,
    history: [{ id: '00000000000000000000000002', description: 'Previous edit', actor: { origin: 'user', actor: 'editor', model: null, conversation_id: null } }],
    applied: 1, schemas: { Project: schema }, console: [], viewport_error: null, agent_budget: { available: true },
  };
}
const current: AgentSessionSnapshot = {
  version: 1, budget: { max_tokens: 100, used_tokens: 40 }, accounting_uncertain: false, worker_running: true,
  turns: [{ id: 1, request: { prompt: 'Edit', model: 'gpt-6-astra', approval: 'always' }, status: 'awaiting_approval',
    text: 'Review this edit.', approvals: [{ id: 5, tool: 'doc_patch', bytes: 15, decision: 'pending' }],
    pending: { turn: 1, id: 5, tool: 'doc_patch', arguments: { description: 'Edit', expected_revision: 7, ops: [] } },
    report: null, failure: null, charged_tokens: 0, accounting_uncertain: false }],
};
describe('exact integrated candidate approval readiness', () => {
  it('observes an enabled Allow during the native Undo barrier and its rejected request', async () => {
    let project = engine(100, 7);
    const history = deferred<EngineRead>();
    const calls: string[] = [];
    const invoke: Invoke = async <T,>(name: string) => {
      calls.push(name);
      if (name === 'engine_read') return project as T;
      if (name === 'engine_agent_read') return { session: current, project_revision: project.revision } as T;
      if (name === 'engine_history') return await history.promise as T;
      if (name === 'viewport_bounds') return null as T;
      throw new Error('unexpected test-only IPC');
    };
    const bridge = new NativeBridge(invoke);
    await bridge.start();
    bridge.updateProvider({ status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'Test', activeAccount: 'test' });
    await bridge.refreshAgent();
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const pane = () => document.querySelector<HTMLElement>('[data-region="agent"]')!;
    expect(within(pane()).getByRole('button', { name: 'Allow' }).getAttribute('aria-disabled')).toBeNull();
    let undo!: ReturnType<NativeBridge['dispatch']>;
    act(() => { undo = bridge.dispatch({ type: 'history.undo' }); });
    expect(calls).toContain('engine_history');
    expect(bridge.getSnapshot().agent.status).toBe('running');
    expect(bridge.getSnapshot().projectAgentBudget!.maxTokens).toBe(100);
    expect(bridge.getSnapshot().agentSession!.budget.max_tokens).toBe(100);
    // Actual observed bug: no UI hold although the authoritative bridge is not ready.
    expect(within(pane()).getByRole('button', { name: 'Allow' }).getAttribute('aria-disabled')).toBeNull();
    expect(within(pane()).getByRole('button', { name: 'Stop' }).getAttribute('aria-disabled')).toBeNull();
    await act(async () => { fireEvent.click(within(pane()).getByRole('button', { name: 'Allow' })); });
    expect(calls).not.toContain('engine_agent_decide');
    expect(within(pane()).getByText('Connect an OpenAI account and refresh the native session before deciding an approval.')).toBeTruthy();
    await act(async () => { project = engine(100, 8); history.resolve(project); await undo; });
  });
});
