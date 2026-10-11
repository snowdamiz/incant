# Cooked skeletal and TRS animation foundation

This slice imports glTF skins and translation/rotation/scale clips, cooks their
actual data, and evaluates it through a source-independent CPU runtime. It does
not implement animated rendering or complete PLAN Phase 1 animation.

## Supported route

`incant_assets::cook_gltf` uses the existing source confinement, source recipe,
material, texture-role and cache transaction path. The existing public model
import dispatch calls this same cooker. `incant_assets_runtime::load_model`
loads the result; `animation::AnimationPlayer::new` makes a validated retained
copy and preallocates both live and staging poses. `sample(Some(clip), seconds)`
evaluates a clip, while `sample(None, seconds)` evaluates the authored rest pose.
No source, project, JSON, importer or authoring validation is used by sampling.

Supported glTF data includes:

- Complete node hierarchy including non-joint ancestors; affine static matrices
  without shear and authored TRS. A matrix node cannot be an animation target.
- Skin joint indices, optional skeleton root, inverse bind matrices (identity
  when absent), skin references per mesh node, and four JOINTS_0/WEIGHTS_0
  influences per vertex. U8/U16 joints and normalized integer or float weights
  are read by the pinned glTF reader. Finite nonnegative weights at most one,
  with positive total, are normalized once during import.
- Named clips containing LINEAR or STEP translation, rotation and scale
  channels. Times must be finite, nonnegative and strictly increasing; zero is
  a valid first time. Duplicate node/property channels refuse. Rotation keys
  must be near unit quaternions (squared-length tolerance 0.002) and are
  normalized before shortest-path spherical interpolation. Numeric T/R/S
  channels retain the glTF coordinate and time conventions.

See the [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html),
sections 3.7.3, 3.11 and appendix C. Finite supplied time clamps independently to
each channel's endpoints; the evaluator has no hidden loop or clock. STEP changes
at the exact next key. Negative scales are retained; singular skin matrices are
allowed, but conversion through a singular mesh transform explicitly refuses.

`joint_palette(skin)` returns model-space `global(joint) * inverse_bind` matrices.
The skinned mesh node's transform is ignored in this representation, as required
by glTF. `mesh_local_palette(node, caller_buffer)` optionally applies the inverse
mesh-node global matrix; a consumer must apply the mesh global afterward.
Multiple mesh nodes may share one skin without sharing an incorrect mesh-local
palette. `skin_position` is a CPU reference linear-blend position calculation
using the retained influences and a caller-supplied matching vertex position;
it does not deform normals/tangents or draw anything.

Normals and generated tangents can expand indexed vertices. Each expansion
applies the exact same source-index mapping to influences. Version-4 models use
the existing checked meshopt stream without the static vertex-fetch reordering;
static models retain their original optimization and bytes.

CUBICSPLINE, morph targets/weight animation, extra influence sets, glTF
extensions, cameras and sheared matrices in animated models explicitly refuse.
Existing static FBX scope is unchanged. Browser source cooking remains explicitly
unsupported; the optional visual runtime contains the portable reader/evaluator.

## Format and bounds

Static models continue to write INCMOD03 using the unchanged static recipe.
Readers retain INCMOD01/02/03 compatibility. Only models with a skin or clip write
INCMOD04, with the same metadata/geometry/texture sections followed by one
length-delimited INCANM01 binary section and the existing whole-file checksum.
Animated imports use a distinct `incant-gltf-skeletal-v1-incmod04` recipe. Future
shared import changes must update each affected recipe. Old immutable caches are
neither rewritten nor repaired silently. Fingerprints identify source recipes;
checksums detect corruption, neither authenticates untrusted asset bytes.

INCANM01 stores, in order: rest-transform tags and TRS, node skin references,
skins with joint indices and inverse binds, influence streams, then named clips
with node/property/interpolation tags and interleaved time/value records. Counts
and indices are little-endian u32; joint influence lanes are u16; all stored
numerics are finite f32. Optional indices/names use u32::MAX. Translation/scale
values have a checked zero fourth lane. Unknown tags, truncated collections,
trailing bytes, bad references, cycles, duplicate joints/targets, invalid skin
scene membership and numerical violations refuse before publication.

Aggregate limits for an animated model are 4,096 nodes, 4,096 scenes, 65,536
node/scene references, 256 skins, 256 joints per skin and 4,096 total joints,
256 clips, 8,192 channels, 1,000,000 keys and 1,000,000 influences. Names are at
most 1,024 UTF-8 bytes. Animation is at most 32 MiB serialized, within the
unchanged 128 MiB whole-model bound, existing 8 MiB metadata limit and existing
1,000,000-vertex/3,000,000-index geometry caps. Source preflight charges the
aggregate animation footprint and expanded vertex/reference counts before those
collections are created. Binary counts, minimum remaining record bytes and
aggregate counters are checked before each corresponding allocation. These are
payload/work bounds, not a universal process peak-memory or RSS proof; source
parsing, decoded model plus retained player copy, allocator overhead and texture
staging remain separate allocations.

A retained player copies bounded animation data and node topology at startup;
it does not clone geometry or image pixels. At the maximum node/joint counts,
its six live/staging matrix arrays total 3,145,728 logical bytes (f64 matrices),
plus TRS scratch, ordering/parent/offset arrays and retained data. Accepted sample,
mesh-local palette conversion and CPU position blending perform no allocation.
Staged evaluation checks composed matrices; invalid time/index or numerical
overflow preserves the entire last published pose. Invalid palette conversion
preserves the caller's output. This is repeatable same-build numerical evaluation,
not a cross-device bit-exact or shipping performance claim.

## Deliberate consumer boundary

`RuntimeAsset::load_model` explicitly refuses animated models because current
renderer consumers implement only static geometry. Direct cooked loading and
CPU sampling are available. `model_navigation_geometry` likewise refuses to
flatten animated geometry. Consequently an animated asset may be cooked and
registered by authoring, but current AssetStore/static preview/native render
binding cannot consume it. This is a temporary missing integration, not completed
skeletal playback in a native scene. No GPU pipeline, native world, checkpoint,
player, SDK, Wasm ABI or editor file changes are included.

Blend trees, state machines, IK, root motion, animation retargeting, GPU skinning,
normal/tangent deformation, native scene clip controls and SDK/export integration
remain later work. No rendered appearance, screenshot, device or phase gate is
claimed here.

## Verification

The local evidence JSON records exact source/log hashes and commands. Scoped
asset/runtime checks pass 89 tests across 16 result groups, with no failures or
ignored tests, including 14 skeletal tests. A fresh child process loads cooked
animation after both source files are deleted and computes `[12, 1, 0]` for the
fixture's half-time influenced vertex. This is an actual process/test executable,
not a shipped-player/export proof. Normal dependency guards separately verify the
runtime graph.

Controls cover mathematical translation/rotation/STEP/negative-scale palettes,
non-joint ancestors, shared mesh instances, missing inverse binds, skinless
clips, antipodal quaternions, endpoint clamping, source-buffer invalid times and
weights, structural/reference and checksum-valid binary refusals, generated
normal/tangent expansion, immutable cache versions, numerical publication
rollback and malformed-count refusal before allocation. Test-only thread-local
allocator instrumentation measures 1,024 accepted sample/conversion/blending
iterations at zero allocations; no production unsafe code is introduced. An actual malformed U32 rotation accessor reproduced an `unreachable!` panic
in the pinned glTF utility reader before the correction. Explicit semantic
component/dimension/normalization preflight now rejects it and seven other invalid
accessor cases; U8/U16 normalized weights and U8 joints have numerical controls.
Clippy
failures encountered during development were a needless encoder borrow and a
test modulo-style lint; both were corrected after their processes exited.

Evidence: [exact local checks](evidence/gltf-skeletal-animation.json).
