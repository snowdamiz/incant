import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeResult, BridgeSnapshot, EditorBridge, HostRequest, ProviderState } from '../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../bridge/fixture';
import { PROVIDER_FIXTURES, PROVIDER_FIXTURE_NAMES, simulateProvider } from '../bridge/providerFixture';
import { resolveBridge } from '../bridge/resolve';

const PROVIDER_CAPS = ['provider.connect', 'provider.cancel', 'provider.disconnect', 'provider.switch'];

/**
 * TEST DOUBLE, not a bridge implementation: records host requests and lets the
 * test publish provider states, as the native host does via incant:provider-changed.
 */
function providerBridge(initial: ProviderState, options: { capabilities?: string[]; reply?: BridgeResult } = {}) {
  const requests: HostRequest[] = [];
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
      requests.push(request);
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
    expect(within(d).getByText(/stay signed in on this computer when you quit, restart or install a new build of Incant/)).toBeTruthy();
    expect(within(d).getByText(/private to your user account on this computer and is never saved in your projects/)).toBeTruthy();
    // No storage implementation details or OS permission prompts are promised or described.
    expect(d.textContent).not.toMatch(/may ask|keychain|permission|encrypt/i);
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

  it('signed in: keyboard focus never starts on a destructive action', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['signed-in']);
    renderBridge(bridge);
    // Enter on a focused button activates it; model that as a click on whatever holds focus.
    const pressEnter = () => fireEvent.click(document.activeElement as HTMLElement);

    let d = openDialog();
    expect(document.activeElement).toBe(within(d).getByRole('button', { name: 'Close' }));
    pressEnter();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(chip());

    // Even after choosing Sign out, the confirmation starts on the safe answer.
    d = openDialog();
    fireEvent.click(within(d).getByRole('button', { name: 'Sign out' }));
    const confirm = within(d).getByRole('group', { name: /Sign out of ada@example.com/ });
    expect(document.activeElement).toBe(within(confirm).getByRole('button', { name: 'Keep signed in' }));
    pressEnter();
    expect(within(d).queryByRole('group', { name: /Sign out of/ })).toBeNull();
    expect(d.contains(document.activeElement)).toBe(true);
    expect(document.activeElement?.textContent).not.toMatch(/Sign out/);
    await Promise.resolve();
    expect(requests).toEqual([]);

    // Focus stays trapped: Tab from the last control wraps to the first.
    const focusable = [...d.querySelectorAll<HTMLElement>('button:not([disabled]), [href], [tabindex]:not([tabindex="-1"])')];
    focusable.at(-1)!.focus();
    fireEvent.keyDown(d, { key: 'Tab' });
    expect(document.activeElement).toBe(focusable[0]);
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
    expect(document.activeElement).toBe(within(confirm).getByRole('button', { name: 'Keep signed in' }));
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
    expect(chip().textContent).toContain('Needs attention');
    const d = openDialog();
    const alert = within(d).getByRole('alert');
    expect(alert.textContent).toContain('Could not open the default browser: no handler is registered for https links.');
    expect(alert.textContent).toContain('provider.auth');
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

  it('simulates the host: cancelling an add keeps the earlier account, signing out leaves no active account', () => {
    const multi = PROVIDER_FIXTURES['signed-in-multi'];
    expect(simulateProvider(PROVIDER_FIXTURES.browser, { type: 'provider.cancel' })).toEqual({ status: 'not-connected', provider: 'openai' });
    const adding = simulateProvider(multi, { type: 'provider.connect', method: 'oauth', add: true });
    expect(adding).toMatchObject({ status: 'connecting', phase: 'browser', activeAccount: multi.activeAccount });
    expect(simulateProvider(adding, { type: 'provider.cancel' })).toMatchObject({ status: 'connected', accountLabel: 'studio@example.org' });
    const signedOut = simulateProvider(multi, { type: 'provider.disconnect' });
    expect(signedOut.status).toBe('not-connected');
    expect(signedOut.activeAccount).toBeUndefined();
    expect(signedOut.accounts).toHaveLength(3);
    const switched = simulateProvider(signedOut, { type: 'provider.switch', accountId: 'acct-fixture-1' });
    expect(switched).toMatchObject({ status: 'connected', accountLabel: 'ada@example.com', activeAccount: 'acct-fixture-1' });
    expect(simulateProvider(multi, { type: 'provider.switch', accountId: 'missing' })).toBe(multi);
  });

  it('signed-out fixtures never name an active account', () => {
    for (const state of Object.values(PROVIDER_FIXTURES)) {
      if (state.status === 'not-connected' || state.status === 'checking') expect('activeAccount' in state).toBe(false);
    }
  });
});

describe('account semantics in the dialog', () => {
  it('signed out with saved accounts: none is marked in use, and each offers sign-in', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['signed-out-saved']);
    renderBridge(bridge);
    const d = openDialog();
    expect(d.querySelector('[aria-current]')).toBeNull();
    expect(within(d).queryByText('In use')).toBeNull();
    expect(within(d).getByText(/Remote revocation could not be confirmed/)).toBeTruthy();
    fireEvent.click(within(d).getByRole('button', { name: 'Sign in as studio@example.org' }));
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.switch', accountId: 'acct-fixture-2' }]));
  });

  it('adding while signed in: says the current account stays in use, and cancel keeps it', async () => {
    const { bridge, requests, publish } = providerBridge(PROVIDER_FIXTURES['adding-browser']);
    renderBridge(bridge);
    const d = openDialog();
    expect(within(d).getByText(/You are still signed in as/).textContent).toContain('ada@example.com');
    expect(within(d).getByText(/Cancelling keeps it in use/)).toBeTruthy();
    fireEvent.click(within(d).getByRole('button', { name: 'Cancel sign-in' }));
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.cancel' }]));
    publish(simulateProvider(PROVIDER_FIXTURES['adding-browser'], { type: 'provider.cancel' }));
    expect(within(d).getByText('Signed in')).toBeTruthy();
    expect(d.querySelector('[aria-current="true"]')!.textContent).toContain('ada@example.com');
  });

  it('a failed add or switch while signed in keeps the account in use', async () => {
    const { bridge, requests } = providerBridge(PROVIDER_FIXTURES['error-while-signed-in']);
    renderBridge(bridge);
    const d = openDialog();
    expect(within(d).getByText("That didn't finish")).toBeTruthy();
    expect(within(d).getByText(/You are still signed in as/).textContent).toContain('ada@example.com');
    expect(within(d).getByRole('alert').textContent).toContain('The sign-in window expired after 5 minutes.');
    expect(d.querySelector('[aria-current="true"]')!.textContent).toContain('In use');
    const retry = within(d).getByRole('button', { name: 'Add another account' });
    expect(document.activeElement).toBe(retry);
    fireEvent.click(retry);
    await waitFor(() => expect(requests).toEqual([{ type: 'provider.connect', method: 'oauth', add: true }]));
  });
});
