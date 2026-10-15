# Opt-in DX12 pipeline libraries

Source implementation only; Mac CPU/cross-check and Windows execution are not
qualified yet. The default feature graph, shader equations and renderer quality
policy remain unchanged. This does not close the shader-stall gate in `PLAN.md`.

`incant_render/dx12-pipeline-library` enables a separate native HAL session on
DX12 devices. It does not advertise wgpu's Vulkan `PIPELINE_CACHE` capability.
The renderer attaches before its finite catalog is prepared and persists only
after successful explicit cold preparation and a closed validation scope.
Later optional catalog preparation uses the same session. Draws and `Drop` do
not read or publish files. This is independent of material permutation recording.

The native session covers `ID3D12Device2` pipeline streams, classic graphics and
compute descriptors. One mutex serializes native load/create/store/serialize,
including concurrent requests for the same pipeline. Semantic keys encode actual
compiled bytecode, the exact serialized root-signature digest and effective
native descriptor fields. They do not use labels, pointer values, raw C layout,
padding or creation order. Stream and classic graphics use the same semantic key;
the selected native load API also validates its descriptor. Mesh/task/view-
instancing descriptors explicitly refuse the diagnostic classic route.

Capture mode reports successful native hits, missing-or-mismatched entries,
other native errors, fallback attempts and successful/failed stores separately.
`new_requiring_dx12_library` is a trusted host diagnostic with no project path or
mode input. It requires an admitted existing library and actual native hits;
unsupported drivers, misses, disabled/overflowed counters and device errors
refuse. It never falls back, stores or publishes. A later cold miss leaves the
previous renderer resources in place.

The only production storage source is a fixed leaf in the current user's known
LocalAppData folder. Every ancestor is held without DELETE sharing and reparse
points are refused. The private leaf and each regular single-link file must be
owned by the exact current user, with a protected explicit DACL granting only
that user, SYSTEM or Administrators. Existing ACLs are never repaired. Fresh
objects receive the restrictive DACL during creation. Directory ancestry is
bounded to 64 components; native payloads are bounded to 64 MiB each. A fixed
`current.library` plus exclusive `pending.library` bounds stored generations and
crash leftovers. Publication checks `FlushFileBuffers` and uses a same-directory
write-through replacement; failures remain observable. These are private host
caches, not a defense against hostile code already running as the same user or
an administrator.

The length/checksum envelope binds conservative engine/HAL source, target,
features, compiler flags/compiler identity, native adapter identifiers and the
queried driver version. Native input backing is immutable and outlives every
library COM interface. Unsupported storage or capture faults retain ordinary
pipeline creation and report that outcome; they never count as cache hits.

Cooked builds keep selecting engine-owned backend artifacts. DX12 still invokes
the existing HLSL-to-DXIL/DXBC compiler before native PSO lookup. A pipeline-library
hit is therefore not evidence that shader compiler work was skipped, nor a
performance measurement. No project-provided executable shader/cache bytes enter
this route.

The Windows-only staged fixture is authored for fresh-process seed, strict
stream/classic reuse, empty/missing/corrupt input and disabled/late-miss controls.
It retains numerical RGBA before assertions and requires exact cross-stage and
ordinary/cooked equality, including opaque/MASK/BLEND models, sprites and text.
Native support must succeed; an unsupported adapter is not an accepted hit or a
silent skip. Windows filesystem controls cover canonical anchors, wrong owner,
broad/null/unprotected ACLs, hardlinks, reparse points, concurrent staging and
held-ancestor replacement. Symlink controls explicitly require Windows symlink
privilege or Developer Mode. No Windows driver or filesystem execution is claimed
by source review or a Mac cross-check.
