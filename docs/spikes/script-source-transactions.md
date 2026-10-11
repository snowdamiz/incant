# Script source transactions

`incant_cmd::Command::WriteScript` edits one real UTF-8 `.ts` file, at most
256 KiB, in the same atomic transaction as document metadata and entity
attachments. It requires the observed document revision and a `SourceVersion`
(`missing` or `present` with SHA-256). The final project is validated before
any file operation; the written stable script ID resolves against that final
project, so registration, source writing and attachment work in either order.
Multiple source writes, aliased registered paths, missing parents, `.d.ts`,
invalid paths, symlinks/reparse points and hardlinks are refused.

`Project.scripts` remains metadata only. The bus captures the actual disk
preimage and postimage in its transaction history, outside the document/CRDT.
Undo restores the exact captured preimage; Redo restores the postimage. Both
require that the current file still matches the expected predecessor. Undo of
initial creation removes only the matching created file. `RemoveScript`
continues to unregister metadata without deleting a file. Reads return actual
disk content and `matches_history`, exposing external edits rather than silently
treating a document copy as filesystem truth.

## Capability and approval

Only trusted hosts construct `CommandBus::persistent_with_sources(journal,
project, root)`. The root is derived from the project opened by the native
editor or CLI; no engine-agent argument supplies a root or arbitrary filename.
Source-independent, simulation and CRDT-only buses explicitly return typed
`SourceError::Unavailable` for content edits. `ProjectHost` does not grant the
capability. The authoring-only `incant_source` crate is forbidden transitively
from shipped runtime roots by the global dependency guard.

Agent `script_read` returns an `untrusted_source_data` wrapper; `script_write`
uses the shared command bus and requires destructive approval under the existing
Destructive/Always policies. Combined registration/content/attachment uses
`doc_patch` with the same approval rules. Cancellation is checked after approval
and before entering the transaction; an in-flight prepared transaction finishes
its commit/rollback protocol. The transaction records the current Actor, model
and conversation ID. It does not claim authorship of an adopted external
preimage. The source-independent host does not advertise the read/write tools.

Public RPC exposes `script.read`/`script.write` using those same dispatchers.
The native editor's generic typed command endpoint accepts `write_script`; file
commands, Undo/Redo and the registered-ID `engine_script_read` run on blocking
workers rather than the native event thread. No visual editor or agent shell
access is introduced. Commands do not compile or execute source.

## Durable protocol and recovery

Each operation has a fresh ULID. One root lock covers every engine writer,
including writers using different journals. A durable pending marker binds the
operation to its project ID and canonical journal path. The ordered steps are:

1. Validate the complete transaction, expected revision, current source version
   and confined path. Stage the new content and publish the pending marker.
2. Append and sync a version-2 journal Prepare containing the logical change,
   exact file effect and intended CRDT projection.
3. Move the actual old file into retained transaction storage without clobbering
   and verify it. Move the staged file into place without clobbering and verify
   its contents. Creation never overwrites a newly appeared external file.
4. Append and sync Commit, publish the source epoch receipt, remove the marker,
   and publish the matching logical document/history state.

An ordinary installation or journal-write failure rolls back only matching
files, appends Abort and clears the marker. Partial journal writes are truncated
and synced back to the prior position. If that repair is uncertain, the journal
is poisoned and the bus returns typed `RecoveryRequired` without guessing which
version committed. A durable Commit is authoritative: if receipt/marker cleanup
fails, the logical transaction remains committed and the caller receives
`RecoveryRequired`; reopening finishes cleanup before another operation.

Reopen first replays logical history without executing historical file effects.
Only the single unresolved root-bound pending operation may reconcile disk.
An unresolved Prepare rolls back and appends Abort; a committed or aborted
operation only completes receipt/marker cleanup. A crash before Prepare clears
its marker only if the source still matches its preimage. A missing/wrong root,
foreign bytes or a foreign node preserves retained files and requires explicit
reconciliation. Completed historical source changes never overwrite later
external edits or recreate subsequently deleted files. Existing version-1
metadata journal serialization remains unchanged.

The [platform store](script-source-store.md) uses directory capabilities on
Linux/macOS and pinned no-reparse directory handles on Windows. Actual displaced
nodes remain available, including conflicting external edits. This is not a
filesystem compare-and-swap against arbitrary external editors; open handles
and in-place external writes can outlive a rename. The live source pathname can
briefly be absent between moves. Standalone consumers call read-only
`incant_source::check_recovery(root)` before and after graph capture and compare
the opaque `SourceEpoch`; a pending operation or changed epoch refuses capture.
Readiness checks do not create files or recover on read. The separately owned
fixed compiler/build integration wires that guard into standalone consumers;
the bus, RPC and agent entrypoints are guarded here.

## Local evidence

Frozen sources were built with `./tools/cargo`, `--locked -j2 --profile ci` and
the private `/tmp/incant-script-registration-target` target. Actual Cargo handles
were reaped before further edits. The following passed locally on macOS:

- `cargo test -p incant_source -p incant_cmd -p incant_agent -p incant_headless -p incant_editor --all-targets`: 145 tests passed across 40 groups; three GPU tests remained ignored.
- Matching scoped all-target Clippy with `-D warnings`, Rust format check and seven runtime dependency guard regressions. The resolved runtime graph remained authoring-free for `incant_runtime`, `incant_script`, `incant_game_runtime` and `incant_game_player`.
- Actual subprocess death at seven journal boundaries for each of Apply, Undo and Redo (21 cases), including partial Prepare/Commit records; six actual partial-write/OS-error abort cases; missing-root and post-crash foreign-content refusal.
- Seven source-store process-crash boundaries, including file displacement and both receipt moves with an existing receipt. Path, link, create-race, displaced-external-file, exclusive-root-lock and read-only epoch tests exercise the real local filesystem.
- Final-project registration/content/attachment, exact external preimage Undo/Redo, actor provenance, stale revision/version refusal, bounded writes, read visibility, historical replay without disk effects, absent capability, approval denial and cancellation after approval.
- A fresh headless binary generated all 59 contract files byte-for-byte. SDK and native bridge generation checks, strict SDK/UI TypeScript, generated conventions and whitespace checks passed.
- The source crate checked for `wasm32-unknown-unknown`; unsupported targets return typed Unavailable with no weaker filesystem adapter.

The public [probe](../../tools/probes/script-source-transactions.py) passed
32 RPC requests across two fresh headless processes. It creates all `.ts`
content through `incant_cmd`, atomically registers/writes/attaches a behavior,
then observes X motion 2, 4, 2, 4 and 2 over five 20-tick native Play sessions
after replacement, Undo, Redo and restart. Play leaves authoring unchanged;
Undo of creation restores the initial document and removes its matching file.
The project checkpoint remains unchanged without an explicit save.
[Machine-readable result](evidence/script-source-transactions.json).

Existing macOS CI and Windows/Linux desktop workspace tests execute the actual
filesystem/crash tests; the public CPU RPC proof reuses their existing binaries.
The platform Clippy step reuses desktop CI's target. No extra three-job cold
matrix is added. Linux/Windows execution is still evidence to collect in those
jobs, not established by local macOS tests or source review. Windows
write-through moves and synced files do not certify physical power-loss
behavior across storage devices.

## Remaining scope

This slice supports one source file per transaction and existing parent
directories. Multi-file graph changes, file rename/delete, source-directory
creation, retained-file garbage collection, explicit conflict-reconciliation
tools/UI, typecheck diagnostics, a visual script editor and complete
agent-authored gameplay remain separate work. It requires no hosted service or
account. The local scripted approval tests and actual RPC processes are not a
live-model, shipping-profile, reference-phone, performance or phase-gate result.
