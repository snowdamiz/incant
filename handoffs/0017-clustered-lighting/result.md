# 0017 clustered lighting: pixel review result

## Status

**Final round (integrated review): complete. No remaining material defect
within this handoff's scope. No new code change was needed.** See "Round 4"
at the end.

- Integrated runtime is 2ee02d0 / 00dfbd1. The final GPU images match my
  independent local run, and every final oracle pair and CLI Undo/Redo hash
  identity was verified directly.
- Every defect raised in earlier rounds is closed: the range rim, X/Y/Z
  colour labels, invisible light units and the uninspectable spot aim.
- Inspector scrolling at minimum size: verdict is no correction required.
  The reasons are in Round 4.
- Not approved here: production lighting, any phase gate, or publication of
  account-bearing native images.

Round 3 summary, kept as history:

- Range rim: fixed in `crates/incant_render/src/lighting/shade.wgsl` with a
  squared quartic window, as the brief directed. All 17 `model_gpu` GPU tests
  and all 14 `incant_render` library tests pass locally, and oracle identity
  still holds.
- Near-patch plateau: confirmed as intended by Astra (1 cm distance floor).
  Not a cluster omission. Question closed.
- Dielectric limitation: closed by the new `punctual-low-dielectric-*`
  captures.
- Native captures: reviewed. Color showed X/Y/Z labels, now fixed in generic
  FieldView presentation. Three further issues are reported, not fixed.
- This round supersedes the "Mac is locked" statements below for native
  review only. Those statements stay as history. Native final captures with
  both fixes are still pending.

Earlier rounds, kept as history:

- Initial review: no visual defect. Verdicts are preserved unchanged below.
- Follow-up review: all supplied oracle pairs verified pixel-identical. No
  tile seam, slice seam or hue-shifted clipping was found. There is one open
  look-development defect: the visible range cutoff has a slope crease. There
  is one open question about the two nearest depth patches. One evidence
  limitation affects the dielectric sweep. See "Follow-up review" below.

This is a fixture-level pixel review only. It is not production lighting
approval, not native-capture approval, and not a phase gate approval. Native
work remains explicitly pending because the Mac is locked.

- Model: Claude Opus 5.5 (`claude-opus-5-5`), Anthropic.
- Transport: Claude Code through ACP protocol 1. Per the follow-up brief,
  Astra's runner (`tools/handoff/main.py`) starts `claude-agent-acp`,
  exchanges JSON-RPC initialize with ACP protocol 1, creates or resumes the
  worktree session, selects Opus 5.5 and sends `session/prompt`. The runner's
  bypassPermissions mode is director-authorized.
- Director feedback acknowledged (follow-up brief): record the transport as
  Claude Code through ACP 1, with no separate identifier needed. Keep
  throwaway review scripts in the ignored `artifacts/` directory of this
  worktree. Preserve the initial verdicts and clearly separate the added
  evidence. All three points are applied here. The initial open question
  about transport is closed.
- Native captures: none. The Mac is locked. No shell-native capture was used.
  No credentials were read. No file outside this worktree was edited. The
  initial review's temporary `/tmp/review0017/` scripts were moved into
  `artifacts/review0017/`, which git ignores, and the `/tmp` copy was deleted.

# Initial review (preserved)

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

# Follow-up review: expanded real GPU coverage (added evidence)

Everything in this section is new evidence from the priority follow-up. It
does not change any initial verdict above.

## Method (follow-up)

1. Viewed every added PNG visually.
2. Ran the same decoder and checks from `artifacts/review0017/` on the added
   images: SHA-1, decoded pixel identity against each supplied `-oracle`
   image, clipping and hue-at-clip checks, the 64 px tile seam detector,
   scanline profiles, sRGB-to-linear edge slopes and per-patch statistics.
3. The oracle images were supplied this time, so the oracle identity claim
   was checked directly.

## Oracle and state identity (verified directly)

| Capture | Compared with | Result |
|---|---|---|
| spatial-overflow-96-640x480 | its -oracle | pixel-identical, same SHA-1 |
| spatial-overflow-96-513x385 | its -oracle | pixel-identical, same SHA-1 |
| depth-boundary-patches-640x480 | its -oracle | pixel-identical, same SHA-1 |
| depth-boundary-patches-513x385 | its -oracle | pixel-identical, same SHA-1 |
| cli-undo-point | cli-authored-point | pixel-identical |
| cli-restored-point | cli-authored-point | pixel-identical |
| cli-source-free-point | cli-authored-point | pixel-identical |
| cli-redo-point | cli-dimmed-point | pixel-identical |
| cli-dimmed-point | cli-authored-point | differs in 37391 px, as an intensity edit should |

The identity proves that clustered assignment equals brute-force evaluation
of all lights. It cannot catch an error in shading code that both paths
share, such as BRDF, attenuation or tone mapping. Those were judged visually
below.

## Measured results (follow-up)

| Image | Size | Max RGB | Clipped px |
|---|---|---|---|
| spatial-overflow-96 | 640x480, 513x385 | 229,219,208 | 0 |
| depth-boundary-patches | 640x480, 513x385 | 143,147,129 | 0 |
| visible-range-edge | 640x480, 513x385 | 252,222,171 | 0 |
| spot-centered-reference | 321x181 | 110,110,110 | 0 |
| spot-penumbra | 321x181 | 109,109,109 | 0 |
| punctual-hdr-metal-0.045 | 640x480 | 255,255,255 | 22 |
| punctual-hdr-metal-0.3 | 640x480 | 255,254,252 | 708 |
| punctual-hdr-metal-0.7 | 640x480 | 254,238,212 | 0 |
| punctual-hdr-metal-1 | 640x480 | 247,225,185 | 0 |
| punctual-hdr-dielectric-0.045 | 640x480 | 255,255,254 | 8 |
| punctual-hdr-dielectric-0.3 | 640x480 | 255,247,234 | 88 |
| punctual-hdr-dielectric-0.7 | 640x480 | 254,238,212 | 0 |
| punctual-hdr-dielectric-1 | 640x480 | 254,238,210 | 0 |
| cli-authored-point | 640x480 | 255,253,249 | 406 |
| cli-dimmed-point | 640x480 | 255,246,232 | 34 |

Every clipped pixel is near-white. No image contains a pixel with red at 255
and green below 230, so the highlights show no orange or yellow hue ring at
the clip.

Seam detector at 64 px tile lines vs the image mean (column / row):

| Image | Column mean | Tile columns | Row mean | Tile rows |
|---|---|---|---|---|
| overflow 640x480 | 0.82 | 0.69 to 0.92 | 0.85 | 0.61 to 1.00 |
| overflow 513x385 | 0.91 | 0.78 to 1.07 | 0.89 | 0.65 to 0.96 |
| depth patches 640x480 | 1.03 | 0.88 to 1.28 | 1.01 | 0.89 to 1.43 |
| range edge 640x480 | 0.41 | 0.00 to 0.51 | 0.32 | 0.00 to 0.61 |
| metal-0.3 sphere | 1.56 | 0.03 to 1.04 | 1.66 | 0.87 to 2.13 |

The worst-scoring lines in every image are sphere or quad silhouettes or
light peaks. None falls on a tile line.

## Verdicts (follow-up)

- **spatial-overflow-96 (both sizes): pass.** The 96 overlapping coloured
  lights form a smooth green-to-rose blend with no tile seam, no block
  pattern and no clipping. The oracle match is verified directly, so the
  overflow path is exact. Odd and even sizes both pass.
- **depth-boundary-patches (both sizes): pass with one open question.** The
  24 patches form a clean numerical grid. Peak red follows the column (93,
  113, 129, 143) and peak green follows the row (107, 119, 129, 139, 147).
  Blue stays at 129. Patches on opposite sides of each logarithmic boundary
  match each other's pattern exactly, and the odd 513x385 size gives the same
  per-patch values within 1 code value. That is direct visual evidence of no
  slice-boundary discontinuity from about 0.1 to 681 units. The exception is
  patches 00 and 01, see the open question below.
- **visible-range-edge (both sizes): open look-dev defect.** The cutoff is
  continuous, with no hard step and no tile seam. In linear light, though,
  the falloff reaches zero with a nearly constant slope instead of easing
  out (see the defect below). On screen the pool reads as a disc with a
  defined rim rather than a fade.
- **spot-centered-reference: pass.** Odd dimensions put the plateau centre at
  x=160, the column of the world origin. The plateau holds 107 to 110 with a
  monotonic ramp to 0 on both sides. The ramps differ in length (8 px vs
  11 px) only because the quad is tilted in perspective. No ring and no step.
- **spot-penumbra: pass.** The penumbra is wider and monotonic, rising from 0
  to 97 over 11 px and then easing into the plateau. The inner-cone slope
  "knee" noted initially is still visible (109 to 103) but stays a look-dev
  note, not a defect.
- **punctual-hdr metal sweep: pass.** Roughness reads correctly. At 0.045 the
  highlight is a tight pinpoint on a black ball. It widens through 0.3 and
  0.7, and at 1 it becomes a broad, nearly uniform lobe from 244 at the
  centre to 172 near the lower rim. The highlight rolls through the tone-map
  shoulder to near-white without hue rings. Black low-roughness metal is
  expected because no environment lighting exists. That is consistent with
  the black quads of the initial fixtures.
- **punctual-hdr dielectric sweep: pass, with an evidence limitation.** The
  specular pinpoint at 0.045 and the soft highlight at 0.3 are correct and
  clip only to near-white. The grazing lower rim darkens smoothly from 241 to
  101 over the last few pixels without a band. However, the diffuse body sits
  at 248 to 254 sRGB across almost the whole disc, deep in the tone-map
  shoulder. That flattens the shading, so the 0.7 and 1 dielectrics are
  nearly indistinguishable and Lambert falloff is barely visible.
- **cli-*-point: pass.** The authored, Undo, restored and source-free renders
  are pixel-identical. Redo is pixel-identical to the dimmed render. The
  dimmed sphere shows a smaller, still near-white highlight and a darker
  body, which is a plausible intensity reduction. The highlight shows no hue
  shift at the clip. This is visual confirmation that Undo and Redo restore
  exactly and that rendering does not depend on source files. Journal reopen
  and the atomic rejection of an invalid range are not pixel-observable
  beyond these identities, so I took them from Astra's tests.

## Open defects (follow-up)

1. **Range cutoff slope crease (look-development, non-blocking for
   clustering correctness).** At the left edge of the range pool, linear red
   per pixel inward from the cutoff measures:

       640x480: 0.0006, 0.0152, 0.0307, 0.0467, 0.0630, 0.0802
       513x385: 0.0027, 0.0212, 0.0409, 0.0612, 0.0823, 0.1046

   Each step adds about 0.015 to 0.020, so the attenuation reaches zero with a
   nonzero slope. A smooth window, for example `(1 - (d/r)^4)^2`, would give
   roughly quadratic values near the edge, about 1:4:9, and they are not.
   Value continuity is fine, but the slope discontinuity reads as a defined
   ring, which is what the brief asked me to judge for. Request: Astra
   confirms the intended range-window function. If the spec calls for a
   zero-derivative window, this is a mismatch. If the linear end is intended,
   document it as a look-dev choice. I did not change any shader.

## Open questions (follow-up)

1. **Depth patches 00 and 01 have flattened peaks.** Both keep their hue and
   match the row below at their edges, with a linear ratio of 1.0. Their
   centres are dimmer, though. Patch 00 drops to about 0.25 to 0.55 of the
   patch below it, and patch 01's peak plateaus at 84 instead of 113. The
   shape fits a minimum-distance or radius clamp on inverse-square
   attenuation for the two nearest patches near 0.1 units. Because the oracle
   is identical, clustering is not the cause. Please confirm that this is
   intended clamp behaviour and document it. If not, it is a shared shading
   issue near the near plane.

## Evidence limitations (follow-up)

- **Dielectric sweep saturates in the tone-map shoulder.** Roughness and
  Lambert falloff on dielectrics cannot be judged well at this intensity. A
  lower-intensity companion sweep would give useful look-dev evidence, either
  bracketed or with a darker albedo. Exposure controls do not exist yet, and
  I did not tune the authored lights.
- **The oracle shares shading code.** Byte identity verifies assignment only.
  An independent analytic reference for a few pixels, such as the patch peaks
  or the spot centre, would cover shared BRDF and attenuation code.
- **Native captures remain pending** for every family. The Mac is locked.

## Status of initial evidence requests

| Initial request | Status |
|---|---|
| 1. Spatially distinct overflow | Supplied and verified: spatial-overflow-96 with oracle |
| 2. Depth-slice boundaries | Supplied: depth patches with oracle, plus Astra's readback test; no slice-debug view, accepted as a diagnostic fixture |
| 3. Visible range edge | Supplied: reveals the slope crease above |
| 4. Lights straddling tile boundaries | Covered by the overflow and range-edge captures crossing tile lines with no seam |
| 5. High-intensity capture | Supplied: HDR sweep and CLI fixtures; highlights roll off cleanly |
| 6. Specular and grazing angles | Supplied: metal and dielectric sweep; dielectric limited by saturation |
| 7. Native captures | Still pending: Mac locked |

## Changed paths

- `handoffs/0017-clustered-lighting/result.md` (this file)

Throwaway review scripts are in `artifacts/review0017/`, which git ignores,
and are not committed.

## Run commands

Initial review (scripts since moved from `/tmp/review0017/`):

    shasum artifacts/cluster-initial/*.png
    sips -g pixelWidth -g pixelHeight <each png>
    python3 -I artifacts/review0017/stats.py <pngs>   # max, clipping, unique colours
    python3 -I artifacts/review0017/seams.py <pngs>   # tile seam detector
    python3 -I artifacts/review0017/rows.py <png> y=90 x=162   # scanline profiles
    python3 -I artifacts/review0017/res.py artifacts/cluster-initial   # resolution consistency

Follow-up review:

    python3 -I artifacts/review0017/ident.py artifacts/cluster-initial   # oracle and CLI identity
    python3 -I artifacts/review0017/prof.py artifacts/cluster-initial    # HDR, spot, range profiles
    python3 -I artifacts/review0017/patches.py <depth patch pngs>        # per-patch stats
    python3 -I artifacts/review0017/edge.py <range png> <y>              # range edge pixels
    python3 -I artifacts/review0017/lin.py artifacts/cluster-initial     # linear slopes, patch ratios
    python3 -I artifacts/review0017/pmap.py <depth patch 640x480 png>    # patch 00/01 ratio map

All ran successfully.

## Screenshot paths

Reviewed, not produced. These are all in `artifacts/cluster-initial/`:

- Initial: 13 PNGs (point, spot, directional, disabled, cluster-64/65/128,
  spatial-clusters at three sizes).
- Follow-up: spatial-overflow-96 and depth-boundary-patches at 640x480 and
  513x385, each with -oracle; visible-range-edge at both sizes;
  spot-centered-reference; spot-penumbra; punctual-hdr metal and dielectric
  at 0.045, 0.3, 0.7 and 1; cli authored, dimmed, undo, redo, restored and
  source-free point.

No new screenshots or visual-regression images were created.

## Unavailable

- Native macOS captures: unavailable, Mac locked. Native work remains pending.
- Initial spatial-clusters oracle images: not supplied, so not re-verified.
  The follow-up oracles were verified.
- Visual regression baseline comparison: no baseline exists for this increment.

## Rationale and limits

Shader math, tests, authored lights, project content and UI were not touched,
per the brief. Passing these fixtures does not approve production lighting:
shadows, antialiasing, exposure controls, mobile tiers and a complete render
graph do not exist yet. No phase gate is approved by this review.

## Open questions

- Range window function: is a nonzero slope at the cutoff intended? See the
  follow-up defect 1.
- Depth patches 00 and 01: is the flattened near-plane peak an intended
  distance clamp? See follow-up question 1.
- Resolved: transport is recorded as Claude Code through ACP 1, per the
  follow-up brief.
- Resolved: oracle captures were supplied with the follow-up.

# Round 3: range-window correction and native capture review

Everything above this heading is preserved history. This round is new.

## Director feedback acknowledged (round 3)

- I own the focused look-dev correction to the range window. This narrow
  shader edit overrides the earlier no-shader-edit rule. I did not change
  cluster assignment, light units, tone mapping or any other BRDF math.
- Astra confirmed the near-patch plateau is intended. The attenuation
  denominator uses a 1 cm minimum distance (`max(d², 1e-4)` in m²) to keep
  coincident sources finite. The first two depth patches put their sources
  5 to 7 mm from the patch. **Confirmed and documented: not a cluster
  omission.** Follow-up open question 1 is closed.
- `punctual-low-dielectric-*` at intensity 50 is the lower-intensity
  companion I requested.
- The Mac is unlocked, and five real CUA captures were supplied for viewport
  and field review. Saved account restored without interaction. I did not
  transcribe the account identity shown in the title bar.
- Handoff 0016 owns generic Inspector label-wrapping CSS. I made no CSS edits.
- Rendering used this worktree's own target directory through `./tools/cargo`.
  `CARGO_TARGET_DIR` was unset, so nothing was shared.

## Change 1: C1 range window (shader)

`crates/incant_render/src/lighting/shade.wgsl`, in `shade_light`:

- Before: `window = max(1 - (d²/r²)², 0)` and `attenuation = window / max(d², 1e-4)`.
  The window has slope -4 at d = r, which is the measured crease.
- After: the same window is squared, giving
  `attenuation = window² / max(d², 1e-4)`. Value and first derivative are both
  zero at d = r, so the cutoff is C1-continuous.
- Reference: Filament, "Physically Based Rendering in Filament", punctual
  light attenuation, equation 65 (as cited in the brief). The arithmetic is
  expressed independently. No shader listing was copied.
- Preserved: authored units and radiance scaling, inverse square away from
  the cutoff, the early return giving strict zero for d ≥ r, the 1 cm
  denominator floor, and the spot cone factor.

Note on window shape: squaring also lowers the window inside the range. At
half range the window is 0.88, against 0.94 before. At a quarter range it is
0.992, and at a tenth of the range it is 0.9998.
Lights authored with a range close to the lit distance will look slightly
dimmer than before. That is visible in the small-range fixtures below and is
the expected cost of a C1 window.

## Evidence for change 1 (real GPU, regenerated locally)

Command (worktree-local target, output git-ignored):

    INCANT_LIGHT_EVIDENCE=<worktree>/artifacts/cluster-final \
      ./tools/cargo test -p incant_render --release --locked --test model_gpu lights::coverage -- --ignored
    # 3 passed

Range edge, linear red per pixel inward from the left cutoff:

| Capture | First five pixels inward | Step at cutoff |
|---|---|---|
| 640x480 before | 0.0006, 0.0152, 0.0307, 0.0467, 0.0630 | about 0.015, constant |
| 640x480 after | 0.0009, 0.0040, 0.0086, 0.0152, 0.0232 | 0.003, growing inward |
| 513x385 before | 0.0027, 0.0212, 0.0409, 0.0612, 0.0823 | about 0.019, constant |
| 513x385 after | 0.0018, 0.0065, 0.0144, 0.0242, 0.0382 | 0.005, growing inward |

The steps now grow with distance from the cutoff instead of staying
constant, so the falloff eases into zero. The interior peak is unchanged at
252, 222, 170. No clipping.

Seam detector, range edge (worst-scoring line, which was the cutoff):

| Capture | Worst column before | Worst column after | Column mean before | Column mean after |
|---|---|---|---|---|
| 640x480 | 3.40 | 0.56 | 0.41 | 0.21 |
| 513x385 | 4.59 | 0.92 | 0.52 | 0.23 |

Tile lines remain at the noise floor.

Before vs after over all regenerated images (largest per-pixel channel
change):

| Images | Max change | Reason |
|---|---|---|
| visible-range-edge, both sizes | 39 | intended rim removal (range 1.8) |
| spatial-overflow-96, both sizes | 16 | small ranges (4): slightly dimmer, smoother pools |
| depth-boundary-patches, both sizes | 10 | small ranges (0.16 × depth): peak 143 → 142 |
| punctual-hdr metal and dielectric, all 8 | 1 | range 100 at about 5 units: inverse square preserved |
| punctual-low-dielectric, all 4 | 1 | same |

Oracle identity after the change was verified directly by decoding. Both
spatial-overflow-96 sizes and both depth-boundary-patches sizes are
pixel-identical to their `-oracle` images.

Verdicts after change 1:

- **visible-range-edge (both sizes): pass.** The pool now fades out with no
  defined rim. Follow-up defect 1 is resolved.
- **spatial-overflow-96 (both sizes): pass.** Same green-to-rose blend,
  slightly softer, no seams, oracle-identical.
- **depth-boundary-patches (both sizes): pass.** Same grid pattern, peaks
  within 1 code value of before, oracle-identical. Patches 00 and 01 keep the
  intended 1 cm plateau.
- **punctual-hdr (8 images): pass, unchanged** within 1 code value.

## Evidence: low-intensity dielectric companion (new captures)

| Roughness | Peak sRGB | Vertical profile at x=320 (R, top to bottom) |
|---|---|---|
| 0.045 | 255,249,239 (2 px pinpoint) | 71 … 122 … 36 |
| 0.3 | 216,196,159 | 71 … 186 highlight … 36 |
| 0.7 | 126,114,91 | 72 … 126 … 37 |
| 1 | 123,111,89 | 72 … 123 … 38 |

- **punctual-low-dielectric (4 images): pass.** The diffuse body sits well
  below the shoulder (about 70 to 125), so Lambert falloff toward the rim is
  visible. The highlight goes from pinpoint to soft lobe to broad and nearly
  absent as roughness rises. No hue ring at the clip. The follow-up
  dielectric limitation is closed.

## Other verification run locally

    ./tools/cargo test -p incant_render --release --locked --test model_gpu -- --ignored
    # 17 passed: includes inverse square, outside-range, spot falloff,
    # 64/65/128 overflow and brute-force spatial cluster checks
    ./tools/cargo test -p incant_render --release --locked --lib -- --include-ignored
    # 14 passed: includes per-slice GPU readback and HDR output
    ./tools/cargo fmt --all -- --check          # ok
    ./tools/cargo clippy -p incant_render --locked --all-targets -- -D warnings   # ok

Evidence from the full run was written to `artifacts/cluster-final-all/`,
also git-ignored. Astra still owns the authoritative numerical run and the
final evidence regeneration.

## Native capture review (artifacts/lighting-native/, real CUA, JPEG)

| Capture | Size | Verdict |
|---|---|---|
| 01-point-dim-wide | 1440x900 | Pass for viewport. Inspector Color shows X/Y/Z (fixed, see change 2). |
| 02-point-undo-bright | 1440x900 | Pass. Intensity 300, larger near-white highlight, Redo enabled. |
| 03-point-redo-dim | 1440x900 | Pass. Intensity 60, viewport matches 01, Redo disabled again. |
| 04-spot-wide | 1440x900 | Pass for fields. Zero-intensity probe leaves the viewport unchanged, as intended. |
| 05-spot-minimum | 1000x650 | Issue: the Outer degrees row is cut off behind the Agent panel. |
| resize-check (not listed in brief) | 1000x900 | Pass. All spot fields visible, sphere centred. |

Viewport observations, all captures: the authored point light gives a warm
near-white highlight with a smooth falloff on a dark metallic-looking sphere.
There is no hue ring at the clip and no seam. The sphere edge is aliased
because no antialiasing exists yet. The lower rim drops below the background
because no ambient term exists. Both match the headless fixtures. The cursor
hovers over hierarchy rows in 01 and 04, which is a capture artifact. Undo and
Redo button states are correct in each capture. These captures predate change
1, but the point light's range of 100 does not reach the sphere's distance
limit, so they are unaffected.

## Change 2: Color channels labelled R/G/B (generic FieldView presentation)

The issue was material. The native PointLight and SpotLight Color showed
"X 1 / Y 0.8 / Z 0.5", which reads as a position or direction next to the
Transform section's identical X/Y/Z rows.

- `editor/ui/src/components/inspector/FieldView.tsx`: new exported
  `arrayChannels` helper. A 3- or 4-number array is shown as R/G/B(/A) when
  the schema widget is `color`, `rgb` or `rgba`, or when there is no widget
  hint and the field key ends in "color" or "colour" (`color`, `baseColor`,
  `emissive_color`). Each input gets an accessible name (Red, Green, Blue,
  Alpha), and the visible letter is hidden from assistive tech to avoid a
  double reading. All other arrays keep X/Y/Z/W, and `quat` is unchanged.
- The tint classes stay positional (`axis--x/y/z/w`), and X/Y/Z already use
  red, green and blue tints. No CSS changed, so there is no conflict with
  handoff 0016.
- Presentation only: values, read-only state, diagnostics and commands are
  unchanged.
- `editor/ui/src/App.test.tsx`: one new test. It uses the observed native
  PointLight schema shape (an untitled 3-number `color`, no widget). It checks
  that Color reads R/G/B, that the Green input has the accessible name
  "Green" and value 0.8, that a spatial array in the same component stays
  X/Y/Z, and that axe reports no violations.

Commands, from `editor/ui`, after `npm ci --offline` at the repo root
(node_modules is git-ignored):

    npx tsc -b --noEmit     # ok
    npx vitest run          # 11 files, 283 tests passed (1 new)

Not yet verified visually. No native or browser screenshot of the R/G/B
labels exists. They need the refreshed native captures.

Data-binding request for Astra: the native light schemas should emit an
explicit `x-incant-widget: "color"` on `color`. That would make the key-name
rule only a fallback. I did not edit the engine schemas.

## Other native issues (reported, not fixed)

1. **Light units are invisible.** The schemas describe Intensity in candela,
   or lux for DirectionalLight, and Range in metres. Those descriptions only
   appear as a hover tooltip, so the Inspector shows a bare "60" or "100".
   FieldView already renders `x-incant-unit`, but the native schemas do not
   emit it. Data-binding request: emit `x-incant-unit` with `cd` or `lx` for
   intensity, `m` for range and `°` for the cone angles. The renderer needs
   no change.
2. **Minimum-size Inspector clips its last field.** At 1000x650 the Outer
   degrees row is cut off at the Agent panel boundary. The Inspector body
   does scroll (`overflow: auto`), but macOS overlay scrollbars give no
   visible cue that more fields exist. Recommended for a layout handoff:
   either a scroll-edge shadow on the Inspector, or let the Agent panel
   collapse or shrink first at small heights. This touches Inspector CSS, so
   I left it out of this handoff because of 0016.
3. **Spot aim is not inspectable on the probe.** The SpotLight probe shows
   no Transform section, and spot direction is transformed local -Z. If the
   probe has no Transform, its default orientation is implicit. Question for
   Astra: is that intended for a probe? A real spot should expose its aim.
4. **Account identity appears in captures.** The title-bar account chip shows
   the signed-in identity in every native capture. These files are
   git-ignored here. If native captures are ever committed, attached to a PR
   or published as evidence, crop or redact that chip first.

## Remaining verification (round 3)

- Astra: the authoritative numerical test run and final evidence
  regeneration with the new window.
- Refreshed native captures with both changes: the range edge in the native
  viewport, Color showing R/G/B, and 1000x650 Inspector behaviour.
- Visual confirmation of the R/G/B labels in any real renderer, which no
  screenshot shows yet.
- Data bindings requested above: `x-incant-widget: "color"` and
  `x-incant-unit` on light schemas.
- Not approved by this review: production lighting, with no shadows,
  antialiasing, exposure controls, mobile tiers or complete render graph. No
  native approval and no phase gate approval either.

## Changed paths (round 3)

- `crates/incant_render/src/lighting/shade.wgsl`: C1 range window
- `editor/ui/src/components/inspector/FieldView.tsx`: colour channel labels
- `editor/ui/src/App.test.tsx`: one new presentation test
- `handoffs/0017-clustered-lighting/result.md`: this file

These are git-ignored and not committed: review scripts in
`artifacts/review0017/` (including `edge2.py` and `before_after.py`),
regenerated images in `artifacts/cluster-final/` and
`artifacts/cluster-final-all/`, and run logs.

## Screenshot paths (round 3)

Reviewed, not produced:

- `artifacts/lighting-native/01-point-dim-wide.jpg`,
  `02-point-undo-bright.jpg`, `03-point-redo-dim.jpg`, `04-spot-wide.jpg`,
  `05-spot-minimum.jpg` and `resize-check.jpg`.
- `artifacts/cluster-initial/punctual-low-dielectric-*.png`.

Regenerated locally by the real GPU test, after change 1:

- `artifacts/cluster-final/*.png` (22 images, including 4 oracles).

No screenshots were fabricated, and no shell-native capture was used.

# Round 4: integrated final review

Everything above is preserved history. This round reviews the integrated
runtime: 2ee02d0, plus 00dfbd1, which only fixes an account-dialog test
race. The initial images are historical only.

## Director feedback acknowledged (round 4)

- All corrections are integrated. Astra reports passing results for 135 Rust
  behaviour tests, 24 real GPU checks, 283 UI tests, workspace Clippy,
  generated files, SDK typecheck and the native release build. My range fix
  passes Astra's new regression, which failed on the original shader. I did
  not re-run Astra's full matrix this round. The independent checks I did run
  are listed below.
- Native schemas now emit RGB widget hints and cd, lx, m and degree units.
  The spot and directional probes carry explicit identity Transforms.
- Handoff 0016's two-line label fix is integrated and approved. I was allowed
  to make a focused Inspector scroll-affordance fix only if 05 and 06 still
  showed a material UX problem. They do not, so I made no change.
- Account identity: not transcribed. No account-bearing image is committed or
  published. All images remain git-ignored.

## Final GPU evidence (artifacts/cluster-final/, verified directly)

Oracle and CLI identity, decoded pixel comparison:

| Capture | Compared with | Result |
|---|---|---|
| spatial-overflow-96-640x480 | its -oracle | pixel-identical |
| spatial-overflow-96-513x385 | its -oracle | pixel-identical |
| depth-boundary-patches-640x480 | its -oracle | pixel-identical |
| depth-boundary-patches-513x385 | its -oracle | pixel-identical |
| cli-undo-point, cli-restored-point, cli-source-free-point | cli-authored-point | pixel-identical |
| cli-redo-point | cli-dimmed-point | pixel-identical |
| cli-dimmed-point | cli-authored-point | differs (intensity edit), as expected |

Reproducibility: I compared the final images with my independent round 3 run
of all GPU tests in the same worktree (`artifacts/cluster-final-all/`). All 33 non-oracle
shared images match exactly, except one pixel in each spatial-overflow-96
image, which differs by one code value. Oracle identity holds within each run,
so this is a cross-run rounding difference in the 96-light sum, not an
assignment difference.

Changes from the historical initial images:

| Images | Max change | Verdict |
|---|---|---|
| point-near/far, outside-range, spot-*, directional-red, disabled, cluster-64/65/128 | 0 | Pass: byte-identical, initial verdicts stand |
| punctual-hdr-*, punctual-low-dielectric-*, cli-* | 1 | Pass: inverse square preserved where range is far |
| visible-range-edge, both sizes | 39 | Pass: rim removed (below) |
| spatial-overflow-96, both sizes | 16 | Pass: small-range windows, smoother, oracle-identical |
| spatial-clusters, three sizes | 10 | Pass: small-range windows, no seams |
| depth-boundary-patches, both sizes | 10 | Pass: grid pattern intact, oracle-identical |

Range edge, final, linear red stepping inward from the left cutoff:

    640x480: 0.0009, 0.0040, 0.0086, 0.0152, 0.0232
    513x385: 0.0018, 0.0065, 0.0144, 0.0242, 0.0382

The steps grow inward, so the falloff eases into zero. The worst seam score
at the cutoff is 0.56 for 640x480 and 0.92 for 513x385, against 3.40 and
4.59 originally. Tile lines are at the noise floor. No clipping anywhere
except near-white specular cores, with no hue ring at the clip.

New regression fixture: `range-98-percent` and `range-99-percent`, both
321x181. The light sits at 98% and then 99% of its 2 m range from the
surface.

| Capture | Peak sRGB | Peak linear |
|---|---|---|
| range-98-percent | 135 | 0.2423 |
| range-99-percent | 70 | 0.0612 |

    measured ratio 3.96
    C1 window prediction  3.961
    old C0 window         2.01

Both are small soft discs with no ring. **Pass.** The measurement
independently confirms the shipped window is the C1 form.

## Final native capture review (artifacts/lighting-native-final/, real CUA)

| Capture | Size | Verdict |
|---|---|---|
| 01-point-dim-wide | 1440x900 | Pass. Color reads R 1, G 0.8, B 0.5. Intensity 60 cd, Range 100 m. Viewport highlight warm near-white, no seam. |
| 02-point-undo-bright | 1440x900 | Pass. Undo restores 300 cd. Larger highlight, Redo enabled. |
| 03-point-redo-dim | 1440x900 | Pass. Redo restores 60 cd. Viewport matches 01, Redo disabled. |
| 04-spot-wide | 1440x900 | Pass. R/G/B, cd, m and ° units. Identity Transform now shown, so aim (local -Z) is inspectable. Zero intensity leaves the viewport unchanged, as intended. |
| 05-spot-minimum | 1000x650 | Pass, see the scroll verdict. Last visible row partly cut at the Agent boundary. |
| 06-spot-minimum-scrolled | 1000x650 | Pass. Outer degrees and the full Transform are reached by scrolling inside the Inspector. |
| 07-directional-minimum | 1000x650 | Pass. Intensity shows lx. Overlay scrollbar visible at the right edge during interaction. Transform present. |

Viewport (all captures): the sphere lighting matches the headless fixtures.
Edges are aliased because no antialiasing exists, and the rim is dark because
no ambient term exists. Both are known limits, not defects. Units align
right in the field. The degree glyph sits high, which is normal for the
glyph and does not affect legibility.

## Scroll-affordance verdict (05 and 06)

**No correction required in this handoff.** Field access works through
scrolling, as 06 shows. The minimum-size presentation is not a material UX
problem, for four reasons:

1. In 05 the last visible row is visibly cut at the Agent panel boundary. A
   partly visible row is itself a standard cue that the region continues.
2. The Inspector uses the platform overlay scrollbar, which appears during
   interaction (visible in 07). That matches the Hierarchy and the other
   scroll regions, so adding an Inspector-only cue would make the editor
   inconsistent.
3. The Inspector/Agent split is user-resizable by pointer and keyboard
   ("Resize agent panel", minimum 180 px), so a user can trade Agent height
   for Inspector height at small window sizes.
4. Handoff 0016 just approved this Inspector baseline. Changing it for a
   non-material issue would add risk without a clear benefit.

Non-blocking suggestion for a future layout pass, not this handoff: at
small heights, the Agent panel could default to its minimum height while no
provider is attached, since it then only shows a static "Agent not ready"
message. Any edge-fade cue should be applied to every scroll region at once,
not to the Inspector alone.

## Defect ledger (final)

| Raised | Item | Final state |
|---|---|---|
| Follow-up | Range cutoff slope crease | Closed: C1 window (8a5cf38), regression ratio 3.96 |
| Follow-up | Depth patches 00/01 flattened | Closed: intended 1 cm floor, not a cluster omission |
| Follow-up | Dielectric sweep saturated | Closed: punctual-low-dielectric-* |
| Follow-up | Oracle shares shading code | Limit stands: covered by analytic regressions such as the range ratio, not by the oracle |
| Round 3 | Color shown as X/Y/Z | Closed: R/G/B in native 01, 04 and 07 |
| Round 3 | Light units invisible | Closed: cd, lx, m and ° in native captures |
| Round 3 | Minimum-size Inspector clipping | Closed: reachable by scrolling, no correction required |
| Round 3 | Spot aim not inspectable | Closed: identity Transform shown in 04 and 06 |
| Round 3 | Account identity in captures | Standing handling rule: images stay git-ignored and are never published |

## Remaining limits (not defects in this increment)

- Production lighting is not approved. There are no shadows, antialiasing,
  exposure controls, mobile tiers or complete render graph.
- No phase gate approval by this review.
- Native captures contain account identity. Crop or redact before any
  publication. None is committed.
- The quaternion rotation shows aim numerically only. A visual direction
  gizmo would be a later editor feature, not required here.

## Commands run (round 4)

    python3 -I artifacts/review0017/ident.py   artifacts/cluster-final   # oracle and CLI identity
    python3 -I artifacts/review0017/before_after.py artifacts/cluster-final-all artifacts/cluster-final
    python3 -I artifacts/review0017/before_after.py artifacts/cluster-initial artifacts/cluster-final
    python3 -I artifacts/review0017/edge2.py   artifacts/cluster-final   # range-edge linear steps
    python3 -I artifacts/review0017/seams.py   <final range, overflow and patch pngs>
    python3 -I artifacts/review0017/stats.py   artifacts/cluster-final/*.png
    # inline decode of range-98/99-percent peaks and ratio (same decoder)

All ran successfully. Scripts are git-ignored in `artifacts/review0017/`.

## Changed paths (round 4)

- `handoffs/0017-clustered-lighting/result.md` (this file)

The brief was already committed at 1f8033f and is unchanged in this
worktree. No source, test, CSS or schema change was made this round.

## Screenshot paths (round 4, reviewed, not produced)

- `artifacts/cluster-final/`: 45 PNGs, including 4 oracles, the range-98 and
  range-99 regression captures, and 6 CLI captures.
- `artifacts/lighting-native-final/`: 01-point-dim-wide, 02-point-undo-bright,
  03-point-redo-dim, 04-spot-wide, 05-spot-minimum,
  06-spot-minimum-scrolled and 07-directional-minimum (JPEG, account-bearing,
  git-ignored).
