# Browser captures for handoff 0047

**These are browser evidence, not native evidence.** Every PNG is a capture from headless
Chromium 156.0.8078.4, launched by the pinned playwright-core 1.64.0 and driven with real
mouse and keyboard input. Nothing here was taken from the native WebKit window or the
user's desktop.

`tools/browser-review.mjs` wrote all 24 files in a single run, together with
`evidence/browser-review.json`.
- **"before"** is the Splitter from base commit `dc5e620`, the source the native review
  found the defect in. SHA-256 `d96979c2…29153`.
- **"after"** is this handoff's Splitter. SHA-256 `76d1fa85…40a9c`.
- **Harness pages** are 1280×800 at 1×. **Editor pages** are 1440×900 at 1×, using the
  sample fixture; the editor labels itself "Sample data".
- **Transitions are completed before each capture**, so the separator's focus colour is
  shown at its final value.

## Harness: the real Splitter between selectable panes

Each caption bar names the variant, the layout and the current value.
- **Vertical layout:** the left pane's width; invert is off; the range is 200–560.
- **Horizontal layout:** the bottom pane's height, as for the dock; invert is on; the range
  is 120–420.

| File | What it shows |
| --- | --- |
| `harness-{vertical,horizontal}-{before,after}-drag.png` | The button is held after a drag past the clamp. The pointer is over the other pane's text. |
| `harness-{vertical,horizontal}-{before,after}-keyboard.png` | After one arrow key following the drag. The separator keeps focus and shows the shell's keyboard ring, the periwinkle line. |
| `harness-{vertical,horizontal}-{before,after}-text-selection.png` | A drag that starts on paragraph text still selects that text. |
| `harness-{vertical,horizontal}-{before,after}-lost-capture.png` | The button is still held after the page released pointer capture and the pointer crossed the text. |

**Drag captures look the same in both variants.** Chromium's pointer capture stops a
selection from extending during the drag, so neither shows selected text. The old press
does still start a selection in Chromium: `browser-review.json` records a `selectstart`
event on every "before" press, and none on any "after" press. The `lost-capture` pair
shows what that selection does once capture ends:
- **before:** the started selection spreads across the pane's text, and the drag keeps
  resizing;
- **after:** nothing is selected, and the drag has stopped.

The native WebKit window shows the selection during an ordinary drag, as in handoff 0044's
`defect-splitter-drag-selects-text.png`.

## Editor shell (`?fixture=sample`, Player selected)

| File | What it shows |
| --- | --- |
| `editor-{before,after}-inspector-drag.png` | The Inspector separator mid-drag, held at the Inspector's 280 px minimum, with the pointer over the Inspector fields. |
| `editor-{before,after}-dock-drag.png` | The output dock separator mid-drag, held at the dock's 120 px minimum, with the pointer over the dock. |
| `editor-{before,after}-resized.png` | The editor after both drags. |
| `editor-{before,after}-text-selection.png` | A drag from the Inspector's "Translation" label to its "Scale" label still selects that text. |
