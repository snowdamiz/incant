# Native project agent token-cap bridge

Implementation routing: `gpt-6.1-sol`, Extra High (`xhigh`), Astra role.
Base: `5c3eab55f5f6c3b972b1046e65f77fa7107ad014` in the reused
`agent-budget-settings` checkout, branch `codex/native-agent-budget-bridge`.

The bridge exposes optional `BridgeSnapshot.projectAgentBudget.maxTokens` and
`EditorCommand { type: 'agent.budget.set', maxTokens }`. Its capability is advertised
only after the native host returns `agent_budget: { available: true }`, the published
Project schema describes the bounded budget settings, and the project settings are
valid. An omitted `settings.agent_budget` uses the schema-declared 1,000,000 default;
missing or malformed settings, metadata, or schemas never create writable support.
Caps are nonnegative safe integers through 9,007,199,254,740,991; zero disables turns.
Old hosts retain their existing transport behavior and refuse this edit without IPC.

The edit invokes the existing native `engine_execute` command with
`commands: [{ op: 'set_agent_budget', settings: { max_tokens: maxTokens } }]`,
`description: 'Set project agent token cap'`, and the actual `expectedRevision`.
Native `engine_execute` already supplies `Actor::user("editor")`; document validation,
atomic commit, journaling, persistence, and Undo/Redo remain in `incant_cmd`.
No project mutation or usage ledger is implemented in the webview. Manual cap
editing remains available without a connected provider.

Cap edits and budget-aware Undo/Redo immediately invalidate agent admission and
fence older session replies/errors. Even a fresh read while the authoring command
is pending cannot release that barrier. After the command or its recovery read, a
fresh native session verifies the currently published cap before Send or approval
can resume. Project identity/cap changes observed through engine events also fence
old reads. A session indicating a newer document revision blocks admission while
that document is read. Older command replies and replies that do not show the
requested cap fail visibly and recover current state. Unknown exceptions in budget
or agent-triggered project recovery use fixed transport messages.

Usage, uncertainty, transcript, and active-worker accounting always come from the
native Session. Changing or undoing a cap never reduces used tokens or synthesizes
completion. An existing worker can still be cancelled during cap recovery. A cap
increase above measured consumption enables an actual subsequent Send after fresh
verification; a decrease below usage remains exhausted. Failed session recovery
preserves a successful authoring receipt and supports explicit `refreshAgent()`.

The only UI-tree change is the exhaustive capability-label entry in `resolve.ts`,
required by the shared protocol type. Root owns capability-set freshness in
ShellContext, native adapter/metadata integration, and subsequent Claude controls.
The existing authoringReader rest-spread already preserves projectAgentBudget.
No component, appearance, Rust source, or Claude child checkout is edited here.

Validation on the frozen source:

- All five focused native bridge/resolve test files passed: 155 tests. Includes old
  host/default/zero/MAX/invalid cases, exact shared command/revision, offline/no-op
  authoring, rejected/stale/malformed replies, cap increase from exhaustion, deferred
  old replies/errors, pending mutation reads, Undo/Redo, cancellation, worker usage,
  provider changes, read failure/retry, and newer-revision admission fencing.
- Strict app and test TypeScript checks passed with `--noEmit --incremental false`.
- `node tools/build_bridge.mjs --check` and `git diff --check` passed.
- Node 22.23.3, TypeScript 5.9.3, Vitest 5.0.3. Pinned existing node_modules are
  reused through an ignored symlink. No native process, GPU, provider/account call,
  Rust build, publication, or phase/device gate was run or claimed.

Reproduce from the checkout:

```sh
node tools/build_bridge.mjs --check
node node_modules/typescript/bin/tsc -p editor/ui/tsconfig.app.json --noEmit --incremental false
node node_modules/typescript/bin/tsc -p editor/ui/tsconfig.test.json --noEmit --incremental false
cd editor/ui
node ../../node_modules/vitest/vitest.mjs run --config vitest.config.ts ../bridge/*.test.ts src/bridge/resolve.test.ts
```

Source SHA-256:

| Path | SHA-256 |
| --- | --- |
| editor/bridge/contract.ts | c486ded0a06da20e1db8b51a4258bdeac2eef36c496446df51937915415be812 |
| editor/bridge/native.ts | 0749f1459926bc2c51e2a17a30aa058eb3e1ce1e52b6ea55aa3fece0197fc264 |
| editor/bridge/native.generated.js | 9204c04838e321351d42d570d8e31e04518168279c990a906623c59151c0cee4 |
| editor/bridge/native.budget.test.ts | 5e401c5a045f4486aa9153e7058c3af15b09345b66601ceb73febdc3cfc4874b |
| editor/ui/src/bridge/resolve.ts | 08698dfe1c93f7e23e5cc0bb4c5b970962c337415b5fe44366fe91e7193f4e54 |
| schemas/Project.schema.json (unchanged evidence input) | 00cb938ddd2a90fc6575006f57aaccb9bed7785b9a980263a8ef174156d371e7 |

Root's reviewed native `main.rs` metadata/admission source was
`9a1dc8f20d3e0a8f1d63e3db9958102529991a11233c05c14fb47ebdcc1d1e12`
at handoff; it is a separate integration prerequisite, absent from this patch.
The source-only durable-adapter review record is
`/tmp/incant-native-budget-adapter-review.json`, SHA-256
`75ec74c93145f49998a1c795876195122c377ddbc132c63b732af6076a1ef24d`.

Local test log `/tmp/incant-agent-budget-bridge-tests.log` has SHA-256
`c1aac779403435ae3374c27f826e7758555681189395f8ce62702215e766cec1`.
The successful strict type logs `/tmp/incant-agent-budget-bridge-app-types.log`
and `/tmp/incant-agent-budget-bridge-types.log` are empty, SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
Mocked transport tests establish bridge behavior only; root retains native/live
integration and UI acceptance work.
