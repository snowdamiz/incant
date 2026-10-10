# ADR 0008: Canonical JSON and Loro authoring state

Date: 2026-10-08. Updated: 2026-10-10. Status: per-scene authoring architecture
adopted by the director in PLAN.md revision 4; Loro confirmed by the revision 5
decision record (decision 2). Scaling migration incomplete.

## Decision

Choose the plan’s JSON plus JSON Schema fallback over a new RON dialect. Use derived Rust serializers and schemas, stable ULIDs, sorted maps and a canonical formatter. Semantic validation supplements structural schemas. Use nested Loro maps with atomic array values.

Use one lazily loaded CRDT document per scene, keeping project metadata and stable
scene/asset references separately addressable. Authoring commands remain atomic,
reversible and provenance-bearing across affected documents; splitting storage
does not relax cross-scene transaction or recovery guarantees. Cook validated
authoring scenes to a binary runtime format. Loro, project JSON and authoring
commands never enter the shipped simulation dependency graph or frame path.

## Evidence and implementation boundary

Round trips, hierarchy/reference validation, invalid-merge isolation, concurrent edits, undo sequences and crash recovery are tested. The 10,000-entity microbenchmark measured both Loro and Automerge; Loro initialized faster but Automerge had smaller snapshots and a faster measured merge workload.

The current CrdtProject still owns one LoroDoc for the entire Project. Lazy scene
loading is not implemented, and the old benchmark does not satisfy the new
100,000-entity gate. Source/destination scene ownership, references, recovery and
Undo/Redo must remain valid across lazy loads and multi-scene transactions.

## Consequences and revisit trigger

Loro is decided (decision 2) but was not an across-the-board performance winner.
Automerge stays the fallback behind the document boundary. Switch only if the
per-scene Loro implementation misses the 100,000-entity gate below while
Automerge meets it on the same workload. Journal entries currently store full states plus CRDT snapshots, so storage and replay costs need compaction before large projects. Arrays do not merge element-by-element. Rejected semantically invalid merges need an explicit conflict-resolution UX in later phases.

Measure edit latency below 100 ms and viewport responsiveness at 60 fps on the
M1 MacBook Air with 100,000 entities. Virtualize hierarchy, inspector and asset
views; their mounted controls must not grow with every item in an unbounded
document or array. Per-subtree CRDT documents are the fallback if measured
per-scene memory/merge costs miss the target. These are outstanding requirements,
not a retroactive performance claim about the Phase 0 microbenchmark.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
