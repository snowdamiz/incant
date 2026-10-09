import { useShell } from '../shell/ShellContext';

/**
 * Provider connection status. Shows metadata only (provider, method, account
 * label); the contract has no field that could carry a key or token.
 */
export function ProviderChip({ compact = false }: { compact?: boolean }) {
  const { snapshot } = useShell();
  const provider = snapshot?.provider;
  let tone = 'neutral';
  let text = 'Unknown';
  if (!provider) {
    text = 'No engine';
  } else if (provider.status === 'not-connected') {
    text = 'Not connected';
  } else if (provider.status === 'connecting') {
    tone = 'pending';
    text = 'Connecting…';
  } else if (provider.status === 'connected') {
    tone = 'ok';
    text = `${provider.accountLabel} · ${provider.method === 'oauth' ? 'Sign-in' : 'API key'}`;
  } else {
    tone = 'error';
    text = 'Connection error';
  }
  return (
    <span
      className={`provider-chip provider-chip--${tone}${compact ? ' provider-chip--compact' : ''}`}
      title={provider?.status === 'error' ? provider.error.message : undefined}
    >
      <span className="provider-chip__dot" aria-hidden="true" />
      <span className="provider-chip__name">OpenAI</span>
      <span className="visually-hidden">:</span> {text}
    </span>
  );
}
