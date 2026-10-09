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
import type { AnyHostRequest } from './provider';

const ADA: ProviderAccount = { id: 'acct-fixture-1', label: 'ada@example.com' };
const STUDIO: ProviderAccount = { id: 'acct-fixture-2', label: 'studio@example.org' };
const LONG: ProviderAccount = { id: 'acct-fixture-3', label: 'a.very.long.account.name.for.wrapping@example.net' };

export const PROVIDER_FIXTURES = {
  checking: { status: 'checking', provider: 'openai' },
  'signed-out': { status: 'not-connected', provider: 'openai' },
  'signed-out-saved': {
    status: 'not-connected',
    provider: 'openai',
    accounts: [ADA, STUDIO],
    activeAccount: ADA.id,
    message: 'Your saved sign-in for ada@example.com has expired. Continue to sign in again.',
  },
  browser: { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'browser' },
  validating: { status: 'connecting', provider: 'openai', method: 'oauth', phase: 'validating' },
  'signed-in': { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: ADA.label, accounts: [ADA], activeAccount: ADA.id },
  'signed-in-multi': {
    status: 'connected',
    provider: 'openai',
    method: 'oauth',
    accountLabel: STUDIO.label,
    accounts: [ADA, STUDIO, LONG],
    activeAccount: STUDIO.id,
  },
  'signed-in-cli-key': { status: 'connected', provider: 'openai', method: 'api-key', accountLabel: 'Command-line API key' },
  error: {
    status: 'error',
    provider: 'openai',
    error: { code: 'oauth.browser_launch', message: 'Could not open the default browser: no handler is registered for https links.' },
    accounts: [ADA],
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

/** Pure simulation of host behavior, exported for tests. */
export function simulateProvider(state: ProviderState, request: AnyHostRequest): ProviderState {
  const keep = extras(state.accounts, state.activeAccount);
  switch (request.type) {
    case 'provider.connect':
      return {
        status: 'connecting',
        provider: 'openai',
        method: 'oauth',
        phase: 'browser',
        ...extras(state.accounts, 'add' in request && request.add ? state.activeAccount : (('accountId' in request ? request.accountId : undefined) ?? state.activeAccount)),
      };
    case 'provider.cancel':
    case 'provider.disconnect':
      return { status: 'not-connected', provider: 'openai', ...keep };
    case 'provider.switch': {
      const account = state.accounts?.find((candidate) => candidate.id === request.accountId);
      if (!account) return state;
      return { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: account.label, ...extras(state.accounts, account.id) };
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
      const any = request as AnyHostRequest;
      if (!any.type.startsWith('provider.')) return Promise.resolve(READ_ONLY);
      snapshot = withProvider(snapshot, simulateProvider(snapshot.provider, any));
      for (const listener of listeners) listener();
      return Promise.resolve(OK);
    },
  };
}

function withProvider(snapshot: BridgeSnapshot, provider: ProviderState): BridgeSnapshot {
  return { ...snapshot, provider, agent: agentFor(provider) };
}
