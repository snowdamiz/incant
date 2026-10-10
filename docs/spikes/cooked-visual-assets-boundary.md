# Cooked visual asset reader boundary

2026-10-10, Astra. Based on reviewed sprite/atlas head
`3e0cbcbb0254a0ccab51e9a20a81cc1554f6daeb`. This is the first native spatial/render
prerequisite: reuse existing cooked readers without importing authoring code.
It does not implement native Camera/MeshRenderer cooking, renderer extraction,
pixel placement, or a new gameplay path.

## Module split

`incant_assets_runtime`, under its additive `visual` feature, now owns:

- `mesh`: Mesh/Vertex values, their validation and the existing bounded meshopt
  decoder. Vertex/index counts, byte lengths, digest and decoded indices/floats
  retain their checks.
- `texture` and `ktx`: Texture/TextureFormat/TextureUsage, extent/mip/float checks
  and the existing KTX2 decoder. Format/layout, orientation, DFD, mip byte counts
  and all image bounds retain their checks.
- `model`, `model_node`, `model_textures` and `material`: CookedModel and metadata,
  node and image-role values, typed materials, node-graph/reference validation,
  version-1/version-2 model reads and startup material resolution.
- `texture_cache`: CookedTexture/TextureMetadata, the unchanged ordinary-v1 and
  atlas-v2 key recipes, bounded metadata/KTX reads, checksum and atlas validation.
  AtlasLayout/AtlasError remain the exact types from the already authoring-free
  `incant_2d`; atlas packing is unchanged.

`incant_assets` retains source snapshots, glTF/image import, mesh optimization and
encoding, KTX encoding, mip generation, atlas cooking/extrusion, cache writes,
Project-driven AssetStore synchronization and immutable version publication.
Its existing exported names reexport the same consumed value types. Reader free
functions are thin error-converting compatibility wrappers; decoding is not
duplicated. Imported-model conversion remains in the authoring crate. No renderer,
audio source/test, CLI, schema, shader, visual evidence or pixel-marker file changed.

The shared validators needed by the cook now have documented public entry points.
The newly public in-memory model decoder also checks total bytes and key syntax;
the shared node validator checks child indices before indexing. These checks were
previously supplied by their containing reader/importer and are now explicit at
the new call boundary. No unsafe code was added.

## Compatibility and limits

Existing public reader functions in `incant_assets` keep their authoring
`AssetError` return type and categories/messages, including typed Atlas and
Unsupported errors. **Source-level qualification:** methods on the relocated
Mesh/Texture values now return `incant_assets_runtime::AssetError`. Normal `?`
callers with the authoring error still work through `From`; a caller explicitly
annotating or matching that method's former error type must convert it. The one
internal tail-return caller does so explicitly.

Public consumed-value fields retain their previous mutability. This extraction
does not turn them into sealed, permanently validated capabilities. Existing
AssetStore consumers still retain immutable published versions through Arc;
moving that Project-bound store and preparing native rendering remain future
work. Cooked metadata still parses once at load, never on a gameplay frame path.
Content digests detect data corruption and are not authentication or executable
trust. No cache format, fingerprint recipe or source dependency policy changed.

The visual feature adds optional edges to already pinned `meshopt 0.6.2`,
`ktx2 0.5.0`, `incant_types` and `incant_2d`. `incant_assets` enables it, so normal
workspace tests exercise the moved readers. Default `incant_assets_runtime`
consumers retain the audio-only dependency graph. Cargo.lock changes only those
four edges; no package version/resolution changed.

## Verification

All compile/test commands used Rust 1.99.0 and `-j2`; no source changed while a
Cargo build was running. Final commands:

```sh
./tools/cargo test -j2 -p incant_assets -p incant_assets_runtime --features incant_assets_runtime/visual --release --locked
./tools/cargo test -j2 -p incant_assets_runtime --no-default-features --release --locked
./tools/cargo clippy -j2 -p incant_assets -p incant_assets_runtime --features incant_assets_runtime/visual --all-targets --release --locked -- -D warnings
./tools/cargo clippy -j2 -p incant_assets_runtime --no-default-features --all-targets --release --locked -- -D warnings
./tools/cargo fmt --all --check
python3 tools/generate_conventions.py --check
git diff --check
```

- Explicit visual suite: **40 passed**, including all prior importer/cache,
  atlas, retained-version and audio tests; both Clippy configurations pass.
- Default audio-only suite: **3 passed**; feature-gated visual tests are absent.
- Five new cross-crate tests compare actual cook outputs, decoded types and
  authoring/runtime error categories. They remove glTF/buffer/image/atlas sources,
  load both model versions and ordinary/atlas texture manifests, retain optional
  old manifest defaults, exercise all three supported KTX formats and every
  truncated offset, reject corrupt hashes/counts/references/cycles/material
  extensions/atlas metadata and reject symlink cache entries on Unix.
- Two runtime-only tests exercise malformed data, mip/float/dimension checks and
  the exposed node validator without an importer dependency. The two existing
  checked-material tests moved with their implementation and pass.

Normal graph checks used separate `cargo tree --locked -p incant_assets_runtime
-e normal --prefix none --format '{p}'` invocations with `--no-default-features`
and then `--no-default-features --features visual`. The default tree has 23 unique
packages including its root and none of the four optional additions; visual has
43. Both exclude incant_assets/incant_core/incant_doc/command/agent/CRDT packages
and gltf/image/exr/symphonia/mikktspace importers. Explicit visual Cargo metadata
also passes the repository guard's traversal of all normal dependency edges.
The existing workspace guard covers this feature because incant_assets enables it.

Local logs: `/tmp/incant-visual-assets-feature-tests.log`,
`/tmp/incant-visual-assets-audio-default-tests.log`,
`/tmp/incant-visual-assets-clippy.log`,
`/tmp/incant-visual-assets-default-clippy.log`; dependency inventories are
`/tmp/incant-visual-assets-{default,feature}-tree.txt`.

These are scoped desktop reader and dependency checks. They do not claim a fresh
shipping renderer executable, live mobile/device performance, native spatial
rendering, legacy Play migration, visual acceptance or the pixel contract complete.
