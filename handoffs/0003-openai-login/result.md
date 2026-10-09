# Result: handoff 0003, ChatGPT account UI

Status: **UI follow-up delivered; native UI not verified.** The account interface
matches the host semantics Astra confirmed in 86aa0da. It is tested and was inspected
in a real browser against explicit fixtures. No native screenshot was taken in this
packet, so nothing here is native evidence. Phase gate approval is not claimed.

Model: Claude Opus 5.5 (`claude-opus-5-5`), run through Claude Code in this worktree
with the director's bypassPermissions authorization. No other model was substituted.

Commits by Claude: 35f3527 holds the first UI. 8bb34e1 is the follow-up on top of
Astra's 86aa0da, which remains attributed to Astra. The Keychain copy correction
below is its own commit.

## Latest correction: no Keychain prompt line (director feedback)

I acknowledge the director's request to stop the repeated Keychain password prompts.
Astra is moving macOS sign-in storage to private local files. The dialog no longer
says "After Incant changes, your computer may ask once to allow access to it." I
chose generic cross-platform wording, so no platform branch is needed. The facts now
read:

- You stay signed in on this computer when you quit, restart or install a new build
  of Incant.
- Your sign-in stays private to your user account on this computer and is never
  saved in your projects.
- Everything except the agent works offline, without an account.

The new wording promises no encryption and names no file modes, Keychain or other
developer detail. A test now fails if the dialog text mentions "may ask",
"keychain", "permission" or "encrypt". `DESIGN.md` was updated to match.

I checked this against browser fixtures only. I did not touch the real account, the
native app or any credentials.

Visual QA: I re-captured all 32 fixture screenshots and inspected the signed-out and
narrow 900x600 signed-in dialogs. The three facts take two lines each, and nothing
clips or wraps badly.

Commands, run from `editor/ui`:

```
npm run typecheck                    # pass, 0 errors
npx vitest run                       # 180 tests passed
npm run build                        # pass
node scripts/account-evidence.mjs    # 32 screenshots, 13 axe runs, 0 violations
```

## Follow-up after 35f3527 (priority revision)

I read Astra's response at the end of native-requests.md first. I left that file
unstaged because the text is Astra's. The shared contract is untouched.

- **Local request type and cast removed.** The UI now uses the shared `HostRequest`
  directly. `ProviderRequest` in `editor/ui/src/bridge/provider.ts` is only a narrowing
  alias of the shared union. It does not duplicate any shape.
- **Signed-out saved accounts carry no active account.** Rows for those accounts offer
  "Sign in", which sends `provider.switch`. No row is marked "In use" while signed out.
  The fallback that marked a lone account as active is gone, and so is the label
  "Selected, signed out".
- **Cancelling an add keeps the earlier account.** While adding an account, the dialog
  says you are still signed in as that account and that cancelling keeps it in use.
  The fixture simulation returns to that account on cancel, as the host does.
- **Failed add or switch while signed in.** The headline is "That didn't finish" and
  the body says you are still signed in as the account. That account stays marked
  "In use". The retry is "Add another account", since the failed action could also
  have been a switch.
- **Error without a signed-in account.** The headline is "Sign-in didn't finish". The
  body drops "Nothing was changed", which the UI cannot know.
- **Chip.** Any error now reads "Needs attention" instead of "Sign-in problem".
- **Persistence copy.** It promises the sign-in survives restarts and new builds. The
  latest correction above superseded the operating-system prompt line added here.
- **Sign-out confirmation.** It now adds that other saved accounts stay listed.
- **Fixtures.** They follow `editor/app/src/provider.rs`. The new `adding-browser` and
  `error-while-signed-in` states were added. Error codes are `provider.auth`, the API
  key label is "Personal API key", and the signed-out message is the host's
  remote-revocation text.

### Native review not done, and why

I did not build or launch my own worktree app. The host's `restore` action runs at
launch and on every window focus. It reads the shared account store in the user
config directory, loads the keychain record, and refreshes the access token. In my
app that would mean reading credentials and possibly refreshing the director's token
while they validate login in the main checkout. It could also raise Keychain prompts
for them and show their personal account. None of these can be ruled out while they
may be signing in. Following the brief's alternative, I continued fixture polish.
**No native state is claimed as tested.**

Astra can capture the signed-out native UI safely by launching with a store that has
no accounts.

## What the user sees

- **Titlebar chip.** It shows a status dot, "ChatGPT", and either the account or the
  state: "Checking…", "Sign in", "Waiting for browser…", "Finishing sign-in…" or
  "Needs attention". It opens the account dialog.
- **Account dialog.** It is modal, focus is trapped, and Escape closes it and returns
  focus to the opener. Escape never cancels a sign-in.
  - **Checking:** a spinner and "Checking for a saved sign-in…".
  - **Signed out:** a white "Continue with ChatGPT" button. Saved accounts are listed
    with "Sign in" buttons, plus "Add another account". Host messages appear verbatim.
  - **Browser wait:** "Open browser again", which relaunches the same pending attempt,
    and "Cancel sign-in". The copy says no code is ever copied into the editor.
  - **Validating:** "Finishing sign-in…" and "Cancel sign-in".
  - **Connected:** the active account is marked "In use", and other accounts offer
    "Use". "Add another account" is available. "Sign out" asks inline first.
  - **Error:** the host's exact message, with its code on a second line.
- **Agent panel.** When signed out, it shows "Continue with ChatGPT". While signing in
  or after an error, a status button reopens the dialog.
- **No secret entry.** No state has any text input. A connection made with an API key
  from the command line shows as connected.

## Branding

I read OpenAI's Sign in with ChatGPT pages: developers.openai.com/siwc/quickstart,
/siwc/website and /siwc/ui-ux-guidelines. They approve "Continue with ChatGPT" on
white or black with the ChatGPT logo. The UI uses the white button. No approved logo
file is published, and I did not redraw OpenAI's mark, so the button stays text-only.

## Changed paths in the follow-up

- `editor/ui/src/bridge/provider.ts` now holds the narrowing alias and
  `signedInLabel`, which replaced `activeLabel`. The chip summary changed.
- `editor/ui/src/bridge/providerFixture.ts` has host-faithful states and simulation.
- `editor/ui/src/shell/ShellContext.tsx` no longer has the cast or the widened type.
- `editor/ui/src/components/AccountDialog.tsx` has the copy and active-account
  changes.
- `editor/ui/src/components/AccountDialog.test.tsx` has six new tests and updates.
- `editor/ui/scripts/account-evidence.mjs` captures the two new states.
- `editor/ui/DESIGN.md` documents the account component.
- `handoffs/0003-openai-login/result.md` and `screenshots/` were updated.

## Commands and results

Run from `editor/ui` after the follow-up:

```
npm run typecheck                    # pass (strict, exactOptionalPropertyTypes)
npx vitest run                       # 7 files, 180 tests passed
npm run build                        # pass
node scripts/account-evidence.mjs    # 32 screenshots, 13 axe runs, 0 violations
```

The new tests cover these behaviors:
- Signed-out saved accounts show no "In use" mark, and "Sign in as" sends a switch.
- While adding, the dialog shows the "still signed in" note, and cancel returns to
  the earlier account.
- A failed add while signed in keeps the account in use.
- The simulation keeps the account on cancel during an add, and signing out leaves
  no active account.
- No signed-out fixture names an active account.

## Screenshots

These are browser fixture captures from headless Chrome at device pixel ratio 2.
They are not native. All are in `handoffs/0003-openai-login/screenshots/`:

- `dialog-<state>.png` and `titlebar-<state>.png` for each state:
  - checking, signed-out, signed-out-saved
  - browser, validating, adding-browser
  - signed-in, signed-in-multi, signed-in-cli-key
  - error, error-while-signed-in
- `window-signed-out.png`, `window-signed-in-multi.png` and
  `window-narrow-signed-in-multi.png`.
- `dialog-signed-in-multi-confirm-signout.png` and `focus-signed-out-initial.png`.
- `agent-{signed-out,browser,error,signed-in}.png` and `flow-agent-continue.png`.

Inspecting the renders led to three fixes. The retention note ran four lines and was
split. A retry label assumed the failed action was an add. Earlier, the error code
broke mid-word.

## Limitations and open questions

1. **Native UI.** It is unverified for the reason above.
2. **Default connect.** With saved accounts but no active one, "Continue with ChatGPT"
   resumes the host's selected registration, which the UI cannot name. Astra: should
   the state expose that selection as a separate nonsecret field, distinct from
   `activeAccount`? The UI could then say which account the button resumes.
3. **Logo.** The ChatGPT logo is missing from the button until an approved asset is
   supplied.
4. **Agent panel error label.** After an error it still says "Sign-in problem…", even
   when an account is still signed in. A follow-up could align it with the chip.
