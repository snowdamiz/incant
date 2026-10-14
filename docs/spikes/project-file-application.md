# Host-bound project save and pin application

This scoped producer is based on actual main274 `b2475341` (reviewed
`c2a055c9` migration production plus the Windows test-only successors). The
prior `999f0cde` pin-only producer remains immutable. It does not include or
claim qualification of the later main275 profiler composition.

The host opens one project and its existing journal before parsing the document.
Editor startup and default CLI authoring keep `.journal.jsonl`; explicit custom
journals retain their canonical path, node, lock and script transaction binding.
The original `.journal` library opener is unchanged. RPC parameters cannot select
paths or backups. No automatic journal discovery, renaming, or merging occurs.
A custom journal can be in another host-selected directory; both directory
handles and the locked journal node remain retained.

Version four adds checkpoint Prepare/Commit/Abort, independently of version
three's strictly validated legacy pin transaction. Save binds the live revision,
trusted actor and *exact last durable CRDT bytes*. It publishes the canonical
current projection and adds no command, revision or undo step. It verifies both
retained document node and exact admitted bytes before staging; same-byte inode
replacement and in-place external edits refuse. Successful publication admits
its own new node and bytes. Logical edits stay durable in the journal even when
save fails. Legacy canonical unpinned documents stay byte-identical on no-op save.

Order is: validate/check revision and capability, reserve journal space for both
Prepare and terminal record, create-only backup/stage/marker, durable Prepare,
displace original, publish staged node, durable Commit, admit result, cleanup.
Known write failure rolls back and records Abort; ambiguous durability fails
closed until recovery. Save Abort and Commit retain exact causal checkpoint
encoding. Read-only scanning validates all records and projection/identity joins
before truncation, filesystem recovery or application admission.

Pin review requires saved logical content and returns one retained review ID;
a new review retires the old one. Apply accepts ID plus expected revision only.
Ordinary commands remain logical edits. Undo/redo of a file pin uses a version-four
checkpoint with the original validated history action and its target text, taking
the current admitted disk checkpoint as preimage. This lets intervening ordinary
edits, saves and history operations compose without pretending a generic save is
a new pin transaction. Undo restores the pin's exact original bytes. Ordinary
undo/redo remains logical until explicitly saved. Checkpoint history replay must
join the current undo/redo entry; arbitrary Apply is refused.

Editor file requests run on the blocking worker against its existing shared bus;
loading/failed/timeout states cannot grant authority. Headless RPC now opens a
persistent host bus even without `--journal`, using its historical derived name.
`project.file` is a closed request with review_pin/apply_pin/save and no text/path.
The legacy `project.save` adapter accepts an omitted expected_revision, selecting
the current revision inside this trusted host handler, and uses the same capability.
A supplied revision must be a u64 and current; null and unknown fields refuse.
The new `project.file` Save requires expected_revision. Neither route accepts
project text, paths or backup names. Standalone
review-engine-pin and project.review_engine_pin remain read-only previews and do
not supply IDs usable for apply. Import/localization/watch checkpoint through the
bus; their old temp-file project replacement is removed. Agent output remains a
separate derived agent-result document, after durable token settlement; it never
serves as the opened project save. Its accounting bootstrap still requires a
readable initial project ID before opening the ledger (no new absent-file agent
bootstrap contract is claimed); normal application/RPC open performs recovery.

Scope is backend/IPC/schema transport, with no UI controls/layout and no gameplay
runtime dependencies. Older-format migration, Save As, journal compaction and
arbitrary file editing are unsupported. The actual headless binary generated four new schemas; all 74 pre-existing
schemas remain byte-identical. The SDK and native transport bundle are generated
from these sources. No UI layouts or new UI controls are included.

Executed controls: legacy/custom journals and script bindings;
ordinary edit/save/pin/undo/redo/reopen; no-op save with exact causal/history
preservation; stale revision/review/node/content refusal; closed request fields;
actual subprocess crash/fault matrix for checkpoint phases; malformed causal
checkpoint and foreign marker refusal without artifact mutation; editor worker
availability/timeout and real RPC same-session roundtrip. Windows observations
must use owning handles or wait until sessions terminate before reading journals.


## Qualified source and evidence

The scoped Mac arm64 checks use Rust 1.99, locked/offline CI profile, an explicit
`aarch64-apple-darwin` target, two compiler jobs and two Rayon threads. All owned
supervisors and process groups are terminal and reaped; the target is released.
This is correctness evidence, not a performance or phase-gate result.

- Source003 passes all 107 `incant_source`/`incant_cmd` tests and strict all-target
  Clippy, including real crash/restart, write/poison, same-byte replacement,
  cumulative journal capacity and causal checkpoint controls.
- Source004 changes only the retained atlas test to use the owned opener/save.
  The headless binary plus nine affected integration harnesses pass 39 tests,
  including live RPC, pin/save/history/reopen, imports, watcher refusal and
  cooked native RPC/UI consumers. All Rust source remains exact in source005.
- Source005 adds generated outputs, nonvisual exhaustive capability mappings and
  bridge controls. Four editor loading/timeout/recovery/file tests pass; the
  other 39 editor tests were explicitly filtered. Both headless and editor pass
  strict all-target Clippy. The normal editor bundle/typecheck, 189 bridge tests,
  SDK typecheck and generator freshness checks pass. All 19 built editor assets
  and immediate executables are retained with their actual source identities.
- Formatting, generated conventions and diff checks pass. No GPU/window was
  executed, and no native IPC pixel or UI interaction acceptance is claimed.

Three failed generations are retained without relabelling: source001's private
snapshot-observer compile mismatch; source002's 28-pass/1-fail collision control,
where the v4 test operation ID did not use the existing injection hook; and the
first normal editor bundle's missing exhaustive capability mappings and invalid
new test observations. Source003 reuses the cfg(test) operation-ID hook while
normal production ULID generation stays unchanged. The transport correction
adds only capability routing/reason mappings, not UI layout or appearance.

Independent root/core and Sol boundary reviews precede the successful execution.
Source003/004 receipts and source001 findings are retained in the lossless packet.
The final frozen code, generated artifacts, raw logs and historical source copies
can be replayed without compiling or running the engine:

```sh
python3 docs/spikes/evidence/project-file-application/collect.py --root . --verify-only
```

Add `--originals --local-binaries` on the producer host to verify the external raw
files and immediate executable copies too. [Manifest](evidence/project-file-application/manifest.json).
Application source and local scoped checks are complete; current-main intake,
required hosted checks, editor UI controls and older-format conversion remain
separate work.
