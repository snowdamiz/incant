# Native agent session controller review

Independent read-only source review accepted the seven frozen source/test inputs
in /tmp/incant-agent-session-freeze.json. No remaining actionable code blocker.
Owner logs report 62 passing agent tests and all-target Clippy; the reviewer did
not duplicate those executions or perform live-provider/visual testing.

The worker lock serializes starts and is never acquired by callbacks; physical
thread completion/join precedes releasing single-flight ownership. State guards
are brief, and approval serialization/report bounding occur outside them.
Cancellation immediately revokes publication, drops pending source arguments,
wakes approval waiters and keeps the slot occupied until the worker ends. Turn
and approval IDs prevent stale/replayed decisions. Unchanged-version reads avoid
cloning transcript/payloads. UTF-8 chunks are accepted whole or rejected with an
explicit interrupted limit; no truncated response is presented as complete.

Prompts, model/tool metadata, text, history, pending approval depth/node count,
encoded size, aggregate approval bytes, approval counts and report rows are
bounded. Completed approvals retain only bounded summaries. Provider/internal
error strings do not enter snapshots; fixed typed failures do. Worker results
preserve the original allowance and nondecreasing charges; under-accounted or
missing outcomes exhaust it conservatively and mark uncertainty.

Review requested two accounting corrections, now covered by regressions: a
successful report exceeding the allowance is BudgetExhausted while preserving
actual returned usage, and an unexpected join failure recomputes the conservative
charged delta from the original turn budget. Neither can appear as free or
successful work. Actual Agent-loop tests cover all three approval modes,
provenance/Undo through the shared bus and failed-response usage/redaction.

The controller remains transport only: the trusted caller owns credentials,
project bindings, host lifecycle, conversation and invocation of Agent approval
modes. New AgentError::ContextLimit from root's separate conversation commit must
map to Failure::ContextLimit during integration and be revalidated. No native
editor connection or live-provider gate is claimed by this review.
