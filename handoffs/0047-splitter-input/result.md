# 0047 splitter input: result

## Status

- **Fixed in source, tested in jsdom, reviewed in a browser. Not accepted natively.**
  - **The defect:** dragging a panel separator no longer starts a text selection.
  - **Other behaviour:**
    - after a pointer press, focus is on the separator, so arrow keys continue the resize;
    - secondary clicks do not start a drag;
    - a drag ends cleanly on pointer up, pointer cancel, lost capture, or a lost release.
  - **Where:** all changes are in `editor/ui/src/components/Splitter.tsx`.
- **Not verified natively.** The native WebKit window is the root's to test, with a binary
  built from this source. That is where the visible symptom occurred. The checklist is
  under "Native acceptance".
- **Gates and claims:** no phase gate is approved or claimed. Nothing here is a device,
  phone or performance claim.

## Model and effort

- **Exact model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max. The session reports reasoning effort "max".
- **How it was verified:**
  - This session's prompt is the handoff runner's post-verification prompt.
  - The runner's `configure_model_and_effort` (`tools/handoff/main.py`) sets both settings
    through ACP, and it raises before prompting unless ACP confirms them.
  - I cannot see the runner's own confirmation output from inside the session.
- **No substitution, and no routing exception.**

## Packet revisions and feedback

- **No changes to apply:** the packet has no priority revision and no new director
  feedback.
- **Clean start:** the worktree was clean at `dc5e620`. No files were left from an
  interrupted attempt.

## Cause

`Splitter.tsx` captured the pointer on press but kept the press's default action.

**The press.** On a mouse press, the browser's default action focuses the target and starts
a text selection. The start is at the press point, beside the separator.

**The drag.** The drag then extends that selection. In WebKit the extension follows the
pointer across the neighbouring panel, which is the Inspector text in 0044's
`defect-splitter-drag-selects-text.png`.

**Secondary buttons.** Any button started a drag. A right-click or Control-click can open
WebKit's context menu. If the menu takes the button release, the drag keeps following the
mouse after the menu closes. That last step is my reading of the code; I did not observe
it.

## Fix

All changes are in `Splitter.tsx`.

1. **Selection.** Every primary-button press cancels the `mousedown` default action, so no
   selection starts. That includes a Control-click: on Windows or Linux it is an ordinary
   press, and the selection must not start there either.
   - **Why `mousedown`:** the 0044 note suggested cancelling `pointerdown` instead.
     Cancelling `pointerdown` also suppresses the compatibility `mousedown` event, and
     `Menu.tsx` closes an open menu on a window-level `mousedown`.
   - **Effect:** with the suggested version, pressing a separator would leave an open menu
     open. A mutation test confirms the new tests catch that.
2. **Focus.** The separator takes the focus that the default action would have given:
   `focus({ preventScroll: true, focusVisible: false })`.
   - **Why `focusVisible: false`:** in Chromium 156, script focus after keyboard use
     matches `:focus-visible`, so the separator's focus line stayed after a mouse drag.
     The default mouse focus does not do this, and neither does script focus with
     `focusVisible: false`. `tools/probe-focus-visible.mjs` measured all three
     (`evidence/probe-focus-visible.json`), and the browser review checks the result.
   - **Other engines:** engines without the option ignore it.
   - **TypeScript:** its DOM types lack the option, so it is passed as a constant.
3. **Buttons.** Only a primary-button press from the primary pointer, without Control,
   starts a drag.
   - **Secondary and middle buttons, and a second touch:** keep their default action, and
     neither capture the pointer nor resize.
   - **Control-click (the macOS secondary click):** does not drag either. It still loses
     its selection start, and its context menu is untouched.
4. **Ending.**
   - A drag belongs to the pointer that started it.
   - It ends on `pointerup`, `pointercancel` or `lostpointercapture` from that pointer.
   - It also ends at the first move without the primary button held, in case the release
     was lost (for example, to a native context menu opened mid-drag).
   - Other pointers neither move it nor end it.

**Unchanged:**
- keyboard behaviour: Arrow keys step 16 px (64 px with Shift); Home and End go to the
  bounds;
- `aria-valuenow`, `aria-valuemin`, `aria-valuemax` and the label;
- clamping, axis and invert;
- styles and text copying.

No `user-select` rule was added anywhere.

## Tests

**New file:** `editor/ui/src/components/Splitter.test.tsx`, with 15 tests.

**The press helper.** jsdom runs no default actions. So `press()` dispatches what a browser
dispatches:
- `pointerdown`;
- then the compatibility `mousedown`, only for a primary mouse or pen pointer, and only if
  `pointerdown` was not cancelled.

It returns whether the default action, focus and selection start, would still run.
jsdom also lacks pointer capture, so a labelled test double records captures.

**Pointer resizing tests:**
- **Primary press:** a primary press from a focused text field cancels the default action,
  captures pointer 1 and focuses the separator. The next ArrowRight resizes by 16, and the
  field's value is unchanged.
- **Presses that must not drag** (3 tests):
  - secondary button;
  - middle button;
  - second (non-primary) touch.

  Each keeps its default action, moves nothing, captures nothing and leaves focus where it
  was.
- **Control-click:** cancels the default action and focuses the separator, but neither
  captures nor resizes.
- **Axis and bounds** (4 tests): one for each orientation and invert setting. Movement
  along the axis resizes; movement across it does not; values clamp at both bounds.
- **Ending** (3 tests): the drag ends on `pointerup`, `pointercancel` and
  `lostPointerCapture` from its own pointer.
  - Another pointer's move or end is ignored; each is checked.
  - After the end event, a move with the button still held changes nothing.
  - The next press starts from the new size.

**Lost release test:** the first move without the primary button ends the drag.

**Editor shell tests:**
- **Hierarchy separator:** a pointer drag resizes the grid column. The separator is
  focused without the shell's keyboard ring, and ArrowRight continues the resize with the
  ring.
- **Open menu:** pressing the Inspector separator closes the hierarchy context menu and
  focuses the separator.

**Before the fix:** 11 of the 15 fail on the base-commit component
(`evidence/vitest-splitter-before-fix.txt`). The four axis and bounds tests pass there too,
by design: they pin behaviour that had to stay the same.

**Mutation checks** (`evidence/mutations.txt`). Each mutation removes or replaces one part of
the final file. Every one fails at least one test:

| Mutation | Tests that fail |
| --- | --- |
| No pointer filter on move | the 3 ending tests |
| No lost-release guard | the lost release test |
| Control-click not cancelled | the Control-click test |
| No `lostpointercapture` handler | its ending test |
| No explicit focus | 4 tests |
| The 0044 suggestion: cancel `pointerdown` instead of `mousedown` | the menu test and the Control-click test |

Each mutation started from the final file. Afterwards the file was restored, and `cmp`
confirmed it byte for byte.

## Commands and results

Commands run from the worktree root unless noted.
- **Dependencies:** the parent checkout's pinned `node_modules` is symlinked, as the packet
  allows; both checkouts' `package-lock.json` SHA-1 is `e33abebd…cc944`.
- **Caches:** a real, ignored `editor/ui/node_modules/` keeps TypeScript build info and
  Vite caches inside this worktree.
- **No installs or upgrades.**

| Command | Result |
| --- | --- |
| `cd editor/ui && ../../node_modules/.bin/vitest run --maxWorkers=4` (baseline, before any change) | 40 files, 782 tests passed |
| `cd editor/ui && ../../node_modules/.bin/vitest run --maxWorkers=4 --reporter=verbose src/components/Splitter.test.tsx`, with the `dc5e620` Splitter put back for the run | 11 failed, 4 passed (`evidence/vitest-splitter-before-fix.txt`) |
| same command, final Splitter | 15 passed (`evidence/vitest-splitter-after-fix.txt`) |
| mutation checks, same command | each of 6 mutations fails at least one test (`evidence/mutations.txt`) |
| `cd editor/ui && ../../node_modules/.bin/vitest run --maxWorkers=4` (final) | 41 files, 797 tests passed (`evidence/vitest-full.txt`) |
| `cd editor/ui && ../../node_modules/.bin/tsc -b --noEmit`, with build info removed first so all three projects (app, node and test) are fully checked | exit 0 (`evidence/typecheck.txt`) |
| `cd editor/ui && ../../node_modules/.bin/tsc -b && ../../node_modules/.bin/vite build` | exit 0 (`evidence/build.txt`) |
| `node handoffs/0047-splitter-input/tools/browser-review.mjs` | exit 0. All 34 checks pass on the fixed Splitter; 12 of the 34 fail on the base-commit Splitter (`evidence/browser-review.txt`, `evidence/browser-review.json`) |
| `node handoffs/0047-splitter-input/tools/probe-focus-visible.mjs` | After a keyboard-focused field, `:focus-visible` matches with script focus only: default focus `false`, script focus `true`, script focus with `focusVisible: false` `false` (`evidence/probe-focus-visible.json`) |

In the saved logs, the worktree's absolute path is replaced with `<worktree>`.

## Browser review

**This is browser evidence, not native evidence.**
- **Browser:** headless Chromium 156.0.8078.4, from the Playwright cache that matches the
  pinned playwright-core 1.64.0.
- **Input:** real mouse and keyboard.
- **Servers:** `vite preview` on private ports from 47470, bound to 127.0.0.1.
- **Not used:** native apps, the user's desktop, accounts or provider calls.

**What `tools/browser-review.mjs` builds:**
- the harness in `tools/harness/`: the real Splitter, shell styles and input-modality
  tracking, with selectable paragraphs on both sides, in both orientations;
- the editor UI on `?fixture=sample`.

It builds both twice, "before" and "after":
- **"before"** is the Splitter from `dc5e620`. SHA-256 `d96979c2…29153`.
- **"after"** is this handoff's Splitter. SHA-256 `76d1fa85…40a9c`. That hash is the
  committed file, which was under review while still uncommitted.

The script checks the bundle to confirm which Splitter each build contains.

**Results on the fixed Splitter**, all passing, for each orientation:
- **Primary drag:** no selection starts, so no `selectstart` event fires, and nothing is
  selected during or after the drag, even past the clamp across text.
- **The drag itself:**
  - it resizes and clamps;
  - pointer capture is taken and released;
  - hovering back without a button does not resize.
- **Focus:** the press focuses the separator with no focus ring. This holds both when the
  field left behind was focused by keyboard and when it was focused by pointer.
- **Keyboard:** ArrowLeft or ArrowDown continues from the separator, now with the shell's
  keyboard ring.
- **Ordinary selection:** a drag that starts on paragraph text still selects it.
- **Secondary presses:**
  - a secondary-button drag neither resizes nor captures;
  - a Control-click drag neither resizes, captures nor starts a selection. Headless
    Chromium on macOS sends its `contextmenu`.
- **Lost capture:** releasing capture mid-drag ends the drag. A 2 px nudge that stays on the
  separator does not resize, and moving on across text selects nothing.

**Results in the editor shell:**
- **Inspector and dock separators:** neither starts a selection or selects anything, both
  resize to their minimum, and both take focus without a ring.
- **Open menu:** a separator press closes it.
- **Inspector text:** a drag from "Translation" to "Scale" still selects the labels.

**Results on the base-commit Splitter.** The 12 checks that tell the variants apart all
fail on it:
- every press starts a selection, with `selectstart` on both harness orientations and on
  the Inspector and dock separators;
- secondary-button and Control-click drags resize, and a Control-click starts a selection;
- after capture is lost, the drag keeps resizing, and the selection the press started
  spreads across the text.

**Chromium does not reproduce the visible symptom.** With the old Splitter, Chromium still
starts the selection on press. In this run, though, the selection stayed empty while the
pointer was captured, and spread only once capture ended. So the "before" drag captures
look clean. Native WebKit extends the selection during the drag itself.

So the browser review proves two things:
- the fix removes the selection's start;
- normal selection, focus and menu behaviour are kept.

The visible WebKit symptom itself still needs native acceptance.

## Screenshots

All are browser captures, explained in `screenshots/browser/README.md`:
- **Harness, both orientations, before and after:**
  - `screenshots/browser/harness-{vertical,horizontal}-{before,after}-drag.png`
  - `screenshots/browser/harness-{vertical,horizontal}-{before,after}-keyboard.png`
  - `screenshots/browser/harness-{vertical,horizontal}-{before,after}-text-selection.png`
  - `screenshots/browser/harness-{vertical,horizontal}-{before,after}-lost-capture.png`.
    The base-commit captures show text selected; the fixed ones show none.
- **Editor shell, before and after:**
  - `screenshots/browser/editor-{before,after}-inspector-drag.png`
  - `screenshots/browser/editor-{before,after}-dock-drag.png`
  - `screenshots/browser/editor-{before,after}-resized.png`
  - `screenshots/browser/editor-{before,after}-text-selection.png`

No native screenshots were taken, and none are claimed.

## Native acceptance (root)

**Limitation.** I did not launch the native editor. The packet reserves native runs and
the graphics slot for the root. The fix relies on standard behaviour: cancelling
`mousedown` cancels its default selection start in WebKit as it does in Chromium. That is
an expectation, not a measurement in WKWebView. Unknown until measured natively:
- how WKWebView treats `focusVisible: false`;
- WebKit's own `:focus-visible` heuristic after a pointer press.

**Checklist for the native window:**
1. **Drag across text:** drag the Inspector, output dock, hierarchy and agent separators
   past their clamps, across text. Nothing is selected. Compare with 0044's
   `defect-splitter-drag-selects-text.png`.
2. **Keyboard after a drag:** arrow keys resize the dragged separator.
   - The shell's keyboard ring appears only after a key press.
   - Note whether WebKit draws `:focus-visible` after a plain pointer drag.
3. **Secondary clicks:** a right-click or Control-click on a separator, then a mouse move,
   does not resize. This includes after the context menu closes.
4. **Open menu:** with a hierarchy context menu open, pressing a separator closes it.
5. **Ordinary selection:** a drag that starts on Inspector text still selects it, and copy
   works.

**Not tested in a real browser, recorded explicitly:**
- a real `pointercancel` (jsdom only);
- a lost release (jsdom only). A real one needs a native context menu, which headless
  Chromium does not open;
- touch and pen input (jsdom only);
- any WebKit engine: Playwright WebKit is not installed here, and I did not download it;
- Windows WebView2.

## Changed paths

**Edited:** `editor/ui/src/components/Splitter.tsx`.

**New:**
- `editor/ui/src/components/Splitter.test.tsx`;
- `handoffs/0047-splitter-input/result.md`;
- `handoffs/0047-splitter-input/tools/browser-review.mjs` and `probe-focus-visible.mjs`;
- `handoffs/0047-splitter-input/tools/harness/index.html`, `main.tsx` and `harness.css`;
- `handoffs/0047-splitter-input/evidence/`:
  - `vitest-splitter-before-fix.txt` and `vitest-splitter-after-fix.txt`;
  - `vitest-full.txt`, `typecheck.txt` and `build.txt`;
  - `browser-review.txt` and `browser-review.json`;
  - `probe-focus-visible.json` and `mutations.txt`;
- `handoffs/0047-splitter-input/screenshots/browser/`: 24 PNGs and a README.

**Not edited:**
- App, DESIGN, styles, Play, bridge and Agent files;
- every other path outside this packet.

**Local, ignored and not committed:**
- the `node_modules` symlink;
- `editor/ui/node_modules/` (build info and Vite caches);
- `editor/ui/dist/`;
- `handoffs/0047-splitter-input/artifacts/`: the four review builds and the base-commit
  copy.

## Review

An independent read-only review ran as a subagent of this session. It changed nothing; I
made and verified every change. Its own model and effort were not separately confirmed
through ACP. It found three problems, all fixed above:
1. **Test gap:** the ending tests did not assert another pointer's move. The assertion is
   added, and the "no pointer filter" mutation now fails.
2. **Stuck drag:** a drag could stick if its release was lost, a problem the base commit
   had too. This is now the lost-release guard.
3. **Control rule on other hosts:** the Control exclusion applied on every platform, so a
   Control-press on a non-macOS host started a selection. Every primary-button press now
   cancels the selection start; only the drag excludes Control.

From reading WebKit's source logic, not by running WKWebView, it confirmed:
- a cancelled `mousedown` stops WebKit from starting a selection or a drag;
- the explicit focus is needed.

It also found `focusVisible` among the macOS system WebKit's focus options. Whether
WebKit then hides the ring is measured only in Chromium.

## Open questions

1. **For the root:**
   - build a binary from this source and run the native checklist above;
   - integrate it with the pending budget UI.
2. **For the director:** none.
