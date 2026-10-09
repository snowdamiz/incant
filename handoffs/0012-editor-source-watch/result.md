# Handoff 0012 result: native editor source-watch review

## Status

- **Minimum-width readability fix: done, pending native confirmation.** Problems rows
  now wrap instead of truncating. The full error, including line and column, is in the
  row at 1000×650 in browser fixture captures. Native confirmation is requested from
  Astra in `native-requests.md`. No native pixels of the fix exist yet, and none are
  claimed.
- **Earlier native review: pass.** All thirteen supplied native captures were reviewed.
  The source-watch states, the bridge path fix and restart recovery are confirmed in
  native pixels. That verdict is unchanged.
- **Tests and build pass.** All UI and bridge tests pass, including two new ones. The
  build passes with main JS at 101.62 KiB gzip.
- No phase gate is approved or claimed by this result.

## Model and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code (Claude Agent SDK) session through ACP on the director's Claude
  subscription.
- No native computer use was available or used. No screencapture, CGWindow capture,
  AppleScript or synthetic native input was used. The app was not built or launched.

## Packet revisions acknowledged

- **Priority: finish minimum-width diagnostic readability, 2026-10-09.** Read first and
  applied. It expands the earlier review-only scope to a focused Problems-row change.
  The separate source path, panels, palette and typography are kept. No new control was
  added. Rust, bridge and contracts were not touched. Pinned dependencies were installed
  with `npm ci` from the workspace lockfile.
- **Final native follow-up, 2026-10-09.** Read first and applied. Astra integrated the
  0b8c405 review and implemented the bridge recommendation. The five new captures were
  reviewed in 57713ad. That pass was review-only: no layout change was made and tests
  were not rerun. The priority above supersedes its "no layout change" limit.
- **Commit identity.** The brief cites 3580aff for the bridge change. In this worktree
  the same change is 99a11dd. Both have the same patch ID. The diagnostic now keeps the
  engine message as is and puts the source into the diagnostic path.
- **Director feedback on the layout.** The rejection of the old bottom asset browser was
  respected. This review only checks that the merged layout held and proposes no
  redesign.
- The earlier optional request for an error raised while History is open is answered by
  w09-final-error-in-history.jpg.

## Problems row wrapping fix

### Problem

In native w11 at 1000×650 the Problems row ended at "invalid glTF:…". The line and
column were hidden. The cause was the shared list style, which keeps every row on one
line and truncates it. Console rows already wrap, but Problems rows did not.

### Change

- **Messages wrap.** The message wraps onto as many lines as it needs. Long unbroken
  tokens break inside the row instead of overflowing.
- **The location stays separate.** It stays in the mono right-hand column while it fits
  beside at least half of the row. A longer path drops onto its own line under the
  message, at full width and in the same subtle mono style. This uses flex wrapping and
  needs no width breakpoint.
- **Paths break at folders.** A break opportunity is added after each "/" in the message
  and the location. The text that assistive technology reads is unchanged.
- **Alignment.** Message and location align on their first baseline. The severity icon
  is centred on the first line. A one-line row keeps the shared 28px height.
- **Behavior kept.** Entity problems are still buttons that reveal the entity. Their
  focus ring now surrounds the whole wrapped row. Project-level problems are still not
  actions.

### Fixture evidence

These are BROWSER FIXTURE captures, not native evidence. The built UI ran in headless
Chrome 155 at device scale 1, fed by a capture-only bridge double. The double's first
diagnostic copies the w11 source error text. The others stress a long nested path, a
96-character unbroken name and an entity warning.

```
npm ci --ignore-scripts --no-audit --no-fund
npm run build --workspace editor/ui
node handoffs/0012-editor-source-watch/tools/capture-problems.mjs before   # unchanged UI
node handoffs/0012-editor-source-watch/tools/capture-problems.mjs after    # with the fix
```

Measured in the browser. A row counts as clipped when its text box scrolls in either
direction.

| Window | Diagnostic | Before | After |
| --- | --- | --- | --- |
| 1000×650 | w11 source error | clipped, 28px row | full text, 48px row |
| 1000×650 | long nested path | message and path clipped | full text, 104px row, path below |
| 1000×650 | unbroken name | message and path clipped | full text, 142px row, path below |
| 1000×650 | entity warning | clipped | full text, 48px row |
| 1440×900 | w11 source error | full text | full text, 28px row, unchanged |
| 1440×900 | long nested path | message and path clipped | full text, 66px row, path below |
| 1440×900 | unbroken name | message and path clipped | full text, 86px row, path below |

Other results, after the fix, at both sizes:

- No horizontal overflow in the Problems list and no text box past its row.
- The axe-core scan of the dock reports no violations.
- Keyboard: Tab reaches the entity row with a visible 2px focus ring. Enter selects the
  entity in the Hierarchy.
- No page errors.

## Earlier review: inspected native captures

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

- `editor/ui/src/components/BottomDock.tsx`: Problems rows get a wrapping text group
  and break opportunities after "/".
- `editor/ui/src/styles/app.css`: the Problems row styles, scoped to Problems only.
- `editor/ui/src/components/ProblemsList.test.tsx`: two new behavior tests.
- `handoffs/0012-editor-source-watch/tools/capture-problems.mjs`: the fixture capture
  tool.
- `handoffs/0012-editor-source-watch/screenshots/before/` and `after/`: fixture PNGs
  and `report.json`.
- `handoffs/0012-editor-source-watch/native-requests.md`: native capture requests.
- `handoffs/0012-editor-source-watch/result.md`: this file.

No Rust, bridge, contract, dependency, camera, lighting or shader change. Console,
History and other lists keep their existing styles.

## Tests and builds

```
npm test --workspace editor/ui
npm run build --workspace editor/ui
```

| Check | Result |
| --- | --- |
| UI and bridge tests | 282 passed in 11 files: the earlier 280 plus 2 new |
| Typecheck and build | passed |
| Main JS | 101.62 KiB gzip, under the 110 KiB budget |
| CSS | 9.89 KiB gzip |

- The new tests check that the full message is the row's own text, that the path stays
  in its own location element, and that break opportunities do not change the text.
  They also check that entity problems remain buttons that reveal the entity.
- jsdom has no layout, so wrapping itself is measured in the fixture captures, not the
  unit tests.
- The native app was not built, per the packet.

## Screenshots

All are browser fixture captures, not native evidence:

- `handoffs/0012-editor-source-watch/screenshots/before/`: the unchanged UI.
- `handoffs/0012-editor-source-watch/screenshots/after/`: the fix. Key files are
  `problems-source-1000x650.png`, `problems-stress-1000x650-dock.png`,
  `problems-stress-1000x650-dock-scrolled.png`, `problems-stress-1000x650-focus.png`
  and `problems-stress-1440x900-dock.png`.
- Each set has a `report.json` with the row measurements, axe results and keyboard
  checks.

Native captures stay private in the ignored `artifacts/native-source-watch-review`.

## Limitations

- The wrapping fix has no native pixels yet. Chrome and the native WebKit view may
  wrap text at slightly different points.
- Wrapping makes long problems taller. In the 650px window the dock shows fewer rows
  at once, and the list scrolls.
- Retina or any scale other than 1 was not observed and is not claimed.
- Minimum size was observed only at 1000×650 for the Problems error and recovered
  states. History and Console at minimum size were not shown.
- The Console entries behind the count changes were not shown in any capture.
- Journal recovery facts, revision 6 and import origins, come from Astra's report. The
  pixels agree with them but cannot prove them.
- The monochrome triangles are diagnostic fixtures. Production PBR, antialiasing and
  camera work remain open.

## Open questions

- Should a project-level source diagnostic reveal the asset in the Assets view when
  clicked, the way entity diagnostics reveal the entity? That is new behavior and would
  need a binding first, so nothing was added.
