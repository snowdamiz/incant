# Native editor-agent bridge transport

2026-10-11. This nonvisual TypeScript slice was implemented by the director-authorized
`gpt-6.1-sol` agent with Extra High (`xhigh`) reasoning, classified as `Built-by:
astra` under repository ownership conventions. Its baseline is editor native Play
commit `3910a8569ec65c409b9dc82644ea9eec882a5735`. It preserves the existing native
Play transport and changes no UI components, layouts, styling or screenshots.

The shared contract mirrors `incant_agent::session` metadata, including explicit
turn/approval IDs, pending JSON arguments, bounded report rows, terminal failures
and accounting uncertainty. `agentSession` and `agentError` are optional for
older/fixture hosts. Existing `agent.send {text}` requests remain valid and default
to `gpt-6-astra` with destructive approval; optional model/approval fields and
explicit `agent.cancel`/`agent.approve` requests map to native IPC. A successful
start returns `agentTurnId`, which is a start receipt rather than a completion
claim.

The optional `EditorBridge.refreshAgent()` method provides explicit read-only
bootstrap/recovery while older/fixture hosts can omit it. The bridge polls active workers at 250 ms
with coalesced reads. It sends the last accepted `afterVersion`; native `session:
null` means unchanged state and retains snapshot identity. Session versions and
request generations prevent old replies/failures from replacing newer state.
Account transitions also fence provider reads and require fresh session metadata
before approval or send. Duplicate starts are blocked while awaiting the start
receipt, a known turn snapshot or the native worker's actual stop. An existing
worker remains cancellable during provider/project/transport failures, and its
partial, stopping, cancelled or failed transcript is retained.

Streaming metadata never requests a full project by itself. A native
`project_revision` newer than the currently published engine revision triggers
one engine refresh; an already published newer revision covers an older poll.
Normal engine change events continue to handle project loading/replacement.
Unknown or malformed session responses fail visibly without publishing their
payload. Unknown thrown values produce a fixed `agent.transport` message and
are never logged. Structured native `agent.*` errors retain the host's fixed,
credential-safe messages. Approval validation mirrors native depth 32, node
count 4096, payload 2 MiB and per-turn total 8 MiB limits; typed counters must be
JavaScript-safe integers. No provider keys, tokens or configuration fields are
added to the webview contract.

Validation on the frozen TypeScript inputs:

- 29 focused native-agent behavior tests pass. They cover streaming/unchanged
  versions, polling/disposal, exact approval/cancel IDs, duplicate starts and
  decisions, stale/out-of-order replies and failures, account transitions,
  actual project revision refresh, terminal/provider/setup failures, accounting
  metadata, malformed/bounded responses and unavailable requests without IPC.
- All 574 UI/bridge tests pass, including existing Play, capability resolution,
  standalone fixture and AgentPanel behavior.
- `npm run typecheck --workspace editor/ui` passes.
- `npm run build --workspace editor/ui` passes strict TypeScript and Vite build.
- `node tools/build_bridge.mjs --check` and `git diff --check` pass.

These are transport behavior tests with mock IPC. They are not native IPC or
live-provider evidence and do not approve a phase or performance gate. The root
integration owns the Rust IPC binding, real provider checks, current `STATUS.md`
and combined validation. Claude Opus 5.5 at Max owns the complete chat UI and
rendered review. No publication, PR or merge occurs from this subtask.

Independent review of `c4e4471` reproduced a stale-error race after the request's
automatic recovery read: send and cancel/approval error handlers checked the
generation before awaiting recovery but could publish the old error after a
new account had already refreshed successfully. Both handlers now check again
after recovery. Three deferred-response tests reproduced the failure on the
reviewed code and pass with this correction, while the original request still
returns its own failure receipt. Final scoped validation passes 32 agent tests
and 88 combined transport/resolve/AgentPanel tests, strict TypeScript, generated
bridge equivalence and diff checks. The earlier full 574-test/build result above
precedes this reviewed correction; combined native integration remains separate.

The provider lifecycle follow-up treats `incant:provider-changed` as an invalidation
hint. Its payload is never applied, including null hints and delayed connecting
snapshots from the native browser opener. Every hint re-reads `provider_read`;
read sequence and account/action generation checks prevent an overlapping old
reply or failure from replacing a newer account publication. Native reads receive
a basic safe-metadata shape check, preserving the prior-account display fields
that actual connecting states retain. Malformed or failed reads publish a fixed
`provider.transport` error without exception text. A failed native action also
re-reads current state for recovery while preserving that action's failure receipt.
Event and bootstrap promises handle failures without trusting delayed payloads.
A hint invalidates send/approval readiness until fresh provider and session reads
recover; it does not cancel a same-account worker.

Fourteen focused provider tests cover delayed/null hints, overlapping read success
and failure, bootstrap ordering, explicit/hint recovery, action failure recovery,
safe connecting metadata, pending cache readiness and malformed responses. All 102 combined
transport/resolve/AgentPanel tests, strict TypeScript, generated bridge equivalence
and diff checks pass for this separate follow-up. No native browser, live account,
UI layout or phase gate evidence is implied.

Review of `51737ab` found that a session refresh could restore readiness while the
hint's authoritative provider read was still pending. The original provider tests
resolved that read before refreshing the session. Three deferred-read regressions
reproduced the missing barrier for send, approval and overlapping provider reads.
The bridge now retains only the latest provider read's pending generation/sequence;
session reads during that interval can update worker metadata but cannot verify
send or approval readiness. Publishing even unchanged account metadata invalidates
session reads begun before provider recovery, requiring a subsequent session read.
Older provider completions cannot release the current barrier, and superseded
pending reads do not block readiness after the latest read and session recover.
No worker cancellation or transcript clearing is introduced.

Twenty provider tests now cover this ordering, post-provider session fencing,
current failure recovery and obsolete reply/failure completion. All 108 combined
transport/resolve/AgentPanel tests pass, including the original action failure
receipt tests. Strict TypeScript, generated bridge equivalence and diff checks
pass. This follow-up is mock transport evidence only; it adds no native account,
browser or live-provider validation.

The Play polling follow-up suppresses publication of an identical accepted
`PlayStatus`, retaining the complete bridge snapshot and Play object identities.
The native `Service::snapshot` clones metadata without changing its sequence;
production mutations go through `Service::edit` and worker publications advance
that sequence (`editor/app/src/play/mod.rs:122`, `:128` and
`editor/app/src/play/worker.rs:255`). The bridge nevertheless compares every flat
status field, including counters, so changed fields at equal counters still
publish. An accepted-update marker preserves failure fencing when a newer
successful reply is deduplicated. A matching successful status clears a prior
`playError` and notifies once. Poll frequency, native input notification behavior
and stale-generation/sequence rejection remain unchanged.

Five regression cases reproduced unnecessary notifications/identity replacement
before the fix. Six new observable cases cover four-Hz duplicate idle/paused/
running reads, genuine tick/sequence changes and equal-counter field changes,
error-to-identical-status recovery and an obsolete failure after a newer identical
reply. All 114 scoped bridge/resolve/AgentPanel tests, strict TypeScript, generated
bridge equivalence and diff checks pass on this frozen correction. These mock
subscriber counts establish publication behavior; they are not native execution,
rendering or measured frame-performance evidence.
