# ADR 0009: React TypeScript chrome in Tauri

Date: 2026-10-08. Updated: 2026-10-10. Status: decided by director instruction
in the PLAN.md revision 5 decision record (decisions 1 and 5). This does not
approve the Phase 0 gate.

## Decision

Choose React for the Phase 0 shell and Tauri for the native host. UI snapshots are immutable. Only editor/bridge sends commands to the Rust bus. Capabilities describe what the current host actually supports; disconnected or fixture state must be explicit.

## Evidence and implementation boundary

Claude Opus 5.5 owns the visual shell over ACP. Astra owns IPC translation, correctness and integration. No UI writes bypass incant_cmd.

## Consequences and revisit trigger

React versus Solid can be revisited with measured large-hierarchy interaction costs. The initial inspector presents schema values; it is not a complete property-editing UI. Browser mode ships in 1.0 with the scope in PLAN.md decision 5: editing,
TypeScript, the agent, in-browser play-tests and web export. Rust module
compilation, native plugins, native-platform export and file access beyond the
File System Access API stay desktop-only; browsers without WebGPU get a clear
unsupported message. It is not delivered yet. Hierarchy, inspector and asset
panels must be virtualized for the 100,000-entity Phase 2 gate; the redesign and measurement are in progress.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
