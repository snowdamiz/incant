# Editor subtree duplication

The native host prepares fresh identities, then executes the existing shared
`DuplicateSubtree` command as one user transaction. It accepts the observed
project revision and refuses a stale request before allocating or writing.
The shared command retains its complete-subtree, reference-remapping, singleton,
prefab and expansion rules. Undo removes the whole copy; redo restores its same
identities and provenance. No GUI-only document mutation is introduced.

Preparation scans the scene once to build a parent adjacency map, then bounds
the copied subtree and its pending traversal frontier to 1,024 nodes. Adjacency
storage scales with the scene; this is authoring work, not a runtime operation or
a 100,000-entity editor performance result. Preparation alone does not mutate or
reserve IDs. Execution still validates freshness against all project identities.

The bridge advertises duplication only when the native host explicitly supports
it. It sends one request and publishes the authoritative returned snapshot, never
an optimistic copy. The receipt must identify the same project, a fresh root,
the next revision and a valid transaction ID. An exact-revision snapshot must
contain that root in the requested scene. A later snapshot may legitimately
reflect a subsequent deletion; selection must use the current hierarchy.

Rejected IPC can follow a committed transaction because snapshot reading happens
after execution. Both rejected and unverifiable completions retain an unknown
outcome, refresh state and never automatically retry. Predispatch capability,
missing-entity and busy refusals remain ordinary failures.

## Qualification so far

Native source `88d51b41` passed six subtree tests, nine refusal tests and the real
editor host transaction test, plus all-target strict Clippy for `incant_cmd` and
`incant_editor`. The first build supervisor stopped when `du` raced removal of a
temporary compiler file. Its process group was cleaned up; the bounded monitor
repair resumed only the interrupted test and pending lint. The original failure
and cumulative storage/time accounting remain retained in
`editor-subtree-duplicate-001` under the integration evidence directory.

Independent review identified two completion-contract gaps. Bridge successor
`1885d139` fixes both, with 62 native bridge tests and the project-reference
TypeScript check passing. Its generated transport was refreshed. Native behavior
is unchanged. Final source review accepts those repairs.

The editor interaction is still being implemented by Claude Opus 5.5 with Max
thinking in handoff 0078. UI tests, rendered review and an actual native editor
workflow remain pending. This document does not claim Phase 2 completion.
