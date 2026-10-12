# Native game UI reserved-frame sizing

The managed desktop input lane and reviewed screen-space GPU consumer now compose
in the same native player. Before each simulation or refresh presentation, the
window host compares the UI viewport with the dimensions of the actual reserved
surface frame at the window scale. A change fences pointer receipts from the
previous displayed layout, admits the replacement layout, and publishes new hit
coordinates only after the frame is consumed. Text receipts retain their separate
focus ownership. Unchanged layouts reuse their resources.

Invalid scales and logical dimensions fail explicitly instead of silently
stretching layout independently of pointer coordinates. A missing reservation
cannot expose an old configured size. Native reports include committed viewport,
focus, draw counts and upload counters without recording field text.

The two-file change passed independent source review and the affected player,
UI, input and render-host CPU tests, player all-target Clippy, dependency closure,
formatting and conventions checks. Exact counts, source hashes, retained logs and
review are in [the evidence](evidence/native-window-ui-reserved.json).

This evidence is local CPU verification. Claude0057 will build the exact combined
source and inspect a real native game window. Native IME interaction, candidate
caret placement, high-density glyph rasterization and device performance are not
claimed by these tests. This does not complete Game UI or Phase1.
