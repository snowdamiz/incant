# Native screen-space UI consumer handoff

Claude Opus 5.5 with explicitly verified Max owns presentation, shader/draw work
and rendered-pixel acceptance. This packet describes tested CPU ownership and
the remaining consumer seam; it does not approve an appearance or style.

## Canonical input

`incant_ui::UiPaint` retains a `LayoutFrame`, text placements and one immutable
`Arc<GlyphAtlas>`. `NativeRenderHost::ui()` and `ui_paint()` expose the accepted
owner/paint. `UiSettings.viewport` uses logical screen pixels, top-left origin,
Y down. World-space Text and camera projection are separate and unchanged.

Layout nodes are in canonical `(order, stable ID)` hierarchy preorder. Each
`UiTextPaint` binds its widget ID, content-box origin, effective ancestor clip,
actual shaped layout and glyph handles in shaping order. Raster coverage starts
at `paint.origin + glyph.position + atlas_entry.origin`; the last term is the
signed raster bearing. Glyph positions already include shaping offsets/baselines.
Do not reverse RTL glyphs or assume layout width bounds all positioned glyphs.
An unbreakable RTL-base Word token may start at a negative x. Use actual positioned
glyph bounds and the explicit clip. Preserve mixed-font baseline semantics.

Atlas data is row-major R8 alpha with stride equal to width (at most 2048 in the
default 4 MiB UI atlas). Entries identify the inner coverage rectangle; existing
atlas padding/empty glyph handling remains authoritative. A glyph handle belongs
to the exact atlas owner and never silently rebinds. `None` coverage for an empty
glyph is valid. Font keys and bytes are content-bound packaged data, not host fonts.

Button/TextInput currently contribute text only. Decoration, focus treatment,
selection/caret appearance, colors and pixel-ratio choices are not supplied here.
No new styling should change the typed layout/input transaction semantics.

## Consumer transaction that still needs implementation

`crates/incant_render_game/src/ui.rs::Binding::prepare` stages candidate layout,
fonts, edits and paint; `commit` publishes only after spatial consumption succeeds.
The current `RenderInputConsumer::consume` sees `NativeRenderInput`, while CPU UI
paint is retained beside it. A real graphics consumer must receive the complete
candidate UI alongside spatial input through this same prepare/consume/commit
boundary. Setting `SUPPORTS_UI` alone is insufficient. Startup, ordinary ticks,
structural reconcile and viewport-only refresh must all use the same canonical
screen-space input; failed GPU/atlas/spatial preparation must not publish half a
candidate. A viewport refresh advances no world, input or audio clock.

Retain old GPU buffers until candidate validation/uploads are ready. Bounds are
1,024 widgets, 65,536 glyph placements, 16,384 lines, one 4 MiB accepted plus one
4 MiB candidate UI atlas, in addition to separately bounded world Text. No-change
layout/paint reuse exact Arcs; do not reraster unchanged frames. CPU code allows
one individually bounded shaping result beyond its aggregate before refusal.
These are resource admission rules, not a whole RSS/device performance claim.

The existing GPU/window consumers explicitly refuse UI before submission. Remove
that refusal only as the actual consumer route and tests land. Integrate current
main's accepted world-Text draw and pinned bidi correction rather than replacing
their semantics. Parent owns integration of overlapping host publication seams.

## Observable fixtures and boundaries

`tools/probes/native-ui-export.py OUTPUT --binary /absolute/incant_headless`
authors through real import/RPC commands and runs the existing public exporter.
It exercises mixed world model/Text, native scripted motion, nonzero PCM,
keyboard/gamepad button activation, text composition/commit/submit and resized
pointer routing through the ordinary copied `--package` player. The first export
attempt and final-source proof status are separately recorded in CPU evidence.

Focused CPU regressions are in `incant_ui/tests/{layout,focus,reconcile,routing,
fonts}.rs` and `incant_render_game/tests/ui.rs`. They cover stale receipts,
1024-node navigation, captured-control handoff, malformed retry, failed accepted
ticks, aggregate paint refusal and old atlas/layout preservation.

Actual platform pointer/IME delivery, OS IME activation, clipboard, widget
accessibility and full UI save/restore are separate open work. Pointer receipts
must not also be forwarded as duplicate gameplay input. The viewport declaration
is not a new authored mutation path. No GPU or appearance proof accompanied this
CPU commit.
