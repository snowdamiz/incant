# ADR 0008: Canonical JSON and Loro authoring state

Date: 2026-10-08. Status: proposed for director review.

## Decision

Choose the plan’s JSON plus JSON Schema fallback over a new RON dialect. Use derived Rust serializers and schemas, stable ULIDs, sorted maps and a canonical formatter. Semantic validation supplements structural schemas. Use nested Loro maps with atomic array values.

## Evidence and implementation boundary

Round trips, hierarchy/reference validation, invalid-merge isolation, concurrent edits, undo sequences and crash recovery are tested. The 10,000-entity microbenchmark measured both Loro and Automerge; Loro initialized faster but Automerge had smaller snapshots and a faster measured merge workload.

## Consequences and revisit trigger

Loro is provisional, not an across-the-board performance winner. Journal entries currently store full states plus CRDT snapshots, so storage and replay costs need compaction before large projects. Arrays do not merge element-by-element. Rejected semantically invalid merges need an explicit conflict-resolution UX in later phases.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
