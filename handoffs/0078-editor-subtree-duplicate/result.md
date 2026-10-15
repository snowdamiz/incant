# 0078 — Hierarchy subtree Duplicate: result

Status: **implemented; verified by focused UI tests, mutation checks and a headless-browser
fixture. Awaiting root review and native verification.** The work is committed on
`handoff/0078-editor-subtree-duplicate` only: nothing is pushed, no PR is opened or merged.
This is editor UI work. It claims no Phase 2 gate. No test here exercises the command bus,
the native host or WebKit.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`). **Thinking effort:** max. The session's own
  configuration reports both; it cannot read the ACP launcher's settings back.
- **Brief applied:** `handoffs/0078-editor-subtree-duplicate/brief.md`, SHA-256
  `d347575e2dbc20925484d56d599c0b83c07a6b45f86a64d682dfb94b1d022650`, identical to the
  committed copy. It had no priority revisions when the session started or before the final
  commit. The brief is unchanged; no root review receipts were present.
- **Source identity:** implementation commit `2350a063f6e8968db9a87bfb114f241116e5d885`
  and test follow-up `71a46a8bf099d81c04e571f089ec6c52e84ca96d` (tree `4374b4d3…`), on
  root's prep commit `88d51b41`. Every result below was produced at `71a46a8b`; blob hashes
  of every changed file are in `evidence/source.json`. This result, the evidence and the
  screenshots are in a third commit on top.
- **Machine and tools:** Mac17,9, macOS 27.0.1 (26A434). Node v22.23.3 (the pinned
  installation), TypeScript 5.9.3, Vitest 5.0.3, Vite 8.3.4, React 19.3.0, axe-core 4.14.0,
  playwright-core 1.64.0, Chrome 155.0.8059.40 headless with its own temporary profile. The
  parent checkout's `node_modules` was used read-only: its `package-lock.json` SHA-256
  (`cbea068b…7c60`) matches this worktree's. Nothing was installed. No Cargo, rustc, editor
  binary, GPU test or shared target was used.

## What the user sees

- **Context menu:** Rename `F2` · **Duplicate `⌘D`** (`Ctrl+D` on Windows and Linux) · Copy ID
  · Delete. Duplicate sits with the other authoring edit and is kept away from the danger item.
  It has a new 16 px, 1.5 px-stroke glyph, `duplicate` (a square with a plus, in front of
  another square). In the 2× and 1× captures it reads as distinct from Copy ID's plain copy
  glyph.
- **Shortcut:** `⌘D` on macOS and `Ctrl+D` elsewhere, only while the hierarchy tree has focus.
  It matches the layout's D, or the physical D key on a layout without Latin letters, like
  Save. It ignores the other platform's modifier, Shift, and Option/Alt (AltGr). Held keys do
  not repeat it. Whenever the tree handles the key (acting, explaining or ignoring a repeat),
  it is taken (`preventDefault`). It does nothing in the filter or rename field, in dialogs
  or in other panels.
- **While it runs:** one `entity.duplicate` command. Nothing new is shown: no spinner, no
  placeholder row, no selection change. The tree is `aria-busy`. Pressing again announces
  "Wait for the current duplicate to finish.", and the menu item says the same.
- **Success:** the copy whose ID the engine returned is selected and scrolled into view in the
  virtualized tree. Its parent stays expanded and the copy itself is collapsed. It takes
  keyboard focus if the tree had focus. The status line says "Duplicated Crate 01". The
  engine names and places the copy; the UI sends no rename or second command. Undo and Redo
  are the existing History commands.
- **No selection** (the copy stays in the tree if the engine made it) when:
  - another row was chosen while the command ran;
  - another project opened, or the bridge was replaced (nothing is announced either);
  - the hierarchy panel that asked was unmounted (nothing is announced either);
  - the returned copy is not in the latest snapshot: "Duplicated Crate 01, but the copy is no
    longer in the hierarchy."
- **Failure and unknown results:** the selection and focus stay on the source.
  - A refusal is shown in the host's words: "Duplicate failed: invalid command: duplicate
    subtree: detach linked instances explicitly before duplication" (and likewise the singleton
    and reference-semantics refusals).
  - An unverified completion: "Duplicate result unknown: The duplicate result could not be
    verified. Check the hierarchy before retrying."
  - A success without an ID: "Duplicate result unknown: the host reported success but
    returned no copy."
  - A source that vanished before sending: "Duplicate failed: the entity no longer exists."
    (No command is sent.)
- **Unavailable**, with the reason announced for the shortcut and shown by the menu item
  (dimmed, still focusable):
  - an older or read-only host ("Duplicate is unavailable in this editor build
    (entity.duplicate)." / "…: the sample fixture is read-only.");
  - a scene ("Scenes cannot be duplicated.");
  - Play has a world, including while it is preparing ("Stop Play to duplicate.") or stopping
    ("Wait for Play to stop."). This keeps the selection that runtime inspection reads, and
    sends no new revision while a session is starting. Preparing, running and stopping are
    each tested.
- **Keyboard shortcuts dialog:** "Duplicate ⌘D" (Ctrl+D elsewhere) in Hierarchy, between Rename
  and Delete. Plain text, like the other rows.
- **Shell (`src/shell/ShellContext.tsx`):**
  - A rejected `dispatch` now resolves, as `request` already did: it returns `outcomeUnknown`
    with the thrown error's code and message, or `request.rejected`.
  - Unknown results from `dispatch` are announced as "… result unknown: …", not as "failed".
  - Duplicate failures use the plain label "Duplicate failed". Other commands keep their
    existing labels.
- `editor/ui/DESIGN.md` describes the item and these rules in the Hierarchy section.

## Changed paths (commits `2350a063` and `71a46a8b`)

| Path | Change |
| --- | --- |
| `editor/ui/src/hierarchy/duplicate.ts` (new) | Shortcut matcher, availability reasons, `useDuplicate` command lifecycle |
| `editor/ui/src/components/HierarchyPanel.tsx` | Shortcut, menu item, `aria-busy`, focus hand-off |
| `editor/ui/src/shell/ShellContext.tsx` | Rejected dispatch resolves; unknown vs failed labels |
| `editor/ui/src/components/ShortcutsDialog.tsx` | Reference row |
| `editor/ui/src/icons/Icon.tsx` | `duplicate` glyph |
| `editor/ui/DESIGN.md` | Hierarchy section |
| `editor/ui/src/components/HierarchyDuplicate.test.tsx` (new) | 32 focused tests |
| `editor/ui/src/components/HierarchyMenu.test.tsx` | Menu order now includes Duplicate |
| `editor/ui/src/components/ShortcutsDialog.test.tsx` | ⌘D / Ctrl+D row and filter |
| `editor/ui/src/shell/ShellContext.test.tsx` | Rejected and unverified dispatch labels |
| `handoffs/0078-editor-subtree-duplicate/tools/*` | Browser fixture, capture, mutation and collection tools |

The following were not edited: `crates/incant_cmd`, `editor/app`, `editor/bridge/contract.ts`,
`editor/bridge/native.ts`, their tests, and the generated bridge.

## Commands and results

All runs used the pinned Node. Logs are in `evidence/`.

| Command | Result |
| --- | --- |
| `node ../../node_modules/typescript/bin/tsc -b editor/ui --noEmit` (worktree root) | exit 0, no output (`evidence/typecheck.txt`) |
| `vitest run` (all of `editor/ui`, JSON reporter) | exit 0; **49 files, 1,016 tests passed**, 0 failed (`evidence/vitest-full-summary.json`). Baseline before the change: 48 files, 983 tests passed |
| `vitest run` on the four focused files | exit 0; 48 tests passed (`evidence/vitest-focused.txt`) |
| `vite build` (production) | exit 0. The bundle is byte-identical to the one the screenshots were taken from: JS `783787e5…48da`, CSS `0b2651de…77dc` (`evidence/build.txt`, `evidence/capture-bundle.txt`) |
| `python3 -I tools/mutations.py` | **27 of 27 guard mutations caught**. Each removes one guard and reruns the focused files; the source is restored and its SHA-256 checked (`evidence/mutations.json`) |
| `node tools/capture.mjs r2` | 21 captures and 5 flows. axe found 0 violations on every capture (full document, real layout, colour contrast included). 0 page errors, 0 requests outside the local server. Chrome and the preview server closed: afterwards no listener and no headless Chrome process remained (`evidence/browser-report.json`) |
| `python3 -I tools/collect.py` | Assembled `evidence/` and `screenshots/` (raw log hashes recorded) |

What the focused tests cover:

- the shortcut on macOS and elsewhere, and modifier strictness;
- the menu (pointer and keyboard), placement and the shortcut label;
- the nested subtree, with the copied root selected and its descendants under it;
- the long virtualized list (the copy lands as the 51st child, outside the initially mounted
  rows, and is mounted and focused as the active row);
- selection of the exact returned ID even with same-named new rows;
- an absent capability (read-only fixture and older host);
- pending input (held keys, a second press, two presses in one task, the menu while pending);
- refusals, unknown results, a rejected dispatch, and a success without the copy;
- source disappearance (before dispatch, and during the command);
- typing, dialog and other-panel guards;
- scenes;
- another project, a replaced bridge, an unmounted asker, and a selection or focus that moved
  on;
- the real `NativeBridge` with a scripted host (arguments, receipt failure, older host);
- live Play through the shared Play double: preparing, running and stopping.

## Screenshots — FIXTURE, not live engine evidence

`handoffs/0078-editor-subtree-duplicate/screenshots/` (hashes and measurements are in
`manifest.json`).

How they were made:

- **Host:** `tools/fake-host.js`, a scripted IPC under the real compiled
  `editor/bridge/native.generated.js`. The bridge is relabelled "Duplicate fixture: scripted
  host IPC, not live" and marked as a fixture, so the titlebar says "Sample data".
- **Project:** handoff 0033's invented harbor project.
- **Rendering:** headless Chrome, dark scheme, device scale 2 (one at 1×). Before each capture
  the pointer is moved over the empty viewport, so no row shows a hover state.

| File | Shows |
| --- | --- |
| `menu-desktop-hierarchy.png`, `menu-narrow-hierarchy.png` | Context menu with Duplicate ⌘D (1440×900, 1024×680) |
| `menu-keyboard-desktop-hierarchy.png` | Menu opened with Shift+F10, Duplicate with keyboard focus ring |
| `menu-1x-desktop-hierarchy.png` | Same menu at device scale 1 (glyph legibility) |
| `menu-windows-desktop-hierarchy.png` | Windows chrome: Ctrl+D, Delete |
| `copied-desktop-window.png`, `copied-narrow-window.png` | "Player Character" (with a child) copied: copy selected and collapsed at the end of the scene, Inspector and status line |
| `copied-nested-desktop-hierarchy.png` | "Map Camera" copied under Cameras |
| `copied-long-desktop-window.png` | 600-child group: tree scrolled from 0 to 16,274 px; the copy (601 of 601) is selected and focused; 37 rows mounted |
| `refused-singleton-desktop-window.png`, `refused-singleton-status-narrow-statusbar.png` | Singleton refusal; selection stays on Environment; the full text fits at 1024 px |
| `refused-linked-desktop-statusbar.png`, `refused-references-desktop-statusbar.png` | Linked-instance and unregistered-reference refusals |
| `unknown-desktop-window.png` | Unverifiable receipt: "result unknown". The fixture did commit, so the refreshed tree shows the copy, unselected |
| `pending-desktop-hierarchy.png` | Menu while a copy is pending: Duplicate dimmed, nothing else changed |
| `older-host-desktop-hierarchy.png`, `scene-desktop-hierarchy.png` | Duplicate dimmed for a host without the capability and for a scene |
| `playing-desktop-window.png` | Live Play (0044 Play double): Runtime section, Duplicate dimmed, "Stop Play to duplicate." |
| `history-desktop-window.png` | The fixture's History after one duplicate: one "Duplicate subtree" user entry |
| `shortcuts-desktop-dialog.png`, `shortcuts-windows-desktop-dialog.png` | Reference rows ⌘D / Ctrl+D |

The pixel review was done by reading these images. The menu, reference row and selection
use the existing tokens, rhythm and focus ring, and nothing ornamental was added. An earlier
set, `r1` (ignored, not committed), had a wrong History selector and pointer-hover highlights
on source rows; `r2` corrects both.

### Browser flows (fixture)

- **Keyboard only:** Tab to the tree, type-ahead to "Map Camera", ⌘D. Focus moves to the copy
  with a visible ring. ⌘Z sends `engine_history {redo:false}`, ⇧⌘Z sends
  `{redo:true}`, and History holds one "Duplicate subtree (user)".
- **Held keys:** holding ⌘D sends one `engine_duplicate_subtree` and says nothing. A second
  press says "Wait for the current duplicate to finish." No row is added.
- **Guards:** none of these sent a request:
  - the filter field keeps "cra";
  - the rename field stays open;
  - the viewport panel and the shortcuts dialog do not react;
  - the scene row says "Scenes cannot be duplicated.".
- **Live Play:** "Stop Play to duplicate."; the selection and the Runtime section stay on
  Crate 01. After Stop the item is available again.

## Accessibility

- axe on every capture (Chrome, real layout and colours) found 0 violations. axe in jsdom
  with the menu open found 0 (colour contrast is covered by the browser run).
- An unavailable item keeps its role and focus. Its accessible name ends ", unavailable:
  <reason>" and its title carries the reason.
- Announcements go to the existing polite status region.
- The keyboard ring on the copy (measured in Chrome after the keyboard-only flow) comes
  through the existing input-modality rule. That rule also sets `data-focus-visible` for
  WebKit, so the same ring is predicted natively; WebKit was not run. After a pointer click
  and then ⌘D, Chrome focused the copy without a ring.

## Limitations and open questions

1. **Native runs were not done.** This includes the editor binary, WebKit, and whether ⌘D
   reaches the webview natively.
   - `editor/app/src/menu.rs` replaces only Undo and Redo in Tauri's default menu. A search of
     `editor/app` (sources, `Cargo.toml`, `tauri.conf.json`) found no other accelerator and no
     global-shortcut plugin.
   - Tauri's default menu contents were not inspected.
   - Root should confirm ⌘D, Ctrl+D and the copy's selection in the native editor.
2. **Command-bus correctness is not shown here.** That covers the subtree copy, internal
   references, provenance and a single reversible transaction. These tests and the fixture
   are doubles. The fixture copies the bus's refusal texts and imitates its ordering and three
   typed reference remaps; it is not engine output.
3. **Focus after Undo (pre-existing tree behaviour, not changed).**
   - When Undo removes the focused copy, focus falls to the page body and nothing is selected.
     Redo brings the copy back selected but not focused.
   - Delete of a focused row behaves the same in the fixture (`delete-focus-comparison`): the
     tree has no handling for a removed focused row.
   - Possible follow-ups, not implemented because the brief keeps Undo/Redo on existing
     history:
     - after Undo removes a copy that is still selected, select and focus its source again (as
       Unity and Figma do);
     - or, generally, keep focus in the tree when the focused row disappears.
   - Director decision requested.
4. **Remounts while a command runs.**
   - If the tree remounts mid-command (panel hidden, Assets tab, reload, bridge replaced), the
     answer is not selected or announced (the scope rule from 0077).
   - The new instance does not know a command is in flight. A second press would be refused
     by the native bridge's own "Wait for the current engine operation." rather than by the
     UI's pending guard.
5. **Fixture labelling changes one reason.** In the browser fixture, the relabel sets
   `isFixture`, so the older-host capture's reason reads "…the sample fixture is read-only."
   The real older-host text is covered by the jsdom test with the real `NativeBridge`.
6. **Interpretation of the live-Play guard.** "Live play inspection" is read as any Play world
   (preparing, running, suspended, paused, stopping). Duplicate is unavailable there in both
   the menu and the shortcut, while Rename and Delete keep their existing behaviour during
   Play. Root may prefer a narrower guard.
7. **`Shell.request` is unchanged.** It still announces unknown outcomes as "… failed"; only
   `dispatch` now says "result unknown". The project-file UI (0077) titles its own unknown
   results.
8. **Long status-line messages** truncate with an ellipsis when they do not fit (the existing
   `text-overflow` rule). At 1,024 px the singleton refusal (107 characters, tied for the
   longest) fits beside the fixture label. Narrower windows, down to the 960 px minimum, were
   not captured.
9. **`isDuplicateShortcut` mirrors App.tsx's private `isSaveShortcut`** rather than sharing it,
   to leave App.tsx untouched.

Not done here, and human-owned or root-owned: native verification, root review, push, PR,
merge and any phase-gate decision.
