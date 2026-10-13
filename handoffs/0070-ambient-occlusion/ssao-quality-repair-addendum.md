# 0070 SSAO quality repair: addendum on N6, N7 and N8

## Status

**CPU design addendum only.** There was no GPU, screenshot, Cargo, production
edit or target use. Nothing under `tools/` and none of the earlier reports
changed:

- `result.md` remains the build002 report;
- `corrected-gpu-result.md`, `corrected-bloom-preparation.md` and
  `ssao-quality-repair.md` (eeeac1d4, "the design") are untouched.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**. ACP verification is root's.

**Inputs, all read only:**

- the final design;
- Astra's preliminary `ssao-quality-oracle-001`: `tests-001.log` (sha256
  `4553ca99…`), `source-001/freeze.json` (`39b4fb4c…`) and its five
  `quality_*.rs` files, which match the freeze hashes;
- for definitions and lifetimes: `tools/ssao_reference.py`, `tools/analyze.py`
  and `ambient_occlusion/{targets,mod,draw}.rs`.

**Evidence:** `ssao-repair/quality_report.py` writes
`ssao-repair/cpu-evidence-001.json`, using `ssao-repair/quality_model.py`. It
is standard library only, runs in the foreground in 278 s, and reproduces every
value in Astra's log; the driver exits 1 if it does not.

**Outcome:**

- **N6 stays failed** (7/255 against 1/64). Its bound sits at the
  continuous-sampling limit of the same test, so it cannot separate banding
  from the contact gradient. I propose a replacement, N6T, for root's decision.
- **N7 is now exact.** The baselines are 1382/1767/2027. They replace my
  1397/1782/2046, and the difference is attributed texel by texel. The design
  scores 0/0/0, but N7 only separates filter from no filter.
- **N8 is now exact and fails in perspective:** 0.376 of the shipped comb
  against a bound of 0.25, and 0.27–0.53 on the next six rows. Orthographic
  passes at 0.176. The bound is attainable, since the continuous limit reaches
  0.15. None of three bounded changes fixes it robustly, so I propose **no
  algorithm change**, and the failure stands.
- **Rotation and jitter:** the authored controls do not show either to be
  necessary. Under N6T both are necessary, and both are cheap.

## Shared conventions

- **Texels:** integer `(x, y)` with the origin at the top left of the 321×193
  target, not the viewport. A texel's centre is `(x + 0.5, y + 0.5)`. The
  viewport `(13.25, 17.5, 280.5, 160.5)` uses the same units. A texel whose
  centre lies outside the viewport has no surface.
- **View space:** right-handed, `−Z` forward, camera at the origin.
  - Perspective: vertical field of view 1 rad. Orthographic: half-height 4.
    Aspect 280.5/160.5.
  - Receiver: `z = −4`, `x ∈ [−8, 8)`, `y ∈ [−6, 6)`. Every surface normal is
    `+Z`.
- **Values:** accessibility `A`, written as `A × 255`.
  - The shipped algorithm's `A = 1 − k/16` stays unquantized, as in every
    recorded baseline.
  - Filtered outputs are R8-rounded after the input, the H pass and the V
    pass, as in the design.
- **`interior(x, y)`:** the texel and all 24 texels within Chebyshev distance 2
  hit the same surface. Background, off-target and off-viewport texels fail.
- **Converged family reference:** the expectation of a configuration's raw
  estimator, then the same H/V filter without R8 rounding.
  - Integration: `alpha` uniform in `[0, 2π/16)` and `t` uniform over the
    family's jitter range, by the midpoint rule with 32 × 16 strata.
  - Precision against 64 × 32 strata: at most 0.36/255 for the design and
    8-probe families, and 0.81/255 for full-stratum jitter.
- **Continuous limit:** the same single-layer test with 24 × 64 cosine-weighted
  midpoint directions and 32 midpoint steps, with no rotation, jitter or
  filter. Against 32 × 96 directions and 48 steps, filtered N6 steps move by at
  most 0.39/255.

## N6: banding along the `white_slab` perspective row

**Authored definition, now explicit:**

```
pairs = { x in [100, 170] : interior(x, 97) and interior(x + 1, 97) }    // persp: x 100..169, 70 pairs
N6    = max over pairs |A(x+1, 97) - A(x, 97)|                            // bound 1/64 = 3.98/255
```

**Retained failure:** the design scores 7.00/255 at x 161, which fails.

**Why N6 cannot discriminate:** the measured step is the contact gradient.

- The design's own converged expectation steps 7.09/255 at x 162.
- The continuous limit steps 4.98/255 raw and 3.82/255 after the filter
  (3.79 on the finer grid). That is within 0.2/255 of the bound, inside its
  0.39/255 convergence band.
- The bound therefore leaves no room for any sampling error or bias.
- Jitter-off also reads 7.00, but at x 146: a sampling artifact that N6 cannot
  tell apart from the gradient.

I authored this metric, and the defect is mine. In orthographic, which N6 does
not cover, the filtered continuous limit is 6.86/255, so no estimator could
pass there either.

**Proposed replacement, N6T, for root's decision.** It is not adopted by me,
and it does not turn N6 into a pass:

```
truth = continuous limit, filtered with the candidate's own H/V taps (no R8 rounding)
N6T   = max over pairs |(A(x+1) - A(x)) - (truth(x+1) - truth(x))|       // bound 1/64, unchanged
```

The scope stays `white_slab` perspective, as authored. Orthographic values
appear in brackets as diagnostics.

| Output (/255) | N6 | N6T | N6R diagnostic: against its own family reference |
|---|---|---|---|
| Shipped | 31.88 | 29.22 | — |
| Design | 7.00 | **3.37** [4.63] | 1.61 [2.20] |
| Rotation off | 8.00 | 4.83 [6.50] | 5.05 [8.80] |
| Jitter off (`t = 1`) | 7.00 | 4.51 [5.88] | 4.86 [5.60] |
| Filter only | 12.00 | 9.55 [10.69] | 9.79 [12.70] |
| Filter off | 31.88 | 28.22 [26.40] | 28.33 [24.90] |

Margins against the 0.39/255 reference uncertainty:

- the design passes N6T with 0.61/255 to spare;
- the closest mutation, jitter off, fails by 0.53/255.

The design's orthographic N6T of 4.63 would fail. It is recorded, but it lies
outside N6's scope.

## N7: hard steps, exact definition

```
count (x, y) over the whole target with
  interior(x, y) and interior(x + 1, y) and interior(x, y + 1)
  and max(|A(x,y) - A(x+1,y)|, |A(x,y) - A(x,y+1)|) >= 1/16
bound: <= 10% of the shipped count
```

This is Astra's definition, reproduced exactly. It is measured on
`white_slab` perspective, `white_slab` orthographic and `edge_slab`
perspective.

- **Shipped baselines:** 1382, 1767 and 2027.
- **Bounds:** at most 138, 176 and 202.

**Why the eeeac1d4 figures differ.** Those figures (1397/1782/2046) came from
a different rule, recorded here so they stay interpretable:

- they used the analyzer's interval oracle;
- they counted only texels with exact intervals;
- they applied the 2-px band (a depth change within Chebyshev 2, with
  neighbours outside the interior counted as unchanged) to the counted texel
  only.

The two texel sets were compared exactly:

| Fixture | Previous | Exact | Only previous: forward neighbour in the band (right, lower) | Only exact: an endpoint with an inexact interval |
|---|---|---|---|---|
| `white_slab` persp | 1397 | 1382 | 71 (40, 31) | 56 |
| `white_slab` ortho | 1782 | 1767 | 15 (8, 7) | 0 |
| `edge_slab` persp | 2046 | 2027 | 55 (35, 20) | 36 |

So the difference is entirely endpoint banding plus interval exactness. The
CPU model and the interval oracle agree on every exact texel.

**Results:** the design scores 0/0/0, matching Astra's run of the same
provisional repair.

| Mutation | Counts |
|---|---|
| Rotation off | 0/51/0 |
| Jitter off | 0/7/0 |
| Filter only | 0/89/15 |
| Filter off | 5462/5272/8106 |

Only removing the filter fails N7.

## N8: comb under thin occluders, exact definition

**Fixture `stripes`.** The receiver as above, plus four stripes, owners 1–4:

```
stripe i, i = 0..3:  x in [-0.5 + 0.3 i, -0.4 + 0.3 i),  y in [-0.5, 1.5),  z = -3,  normal +Z
```

The stripes are 0.1 wide with a pitch of 0.3, centred on `x = 0`. Their
bottom edge at `y = −0.5` leaves receiver rows below them in both projections,
clear of the viewport edge.

**ROI and metric:**

```
window = texels x whose centre ray meets z = -4 at view x in [-0.15, 0.15)   // one period, centred on the middle gap
row    = smallest y below every stripe texel such that, for every x in window,
         interior(x, y) and the texel is receiver
N8     = (max - min) of A over window on row                                 // bound <= 0.25 x shipped N8
```

The ROI is fixed by geometry before any measurement.

| Projection | Row | Window | Shipped N8 (/255) | Continuous limit |
|---|---|---|---|---|
| Perspective | 124 | x 148–158 (11 texels) | 31.88 | 0.151 (fine grid 0.146) |
| Orthographic | 110 | x 150–156 (7 texels) | 79.69 | 0.048 (fine grid 0.053) |

The bound is therefore attainable. Astra's rows 97, 130, 150 and 170 used
another placement (stripes from `x = 0.4`, `y ± 1.5`) and all crossed the
stripes. They were diagnostics, and this ROI supersedes them.

**Results, as a fraction of the shipped N8:**

| Output | Perspective | Orthographic |
|---|---|---|
| Design | **0.376 (fails)** | 0.176 |
| Rotation off | 0.690 | 0.125 |
| Jitter off | 0.345 | 0.050 |
| Filter only | 0.408 | 0.188 |
| Filter off | 1.500 | 0.600 |

**Retained failure:** the design fails N8 in perspective.

- **Persistence below the ROI:** a diagnostic only, not the ROI. On rows
  125–130 the design scores 0.439, 0.502, 0.533, 0.356, 0.335 and 0.272.
- **Cause, from the converged design expectation:** its peak-to-peak is
  15.80/255, against 4.81 for the continuous limit. The upper-half jitter
  probes only `[0.5, 1)` of each step stratum. The gaps it leaves (0–0.125,
  0.25–0.375, 0.5–0.625 and 0.75–0.875 of the radius) alias with the 0.1-wide
  stripes. The design therefore converges to a soft comb, not to the
  continuous answer.

**Bounded changes tested.** All keep the 16-direction kernel and the binary
test, and none is proposed:

| Change | Static extra cost | N8 persp at the ROI | Rows 125–130 | N6T persp | N7 |
|---|---|---|---|---|---|
| Full-stratum jitter, `t = (b_step + 0.5)/16` | none | 0.471 | — | 3.39 | 0/0/0 |
| 8×8 Bayer tile (64 strata), full-stratum jitter, 9-tap filter `[0.5, 1 × 7, 0.5]` | +40 B/px; filter ALU about ×1.8; 1 KiB table | 0.314 | — | 2.44 | 0/0/0 |
| 8 probes per ray, upper-half jitter | +256 B/px and about +4,500 ops: the AO pass roughly doubles | 0.220 | 0.282, 0.376, 0.408, 0.293, 0.230, 0.146 | 5.37 | 0/0/0 |

**The 8-probe variant is not robust.** It passes only at the defined row and
fails four of the next six. It also fails N6T, because its converged contact
gradient is steeper.

**Options for root:**

- **(a)** Accept the design with N8 perspective recorded as failed. This is a
  documented fixed-kernel limitation for thin occluders. On rows 124–130 the
  shipped comb is hard, two or three 16-code levels (32–48 codes). The design
  turns it into a soft 12–17-code modulation: 1.9 to 3.7 times smaller, short
  of the required 4.
- **(b)** Authorize a further bounded study of more independent samples per
  output texel. Its cost would be at least the 8-probe variant's.

A smaller art radius would shorten probe spacing proportionally. That stays
within the separate art evaluation and is not a claim here.

## Do rotation and jitter earn their cost?

**Cost in the design:**

- **Rotation:** one 16 B table load, the Bayer index and 12 operations per
  texel, plus 256 B of per-frame uniform.
- **Jitter:** 3 operations per texel and one add per probe.
- **Together:** about 90 operations, or 2% of the worst-case probe loop, and
  no bandwidth.

**Evidence** (perspective, as authored; orthographic and `edge_slab` are in
the evidence file):

| Control | Rotation | Jitter |
|---|---|---|
| N7 | Not needed: filter only scores 0 | Not needed |
| N6, authored | Cannot discriminate | Cannot discriminate |
| N6T, proposed | Needed: off gives 4.83 | Needed: off gives 4.51 |
| N8, authored | Helps but is not enough: 0.690 off against 0.376 on | Slightly hurts: 0.345 off against 0.376 on |

**Conclusion:**

- **Against the authored controls alone,** neither component is shown to be
  necessary. Rotation's only visible benefit is halving the N8 comb, which
  still fails.
- **Against N6T,** both are necessary and cheap.

**Recommendation:** keep both if root adopts N6T. Otherwise, the retained
controls do not justify the jitter.

## Retained lifetime cost

These are logical texel payloads under the root-approved cap of 14 B/px,
4,194,304 texels and 56 MiB. Physical allocation, padding, compression and
driver overhead are not measured.

| Item | Shipped | Candidate |
|---|---|---|
| Per texel | Depth32 4 + RGBA16F 8 + R8 `T0` 1 = 13 B | + R8 `T1` 1 = 14 B |
| One-size idle cache at 4,194,304 texels | 54,525,952 B (52 MiB) | 58,720,256 B (56 MiB) |
| Resize while one frame of another cap-sized extent is in flight | 104 MiB | 112 MiB |
| Per prepared frame: immutable uniform | 480 B | 736 B |
| Per prepared frame: bind groups | 2 | 4 |
| Pipelines | 17 | 19 |

**Lifetime rules, from the code:**

- `targets.rs` caches one `Arc<Targets>` for a single size, and a resize
  replaces it.
- Each `Prepared` frame keeps its own `Arc`, so a replaced set lives until
  every frame encoded with it completes. That depends on the caller's queue
  depth, which this module does not bound; `math.rs` already says so.
- `disable_ambient_occlusion` drops the system and its cache, while encoded
  frames keep their sets.
- The 8-probe variant would not change any attachment.

## Astra's preliminary sphere test

That test expected the old shading-normal input to darken the isolated 32×16
sphere cap. It measured `historical_shading_dark=0` and failed. The final
design does not require that mutation: its N5 expects identity for both
inputs, and the measurement agrees. Keep that result out of acceptance.

## If root adopts this addendum (Astra's oracle)

- **N7:** the exact definition, with baselines 1382/1767/2027.
- **N6:** keep the recorded failure, and add N6T with the stated continuous
  limit.
- **N8:** the `stripes` fixture and ROI above, with perspective recorded as
  failed.
- **Unchanged:** this addendum changes no rotation, jitter, filter, setting or
  memory figure.

## Return record

**Changed paths** (all new):

- `ssao-quality-repair-addendum.md`;
- `ssao-repair/quality_model.py` (sha256 `e7d083e5…`);
- `ssao-repair/quality_report.py` (`882953de…`);
- `ssao-repair/cpu-evidence-001.json`.

These files sit outside `tools/`, so the build004 helper digest is unaffected.

**Command,** run from the worktree root:

```
env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B \
  handoffs/0070-ambient-occlusion/ssao-repair/quality_report.py \
  > handoffs/0070-ambient-occlusion/ssao-repair/cpu-evidence-001.json
```

**Result:**

- Exit 0 in 278.1 s on Python 3.14.8, with `astra_log_reproduced: true`.
- Output sha256 `e2f427d7…`. Two consecutive runs matched in every field
  except `seconds`; the computation uses no random sampling.
- It imports `tools/analyze.py` read-only, with `-B`.
- Root's modified `brief.md` and the untracked `tools/effects/Cargo.lock` are
  untouched.

**Screenshots:** none. Nothing new was rendered or captured.

**Limitations:**

- f64 CPU model, planar fixtures, one stripe geometry;
- no GPU agreement (N4), device cost or appearance review;
- GPU f32 rounding can differ near decision thresholds, and the N8 R8 margins
  are integer codes.

**Open questions for root:**

1. Adopt N6T?
2. Accept the N8 perspective failure as option (a), or authorize study (b)?
3. Keep the jitter if N6T is not adopted?
