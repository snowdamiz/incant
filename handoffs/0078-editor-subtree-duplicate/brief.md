# Editor subtree duplication interaction

Implement the missing Phase2 ordinary subtree Duplicate interaction using the prepared real engine command. Required model Claude Opus5.5, effort explicitly Max through ACP. Root owns native command correctness and integration. You own editor UI design/layout/interaction and rendered review. Preserve the clean restrained app design: neutral palette, clear hierarchy and consistent spacing; no decorative glows, colored side bars, oversized explanatory cards or implementation prose.

## Starting behavior and exact contract

The current source exposes a capability-gated `EditorCommand { type: 'entity.duplicate', entity: Ulid }`. The bridge calls the host with the current scene/entity/revision, and the host allocates fresh ULIDs and executes one existing `incant_cmd::DuplicateSubtree` transaction with user provenance. A verified response is `{ok:true, duplicatedEntity:Ulid}`; failed or unverifiable completion never invents an entity. `Shell.dispatch` returns that exact result and announces errors. Older hosts and static fixtures do not advertise this capability; keep read-only fixtures read-only.

Astra owns `crates/incant_cmd`, `editor/app`, `editor/bridge/contract.ts`, `editor/bridge/native.ts`, its tests and generated bridge. Do not edit those paths. You may change editor/ui components, shell interaction, keyboard catalog, associated focused tests and evidence. Do not add a second document model or mutate hierarchy snapshots. Use the returned engine entity only after it exists in the latest same-project snapshot; preserve selection on failure or unknown completion and avoid selecting into a switched project. Undo/redo uses existing history. Linked prefabs and singleton-containing subtrees explicitly refuse in the shared bus; surface real failure without silently detaching/removing components.

## Interaction acceptance

- Add Duplicate where it belongs in the hierarchy context menu, consistent with Rename/Delete. Add Cmd+D on Mac/Ctrl+D elsewhere while the authoring hierarchy owns focus. It must not hijack typing, dialogs, live play inspection, a scene root, unavailable capability, or unrelated panels.
- On success select/reveal the copied root, maintaining parent expansion, virtualized scrolling and keyboard focus. Copy includes all descendants and internal references, retains same parent and name, and is one reversible transaction. Do not synthesize suffixes or additional rename transactions.
- Prevent repeat activation while pending. No optimistic rows or fake success. Failure/unknown result remains announced and selection stays on the source.
- Add its shortcut to the existing restrained keyboard reference. No redesign of unrelated UI or extra rows in the titlebar.
- Focused observable UI tests must cover keyboard/menu, descendant source selection, capability absence, pending input, failure/unknown result, source disappearance, focus/typing/live-play guards, and actual returned-copy selection. Mock UI tests do not prove the command bus or native workflow.

## Validation and resource coordination

Use pinned Node22.23.3 (`/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin`). Native builds are owned by root: NO Cargo/rustc/editor binary/GPU test or shared target access. Source/frontend build/test work is allowed. Use the existing available node_modules or install only the pinned lockfile without scripts if necessary. Typecheck with actual project references (`tsc -b editor/ui --noEmit`), run relevant UI tests and production UI build. Maintain exact source identity in result.

Screen capture is permitted again, but use a bounded headless browser render of this UI for appearance and keyboard/layout checks; do not start persistent desktop capture helpers or interfere with native windows. Close browser/server processes on completion, retain screenshot files and precise fixture labeling; never present a fixture as live engine evidence. Run any accessibility checks relevant to the changed interaction. Limit browser work to this handoff and do not touch the user's open browser tabs.

Return handoffs/0078-editor-subtree-duplicate/result.md with verified model/effort, implementation summary, commands/results, actual screenshot paths and limitations. Native correctness and root review remain separate. Do not push, open/merge PRs, publish, change accounts, or sign. Include Built-by: claude on authored commits. Do not claim the entire Phase2 gate passed.
