# 0073 reconstruction, r2: corrected selected strategy

**Status:** source-only prescription for root and Astra to review. Nothing was built, run or rendered. Acceptance of this proposal is separate from rendered success.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP.

**Base:** production `76be9608e99bf089fcb1e0908827330f70c28ac1`. Its reconstruction WGSL, jitter and motion owner match ce6171957.

**Relation to d05ea011:** that proposal (`reconstruction_next/proposal.md`) is preserved unchanged and is **not** implementation-ready. Where they conflict, this document replaces its §3–7. Its scalar-model metrics were measured before the capped transient prior, so they are not evidence for the equations below.

**Preserved:**
- the accepted output-resolution sprite, world-text and BLEND route, its `clean()` sanitation, unjittered occlusion and output-grid lifetimes;
- default-off behaviour;
- every historical failure and threshold: ebc7e690, 9843a6c3, a5ae3179, GPU001 case 3, C7 0.0918/0.06, C9 0.683/0.869 and 0.741/0.943, and P1.

## 0. What reduced resolution can and cannot give

**Achievable at internal 50–75%:**
- **Detail:** output-space accumulation of true-position samples gives each output pixel samples from 8 jitter phases. Detail present in internal frames below the internal Nyquist rate can be placed at output resolution, so contrast can exceed DRS.
- **Thin features:** a 0.2-internal-px feature becomes sampleable once there are 8 distinct offsets per axis.
- **Silhouettes:** antialiased only where coverage history is provable (§3, §4).

**Unavailable from confidence alone:**
- **Texture detail filtered away before sampling:** 1-texel lines at internal-derivative LOD need the LOD dependency **D-L** (§4.5).
- **Never-sampled detail:** anything not sampled in any phase.
- **Exact FULL identity:** FULL stays a reference. The C7, C9 and P1 definitions are unchanged.

## 1. History colour and confidence from the same admitted support (finding 1)

**Admission:** for the bilinear taps `i` at the reprojected position `old`, with weight `b_i ≥ 0` (`tap_weight`), a tap is admitted when:
- it is inside the output rect;
- `admitted(c_i)`, and `c_i.a ≥ 1`, except for the OWN coverage rule in §4.2;
- `compatible(z_i, expected)`;
- for OWN, its record is admissible.

**Per admitted tap:**
- `e_i = stored_evidence(W_i)`, which is 0 unless finite and in [0, 65504], and at most `cap`. It is 0 when the evidence chain is invalid (§2).
- Confidence: `C = Σ b_i·e_i`.
- Numerator, premultiplied RGBA: `N = Σ b_i·e_i·clean(c_i)`.

**Normalisation and blend:**
- The history colour exists only if `C > 0`; then `H = N / C`.
- Weight: `W′ = min(C·density, cap)·(1 − strength·reaction)·A(f)`, where `A` is defined in §4.1.
- Scaled history numerator: `N·(W′/C)`, so colour and confidence always come from the same taps.
- With current evidence `S, m` (§3) and prior `E, k` (§4.3): `colour = clean((N·W′/C + S + k·E)/(W′ + m + k))` and `W_new = min(W′ + m, cap)`.

**Kernels:** only nonnegative weights ever carry confidence. Catmull-Rom (`sharp_history`) is removed from history; it remains only inside the transient prior `E`, which carries no confidence.

**Rejection and finite handling:**
- A rejected tap adds nothing to `N` or `C`.
- `C = 0` or non-finite means no history.
- A NaN or ±inf colour becomes `clean()` zero but keeps its confidence.

**Regression** (`scalar_controls.py` check 1): weights [.5,.5], colours [1,0], confidence [0,8], and `S = 0, m = 1` give **0**, against the 0.4 an independently filtered `H` would give.

## 2. Evidence-target lifetime (finding 2)

**Target:** `Output.evidence[2]`, R16Float at output extent, with the same Arc lifetime and slot alternation as `color` and `depth`.

**Validity flag:** a new host flag `State.evidence_valid` is uploaded as `frame.reactive.z` (0 or 1). The shader reads previous evidence only when it is 1; otherwise every `e_i` is 0.

| Event | Rule |
|---|---|
| Portable resolve frame accepted | `evidence_valid = true`: its resolve wrote `evidence[slot]` |
| Native MetalFX frame accepted | `false`: `Pending::native_input` exposes only colour and depth, and native `normalize` does not write evidence, so the next portable frame never pairs stale W with native colour |
| MetalFX selection change | Also sets `history_weight = 0`, so cap = 0 and W′ = 0 |
| New `Output` allocation (output resize, owner toggle) | `false` until a portable frame on that allocation is accepted |
| Cut or reset (`has_previous = false`) | History is unused, so the flag does not matter; the frame writes fresh W |
| Failed prepare or submission | `Pending` dropped, `State` unchanged, nothing published |
| Source-only DRS change on the same `Output` | Flag kept; W rescaled by `density` |

There is no silent hole. Native normalize, its shader and its ABI are unchanged; any future native evidence must set the flag explicitly.

## 3. Current evidence and provenance (finding 3)

**Current evidence `S, m`** uses the 3×3 internal texels around `floor(source_position(p))`:
- Taps outside the source rect are skipped. Each tap sits at its true output position `pos(t) = output.xy + ((t + 0.5 − source.xy)/source.zw − jitter.xy)·output.zw`.
- `w = norm·exp(−|p − pos|²/2ρ²)`, with `ρ = 0.35 + 0.325·max(r − 2, 0)` output px and `norm = r²/(2πρ²)`. These are the oracle values.
- A tap contributes only if it is admitted, opaque and on the anchor surface. In OWN, additionally, its owner must be the anchor's owner, or valid and unchanged.
- Changed or invalid non-anchor taps are **excluded with their whole weight**. Their colour never enters, so the near-positive case 3 keeps colour 0.

**Every contributing support joins the existing record semantics:**
- `with_id`: two exact IDs, otherwise an overflow class mask;
- canonical `(0,0)` for empty or unprovable;
- the 9 source taps, the prior `E`'s Catmull-Rom taps (existing `sharp_current` provenance: outer taps need valid, unchanged owners, otherwise bilinear fallback), every admitted history record (`with_record`), and the clamp-box owners.

**History records:**
- A history tap is admitted only if `r.a == anchor` and every other recorded owner is unchanged. This is the existing `record_admissible` rule, applied at **any** positive bilinear weight, including 2⁻²⁰ (check 9).
- Overflow records stay conservative by class.

**Static persistence (OWN) requires all of:**
- camera static (flags bit 0) with bit 1 clear;
- the 2×2 support valid and unchanged;
- **every** 3×3 tap with positive Gaussian weight either clear or owned by a valid, unchanged owner. Unlike d05ea011, which skipped such taps, any changed tap now disables persistence;
- the own-texel record canonical and `record_unchanged`;
- a static anchor;
- the opaque guard: with no surface in the 3×3, `H.a < 1` is required (checks 7a and 7b).

**Why moved or removed owners are safe:**
- A moved or posed owner sets its changed bit, so `record_unchanged` fails.
- An added or removed placement fails `motion/retained.rs::topology()` with `MotionError::Topology` before any history use.

**Dependency D-P, remaining and bounded:**
- **Gap:** a material-parameter or visibility change that alters neither pose nor topology is not marked changed.
- **Fix:** mark the owner changed in the existing changed table when the batch material version or instance visibility changes (`motion/retained.rs`). That is one bit per owner per frame, with no new buffer.
- **Until then:** only partial-coverage history can persist over all-clear pixels. The worst case is a thin ghost that decays by `W/(W + m)` per frame, at most `cap` frames.

## 4. Motion, thin geometry, disocclusion, DRS and LOD (finding 5)

### 4.1 Moving geometry

History is reprojected with the anchor's motion, as now.

**Resampling attenuation:** with `f = fract(old − 0.5)`, `A(f) = (1 − 2f_x(1−f_x))·(1 − 2f_y(1−f_y))`. It is exactly 1 for a static, integer reprojection and 0.25 at a half-texel offset in both axes (check 8). Each bilinear resample tent-filters history, so `A` lowers confidence in proportion to that blur, and fresh true-position samples restore detail.

### 4.2 Thin and sub-pixel coverage while moving (OWN only)

**Single-owner coverage reprojection:**
- In the ordinary OWN path, a history tap with `a < 1` is admitted **only** if its record is exactly `{anchor}` (`b == 0`, not overflow) and its depth is compatible.
- That covers a thin object over the backdrop moving with its own motion, so it no longer blinks. Its coverage history moves with it.
- A mixed record (object over another surface) stays current-only.

**UP, which has no provenance:** mixed and clear pixels stay current-only, so stair-steps in motion remain. **This is a stated limit, not a claim.**

### 4.3 Disocclusion and reset

- Rejected taps shrink `C`.
- The transient prior `E` is `sharp_current`, with weight `k = max(min(8, cap/2) − W′, 0)`. It is never counted in `W_new`, so it fades after one jitter cycle of real evidence.
- The 3×3 clamp stays for UP, and for OWN without the proof.

### 4.4 DRS

**Density:** `density = (old_source_w / new_source_w)²` in both directions, then capped at the new ratio's cap (checks 5a and 5b).

**DRS-change frames:** these are frames where `camera_static` is false because only the extent changed. The host sets the new flag `frame.reactive.w = 1` for them, and on those frames the clamp is **mean-only**:
- `H_low` is the mean of the admitted 3×3 previous texels around `round(old)`;
- `H′ = clamp(H_low, box) + (H − H_low)`;
- the coarse frame bounds low-frequency (lighting or shadow) drift without wiping detail it cannot resolve (checks 10a and 10b). This targets the first-4/16 wipe.

**Risk:** a high-frequency shadow-edge change caused by cascade quantization can survive for the reaction-weighted transient of those frames.

### 4.5 Texture LOD: essential for 1-texel detail, dependency D-L

**Change:**
- The models `Frame` uniform grows from 96 to 112 B and gains `lod_bias`. It is `−log2(r)` only when an upscale plan exists, and 0 otherwise, which keeps default behaviour.
- `model_material.wgsl` samples with `textureSampleBias`.
- The motion `mask()` sampling uses the same bias, so MASK coverage stays consistent.

**Cost:**
- default-path shader ABI and cooked material pin changes, reviewed separately;
- more texture aliasing in the internal frames, which accumulation must resolve.

**Without it:** sub-internal-texel texture detail stays grey. That is honest, not a placeholder.

## 5. Targets, passes and exact admission (finding 4)

**New target:** `evidence[2]`, R16Float, output extent, 2 B per pixel per slot, so +4 B per output pixel. It needs a startup format check that refuses explicitly if unsupported.

| Variant | Output bytes/px | Colour attachments | Bytes per sample (limit 32) |
|---|---|---|---|
| Ordinary | 36 → 40 | 8+4+2 | 14 |
| Owner | 52 → 56 | 8+4+8+2 | 22 |

**Passes and submissions:** none added. The resolve gains:
- one attachment;
- one binding, previous evidence at binding 6; OWN's owner, record and changes bindings move to 7, 8 and 9;
- 9 extra texel loads, only on DRS-change frames.

**Uniform:** `reactive.y` becomes density, replacing global mass (`frame_mass`, `State.mass` and `history_fraction` are removed). `reactive.z` becomes evidence-valid and `reactive.w` the DRS-frame flag.

**Exact logical admission:**
- **Extents:** `source = ceil(output·n/16)` per axis. Ordinary uses the `plan.rs` preflight, which charges old plus candidate with no reuse discount. Owner uses `admit_records`, with exact reuse. Both add the output light grid, old plus candidate.
- **Ceiling:** 671105024 B, unchanged.
- **Out of scope:** native, vendor and queued driver memory is excluded from logical admission and not bounded by it.

| Case | Ordinary 36 | Ordinary 40 | Owner 36 | Owner 40 |
|---|---|---|---|---|
| 2048² steady at 8/16 | fits | fits | fits | fits |
| 2048² 16/16 → 8/16 | fits | **refused** (+16,973,824 B) | fits | fits |
| 2014² 16/16 → 8/16 | fits | fits (670,677,240) | fits | fits |
| 2015² 16/16 → 8/16 | fits | **refused** (671,209,080) | fits | fits |
| 2560×1440 steady 16/16 | fits | **refused** | fits | fits |
| 1923² steady 16/16 | fits | **refused** | fits | fits |
| 1890² steady 16/16 | fits | fits | fits | fits |
| 2532×1170 16/16 → 8/16 | fits | fits | fits | fits |

- **Odd extents:** the 2015 case confirms the exact ceil source differs from the `n·n//4` prototype by 24,192 B. Both still cap the ordinary square at 2014. That is not a change in the stated maximum.
- **No ceiling raise:** the smaller ordinary maxima are an honest consequence of +4 B.
- **Root's choice:** accept them, or take the zero-byte option, which packs 8-bit evidence into the R32F depth mantissa. That widens the owner device-test depth tolerance from 1e-4 to 1e-3 at orthographic z 50.05, an explicit test-contract change.

## 6. Exact next patch (one patch, implementation order)

1. **Targets and admission:**
   - `upscaling/targets.rs`: `Output.evidence` and `bytes()` 40/56.
   - `upscaling/plan.rs`: `OUTPUT_PIXEL_BYTES = 40`, with exact tests.
   - `motion/attachment.rs`: stride 40, recomputed totals.
   - `upscaling/owner.rs`: `evidence_valid`, the density and DRS flags, mass removed.
   - `upscaling/system.rs`: R16Float format check, layouts (binding 6; OWN 7–9), the evidence attachment, uniform fields.
   - Native MetalFX: no code change; validity is handled by the flag.
2. **`upscaling/common.wgsl`:** `kernel_weight`, `evidence_cap`, `stored_evidence`, `prior_weight`, and a `same_support_history` accumulator.
3. **`upscaling/resolve.wgsl` (UP):** §1, §3 evidence, §4.1 attenuation, §4.3 prior, §4.4 density and mean clamp; output `@location(2) evidence`.
4. **`upscaling/resolve_owner.wgsl` (OWN):** the same, plus §3 provenance, static persistence and §4.2 single-owner coverage; `@location(3) evidence`.
5. **Jitter:** `temporal/jitter.rs`, `motion/owner.rs`, `motion/retained.rs` and `retained.rs` get the upscale-only split sequence (d05ea011 §3E, unchanged). The fixed path keeps `SAMPLES`.
6. **Tests:**
   - `scalar_controls.py` checks 1–10 as Rust unit tests;
   - device expectations from an f64 kernel helper: UP's 19 modes, and OWN cases 3, 7/8 (UP equality without the proof), 11 and 12;
   - kernel and confidence oracle tests updated for symmetric density and split-jitter mass, with exact measured bounds.

**Cooker (root-owned):** new trusted pins for the `Upscale` and `UpscaleOwner` sources and ABI. Composite, MetalFX pack and normalize are unchanged; they share `common.wgsl`, so names must not collide.

**Separate later patches:** D-L (§4.5) and D-P (§3).

## 7. Rendered protocol

**Later grant:** the same 15 images and five groups. The motion group's camera pan already moves the antenna wires, which is the §4.2 observation. The stationary phases, transitions strip and C7/P1 cover the rest.

**Genuinely missing control (optional):** a thin wire over a surface, not the backdrop. Add it only if root wants it.

## Limitations

- **Unbuilt and unrendered:** the analytic controls (21/21) transcribe equations; they are not renders or gates.
- **Remaining limits:**
  - UP motion silhouettes;
  - mixed-owner thin features in motion;
  - shadow-edge risk on DRS-change frames;
  - D-L and D-P.
- **Budget:** smaller ordinary maxima at +4 B.
- **Scratch:** none outside this packet. No memory was read or written.
