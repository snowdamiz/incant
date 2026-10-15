# Current project context for agent turns

Every exclusive, shared and retained-conversation agent turn now starts with a
fresh read-only authoring snapshot. It includes the observed revision, project
identity, tick/physics settings, bounded scene/entity and asset/script inventories,
project memory, the latest applied transaction descriptions, and up to two
complete schemas for components on sampled entities. No source files, account
credentials, transaction before/after documents or screenshot pixels are read.

The shared bus is held only while reading bounded collection prefixes and recent
history. Schema generation, encoding, provider requests and approval callbacks
run after releasing it. All mutations still require the existing tools, approval
policy and revision checks; a snapshot can become stale while a provider works.
The ordinary cancellation and project/account fences remain in place.

Project content is explicitly untrusted user-role data. Names, descriptions and
memory never enter a system message. The actual user prompt remains last. A
retained conversation receives a new snapshot on every turn; previous snapshots
are excluded from retained exchanges, including after manual edits and Undo.
This role separation is not a completed prompt-injection red-team gate.

## Bounds and coverage

The read samples at most eight scenes and eight entities per sampled scene, with
at most sixteen component names per entity. Assets, scripts and memory each
sample at most eight records; history samples the latest eight applied
transactions. Collection totals and sampled rows are separate, and the snapshot
explicitly says missing entries do not establish absence. Text excerpts carry a
truncation flag and stop on UTF-8 boundaries. Command, script and schema tools
remain available for exact follow-up queries.

Scene rows share 16 KiB; asset and script rows each have 2 KiB; memory has 8 KiB;
recent transaction rows have 4 KiB. Separate allowances prevent a dense scene
from consuming the memory/history allowance. Complete schemas share 8 KiB; a
schema that does not fit is omitted, never clipped. Final serialized message
admission is 96 KiB, including JSON escaping. The existing complete-request and
usage-budget checks also apply before networking.

## Qualification

The scoped agent suite passes 113 tests, including five new controls for actual
provider request contents, shared-lock release, fresh manual-edit/Undo state,
untrusted message roles, real registry schemas, bounded Unicode/control-character
data and retained-history behavior. Providers in these controls are deterministic
protocol doubles, not live-account evaluations. No GPU, native window or screen
capture is used.

The first build was interrupted before tests by a disk-growth monitor that treated
disappearing compiler temporary files as a fatal `du` error. Its original log is
retained. The repaired sampler skips only files that vanish during traversal and
keeps the preceding measured growth in the cumulative ceiling. The subsequent
managed test child and strict all-target Clippy completed successfully. Independent
source review found no blocker. The [retained evidence](evidence/agent-project-context.json)
binds the five source files, original interruption, passing logs and review.

This supplies current project orientation for the overlapping Phase 3 work. Open
panel context, the skill library, broader context selection and full live-agent
evaluation remain separate requirements. It does not approve a phase gate or
claim the model will follow untrusted-data instructions correctly in every case.
