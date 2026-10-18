# Agent tilemap paint adapter

`tilemap_paint` exposes the existing [tilemap authoring service](tilemap-paint-authoring.md)
to an engine agent with project file access. Generic `doc_patch` can already
replace a component, but it does not prepare a stroke against the checked cooked
atlas or apply the shared cardinal rules. This adapter uses that service without
adding another project mutation path.

Arguments are exactly `incant_import::TilemapPaintRequest`: scene/entity/layer
IDs, expected revision, distinct local grid cells, and a paint, erase or autotile
brush. The schema is derived directly from that type. There is no model-provided
root, cache path, actor, transaction description or arbitrary command list.
Project file access is host-owned and bound to a project identity. Without it,
the tool is omitted from provider discovery and direct calls refuse.

The normal destructive/always approval modes apply. Preparation reads the
validated cooked atlas outside the shared editor lock; publication rechecks
cancellation, project ownership, revision and the complete captured document.
Changes use one ordinary `SetComponent` transaction with the model/conversation
actor. Unchanged strokes return `transaction_id: null`, preserving revision,
history and Redo. Results are explicitly untrusted project data and include the
checked atlas identity and ordered before/after cell changes.

The shared limits remain 1–4096 selected cells and at most 20480 changed cells,
within the existing map bounds. This bounds the returned change list; it is not
a reservation of future provider context or token budget. Existing agent request
limits still apply to subsequent turns. There is no additional network, shell,
runtime, UI or filesystem authority. Rules remain one-shot operation inputs:
only their frame family connects, and future edits do not automatically reuse
them. GUI painting and general Phase 2 editor completion remain unqualified.

Ten focused CPU controls are authored in `incant_agent/tests/tilemap_paint.rs`
and its modules. They cover source-free cooked atlas use, equality to shared
prepared commands, actual simultaneous neighbor results, unrelated layer
preservation, actor/entity provenance, durable Undo/Redo/save/reopen, no-op
provider receipts and Redo/journal preservation, strict schema/refusals,
foreign-terrain refusal, absent/wrong host, normal approval modes, denial,
cancellation, approval-time edits and final publication after concurrent edits
or project replacement (including same-ID/revision replacement).

These new controls are not executed yet. Proposed qualification is the new
`tilemap_paint` suite plus the existing `alpha_colliders`, `assets` and `shared`
agent suites, followed by strict all-target agent Clippy. The generated
`schemas/agent-tools.json` update is deliberately reserved for root integration
with the concurrent Wasm tool changes and must be generated/checked before
publication. No native window, GPU, computer-use or capture is part of this slice.
