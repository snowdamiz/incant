# Canonical text draw integration handoff

For Claude Opus 5.5 with thinking explicitly verified as Max. This is technical
input for the next implementation/review packet, not approval of presentation.
No screenshot, draw, shader, visual design or appearance work was performed in
the CPU input slice. Coordinate the actual GPU/native slot with the root before
launching any renderer process.

## Existing owned input

`crates/incant_render_data/src/text.rs` defines `SceneInput.text: Option<TextInput>`.
The input owns one `Arc<GlyphAtlas>` and a bounded, stable-ID-sorted vector of
`TextNode`. Each node has its native entity ID, committed `DMat4` global transform,
`Arc<ShapedText>`, and `Arc<[GlyphHandle]>` with exactly one handle per shaped glyph.
No project/document/source font lookup belongs in the draw path.

`ShapedText` in `incant_text/src/layout.rs` exposes immutable glyph and line arrays.
Glyph `position` is the logical glyph origin including shaping offsets and the
line baseline. Layout coordinates use logical pixels with Y down. Source ranges
are UTF-8 byte clusters, and bidi levels already describe shaped visual runs;
do not reverse Arabic glyphs again. The layout includes all lines, not a clipped
viewport. Font size, line height and width are logical pixel metrics, not meters
or physical framebuffer pixels. There is no baked device pixel ratio.

`TextNode.world` is the unchanged native entity hierarchy transform in the same
game-world units as other native spatial nodes. It does **not** itself define a
pixel-to-world conversion, screen anchoring, widget plane, billboarding, clipping,
depth, color, alpha blending or hit testing. The renderer must give the initial
supported draw route an explicit coordinate contract and numerical tests rather
than infer a silent conversion from the font size. Future flexbox/widgets/IME
remain a separate layer and must not be claimed by drawing literal text.

Resolve each glyph handle with this exact `input.atlas.get(handle)`; a foreign or
reset handle returns `None` and must refuse preparation. `AtlasEntry.rect` is the
inner `[x,y,width,height]` coverage rectangle; one zero texel of padding surrounds
it. `origin` is its signed top-left bearing relative to the glyph baseline, Y
down. Thus the glyph-local coverage rectangle starts at `glyph.position + origin`
and has the returned pixel width/height, before the draw route's explicit unit
conversion. `rect == None` is a valid empty outline (including spaces), not an
error or a quad. Glyph zero remains observable `.notdef`; do not conceal it.

Atlas pixels are tightly packed, row-major unsigned 8-bit coverage, stride equal
to `atlas.size()[0]`, currently at most 2048×2048. There are no RGB values, mipmaps,
subpixel phases, hinting or color emoji. New atlas identity owns all its handles;
`revision()` alone is not globally unique. Accepted input holds the atlas immutable.

## Canonical integration points and publication

Current explicit text refusals are in `incant_render/src/scene.rs::resolve`,
`Renderer::prepare_retained`, and `RetainedScene::stage` in `retained.rs`. Replace
them only when the same canonical consumer actually resolves, uploads and draws
the supported data. Do not create another player or side renderer. The existing
`incant_render_gpu_host` uses the same mandatory `RenderInputConsumer` as native
game and editor Play. Renderer/device construction by callers is separate from
the current refusal-before-submission guarantee.

`incant_render_native/src/text.rs` prepares shape/atlas data at startup or changed
text generation. Spatial-only topology changes reuse immutable layout/atlas Arcs.
Ordinary motion updates only preallocated numeric transforms; the consumer must
not reshape, rerasterize, reread fonts, rebuild unchanged glyph geometry or upload
an unchanged atlas each frame. `UpdateKind::Rebuilt` can also be a spatial change,
so compare actual retained identities to avoid unnecessary text uploads.

The adapter invokes the consumer with a complete candidate. Rejection or unwind
preserves its old text, spatial state, tick and generations together. GPU effects
cannot be rolled back: follow existing preflight/encode/submit ordering, and make
post-submission failures terminal through the existing host contract. Do not
publish half a text/spatial candidate or expose mutable/unsubmitted GPU resources.
Keep both old and candidate budgets explicit before new GPU allocation or geometry
expansion. See `native-text-input.md` for precise CPU overlap limits; 4 MiB bounds
the alpha plane, not an entire atlas's maps, handles and raster scratch.

## Reusable controls and fixture

`incant_render_native/tests/text_input.rs` covers source/cache deletion, retained
Arc reuse, movement, topology, rejected preparation, consumer error/unwind,
despawn/reuse and save invalidation. `tests/allocations.rs` measures only adapter
motion sync. `incant_render_game/tests/hosted.rs` has an actual mixed
model/QuickJS/audio/Latin-Arabic-CJK fixture. Preserve their CPU guarantees and add
bounded numerical GPU coverage before Claude's rendered appearance acceptance.

`tools/probes/native-text-export.py OUTPUT --binary HEADLESS` creates the fixture
through public imports and reversible command transactions, runs the ordinary
fresh exporter, removes private authoring/build inputs, and launches the actual
CPU `--package` player. It is not a graphics launcher. The literal string is
`ffi سلام 中文`, with explicit packaged Noto Sans, Noto Sans Arabic and the small
licensed CJK test subset. These fixtures are numerical test data, not game art.
Use a separate disposable fixture for rendered review; never mutate preserved
proof packages or claim the CPU receipt establishes appearance.

Next acceptance needs actual canonical text draw, correct baseline/bearing/RTL
placement, space/missing behavior, retained motion/rebuild/refusal controls,
mixed spatial/audio game operation and Claude pixel review. It still does not
complete widget layout, IME, input focus/gamepad navigation, emoji fallback,
reference-phone memory/performance, or the complete Phase 1 Game UI gate.
