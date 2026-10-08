# ADR 0004: Kira with a thin engine mixer

Date: 2026-10-08. Status: proposed for director review.

## Decision

Retain Kira and an engine-owned bus abstraction. Persistent document references describe clips and buses; runtime handles do not appear in documents or agent tools.

## Evidence and implementation boundary

No audio engine exists yet. Phase 1 owns implementation and platform latency measurements.

## Consequences and revisit trigger

Spatialization, interruption handling, device changes and mobile audio sessions require target tests. FMOD is an explicit optional adapter for a later studio requirement, subject to licensing review.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
