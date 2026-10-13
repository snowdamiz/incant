# Retained opaque-surface motion and depth

This opt-in CPU implementation is a foundation for PLAN revision 8 temporal
rendering. It does not implement temporal antialiasing, jitter, color history
reconstruction, disocclusion rejection, temporal upscaling or MetalFX. Native
GPU execution and appearance acceptance remain pending Claude. No device or
performance gate is claimed.

## Admission and ownership

The source is based on publisher's immutable cluster + dynamic-resolution union
`9b1df9db27ead20253d9f7b24c6cdc41d459f086`. Existing shaders, render dimensions,
output transfer, UI composition and dynamic-resolution policy are unchanged.
`Renderer::enable_motion_vectors` prepares four fixed portable pipelines at
startup. It is off by default. All native cooked shader feature combinations
return `MotionError::Cooked` before pipeline creation; this pass has no cooked
artifacts yet. Browser builds retain the portable path. Unsupported attachment
formats produce a typed refusal.

A retained scene separately opts in with `enable_motion_history`. The native
`GpuRenderSettings::with_motion_history` wrapper connects this to ordinary host
preparation, accepted sync and replacement. Its `MotionReset::camera_cut` handle
is a checked monotonic trusted presentation epoch. Dynamic-resolution and motion
wrappers forward each other's settings. Authored documents, world ticks and save
formats do not contain history or camera-cut state.

Every retained history owns a unique token, submitted-frame sequence and reset
epoch. A newly prepared replacement cannot reuse the old owner's history, even
with identical stable entity names and simulation ticks. Camera motion normally
produces vectors. The host must explicitly signal a discontinuous camera cut.
Changes in output or internal extent, viewport, projection parameters or pixel
camera mode invalidate prior history automatically. Pixel cameras bypass this
pass. Disable/re-enable creates a fresh owner. Device failure remains terminal to
the existing native consumer.

## Actual retained geometry and atomic submission

History advances immediately after the existing renderer queue submission, not
when simulation ticks, CPU staging, encoding or GPU completion callbacks happen.
This is accepted submission chronology, not an assertion of device completion or
presentation success. The ordinary host already becomes terminal on later GPU or
presentation failure and drops its live owner.

The pass uses the same immutable primitive, index buffer, source generation,
material UV, MASK sample/cutoff, current instance matrix and culling variants as
canonical model drawing. Stable sealed placement-to-node slots associate previous
world transforms; mirror-bucket packed offsets are never treated as identity.
Exact source Arc/generation, primitive, animation binding, placement and buffer
size must agree across the private live/staging scenes.

For animated rigid nodes the previous matrix includes the accepted sampled local
hierarchy. Skinned meshes use the actual accepted live vertex buffer containing
CPU deformation after pose/root-motion stripping/IK; the newly sampled staging
buffer supplies current positions. Previous geometry is not resampled or inferred
from the current rest mesh. Mesh/entity topology changes require a replacement.

All staging and frame encoding finish before existing instance/deformation/text
writes and the new previous-instance writes. Only then does the owner submit,
swap its actual live/staging render scenes and accept the new matrices/targets.
Ordinary validation refusal leaves accepted history, geometry buffers and sequence
unchanged. A pending cut survives failure. Partially staged CPU scratch is private
and overwritten by a retry. No new shader or pipeline compiles per frame.

## Attachment and coordinate contract

The motion attachment is RGBA32F. At an opaque or surviving MASK sample it stores
`(previous_uv - current_uv, previous_projected_depth, geometric_history_valid)`.
UV is top-left normalized within the camera's **internal viewport**, including
its offset; depth is WebGPU's `[0,1]` projected depth. Motion is unjittered. The
current viewport comes from the exact existing `FramePlan`, including dynamic
resolution's single binary scale and odd-extent rounding. Physical UI/output
coordinates are untouched. A scale/extent change resets history.

Previous homogeneous positions must be finite, in front of the previous camera,
in its depth range and inside the half-open previous viewport. Invalid history
writes zero including validity, while valid stationary geometry writes zero
motion with validity one. The current motion pass writes its own depth surface;
two depth surfaces alternate so the last submitted depth is retained. Current
and previous depth plus motion stay private to the owner for a future resolve.
No public texture handles or stale borrowable history escape.

BLEND materials, sprites, world text, diagnostics, backgrounds and screen UI are
excluded. Thus this is an **opaque-surface field**, not valid history for every
final HDR pixel under transparency or overlays. A future color resolve must
handle coverage/reactivity and compare depth; geometric validity here does not
claim visibility/disocclusion validity. Empty opaque scenes clear both outputs.

## Bounded logical allocation

Admission checks complete logical payload before motion allocations:

- At most 4,194,304 internal pixels and no dimension above the device limit or
  8,192. One RGBA32F plus two Depth32F surfaces use 24 bytes/pixel, at most 96 MiB.
  Resize admits the old and candidate family together, at most 192 MiB.
- At most 64 MiB of motion instance/packet logical payload per owner. This
  accounts for both mirrored capacities, 80-byte CPU and GPU previous rows,
  stable slot indices, previous f64 entity matrices, worst-case temporary node
  list capacity, row/draw/owner/target headers and the 144-byte CPU/GPU frame
  uniform. Aggregate slots are capped at twice the existing 100,000 instance
  limit; each GPU buffer must also fit its device limit. Old and candidate
  instance owners are admitted together, at most 128 MiB.
- Pixel-camera bypass allocates no rows or attachments. Static row packing uses
  retained capacity and its CPU test counts zero allocations. Per-submit frame
  uniform/bind group and bounded draw packet are still allocated; this is not a
  zero-allocation complete renderer claim.

The combined transition ceiling is 320 MiB of these logical families, not an RSS
limit. Existing model/live/staging geometry is reused, not newly duplicated by
motion. Immutable source assets, pre-existing renderer resources, four fixed
pipelines, opaque driver allocation/padding and GPU in-flight resource retirement
remain governed by existing owners. Multiple caller-owned retained scenes are not
a process-wide allocator budget. No mobile memory/performance claim is made.

## Verification boundary

CPU tests cover accepted versus dropped preparation, failed cuts, unique owners,
repeated simulation states with distinct submissions, resize/projection/viewport/
DRS/pixel invalidation, finite projection math, camera/object/deformation motion,
allocation overflow, source-matched MASK semantics, portable Naga validation and
native cooked refusal. The production static row-packing test verifies stable
placement identity across mirrored ordering with zero allocations. Native cut
handles test shared monotonicity and exhaustion.

Two ignored device tests exercise the actual existing live/staging buffer swaps,
failed staging and target encoding, source deletion, same-world camera/object
motion, reflected packing, sampled skeletal vertices, post-sample IK, rebound
asset generation and fresh owner. Their raw numeric RGBA32F readback must be run
by Claude under a bounded graphics lease. They have not been executed here.
Final check/source receipts will distinguish CPU execution from these pending
GPU cases. Default/cooked/browser compilation and dependency checks remain part
of producer closure; they do not replace actual backend execution.
