# Prevent text selection while resizing editor panels

Claude Opus5.5 via ACP with explicitly verified Max. Read CLAUDE.md. This is a
small, bounded interaction fix reported by the actual native0044 review, now
explicitly assigned to you. No redesign, backend changes or main publication.

See handoffs/0044-native-editor-play/result.md, item3 under native defects, and its
native defect-splitter-drag-selects-text.png. Dragging a splitter currently selects
text across the Inspector. Splitter.tsx captures the pointer but leaves the
pointerdown default selection behavior in place. The proposed fix was not tested.

Own editor/ui/src/components/Splitter.tsx, its focused tests, and this packet.
You may read shared App/layout/test helpers but do not modify App, DESIGN, styles,
Play, bridge or Agent files; other Claude sessions own those. Use the smallest
sound fix: primary-pointer drag should suppress selection, preserve explicit
separator keyboard focus, clamp/respect axis/invert as before, and end cleanly
on pointer up/cancel/lost capture. Secondary clicks must not begin a drag.
Keep keyboard arrow/Home/End semantics and accessible separator values unchanged.
Do not add a global permanent user-select:none rule or disable text copying.

Add observable regressions for prevented selection initiation/focus, primary vs
secondary button behavior, drag movement and cleanup. Inspect actual browser drag
behavior with a surrounding selectable paragraph and both orientations; verify
text can still be selected normally outside a drag. Label browser evidence as such.
Use own headless browser only; root owns native GPU for live budget acceptance.
Do not launch native apps, inspect account state, make provider calls or capture
the user's desktop. No dependency upgrades. Pinned node_modules may be symlinked
from this repository parent. Use max4 Vitest workers and private review ports.

Run full UI tests, strict app/test TypeScript and build. Return result.md with
model/effort, exact tests, browser review evidence and native-acceptance limitation.
Commit only assigned changes with Built-by: claude. Finish promptly once this
bounded defect is solved; root will integrate it with the pending budget UI and
supply a matching immutable binary for final native review.
