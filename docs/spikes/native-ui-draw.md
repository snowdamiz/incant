# Native screen-space game UI drawing

The renderer now draws the native UI tree from `incant_ui::UiPaint` through the
ordinary consumer route: `NativeRenderHost` → `RenderInputConsumer` →
`GpuRenderConsumer` → `incant_render::UiLayer`. World `Text` keeps its separate
world-space pass. This is a working consumer, not a complete Game UI: native
window wiring, platform pointer/IME receipts, hover, HiDPI glyph rasterization,
accessibility and UI save state remain open (see the end).

## Data path and transaction

`UiPending::visual` / `UiRuntime::visual` (incant_ui) borrow the candidate's or
the accepted owner's focus, captured pointer press and TextInput buffers in
canonical layout order. The render-game binding joins that projection with the
exact `UiPaint` (layout, shaped glyphs, atlas handles) into
`incant_render_data::UiInput`, one `UiWidget` per text-bearing widget. Root and
Container draw nothing. An unchanged paint and widget state reuse the accepted
`Arc<UiInput>`; nothing is reshaped or inferred from text.

`RenderInputConsumer::prepare/accept` receive the candidate `UiInput` of the
same transaction, and `admit_ui` admits a viewport-only relayout without
presenting. The CPU owner publishes only after the consumer succeeds; a refusal
leaves the accepted owner, layout and input Arc unchanged (tested with a
refusing consumer). `GpuRenderConsumer` advertises `SUPPORTS_UI`, stages the UI
before it takes its live state or a target, submits the world frame, then the
UI over the same target, then presents. A refused stage never touches accepted
GPU resources.

## Numerical contract

Source of truth: `crates/incant_render/src/ui/mod.rs` and `theme.rs`.

- **Space.** Logical pixels, top-left origin, Y down. The target region (whole
  target, or `RetainedTarget::viewport`) shows the logical viewport at
  `region / viewport` physical pixels per logical pixel, per axis. Hosts keep
  the viewport at `physical size / scale factor`.
- **Snapping.** Rectangle and clip edges round to the nearest physical pixel.
  A glyph pen (content origin + shaped position) rounds to the nearest physical
  pixel; its padded coverage rectangle follows at `scale` pixels per texel. At
  1x every texel lands on one pixel unresampled; glyphs sit within 0.5 physical
  pixel of the layout's own positions. Layout and drawing use the same shaped
  positions, including Word overflow and RTL glyphs at negative x.
- **Clipping.** Each quad is intersected with its own clip in the vertex stage.
  Glyphs use the effective content clip (ancestors ∩ own content box) on their
  actual padded bounds; surfaces, selection, caret and focus ring use the
  visible border box (ancestors ∩ border box).
- **Order.** One instanced draw per frame in canonical preorder; per widget:
  surface, selection, glyphs (label shadow pass first), composition underline,
  caret, focus ring.
- **Colour.** Display-referred sRGB with straight alpha, never tone mapped.
  sRGB targets decode and blend premultiplied in linear light; UNORM targets
  blend the encoded values. Glyph texels are linear area coverage with the
  world-text filter (one bilinear tap up to 1.25 texels, then ≤4×4 taps).
- **States.** Surfaces are opaque. Disabled: flatter surface, muted text.
  Focus: 2 px inset accent ring. Captured pointer press: darker button. Caret,
  selection and composition draw only in a focused, enabled field from `UiEdit`
  byte ranges mapped through shaped clusters (downstream affinity at wraps and
  direction boundaries, per-glyph bidi edges, proportional split inside
  ligatures, cosmic-text paragraph rules for empty lines). An input-method
  cursor range is a double-weight underline; `None` hides the caret.

## Appearance

A restrained dark control family over the game, light text and one accent for
focus and selection. Text `#ECEEF2`; buttons `#2E323A`, pressed `#1F2228`,
disabled `#23262C` with 41% text alpha; fields `#15171C` with a 1 px `#646A77`
outline that marks an editable region; accent `#4C92FF`, selection the accent at
43%. Corner radius 4 px. Labels sit directly on the scene and get a hard 1 px
shadow at 59% black: invisible over dark scenes, legible over bright floors
(found in review). No glow, gradient, decorative edge or animation; the caret
does not blink (no presentation clock is involved). Measured WCAG contrast:
text 11.1:1 on buttons and 15.4:1 on fields, selected text 5.0:1, the accent
ring 4.2:1 against buttons, and the field outline 3.3:1 against the field and
3.36:1 against the dark backdrop (raised from 2.75:1 during review).

## Retention and resource bounds

| Resource | Size | Bound |
| --- | --- | --- |
| UI atlas prefix texture (R8, by atlas Arc) | referenced prefix | ≤ 4 MiB |
| Instance buffer (by `UiInput` Arc) | quads × 64 B | ≤ 169,984 quads (10.4 MiB) |
| Frame region uniform (per layer) | 48 B | rewritten only when the region changes |
| Accepted + candidate overlap during a change | 2 × (atlas + instances) | ≤ 28.8 MiB |
| Transient CPU quad list while staging | quads × 64 B | ≤ 10.4 MiB, dropped after upload |

These are GPU additions to the CPU owner's own bounds (two 4 MiB alpha planes,
1,024 widgets, 65,536 glyphs, 16,384 lines). The quad bound counts every glyph
plus its label shadow, one selection and one composition span per byte of the
focused field, and six surface/caret/ring quads per widget. The review fixture
measured 240 quads and 37,746 GPU bytes. Unchanged frames upload nothing; a
focus, press or selection change uploads one instance buffer; a text change
uploads one atlas prefix because the CPU owner re-rasterizes per paint. Debug
and `ci` builds are not performance evidence.

## Evidence

GPU tests (Metal, `--include-ignored`) compare actual draws with an independent
CPU compositor written from this contract: 1x, 2x and 1.5x, UNORM and sRGB
targets, an offset editor region, nested clips with negative-x RTL overflow,
composition over a finished world frame (with an order-sensitivity check) and
retention/refusal identity; every case agreed within 1 code value. A hosted
composed game drives routed Tab/Enter, receipts for focus, commit and preedit,
refresh and relayout through the real consumer. Handoff 0056 records the
review fixture, offscreen captures and logs.

## Open work and requests

- Native window: Astra's input lane owns `window/{input.rs,mod.rs}`. Required
  startup wiring: `UiSettings` from the surface size divided by the scale
  factor when the scene has UI, a relayout to the reserved surface's logical
  size before each presentation, and UI stats in the run report (handoff
  0056 includes a proposal patch). No native UI pixels or input are claimed.
- Hover: the owner has no pointer-motion state; a hover visual needs it.
- HiDPI text: the atlas is rasterized at logical size, so 2x glyphs are
  bilinear-magnified (soft), while surfaces stay crisp. Request: rasterize UI
  glyphs at `font_size × scale` (atlas key including the scale) and expose the
  raster scale on `UiPaint`; the consumer then maps texels at `1 / scale`.
- Text alignment inside widgets is authored only through padding and sizes;
  there is no content alignment for button labels.
- OS IME activation, preedit/cancel delivery, clipboard, accessibility, UI
  save/restore, phone and performance gates are not addressed here.
