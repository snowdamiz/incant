# Cooked motion and temporal programs

`incant_render/cooked-temporal` is an opt-in complete native shader combination
for motion vectors, fixed-resolution temporal accumulation and output-resolution
reconstruction. It also selects the existing output, geometry, sprites, UI, text,
material and cluster families. The GPU host and player forward this feature;
the player enables its existing `render-gpu` path. Partial native cooked builds
continue to return `MotionError::Cooked`. No renderer policy is enabled by default.

The five fixed graphics programs are motion vertex/fragment, temporal resolve,
temporal copy, output resolve and output composite. The sixth program is the
conservative temporal cluster compute variant. All seventeen renderer WGSL files
remain byte-identical to the producer base. A shared, guarded source function
performs the same two cluster-bound substitutions at build time and in the
portable path. The native selection checks that variant's exact source digest.
Default and browser builds continue to compile the original WGSL.

The trusted host cooker admits only the reviewed source digests, then validates
Naga IR: exact entries/interfaces, interpolation, resource count and visibility,
uniform members/types/offsets, depth versus color textures and output locations.
The output resolve preserves its two outputs (HDR color and scalar linear depth).
It emits six graphics artifacts per program and three compute artifacts, with
source/producer/content hashes and versioned ABI identities. These are private
engine build products, not an external shader loader. Bounds restrictions and
loop bounding remain enabled. Unsupported backend, feature, layout, artifact or
source identity returns a typed error; it never retries with runtime WGSL.

## Native binding contracts

Motion uses four vertex buffers with strides 48/48/112/80 and unchanged vertex
locations/offsets and instance steps. Pinned wgpu-hal 29.0.4 assigns Metal pulls
15/14/13/12; the frame is buffer 0 in both stages, the material is fragment buffer
1, and vertex lengths use buffer 1. DX12 assigns frame b0, material b1, special
constants b2 and material sampler indices t5. The last position follows the full
five-texture/five-sampler material layout, even though motion samples only the
base-color pair. The frame remains 144 bytes, material 64 bytes.

The fullscreen programs use the unchanged fragment-only uniform/texture layouts:
64-byte temporal resolve with four textures, texture-only copy, 96-byte output
resolve with five textures and two output attachments, and 96-byte composite with
two textures. Host checks inspect the same declarations used by pipeline creation.
The temporal compute variant reuses the exact checked cluster ABI: 64×1×1 group,
516 bytes of static shared storage, explicit buffer lengths and the existing
Metal/SPIR-V workgroup lowering. No native resource layout is inferred from only
the subset of variables that happens to survive compilation.

Initialization keeps device validation scopes balanced when trusted shader
selection fails. Candidates publish only after the scope and typed selection
results succeed. Existing history, frame math, memory limits and default behavior
are unchanged; shader selection adds no per-frame source translation.

## CPU and offline evidence

The [evidence manifest](evidence/cooked-temporal-programs.json) binds the exact
source freeze, copied pinned backend implementation, original WGSL identities,
commands, compiler products and lossless raw logs. Independent source review is
accepted (45 exact inputs and pinned backend contracts); final execution-packet
review remains pending. No GPU execution, screenshot, image comparison or appearance approval
is part of this producer packet.

- Cooker: 28 passing tests, including exact source refusal, independent malformed
  IR controls, native register/vertex-length checks and parsed SPIR-V validation.
- Complete renderer/GPU-host libraries: 128 passing CPU tests; 40 GPU tests ignored.
- Complete-feature integration targets: 3 passing CPU tests; 58 GPU tests ignored.
- Default renderer/GPU-host libraries: 114 passing CPU tests; 40 GPU tests ignored.
- Partial cooked combination: the explicit motion refusal test passes.
- Four-package all-target Clippy, browser compilation, selected normal dependency
  closure, authoring-free runtime dependency guard, formatting and conventions pass.
- Offline: 11 emitted MSL stages compile and link, and all 11 SPIR-V stages pass
  `spirv-val --target-env vulkan1.0` (36 actual compiler/tool commands). The 11 HLSL
  translations are retained. DXC is unavailable locally; no DXIL compilation or
  DX12 execution is claimed.

The first cooker assertion incorrectly expected HLSL texture loads to emit an
explicit `GetDimensions`; Naga emits native `Load` operations. That failed run is
retained, followed by the corrected passing assertion. An initial invocation of
the existing dependency guard omitted its mandatory root arguments; its usage
refusal and the corrected passing invocation are both retained. Browser check
warnings are the pre-existing native timestamp paths, not failures or waived
cooked-temporal diagnostics.

## Remaining execution boundary

Existing ignored motion/temporal/upscaling numerical tests exercise the complete
native path when invoked with `--features cooked-temporal`. They need delegated
actual backend execution and same-source baseline comparison. Their old partial
cooked runs only prove refusal and must not be relabeled as temporal execution.
Metal libraries above are offline validation products; runtime embeds MSL and
HLSL source for their respective drivers, and SPIR-V for Vulkan. Driver compilation,
visual acceptance, Vulkan/DX12 parity, device memory/timing and the reference-phone
budgets remain separate. MetalFX and subsequent effects composition are not added
by this slice. Existing CI's partial-family commands are unchanged.

A concurrent portable temporal edge/overlay correction follows the rejected
Claude 0072 interpretation. This packet intentionally retains the prior `a337c86d`
math and does not approve that rendering. Corrected WGSL must be composed from
its immutable reviewed source, explicitly recooked with updated trusted source
identities, and revalidated; the current hashes must not be relabeled as evidence
for those future bytes.
