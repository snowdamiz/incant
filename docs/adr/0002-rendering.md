# ADR 0002: wgpu renderer with explicit device ownership

Date: 2026-10-08. Status: proposed for director review.

## Decision

Use wgpu 29 over Metal, Vulkan, DX12 and WebGPU. The renderer owns its adapter, device, queue, surface configuration and readback buffers. Feed validated engine state into it; screenshots must read the actual GPU target.

## Evidence and implementation boundary

The initial surface/readback proof now also renders retained imported PBR models, authored environment and clustered lights, and selected perspective cameras. Scene-linear HDR passes and display composition execute through a custom Bevy ECS schedule. [The pass-graph evidence](../spikes/render-pass-graph.md) describes the exact ownership and integration boundary; the full Phase 1 production renderer remains incomplete.

## Consequences and revisit trigger

Pipeline, geometry, environment and bounded attachment/light-grid caches retain GPU resources. Per-frame uniforms are immutable; the caller still owns submission and presentation. The schedule uses pinned bevy_ecs directly rather than Bevy's default RenderPlugin and window driver. Shadows, post-processing beyond tone mapping, mobile tiers, golden-image certification and platform parity remain open. Vulkan-only ash is a fallback requiring a separate portability decision. This implementation update does not change the proposed director-review status.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
