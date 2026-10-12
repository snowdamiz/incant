# Persistent project agent allowance controls

Required routing: Claude Opus 5.5 through ACP, explicitly verified Max on this
new session. Astra owns backend/transport/correctness; you own appearance,
interaction design and rendered review. Read CLAUDE.md and editor/ui/DESIGN.md.
This packet supersedes old handoff0045 statements that all usage is session-only
or that no budget command exists. Work in this isolated handoff, no publishing.

## Goal and boundaries

Finish the editor controls for the now-real durable per-project token allowance.
Extend the current professionally restrained Agent pane, keeping conversation,
approvals, Stop, offline editing and narrow widths usable. Avoid clutter, repeated
help paragraphs, decorative borders/glows and excessive cards. You choose the
compact interaction/layout. The user explicitly prefers thoughtful design rather
than adding a dense form to the inspector. Preserve existing app colors/titlebar.

Own AgentPanel.tsx, components/agent/*, styles/agent.css, related tests, the Agent
section of DESIGN.md and this packet's evidence. Do not edit bridge/backend,
ShellContext, App, Play, Viewport, Splitter, Inspector, base styles or CI. Report
shared changes as exact follow-up patches. Handoff0044 owns native Play changes.
You may use a labelled test fixture to exercise the real NativeBridge API;
fixtures must not invent capabilities absent from the production contract.

## Integrated real behavior

Read editor/bridge/contract.ts, native.ts and native.budget.test.ts. Root has wired
and tested dynamic capabilities in ShellContext. The engine exposes the cap only
when explicit native budget support, published Project schema and valid authored
settings agree. The capability can appear after loading or disappear on failure.

- snapshot.projectAgentBudget?: {maxTokens:number} is the authored project cap.
  Capability 'agent.budget.set' permits editing through
  shell.dispatch({type:'agent.budget.set',maxTokens}). This invokes the shared
  command bus with revision checks, Actor::user, journal and Undo/Redo. Never
  mutate local snapshot or call host IPC directly.
- Nonnegative safe integer caps, inclusive 0..9,007,199,254,740,991. Zero disables
  new turns. Default is 1,000,000 but display the actual snapshot, never fabricate
  it for old hosts. Cap editing works offline without a provider account.
- Actual usage remains snapshot.agentSession.budget.used_tokens and
  accounting_uncertain. Durable usage survives app exit/rebuild, account change,
  cap edits and Undo/Redo for the same project ID; Undo changes only the authored
  cap. Do not provide a usage-reset button or imply raising the cap resets spend.
- Cap can be less than usage, including zero. Keep actual usage visible; clamp
  any derived remaining amount to zero, not the recorded consumption.
- Cap edit and budget-aware Undo/Redo fence Send/approval until the bridge reads
  a matching actual Session. The retained session's old max_tokens can temporarily
  differ from projectAgentBudget; don't claim stale values match or recompute native
  usage. Use actual bridge readiness. Existing Stop remains available.
- The command returns BridgeResult. Preserve unsuccessful draft input and safe
  inline error. A successful authoring command can coexist with a later failed
  session refresh; display actual authored value and existing agentError/Retry,
  never claim the save failed simply because usage refresh failed.
- In-flight changes must be reconciled against snapshots. Guard duplicate submits,
  cap changes from Undo/another actor, panel unmount, and stale success clearing newer
  typing. The current per-bridge memory pattern is available but need not persist
  closed form state unnecessarily. Never allow an older project to save a new one's
  draft accidentally. Project identity is snapshot.connection.project.id when connection.status is ready.
- Older hosts lack authored projectAgentBudget/capability and keep session-only
  usage. Give no cap editor. Use truthful capability-aware scope wording or neutral
  'token allowance' text. Current AgentPanel exhaustion text, agentText failure text
  and native bridge fallback say 'session'; correct UI presentation without falsely
  relabelling old hosts. Backend fallback changes can be exact follow-up patches.
- A denied operation becomes a tool result and the agent can continue. Do not undo
  root's correction that a later Tool failure is not necessarily caused by denial.
  Uncertain usage may be provider uncertainty OR failed durable receipt storage;
  keep cause-neutral language. It is a conservative recorded charge, not dollar cost.
- No reliable model pricing/catalog or dollar cost estimate exists yet. Do not invent
  prices, account spend, billing totals or unsupported model choices. Keep full plan
  cost/model/transcript requirements explicitly open in the returned limitations.

## Validation and review

Test public user interactions and actual dispatch values for default/existing/zero/
MAX caps, invalid negatives/fractions/unsafe integers/empty input, failed saves,
old host, no provider, delayed metadata, duplicate submissions, active worker,
usage over cap, uncertain usage, cap/usage mismatch during refresh, success+refresh
failure, external cap changes and remount. Preserve prior 777 passing UI cases.
Use max4 Vitest workers to avoid oversubscribing concurrent work. Run strict app
and test TypeScript checks plus editor production build. Reuse pinned dependencies;
no dependency upgrades. Root owns final integrated full checks.

Capture and inspect relevant states at 280/380px sidebar widths and1000x650/
1440x900 windows. Confirm keyboard focus, labels, inline errors, wrapping and
scroll behavior. Use isolated browser automation; do not capture the desktop or
launch a native renderer now. Root will supply a new immutable native build for
combined acceptance after integrating0044/0046. No account/credential reads,
provider calls or external state changes in this session.

Commands:
- npm run test --workspace editor/ui -- --maxWorkers=4
- npm run build --workspace editor/ui
- node node_modules/typescript/bin/tsc -p editor/ui/tsconfig.test.json --noEmit --incremental false
- node tools/build_bridge.mjs --check

The shared node_modules in the parent repository are pinned and may be symlinked
into this worktree. Use private ignored artifacts paths/ports for review scripts.
Existing0045 screenshots/result are historical context, not acceptance for this
new work. Return result.md with exact model/effort, source paths, commands/results,
reviewed screenshots, tested behavior and limits; commit with Built-by: claude.
