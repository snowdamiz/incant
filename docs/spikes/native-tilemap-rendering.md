# Retained native tilemap rendering

This extends the typed v6 tilemap data boundary with actual rendering through the existing sprite pipeline. One sealed `TilemapInput` owns checked color-atlas frame rectangles and compact occupied-cell records. Native startup and configuration rebuilds bind binary IDs; authoring preview converts validated ULIDs through the same constructor. Neither path changes project data outside `incant_cmd`.

Each occupied cell displays its whole atlas frame across the cell's local rectangle, with nearest sampling, white tint, no per-cell flips and unlit sprite shading. Local (0,0) is the lower-left corner and +y moves up; packed atlas image coordinates remain top-down. A map keeps its continuous f64 hierarchy transform, including rotation and reflection. This adds no render-origin snapping contract.

Painter order is (layer, order, entity ID, layer ID, y, x), with ordinary sprites and cells in the same ordered stream. Compatible adjacent items share existing sprite texture/sampling batches; interleaved content is never regrouped across painter-order boundaries. The authoring adapter's textual ULIDs and native adapter's binary-ID labels preserve the same stable ordering.

## Retention and atomic boundaries

A native map retains one scene node/world-matrix slot. `TilemapInput` exposes only shared data and sealed asset versions; motion checks its Arc identity without enumerating cells, copying maps, rebuilding geometry or consulting a project. Retained local vertex positions and immutable attributes share the sprite pipeline. CPU staging writes only preallocated numeric positions. Accepted order/content/asset/topology changes rebuild a complete candidate; callback rejection preserves prior input generation, poses and live GPU resources. Matching-value bindings constructed as a different Arc deliberately require rebuilding.

The startup preflight first checks all maps/cell slots and the minimum six vertices per visible item, then calculates the exact existing 64-source-texel nearest-span expansion. Both counts combine sprites and tile cells. The 128 MiB logical geometry cap and device buffer limit are checked before expanded vertex allocation or texture upload. The 256 MiB texture allocation cap counts unique sealed asset versions across both kinds. Map limits remain 1,024 maps, 262,144 slots/map and 1,048,576 slots/scene, including empty slots.

Retained geometry has the existing 116 bytes per vertex across shared immutable attributes/local positions plus two CPU/GPU position streams: at most 232 MiB at the 128 MiB logical cap, excluding textures, binding and draw metadata, allocator/driver overhead and staging. Compact startup draw records and one map binding per revision are additional bounded storage. Preflight has only at most 54 base vertices per item; expanded tessellation scratch is bounded by one sprite/frame at a time. Exact contiguous-span capacities avoid geometric growth of retained local arrays. Structural replacement can temporarily retain both old and candidate resources. These are bounds, not measured phone budgets.

## Executed correctness checks

On macOS 26.6 (25G72), Apple M5 Pro/Metal, Rust 1.99.0, 45 scoped ordinary release tests and matching all-target release Clippy pass. All 62 actual renderer GPU tests pass serialized. New v6 cases cook a real atlas and native scene, discard authoring/source/cache storage, and compare eight retained GPU frames with an independent ordinary-sprite oracle enumerating the raw cell declarations. They cover hierarchy motion, reflection, transparent ordering, content/order rebuild and rejected-target rollback. A separate test records zero allocations during 64 accepted CPU staging calls only; it does not claim allocation-free submission or driver work.

The CPU authoring/native parity regression compares actual bound frame rectangles, cell positions, layer order and f64 transforms after source/cache deletion. Wrong texture identity, unavailable frames and failed configuration replacement refuse before publication. A combined-cap case accepts 337,920 simple cells, then rejects adding 12,000 sprites before any expanded vertex allocation. Existing worst-case nearest expansion tests verify exact preflight counts for 8,192-square reflected/nine-slice frames. Normal renderer/data/native dependency graphs remain authoring-free, and no package resolution version changes.

## Scope and limitations

Claude Opus 5.5 at explicitly verified Max owns separate rendered appearance acceptance; numerical equality is not visual approval. Headless authoring preview uses the same canonical renderer, but its captures do not prove native game execution. The v6 native numerical tests exercise the actual cooked runtime path. Existing nearest-sampling high-magnification/two-axis tie and camera-following limitations remain unchanged.

A solid tilemap still refuses native simulation until the separate Rapier2D integration is complete; legacy Play startup/sync retains its explicit refusal. No tile painting, autotiling, multi-tileset layers, per-cell material modes, animation, lighting/shadows, partial-cell collisions, saved spatial world, interactive tilemap shipping proof, phone execution, performance gate or whole migration completion is claimed.
