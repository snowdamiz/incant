# Fixed screen-space UI shader cooking

The screen-space game UI has an opt-in `cooked-ui` path for private, build-generated
Metal, SPIR-V and HLSL artifacts. The existing WGSL, UI theme, quad geometry,
blending and output-format catalogue are unchanged. Web and default builds retain
WGSL. This follows the cooked sprite work without claiming the renderer is fully
free of runtime shader compilation.

The cook validates both stage interfaces, the 48-byte frame uniform used by both
stages, and the fragment atlas/sampler. Native host checks bind these to actual
Rust field offsets and the 64-byte instanced quad, including packed normalized
colour. Metal uses bounded instance pulling at buffer 15, lengths at buffer 1
and frame at buffer 0. Vulkan retains explicit bindings and image-query support;
DX12 uses frame b0 and the pinned HAL sampler-index mapping. Mismatches return a
typed error before the private shader passthrough path.

Independent source review accepts the 15 frozen files. All seven local checks
pass: 87 enabled CPU tests (82 GPU tests ignored), Clippy, wasm32 compilation,
63 default library tests (32 GPU tests ignored), formatting, conventions and diff
checks. Source001's temporary-attribute lifetime error is retained; source002 uses
an identical constant attribute array. Full source identities, commands and raw
lossless logs are in [the evidence manifest](evidence/cooked-ui-shaders.json).

Work is saved at the director's requested pause. Actual offline Metal compilation
of the UI artifacts, Claude's native GPU and rendered-pixel comparison, and
integration remain pending. These CPU checks do not establish a rendering,
physical-device, performance or phase exit gate. Resume by completing those checks
on the frozen source before enabling this path by default.
