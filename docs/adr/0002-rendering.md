# ADR 0002: wgpu renderer with explicit device ownership

Date: 2026-10-08. Updated: 2026-10-10. Status: architecture adopted by the
director in PLAN.md revision 4; implementation and device gates incomplete.

## Decision

Use wgpu 29 over Metal, Vulkan, DX12 and WebGPU. The renderer owns its adapter, device, queue, surface configuration and readback buffers. Feed validated engine state into it; screenshots must read the actual GPU target.

Cook native shaders per backend, persist device/driver-compatible pipeline caches,
and pre-warm each level's recorded permutations while loading. An unexpected
permutation compiles asynchronously with a pre-warmed fallback material; pipeline
creation must not block gameplay. WebGPU retains WGSL, compiled during loading.
Track compilation events alongside frame times so hitch checks measure causality.

Use temporal upscaling and dynamic resolution on every tier, including a portable
temporal implementation and MetalFX on Apple platforms. Native Metal/Vulkan/DX12
interop passes declare resource ownership, synchronization and capability needs
in the render graph and retain a portable fallback. Vendor upscalers, async
compute and hardware ray tracing do not justify abandoning those fallbacks.
Claude owns rendered appearance review; Astra owns implementation and timing.

## Evidence and implementation boundary

The initial surface/readback proof now also renders retained imported PBR models, authored environment and clustered lights, and selected perspective cameras. Scene-linear HDR passes and display composition execute through a custom Bevy ECS schedule. [The pass-graph evidence](../spikes/render-pass-graph.md) describes the exact ownership and integration boundary; the full Phase 1 production renderer remains incomplete.

## Consequences and revisit trigger

Pipeline, geometry, environment and bounded attachment/light-grid caches retain
GPU resources in memory. They are not persistent driver caches. Per-frame uniforms
are immutable; the caller still owns submission and presentation. The schedule
uses pinned bevy_ecs directly rather than Bevy's default RenderPlugin and window
driver. Current shader modules are created from WGSL; cook-time translation,
persistent caches, pre-warming, hitch instrumentation, temporal reconstruction,
dynamic resolution and native interop remain implementation work. Existing
directional shadows do not complete the full lighting/post-processing or mobile
tier gates. Golden-image certification and platform parity remain open.

wgpu 29's public pipeline cache currently supports Vulkan only; Metal archives
and DX12 libraries require separate interop implementation and measurement. See
the [revision 4 review](../spikes/performance-plan-v4-review.md) for sources and
migration sequencing. Adopting this architecture does not approve a phase gate.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
