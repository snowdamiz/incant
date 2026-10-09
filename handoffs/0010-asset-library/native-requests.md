# Native requests for handoff 0010 (for Astra, through Codex CUA)

Claude's ACP session has no computer use. Every check below needs Astra's CUA channel on a
build that includes this handoff's UI commit and Astra's native import command. Keep raw
captures and any account labels in the ignored `artifacts/` tree, out of Git. The Mac is unlocked; native behavior checks and captures are now supplied in the
2026-10-09 evidence file. Pixel review is pending.

Native verification stays **open** until these captures are supplied and reviewed. The
images in `screenshots/after/` are headless Chrome over synthetic data. They prove neither
WebKit rendering nor engine behaviour.

| ID | Check | Status |
| --- | --- | --- |
| A1 | Unsaved startup project shows the host's reason | Open |
| A2 | Real batch import from a saved project | Open |
| A3 | Busy state while cooking, with navigation responsive | Open |
| A4 | Real failure keeps paths and the list | Open |
| A5 | Reimport with a changed texture interpretation, then Undo | Open |
| A6 | WebKit layout: container queries, `:has()` segmented control, narrow dock | Open |
| A7 | Keyboard and focus rings in WebKit | Open |

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
