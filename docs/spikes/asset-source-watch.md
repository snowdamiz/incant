# Phase 1 source watching and CPU reload

2026-10-09. `incant watch-assets PROJECT` now watches registered model/image
sources, reimports changed content and updates the CPU runtime asset store. This
is a usable authoring CLI increment, not completion of engine asset hot reload.

## Behavior

The shared `incant_import::SourceWatcher` polls content hashes of the primary
source and its cooked dependencies. Same-size writes, external glTF buffers,
external images and atomic file replacements are detected without relying on
mtime granularity. First observation loads dependencies from validated cooked
entries; changes made while the command was closed are reconciled as well.
Missing/corrupt initial cache entries are rebuilt from source. Each later scan
reads/hashes known source bytes without decoding unchanged models or images.
Unregistered files and cache writes do not create imports.

Changed observations must remain stable for the debounce duration (300 ms by
default). Up to 64 ready assets cook per poll, with never-attempted assets taking
priority over retries. Successful assets commit together in one command-bus
transaction with import provenance. The service preserves ULIDs, user names and
saved texture interpretation. A failed source produces a diagnostic and leaves
its previous document/cache version usable; it does not block unrelated valid
sources. Repeated identical errors are suppressed. Failed cooks retry at most
every two seconds unless a new observation arrives, allowing a newly created
dependency to repair a previously failed glTF import. Repairs clear diagnostics.

The live watcher retains its source baseline across document Undo, so unchanged
files do not immediately overwrite that Undo. A later source edit can reimport
again. This suppression is session-local: restarting the watcher reconciles the
current sources against the document's cooked version. Old cooked versions remain
available for history; cache garbage collection is still pending.

The CLI holds exclusive journal ownership, checkpoints successful transactions
with atomic file replacement, and reloads `AssetStore` after each poll. Existing
consumers can retain immutable old versions. Runtime load/budget errors preserve
the previous entire store and are reported separately from source diagnostics.
The watcher stops if it detects an external edit to the project file rather than
overwriting that edit. This check is not a filesystem compare-and-swap; external
editors should not modify a checkpoint owned by a running command bus. If a file
conflict appears after a committed import, the transaction remains durable in the
journal while the conflicting file remains untouched.

`watching`, `assets` and `stopped` events are newline-delimited JSON with revisions,
changed imports, diagnostics, CPU asset generations and runtime errors. Ctrl+C
stops the normal unbounded process; committed history is already durable.
`--polls N` supports bounded jobs and returns a failing status if it finishes with
pending changes or errors. A `watching` event acknowledges journal ownership; it
does not claim every asset is loaded.

## Verification

Six shared-service tests use real temporary source/cache/journal files and an
injected monotonic time to verify debounce without sleeps. They cover write
bursts and one-transaction publication, retained runtime versions, Undo and
journal recovery, same-size external-buffer edits, new-dependency repair,
malformed/deleted/restored inputs, independent valid imports, diagnostic clearing,
cache rebuilding without history, changed texture interpretation, unrelated files,
65-source batch progress and outside-symlink rejection (Unix).

Two separate-process CLI tests exercise a running watcher while writing real
files. They verify last-good preservation, repair, increasing runtime generation,
checkpoint persistence, exclusive journal locking, external-file conflict
protection, recovered Undo and unsuccessful bounded runs with unresolved errors.
The tests access no provider credentials and require no display or account.
The full workspace release tests, workspace Clippy with warnings denied,
formatting, generated conventions and generated bridge checks pass locally.

## Limits and remaining work

Polling currently hashes all known source bytes every interval (500 ms default),
with the existing 128 MiB per-asset source bound. Initial dependency loading
decodes each existing cooked entry once. The command has no measured large-project
latency/IO budget yet. Shared dependencies may be read once per parent asset.
OS file notifications, incremental hash caching and cancellation of a blocked
filesystem read remain future optimizations.

The service is synchronous and must run on a worker that owns its command bus,
not while holding the editor's UI state lock. An asynchronous editor dispatch
path, editor/agent import controls, GPU/ECS binding and renderer hot reload remain
open. Source watching is authoring work and is not enabled in shipped games.
FBX, WAV/OGG, animation and block-compression support remain separate Phase 1
requirements. No platform or phase gate is approved by these tests.
