# 0031 pixel-perfect cameras: appearance and Inspector result

## Status

Engine and browser review complete. Native review is pending Astra's captures.

- **Engine rendering: pass, with one concrete renderer request.** The pixel-perfect
  path is exact in every measured case:
  - integer replication;
  - letterbox placement;
  - sub-pixel stability;
  - one-cell steps;
  - inherited roll;
  - projection switching;
  - from-scratch repeatability.

  One defect remains. Camera snapping breaks exact half-pixel ties away from zero, while the
  rasterizer's top-left fill rule breaks them the other way on two of the four
  screen directions. The result is a 1 px flick of a grid-aligned object that moves
  with the camera, on tie ticks only. See "Request for Astra".
- **Browser Inspector: pass.** The Camera Inspector presents Pixel perfect, pixel
  density and the logical resolution at the normal and the 280 px minimum widths.
  Strict typecheck, all 420 UI tests, the build and the five tool tests pass.
- **Native Inspector: not reviewed.** The packet states that Astra supplies native
  captures after UI integration and a rebuild. Browser and headless evidence are
  not native acceptance.

This verdict covers the Camera presentation and the reviewed pixel-perfect camera
path only. It does not approve a phase gate, sprites, atlases, tilemaps, 2D physics
or an agent-ready claim.

## Model and transport

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner started this session in the handoff worktree through the Claude agent
  harness. I cannot independently inspect the ACP transport from inside the
  session. No model substitution occurred.

## Director feedback acknowledged

The current packet has no priority-revision section and no new director feedback
beyond its brief. I applied the standing preferences:
- neutral connected panels;
- restrained accent;
- correct spacing;
- no floating cards;
- no redesign of unrelated UI.

No files from an interrupted attempt existed. The worktree was clean at `04a39ac`.

## Provenance

| Item | Value |
| --- | --- |
| Base commit | `04a39ac` (also `binary.json` `source_commit`) |
| Headless binary | `artifacts/tools/incant_headless` |
| Binary sha256 | `f9a9ed5506cd49b5b2597230b5f34cc8ea35713e6cc57b0321c779abe0fa5cb6` |
| Binary check | Matched `binary.json` before and after every engine call in all runs |
| Browser | Headless Chrome 155.0.8059.40 through pinned `playwright-core` 1.64.0 |
| Node dependencies | `npm ci --ignore-scripts` from the pinned lockfile |

## Changed paths

UI presentation and tests:
- `editor/ui/src/components/inspector/presentation.ts`:
  - the "Pixel perfect" variant name;
  - "Pixel density" and "Resolution" rows with Width and Height entry names;
  - the strict `isPixelPerfectCamera` check behind the field-of-view "Unused" tag.
- `editor/ui/src/components/inspector/FieldView.tsx`: a fixed-length list can carry
  named entries (W, H) with neutral prefixes. A list of another length is shown raw
  as a mismatch.
- `editor/ui/src/styles/app.css`: one neutral `.axis--plain` prefix colour.
- `editor/ui/src/bridge/fixture.ts`: the Camera schema mirror now equals the native
  conversion exactly. A "Pixel Camera" (32 px/m, 320×180) joins the sample scene.
- `editor/ui/src/components/inspector/CameraFields.test.tsx`: parity and behaviour
  tests, listed below.
- `editor/ui/src/App.test.tsx`: the sample hierarchy count rises from 24 to 25 for
  the new camera.

Handoff evidence:
- `tools/pixel_lookdev.py`: builds and renders the engine scene.
- `tools/analyse.py`: measures it from the PNG bytes.
- `tools/capture-inspector.mjs`: the browser captures.
- `screenshots/engine/`: 26 unedited engine PNGs.
- `screenshots/inspector/`: 10 browser PNGs.
- `evidence/`:
  - `analysis-r3.json`;
  - `helper-r3.json` and `helper-r4.json`;
  - `scene.json`;
  - `live_x.generated.ts`;
  - `inspector-report.json`;
  - `screenshots.sha256`.

Ignored, not committed, under `artifacts/0031-pixel-perfect-cameras/`:
- `engine-r3` and `engine-r4`: the two final from-scratch runs with all raw frames.
- `inspector/r1` and `inspector/r2`: the full browser sets.
- Superseded probes:
  - `engine-p1`, which stopped on an expected failure the helper did not yet tolerate;
  - `engine-p2`, `engine-r1` and `engine-r2`, which predate the tie cameras and the diagonal runs.

No Rust, SDK, CI, bridge transport, mutation routing or account settings changed.
No Cargo command ran.

## Commands and results

```sh
npm ci --ignore-scripts                    # pinned lockfile, 0 vulnerabilities
cd editor/ui && npx tsc -b --noEmit        # strict: 0 errors
npx vitest run                             # 16 files, 420 tests passed
npm run build                              # built
python3 -m unittest discover -s tools/tests   # 5 tests OK
node handoffs/0031-pixel-perfect-cameras/tools/capture-inspector.mjs r2
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_lookdev.py artifacts/0031-pixel-perfect-cameras/engine-r3
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_lookdev.py artifacts/0031-pixel-perfect-cameras/engine-r4
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse.py \
  artifacts/0031-pixel-perfect-cameras/engine-r3 artifacts/0031-pixel-perfect-cameras/analysis-r3.json \
  artifacts/0031-pixel-perfect-cameras/engine-r4
shasum -a 256 -c handoffs/0031-pixel-perfect-cameras/evidence/screenshots.sha256   # 36 OK
```

Each helper run printed:
```
binary_unchanged: true, authored_unchanged: true, screenshot_repeats_equal: true,
screenshots: 58, failures: {base-319x180: [1, false], base-320x179: [1, false],
ppu32-256x224-320x180: [1, false]},
play_repeats_equal: {1x: true, 3x-odd: true, diag-1x: true, diag2-1x: true}
```

The baseline before my changes was 385 passing and 2 failing UI tests. Both
failures were Camera-schema parity expectations, as the packet stated.

Not run by me:
- Cargo, Clippy and the GPU tests;
- native capture;
- native WebKit.

## Engine scene

The scene is original and authored only through public commands:
1. `init` creates the project.
2. Mathematical glTF is written by the helper and registered with `import`.
3. One `rpc command.execute` transaction with provenance creates every entity.
4. `project.save` and `validate` follow.

No project JSON was edited.

The scene is a 10 × 5.625 m "pixel test card". Every edge lies on the 1/32 m grid,
so at 32 px/m each logical pixel is either fully inside or fully outside a shape.
Materials are emissive on a black metallic base, so pixels are flat and independent
of lighting. The card contains:
- a 1 px amber ring on the outermost logical pixels, so any crop or offset breaks it;
- red, green, blue and white corner markers, which check orientation and flips;
- vertical and horizontal bar gauges: four bars each of 1, 2, 3 and 4 px with equal gaps;
- 16 × 16 px checkers with 1, 2 and 4 px cells;
- a disc of radius 24 px;
- 2 px strips at 45°, 1:2 and 17°;
- 1 px crosshair lines;
- an 8 × 8 px green mover that scripts move with the camera.

Cameras use pixels per unit 32 and logical 320×180 unless named otherwise. They sit
at z = 10 looking down −Z. Variants:
- sub-pixel offsets in x and y, including exact ±0.5 ties;
- one cell and more than one cell;
- a forward-depth move of 3.3 m;
- a direct 90° roll and the same roll inherited from a parent rig, with 0.3 and 0.7 px rig offsets;
- a 10° roll;
- an orthographic twin with vertical size 5.625 and a legacy perspective twin;
- 16 px/m at 160×90, 32 px/m at 256×224 (8:7), and 32 px/m at an odd 255×143.

Live runs use strict-TypeScript behaviours with ordinary `set_component` commands.
The camera and the mover advance 1/256 m, which is 1/8 logical pixel, per tick:

| Run | Direction | Ticks | Output | Captured |
| --- | --- | --- | --- | --- |
| `1x` | +x, with projection switches | 100 | 320×180 | every tick, 101 frames |
| `3x-odd` | the same script | 100 | 961×541 | every 4 ticks, 26 frames |
| `diag-1x` | −x, +y | 48 | 320×180 | every tick, 49 frames |
| `diag2-1x` | +x, −y | 48 | 320×180 | every tick, 49 frames |

The `x` script switches projection with this sequence:
1. pixel perfect at the start;
2. orthographic at tick 64;
3. pixel perfect at tick 72;
4. legacy perspective at tick 80;
5. pixel perfect at tick 88.

Each switch is visible from the following frame, as with 0030. Every capture
stays within 128 frames and 256 MiB. The largest is about 54 MB raw.

## Measured results

All values come from `evidence/analysis-r3.json`, measured from PNG bytes.

**Native logical frame.** The 320×180 frame has exactly 8 colours: the
background plus 7 emissive colours. No pixel shows an intermediate edge colour.
- Bar gauges measure 1,1,1,1 / 2,2,2,2 / 3,3,3,3 / 4,4,4,4 px with matching gaps, in
  both directions.
- All three checkers are 256/256 correct.
- All 1000 ring pixels are amber.
- All four corner markers are the right colour in the right corner.

**Integer replication and letterboxing.** Every physical pixel is compared with
the logical frame replicated by the expected scale and offset. The letterbox
widths are also measured from the image alone, as whole backdrop rows and
columns. The two methods agree everywhere, with 0 mismatched pixels.

| Output | Scale | Left | Right | Top | Bottom |
| --- | --- | --- | --- | --- | --- |
| 320×180 | 1 | 0 | 0 | 0 | 0 |
| 640×360 | 2 | 0 | 0 | 0 | 0 |
| 960×540 | 3 | 0 | 0 | 0 | 0 |
| 961×541 | 3 | 0 | 1 | 0 | 1 |
| 1001×563 | 3 | 20 | 21 | 11 | 12 |
| 1283×723 | 4 | 1 | 2 | 1 | 2 |
| 800×600 (4:3) | 2 | 80 | 80 | 120 | 120 |
| 360×640 (portrait) | 1 | 20 | 20 | 230 | 230 |
| 1280×360 (32:9) | 2 | 320 | 320 | 0 | 0 |

- **Odd spare pixels.** These go to the right and bottom, as the contract requires.
- **Colours.** Scaled outputs contain only the 8 scene colours plus the backdrop
  `(20, 21, 25)`. Nothing is blurred or filtered.
- **Block sizes.** Every block is exactly s × s pixels.
- **Stretching.** Aspect is never stretched.

Other logical configurations, each against its own scale-1 frame, also have 0
mismatches:

| Camera | Output | Scale | Left / Right | Top / Bottom |
| --- | --- | --- | --- | --- |
| 16 px/m, 160×90 | 320×180 | 2 | 0 / 0 | 0 / 0 |
| 16 px/m, 160×90 | 960×540 | 6 | 0 / 0 | 0 / 0 |
| 32 px/m, 256×224 | 960×540 | 2 | 224 / 224 | 46 / 46 |
| 32 px/m, 255×143 | 320×180 | 1 | 32 / 33 | 18 / 19 |
| 32 px/m, 255×143 | 960×540 | 3 | 97 / 98 | 55 / 56 |

**Explicit failure.** Each of these cases exits 1 and writes no image:

| Camera | Output | Error |
| --- | --- | --- |
| 320×180 | 319×180 | `viewport is smaller than the pixel-perfect camera's 320x180 logical resolution` |
| 320×180 | 320×179 | the same |
| 256×224 | 320×180 | the same, naming `256x224` |

The 256×224 case is wider than its output needs but too short, so a single short
dimension is enough to fail.

**Camera motion.** Each frame is compared at 1x with the base frame shifted by a
whole number of pixels. Every case has 0 residual mismatches. Every 960×540
version is an exact 3x replication of its 1x frame.

| Camera offset, logical px | Content shift | Hash equal to base |
| --- | --- | --- |
| x +0.25, +0.49, −0.49 | none | yes |
| y +0.49 | none | yes |
| depth +3.3 m | none | yes |
| x +0.50 / −0.50 (tie) | −1 / +1 | no |
| y +0.50 / −0.50 (tie) | +1 down / −1 | no |
| x +0.51, +1.00, +1.49 | −1 | no |
| y −0.51 | −1 | no |
| x +2.3, y +2.3 | −2, +2 | no |

Inside a cell the frame is byte-identical, with no shimmer. Crossing the boundary
moves the whole image by exactly one logical pixel. Exact ties round away from
zero on both axes.

**Roll.** These results show that snapping happens in the camera plane, including
inherited roll:
- A direct 90° roll and the same roll inherited from a parent rig are
  byte-identical.
- A rig offset of 0.3 px leaves the frame unchanged.
- A rig offset of 0.7 px shifts it by exactly one pixel along the rolled camera's
  vertical, with 0 mismatches.

The 10° roll keeps its authored rotation. It renders crisp stair-stepped edges
with no blended colours, and its 3x output replicates its 1x frame exactly.

**Ordinary projections are retained.**
- The orthographic twin at 320×180 is byte-identical to the pixel-perfect frame.
- At 960×540 the orthographic twin renders at full resolution. 1507 pixels differ
  from the pixel-perfect 3x frame: 288 on the disc edge, 1219 on the diagonals and
  none elsewhere.
- Neither ordinary projection letterboxes.

**Live sequence.** Frame 0 equals the static base screenshot byte for byte.
- **1x pixel frames.** In all three 1x runs, every pixel-perfect frame shows exactly
  the predicted whole-pixel shift with 0 residual mismatches. That is 85, 49 and 49 frames.
  The shift steps by exactly 1 px every 8 ticks and never moves backwards.
- **Match with static screenshots.** Frames 4 to 11, at camera x from 0.5 to
  1.375 px, equal the static +1.00 px screenshot outside the mover.
- **Orthographic segment.** Ticks 65 to 72 differ from a pure translation by 21, 38,
  106, 213, 112, 46, 28 and 0 pixels on the disc and diagonals. This is the
  sub-pixel shimmer that pixel-perfect removes. It is 0 at tick 72, where the
  camera sits exactly on a cell.
- **961×541 run.** Every pixel-perfect frame is an exact 3x replication of the 1x
  frame from the same tick, with 1 px of backdrop at the right and bottom. The
  orthographic and perspective frames fill the whole output. Switching back
  restores the letterbox on the next pixel-perfect frame.

**Repeatability.**
- Within each run, every screenshot pair and both `play` repeats are hash-equal.
- Across the two from-scratch runs, `engine-r3` and `engine-r4`, all 341 screenshots
  and live frames are byte-identical.
- Authored project and journal bytes are unchanged by rendering and playback
  within each run.
- They differ between runs only because `init` assigns new project and scene IDs.

## Appearance review

I viewed the frames themselves, not only their statistics.

- **Aliasing and blur.** Edges are hard everywhere. The 17° strip shows the
  expected irregular steps, and the disc shows clean stair steps. Neither is
  blurred or ghosted.
- **Pixel blocks.** Blocks are uniform at 2x, 3x, 4x and 6x. No column or row is
  doubled or dropped, including at odd sizes.
- **Stretching.** None. 4:3, portrait, 32:9 and 8:7 all keep the card's proportions.
- **Letterbox placement.** It is centred and uses the neutral backdrop
  `(20, 21, 25)`. That is darker than the card's background `(48, 52, 63)`, so the
  frame edge reads clearly without a bright border.
- **Movement and stability.** The camera is perfectly stable inside a cell and
  steps cleanly across cells. Orthographic motion at the same speed visibly
  shimmers on curves and diagonals.
- **Density mismatch is honest, not hidden.** The card's 1 px details are half a
  logical pixel at 16 px/m:
  - the 1 px checker becomes a solid white square;
  - the 1 px bar group and the top and left ring sides vanish;
  - the right and bottom ring sides survive.

  This is point sampling of art below one logical pixel, not a renderer fault.
  Authors should keep details at least one logical pixel wide at their camera's
  density.

Committed frames in `screenshots/engine/`:
- `base-*`: every replication size.
- `x+0.51` and `xy+2.3` at 960×540.
- `roll10`, `rig90-base`, `ortho` and `persp` at 960×540.
- The three other configurations.
- Live 1x ticks 3, 4 and 5, where tick 4 is the follower flick.
- Live 961×541 ticks 64 (pixel perfect), 68 (orthographic), 84 (perspective) and 92
  (pixel perfect again).
- Live diagonal tick 4.

## Request for Astra: tie-break camera snapping to match the fill rule

Severity is low to medium. Camera behaviour itself is exact. The flaw only shows on
grid-aligned objects that move with the camera, such as a followed player.

- **What happens.** The mover keeps a fixed offset from the camera, so it should hold
  one screen position. In the +x run it jumps 1 px left on 11 of 85 pixel-perfect
  frames: ticks 4, 12, 20 and every 8 ticks after. Those are exactly the ticks where
  the camera sits on a half pixel. In the +x/−y run it jumps by (−1, −1) on 6 of 49
  frames. In the −x/+y run it never moves.
- **Why.** Snapping rounds ties away from zero, as the tie cameras measure. The
  object's edge then lands exactly on a pixel centre. The rasterizer's top-left rule
  includes a pixel whose centre lies on a left or top edge. A follower is stable
  only when its offset from the snapped camera is:
  - in (−0.5, 0.5] along camera right;
  - in [−0.5, 0.5) along camera up.

  Rounding away from zero violates this for +x and −y ties. The two measured
  quadrants confirm the prediction in both directions.
- **Fix.** Snap along camera right with `ceil(v - 0.5)` and along camera up with
  `floor(v + 0.5)`. Here `v` is the camera-plane coordinate times pixels per unit.
  In screen space, which points right and down, that rounds ties toward −∞ on both
  axes. Everything else measured here is unchanged, because only exact ties move.
- **Regression test.** Run the same scene with a grid-aligned follower moving
  1/(8·ppu) per tick in all four diagonal directions. Its screen position must stay
  constant.

Ties are common in practice, because games often step in dyadic sub-pixel units
such as 1/8 px. Followers that are not grid-aligned will still jitter by up to
1 px. That is the stated contract, since objects are not snapped, and it is
authoring guidance rather than a defect.

## Inspector design

The Camera section keeps its schema order:

| Row | Pixel-perfect value | Notes |
| --- | --- | --- |
| Projection | Pixel perfect | |
| Pixel density | `32 px/m` | Indented beneath Projection |
| Resolution `px` | `W 320`, `H 180` | Indented beneath Projection |
| Field of view | `60 °` | Tagged "Unused" |
| Near clip | `m` | |
| Far clip | `m` | |

- **Copy.** "Pixel density" with the schema's `px/m` unit reads as pixels per
  metre without repeating it. The schema help, "Logical pixels per world metre.
  Camera motion snaps to this grid.", is announced and shown on hover.
  "Resolution" was first "Logical resolution". At the 280 px minimum the clamped
  two-line label then cut off the `px` unit, which the first browser set measured,
  so I shortened it.
- **Width and height entries.** The visible prefixes are `W` and `H`. Their
  accessible names are "Logical width" and "Logical height". The help reads "Fixed
  logical width and height, scaled by whole pixels with letterboxing." The
  prefixes use the neutral muted colour, not the X/Y axis tints, because they are
  not spatial axes.
- **Field of view.** The "Unused" tag's sentence is "Not used by pixel-perfect
  projection. Kept for switching back to perspective."
- **Strict "Unused" rule.** The tag appears only when the projection is well formed.
  That means exactly `kind`, `pixels_per_unit` and `reference_resolution`, a finite
  density from 1 to 4096, two whole sides from 16 to 4096, and at most 4,194,304
  pixels. Anything else suppresses the tag and is still shown as authored.
- **Malformed data stays visible.**
  - A resolution with 1, 3 or 0 entries shows the dashed notice "Cannot show as 2
    numbers (logical width and logical height); raw value:" with the raw JSON. It
    never guesses a width or height.
  - Missing or text density or resolution values show the existing number and list
    mismatch notices.
  - Extra fields are flagged by name.
  - Out-of-range values are shown as authored, with the engine diagnostic attached
    to the exact entry: `aria-invalid` on Height only for `/reference_resolution/1`.
  - Unknown kinds now list "perspective, orthographic or pixel_perfect".
- **Tiny numbers.** A density of 2.5e-6 shows as `2.5e-6`, with the exact value on
  hover.
- **Preserved behaviour.** Legacy cameras, the "Default" tag, explicit perspective,
  orthographic rows and every existing test are unchanged.
- **Read-only.** There is no selector, button or write path. The "Read-only" chip
  stays. Tab order is Projection, Pixel density, Logical width, Logical height,
  Field of view, Near clip, Far clip.

Tests added or updated in `CameraFields.test.tsx`:
- native schema parity, now requiring three variants, the density bounds and unit,
  and the two-integer `px` resolution;
- exact fixture equality with the native conversion;
- pixel-perfect rows, names, help and neutral prefixes;
- Tab order and read-only state;
- wrong-length, missing, text and extra-field cases;
- an out-of-range resolution with a diagnostic;
- tiny and fractional densities;
- axe on pixel-perfect and malformed cameras;
- a 20-case `isPixelPerfectCamera` table and a non-mutation check.

**Browser measurement** is recorded in `evidence/inspector-report.json`.

States, each at 1440×900 with a 331 px Inspector and at 1000×650 with the 280 px
minimum, 14 captures in all:
- legacy;
- orthographic;
- pixel perfect;
- pixel perfect with keyboard focus;
- short resolution list;
- tiny density with 4096×1024;
- out-of-range resolution with a diagnostic.

Results across all 14 captures:
- zero horizontal overflow;
- no clipped labels or values;
- no page errors;
- no remote requests;
- zero axe violations, with the contrast rule enabled.

Contrast:

| Element | Contrast |
| --- | --- |
| Labels | 7.98:1 |
| Error label | 6.99:1 |
| Values | 16.28:1 |
| Units | 6.96:1 |
| Tags | 8.62:1 |

At 280 px, "Pixel density" wraps to two lines and the `px` unit drops under
"Resolution", which is the shared vector-unit behaviour. Each W/H box is 75 px
wide and fits `4096`.

These are headless Chrome captures of the built UI. They are not native WebKit
evidence. The short-list, tiny-large and out-of-range states use a labelled
evidence test-double bridge that rejects every command.

## Native captures requested from Astra

After integrating this branch and rebuilding, please capture these states:
- the sample "Pixel Camera" at 1440×874 and at the 280 px minimum;
- a Tab-focus frame on Logical width or Logical height;
- an out-of-range or short resolution camera;
- a legacy camera, for regression.

I will review them and commit only account-free crops.

## Limitations and open items

- **Native acceptance** is open until native captures are reviewed.
- **The tie-break request above** is open.
- **Spacing in the accessible name.** Chrome computes a unit-bearing vector group's
  name as "Resolution , in px", with a stray space before the comma. All existing
  vector rows with units share this. It is harmless when spoken. It is noted for a
  later shared FieldView pass and was not changed here.
- **Unexercised path.** Native chrome masking at physical resolution is outside the
  headless path and was not exercised.
- **Scope of this verdict.** Sprites, atlases, tilemaps and 2D physics are not
  implemented and not reviewed.
- **Gate approval** remains human-owned. No phase gate is approved or claimed.
