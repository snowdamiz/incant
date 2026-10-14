# Project engine compatibility and pin review

This backend slice implements engine declarations, load/cook validation, explicit
undoable pinning and read-only pin review. It does **not** implement project-file
migration or backup, convert an older format, or complete the Phase 2 migration
requirement. There is no supported pre-v1 project format to convert.

## Compatibility contract

The schema stays v1 using its existing additive optional-field convention.
`engine_version` is an optional SemVer requirement string. Loading a legacy v1
project without it leaves it absent, including canonical save and undo. Creating
a new project with `Project::empty` (including CLI/editor creation) declares the
exact current version, presently `=0.0.1`. This is an engine compatibility range,
not a build or release-content identity. SemVer ignores build metadata.

Requirements are bounded before parsing to 256 UTF-8 bytes and 16 comparators,
with standard semver 1.0.28 syntax and matching. Comma-separated comparators form
an intersection; `||` is not accepted. Whitespace-only input is refused. Matching
uses SemVer prerelease opt-in: ordinary stable ranges and `*` do not admit a
prerelease; an explicit comparator for the same major/minor/patch prerelease is
required. Exact pins use `=MAJOR.MINOR.PATCH[-PRERELEASE]`. Input text is retained,
not rewritten or silently loosened. Incompatible requirements fail semantic
validation even for directly constructed or CRDT-decoded documents. Unsupported
schema versions still fail before typed projection. New fields fail closed in
older strict readers, rather than being silently discarded.

`SetEngineVersion` is an ordinary `incant_cmd` transaction, with the existing
actor, expected revision, validation, journal, undo/redo and replay. The resulting
requirement must match this engine. It does not read/write files or clear a pin.
Undo can restore a legacy absence. Agent and GUI clients use the same command.
Loading an incompatible project never offers an automatic repin: claiming a new
requirement is not a format or behavior migration.

`CommandBus::review_engine_pin`, CLI `review-engine-pin`, and RPC
`project.review_engine_pin` produce a review for an unpinned v1 project only.
The RPC accepts only the expected revision and reads the host's already opened
project path; it cannot accept an arbitrary path. The review binds exact source
SHA-256 and size, project identity, current revision, previous absence, the exact
current requirement, one typed command, and canonical candidate bytes/digest.
The source must parse to the current document. Input and candidate are each
limited to 32 MiB. Preview never advances history or edits a file. Its
`file_migration_supported` field is explicitly false. It is not a stale-proof
filesystem capability: executing its ordinary command changes only the document
and journal; the existing explicit save remains a separate operation.

## File migration follow-up: reviewed design, not implemented

The current source transaction machinery is deliberately limited to registered
`.ts` files. It must not become a generic JSON writer. A later project-file
capability must be created only by the host when opening the canonical document,
binding its directory/file identity, exact digest, project ID and journal. Neither
project data nor an agent command may select an arbitrary `.json` path. The
create-only backup must contain the **exact original bytes**, including original
whitespace, and refuse existing backup paths, aliases, symlinks and stale files.

Required ordering is: validate and stage the complete document/CRDT transaction;
verify opened source identity/digest and expected revision; create and sync the
exact backup; durably record the recovery marker and journal Prepare; displace
and recheck the exact source without clobbering a concurrent edit; publish/sync
the candidate; durably journal Commit; only then advance live document/history;
finish recovery cleanup. Backup/save/commit failures must leave no committed
logical transaction. Abort/rollback must preserve unrelated concurrent edits;
ambiguous failures require explicit recovery, not overwrite. An unused exact
backup after a failed attempt is evidence, not proof of a committed migration.

The existing script protocol cannot simply be reused unmodified: project open
currently parses the document before opening/recovering its journal. A crash
after displacement may leave the document path absent. The follow-up needs a
host-bound recovery bootstrap that obtains the original/candidate checkpoint
from a verified marker/backup before normal project parsing, handles absent or
foreign destination nodes, and reconciles Prepare/Commit/Abort with the journal.
It must also define reversible file-pin undo/redo and narrow size limits. That
cross-layer authority/recovery change will get a separate source review and
fault-injection tests before any migration application endpoint is exposed.

## Verification scope

Focused controls cover legacy canonical bytes/new-project pins, range and
prerelease boundaries, future/old format refusal, older reader rejection,
transaction rollback/revision/provenance/reopen/undo/redo, exact review digests,
concurrent source edits, absence of preview writes, and live headless CLI/RPC.
Published JSON schemas and generated TypeScript retain the same typed contract.
No GUI, file migration, older-format conversion, or Phase 2 gate is claimed.

## Retained CPU results

On the private main269 base `3610036d`, Rust 1.99.0, locked/offline native CI
profile (`aarch64-apple-darwin`, -j2, Rayon2): all 80 document/command tests passed,
as did the live headless CLI/RPC control and the cook/source-free native control
(60 steps). The latter proves exact cooked-byte equality between compatible
pinned and legacy unpinned input and refusal of an incompatible declaration.
Four affected crates pass all-target strict Clippy. All 71 published schemas,
generated SDK and strict TypeScript controls, formatting, conventions and diff
checks pass. The command/doc suite precedes only a headless test binary-name
correction; its tested crate sources are identical. No unrelated golden or
expected asset hash was refreshed.

The first build's schema container-annotation compile error and the first
headless test's incorrect Cargo binary environment name are retained; both
received narrow corrections. A PATH setup failure occurred before any builder.
All seven Rust attempts and all test children are terminal/reaped; the private
target is released. [Portable evidence](evidence/project-engine-compatibility/manifest.json)
retains the exact source/checks and executable identities. No GPU, performance,
GUI, persisted migration or release/phase gate was exercised.
