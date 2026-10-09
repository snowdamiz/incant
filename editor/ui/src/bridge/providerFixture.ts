/**
 * SAMPLE FIXTURE. NOT ENGINE DATA, NOT A SIGN-IN.
 *
 * Account states for visual review of the ChatGPT sign-in UI, selected with
 * `?fixture=<variant>&provider=<state>`. The fixture advertises the provider.*
 * capabilities and simulates state changes locally so the flow can be clicked
 * through in a browser. It never opens a browser, contacts OpenAI or stores
 * anything. Labels are invented example addresses. There are deliberately no
 * token, code or URL fields: the contract has none.
 */
import type { AgentState, BridgeResult, BridgeSnapshot, EditorBridge, HostRequest, ProviderAccount, ProviderState } from './contract';
import { UI_PROTOCOL_VERSION } from './contract';
import type { FixtureVariant } from './fixture';
import { fixtureSnapshot } from './fixture';

const ADA: ProviderAccount = { id: 'acct-fixture-1', label: 'ada@example.com' };
const STUDIO: ProviderAccount = { id: 'acct-fixture-2', label: 'studio@example.org' };
const LONG: ProviderAccount = { id: 'acct-fixture-3', label: 'a.very.long.account.name.for.wrapping@example.net' };

/**
 * Mirrors editor/app/src/provider.rs (Astra, 86aa0da): `activeAccount` is present
 * only while an account is signed in, including while another one is being added
 * or after a failed add/switch. Signed-out saved registrations carry no active id.
 */
export const PROVIDER_FIXTURES = {
  checking: { status: 'checking', provider: 'openai' },
  'signed-out': { status: 'not-connected', provider: 'openai' },
  'signed-out-saved': {
    status: 'not-connected',
    provider: 'openai',
    accounts: [ADA, STUDIO],
    message: 'Signed out on this computer. Remote revocation could not be confirmed; you can also disconnect Incant in ChatGPT Settings.',
  },
  browser: { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser' },
  validating: { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'validating' },
  'adding-browser': { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser', accounts: [ADA], activeAccount: ADA.id },
  'signed-in': { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: ADA.label, accounts: [ADA], activeAccount: ADA.id },
  'signed-in-multi': {
    status: 'connected',
    provider: 'openai',
    method: 'oauth',
    accountLabel: STUDIO.label,
    accounts: [ADA, STUDIO, LONG],
    activeAccount: STUDIO.id,
  },
  'signed-in-cli-key': { status: 'connected', provider: 'openai', method: 'api-key', accountLabel: 'Personal API key' },
  error: {
    status: 'error',
    provider: 'openai',
    error: { code: 'provider.auth', message: 'Could not open the default browser: no handler is registered for https links.' },
    accounts: [ADA],
  },
  'error-while-signed-in': {
    status: 'error',
    provider: 'openai',
    error: { code: 'provider.auth', message: 'The sign-in window expired after 5 minutes.' },
    accounts: [ADA, STUDIO],
    activeAccount: ADA.id,
  },
} as const satisfies Record<string, ProviderState>;

export type ProviderFixture = keyof typeof PROVIDER_FIXTURES;
export const PROVIDER_FIXTURE_NAMES = Object.keys(PROVIDER_FIXTURES) as ProviderFixture[];

export function isProviderFixture(value: string): value is ProviderFixture {
  return Object.hasOwn(PROVIDER_FIXTURES, value);
}

const PROVIDER_CAPABILITIES = ['provider.connect', 'provider.cancel', 'provider.disconnect', 'provider.switch'];

const OK: BridgeResult = { ok: true };
const READ_ONLY: BridgeResult = {
  ok: false,
  error: { code: 'fixture.read-only', message: 'The sample fixture is read-only and has no command bus.' },
};

function agentFor(provider: ProviderState): AgentState {
  return provider.status === 'connected'
    ? { status: 'unavailable', reason: 'Agent messaging is not enabled in this build yet.' }
    : { status: 'unavailable', reason: 'Sign in with ChatGPT to use the agent.' };
}

/** Optional fields without explicit `undefined` values (exactOptionalPropertyTypes). */
function extras(accounts: readonly ProviderAccount[] | undefined, activeAccount: string | undefined) {
  return { ...(accounts ? { accounts } : {}), ...(activeAccount !== undefined ? { activeAccount } : {}) };
}

/** The signed-in state for `activeAccount`, or signed out when there is none (as the host does). */
function settled(accounts: readonly ProviderAccount[] | undefined, activeAccount: string | undefined): ProviderState {
  const active = accounts?.find((account) => account.id === activeAccount);
  if (active) return { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: active.label, ...extras(accounts, active.id) };
  return { status: 'not-connected', provider: 'openai', ...extras(accounts, undefined) };
}

/** Pure simulation of host behavior, exported for tests. Never completes a sign-in. */
export function simulateProvider(state: ProviderState, request: HostRequest): ProviderState {
  switch (request.type) {
    case 'provider.connect':
      // A pending attempt keeps any signed-in account; it stays in use until a new one is verified.
      return { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser', ...extras(state.accounts, state.activeAccount) };
    case 'provider.cancel':
      return settled(state.accounts, state.activeAccount);
    case 'provider.disconnect':
      return settled(state.accounts, undefined);
    case 'provider.switch': {
      const account = state.accounts?.find((candidate) => candidate.id === request.accountId);
      return account ? settled(state.accounts, account.id) : state;
    }
    default:
      return state;
  }
}

export function createProviderFixtureBridge(variant: FixtureVariant, provider: ProviderFixture): EditorBridge {
  let snapshot: BridgeSnapshot = withProvider(fixtureSnapshot(variant), PROVIDER_FIXTURES[provider]);
  const listeners = new Set<() => void>();
  return {
    protocolVersion: UI_PROTOCOL_VERSION,
    capabilities: PROVIDER_CAPABILITIES,
    label: `Sample fixture: ${variant}, account ${provider} (simulated)`,
    isFixture: true,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: () => Promise.resolve(READ_ONLY),
    request: (request: HostRequest) => {
      if (!request.type.startsWith('provider.')) return Promise.resolve(READ_ONLY);
      snapshot = withProvider(snapshot, simulateProvider(snapshot.provider, request));
      for (const listener of listeners) listener();
      return Promise.resolve(OK);
    },
  };
}

function withProvider(snapshot: BridgeSnapshot, provider: ProviderState): BridgeSnapshot {
  return { ...snapshot, provider, agent: agentFor(provider) };
}
