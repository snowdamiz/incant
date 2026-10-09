# Result: handoff 0003, ChatGPT account UI

Status: **UI delivered for integration; native verification not done.** The account
interface is implemented, tested and inspected in a real browser against explicit
fixtures. No native host in this worktree advertises `provider.*` yet, so nothing here
is native evidence. Phase gate approval is not claimed.

Model: Claude Opus 5.5 (`claude-opus-5-5`), run through Claude Code in this worktree
with the director's bypassPermissions authorization. No other model was substituted.

## Director feedback acknowledged

- OpenAI authentication is the priority. This packet delivers the full sign-in UI
  first; polish items were limited to the account surface.
- Traffic-light y22 and the other 0002 refinements are untouched. No titlebar layout,
  inset or native code was changed. The chip changed from a span to a button with the
  same height and position.
- No credentials were read, no account consent was started, and no external account
  was changed. Fixture labels are invented `example.com/org/net` addresses.

## What the user sees

- **Titlebar chip.** It shows a status dot, "ChatGPT", and the account or state:
  "Checking…", "Sign in", "Waiting for browser…", "Finishing sign-in…", the account
  label, or "Sign-in problem". It opens the account dialog.
- **Account dialog** (modal, focus trapped, Escape closes, focus returns to the opener):
  - **Checking:** a spinner and "Checking for a saved sign-in…".
  - **Signed out:** a white "Continue with ChatGPT" button, plus "Add another account"
    when saved registrations exist. A host `message` shows as an info note.
  - **Browser wait:** "Continue in your browser", with "Open browser again" and
    "Cancel sign-in". The copy says no code is ever copied into the editor.
  - **Validating:** "Finishing sign-in…", with "Cancel sign-in".
  - **Connected:** "Signed in" with the account, a saved-accounts list marking the
    active one "In use", "Use" on the others, "Add another account", and "Sign out".
    Sign-out asks inline first, and Escape backs out of that question.
  - **Error:** the host's exact message with its code on a second line, and
    "Continue with ChatGPT" to retry.
  - **Every state:** "Your sign-in is kept on this computer, so you stay signed in when
    you quit, restart or update Incant. It is never saved in your projects." It also
    says everything except the agent works offline.
- **Agent panel.** When signed out, the composer bar has "Continue with ChatGPT". It
  starts sign-in and opens the dialog. While signing in or after an error, a status
  button reopens the dialog.
- **Rejected requests.** A rejected host request shows inline in the dialog with the
  exact host error. Missing capabilities are explained, and no request is sent.
- **No secret entry.** No input exists for API keys, tokens or codes. A connection
  made with an API key from the command line shows as "Connected with an API key".

## Branding

I read OpenAI's Sign in with ChatGPT pages before branding:
developers.openai.com/siwc/quickstart, /siwc/website and /siwc/ui-ux-guidelines.

- They approve four formats: "Continue with ChatGPT" or "Sign in with ChatGPT", on
  black or white, each with the ChatGPT logo.
- The UI uses "Continue with ChatGPT" on white, which reads as primary on the dark
  shell.
- **Limitation:** the pages publish no logo file or usage spec. I did not redraw
  OpenAI's mark, so the button is text-only until an approved asset is supplied.
- openai.com/brand returned HTTP 403 and could not be read.

## Changed paths

- `editor/ui/src/components/AccountDialog.tsx` is new. It holds the dialog and the
  `ChatGPTButton`.
- `editor/ui/src/components/AccountDialog.test.tsx` is new and has 14 tests.
- `editor/ui/src/bridge/provider.ts` is new. It holds the request shapes that are
  missing from `HostRequest` (see native-requests.md N1) and status helpers.
- `editor/ui/src/bridge/providerFixture.ts` is new. It holds the simulated account
  states, selected with `?fixture=sample&provider=<state>`.
- `editor/ui/src/bridge/resolve.ts` gains labels for the new capabilities, which also
  fixes the typecheck the contract change broke. It also adds the `provider=` parameter.
- `editor/ui/src/shell/ShellContext.tsx` adds `request` (returns the host result),
  widens `ask`, and owns the account dialog's open state and focus return.
- `editor/ui/src/components/ProviderChip.tsx`, `AgentPanel.tsx`, `src/App.tsx`,
  `src/icons/Icon.tsx` (external, plus, signOut), `src/styles/app.css` and
  `src/bridge/fixture.ts` were updated, as were the agent copy and `src/App.test.tsx`.
- `editor/ui/DESIGN.md` documents the account component.
- `editor/ui/scripts/account-evidence.mjs` is new and produces the browser evidence.
- `handoffs/0003-openai-login/native-requests.md`, `result.md` and `screenshots/` are new.

## Commands and results

Run from `editor/ui`:

```
npm run typecheck                      # pass (tsc strict, exactOptionalPropertyTypes)
npx vitest run                         # 7 files, 174 tests passed
npm run build                          # pass; JS 296.6 kB (91.4 kB gzip), CSS 43.6 kB (8.3 kB gzip)
node scripts/account-evidence.mjs      # 28 screenshots, 11 axe runs, 0 violations
```

- **Axe.** Real-browser axe initially found `label-content-name-mismatch` on the
  chip. I fixed it, and the final run is clean, with color contrast included.
- **Unit tests** cover the request each action sends and initial focus. They also
  cover focus recovery when host state changes, Escape not cancelling sign-in, the
  inline sign-out confirmation, verbatim errors, and that no state renders a text
  input. They check that browser storage stays empty, fixtures hold no
  credential- or URL-shaped strings, and plain fixtures still advertise nothing.

## Screenshots

These are browser fixture captures from headless Chrome 155 at device pixel ratio 2.
They are not native. All are in `handoffs/0003-openai-login/screenshots/`, with
metadata in `evidence.json`:

- `dialog-<state>.png` and `titlebar-<state>.png` for checking, signed-out,
  signed-out-saved, browser, validating, signed-in, signed-in-multi,
  signed-in-cli-key and error.
- `window-signed-out.png`, `window-signed-in-multi.png` and
  `window-narrow-signed-in-multi.png` at 900x600.
- `dialog-signed-in-multi-confirm-signout.png` and `focus-signed-out-initial.png`.
- `agent-{signed-out,browser,error,signed-in}.png` and `flow-agent-continue.png`.

Issues found by inspecting the renders and fixed: the error code broke mid-word, and a
redundant "Not now" button sat orphaned under the full-width retry button.

## Not done, and why

- **Native verification.** No native host here advertises `provider.*` or emits
  `incant:provider-changed`. Native captures are listed for Astra in
  native-requests.md N3.
- **Live sign-in.** Out of scope by instruction. The director signs in.
- **ChatGPT logo in the button.** No approved asset is available.
- **Native window screenshots.** None were taken in this packet, so no other Incant
  instance was touched.

## Open questions

1. Please add the provider request shapes to the shared `HostRequest` (N1).
2. Please confirm the host semantics in N2, especially that disconnect signs out only
   the active account and that a repeat connect relaunches the browser.
3. Can the director supply an approved ChatGPT logo asset for the button?
4. After the director signs in, should the agent panel show "Using ChatGPT plan" near
   the composer, as OpenAI's guidelines suggest? That needs `agent.send` to be live
   first.
