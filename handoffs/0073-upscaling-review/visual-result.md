# 0073 visual execution — corrected portable output-upscaling review

**Status: executed under root's grant, and the GPU is released.**
- **Hard results: PASS.** All 20 contract checks pass, and every integrity check
  in the fail-closed analysis passes.
- **My quality verdict: FAIL.** At 8/16 the upscaler converges to a stable but
  soft image. Accumulation removes texture detail that the current internal frame
  still has, instead of adding output-resolution detail. Edges and sub-pixel
  geometry stay phase-unstable.
- **What works:**
  - current-only overlays and physical UI are stable;
  - history survives internal-only scale changes, and the cut resets exactly;
  - the fractional viewport and its chrome are clean;
  - there is no ghosting in motion.
- Production was not changed; the findings go to coordinated repair. This is not
  a phase completion, performance result or phone result.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-visual-execute-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`. No subagents, memory writes, browser or
system screenshots, pushes or PRs.

## Exact source, binary and bound evidence

| Item | Value |
|---|---|
| Brief | `brief.md` `1fc2219d…`, root's priority revision committed in `32d6d6bb` |
| Engine source | `535003d766bcc1386fd2d2f454c83808a0fcc4c8` (tree `e98b2484…`), root's corrected union. Default portable WGSL path; not cooked, not native MetalFX |
| Source freeze | `temporal-metalfx-union-001/committed-build-source.json` `bd8295b0…`. Exact coverage: 2,448 build inputs, 0 mismatched |
| Visual tree | `7e6d4b08d6da34bbb3c660fe448aa6cf318c81ae`, built at root's `ce20026a`. That commit holds the lint-only fixture edits, the build.py lint latch and the `-002` evidence path. Fixture `c0169e34…` |
| Build record / frozen helper | `build/review.json` `b52c6918…`. `bin/upscaling_review` `786adfbb…` (mode 0555, 18,120,064 bytes, features `default` and `physics-deterministic`, no cooked or MetalFX feature) |
| Grant / root build receipt / prior release | `review-ready.json` `ebf34321…`; `upscaling0073-build-final-20261013.json` `6109ca8b…`; `0070-effects-build-002/released.json` `aa8fd844…` |
| Ledger / debits / GPU release | `88b269e8…` / `f0b9aceb…` / `d0998c95…` |
| Analysis / published manifest | `summary-2026-10-13T082346…` `2fba8a75…` / `evidence/visual/manifest.json` `c0552e9a…` |
| Host | `Mac17,9`, Apple M5 Pro, Metal (IntegratedGpu), macOS 27.0.1, rustc 1.99.0 |

## Run

All steps ran serially in the foreground under
`~/.codex/incant-evidence/0073-upscaling-visual-002`. Each step's console output
is kept create-only in `console/01-list.txt` through `console/08-publish.txt`.

| Step | Result |
|---|---|
| `gpu.py list` | Exit 0, pid 98558. Listed exactly `motion`, `probe`, `stationary`, `transitions` |
| `gpu.py review probe` | Exit 0, pid 98590, 0.90 s, 0 frames. Adapter Apple M5 Pro on Metal; 8 of 8 checks pass |
| `gpu.py review stationary` | Exit 0, pid 98614, 0.90 s, 36 frames, 6 PNGs, 5 of 5 checks pass |
| `gpu.py review motion` | Exit 0, pid 98635, 0.81 s, 24 frames, 3 PNGs, 3 of 3 checks pass |
| `gpu.py review transitions` | Exit 0, pid 98656, 0.94 s, 29 frames, 4 PNGs, 4 of 4 checks pass |
| `gpu.py release` | At 08:23:36Z. 5 GPU children; **89 of 96** frames debited and charged; **13 of 16** PNGs. No owned or live process, no orphan debit, no stop, no refusal |
| `analyze.py check` | **Exit 0, verdict pass, complete, no problems** (details below) |
| `analyze.py publish` | 37 receipt files to `evidence/visual/` and 13 PNGs to `screenshots/`, bound to that analysis |

The analysis check confirmed:
- 19 of 19 figure tiles re-cut from the raw frames;
- the stationary UI interior is identical in FULL, DRS and UP frame 24;
- both statistics reproduce from the raw frames;
- the transitions chrome is identical;
- every PNG is 960×540, and every file, RGBA and hash binding holds.

## Quality verdict and findings (mine; pixels inspected)

I inspected all 13 PNGs, plus scratch 2–4× crops cut from the retained frames.
The crops were not retained and have been deleted.

Two kinds of numbers appear below. Neither is a gate; both are supporting
measurements.

| Kind | What it is |
|---|---|
| Contrast | Luminance standard deviation in regions I placed by hand on the retained PNGs |
| Error | The helper's report observations: mean largest-channel difference from FULL, in 8-bit sRGB codes |

1. **Textures lose detail that the internal frame itself resolves.** Contrast in
   the stationary chart:

   | Chart quadrant | FULL | DRS 8/16 | UP 8/16 (frame 24) |
   |---|---|---|---|
   | 1-texel lines | 43.9 | 2.5 | 1.4 |
   | 2-texel checker | 53.8 | 25.2 | **3.3** |
   | Rings | 52.8 | 28.3 | 19.8 |
   | 4-texel checker | 70.5 | 60.8 | **30.6** |

   - UP halves the 4-texel checker, which the 480×270 internal frame resolves
     almost fully.
   - It also nearly erases the 2-texel checker.
   - The nearest-sampled sprites are softer, too (see 5).

   Error against FULL:

   | Region | DRS 8/16 | UP 8/16 |
   |---|---|---|
   | Stationary frame, interior / edges | 0.11 / 31.7 | 0.25 / 35.9 |
   | Chart crop (`sign`) | 37.3 | 38.8 |
   | Sprite crop | 6.0 | 8.4 |
   | Wire crop | 17.6 | 17.0 |
   | Card-edge crop | 1.17 | 1.07 |
   | Text crop | 2.75 | 2.63 |
   | Motion cube crop, frames 14 / 20 | 15.5 / 21.5 | 19.3 / 24.4 |
   | Motion pan crop, frames 14 / 20 | 1.36 / 1.25 | 1.39 / 1.28 |
   | Fractional viewport, interior / edges | 0.44 / 32.3 | 0.25 / 33.6 (frame 10) |

   - In the stationary frame and in six of the nine crop measurements, UP is
     further from FULL than DRS is.
   - UP is marginally closer in the wire, card-edge and text crops.
   - Its interior is clearly closer only inside the fractional viewport. There,
     DRS's nearest expansion is non-integer (885/480).
   - **Likely causes, read in `upscaling/resolve.wgsl` and
     `model_material.wgsl` at `535003d7`:**
     - The current frame is reconstructed with a 2×2 bilinear tap at internal
       resolution.
     - The output keeps a 0.9-weight running mean of those reconstructions,
       clamped to the current 3×3 neighbourhood. That averages bilinear
       reconstructions instead of placing jittered samples at output resolution.
     - Material textures use `textureSample` with implicit internal-resolution
       derivatives, and `incant_render` has no texture LOD bias.
2. **Interiors converge, but accumulation removes detail instead of adding it.**
   - Over the last 8-phase cycle, interior activity averages 0.38 codes, and only
     1.4% of interior pixels move by more than 2.
   - Error against FULL does not fall from the current-only frame 1 to frame 24:
     interior 0.250 vs 0.251, edges 35.5 vs 35.9.
   - In the transitions strip crop, the current-only frame right after the cut
     shows the 2-texel checker at 8.3. That falls to 3.5, 3.1 and 2.6 at frames
     24, 25 and 27. The same crop measures 26.6 in DRS and 51.0 in FULL.
3. **Edges never converge.** Taps with mixed coverage or incompatible depth
   return before history, so silhouettes and depth edges are current-only. They
   change with every jitter phase.
   - 43% of edge pixels change by more than 2 codes across the cycle, and 17% by
     more than 32.
   - Mean consecutive-frame change on edges is 3.4–13.0 codes.
   - The activity map lights up:
     - the wires, the mast and their shadows on the ground;
     - the full-width horizon;
     - the sphere rim and its shadow edge;
     - the fence posts;
     - the box behind the card;
     - the chart border.
   - The chart texture itself shimmers mildly: chart-crop activity is 3.1.
   - The horizon is as coarsely stepped as DRS, with a soft 1–2 px band that
     changes by phase.
4. **Sub-pixel geometry flickers or vanishes.** FULL draws all three wires solid
   (presence 169), and DRS drops all three. UP presence by phase:

   | Wire | Presence, phases 0–7 |
   |---|---|
   | 1.6 px | 158, 158, 158, 62, 62, 158, 158, 158 (dims in 2 of 8) |
   | 0.8 px | 0, 62, 158, 0, 0, 158, 62, 0 (blinks) |
   | 0.4 px | never present |

   The 0.56 px diagonal renders as dots whose positions shift with phase.
5. **Current-only overlays are stable.** This verifies the corrected
   unjittered-overlay contract.
   - **Activity over 8 phases:**
     - Sprite crop 0.0.
     - World-text crop 0.16. The "Ag" glyphs look identical in all phase tiles;
       the activity is a thin fence post that appears only in some phases.
     - Card-edge crop 0.45. The activity map shows it on the opaque box edge;
       the BLEND card's own edge shows none.
   - **They are mildly softened by the bilinear composite:**
     - Sprite contrast is 39.4, vs FULL 44.6 and DRS 45.7, and the
       nearest-sampled texel edges are visibly blurred.
     - Text contrast is 25.2 vs 27.1.
6. **Physical UI is stable.** The opaque button interior is byte-identical on
   all ten UI frames (contract). UI glyphs stay crisp.
7. **Scale changes and the cut keep the contract, with visible steps.** One owner
   and history survive 8→12→4→16→8 sixteenths. Edge change at each boundary,
   against the change in the frame before it:

   | Change | At the boundary | Frame before |
   |---|---|---|
   | 8→12 | 9.9 | 9.9 (no pop) |
   | 12→4 | 16.9 | 3.2 |
   | 4→16 | 17.4 | 13.5 |
   | 16→8 | 13.4 | 4.6 |

   - Current-only edges follow each new internal size at once.
   - **Detail is lost at once going down, but regained slowly going up.** The
     3×3 clamp drops it immediately: in the strip, the 2-texel checker goes from
     6.2 at frame 13 to 0.1 at frame 14. It returns only at the accumulation
     rate: 0.1, 3.2 and 8.0 at frames 16, 17 and 19.
   - **16/16 does not reach FULL.** After three 16/16 frames, where internal
     equals output, the 1-texel lines measure 3.3 against FULL's 40.9, and the
     2-texel checker 8.0 against 51.0. The segment is too short to show where
     16/16 settles.
   - **The cut resets exactly.** Frames 23–27 are byte-identical to frames 1–5,
     with the same RGBA hashes. The cut's frame change (8.2 edge codes) is
     smaller than the frame before it (11.9).
   - **Fractional viewport:** the border band matches FULL (mean 0.002 codes
     interior, 0.12 at edges). The chrome is identical over 74,900 pixels.
8. **Motion: no ghosting, but texture flattens inside the moving shadow.**
   - The cube emerging from behind the sphere, its moving shadow and the camera
     pan leave no trail or doubled edges.
   - Global error against FULL stays at stationary levels: interior 0.26 and
     0.22, edges 35.7 and 35.3, at frames 14 and 20.
   - **The shadow on the chart:** at frame 20, where the cube's shadow crosses
     the chart, FULL still resolves the shadowed rings (contrast 27.8). UP shows
     a flat patch (7.2) whose extent matches FULL's shadow within a few pixels.
   - **That is a larger relative loss than in the lit rings just above it**
     (15.5 vs 45.5).
9. **Fixture flaw (mine, not the renderer):** the physical UI label's own clip
   rectangle cuts off the bottom of its 18 px text in every variant. The UI
   contract uses only the button.

**What this means for repair (not acted on here).** The corrected portable path
is functionally sound: the contract holds, overlays and UI are stable, and
history behaves. But it is not yet a quality upscaler.
- Converged texture detail is lower than nearest-neighbour DRS's.
- Even 16/16 trails FULL.
- Edges and thin geometry shimmer with phase.
- Texture LOD follows internal derivatives.

Likely needs:
- reconstruction that weights jittered samples at output resolution, instead of
  averaging bilinear reconstructions;
- an edge history policy;
- a texture LOD bias.

These are design decisions for coordinated renderer work, not tuning of this
review's fixed policy.

## Screenshots

`handoffs/0073-upscaling-review/screenshots/`, 13 PNGs at 960×540, each copied
from its capture and verified against the ledger hash:

| Case | Files |
|---|---|
| Stationary | `stationary-full`, `-drs50`, `-up50`, `-phases`, `-detail`, `-activity` |
| Motion | `motion-full`, `-up50`, `-detail` |
| Transitions | `transitions-full`, `-up50`, `-strip`, `-cut` |

Receipts, logs, reports, events and the bound analysis are in
`handoffs/0073-upscaling-review/evidence/visual/` (manifest `c0552e9a…`).

## Limits

- **Scope:** one local Metal adapter, and the fixed review policy (history 0.9,
  depth 0.05 relative and 0.02 absolute, reactive 4.0 at 0.5).
- **Tiers:** the main tier is 8/16. 12/16, 4/16 and 16/16 appear only as 3-frame
  segments in transitions.
- **Flicker method:** judged from phase strips, activity maps and
  consecutive-frame statistics in still frames, not by watching motion.
- **Region placement:** my contrast regions are placed by hand from the chart
  layout.
- **Not covered:** cooked-temporal, native MetalFX, other backends, phones and
  performance.
- **StageA:** its numeric proof remains historical evidence for the `0452c5f5`
  binary only.
- No phase gate is claimed or approved.
