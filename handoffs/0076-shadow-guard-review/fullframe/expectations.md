# 0076 full-frame stationary coverage: pre-registered expectations

This was written and committed **before** either child ran. It states what the
source derivation predicts, which metrics will be reported and which observations
will trigger closer inspection. **There is no pass threshold.** These are
expectations, and a mismatch will be reported, not explained away.

## The run (root grant `228b2f20…`)

- **Children.** `baseline-stationary`, then `candidate-stationary`, using the frozen
  binaries `bd81f74e…` and `e693f872…`, read-only.
- **Counts.** The counts come from the fixture itself. `Report::open("stationary",
  17, 0)` declares one temporal-off frame plus 16 temporal-on frames. There are
  3 PNGs: `stationary-off.png`, `stationary-on-f16.png` and
  `stationary-activity.png`. Each child must pass 3 contract checks, which match
  0072's committed plan.
- **Limits.** 34 submissions, 6 PNGs at 960×540, 120 s per child and no retry.
- **Helper.** `run.py`, adapted narrowly from `determinism/run.py` (`93ab2d85`).
  Neither that wrapper nor its namespace is touched.

## What the source derivation predicts (`cascades.py` → `cascades.json`)

Temporal-on frames fit their cascades with `guard(rect)` in both sources. The
candidate also fits its temporal-off reference that way, while the baseline's does
not. At 960×540:

| Cascade | View depth | Texel | Snapped centre | Depth range | What can change in the temporal-off reference |
|---|---|---|---|---|---|
| 0 | ≤ 7.785 (blend from 7.017) | same | same | same (caster bounds) | **nothing** |
| 1 | 7.016–16.25 | same | same | far plane 14.940 → 14.959 | sparse depth-comparison flips at most |
| 2 | 15.403–28.574 | same | same | far plane 24.726 → 24.761 | sparse depth-comparison flips at most |
| 3 | 27.342–60 | 0.116422 → 0.116544 (+0.105%) | re-snapped | both planes move | far shadow edges re-quantized |

The depth ranges of cascades 1–3 extend past the caster bounds because the
frustum's lower corners do. The quantization margins are wide: the radius
fractions are 0.09–0.95 of a sixteenth and the centres sit 0.06–0.45 of a texel
from rounding. So the texel and centre predictions above are robust.

## Expectations

1. **Temporal-on frames and the activity map.** Expected byte-identical across
   binaries.
   - Any differing frame is reported individually, as observed.
   - With one run per binary it cannot be separated from same-binary variation:
     0076 demonstrated one varying frame in 144. No modal rule is applied.
2. **The temporal-off reference, by view-depth class** (`compare.py`):
   - **c0 (≤ 7.017):** byte-identical.
   - **c0c1, c1, c1c2, c2:** unchanged, or isolated pixels at shadow boundaries.
   - **c2c3, c3, c3fade:** differences at far shadow edges (fence-post shadows,
     far ground).
   - **beyond 60, and no surface:** byte-identical, since no shadow is sampled.
3. **The baseline's temporal-off PNG.** Expected byte-identical to 0072's retained
   `stationary-off.png`, because the non-temporal rendering is the same.

## Metrics to report (descriptive)

- Frame and PNG hash identity across binaries (`run.py summary`).
- Per depth class and per PNG: pixels, differing, over 2 steps, over 8 steps, and
  maximum step.
- Connected clusters of differing pixels (over 0 and over 8 steps), with their
  size, bounding box, maximum step and depth classes.

## Inspection triggers (declared now)

I will inspect any of these at full size and describe it:
- a difference in c0;
- a difference on a lit, unshadowed surface;
- a change to the grille card's own pixels;
- a change at the frame borders that is not a shadow edge;
- any newly visible artifact.

**Already open and not re-judged here:** thin MASK shadow fidelity, temporal
flicker, and the one-frame same-binary variation.
