# 0017 clustered lighting: pixel review result

## Status

Review complete. No visual defect found in the supplied fixtures. This is a
fixture-level pixel review only. It is not production lighting approval, not
native-capture approval, and not a phase gate approval.

- Model: Claude Opus 5.5 (`claude-opus-5-5`), Anthropic.
- Transport: Claude Code agent session launched by the handoff runner
  (bypassPermissions, director-authorized). Images were read directly from the
  worktree. No ACP-specific tool was exposed to this session beyond the runner
  itself; record that if the routing log expects a distinct ACP transport id.
- Director feedback: the current brief carries no priority revisions or new
  director feedback. No earlier result.md existed, so this is a fresh review.
- Native captures: none. The Mac is locked. No shell-native capture was used.
  No credentials were read.

## Method

1. Viewed every PNG in `artifacts/cluster-initial/` visually.
2. Decoded each PNG with a standalone pure-Python decoder (no third-party
   packages, kept outside the worktree in `/tmp/review0017/`). Ran:
   - SHA-1 identity check across all images.
   - Per-image max channel, clipped-pixel count (any channel = 255), unique
     colour count, background colour.
   - Seam detector: mean absolute second difference of luminance across every
     column and row, restricted to quad-interior pixels. Values at tile columns
     and rows (multiples of 64 px) compared against the image mean and the
     worst-scoring lines.
   - Scanline profiles through the spot cone, point gradient and directional
     quad.
   - Resolution consistency: 640x360 spatial-cluster image box-downsampled 2x
     and compared to the 320x180 capture over quad-interior pixels.
3. The brute-force GPU oracle images were not supplied to this review, so the
   "byte for byte" oracle match is taken from Astra's report and not
   re-verified here.

## Measured results

| Image | Size | Max RGB | Clipped px | Unique colours |
|---|---|---|---|---|
| point-near.png | 320x180 | 110,110,110 | 0 | 49 |
| point-far.png | 320x180 | 55,55,55 | 0 | 12 |
| point-outside-range.png | 320x180 | background only | 0 | 2 |
| spot-center.png | 320x180 | 110,110,110 | 0 | 112 |
| spot-away.png | 320x180 | background only | 0 | 2 |
| directional-red.png | 320x180 | 110,0,0 on quad | 0 | 3 |
| explicit-light-disabled.png | 320x180 | background only | 0 | 2 |
| cluster-64-lights.png | 320x180 | 110,110,110 | 0 | 49 |
| cluster-65-lights.png | 320x180 | 110,110,110 | 0 | 49 |
| cluster-128-lights.png | 320x180 | 110,110,110 | 0 | 49 |
| spatial-clusters-320x180.png | 320x180 | 99,123,100 | 0 | 3633 |
| spatial-clusters-640x360.png | 640x360 | 100,123,100 | 0 | 5707 |
| spatial-clusters-257x191.png | 257x191 | 100,123,100 | 0 | 3879 |

Identity groups (SHA-1):

- `point-near`, `cluster-64-lights`, `cluster-65-lights`, `cluster-128-lights`
  are byte-identical (`34eda551…`).
- `point-outside-range`, `spot-away`, `explicit-light-disabled` are
  byte-identical (`61e3cf52…`): background (20,21,25) plus a pure-black quad.

Seam detector, mean second difference (luminance code values):

| Image | Column mean | Tile columns | Row mean | Tile rows |
|---|---|---|---|---|
| spatial 320x180 | 1.14 | x128 1.34, x192 1.22 | 1.12 | y64 1.06, y128 1.06 |
| spatial 640x360 | 0.94 | x256 0.86, x320 0.88, x384 0.96 | 1.02 | y128 0.95, y192 1.01, y256 1.13 |
| spatial 257x191 | 1.09 | x128 1.06 | 1.08 | y64 0.97, y128 1.14 |

Tile boundaries score at the noise floor. The worst-scoring lines in every
image fall on the quad's own silhouette edges or on light peaks, not on
64-pixel boundaries.

Resolution consistency (640x360 downsampled 2x vs 320x180, 7439 interior px):

    median 0.5, p99 0.75, max 1.0 code values

## Verdicts

- **point-near.png: pass.** Smooth radial falloff from 110 to about 78 at the
  quad edge, one code value per pixel step, no banding, no clipping, neutral
  grey with no tint.
- **point-far.png: pass.** Peak 55 vs 110 in sRGB is about 0.038 vs 0.155
  linear, a ratio near 4, consistent with inverse square at double distance
  through the unchanged tone map. Same shape, no seams.
- **point-outside-range.png: pass, intentional black.** The quad is (0,0,0)
  and the background is unchanged. Note that this proves the cutoff but shows
  no visible range edge (see evidence request 3).
- **spot-center.png: pass.** Elliptical pool from quad perspective. Plateau of
  107 to 110 inside the inner cone, then a monotonic ramp to 0 over about 10
  px with no hard step or ring. There is a mild slope change ("knee") where
  the plateau meets the ramp (107 to 101 in one pixel, then about 8 to 10 per
  pixel). This is a look-development note, not a defect: a smoother angular
  falloff curve would soften the disc edge when art direction exists.
- **spot-away.png: pass, intentional black.** Away-facing spot contributes
  nothing.
- **directional-red.png: pass.** Quad is exactly (110,0,0) everywhere: pure
  red, flat as expected for a directional light on a planar diffuse quad, no
  green or blue leakage, no clipping.
- **explicit-light-disabled.png: pass, intentional black.** Byte-identical to
  the other zero-contribution fixtures.
- **cluster-64/65/128-lights.png: pass.** Byte-identical to each other and to
  point-near, which is the strongest possible "look the same" evidence across
  the 64-light bound and the 65/128 overflow path.
- **spatial-clusters-320x180 / 640x360 / 257x191: pass.** Regular 4x3 grid of
  soft coloured pools (green-cyan upper rows, lavender-neutral lower row).
  Vertical and horizontal profiles rise and fall smoothly with no plateaus or
  steps between pools. No seams at tile columns, tile rows or the odd-size
  257x191 partial tiles. Resolution-independent within 1 code value. Colour
  variation is explained by the authored per-light colours. No unexplained
  tint, no clipping.

Quad silhouettes are aliased because no antialiasing exists yet. That is the
test-quad boundary and is expected, not a lighting defect.

## Open defects

None found in the supplied images.

## Additional evidence needed for real visual coverage

These fixtures are dim (peak 110 of 255), planar and at nearly constant
depth, so several of the review targets in the brief are only weakly exercised.

1. **Spatially distinct overflow.** The 64/65/128 fixtures collapse to one
   point light visually. Capture a scene where one cluster holds more than the
   bound of lights at visibly different positions and colours, with an oracle
   diff.
2. **Depth-slice boundaries.** The quad spans few of the 24 logarithmic
   slices. Capture a receding floor or corridor from near to far plane with
   lights placed across slice boundaries, plus an oracle diff and a
   slice-index debug view.
3. **Visible range edge.** Capture a point light whose range boundary crosses
   a lit surface, so the windowing falloff at the cutoff can be judged for a
   ring or hard edge.
4. **Lights straddling tile boundaries** with small ranges, so a light's
   influence is split across several 64 px tiles at a known pixel location.
5. **High-intensity capture** that drives values near or past 1.0, to judge
   the tone map shoulder and highlight clipping. No image here exceeds 110.
6. **Specular and grazing angles.** All fixtures read as flat diffuse. A
   rough-to-smooth material sweep and a grazing-angle quad would expose
   highlight or terminator artifacts.
7. **Native captures** on the Mac once unlocked, at least one per fixture
   family, compared against these headless captures.

## Changed paths

- `handoffs/0017-clustered-lighting/result.md` (this file)

## Run commands

    shasum artifacts/cluster-initial/*.png
    sips -g pixelWidth -g pixelHeight <each png>
    python3 -I /tmp/review0017/stats.py <pngs>   # max, clipping, unique colours
    python3 -I /tmp/review0017/seams.py <pngs>   # tile seam detector
    python3 -I /tmp/review0017/rows.py <png> y=90 x=162   # scanline profiles
    python3 -I /tmp/review0017/res.py artifacts/cluster-initial   # resolution consistency

All ran successfully. The review scripts are throwaway tools in `/tmp` and are
intentionally not committed.

## Screenshot paths

Reviewed, not produced: all 13 PNGs in `artifacts/cluster-initial/`. No new
screenshots or visual-regression images were created.

## Unavailable

- Native macOS captures: unavailable, Mac locked.
- Brute-force oracle images: not supplied, oracle equality not re-verified.
- Visual regression baseline comparison: no baseline exists for this increment.

## Rationale and limits

Shader math, tests, authored lights, project content and UI were not touched,
per the brief. Passing these fixtures does not approve production lighting:
shadows, antialiasing, exposure controls, mobile tiers and a complete render
graph do not exist yet. No phase gate is approved by this review.

## Open questions

- Should the ACP transport be recorded with a distinct identifier in
  result.md, given this session ran as a Claude Code agent under the runner?
- Can the oracle captures be copied next to the fixtures in future packets so
  the byte-for-byte claim can be checked during pixel review?
