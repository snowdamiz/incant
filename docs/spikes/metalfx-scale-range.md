# MetalFX scale factors

The native selector treated Apple's scale-factor limits as input/output fractions.
The M5 Pro capability query returned `[1, 3]`, so intersecting with `[0.25, 1]`
collapsed admission to exactly 1. The prior cooked native review therefore fell
back to the portable renderer for every 75% and 50% content frame.

The correction keeps the native descriptor and selector in output/input units.
The renderer's quarter-resolution floor bounds this range to `[1, 4]`; a device
reporting `[1, 3]` admits native, 75%, 50% and one-third content, but refuses
quarter-resolution content. Fractional rectangles, unequal aspect ratios,
downscaling, unsupported capabilities and allocation limits still refuse or
select the configured portable fallback. Source/output texture extents, frame
jitter, motion, history, shaders and resource budgets are unchanged.

The scale terminology follows Apple's [temporal scaler API](https://developer.apple.com/documentation/metalfx/mtlfxtemporalscalerdescriptor/supportedinputcontentmaxscale(device:)).
The local query used the installed SDK on Apple M5 Pro, macOS27.0.1 (26A434),
and created no textures, queues, encoders or GPU submissions. Its source and raw
output are retained, so the reported range is an observation rather than an
inferred limit from fallback decisions.

Complete-feature renderer CPU tests passed139 cases with41 GPU tests ignored;
all-target strict Clippy also passed. The added behavior checks exercise actual
reported range endpoints and reject unsupported scales, downscaling, malformed
ranges, nonfinite values and empty intersections. The existing aspect, integral
viewport and allocation checks remain. See the [evidence manifest](evidence/metalfx-scale-range/manifest.json).

Corrected native GPU execution and rendered appearance review are pending.
The earlier535 fixed-temporal and native reviews still reject silhouette quality;
this unit correction does not claim to fix coverage accumulation or approve a
performance, phone or phase gate.
