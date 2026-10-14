# 0077 — Host-bound project Save and engine-pin review

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
