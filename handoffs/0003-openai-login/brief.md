# OpenAI login UI and continued refinement

The director explicitly prioritizes finishing OpenAI authentication now, including
persisting login across app restarts and development rebuilds. Astra is fixing Rust
authentication and shared persistent account management. You own all UI visuals.
Use Claude Opus 5.5 through ACP. The director has authorized bypassPermissions.

Your native review 0002 is integrated, including traffic-light y22. Keep improving
small usability details while Astra implements logic. Immediate deliverable: a
polished, clear account/sign-in interface, integrated into the existing editor.

## Scope and contract

Only edit editor/ui and this handoff packet. Do not edit Rust or editor/bridge;
shared contract changes are already in editor/bridge/contract.ts. Ask Astra via
handoffs/0003-openai-login/native-requests.md for additional contract requirements.

ProviderState now includes checking, not-connected, connecting (optional phase
browser/validating), connected, error, and optional accounts[{id,label}],
activeAccount, message. Tokens and authorization URLs MUST NEVER enter UI state,
browser storage, logs or fixtures. All metadata is safe nonsecret status.

Host requests/capabilities: provider.connect {method:'oauth',accountId?,add?},
provider.cancel, provider.disconnect, provider.switch {accountId}. Default connect
resumes the selected registration if known; add:true explicitly registers another.
Switch chooses a saved account; if signed out it may open browser reauthorization.
Astra launches the system browser (not embedded browser) and publishes updates via
incant:provider-changed. No API-key input UI in this packet; CLI hidden-prompt
support exists. Do not display a fake working API-key button.

## Experience

Provide a visible Continue with ChatGPT action, checking/signed-out/browser-wait/
validation/connected/error states, cancel and retry, account picker/add account,
sign out and a clear indication which account is active. Explain login is retained
on this computer across restarts and rebuilds, without implementation jargon.
If the browser doesn't open, expose a retry action with clear status; never ask a
user to paste tokens/codes into the editor. Keep all editor features offline.
Show exact safe backend error messages. Use accessible keyboard/focus/dialog
behavior consistent with this app. Read official OpenAI Sign in with ChatGPT
UI/UX guidance before branding. Native provider plumbing is in progress: use
explicit fixtures for disconnected/connecting/connected/error visual testing,
then coordinate with Astra for actual native verification. Do not initiate real
account consent or touch credentials. User will login once we have fixed it.

Existing agent.send is not yet advertised by native host; don't fabricate a live
agent transcript. Astra can run live game creation/evaluations through the CLI
with the exact same saved account after the director signs in.

Run UI strict build/tests, inspect actual rendered UI and fix issues. Preserve
native traffic-light alignment and other integrated refinements. Browser fixtures
are not native evidence. Capture only Incant, not other user windows. Never close
other Incant instances. Your older review worktree app can remain alone.

Commit UI changes and result.md with Built-by: claude; list tests/evidence and
remaining issues. No publishing, merging or phase approval. Return soon enough
that Astra can integrate and validate the real login flow while you continue polish.
