# Retained temporal color accumulation (CPU implementation, GPU pending)

This opt-in renderer path adds fixed-internal-resolution opaque/MASK color
accumulation on the accepted [motion owner](retained-motion-history.md).
It is not temporal upscaling, complete TAA quality acceptance, MetalFX, mobile
budget evidence or default enablement. GPU numerical execution and Claude
appearance review are pending. The original shaders remain byte-identical.

Call `Renderer::enable_temporal_accumulation` once during startup, then supply
an explicit validated `TemporalConfig` to `RetainedScene::enable_temporal_history`
or the real native `GpuRenderSettings::with_temporal_history` wrapper. There is
no quality preset or authored scene/save setting. All native cooked shader
combinations refuse explicitly; browser uses the portable path. Ordinary
`draw_scene` and ordinary constructors retain their existing behavior.

## Raster and accepted ownership

An eight-position, zero-mean sequence within half an internal pixel jitters the
clip X/Y translation. Current model color, motion, sprite and world-text raster
projections share that translation; stored previous motion projection includes
its exact old jitter and accepted rigid/deformed vertex data. Physical output
and screen-space UI are never jittered or resized by this feature. Cluster
selection expands each internal-pixel tile by one pixel in a separate fixed
portable startup pipeline; the existing grid body is unchanged. Cascade fitting
conservatively includes the full jitter envelope. No per-frame pipeline creation.

The motion owner is also the color owner: color/depth slot, camera matrix,
sequence, epoch and jitter phase advance only after the same immediate successful
queue submission. Failed CPU staging/format/encoding preparation consumes none.
GPU device loss or post-submission host failure remains terminal. Replacing the
retained scene creates a unique owner. Explicit cut, viewport/output/internal
extent, DRS or projection changes reset phase and both history families. Pixel
cameras bypass jitter/accumulation and release histories. Config replacement is
an explicit reset. Simulation tick repetition has no influence on history.

## Resolve and exclusion

The Models group encodes motion/depth, opaque/MASK HDR, resolve into private
RGBA16Float history, an exact load/store copy to the ordinary HDR attachment,
then current-only BLEND models. Sprites and world text follow, then existing
display transform/composition and physical UI. Only pre-overlay opaque radiance
is retained. Retained diagnostic placeholders already refuse. Empty scenes are
well-defined clear/current-only frames.

Resolve uses top-left internal viewport coordinates including offsets and odd
sizes. Motion gives previous minus current UV and expected previous projected
depth. Every manually bilinear-loaded history tap must be in the old viewport
and pass view-space depth rejection; perspective depth uses a reciprocal
interpolation, orthographic depth a linear interpolation. Invalid/offscreen
motion, far/clear depth, nonfinite data and nonopaque coverage use current color.
There is no fabricated camera-only motion for moving or skinned objects.

Accepted history is clamped to the current 3x3 neighborhood. A finite bounded
scene-linear max-channel change attenuates its weight before convex mixing.
History weight is at most 0.99. Alpha-zero/fractional-coverage current samples
never accumulate. RGB is bounded to finite nonnegative half-float range, alpha
to [0,1]. Only the existing linear HDR format is used; output encoding and
source alpha/blending contracts are unchanged.

This conservative rejection is not a claim of complete silhouette reconstruction.
BLEND/sprites/world text can still exhibit current-frame jitter; reactive model
coverage is limited to depth/coverage and radiance-change evidence. Particle/
transparent temporal reconstruction, material-authored reactive masks and
spatiotemporal anti-aliasing quality still require implementation/Claude review.

## Bounded storage and timing

Motion and color target admission runs before either family allocates. At most
4,194,304 pixels (each dimension <= min(device limit,8192)) are admitted. New
color history is two RGBA16Float targets: 16 bytes/pixel, 64 MiB maximum. Together
with motion's 24 bytes/pixel, two old/candidate target sets are at most 320 MiB.
Motion CPU/GPU instance payloads add at most 128 MiB old+candidate. Two fixed
4096-byte metadata/uniform allowances give the explicit combined ceiling
`MAX_TEMPORAL_TRANSITION_BYTES` (448 MiB + 8 KiB). This logical ownership bound
excludes existing ordinary frame/model/UI storage, allocator/driver overhead,
and resources retained by older queued submissions or caller-owned scenes.
Frame packets/uniform bindings still allocate; no whole-renderer zero-allocation
or RSS bound is claimed. No full-resolution output history is allocated.

The private branch includes accepted timestamp producer `40f138f7`. The seven
logical groups are unchanged. Models explicitly spans all actually encoded
motion/opaque/resolve/copy/transparent passes, using exactly one first/last
pass-descriptor endpoint pair; empty interior descriptors are absent. Its
completion-triggered deferred resolve and bounded ownership are unchanged.
Temporal+timing GPU execution remains pending.

## Validation and next milestone

CPU checks cover admitted policy, both projection families, centered jitter,
old/candidate memory limits, acceptance/failure/cut/resize/pixel phase behavior,
view-space disocclusion, neighborhood clamp/reactivity/constant radiance,
coverage, conservative clusters/shadows, and actual Naga WGSL parsing/validation.
Prepared ignored tests execute actual resolve/copy numerical controls and real
retained scene/buffer publication; compilation is not their execution evidence.
No screenshots or GPU calls were made by Astra.

The next built-in temporal upscaler milestone must reconstruct into physical
output resolution with explicit source/output footprints, persistent output-size
history and dynamic internal-resolution transitions, richer reactive coverage,
then bounded Claude GPU/pixel acceptance on supported backends. MetalFX remains
a separate native interop implementation with a portable fallback. This slice
does not satisfy those plan gates or reference-phone performance requirements.
