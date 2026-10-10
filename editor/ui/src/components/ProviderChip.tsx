import { providerSummary } from '../bridge/provider';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';

/**
 * The ChatGPT account in the titlebar, as one compact button: the account's initial
 * in a circle when signed in, otherwise a profile glyph, with a small dot while
 * signing in or after a problem. No visible label; the accessible name and tooltip
 * carry the account and its state ("ChatGPT account, signed in as ..."), and the
 * button opens the account dialog. Metadata only: the contract has no field that
 * could carry a key or token.
 */
export function ProviderChip() {
  const { snapshot, openAccount } = useShell();
  const provider = snapshot?.provider;
  const { tone, text } = providerSummary(provider);
  const initial = provider?.status === 'connected' ? (provider.accountLabel.trim()[0] ?? '?').toUpperCase() : null;
  const name = `ChatGPT account, ${text}`;
  return (
    <button
      type="button"
      className={`account-button account-button--${tone}`}
      aria-haspopup="dialog"
      aria-label={name}
      title={provider?.status === 'error' ? `${name}: ${provider.error.message}` : name}
      onClick={(event) => openAccount(event.currentTarget)}
    >
      {initial ? (
        <span className="account-button__avatar" aria-hidden="true">
          {initial}
        </span>
      ) : (
        <Icon name="profile" size={16} />
      )}
      {tone === 'pending' || tone === 'error' ? <span className="account-button__dot" aria-hidden="true" /> : null}
    </button>
  );
}
