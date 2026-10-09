# Background editor source watching

Phase 1 increment, 2026-10-09. Saved native projects now start a source watcher
after opening. The worker polls registered source dependencies every 500 ms and
debounces changes for 300 ms. It uses the same content-based watcher and cooked
cache as the CLI. Unrelated files and cache writes do not create document edits.

The shared watcher now has an owned preparation result. The editor snapshots the
document under its lock, releases it for source reads/cooking, then commits with
revision and full-document checks. Only successful commit advances the watcher's
source baselines or consumes diagnostic changes. Out-of-order and foreign watcher
results are rejected. Concurrent user edits invalidate preparation and retry from
a fresh snapshot; they are never overwritten. Manual asset imports take priority.

Successful source changes enter the persistent command history as one import
transaction. Undo restores earlier assets and is not immediately overwritten by
the unchanged source state. A new external write triggers a new import. The
renderer sees the resulting document revision; actual GPU replacement is supplied
by the imported-geometry increment, not duplicated here.

Broken sources retain the last valid asset. The worker publishes source errors to
Problems and Console and clears them on recovery; repeated unchanged errors do
not spam events. Source reads/cooking are outside the native event thread and
document lock. Closing the app discards unfinished preparation before publication.

## Verification in progress

- Existing shared watcher behavior checks still cover content hashing, dependency
  edits, coalesced transactions, source recovery, Undo, journal replay, fairness
  beyond 64 assets, cache repair and symlink rejection.
- New checks cover stale preparation preserving both concurrent edits and pending
  diagnostics; out-of-order/foreign cycles; and the editor coordinator's manual
  import priority, history/Undo behavior, recovery and close handling.
- Bridge checks verify source diagnostics arrive and clear while the document and
  viewport remain ready. Native source-edit/reimport/Undo review and full final
  hosted verification are pending.

Polling hashes registered bytes and can consume IO for large source sets. A single
OS source read or cook cannot yet be interrupted; closure prevents its result from
being published but does not forcibly stop that IO. No responsiveness or memory
budget gate is claimed without measurement. Watch controls, user configuration,
file picking/copying and production asset tiers remain open.
