# Native editor source-watch review

Claude Opus 5.5 via ACP owns rendered-pixel and UI review. Review the real native
captures supplied in ignored artifacts/native-source-watch-review. Preserve the
approved left Assets/Hierarchy library, main Inspector, neutral palette and output
only dock (Problems/Console/History). The director rejected the old bottom asset
browser as messy; that redesign is already merged. This is a focused review of
source-watch behavior in that approved layout, not another shell redesign.

## Implemented by Astra

Saved projects watch registered sources/dependencies off the native event thread.
Successful changes cook and commit through shared history, and the renderer loads
the new asset version. Source errors retain the last valid geometry and publish
Problems/Console diagnostics. Undo stays until the next external write. Fixing a
source clears the diagnostic and can commit a new version. No watch controls or
new visual components were introduced. Bridge source diagnostics are correctness
work owned by Astra; existing Problems presentation is reused.

## Supplied actual native evidence

Captured through Codex CUA from a release app at ac2b162, 1440x900, scale 1:
- w01-original.jpg: original two imported triangles.
- w02-external-edit.jpg: external same-size binary edit, automatic reimport.
- w03-undo.jpg and w03b-undo-stable.jpg: Undo, stable across subsequent polling.
- w04-redo.jpg: Redo restores changed geometry.
- w05-broken-source.jpg: malformed glTF, source error in Problems, viewport remains
  Attached with previous valid geometry; no new history transaction.
- w06-recovered-new-source.jpg: repaired source and changed vertices, automatic
  recovery, zero Problems, new import transaction.
- w07-shared-history.jpg: four shared history entries, two automatic imports.

Review pixels for actual original/changed/Undo/Redo/recovery differences, retained
geometry during error, readability and spacing of diagnostic/history states, and
any regression to the approved asset/output separation. Account labels are private:
do not quote them or commit native captures. Astra subsequently verified journal
recovery: four transactions, revision 6, automatic imports tagged source-watch.
Astra is building the final integrated cache-scope fix; it changes filesystem
validation only. A final reopened-app capture will be supplied if needed.

## Scope and constraints

Return handoffs/0012-editor-source-watch/result.md with exact model/ACP transport,
inspected capture list, verdict and limits. Record concrete native capture requests
in native-requests.md if required. Do not claim unobserved minimum-size or Retina
review; existing layout has its own prior native review. Do not modify Rust,
contracts, dependencies, camera, lighting or shader appearance. Production PBR,
AA and camera work remain open; monochrome triangles are diagnostic fixtures.

If a focused UI readability fix is necessary, make it in editor/ui, run UI tests
and build, and request precise replacement native evidence. No new controls with
unimplemented behavior. Main JS remains below 110 KiB gzip. For a review-only
result, no need to rerun tests. Never build the native app or share a cargo target.

Computer capture is authorized, but this ACP session has no native CUA. Only
Astra may perform native CUA actions/captures. Do not use screencapture, CGWindow,
AppleScript or synthetic native input. Do not access credentials, sign out,
publish, merge or edit outside this worktree. Commit your review/fixes with
Built-by: claude. Astra integrates, verifies and merges after checks.
