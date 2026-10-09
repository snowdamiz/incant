# OpenAI login repair — 2026-10-08

The initial Phase 0 CLI generated `incant:<ULID>` for `ext_agent_host_id`.
OpenAI accepts a persistent UUIDv4 URN, JWK thumbprint URI or did:key, not that
custom format. The original browser failure was never captured, so this is a
confirmed request defect, not a claim about the exact uncaptured server error.
Unregistered legacy hosts migrate once to `urn:uuid:<UUIDv4>`. Existing registered
hosts are never silently replaced. Every sign-in uses fresh state, nonce and PKCE,
with its bound loopback listener already running before the system browser opens.

Account metadata now lives in the OS user configuration directory under
`Incant/accounts.json`, shared by editor, CLI, debug/release builds and worktrees.
The stable namespace is `dev.incant.openai`, independent of executable hash,
project, build directory or bundle version. Tokens never enter the webview,
project files, browser storage, authorization logs, or handoff packets. Returning
sign-in keeps the issued client ID, verified subject and retained ID-token hint.
The system browser is used, not Codex's embedded browser. Pending attempts can be
cancelled and the same browser request reopened. Invalid/stale callback requests
do not consume a pending attempt. Token refresh and account writes are serialized
across processes; metadata writes are atomic and owner-only on Unix.

The native editor restores the saved account on launch and focus. It reports
checking, browser wait, validation, connection and safe errors. Account selection,
adding another registration, cancellation and active-account sign-out are wired
to the same store the CLI agent and evaluation runner use. Editing stays offline.
Actual inference is still the CLI Phase 0 agent; this repair does not claim the
later-phase in-editor conversation loop is complete.

Large OAuth records exceed Windows Credential Manager's 2560-byte blob limit.
Credentials use bounded OS-keychain chunks and an atomically replaced manifest.
Failed writes leave the old record readable; successful writes remove the old
generation. Legacy single-entry credentials remain readable. Temporary errors do
not erase credentials. OS access controls remain in force.

## Verification before the director's real sign-in

- 38 Rust release tests passed; workspace/all-target Clippy passed with warnings
  denied. New tests exercise host migration, metadata restart persistence,
  callback TCP handling, cancellation, stale states, large Unicode credentials,
  and failed refresh-write preservation.
- Strict UI build and 176 UI/bridge tests passed; 4 Python tool tests passed.
- Claude 5.5 returned the account UI in commit 35f3527 (integrated as 815af9d):
  11 real-browser fixture accessibility runs had zero violations. Those are not
  live account or native evidence.
- A 14 KB synthetic credential was saved by one native executable, the executable
  was rebuilt with changed contents, and a new process read the complete record.
  The director approved macOS's Keychain access prompt. The probe then deleted
  the credential and its chunks and verified absence. No real account was used.
  Binary SHA-256 before: 7269e9d7b1eb841e5697e3fad63fc2d65d608716920151c15a1af04b43903041.
  After: 07868824d7153b8fe02b439ca99d298517420cb2f46ad6ba2961af5d0f3caf67.

Keychain can ask for OS access after a development executable changes. This is
separate from OpenAI reauthentication: Incant retains the saved session rather
than creating a registration on each build. A real OpenAI consent, restart check
with that account and live agent request still require the director's first login.
No live provider or Phase 0 approval is implied by synthetic tests.

References: [OpenAI registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[host identifiers](https://developers.openai.com/siwc/token-sharing-open-source),
[account/session lifecycle](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[Microsoft credential limits](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw).
