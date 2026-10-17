# Editor subtree duplication

Follow-up: the [October 16 native hierarchy workflow](native-hierarchy-window.md)
now verifies the shortcut, menu, history, focus and saved-project route on the
unlocked Mac. Earlier attempts and their limits below remain historical.

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

Claude Opus 5.5 with Max thinking implemented the hierarchy menu, platform
shortcut and shortcut reference in handoff 0078. Its browser fixture review
covers 21 images and five keyboard flows, separately from the native engine.
The root integration adds two reviewed correctness fixes: every late result,
including failures, is fenced to its mounted hierarchy, bridge and project;
explicit selection intent is retained even when the newer selected entity is
subsequently deleted. Automatic disappearance of the original entity alone does
not prevent selection of an accepted copy. Nothing is copied optimistically and
the operation is never automatically retried.

Integrated source `5a819493e89780bb499bfd35df9ac0316e48d70c` passes all 1,025 UI
tests in 49 files, the project TypeScript build, generated-bridge consistency and
the production frontend build. The real host transaction test was rerun after
composition with main342; the native custom-protocol editor and matching
headless tool build successfully. The [qualification receipt](evidence/editor-subtree-ui.json)
binds these distinct sources, raw logs and copied binaries.

The actual Mac editor workflow remains **unverified**. A bare executable could
not be bound by CUA; its owned process was stopped without edits. The identical
binary in a uniquely identified test bundle opened a blank native client.
Read-only OS inspection reported both `IOConsoleLocked` and
`CGSSessionScreenIsLocked` true. The contemporaneous locale player also deferred
every drawable and advanced zero simulation steps. The root stopped and reaped
the editor; its project and journal remain unchanged. No Duplicate input was
delivered, and no actual selection, Undo/Redo or reopen result is claimed.
Repeat that native workflow after the desktop is unlocked. A temporary startup
error diagnostic build was retained but never launched; its source was restored
and it is not the qualified production binary. No Phase 2 gate is passed.
