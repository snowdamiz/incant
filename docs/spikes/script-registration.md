# Undoable script-source registration

The shared `incant_cmd` bus now accepts `upsert_script` with an existing typed
`ScriptSource` (`id`, `name`, `path`) and `remove_script` with a `script_id`.
These commands edit `Project.scripts` metadata. They do not create, read, write,
delete or compile source files and do not require a filesystem root or account.

Registration and a `Script` component attachment can share one transaction, in
either command order. Removal and detachment work the same way. The bus stages
all commands and validates the completed project before committing, so existing
ULID uniqueness, portable relative path and source-reference checks apply.
Removing a missing source fails explicitly; removing a referenced source fails
unless the transaction also removes or replaces its references. Failed commands
cannot commit a valid prefix or change revision, undo, redo, CRDT or journal state.

Upsert replaces the metadata under its existing stable ID. The document's prior
semantics are preserved: registration does not require a particular extension,
a nonempty display name or an available file. Undo and redo restore complete
transaction snapshots. The journal retains the original `Transaction.actor`,
including agent model and conversation ID, and the commands. Entity attachment
and detachment retain the normal entity provenance. `ScriptSource` itself has no
new provenance field.

The existing agent `doc_patch` tool exposes these operations from the derived
command schema. All document patches already require approval in `Destructive`
and `Always` modes, covering script registration, replacement and removal.
`Auto` keeps its existing behavior. The description now explicitly identifies
the metadata-only scope. There is no new agent tool or editor mutation route.
The generic native command endpoint accepts the same typed commands; the
existing editor bridge needs no new operation for this foundation.

## Local validation

The following passed on frozen Rust sources with a fresh isolated Cargo target
at `/tmp/incant-script-registration-target`; no other agent's target was used:

- `./tools/cargo test -p incant_cmd -p incant_agent --all-targets --profile ci --locked -j2 --target-dir /tmp/incant-script-registration-target`: 68 tests passed (31 command, 37 agent), including eight new tests and the existing 10,000-sequence undo test.
- `./tools/cargo clippy -p incant_cmd -p incant_agent --all-targets --profile ci --locked -j2 --target-dir /tmp/incant-script-registration-target -- -D warnings`.
- `./tools/cargo run -p incant_headless --profile ci --locked -j2 --target-dir /tmp/incant-script-registration-target -- schema schemas`: regenerated the Rust-derived command and agent-tool schemas.
- `node tools/generate_sdk.mjs`, `node tools/generate_sdk.mjs --check` and `node_modules/.bin/tsc -p sdk/ts/tsconfig.json`: regenerated and checked the TypeScript command declarations.
- `node tools/build_bridge.mjs --check`, `./tools/cargo fmt --all --check`, `python3 tools/generate_conventions.py --check` and `git diff --check`. Both instruction files were regenerated and remained unchanged.

New behavioral coverage includes both command orders for registration/attachment
and removal/detachment; stable-ID replacement with undo/redo; invalid paths,
ULIDs, globally colliding IDs, missing sources and dangling references; stale
revisions and incomplete agent metadata; refusal preserving pending redo, CRDT
and journal bytes; journal reopen across registration, replacement, removal and
undo/redo; retained actor provenance; and approval denial before mutation.
Agent checks use a local scripted provider and do not count as a live-model gate.

## Remaining boundary

This registration change did not include file-content editing. The subsequent
[source transaction slice](script-source-transactions.md) adds bounded
`script.read`/`script.write`, exact-preimage Undo/Redo and crash recovery through
the shared bus. A script editor and complete agent-authored gameplay remain open.
These functional checks do not satisfy a phase, device or performance gate.

## Combined authored-native integration

The scoped change was integrated with project-attached native TypeScript.
115 command, agent and headless release tests across 35 groups and scoped
all-target Clippy passed; two GPU tests were ignored. A rebuilt public CLI
regenerated all 58 schemas identically; strict SDK/UI types and generator,
formatting and conventions checks pass.

A separate real two-process RPC exercise registers both source records and
attaches three entities in one command transaction, then runs 20 native ticks
and observes the expected X positions 2, 3 and 4. Play preserves the authoring
document and revision. Undo returns to the initial project; Redo restores the
complete registration/attachment transaction; a fresh process recovers that
same revision and project from the journal. Restart creates a new tick-zero
world. This used new engine-owned fixture files, with no direct mutation of an
existing project. [Integration evidence](evidence/script-registration-native-play.json).

The generated Rust SDK/color union and hosted checks remain separate integration
work. The source-content followup is documented above; complete script/game
authoring remains open.
