# Fixed world-space text shader cooking

The world-space text path now has an opt-in `cooked-text` feature for private,
build-generated Metal, SPIR-V and HLSL stages. The existing WGSL, glyph geometry,
coverage filter, blending and depth behavior are unchanged. Default and browser
builds continue to use WGSL. Project shader input cannot enter this native path.

The cook requires the exact vertex index, instanced glyph inputs, flat atlas
bounds, 80-byte frame uniform shared by both stages, 64-byte vertex-only node
matrix and fragment atlas/sampler. Host validation checks the actual Rust member
offsets and pipeline binding entries, including the dynamic node offset and
32-byte glyph stride. Metal uses bounded instance pulling at buffer 15, lengths
at buffer 2, frame at buffer 0 and node at buffer 1. Vulkan keeps descriptor
groups explicit; DX12 uses the pinned HAL's group-2 sampler-index mapping.
Contract failures return a typed error before native passthrough.

Independent review accepts the 22 frozen inputs. All seven scoped checks pass:
90 enabled CPU tests, Clippy, wasm32 compilation with all cooked features,
63 default library tests, formatting, conventions and diff checks. The enabled
run skips 82 GPU tests and the default library run skips 32; these are pending
native execution. Both generated Metal stages also pass actual Apple compiler
and metallib linking. The shader contracts exercise changed uniform layout,
resource binding, stage access, builtins and interpolation refusals, as well as
SPIR-V round-trip reflection across all five geometry shader families.

The [evidence manifest](evidence/cooked-text-shaders.json) preserves the source
freeze, exact commands, lossless raw outputs, backend artifacts and review.
Claude's GPU and rendered-pixel comparison and integration remain pending.
HLSL emission and SPIR-V validation do not establish DX12 or Vulkan driver
execution. Other shader families, persistent pipeline caches, asynchronous
fallback and physical-device performance gates remain open.
