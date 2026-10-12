# Native skeletal model consumption

This slice connects cooked INCMOD04 data to the canonical model renderer and the
retained native input adapter. Simulation owns playback; the renderer receives
only a committed `AnimationSample`. It does not implement blend trees, state
machines, IK, root motion, retargeting, or GPU skinning. Numerical GPU comparisons
now pass on the frozen consumer; Claude appearance
acceptance and the integrated authored-player proof remain pending. Neither
desktop check establishes reference-device performance.

## Binding and lifetime

`MeshInput.animation` selects a clip/time. Absent animation on an animated model
selects the authored rest pose. A sample on a static model refuses. Clip indices,
finite nonnegative time and canonical rest time are checked; sampling beyond the
selected clip's duration refuses. The underlying CPU sampler's independent
clamping API is unchanged.

At startup and configuration rebuilds, `NativeRenderInput` compares the complete
native clip-duration table, including unselected clips, with the sealed model.
The table's fingerprint must match the loaded version and every duration must
match the exact f32 bits of the largest final channel key. This binds clock
metadata to source-recipe identity, not authentication. Motion copies the native
sample into existing slots. The same numeric undo guard restores sample, pose
and sprite frame on rejection or unwind. No per-tick asset lookup, string/Arc
copy, document/JSON, hierarchy expansion or metadata validation occurs there.

The authoring adapter reads the typed initial sample at a project revision.
Animation without the selected MeshRenderer refuses, including sprite/tilemap
precedence. Native Play uses the native clock rather than this still projection.

## Geometry and publication

Each animated entity retains a sampler and deformed vertices. Mesh nodes without
skins use sampled node globals, including non-joint ancestors. Skinned vertices
use global-joint times inverse-bind matrices in model space, deliberately ignoring
the glTF mesh node frame. The outer native entity frame is applied afterward.
Indices, materials, textures and the immutable source version remain shared.

The CPU deformation computes linear-blend positions, inverse-transpose normals,
linear tangents and determinant-corrected tangent handedness. UVs stay unchanged.
Existing shaders perform their existing final normalization/orthogonalization.
Singular/nonfinite/overflowing blended bases refuse before frame publication.
A primitive with different per-vertex reflection signs refuses in this initial
consumer. A uniform skin reflection combines with the outer entity reflection
when selecting the existing material and shadow culling variants; tangent signs
are corrected only once for each transform. This does not attempt to repair
self-intersecting/folded geometry or infer a new topology from deformed triangles.

Live and staging scenes own separate CPU pose/vertex state and separate private
COPY_DST vertex/instance buffers. Sampling and bounds calculation precede frame
encoding; buffer writes occur only after successful validation/encoding and are
immediately submitted. Ordinary rejected frames retain the previous accepted
CPU/GPU state. Device/driver failures keep the existing terminal host contract.

## Resource scope

An aggregate 128 MiB admission cap accounts for two sampler payloads per entity,
live/staging instance and geometry rows, two CPU and two GPU vertex arrays for
every skin placement, and reserved mirrored batch/instance payloads. Geometry
rows reserve exact placement count. Sampler accounting includes copied animation
keys, node topology, names, clip durations and both local/global/joint pose sets.
Existing primitive and decoded-asset caps continue to apply; loaded asset logical
and resident payload reporting now includes animation data.

This is a bounded retained payload contract, not RSS or driver peak accounting.
Allocator/Arc allocation headers, startup traversal/plans, existing immutable
source/GPU models, frame uniforms/draw packets and concurrently retained old
candidates remain separate. No whole-renderer zero-allocation or phone budget
claim follows. Tests measure successful CPU sampling/fill and native extraction
separately; GPU/driver work is outside those counters.

## CPU checkpoint verification

The affected five-package suite passes 155 tests in 30 result groups, with 70
GPU tests initially unrun at that CPU checkpoint. Six-package all-target Clippy,
formatting and
explicit visual-feature normal dependency guards pass. The native source/cache
deleted test advances two independent clocks for 90 frames while numeric input
extraction records zero allocations; rejected/unwinding callbacks retain the
previous complete input. A separate 64-call deformation fill counter records no
allocations. CPU fixtures cover full normal/tangent/UV transformation, clip-only
non-joint hierarchy, static/sample refusal, byte-cap refusal before pose copies,
uniform reflected skin and mixed-sign refusal.

Two ignored numerical GPU tests are compiled: independent mathematical static
pose parity and single-sided reflected-skin/static-transform parity with an
observable shadow receiver. They were initially unrun at the CPU checkpoint.
The latter
was added for independent review's reflected-skin culling finding; the source
correction combines skin and outer-entity signs for both existing cull paths.

The Clippy Cargo command finished successfully; its shell wrapper then used a
read-only zsh variable and exited 1. Dependency/fmt checks were separately run
and passed. No test or Cargo failure is represented as success. Original
foundation and scene-root correction evidence remains unchanged.

[Exact CPU source/log receipt](evidence/native-animation-rendering-cpu.json).

## Numerical GPU follow-up

The unchanged consumer `cdd090324285c9822119ee2340ae00f56e94e375` passes all
70 affected GPU regressions: 20 unit and 50 integration tests, serialized on
Apple M5 Pro with the Metal backend. Both new animation cases also passed
individually before the complete run. All 3,521 tracked source hashes were
verified unchanged before release, and the executed copies have pinned binary
hashes. The earlier CPU evidence is preserved unchanged.

Independent static-geometry references match five sampled skin poses including
outer reflection. A separate single-sided uniformly reflected skin matches a
static transform reference under both outer orientations, with explicit controls
showing the skin is visible and casts a nonempty shadow onto the receiver. The
sources and cache are deleted before these retained checks. Invalid sample/target
refusal preserves previous output and retry succeeds. After GPU setup, 64
complete accepted retained CPU staging calls record zero heap allocations; this
excludes submission, frame encoding and driver work.

No capture images were exported or reviewed. Existing regressions also compare
in-memory PNG codec output; the new animation oracles compare raw readback. This
is numerical rendering evidence,
not Claude appearance acceptance, a public exported animation-game proof, or a
reference-phone/performance gate. Both test processes exited successfully and
were verified absent before the exclusive GPU interval was released.

[Exact source, binary, log and release receipt](evidence/native-animation-rendering-gpu.json).

The separate authored startup/cook boundary at `1c8e697` also received a
[read-only independent review](native-animation-authoring-review.md). Its scoped
acceptance does not replace the upcoming integrated player proof.
