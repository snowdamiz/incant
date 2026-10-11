# Cooked diagnostic and shadow shaders (opt-in)

`incant_render/cooked-geometry` extends the optional output-pass path with actual
build-produced diagnostic and shadow stages. It implies `cooked-output` and is
off by default. The default native and Web routes keep WGSL. No GPU, native
window, screenshot or provider was used for this slice. Numerical and Claude
pixel acceptance remain pending; CPU results do not enable the feature.

## Contract and consumption

The build-only `incant_shader_cook::geometry` reads the existing engine-owned
`fixture.wgsl` and `shadows/caster.wgsl`. Neither source, cascade mathematics,
alpha cutoff nor eight opaque/mask × double-sided × reflected-winding variants
changed. Exact Naga29.0.4 IR validation checks both entry points, complete stage
IO, resource types/visibility, camera and material layout, and refuses extra
storage/workgroup resources or overrides. Stage compaction emits six payloads
per family: vertex/fragment MSL2.4, SPIR-V1.0 and HLSL6.0, with reflected entries.

A private common consumer selects only the embedded fixed-family manifest.
Contract, source/producer identity, selected stages, entries and payload hashes
are checked before either unsafe shader-module call. Geometry also checks the
actual host vertex/camera/material layouts first. No project path or arbitrary
project artifact reaches this boundary; hashes are integrity checks, not an
untrusted shader admission policy. Unsupported native capabilities/backends fail
explicitly, with no silent WGSL fallback. The diagnostic pipeline now supplies
its explicit empty layout. The cooker is absent from resolved normal runtime
dependencies; wgpu's compiler remains for other families.

The complete host layouts, not only used attributes/resources, determine pinned
wgpu-hal29.0.4 bindings:

- Diagnostic: stride28 vertex locations0/1, no resources. Metal pulls bounded
  vertices from slot15 and reads supplied lengths at buffer0; DX12 special
  vertex/instance constants use b0. Its fragment returns RGBA.
- Shadow: stride48 mesh and stride112 instance streams preserve every existing
  attribute, offset and step mode. Metal uses slots15/14, camera buffer0 and
  lengths buffer1; fragment material/texture/sampler slots are each0. Bounds and
  base vertex/instance behavior match the pinned normal shader path.
- The full material layout has five texture/sampler pairs, even when shadow
  shading uses only the first. DX12 camera/material use b0/b1, texture t0,
  sampler-index SRV **t5**, base sampler index0, standard/comparison heaps
  s0/s2048 and special constants b2, all in space0. Paired fragment reflection
  supplies the vertex IO contract; opaque variants retain the same vertex stage
  and omit the fragment stage, as before. Actual depth-only pipeline acceptance
  is still a driver/GPU obligation.
- Vulkan keeps explicit sets/bindings, bounded access/loops and SPIR-V1.0 with
  no added Y flip (hal supplies the negative-height viewport). Geometry needs
  only the Shader capability. Storage-length and compute metadata are outside
  this fixed contract.

Both shaders remain triangle programs. Pipeline blend/depth/cull/bias settings,
batching and arithmetic are unchanged. The finite eight-variant shadow catalog
is preserved; it is not per-level permutation recording.

## CPU evidence

[Exact source, logs, pinned backend files and artifact receipt](evidence/cooked-geometry-shaders.json)
bind the freeze on `df71ef8` (preserved shader branch plus actual main103).

- Ten cooker tests pass: five existing output cases and five geometry cases
  covering deterministic emission, SPIR-V parsing/validation/resource interfaces,
  host-binding/IO/refusal mutations, Metal bounded pulling and DX12 sampler
  indirection. This is emitted-source validation, not driver execution.
- 42 cooked renderer and 36 default renderer CPU tests pass; 18 GPU tests remain
  ignored in each run. Private tests select every native descriptor without WGSL,
  check host/source identity and reject cross-family manifests. Existing output
  corruption/capability controls are retained through the shared consumer.
- Default and cooked all-target Clippy, default and feature wasm32 library
  compilation, fmt/conventions and the actual runtime dependency guard pass.
  Main103's WebGPU portability repair is preserved.
- Apple Metal32023.921 compiles and links all four new stages with
  `-std=macos-metal2.4`. Their compiled libraries are validation artifacts; the
  runtime embeds MSL for normal backend driver compilation. No verified DXC was
  available, so no HLSL/DXIL compilation is claimed.
- All six original output payloads remain byte-identical. Its producer identity
  changes because the cooker module graph changed; the shader payloads did not.
  The immutable `550b563` default/cooked comparison binaries and their pairing
  manifest were rehashed unchanged. They remain the first planned GPU comparison,
  separate from this later geometry source.

Initial attempts found a missing Naga `per_primitive` pattern, temporary Rust
vertex attribute arrays and a test expecting shortened HLSL register syntax.
These were corrected before the final source freeze. The first Metal CLI used
an unsupported shorthand standard flag; rerunning with the documented platform
spelling compiled the identical sources. Failed logs remain in the receipt.

## Remaining renderer work

Sprites, light-grid compute and concatenated material shaders still translate
from WGSL at runtime. The latter two need a separate, exact storage-length and
compute contract before native passthrough. Persistent driver caches, recorded
per-level permutations, asynchronous missing-permutation fallback and hitch/
device measurements remain unfinished PLAN requirements. This slice makes no
whole-renderer translation-removal, backend/device parity or performance claim.

After the existing GPU owner releases its grant, run the preserved output-only
comparison first. Then reuse the existing geometry, shadow mask/reflection/range,
queued-atlas and pipeline-warmup numerical tests under default and cooked geometry.
Any byte mismatch requires diagnosis; Claude owns rendered pixels and appearance
acceptance. Metal success will not imply Vulkan/DX12 or reference-phone evidence.

## Independent review and subsequent output-only check

The source freeze is commit `c5268ae05236e5836f04a6098ac0eef31ca6ad43`.
[Independent review](evidence/cooked-geometry-shaders-review.json) accepted the
18 source files, complete host/unsafe boundary and CPU evidence with no finding.
It does not establish geometry driver or appearance acceptance. No geometry
fixture has run.

After the native review explicitly released its grant, only the preserved
`550b563` output comparison pair ran its two existing HDR tests. Both processes
passed; all 16 normalized readbacks across four formats and resize cases were
byte-identical on Apple M5 Pro/Metal, with matching shader and adapter identities.
The process IDs were independently absent before graphics ownership was returned.
[Exact output-only receipt](evidence/cooked-output-metal-numerical.json).
No PNG, screenshot, geometry draw, provider call or performance measurement was
made. Output and geometry remain optional pending their respective remaining
backend and Claude acceptance.
