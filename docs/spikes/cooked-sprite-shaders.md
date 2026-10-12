# Cooked fixed sprite shaders

The opt-in `incant_render/cooked-sprites` feature translates the existing sprite
WGSL at build time into stage-specific MSL, SPIR-V and HLSL. The renderer then
selects only private, embedded, hash-checked artifacts. The default and WebGPU
paths keep their existing WGSL behavior. Sprite color, sampling, derivatives,
alpha, painter order and depth behavior are unchanged.

The cooker checks both stage interfaces, including the integer sampling selector
and flat varyings, plus exact camera/texture/sampler types and stage visibility.
It rejects added storage, workgroup state, overrides and binding changes. Metal
uses the pinned HAL vertex-pulling contract: two bounded per-vertex buffers with
12/52-byte strides in slots 15/14, camera slot 0 and lengths slot 1. Vulkan uses
explicit descriptor bindings and image-query bounds for nearest texture loads.
DX12 matches the complete camera/texture/sampler-index root layout. The runtime
checks the actual host buffer and resource layouts before native shader calls.
This is never a project-provided shader passthrough.

The [retained CPU/compiler evidence](evidence/cooked-sprite-shaders.json) binds
13 source inputs, unchanged WGSL, pinned backend source, compiler outputs and
72 losslessly retained artifacts. The enabled path passes 82 ordinary tests;
the default renderer passes 63. All-target Clippy, wasm32 compilation with both
cooked feature flags, formatting and conventions pass. Actual Apple Metal
compilation and library linking pass for both emitted sprite stages. Independent
source review accepts the exact host/backend contract.

All 82 GPU tests were intentionally left unexecuted for this CPU pass. Native
submission, pixel comparison and Claude appearance acceptance remain pending.
HLSL and SPIR-V translation are not DX12/Vulkan driver evidence. MSL/HLSL still
require backend compilation, other renderer families still use WGSL, and this
does not supply pipeline caches, asynchronous fallback, device performance or a
phase gate. The feature remains opt-in pending those scoped backend checks.
