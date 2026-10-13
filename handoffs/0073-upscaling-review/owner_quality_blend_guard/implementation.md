# 0073 BLEND guard: output-grid ownership and display sanitation

**Status:** source correction of packet `36b5f0dad20f4c34b693affe7fdfd8e57e4bd617`. Nothing was compiled or run (no Cargo, renderer, GPU or images). Root and Astra review and build.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Per the packet, root verified both through ACP.

**Base:** exact `f075b3900d086acbfc0b93ee036bb5fbea90cd02`.

**Deliverables:**
- **Patch:** `blend_guard.patch`, 10 files. It passes `git apply --cached --check` (`controls/patch_check.json`); its sha256 is recorded in `controls/patch_check.json` and reported with the commit.
- **Source:** exact changed files under `src/`.
- **Review diff:** `review_vs_blend.diff` is the minimal diff against the `owner_quality_blend` copies. Files that packet left at f075 are diffed against f075, and `retained.rs` returns to exact f075.

**Unchanged:**
- every WGSL file, so no cooker or trusted-pin change is needed;
- reconstruction, history and jitter;
- the sprite and text output route;
- opaque and MASK occlusion through the output re-raster;
- the default, fixed and pixel-camera paths;
- the cluster mapping (output viewport, unjittered output camera, `pixel − viewport.xy`).

## 1. Who owns the output grid, and how it is charged

**Previous defect:** `LightingSystem.output_cache` was renderer-wide. It outlived upscaling and was charged only when a plan existed. It was also shared by every retained scene.

**Correction:**
- The renderer no longer holds an output grid.
- `LightingSystem::prepare_output(.., retained)` reuses `retained` when the dimensions and mask words match. Otherwise it allocates new `GridBuffers`. It returns the grid without caching it.
- The internal grid keeps its f075 single-entry cache unchanged.

**Ownership is per upscaling transaction, like `Output` and `Overlay`:**
- `State.grid` holds the grid of the last accepted output-resolution BLEND frame.
- `Pending` carries `retained_grid`, a clone of `State.grid`, and a `OnceLock` candidate. `lib.rs` offers the grid returned by `prepare_transparent_output`.
- `State::accept` takes the candidate. A frame without BLEND keeps the retained grid, which is still charged.
- An abandoned or failed frame drops its candidate with the `Pending`.
- Disabling upscaling drops the `State` and therefore its grid.

**Accounting:**
- `State::preflight` and `preflight_owner` charge the retained bytes (`GridBuffers::bytes`, the counts plus the masks) plus a worst-case candidate. The candidate is ⌈w/64⌉·⌈h/64⌉·24 clusters at 516 B each, charged even when the grid is reused.
- All arithmetic is checked, under the unchanged 671105024 B ceiling.
- No unaccounted output grid can exist without upscaling, because nothing renderer-wide owns one.

**Logical versus driver scope:**
- Dropping `State` ends logical ownership only.
- Bind groups inside submitted, unfinished command buffers keep the buffers alive until the GPU completes them. That is wgpu and driver residency, which admission never claimed to bound.

## 2. Display sanitation

**Previous defect:** BLEND blended directly onto the composite, bypassing `clean()`.

**Correction, with no shader change:**
- BLEND renders into the existing current-only `Overlay`. It is now **allocated at the output extent** (`Overlay::prepare(d, p.output, ..)`) and is still cleared by the existing `clear_overlay` pass. BLEND uses the output depth, output rect, output camera and output grid.
- The unchanged `composite.wgsl` then computes `clean(clean(overlay)·w + clean(under)·(1 − over.a))` exactly as before.
- It binds a **second composite uniform**, identical to the resolve uniform except `source = output rect`, so the shader reads the overlay 1:1:
  - `base = p − 0.5` and `f ≈ 0`;
  - f32 rounding of `(p − o)/w·w` can leak at most one ulp of the coordinate as neighbour weight; the measured worst is 3.05e-5, below RGBA16F precision;
  - the `clean()` calls bound the result for any overlay texel, including NaN, ±inf, negative values, alpha > 1 and alpha ≤ 0.
- This is a post-condition of the existing sanitation, not an inferred domain. No real NaN has been observed.

**Order (Models, with a plan):** resolve, overlay clear, output re-raster depth, BLEND into the overlay, sanitizing composite. Then sprites and world text draw over the composite, then Display. MetalFX post runs normalize, overlay clear, output depth, BLEND, composite. Without a plan the order is f075.

**Resources:**
- The overlay changes from 8 B per source pixel to 8 B per output pixel.
- `plan.rs` charges `(36 + 8)·output + 24·source`. `motion/attachment.rs` uses the output extent for the overlay component.
- Recomputed exact test totals: 606,552,064 B and 655,654,912 B. The other totals are unchanged.
- Boundaries: 1923² full scale still fits without a grid, 1924² is refused, and 2048² at 8/16 fits.

**GPU work:**
- One extra uniform (96 B) per frame.
- No added pass or submission; the composite pass only moves after BLEND.
- The output cluster dispatch is still untimed: it is not covered by the existing AssignLights timestamp endpoints.

## Controls

`controls/guard_controls.py` passes 13/13 checks. It is a transcription, not engine execution. It covers:
- **Lifecycle:** a reference-counted mirror of accept, reuse, abandon, a frame without BLEND, disable with a queued reference, and resize.
- **Accounting:** the 2048² frame at 8/16, with a retained grid and a candidate grid, totals 654,524,416 B. The 1923² full-scale candidate grid is refused. The ceiling is unchanged.
- **Sanitation:** composite f32 arithmetic with adversarial texels on fractional offset viewports and an 8191-px-wide rect.

## Limitations

- **Not built or exercised:** no compile, clippy or GPU run. The MetalFX and cooked builds are unverified.
- **Pre-existing f075 behaviour, unchanged here:**
  - sprites and text still draw over the composite after sanitation;
  - the overlay composite is 1:1 only up to the stated f32 rounding leak;
  - the second cluster dispatch is untimed.
- **Grid retention:** a grid kept through frames without BLEND stays allocated and charged until the state is dropped or a new grid is accepted.
- **No claims:** no queued-driver memory, performance, device or appearance claim.
- **Untouched:** the dirty brief, the three `owner-attachment-*` inputs, every prior packet and the top-level `result.md`.
- **Scratch:** only `/tmp/0073-blend-guard-scratch`. No memory was read or written.
