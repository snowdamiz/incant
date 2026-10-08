# ADR 0001: Bevy ECS on Rust

Date: 2026-10-08. Status: proposed for director review.

## Decision

Use Rust 2024 and Bevy 0.19, with default rendering/window plugins disabled in the shared core. Pin the toolchain and Cargo.lock. The editor document is authoritative; the ECS is a disposable projection with stable document IDs. Fixed ticks are explicit rather than wall-clock driven.

## Evidence and implementation boundary

incant_core runs fixed-step systems, snapshot queries and validated projection reloads. The platform probe links this actual crate. The pure runtime projection does not link authoring CRDT code.

## Consequences and revisit trigger

Projection rebuilding is intentionally simple in this spike and must be replaced with incremental updates before production-scale scenes. Adopt a custom ECS only after profiling demonstrates a scheduler bottleneck.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
