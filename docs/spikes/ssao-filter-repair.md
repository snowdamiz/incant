# SSAO rotation and plane filter: provisional implementation

This private candidate implements Claude's R1/R3 prescription on the retained
`72a16a9d` composition. It is an experiment, with default-disabled behavior and
native cooked-material refusal preserved. The original quality failures remain
recorded in [the CPU oracle experiment](ssao-quality-oracle.md); compilation and
numerical agreement do not approve appearance.

The sampler now uses a per-internal-pixel 4×4 Bayer rotation and a transposed
upper-half step jitter. Its 16 directions, four radial samples, authored shading
normals, radius and bias are unchanged. A 736-byte typed uniform preserves its
old 480-byte prefix and appends sixteen rotation vectors, calculated in f64 and
rounded once to f32. Shader reflection verifies the offsets and array strides.

Raw accessibility passes through horizontal and vertical five-tap filters.
Each pass uses weights `[0.5, 1, 1, 1, 0.5]` multiplied by the center-normal
plane-distance weight, with threshold `max(bias, 0.01 * view_depth)`. The center
has positive weight. Background and invalid center normals produce white;
samples outside the fractional viewport or on background are excluded. Each
stage writes R8 UNORM. The independent scalar observer models the intermediate
quantization explicitly.

The pass order is sample → horizontal scratch → vertical accessibility. Frozen
bind groups and retained texture/pipeline ownership prevent later extent changes
from replacing resources referenced by a prepared frame. Four targets occupy
14 logical bytes per internal pixel. The existing 4,194,304-pixel cap therefore
allows 56 MiB for the one-size idle cache; one old maximum plus a new maximum is
112 MiB. These counts exclude driver overhead and unbounded caller retention.
They are not a total renderer memory bound or a reference-phone measurement.

Independent production source review found no blocker in the rotation, uniform,
filter, viewport, binding, lifetime and opt-in/refusal contracts. A separate
19×15 numerical fixture supplies known Depth32 values and quantized normals to
the actual filter shader. It reads each R8 stage independently, comparing with
outward f32 intervals around scalar arithmetic. Wrong axis ordering, omitted
vertical filtering, eye-z substitution and unrestricted averaging are negative
controls. Its two GPU submissions are compiled but have not yet been executed.
The constant-input observer checks enclosure rather than a singleton interval:
independent interval arithmetic loses numerator/denominator weight correlation.
Its original overly strict singleton failure is retained. No production shader
or quality threshold changed to correct this test assertion.

Current qualification passes all ten scoped commands: 108 default CPU cases
across all targets, 113 cooked-material library cases, focused filter controls,
strict Clippy in both modes, browser compilation, the runtime dependency guard,
formatting and repository convention checks. Ignored GPU cases remain unexecuted.
The first fixture compile failure used pre-wgpu29 nonoptional depth-state fields;
wrapping those fields in `Some` corrected the test's API use only.

The exact N8 stripe geometry is now explicit, with a receiver row selected from
geometry before evaluation. Perspective reduction is 0.376470588 of the original
adjacent maximum, failing the required 0.25; orthographic is 0.175686275 and passes.
The original N6 maximum remains 7/255 and fails 1/64. Claude's N6T is a separate
diagnostic with physical interpretation, not a replacement for that failure.
No radius, bias, threshold or fixture has been tuned to manufacture acceptance.

Remaining work: execute the actual filter and integrated renderer controls,
establish the bounded raster observer,
and let Claude assess the rendered result. The perspective stripe failure and
all original quality requirements must stay explicit through that review.

## Optional stage observer

The nondefault `ssao-stage-readback` feature adds native diagnostic capture of
the raw, horizontal and vertical R8 planes, with no shader change. A single
armed request is consumed by the next encoded SSAO execution. Copies run after
their corresponding passes and retain their buffer independently of textures.
A marker initialized on the CPU is cleared only after the final encoded copy;
mapping an unsubmitted/discarded command therefore refuses instead of presenting
zero-filled memory as GPU evidence. Taking a ready capture consumes it even on
failure. Replacing/disabling SSAO abandons the previous system's request.

Readback uses padded rows, checked pixel/buffer limits, fallible host allocation,
and bounded mapping waits. CPU controls verify one-shot state, completion-marker
refusal and all three planes' padding removal. On WebAssembly both public calls
return `NativeOnly`; no capture storage or copies are compiled there. Default
and shipping feature sets do not enable the observer. Its temporary CPU/GPU
buffers are additional diagnostic overhead, not part of the 56 MiB idle cache.

Independent review accepts the observer and final native-only guards. Current
feature qualification passes 111 default-feature CPU cases, 116 cooked-material
library cases and native strict Clippy, plus ordinary browser compilation without
warnings. An additional broad browser Clippy run encountered 11 existing
`arc_with_non_send_sync` findings in unchanged lighting/model/sprite/text/UI
code; that failed run is retained, and no browser strict-Clippy pass is claimed.
Actual staged GPU capture and its submission-marker behavior await execution.

Built-by: astra
