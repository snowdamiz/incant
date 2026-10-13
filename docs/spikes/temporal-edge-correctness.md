# Temporal edge and excluded-overlay correction

This private correctness slice follows Claude0072's rejected temporal quality
review. It changes no policy values, presets, authored assets, physical UI, or
default enablement. Numerical CPU references and compilation do not establish
that stationary shimmer is visually resolved. Claude must re-run the bounded
stationary, disocclusion, MASK, skinned, overlay and offset/DRS cases.

## History support and presentation

Both history gathers admit only finite, positive bilinear weights whose tap
centres are inside the previous viewport and whose covered color/depth agree
with the reprojected surface. They renormalize accepted support. Zero-weight,
outside, uncovered, negative/nonfinite or depth-incompatible history cannot
invalidate other surviving support or be clamped into plausible history. No
surviving support means current-only disocclusion. Existing clamp and radiance
reaction parameters remain unchanged.

Skipping zero weights alone was insufficient: changing jitter phases produce
positive taps across depth discontinuities. Fixed-mode history remains on the
jittered raster grid, paired with its original motion and depth. Presentation
samples at output position plus current pixel jitter to undo that shift, using
positive bilinear premultiplied reconstruction. It does not mutate history.
Opaque/clear current coverage is preserved rather than filling newly empty
pixels from history. This conservative policy can still leave phase-dependent
coverage at subpixel silhouettes; visual acceptance is pending, not implied.
Upscaling already reconstructs the unjittered output grid. Its mixed-depth or
fractional current coverage remains current-only; only previous-history tap
admission is relaxed to valid-support renormalization.

## Current-only overlays

BLEND models, sprites and world text use the exact unjittered current camera.
After opaque resolve, an additional depth-only pass rerasterizes canonical
opaque/MASK geometry into the existing disposable scene-depth attachment. It
shares current animated vertices, placements, material texture/cutoff, culling
and mirrored variants with motion, using an unjittered matrix. It never writes
motion color or either saved history-depth attachment. The pass runs only with
active temporal/upscale AND actual current-only overlay draws. Subsequent
BLEND/sprite/text order and depth-write policies are unchanged. Upscale overlay
composition maps output to the internal rectangle without adding jitter.

There are four extra startup depth pipeline variants (eight motion/depth
variants total). No per-frame compilation or new image allocation is added.
One optional 144-byte frame uniform and its CPU/header payload are charged in
the motion budget; draw headers include the extra pipeline handle. Models
queries count the real optional depth pass. No GPU timing implementation changes.

## Explicit integration work

This producer starts at 068ed0aa and does not contain native MetalFX or the
separately frozen cooked-temporal implementation. Their original receipts remain
honest for those sources. Root owns the explicit union/recook. Fixed copy now
adds fragment binding 1, a 64-byte Frame uniform, beside color binding 0.
reactive.yz carry current pixel jitter; reactive.x keeps its existing meaning.
Fixed resolve and upscaling layouts are unchanged. Motion adds the fragment
entry depth_only, with no color outputs, retaining the same 144-byte frame,
vertex layouts and material resources. Four depth variants need cooked artifacts.

The MetalFX postbuffer must run the same optional unjittered depth pass after
normalize/overlay clear and before current-only drawing, and count it in Models.
Its shared output overlay composite must take this unjittered mapping. Native
pack/normalize and immutable MetalFX4bc810fe are not changed by this branch.
No cooked feature admission is relaxed here.

## Validation boundaries

CPU references cover all 64 current/prior phase combinations and each nonempty
valid-tap subset, disocclusion, invalid weights/depth/colors, exact projection
sign and depth for perspective/orthographic offset views, allocation and pass
count contracts. Real WGSL is parsed and validated. Compiled ignored GPU cases
extend the fixed resolve fixture to fourteen 8x8 submissions (including a real
nonuniform depth pass and affine presentation oracle), and the output fixture to
nineteen 8x8 submissions (including phase-invariant current-only overlay samples).
They remain unexecuted in this lane. No GPU, screenshots, quality, performance,
phone budget, whole-renderer or phase-gate claim is made.
