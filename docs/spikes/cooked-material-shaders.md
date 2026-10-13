# Fixed material backend shaders

The opt-in `incant_render/cooked-materials` path translates the existing imported
material vertex and fragment programs at build time. It includes their lighting
and shadow sampling functions, preserving all three WGSL files byte for byte.
Default and browser builds retain WGSL. Native bundles contain MSL, SPIR-V and
HLSL, bound to source/producer hashes and a fixed host ABI; project content cannot
provide passthrough code. No appearance acceptance is claimed here.

The cook validates both stage interfaces, exact visibility, all 23 resources,
uniform member offsets, runtime storage strides/order, textures and samplers.
SPIR-V is parsed and validated again. Metal uses the existing mesh/instance
pulling contract at slots 15/14, with its vertex lengths at slot 1. DX12 preserves
the full four-group layout, three sampler-index tables, comparison heap and
special constants. Bounds restriction and loop bounding remain enabled.

## Metal storage lengths

Pinned wgpu-hal 29.0.4 drops `sized_bindings` when it receives passthrough Metal
shaders. Its ordinary fragment lengths slot therefore has no values. The shader
cannot safely use that slot merely because offline MSL compilation succeeded.

This path supplies three actual allocation lengths—punctual lights, cluster
counts and cluster masks—through an explicit 16-byte uniform at group 3 binding
7 / Metal buffer 7. The cooker verifies the compact fragment's dynamic arrays
have that exact order, and emits Naga's lengths parameter at that explicit slot.
wgpu reserves its unused automatic slot after it, at 8. The host checks exact
layouts and rejects zero, misaligned or unrepresentable buffer lengths. Counts
come from retained immutable bindings, not the current logical light count.
The cluster compute layout and algorithm are unchanged.

## Verification and boundary

The frozen 33-input source passes 92 CPU tests across ten groups with every
implemented cooked family enabled, scoped all-target Clippy, 63 default library
tests, browser compilation, formatting and conventions. The 82 GPU tests remain
ignored in this CPU run. Both generated material stages also compile and link
with the installed Apple Metal compiler: five commands return zero. These are
compiler checks, not GPU execution. Prototype compile errors were corrected
before this freeze; they were not counted as passing checks.

The [manifest](evidence/cooked-material-shaders.json) records each command,
source hash and a lossless archive of raw logs and generated artifacts.
Independent nonvisual review is accepted after rehashing all 33 inputs and
tracing the pinned backend bindings. Claude's same-source baseline/candidate
native GPU and rendered-image review remain pending. Existing shader math and goldens
must not be altered to manufacture parity. Vulkan/DX12 and physical-phone
execution remain separate evidence.

This does not finish offline rendering. Cluster assignment still uses runtime
WGSL; Metal compute passthrough additionally lacks workgroup-memory reflection.
That requires a separate verified backend solution. Persistent pipeline caches,
asynchronous fallback/prewarming and the plan's device hitch budgets remain open.
