# Native agent session controller

`incant_agent::session::Session` provides bounded native worker transport for the
editor's agent connection. The caller constructs the provider, host and job;
the controller owns no account, credentials, project access or conversation
context. The only existing production-file change is the module export.

## Contract

`start(TurnRequest, job)` passes the job a `TurnContext` containing the selected
model, prompt, approval mode, turn ID, cancellation flag, text/approval handles
and the current session token budget. The job returns `TurnOutcome`: the actual
updated budget plus the agent report or error. `ApprovalChoice::mode()` converts
Auto, Destructive and Always to the existing agent modes; the agent enforces
which tools need approval. The controller does not authorize tool execution.

Only one worker may exist. Competing starts serialize, and a new start remains
refused until the previous thread physically finishes and is joined, including
its teardown. Cancellation immediately marks the turn as stopping, revokes
stream/approval publication, discards a pending argument body and wakes approval
waiters. It does not kill a provider thread. Late returned usage is still charged.
Dropping the controller cancels and wakes its worker without blocking the UI
thread on a provider that has not returned.

Approvals carry both a turn ID and a session-unique approval ID. Decisions for
old turns, different pending approvals or already resolved approvals fail.
After a decision or cancellation, only tool/ID/decision/serialized-size metadata
remains. Full arguments are retained only for the current pending approval.
IDs and snapshot versions stop before losing integer precision in a JavaScript
bridge.

Snapshots carry turn status, bounded transcript, selected model/mode, pending
approval, bounded report summaries and budget/usage. `snapshot_if_changed`
compares versions under the same lock before cloning history or a pending
payload. Native callers can poll and coalesce unchanged snapshots. Internal
provider/tool error text is mapped to fixed failure categories, not serialized.
Reports preserve total counts and usage while explicitly marking trimmed
metadata. A cancelled or failed turn never becomes completed because a late
worker returns a successful report.

The controller retains the budget across successful, failed and cancelled
turns. Workers cannot lower usage or change the allowance; reported response
usage is a lower bound on the returned budget delta. Missing or inconsistent
accounting exhausts the allowance conservatively and marks it uncertain rather
than claiming measured usage. A successful report exceeding the allowance
keeps its actual charge and becomes `BudgetExhausted`. A worker join failure
after publication also recomputes the conservative charge from the turn's
initial budget. There is no per-start budget reset or dollar estimate.

## Bounds

| Retained or accepted data | Limit |
| --- | --- |
| Nonblank prompt | 16 KiB UTF-8 |
| Nonblank selected model or tool metadata | 128 bytes |
| Streamed text per turn | 64 KiB UTF-8 |
| Pending approval JSON | 2 MiB serialized |
| Cumulative approval JSON per turn | 8 MiB |
| Approval count per turn | 32 |
| Approval structure | 32 levels and 4,096 values |
| Turn history | 16 turns |
| Report transaction/tool rows | First 128 of each, with original counts |

Streaming accepts or refuses whole UTF-8 chunks. Overflow interrupts the turn;
it never silently clips text and presents it as complete. Approval preparation
checks structure/raw text bounds first, then uses capped serialization outside
the shared-state lock. A 256 KiB script containing characters that JSON escapes
fits the larger pending-payload bound. Too-large, too-deep and excess aggregate
operations refuse with explicit failure categories; resolved source bodies are
not accumulated in history.

## Local validation

All 62 `incant_agent` tests pass on the local macOS host: 47 existing cases and
15 controller cases. Scoped all-target Clippy with denied warnings, formatting
and diff checks pass. Source and log identities are recorded in
[native-agent-session.json](evidence/native-agent-session.json).
Independent read-only review accepted all seven frozen source/test inputs after
the two accounting corrections described above; the
[review report](evidence/native-agent-session-review.md) is retained with its
hash in that evidence record.

The controller tests use real threads, barriers/channels and deadline-bounded
state observation. They cover competing starts; delayed cancellation and
physical single-flight ownership; expired callbacks and replayed decisions;
denied then approved operations; cancellation/drop waking approval waiters;
Unicode text overflow; bounded large/deep/cumulative approval data; history and
report limits; unchanged snapshot polling; provider errors, panics, invalid
accounting and successful over-budget results. A private test joins a real
post-publication failing thread to exercise late failure accounting without
triggering an abort-prone thread-local destructor panic.

Two tests connect the controller to the real `Agent::run_shared` and command
bus with a deterministic provider. All three approval modes keep one existing
undoable transaction with model provenance. Actual agent response-error usage
is retained while its private error reason is absent from the snapshot. These
are local fixtures, not live-provider evaluations.

## Integration and limits

The editor still supplies the actual provider/account/native-host binding and
must keep one session controller for the intended budget scope. Recreating the
controller per request would violate that caller contract. The caller also owns
project lifetime, conversation context and persistent budget settings. This
slice adds no UI, project writer, provider retry policy, durable transcript,
browser worker or shipped-game dependency. Unsupported native-thread targets
refuse starts explicitly. It neither sanitizes user/model text as a secret
detector nor accepts credentials as controller configuration.

The separately developed bounded conversation runner adds
`AgentError::ContextLimit`; this base does not yet contain that variant. The
snapshot failure enum reserves `ContextLimit`, and integrating that runner must
add its explicit arm to the exhaustive safe error mapping. No wildcard silently
accepts future error variants. Editor integration, cross-platform checkpoint
execution, live-provider gates, visual approval/history/budget UI and cost
estimation remain outside this locally tested transport slice.
