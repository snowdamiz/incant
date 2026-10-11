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
