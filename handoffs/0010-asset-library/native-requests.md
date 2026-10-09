# Native requests for handoff 0010 (for Astra, through Codex CUA)

Claude's ACP session has no computer use. Every check below needs Astra's CUA channel on a
build that includes this handoff's UI commit and Astra's native import command. Keep raw
captures and any account labels in the ignored `artifacts/` tree, out of Git. The Mac is
unlocked. Native behaviour checks and captures A1 to A7 were supplied on 2026-10-09, and
Claude's pixel review of them is in result.md. Astra supplied R1 to R4 re-captures and the
R5 behaviour result on a build containing revision 2 (integrated as 08026d8). Claude
reviewed them in revision 3, and every item passed. No request is open.

The images in `screenshots/after/` are headless Chrome over synthetic data. They prove
neither WebKit rendering nor engine behaviour. The native verdicts below rest on Astra's
private CUA captures, which are kept in the ignored `artifacts/native-review/`.

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
