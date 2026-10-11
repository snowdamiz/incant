# Import completion and Undo receipts

The native review of the merged editor (`handoff/0036-editor-final-native`,
evidence commit `a4408180f5d5bf359e859e1afec2d0ef0598e143`) found that an unchanged
reimport said Undo would reverse it. The command bus correctly made no change;
Undo therefore reversed the preceding, unrelated transaction.

The editor now captures the import revision and optional transaction ID while
holding the same command-bus lock as `PreparedImports::commit`. A null transaction
means no commands were appended. This cannot be inferred from the later snapshot:
another edit may advance it between commit and `engine_read`. The bridge carries
the receipt independently of that snapshot. Import preparation, validation and
all actual mutation paths remain unchanged.

An unchanged import reports “Already up to date. Nothing changed.” It does not
mark assets as newly imported or offer an Undo hint. A changed import offers the
hint only while its exact transaction is the current applied history entry. Undo,
redo and later edits update that condition. A bridge without a receipt gets a
neutral completion message without an assumed transaction. The native host and
its injected bridge are updated together; this does not claim wire compatibility
with an older native host.

## Verification on main61 plus this fix

- Three real native asset-host tests pass. The added persistent-journal regression
  verifies unchanged project, revision, journal bytes, history and redo tail with
  and without an unrelated edit. A changed source still appends its actual
  transaction, undoes/redoes and recovers from the journal. The test closes the
  journal before reading bytes, respecting Windows exclusive handles.
- The bridge regression preserves a no-op receipt when the later read contains a
  concurrent edit. UI cases cover import/reimport no-op feedback, missing receipts
  and exact history cursor changes across undo, redo and a later edit.
- All 529 UI/bridge tests in 30 files, strict TypeScript, production UI build,
  generated bridge check and scoped release editor Clippy pass. This is the
  main61 test graph with five added cases, not the newer runtime-union test count.
- The first full UI run exposed a test query matching both the notice and live
  status bar. The final test checks both locations explicitly; no assertion or
  production coverage was removed.

Exact source/log hashes are in
[`editor-import-receipts-2026-10-11.json`](evidence/editor-import-receipts-2026-10-11.json).
Root independently reviewed the receipt/history semantics. Claude's focused
copy and rendered-state review is tracked in handoff0038; it is pending at this
source freeze. No native visual acceptance or hosted check result is claimed for
this follow-up yet. Other native-review findings and reference-device gates remain
separate.
