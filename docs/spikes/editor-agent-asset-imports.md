# Shared editor and agent asset imports

2026-10-09. The native editor and headless agent now use the same import service
as the authoring CLI. Claude handoff 0010 supplies the integrated asset library
and import/reimport interface. Native behavior checks pass; Claude is reviewing
actual WebKit captures and finishing visual corrections.

## Native editor backend

`engine_import` captures the current document/revision under a short lock, cooks
the entire batch on Tauri's blocking worker pool, then reacquires the bus only
for revision-checked commit. Reads and native input do not wait on source parsing.
Concurrent edits make the prepared import fail instead of overwriting those edits.
The bridge permits normal edits and history operations while cooking; only a
second simultaneous import is rejected. A stale import can be retried explicitly.
One native import can run at a time; the guard releases on errors. A closed editor
rejects a completed preparation before commit. Default unsaved projects explicitly
report that a saved project must be opened before importing local sources.

The bridge exposes actual asset IDs, names, kinds, source paths, fingerprints and
texture interpretation, using the Rust-derived asset type. It maps loading,
failure and absent capability separately. Import dispatch includes the last
observed revision; failures retain the last asset list, and success publishes
the real command history and metadata. No GPU instance is invented for an asset.

Two Rust tests verify preparation outside the bus lock, rejected stale commits,
durable history/recovery/Undo, import serialization and unavailable loading state.
Three bridge tests verify request serialization, revision dispatch, real asset
mapping, unavailable hosts and preserved state after import failure. The complete
pre-handoff UI/bridge suite has 243 passing tests, including edits during cooking;
native Rust tests and Clippy pass.

## Agent tools

`asset_list` provides a bounded, stable-ID cursor over registered assets with a
name/path/kind filter. `asset_inspect` returns one asset's metadata. Neither reads
arbitrary source bytes. `asset_import` uses the shared strict `ImportRequest`
schema and an expected document revision to cook 1–64 local sources, then applies
one reversible transaction with the actual agent model and conversation ID.
Unchanged imports do not add history or return an older transaction ID.

The trusted host grants a `ProjectFiles` capability bound to a project ID and a
canonical project root. Tool arguments cannot select another root or cache path.
`ProjectHost` adds this capability to the existing viewport host, which retains
its screenshot behavior. `EngineHost` replaces the viewport-only trait name;
`Perception` remains a compatibility alias. Hosts without file access do not
advertise `asset_import` and reject direct attempts to call it. Registered-asset
listing and inspection are available without file access.

Destructive/always approval modes ask before importing, just as for document
patches. Cancellation is checked before cooking and again before commit. Parsing
itself is not interruptible. Errors preserve authored state; reusable cooked
files may remain after a failed preparation or commit. Source path traversal,
outside source symlinks, network URLs, stale revisions, wrong-project capability
use, malformed batches and extra arguments are rejected. Default cache directory
components cannot redirect writes through symlinks; each directory is checked
before descending, including canonical confinement. CLI's explicitly selected
`--cache` override remains available to its trusted caller and is not an agent
argument. As with source resolution, this is not a guarantee against a separate
malicious local process racing filesystem checks.

Three agent behavior tests verify real cooking, atomic batches, no-op imports,
provenance, durable Undo/Redo, pagination/inspection, denied/cancelled operations,
capability advertising, stale/wrong-project access and source confinement. An
additional shared importer test checks redirects at all three cache directory
levels on Unix and confirms no files appear in the external target. These tests
use synthetic providers and do not count as live model evidence.

## Live verification

One real `gpt-6-astra` run used the already saved OAuth session without signing in
again or changing credentials. In a disposable project it called `doc_query`,
`asset_import` and `asset_inspect`, with all three succeeding. The import produced
one agent transaction. A separate process recovered the journal, undid the import,
redid it and recovered the identical project. After deleting both original glTF
and buffer files, the headless runner loaded 156 bytes of decoded cooked geometry
and ran two simulation ticks. The test driver preapproved the single import within
the authorized disposable project. This does not claim native UI approval or GPU
asset rendering. The run used 5,244 input and 210 output tokens over four steps.
[Exact evidence](evidence/agent-asset-import-2026-10-09.json).

## Integrated native verification

The combined UI/bridge suite passes 265 tests, the production UI builds, and
`tools/editor-dev.py` builds the native Tauri bundle. Actual macOS CUA checks on a
disposable saved project verify the unavailable unsaved-project reason, a real
model/normal-map batch, one-step Undo/Redo, Linear reimport with preserved texture
ID, and Undo restoring the saved Normal map and fingerprint. A malformed glTF
preserves both the asset list and typed source path. Model details expose no
texture interpretation controls.

An eight-image 2048×2048 batch exercises the actual background worker. Native
hierarchy rename and History navigation stay responsive while cooking. After a
concurrent rename commits, the stale import fails with `asset.conflict` and keeps
all paths for retry. Native import failures now retain typed codes through the
bridge, avoiding reliance on message wording. Keyboard and native Edit-menu Undo
in a path field leave project history unchanged. Detail focus and Escape return
are observed through accessibility; ring appearance belongs to Claude's review.
The saved account restores without a new login or Keychain prompt.
[Sanitized native evidence](evidence/editor-assets-native-2026-10-09.json).

Raw captures remain in ignored artifacts because the account label can appear.
Claude is reviewing wide and narrow native layouts, controls, focus rings and
chrome; Astra does not self-approve pixels. Busy-state copy still requires a
correction to reflect concurrent edits. Browser fixtures remain synthetic evidence.

File picking/copying, automatic editor source watching, GPU/ECS binding and the
rest of Phase 1 remain open.
