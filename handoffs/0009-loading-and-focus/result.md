# Result: native keyboard focus and project-load states (handoff 0009)

**Status:** the UI work is complete and verified in the browser. Native WebKit verification is
**open**. It waits on Astra's CUA captures, listed in `native-requests.md`. No phase gate is
approved by this work, and nothing was pushed or merged.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) through Claude Code, the configured ACP Claude.
No other model was used.

## Director feedback acknowledged

- **Computer use and screen capture are allowed again.** I used only headless-Chrome browser
  fixture captures. My ACP session has no CUA. I made no native captures, used no native
  helpers, screencapture, CGWindow or AppleScript, and sent no synthetic native input.
- **No credentials or account actions.** Every account label in code, tests and images is
  synthetic.
- **The design is not restarted.** The neutral charcoal connected panels and restrained
  periwinkle accent are unchanged. The new states reuse the existing strip, state tile, spinner
  and pill. No site files were edited.
- **No unconnected retry button.** Reopening the project is the retry path, and the strip says so.
- **Astra's files are untouched.** I read `editor/bridge/native.ts` and
  `editor/app/src/project.rs` but did not edit them.

## 1. Keyboard focus ring on the account dialog

**Cause.** Handoff 0008 capture d05 showed that WebKit focuses Close after Enter but draws no
ring. WebKit does not match `:focus-visible` when script moves focus after a key press.
Chrome does.

**Fix.** The new `editor/ui/src/shell/inputModality.ts` tracks the last input modality:

- A key press without ⌘, Ctrl or Alt switches to keyboard modality. Modifier-only presses and
  shortcuts like ⌘Z do not.
- While in keyboard modality, the focused element gets `data-focus-visible`.
- A pointer press switches back and clears the attribute.

Listeners run in the capture phase, so the modality is known before React handlers move focus.
Every `:focus-visible` rule in `app.css` now has a `[data-focus-visible]` twin with the same
ring. The editor-wide install means any keyboard-driven script focus stays visible, including
F6 panel focus.

**Unchanged:**

- Initial focus: Close when signed in, and "Keep signed in" for the sign-out question.
- Escape behavior.
- Account auth, storage and sign-out semantics.
- The `AccountDialog` component itself.
- Pointer-open: Close is focused without a ring, as before.

## 2. Truthful project loading and failure states

`editor/ui/src/shell/projectState.ts` derives one state, opening, failed or ready, from the
connection the adapter publishes. Each panel uses it. A failure is shown in full once, in the
strip with an alert role and its code. Problems lists the engine diagnostic. Other panels give
one short line.

| Panel | Opening (before → after) | Failed (before → after) |
| --- | --- | --- |
| Titlebar | "Connecting…" → "Opening project…", muted | "Disconnected" → "Project not opened" for `project.*` codes |
| Strip | "Connecting to the editor process…" → "Opening project…" | "Lost connection…" plus "Nothing shown is current" → "The project could not be opened." with message and code |
| Hierarchy | Skeleton, unchanged | Full message repeated → "No project loaded", without a second alert |
| Viewport | "Not attached" → "Waiting" and "Opening project…" | "The viewport failed to attach" plus message → "No project loaded", pill stays "Error" |
| Problems | **"No problems"** → spinner, "Opening project…" | Diagnostic listed, unchanged. Without one: "Not validated" |
| History | **"0 of 0 applied", "No history yet"** with Undo/Redo → spinner tile | Same false claims → "No project loaded" |
| Inspector | "Nothing selected" → spinner, "Opening project…" | "Nothing selected" → "No project loaded" |
| Agent | Host reason, unchanged | Full message repeated → "The agent needs an open project." |
| Status bar | **"0 errors, 0 warnings"** → counts hidden, amber dot | Counts shown (1 error), unchanged |

Other details:

- The hidden "Project:" prefix is read only before a real project name, so screen readers
  never hear "Project: Project not opened".
- Only `project.*` codes say "could not be opened". Other codes keep "Lost connection to the
  editor process."
- "Reopen the project to try again." is added only when the engine message does not already
  say to reopen.
- A hierarchy-only error on a ready project still shows its full text in the panel.
- The ChatGPT chip keeps showing saved account metadata in every state.

**Fixtures.** I added `project-loading` and `project-error` fixture variants. They mirror
`snapshotFromEngine()` for engine `loading` and `error` responses. The `connection-error`
fixture now has the same shape as the adapter's error snapshot. Previously it showed a
hierarchy skeleton.

## Changed paths

- `editor/ui/src/shell/inputModality.ts` (new)
- `editor/ui/src/shell/projectState.ts` (new)
- `editor/ui/src/App.tsx`: modality install, strip and status bar
- `editor/ui/src/components/Titlebar.tsx`, `HierarchyPanel.tsx`, `ViewportPanel.tsx`,
  `BottomDock.tsx`, `AgentPanel.tsx`, `StateView.tsx`, `inspector/InspectorPanel.tsx`
- `editor/ui/src/styles/app.css`: focus twins and the muted pending titlebar label
- `editor/ui/src/bridge/fixture.ts`: the two new variants and the corrected `connection-error`
- `editor/ui/src/components/AccountDialog.test.tsx`: six focus tests
- `editor/ui/src/ProjectStates.test.tsx` (new): six state tests
- `editor/ui/DESIGN.md`: project states and keyboard modality
- `handoffs/0009-loading-and-focus/`: this file, `native-requests.md`, `tools/capture-editor.mjs`
  and `screenshots/`

## Commands and results

Run from the worktree root:

```
npm test --workspace editor/ui          # 8 files, 239 tests passed (baseline: 7 files, 225)
npm run typecheck --workspace editor/ui # tsc -b --noEmit, strict, no errors
npm run build --workspace editor/ui     # built, no errors
node handoffs/0009-loading-and-focus/tools/capture-editor.mjs before   # on the unmodified build
node handoffs/0009-loading-and-focus/tools/capture-editor.mjs after    # 23 captures, 0 page errors
```

The test count includes the 12 `editor/bridge/native.test.ts` tests, which the UI vitest
config runs.

**Regression tests of observable behavior:**

- **Keyboard open.** Enter on the chip focuses Close and marks it. Escape returns a marked
  focus to the chip.
- **Pointer open.** Close is focused, and nothing is marked.
- **Tab after a pointer open.** Shift+Tab through the dialog's trap marks only the newly
  focused control. A pointer press clears the mark.
- **Sign-out question.** By keyboard, "Keep signed in" is focused and marked. By pointer, it is
  focused and unmarked. No disconnect request is sent.
- **Shortcuts.** ⌘Z and Shift do not switch modality.
- **Stylesheet guard.** Every `:focus-visible` selector has a `[data-focus-visible]` twin.
- **Opening state.** No "No problems", "No history yet" or "0 of 0", no Undo in the dock, and no
  status-bar counts. The account label is visible, and axe finds no violations.
- **Failed state after a ready project with a selection.**
  - There is exactly one alert, with message and code.
  - The message text appears exactly twice, in the strip and in Problems.
  - No tree and no stale inspector are shown.
  - History shows no "applied" count, and the titlebar Undo is aria-disabled.
  - The account label is visible, and axe finds no violations.
- **Recovery.** A failed project, then an opening one, then a ready one restores the tree and
  "4 of 5 applied".
- **Other errors.** A lost-process error keeps its wording and gets the reopen hint. A
  hierarchy-only error keeps its full panel alert.

**Mutation check.** With the modality install disabled, three of the keyboard focus tests fail.
App.tsx was restored afterwards.

**F2 rename, project and text Undo routing, and account restoration** are covered by the
existing tests, which all pass unchanged.

**Bundle**, measured with the same build command before and after:

| Asset | Before (gzip) | After (gzip) |
| --- | --- | --- |
| JS | 299.65 kB (92.62 kB) | 303.37 kB (93.55 kB) |
| CSS | 43.76 kB (8.33 kB) | 44.19 kB (8.38 kB) |

No numeric gzip budget is defined for the editor UI in the repository. No fonts, icons or
dependencies were added or upgraded.

## Browser evidence (synthetic, headless Chrome 155.0.8059.40, DPR 2)

**Data sources.** The opening and failed states are an evidence test double. It copies the
adapter's snapshot shape, carries the real `project.timeout` message from `project.rs`, and uses
a synthetic "Sample Account". Ready and account states use `?fixture=sample&provider=signed-in-multi`.

**"webkit-sim" captures.** These rewrite every `:focus-visible` selector in the loaded
stylesheets so it never matches. That reproduces the d05 defect inside Chrome. **It is a
simulation, not WebKit.**

Sizes are 1000×650 and 1440×900. Measurements are from `screenshots/after/report.json`, and the
before run is in `screenshots/before/report.json`:

| Path (both sizes) | Before: outline on focused control | After |
| --- | --- | --- |
| Chrome, pointer-open | Close, none | Close, none |
| Chrome, keyboard-open | Close, 2 px `#c3c8ff` | Close, 2 px `#c3c8ff`, marked |
| webkit-sim, pointer-open | Close, none | Close, none |
| webkit-sim, keyboard-open | **Close, none** (the d05 defect) | **Close, 2 px `#c3c8ff`** |
| webkit-sim, keyboard sign-out question | **Keep signed in, none** | **Keep signed in, 2 px `#c3c8ff`** |
| Chrome, Tab inside the dialog (1440) | Next control, ringed | Next control, ringed and marked |

axe-core found no violations in the opening, failed and keyboard-open account states at
1440×900, before or after. There were no console errors.

**Pixel review.**

- **Panels.** Panel joins, header rule and colours are unchanged.
- **Opening state.** At 1000×650 the spinner tiles sit centred in the inspector and dock
  without clipping.
- **Failed state.** At 1000×650 the strip wraps the timeout message to two lines, and the code
  stays right-aligned.
- **Focus ring.** The keyboard ring on Close is the same 2 px periwinkle, 2 px offset ring as
  Chrome's native one.
- **Weakness.** The opening state still says "Opening project…" in five places: titlebar,
  strip, viewport, inspector and dock. Each one labels its own panel, so I kept them. I removed
  the status-bar copy to reduce the repetition.

**Screenshots** are in `screenshots/after/`:

- `01-project-loading-*`, `01b-project-loading-history-*`
- `02-project-failed-*`, `02b-project-failed-history-*`
- `03-project-ready-*`
- `04-account-pointer-open-*`, `05-account-keyboard-open-*`
- `05b-account-keyboard-tab-1440x900`
- `06-webkit-sim-account-pointer-open-*`, `07-webkit-sim-account-keyboard-open-*`
- `07c-webkit-sim-account-keyboard-open-detail-*`
- `08-webkit-sim-signout-question-keyboard-*`

`screenshots/before/` keeps only the defect images: the 01, 01b, 02, 02b, 06, 07 and 07c sets.
No native capture is committed.

## Limitations

- **Native WebKit is unverified.** The browser tests and webkit-sim captures show the marker
  works when `:focus-visible` fails. They do not prove WKWebView behavior. Requests N1 to N6
  are in `native-requests.md`. The Mac was locked at the last CUA attempt.
- **Modality heuristic.** Any non-modifier key switches to keyboard modality, including typing.
  This matches Chrome's documented heuristic. In WebKit, a dialog opened by a pointer click
  right after typing in a text field will not ring until a key is pressed.
- **No timeout capture.** The opening state was captured only with a static double. A real
  15-second timeout was not reproduced.

## Open questions

- Should "Opening project…" be shortened in some panels, such as the inspector tile, to reduce
  the five-fold repetition? I kept panel-local labels for screen-reader context.
