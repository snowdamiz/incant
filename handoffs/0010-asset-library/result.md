# Result: handoff 0010, editor asset library and import workflow

**Status:** UI is integrated. Astra has verified native behaviour, and Claude has reviewed
the native pixels from 2026-10-09. Four visual corrections and one interaction change from
that review are committed. Their native re-captures are **open**. Nothing here approves a
phase gate or claims complete feature integration.

**Model and transport (verified by Astra):** Claude Opus 5.5 (`claude-opus-5-5`)
through Claude Code and `claude-agent-acp`. Astra launched this session with
`python3 tools/handoff/main.py run 0010-asset-library --permission-mode bypassPermissions`.
The runner initialized ACP protocol 1, selected the configured Opus 5.5 option
and delivered the packet through `session/prompt`. Claude’s original self-report
misidentified the outer transport. No routing exception or model substitution occurred.

## Native pixel review, 2026-10-09 (revision 2)

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

## What was built

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

## Changed paths

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

## Commands and results

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

## Browser evidence (synthetic, not native)

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

## Limitations

- **Revision 2 fixes are browser-verified only.** Native re-captures are requested as R1 to
  R4 in native-requests.md. Browser captures never prove WebKit behaviour.
- **Not offered, because they do not exist yet:** a file picker, drag and drop, source
  copying, thumbnails, cancellation, or viewport display of models.
- **Conflict detection.** Conflicts are detected by the `asset.conflict` code, with a
  message-text fallback for older hosts. Astra integrated this.
- **Pending state is local.** The UI tracks the import it started. Concurrent imports from
  elsewhere surface only as the bridge's busy error.
- **The dock grows on demand.** Opening a pane grows the dock to 300 px when there is room.
  It does not shrink back on close, and that is deliberate.

## Open questions

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
