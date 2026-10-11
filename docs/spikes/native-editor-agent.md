# Native editor agent binding

The editor owns a bounded native agent session and routes four typed IPC commands
through it: read, start, cancel and approval decision. The session worker uses the
saved Incant account, the same loaded command bus and journal as manual edits, and
bounded in-memory conversation context. No token or authorization URL enters the
webview. No separate project mutation path is introduced.

A short admission mutex serializes turn starts, local account operations and
window shutdown. Account work and network requests run off the event thread and
outside that mutex. Signing out, switching accounts or reconnecting cancels the
current turn before scheduling the account operation. Unchanged focus restoration
does not cancel a turn. Closing the window permanently closes admission and cancels
publication, while an already blocked provider request can finish cooperatively.

The account provider validates a durable, non-secret registration receipt before
requests, after charged responses, and around individual tool/approval boundaries.
A switch away and back invalidates an old receipt. Conversation context is scoped
to that receipt so a new registration cannot send old context. Ordinary token
renewal preserves the receipt. External account checks are cooperative boundaries;
they do not retroactively revoke a tool already admitted by the final check.

Read IPC returns a small session snapshot only when its version changes and the
current authoring revision separately. The bridge refreshes project state only
when that revision changes, instead of serializing the project for streamed text.
Failures use fixed safe codes/messages, not arbitrary provider error strings.
The turn records actual charged allowance and explicitly flags unmeasured provider
reservations; it does not invent a dollar estimate. Provenance uses a new ULID for
every actual run, independently of session IDs which restart when the app opens.

Saved projects bind project-confined imports and a concrete native Play host.
The host cooks authored scripts into a disposable native world. Scenes without
scripts use the editor's trusted no-op fallback. A dedicated thread now retains
the native host across turns in the same account/project scope; later queries
observe the last accepted native world without rerunning it.
Unsaved projects have no file/import/native Play capability. Authored viewport
capture is lazy and real; it is not a captured native gameplay frame.

The integration tests exercise streaming and pending approval, real command-bus
commit/provenance/undo, concurrent manual edits that invalidate a queued agent
patch, account-scoped context, shutdown/admission fences, incremental reads and
actual native-world startup without changing the authoring document/history.
Provider fixtures in these tests are deterministic, not live provider evidence.
Exact validation and independent review are recorded in the accompanying evidence.

This slice does not finish Phase 3. Persistent transcript/settings, configurable
per-project budgets, provider dollar pricing, additional providers, full context
assembly, inline image/diff history, turn-specific revert, skills and the full live
evaluation gate remain outstanding. The Claude chat interface and actual native
window/live-account acceptance are integrated and recorded separately.

## Review checkpoint and retained-owner correction

The initial unpublished checkpoint passed 109 scoped tests and independent
lifecycle/account review, but retained native Play only within one turn. That P2
was corrected by the dedicated owner described in
[persistent-native-agent-host.md](persistent-native-agent-host.md). All 109 prior
checks plus eight owner regressions pass, with independent exact-source review
and four-package all-target Clippy. Failed candidates preserve the accepted world
across turns; account/project scope changes cannot reuse it.

The separately reviewed provider invalidation-hint listener still needs union
integration, and the Claude UI/native live-account acceptance remains separate.
These checks do not claim completed WebView dispatch, appearance or live-provider
evidence.
