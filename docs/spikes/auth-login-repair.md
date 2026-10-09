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
Windows/Linux credentials use bounded OS-keychain chunks and an atomically replaced manifest.
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
than creating a registration on each build. Real consent, a restart check and live
requests were still pending at this initial checkpoint. The real-account results
below supersede that limitation. No Phase 0 approval is implied by synthetic tests.

References: [OpenAI registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[host identifiers](https://developers.openai.com/siwc/token-sharing-open-source),
[account/session lifecycle](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[Microsoft credential limits](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw).


## Director-requested macOS storage change

After real sign-in and a successful app restart, the director reported repeated
Keychain password dialogs and explicitly requested a workaround. macOS now uses
OpenAI's documented local credential-file method, in
`~/Library/Application Support/Incant/credentials/` with a private 0700 directory
and atomic owner-only 0600 files. These files are protected by OS file permissions,
not an additional encryption layer. No authentication Keychain backend is linked
on macOS; there is no automatic legacy-Keychain read or migration. Existing
host ID and account/client mappings stay intact. One returning sign-in populates
the new store, without re-registering the client. Old Keychain records are left
unread to avoid more password prompts. Windows/Linux keep their OS stores.

The change is authorized by the director's later explicit instruction and
supersedes PLAN.md's Keychain storage choice for this macOS development setup.
It does not change other applications, OS Keychain access policies or security
settings. No Codex/Claude session is borrowed. Tests reject symlink credential
files and files readable by other users; atomic replacement leaves outside
symlink targets untouched.

A second cross-build test used the new backend: build `local-first` saved a large
synthetic record, build `local-second` read it, and deleted it. All operations
completed without a Keychain dialog or password. Tests also confirm metadata and
credential persistence independently of executable path and owner-only modes.

## Live transport defect discovered during connection testing

The real OAuth account fetched the model catalog and completed a Responses call.
The direct route emitted complete tool calls through `response.output_item.done`,
then sent an empty terminal output array. The previous SSE reader discarded those
items and reported a no-op success. It now retains completed output items until
`response.completed`, preserves their order and refuses an entirely empty result.
Tests ensure interrupted/failed streams cannot execute those buffered tool calls.
A repeated live probe returned the expected `doc_query` call. A scene-edit test
then reached the approval boundary correctly; no edit was claimed before approval.

## Real saved-session verification after the storage change

The director completed returning browser sign-in. CLI status confirmed one active
OAuth account with a credential available from `private-local-file`. A metadata-only
filesystem check confirmed directory mode 0700 and credential mode 0600. No real
credential contents were printed or copied into evidence.

- Explicit `auth refresh` renewed the real session and atomically saved its rotated
  credential without a Keychain dialog.
- The native application was rebuilt after source changes, then relaunched. Its
  account state restored successfully without browser sign-in or a password prompt.
- A fresh CLI agent process used that saved account with `gpt-6-astra`. In one
  approved command-bus transaction it renamed the sole test entity to `Player` and
  set translation to `[0, 1, 0]`, preserving rotation, scale and entity count.
  A subsequent query verified the result. All three tool calls succeeded; usage was
  4,234 input tokens and 302 output tokens across four model steps. The saved
  document records agent provenance and the matching transaction ID.
- The latest local suite has 41 Rust behavior tests and 180 UI/bridge tests passing;
  workspace/all-target Clippy passes with warnings denied.

A subsequent repeated callback test exposed inherited nonblocking sockets on
macOS. Accepted connections now explicitly use bounded blocking reads, tolerate
gaps between packets and require complete HTTP headers. The regression test sends
each request in separated writes so that a partial callback cannot be accepted.

The native app remains connected. This proves macOS persistence, refresh and live
inference; Windows/Linux real-account authentication, API-key inference and live
revocation remain outstanding. The single approved edit does not count as an
unassisted twenty-task evaluation or Phase 0 approval.

The subsequent unassisted [twenty-task run](evidence/live-agent-2026-10-08.json)
completed with the same saved account: 19/20 passed, with no additional browser
login or Keychain prompts. One incomplete provider response is counted as a failed
task. The final app build, including the callback fix, also restored the account
after restart; the native account button displayed the saved profile.
