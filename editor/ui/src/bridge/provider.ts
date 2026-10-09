import type { HostRequest, ProviderAccount, ProviderState } from './contract';

/**
 * Provider account requests described in handoff 0003. The shared HostRequest
 * union in editor/bridge/contract.ts does not list these shapes yet (it has only
 * `provider.connect {method, phase?}`); see handoffs/0003-openai-login/native-requests.md.
 * Every field is nonsecret: ids and flags only, never a token, code or URL.
 */
export type ProviderRequest =
  | {
      readonly type: 'provider.connect';
      readonly method: 'oauth';
      /** Resume or reauthorize this saved registration. Omitted: the host's selected one. */
      readonly accountId?: string;
      /** True registers another account instead of resuming the selected one. */
      readonly add?: boolean;
    }
  | { readonly type: 'provider.cancel' }
  | { readonly type: 'provider.disconnect' }
  | { readonly type: 'provider.switch'; readonly accountId: string };

export type AnyHostRequest = HostRequest | ProviderRequest;

export type ProviderTone = 'neutral' | 'pending' | 'ok' | 'error';

/** Saved accounts in host order. Falls back to the connected label when no list is sent. */
export function savedAccounts(state: ProviderState): readonly ProviderAccount[] {
  if (state.accounts && state.accounts.length > 0) return state.accounts;
  if (state.status === 'connected') return [{ id: state.activeAccount ?? '', label: state.accountLabel }];
  return [];
}

/** Label of the selected account, if the host told us which one it is. */
export function activeLabel(state: ProviderState): string | null {
  if (state.status === 'connected') return state.accountLabel;
  const match = state.accounts?.find((account) => account.id === state.activeAccount);
  return match ? match.label : null;
}

/** Short status for the titlebar chip. */
export function providerSummary(state: ProviderState | undefined): { tone: ProviderTone; text: string } {
  if (!state) return { tone: 'neutral', text: 'No engine' };
  switch (state.status) {
    case 'checking':
      return { tone: 'pending', text: 'Checking…' };
    case 'not-connected':
      return { tone: 'neutral', text: 'Sign in' };
    case 'connecting':
      return { tone: 'pending', text: state.phase === 'validating' ? 'Finishing sign-in…' : 'Waiting for browser…' };
    case 'connected':
      return { tone: 'ok', text: state.accountLabel };
    case 'error':
      return { tone: 'error', text: 'Sign-in problem' };
  }
}
