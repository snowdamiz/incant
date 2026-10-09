import { providerSummary } from '../bridge/provider';
import { useShell } from '../shell/ShellContext';

/**
 * ChatGPT account status in the titlebar; opens the account dialog. Shows
 * metadata only (status and account label). The contract has no field that
 * could carry a key or token.
 */
export function ProviderChip() {
  const { snapshot, openAccount } = useShell();
  const provider = snapshot?.provider;
  const { tone, text } = providerSummary(provider);
  return (
    <button
      type="button"
      className={`provider-chip provider-chip--${tone}`}
      aria-haspopup="dialog"
      title={provider?.status === 'error' ? provider.error.message : 'ChatGPT account'}
      onClick={(event) => openAccount(event.currentTarget)}
    >
      <span className="provider-chip__dot" aria-hidden="true" />
      <span className="provider-chip__name">ChatGPT</span>{' '}
      <span className="provider-chip__text">{text}</span>
    </button>
  );
}
