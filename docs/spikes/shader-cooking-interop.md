# Offline shader translation and the Metal integration boundary

The current renderer still loads WGSL. Preparing its fixed pipeline catalog at
initialization removes lazy pipeline construction from the drawing path, but
does not satisfy PLAN's offline shader or persistent driver-cache requirements.

The standalone [probe](../../tools/probes/shader-cook/README.md) establishes the
next compiler boundary using the same pinned Naga 29.0.4 as wgpu. All six current
shader families (11 entry points) parse, validate, and translate to MSL and
SPIR-V. All 11 emitted MSL stages compile with Apple Metal 32023.921 and link
into separate Metal libraries, totaling 65,567 bytes on this host. Source,
compiler, output hashes and exact scope are retained in the
[local evidence](evidence/shader-cook-translation-local.json). The Xcode Metal
Toolchain 27A266a was installed for this check after the compiler stub reported
that the optional component was missing.

This is compiler evidence, not a usable runtime library bundle. Probe resource
bindings are compact fixtures rather than the actual renderer layout, and
vertex pulling is omitted. No generated library was submitted to a GPU. HLSL/
DXIL, driver acceptance of the SPIR-V output, appearance, runtime integration,
cache persistence and the phone hitch gates remain open.

## Metadata that a native backend must preserve

The pinned `wgpu-hal` package records upstream source commit
`e99f5305ded96ff7006f0714d043a7f735bd45c2`. Inspection of
`wgpu-hal/src/metal/device.rs::load_shader` and
`wgpu-hal/src/metal/command.rs::make_sizes_buffer_update` establishes that its
ordinary Naga path retains dynamic-buffer bindings and threadgroup allocation
sizes. The MSL/Metal-library passthrough path constructs empty lists for both.
Passing translated source to that API is therefore insufficient for our current
material and light-grid shaders.

The actual stage-specific translation confirms these requirements:

| Entry point | Dynamic storage-buffer lengths | Logical reflected threadgroup sizes |
| --- | --- | --- |
| Material fragment | `punctual_lights`, `cluster_counts`, `cluster_masks` | None |
| Light-grid compute | `lights`, `counts`, `masks` | `hits`: 4 bytes; `membership`: 512 bytes |
| Other nine stages | None after selecting and compacting the entry point | None |

These are logical sizes, not the final Metal allocation sizes. The pinned
`metal/command.rs` rounds each threadgroup allocation up to 16 bytes before
calling `setThreadgroupMemoryLength`; the actual allocations are 16 and 512
bytes. A native backend contract must preserve that alignment rule too.

Naga emits a buffer-size structure used by bounds checks. Its compute kernel
also receives the two threadgroup objects as arguments. A precompiled shader
must receive those exact resources, rather than merely compiling successfully.
The current native vertex path additionally uses backend-specific vertex
pulling; its buffer mapping and size data must remain correct. Entry-point
selection removes unused material storage globals from the vertex module, as
confirmed by the probe, but does not remove that vertex-buffer requirement.

The implementation must bind this metadata through a checked native backend
contract, with matching complete layouts and preserved bounds checks, before
enabling passthrough libraries. Numerical GPU comparisons, failure controls and
Claude appearance review must validate that integration. Merely switching to
unchecked passthrough or weakening bounds checks is not an acceptable shortcut.
The eventual shipped graph must also exclude source parsers and backend
translation; this probe makes no claim that this is already true.
