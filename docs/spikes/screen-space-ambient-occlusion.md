# Opt-in screen-space diffuse ambient occlusion

This is a CPU-validated candidate with independent scoped source and final CPU evidence acceptance;
Claude backend/appearance acceptance remains pending. It is disabled by default. It does not complete
Phase 1 rendering, temporal upscaling, MetalFX, mobile quality, or device budgets.

## Connected path

Call `Renderer::enable_ambient_occlusion(AmbientOcclusion { radius, bias, strength })`
once during renderer setup, before giving it to a render host. It validates the
settings and builds 17 fixed pipelines under the renderer's validation error
scope. A failed enable leaves the previous configuration intact. Disabling affects
future draws; already encoded command buffers retain their resources. There is
no project mutation, scene serialization, new window input path or hidden enable
flag. Native `cooked-materials` explicitly refuses this request: a corresponding
cooked SSAO material family has not been implemented. Browser/default portable
WGSL is supported at source/compile level; GPU execution is still pending.

The existing `ModelDraw` packet encodes opaque/MASK normal/depth, fixed SSAO, and
then forward geometry, in that order. Its original vertex function, retained
animated vertex buffer, instance inverse-transpose normal transform, index range,
front-face/cull settings and alpha-mask cutoff are reused. Normals are the
interpolated authored surface normals, before normal-map perturbation; double
sided backfaces use the same sign rule as forward materials. The prepass has its
own depth attachment, so diagnostic geometry and the ordinary scene depth path
remain unchanged. BLEND models and diagnostics are neither AO receivers nor AO
occluders. Sprites, world text and UI are untouched.

Only the diffuse environment irradiance product is multiplied by accessibility.
Direct/clustered lighting, directional shadows, specular IBL and emissive are not
multiplied by SSAO. Existing material-authored occlusion retains its original
meaning. Default material/shadow/lighting/output/sprite/text/UI WGSL bytes remain
identical to main192.

## Numerical contract

Right-handed view space looks along -Z. WebGPU depth is [0,1], cleared to 1;
framebuffer coordinates have a top-left origin. The actual camera projection and
its inverse are used for perspective and orthographic cameras, including offset
and fractional viewports. No reconstruction formula assumes perspective. Camera
projection extraction preserves the existing projection-times-view multiplication.
Matrices, viewport and inverse-frustum corners must remain finite; unsupported
inverse ranges refuse before command encoding.

Sixteen deterministic cosine-weighted hemisphere directions use equal-probability
radial strata and a bit-reversed angular sequence. Each probes four distances out
to a radius in view/world units. A ray is blocked when a sampled visible surface
is in front of its probe, above the biased normal plane, and within the radius.
Each direction contributes at most one hit. Accessibility is
`clamp(1 - strength * blocked / 16, 0, 1)`. Clear depth, invalid normal coverage,
out-of-viewport probes, points behind the eye and points beyond clip planes do
not occlude. Texel centers, not continuous probe positions, reconstruct loaded
depth. This is a finite first-depth-layer approximation of diffuse visibility,
not ray-traced ground truth: offscreen/hidden geometry and thin-depth limitations
remain. It has no temporal history, noise animation, denoising or tuned quality
preset. Specular visibility requires a different angular model and is excluded.

Primary background: NVIDIA's [Screen Space Ambient Occlusion](https://developer.download.nvidia.com/SDK/10.5/direct3d/Source/ScreenSpaceAO/doc/ScreenSpaceAO.pdf)
and the [WGSL coordinate/texture contract](https://www.w3.org/TR/WGSL/).
The fixed estimator here is deliberately documented independently; it is not an
implementation or performance claim for HBAO/GTAO or a vendor SDK.

## Bounds and ownership

Settings require radius 0.0001–1000, bias in [0,radius), and strength in [0,1], all
finite. SSAO accepts at most 4,194,304 target pixels, dimensions at most 8192 and
within device limits. Depth32 + normal RGBA16Float + AO R8Unorm are 13 logical
bytes/pixel: at most 52 MiB logical texel payload in its one-size idle cache. A
resize can temporarily retain old and candidate payloads; callers may retain
arbitrarily many encoded frames. Driver padding, allocator overhead, other
renderer targets and caller queue depth are not bounded by that number. The
maximum sampling loop is 64 probes/pixel. This establishes finite work, not an
acceptable frame time. Default rendering keeps its existing larger target limit.

Every frame owns a 480-byte immutable sampling uniform plus bind groups and
pipeline handles. Resizing replaces the cached attachments; no queue write can
change an earlier frame's parameters. Pipeline compilation occurs only during
explicit setup, never during `draw_scene`. The target extent comes from the
actual retained model color attachment and its scene viewport, not physical UI
or output dimensions. Pending DRS and GPU timestamp integration must preserve
this seam; the new passes currently sit within the model group and have no
individual timestamp/profile entries.

## Verification and remaining acceptance

Eight CPU tests currently cover typed bounds, cosine strata, perspective and
orthographic reconstruction, translated/negative-scale normal transforms,
analytic flat/foreground visibility, exact material rewrite isolation and Naga
29.0.4 validation of all three actual WGSL programs. A device test is present but
ignored: it checks opt-in zero-strength output, queued resize lifetime, failed
reconfiguration and empty scenes. CPU results do not establish actual GPU
pipelines, rendered appearance, cross-backend equality or performance. The full CPU renderer suite passed 73 tests, with 83 GPU tests ignored. Default
and cooked-material all-target Clippy, browser compile, focused cooked-material
CPU tests, formatting, conventions and lean dependency checks all passed.
The exact [source/check manifest](evidence/ssao.json) and
[independent source receipt](evidence/ssao-review.json) retain those scopes.
The [final CPU audit](evidence/ssao-final-review.json) independently rehashes the
packet and reruns the focused CPU tests on both frozen binaries.

Claude must run the actual device tests plus bounded nonzero AO, perspective/
orthographic, MASK, mirrored/deformed normal, transparent, offset viewport and
physical UI controls. No images or GPU execution were produced by Astra. Native
consumer enablement and cooked SSAO artifacts remain explicit follow-ups; the
API must not be silently enabled by a Cargo feature or a default renderer option.

The prepared [Claude 0070 packet](../../handoffs/0070-ambient-occlusion/brief.md)
requires nonzero AO and lighting-isolation controls; it grants no graphics lease.
