# Canonical native text drawing (initial world-space route)

The renderer now draws the composed native text input that
[native-text-input.md](native-text-input.md) prepares. The same `SceneInput`,
`NativeRenderHost`, retained scene and `GpuRenderConsumer` serve the native
game window and editor Play; there is no text-only player, overlay or frame loop.
This draws literal shaped strings. It is not widget layout, editable text, IME,
focus or gamepad navigation, and it does not complete the Phase 1 Game UI gate.

## Coordinate and presentation contract

The numbers below live in `crates/incant_render/src/text/` and are exported as
`TEXT_LOGICAL_PIXELS_PER_UNIT`, `TEXT_COLOR` and `text_layout_to_local()`.

- **Units and placement.** `TextNode.world` is used unchanged as the entity's
  transform. Layout logical pixels map to that entity's local XY plane at 100
  pixels per local unit: layout `(x, y)`, Y down, is local `(x/100, -y/100, 0)`.
  The layout box's top-left corner is the entity origin, so line `i` has its
  baseline at local `y = -baseline_i / 100`. Translation, rotation, scale and
  shear then apply exactly as they do for sprites. No pixel-to-world conversion
  is inferred from the font size, and there is no billboarding or screen anchor.
- **Glyphs.** Each glyph's inner coverage rectangle starts at
  `position + origin` (the atlas entry's Y-down bearing from the glyph origin
  on its baseline) with the entry's texel width and height. The quad adds the
  entry's one-texel zero padding so filtered edges are complete; one layout
  pixel is one atlas texel. Empty outlines such as spaces draw nothing but keep
  their advance. Glyph zero draws its own `.notdef` outline. Shaped visual order,
  bidi placement and font fallback come from the layout; nothing is reversed or
  substituted again.
- **Depth and clipping.** Quads are two-sided (mirrored when seen from behind),
  depth-tested `LessEqual` against the opaque scene and do not write depth. Text
  draws after sprites, in stable-ID node order, then glyph order. There is no
  widget clip rectangle: every shaped line is drawn; only the view frustum and
  depth clip. Transparent model materials drawn earlier do not occlude text.
- **Coverage and color.** The 8-bit atlas holds linear area coverage. It
  multiplies unlit linear Rec. 709 radiance `TEXT_COLOR = (1, 1, 1)`, the value
  of an untinted white unlit sprite texel, and blends premultiplied over the HDR
  scene before the shared tone map (radiance 1.0 displays as linear 0.9, sRGB
  243). Authored text has no color field yet; the color is renderer policy.
- **Filtering.** Bilinear at up to 1.25 atlas texels per screen pixel. Beyond
  that, an N×N grid of bilinear taps (N = ceil(footprint − 0.25), at most 4)
  averages the pixel's footprint. Taps are clamped to the glyph's own padded
  rectangle, so neighbouring atlas glyphs never bleed in. There are no mipmaps,
  so stronger minification still aliases, and magnified text is soft because
  coverage is rasterized once at 1 texel per logical pixel.

## Resolution, retention and publication

`scene::resolve` replaces the old refusal with a counted plan: at most 1,024
nodes in strictly increasing stable-ID order and 65,536 glyph handles in total;
one handle per shaped glyph; finite affine transforms. Every handle must
resolve with `input.atlas.get(handle)` on this exact atlas, so a handle from
another atlas or from before `GlyphAtlas::reset` refuses with
`SceneError::TextGlyph` before any GPU allocation or quad expansion. Device
limits are checked before the scene uploads anything else.

Immutable GPU text resources are keyed by identity, never by revision number:
the exact atlas `Arc` plus every node's ID, layout `Arc` and glyph-handle `Arc`.
Holding the atlas `Arc` keeps it immutable (mutation needs unique ownership)
and its pointer unique while the resource lives.

- The atlas texture is the referenced prefix only (`R8Unorm`, width/height up to
  the farthest padded rectangle), uploaded once per atlas identity and shared by
  every scene using that atlas.
- Glyph quads (32 bytes each) are uploaded once per atlas/node-set identity and
  shared by the accepted and staged retained frames, by a topology rebuild that
  reuses the adapter's layouts, and by fresh reference scenes.
- Motion stages one 256-byte transform slot per node (entity world × layout map
  in f64, then f32) into a preallocated row, checks that the node's transformed
  bounds stay finite, and writes one buffer. It allocates nothing and never
  reshapes, rerasterizes, rereads fonts, rebuilds quads or uploads the atlas.
  `Renderer::text_uploads()` counts atlas and quad uploads for these checks.
- A retained frame refuses a changed identity, a missing or added text binding,
  or a non-finite transform during staging, before any buffer write; the accepted
  frame and its GPU buffers stay unchanged and the call is retryable. Submission,
  presentation and device loss keep the existing terminal host contract.

### Resource accounting per text scene

| Resource | Bytes | Bound |
| --- | --- | --- |
| Atlas prefix texture (shared by identity) | prefix width × height | ≤ 4 MiB (2048²) |
| Glyph quad buffer (shared by identity) | quads × 32 | ≤ 2 MiB (65,536 quads) |
| Node transforms, accepted + staged | 2 × nodes × 256 | ≤ 512 KiB |
| CPU staging rows, accepted + staged | 2 × nodes × 256 | ≤ 512 KiB |
| Transient expanded quads at upload | quads × 32 | ≤ 2 MiB, dropped after upload |
| Per-frame camera/color uniform | 80 + bind group | one per drawn frame |

During a text change the accepted scene and the candidate coexist until the
swap, so text GPU memory peaks at old + candidate (at most about 13 MiB), plus
the transient quad vector and wgpu's own upload staging. These are additional
to the CPU atlas planes and layouts counted in native-text-input.md (old +
candidate alpha planes up to 8 MiB). `SceneStats.text_gpu_bytes` reports the
actual atlas prefix, quad and transform bytes of a scene; it excludes driver
overhead. No device memory or performance gate is inferred.

## Entry points

The native window route now accepts `--font-cache FONT_CACHE` (between the camera
and `--stop-after`), and `--package ... --window` passes the package's `fonts`
directory, as the offline route already did. Editor Play already supplied the
project's checked font cache. Worlds without a font catalog keep the strict
spatial-only adapter; text there still refuses with `TextUnsupported`, whose
message now names the missing font binding instead of missing rendering.

## Verification (local macOS, Apple M5 Pro, Metal, `ci` profile)

GPU checks are `#[ignore]` tests run locally with `--include-ignored` under the
0054 native/GPU grant; desktop CI workflows run them the same way.

- **Draw placement** (`text_draw_tests.rs`). Real shaped `ffi سلام 中文 ש`
  (Noto Sans, Noto Sans Arabic, the CJK fixture; Hebrew is uncovered) is drawn by
  the actual text pass and read back from an HDR target. An independent CPU
  composition writes the contract out (local = layout × (0.01, −0.01), padded
  `position + origin` rectangles, the tap rule, premultiplied over) and samples
  the CPU atlas. Max alpha error: 0.00135 at 1:1 (pixel-perfect camera at 100 px
  per unit), 0.00201 minified at 0.42× with rotation (3×3 taps), 0.00227 at
  0.24× (4×4 taps), 0.00324 magnified 1.7× with rotation. Independent landmarks: the ligature's ink ends
  on the layout baseline row (±1 px), CJK ink lies right of the RTL word, and the
  `.notdef` box is drawn. The same frame through the canonical graph and display
  transform matches `tone_map(1.0) = 0.9` over the backdrop within 1 code value.
- **Retention** (`text_retained_tests.rs`). Eight moving frames (entity
  rotate/scale/translate and camera motion) are byte-identical to fresh frames,
  stage with zero allocations, keep the same staging rows and upload no text.
  A same-identity rebuild shares the geometry with no uploads; a same-atlas
  subset reuses the atlas texture and uploads one quad buffer; changed text
  uploads exactly one atlas prefix and one quad buffer. Equal content in a new
  layout `Arc`, a missing text binding and an invalid target are refused without
  changing the accepted frame, which then retries successfully. Three resized
  targets match fresh frames. Foreign and reset handles refuse before any upload.
- **Depth** — opaque model pixels are byte-identical with text placed behind the
  model, text in front changes them, and text turned 180° stays visible.
- **Native adapter** (`text_native_tests.rs`). A source-deleted world with cooked
  fonts runs through `NativeRenderInput::try_sync_with_text`: motion frames match
  fresh frames without uploads, a `SetParent` topology rebuild shares geometry
  with no uploads, `SetText` uploads once, a refused consumer frame keeps the
  tick and the accepted pixels.
- **Hosted game** (`incant_render_gpu_host/tests/game.rs`). A text child of the
  scripted, physics-moved model in a QuickJS + audible PCM game draws through
  `GpuRenderConsumer` for eight ticks with the same atlas, no uploads and
  constant text accounting; frames differ from the same game without text.
- **Mutation checks** (`handoffs/0054-native-text-draw/tools/mutate.py`). Each
  of eight single mutations failed at least one test: quad offset by its
  padding, unflipped Y, unstaged motion transforms, atlas prefix missing its
  padding column, single tap when minified, accepted missing text binding,
  depth test disabled and unchecked foreign handles.

## Native rendered review (Claude, local macOS)

A disposable fixture authored only through `init`, `import` and one
`command.execute` transaction was exported with
`tools/export_native.py --mode window`. The shipping player
(`a5d6a9bd…`, fat LTO, aarch64-apple-darwin, unsigned) ran through
`--package data --window` on Apple M5 Pro / Metal for 4,073 ticks, one frame per
tick. Three window captures cover the startup size, the window's maximum size
(960×540 pt) and the smallest frame macOS allows (content 174×16 px). Latin
ligatures, right-aligned Arabic, CJK fallback, spaces, `.notdef`, the label on a
rotating lit cube and depth occlusion by the cube draw as specified. Measured
text plateaus are sRGB 243 over the (20, 21, 25) backdrop. Magnified text is
soft; minimum-size text collapses to specks beyond the 4-tap filter. Captures,
crops, identities and the full review are in
`handoffs/0054-native-text-draw/result.md`. No phone, shipping, memory or
frame-pacing gate is inferred from this desktop run.

The review also exposed a foundation layout defect (pinned cosmic-text 0.14.2
via `incant_text`). With `Word`/`WordOrGlyph` wrapping, a line with an RTL run
followed by LTR text drops the space after the RTL run and exceeds the declared
width: `Wrapped ffi سلام 中文 ש and spaces` at 300 px yields a 350.8 px first
line ending `שand`. The renderer draws that layout faithfully. The repair belongs
to the shaping layer (see the result's proposed narrow fix).

Outstanding Game UI work: widget layout and screen-space anchoring with device
pixel ratio, styling (color, outline, clipping), editable text/IME,
focus/gamepad navigation, emoji/color fonts, localization-driven text and
reference-phone measurements.

## Current-main integration

The original Claude source/evidence commits are preserved with actual main123.
The merge is automatic: six existing files also retain main's opt-in cooked shader
path and crossfade fields/undo/static-model refusals; the other sixteen reviewed
source/test files are byte-identical. The subsequent actual-main ancestry merge
leaves the tested tree unchanged.

Integrated CPU validation passes 127 tests in 28 groups, with 82 GPU tests ignored,
all-target Clippy, wasm32 render/native checks, cooked-geometry compilation, normal
runtime dependency guards, formatting and conventions. The exact observation
routing regression that failed on the older Claude base now passes. Its original
failed log remains in the handoff. No additional graphics run or appearance claim
is inferred: original Metal and Claude evidence above stays bound to that frozen
source/binary. The existing bidi layout defect remains a separate follow-up.

[Integrated evidence](evidence/native-text-draw-main123.json) and
[independent source review](evidence/native-text-draw-root-review.json) retain the
source/log identities, merged seams and explicit limits.
