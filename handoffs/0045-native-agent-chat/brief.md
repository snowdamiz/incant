# Native editor agent conversation and approvals

Use Claude Opus 5.5 via ACP, explicitly Max. Astra owns Rust/backend/transport and
correctness. You own visual design, layout, interaction and rendered-pixel review.
Read CLAUDE.md, editor/ui/DESIGN.md, and the actual shared bridge contract first.
Work only in this isolated handoff; no publishing, PRs, or main/candidate changes.

## Goal

Replace the Agent panel's empty transcript with the real bounded native session:
streamed replies, current/finished turns, approval decisions, cancel, safe failures,
actual token accounting and model/approval selection. Make this feel like a polished
professional editor. The user's consistent direction is restrained, clean, useful
and spacious at small widths: no decorative stripes, glows, giant status badges,
tutorial paragraphs or redundant identity rows. Preserve the app's existing colors,
account dialog, shortcuts, inspector, native titlebar and traffic-light alignment.

This work is concurrent with handoff0044's native Play controls. Do not edit its
Play files, ViewportPanel, InspectorPanel, App, main, ShellContext, Icon or base.css.
Keep changes scoped to AgentPanel and small new agent components/styles/tests,
existing agent.css if present, and handoff evidence. Import dedicated CSS from your
agent component if needed. If a shared-file change is necessary, report the exact
small patch as a follow-up instead of conflicting. No engine/compiler/CI/dependency
or editor/bridge source edits; report contract gaps instead of inventing behavior.

## Real contract

Read editor/bridge/contract.ts (re-exported by the UI):
- snapshot.agentSession contains version, budget {max_tokens,used_tokens},
  accounting_uncertain, worker_running and bounded turns (16). It is absent on
  older hosts or until bootstrap. Never fabricate conversation state from a send.
- Each turn has id, request {prompt,model,approval}, status, text, approvals,
  pending, report, failure, charged_tokens, accounting_uncertain. Status is running,
  awaiting_approval, stopping, completed, cancelled or failed. Text is plain streamed
  provider output (untrusted); render safely, never HTML/eval. Request.prompt is
  also plain text. Missing usage is not zero cost.
- Pending approval has turn/id/tool/arguments. Show a concise understandable summary
  and expandable exact arguments or document-operation review, with clear Allow and
  Deny choices. Never silently approve. Requests use explicit turn/approval IDs;
  disable/reconcile stale or already decided controls. The backend removes full
  pending arguments once decided/cancelled and keeps a small decision summary.
- shell.request({type:'agent.send',text,model?,approval?}) returns BridgeResult,
  optionally agentTurnId on success. Keep draft on failed send, guard duplicate
  submission and stale success clearing newer user typing. Defaults are
  gpt-6-astra and destructive. Modes auto/destructive/always are implemented.
  No provider model catalog is available yet: do not invent verified choices.
- shell.request({type:'agent.cancel',turnId}) immediately revokes future publication
  but a blocking provider call may take time to stop. Keep Stop usable when provider
  disconnects or the project errors while an existing worker is still running.
  A new turn stays unavailable until the physical worker finishes.
- shell.request({type:'agent.approve',turnId,approvalId,approved}) resolves exactly one
  approval. Use shell.snapshot and capabilities. Existing shell.request works.
- snapshot.agentError is a safe transport failure distinct from a turn's failure.
  shell.bridge?.refreshAgent?.() is the explicit optional Retry hook. Readiness is
  provided by legacy snapshot.agent (idle/running/unavailable) and actual capabilities.
  Transport recovery must not erase visible history or claim a failed turn succeeded.
- Tokens are actual charged session allowance, with an explicit uncertainty flag.
  There is no reliable dollar-cost quote, configurable budget IPC, model catalog,
  transcript persistence, image transcript, inline document diff or turn-specific
  revert command yet. Do not fabricate these. History is the existing correct
  provenance/undo path; a useful action may open that panel through shell methods.
  These are later functional work, not a reason to stall this shipped capability.

The real Rust binding uses the existing command bus/journal, project-bound bounded
conversation context, saved OpenAI account and native Play host. Unsaved projects
have no import or native Play file capability. Native Play state lives within one
agent turn; authoring screenshot capture is real but not a gameplay-frame capture.
No credentials/authorization URLs/raw provider errors belong in UI fixtures/logs.
The manual editor stays usable offline and disconnected states must remain clear.

## Interaction and acceptance

Keep inspector/agent space practical at 280px and 380px sidebar widths, 1000x650 and
1440x900 windows. Stream without stealing scroll when the user is reading older
turns. Keyboard navigation, accessible names/status and focus after decisions and
cancel must work. Do not announce every streamed token. Long prompts/replies/tool
arguments must wrap or scroll without expanding the shell. Show partial replies and
failed/cancelled statuses truthfully, preserve prior turns through account transitions.

Test actual requests and ID wiring, duplicate send, pending decisions, stale replies,
stop while disconnected, stopping/worker-running distinction, retry, invalid/unknown
failure fallback, actual/uncertain usage, older-host absence, and long content. Use
clearly labelled deterministic bridge fixtures, never imply these are live-provider
or real-engine results. Run full UI tests, strict typecheck and build; capture and
review all important states at the specified widths using browser automation.
Do not launch a native renderer until Astra supplies an exact binary/fixture and
explicit graphics grant. No real account mutation or provider usage by this session.

Return result.md with verified Opus5.5/Max, source paths, tests, screenshots and
remaining scope. Commit with Built-by: claude. Finish this coherent scoped work
rather than expanding into unrelated app restyling.
