# Color mip roles

Status: unpublished implementation integrated with reviewed PR81. Scoped CPU,
Clippy and the full renderer numerical GPU suite pass. Claude Opus 5.5 / Max
appearance acceptance and integrated hosted checks remain pending. This is separate from the accepted PR77 generated-tangent correction.

## Defect and selected behavior

Claude0041 found alpha-zero color texels darkening in OPAQUE minification. The
same filter also loses alpha-zero emissive and Color environment radiance.
A shared image can supply both alpha-sensitive and alpha-independent consumers,
so changing the global Color filter or special-casing all-zero alpha is incorrect.

The cook distinguishes color space from alpha policy. OPAQUE base color and all
emissive maps average every RGB sample in linear light. MASK/BLEND base color and
sprites retain the existing alpha-weighted, straight-alpha mip chain. Both
policies preserve source RGB/alpha at level zero and area-average alpha; odd
extents include every texel. Linear and Normal processing is unchanged. No WGSL,
material factor, UV, tangent, draw-order or pipeline change is involved.

Models emit INCMOD03 with an explicit `color_unweighted` image reference in each
relevant texture binding. One glTF source/sampler can resolve both roles while
retaining its source texture and sampler indices. FBX allocates bindings per
(ufbx texture ID, role); its numeric indices are local to each cook and are not
promised stable across old/new recipes. The existing 768-image and 128 MiB
model/decoded-image limits still apply across all variants. FBX retains its
stricter 256-image limit. Referenced missing/wrong-format/out-of-range roles fail.

New standalone Color textures and atlases emit manifest version3. The primary
KTX remains the coverage chain for sprites. A required independent KTX digest
selects alpha-independent RGB for the environment preparation path. Both files
are bounded and checksummed before publishing a loaded version, their layouts
and full base bytes must match, and combined serialized/logical limits remain
128 MiB. Non-Color textures retain version1. Metadata hashes detect corruption;
recipe keys are not authentication.

## Compatibility and transaction boundary

Readers still accept INCMOD01/02 and texture manifest1/2 using exactly their
historical role choice. An immutable old cache/version is never silently fixed:
source-backed import/reimport through incant_cmd is needed to adopt new mips.
Undo and retained version handles can continue using the old appearance after
source removal. All four model/ordinary-image/atlas recipes are changed to
prevent a new cook from hitting a historical filtered result. No existing cache
blob is removed.

The native/authoring boundary is unchanged. Cooking and filesystem writes remain
in incant_assets; bounded readers and sealed versions remain in the visual
feature of incant_assets_runtime. Normal audio-only graphs get no new dependency.
No dependency resolution or user project/schema field changes are needed.

The public consumed `Texture.levels` value changes from `Vec<Vec<u8>>` to
`Vec<Arc<[u8]>>`. Individual levels are immutable and cheap to retain. An authoring
test/helper needing to mutate a descriptor can use `Arc::make_mut`; sealed
RuntimeAsset only exposes shared data. Source-level consumers must use `.as_ref()`
or deref rather than Vec-specific methods.

Import failures retain project, revision, journal bytes and history. Tests
exercise a valid large high-precision PNG whose two chains exceed the combined
bound after an earlier staged import; neither asset publishes, and Undo/Redo and
source-free loading of the old accepted version still work. Content-addressed
unreferenced cache blobs from an earlier staged member may remain, as before.

## Storage accounting

After complete layout/base equality validation, role variants share level-zero
Arc storage. Lower mip chains remain independent. Startup model loading uses a
bounded content index followed by byte equality to restore base sharing across
separate sampler/binding records; no frame hashes, document clones or recooking
are added. Hash collisions do not establish equality.

`RuntimeAssetInfo.payload_bytes` remains the conservative full logical payload
used by existing store/native binding caps. The new read-only
`resident_payload_bytes()` separately counts unique Arc allocation identities.
Equal bytes in distinct allocations count twice. Neither figure includes Rust
metadata, allocator overhead, transient decode staging, GPU storage or disk.
The loader temporarily holds serialized files and staged decoded bases before
sharing; the shared resident result is not a process peak-memory claim.

Real RGBA8 checkerboard cooks produced these exact payload bytes:

| Extent | Single chain | Two logical chains | Shared-base resident | Resident increment | Unique KTX disk |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1x1 | 4 | 8 | 4 | 0 | 252 |
| 2x1 | 12 | 24 | 16 | 4 | 568 |
| 4x4 | 84 | 168 | 104 | 20 | 760 |
| 1x8192 | 65,532 | 131,064 | 98,296 | 32,764 | 132,184 |
| 3x5 | 72 | 144 | 84 | 12 | 736 |
| 256x256 | 349,524 | 699,048 | 436,904 | 87,380 | 699,928 |
| 1024x1024 | 5,592,404 | 11,184,808 | 6,990,504 | 1,398,100 | 11,185,784 |

Each fixture's manifest is 397 bytes. KTX files include headers/alignment. The 1x1
chains have the same digest and occupy one file, although limits still charge
both logical roles. Other rows have distinct chains and two serialized bases.
Model GPU images for two distinct bound roles likewise remain separate textures;
CPU base sharing does not eliminate those GPU uploads. A standalone sprite uses
only its primary chain; environment preparation consumes independent radiance.
There is no universal percentage saving: thin chains have proportionally larger
tails than square chains.

## Verification

The final focused run passed 126 ordinary release tests/doctests across
assets/runtime/import/render (26 result groups), followed by release all-target
Clippy. The earlier narrative count of 134 was preliminary and is superseded by
this authoritative count from the final log; it is not additional coverage.
Logs: `/tmp/incant-color-roles-final-cpu-tests.log` and
`/tmp/incant-color-roles-final-clippy.log`. The real storage accounting test also
produced `/tmp/incant-color-roles-storage.log`. Two CPU-only capture-gate tests
passed, without launching a renderer.

Coverage includes mixed/all-zero alpha and HDR odd extents; one glTF image across
OPAQUE/MASK/BLEND/emissive and two samplers; one FBX texture across diffuse and
emissive under opaque and blended materials; recipe changes for both formats;
legacy model1/2 and texture1/2 source-free reads; mandatory modern role fields,
digests, formats, bad indices and mismatched bases; square/thin/odd allocation
identity; unchanged logical store budgets; and failed batch reimport/Undo/Redo.

After integration with reviewed PR81, 132 scoped ordinary ci-profile tests and
all-target Clippy pass. The full renderer numerical GPU suite passes 68 tests
(18 unit and 50 integration), including eleven comparisons across two new test
functions: eight opaque/emissive minification pairs against all-alpha-one RGB
controls, two coverage-role pairs, and a Color environment comparison. Wrong-color
controls make the oracle nonvacuous; retained source/cache-deleted replays pass.
This preserves the prewarming, flipbook and generated-tangent regression suites.
A subsequent whole-workspace ci-profile run passed 703 ordinary tests/doctests
(201 result groups; 73 ignored cases were not executed by that CPU command),
followed by workspace all-target Clippy. These runs reused a target directory
that serialized with the coordinator's independent build; no timing claim is made,
and subsequent color builds will use a separate target.
The integrated source is `02d1872f43a0aae7daf8df849f48ac0c9f442aff`; only STATUS
required manual merge resolution. Exact logs and source hashes are recorded in
[evidence/color-mip-roles-integrated.json](evidence/color-mip-roles-integrated.json). Claude0043 owns separate appearance acceptance. No phone, performance,
GPU allocation budget or whole-engine completion claim is made.
