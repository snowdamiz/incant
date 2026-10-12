# Native text editing and composition state

`incant_text::editing::TextBuffer` is a bounded, disposable game-widget buffer.
It implements committed text, directional selection, logical grapheme movement,
backspace/forward deletion, and a transient IME preedit view. It is independent of
project documents and never writes authoring content. Authoring changes continue
to require `incant_cmd`.

All positions are UTF-8 byte offsets. Ordinary selection and deletion respect
extended grapheme boundaries using the existing pinned `unicode-segmentation`
1.13.3 resolution. A combining sequence, emoji ZWJ sequence, flag or CRLF is one
editing unit. Edits preserve exact input bytes and do not implicitly normalize
Unicode or line endings. Joining neighboring clusters after an edit moves the
caret to the next valid boundary; external invalid selections are refused.
Previous/next are logical string operations, not visual left/right in bidi text.

Preedit replaces the selected range only in `display_text()`. The committed
`text()` and original selection stay unchanged through repeated preedits.
Composition metadata exposes its replacement range, display span and optional
relative cursor range. IME cursor endpoints may lie between scalars within a
grapheme; they must still be valid UTF-8 boundaries. A hidden cursor is explicit.
Commit atomically replaces the original selection once; empty preedit clears the
preview without deleting the selection, whereas empty commit deletes it.

Focus loss cancels composition without committing it. Unfocused user input is
refused. During composition, ordinary navigation/deletion refuses input so an
adapter cannot accidentally consume keys owned by the input method. A valid
programmatic selection or text replacement cancels composition. Invalid changes
preserve all prior state, including an active preedit.

Committed and composed display text each use the shaper's 16 KiB byte cap.
Replacement size is checked before allocating. Persistent string payload is at
most 32 KiB; an in-progress replacement can temporarily retain a third bounded
string (48 KiB total), excluding allocator overhead and caller/shaper storage.
Reads borrow storage. Repeating the identical preedit reuses the display string;
the existing shaper also retains the same shaped result for identical text.
Operations are bounded by buffer length; these are not per-tick world snapshots.

The 12 new behavioral tests exercise combined Unicode deletion, reversed
selection, logical RTL navigation, insertion/deletion that merges clusters,
repeated preedit, clear-before-commit delivery, cancellation, focus loss, invalid
cursor/selection admission and replacement at the exact byte cap. A real
packaged-font test shapes the composed Latin/CJK display and verifies source
clusters and retained shaping identity before and after commit. All 37 text
tests pass, alongside all-target Clippy, wasm32 compilation, formatting,
convention and authoring-free dependency checks. Exact source/log identities
are recorded in [the evidence](evidence/native-text-editing.json).

This is the editing state layer, not a completed input widget or live IME gate.
The widget tree/layout, appearance, glyph hit testing and visual bidi caret
movement, OS IME activation/candidate positioning, clipboard, desktop event
delivery and mobile keyboard remain to be connected. A platform adapter must
suppress duplicate key text during composition and fence delayed events across
focus-owner changes. Synthetic events here do not establish those OS behaviors.
Desktop IME and real-device on-screen keyboard checks remain explicit Phase 1
requirements; no performance or appearance gate is claimed.
