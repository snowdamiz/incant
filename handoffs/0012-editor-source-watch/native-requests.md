# Native capture requests: Problems row wrapping

Requested from Astra, who alone performs native CUA captures. Build the release app from
the commit that contains this file, with the same saved project and fixture as w08 to
w12. Keep the captures private in `artifacts/native-source-watch-review` and do not
commit them. Browser fixture captures in `screenshots/after` are not native evidence.

All captures are full-window JPEG or PNG at scale 1. Report the measured window size.

## Required

1. **w13-wrap-error-minimum.** Window 1000×650. Problems tab active. Write the same
   malformed `triangle.gltf` used for w11.
   - Expected: the message wraps onto a second line and ends with
     "expected value at line 1 column 1". No ellipsis.
   - Expected: `triangle.gltf` stays in the mono right-hand column, top-aligned with the
     first message line. The red icon sits on the first line.
   - Expected: no horizontal scroll bar in the dock. The viewport stays Attached with the
     last valid geometry.
2. **w14-wrap-error-wide.** Window 1440×900, same error and tab.
   - Expected: the row is one line and 28px tall, as in w10. The path stays at the right.

## Optional

3. **w15-wrap-recovered-minimum.** Window 1000×650 after repairing the source.
   - Expected: "No problems" empty state as in w12. This checks the change did not alter
     the empty state.
4. **w16-wrap-keyboard-minimum.** Window 1000×650 with the error present. Press F6 until
   the dock has focus, then Tab into the Problems list. The project-level source row is
   not a button, so focus should skip it. Capture only if an entity diagnostic is
   available in the project; then capture its focus ring around the full wrapped row.
