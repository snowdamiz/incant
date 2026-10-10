import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../App';
import type { EditorBridge } from '../bridge/contract';
import { fixtureSnapshot } from '../bridge/fixture';
import { createProviderFixtureBridge } from '../bridge/providerFixture';
import { snapshotFromEngine } from '../../../bridge/native';
import type { EngineRead } from '../../../bridge/native';

/** TEST DOUBLE: the sample snapshot with a ready agent; every command and request is refused. */
function readyAgentBridge(): EditorBridge {
  const snapshot = { ...fixtureSnapshot('sample'), agent: { status: 'idle' as const } };
  const refused = Promise.resolve({ ok: false as const, error: { code: 'test-double', message: 'Test double.' } });
  return {
    protocolVersion: 1,
    capabilities: ['agent.send'],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
    dispatch: () => refused,
    request: () => refused,
  };
}

/**
 * jsdom has no layout, so the transcript's box and content heights are stubbed. The
 * browser capture (handoffs/0022-physics-inspector/tools/capture-agent.mjs) checks the
 * real 180 px panel; this pins the keyboard contract that depends on it.
 */
function stubTranscript(content: number, box: number) {
  const scroll = vi.spyOn(HTMLElement.prototype, 'scrollHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('agent__transcript') ? content : 0;
  });
  const client = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('agent__transcript') ? box : 0;
  });
  return () => {
    scroll.mockRestore();
    client.mockRestore();
  };
}

let restore = () => {};
afterEach(() => restore());

describe('Agent transcript in a short panel', () => {
  it('makes the transcript a keyboard stop only while its text overflows', () => {
    restore = stubTranscript(78, 62);
    render(<App resolution={{ kind: 'bridge', bridge: readyAgentBridge() }} />);
    const log = screen.getByRole('log', { name: 'Agent conversation' });
    expect(log.tabIndex).toBe(0);
    expect(log.textContent).toContain('Describe a change');
  });

  it('adds no extra tab stop when everything fits', () => {
    restore = stubTranscript(62, 62);
    render(<App resolution={{ kind: 'bridge', bridge: readyAgentBridge() }} />);
    const log = screen.getByRole('log', { name: 'Agent conversation' });
    expect(log.hasAttribute('tabindex')).toBe(false);
  });
});

describe('Agent that cannot run', () => {
  it('says so in one note, keeps no transcript or composer, and collapses to its status line', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createProviderFixtureBridge('sample', 'signed-in') }} />);
    const agent = document.querySelector<HTMLElement>('[data-region="agent"]')!;
    expect(within(agent).getByText('Agent not ready')).toBeTruthy();
    expect(within(agent).queryByRole('log')).toBeNull();
    expect(within(agent).queryByRole('textbox')).toBeNull();
    const toggle = within(agent).getByRole('button', { name: /^Agent/ });
    expect(toggle.textContent).toContain('Unavailable');
    fireEvent.click(toggle);
    expect(within(agent).queryByText('Agent not ready')).toBeNull();
    fireEvent.click(toggle);
    expect(within(agent).getByText('Agent not ready')).toBeTruthy();
  });
});

/** The agent state the real native bridge conversion reports today. */
function nativeAgent() {
  const read = { project: { id: '00000000000000000000000001', name: 'Agent', scenes: {} }, revision: 1, can_redo: false, history: [], applied: 0, schemas: {}, console: [], viewport_error: null } as unknown as EngineRead;
  return snapshotFromEngine(read).agent;
}

describe('Agent reason wording', () => {
  it.each([
    ['the native bridge reports today', nativeAgent()],
    ['an earlier native build reported', { status: 'unavailable' as const, reason: 'Use the headless agent command for the Phase 0 provider spike.' }],
  ])('presents the "not in the editor" reason %s as its own state, without claiming the agent works', (_name, nativeState) => {
    expect(nativeState.status).toBe('unavailable');
    const snapshot = { ...fixtureSnapshot('sample'), agent: nativeState };
    const bridge: EditorBridge = { ...readyAgentBridge(), capabilities: ['provider.connect'], getSnapshot: () => snapshot };
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const agent = document.querySelector<HTMLElement>('[data-region="agent"]')!;
    expect(agent.textContent).not.toMatch(/headless|Phase 0|spike/);
    expect(within(agent).getByText('Not in the editor yet')).toBeTruthy();
    // No invitation to sign in for an agent that signing in would not enable.
    expect(within(agent).queryByText('Use your ChatGPT account')).toBeNull();
    expect(within(agent).getByRole('button', { name: /^Agent/ }).textContent).toContain('Unavailable');
    expect(within(agent).queryByRole('textbox')).toBeNull();
    // Connecting an account is a real bridge action, so it stays offered.
    expect(within(agent).getByRole('button', { name: /Continue with ChatGPT/ })).toBeTruthy();
  });
});
