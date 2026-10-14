# 0077 — Host-bound project Save and engine-pin review: result

Status: **implemented, verified in tests and in the local browser fixture; awaiting root
review.** Nothing is committed, pushed or merged. This is editor UI work only: no phase gate
is claimed. The browser evidence uses a scripted host fixture. It does not show native file
persistence or native WebKit behaviour.

- Model: Claude Opus 5.5 (`claude-opus-5-5`). Thinking effort: **max**, as the packet requires
  and the ACP launcher sets. This session cannot read the launcher's settings back itself.
- Brief applied: `handoffs/0077-project-file-ui/brief.md`, SHA-256
  `e37d12740d232104f9181ddb2cb58d9bfd96108c21fecd5a463e092e06682dc8`. This includes root's
  priority revision (correctness review). The brief and `quiet.json` (root's files) are left
  uncommitted and unchanged; `quiet.json` read `hold: false` before every frontend and
  browser run.
- Base: worktree HEAD `8feae09b`, on main `78d41fdb`.
- Machine: Mac17,9, macOS 27.0.1 (26A434). Node 22.23.3, Vitest 5.0.3, TypeScript 5.9.3,
  Vite 8.3.4, axe-core 4.14.0, playwright-core 1.64.0, Chrome 155.0.8059.40 headless. The
  parent dependency runtime was used read-only: `<checkout>/node_modules`, whose
  `package-lock.json` SHA-256 `cbea068b…7c60` matches this worktree's. No dependency was
  installed or changed.

## Root's priority revision: fixes and outcomes

1. **Answers are bound to the bridge and project that asked**
   (`src/projectFile/ProjectFileContext.tsx`).
   - **Scope.** A `Scope` (bridge, project) is a new object whenever the bridge is replaced
     or a *different* project is read. A project that is only reloading or failed does not
     end it. It is updated synchronously from `bridge.subscribe` on every publish. The
     bridge publishes before it resolves the request that publish answers.
   - **Each request.** Save, review and apply each record a `Run {operation, scope}`. After
     the `await`, `belongs(run)` drops an answer from an earlier scope: no announcement, no
     receipt, no notice, no dialog state.
   - **A scope change** also releases the request slot (the newly open project has no
     request of its own; a late `end(run)` cannot clear a newer one) and clears notices.
   - **The pin dialog** is bound to the scope and project it was opened for. If either
     changes, it moves to an explicit *retired* state, for example: "**Harbor is no longer
     the open project.** Lighthouse is open now. Whether Harbor was pinned is not shown
     here. Review again before pinning." Pin is removed; Review again asks the open
     project's host for a new review. The review summary and the "Pinned." sentence use the
     name captured when the review was requested, never the current project's name.
   - **Backend authority and revision enforcement are unchanged.** On the same native
     bridge, a file answer whose snapshot belongs to another project is still refused by
     the bridge itself ("Invalid project file response."). The UI drops that refusal too.
2. **A rejected `bridge.request` becomes a failure**
   (`src/shell/requests.ts`, `src/shell/ShellContext.tsx`).
   - `Shell.request` always resolves. A rejection becomes `{ ok: false, error }`, keeping
     the host's `{code, message}` when the rejection carries them. Otherwise the code is
     the editor's own `request.rejected`, with the rejection's text (or "The request ended
     without a result.").
   - The project-file UI titles a `request.rejected` failure, or a success without its
     typed receipt, as an **unknown** result: "Save result unknown.", "No review was
     returned.", "Engine pin result unknown." A host-coded failure is titled as refused.
   - Requesting, applying and busy states always end (`finally` with the scope-safe
     `end(run)`). Nothing claims success from a rejection.
3. **Delayed-response and rejection tests**, all passing (`ProjectFile.test.tsx`,
   `ShellContext.test.tsx`).
   - **Save, another project opened (same bridge).** The title shows Lighthouse with no
     spinner; Harbor's late answer produces no status message and no notice; Lighthouse's
     own Save then sends `{operation:'save', expected_revision:7}`.
   - **Save, bridge replaced.** Harbor's host answers with a *valid* receipt; nothing is
     announced in Lighthouse. A late refusal after the reverse switch shows no notice
     either.
   - **Pin in flight, another project opened.** The dialog shows the retired text and no
     Pin. The late answer shows no "Pinned" and nothing in the status line or notices.
     Review again sends `review_pin` at revision 7. Pin then sends only the *new* review ID.
   - **Bridge replaced with a review ready.** The old review offers no Pin. Reconnecting to
     the same project is said as such ("The editor reconnected to Harbor after this review
     was requested."). No `apply_pin` reached any of the three hosts.
   - **Pin answered after its dialog closed** (same bridge and replaced bridge): no notice
     and no announcement in the open project.
   - **Rejections.**
     - Save rejected with an `Error` gives "Save result unknown. IPC channel closed" and
       `request.rejected`; the spinner ends.
     - A rejection carrying a host code gives "Save failed. The editor process exited."
       and `host.closed`.
     - A rejected review gives "No review was returned." and leaves the pending state.
     - A rejected pin gives "Engine pin result unknown.", removes Pin, and frees the slot
       for the next save.
     - `Shell.request` keeps or assigns codes as above, and announces non-quiet failures.
   - **Mutation checks** (`tools/mutations.py`, `evidence/mutations.json`): each removes
     one guard, reruns both files and restores the source byte-exactly (SHA-256 checked).
     | Guard removed | Tests that fail |
     | --- | --- |
     | Save scope guard | 2 |
     | Apply scope guard | 1 |
     | Slot release on scope change | 3 |
     | `Shell.request` catch | 3 |
     | Stale latch | 1 |
     | Review scope guard | 0 |
     The review guard is not observable through the UI: the dialog's own scope binding
     hides a late review either way. It is kept as defence in depth. The first mutation
     run also left the stale latch undetected, because my test reached "the same History"
     through a lower revision that the bridge ignores. The test now uses an Undo then a
     Redo (revisions 4 and 5) and catches it (`evidence/runs/mutations-run1.json.gz`).

## What the user sees

- **Entry point.** Once a project is open, its name in the titlebar (still the page's `h1`)
  is also the project file menu's button: a quiet button with a chevron, filled while open
  like the Play target picker. The menu is the existing `ContextMenu` with two items:
  - **Save** (`⌘S` on macOS, `Ctrl+S` elsewhere, written in the platform's notation);
  - **Pin engine version…**.

  Items the host does not offer stay listed, marked unavailable, and say why when chosen.
  Examples: "Project file access is unavailable: the sample fixture is read-only.", "…in
  this editor build (project.file).", "Wait for the project to open.". While a file
  request runs, a small spinner replaces the chevron and the button's name adds
  ", saving".
- **Save.** The menu and the shortcut send the one existing `project.save` request. The
  receipt is reported as returned: "Saved revision 7 to the project file." or "Nothing to
  save: the project file already matches revision 7." There is no dirty indicator: the
  interface has no unsaved-state readback.
  - A failure stays in the existing error strip under the titlebar until dismissed, a
    later save succeeds, or another project opens. It shows the host's words verbatim,
    then the code, then Dismiss.
  - The shortcut works from anywhere outside a dialog, text fields included. Inside a
    dialog the keys are taken (no browser Save Page) and do nothing; held-key repeats
    do nothing.
  - Game input is preserved. On macOS, ⌘S stays the editor's in the game view, as other
    ⌘ combinations do. On Windows and Linux, Ctrl+S goes to the game while the game view
    takes keys (the existing `isEditorChord` rule is unchanged).
- **Pin.** A dedicated 480 px dialog. While preparing: "Preparing the review…". With the
  host's review: "**Harbor** will require engine `=0.1.0`." and "Reviewed at revision 7.
  The project file changes only when you pin."
  - **Review details** (on request): the command (`set_engine_version`), review ID,
    current and pinned SHA-256, and the pinned file as plain text. The text is bounded:
    64 K characters until "Show all 1.3 MB". Hidden direction characters are shown as
    escapes; nothing is HTML.
  - **Focus.** Cancel has focus. Pin is the accent and never takes focus by itself. Pin
    sends `project.pin.apply` with only `reviewId`.
  - **Outcomes.** "Pinned. Harbor now requires engine =0.1.0 (revision 8)." A refusal
    shows the host's words and code. A stale, retired or refused review offers "Review
    again" instead of Pin, and says why. Nothing is retried or applied by itself.
  - **Close.** There is no close glyph: Cancel or Close is always in the footer, so the
    two never share a name.
- **Shortcut reference.** The Keyboard shortcuts dialog lists "Save the project" (⌘S /
  Ctrl+S).

## Screenshots

All in `handoffs/0077-project-file-ui/screenshots/`, from the final build (capture set r4).
"Desktop" is 1440×900 and "narrow" is 1024×680. Titlebar and dialog crops are at 2× device
pixels; `*-window.png` images are at CSS scale. Every page is labelled "Sample data", with
the status line reading "Project file fixture: scripted host IPC, not live". No account is
signed in.

| State | Files |
| --- | --- |
| Menu, pointer / keyboard focus | `menu-desktop-titlebar.png`, `menu-narrow-titlebar.png`, `menu-keyboard-desktop-titlebar.png` |
| Menu, host without file access | `menu-unavailable-desktop-titlebar.png` |
| Long project name, narrow | `menu-long-name-narrow-titlebar.png` |
| Saving (busy) | `saving-desktop-titlebar.png`, `saving-narrow-titlebar.png` |
| Saved / nothing to save | `saved-desktop-window.png`, `saved-narrow-window.png`, `nothing-to-save-desktop-statusbar.png` |
| Save failed (host refusal) | `save-failed-desktop-window.png`, `save-failed-narrow-window.png` |
| Review requested / shown / details | `review-requesting-desktop-dialog.png`, `review-desktop-dialog.png`, `review-desktop-window.png`, `review-narrow-dialog.png`, `review-narrow-window.png`, `review-details-desktop-dialog.png`, `review-details-narrow-dialog.png` |
| Review refused (unsaved document, already pinned) | `review-refused-unsaved-desktop-dialog.png`, `review-refused-unsaved-narrow-dialog.png`, `review-refused-pinned-desktop-dialog.png` |
| Pinning / pinned / pin refused | `applying-desktop-dialog.png`, `applied-desktop-dialog.png`, `applied-narrow-dialog.png`, `apply-failed-desktop-dialog.png` |
| Stale review | `stale-desktop-dialog.png`, `stale-narrow-dialog.png` |
| Retired (another project opened during a pin) | `retired-desktop-dialog.png`, `retired-desktop-window.png` |
| Keyboard focus on Pin | `dialog-keyboard-focus-desktop-dialog.png` |

**Rendered-pixel review.** I inspected these images, including 2× crops. Three defects were
found and fixed; the before images are in `evidence/iterations/`.

1. **Truncated name** (r1). At 1440 px the name rendered as "Har…": the button's
   `max-width: 100%` resolved against a heading width that already counted its −5 px
   alignment margin. A probe (`tools/probe-title.mjs`) measured 38 px given for 43 px
   needed. With the `max-width` removed, the full name renders at 1440 and 960 px, and only
   long names truncate.
2. **Run-on hint** (r2). The apply-refusal hint ran into the host's unpunctuated message
   ("…current 8 A new review…"). The hint now has its own line after the code; the host's
   words are left untouched.
3. **Merged emphasis** (r2). In the retired message, "Lighthouse" took the name emphasis and
   merged with the bold sentence, and "its pin" was ambiguous. Names inside notices are now
   plain, and the text says "Whether Harbor was pinned is not shown here."

Dialogs measured 130–559 CSS px tall, all inside the window; none scrolls, including
details open at 1024×680. The page never scrolls (`scrollWidth`/`scrollHeight` equal to the
viewport in all 28 captures).

## Commands and outcomes

Run from `editor/ui` unless noted, with the parent checkout's binaries
(`../../../../node_modules/.bin/…`), serially, with no watch mode. Logs are kept
byte-exact as deterministic gzip, with raw and gzip SHA-256 in each `manifest.json`.

| Command | Outcome | Log |
| --- | --- | --- |
| `vitest run --no-file-parallelism src/components/ProjectFile.test.tsx` (first run) | **failed**: 11 failed, 6 passed (jsdom drops the space in a visually hidden "Project: " span, so names were "Project:Harbor"; the hierarchy filter was absent with no scene) | `evidence/original-failures/run1-projectfile-failed.txt.gz` |
| `tsc -b --noEmit` (after adding tests) | **failed**: TS2345, a test helper typed `EventTarget` | `evidence/original-failures/tsc2-failed.txt.gz` |
| Full suite, interrupted by the session restart | no result (captured only jsdom canvas warnings); not counted | `evidence/original-failures/full1-interrupted.txt.gz` |
| Same file after the `aria-label` and fixture-scene fixes | pass, 17 | `evidence/runs/run2-projectfile.txt.gz` |
| Project-file + shell tests after root's revision | **failed**: 1 failed, 26 passed (an expectation for the superseded project-change wording) | `evidence/runs/run3-projectfile-shell.txt.gz` |
| Successors | pass 27, 24, 28 | `run4…`, `run5…`, `run6….txt.gz` |
| Existing related files after the first implementation (App, ProjectStates, ShortcutsDialog, Windowing, Menu, resolve, ShellContext) | pass, 7 files, 99 tests | `evidence/runs/existing-related-tests-before-new-tests.txt.gz` |
| **Final** `tsc -b --noEmit` (app, node and test projects) | **pass**, no output | `evidence/runs/final-typecheck.txt.gz` |
| **Final** `vitest run --no-file-parallelism` (whole UI suite, including `../bridge` tests) | **pass**: 47 files, 965 tests | `evidence/runs/final-tests.txt.gz` |
| **Final** `tsc -b && vite build` | **pass** | `evidence/runs/final-build.txt.gz` |
| `/opt/homebrew/bin/python3 -I handoffs/0077-project-file-ui/tools/mutations.py …` (worktree root) | 5 of 6 detected (see above); sources restored and hash-checked | `evidence/mutations.json`, `evidence/runs/final-mutations.txt.gz` |
| `node handoffs/0077-project-file-ui/tools/capture.mjs r4` (worktree root) | 28 captures, axe 0 violations each (real Chrome, colour contrast on), 0 page errors, 0 external requests; 5 flows pass | `evidence/browser-report.json`, `evidence/runs/capture-r4.txt.gz` |
| `git diff --check`; `git diff --no-index --check` for every new text file | pass (the second check was verified to fail on a planted trailing space) | — |

**Browser flows** (real key presses in headless Chrome, through the real compiled bridge;
`evidence/browser-report.json`):

- **⌘S in the hierarchy filter.** One `{operation:'save', expected_revision:7}`; the
  filter kept its text and focus.
- **Keyboard only.** Tab to the title (focus ring), then Enter opens the menu on Save;
  ArrowDown and Enter open the dialog with focus on Cancel. Tab reaches Pin; Enter pins.
  Focus moves to Close, and Escape returns it to the title. The host received
  `review_pin` and then `apply_pin` with the returned ID; History gained "Pin opened
  project to the current engine".
- **Repeated Save.** ⌘S three times plus choosing Save while saving sent one request.
- **Drag areas.** Pressing the title and a menu item started 0 window drags; the empty
  titlebar started 1.
- **Escape** returns focus to the title.

## Request routing and backend boundary

- The new UI sends only the three existing requests, through `Shell.request`:
  ```
  ProjectFileContext.tsx:159  request({ type: 'project.save' }, …)
  ProjectFileContext.tsx:186  request({ type: 'project.pin.review' }, …)
  ProjectFileContext.tsx:253  request({ type: 'project.pin.apply', reviewId: review.review_id }, …)
  ```
  No path, text, candidate or revision is sent from the UI. The bridge adds the current
  revision itself, unchanged. Tests assert the exact host payloads (`{operation,
  expected_revision[, review_id]}`), as does the browser report.
- Unchanged:
  - `editor/bridge/contract.ts`, `native.ts` and `native.generated.js`
    (`git diff --quiet HEAD` passes);
  - `git status` shows nothing under `editor/bridge`, `crates`, `editor/app`, `schemas`,
    `sdk`, `tools`, `package.json`, `package-lock.json` or `editor/ui/package.json`.
- No Rust, Cargo or native build ran, no app was launched, and no GPU work, performance
  sample or timing claim was made. The Rust files were read as background only.
- Fixture mode stays limited:
  - The sample fixture advertises no capabilities, so Save and Pin are unavailable there
    and say why.
  - The browser captures run the real compiled bridge over `tools/fake-host.js`, a
    scripted IPC whose "file" is a string in the page. No file or journal is written, and
    the bridge is relabelled as a fixture on screen.

## Changed paths

Modified:
- `editor/ui/src/App.tsx`: provider; ⌘S/Ctrl+S routed to Save (taken but inert in
  dialogs); dialogs block other shortcuts while the pin dialog is open; the pin dialog
  mount; the failure notice in the existing "Project status" region.
- `editor/ui/src/components/Titlebar.tsx`: the project title comes from `ProjectTitle`.
- `editor/ui/src/shell/ShellContext.tsx`: `request` turns rejections into failures.
- `editor/ui/src/bridge/resolve.ts`: label `'project.file'` changed from "Project file
  operations" to "Project file access", fixing "operations is unavailable".
- `editor/ui/src/components/ShortcutsDialog.tsx`: "Save the project" row.
- `editor/ui/src/icons/Icon.tsx`: `save` and `pin` glyphs for the menu.
- `editor/ui/src/styles/shell.css`: the title button and the notice's Dismiss.
- `editor/ui/src/styles/dialogs.css`: the pin dialog section.
- Tests: `editor/ui/src/components/ShortcutsDialog.test.tsx` (Save row on both
  platforms) and `editor/ui/src/shell/ShellContext.test.tsx` (rejection).

New:
- `editor/ui/src/projectFile/model.ts`: availability reasons, staleness, receipt words,
  failure titles.
- `editor/ui/src/projectFile/ProjectFileContext.tsx`: requests, scope binding, pin state.
- `editor/ui/src/components/ProjectMenu.tsx`: the title and its menu.
- `editor/ui/src/components/PinDialog.tsx`: the review dialog.
- `editor/ui/src/shell/requests.ts`: the rejection to failure mapping.
- `editor/ui/src/components/ProjectFile.test.tsx`: 24 flow tests against the real
  `NativeBridge`.
- `handoffs/0077-project-file-ui/`: `result.md`, `screenshots/` (31 PNG),
  `evidence/` (logs, manifests, mutation report, browser report, iteration images) and
  `tools/` (`fake-host.js`, `scenarios.mjs`, `capture.mjs`, `probe-title.mjs`,
  `mutations.py`, `collect.py`).

Source size: 11 tracked files, +445/−24 (including root's 10-line brief revision); new
source and tests 1,657 lines.

## Accessibility observations

- **Menu.** The title button has `aria-haspopup="menu"`, `aria-expanded` and the name
  "Project: Harbor" (", saving" etc. while busy). The menu is the existing
  `ContextMenu`: arrows, Home/End, Enter/Space, Escape and Tab close it and return focus.
  Unavailable items are spoken with their reason.
- **Dialog.**
  - `role="dialog"`, `aria-modal`, labelled by the title and described by the current
    state's lead: preparing, summary, refusal, retired or pinned.
  - `role="status"` for progress and outcome; `role="alert"` for refusals, stale and
    retired.
  - Focus is trapped (Tab and Shift+Tab wrap); Escape closes; focus returns to the
    opener.
  - Initial focus is Cancel, never Pin. When Pin finishes or Review again starts, focus
    moves to the remaining safe button rather than being lost.
  - "Show all" moves focus into the text well it expands.
- **Notice.** `role="alert"` inside the existing "Project status" region. Dismissing it
  from the keyboard moves focus to the title.
- **Focus rings.** Visible in Chrome captures (title, menu item, Cancel, Pin). New
  focusable parts have `[data-focus-visible]` twins for WebKit script focus.
- **axe.** 0 violations in all 28 Chrome captures with colour contrast enabled; 0 in jsdom
  for menu, dialog and refusal states. Text uses the existing token pairs that
  `tokens.test.ts` enforces.
- **Drag area.** The title button and the menu (`data-no-drag`) start no window drag;
  nothing interactive is nested.
- **Narrow widths.** At 1024×680 the panels, viewport and dock stay in place. Long names
  ellipsize; the scene breadcrumb's existing shrink is unchanged.

## Process closure

The test, build, probe and capture runs were all bounded and serial. The capture and probe
scripts closed their own Vite preview servers and headless Chrome instances (each with a
temporary profile). Afterwards `ps` showed no `vitest`, `vite` or headless Chrome process.
Ignored outputs remain: `editor/ui/dist`, `editor/ui/node_modules/.tmp|.vite|.vite-temp`,
and `artifacts/0077-project-file-ui/r1–r4` (the earlier sets hold the before images).

## Limitations and missing bindings

1. **No revision in the snapshot.** `BridgeSnapshot` has no project revision, so the UI
   detects change through History (applied position, entry count and the transaction IDs
   at both ends). In the command bus each revision step pushes or pops a History
   transaction. An undo and redo that both land between two reads look unchanged to the
   UI; the bridge and host revision checks still refuse that review. A `revision` field
   in the snapshot would remove the proxy.
2. **No engine requirement in the snapshot.** "Pin engine version…" is offered for every
   file-backed project, and an already-pinned project shows the host's refusal ("engine
   pin review applies only to a legacy project without an engine declaration").
3. **Error classes.** Host errors arrive as strings with the bridge code
   `engine.request`. The UI does not parse them, so stale, unsupported, I/O and
   already-pinned refusals all get the same title and generic next step. Observed
   backend wording: a project-file conflict is reported with the shared source error
   "script source changed; read it again before editing" (`incant_source::SourceError`).
   That is a backend copy issue for root; the UI shows it verbatim.
4. **Uncommitted drafts.** ⌘S from a text field saves the document as it is. An
   uncommitted rename or allowance draft is not in the document and is not saved; the
   receipt reports the saved revision.
5. **Not native.** No native WKWebView or file was exercised. Native acceptance remains
   with root. Untested natively: whether a native menu item claims ⌘S first, and real
   save, pin and journal behaviour.
6. **`Shell.dispatch`** has the same uncaught-rejection shape that `request` had. It is
   unchanged here (outside these flows) and reported for root.
7. **Docs not updated.** `editor/ui/DESIGN.md` and `TITLEBAR.md` are outside the allowed
   paths. Proposed text follows.

### Proposed DESIGN.md / TITLEBAR.md text (not applied)

- **Titlebar:**
  > Once a project is open, its name is also the project file menu's button: quiet, with
  > a chevron, filled while open. It offers Save (⌘S / Ctrl+S) and Pin engine version….
  > Unavailable items stay listed and say why. A small spinner replaces the chevron while
  > a file request runs. The button is not a drag area.
- **Banner:**
  > …or a project file request failed outside a dialog: the host's words and code, with
  > Dismiss.
- **Dialogs:**
  > **Pin engine version.** 480 px. One sentence with the project and the requirement the
  > host returned, and a quiet line with the revision it was reviewed at. Review details
  > on request: command, review ID, both SHA-256, and the pinned file text (64 K
  > characters until Show all). Cancel has focus; Pin is the accent and never takes focus
  > by itself. A review that is stale, for another project, or without file access offers
  > Review again with the reason. Refusals show the host's words and code; results the
  > host never reported are titled unknown. No close glyph.
- **Keyboard:**
  > ⌘S / Ctrl+S saves from anywhere outside a dialog, text fields included; in a dialog
  > the keys are taken and do nothing.

## Open questions for root

1. Should the bridge snapshot expose `revision` and the project's `engine_version`? Either
   would let the UI replace a proxy (limitations 1 and 2) without new operations.
2. Should a native menu item own Save (as History has `incant:history-request`)? The UI
   would then route that intent to the same `project.save` path.
3. Should host refusals carry stable codes, so the UI could give specific next steps
   (save first, reopen, already pinned) without reading message text?
