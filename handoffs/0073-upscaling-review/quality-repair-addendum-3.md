# 0073 quality repair addendum 3: bounded provenance support

**Status: bounded design correction for root review, CPU only.** No renderer
source, GPU, Cargo, build, target or capture was used, and nothing is
implemented. These stay byte-exact:
- addendum 2 (`quality-repair-addendum-2.md` `4ece422d…`);
- `repair-model/model_v2.py` (`57295562…`) and `results-2.json` (`cb9c256e…`);
- every earlier record.

This addendum replaces only the parts of addendum 2 named in section 9.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-provenance-support-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs:**
- root's priority revision in `brief.md` (`8157b230…`, uncommitted);
- root's witness `upscaling-full-support-counterexample-20261013.json`
  (`47cf05d8…`);
- 0074's `native-coverage-repair-addendum.md` (`ecb7bba3…`), read only;
- `kernel_oracle.rs` and `common.wgsl` at 2fe23dce, for the window and tap
  positions.

**Evidence:**
- [`repair-model/witness_v3.py`](repair-model/witness_v3.py) and its output
  [`repair-model/results-3.json`](repair-model/results-3.json);
- one static output pixel with the real 8-phase jitter and kernel, plus an
  exhaustive extent check;
- 12 cases, 5 of them expected failures, none unexpected;
- no image simulation, and no re-run of the earlier suites.

## 1. Root's decisions, applied

| Decision | Applied as |
|---|---|
| Changed-caster footprints, not per-light signatures | The signature alternative (addendum 2 §3.4) is withdrawn |
| Retain exact provenance | Provenance is required, including through clip-in-place frames (section 5) |
| Compare accepted joint palettes when already available | Otherwise an animated owner counts as changed every frame |
| C7 and C9 failures and the unresolved P1 stay explicit | Section 8. No threshold is reduced, and no appearance acceptance is claimed |

## 2. The support counterexample holds

**Root's case (S1):**
- **Setup:** 16/16, ρ 0.35, `R` 1.4. A 65504 emitter that casts no shadow sits
  in [1.3, 1.45] × [−0.95, −0.8] relative to P.
- **Why it is never tracked:** only phase 0 samples it, through the corner tap at
  (1.375, −0.875). That is 1.63 px away, outside `R`, at a weight of
  2.54·10⁻⁵ against the peak's 1.30.
- **Contribution:** 1.66495 per hit.

`witness_v3.py` reproduces root's values bit for bit:

| Frame (emitter removed at 505) | 504 | 505 | 512 | 519 |
|---|---|---|---|---|
| Addendum 2: owners tracked only within `R` (expected fail) | 0.29309 | 0.26420 | 0.12633 | 0.06040 |
| This addendum: every positive-weight tap tracked | 0.29309 | 0 (dropped) | 0 | 0 |

**The cause.** The kernel sums every positive-weight 3×3 tap. Addendum 2's
owners, changed checks, stored depths and footprint extent stopped at `R`.
A small kernel weight bounds the omitted mass, not the radiance, and HDR
values go up to 65504.

## 3. Kernel, clamp and admission radii versus provenance support

| Role | Taps used |
|---|---|
| Kernel sum and mass | All valid 3×3 taps (unchanged) |
| Rectification box | Near taps, `d ≤ R = max(r, 4ρ)` (unchanged) |
| Primary, motion source and moving-path same-motion set | As in addendum 2: near taps (unchanged) |
| **Provenance:** recorded owners, changed checks, stored support depths, footprint extent | **All valid 3×3 taps with depth > 0 (the support)** |

The kernel is not truncated, and the clamp radius is unchanged.

## 4. Exact support extent (E1)

**Derivation.** These are the oracle's definitions (`kernel_oracle.rs`
`Kernel::taps`/`position`; `common.wgsl` `source_position`):
- `s = source.xy + ((P − output.xy) / output.zw + J / source.zw) · source.zw`;
- the window is `t ∈ floor(s) + {−1, 0, 1}`;
- each tap sits at `X_t = output.xy + ((t + 0.5 − source.xy) / source.zw − J / source.zw) · output.zw`.

So `X_t − P = (t + 0.5 − s) · k`, with `k = output.zw / source.zw = 16/n`
exactly (FramePlan's exact scale). Because `t + 0.5 − s ∈ (−1.5, 1.5]`, every
tap lies within `1.5k` of P per axis, and within `1.5√2·k` in distance.
- **Holds for:** any jitter, fractional origin or clipped viewport. Clipped taps
  are skipped, never moved.
- **Rounding margin:** in f32, `floor(s)` can shift the window by one texel when
  `s` is within rounding of an integer. For `s < 8192` one f32 ulp is at most
  2⁻¹¹ texel, so the extent uses `1.5 + 1/64` texels, which covers many ulps.

**E1 checks it** over n = 4…16, four fractional origins (37×29 rectangles),
every pixel centre and all 8 phases. The largest per-axis offset is exactly
1.5 texels, and the largest radius 2.1213 (= 1.5√2).

| Scale | 16/16 | 12/16 | 8/16 | 4/16 |
|---|---|---|---|---|
| Support radius, `√2·(1.5 + 1/64)·k` px | 2.14 | 2.86 | 4.29 | 8.57 |
| Near radius `R` px | 1.4 | 1.4 | 2.0 | 4.0 |

## 5. Corrected per-pixel rules

These replace addendum 2 §5.1 steps 1, 3 and 4. Step 2 (static), step 5 and
moving-path admission are unchanged.

**Taps:**
- the near taps give the box, front owner and admission;
- the support taps give their owners, with changed bits (bit 31 of the owner
  target), and their view depths.

**Static pixel, canonical history, exact owners (at most 2 recorded):**
- **Persist (unclipped)** when no recorded owner is changed, no support owner is
  changed, and the frame-global flag is clear. Then apply the footprint test.
  Owners become recorded ∪ support.
- **Drop** when a changed recorded owner has no *near* tap. A support-only owner
  (S1) is never near, so any change to it drops. That is exact: the history
  restarts from current samples.
- **Otherwise clip in place.** Reset `W ← min(W, 0.125)` if a changed owner is
  present in the support or the frame-global flag is set; otherwise apply the
  footprint test. Owners become recorded ∪ support, not the support alone (U1).

**Static pixel, overflowed owners (section 7):**
- `M` = the stored class mask ∪ the support owners' classes;
- `C` = the frame's changed-class mask.
- **Drop** if `M ∩ C ≠ ∅`.
- **Otherwise** no contributor changed, so the pixel persists like the exact
  case, with the footprint test. If the frame-global flag is set, it clips in
  place with a reset instead. This replaces addendum 2's depth fallback and its
  permanent clip in place.

**Kept histories** (persisted or clipped in place) merge the support range,
taking the smaller minimum and the larger maximum, and keep the smaller
`n_min`.

**Unbounded history (section 6):** on a static frame where any light has a
rectangle, drop. Otherwise the rules above apply.

**Moving pixel:**
- **Admission:** unchanged.
- **Owners:** support ∪ the admitted taps' owners or masks.
- **Unbounded flag:** set if any history tap was admitted, clear otherwise.
- **Support depths, front owner and scale:** the current frame's.

**Drop or fresh history:** owners come from the support, the unbounded flag is
clear, and support depths and scale are the current frame's.

## 6. Footprint extent and when its proof holds

**For a bounded history** (no moving-path ancestry since its last drop):
- **Depth range:** `d_lo` and `d_hi` span the stored support range and the
  current support depths, with clear excluded.
- **Grown box:** the bounding box of P's ray points at `d_lo` and `d_hi`,
  projected to the light's xy, grown by
  `√2 · (1.5 + 1/64) · 16/n_min · pixel_world(d_hi) + 2.25 · t`.
  `n_min` is the history's smallest scale numerator since its last drop
  (stored in meta `x`, section 9).
- **Containment:** every contributing sample lies on a ray within
  `(1.5 + 1/64)·16/n_min` px per axis of P, at a view depth in
  `[d_lo, d_hi]`. It is therefore within that radius × `pixel_world(depth)`
  of P's ray, and `pixel_world` is largest at `d_hi`. Projection to light xy
  cannot lengthen distances, so the grown box contains every contributor. The
  2.25 texels and A1–A5 are unchanged.

**Old-view depths.** Every output pixel is written every frame. A static frame
therefore reads support depths written in the previous frame, whose view equals
the current one. Samples from older views reach a history only through moving
admission, and that sets the unbounded flag. So a static frame never evaluates
the slab with depths from an older view.

**Unbounded histories:** the stored bounds cannot express their support, so no
"cannot miss" claim is made for them. Instead, the first static frame on which
any light has a rectangle drops them, and from then on they are bounded.
- **B1** (algebra, 16/16): a pan leaves 20% of surface G in P's history, and
  after the stop G lies 5 px from P's ray. A caster rectangle covers G, which
  goes from lit 1.0 to shadowed 0.25.
- **Without the flag**, both stored-bound tests miss it: half-widths of 2.53 px
  (addendum 2) and 3.27 px (bounded support). The excess is 0.150 → 0.072 →
  0.034 at frames 0, 7 and 14.
- **With the flag:** 0.

**Alternative, not adopted:** clip and reset instead of a drop. The flag never
clears without a drop, so while casters move the pixel would stay at about 11%
history weight indefinitely.

## 7. Owner overflow

**Encoding.** When more than two owners are recorded:
- meta `y` sets bit 31;
- bits 0–30 hold the class mask: bit `id mod 31` for every owner recorded since
  the last drop;
- `x` keeps the front owner for moving admission.

**Why this is a proof, not hash identity.** Classes partition the ids, and the
mask records exactly which classes have contributed.
- If no changed owner's class is in the mask, no contributor changed. That is
  exact.
- A class match never proves anything is unchanged. It causes a drop.

**O1:**
- **Setup:** a white strip, the black background and the far emitter make three
  owners; then the emitter leaves.
- **Addendum 2's overflow rule:** the front owner is unchanged, nothing changed
  is present, so the pixel clips in place without a reset. The box is [0, 1], so
  the emitter's share stays: 0.264 → 0.126 → 0.060 at frames 505, 512, 519
  against a matched reset.
- **The mask:** drops at 505, giving 0.

**Cost (reported):**
- A non-contributor whose class collides with a recorded one (owner 32 against
  owner 1) causes an unnecessary drop at frame 300. One with a free class (40)
  does not.
- Once a frame's changed owners cover all 31 classes, every overflowed pixel
  drops. That needs at least 31 changed owners.
- With the support wider than `R`, overflow is more frequent at owner junctions.

## 8. Controls: improved, unresolved and failed

| Control | Old result | This addendum |
|---|---|---|
| S1 far-tap emitter (root's witness) | 0.126 at +7, 0.060 at +14 | 0 |
| O1 unrecorded third owner leaves | 0.264, 0.126, 0.060 | 0 |
| U1 provenance through a clip-in-place frame: an owner arrives in a phase that misses the emitter and stays; the emitter leaves two frames later | 0.0117, 0.0045, 0.0021 | 0 |
| B1 moving-path history after a stop (algebra) | 0.150, 0.072, 0.034 | 0 |
| E1 support extent | — | max per-axis offset exactly 1.5 texels; max radius 2.1213 |

These are the new controls that demonstrably improve. They are separate from
the earlier controls.

- **P1 stays unresolved.** Its worst residuals (0.349, 0.221, 0.296, 0.063) are
  the same with and without provenance (`results-2.json`). It is not claimed
  fixed. Its matched-reset reference also differs in clip boxes and drop timing,
  so its definition needs root's numerical review.
- **C7 and C9 as written** stay recorded failures (C7 0.0918; C9 0.683 / 0.869
  against the converged state, 0.741 / 0.943 against frame 39). No threshold is
  reduced.

## 9. Mechanics changes to addendum 2 §7

| Item | Addendum 2 | Addendum 3 |
|---|---|---|
| Meta `x` | Front owner id | Bits 0–16: front owner id (≤ 100,000 < 2¹⁷). Bits 17–21: `n_min` (4–16). Bit 31: unbounded. Bits 22–30: 0 |
| Meta `y` | Second id, or `0xFFFFFFFF` | Bit 31 clear: second id (0 = none). Bit 31 set: class mask in bits 0–30 |
| Meta `z` | `bitcast(W)` | Unchanged |
| Meta `w` | f32 far depth | Support depth range as two f16 values: min rounded down, max rounded up. 0 means no geometry |
| R32Float history depth | Front depth | Unchanged: the near front depth, used for admission |
| `Portable` | 896 B | Adds `classes: vec4u` at offset 896 (x = changed-class mask; y–w = 0), so the span is 912 B |
| Host, per frame | Changed bitset | Also the changed-class mask, OR of `1 << (id mod 31)`, O(changed owners) |

**Directed f16 rounding of `w`:**
- pack each half separately and compare its unpacked value with the input;
- if the low half rounded up, step its bits down by 1;
- if the high half rounded down, step its bits up by 1;
- a max above 65504 is stored as `0x7C00` (+∞), and the history is then treated
  as unbounded.

**Canonical read, added to addendum 2 §7.4:**
- `n_min` in 4–16 whenever `W > 0`;
- low half ≤ high half;
- whenever the near front depth is > 0, it lies within the support range;
- the overflow mask is non-zero. Three owners can share fewer than three
  classes.

Anything else means no history.

**Cooker:** the `Portable` contract gains `classes` (offset 896, span 912). A
new negative control moves it.

**Accounting:** `Portable` becomes 2 × 912 B, so the total is
**973,121,760 B**: 32 B more than addendum 2. Every other term is unchanged.

**Per-pixel work added:**
- support min and max over 9 depths;
- two directed f16 roundings;
- one mask test.

The 9 owner loads were already counted.

## 10. What exact support costs (unmeasured)

These trade visual quality for exactness, as root's decision accepts:
- **Halo:** static pixels within the support radius of a moving or deforming
  owner drop or reset every frame. That radius is wider than `R` (section 4
  table).
- **Overflow:** more frequent, with drops on class collisions.
- **Rebase:** after camera motion, the first frame with any changed caster drops
  every history written along the motion. That is one restart, like a cut.

No GPU or device measurement exists for any of these.

## 11. Shared substrate with 0074 (no MetalFX redesign)

0074's R1a leaves two bounded residuals:
- **R-tol:** same-depth ghosts;
- **R-miss:** missed thin features.

0074 states that exact separation needs owner identity: a source-resolution
owner-ID channel, stored ids, and persistence only for owners unchanged this
frame.

**Shared:**
- stable per-placement ids, with the motion owner's lifetime (addendum 2 §7.1);
- the source-resolution owner word `id | changed << 31`. It must be R32Uint:
  100,000 ids need 17 bits, so 0074 §4's R16Uint cannot hold them;
- the accepted-state change witness, with the changed bitset and class mask;
- the provenance algebra from section 5:
  - record every positive-weight tap of whichever filter the history uses
    (0074's 2×2 and 3×3);
  - keep the union in every kept branch;
  - drop when a changed recorded owner leaves, or when overflow cannot prove
    authenticity.

**Not decided here:**
- MetalFX colour;
- 0074's α / owner-depth history format;
- the stored-id width per output pixel;
- whether the MetalFX path renders an owner target, which would be an extra
  target and is not authorized.

Root coordinates.

## 12. For the CPU admission tests

**Test-only cases:**
- S1, O1 and its cost case, U1, and E1;
- B1 as a two-surface case with a real pan;
- the f16 directed rounding, with values at 65504 and above;
- `n_min` through a 16 → 8 → 16 scale sequence.

**Also re-establish:** all addendum-2 cases under these rules.

## 13. Remaining limitations

- **Coverage of the witnesses:** a single pixel at 16/16, with B1 as algebra.
  There are no world points, light space or cascades, so A1–A5 and the slab
  containment are derivations, not simulations.
- **Not re-run:** section 5 changes rules that the retained and addendum-2
  cases exercised. Wider changed checks, union provenance, overflow persistence
  and the new drops apply to C14, C15, C19 and N4–N6. These suites were not
  re-run, as root asked, so their numbers describe addendum 2's rules, not
  these. The CPU admission tests must establish them again.
- **Exactness rests on:**
  - A1–A5 and deterministic shadow rasterisation;
  - the f32 extent margin;
  - correct directed f16 rounding;
  - unbounded histories handled by drop rather than proof.
- **Unmeasured:** the quality and performance costs in section 10, and every
  phone and GPU gate. No appearance claim is made.
- **Still open from addendum 2 (unchanged):**
  - exact-depth ties follow row-major order;
  - mobile uniform-array indexing is unmeasured.
