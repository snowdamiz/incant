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
  visible border box (ancestors ∩ border box). A label halo with an m-texel
  margin (m = ⌈r⌉ for radius r) may extend m + 1 logical pixels past the text
  clip but never past the visible border box, and is empty where the text clip
  is empty; a label with that much padding keeps its whole halo, and a glyph
  the text clip cuts keeps that much of its hidden part's halo.
- **Order.** One instanced draw per frame in canonical preorder; per widget:
  surface, selection, glyphs (label halo pass first), composition underline,
  caret, focus ring.
- **Colour.** Display-referred sRGB with straight alpha, never tone mapped.
  sRGB targets decode and blend premultiplied in linear light; UNORM targets
  blend the encoded values. Glyph texels are linear area coverage with the
  world-text filter (one bilinear tap up to 1.25 texels, then ≤4×4 taps), at
  the backend's filtering precision. A label halo is its own prepared atlas
  entry (below) drawn through exactly this glyph path; the shader has no halo
  code. Blending precision is also the backend's: a UNORM target may blend at
  its own 8-bit precision. Zero-coverage fragments are discarded, so pixels a
  quad does not cover keep their stored value on every backend.
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
43%. Corner radius 4 px. Labels sit directly on the scene, so every label glyph
first draws an opaque `#0A0C10` halo: its own coverage dilated by 1/16 of its
font size, between 1 and 2.5 px. It nearly disappears over dark scenes and
separates every stroke from bright ones; text measures 16.8:1 against it, and
being opaque, overlapping halos between letters do not darken twice. It
replaced a hard 1 px shadow at 59% black (found in review) after result titles
over a bright horizon measured below 3:1 (handoff 0065). No glow, gradient,
decorative edge or animation; the caret does not blink (no presentation clock
is involved). Measured WCAG contrast:
text 11.1:1 on buttons and 15.4:1 on fields, selected text 5.0:1, the accent
ring 4.2:1 against buttons, and the field outline 3.3:1 against the field and
3.36:1 against the dark backdrop (raised from 2.75:1 during review).

## Label halo preparation

The halo is prepared with the glyph raster data, never computed while drawing.
When `UiPaint::prepare` builds a paint it rasterizes every glyph into that
paint's atlas, as before, and also caches one halo entry per label glyph
(`TextEngine::cache_halo`): the glyph's coverage dilated on the CPU at the
radius `incant_text::HaloRule` gives its font size. The game binding passes
`incant_render_data::UI_LABEL_HALO` (1/16 em within 1–2.5 px), the rule the
renderer theme re-exports. A halo entry is keyed by the glyph's raster key plus
the radius bits (`GlyphKey::halo_radius_bits`, zero for ordinary coverage), is
⌈r⌉ texels larger on every side with its origin moved by that margin (also for
an empty outline), and leaves the glyph's own entry byte-identical. The
renderer draws it as one ordinary glyph quad under its glyph, and refuses a
label without halos, a halo of another glyph or radius, halos on another role,
or a halo entry passed as a glyph.

Each halo texel is the largest `clamp(r + c − d, 0, 1)` over the glyph texels of
coverage c > 0 at centre distance d: a straight edge through a half-covered
texel grows by exactly r, and the halo is never below the glyph's own coverage.
The arithmetic is bounded, plain f32: at most 81 candidate texels per halo
texel, square roots of integers up to 32, one division by 255, additions, a
clamp and a final rounding to 8 bits. The oracles are local. `incant_text` unit
tests check exact cases (a single texel, a half-covered edge, growth with the
radius, empty input, repeated preparation in one process), and a renderer CPU
test recomputes every texel of real Latin, CJK and Arabic halos at 12–40 px
independently in f64 over all glyph texels, accepting at most one code of
difference, and finds nothing outside the margin. Whether every platform
prepares identical bytes has not been verified by execution.

Cost:

- GPU: one more glyph quad per label glyph with the same filter as its glyph
  (one bilinear tap at 1x, at most 16 when minified). The earlier draft's 25
  taps per filter tap, up to 400 texture reads per fragment, are gone.
  `ui.wgsl` is byte-identical to main (SHA-256 `f67e4982…`), so the cooked UI
  shader, its digest check and the instance layout are unchanged.
- CPU: only when a paint is rebuilt, never for a reused one. A paint is rebuilt
  when the layout candidate changes: the first bind, a viewport change, changed
  text content (such as a HUD binding) or reconciled widgets. In the Core HUD
  the bindings change on gameplay events (health, samples, objective, result),
  not per frame. Each halo texel reads at most (2⌈r⌉ + 1)² glyph texels (49 at
  the 2.5 px maximum, 81 at `MAX_HALO_RADIUS` = 4) and stops at full coverage.
- Memory: halo entries share the paint's atlas and its existing limits (the
  game binding's 2048 × 2048 plane, 8,192 entries); the dilation scratch is
  bounded by the largest glyph plus its margin.

No preparation time or GPU time has been measured; CPU checks and desktop runs
are not phone evidence. Cooked data and layout are unaffected: cooked UI
declarations carry no halo field, layout never measures it, and the rule is
engine code. Core gives its labels 2 px of side and 1 px of vertical padding
(handoff 0069); where a glyph's ink meets its content box the visible border box
can cut the outer part of its halo, which the rendered review must check.

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
plus its label halo, one selection and one composition span per byte of the
focused field, and six surface/caret/ring quads per widget. The review fixture
measured 240 quads and 37,746 GPU bytes (with the earlier shadow; a halo is
one quad per label glyph, as the shadow was, but its atlas entries add to the
uploaded prefix). Unchanged frames upload nothing; a focus, press or selection
change uploads one instance buffer; a text change uploads one atlas prefix
because the CPU owner re-rasterizes per paint. Debug and `ci` builds are not
performance evidence.

## Evidence

GPU tests (`--include-ignored`) compare actual draws with an independent
CPU compositor written from this contract: 1x, 2x and 1.5x, UNORM and sRGB
targets, an offset editor region, nested clips with negative-x RTL overflow,
composition over a finished world frame (with an order-sensitivity check) and
retention/refusal identity. On Apple Metal every case agrees within 1 code
value of exact arithmetic (1x UNORM exactly). The reference admits, per
channel, only the codes the backend precision allows: 8-bit fixed-point
filtering of glyph taps between texel centres, the sRGB read-back and store
between blends, and a UNORM target blended at its own 8-bit precision, which
Vulkan and D3D11.3 §17.5 permit (source and alpha codes within 0.6 code, the
destination product within 128/255 code, a saturating sum: at most 1.71 codes
from exact per blend, exact for opaque fragments). Checkpoint 12's hosted
runs printed their adapters: llvmpipe (LLVM 20.1.2, Mesa 25.2.8) on Linux and
WARP (Microsoft Basic Render Driver) on Windows. Both failed only the 1x UNORM
case of the float-only reference, with identical statistics that an emulation
of llvmpipe's 8-bit integer blend reproduces exactly, as do correctly rounded
8-bit blends with nearest or half-up conversion; float blending and coarser
integer arithmetic do not (handoff 0059). A CPU emulation of llvmpipe's sRGB
path differs from exact by up to 7 codes near black at 2x, where the sRGB curve
magnifies a 1/255 coverage step (handoff 0057). The reference admits both
emulations while rejecting text placed one pixel off, 90% text opacity,
blending in the other colour space and a missing ancestor clip, and, for label
halos, a missing halo, the undilated glyph as its halo, a halo over its text
and (with a text clip hugging the ink) a halo cut at the text clip. These halo
checks have not run yet: no build or GPU use was granted for handoff 0069. It
requires at
least 8 sub-texel bits and nearest-rounding blends; Vulkan's permitted minimum
of 4 sub-texel bits and truncating blends remain outside this reference. Hosted
Linux and Windows runs of the corrected reference are pending. A hosted
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
