# Opt-in retained dynamic resolution

This slice connects a bounded frame-duration controller to the retained renderer's
actual scene color/depth attachment selection. Existing rendering stays at its
previous size unless a trusted host opts in. The feature is spatial scaling only:
there is no temporal reconstruction, motion-vector history, MetalFX or device
performance acceptance in this work.

`ResolutionConfig` selects linear scale in exact sixteenths between 1/4 and 1,
a target duration, deadband, separate sustained-overload/recovery sample counts,
cooldown and a bounded scale step. Invalid policies refuse before consumer GPU
preparation. Invalid finite/range telemetry preserves the previous controller
state. The controller uses constant storage and never serializes a world or
creates a pipeline. Receipt preparation, unchanged-size planning and feedback
have a CPU allocation-counter regression; this does not assert that complete
frame encoding or wgpu callback registration is allocation-free.

`RetainedScene::set_dynamic_resolution` is explicit and replaces the telemetry
owner. Successful submission produces an opaque `ResolutionFrame`; failed staging
or encoding never publishes it. Feedback from another owner, an older accepted
viewport epoch, an already observed frame or a previous scale is discarded.
Native scene replacement creates a fresh owner. Output size, camera viewport,
screen-space UI layout/coordinates and chrome remain physical output dimensions.
An authored pixel-perfect camera keeps its exact existing logical target and does
not drive the adaptive controller.

The attachment dimensions are rounded up from the scaled output dimensions.
Every scene viewport coordinate uses the same binary scale, preserving camera
aspect instead of deriving it from independently rounded dimensions. Existing
output shader coordinate mapping expands the scene texture; its textureLoad
sampling, tone map, chrome mask and all WGSL remain unchanged. This is the existing
nearest-texel spatial mapping, not a new reconstruction filter. Screen-space UI
is submitted afterwards into the full-size output target. The existing single-size
attachment cache reuses unchanged dimensions; changes replace that idle cache,
while in-flight commands retain their own textures under the existing ownership
contract. No total driver-memory or reference-phone budget claim is made.

## Native completed-work telemetry

A trusted composed host can call
`GpuRenderSettings::with_dynamic_resolution(config)` with its existing target
provider. This enables actual `wgpu::Queue::on_submitted_work_done` callbacks with
one pending callback/result slot. The ordinary provider default is disabled.
No production queue wait, new timestamp query feature or simulation clock is used.
The automatic native timer explicitly refuses browser targets before GPU
preparation; the portable renderer controller can receive browser-owned timing
through its explicit receipt API. No browser-native timer integration is claimed.

The measurement is wall-clock latency from render preparation to invocation of
the queue-completion callback. It includes CPU preparation, queue backlog,
presentation blocking and callback dispatch delay. It excludes earlier game
simulation and is **not isolated GPU execution time, a refresh interval, or whole
game frame time**. This distinction matters when selecting a policy target.
Callback sampling and queue behavior require later real GPU acceptance. Oversized
or invalid measurements are counted and discarded; a poisoned callback mailbox
fails explicitly. No unbounded queue of samples survives stalls, and delayed
callbacks retain only their original owner. Device loss/presentation errors keep
the existing terminal consumer behavior.

## Validation status

Independent [source and evidence review](evidence/dynamic-resolution-source-review.json)
accepts this bounded CPU slice. The exact reviewed manifest is retained losslessly;
final manifest metadata records that acceptance. Actual GPU callback lifecycle
and rendered scaling remain pending Claude0067, which is preparation only until
the root grants the GPU and target after0065/0066.

The final two-package CPU suite passes 73 tests in seven groups; 89 GPU tests
remain ignored. All-target Clippy, renderer wasm32 compilation, normal runtime
dependency checks, formatting and conventions pass. Seven new renderer tests
exercise the policy, ownership, layout math and allocation counter; three native
host tests exercise the actual bounded completion slot across thread delivery,
retirement and a destructor/reservation interleaving. They do not execute wgpu.
Initial Clippy failures and their corrections are retained in the
[evidence manifest](evidence/dynamic-resolution.json). No GPU/window/capture was
launched by this implementation lane. Visual scaling quality, native queue callback behavior,
backend execution, mobile defaults and frame-time budgets require Claude and the
plan's actual device evidence. Enabling this explicitly does not close the full
temporal-upscaling requirement.
