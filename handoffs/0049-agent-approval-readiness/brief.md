# Active approval readiness: bounded UI correction

Use Claude Opus 5.5 through ACP, explicitly verified Max, for UI and rendered review.
Read AGENTS.md and editor/ui/DESIGN.md. No publishing/merging. This is one concrete
review finding before combined native acceptance0048; keep the current design.

## Confirmed issue and integrated backend

Independent review of0046 found that Allow and Deny remain enabled while Undo/Redo
waits for engine_history or a fresh agent read. Both cached caps may still match,
no local allowance save exists, and the worker must remain running for Stop. Native
admission safely refuses, but clicking Allow produces an unrelated connection
error. The observational repro and exact-source receipts are in review/.

Astra has integrated0e81494: AgentState.running now optionally publishes
approvalReady. NativeBridge derives it from the exact approval-admission guard
and reuses that guard before deciding; existing Stop behavior is unchanged. False
means decisions must wait. Omission is the older-host contract and must preserve
existing UI behavior. It is independent of worker activity, cap equality and
whether the pane initiated the change. All103 scoped bridge tests and strict types
pass. The source is now in your branch. Do not reapply0046 followup patches;
Astra already integrated all three and the later Latest correction.

## Your scope

Consume explicit approvalReady===false in the existing AgentPanel decision hold,
retaining other precise reasons and Stop. Use a short truthful refresh message
for an otherwise connected active worker. Do not make the worker unavailable,
hide the conversation, discard drafts or invent a new layout. Both decision
buttons and keyboard activation must honor the hold. Keep older-host behavior
when the optional field is absent. Let the exact native guard remain authoritative.

Add meaningful observable regressions on the actual NativeBridge + deferred fake
IPC: awaiting approval, start Undo and Redo with equal cached caps, hold through
the history response and subsequent fresh-session read, keep Stop available,
block decision IPC while held and re-enable after verified recovery. Verify that
the real decision can be sent after recovery. Explicit false must dominate equal
caps; omission must not disable a previously usable older host. Keep existing
local cap mutation/mismatch and sign-out/error reasons intact. Reproduce failure
on the uncorrected component before declaring closure.

Review affected rendered pending/recovered state at280/380px sidebar widths,
using the existing browser harness or a small extension. This pass uses browser
fake IPC only, not native/GPU/account/provider calls. Review pixels yourself and
label evidence accurately; no need to recapture every0046 unrelated state.

Own AgentPanel.tsx and related agent components/tests/styles if strictly needed.
Bridge/core changes are Astra-owned: return exact followup requests if needed.
Do not alter contracts or tests merely to make the result pass.

## Checks and return

Run full UI tests max4workers, strict app and test TypeScript, production build,
bridge generated check and diff whitespace check. Preserve any baseline failure.
Return result.md with verified model/Max, source commits, exact tests/capture
results and limitations; commit Built-by: claude. Close every owned browser/helper
process. Root reviews/integrates before granting immutable native0048 acceptance.
