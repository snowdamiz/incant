import type { HostRequest, ProviderAccount, ProviderState } from './contract';

/** Host requests that manage the ChatGPT account. Ids and flags only, never a token, code or URL. */
export type ProviderRequest = Extract<HostRequest, { readonly type: `provider.${string}` }>;


export type ProviderTone = 'neutral' | 'pending' | 'ok' | 'error';

/** Saved accounts in host order. Falls back to the connected label when no list is sent. */
export function savedAccounts(state: ProviderState): readonly ProviderAccount[] {
  if (state.accounts && state.accounts.length > 0) return state.accounts;
  if (state.status === 'connected') return [{ id: state.activeAccount ?? '', label: state.accountLabel }];
  return [];
}

/**
 * The signed-in account, if any. The host sends `activeAccount` only while an
 * account is signed in, including during an add attempt or after a failed one.
 * A signed-out saved registration is never reported as active.
 */
export function signedInLabel(state: ProviderState): string | null {
  if (state.status === 'connected') return state.accountLabel;
  if (state.activeAccount === undefined) return null;
  return state.accounts?.find((account) => account.id === state.activeAccount)?.label ?? null;
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
      return { tone: 'error', text: 'Needs attention' };
  }
}
