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
  world-text filter (one bilinear tap up to 1.25 texels, then ≤4×4 taps), at
  the backend's filtering precision. Zero-coverage fragments are discarded, so
  pixels a quad does not cover keep their stored value on every backend.
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

GPU tests (`--include-ignored`) compare actual draws with an independent
CPU compositor written from this contract: 1x, 2x and 1.5x, UNORM and sRGB
targets, an offset editor region, nested clips with negative-x RTL overflow,
composition over a finished world frame (with an order-sensitivity check) and
retention/refusal identity. On Apple Metal every case agrees within 1 code
value of exact arithmetic. The reference admits, per channel, only the codes
the backend precision allows: 8-bit fixed-point filtering of glyph taps between
texel centres and the sRGB read-back and store between blends. A CPU emulation
of Mesa llvmpipe produces differences up to 7 codes from exact near black at
2x, where the sRGB curve magnifies a 1/255 coverage step. This is a candidate
explanation for the Linux failure: installed Mesa packages do not identify
the adapter that executed the failed test, and the corrected test has not yet
run on Linux. The reference admits this emulation while rejecting text placed
one pixel off, 90% text opacity and a missing ancestor clip (handoff 0057).
It requires at least 8 sub-texel bits; Vulkan's permitted minimum of 4 remains
outside this reference. A hosted composed game
drives routed Tab/Enter, receipts for focus, commit and preedit, refresh and
relayout through the real consumer. Handoff 0056 records the review fixture,
offscreen captures and logs.

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


## Current-main138 integration

All six original Claude commits retain their author trailers on the actual
main138 union. Nineteen of the 22 accepted source inputs are identical. The
three differences preserve current main's additive widget-checkpoint exports,
the existing native-module conversion in a Play test, and existing Cargo.lock
edges/Tao source alongside the reviewed GPU-host test dependencies. No new
renderer, shader, resource math or appearance change was made during integration.

[Current-main evidence](evidence/native-ui-draw-main138.json) retains frozen
inputs, exact integration seams and seven completed command logs. It records
206 CPU tests across 40 groups, with zero failures and 89 GPU cases explicitly
ignored, eight-package all-target Clippy, native-window/downstream compilation,
default wasm32 renderer compilation, dependency checks, formatting and conventions.
The actual-main ancestry merge followed terminal checks and preserved the tested
tree exactly. [Root's original review](evidence/native-ui-draw-root-review.json)
remains bound to the original frozen offscreen build and its Claude acceptance;
this is not a fresh current-main GPU or packaged-player execution.

The now-supported consumer also lets the existing desktop startup admit UI.
The separately reviewed reserved-frame correction and actual native window/IME
acceptance are still queued with Claude0057. Precise native caret placement,
HiDPI glyphs, hover, complete UI checkpoint composition, mobile and device gates
remain open. The earlier evidence and handoff limitations remain historical
records of their exact tested source.

[Independent integration closure](evidence/native-ui-draw-main138-root-review.json)
verified all 1,474 frozen inputs, the three additive seams, seven command logs and
the identical tested/ancestry tree with no actionable finding.

## 0057 integration correction

The original Claude commits are preserved. Their confirmed-Linux-backend wording
is superseded by the corrected candidate diagnosis above and in the handoff
result/JSON. Installed driver packages and numerical emulation do not identify
the adapter used by the failed hosted run. Test-only logging now prints actual
selected wgpu adapter information during the existing checkpoint tests. No
reference arithmetic, shader logic or appearance changed in this correction.

[Source review](evidence/native-ui-reference/native-ui-0057-source-20261012.json),
[provenance correction review](evidence/native-ui-reference/native-ui-0057-provenance-final-20261012.json)
and [adapter logging review](evidence/native-ui-reference/native-ui-adapter-logging-20261012.json)
retain the exact reviewed boundaries. Integrated CPU checks and actual Linux
checkpoint results remain pending.
