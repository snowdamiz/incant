import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent, MouseEvent, ReactNode } from 'react';
import type { BridgeError, ProviderAccount, ProviderState } from '../bridge/contract';
import type { ProviderRequest } from '../bridge/provider';
import { savedAccounts, signedInLabel } from '../bridge/provider';
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
    // The sign-out question always takes focus when it opens (a mouse click does not move
    // focus in WebKit), and it lands on the safe answer.
    const question = confirmingSignOut ? dialog.querySelector<HTMLElement>('.account__confirm [data-autofocus]') : null;
    if (question) {
      question.focus();
      return;
    }
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
            <Status id="account-status">No engine connected. Open a project in the Incant editor to sign in.</Status>
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
  const signedIn = signedInLabel(provider);
  const connect = (extra: { add?: boolean } = {}) => void send({ type: 'provider.connect', method: 'oauth', ...extra });
  const signInAvailable = can('provider.connect');
  const showAccounts = accounts.length > 0 && provider.status !== 'connecting' && provider.status !== 'checking';

  // One status line, then the accounts, then one row of actions. The account list is
  // the identity: no headline repeats who is signed in.
  let status: ReactNode = null;
  let actions: ReactNode = null;
  let primary: ReactNode = null;
  let note: ReactNode = null;
  switch (provider.status) {
    case 'checking':
      status = (
        <Status id="account-status" spinner>
          Checking for a saved sign-in…
        </Status>
      );
      break;
    case 'not-connected':
      status = (
        <Status id="account-status">
          {!signInAvailable
            ? 'Signing in is not available from this editor build yet.'
            : accounts.length > 0
              ? 'Sign in to a saved account, or continue in your browser.'
              : 'Sign in to use the agent. ChatGPT opens in your browser.'}
        </Status>
      );
      primary = <ChatGPTButton available={signInAvailable && !busy} autoFocusTarget onClick={() => connect()} />;
      if (accounts.length > 0 && signInAvailable) {
        actions = (
          <ActionButton icon="plus" onClick={() => connect({ add: true })} disabled={busy}>
            Add another account
          </ActionButton>
        );
      }
      break;
    case 'connecting':
      status =
        provider.phase === 'validating' ? (
          <Status id="account-status" spinner>
            Finishing sign-in…
          </Status>
        ) : (
          <Status id="account-status" spinner detail="Approve access in ChatGPT; Incant finishes automatically.">
            Continue in your browser
          </Status>
        );
      if (signedIn) {
        note = (
          <>
            You are still signed in as <strong>{signedIn}</strong>. Cancelling keeps it in use.
          </>
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
      if (provider.method === 'api-key') {
        status = <Status id="account-status">Connected with an API key, set up from the command line, where you can change it.</Status>;
      } else {
        // Named for assistive tech; the list below shows the signed-in account visibly.
        status = <span id="account-status" className="visually-hidden">Signed in as {provider.accountLabel}</span>;
      }
      actions = confirmingSignOut ? null : (
        <>
          {provider.method === 'oauth' ? (
            <ActionButton icon="plus" onClick={() => connect({ add: true })} disabled={busy || !signInAvailable}>
              Add another account
            </ActionButton>
          ) : null}
          {/* Never the initial focus: opening account settings must not put Enter on a
              destructive path. With no autofocus target here, focus starts on Close. */}
          <ActionButton
            icon="signOut"
            tone="danger"
            onClick={() => setConfirmingSignOut(true)}
            disabled={busy || !can('provider.disconnect')}
          >
            Sign out
          </ActionButton>
        </>
      );
      break;
    case 'error':
      if (signedIn) {
        // A failed add or switch. The signed-in account is untouched; rows offer switching, this offers adding.
        status = (
          <Status id="account-status" tone="error">
            That didn't finish.{' '}
            <span>
              You are still signed in as <strong>{signedIn}</strong>.
            </span>
          </Status>
        );
        actions = (
          <ActionButton icon="plus" onClick={() => connect({ add: true })} disabled={busy || !signInAvailable} autoFocusTarget>
            Add another account
          </ActionButton>
        );
      } else {
        status = (
          <Status id="account-status" tone="error">
            Sign-in didn't finish. You can try again now.
          </Status>
        );
        primary = <ChatGPTButton available={signInAvailable && !busy} autoFocusTarget onClick={() => connect()} />;
      }
      break;
  }

  return (
    <>
      {status}
      {provider.status === 'error' ? <ErrorNotice error={provider.error} /> : null}
      {failure ? <ErrorNotice error={failure} /> : null}
      {note ? (
        <p className="account__note" role="note">
          {note}
        </p>
      ) : null}
      {provider.message ? (
        <p className="account__note" role="note">
          {provider.message}
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
      {primary}
      {confirmingSignOut && provider.status === 'connected' ? (
        <div className="account__confirm" role="group" aria-labelledby="signout-question">
          <p id="signout-question">
            Sign out of <strong>{provider.accountLabel}</strong> on this computer? The agent stops working until you sign in
            again.
          </p>
          <div className="account__actions">
            <ActionButton tone="danger-solid" onClick={() => void send({ type: 'provider.disconnect' })} disabled={busy}>
              Sign out
            </ActionButton>
            {/* The safe answer takes focus, so Enter twice cannot sign the user out. */}
            <ActionButton onClick={() => setConfirmingSignOut(false)} autoFocusTarget>
              Keep signed in
            </ActionButton>
          </div>
        </div>
      ) : null}
      {actions ? <div className="account__actions">{actions}</div> : null}
      {provider.status === 'not-connected' ? (
        <p className="account__footnote">Your sign-in stays on this computer and is never saved in your projects.</p>
      ) : null}
    </>
  );
}

/** The account's state in one line, with a spinner while working and an optional detail. */
function Status({
  id,
  children,
  detail,
  spinner = false,
  tone = 'neutral',
}: {
  id: string;
  children: ReactNode;
  detail?: string;
  spinner?: boolean;
  tone?: 'neutral' | 'error';
}) {
  return (
    <div id={id} className={`account__status account__status--${tone}`} role="status" aria-live="polite">
      <p className="account__headline">
        {spinner ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
        <span>{children}</span>
      </p>
      {detail ? <p className="account__detail">{detail}</p> : null}
    </div>
  );
}

function ErrorNotice({ error }: { error: BridgeError }) {
  return (
    <div className="account__error" role="alert">
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
  // Only a signed-in account is marked. Saved, signed-out registrations are listed for sign-in, never as active.
  const activeId = signedInLabel(provider) === null ? undefined : (provider.activeAccount ?? accounts[0]?.id);
  return (
    <ul className="account__list" aria-label="Accounts on this computer">
      {accounts.map((account) => {
        const active = account.id === activeId;
        const verb = activeId === undefined ? 'Sign in' : 'Use';
        return (
          <li key={account.id} className={`account-row${active ? ' is-active' : ''}`} aria-current={active || undefined}>
            <span className="account-row__avatar" aria-hidden="true">
              {account.label.trim().charAt(0).toUpperCase() || '?'}
            </span>
            <span className="account-row__label" title={account.label}>
              {account.label}
            </span>
            {active ? (
              <span className="account-row__state">Signed in</span>
            ) : (
              <button
                type="button"
                className="button button--small account-row__switch"
                aria-label={`${verb} ${verb === 'Sign in' ? 'as ' : ''}${account.label}`}
                aria-disabled={!canSwitch || undefined}
                onClick={() => canSwitch && onSwitch(account.id)}
              >
                {verb}
              </button>
            )}
          </li>
        );
      })}
    </ul>
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
