# 0073 owner_quality_repair: diagnosis and focused repair proposal

**Status:** this is a CPU/source proposal for root review.
- **Not done:** no build, test-binary or renderer run, GPU, image or capture.
- **Not claimed:** no phase gate, appearance acceptance or performance claim.
- **Kept open:** every historical failure, including a5ae3179, GPU001, the linear-model C7/C9, the unresolved P1, and the failed owner_quality appearance result ebc7e690.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort Max. Root verified both through ACP before prompting, and this session ran at max.

**Inputs**
- Brief sha256 `288ba967d387f9ac11f56d7410ec04885ae2b0e4e37907f8246a4bfa80998f47`.
- `owner_quality/result.md` and `owner_quality/result.json` at ebc7e690.
- Production `fd5b86d03a2cabfcc9bd5a8441a137645fb858ae`, read with `git show`. This child repository holds the same commit. Root's checkout was not touched.

## 1. Causes

I mark each cause by what supports it:
- **Demonstrated:** retained evidence plus source show it.
- **Source:** reading the code shows it.
- **Model:** the scalar CPU control in `controls/reconstruction_model.py` shows it. Model values are in linear scene units.

### Lost high-frequency detail against DRS

| Cause | Basis | Detail |
|---|---|---|
| **Bilinear current reconstruction** | Source; model matches the observed ranking | `resolve.wgsl:18-26` and `resolve_owner.wgsl:86-102` reconstruct the jittered internal image with 2x2 bilinear taps. That is a tent ±1 internal pixel wide, ±2 output pixels at 8/16. With a fixed history fraction, the converged image is that tent averaged over the jitter. |
| **Bilinear history resampling** | Source; model | Motion resamples history bilinearly at the reprojected position on every frame (`resolve.wgsl:44-56`), so blur compounds during pans. |
| **Texture LOD from internal derivatives** | Source | `model_material.wgsl:39-43` samples textures with implicit derivatives. That caps texture detail at the internal resolution for DRS, UP and OWN alike. |
| **The documented history maximum** | Source; model | `TemporalConfig.history_weight` is the documented "maximum old-color contribution". At 8/16, only about 2 of the 8 phases put a sample within half an output pixel of a given pixel, yet every frame must still contribute at least 1−h. A narrower kernel under the same h trades detail for shimmer. The design-only confidence oracle avoids this only by exceeding h, which slows stale-history decay. I did not adopt it (section 4, D4). |

Model, 4 px cells: the converged contrast is 0.171, against 0.301 for DRS and 0.320 for FULL. This matches the ranking observed in the stationary group: 4-texel checker contrast 31 sRGB codes for UP, 61 for DRS.

Model, 4 px cells during a pan: contrast is 0.112 moving against 0.171 static.

Model, narrower Gaussian kernel at the same h: contrast rises by 32%, but temporal activity rises 4×.

### Phase-dependent thin geometry

**Coverage exclusion (source).** If any positive-weight 2x2 tap has alpha below 1 or no depth, the shaders take the current-only exit. Depth stays 0 and no record is kept (`resolve.wgsl:24,27,33`; `resolve_owner.wgsl:92,104,110`).
- Wires, the diagonal and the mast against the clear sky are therefore never accumulated.
- Each frame shows only that phase's samples. That is the observed 4-of-8-phase blinking, identical in UP and OWN.
- OWN's unclipped persistence never reaches these pixels.

**Jitter lattice (source; model).** `temporal/jitter.rs` `SAMPLES` has only 4 distinct x and 4 distinct y offsets.
- At 8/16, sample rows fall 0.5 output pixels apart.
- A 0.4 px horizontal feature misses every row in 18.8% of its positions, so it is never sampled in any phase.
- That matches the observed 0.4 px wire, absent in all 8 phases.

### Slow 4/16 → 16/16 recovery

**Fixed fraction h on every frame (source; model).** After a scale increase, history gathered at 1/16 the sample density keeps weight 0.9. The model's fourth 16/16 frame reaches only 0.43 of the matched reference for 2.3 px cells; the rendered ratios were 0.38–0.68.

**The wipe at the 12→4 switch is an existing invariant (source).** `camera_static` is false on every DRS change by design (`motion/owner.rs:206-211`: "source-sized receiver guards can change shadow cascade quantization"). OWN therefore takes the clamped path. The box, built from mip-filtered 4/16 samples, clamps the history detail away in UP and OWN alike. This repair does not change that invariant.

### The step-20 "dark block" is the cube's real shadow (demonstrated)

`controls/dark_block.py`:
- **Geometry:** it replays the fixture's camera, mover and `sun_rotation(38, 30)` with the engine's world·−Z light convention. That reproduces the cube's step-20 screen box exactly (667–731 × 246–297). The cube's shadow on the sign then falls at 602–658 × 287–312, which contains the block I recorded (620–655 × 290–312).
- **Retained frames:** in FULL, which has no history, that polygon is also dark. Its mean Rec. 709 luma is 87.6 sRGB codes, against 163.3 for the lit rings above it.
- **UP and OWN:** both show 106.8 against 169.1. The ring texture inside is flattened: luma SD 38.1, against 50.8 in FULL.

**Ruled out by source**
- **Stale shadow maps.** `shadow_draw.rs` `encode` clears every cascade layer each submission with `LoadOp::Clear(1.0)` and redraws the current casters. `shadows/mod.rs` caches only the atlas allocation, keyed by light count.
- **A history ghost.** FULL shows the same darkening without any history.

So no host or shadow fix is needed. The visible defect is texture lost inside a real shadow, which is the detail cause above.

**Correction to my ebc7e690 record.** I wrote that FULL "shows crisp rings there". FULL does show the rings, but they are in shadow. That record stays unchanged.

## 2. Repair: `repair.patch`

8 files, +198/−9 lines, applies to exact fd5b86d0.

**R1: Catmull-Rom current reconstruction**
- **Where:** `sharp_current` in both shaders, with helpers in `common.wgsl`.
- **When:** for interior pixels, after the unchanged 2x2 same-surface test. It interpolates over the 4x4 clamped source taps.
- **Clamp:** the result is clamped to the central 2x2 (bilinear) range, so it adds no new extrema. It is premultiplied and finite through `clean()`, with alpha 1.
- **Fallback to today's bilinear estimate:** if any nonzero-weight tap is not admitted, not opaque, or not depth-compatible with the anchor. In OWN, also if an outer tap has an invalid or changed owner.
- **Provenance (OWN):** outer owners join the record, and their classes join the support classes. Every tap that influences colour therefore stays in the provenance.

**R2: Catmull-Rom history resampling**
- **Where:** `sharp_history`, run after the existing bilinear history and its per-tap rejection.
- **When:** only if every nonzero-weight 4x4 previous tap is inside the output, admitted, opaque and depth-compatible. In OWN, each tap also needs a canonical record with `r.a == anchor_id` that passes `record_admissible`, and all those records join.
- **Fallback:** otherwise the bilinear history.
- **Static frames:** the fractional offset is 0, so only the centre tap carries weight and the result is identical to today.

**R3: frame-mass history fraction**
- **Rule:** fraction = min(h_r, W_r/(W_r+1)), where h_r is today's reactive-reduced h.
- **Host (`owner.rs`):** W is 0 after a cut. It grows by 1 per accepted frame up to h/(1−h), and is multiplied by (old/new width)² after an internal-scale increase. It reaches the shader through the zero `reactive.y` lane of the existing 96-byte uniform.
- **Steady state:** exactly h_r. CPU emulation shows cap/(cap+1) = h exactly in f32 for h of 0, 0.25, 0.5, 0.9 and 0.99; the unit test asserts the 0.9 case and the h bound for all five.
- **Bound:** the fraction is never above today's per-pixel fraction, so stale history decays at least as fast per frame.
- **OWN static persistence:** uses min(h, W/(W+1)).

**Tests in the patch**
- **`tests.rs`:** adds `frame_mass_resets_attenuates_and_never_exceeds_history_weight`.
- **`device_tests.rs`:** sets `reactive[1] = h/(1−h)` (1.0).
- **`owner_device_tests.rs`:** sets `reactive[1] = h/(1−h)` (9.0).
- **Expected values unchanged:** I traced the asserted values by hand. Those fixtures use ratio 1 with zero jitter (only the centre tap has weight), or the Catmull-Rom estimate is pinned by the 2x2 clamp, or a rejected tap forces the bilinear fallback.

**Unchanged**
- **Shader logic:**
  - the mixed and clear current-only exits;
  - depth, owner and changed-contributor admission;
  - the 3x3 clamp box and its owner join;
  - the reactive term;
  - the camera-static and global-change flags;
  - the record format.
- **Interfaces and resources:**
  - composite and overlays;
  - targets, pipelines and uniform size;
  - the 671,105,024-byte ceiling;
  - the `u32` array accesses;
  - the public config, presets and defaults.

## 3. CPU controls

These come from the independent scalar model at 8/16, in linear units. They are not rendered results. Contrast is the standard deviation; error is the mean absolute difference against FULL; activity is the per-pixel range over the last 8 frames.

| Case | Today, UP | Repair, UP | DRS | FULL |
|---|---|---|---|---|
| 5.3 px cells, static: contrast / error / activity | 0.228 / 0.137 / 0.0113 | 0.260 / 0.105 / 0.0108 | 0.296 / 0.127 | 0.350 |
| 4 px cells, static | 0.171 / 0.153 / 0.0115 | 0.225 / 0.107 / 0.0103 | 0.301 / 0.087 | 0.320 |
| 2.3 px cells, static | 0.038 / 0.211 / 0.0114 | 0.053 / 0.198 / **0.0197** | 0.166 / 0.196 | 0.282 |
| 4 px cells, pan 0.37 px/frame: contrast / error | 0.112 / 0.223 | 0.204 / 0.144 | | |
| Transition, third / fourth 16/16 ratio, 5.3 px cells | 0.84 / 0.88 | 0.95 / 0.97 | | |
| Transition, third / fourth 16/16 ratio, 2.3 px cells | 0.325 / 0.427 | 0.825 / 0.883 | | |
| Stale shading residual at frame 18 / 24 / 40 | 0.648 / 0.280 / 0.043 | 0.642 / 0.276 / 0.042 | | |
| Error after a cut at frame 18 / 24 | 0.026 / 0.013 | 0.007 / 0.001 | | |
| Error after a local reset at frame 18 / 24 | 0.026 / 0.013 | **0.028 / 0.014** | | |

- **Stale history:** the repair's residual is at or below today's at every frame, for both UP and OWN.
- **OWN:** tracks UP. Its static 2.3 px detail is 0.067, against 0.041 today.
- **Cost, shimmer:** near-Nyquist 2.3 px textures shimmer more in UP. Activity rises 1.7×.
- **Cost, local resets:** after a local reset such as a disocclusion, error is about 8–10% higher for the first frames. The sharper single-frame estimate starts further from the converged image, and the global W cannot see a local reset.

## 4. Decisions for root (not in the patch)

**D1: jitter lattice.**
- **Change:** 8 distinct x and y offsets (odd sixteenths).
- **Effect:** removes the never-sampled 0.4 px case (18.8% → 0 in the model).
- **Blast radius:** `SAMPLES` is shared with the fixed-resolution temporal path and the kernel oracles and tests, so this is root's call.

**D2: texture LOD bias.**
- **Change:** a bias of log2(n/16) for UP/OWN internal renders.
- **Affected files:** `model_material.wgsl:39-43`, and the MASK alpha samples in `motion/shader.wgsl:26` and `motion/owner.wgsl:27`. It needs a per-frame uniform field, an ABI review and a cooker pin.
- **Why:** it is the only way past DRS's texture ceiling.

**D3: OWN static persistence for mixed and clear pixels (thin geometry).** This is not implemented, because it changes the clear-handling invariant the brief says to preserve.
- **Rule:** on a static frame with exact provenance, OWN may persist own-texel history that has alpha ≤ 1:
  - mixed exits keep their record and a positive depth;
  - clear pixels persist whenever the recorded owners are unchanged;
  - the output alpha is always below 1, so the ordinary path still never admits it.
- **Soundness:** retained bindings are fixed, and every coverage change is a marked world or pose change (`motion/retained.rs:441,636`).
- **Model:** a 0.8 px line goes from on/off (0 ↔ 0.875) to a steady 0.43–0.51.
- **Test impact:** it changes owner device test case 11, probe (3,11), and case 12, "clear current is clear output".

**D4: evidence-mass history beyond `history_weight`.** The design-only `confidence_oracle` would raise 5.3 px contrast from 0.260 to 0.282. It is not adopted: it breaks the documented maximum and leaves 1.9× the stale residual at frame 24 (0.53 against 0.28).

## 5. Follow-up diagnostic and acceptance plan

**Build and CPU tests (root)**
- Root builds fd5b86d0 plus `repair.patch` and re-cooks the cooked-temporal shaders.
- `tests.rs` `actual_upscale_shaders_validate` runs naga over the changed WGSL. It has not been run here.
- The new frame-mass test runs alongside it.

**GPU tests (under a grant):** the device tests, whose expected values should be unchanged, and the retained tests.

**Rendered re-run**
- Use the same owner_quality fixture, runner shape and budgets: 5 processes, 778 queue submissions, 380 maps, 15 PNGs. Only the frozen binary changes.
- Compare FULL, DRS, UP and OWN against ebc7e690, which stays unchanged.

**Expected:**
- higher checker contrast in stationary and motion;
- lower mover and pan error against FULL;
- higher 16/16 transition ratios;
- C7 within about 10%;
- a P1 interior residual of 0;
- unchanged wire presence, since D1 and D3 are not applied.

**Dark block:** re-run `controls/dark_block.py` on the new step-20 PNGs; no extra capture is needed. The expected sign is shadow-polygon luma SD rising toward FULL's 50.8.

**Acceptance:** Claude's visual review of the retained PNGs decides, not these numbers. The specific risks to look for are edge halos, new shimmer and longer trails. Performance needs reference-phone measurements; none is claimed.

## 6. Limits, commands and hashes

**Runtime cost.** The extra texture loads per pixel are unmeasured:
- up to 16 current colour and depth taps on interior pixels;
- 12 owner taps in OWN;
- up to 16 history taps, only while moving, since zero-weight taps are skipped before loading.

**Not run here:**
- the naga WGSL validation (my own scan found no reserved-word collisions);
- any GPU, renderer, test binary or Cargo command.

**Commands run (CPU only):**
- the three standard-library Python controls, each re-run with byte-identical output;
- `git show`, `rev-parse` and `hash-object` without `-w`;
- `read-tree` into a temporary index inside this folder (deleted), followed by `git apply --cached --check`, which passed;
- `rustfmt --edition 2024 --check` from stdin with the pinned 1.99.0 toolchain, on the original and proposed Rust files: exit 0.

**Deviation:** two scratch files in `/tmp`, a model-output redirect and a rustfmt check diff, were created and deleted. This is outside the packet folder.

**Hashes (sha256)**

| File | sha256 |
|---|---|
| `repair.patch` | `7dc2c3303784cbbd5ac6bbe464f9f614ce743691d5664edd088e0a58c9cf29aa` |
| `src/.../common.wgsl` | `e1c2e552e462a3e27ad6632324b3880ef0cb8f96ec2e1c346ec6205e0520c879` |
| `src/.../resolve.wgsl` | `296fe89066865c4feff4d486506d762a4955a8351aa3f83aa0eff72ba375fc03` |
| `src/.../resolve_owner.wgsl` | `23c094e20e62bba3da35fca373457024cfe5405c2be042802493bbfc1cea4c9f` |
| `src/.../owner.rs` | `5a726faa275d0a2b8782caaaca872e6bd70c97f66df2215aabf5355b777f2642` |
| `src/.../system.rs` | `00ddce30484539148b2887e778db8203f4bfbec222964978bf6ad9a91fcb5442` |
| `src/.../tests.rs` | `a106ea8c249ab7c9b9c44ce8c3041a6f189e6070d9a8eeec68f0029140b1749f` |
| `src/.../device_tests.rs` | `5665df3ad9395fed273e1057c31acef2fe482fdc77edf7ce743804a00c99257e` |
| `src/.../owner_device_tests.rs` | `40d3f0465934ca0dd252d57953c7e41457dfbfdbab6c87070ea20af99f9347dc` |
| `controls/reconstruction_model.py` | `003ec456fb0f14305d997a0f8d60c05f390f6d327844c7194511da4c8309506b` |
| `controls/reconstruction_model.json` | `b8edbde6a3e13451d659714fba2a8507efefeab144b6e3b496f630179c9b88d1` |
| `controls/dark_block.py` | `6333680042391c9c6973bce0a2671bfede83f0ab559570e517429ce6ad170055` |
| `controls/dark_block.json` | `9554b95ee34675fdfca5796b8ad6316e783dc00d1dcb519c60b8aafbb4fc8a0c` |
| `controls/patch_check.py` | `44b1802179e81ebcb7e9e3b1a21081bc188f30d714a6eff06beb8d57f1a7fcb9` |
| `controls/patch_check.json` | `db6fe92ef21a30c0512598d12e329bf52533ae93773d5a7589898b08a92e4911` |
