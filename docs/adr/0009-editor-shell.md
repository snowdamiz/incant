# ADR 0009: React TypeScript chrome in Tauri

Date: 2026-10-08. Status: proposed for director review.

## Decision

Choose React for the Phase 0 shell and Tauri for the native host. UI snapshots are immutable. Only editor/bridge sends commands to the Rust bus. Capabilities describe what the current host actually supports; disconnected or fixture state must be explicit.

## Evidence and implementation boundary

Claude Opus 5.5 owns the visual shell over ACP. Astra owns IPC translation, correctness and integration. No UI writes bypass incant_cmd.

## Consequences and revisit trigger

React versus Solid can be revisited with measured large-hierarchy interaction costs. The initial inspector presents schema values; it is not a complete property-editing UI. Browser mode remains in the 1.0 plan but is not delivered by a standalone shell.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
