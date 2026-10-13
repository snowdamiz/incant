# 0073 output-resolution current-only overlays: implementation

**Status: partial, source only.** Sprites and world text now draw at output resolution for portable temporal upscaling. **BLEND models are not moved**; the exact missing dependency is stated below. Nothing was compiled, run or rendered: no Cargo, GPU, test binary or new images. Root/Astra review, import and compile.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. The packet states root verified both through ACP before prompting.

**Base:** exact `ce6171957eda212d6796335623d8040a0f06b500`. The patch `overlays.patch` (sha256 `bd89d69d…379113d`, 10 files) passes `git apply --cached --check` against a temporary ce617 index (`controls/patch_check.json`).

**Unchanged:**
- all reconstruction, history, jitter and quality constants;
- every WGSL shader, so no cooked-shader re-pin is needed;
- physical UI;
- the fixed-resolution path and every inactive or bypassed route.

No thin-wire, detail or C7 remedy is claimed.

## The failure and the change

**Why overlays were blurry:** in ce617, BLEND models, sprites and text rasterize into the internal-size overlay. `composite.wgsl` then bilinearly upsamples that overlay over the resolved output, so internal 50% means half-resolution sprites and glyphs.

**New order, only when a portable upscaling plan exists:**
1. Opaque passes and the resolve run as before.
2. The overlay clear and the BLEND passes into the internal overlay run unchanged.
3. The **composite moves from Display to the end of the Models interval**, still drawing the BLEND overlay over the resolved output.
4. A new output depth pass re-rasterizes the opaque and MASK draws at output resolution with the unjittered camera, into a new output depth target.
5. **Sprites, then world text**, draw directly over the composite at output resolution. They use their existing pipelines (premultiplied over, LessEqual, no depth write) against that output depth.
6. Display is the output pass only.

**History:** current-only overlays still never enter temporal history. The composite is a separate per-frame target, and the history color/depth/record slots are untouched.

**Draw order and alpha are preserved:** the old order was `(text over (sprite over T)) over R`; the new order is `text over (sprite over (T over R))`. Premultiplied over is associative, so the two are equal in exact arithmetic; only fp16 rounding differs.

**Depth:** no jittered or internal depth view is reused as output depth. The re-raster uses:
- the motion system's existing depth-only pipelines, so MASK discard, cull and mirroring variants and canonical geometry are kept;
- a new bind group with `camera.unjittered().matrix(output_rect.w / output_rect.h)`, the same matrix sprites and text compute from their output `ModelTarget`;
- `set_viewport(output_rect)`.

It writes no colour, motion or history, and it does not touch the internal scene depth.

**Viewport math:** exact DRS (`Plan::new`) requires `source_rect == rect · n/16`. The output and internal aspects are therefore equal up to f32 rounding. Projecting with the output aspect into `output_rect` equals the internal unjittered projection mapped by `rect + (x − source)·16/n`. This was checked for fractional offset viewports at n = 5, 7, 8, 9, 11, 12, 13 and 15, with disagreement below 1e-3 px.

**Overlay depth pass:**
- The internal pass now runs only for BLEND models when upscaling.
- On the fixed path it keeps the ce617 condition exactly.

## Resources, passes and admission

**New target:**
- **Format and extent:** Depth32Float at the output attachment size. Usage is RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC, through the existing `make` helper.
- **Cost:** 4 B per output pixel. It is owned by `upscaling::targets::Output`, with the same Arc lifetime and exact-size and owner-flag reuse as `composite`.
- **Lifecycle:** a failed encode or reset publishes nothing, because Output is accepted only with the pending frame.

**Reused targets:**
- `composite` (RGBA16F, output size) is now also the sprite/text colour target;
- the internal BLEND overlay is unchanged (RGBA16F, source size, 8 B/px);
- history colour, depth and records are unchanged.

**Bytes per output pixel:**

| Route | ce617 | Patched |
|---|---|---|
| Portable | 32 | 36 |
| Owner | 48 | 52 |

**Admission:** `plan.rs` and `motion/attachment.rs` charge the 36 B stride for old and candidate outputs, with checked arithmetic. `MAX_OUTPUT_BYTES` becomes `36·2^22`.

**Ceiling:** 671105024 B, unchanged. The cost is honest refusal of the largest extents:
- a full-scale 2048² candidate after a maximal old set now exceeds the ceiling by exactly 33554432 B and is refused;
- 1923² still fits;
- 2048² at 8/16 still fits;
- the updated test values are recomputed exactly in `controls/overlay_controls.py`. They are listed in `motion/attachment.rs` and `upscaling/tests.rs`, and replaying the controls at the ce617 stride reproduces the ce617 values.

**GPU work per upscaled frame with sprites or text:**
- one more depth-only geometry pass, over the opaque and MASK draws, at output resolution;
- the internal overlay-depth pass is dropped when there are no BLEND models;
- the composite pass is moved, not added;
- sprite and text fragments scale by the area ratio (4× at 50%).

**Queue:** no extra submission. No queued-driver-memory, device or performance claim is made.

## Files (`src/` exact copies, `overlays.patch`)

- **`upscaling/targets.rs`, `owner.rs`, `plan.rs`, `tests.rs`:** the output overlay depth, `Pending::screen_overlay`, the 36 B stride and the exact admission boundary test.
- **`motion/draw.rs`, `motion/retained.rs`:** the output frame and depth pass, `set_output_depth`, and `encode_pass` with an explicit viewport and frame.
- **`lib.rs`:** the sprite/text target and depth selection, and the depth-pass conditions.
- **`graph.rs`:** the composite and output depth inside Models, the plan counts and a new counting test.
- **`graph/native.rs`:** the same order for MetalFX.
- **`motion/attachment.rs`:** the 36 B stride and recomputed exact totals.

## Controls (CPU only)

Run with `python3 -I -B …/controls/overlay_controls.py --out …/overlay_controls.json`. **51/51 checks pass.** They cover:
- **Viewport and projection:** output viewport and unjittered projection at internal 50/75% and odd DRS scales; the internal jitter would shift output overlays by at least 0.25 px.
- **Order and alpha:** the moved composite and the old order agree within 1e-12 over 2000 random BLEND, sprite and text stacks, including alpha-zero and clamp; depth LessEqual cases.
- **Routing:** the routing table, with bypass and default unchanged.
- **Admission:** exact admission totals, the stride-32 ce617 replay, overflow refusal and the unchanged ceiling.

`controls/patch_check.py` rebuilds the patch and checks it applies.

## Exact missing dependency: BLEND models at output resolution

**Why BLEND cannot simply move:** `model_material.wgsl` → `direct_lighting(in.position.xy, …)` indexes clusters by `(pixel − light_grid.viewport.xy) / 64`. Those clusters are built by `lighting.prepare(…, target.rect, …)` for the internal viewport only, and `LightingSystem` keeps a single-entry `GridBuffers` cache keyed by dimensions. Drawing BLEND at output resolution would read the wrong clusters, and a second `prepare` per frame would thrash that cache.

**What it needs, still under the same transparent target and order:**
1. a second cached grid for the output rect;
2. its `ClusterPass` encoded in AssignLights;
3. a transparent-only shading bind group;
4. that buffer in the accounting;
5. `ModelDraw::set_transparent_target` taking the colour, depth and rect.

No placeholder is added. BLEND stays internal-resolution and bilinear, exactly as in ce617.

## Limitations

- **Not compiled or run here:** no clippy and no device execution. The `native-metalfx` graph edit is unverified because that feature is not built by default.
- **Untested routes:** the owner device tests are untouched and do not cover the new passes, and no retained GPU test covers sprites or text under upscaling.
- **Rendered evidence:** the 15-image protocol is unchanged and still to be run by root.
- **Expected appearance:**
  - sprites and text should match FULL's rasterization;
  - BLEND cards stay as soft as before;
  - opaque occlusion of sprites and text follows the exact output re-raster, so it is not affected by the reconstructed colour.
- **Untracked files:** the dirty `brief.md` and the three untracked `owner-attachment-*` inputs stay unstaged. The old packets and the top-level `result.md` are unchanged.
- **Scratch:** `/tmp/0073-overlay-scratch/admit.py` only. No memory was read or written.
