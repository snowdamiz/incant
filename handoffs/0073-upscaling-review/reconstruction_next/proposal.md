# 0073: next temporal reconstruction repair (source-only proposal)

**Status:** this is a proposal for root and Astra to review before any implementation, build or GPU run. Nothing was built, run or rendered in this turn, and no production source changed.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP.

**Base:** production `76be9608e99bf089fcb1e0908827330f70c28ac1`. Its `upscaling/*.wgsl`, `temporal/jitter.rs` and `motion/owner.rs` are byte-identical to ce6171957. Only owner, plan, system and target accounting changed for the output overlays.

**Preserved:**
- the output-resolution sprite, world-text and guarded BLEND route;
- the sanitation (the composite's `clean()`);
- unjittered occlusion;
- output light-grid ownership;
- the 671105024 B logical ceiling;
- default-off behaviour.

## 1. Evidence (observed)

**Rendered-output evidence.** These come from the 15-image run (`0073-owner-output-quality-gpu-001`, my result `0e169e36`) and the rejected repair (`0073-owner-quality-gpu-002`, `9843a6c3`):
- **E1, thin wires:** in UP and OWN, the 0.8 px wire appears in about half of the 8 jitter phases and the 0.4 px wire is absent (`stationary-phases.png`, byte-identical in both runs). In full frames only the 1.6 px wire is stable, and the diagonal is a dotted line.
- **E2, chart detail:** the 2-texel checker and 1-texel lines are near-flat grey in UP and OWN, while DRS shows aliased contrast. The repair run measured 2-texel luminance SD (sRGB codes) of UP 4.6, OWN 7.0, DRS 26, FULL 53, and 1-texel lines at 1.4 and 0.3.
- **E3, silhouettes:** sphere, mast and horizon silhouettes are stair-stepped in UP and OWN.
- **E4, transitions:** the first 4/16 frame wipes the chart to grey, and recovery to 16/16 happens over about 3–4 frames.
- **E5, motion, C7 and P1:** byte-identical between the two runs. C7 worst trailing column, UP: 0.081, 0.095 and 0.062 (codes ÷ 255) at steps 12, 16 and 20.
- **E6, OWN versus UP:** OWN is nearly indistinguishable from UP in every pair; the OWN-versus-UP frame-24 interior mean absolute difference is 0.0097 codes.

## 2. Diagnosis (inferred from the exact source)

These are the source mechanisms behind each failure, all in production `resolve.wgsl` and `resolve_owner.wgsl` unless stated otherwise.

**D1: each frame contributes an interpolated estimate, not samples.** The colour is `mix(sharp_current(q), clamp(history), history_fraction(...))`. `sharp_current` is a Catmull-Rom or bilinear interpolation of the internal frame at the jittered position. A blend of interpolations converges to the scene convolved with the interpolation kernel and the jitter spread. It cannot place a sample's energy at its true sub-pixel position, so the result stays at or below DRS-level sampling detail. This explains E2.

**D2: one global weight for every pixel.** `history_fraction` uses one host scalar (`reactive.y = frame_mass`) for all pixels. Freshly disoccluded pixels and converged pixels get the same weight, so convergence after a cut or disocclusion and steady-state stability cannot both be good. This contributes to E2 and E5. It is global mass, not per-pixel confidence.

**D3: mixed and clear pixels never accumulate.** `if !same_surface {return result;}` makes every 2×2 footprint that touches clear or a second depth current-only, with depth 0 and no record. Every silhouette and every thin feature is re-sampled from scratch each jitter phase, which explains E1 and E3.

**D4: the history clamp wipes detail.** The per-frame 3×3 internal-neighbourhood clamp applies in UP always, and in OWN unless its static proof holds. At Nyquist-level detail some phases yield a narrow box, so the clamp removes accumulated detail. On DRS-change frames `camera_static` is false by design, so even OWN clamps against the coarse 4/16 frame. This explains E4 and part of E2.

**D5: the jitter lattice leaves gaps.** `temporal/jitter.rs` `SAMPLES` has only 4 distinct offsets per axis, 0.25 internal px apart. A 0.2-internal-px feature (0.4 px at 2×) falls between all rows for about 20% of sub-pixel positions; the fixture's 0.4 px wire is one of them. This explains E1 for the thinnest wire.

**D6: textures are filtered at internal resolution.** Material sampling uses internal-resolution derivatives and has no negative LOD bias. At 2× the sign's 1-texel lines are already averaged to grey in the internal frames, so no temporal method can recover them. This is a separate default-path dependency (§8).

**D7: OWN's provenance is wasted.** OWN uses its proof only to skip the clamp on fully opaque, same-surface pixels, with the same interpolated estimate and global weight. This explains E6.

## 3. Selected repair: per-pixel evidence accumulation

This is one coherent change. Its kernel and confidence semantics are the in-tree `#[cfg(test)]` design oracles that the shader never implemented (`upscaling/kernel_oracle.rs`, `confidence_oracle.rs`). Their exact formulas follow.

**A. Gaussian evidence at true sample positions (UP and OWN, `common.wgsl`).** For output pixel `p`, internal ratio `r = output.z/source.z`, and internal texel `t` in the 3×3 around `floor(source_position(p))`:
- Taps outside the source rect are skipped, never clamped.
- `pos(t) = output.xy + ((t + 0.5 − source.xy)/source.zw − jitter.xy)·output.zw` is the true jittered sample position.
- `ρ = 0.35 + 0.325·max(r − 2, 0)` output px, and `norm = r²/(2πρ²)`, so the expected mass per frame is 1.
- `w(t) = norm·exp(−|p − pos(t)|²/(2ρ²))`.
- `S = Σ w·clean(c_t)` and `m = Σ w` over admitted taps.

**B. Per-pixel evidence weight `W`.**
- **Storage:** a new pair of output-sized R16Float targets, `evidence[2]`, alternating slots like colour and depth.
- **Cap:** `cap = h/(1−h)·min(r², 4)`, where `h` is `history_weight`, the oracle cap. `h` now bounds the evidence cap, and the old colour's share `W/(W + m + k)` stays below 1. This is a documented reinterpretation of the `TemporalConfig::history_weight` doc for the upscaler only; the fixed path is unchanged.
- **Reprojected weight:** `W' = min(Σ_admitted bilinear_wt·W_tap · density, cap)·(1 − strength·reaction)`. Rejected taps remove their share.
- **Density:** `density = (old_source_w/new_source_w)²` on every extent change. This is symmetric, and deviates from the oracle, which keeps W on decreases. It converts W to units of the current sample density.
- **Prior:** `k = max(min(8, cap/2) − W', 0)`, applied to `E`, the existing interpolated `sharp_current` estimate. The prior is not counted as evidence, so it only fills the first jitter cycle after a reset or disocclusion. This answers the D2 transient (§9 model).
- **Blend:** `colour = clean((W'·H + S + k·E)/(W' + m + k))`, then `W_new = min(W' + m, cap)`.

**C. UP keeps its safety.** UP has no provenance, so:
- the reprojected history still needs opaque, depth-compatible taps;
- the 3×3 clamp is kept;
- mixed and clear pixels stay current-only (bilinear coverage, depth 0, W 0).

UP's gain is therefore limited to W semantics, true-position evidence and the first-cycle prior; its Nyquist detail stays clamp-bounded.

**D. OWN's static-proof coverage persistence (`resolve_owner.wgsl`).**

*The proof:* `has_previous` and camera static (flags bit 0), and no light or caster change (bit 1 clear). The 2×2 support owners must be valid and unchanged. If an anchor exists, its motion must be finite and valid with `|old − p| < 1/256`.

*Persisted history:* when the proof holds and the own-texel history `H` passes all of these, it is persisted for **any** support kind (opaque, mixed or all-clear) with no depth test and no clamp:
- admitted: finite, nonnegative, `a ≤ 1`, partial coverage allowed;
- canonical and present record;
- `record_unchanged` against support plus evidence classes;
- **opaque-history guard:** if no surface tap is present in the 3×3, `H.a < 1` is required, so a fully covered history never persists over an all-clear neighbourhood.

*Coverage evidence:* the evidence is coverage evidence over every 3×3 tap. Clear taps count as zero colour at full weight. A surface tap counts only if its owner is valid and unchanged; otherwise it is skipped.

*Record and depth:* the record is the contributing owners, anchor first, plus the persisted record. Depth is the 2×2 nearest if one exists, otherwise the persisted depth. An empty set writes record `(0,0)`, W 0 and depth 0.

*When the proof fails:* the OWN ordinary path is UP's arithmetic plus provenance. Outer taps whose owner is neither the anchor's nor valid and unchanged are skipped. The near-positive case 3 therefore keeps colour 0, and cases 7 and 8 keep byte equality with UP.

**Why this is the next step:** this is the provenance-based thin-geometry and edge accumulation that UP cannot safely do. It addresses D3, D4 and D7 in static scenes.

**E. Upscale-only split jitter (`temporal/jitter.rs`, `motion/owner.rs`, `motion/retained.rs`, `retained.rs`).**
- `UPSCALE_SAMPLES` is the lattice with frames 0–3 moved by (−1/16, −1/16) and frames 4–7 by (+1/16, +1/16).
- That gives 8 distinct offsets per axis, zero mean, and each half cycle still covers all four 2× quadrants.
- The worst per-cycle kernel-mass deviation is 0.127 at n = 8, against the lattice's 0.073. A free rook pattern measured 1.2–1.7.
- Every 0.2-internal-px band is sampled at least once per cycle, which addresses D5.
- The fixed path keeps `SAMPLES`. `MotionFrame` gains a private jitter offset, so the camera, previous-jitter and MetalFX consumers all agree.

**Sketch** (exact names for the next patch; the logic mirrors the preserved draft at `owner_quality_reconstruction`, 372978f3):

```wgsl
fn kernel_weight(p:vec2f,t:vec2i)->f32 { let r=frame.output.z/frame.source.z; let rho=0.35+0.325*max(r-2.0,0.0);
  let d=p-(frame.output.xy+((vec2f(t)+vec2f(0.5)-frame.source.xy)/frame.source.zw-frame.jitter.xy)*frame.output.zw);
  return r*r/(6.283185307*rho*rho)*exp(-dot(d,d)/(2.0*rho*rho)); }
fn evidence_cap()->f32 { let h=clamp(frame.options.x,0.0,0.99); let r=frame.output.z/frame.source.z; return h/(1.0-h)*min(r*r,4.0); }
fn stored_evidence(w:f32)->f32 { if !(w>=0.0 && w<=65504.0) {return 0.0;} return min(w,evidence_cap()); }   // NaN -> 0
fn prior_weight(w:f32)->f32 { return max(min(8.0,0.5*evidence_cap())-w,0.0); }
fn accumulate(h:vec4f,w:f32,s:vec4f,m:f32,e:vec4f,k:f32)->vec4f { return (h*w+s+e*k)/(w+m+k); }  // callers clean()
```

`frame.reactive.y` changes meaning from global mass to the density factor. `State.mass`, `frame_mass` and `history_fraction` are removed.

## 4. History retained, rejected and reconstructed

| Situation | UP | OWN |
|---|---|---|
| Opaque, same surface, admitted reprojection | reprojected H clamped, W′ from taps | same, with provenance (unchanged rules) |
| Static proof holds | as above (no proof) | own-texel H persisted unclipped, W′ = stored W, any support kind |
| Mixed or clear, no proof | current-only bilinear, depth 0, W 0 | same, record (0,0) |
| Disocclusion, depth or record mismatch | taps rejected, W′ shrinks by their weight | same |
| Reset or cut | W′ = 0, prior fills one cycle | same |
| Partial-coverage history (`a<1`) | never admitted (needs `a ≥ 1`) | only through the static proof |
| Light or caster change (bit 1), or DRS-change frame | n/a | no proof, ordinary path |

## 5. Changing extent, rect and accumulated footprint

**Rect and kernel:** the output rect is fixed under DRS, and the source rect is `rect·n/16` exactly (`Plan::new`). Sample positions use the true source rect and jitter, so odd and fractional extents need no aspect guess. Taps outside the source rect are skipped and m renormalizes.

**Scale changes:** history keeps its output-space footprint. W is rescaled by the density ratio in both directions and then capped at the new ratio's cap. For example, 4/16 → 16/16 gives W′ = W/16, so the CR prior and new samples dominate (fast recovery, visible pop). In the other direction, 16/16 → 8/16 gives W′ = min(4W, cap), so the sharp history persists while coarser evidence accumulates (no softening pop).

**DRS-change frames:** they still lose the OWN proof by the existing `camera_static` rule, so the first 4/16 frame is still clamped in both UP and OWN. The E4 wipe is therefore **not** removed by this patch (§8 dependency).

## 6. Thin coverage, finite premultiplied RGBA and provenance

**Coverage:**
- Clear taps are zero-colour evidence, so OWN's persisted colour is the time-averaged premultiplied coverage, an antialiased wire.
- `a < 1` history is produced only under the proof and is never admitted by ordinary reprojection.
- Over-coverage cannot arise: `a = (W·Ha + Σw·a_t + k·Ea)/(W+m+k)` is a convex combination of values in [0,1].

**Finite RGBA:**
- Every texel read goes through `clean()` or `admitted()`, and every write is `clean()`.
- Stored W is sanitized, so NaN, ±inf and negative values become 0.
- Alpha ≤ 0 becomes clear.
- The opaque path forces alpha to 1.

**Provenance:**
- Every tap whose colour contributes joins the record, as do the clamp-box owners (unchanged).
- Invalid outer IDs are skipped.
- The changed-owner rules are unchanged.

## 7. Passes, targets, bytes and admission (logical; queued and vendor memory excluded)

**Passes:** none added. The resolve gains one colour attachment, the R16Float evidence write. It also gains one texture binding: previous evidence at binding 6. OWN's owner, record and changes bindings move to 7, 8 and 9.

**Bytes per sample:**

| Variant | Attachments | Bytes per sample (limit 32) |
|---|---|---|
| UP | 8+4+2 | 14 |
| OWN | 8+4+8+2 | 22 |

**Per output pixel:**

| Variant | Production | Proposed |
|---|---|---|
| Portable output | 36 B | 40 B (+2×2 B evidence) |
| Owner output | 52 B | 56 B |

The output-extent overlay (8 B), source motion (24 B, plus 4 for owner) and the output light grid (516 B per 64²·24 cluster) are unchanged.

**Admission:** `MAX_OUTPUT_BYTES` and `OUTPUT_PIXEL_BYTES` go from 36 to 40, with the exact tests recomputed. There is no ceiling or per-target-limit increase.

**Exact consequences (old plus candidate, both grids):**

| Case | Production | Proposed |
|---|---|---|
| 2048² steady at 8/16 | 579,026,944 B, fits | 612,581,376 B, fits |
| 2048² 16/16 ↔ 8/16 transition | 654,524,416 B, fits | 688,078,848 B, **refused by 16,973,824 B** |
| Largest square for that transition | 2048² | 2014² |
| Largest steady full-scale square | 1942² | 1890² |
| 2560×1440 full scale | fits | **refused** |
| 2532×1170 and 1920×1080 | fit | still fit |

R8Unorm evidence (38 B) would still refuse 2048² by 196,608 B.

**Zero-byte alternative, if root rejects the smaller maxima:** pack an 8-bit W into the low 8 mantissa bits of the existing R32F history depth. The depth error is ≤ 2⁻¹⁵ relative, well inside `compatible()` tolerances. However, the owner device-test probe depth tolerance (1e-4 absolute at the orthographic 50.05) must widen to 1e-3. That is a test-contract change I do not recommend making silently.

## 8. Achievable versus FULL, and dependencies

**Usable reduced-resolution targets for this patch (OWN, static):**
- 2-texel contrast at or above DRS, with low activity;
- all fixture wires present in every phase, with phase range ≤ 25% of the mean;
- antialiased static silhouettes.

**UP:** no regression and a modest medium-detail gain.

**Physically unachievable without a dependency:** FULL-level 1-texel lines (D6).

**Dependency 1, LOD bias:** a `textureSampleBias(…, −log2(r))` LOD bias in `model_material.wgsl` under upscaling only. It needs a material-uniform ABI change, cooked material re-pins and a default-path review. The scalar model, run at the draft cap/2-prior parameters, estimates OWN 1-texel SD 0.006 → 0.085 (FULL 0.217). It is not part of this patch.

**Dependency 2, DRS-change frames (the E4 wipe):** allowing the OWN proof on DRS-change frames would need a reviewed source-size-independent shadow-guard rule in `motion/owner.rs` `camera_static`. That is out of scope.

**Not solved here:**
- moving thin geometry, because the proof needs a static camera and owners;
- moving silhouettes in UP;
- P1, which is unchanged.

## 9. Controls and rendered protocol

**CPU and unit controls (root builds):**
1. `kernel_tests` and `kernel_patterns` switched to `UPSCALE_SAMPLES`. The per-n mass bounds are replaced by measured exact maxima (deliberate contract change). A stronger control asserts that every 0.2-internal-px band is sampled once per cycle; the lattice fails it.
2. `confidence_oracle` with symmetric density, plus a test that a 16→8→16 round trip restores W.
3. Rewritten device expectations computed from an independent f64 kernel helper:
   - UP 19-mode;
   - OWN case 3 (near-positive: colour 0, record either);
   - cases 7 and 8 (UP and OWN byte equality without the proof);
   - case 11 (invalid IDs; a non-opaque probe stays current-only without the proof and accumulates with it);
   - case 12 (an all-clear neighbourhood with opaque history stays all clear).
4. **Negative controls:**
   - a changed recorded owner rejects persistence;
   - partial-coverage history is never admitted by UP or ordinary reprojection;
   - NaN, inf or negative stored W is read as 0;
   - alpha-zero is canonical;
   - admission refuses exactly at the boundaries in §7.

**Scalar model:** the preserved draft (`owner_quality_reconstruction/controls/reconstruction_model.py`, 372978f3). These are model observations, not renders:

| Measure | ce617 | Proposed |
|---|---|---|
| OWN 2-texel SD, frame 24 (DRS 0.082, FULL 0.261) | UP 0.028, OWN 0.041 | 0.091 |
| Wire min presence across phases, OWN (measured with the cap/2 prior variant; not re-run at the min(8, cap/2) prior) | 0 for all three | 0.79, 0.40, 0.33 |
| C7 trailing (UP), steps 12/16/20 | 0.056/0.052/0.050 | 0.066/0.032/0.051 |

The proposed C7 row is mixed, so no C7 improvement is claimed. The pre-repair ebc baseline was 0.033/0.030/0.040.

**Rendered protocol:** the same 15 images and five groups, unchanged. `stationary-phases` and `stationary-detail` cover wires and the chart, the transitions strip covers E4, and c7/p1 cover regressions.

**Optional missing control (proposed only if root wants it):** a thin wire over a *surface* rather than the backdrop. The static proof's no-depth-test persistence exists for exactly that case, and the current fixture has no instance of it.

## 10. The one next patch, and implementation order

1. **Evidence targets and accounting:** `targets.rs` (evidence[2]), `plan.rs` and `motion/attachment.rs` (36 → 40, 52 → 56, exact tests), `owner.rs` (drop mass, density factor). Also add the R16Float startup format check in `system.rs`.
2. **`common.wgsl` and `resolve.wgsl` (UP):** evidence, W, prior and blend; binding 6; Resolved @2.
3. **`resolve_owner.wgsl` (OWN):** the proof path, coverage evidence, ordinary path with provenance; bindings 6–9; @3.
4. **Jitter:** `jitter.rs`, `motion/owner.rs`, `motion/retained.rs` and `retained.rs` for `UPSCALE_SAMPLES`.
5. **Tests:** the controls in §9.

**Cooker contract (root-owned):** the `Upscale` and `UpscaleOwner` shader sources and ABI change: an extra target and a binding renumber. Both need new trusted pins in `incant_shader_cook`. `Composite` and MetalFX `Normalize` are unchanged, though they include `common.wgsl`, so no names may collide.

**Proof check before the patch:** for `UpscaleOwner` the shader must be validated with naga, because it is not compile-checked here.

Root and Astra then compile and run the CPU tests, followed by one granted 15-image run.

## Limitations

- **Unbuilt:** this is a proposal only.
- **Model scope:** model numbers are scalar f64 observations on synthetic inputs, not rendered results, gates or appearance claims.
- **Budget:** R16Float evidence lowers the maximum admissible extents as stated in §7.
- **Open failures:** E4 and D6 remain dependencies. Every historical failure stays open (ebc7e690, 9843a6c3, a5ae3179, GPU001 case 3, C7 0.0918/0.06, C9 0.683/0.869 and 0.741/0.943, P1).
- **Scratch:** only `/tmp/incant-0073-reconstruction-proposal-budget` (budget arithmetic). No memory was read or written.
