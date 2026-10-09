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
and automatic source watching are still pending.

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

## Runtime versions

`AssetStore` loads the project’s models and textures from cooked cache entries,
keyed by their stable document IDs. Sync stages the entire next asset set before
publishing it. Missing or corrupt files, unsupported asset kinds, mismatched
texture settings and payload-budget failures leave every previous version intact.
An unchanged asset retains the same allocation and generation. Replacement and
remove/re-add assign increasing generations; consumers holding an `Arc` can
finish using an older immutable version safely. Document undo/redo can therefore
restore an earlier cooked version without accessing source files.

The default published payload budget is 256 MiB, covering decoded vertices,
indices and texture mips. Metadata, allocator overhead, staging and older versions
retained by consumers are additional memory; this is not a process-memory ceiling
or measured streaming budget. Unknown runtime asset kinds fail explicitly.

`incant run PROJECT` now loads this store from the project’s `.incant/cache` before
starting simulation and reports asset IDs, content fingerprints, generations and
payload bytes alongside its state. Missing cooked content fails before ticks run.
This is CPU loading and version management. Binding these versions to GPU/ECS
instances and automatically watching/reimporting changed source files remain open.
No document edits bypass the shared command bus.

## Verification

Local checks pass:

- Eight geometry/cache tests, including GLB, embedded buffers, external-resource
  changes, source-independent loading, version 1 compatibility, malformed data,
  path confinement and hard-edge normals.
- Eight texture tests cover color/data separation, alpha fringes, odd dimensions,
  normal-vector filtering, JPEG, 16-bit PNG, HDR EXR, malformed/truncated KTX2,
  cache repair and textured-model bindings, tangents and image dependencies.
- Four runtime-store tests cover source removal, immutable retained versions,
  reimport/undo/redo, atomic failed batches, removal/re-add, payload budgets,
  settings mismatches, corrupt replacements and unsupported kinds. The geometry
  test also loads a cooked model through the store.
- Four asset-command tests and two separate-process CLI tests cover stable IDs,
  atomic references/removal, undo/redo, usage changes and cache rebuilds. The CLI
  test runs a model after deleting its source, verifies loaded geometry bytes and
  checks missing cooked data fails without editing the project. Existing
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
The combined import revision `aed4cbe` passed source checks, Windows/Linux native
editor builds and renderer probes, all three credential-storage jobs, all six
platform probes, and website checks. [Exact run evidence](evidence/asset-import-2026-10-09.json).
PR #3 merged into main as `acc123b`. Those platform probes exercise the existing
platform workload; they do not claim asset GPU/ECS integration on six targets.
No local browser, renderer capture, screenshot, account or credential access ran
as part of the asset-import checks. The later runtime-version increment remains
separately locally verified pending hosted integration.

## Still open

FBX conversion, WAV/OGG import, target block compression, streaming, automatic
file watching/runtime reload, renderer/ECS integration, and editor/agent import
entry points remain. Animated/skinned glTF, morphs, sparse accessors, extensions,
non-triangle primitives, cameras, vertex colors and UV sets beyond UV0 are still
rejected explicitly. The rest of Phase 1 and Core Sample remain open in PLAN.md.

Runtime asset loading and handoff 0008 polish merged in PR #4 on 2026-10-09.
All thirteen hosted checks passed on c063a99, including Windows/Linux editor
builds and tests, three credential stores, all six platform probes and site
browser tests. The exact revision and jobs are retained in
[evidence/runtime-assets-2026-10-09.json](evidence/runtime-assets-2026-10-09.json).

## Shared import preparation and commit

`incant_import` is the authoring service between `incant_assets` and `incant_cmd`.
`ImportSnapshot::capture` copies the current document and revision; preparation
can then move to a worker without holding the editor's command-bus lock. A batch
of one to 64 canonical project-relative sources validates identities/options and
cooks every input before returning an immutable prepared batch. Commit verifies
project identity, revision and contents, then issues all changed `UpsertAsset`
commands as one atomic transaction with the caller's validated provenance.
Unchanged imports do not add history, but still reject stale snapshots.

The CLI import path now calls this service and preserves its existing result
shape, cache override, stable ULIDs, user-renamed labels and texture usage settings.
Repeated separators and other noncanonical source paths now fail explicitly,
preventing textual aliases from creating separate source identities. Runtime
loading remains in `incant_assets` without an authoring command-bus dependency.

Five service tests cover worker preparation, an atomic persistent batch, undo/redo,
loading after source deletion, cache rebuild and saved settings, corrupt later
inputs, invalid provenance, stale revisions (including no-ops), different project
identities and same-ID/revision documents with different content, and malformed/
ambiguous sources. Existing real CLI import/reimport and runtime tests also pass.
Cooking can leave content-addressed cache files when preparation or commit fails;
no authored document or history entry is partially published. Editor and agent
import entry points, progress/cancellation and automatic source watching remain
open. This service is not claimed as a completed asset hot-reload feature.
