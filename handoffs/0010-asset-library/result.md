# Result: handoff 0010, editor asset library and import workflow

**Status, revision 4:** The director rejected the asset UI in the bottom dock. It is
replaced by a dedicated design: assets are a left-column view beside the Hierarchy, and
their details, reimport and import open in the Inspector. Problems, Console and History
keep the bottom dock. Every backend behaviour is unchanged and still goes through the one
`asset.import` command. The browser review over synthetic data is done. Native capture of
the new layout is **open**, with requests N1 to N6 in native-requests.md. Nothing here
approves a phase gate or claims complete feature integration. Astra integrates this into
PR #8.

**Model and transport (verified by Astra):** Claude Opus 5.5 (`claude-opus-5-5`)
through Claude Code and `claude-agent-acp`. Astra launched this session with
`python3 tools/handoff/main.py run 0010-asset-library --permission-mode bypassPermissions`.
The runner initialized ACP protocol 1, selected the configured Opus 5.5 option
and delivered the packet through `session/prompt`. Claude’s original self-report
misidentified the outer transport. No routing exception or model substitution occurred.


## Revision 4: why and what changed

The director wrote: "I dont like this UI at the bottom. Looks messy, unprofessional, bad
spacing. And I dont even think the assets belong in that bottom sections where the compiler
issues are presumably right next to it." I read the private screenshot,
`director-rejected-bottom-dock.png`. It is 3022×1896 and ignored, and its account label is
not quoted here. It shows the faults:

- **Wrong neighbours.** The asset browser sat beside compiler output.
- **Cramped table.** A four-column table was squeezed into a short strip.
- **Duplicate inspector.** A second mini-inspector competed with the real Inspector a few
  centimetres away.
- **Scattered notices.** Notices lived in pane-local places.

The design decision follows the editor's own model: navigation on the left, details on the
right, output at the bottom.

- **Left column: "Hierarchy" and "Assets".**
  - The header is a two-tab switcher. Each tab has a count, the selected one has the accent
    underline, and arrow keys move between them.
  - The Assets view is a calm list grouped by source folder. Long folders keep their last
    segments, such as "…/surfaces/wood/", with the full path in the tooltip.
  - Rows match the hierarchy's 28 px pill: a kind icon, the name, and a quiet
    interpretation tag (Color, Linear, Normal map) for textures.
  - A filter and an Import button sit above the list.
- **Inspector follows the left column.** Hierarchy shows entity components, as before.
  Assets shows the selected asset or the import form.
  - Asset details reuse the entity identity block: kind tile, name and type. Then come
    inspector-style sections for Source (the full path in a recessed mono well, wrapped at
    slashes), Interpretation (segmented control and hint), Reimport (action and one-line
    explanation) and a collapsed Identifiers disclosure (ID and fingerprint).
  - The import form has the same identity block ("Import assets", "From the project
    folder"), the path rows and a pinned action row.
- **The bottom dock is output again.** It holds only Problems, Console and History. The dock
  no longer resizes itself.
- **Notices have fixed homes.**
  - A running or failed import shows one line under the asset toolbar: "Importing 8 files…"
    or "Last import failed" with a Show link. It appears wherever the Inspector is.
  - The Assets tab carries a spinner while an import runs, or a red dot after a failure.
  - The full error, with the engine's exact message, stays at the top of the Inspector view
    it belongs to until it is dismissed or retried.
  - The cooking note in the pinned action row reads "You can keep working. Editing now
    means retrying the import."
- **Moving between views.**
  - Selecting an asset with a click or the arrow keys updates the Inspector without moving
    focus.
  - Enter moves focus to the asset name in the Inspector, and Escape returns to the same
    row.
  - Returning to Hierarchy restores the entity selection and its Inspector.
  - Revealing a problem from the dock switches back to Hierarchy, so the selected entity is
    visible.

Behaviour that was kept and is still tested:

- Batch import as one command.
- Usage sent only for images, and only when it changes something.
- Reimport without typing a path.
- Path validation that mirrors the engine.
- Drafts, pending state and errors retained across view switches.
- The `asset.conflict` code with its text fallback.
- Busy blocking of a second import.
- Paste over a selection.
- Text and menu Undo inside path fields, with project Undo from the list.
- F2 rename in the hierarchy.
- Untrusted names rendered as text.

No thumbnails, file pickers, model placement or cancellation were added.

Changed in revision 4:

- **New:** `src/components/assets/AssetBrowser.tsx`, `src/components/assets/AssetInspector.tsx`
  and `src/assets/library.ts`.
- **Removed:** `AssetsTab.tsx` and `AssetPane.tsx`.
- **Changed:** `HierarchyPanel.tsx` (the left-column switcher), `InspectorPanel.tsx`,
  `BottomDock.tsx` (restored to its pre-0010 form, plus a switch back to Hierarchy when a
  problem is revealed), `App.tsx`, `ShellContext.tsx` and `AssetsContext.tsx` (`view`).
- **Also changed:** `styles/assets.css` (rewritten), `DESIGN.md`, the tests and the capture
  tool.

### Revision 4 commands and results

| Command | Result |
| --- | --- |
| `npx vitest run` in `editor/ui` | 273 tests passed: the integrated 267 plus 6 for placement, Inspector switching, problem reveal, folder grouping and folder labels |
| `npx tsc -b --noEmit` | clean |
| `npm run build --workspace editor/ui` | built |
| `node handoffs/0010-asset-library/tools/capture-assets.mjs revision-4` | 40 captures, 0 page or console errors |

Bundle, against HEAD `93dbfc5`, the pre-0010 baseline:

| Asset | Baseline, Vite gzip | Revision 4, Vite gzip | Delta |
| --- | --- | --- | --- |
| Main JS | 93.57 kB | 101.43 kB | +7.86 kB |
| CSS | 8.38 kB | 9.79 kB | +1.41 kB |

At gzip level 9 the main JS went from 92,450 to 100,293 bytes, or 90.28 to 97.94 KiB. That is
within the 110 KiB budget. No dependencies, fonts or icon packages were added.

### Revision 4 browser evidence (synthetic, not native)

Captures are in `screenshots/revision-4/`, at 1000×650 and 1440×900 and 2× scale. They were
made with Chrome 155 headless through the pinned `playwright-core` on port 4190, using the
same states and evidence doubles as before. `report.json` holds focus, layout, axe and
dispatch data. The revision 1 to 3 captures of the rejected dock design are removed from the
tree and remain in Git history.

- **axe-core:** zero violations in the details, import form, busy, failed, conflict and
  success states.
- **Keyboard:** F6 reaches the asset list row directly. Down moves the selection, Enter
  focuses the asset name in the Inspector, and Escape returns to the same row. Every step
  has a visible 2 px ring.
- **Layout:** no visibly overflowing elements.

| Size | Left column | Inspector |
| --- | --- | --- |
| 1000×650 | 232 px wide | 320×321 px |
| 1440×900 | 245 px wide | 331×534 px |

- **Pixel review:** I reviewed populated, long path, empty, unsaved, busy, failure and
  success states at both sizes. At 1000×650 the Inspector is short, so the import form and
  the lower details sections scroll. The import action row stays pinned. I did not shrink
  the Agent panel automatically.

### Revision 4 limitations

- **No native evidence of the new layout yet.** The browser captures do not prove WebKit
  rendering. Requests N1 to N6 are in native-requests.md.
- **Inspector height at the minimum window.** At about 1000×650 the Inspector is roughly
  320 px tall, so asset sections and the import rows scroll. The import action row stays
  pinned. Resizing the Agent panel gives more room. The UI does not resize it on its own.
- **Narrow left column.** At 232 px, long asset names end in an ellipsis. The full name is in
  the Inspector, and the full path is in the tooltip and the Inspector.
- **Not offered, because they do not exist yet:** a file picker, drag and drop, source
  copying, thumbnails, cancellation, or viewport display of models.
- **View state is local.** The Hierarchy or Assets choice is not saved between sessions.

## Earlier revisions (superseded by revision 4)

The sections below record revisions 1 to 3 of the dock-based design. Their native
behaviour checks remain valid evidence for the backend. Their visual verdicts no longer
apply.

### Native pixel review, 2026-10-09 (revision 2)

Astra captured these with Codex CUA on the rebuilt Tauri/WebKit app. The captures are in the
ignored `artifacts/native-review/` directory. They stay out of Git, and no account label is
quoted here. Behaviour verdicts come from Astra's evidence file,
`docs/spikes/evidence/editor-assets-native-2026-10-09.json`. This section judges pixels only.

Capture dimensions, measured from the files:

| Capture | Pixels | CSS size at 2× | Traffic lights |
| --- | --- | --- | --- |
| a01-unsaved | 1440×900 | 1x capture | covered by the capture indicator |
| a02, a03, a03b, a04, a05, a07 | 2880×1748 | 1440×874 | covered by the capture indicator |
| a06-wide-details | 2880×1748 | 1440×874 | **visible**, correctly placed |
| a04b-conflict-narrow, a06b-narrow-details | 2002×1302 | 1001×651 | covered by the capture indicator |
| account-keyboard-focus | 2880×1748 | 1440×874 | covered by the capture indicator |

The narrow pair is the Window > Move & Resize > Bottom Right placement. In every capture
except a06 the purple capture indicator sits over the traffic lights, so only a06 shows them.
There the lights clear the wisp and project name, and the titlebar is unchanged.

What the pixels show:

- **Fine as built.**
  - The wide list and details layout.
  - Every list row is drawn: 2 rows in a02 to a05 and 7 visible of 10 in a06. The native AX
    first-row-only listing is not a rendering fault.
  - The selected texture segment, which renders in WebKit, so `:has()` works.
  - The disabled segments and read-only paths while busy.
  - Error readability in both the wide and narrow alerts. The engine message wraps in mono
    and nothing is clipped.
  - The success note and recent dots.
  - The inspector Translation, Rotation and Scale rows, and its ellipsised entity ID.
  - The account dialog: Close has a clear 2 px periwinkle ring with keyboard focus, and the
    dialog does not clip.
- **Fixed: busy copy (a03).** The strip said "Other edits wait until it finishes." Edits now
  succeed while cooking, and a concurrent edit makes the import fail with a retry hint. The
  copy is now "You can keep working. Editing now means retrying the import." The pane's
  pending status shows it too, because the strip is hidden behind the pane in a narrow dock.
- **Fixed: narrow tab clipping (a04b, a06b).** At 1001 px wide the History tab's count was
  cut off at the dock edge. My own 1000×650 browser capture showed the same fault. Below
  520 px of dock width the four tabs now drop their icons and tighten their padding, so
  every label and count fits.
- **Fixed: heading focus ring (a06, a06b, a07).** After Enter opened details, the focused
  title stretched across the header, and its ring touched the text. It read as an editable
  text field. The title now fits its text with 6 px padding and a rounded ring.
- **Fixed: scrolled reimport result (a05).** After a reimport the pane kept its scroll
  position, so the success note was half hidden under the header. Reimport now scrolls the
  pane to its result, as import already did.
- **Changed: multi-line paste (observed by Astra).** Select all in a filled path, then paste
  several lines. The old path was kept and the lines were appended below it. Now a wholly
  selected or empty field takes the first line and the rest become new rows. A caret paste
  still keeps the path and adds every line below it. Two interaction tests cover both cases.

### Revision 3: native re-capture review

Every verdict passed, from Astra's private CUA captures of the integrated build, 08026d8.
Per-capture details are in native-requests.md.

| Item | Capture and size | Verdict |
| --- | --- | --- |
| R1 busy copy | `r01-busy-copy-wide` 1440×900 (external display, 1x); `r01b-busy-copy-narrow` 2002×1302 | Pass: new two-line status in the pane, no "Other edits wait" |
| R2 narrow tabs | `r01b`, `r04-heading-focus-narrow` | Pass: every label and the Console and History counts are visible |
| R3 reimport result | `r03-reimport-result` 2002×1302 | Pass: pane at top, full "Reimported." note, Linear applied |
| R4 heading ring | `r04-heading-focus-narrow` 2002×1302; `r04-heading-focus-wide` 3024×1412 | Pass: the ring fits the title and no longer looks like a field |
| R5 paste over selection | Astra behaviour check | Pass for both the select-all and caret cases |

The capture indicator still covers the traffic lights in every r-capture. `a06-wide-details`
remains the only native capture that shows them, and there they are correct. Revision 3
changed documentation only. The tests and browser captures from revision 2 still describe
the code: 267 tests passed and 40 captures were taken. Astra's integration run of 08026d8
also passed the 267 UI tests.

### Revision 1 to 3 design (superseded)

- **Assets tab.** It is the first tab of the bottom dock, ahead of Problems, Console and
  History. It shows a filterable list of the project's assets. Each row has a kind tile,
  name, source path and type, for example "Texture · Normal map". Long paths truncate the
  folder before the file name. Below about 520 px the rows become two lines. The tab shows
  the asset count, a spinner while an import runs, and a "!" mark after a failure that
  happened while another tab was open.
- **Details pane.** Selecting a row opens it. It shows the type and the full source path,
  wrapped at slashes. Textures get a Color, Linear or Normal map choice with a one-line hint.
  Reimport needs no typing. Its label becomes "Reimport as Normal map" when the choice
  differs from the saved one. Models note "Placing models in a scene is not available yet."
  The ID and fingerprint sit in a collapsed "Identifiers" disclosure.
- **Import form.** It lives in the same pane and holds 1 to 64 path rows. It explains paths in
  two sentences. Each row shows the detected kind and, when the path is already in the
  library, which asset it updates. Image rows get the interpretation control. Pasting several
  lines adds one row per line, and Shift+Enter adds a row. Validation mirrors the engine's
  `validate_path` and extension table, with one specific message per problem.
- **Requests sent.** One `asset.import` command per batch. A texture usage is sent only for
  images, and only when it differs from what the engine would keep. That keeps the saved
  setting by omission, and model rows never get a usage.
- **States.**
  - Populated, empty, loading, failed project, and older host without assets.
  - Import unavailable, with the host's own reason, and read-only fixture.
  - Busy: a strip, tab spinner and inline status, with read-only fields and no percentage or
    cancel.
  - Success: a note, recent-row dots and a status announcement.
  - Failure: an inline alert with the exact message, every path kept and "Try again".
  - Conflict: its own title and a retry hint.
- **Layout.** Opening a pane asks the dock to grow to at least 300 px, within the existing
  layout clamp. Below about 640 px of dock width, the pane replaces the list and shows a Back
  arrow. Container queries make this follow splitter changes as well as window size.
- **Persistence.** Import state lives in `src/assets/AssetsContext.tsx`, above the dock. Typed
  paths, a running import and its error survive tab switches and hiding the dock. It holds no
  project data. The list always comes from the bridge snapshot.
- **Shell.** `Shell.dispatch` returns the `BridgeResult` so the form can show the exact
  message inline. `Shell.run` now wraps it, with the same behaviour. Import failures are
  announced as "Import failed: <message>".

Retained behaviour, each covered by tests:

- F2 rename in the hierarchy.
- Text undo in path fields from both the keyboard and the native menu, with project undo from
  the list.
- Account-dialog and shortcut-dialog history blocking.
- File names are rendered only as text.

### Changed paths

- `editor/ui/src/assets/AssetsContext.tsx`, `paths.ts` and `paths.test.ts` are new.
- `editor/ui/src/components/assets/AssetsTab.tsx`, `AssetPane.tsx` and `Assets.test.tsx` are
  new.
- `editor/ui/src/styles/assets.css` is new. The `focus.test.ts` source-mirroring test from
  revision 1 was removed by Astra during integration and is not restored.
- `editor/ui/src/components/BottomDock.tsx`: the Assets tab, busy and failure marks, and a
  height request.
- `editor/ui/src/App.tsx`: `AssetsProvider` and the dock height request.
- `editor/ui/src/shell/ShellContext.tsx`: `dispatch`, the `assets` dock tab and the import
  failure label.
- `editor/ui/src/icons/Icon.tsx`: model, texture, import, reimport and back icons.
- `editor/ui/src/bridge/fixture.ts`: read-only sample assets for the sample, large, empty and
  loading variants. The project-loading and project-error variants mirror the native
  adapter's `assets` and `assetImport` values.
- `editor/ui/src/main.tsx`: imports `assets.css`.
- `editor/ui/DESIGN.md`: the asset library component and its keyboard rules.
- `handoffs/0010-asset-library/`: `result.md`, `native-requests.md`,
  `tools/capture-assets.mjs` and `screenshots/after/`.

No changes were made to `editor/app/**`, `editor/bridge/**` or the contract. The contract
question for Astra is in native-requests.md.

### Commands and results

| Command | Result |
| --- | --- |
| `npm test --workspace editor/ui`, revision 2 | 267 tests passed: the integrated 265 plus 2 paste tests |
| `npx tsc -b --noEmit` in `editor/ui` | clean |
| `npm run build --workspace editor/ui` | built |
| `node handoffs/0010-asset-library/tools/capture-assets.mjs after` | 40 captures, 0 page or console errors |

Bundle sizes. "HEAD" is `93dbfc5`, built from a `git archive` extract.

| Asset | HEAD, Vite gzip | After, Vite gzip | Delta |
| --- | --- | --- | --- |
| Main JS, revision 2 | 93.57 kB | 100.65 kB | +7.08 kB |
| CSS, revision 2 | 8.38 kB | 10.02 kB | +1.64 kB |

Measured with `zlib` at gzip level 9, the main JS went from 92,450 to 99,525 bytes. That is
90.28 to 97.19 KiB, an increase of 6.91 KiB. The JS stays under the 110 KiB budget. No
dependencies, remote fonts or icon packages were added.

### Browser evidence (synthetic, not native)

Captures are in `screenshots/after/` at 1000×650 and 1440×900, at 2× scale. They were made
with Chrome 155 headless through the pinned `playwright-core` 1.64, using `vite preview` on
port 4190. The user's port 4176 was not used. `report.json` records the dispatched commands,
focus, layout and axe results.

- **Shipped read-only fixture:** 01 populated, 02 long-path details, 03 import unavailable,
  04 empty, 05 project loading, 06 project failed, 07 a large list of 240 rows.
- **Capture-only evidence doubles:** these are injected by the script, labelled "Evidence
  double (not engine)" and never shipped.
  - 08 older host without assets, and 09 unsaved project with the host's reason.
  - 10 and 10b import validation and a ready form, 11 and 11b busy, including another tab
    while busy.
  - 12 failed, 13 conflict, 14 success, 15 reimport with a changed interpretation, and
    16 reimport busy.
  - 17 and 17b keyboard focus in the list and on the details heading.
- **The success double** swaps to a second, hand-written snapshot. It cooks nothing and is
  not a mutation model.
- **axe-core 4.14:** zero violations in the details, import form, busy, failed, conflict and
  success states at 1440×900. An earlier label-content-name-mismatch on list rows was fixed.
- **Keyboard:** F6 reaches the Assets tab and Tab reaches the first row. Down moves the
  selection, Enter focuses the details heading, and Escape returns to the same row. Every
  step matches `:focus-visible` with a 2 px periwinkle ring.
- **Layout:** no visibly overflowing elements. At 1000×650 the pane fills the 446 px dock.
  At 1440×900 the list is 542 px and the pane 320 px.

### Limitations

- **Native evidence is CUA captures on macOS only.** Its pixel verdicts are Claude's, and its
  behaviour verdicts are Astra's. No Windows or Linux native capture was requested. The
  synthetic browser captures never prove WebKit behaviour.
- **Not offered, because they do not exist yet:** a file picker, drag and drop, source
  copying, thumbnails, cancellation, or viewport display of models.
- **Conflict detection.** Conflicts are detected by the `asset.conflict` code, with a
  message-text fallback for older hosts. Astra integrated this.
- **Pending state is local.** The UI tracks the import it started. Concurrent imports from
  elsewhere surface only as the bridge's busy error.
- **The dock grows on demand.** Opening a pane grows the dock to 300 px when there is room.
  It does not shrink back on close, and that is deliberate.

### Open questions

1. Resolved: the engine now exposes `asset.conflict`.
2. Should Assets become the default dock tab once native import is verified? Problems stays
   the default for now.
3. The transport concern in the original report is resolved by the ACP runner
   evidence above; no director action is needed.

## Astra integration notes (kept as written)

The two CSS-selector source-mirroring tests were removed during integration;
behavior tests and Claude’s pixel review remain. The historical 266-test count
above is Claude’s original result. Native errors now preserve stable codes,
including `asset.conflict`, and the conflict UI uses the code with a legacy
message fallback. Normal document edits can proceed during import preparation;
a stale import is rejected instead of blocking the user. Native verification
and the final integrated test count are recorded separately.
