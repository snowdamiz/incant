## Final priority: native wrapping captures are ready

Astra integrated d497e2c as 3cb5c8b. All 282 integrated UI/bridge tests and the
UI/native custom-protocol release builds passed. Actual CUA captures now supplied:
- w13-wrap-error-minimum.jpg: measured 1000x650, scale 1, actual malformed-source
  Problems row from the final wrapping build.
- w14-wrap-error-wide.jpg: measured 1440x900, same row/build.
- w15-wrap-recovered-minimum.jpg: measured 1000x650 after restoring exact source
  bytes; Attached, zero errors, no extra history transaction.

Finish rendered-pixel review and update result.md/native-requests.md. The optional
entity-focus native check was not performed because this fixture has no entity
error. Existing browser behavior/focus evidence remains explicitly fixture-only.
Do not rerun tests for review-only changes. Commit review with Built-by: claude.

## Priority: finish minimum-width diagnostic readability now

Your final review 57713ad is integrated. The director previously asked you to keep
improving the UI and fixing small issues. Please fix the minimum-size Problems
message truncation you found in w11 now. This explicitly expands the earlier
review-only scope: a focused Problems-row layout/style change is permitted. Keep
the separate source path and existing overall panels, palette and typography.

The full precise error, including line/column, must be available directly in the
Problems row at 1000x650. Prefer sensible wrapping/alignment over hiding essential
detail or adding a new control. Account for long unbroken paths/messages without
horizontal overflow. Preserve entity diagnostic reveal behavior, accessibility,
keyboard behavior and the separate meta path. This is Claude-owned visual work.

Use the existing pinned UI tooling; install pinned node dependencies here if
needed. Run UI tests/build for the changes. Main JS remains below 110 KiB gzip.
Do not modify Rust/bridge/contracts or fabricate native evidence. Browser fixture
review is allowed and must be labeled as fixture evidence. Commit the focused fix
with Built-by: claude. Astra will rebuild and capture the actual native error at
1000x650 and 1440x900 for your final review; request exact additional evidence.
Keep native captures private. Do not defer this directly observed readability
issue merely because the earlier packet was review-only.

## Final native follow-up: diagnostic binding and restart, 2026-10-09

Your 0b8c405 review is integrated. Astra implemented your bridge recommendation
in 3580aff: issue.message is preserved, issue.source becomes diagnostic.path.
All 280 UI/bridge tests and the UI build pass (101.59 KiB gzip main JS). Native
release was rebuilt with custom-protocol. Additional actual CUA captures now live
alongside the first eight in artifacts/native-source-watch-review:

- w08-final-reopened.jpg: final build reopened the same saved project, Attached,
  no errors, all four recovered history entries, no redundant import; saved account
  restored. Journal recovery independently confirms revision 6 and import origins.
- w09-final-error-in-history.jpg: new malformed source while History is open;
  Problems badge and status count show the error without changing history.
- w10-final-source-path.jpg: wide corrected Problems row, separate asset path.
- w11-final-source-path-minimum.jpg: measured 1000x650, scale 1, corrected error.
- w12-final-recovered-minimum.jpg: same minimum size, repaired source identical to
  the prior valid version; no error, Attached, history remains four entries.

Review these final pixels and update result.md. No new look-dev or layout change.
Do not rerun tests for review-only docs. Keep captures private and account labels
out of the result. Commit only your review with Built-by: claude.

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
