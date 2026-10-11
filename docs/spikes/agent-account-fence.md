# Agent account selection fence

Account-backed provider turns now retain a non-secret selection receipt. The
agent checks it before a request, after charging a completed response, before
each tool, and after any approval wait before dispatch. An account changed by
another Incant process can no longer pass those boundaries merely because its
previous network response arrived successfully. A rejected response keeps its
actual usage charge; no stale tool batch is executed or recorded as successful.

## Receipt and credential lifecycle

`AccountStore::selection()` returns an opaque `AccountSelection` with equality
over the host, durable generation and selected OAuth registration identity or
API-key selection. `AccountStore::provider` captures the receipt from that same
locked store. The selected model, credentials and tokens are not receipt data.
The editor may use this receipt to scope its separate conversation context,
including same-registration reconnection and switching A to B to A.

Account metadata now stores a checked `selection_generation`. Older files load
with generation zero; ordinary reads do not rewrite a valid file. Each metadata
save increments the generation, and overflow refuses the write. Selection
identity is checked as well, so a mismatched or missing registration never falls
through to a saved API key.

`invalidate_selection()` persists a new generation before a credential is
replaced or deleted. It is called by disconnect, OAuth sign-in completion and
the headless API-key replacement path. Failed invalidation prevents credential
mutation. If credential cleanup later fails, the selected registration remains
available for retry but receipts from earlier turns are already invalid.
Ordinary access-token renewal does not invalidate selection: the common
`save_record`, `access_token_inner` and refresh path remain unchanged. OAuth
sign-in acquires the account store after its browser/token exchange, so this
change introduces no nested acquisition of the account lock.

## Provider and agent boundaries

`Provider::check_active` defaults to success for providers without an account
store. Account-backed `OpenAiProvider` implementations reopen only account
metadata and compare the captured receipt. These checks never load keychain
credentials, refresh a token or call the network. Existing token resolution
still reloads credentials before each HTTP attempt and now validates the same
receipt before doing so.

The agent invokes checks outside the command-bus lock. A response is charged
before its fence is evaluated. The post-approval fence is outside the ordinary
tool-error conversion, so a stale account ends the turn rather than becoming a
tool error the model can retry. Automatic mode also checks each tool in a batch.
Earlier committed tools retain their ordinary provenance and Undo history;
rejecting a later tool does not claim that those earlier edits were rolled back.

Errors returned by `provider.complete` retain an existing typed `Response`
usage record. Other failed requests retain the same conservative charge and
become `Response { usage: None }` with a fixed safe reason. This distinguishes
unknown request cost from a metadata fence error after known response usage.
The editor's separately integrated controller can therefore mark only the
unmeasured request charge uncertain. The cancellation flag stays authoritative;
normalization neither clears it nor reports an interrupted request as success.

These are cooperative admission boundaries. The account guard is released
before dispatch, so this is not an atomic cross-process account/project
transaction. A tool already admitted by its final fence may finish while an
external account change occurs. Local editor cancellation continues to use the
existing cancellation check after acquiring project access. The editor must
separately serialize turn admission, account operations and shutdown; this
patch supplies the provider/metadata portion, not that native binding.

## Local verification

The local agent suite passes 73 tests, including eleven new account-fence cases.
Agent and headless all-target Clippy with denied warnings, formatting and diff
checks pass. The new tests use private metadata files, real command buses,
deterministic provider doubles and controlled threads/channels. They never
open the user's credential store or contact a provider.

Coverage includes legacy metadata, stable read receipts, A-to-B-to-A changes,
same-registration reconnect, registration identity mismatch, API-key receipt
invalidation, checked generation overflow, failed credential cleanup, failed
invalidation preventing cleanup, and metadata checks that cannot invoke a token
source. Observable agent tests refuse stale requests without billing a request
that was not made; charge a blocked stale response without applying its batch;
refuse account changes during approval; and preserve exactly one earlier
undoable edit when automatic batch authorization changes before the next tool.
The failed-request regression distinguishes a redacted transport failure,
provider interruption with its cancellation flag, and known incomplete-response
usage; conservative reservations never masquerade as the known-usage case.

Source and log identities are in
[agent-account-fence.json](evidence/agent-account-fence.json). This is local
correctness evidence, not a live-provider, cross-platform or phase gate. Existing
account metadata locking remains the coordination boundary between cooperating
Incant binaries; arbitrary external file replacement or older writers that do
not maintain the generation are outside this receipt protocol.

Independent read-only review accepted all seven frozen Rust source hashes,
including the final failed-request accounting classification. Its
[report](evidence/agent-account-fence-review.md) and SHA-256 are retained in the
evidence record.
