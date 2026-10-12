# Native editor-agent lifecycle and account integration review

Reviewed read-only on base `cf4b4784bb409f99422ea01da9e9c9a5438970f4` plus the frozen binding in `/Users/andreyyurlov/.codex/worktrees/native-agent-play/incant`. All twelve inputs match `/tmp/incant-editor-agent-binding-freeze.json`. The final inventory, including seven supporting account/conversation/host sources, is `/tmp/incant-editor-agent-lifecycle-final-reviewed-sources.json` (SHA-256 `8b275279440add9ac045f1ed490c26dfb22f8ede9c353f557dafede79a336231`).

## Remaining actionable finding — P2 retained Play disappears between turns

`editor/app/src/agent/worker.rs:39` constructs a new host for every `run`, and lines 49–53 drop it when that turn returns. `host` constructs `incant_agent_native::Host::new` at lines 70–73, whose fresh Controller has `live: None`. Only Conversation survives in `Context`.

Observable trigger: in a saved project, one accepted turn calls `play_run` and finishes. A second same-account turn calls `play_state_query` for `/tick` or a selected entity. It receives `no play session` from `incant_agent_native/src/controller/run.rs:187–190`, even though the model retains the first turn's successful result. A rejected second-turn `play_run` likewise cannot preserve the previously accepted candidate. The advertised shared tool contract explicitly retains the latest successful session; no turn-scoped restriction is declared.

Preserve the native host/controller across same-project, same-account-receipt turns with a thread-affine owner; expire/reset it when the account/project scope changes or the service stops. Do not make QuickJS state Send unsafely or reconstruct prior gameplay by replay. A meaningful regression should execute two actual worker turns: run then query, plus a failed later candidate followed by query returning the first accepted tick/state. Current `saved_project_host_runs_native_world_without_changing_authoring_state` keeps a manually created local host and therefore does not cover the actual per-turn construction path. This finding comes from source ownership/lifetime analysis; no new execution was performed.

## Resolved earlier finding

The browser-reopen admission/shutdown blockage is resolved by `provider.rs:167–213`: the gate schedules one bounded worker, the opener runs without admission/browser/state locks, and an RAII guard releases the single-flight flag. Failure publication is fixed safe text only when the same BrowserAuthorization Arc is still current and the state remains connecting. Browser clearing precedes terminal publication. Notifications are invalidation hints emitted after locks, preventing a delayed captured payload from overwriting current account state. The injected blocking-opener, duplicate-coalescing, terminal/replacement-race and redaction tests exercise the relevant lock/identity boundaries. Original finding is preserved in `/tmp/incant-editor-agent-lifecycle-review-initial.md`.

## Other reviewed boundaries

No additional actionable defect found in:

- Service admission/account-operation/shutdown ordering, physical worker exclusion until actual reaping, cancellation/approval IDs, fixed IPC error mapping, or unchanged-session reads. State publication releases the provider mutex before entering admission; worker context is absent from that lock order. Existing Play UI service remains unchanged.
- AccountSelection integration: worker captures selection and provider from the same locked store, opaque receipt scopes Conversation, and provider checks occur before requests, after measured responses are charged, for each tool and after approval. Checks avoid project locks and do not fetch credentials. A changed account blocks pending tools; already committed earlier transactions remain honestly committed.
- Bounded Conversation: whole-turn eviction, explicit incomplete-turn summaries, screenshot omission and request-size limits preserve protocol grouping. Account receipt changes clear prior context. This is independent of the missing retained Play owner above.
- Session accounting: only charged `Response { usage: None }` marks a conservative request allowance; metadata fence failures after measured responses retain exact usage. Cancellation remains cancellation except malformed accounting/worker panic. Existing bounded text/approval/report snapshots and ContextLimit mapping remain intact.

Owner's final test log `/tmp/incant-agent-binding-final-tests.log` reports 109 passed and one existing GPU test ignored. The four-package Clippy run was still owner-managed when this report was written; this review does not claim its completion. No source edits, tests/builds, external provider calls, GPU/captures, PRs or main changes were performed. The separate bridge correction that reads current provider state after invalidation hints still needs exact-source integration review before whole-path acceptance.
