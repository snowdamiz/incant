# 0073 reconstruction implementation, r4: status and rationale

**Status: PARTIAL candidate, compile-unverified, not accepted.** This turn completed the evidence estimator for both programs (UP and OWN), its host, transaction and validity rules, and the shader-cooker ABI and source pins. All corrections from the R4 packet and both supervised interrupts are applied (mass order, then continuation-002).

Not implemented: internal texture LOD/MASK agreement, the DRS shadow treatment, the oracle tests and the numeric expectations in the device tests. §6 lists each with its exact source path.

No Cargo, naga, cook, GPU, render or image was run. No visual acceptance is implied.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Per the packet, root verified both through ACP before prompting. Auto-memory is off; no memory was read or written.
- **Base:** exact production `76be9608e99bf089fcb1e0908827330f70c28ac1`. The source copies continue the immutable R3 `e3bc4cd2` copies.
- **Patch:** `reconstruction_r4.patch`, 21 files, sha256 `ce666f0c03686fd44b7fd3c24e0579112256060c63c70145a63b814bf041529e`. It passes the Python re-apply and `git apply --cached --check` against 76be (`patch_check.json`).
- **Unchanged:** every old packet, the briefs, the top-level `result.md` and the three owner-attachment files.

## 1. Corrections applied

1. **Full-support admission (R3 finding 1):**
   - Both programs build current evidence from every positive in-rect tap of the declared 3×3 Gaussian support.
   - History is admitted only when every contributor is an opaque surface compatible with the nearest one (`single`). The counterexample (an outer HDR-green tap at depth 2 under a black depth-5 surface) keeps its current colour and mass but reuses no history.
   - The clamp box is exactly the contributing support.
2. **Mass order (interrupt 1):**
   - `stored_evidence()` validates only the RG16F wire range: mass finite in [0, 65504], age an integer in [0, 8).
   - Per-tap mass is no longer capped. `N = Σ b·W·c`, `C = Σ b·W`, `H = N/C`.
   - Then, once: `W′ = min(C·density, cap)·(1 − strength·reaction)`.
   - This holds in `resolve.wgsl` and in both `resolve_owner.wgsl` branches. The static branch uses b = 1 and density 1.
3. **OWN retirement before blending (continuation-002 item 1):**
   - The candidate age, `0` if current primary-owner surface evidence exists and otherwise `s.y + 1`, is computed before the blend.
   - Persistence requires `age < MAX_AGE`. On the eighth absent frame, no old colour, record or evidence remains.
   - Only fresh surface evidence seeds a new record (age 0); clear alone never seeds.
4. **Carried primary (continuation-002 item 2):**
   - Static persistence now writes `carried(r, e.owners)`: the record's primary stays primary, and fresh contributors join after it.
   - Overflow is conservative: three or more IDs set the class mask, which keeps every class.
   - Previous `[5]` with current support `[7]` therefore gives `[5,7]` at ages 1…7. At age 8 it is refused and a fresh `[7]` seed starts at age 0.
   - Depth follows the current surface only when its anchor owner is the carried primary.
5. **Full-support motion coherence (continuation-002 item 3):**
   - A new `coherent(p, m)` check in both programs requires every contributing current sample to carry finite, valid motion.
   - Each sample's reprojection must agree with the anchor's within `MOTION_SLACK = 1/256` output pixel. This is the static proof's existing bound, not widened.
   - Each sample's previous depth must be `compatible` with the anchor's (existing tolerance).
   - The moving OWN branch also refuses history when any changed current contributor is not the anchor owner (`changed_secondary`).
   - The OWN static branch keeps its complete proof: camera static, no light or caster change, every contributing owner valid and unchanged, and a motionless anchor.
6. **Alpha-0 claims:**
   - A sample counts as surface, anchor or primary evidence only when `a > 0` and depth > 0.
   - An alpha-0 sample with a positive (malformed) depth is clear. It is zero colour at full mass, with no owner and no age reset.

## 2. Estimator (both programs)

- **Current evidence:** `S = Σ w·clean(c)` and `m = Σ w` over the 3×3 Gaussian at true jittered positions, with `ρ = 0.35 + 0.325·max(r − 2, 0)` and `r²/(2πρ²)` normalisation.
  - Clear contributes zero colour at full mass.
  - A malformed tap gains no weight and refuses history.
- **Fresh colour and update:**
  - Fresh colour is `clean(S/m)`.
  - The update is `colour = clean((H_clamped·W′ + S)/(W′ + m))`, stored as `(min(W′ + m, cap), age)` in RG16F.
  - No prior is stored.
- **UP storage:** UP stores history only for single-surface support, and its age is always 0.
- **OWN storage:** OWN stores partial coverage only through the static proof, with the integer absence clock described in §1.
- **Reactivity:** reactivity 0 cannot disable retirement, because age is independent of `strength`.

## 3. Host, validity and ABI

**Targets and bindings:**

| Program | Bindings | MRT outputs | MRT bytes |
|---|---|---|---|
| UP | 1–6 (6 = previous evidence) | colour RGBA16F, depth R32F, evidence RG16F | 16 |
| OWN | 1–6 float/depth, 7 owner IDs (uint), 8 previous records (uint), 9 owner-change uniform (12,528 B) | colour, depth, records RG32Uint, evidence RG16F | 24 |

- A startup check refuses explicitly if RG16F or R32F is unsupported.
- `Output.evidence` holds two RG16F views.

**Validity (`owner.rs`):**
- `evidence_valid` requires the evidence flag from the previous accepted frame, the same `Output` allocation (`Arc::ptr_eq`) and `has_previous`.
- It is published only in `State::accept`, so a failed prepare or submit changes nothing.
- A native MetalFX frame calls `Pending::mark_native()` before acceptance, under its feature gate. Native-to-portable fallback therefore never pairs native colour with stale evidence.
- `density` is old/new source-rect area from exact rects.

**History-disabled fallback controls:** these are unchanged. A cut, resize, owner toggle or native frame invalidates evidence.

**Cooker contract (`incant_shader_cook/src/temporal`):**
- `contract.rs`:
  - global counts: Upscale 6→7, UpscaleOwner 9→10;
  - outputs: Upscale `[F(0,4), F(1,1), F(2,2)]`, UpscaleOwner `+F(3,2)`. MetalFxNormalize keeps `[F(0,4), F(1,1)]`;
  - OWN uint textures move from 6|7 to 7|8, and the changes uniform moves from 8 to 9.
- `emit.rs`: the MSL buffer 1 mapping moves from binding 8 to 9 (two sites).
- `mod.rs`:
  - contract strings become `incant-temporal-upscale-v2` (`textures6,mrt3`) and `incant-temporal-upscale-owner-v2` (`6float-depth+2uint-textures,mrt=…-rg16f`);
  - the source pins are updated.
- `tests/temporal.rs`: emitted-token expectations add texture 5 / color 2 (UP) and texture 7 / color 3 (OWN).
- `contract.rs` negative controls: the OWN mutations now move the changes uniform from 9 to 10 and add an extra binding at 10, so each still exercises its intended refusal rather than a collision.
- Render-side `incant_render/src/temporal_shader/cooked.rs`: the expected contract strings are the two v2 strings.
- Host check: `incant_render/src/temporal_shader/cooked/host.rs` expects the matching layouts (UP 6 textures; OWN 8 textures with uint at 7 and 8, and a uniform at 9).

**Hash inventory** (`inventory.py` / `inventory.json`). Hashes are sha256 of the cooker composition `common.wgsl + "\n" + program`, and every pin equals the packet source:

| Program | Old | New |
|---|---|---|
| Upscale | `f78bccaaabd6…` | `9c515525e946…` (changed) |
| UpscaleOwner | `1b551b92bf1d…` | `c48f648d65b4…` (changed) |
| Composite | `81cfe7c59c72…` | unchanged |
| MetalFxPack | `8f08b6e0f442…` | unchanged |
| MetalFxNormalize | `0923e7ba4dc5…` | unchanged |

The full hashes are in `inventory.json`. `common.wgsl` is unchanged. These pins are Claude-authored and need root review before any cook.

**Admission costs (`plan.rs`, `motion/attachment.rs`):**
- Output stride is 44 B for UP and 60 B for OWN, with the 8 B overlay output extent and the 516 B per output light-grid cluster charged.
- Source extents use exact ceil. The ceiling stays `MAX_UPSCALE_TRANSITION_BYTES = 671105024`.

| Case | Admitted | Refused |
|---|---|---|
| Ordinary, full scale | 1694² | 1695² |
| Ordinary, 8/16 | 1939² | 1940² and 2048² |

- The records test vectors are 536,887,296, 640,106,496, None, None; fresh 511,721,472; flagged 1320.
- Queued and vendor allocations are excluded from these logical claims.

**Jitter:** `temporal/jitter.rs` `UPSCALE_SAMPLES` is the fixed lattice split by −1/16 (frames 0–3) and +1/16 (frames 4–7).
- It gives 8 distinct offsets per axis with zero mean. That is axis stratification only: a 0.05-px 45° line can miss all 8 samples (control 7).
- The fixed TAA `SAMPLES` and the default path are unchanged.

## 4. Controls (`controls.py` → `controls.json`, 29/29 pass)

These are scalar transcriptions of the shader rules, each with a negative (mutation) control. Each mutation below is detected:

- **Mass order:** 396 at density 1/16 gives 24.75; the mutation (per-tap cap 99) gives 6.1875. `H = 396/416` against the mutation's 99/119. The rule is shared by UP and both OWN branches.
- **Full support:** the outer HDR-green tap makes support ambiguous; the inner-only mutation would admit history.
- **Retirement and age:** ages run 1…8 and are then refused, and primary evidence resets the clock. The non-incrementing-age mutation would persist forever.
- **Retirement before blending:** with old age 7, current clear and reaction 0, no old colour or record remains on the eighth absent frame. The blend-before-test mutation keeps old colour above 0.9.
- **Carried primary:** previous primary 5 with current support 7 only gives `[5,7]` at ages 1…7, then refusal and a fresh `[7]` seed, leaving zero red. The fresh-primary-first mutation gives `[7,5]`, resets the clock, and red survives above 0.5 after 20 frames. Overflow `[5,6] + 7` keeps primary 5 and all classes.
- **Motion:** nine depth-5 samples with one moving outer contributor are refused; the anchor-only mutation admits them. Uniform motion is admitted. Previous-depth disagreement alone refuses.
- **Alpha-0 clear:** an alpha-0 sample with a positive depth is not surface or primary evidence.
- **Clear-inclusive coverage:** clear counts as coverage with zero colour, so red plus clear averages to half coverage.
- **Zero-mass bright tap:** it adds nothing to history.
- **Refusals:** an invalid origin, age or mass refuses history, and so does a changed secondary owner at 2⁻²⁰ weight.

## 5. Rationale and visual intent

- **Detail:** true-position accumulation lets output pixels gather sub-internal-pixel samples.
- **Coverage and edges:** clear-inclusive evidence gives honest partial coverage at edges.
- **Safety:** associated mass and density keep history mass physically scaled across resolution changes, which is the DRS-wipe strategy for colour mass. Full-support depth, motion and owner admission prevents misregistered or stale-owner colour.
- **Cost:** the 1/256-pixel motion agreement is strict. Perspective-varying motion across the 3×3 support (dolly, near rotation) will often fall back to current-only reconstruction. That is a correctness-first quality cost that must be measured in the 15-image comparison.

## 6. Remaining gaps: exact source paths, not implemented

1. **Internal texture LOD/MASK agreement:**
   - **Bias:** `bias = log2(source/output)`, which is negative at reduced scale; 0 for output-grid BLEND, sprites, text, the depth re-raster and fixed TAA.
   - **Material program:**
     - `model_draw.rs` Frame grows from 96 to 112 B (`lod: vec4f`) and its size asserts change;
     - `model_material.wgsl` uses `textureSampleBias` on its five samples (base, metallic-roughness, normal, occlusion, emissive);
     - `lib.rs` `prepare_models` (its only caller) passes the bias from the upscale plan, and `prepare_transparent_output` passes 0;
     - in the material cooker contract, `incant_shader_cook/src/geometry/material.rs` adds the field `("lod", 96, 1)` with size 112, and the geometry source pin changes;
     - the shadow host validator must be re-checked.
   - **Motion/depth and MotionOwner:**
     - the Frame grows from 144 to 160 B with `lod` (`motion/system.rs` min size; `motion/retained.rs` Frame);
     - `mask()` uses `textureSampleBias` in `motion/shader.wgsl` and `motion/owner.wgsl`;
     - the overlay and output frames pass bias 0;
     - the temporal contract's motion uniform names gain `lod`, and the Motion/Depth/MotionOwner pins change.
   - **Until then:** sub-internal-texel texture detail stays limited by the internal sampling rate. No texture improvement is claimed.
2. **DRS shadow treatment:**
   - **Not implemented.** Today the UP and moving OWN branches clamp to the current 3×3 box on DRS frames. That clamp is a biased bound, not a shadow proof.
   - OWN static persistence never runs on DRS frames, because the camera is not static.
   - **Required:** either explicit history invalidation when the shadow filter footprint depends on the internal extent (a host flag on DRS-change frames, at the cost of the DRS wipe), or a proven source-independent bound for the admitted DRS range.
3. **Oracle tests:** update `upscaling/kernel_oracle.rs`, `kernel_tests.rs` and `kernel_patterns.rs` to `UPSCALE_SAMPLES` with re-measured bounds. They still describe the fixed lattice.
4. **Device tests:**
   - **Already updated** in `upscaling/device_tests.rs` and `owner_device_tests.rs`: binding layouts, the RG16F views and the extra attachment.
   - **Still stale:** their numeric expectations predate the new estimator, so they would fail at run time.
   - **Needed:** expectations from an f64 kernel helper. OWN cases 7 and 8 need their baseline redefined.
5. **Moving partial coverage:** mixed layers and moving partial coverage remain current-only in both programs.
6. **Open failures:** all historical failures stay open and are not waived: ebc7e690, 9843a6c3, a5ae3179, GPU001 case 3, C7, C9, P1, the DRS wipe and thin geometry.

## Run commands and results

```sh
python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r4/controls.py \
    --out handoffs/0073-upscaling-review/reconstruction_implementation_r4/controls.json          # checks=29 failed=0
python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r4/patch_check.py \
    --out handoffs/0073-upscaling-review/reconstruction_implementation_r4/patch_check.json       # 21 files, applies to 76be
python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r4/inventory.py \
    --base /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant \
    --out handoffs/0073-upscaling-review/reconstruction_implementation_r4/inventory.json         # all pins match
```

- **Not run:** Cargo build, clippy, tests, naga validation, cook, GPU, render and image capture. All were prohibited by the packet.
- **Screenshots:** none were produced or read.
- **Scratch:** only `/tmp/incant-0073-reconstruction-r4-*` (temporary `common.wgsl` copies, deleted).
