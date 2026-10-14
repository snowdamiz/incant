# R4 pixel review checklist (for the later granted run only)

This checklist is for Claude Opus 5.5 with thinking explicitly Max, verified through ACP. Use it only after root grants and the bounded runner has run.

The numbers in `report.json` never decide quality. No threshold below is new; each is quoted unchanged from `owner_quality/plan.json`. Candid failure is an allowed outcome.

## Before looking

1. Check the run's contract outcome before inspecting any image:
   - every group finished primary-first;
   - no process failed;
   - every image is a PNG of at most 960×540;
   - the image set is exactly the 15 planned PNGs.

   Then read all 15 current PNGs in full: 6 stationary, 4 motion, 2 transitions, 2 c7_disocclusion and 1 p1_crossing.
2. **Do not use older images:**
   - Retained images from earlier runs (`a5ae3179`, `owner_quality`, `owner_quality_repair`, `owner_output_quality`) are context only.
   - Never label them R4 results.
   - Never use them as R4 references.

## What to judge, together, per image

3. **Compare all four modes in each comparison:** FULL, DRS (fixed 8/16), ordinary UP and OWN.
   - Judge whether the reduced-resolution UP and OWN are a usable reconstruction relative to DRS. Do not require identity with FULL.
   - State plainly where FULL is still visibly better.
4. **Texture and fine detail:**
   - Is checker and text texture detail sharper than nearest-neighbour DRS without moiré?
   - Does the negative internal LOD bias add shimmer or aliasing in moving or current-only regions?
5. **Thin geometry:**
   - Check the wires and thin features for phase-dependent gaps, breaks or flicker.
   - The eight jitter offsets stratify each axis only, so a gap at some angles is an expected, reportable limitation, not a pass.
6. **Silhouettes and edges:**
   - Look for stair-stepping, halos and ghost trails behind moving edges.
   - Look for over-blur from strict motion coherence falling back to current-only.
7. **Overlays:**
   - Sprites, mixed-script world text, the BLEND card and physical UI must stay crisp at output resolution.
   - They must show no sanitation artefacts. The previous accepted state is the baseline, so any regression is a failure.
8. **Stability and transitions:**
   - Check for a DRS wipe or transition pop across scale changes.
   - Shadow-cascade stability under DRS is newly targeted by the DRS-stable guard. Shadow comparison is still limited to the fixture's views, so say what can and cannot be seen.

## Historical gates: quoted, never closed by these observations

9. **C7 (unchanged):** "a square moving 3 px/frame over a static textured background; beyond 2 frames of motion behind the trailing edge, the mean absolute difference per column from a never-moved converged reference is at most 0.06".
   - The rendered `report.json` value compares 0.06 with mean 8-bit sRGB code differences / 255 against a never-occluded, same-chronology reference.
   - That value is a rendered-output observation, not a pass or fail.
   - The linear CPU-model failure (0.0918) stays recorded.
10. **C9 (unchanged):** third 16 ≥ 0.75, fourth 16 ≥ 0.9, first 4 vs last 12 ≥ 0.9.
    - The rendered values use the luminance standard deviation of sRGB codes per chart region against a finite matched-cycle reference (frames 17–24). That is an observation only.
    - Convergence is not demonstrated.
    - The model failures (0.683/0.869 and 0.741/0.943) stay recorded.
11. **P1:** unresolved.
    - The fixture covers grey 8/16 only, at one stop.
    - The CPU model's worst residuals stay recorded: clear 8/16 0.349, clear 16/16 0.221, grey 8/16 0.296, grey 16/16 0.063.
    - Do not claim P1 closure.

## Verdict

12. Write separate verdicts:
    - stationary;
    - motion and disocclusion;
    - transitions and DRS;
    - overlays;
    - overall.

    Cite image file names and the exact regions. Do not average a failure away.
13. All historical failures remain open unless the evidence defined for that gate exists:
    - `visual_a5ae3179`;
    - GPU001 case 3;
    - C7, C9 and P1;
    - texture detail and thin geometry;
    - silhouettes;
    - the DRS wipe.

    Pixel inspection can show improvement; it cannot pass a gate whose metric is defined differently.
