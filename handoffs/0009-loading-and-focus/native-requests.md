# Native capture requests for handoff 0009 (for Astra, through Codex CUA)

Claude's ACP session has no computer use. Every request below needs Astra's CUA channel on a
build that includes this handoff's commit. Keep raw captures in the ignored `artifacts/`
tree. Do not commit them, and keep real account labels out of Git. The Mac was locked at the
last CUA attempt, so it must be unlocked first.

Native verification stays **open** until these captures are supplied and reviewed. The
browser captures in `screenshots/` are headless Chrome over synthetic data. They do not prove
WebKit behavior.

| ID | Check | Status |
| --- | --- | --- |
| N1 | Keyboard-opened account dialog rings Close (replaces 0008 D8) | Open |
| N2 | Pointer-opened account dialog has no ring on Close | Open |
| N3 | Keyboard sign-out question rings "Keep signed in" | Open |
| N4 | Project opening state | Open |
| N5 | Project failed state | Open |
| N6 | Ready state after reopening, with F2 and Undo routing intact | Open |

## N1. Keyboard-opened account dialog: `n01-account-keyboard-ring`

1. Click once in an empty area of the viewport, so the last input is a pointer press.
2. Press Tab until the ChatGPT chip in the titlebar has focus. It should show a ring.
3. Press Enter and capture.
4. Expected: the dialog is open and Close has a 2 px periwinkle ring.
5. Press Tab once and capture `n01b-account-keyboard-tab`. The ring should move to the next
   control.
6. Press Escape. Focus should return to the chip with a ring.

## N2. Pointer-opened account dialog: `n02-account-pointer-open`

Click the ChatGPT chip with the pointer and capture. Expected: the dialog opens and focus is
on Close, but no ring is drawn. This matches the earlier native capture 04. Press Escape.

## N3. Keyboard sign-out question: `n03-signout-question-keyboard`

This needs a signed-in account. Open the dialog with the keyboard as in N1. Tab to "Sign out"
and press Enter once. Capture. Expected: the question is shown, and "Keep signed in" has the
ring. Press Escape twice. **Do not press Enter again, so no sign-out happens.**

## N4. Opening state: `n04-project-opening`

Capture while a project is still opening. If the load is too fast to catch, skip this item and
say so. Do not slow it down with a native helper. Expected:

- The titlebar and the strip under it read "Opening project…".
- The hierarchy shows a skeleton.
- The inspector, Problems and History show a spinner tile and never "No problems" or "0 of 0".
- The viewport pill reads "Waiting".
- The ChatGPT chip still shows the saved account state.

## N5. Failed state: `n05-project-failed`

Use a deliberately invalid copy of a sample project in a temporary folder, for example one
with a truncated document. Open it and capture. Then click the History tab and capture
`n05b-project-failed-history`. Expected:

- One red strip reads "The project could not be opened.", followed by the engine message and
  its `project.*` code.
- The hierarchy, viewport, inspector and History each say "No project loaded" in one line.
- Problems lists the engine diagnostic once, and the tab count is 1.
- No entity, value or history entry from an earlier project is visible.
- The account chip is unchanged.

If a timeout can be reproduced naturally, capture `n05c-project-timeout` too. Do not
fabricate one.

## N6. Ready after reopening: `n06-project-ready`

Reopen a valid project and capture. Expected: the strip is gone and the hierarchy is shown.
Repeat the 0008 checks briefly. F2 should open with the name selected. Cmd+Z in a rename draft
should undo text only, and Cmd+Z outside it should undo the project. Report the results in
words. A capture is needed only if something differs.
