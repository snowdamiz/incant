# Handoff 0012 result: native editor source-watch review

## Status

- **Final verdict: pass.** All thirteen supplied native captures were reviewed. The
  first eight came from ac2b162 and the final five from the rebuilt release app.
- **The bridge fix is confirmed in native pixels.** The source error now shows the
  engine message once, with the asset path in the separate right-hand column. This was
  checked at 1440×900 and at the measured 1000×650 minimum.
- **Restart is confirmed.** The reopened app shows the same recovered geometry and all
  four history entries, with no redundant import.
- **No UI fix was made.** Nothing in `editor/ui` changed, so UI tests and the build were
  not rerun, as the packet allows for a review-only result.
- **No further native evidence is required.** `native-requests.md` was not created.
- One non-blocking recommendation remains: at the minimum width the Problems row
  truncates the useful end of the message. The final packet forbids layout changes, so
  it is recorded under open questions and not fixed.
- No phase gate is approved or claimed by this result.

## Model and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code (Claude Agent SDK) session through ACP on the director's Claude
  subscription.
- No native computer use was available or used. No screencapture, CGWindow capture,
  AppleScript or synthetic native input was used. The app was not built or launched.

## Packet revisions acknowledged

- **Final native follow-up, 2026-10-09.** Read first and applied. Astra integrated the
  0b8c405 review and implemented the bridge recommendation. The five new captures were
  reviewed, and this file was updated. No look-dev or layout change was made, and tests
  were not rerun.
- **Commit identity.** The brief cites 3580aff for the bridge change. In this worktree
  the same change is 99a11dd. Both have the same patch ID. The diagnostic now keeps the
  engine message as is and puts the source into the diagnostic path.
- **Director feedback on the layout.** The rejection of the old bottom asset browser was
  respected. This review only checks that the merged layout held and proposes no
  redesign.
- The earlier optional request for an error raised while History is open is answered by
  w09-final-error-in-history.jpg.

## Inspected captures

All captures are in `artifacts/native-source-watch-review`, which is git-ignored. None
were committed or copied into the repository. The title bar shows a private account
label in every capture. It is not quoted here.

First set, release app at ac2b162, 1440×900, scale 1:

| Capture | Observed |
| --- | --- |
| w01-original.jpg | Two triangles, Problems empty, Console 1, History 2, Attached. |
| w02-external-edit.jpg | Top vertex of both triangles moved right. Console 2, History 3. Problems empty. |
| w03-undo.jpg | Geometry back to the original. Redo enabled. History still 3. |
| w03b-undo-stable.jpg | Pixel-identical viewport to w03-undo after further polling. |
| w04-redo.jpg | Geometry matches the external edit again. Redo now disabled. |
| w05-broken-source.jpg | Geometry unchanged from Redo. One error in Problems and the status bar. History stays 3, viewport still Attached. |
| w06-recovered-new-source.jpg | New third shape: top vertices moved left. Problems back to 0, Console 5, History 4. |
| w07-shared-history.jpg | History tab: 4 of 4 applied, two "Reimport changed sources" entries tagged source-watch. |

Final set, rebuilt release app with the bridge fix:

| Capture | Size | Observed |
| --- | --- | --- |
| w08-final-reopened.jpg | 1440×900 | Reopened project, Attached, 0 errors, History 4, Console 1. Undo enabled, Redo disabled. Geometry equals w06. |
| w09-final-error-in-history.jpg | 1440×900 | History tab open. Problems tab shows a red 1 and the status bar shows 1 error. History still 4 of 4 with the same timestamps. |
| w10-final-source-path.jpg | 1440×900 | Problems row shows the message once and the asset path in the mono right column. |
| w11-final-source-path-minimum.jpg | 1000×650 | Same error at minimum size. Message truncates with an ellipsis. Path column stays fully visible. |
| w12-final-recovered-minimum.jpg | 1000×650 | No problems, Attached, History 4, Console 3. Geometry equals w11. |

Capture sizes were confirmed from the files with `sips`.

## Viewport pixel comparison

The JPEGs were converted to BMP with the system `sips` tool and compared over the
viewport interior. A pixel counts as changed when its grey value differs by more than
64, which ignores JPEG noise. Scripts lived in `/tmp` and are not committed.

```
sips -s format bmp <capture>.jpg --out /tmp/sw-review-bmp/<capture>.bmp
python3 -I /tmp/sw-review-scripts/diff.py /tmp/sw-review-bmp
python3 -I /tmp/sw-review-scripts/diff2.py /tmp/sw-review-bmp
```

At 1440×900 the region was x 250 to 1105 and y 80 to 625:

| Comparison | Changed pixels | Meaning |
| --- | --- | --- |
| original vs external edit | 5367 | Reimport changed the rendered geometry. |
| original vs Undo | 0 | Undo restored the original exactly. |
| Undo vs Undo after polling | 0 | Polling did not reapply the external edit. |
| external edit vs Redo | 0 | Redo restored the changed geometry exactly. |
| Redo vs broken source | 0 | The last valid geometry was kept during the error. |
| broken source vs recovered | 8011 | The repaired source produced new geometry. |
| original vs recovered | 5042 | The recovered geometry is a third, distinct version. |
| recovered vs history view | 0 | Opening History did not disturb the viewport. |
| recovered vs reopened app | 0 | Restart restored the last committed geometry. |
| reopened vs new error in History | 0 | The new error kept the last valid geometry. |
| reopened vs corrected path row | 0 | Same retained geometry during the error. |

At 1000×650 the region was x 250 to 665 and y 80 to 375:

| Comparison | Changed pixels | Meaning |
| --- | --- | --- |
| minimum error vs minimum recovered | 0 | The repaired source matches the prior valid version, so no new geometry appears. |

## Review findings

- **Layout separation held.** The left Assets/Hierarchy library, the main Inspector and
  the output-only dock are unchanged in all thirteen captures, including at 1000×650. No
  asset browser returned to the dock and no watch controls were added.
- **The diagnostic binding fix reads correctly.** The file name no longer appears as a
  prefix. The asset path sits in the mono meta column where "Project" appeared before,
  aligned with the History meta column style. The import crate's own message still
  names the file once, which is acceptable verbatim engine text.
- **An error is noticeable from another tab.** With History open, the Problems tab gets
  a red count badge and the status bar error count turns to 1. History is not changed
  by the failed cook.
- **Restart is clean.** After reopening, the two source-watch imports keep their original
  timestamps and actor, and no extra import entry appears. Console starts fresh at 1.
- **Recovery at minimum size is clean.** The red badge clears, the "No problems" state
  returns with readable wrapped copy, and History stays at four entries because the
  repaired source matched the prior version.
- **Undo and Redo toolbar state is correct throughout.** Redo is enabled only after
  Undo. After restart Undo is available and Redo is disabled.

## Resolved: doubled file name in the source diagnostic

The 0b8c405 review reported that the bridge prefixed the source to a message that
already named it, and left the diagnostic path empty. Astra's fix maps the source into
the path and keeps the message as is. w10 and w11 confirm the corrected row in native
pixels. The matching Console line built in the native app was not shown in any capture
and was not reviewed.

## Changed paths

- `handoffs/0012-editor-source-watch/result.md`: this file.

No Rust, contract, dependency, camera, lighting, shader, CSS or component change.

## Tests and builds

- UI tests and the UI build were not run by Claude. No UI file changed and the packet
  waives them for review-only docs.
- Astra reports all 280 UI and bridge tests and the UI build passing, with 101.59 KiB
  gzip main JS. Claude did not rerun or independently verify these.
- The native app was not built, per the packet.

## Screenshots

- No screenshots were produced by Claude.
- Reviewed native captures stay in the ignored `artifacts/native-source-watch-review`.

## Limitations

- Retina or any scale other than 1 was not observed and is not claimed.
- Minimum size was observed only at 1000×650 for the Problems error and recovered
  states. History and Console at minimum size were not shown.
- The Console entries behind the count changes were not shown in any capture.
- Journal recovery facts, revision 6 and import origins, come from Astra's report. The
  pixels agree with them but cannot prove them.
- The monochrome triangles are diagnostic fixtures. Production PBR, antialiasing and
  camera work remain open.

## Open questions

- **Truncated detail at minimum width.** At 1000×650 the Problems row ends at
  "invalid glTF:…", which hides the line and column. The full text is still in the DOM
  for assistive technology and appears wrapped in Console, but the row has no tooltip or
  wrap. A later handoff could let Problems messages wrap the way Console rows already
  do. It was not changed here because the final packet forbids layout changes.
- Should a project-level source diagnostic reveal the asset in the Assets view when
  clicked, the way entity diagnostics reveal the entity? That is new behavior and would
  need a binding first, so nothing was added.
