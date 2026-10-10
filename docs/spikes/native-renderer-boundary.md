# Canonical renderer without authoring dependencies

`incant_render` now consumes typed `SceneInput` with sealed retained cooked assets.
Its normal dependency graph contains no Project, command bus, importer, CRDT or
agent. New `incant_render_data` owns the CPU input and scene error/stat types;
`incant_render_authoring` keeps project validation, JSON projection, current cache
grants, sprite frame lookup and diagnostic selection outside the renderer. Editor,
headless and authoring benchmarks use its extension trait. Model expansion,
lighting, camera conversion, materials, shadows and sprite rasterization remain
one shared renderer. Shader sources are unchanged.

Typed input independently checks bounded unique identities, finite affine
transforms, GPU numeric ranges, camera/light settings, texture usage and primitive
limits. Normal environment textures and non-Color sprite textures fail explicitly.
A startup input can outlive the authoring project, asset store and source files.
This is a startup/revision boundary: `prepare` still resolves model hierarchy and
allocates frame resources. It is not the retained moving-frame path or a claim
that native Play migration or pixel placement is finished.

## Exact loaded versions

Review found that the existing source-recipe fingerprint was insufficient as a
GPU cache identity. Two valid cooked payloads can carry the same recipe key; a
second model could then reuse incompatible primitive buffers from the first.
Sealed RuntimeAsset now receives an opaque process-local AssetVersionId once at
successful construction, using a checked monotonic atomic counter. Labels,
generations and recipe fingerprints cannot supply or reuse this identity.
Model plans and model/environment/sprite caches key the exact immutable loaded
version; sprite texture-budget accounting uses the same identity. Model ordering
retains recipe ordering, with exact identity separating recipe collisions.
Clones of one retained Arc reuse resources. Separately loaded copies can allocate
separate GPU resources even if bytes match; no content-hash deduplication or asset
authentication is claimed. Counter exhaustion fails explicitly; identities are
not persisted or serialized.

## Verification

On local Apple Silicon, pinned Rust 1.99.0, the release suite for render,
render_authoring, render_data, assets and assets_runtime with visual enabled
passes **68 ordinary tests/doctests and 54 native GPU tests**. Matching all-target
release Clippy passes with warnings denied. The actual resolved normal dependency
guard passes for render, render_data and assets_runtime; the CI guard includes
both renderer roots. No dependency versions changed.

The added valid two-cache/same-recipe regression has distinct primitive counts,
checks independent plans/resources and proves old GPU geometry remains unchanged
after new and combined scenes are prepared. Existing queued frame, material,
environment, shadow, camera, sprite and atlas regressions pass. Typed input tests
cover source/project deletion, duplicate IDs, malformed ranges/poses and usage
refusals. Independent Astra review caught the cache/usage issues and accepted the
fix. Compile/test setup mistakes were corrected before these final results.

Full editor/headless integration checks and retained native-frame integration
remain separate work. This scope does not claim phone performance, a shipping
GPU host, a visual redesign or phase-gate approval. Exact log/source hashes are in
`evidence/native-renderer-boundary-2026-10-10.json`.
