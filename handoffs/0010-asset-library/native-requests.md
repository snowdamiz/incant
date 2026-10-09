# Native requests for handoff 0010 (for Astra, through Codex CUA)

Claude's ACP session has no computer use. Every check needs Astra's CUA channel. Keep raw
captures and any account labels in the ignored `artifacts/` tree, out of Git.

## Revision 5 verdicts on the revision 4 layout (Claude, 2026-10-09)

Astra captured these through Codex CUA on build `3a7f7ff`, which is the revision 4 design
(b99c520) plus Astra's nonvisual corrections. Behaviour verdicts are Astra's, from the
`revision_4` section of `docs/spikes/evidence/editor-assets-native-2026-10-09.json`. The
pixel verdicts below are Claude's. The captures stay in the ignored
`artifacts/native-review/`, and no account label is quoted.

| ID | Captures (pixels) | Behaviour (Astra) | Pixels (Claude) |
| --- | --- | --- | --- |
| N1 | `n01-assets-wide` 3024×1898; `n01b-assets-min` 2002×1302 | Pass | **Pass** |
| N2 | `n02-texture-details`, `n02b-model-details` 3024×1898; minimum texture from `n01b` | Pass | **Pass** |
| N3 | `n03-import-form`, `n03b-import-success` 3024×1898; `n03c-import-form-min` 2002×1302 | Pass, real batch | **Pass**, with one polish fix |
| N4 | `n04-busy`, `n04b-failed` 3024×1898; `n04c-failed-min` 2002×1302 | Pass, real 8-image worker and malformed glTF | **Pass** |
| N5 | `n05-keyboard` 3024×1898 | Pass | **Pass** |
| N6 | none; output-only placement is visible in every capture | Placement passes; entity-linked reveal has the automated test only | **Pass** for placement; reveal **not natively verifiable** |

Wide captures are 3024×1898, which is 1512×949 CSS px on the external display at 2×.
Minimum captures are 2002×1302, about 1001×651 CSS px. Traffic lights are visible and
correct in `n04c-failed-min`. In every other n-capture the capture indicator covers them.
The titlebar, project name, layout toggles and account chip are unchanged and unclipped in
all eleven.

- **N1.**
  - "Hierarchy 2" and "Assets 10" sit in the header, with the accent underline on Assets.
  - Folder rows `models/`, `stress/` and `textures/` align with the 8 px row inset. The
    Color and Linear tags right-align in one column, and the recent dots sit after the
    tags.
  - At minimum, nothing in the header or list clips, and the Import button keeps its
    label.
- **N2.**
  - Texture: the identity block shows "normal" and "Texture". Below it come the Source
    well, the Interpretation segmented control (Linear selected, with its hint), Reimport
    with its explanation, and the collapsed Identifiers.
  - Model: "Placing models in a scene is not available yet." appears under Source, with no
    Interpretation section.
  - At minimum (`n01b`), Source and Interpretation fit. Reimport sits below the fold
    because the Agent panel keeps its height. This is the recorded Inspector-height
    limitation, not a defect.
- **N3.**
  - The form shows the identity block, a two-line lede, the path rows, "2 of 64" and the
    formats line. The pinned "Import 2 files" row sits above the Agent header.
  - After the real batch, the green "Imported 2 files…" note shows at the top. The form
    resets, and the list shows the new tag and dots. History goes from 7 to 8.
  - At minimum the pinned row stays visible while the second path scrolls under it, which
    is the intended behaviour.
  - **Polish fixed in revision 5:** on a model row that updates an existing asset, "Model"
    and "Updates “triangle”" ran together. They now read "Model · Updates “triangle”".
    That is verified in the browser capture `screenshots/revision-4/18-model-update-separator-double-1440x900.png`.
    It is a CSS-only change, so no new native capture is requested.
- **N4.**
  - Busy: "Importing 8 files…" appears under the list toolbar, with a spinner in the Assets
    tab. The paths are read-only, the segments are dimmed, and the pinned row shows the
    two-line pending status.
  - Failure: "Last import failed" appears under the toolbar, and the Assets tab has a red
    dot. The Inspector alert shows the engine's exact glTF error in readable mono, the path
    is kept, and "Try again" is pinned.
  - At minimum, the alert and Try again both stay visible. The path rows scroll below them.
- **N5.** The Enter-focused asset name in the Inspector has a fitted periwinkle ring, not a
  field-like box. The selected row keeps its pill.
- **N6.** The dock shows only Problems, Console and History in every capture. Revealing an
  entity-linked problem from the Assets view is covered by the automated interaction test
  only. `editor/bridge/native.ts` does not yet emit entity-linked diagnostics, so no native
  capture exists and none is claimed. This does not block the UI. It needs one capture
  once the host emits such diagnostics.

No other visual defect was found, and no redesign is warranted. No new native capture is
required for revision 5.

## Revision 4: new layout (requests, now answered above)

The director rejected the bottom-dock asset UI. Revision 4 moves assets into the left
column, beside the Hierarchy, and their details and import into the Inspector. The bottom
dock holds only Problems, Console and History again. Backend behaviour is unchanged, so
A1 to A7 and R5 still stand as behaviour evidence. Every pixel verdict below is open until
these captures are reviewed.

Use a build that includes the revision 4 UI commit, and the same disposable saved project
with `models/triangle.gltf`, `textures/normal.png`, the eight `stress/image-*.png` files and
`models/broken.gltf`. Capture the wide size, then the minimum size (Window > Move & Resize >
Bottom Right), then restore it (Return to Previous Size). If possible, keep the capture
indicator off the traffic lights in at least one capture.

| ID | Check | Status |
| --- | --- | --- |
| N1 | Left column switcher and grouped asset list, wide and minimum | Pass |
| N2 | Asset details in the Inspector, texture and model | Pass |
| N3 | Import form in the Inspector, then batch success | Pass |
| N4 | Busy and failure: the list status line, the tab mark and the Inspector error | Pass |
| N5 | Keyboard: tabs, list, Enter to Inspector, Escape back | Pass |
| N6 | Problems reveal and the dock holds only output | Placement passes; native reveal waits for host diagnostics |

- **N1 `n01-assets-wide`, `n01b-assets-min`.** Click "Assets" in the left column header.
  - Expected: "Hierarchy n" and "Assets n" sit in the header, and Assets is underlined.
  - Assets are grouped under folder labels such as `models/`, `stress/` and `textures/`.
    Texture rows show Color, Linear or Normal map on the right.
  - The Inspector shows "No asset selected". The bottom dock shows only Problems, Console
    and History.
  - At the minimum size, no header text or count is clipped. Long names end in an
    ellipsis.
- **N2 `n02-texture-details`, `n02b-model-details`.** Click `normal`, then `triangle`.
  - Expected: the Inspector identity block shows the name and type.
  - It has a Source section with a mono path well, an Interpretation segmented control
    (textures only), a Reimport button with its one-line explanation, and a collapsed
    Identifiers disclosure.
  - The model shows "Placing models in a scene is not available yet." under Source, and no
    Interpretation section.
  - Capture the wide size and the minimum size for the texture.
- **N3 `n03-import-form`, `n03b-import-success`.** Choose Import in the left toolbar, enter
  `models/triangle.gltf` and `textures/normal.png`, and set Normal map.
  - Expected: the Inspector shows "Import assets" and "From the project folder", the path
    rows and a pinned action row reading "Import 2 files".
  - After success, the green note sits at the top of the Inspector. The list rows show
    green dots, and History increments by 1.
- **N4 `n04-busy`, `n04b-failed`.** Import the eight stress PNGs and capture while cooking.
  - Busy: the line "Importing 8 files…" appears under the list toolbar, with a spinner in
    the Assets tab and the two-line pending status in the Inspector's action row.
  - Then import `models/broken.gltf`. Failed: the left status line reads "Last import
    failed". The Assets tab has a red dot. The Inspector alert shows the engine's exact
    message and Try again.
  - Then select an asset. The left status line keeps "Last import failed" with a Show link,
    and Show returns to the import form with the error.
- **N5 `n05-keyboard`.** Click the viewport, then press F6 until focus is on an asset row
  with a ring. Press Down, then Enter.
  - Expected: the asset name in the Inspector has a fitted ring, not a field-like box.
    Escape returns a ringed row.
  - Shift+Tab to the "Assets" tab and press Left. Hierarchy shows the previous entity
    selection, and F2 still renames.
- **N6 `n06-problem-reveal`.** With Assets showing and at least one problem with an
  entity, click it in Problems.
  - Expected: the left column switches to Hierarchy with that entity selected.

## Earlier revisions (dock design, superseded)

The records below refer to the rejected bottom-dock design. Their behaviour results stand.
Their pixel verdicts no longer describe the UI. The revision 1 to 3 browser captures were
removed from `screenshots/` and remain in Git history.

| ID | Check | Status |
| --- | --- | --- |
| A1 | Unsaved startup project shows the host's reason | Behaviour passed (Astra); pixels reviewed |
| A2 | Real batch import from a saved project | Behaviour passed; pixels reviewed |
| A3 | Busy state while cooking, with navigation responsive | Passed; copy fixed in revision 2, see R1 |
| A4 | Real failure keeps paths and the list | Passed; pixels reviewed |
| A5 | Reimport with a changed texture interpretation, then Undo | Passed; scroll fixed in revision 2, see R3 |
| A6 | WebKit layout: container queries, `:has()` segmented control, narrow dock | Pixels reviewed; tab clipping fixed, see R2 |
| A7 | Keyboard and focus rings in WebKit | Passed; heading ring fixed, see R4 |
| R1 | Busy copy, wide and narrow | **Pass** (pixels, revision 3) |
| R2 | Narrow dock tabs show every label and count | **Pass** (pixels, from r01b and r04 narrow) |
| R3 | Reimport result note scrolled into view | **Pass** (pixels; Astra's AX confirms Reimported and the history increment) |
| R4 | Details heading focus ring fits the title | **Pass** (pixels, wide and narrow) |
| R5 | Select-all, then multi-line paste replaces the path | **Pass** (behaviour, Astra) |

## Revision 3 verdicts (Claude, 2026-10-09)

Each verdict below names the capture it rests on. Dimensions are measured from the files.

- **R1 pass, from `r01-busy-copy-wide` and `r01b-busy-copy-narrow`.**
  - `r01-busy-copy-wide` is 1440×900, taken on the external display at 1x. During the
    eight-PNG worker, the strip reads "Importing 8 files…" alone, because the pane is open.
  - The 320 px pane's footer shows "Importing 8 files…" over "You can keep working. Editing
    now means retrying the import." The note wraps to two lines, which is acceptable.
  - `r01b-busy-copy-narrow` is 2002×1302, the minimum laptop placement. The footer shows
    both lines on one line each.
  - "Other edits wait" appears nowhere. The read-only path fields and the disabled segments
    read correctly.
- **R2 pass.** In `r01b`, the narrow tabs read "Assets" with its spinner, "Problems",
  "Console 1" and "History 5". In `r04-heading-focus-narrow` they read "Assets 10",
  "Problems", "Console 1" and "History 6". In `r03` History reads 7, which agrees with the
  reimport increment. Every label and count is fully visible, with no icons and no clipping.
- **R3 pass, from `r03-reimport-result` (2002×1302).** The pane is scrolled to the top. The
  note "Reimported. Undo reverts the whole reimport." sits fully below the header. Linear is
  selected, and the status bar reads "Reimported normal."
- **R4 pass, from `r04-heading-focus-narrow` and `r04-heading-focus-wide`.**
  - `r04-heading-focus-narrow` is 2002×1302. After Enter, the periwinkle ring wraps only
    "normal", with padding. It no longer spans the header or resembles a text field.
  - `r04-heading-focus-wide` is 3024×1412. After Escape to the row and Enter again, it shows
    the same fitted ring next to the Close button. The CUA pointer overlaps part of the
    title in this capture, but the ring is unaffected.
- **R5 pass:** Astra's behaviour result. Select-all, then a multi-line paste replaced Path 1
  and added Path 2. A paste at the end caret kept Path 1 and inserted the rows below it.
  The source paths were synthetic.
- **Traffic lights.** The purple capture indicator still covers the traffic lights in all
  five r-captures. The only native evidence of the lights remains `a06-wide-details`, where
  they are visible and clear the wisp and project name. This is a limit of the capture
  tool, not a UI defect. No further request is made.

## Revision 2 re-captures (Claude, 2026-10-09)

Use a build that includes the revision 2 commit. Place the window as before: use Window >
Move & Resize > Bottom Right for narrow, and Return to Previous Size for wide. If possible,
take one capture where the capture indicator does not cover the traffic lights. Only a06
showed them in the first set.

- **R1 `r01-busy-copy-wide`, `r01b-busy-copy-narrow`.** Start the eight-PNG stress import.
  Wide: the strip reads "Importing 8 files…" and the pane footer reads "Importing 8 files…"
  with "You can keep working. Editing now means retrying the import." beneath it. Narrow:
  the pane footer shows the same two lines. "Other edits wait" must not appear anywhere.
- **R2 `r02-narrow-tabs`.** In the narrow placement with History holding at least one
  entry: the dock tabs have no icons, and "Assets n", "Problems", "Console n" and
  "History n" are all fully visible, including the History count.
- **R3 `r03-reimport-result`.** Scroll the details pane of a texture to the bottom, change
  the interpretation and Reimport. When it finishes, the pane is scrolled to the top and
  the "Reimported." note is fully visible below the header.
- **R4 `r04-heading-focus`, wide and narrow.** Tab to a list row, then press Enter. The ring
  wraps only the asset name, with space around the text. It must not span the header width
  or look like a text field. Escape returns a ringed row.
- **R5, behaviour.** In the import form, type `models/old.glb`, press ⌘A, then paste two
  lines (`models/a.glb` and `textures/b.png`). Expected: Path 1 is `models/a.glb`, Path 2
  is `textures/b.png`, and `models/old.glb` is gone. With the caret at the end and no
  selection, the same paste keeps `models/old.glb` and adds both lines below it.

Use a scratch saved project folder that contains, for example, `models/prop.glb`,
`textures/rock.png` and a deliberately broken `models/broken.gltf` whose `.bin` is missing.

## A1. Unsaved project: `a01-unsaved`

1. Launch the editor on the default unsaved startup project. Open the Assets tab in the dock.
2. Expected: "No assets yet" with the engine's own reason text ("Open a saved project to
   import assets." at the time of writing). The "Import from project folder" button explains
   the same reason. Nothing is dispatched.

## A2. Batch import: `a02-import-success`

1. Open the scratch saved project. Assets tab, then Import.
2. Type `models/prop.glb`, choose Add path, type `textures/rock.png`, select "Normal map".
3. Choose "Import 2 files". Capture after completion.
4. Expected: the list shows both rows with green "just imported" dots, the pane says
   "Imported 2 files. Undo reverts the whole batch.", History shows one "Import assets" entry.
5. Press ⌘Z with focus on a list row. Both assets should disappear in one step. ⇧⌘Z restores
   them with the same IDs (open "Identifiers" in details to compare).

## A3. Busy state: `a03-import-busy`

1. Use a large source, such as a multi-megabyte EXR, so cooking takes long enough to see.
2. Start an import and capture while it runs.
3. Expected: a spinner in the Assets tab, the strip "Importing 1 file… You can keep working",
   read-only path fields, and no percentage or cancel control. Switching to History and
   moving through the hierarchy must stay responsive. Renaming during the import now succeeds. The prepared import must then
   reject its stale revision without reverting the rename. This was verified natively.

## A4. Failure: `a04-import-failed`

1. Import `models/broken.gltf`.
2. Expected: an alert titled "Nothing was imported" with the engine's exact message, the path
   still typed, the asset list unchanged, and the button reading "Try again".
3. If a revision conflict can be provoked, for example with an edit from another client,
   capture `a04b-conflict`. The title should read "The project changed during the import".

## A5. Reimport: `a05-reimport`

1. Select `rock` in the list and choose "Linear". The button should read "Reimport as Linear".
2. Reimport, then check that the Type column shows "Texture · Linear" and that the name and ID
   are unchanged. Undo should restore "Normal map".
3. Select `prop` and choose Reimport. Confirm that no interpretation control is shown.

## A6. Layout in WebKit: `a06-wide`, `a06b-narrow`

1. At about 1440×900, open details. The pane should sit to the right of the list.
2. At about 1000×650, open details. The pane should replace the list and show a Back arrow.
   The dock should have grown to 300 px.
3. Confirm that the Color, Linear and Normal map control shows its selected state. That
   state depends on CSS `:has()`.

## A7. Keyboard: `a07-keyboard`

1. Click in the viewport, press F6 until the Assets tab has a ring, then Tab three times to
   reach the first list row. Press Down, then Enter.
2. Expected: the details heading is focused and ringed. WebKit uses the `data-focus-visible`
   twin. Escape returns to the same row with a ring.
3. In the import path field, ⌘Z must undo typing only, from both the keyboard and the Edit
   menu. History must not change.

## Contract notes for Astra (no contract change made)

- Addressed during integration: native errors preserve stable codes, including
  `asset.conflict`; the UI recognizes that code and keeps a legacy text fallback.
- The UI tracks its own pending import. The bridge exposes no pending flag, so a second window
  or the agent starting an import would be seen only through `engine.busy`. That is acceptable
  for now.
