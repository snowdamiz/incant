# 0073 output-resolution current-only BLEND models: implementation

**Status: complete source patch, not compiled or run.** Nothing was built or executed (no Cargo, renderer executable, GPU run or new image). Root and Astra import, review and build separately.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) at thinking effort max. The packet states root verified both settings through ACP.

**Base and patch:**
- Base: exact `f075b3900d086acbfc0b93ee036bb5fbea90cd02`.
- Patch: `blend.patch` (sha256 `93fd2b23762f484e495a024f2d27548de2dd7aa324d98edbc932d87b33b9f69c`, 9 files). It applies with `git apply --cached --check` (`controls/patch_check.json`).

**Unchanged:**
- reconstruction, history and jitter, with their constants;
- every WGSL shader, so no cooker or trusted-pin change is needed;
- physical UI;
- the f075 sprite and text output route;
- every route without a portable upscaling plan (fixed, default, pixel camera, inactive).

No thin-geometry or C7 remedy is claimed.

## What changed

**Lighting (`lighting/mod.rs`):**
- `LightingSystem` gains a second single-entry `output_cache`, so the internal grid and the output grid never evict each other.
- `prepare` keeps its exact behaviour through `prepare_in(.., output: bool)`.
- New `prepare_output` builds the same clusters for the output viewport with the given camera. It uses the existing grid shader and bindings, including the cooked-clusters size buffer and the cooked-materials binding 7.
- With the unjittered camera it selects the ordinary, non-temporal cluster pipeline.
- New `output_grid_bytes()` reports the retained grid; `output_grid_allowance(viewport)` gives the worst-case bytes.

**Model draw (`model_draw.rs`):**
- New `prepare_transparent_output` runs only when BLEND draws exist. It creates:
  - an unjittered camera bind group at the output aspect `rect.w / rect.h`, which is the exact output rect, not an aspect guess;
  - the output-grid lighting, reusing the frame's shadow maps;
  - a `TransparentOutput {target, camera, lighting}`.
- `encode_transparent` uses that target's colour, depth and viewport, the output camera and the output shading group 3.
- `encode_range` now takes the target and shading explicitly. Opaque, all-draw and f075 transparent calls pass the old values unchanged.
- The BLEND pipelines are unchanged: ALPHA_BLENDING, depth Less with no write, MASK, culling and mirroring keys, and the primitive-centre back-to-front sort.

**Routing (`lib.rs`):**
- With a plan, BLEND targets the f075 composite, the f075 output re-raster depth and the output rect.
- Without a plan, the f075 `set_transparent_target(internal overlay)` stays unchanged.
- With a plan, no internal overlay-depth pass runs. The output re-raster runs when BLEND, sprites or text exist.

**Graph (`graph.rs` and `graph/native.rs`):**
- With upscaling, the Models order is: resolve, overlay clear, composite, output depth, BLEND. Then come the Sprites and Text intervals and Display.
- Without upscaling, BLEND stays at its f075 position.
- AssignLights also encodes the output cluster pass, untimed, after the internal one and before Models.
- MetalFX: the output cluster pass is encoded in `pre`, and the post buffer runs composite, output depth, BLEND.
- Pass counts and queue submissions are unchanged.

**Order and alpha:** the internal overlay stays allocated but empty, so the composite carries the resolved output alone. BLEND blends straight onto it. Scalar algebra equals the f075 order within 1e-12 (`controls/blend_controls.json`); that is algebra, not raster equivalence. Overlays stay outside history.

**Admission (`upscaling/plan.rs`, `upscaling/owner.rs`, `retained.rs`):**
- `Plan::preflight` now returns its total.
- `Plan::with_output_grid` adds the retained output grid and a worst-case candidate with checked arithmetic, under the unchanged 671105024 B ceiling.
- Both the portable and owner preflights apply it before any allocation.

## Resources, passes and lifecycle

**New buffers:** at most one retained output grid, made of two STORAGE | COPY_SRC buffers:

| Buffer | Size |
|---|---|
| Counts | clusters × 4 B |
| Masks | clusters × words × 4 B |

Here clusters = ⌈w/64⌉·⌈h/64⌉·24 over the output rect, at most 131072, and words = ⌈local/32⌉, at most 128.

**Per-frame objects:** the light-grid uniform (128 B), a cooked sizes uniform if that feature is on, two bind groups and the camera uniform (96 B).

**Admission charge:**
- The charge is the retained grid plus 516 B per candidate cluster (1 count word and 128 mask words), taken even on reuse.
- Examples: 12,681,216 B at 2048², 1,387,008 B for the fixture viewport.
- f075's 1923² full-scale boundary had 199,068 B headroom, so it is now refused. The largest full-scale square after a maximal old set is 1880².
- This is logical admission only. It makes no claim about queued or driver allocations.

**GPU work:**
- one extra cluster compute dispatch (⌈w/64⌉·⌈h/64⌉·24 workgroups) when BLEND draws exist;
- BLEND fragments now at output resolution;
- no new render pass and no new submission.

**Lifecycle:**
- The output grid cache is replaced at prepare time, exactly like the internal grid. An abandoned or failed frame leaves at most one cached grid, which is counted as old next time.
- Scene, output and accepted state stay transactional as in f075.
- The grid is never freed when upscaling stops; it stays bounded to a single entry.

## Controls

**Python controls:** `controls/blend_controls.py` passes 31/31 checks. It covers:
- output fragments indexing the output grid, with every tile reachable at 8/16, 12/16, 5/16 and 13/16 on offset fractional viewports;
- internal-grid misuse hitting a wrong tile for 91–98% of pixels on the larger viewports;
- blend order;
- routing;
- exact allowance and boundary numbers.

**New Rust unit test** in `upscaling/tests.rs`: `output_blend_grid_is_charged_as_old_plus_worst_case_candidate_under_the_ceiling`.

## Limitations and missing items

- **Not built:** nothing was compiled, linted or run; the MetalFX and cooked paths are reasoned, not built.
- **No GPU test:** no retained device test covers BLEND under upscaling.
- **Unclamped colour:** BLEND results now reach the composite without the f075 composite clamp on the overlay. Finite shader outputs stay in range, but a non-finite output would not be sanitized at that point.
- **Wasted overlay:** the internal overlay, its clear pass and its 8 B per source pixel remain, now unused when there is a plan. Removing them changes the f075 accounting, so I left them in place.
- **Unmeasured:** no timing for the second cluster pass, and no performance, device or appearance claim. The same 15-image protocol is for root to run.
- **Untouched files:** the dirty brief, the three `owner-attachment-*` inputs, all prior packets and the top-level `result.md`.
- **Scratch:** none. No memory was read or written.
