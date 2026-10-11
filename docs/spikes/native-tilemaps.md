# Native tilemap data and collision preparation

This slice adds typed tilemap authoring, native binary cooking, retained local
placements and exact solid-cell collision preparation. Rendering and a Rapier2D
consumer remain unimplemented and are refused explicitly. It adds no legacy
simulation, renderer, shader, painter or GUI-only mutation path.

## Data and bounds

`incant_2d::Tilemap<I>` holds one atlas texture, rectangular dimensions, finite
cell dimensions and stable-ID layers. `incant_doc` specializes IDs to canonical
ULID strings; `incant_runtime` uses 16-byte `StableId` values. Each layer has an
exact bottom-to-top row-major array of optional frame IDs, explicit layer/order,
and a `solid` flag. All project changes use the existing `SetComponent` command,
validation, provenance and undo/redo. MeshRenderer, SpriteRenderer and Tilemap
are mutually exclusive on an entity; Tilemap requires Transform and a color
texture asset.

The bounds are 1–32 layers, 1–4096 cells per axis, at most 262,144 cell slots per
map, 1,024 map entities and 1,048,576 cell slots per scene. A slot counts even
when empty. Each cell dimension is finite in [0.000001, 1000000] world units;
each map extent is at most 1,000,000. Scene and structural validation enforce the
same aggregate bounds. There are no new dependency versions or feature changes.

Cell (0,0) begins at the entity's local origin; +x points right and +y points up.
Cell dimensions determine local bounds. Atlas image rows remain top-down, and
packing offsets or sprite pivots do not change grid or collision positions.
This is a data contract; no tile appearance has been implemented or accepted.

## Binary and retained boundaries

Scenes containing Tilemap use version 6. Scenes without it keep their existing
version 1–5 encoding. The v6 spatial bit adds a bounded map header followed by
layers in canonical stable-ID order. Each cell is an empty/present tag with a
binary frame ID when present. The reader checks counts, remaining input,
aggregate slot budgets, duplicate/order constraints, flags, numeric ranges and
asset references before publishing a scene. The existing scene checksum detects
corruption; it does not authenticate untrusted content.

At load or accepted configuration change, `PreparedTilemap` creates private
local placements sorted by (layer, order, stable layer ID, y, x) and collision
rectangles. Occupied cells from all solid layers are unioned, horizontal runs are
merged vertically when identical, and the resulting half-open integer rectangles
cover each solid cell exactly once. This is a deterministic decomposition, not a
claim of minimum rectangle count. Placements and rectangles are bounded by cell
slots; row-run scratch is bounded by map width. Binary IDs remain inline.

The committed spatial view lends these immutable slices alongside the existing
f64 world transform. Motion, no-op configuration, reparenting and component
archetype relocation retain the buffers. Rejected spatial/topology batches
preserve prior storage, generations and simulation eligibility. Preparation is
not repeated each tick; no document, JSON or authoring dependency enters the
runtime graph. Existing spatial-world save refusal is retained.

`TilemapAtlasBinding` is a separate checked CPU startup binding. It matches the
binary descriptor's ID/kind/recipe to a sealed cooked color atlas, resolves every
used frame and retains that exact asset version after source/cache deletion. Its
single-asset decoded payload is limited to the existing 256 MiB bound (the reader
may stage one bounded asset before the aggregate check). Recipe fingerprints are
cache identities, not authentication. This binding does not grant tilemap
support to the renderer.

The existing import and cached-validation paths now protect Tilemap references
alongside Sprite references. Diagnostics retain the structured sprite-reference
contract, with a precise Tilemap path and one issue per entity/texture/frame.
A rejected import batch changes neither the asset registry nor command journal;
content-addressed cache blobs written before rejection may remain as harmless
orphans. Cached validation works with source files removed. Accepted replacement,
failed replacement, undo and redo use the same transaction path.

## Explicit unsupported behavior

Native and authoring render adapters reject Tilemap. A native world with any
prepared solid-cell coverage rejects `step` before motion or tick publication;
a maintained ID set avoids scanning every map per tick. Removing that capability
through an accepted structural batch allows simulation again. Nonsolid maps may
retain and move CPU data, but cannot render. Legacy Engine startup and sync share
a capability refusal before changing the running world; static authoring
transform inspection remains available.

No arbitrary polygon/partial-cell shapes, per-cell collision properties,
autotiling, multi-tileset layers, editor painting, GPU geometry, Rapier2D solver,
shipping tilemap player or appearance/performance gate is claimed. Those require
actual consumers of this data rather than another document-backed implementation.

## Executed checks

On macOS 26.6 (25G72), arm64, Rust/Cargo 1.99.0, 192 scoped release tests/doctests
passed across incant_2d, runtime, doc, cmd, core, cook, import and the native and
authoring render adapters. Scoped all-target release Clippy passed. After review
moved the legacy refusal to shared sync preparation, all 13 core tests and its
all-target Clippy passed again. The headless schema tool built, generated the
Tilemap schema and matching SDK declarations, and strict SDK typechecking,
generated-file checks, conventions and formatting passed. The resolved normal
graph for incant_2d, incant_runtime and incant_render_native, explicitly enabling
runtime visual assets, remains authoring-free.

Coverage includes all 512 binary 3×3 solid-cell patterns with an independent
per-cell coverage oracle, overlapping layers, a maximum 512×512 checkerboard,
map/scene caps, malformed/truncated v6 records, historical source-free v1–v5
fixtures with byte-exact re-encoding, retained buffer identity, invalid atomic
batches, shared frame reimports, source-free cached atlas binding and both legacy
startup/sync refusal. No GPU execution was needed or claimed for this slice.
Exact source and log hashes are in the [evidence](evidence/native-tilemaps-2026-10-10.json).
