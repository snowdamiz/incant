# Native coverage repair: design proposal (0074, producer 077f7c39)

**Status: design only.** No code, shader or test was changed. Root owns the Rust and
WGSL implementation and the math.

- **Source read:** only the four scoped files, `metalfx/{prepare.wgsl,
  composite.wgsl,numerics.rs,targets.rs}`, read in root's worktree. Each is
  byte-identical to producer `077f7c39`.
- **Prelude helpers:** `admitted`, `clean`, `finite4`, `inside`, `source_position`,
  `tap_weight`, `lo`/`hi`, `linear_depth`, `compatible` and the `frame` uniform live
  outside those files. Their meaning below is inferred from `numerics.rs`, which
  mirrors them, and from their names. Root should confirm it.
- **Memory figures:** the two constants come from `plan.rs` at 077f7c39, read in the
  scale-correction preparation: `Plan::bytes() = 25·pixels + 2048` with at most
  4,194,304 pixels, and `MAX_METALFX_TRANSITION_BYTES = 200 MiB + 4096`.
- **Other lanes:** the portable redesign (0073) and the fixed-TAA edge repair
  (e237) are referenced by concept only. Nothing here depends on them or edits
  them.

## 1. Current behaviour, precisely

**`pack`** runs at source resolution, for every pixel inside `frame.source.zw`:

| Source pixel | colour | depth | motion | reactive |
|---|---|---|---|---|
| admitted, `a == 1`, depth z in [0, 1) | `clean(c)` | z | `(m.xy + jitter.xy − jitter.zw)·source.zw` if finite, `m.w > 0.5` and within ±65504 | 0 if the motion is valid and `frame.depth.w > 0.5`, else 1 |
| anything else (clear `a = 0`, fractional `0 < a < 1`, not admitted, depth outside [0, 1)) | 0 | 1 (far) | 0 | 1 |

**`normalize`** runs at output resolution, for every pixel p inside `frame.output`:
- **Footprint F(p):** the 2×2 bilinear taps around `q = source_position(p) − 0.5`,
  with weights w.
- **`current`:** `clean(Σ w·clean(c_t))`, the jittered current frame resampled with
  premultiplied coverage.
- **`opaque`:** every tap of F is admitted with `a == 1` and `d > 0`, and every tap
  depth is `compatible` with `z = min d`, that is `|d − z| ≤ max(0.001, 0.01·z)`
  per the oracle.
- **Output:**
  - if `opaque` and the native RGB is finite and non-negative: `(min(native.rgb,
    65504), 1)` and depth z;
  - otherwise: `current` and depth 0, so later overlays are not occluded there.
- **Native alpha:** ignored.

**The defect.** Any output pixel whose 2×2 footprint touches clear or fractional
coverage, or spans more than about 1% of linear depth, shows the single jittered
current sample. MetalFX never reconstructs it. That covers every silhouette against
the sky, every object outline over the floor, the MASK grille and the posts.
- **At 16/16 (002 stationary):** in all 8 compared frames, 13,131 of the 34,327
  reference-edge pixels (38%) were identical to the same-phase current-only control.
  Those edges alias and shimmer with the jitter phase.
- **At 12/16 and 8/16:** the same pixels step at the input pitch (1.33 and 2 px),
  nearly identical to the portable path. MetalFX upscaling therefore cannot improve
  any silhouette.

## 2. Goals

- **G1:** real antialiased coverage at clear and depth-discontinuity boundaries.
- **G2:** no stale opacity or stale RGB.
- **G3:** no leakage from an invalid or disoccluded owner.
- **G4:** no double filtering.
- **G5:** no contamination from excluded BLEND, sprites, text or UI.
- **G6:** native alpha stays untrusted.
- **G7:** each removed safeguard is replaced by an explicit, testable one, and the
  existing oracle rows stay valid.

"Owner" here means the surface that covers a pixel. The engine has no object IDs,
so an owner change is detected only through depth, coverage and motion validity.

## 3. Proposed contract

### 3.1 Pack (source resolution)

Opaque admitted pixels are unchanged. Non-opaque pixels change only in colour.
MetalFX must never see a black fill at an edge, because black would mix into the
edge colour.
- **Fractional (`a ≥ a_min`, for example 1/255):** colour = unpremultiplied
  `c.rgb / a`. Depth is its own when it lies in [0, 1), else 1. Motion follows
  today's rule. Reactive is 1.
- **Clear (`a < a_min`):** colour = the unpremultiplied colour of the nearest-depth
  covered pixel in its 3×3 neighbourhood (fixed tie order), else 0. Depth 1, motion
  0, reactive 1, all as today.

Overlays are composited after `normalize`, so they never enter `pack` (G5).

### 3.2 Composite (output resolution)

**Neighbourhood.** N(p) is the 3×3 source pixels centred on the tap of F nearest
`q`, clamped to the source rectangle. From N:
- `lo`, `hi`: the minimum and maximum coverage `a` over N;
- `L`, `U`: the per-channel minimum and maximum of unpremultiplied colours over N's
  taps with `a > 0`;
- `Lo`, `Uo`: the same over N's opaque taps only.

The jitter is at most ±0.375 source px, so a pixel within about half a source pixel
of an edge sees both covered and uncovered taps in N in every phase. **Coverage
class therefore comes from N, not F.** If it came from the 2×2 F, silhouette pixels
would flip between "opaque" and "boundary" from phase to phase, which recreates the
shimmer.

| Coverage class (by N) | Depth class (by F) | Output colour (premultiplied), α | Coverage history H_next |
|---|---|---|---|
| interior: `lo = 1`, all admitted | **A**: all F taps compatible | native RGB (as today), α 1 | 1 |
| interior | **B**: some F taps incompatible (opaque depth discontinuity) | `clamp(native.rgb, Lo − ε, Uo + ε)`, α 1 | 1 |
| **C** boundary: `lo < hi` | (either) | `α·clamp(native.rgb, L, U)`, α = H_next | accumulated (below) |
| clear: `hi = 0` | (either) | 0, α 0 | 0 |
| native RGB not finite or negative (any class) | | today's `current` (fallback) | α_cur |

**Depth output is unchanged:** z only when every F tap is opaque and compatible,
otherwise 0. Overlay occlusion at silhouettes therefore behaves exactly as today
(decision D3).

**Coverage history H** is engine-owned, at output resolution, R16Float, ping-pong.
For a boundary pixel:
- `α_cur = Σ_F w·a_t`, today's alpha.
- The motion v is the pack displacement at the nearest-depth tap of N (depth-dilated,
  current→previous, in source px).
- The previous position is `p_prev = p + v·(output/source)`.
- `α_prev` is a bilinear sample of H_prev at p_prev. It is invalid if:
  - that tap's motion is invalid;
  - p_prev leaves the output rectangle;
  - the native reset flag is set (first native frame, cut or epoch change,
    content-size change, previous frame portable).
- `H_next = valid ? clamp(lerp(α_cur, clamp(α_prev, lo, hi), h), lo, hi) : α_cur`,
  with h = `TemporalConfig.history_weight` (0.9 in `REVIEW`; decision D4).

**Why this meets the goals:**
- **G1:** α converges to the coverage supersampled over the 8 jitter phases, while
  the colour comes from MetalFX's reconstruction instead of the aliased current
  sample. Opaque depth discontinuities (class B) go to MetalFX, which has colour,
  depth and motion on both sides.
- **G2:**
  - Stale opacity: where the current neighbourhood has no coverage, `lo = hi = 0`
    forces α = 0 in the same frame.
  - Stale RGB: colour is confined to the current neighbourhood's colours (L…U for
    C, Lo…Uo for B). A colour from a vanished owner cannot survive.
- **G3:** invalid motion or a reset uses α_cur. Non-opaque pixels stay reactive = 1,
  so MetalFX leans on current input there. Class B relies on MetalFX's own depth
  disocclusion, bounded by the rectification clamp. That assumption must be
  validated (A2).
- **G4:** each signal is filtered once.
  - Colour is filtered by MetalFX and coverage by H.
  - Native output is never resampled or blended with `current`, except in the
    invalid-native fallback.
  - α_cur is H's single current sample.
- **G5:** `pack`, H and the classification read only the pre-overlay `current` and
  scene depth. BLEND, sprites, text and UI are composited afterwards, with unchanged
  depth semantics.
- **G6:** native alpha is still ignored.
- **G7:** safeguards are replaced, not just removed.
  - The same-depth test is replaced only for all-opaque footprints, by colour
    rectification.
  - The opacity test is replaced by an engine-owned coverage that is clamped to the
    current neighbourhood.

## 4. Replacement oracle contract (`numerics.rs`)

None of the current rows becomes obsolete. Each one is a special case of the new
contract (uniform neighbourhood, no history):

| Current row | Today | New contract | Same result? |
|---|---|---|---|
| solid opaque, same depth, native alpha in {0, 0.5, 1, −100} | native RGB, α 1 | interior, class A | yes |
| uniform fractional a ∈ {0, .25, .5, .75}, native 100 | `clean(c)` | a = 0 → clear (0). Otherwise `lo = hi = a` gives α = a, and `a·clamp(100, c.rgb/a, c.rgb/a) = c.rgb` | yes |
| one clear tap + three solid, native 100 | [0.1875, 0.375, 1.5, 0.75] | class C with no history: α = α_cur = 0.75, `0.75·clamp(100, solid, solid)` | yes |
| one tap at depth 1 vs 2, solid colour | `solid` | interior, class B: `clamp(100, solid − ε, solid + ε)` | exact for ε = 0, within ε otherwise |
| native NaN or negative | `solid` (current) | fallback | yes |

**New rows** (double-precision, written by root):
- **R1:** class B with a non-uniform neighbourhood. Native inside [Lo, Uo] passes
  unchanged; outside, it is clamped per channel.
- **R2:** static edge accumulation. Jittered binary coverage over the 8 `SAMPLES`
  converges to the area coverage within a stated tolerance after 16 frames at
  h = 0.9.
- **R3:** stale opacity. With `H_prev = 1` and a fully clear neighbourhood, the
  output is exactly 0 with α 0.
- **R4:** premultiplied validity. `rgb_c ≤ α·U_c` for every output.
- **R4 identity:** compositing over two canvases A and B gives results that differ by
  exactly `(1 − α)(A − B)`.
- **R5:** invalid motion, p_prev outside the rectangle, or a reset gives α = α_cur.
- **R6:** a black native output at class C is lifted to `α·L`. Vendor black never
  becomes a halo darker than the darkest covered neighbour.
- **R7:** changing only overlay inputs leaves pack outputs and H unchanged.

## 5. Native API assumptions that need real GPU validation on this device

| # | Assumption | How to validate |
|---|---|---|
| A1 | reactive = 1 makes MetalFX favour current input at filled edge pixels | static scene: native RGB at filled pixels stays within ε of the filled current colour over 16 frames |
| A2 | MetalFX handles opaque depth discontinuities under modest motion without trailing, given depth and motion | `late_motion`, class B on vs off: interior "rest" error against same-step off, and trail energy |
| A3 | far-depth (1.0, not reversed) filled clear pixels cause no disocclusion artifacts along sky silhouettes while the camera pans | `late_motion` camera pan over the horizon and post tops |
| A4 | native RGB at edges stays a convex combination of neighbourhood colours (no sharpening overshoot) | rectification activation rate in a static, converged scene, expected about 0; a high rate means the clamp changes vendor output and ε needs review |
| A5 | the reset flag fully clears vendor history | two resets on identical input in one run give identical output |
| A6 | identical inputs give identical output | needed for byte-level controls. The 0.03-code difference between runs in the history-free `cut-f5` frame (descriptor range [1, 1] vs [1, 3]) is unattributed |
| A7 | output alpha | ignored; no assumption |
| A8 | content-size changes within one scaler behave as in 002 | already observed: selection, and the inferred resets at d12-7 and d8-10 |

## 6. Extra cost and memory (estimates; device measurement required, no performance claim)

**Memory.** H as two R16Float textures adds 4 B per output pixel:
- 960×540: 2.07 MB;
- 1920×1080: 8.29 MB;
- the 2048×2048 admitted maximum: 16.8 MB.

Per allocation, `Plan::bytes()` becomes `29·pixels + 2048`. Today's
`MAX_METALFX_TRANSITION_BYTES` is exactly two allocations at the maximum extent:
2·(25·4,194,304 + 2048) = 209,719,296 = 200 MiB + 4096. With H, that maximum needs
243,273,728 B (+32 MiB); with R8Unorm history, 226,496,512 B (+16 MiB). Root must
either raise the constant or lower the admitted extent (decision D2). At 29 B per
pixel, the current budget fits at most 3,615,779 pixels.

**Per-pixel work:**

| Pass | Today | Added |
|---|---|---|
| `pack` (source resolution) | 3 loads, 4 writes | non-opaque pixels only: up to 8 loads for the 3×3 fill, plus one division |
| `normalize` (output resolution) | up to 12 loads (two tap loops) + 1 native load, 2 writes | every pixel: 5 ring loads for N's coverage and colour range, plus 1 H write (2 B). Class C pixels: 5 more depth loads for depth dilation, 1 motion load, 1 bilinear H sample |

An optional optimisation: `pack` writes each source pixel's 3×3 coverage min/max to
a source-resolution RG8 texture (+2 B per source pixel). `normalize` then reads 1–4
taps instead of the 5-load ring. Reads beyond the ring concentrate on boundary
pixels, a few percent of the frame in the 002 scenes.

## 7. Negative controls

Each must fail on a naive change and pass under the contract:

| Control | Naive change it catches | Measure |
|---|---|---|
| N1 stale opacity | coverage history without the neighbourhood clamp | a post or the mover crossing clear sky: the vacated pixels are byte-identical to off in the next frame |
| N2 owner leakage | class B without rectification | an occluder moving off a contrasting opaque background: the disoccluded output stays inside the current 3×3 colour range, and trail energy against off is no higher than current-only's |
| N3 detector check | simply removing the same-depth test | N2 must flag measurable ghosting on such a build, proving the control can see leakage |
| N4 black contamination | native at clear edges with today's black packing | two-canvas identity (R4) on sky silhouettes fails on a dark fringe |
| N5 double filtering | blending native with `current` at edges | each class matches exactly one formula (oracle). The 10–90% edge rise at class C is no wider than at native interiors |
| N6 excluded overlays | overlays reaching `pack` or H | the 002 excluded case still passes (UI byte-identical, sprites and text unchanged), and a BLEND card over sky leaves H at 0 |
| N7 resets | H carried across a cut or content change | the first frame after a reset has α = α_cur |
| N8 fallbacks | collateral changes | FractionalViewport and Aspect stay portable, and the canvas outside the viewport stays byte-identical to off |
| N9 positive measure | — | in the late review at 16/16 and 8/16, the current-only band (`current_band`) shrinks from 38% of edge pixels toward flat interiors only |

**Proposed acceptance contract** to replace the failing silhouette judgement. These
are starting points for root to calibrate, not results:
- (a) N9 band edge pixels at most 1% of reference-edge pixels;
- (b) converged consecutive edge change in a static scene at most 3 codes mean,
  against about 10–15 today in both 0074 runs;
- (c) the two-canvas identity within 1 code;
- (d) N1 exact;
- (e) N2 interior error no higher than today's native.

## 8. Decisions for root

- **D1:** class B (depth discontinuities) is independent of class C and cheap: no
  history, one clamp. It can land and be validated first.
- **D2:** the coverage-history format (R16Float or R8Unorm) and the budget change
  (§6).
- **D3:** overlay depth at silhouettes stays today's F-based rule. A nearest-depth
  policy would be a separate change, to coordinate with the unjittered overlay-depth
  work.
- **D4:** H's weight. Reusing `TemporalConfig.history_weight` keeps native and
  portable coverage consistent.
- **D5:** the neighbourhood clamp, depth-dilated reprojection and premultiplied
  bounds are the same concepts the 0073 portable redesign and the e237 edge repair
  need. Sharing oracle rows where the math is identical avoids a visible pop when an
  owner switches between native and portable.

**Not proposed:**
- removing the opacity check;
- trusting native alpha;
- feeding overlays into `pack` or H;
- blending native output with `current` at edges;
- retiring any current oracle row.
