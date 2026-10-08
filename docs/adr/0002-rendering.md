# ADR 0002: wgpu renderer with explicit device ownership

Date: 2026-10-08. Status: proposed for director review.

## Decision

Use wgpu 29 over Metal, Vulkan, DX12 and WebGPU. The renderer owns its adapter, device, queue, surface configuration and readback buffers. Feed validated engine state into it; screenshots must read the actual GPU target.

## Evidence and implementation boundary

incant_render draws diagnostic geometry and captures a PNG through mapped GPU memory on Apple M5 Pro. It is a surface/readback proof, not the Phase 1 render graph or a game-art renderer.

## Consequences and revisit trigger

Pipelines and buffers are currently rebuilt for diagnostic draws. No lighting tiers, material system, render graph, golden-image certification or platform parity is claimed. Vulkan-only ash is a fallback requiring a separate portability decision.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
