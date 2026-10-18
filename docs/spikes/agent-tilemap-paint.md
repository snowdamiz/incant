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

Ten focused CPU controls in `incant_agent/tests/tilemap_paint.rs`
and its modules passed on producing source
`8bcfa195573e6a695433096222add01a21deb849`. They cover source-free cooked
atlas use, equality to shared
prepared commands, actual simultaneous neighbor results, unrelated layer
preservation, actor/entity provenance, durable Undo/Redo/save/reopen, no-op
provider receipts and Redo/journal preservation, strict schema/refusals,
foreign-terrain refusal, absent/wrong host, normal approval modes, denial,
cancellation, approval-time edits and final publication after concurrent edits
or project replacement (including same-ID/revision replacement).

The same CPU qualification passed the existing `alpha_colliders` (9), `assets`
(3) and `shared` (11) suites: 33 tests total, zero failed or ignored. Strict
all-target agent Clippy also passed. This used Rust 1.99.0 arm64, the `ci`
profile, locked/offline dependencies, two Cargo/Rayon workers, renderer opt-level
2 and SHA-2 opt-level 3. Test/build and lint stages took 88.100318 and 26.492354
seconds; these are qualification costs, not runtime performance measurements.

Actual execution session `86975` exited zero. Four test executables were copied and
rehashed; the final receipt checked all 66 observed owned PID/group identities
absent. The private target's prior five external artifacts and 1133.056417
seconds of native qualification cost remain retained. Final allocated/retained
peak was 7,328,120,832 bytes beneath the unchanged 12 GiB cap; cumulative cost
became 1252.914890 seconds. No cache/fingerprint deletion, retry or assertion
repair occurred. The source review preceded execution and reported no finding.

The [compact evidence](evidence/agent-tilemap-paint/manifest.json) retains raw
test/lint logs, the producing source freeze, exact plan/runner, terminal cleanup,
and the independent root review. Run its data-only verifier with
`python3 docs/spikes/evidence/agent-tilemap-paint/verify.py`.
`--originals` additionally checks external records and the full 23,455-file
producing manifest; `--binaries` rehashes the four external copied executables;
`--source-root PATH` checks the listed implementation/test/support bytes. The
archive contains no binaries and does not rerun native code.

The generated `schemas/agent-tools.json` update remains reserved for root's
combined Wasm integration and must be generated/checked before publication.
This packet proves CPU authoring/agent-loop behavior with in-process mock
providers. It does not establish live provider, editor UI, GPU, window or capture
behavior. No runtime or public-shipping dependency change is introduced.
