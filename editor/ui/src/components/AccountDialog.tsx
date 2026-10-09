import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent, MouseEvent, ReactNode } from 'react';
import type { BridgeError, ProviderAccount, ProviderState } from '../bridge/contract';
import type { ProviderRequest } from '../bridge/provider';
import { activeLabel, savedAccounts } from '../bridge/provider';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';

/**
 * The approved "Continue with ChatGPT" label on a white button, one of the four
 * formats in OpenAI's Sign in with ChatGPT guidance. The guidance also places the
 * ChatGPT logo in the button; no approved logo asset is bundled yet, so the button
 * is text-only rather than carrying a redrawn mark (see handoff 0003 result.md).
 */
export function ChatGPTButton({
  small = false,
  available,
  onClick,
  autoFocusTarget = false,
}: {
  small?: boolean;
  available: boolean;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  autoFocusTarget?: boolean;
}) {
  return (
    <button
      type="button"
      className={`button button--chatgpt${small ? ' button--small' : ''}`}
      aria-disabled={!available || undefined}
      data-autofocus={autoFocusTarget ? '' : undefined}
      onClick={onClick}
    >
      Continue with ChatGPT
    </button>
  );
}

const FOCUSABLE = 'button, [href], [tabindex="0"]';

/**
 * ChatGPT account dialog. Presents only nonsecret status the host publishes:
 * no token, code or authorization URL ever reaches the UI, and the user is never
 * asked to paste one. Sign-in itself happens in the system browser, launched by
 * the host. Escape or the close button dismisses the dialog without cancelling a
 * sign-in in progress; "Cancel sign-in" is a separate, explicit action.
 */
export function AccountDialog() {
  const { snapshot, capabilities, request, closeAccount } = useShell();
  const provider = snapshot?.provider;
  const dialogRef = useRef<HTMLDivElement>(null);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<BridgeError | null>(null);
  const [confirmingSignOut, setConfirmingSignOut] = useState(false);

  const stateKey = provider ? `${provider.status}:${provider.status === 'connecting' ? (provider.phase ?? '') : ''}` : 'none';
  // A new host state supersedes an earlier request failure and any pending sign-out question.
  useEffect(() => {
    setFailure(null);
    setConfirmingSignOut(false);
  }, [stateKey]);

  // Initial focus, and focus recovery when the focused control disappears because the state changed.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (dialog.contains(document.activeElement) && document.activeElement !== dialog) return;
    const target = dialog.querySelector<HTMLElement>('[data-autofocus]') ?? dialog.querySelector<HTMLElement>('.icon-button');
    target?.focus();
  }, [stateKey, confirmingSignOut]);

  const send = async (hostRequest: ProviderRequest) => {
    if (busy) return;
    setBusy(true);
    setFailure(null);
    const result = await request(hostRequest);
    setBusy(false);
    if (!result.ok) setFailure(result.error);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (confirmingSignOut) setConfirmingSignOut(false);
      else closeAccount();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE);
    if (!focusable || focusable.length === 0) return;
    const first = focusable[0]!;
    const last = focusable[focusable.length - 1]!;
    if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="scrim" onMouseDown={closeAccount}>
      <div
        ref={dialogRef}
        className="dialog dialog--account"
        role="dialog"
        aria-modal="true"
        aria-labelledby="account-title"
        aria-describedby="account-status"
        aria-busy={busy || undefined}
        tabIndex={-1}
        onKeyDown={onKeyDown}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="dialog__header">
          <h2 id="account-title" className="dialog__title">
            ChatGPT account
          </h2>
          <button type="button" className="icon-button" aria-label="Close" onClick={closeAccount}>
            <Icon name="close" />
          </button>
        </div>
        <div className="dialog__body account">
          {provider ? (
            <AccountBody
              provider={provider}
              can={(type) => capabilities.has(type)}
              send={send}
              busy={busy}
              failure={failure}
              confirmingSignOut={confirmingSignOut}
              setConfirmingSignOut={setConfirmingSignOut}
            />
          ) : (
            <Status id="account-status" icon="plug" title="No engine connected">
              Open a project from the Incant editor to sign in.
            </Status>
          )}
        </div>
      </div>
    </div>
  );
}

function AccountBody({
  provider,
  can,
  send,
  busy,
  failure,
  confirmingSignOut,
  setConfirmingSignOut,
}: {
  provider: ProviderState;
  can: (type: ProviderRequest['type']) => boolean;
  send: (request: ProviderRequest) => Promise<void>;
  busy: boolean;
  failure: BridgeError | null;
  confirmingSignOut: boolean;
  setConfirmingSignOut: (value: boolean) => void;
}) {
  const accounts = savedAccounts(provider);
  const selected = activeLabel(provider);
  const connect = (extra: { add?: boolean } = {}) => void send({ type: 'provider.connect', method: 'oauth', ...extra });
  const signInAvailable = can('provider.connect');
  const showAccounts = accounts.length > 0 && provider.status !== 'connecting' && provider.status !== 'checking';

  let status: ReactNode;
  let actions: ReactNode = null;
  switch (provider.status) {
    case 'checking':
      status = (
        <Status id="account-status" spinner title="Checking for a saved sign-in…">
          This only takes a moment. The rest of the editor is ready to use.
        </Status>
      );
      break;
    case 'not-connected':
      status = (
        <Status id="account-status" icon="spark" title={selected ? `Sign in again as ${selected}` : 'Use the agent with your ChatGPT account'}>
          {signInAvailable
            ? 'Incant opens ChatGPT in your web browser. Approve access there, then come back here.'
            : 'Signing in is not available from this editor build yet.'}
        </Status>
      );
      actions = (
        <>
          <ChatGPTButton available={signInAvailable && !busy} autoFocusTarget onClick={() => connect()} />
          {accounts.length > 0 && signInAvailable ? (
            <ActionButton icon="plus" onClick={() => connect({ add: true })} disabled={busy}>
              Add another account
            </ActionButton>
          ) : null}
        </>
      );
      break;
    case 'connecting':
      if (provider.phase === 'validating') {
        status = (
          <Status id="account-status" spinner title="Finishing sign-in…">
            Confirming your ChatGPT account. This usually takes a few seconds.
          </Status>
        );
      } else {
        status = (
          <Status id="account-status" spinner title="Continue in your browser">
            ChatGPT sign-in is open in your default browser. Approve access there and Incant will finish automatically. You
            never need to copy a code into the editor.
          </Status>
        );
      }
      actions = (
        <>
          {provider.phase !== 'validating' ? (
            <ActionButton icon="external" onClick={() => connect()} disabled={busy || !signInAvailable} autoFocusTarget>
              Open browser again
            </ActionButton>
          ) : null}
          <ActionButton
            onClick={() => void send({ type: 'provider.cancel' })}
            disabled={busy || !can('provider.cancel')}
            autoFocusTarget={provider.phase === 'validating'}
          >
            Cancel sign-in
          </ActionButton>
        </>
      );
      break;
    case 'connected':
      status =
        provider.method === 'api-key' ? (
          <Status id="account-status" icon="check" tone="ok" title="Connected with an API key">
            This key was set up from the command line, where you can also change it.
          </Status>
        ) : (
          <Status id="account-status" icon="check" tone="ok" title="Signed in">
            The agent uses <strong>{provider.accountLabel}</strong>.
          </Status>
        );
      actions = confirmingSignOut ? null : (
        <>
          {provider.method === 'oauth' ? (
            <ActionButton icon="plus" onClick={() => connect({ add: true })} disabled={busy || !signInAvailable}>
              Add another account
            </ActionButton>
          ) : null}
          <ActionButton
            icon="signOut"
            tone="danger"
            onClick={() => setConfirmingSignOut(true)}
            disabled={busy || !can('provider.disconnect')}
            autoFocusTarget
          >
            Sign out
          </ActionButton>
        </>
      );
      break;
    case 'error':
      status = (
        <Status id="account-status" icon="error" tone="error" title="Sign-in didn't finish">
          Nothing was changed. You can try again now.
        </Status>
      );
      actions = (
        <>
          <ChatGPTButton available={signInAvailable && !busy} autoFocusTarget onClick={() => connect()} />
        </>
      );
      break;
  }

  return (
    <>
      {status}
      {provider.status === 'error' ? <ErrorNotice error={provider.error} /> : null}
      {failure ? <ErrorNotice error={failure} /> : null}
      {provider.message ? (
        <p className="account__message" role="note">
          <Icon name="info" size={14} />
          <span>{provider.message}</span>
        </p>
      ) : null}
      {showAccounts ? (
        <AccountList
          provider={provider}
          accounts={accounts}
          canSwitch={can('provider.switch') && !busy}
          onSwitch={(accountId) => void send({ type: 'provider.switch', accountId })}
        />
      ) : null}
      {confirmingSignOut && provider.status === 'connected' ? (
        <div className="account__confirm" role="group" aria-labelledby="signout-question">
          <p id="signout-question">
            Sign out of <strong>{provider.accountLabel}</strong> on this computer? The agent stops working until you sign in
            again in your browser.
          </p>
          <div className="account__actions">
            <ActionButton tone="danger-solid" onClick={() => void send({ type: 'provider.disconnect' })} disabled={busy} autoFocusTarget>
              Sign out
            </ActionButton>
            <ActionButton onClick={() => setConfirmingSignOut(false)}>Keep signed in</ActionButton>
          </div>
        </div>
      ) : null}
      {actions ? <div className="account__actions">{actions}</div> : null}
      <ul className="account__facts">
        <li>
          <Icon name="lock" size={14} />
          <span>
            Your sign-in is kept on this computer, so you stay signed in when you quit, restart or update Incant. It is never
            saved in your projects.
          </span>
        </li>
        <li>
          <Icon name="plug" size={14} />
          <span>Everything except the agent works offline, without an account.</span>
        </li>
      </ul>
    </>
  );
}

function Status({
  id,
  title,
  children,
  icon,
  spinner = false,
  tone = 'neutral',
}: {
  id: string;
  title: string;
  children: ReactNode;
  icon?: 'spark' | 'check' | 'error' | 'plug';
  spinner?: boolean;
  tone?: 'neutral' | 'ok' | 'error';
}) {
  return (
    <div id={id} className={`account__status account__status--${tone}`} role="status" aria-live="polite">
      <span className="account__badge" aria-hidden="true">
        {spinner ? <span className="spinner" /> : icon ? <Icon name={icon} size={18} /> : null}
      </span>
      <div>
        <p className="account__headline">{title}</p>
        <p className="account__body">{children}</p>
      </div>
    </div>
  );
}

function ErrorNotice({ error }: { error: BridgeError }) {
  return (
    <div className="notice notice--error account__error" role="alert">
      <Icon name="error" size={14} />
      <span className="account__error-text">
        <span>{error.message}</span>
        <span className="account__error-code mono">{error.code}</span>
      </span>
    </div>
  );
}

function AccountList({
  provider,
  accounts,
  canSwitch,
  onSwitch,
}: {
  provider: ProviderState;
  accounts: readonly ProviderAccount[];
  canSwitch: boolean;
  onSwitch: (id: string) => void;
}) {
  const signedIn = provider.status === 'connected';
  const activeId = provider.activeAccount ?? (accounts.length === 1 ? accounts[0]!.id : undefined);
  return (
    <section className="account__list" aria-labelledby="accounts-title">
      <h3 id="accounts-title" className="account__list-title">
        Accounts on this computer
      </h3>
      <ul>
        {accounts.map((account) => {
          const active = account.id === activeId;
          return (
            <li key={account.id} className={`account-row${active ? ' is-active' : ''}`} aria-current={active || undefined}>
              <span className="account-row__avatar" aria-hidden="true">
                {account.label.trim().charAt(0).toUpperCase() || '?'}
              </span>
              <span className="account-row__label" title={account.label}>
                {account.label}
              </span>
              {active ? (
                <span className={`account-row__state${signedIn ? ' is-signed-in' : ''}`}>
                  {signedIn ? (
                    <>
                      <Icon name="check" size={12} />
                      In use
                    </>
                  ) : (
                    'Selected, signed out'
                  )}
                </span>
              ) : (
                <button
                  type="button"
                  className="button button--small account-row__switch"
                  aria-label={`Use ${account.label}`}
                  aria-disabled={!canSwitch || undefined}
                  onClick={() => canSwitch && onSwitch(account.id)}
                >
                  Use
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function ActionButton({
  children,
  onClick,
  icon,
  tone = 'default',
  disabled = false,
  autoFocusTarget = false,
}: {
  children: ReactNode;
  onClick: () => void;
  icon?: 'plus' | 'external' | 'signOut';
  tone?: 'default' | 'danger' | 'danger-solid';
  disabled?: boolean;
  autoFocusTarget?: boolean;
}) {
  return (
    <button
      type="button"
      className={`button${tone === 'danger' ? ' button--danger' : tone === 'danger-solid' ? ' button--danger-solid' : ''}`}
      aria-disabled={disabled || undefined}
      data-autofocus={autoFocusTarget ? '' : undefined}
      onClick={() => !disabled && onClick()}
    >
      {icon ? <Icon name={icon} size={14} /> : null}
      {children}
    </button>
  );
}
