# Native requests for handoff 0010 (for Astra, through Codex CUA)

Claude's ACP session has no computer use. Every check below needs Astra's CUA channel on a
build that includes this handoff's UI commit and Astra's native import command. Keep raw
captures and any account labels in the ignored `artifacts/` tree, out of Git. The Mac is
unlocked. Native behaviour checks and captures A1 to A7 were supplied on 2026-10-09, and
Claude's pixel review of them is in result.md. Requests R1 to R4 below cover the
revision 2 visual fixes, which only browser captures have checked so far.

Native verification stays **open** until these captures are supplied and reviewed. The
images in `screenshots/after/` are headless Chrome over synthetic data. They prove neither
WebKit rendering nor engine behaviour.

| ID | Check | Status |
| --- | --- | --- |
| A1 | Unsaved startup project shows the host's reason | Behaviour passed (Astra); pixels reviewed |
| A2 | Real batch import from a saved project | Behaviour passed; pixels reviewed |
| A3 | Busy state while cooking, with navigation responsive | Passed; copy fixed in revision 2, see R1 |
| A4 | Real failure keeps paths and the list | Passed; pixels reviewed |
| A5 | Reimport with a changed texture interpretation, then Undo | Passed; scroll fixed in revision 2, see R3 |
| A6 | WebKit layout: container queries, `:has()` segmented control, narrow dock | Pixels reviewed; tab clipping fixed, see R2 |
| A7 | Keyboard and focus rings in WebKit | Passed; heading ring fixed, see R4 |
| R1 | Busy copy, wide and narrow | Open |
| R2 | Narrow dock tabs show every label and count | Open |
| R3 | Reimport result note scrolled into view | Open |
| R4 | Details heading focus ring fits the title | Open |
| R5 | Select-all, then multi-line paste replaces the path | Open (behaviour) |

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
