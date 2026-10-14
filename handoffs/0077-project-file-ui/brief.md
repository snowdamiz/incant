# 0077 — Host-bound project Save and engine-pin review

## Newest priority refinement — coordinated bridge outcome marker

This refinement supersedes the Shell-only return subtype proposal in the next section. Root found that production `NativeBridge.requestProjectFile` also converts an invoke rejection or invalid success receipt into a resolved failure. The Rust operation may already have written the project before a later read/receipt fails, so a resolved bridge failure does not by itself prove that no write happened.

Root supplied the exact production bridge changes to the handoff child: `editor/bridge/{contract.ts,native.ts,native.generated.js,native.project-file.test.ts}`. The shared optional local metadata field is `outcomeUnknown?: true` on `BridgeResult`, with a post-invoke catch marker and focused transport regressions. `evidence/root-bridge-overlay/` preserves the separate root-owned patch, exact file hashes, origin and Astra attribution; root qualification is underway. These are local bridge result metadata, with no Rust IPC/schema change. Claude must not implement or alter those four production bridge files; read the supplied root changes and consume the actual field. Do not claim Claude authored the overlay or that root qualification passed before root reports it.

In the UI, set `outcomeUnknown: true` for every caught `Shell.request` promise rejection, retaining actual code/message verbatim regardless of error shape. Convert every project-file failure carrying that marker into `FileFailure.unknown`; never derive unknownness solely from a code. Resolved authoritative host refusals without the marker remain definite. Cover structured rejected Save/apply and resolved marked failures, alongside unmarked host refusal regression. Keep the stale-comment correction and all earlier delayed-response/project-identity requirements.

The actual root-owned shared field is now present in the child. Continue with the UI consumer and focused regressions against that exact overlay; do not invent another production interface. Final validation and evidence must include the coordinated bridge metadata with root ownership explicit. Refresh visible state captures only when output changes, as instructed below.

## Newest priority revision — structured transport rejection

Root's final source review found one remaining correctness gap. Preserve the completed prior result/evidence and make this focused correction before delivery; no redesign is needed:

- `ShellContext` catches a rejected promise with a structured error such as `{ code: 'host.closed', message: '...' }` and correctly preserves its actual code/message. `projectFile.hostFailure` currently marks the outcome unknown only when the code is `request.rejected`. A structured transport rejection must also be unknown: an apply whose host result never arrived must not claim "The engine pin was not applied."
- Preserve code/message verbatim while carrying a rejected-result marker from the `Shell.request` catch to the project-file failure conversion. The original Shell-only subtype proposal is superseded by the newest coordinated bridge outcome marker above. Mark `FileFailure.unknown` for every caught promise rejection regardless of its code or error shape; do not infer the marker from a host-provided code.
- Add observable structured-rejection Save and apply regressions that assert the real code/message are retained and the outcome is unknown. Retain coverage that resolved host refusals remain definite.
- Correct stale comments in `ProjectFileContext` that say the snapshot supplies an authoring revision. `model.ts` correctly describes the history-mark fallback; align the comments without inventing a new readback.

Run the applicable focused tests, typecheck/build and required shared-Shell regressions on the corrected source. Refresh final captures only if visible existing-state output changes; keep actual rendered evidence and label any new structured-rejection capture truthfully. Record the exact correction and tests in the final `result.md`, preserving prior attempts/logs. All earlier scope and acceptance constraints remain in force.

## Priority revision — root correctness review

Root reviewed the first implementation while the flow tests were being added. Preserve all current changes and the original failed-test log, and address these findings before completing the packet:

- Bind every Save, review and apply operation's UI result to the bridge/project identity that initiated it. A Save or apply started for project A may finish after project B opens; the project-change effect clears existing notices but does not retire late results, and apply needs an identity/basis guard after its await. A late A result must not create a success announcement, receipt or failure notice in B. Prefer retiring the old operation's UI result; any retained old-project outcome must be explicitly and truthfully labeled. Keep the existing backend authority and revision enforcement unchanged.
- Add observable delayed-response tests that switch the active project/bridge while Save and apply are pending, then resolve or reject the old response. Assert that the newly opened project's UI does not receive the old project's outcome and that the old review cannot be applied to the new project.
- `Shell.request` does not catch a rejected `bridge.request` promise. Ensure an unexpected rejected request produces a truthful local failure state and clears requesting/applying/busy state instead of leaving the UI pending indefinitely. Preserve actual host error codes when available; do not synthesize backend success or guarantees from the rejected promise. Cover rejected Save, review and apply requests with focused tests.

These corrections are UI-only and need no backend, contract, schema or transport changes. Include the exact fixes and delayed-response/rejection test outcomes in `result.md`. Continue the existing scoped frontend and rendered-pixel acceptance criteria.

## Authorization and exact base

The director authorized this editor UI handoff after the reviewed application backend merged in PR276. Coordination starts from main `78d41fdb2d87546f8a7b93a4e095deaa3693224a`. Use Claude Opus 5.5 with thinking explicitly Max through ACP. The launcher verifies both before this prompt. The director authorized `bypassPermissions` for this scoped session. Do not commit, push, merge, publish, alter shared refs, or approve a phase gate: root must review the complete result first.

The user wants a professional, neutral, compact editor, using the established controls and density. Implement a small usable entry point for the already implemented Save and engine-pin review/apply. Do not restart the broader editor redesign, add decorative boilerplate, account details, invented badges, or fictional authority/status. Root/Astra owns all backend and renderer work; this task is editor UI only.

## User flow

- Provide a clear compact project-file entry point near the existing project/titlebar controls, using existing menu/dialog primitives and established styles.
- Save invokes the existing `project.save` HostRequest. Cmd-S on macOS / Ctrl-S elsewhere invokes that same route, consumes the browser Save shortcut when appropriate, and never directly mutates a document or file. Preserve existing game-input capture, dialogs, edit fields, undo/redo and shortcuts behavior.
- An engine-pin action requests `project.pin.review`. Present the actual returned requirement/change and expected revision in a dedicated accessible review dialog. Permit apply only for that exact still-current returned review ID; never synthesize an ID, candidate, revision, or successful result. Show the actual review in a compact readable form; candidate text is untrusted display text, never HTML or an edit payload.
- Explicit apply confirmation invokes `project.pin.apply` with only `reviewId`. Use the existing bridge revision enforcement. A stale review, changed project/revision, or capability loss must disable/invalidate the pending confirmation and explain how to request a fresh review. No automatic retries or automatic application.
- Show truthful busy, success and failure states for Save/review/apply. Prevent duplicate in-flight requests. Keep failures visible and useful, including unsupported/newer project, unavailable/unopened project, already-pinned/refused review, stale revision, native error and disconnected host. Do not parse arbitrary error strings into fictitious guarantees.
- Do not show an invented dirty indicator: current interface has no authoritative unsaved-state readback. A Save receipt can honestly report saved/unchanged at its returned revision. Fixture/browser mode remains honestly limited and cannot pretend to write files.
- Accessibility: focus trap and focus restoration, Escape behavior, dialog title/description, keyboard-only operation, appropriate status announcements, visible focus, usable contrast, no nested interactive hit targets in the titlebar drag area. Retain existing viewport and panels at narrow window widths.

## Existing authority and code boundaries

Read these first:
- `editor/bridge/contract.ts`: ProjectFileRequest/ProjectFileResponse near line438; no paths/text in requests.
- `editor/bridge/native.ts`: `requestProjectFile` near1241 validates project capability, current revision, host response/snapshot and refresh. This production authority/transport is already reviewed and must remain unchanged.
- `editor/bridge/native.project-file.test.ts`: current transport success/refusal/capability controls.
- `editor/ui/src/shell/ShellContext.tsx`: `request`, `ask`, announcements and `project.file` capability mapping.
- `editor/ui/src/components/Titlebar.tsx`, `Menu.tsx`, `ShortcutsDialog.tsx`, and existing app shortcut handling.
- `editor/ui/src/bridge/resolve.ts`: capability presentation.
- `editor/app/src/project_file.rs`, `crates/incant_cmd/src/project_file.rs`, `project_checkpoint.rs` are READ-ONLY background if needed. Save and pin use the persistent command bus and retained host file capability.

Allowed implementation: focused `editor/ui/src/` components/styles/hooks and tests, scoped UI-only fixture/test setup where needed, and this handoff evidence directory. Existing generated bridge/SDK files, production `editor/bridge`, Rust, schemas, contracts, manifests/locks, runtime, renderer, provider/account behavior, engine settings and tool routing are out of scope. If a needed data binding is absent, report it instead of inventing a new interface. No arbitrary paths, Save As, file picker, direct document writes, JSON mutation endpoint, new backend operation, auto-migration, or engine-version conversion.

## Execution and resource limits

No Rust/Cargo/C++ builds, native app launch requiring compilation, GPU engine tests, GPU performance measurements, shipping exports, benchmark/timing claims, unrelated screenshots or screen capture. Local browser rendering of this editor UI is permitted and expected, only in tabs/processes created for this packet. Use actual rendered pixels to refine the result and retain screenshots of the scoped UI, not unrelated desktop content. These are UI fixture visuals, not native backend execution or phase evidence.

Before any frontend build/test or browser screenshot, check `handoffs/0077-project-file-ui/quiet.json`. If `hold` is true, stop local frontend/browser execution and do source-only work until false; coordinator may update it for a short quiet timing window. Keep tests bounded and serial; avoid watch mode and kill/reap owned dev/preview/browser processes before returning. Do not run performance samples. Do not install/change dependencies or mutate a shared node_modules tree. The parent dependency runtime is available through the inherited node_modules resolution; exact package-lock matches its supplying checkout. If a dependency is genuinely absent, report that limitation rather than modifying dependencies.

## Verification

Run focused UI tests for the new flow and touched shortcut/titlebar/dialog behavior, plus UI TypeScript/build validation where feasible. Use the existing Vitest/Testing Library and axe conventions. Tests must assert observable behavior: exact request types/review IDs, capability omission, Save receipt, busy duplicate suppression, review displayed from host response, stale revision/project change, refusal/error, focus return and keyboard access. Avoid tests that merely mirror implementation. Existing commands include `npm run test --workspace editor/ui -- <test paths>`, `npm run typecheck --workspace editor/ui`, and `npm run build --workspace editor/ui`.

Render the actual UI locally at the normal desktop size and one narrower width; inspect the new menu/dialog and busy/error/refusal state. Capture only the relevant editor UI with no credentials or account identifiers. Do not claim the fixture proves native file persistence. Preserve original logs if a test fails, fix and retain the successful successor result without relabeling failures. `git diff --check` must pass; no backend/generated-file drift.

## Return

Leave all implementation changes uncommitted for root review. Write `handoffs/0077-project-file-ui/result.md` with exact model/effort, changed paths, commands and outcomes, screenshot paths, brief design rationale, backend boundary checks, accessibility observations, process closure and remaining limitations. Include a compact source/diff summary and evidence that request routing uses only existing operations. Do not write a completion or phase claim based solely on tests: inspect rendered pixels first. If blocked by missing authority/data, return the precise missing binding with the useful completed UI work preserved.
