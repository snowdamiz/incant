# Portable output-resolution temporal upscaling (CPU slice)

This explicit opt-in reconstructs opaque/MASK scene-linear color from the actual
DRS-selected internal viewport into persistent physical-output color/depth history.
It builds on the immutable reviewed fixed-resolution producer `e73afe43`. Ordinary
and fixed-internal accumulation paths keep their behavior; no quality preset,
sharpening or default mobile enablement is selected. GPU numerical/Claude appearance
acceptance, device budgets, native cooked artifacts and MetalFX remain pending.

## Public native ownership

At startup call `Renderer::enable_temporal_upscaling`. Then supply a validated
`UpscaleConfig` (the existing finite temporal policy) through
`RetainedScene::enable_temporal_upscaling` or
`GpuRenderSettings::with_temporal_upscaling`. The latter returns the existing
monotonic `MotionReset` and composes with `with_dynamic_resolution` in either order.
Configuration is captured once by the real consumer; replacement scenes obtain
fresh owners. `upscale_frame()` exposes submitted metadata, not texture handles.
Fixed accumulation and output reconstruction are mutually exclusive; unsupported
native cooked combinations refuse explicitly. Portable native and browser code
share these WGSL algorithms; compilation is not backend execution evidence.

The same accepted motion owner controls exact prior rigid/deformed geometry,
camera projection, jitter, epoch and submission sequence. Only internal scale
changes preserve output history. Physical output extent/viewport, projection,
explicit cut, config/mode or scene replacement reset it. Pixel cameras bypass and
release history. Failed staging/format/encoding preparation changes neither the
accepted buffers nor phase. Ordinary post-submission GPU/host failure is terminal.
Simulation ticks do not identify renderer history.

## Reconstruction and mapping

Let `u=(output_pixel-output_rect.xy)/output_rect.zw`; all positions are top-left
pixel centers. The current sample is
`source_rect.xy+(u+current_jitter_uv)*source_rect.zw`. Jitter UV uses the actual
unrounded internal viewport dimensions. Previous history position is
`output_rect.xy+(u+current_jitter_uv+motion_uv-previous_jitter_uv)*output_rect.zw`.
The stored previous jitter is divided by its own accepted source viewport,
including across 25/50/75/100% transitions. Attachment `ceil` sizes never substitute
for viewport extents. Offsets and odd dimensions retain exact binary DRS factors.

Current reconstruction uses nonnegative normalized bilinear weights. All positive
current taps must have opaque coverage and compatible view depth before history
is eligible; mixed clear/geometry coverage remains current-only. A nearest-depth
anchor supplies the actual surface motion, never invented camera-only motion.
Stored history depth is positive view distance in R32Float (zero is invalid).
Each positive bilinear history tap must be in the output viewport, finite,
nonnegative, opaque and depth-compatible with the motion pass's expected prior
surface depth. Outside taps refuse history instead of clamping into edge values.
A compatible current 3x3 neighborhood clips history; scene-linear change attenuates
its explicitly bounded weight. Invalid/negative data cannot authorize history.
Alpha-zero RGB residuals produce zero; half-float radiance is finite and bounded.
There is no silhouette/detail reconstruction or image-quality claim from CPU math.

## Current-only surfaces, graph and color

Opaque/MASK shading stays at internal resolution. Models encodes motion/depth,
opaque shading, output history reconstruction, overlay-color clear, and BLEND
models. BLEND uses a separate transparent-black internal attachment sharing the
original current opaque depth. Sprites and world text follow on that same overlay
and depth, preserving their existing order, blend modes and depth writes.

Display spatially reconstructs/dejitters the premultiplied overlay and composites
`overlay + history * (1-overlay.alpha)` into a separate physical-output HDR target.
Overlay pixels never enter color or depth history. The unchanged output shader
then uses identity physical mapping and its existing tone map, backdrop and sRGB
transfer; physical UI is drawn later, without jitter or scaling. No shader/layout/
copy/style defaults changed. Transparent reconstruction is current-only, not a
claim of temporal quality for particles, crossing surfaces or glyph edges.

Seven timestamp groups remain. Models counts each real motion/opaque/reconstruct/
clear/BLEND pass; Display spans overlay composition then ordinary output. Only the
first and last actual pass receive endpoints. Deferred query resolve and callback
ownership are unchanged; no production waits or synthetic GPU timing.

## Allocation admission

Before any new motion/upscale target allocation, both old and candidate payloads
are charged. Each dimension is at most min(8192, device limit), each extent at most
4,194,304 pixels, internal size no larger than output, with an exact admitted DRS
scale. Fractional viewports with no actual source/output sample refuse explicitly.

Output storage: two RGBA16Float histories (16 bytes/pixel), two R32Float depths
(8 bytes/pixel), one RGBA16Float composite (8 bytes/pixel). Internal storage: the
existing motion/depth family (24 bytes/pixel) plus overlay (8 bytes/pixel). Thus
old+candidate at the maximum dimensions is 512 MiB; maximum old+candidate motion
instance payloads add 128 MiB, plus 16 KiB fixed metadata/uniform allowance. The
logical ceiling is **640 MiB + 16 KiB**, not a phone budget. It excludes ordinary
frame attachments/assets, allocator/driver overhead, older queued submissions and
other caller-owned scenes. Exact-size targets are reused; packets/uniforms still
allocate. No RSS, performance improvement or zero-allocation claim is made.

## Evidence boundary

CPU checks exercise actual ownership/matrix/phase transitions, exact footprint
admission, old+candidate accounting, both shader parses/validation, and independent
double-precision mapping, radiance, coverage and rejection references. Compiled
ignored GPU tests cover fourteen tiny raw resolve/composite cases and the real
retained source-deleted model path with failure/DRS/live-staging checks. They are
not execution evidence until Claude runs a bounded reviewed packet. All default
shader bytes and the prior fixed accumulation WGSL remain unchanged.

Actual temporal behavior at every tier, appearance, native cooked compilation,
MetalFX interop/fallback and reference-phone measurements remain open PLAN gates.
