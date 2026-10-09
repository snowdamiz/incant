# Phase 1: model and texture import

2026-10-09. These are the first asset-pipeline increments after the director
approved Phase 1. The full pipeline, runtime and phase gate remain incomplete.

## Implemented path

`incant_assets` imports static glTF 2.0/GLB models, including PNG/JPEG material
textures, on the CPU. It retains scene roots, node hierarchy/local transforms,
material bindings and standard material JSON. Triangle vertices carry position,
normal, UV0 and tangent data. Missing normals produce flat shading; normal-mapped
meshes without tangents use MikkTSpace, retaining tangent seams. Meshopt 0.6.2
optimizes and compresses vertex/index streams.

The same image can have separate color, numeric and normal-map variants. glTF
base-color/emissive textures use sRGB; metallic/roughness/occlusion use linear
values; normal maps average and renormalize vectors. Sampler filtering and wrap
modes survive cooking. External files, data URIs and buffer-view images work.
Changing an external image or buffer invalidates its parent model.

Standalone PNG, JPEG and single-part non-deep RGB(A) EXR import is also available:

```sh
tools/cargo run -p incant_headless --release --locked -- import /path/game.incant.json models/prop.glb
tools/cargo run -p incant_headless --release --locked -- import /path/game.incant.json textures/rock.png --texture-usage normal
tools/cargo run -p incant_headless --release --locked -- import /path/game.incant.json textures/light.exr
```

The source path is relative to the project file. Texture usage is `color`, `linear`
or `normal`; new assets default to color. PNG/JPEG color inputs are interpreted
as sRGB and EXR as linear RGB. Sixteen-bit images and EXR use RGBA32F without
clipping HDR values. Eight-bit inputs use RGBA8 sRGB or UNORM. A full mip chain
uses an area-weighted filter, including odd-sized edge texels. Color filtering
runs in linear light with premultiplied alpha to avoid transparent-color fringes;
output alpha is straight. Normal maps renormalize after averaging.

The cooker produces lossless, uncompressed KTX2 with a Khronos data-format
descriptor, top-left orientation and all mip levels. These are valid uploadable
texture formats; mobile/desktop block-compressed cook tiers remain to implement.
The writer follows [KTX2's level-index, alignment and descriptor rules](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html).
Custom ICC/chromaticity conversion, cubemaps, texture arrays and layered/deep EXR
are not implemented. The importer explicitly rejects multipart/deep EXR.

## Cache and authoring

The default model cache is `.incant/cache/models`; textures use
`.incant/cache/textures`. `--cache` overrides the directory. Version 2 `.incmodel`
files contain metadata, meshopt streams and KTX2 images. The loader still accepts
version 1 model entries. Standalone texture manifests refer to content-hashed
`.ktx2` files. Runtime CPU loaders work without source files; renderer/ECS loading
and automatic hot reload are still pending.

Keys include importer versions, dependency paths/hashes and texture usage. Model
cache hits currently reparse geometry and image sources; avoiding that work is a
remaining optimization. Binary assets stay outside project JSON. Bounded lengths,
checksums and atomic writes protect cache entries; import rebuilds corrupt data.
Old entries remain available for undo. Retention/garbage collection is pending.

`UpsertAsset` and `RemoveAsset` go through the shared atomic command bus. Reimport
retains the asset ULID/name; unchanged imports add no history. Changed imports
and settings are undoable across process restarts. Texture usage is a typed
`import_settings` field in the project document, so deleting/rebuilding the cache
does not lose it. Older documents without that optional field remain valid.
Generated project/command/tool schemas and TypeScript bindings include the field.
No editor or provider-specific mutation path was introduced.

Local source resolution permits sibling files inside the project and rejects
network URLs, path escapes and resolved outside symlinks. Canonical dependency
names use `/` on all hosts. Initial limits are 128 MiB of source/model data, one
million vertices, three million indices, 4,194,304 texels per image and 8,192
pixels per dimension, with additional object-count bounds. These are not measured
streaming/performance budgets.

## Verification

Local checks pass:

- Eight geometry/cache tests, including GLB, embedded buffers, external-resource
  changes, source-independent loading, version 1 compatibility, malformed data,
  path confinement and hard-edge normals.
- Eight texture tests cover color/data separation, alpha fringes, odd dimensions,
  normal-vector filtering, JPEG, 16-bit PNG, HDR EXR, malformed/truncated KTX2,
  cache repair and textured-model bindings, tangents and image dependencies.
- Four asset-command tests and two separate-process CLI tests cover stable IDs,
  atomic references/removal, undo/redo, usage changes and cache rebuilds. Existing
  eight transaction, seven document and two headless evaluation tests also pass.
- Affected-crate Clippy with warnings denied, formatting, generated conventions/
  schemas/SDK and strict TypeScript compilation pass.
- `python3 tools/validate_ktx.py` generated sRGB, linear and HDR fixtures and
  validated all three with Khronos KTX-Software 4.4.2, with zero diagnostics.
  The script verifies the pinned package SHA-256, extracts tools into a temporary
  directory without installation, and now runs in the source CI workflow.
  [Reference-validator evidence](evidence/ktx-textures-2026-10-09.json).

For initial revision `472cdcc`, Linux desktop, all six platform probes and all
three credential jobs passed. Windows exposed a native-separator bug in source
resolution: `PathBuf` backslashes were being treated as invalid filename
characters. The fix checks path components and has a native-path regression test.
Hosted validation of the fix and texture increment is pending. No local browser,
renderer capture, screenshot, account or credential access ran.

## Still open

FBX conversion, WAV/OGG import, target block compression, streaming, automatic
file watching/runtime reload, renderer/ECS integration, and editor/agent import
entry points remain. Animated/skinned glTF, morphs, sparse accessors, extensions,
non-triangle primitives, cameras, vertex colors and UV sets beyond UV0 are still
rejected explicitly. The rest of Phase 1 and Core Sample remain open in PLAN.md.
