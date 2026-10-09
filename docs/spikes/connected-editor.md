# Connected editor integration

Date: 2026-10-08. Phase 0 follow-up, not phase approval.

Claude Opus 5.5 implemented handoff 0006 through ACP: connected panels with thin
separators, neutral graphite surfaces restored from the previous palette, and a
titlebar logo inset/alignment correction. The landing page received the same
palette direction through handoff 0007 in draft PR #2. The layout and typography
remain Claude-owned. The primary implementation is in draft PR #1.

The browser fixture measured a 15 pt logo-to-green-light gap and matched vertical
centers. Claude reviewed the corrected native captures taken before the director's
computer-use restriction, confirming the neutral frame, darker viewport clear,
flush panel boundaries and the logo's native position. A capture indicator hid the
actual traffic lights, so the latest light-to-logo alignment has not been fully
confirmed in native pixels. Raw captures with the real account label remain local.
See [Claude's result](../../handoffs/0006-connected-editor/result.md).

The signed-in account dialog now focuses Close when opened. Its sign-out
confirmation focuses Keep signed in, including after a mouse click in WebKit.
The regression test activates the focused controls and proves an accidental
Enter does not disconnect the account. Explicit confirmation still signs out.

## Native menu history

Native interaction before the capture restriction exposed a distinct bug: Tauri's
default Cocoa Undo entry consumed Cmd+Z without reaching project history. Removing
the entry made project shortcuts work but broke text-field Undo. That experiment
was replaced before integration.

On macOS the Edit menu retains Undo/Redo and their accelerators as custom actions.
They emit `incant:history-request` to the editor webview. The bridge validates the
payload and forwards intent to the UI. Focused editable controls use WebKit's own
text undo buffer through `document.execCommand('undo'|'redo')`; there is no fallback
to project history if text undo returns false. A modal blocks project history.
Otherwise the request uses the same capability checks, command bus and error
announcements as the keyboard and history controls. Empty history is a no-op.
Windows/Linux keep the existing DOM shortcut route.

`execCommand` is deprecated, but there is no standard replacement for accessing
the browser's text undo stack; [MDN documents this remaining use case](https://developer.mozilla.org/en-US/docs/Web/API/Document/execCommand).
The integration tests prove routing and modal isolation, not WebKit's actual text
buffer behavior. Native text and project Undo/Redo still need an interaction check
when the director permits computer use again. No capture or browser automation was
performed after that restriction.

## Verification

- 223 UI/bridge tests pass, including native menu payload filtering, unsubscribe,
  shared command dispatch, text-draft isolation, both modal guards, empty history
  and safe account focus. CSS-source mirroring tests were removed; meaningful
  contrast and behavioral checks remain. Claude's earlier 226-test result is
  retained as historical evidence rather than relabeled.
- All 48 workspace Rust behavior tests and five Python tool tests pass.
- Workspace Clippy with warnings denied, Rust formatting, strict TypeScript,
  production UI build, generated bridge/SDK checks and convention generation pass.
- `python3 tools/editor-dev.py` rebuilt the unsigned app at
  `artifacts/Incant.app`. It was not relaunched or visually inspected after the
  computer-use restriction. Existing saved-account storage was untouched.
- Source checks, desktop editor builds/readback, credential persistence and all
  six platform probes passed at 1045f4f before this UI/menu revision. These older
  results are not represented as CI validation of the new menu code.

Remaining native visual/interaction work is listed in
[native requests](../../handoffs/0006-connected-editor/native-requests.md) and is
explicitly on hold. The in-editor agent composer remains unavailable in Phase 0;
live agent execution uses the CLI. No later phase or release is claimed.
