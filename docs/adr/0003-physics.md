# ADR 0003: Jolt behind a Rust-owned physics boundary

Date: 2026-10-08. Status: proposed for director review.

## Decision

Retain the plan’s Jolt choice behind an adapter with stable entity IDs. No raw engine pointers cross scripts or document serialization. Physics world state belongs to play sessions, not authored files.

## Evidence and implementation boundary

This phase has only deterministic linear motion for integration tests. Jolt has not been implemented or measured; that work begins after the Phase 0 gate.

## Consequences and revisit trigger

Jolt portability, determinism and mobile FFI costs remain hypotheses. Benchmark Jolt versus Rapier during Phase 1. A binding failure or unacceptable cross-platform determinism is grounds for choosing Rapier; do not silently replace the physics backend.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
