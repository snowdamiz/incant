# Native MetalFX interop contract

This private producer connects Apple's temporal scaler to the retained renderer.
Source/CPU verification and delegated GPU/appearance acceptance are separate.
The feature is off by default. Existing portable shaders and default graph API
are unchanged; this does not claim a measured quality or performance improvement.

## Admission and public path

Build `incant_render` / `incant_render_gpu_host` with `native-metalfx`. Call
`Renderer::enable_metalfx` during startup, then select a retained owner with
`RetainedScene::enable_metalfx`, or the ordinary native host with
`GpuRenderSettings::with_metalfx`. Both take the existing checked portable
`UpscaleConfig` and an explicit `Portable`, `PreferMetalFx`, or `RequireMetalFx`
preference. The configuration controls portable fallback and engine-side depth
compatibility; it does not pretend to control Apple's hidden temporal weights.
The accepted `metalfx_selection()` reports actual native selection or the precise
portable fallback reason. Native selection is published only after queue submit.

The native path requires Apple's Metal backend, the queried device capability,
macOS 14.4 / iOS 17.4 APIs, supported formats/usages, integral content/viewport
rectangles, identical input/output aspect, and the queried dynamic scale range
intersected with [0.25, 1]. Odd/fractional DRS footprints that cannot meet that
contract are never rounded: prefer mode uses the existing portable path; require
mode returns a typed error. Pixel cameras preserve their existing bypass even
in require mode. Unsupported device/OS/format and three-in-flight saturation
likewise report a reason or refuse. The optional Apple framework is linked only
on Apple targets; deployment still requires the MetalFX framework (macOS 13 /
iOS 16 minimum), even if a later API check selects the portable fallback.
Non-Apple targets compile the same preference API and report `Backend`.
Current native-cooked combinations retain the motion admission refusal until
root's separately owned cooker integration is reviewed.

## Ordered transaction and lifetime

The native retained path encodes exactly three buffers and calls `queue.submit`
once with their ordered array. The first ordinary wgpu encoder renders current
opaque/MASK color and motion/depth, packs native inputs, explicitly initializes
all six native targets, and transitions their usages. The middle encoder is
raw-only, following pinned wgpu's prohibition on mixing ordinary and HAL encoding
on the same encoder. MetalFX encodes into that wgpu-owned command buffer: it is
never independently ended, committed, waited on or submitted. The last ordinary
encoder transitions the result, normalizes current coverage, then draws current
BLEND/sprite/world-text overlays and display output. Screen-space UI remains
outside history at physical output size. Motion/history/live geometry/selection
are accepted only after the single submit, as in the prior retained path.

All native resources are private, tracked textures from the exact owning device.
Raw guards fence destruction during encoding. A pure Objective-C association
retains the scaler on its native command buffer; the engine never gives a Rust
native owner to another thread or declares unsafe Send/Sync. A completion lease
holds only safe wgpu texture owners through the final buffer's completion, and
an abandoned callback releases that same lease. The Rust scaler/retained owner
is thread-bound. Invalid preparation publishes no accepted state. Entering native
encoding marks its private history dirty; failed or abandoned encoding requires
reset on retry. Device loss/driver failure after submit remains terminal, as in
the existing renderer.

The native Models timestamp range brackets the real pre/packing/post passes and
therefore includes the intervening native work; no extra query group is invented.
Empty groups remain absent and existing deferred timestamp resolution remains
unchanged. Interpretation of the whole-frame timestamp envelope is root-owned.

## Coordinates, coverage and reset

Inputs are packed at zero offset into fixed maximum-output-size textures. The
active inputContent extent is exact, not the ceiled attachment ratio. RG16
motion is current-to-previous displacement in current-content pixels after
removing both accepted raster jitter offsets. Depth is forward [0,1]; color is
linear premultiplied RGBA16 HDR. Exposure is explicitly 1 and preExposure is 1,
with auto-exposure disabled. The reactive mask rejects clear/fractional/invalid
inputs and missing motion. No SDK-27-only offset or jittered-motion convenience
API is needed.

Engine coverage always comes from the current input: vendor alpha is ignored.
Clear/fractional or mixed-depth reconstruction uses current-only color; residual
RGB at alpha zero is zeroed. BLEND/sprite/world-text overlays enter only the final
composition and are never supplied to MetalFX. The opaque vendor algorithm is
not treated as a proof of disocclusion quality; real moving/occluded fixtures
remain a delegated GPU acceptance requirement.

Camera cuts, owner replacement, output resize, gaps, backend changes, and any
abandoned native encode reset vendor history. Content-size changes also reset
native history conservatively instead of assuming a cross-size convention for
Apple's private state. They reuse the scaler and fixed textures. The unchanged
portable path still preserves output-resolution history across internal DRS
changes. Native scalers request synchronous initialization at owner/extent
preparation; ordinary unchanged frames create no native pipeline or scaler. The
single bounded cache survives temporary portable fallback, with its history
invalidated, so Busy/fractional fallback does not cause native recompilation.
Disabling/replacing the owner releases the cache; completion leases keep any
submitted resources alive independently.

## Bounded resources and pending evidence

At most three native submissions per owner hold completion leases. Native target
allocation accounting includes accepted, candidate and callback-retired targets
before allocation. The logical limit is 200 MiB + 4 KiB: up to 4,194,304 pixels per
set at 25 bytes per pixel plus bounded small metadata (input color/depth/motion/
mask and output color; exposure is 1x1). A separate atomic lifetime reservation
prevents repeated resize from escaping the limit. Reuse does not reserve twice.
The existing portable upscaling 640 MiB + 16 KiB old/candidate limit also applies
before either family allocates. These are conservative logical engine limits,
not a combined process RSS or phone budget. Apple's opaque internal history,
driver/cache memory, other owners and base-renderer retired resources are not
measured or included. No device-budget completion claim is made.

Focused CPU tests cover checked plans/capacity, actual completion callback
retention/abandonment, stale/gapped/reset state, all jitter phase pairs, coverage,
depth edges, offset coordinates, and actual WGSL parse/validation. One ignored
real retained regression compiles a six-frame native/fallback/reset/replacement
route with an exact unsupported-format refusal. It must be delegated to a
supported Metal device; fallback is not accepted as native test success. No GPU
execution, image inspection, appearance acceptance, iOS device run or performance
gate is performed by this producer.
