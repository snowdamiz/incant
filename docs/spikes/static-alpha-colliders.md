# Static collider authoring from sprite alpha

This is a bounded authoring slice of the Phase 2 collider-from-alpha requirement.
It derives a set of fixed, ordinary `Collider2D` boxes from the selected simple
sprite's cooked base-level alpha. It does not complete general contour/compound
colliders, moving-body collision, automatic sprite linkage or GUI selection.
CPU qualification is pending; this document currently describes source behavior
and the proposed controls, not executed results.

## Shared command path

`ImportSnapshot::capture` captures the current command-bus document and revision.
`prepare_alpha_colliders` performs cache IO and geometry preparation outside the
bus lock. The caller can inspect the immutable derivation and ordinary
`CreateEntity` commands. `PreparedAlphaColliders::commit` checks the entire
captured document and revision, including texture asset identity, before one
`incant_cmd` transaction. A same-ID, same-revision replacement document is also
refused. The ordinary actor validation, provenance, command validation, journal,
Undo and Redo apply. No authored project content changes during preparation.

The host CLI uses this shared path and the ordinary Save operation:

```text
incant collider-from-alpha PROJECT SCENE_ULID SPRITE_ULID --threshold 128
```

The optional host `--cache DIRECTORY` has the same explicit authority as import's
cache override. It is not a project-supplied filesystem capability. This delivery
does not expose engine-agent shell access or add a dedicated agent tool. Shared
Rust callers can provide an ordinary user/import/agent actor; resulting edits are
the same commands available to all authoring clients.

Each invocation adds one set of independent root entities, with new stable ULIDs
allocated once during preparation. They are fixed because they have a
`Collider2D` and no `RigidBody2D`. Their names are `Alpha collider N`; the command
transaction description binds source entity, cooked key, content digest and
threshold. The receipt additionally records the selected region and exact
texel rectangles. The original sprite remains unchanged. Undo/Redo and journal
replay preserve the exact generated IDs and geometry, without decoding again.
Save is a separate durable checkpoint; a Save failure is reported and does not
erase a successfully committed recoverable journal transaction.

## Geometry contract and refusals

- Only an unlinked root entity containing exactly `Transform` and
  `SpriteRenderer` is admitted. Parented entities, prefab links, scripts,
  flipbooks, velocity, rigid bodies, existing colliders and other components are
  refused. This restricts the tool to a static authored snapshot; other gameplay
  systems could still move the source later, and there is no automatic linkage.
- Only simple sprites with tint alpha 1 and unit scale are admitted. Rotation
  must be a unit planar quaternion (X/Y components zero, squared norm within
  `1e-4` of one). The mapping uses the normalized planar rotation, matching
  native world-transform composition; output quaternion and Z position are
  preserved. Existing document/physics validation also applies to all results.
- The selected pixel rectangle or stable atlas frame is resolved explicitly.
  Atlas padding and other frames are not included. A pixel rectangle explicitly
  spanning atlas padding remains the caller's selected region. Local dimensions
  determine metres per texel; bottom-left pivot, source top-left row order,
  X/Y flips and planar rotation determine each world-space box center.
- The existing source-free cooked loader verifies recipe key, content digests,
  metadata, dimensions and complete mip layout. Those hashes are corruption and
  identity checks, not authentication. Only color `Rgba8Srgb` is admitted;
  high-precision/float, linear and normal textures refuse. Exactly base mip alpha
  bytes are used. RGB, filtering, sampling, display blending and lower mips are
  not collision coverage. Missing/corrupt cache data never triggers source
  reimport.
- The inclusive test is `alpha_byte >= threshold`, where threshold is 1..255.
  A fully transparent mask refuses without an edit. A fully opaque mask produces
  one box. Rectangles are selected in row-major order, extended right then down
  through unclaimed solid texels. This is deterministic, exact and nonoverlapping,
  but does not promise a minimum rectangle count. Holes and disconnected islands
  remain empty; there is no bounding-box or contour simplification fallback.
- The selected region has at most 1,048,576 texels and the cover at most 64 boxes.
  A more complex mask refuses, even if a different cover could use fewer boxes.
  Temporary mask storage is at most one byte per selected texel, plus bounded
  rectangles/commands. This excludes the ordinary project snapshot/candidate
  copies and the existing cooked loader's bounded payloads (at most 4 Mi texels,
  8192 per axis, 128 MiB serialized/combined color payload limits).
  Existing box half-extents must remain 0.001..10000 metres, with ordinary
  fixed-collider material defaults. No sub-minimum dimension is clamped.
  The transaction adds at most 64 commands, below the bus's 10,000-command
  limit. There is no new general scene/entity authoring cap. Native cooking and
  the planar solver retain their separate total entity/body/shape limits;
  preparing this edit does not qualify an entire scene for runtime admission.

Moving, deleting or reimporting the source afterward leaves the authored
colliders as they were. Re-running adds another set. The caller must explicitly
Undo/delete or regenerate as appropriate. This is not dynamic or live collision
editing and provides no rendered appearance or runtime performance claim.

## Planned CPU qualification

The source includes exhaustive 3x3-mask occupancy checks and real PNG/atlas
imports with known alpha, exact cover/threshold checks, cropped/rotated/flipped
geometry, actual planar physics queries through solid and empty texel centers,
and exact history/provenance/Undo/Redo/Save/reopen observations. Refusal controls
cover transparent masks, threshold zero, pixel/rectangle bounds, unsupported
source transforms/components/prefabs/modes, missing atlas frames, high precision,
wrong-digest/missing cache, matching-digest malformed KTX, invalid actor, stale
asset/document/revision, deleted selection and a replaced project with an equal
revision. CLI grammar controls require explicit selection and the same threshold
range. A separate real `incant_headless collider-from-alpha` child checks saved
JSON/document correspondence, journal ownership, CLI provenance, reopen and
exact Undo/Redo, with a subsequent source refusal preserving project/journal
bytes. Its texture source is deleted before invocation, so the CLI must use the
verified cooked asset. Existing import suites and strict scoped lint
remain part of the proposed qualification. No native command, window or GPU
execution has been performed for this source yet.
