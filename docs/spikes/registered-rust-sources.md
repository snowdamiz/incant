# Registered Rust source authoring

This source foundation adds stable registered Rust entry IDs and shared atomic
source editing. It is not the complete `module.read/write/build` agent workflow:
the dedicated tool adapters, source-ID-to-build resolver and checked candidate
installation remain follow-on work.
No new compiler, shell, runtime authoring dependency or module selection is added.
The implementation and tests in this branch are source-only until CPU qualification.

`Project.module_sources` maps globally unique ULIDs to `RustModuleSource { id,
name, path }`. This optional map defaults to empty and is omitted when empty, so
legacy projects retain their canonical bytes. Registered source IDs are distinct
from compiled Wasm asset IDs and scene programs. Registration does not select,
compile or execute anything.

`incant_cmd::Command` admits `UpsertModuleSource`, `RemoveModuleSource` and
`WriteModuleSource`. The write carries the stable module ID, exact observed
`SourceVersion` and replacement UTF-8 text; execution also requires the observed
project revision. `CommandBus::read_module_source` returns the registered path,
revision, text or missing state, content version and whether current bytes match
the latest tracked source edit. It does not reconcile external changes. These
operations use the same trusted project root, exclusive persistent journal,
source recovery guard and transaction provenance as TypeScript source edits.
There is no direct editor/agent filesystem mutation route.

Exactly one source write is allowed in a transaction, across both TypeScript and
Rust. Mixed TS/Rust or multiple Rust writes refuse before file or document
publication. Registration, one write and ordinary document changes can be one
reversible transaction, with registration resolved from the final document.
The existing Prepare/install/Commit/Abort protocol, exact preimages, Undo/Redo,
source epoch and recovery paths are reused. `SourceEdit.kind` defaults to
TypeScript and is omitted for TypeScript records. Rust records explicitly use
`"kind":"rust"`; the historic `script_id` wire field carries either kind's
stable source ID. Old journal records do not gain a required field or a new
version. Old binaries fail closed on new Rust records.

The source-only child implementation also creates the first nested package
through the same registration and source-write transaction. Paths
must be project-relative, ASCII, at most 1,024 bytes, at most eight components,
and at most 128 bytes per component. Components begin with an ASCII letter,
digit or underscore and otherwise contain letters, digits, `_`, `-` or `.`.
Rust entries end in lowercase `.rs`; hidden names, traversal, Windows device and
trailing-dot/space aliases, `build.rs`, symlinks and hardlinked source files are
refused. Root-level `main.rs` is explicitly refused: the current worker package
snapshot must not include unrelated project-root JSON/assets. A future adapter
can resolve `movement/main.rs` as package `movement`, entry `main.rs`, or
`movement/src/lib.rs` as package `movement`, entry `src/lib.rs`, without accepting
the project root. The package still has to satisfy the worker's complete source
snapshot and Rust-only recipe; registering a path does not certify that package.

Reads and writes admit at most 64 KiB of Rust UTF-8 bytes. TypeScript keeps its
existing 256 KiB bound and path admission, including missing-parent refusal.
Missing Rust parents are observed as `SourceVersion::Missing`; a checked Rust
write can publish their bounded, privately staged subtree. No recursive deletion
is added.
Removing registration or changing its path leaves previous bytes intact. Empty
text is an authored source value, not file deletion. External edits cause stale
write/Undo/Redo refusal rather than being overwritten.

The directory transaction stages at most seven missing directories and one
64 KiB source in `.incant-source-transactions`. Existing ancestors are never
owned or removed. An optional `SourceEdit.created_tree` records a fresh token,
the first missing relative component and exact directory identities; a separate
create-only receipt binds those identities to this root/journal. The field is
omitted for existing-parent edits and all TypeScript edits, preserving historical
wire bytes. No tool argument can supply an ownership record.

After durable Prepare, one no-replace rename publishes the complete subtree.
Unix uses the existing descriptor-relative `renameat_with(NOREPLACE)`; Windows
uses the existing pinned no-reparse parents and `MoveFileExW` without replacement
or cross-volume copy. Unsupported operations fail rather than falling back to a
check-then-overwrite rename. The same primitive moves the subtree into retained
transaction storage on Undo and back on Redo. Nothing recursively deletes an
owned tree, including after an abandoned redo history.

Before and after movement, bounded inspection requires each recorded directory
identity, exactly one expected child per directory and exact regular-file bytes.
Foreign entries, replaced directories, symlinks, hardlinks or destination races
refuse. An ambiguous race after movement preserves the displaced subtree and
pending marker for recovery; foreign content may then reside in retained state
and is never deleted automatically. Partial private stages before a durable
marker remain inert. Reopen validates the ownership receipt alongside the existing
journal before rollback or repeated Undo/Redo. This is not a new guarantee against
arbitrary external writers racing after the final observation.

Ownership binds the canonical journal path and exact root/directory identities.
Copying or moving a project does not transfer first-package Undo/Redo authority;
these operations fail closed rather than silently rebinding receipts. Same-path
reopen remains supported. A focused move/reopen/refusal/return test is authored;
its execution remains pending with the other native controls.

Compatibility review covers serde/CRDT materialization, global ID and path alias
validation, save/reopen, legacy journal decoding and generic destructive `doc_patch`
approval. The UI's existing approval-description renderer identifies the new
operations without implying build authority or file deletion. Fresh schemas and
TypeScript declarations must be generated through the ordinary headless schema
export and SDK generator before integration; handwritten generated snapshots are
not evidence.

Authored controls cover legacy canonical/wire bytes, typed schema admission,
registration/write order, exact provenance, external CAS conflicts, mixed-write
refusal with no published prefix, source bounds/links, metadata-only removal,
save/reopen and real subprocess recovery at every existing durable boundary for
apply, Undo and Redo, including first-package creation. Additional controls cover
an empty saved project, preexisting ancestors, no-replace collisions, same-content
directory replacement, foreign additions and ownership metadata tampering.
The subprocess fixtures are self-contained and need no
external project, compiler, device, window or account. CPU execution and generated
contract checks are pending; no build, rendering or performance gate is claimed.
