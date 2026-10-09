import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeResult, BridgeSnapshot, EditorBridge, HostRequest, ProviderState } from '../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../bridge/fixture';
import type { AnyHostRequest } from '../bridge/provider';
import { PROVIDER_FIXTURES, PROVIDER_FIXTURE_NAMES, simulateProvider } from '../bridge/providerFixture';
import { resolveBridge } from '../bridge/resolve';

const PROVIDER_CAPS = ['provider.connect', 'provider.cancel', 'provider.disconnect', 'provider.switch'];

/**
 * TEST DOUBLE, not a bridge implementation: records host requests and lets the
 * test publish provider states, as the native host does via incant:provider-changed.
 */
function providerBridge(initial: ProviderState, options: { capabilities?: string[]; reply?: BridgeResult } = {}) {
  const requests: AnyHostRequest[] = [];
  const listeners = new Set<() => void>();
  let snapshot: BridgeSnapshot = { ...fixtureSnapshot('sample'), provider: initial };
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities: options.capabilities ?? PROVIDER_CAPS,
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: () => Promise.resolve({ ok: true }),
    request: (request: HostRequest) => {
      requests.push(request as AnyHostRequest);
      return Promise.resolve(options.reply ?? { ok: true });
    },
  };
  const publish = (provider: ProviderState) =>
    act(() => {
      snapshot = { ...snapshot, provider };
      for (const listener of listeners) listener();
    });
  return { bridge, requests, publish };
}

const renderBridge = (bridge: EditorBridge) => render(<App resolution={{ kind: 'bridge', bridge }} />);
const chip = () => screen.getByRole('button', { name: /^ChatGPT / });
const dialog = () => screen.getByRole('dialog', { name: 'ChatGPT account' });
const openDialog = () => {
  fireEvent.click(chip());
  return dialog();
};

async function expectNoAxeViolations(container: HTMLElement) {
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
}

describe('ChatGPT account dialog', () => {
  it('signed out: offers Continue with ChatGPT, explains retention and starts browser sign-in', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['signed-out']);
    renderBridge(bridge);
    expect(chip().textContent).toContain('Sign in');
    const d = openDialog();
    const cta = within(d).getByRole('button', { name: 'Continue with ChatGPT' });
    expect(document.activeElement).toBe(cta);
    expect(within(d).getByText(/stay signed in when you quit, restart or update Incant/)).toBeTruthy();
    expect(within(d).getByText(/works offline, without an account/)).toBeTruthy();
    await expectNoAxeViolations(document.body);
    fireEvent.click(cta);
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.connect', method: 'oauth' }]));
  });

  it('never offers a field for pasting a token, code or key in any state', () => {
    for (const name of PROVIDER_FIXTURE_NAMES) {
      const { bridge } = providerBridge(PROVIDER_FIXTURES[name]);
      const { unmount } = renderBridge(bridge);
      const d = openDialog();
      expect(within(d).queryAllByRole('textbox')).toHaveLength(0);
      expect(d.querySelectorAll('input, textarea')).toHaveLength(0);
      unmount();
    }
  });

  it('browser wait: can reopen the browser or cancel, and closing does not cancel', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES.browser);
    renderBridge(bridge);
    const d = openDialog();
    expect(within(d).getByText('Continue in your browser')).toBeTruthy();
    const again = within(d).getByRole('button', { name: 'Open browser again' });
    expect(document.activeElement).toBe(again);
    fireEvent.click(again);
    await waitFor(() => expect(requests).toHaveLength(1));
    fireEvent.click(within(d).getByRole('button', { name: 'Cancel sign-in' }));
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.connect', method: 'oauth' }, { type: 'provider.cancel' }]));
    fireEvent.keyDown(d, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(chip());
    expect(requests).toHaveLength(2);
  });

  it('follows host state changes and keeps focus inside the dialog', () => {
    const { bridge, publish } = providerBridge(PROVIDER_FIXTURES.browser);
    renderBridge(bridge);
    const d = openDialog();
    publish(PROVIDER_FIXTURES.validating);
    expect(within(d).getByText('Finishing sign-in…')).toBeTruthy();
    expect(document.activeElement).toBe(within(d).getByRole('button', { name: 'Cancel sign-in' }));
    publish(PROVIDER_FIXTURES['signed-in']);
    expect(within(d).getByText('Signed in')).toBeTruthy();
    expect(d.contains(document.activeElement)).toBe(true);
    expect(chip().textContent).toContain('ada@example.com');
  });

  it('marks the active account and switches, adds and signs out on request', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['signed-in-multi']);
    renderBridge(bridge);
    const d = openDialog();
    const active = d.querySelector('[aria-current="true"]')!;
    expect(active.textContent).toContain('studio@example.org');
    expect(active.textContent).toContain('In use');
    fireEvent.click(within(d).getByRole('button', { name: 'Use ada@example.com' }));
    await waitFor(() => expect(d.getAttribute('aria-busy')).toBeNull());
    fireEvent.click(within(d).getByRole('button', { name: 'Add another account' }));
    await waitFor(() =>
      expect(requests).toEqual([
        { type: 'provider.switch', accountId: 'acct-fixture-1' },
        { type: 'provider.connect', method: 'oauth', add: true },
      ]),
    );
    fireEvent.click(within(d).getByRole('button', { name: 'Sign out' }));
    // Sign-out asks first; nothing is sent yet.
    expect(requests).toHaveLength(2);
    const confirm = within(d).getByRole('group', { name: /Sign out of studio@example.org/ });
    expect(document.activeElement).toBe(within(confirm).getByRole('button', { name: 'Sign out' }));
    fireEvent.keyDown(d, { key: 'Escape' });
    expect(within(d).queryByRole('group', { name: /Sign out of/ })).toBeNull();
    fireEvent.click(within(d).getByRole('button', { name: 'Sign out' }));
    fireEvent.click(within(within(d).getByRole('group', { name: /Sign out of/ })).getByRole('button', { name: 'Sign out' }));
    await waitFor(() => expect(requests.at(-1)).toEqual({ type: 'provider.disconnect' }));
    await expectNoAxeViolations(document.body);
  });

  it('error: shows the exact host message and code, and retries', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES.error);
    renderBridge(bridge);
    expect(chip().textContent).toContain('Sign-in problem');
    const d = openDialog();
    const alert = within(d).getByRole('alert');
    expect(alert.textContent).toContain('Could not open the default browser: no handler is registered for https links.');
    expect(alert.textContent).toContain('oauth.browser_launch');
    fireEvent.click(within(d).getByRole('button', { name: 'Continue with ChatGPT' }));
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.connect', method: 'oauth' }]));
  });

  it('shows a rejected request inline with the exact host error', async () => {
    const reply = { ok: false, error: { code: 'oauth.port_in_use', message: 'Port 1455 is already in use.' } } as const;
    const { bridge } = providerBridge(PROVIDER_FIXTURES['signed-out'], { reply });
    renderBridge(bridge);
    const d = openDialog();
    fireEvent.click(within(d).getByRole('button', { name: 'Continue with ChatGPT' }));
    const alert = await within(d).findByRole('alert');
    expect(alert.textContent).toContain('Port 1455 is already in use.');
    expect(alert.textContent).toContain('oauth.port_in_use');
  });

  it('without provider capabilities, explains instead of pretending to sign in', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['signed-out'], { capabilities: [] });
    renderBridge(bridge);
    const d = openDialog();
    const cta = within(d).getByRole('button', { name: 'Continue with ChatGPT' });
    expect(cta.getAttribute('aria-disabled')).toBe('true');
    expect(within(d).getByText(/not available from this editor build/)).toBeTruthy();
    fireEvent.click(cta);
    expect(await within(d).findByRole('alert')).toBeTruthy();
    expect(requests).toHaveLength(0);
  });

  it('agent panel: Continue with ChatGPT starts sign-in and opens the dialog', async () => {
    const { bridge, requests, publish } = providerBridge(PROVIDER_FIXTURES['signed-out']);
    renderBridge(bridge);
    const agent = document.querySelector<HTMLElement>('[data-region="agent"]')!;
    fireEvent.click(within(agent).getByRole('button', { name: 'Continue with ChatGPT' }));
    expect(dialog()).toBeTruthy();
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.connect', method: 'oauth' }]));
    publish(PROVIDER_FIXTURES.browser);
    fireEvent.keyDown(dialog(), { key: 'Escape' });
    expect(within(agent).getByRole('button', { name: 'Signing in…' })).toBeTruthy();
  });

  it('keeps nothing in browser storage', async () => {
    localStorage.clear();
    sessionStorage.clear();
    const { bridge, publish } = providerBridge(PROVIDER_FIXTURES['signed-out']);
    renderBridge(bridge);
    fireEvent.click(within(openDialog()).getByRole('button', { name: 'Continue with ChatGPT' }));
    publish(PROVIDER_FIXTURES['signed-in-multi']);
    await waitFor(() => expect(dialog().textContent).toContain('Signed in'));
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
  });
});

describe('provider fixtures', () => {
  it('resolves ?provider= into a labeled simulated fixture and rejects unknown states', () => {
    const result = resolveBridge(undefined, '?fixture=sample&provider=signed-in');
    if (result.kind !== 'bridge') throw new Error('expected bridge');
    expect(result.bridge.isFixture).toBe(true);
    expect(result.bridge.label).toMatch(/simulated/);
    expect(result.bridge.getSnapshot().provider.status).toBe('connected');
    expect(resolveBridge(undefined, '?fixture=sample&provider=nope')).toEqual({ kind: 'bad-fixture', requested: 'provider=nope' });
  });

  it('plain fixtures still advertise no capabilities', () => {
    expect(createFixtureBridge('sample').capabilities).toEqual([]);
  });

  it('contain no credential- or URL-shaped values', () => {
    const text = JSON.stringify(PROVIDER_FIXTURES);
    for (const pattern of [/sk-[A-Za-z0-9]{8,}/, /bearer/i, /token/i, /secret/i, /https?:\/\//, /code=/i]) {
      expect(text).not.toMatch(pattern);
    }
  });

  it('simulates cancel, switch and add without inventing accounts', () => {
    const multi = PROVIDER_FIXTURES['signed-in-multi'];
    expect(simulateProvider(PROVIDER_FIXTURES.browser, { type: 'provider.cancel' }).status).toBe('not-connected');
    const switched = simulateProvider(multi, { type: 'provider.switch', accountId: 'acct-fixture-1' });
    expect(switched).toMatchObject({ status: 'connected', accountLabel: 'ada@example.com', activeAccount: 'acct-fixture-1' });
    expect(simulateProvider(multi, { type: 'provider.switch', accountId: 'missing' })).toBe(multi);
    const adding = simulateProvider(multi, { type: 'provider.connect', method: 'oauth', add: true });
    expect(adding).toMatchObject({ status: 'connecting', phase: 'browser', activeAccount: multi.activeAccount });
    expect(adding.accounts).toHaveLength(3);
  });
});
