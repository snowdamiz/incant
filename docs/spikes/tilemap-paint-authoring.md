# Shared tilemap painting and cardinal autotiles

Source preparation only; CPU qualification has not run. This is a shared authoring
service and headless command, not an editor painter UI or a completed Phase 2 gate.
Existing maps, cooked atlas format, rendering, collision generation and shipping
runtime are unchanged. No rule interpreter or authoring dependency enters gameplay.

`ImportSnapshot::prepare_tilemap_paint` reads the selected existing `Tilemap` and
stable layer ID and checks the registered texture's cooked digest, atlas metadata
and every existing map frame. Source images and atlas descriptors are not reopened.
The snapshot's expected revision must match. Preparation leaves the command bus
unchanged and exposes immutable commands plus a cell-change preview. Commit checks
the complete project and revision, then uses one ordinary `SetComponent` transaction
with the caller's actor, provenance and exact Undo/Redo/journal behavior. Deleting,
reimporting or otherwise editing the project before commit refuses the entire edit.
The caller owns any approval/cancellation boundary before calling commit; this adds
no separate mutation authority or agent shell access.

A stroke selects 1–4096 distinct, in-bounds `[x,y]` cells. `(0,0)` is the existing
map's local bottom-left, +X right, +Y up. Entity transforms, cell size, layer identity,
ordering and solidity are preserved. Existing map limits remain 32 layers and
262,144 total slots. Filling the first solid cell is validated against the existing
planar transform/collision constraints before returning a preview. The tool does
not create maps/layers, resize them, translate screen/world coordinates, flood fill,
or change assets. Missing frames, invalid requests and unknown fields refuse.

Three brushes are supported:

- `{"kind":"paint","frame":"<stable atlas frame ULID>"}` replaces exactly the selected cells.
- `{"kind":"erase"}` clears exactly the selected cells, including foreign terrain.
- `{"kind":"autotile","action":"paint|erase","frames":["<frame for mask 0>","... fifteen more ..."]}` applies one complete cardinal rule table.

For autotiles, the sixteen outputs define one terrain family; a frame may serve
multiple masks. Connectivity includes only cells whose frame is in this table,
within the selected layer. Mask bits are **north=1, east=2, south=4, west=8**;
map edges and foreign frames are empty. Diagonals and other layers do not join.
Painting replaces selected foreign frames explicitly. Autotile erase refuses a
selected foreign frame instead of silently destroying another terrain; use ordinary
erase for that operation. Empty selected cells are allowed for either action.

The stroke updates all selected occupancy simultaneously, then retiles selected
cells and cardinal neighbors against that final occupancy. Table outputs remain
in the same family, so enumeration order cannot change connectivity. An unchanged
selected cell can still repair a neighbor's frame. No recursive propagation occurs.
Rules are operation inputs, not saved/live links: later plain paint/erase does not
apply an earlier table. Retiling another region requires another explicit stroke.

The preview/result lists actual before/after changes in bottom-to-top row-major
order, bounded by 5 × 4096 cells (and existing map size). A stroke with no changes
returns `changed:false`, creates no transaction, does not increment revision or
clear Redo, and does not save the file. Stale no-ops still refuse. Successful changed
results include prepared/current revision, project/scene/entity/layer IDs, atlas ID
and checked texture recipe/content hashes.

## Headless host

`incant_headless tilemap-paint PROJECT REQUEST.json [--cache DIRECTORY]` reads one
regular, non-symlink JSON request of at most 1 MiB. The strict request contains:

```json
{
  "scene_id": "<scene ULID>",
  "entity_id": "<map entity ULID>",
  "layer_id": "<layer ULID>",
  "expected_revision": 7,
  "cells": [[0, 0], [1, 0]],
  "brush": {"kind": "paint", "frame": "<frame ULID>"}
}
```

The CLI uses the ordinary journal-backed project open, prepares/commits as user
`cli`, saves through the existing project-file API, then prints JSON. Save failure
retains the recoverable journal transaction and returns failure; it does not claim
whole-file rollback. No-op/refusal paths do not save or modify existing project and
journal bytes. A host-granted cache override has the same meaning as import's
existing `--cache`; there are no root/cache paths in the shared request itself.

## Intended CPU checks

The focused import suite covers all sixteen masks, boundaries, diagonals, foreign
terrain, repeated output frames, simultaneous order independence, neighbor repair,
4096-cell admission, malformed rules/coordinates, atlas/digest/decoder refusals,
first-solid planar constraints, stale identity/revision, source-free operation,
provenance, exact Undo/Redo and save/journal reopening. The real headless executable
test checks a persisted autotile stroke, JSON receipt, Undo/Redo, no-op file identity
and stale/duplicate/malformed/oversized refusal. Tests use temporary local assets;
no renderer, GPU, native window, browser, capture, device audio or network is used.

Planned qualification is scoped import and headless tilemap tests plus strict
all-target Clippy for those packages with pinned Rust 1.99, CI profile, locked/offline
and two jobs. It remains unexecuted pending a separately coordinated CPU target.
