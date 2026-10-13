# Fixed cluster assignment backend shaders

The opt-in `incant_render/cooked-clusters` feature cooks the existing clustered
light assignment compute program into MSL, SPIR-V and HLSL. It works independently
of `cooked-materials`; default and browser builds keep the original WGSL path.
The WGSL, cluster dimensions, conservative intersection math, membership masks,
light order and dispatch algorithm are unchanged. GPU execution and appearance
acceptance remain pending with Claude.

The private embedded bundle binds its source, producer and artifact hashes to a
fixed contract: one `build` entry, 64×1×1 invocations, exact builtin inputs, a
128-byte grid uniform, 64-byte light records, two four-byte runtime arrays, and
workgroup storage for one atomic counter plus 128 atomic membership words
(516 bytes). Unknown resource layouts, overrides, storage/access changes and
workgroup sizes refuse during cooking. The host checks its layout and device
limits before creating the passthrough shader. Project content cannot supply
shader code or artifacts.

## Backend storage contracts

Metal passthrough in pinned wgpu-hal 29.0.4 does not populate the generated dynamic
storage lengths or workgroup-memory reflection. The compute host supplies actual
retained allocation byte lengths for lights, counts and masks through a 16-byte
uniform at group 0 binding 4 / Metal buffer 4. Zero, misaligned and unrepresentable
sizes refuse. The compute-only binding is removed before constructing the
fragment group, whose binding 4 remains the shadow texture. The shared length
checker compiles with either the material or cluster feature.

The Metal cooker accepts the exact pinned generated kernel header and moves only
the two workgroup parameter declarations into static kernel-local declarations.
The helper functions and complete kernel body remain byte-identical, including
Naga's zero initialization, three barriers and the shader's own atomic work. The
host passes the exact 64×1×1 workgroup shape; no reflected dynamic threadgroup
argument remains. Changed, additional or ambiguous generated headers refuse.

DX12 keeps the pinned host allocation: grid CBV b0, lights SRV t0, counts/masks
UAV u0/u1, the additional length uniform at CBV b1, and special constants at b2.
It uses robust ByteAddressBuffer loads/stores. No sampler or project resource is
introduced. SPIR-V retains the exact descriptor set 0 bindings 0–3.

## SPIR-V diagnostics and narrow correction

Independent SPIRV-Tools validation found that pinned Naga emits `ArrayStride 4`
on the fixed workgroup membership array. Vulkan rejects this explicit layout
without a block contract ([VUID-StandaloneSpirv-None-10684](https://docs.vulkan.org/refpages/latest/refpages/source/StandaloneSpirv.html#VUID-StandaloneSpirv-None-10684)). The private generated
word correction resolves the uint32 fixed-128 array, its sole Workgroup pointer
and variable, rejects sharing through another pointer/aggregate/signature, and
removes exactly that one decoration. It rejects missing, changed, duplicate,
ambiguous or truncated declarations. Every remaining word—including IDs, header,
initialization and barriers—is preserved. The original invalid bytes and exact
validator failure are retained alongside the corrected validation.

A separate pinned Naga reader limitation remains explicit: its SPIR-V frontend
upgrades the membership array to atomic elements but leaves the aggregate null
store typed as a scalar array. A regression identifies that precise mismatch;
it does not accept arbitrary reader failures. Structural entry/resource checks
still use that reader. Independent Vulkan-environment validation is the SPIR-V
validation evidence, not a claim that the reconstructed Naga IR validates.

## Verification boundary

The final 19-input source passes 25 cooker tests, 69 renderer tests with clusters
alone, 75 renderer tests with all cooked features, and 63 default library tests.
These suites overlap and are not summed. The cooked runs leave all 82 GPU tests
ignored; the default library run leaves 32 GPU tests ignored. All-target Clippy,
both combined and clusters-only browser compilation, the normal runtime graph,
formatting and conventions pass. The three emitted artifacts remain identical
through the final test-helper reorder.

Independent SPIRV-Tools 1.4.363.0 validates the exact final words for Vulkan 1.0
and generic SPIR-V 1.0. Apple Metal 32023.921 compiles and links the final MSL;
only two unused atomic-result warnings remain. Initial reader, metadata and test
failures are retained without counting them as passes. The [manifest](evidence/cooked-cluster-shaders.json)
records frozen sources, commands, raw hashes and lossless retained artifacts.
Independent [source/compiler review](evidence/cooked-cluster-source-review.json)
accepted source005 and its 93 retained artifacts. A subsequent test-only
portability correction keeps the exact Workgroup/fixed128/atomic-scalar checks
while allowing absent SPIR-V debug names. Its seven focused tests pass under the
actual `ci` profile; the separately hashed CI SPIR-V also passes Vulkan 1.0
validation. Naga's default debug metadata differs between profiles, so CI/dev
SPIR-V bytes are recorded separately; MSL and HLSL bytes match. No production
source changed after source005. No GPU was launched. Offline compilation does not establish Vulkan/DX12/Metal
execution parity, appearance, reference-phone budgets or performance.

This closes an opt-in offline compute preparation route only. Pipeline caches,
asynchronous prewarming/fallback, device hitch budgets and actual backend
acceptance remain separate work. The existing WGSL route stays available.
