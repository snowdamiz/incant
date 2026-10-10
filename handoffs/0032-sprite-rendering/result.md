# 0032 sprite rendering: original art and real-engine review

## Status

Engine art and rendered review are complete for this round. The Sprite Inspector is
deferred until packet 0033 is integrated, as the packet directs.

- **Engine sprite rendering: pass for aligned content, with one renderer defect.**
  Every one of 318 real engine frames matches an independent model of the documented
  contract, with zero unexplained pixels. Regions, pivots, flips, asymmetric nine-slice
  borders, painter order, alpha, depth, sampling, seams, tone, reimport and source-free
  loading all behave as documented. Fresh from-scratch runs are byte-identical.
- **Defect for Astra: nearest-sampling ties.** When texel boundaries fall exactly on
  pixel centres, the texel shown depends on floating-point rounding of normalized sheet
  UVs. A 7 px sprite whose centred pivot sits on a pixel corner shows its last texel
  twice and drops its first when flipped, rotated 180 degrees or mirrored. Reimporting a
  taller sheet with identical texels changed which texels unflipped sprites show. See
  request 1.
- **Diagnostics request.** A reimport that shrinks a texture below its sprite regions is
  accepted silently. Every later render then fails with a generic message that names no
  entity, region or size. See request 2.
- **Look-dev recommendation.** Unlit sprite colours pass through the 3D tone shoulder.
  Grays up to sRGB 234 show exactly, but 252 to 255 all show as 243. Pure primaries gain
  a 30-level floor in their zero channels. See request 3.
- **Sprite Inspector: not started.** No `editor/ui` file was edited. The dependency on
  0033 and the data the Inspector will need are recorded below.

This verdict covers headless engine rendering on one Apple GPU. It does not approve a
phase gate, native editor presentation, atlases, tilemaps, sprite animation libraries,
sprite lighting or shadows, or 2D physics.

## Model, effort and transport

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as stated by the session.
- **Thinking effort:** Max, the session's configured reasoning effort.
- **Transport:** ACP through `tools/handoff/main.py` with the pinned
  `@agentclientprotocol/claude-agent-acp` 0.88.0 adapter and
  `@anthropic-ai/claude-code` 2.1.295. The prompt arrived in that runner's exact
  template. The runner sends it only after ACP confirms model "Opus 5.5" and effort
  "max" (`configure_model_and_effort`). I cannot inspect the ACP handshake from inside
  the session. No model or effort substitution occurred.

## Director feedback acknowledged

The packet's priority revision says engine art and review come first and the UI waits
for the full redesign in 0033. I applied it:
- no `editor/ui` edits, so nothing competes with the redesign;
- original art, real engine scenes, helpers, measurements and this result only;
- the Sprite Inspector dependency is recorded under "Deferred: read-only Sprite
  Inspector".

The worktree was clean at `0d29968` with only `brief.md` in this packet folder. No
files from an interrupted attempt existed.

## Provenance

| Item | Value |
| --- | --- |
| Worktree base | `0d29968` |
| Binary source commit | `39e5c38107451ba44a02ef7b80f27bef11d4141d` (`binary.json`) |
| Headless binary | `artifacts/tools/incant_headless` |
| Binary sha256 | `848b8294854add3bc672e6ab97e1c3dc1f5f39a9c5b1c234e61e5edace5820ba` |
| Binary check | Matched `binary.json` before and after every engine invocation |
| GPU adapter | Apple M5 Pro, reported by the screenshot command |
| Host | macOS 26.6 (25G72) |
| Tooling | Python 3.14.8, Node 22.23.3, pinned TypeScript 5.9.3 and @swc/core 1.16.13 |

## Changed paths

All changes are inside `handoffs/0032-sprite-rendering/`.

Tools, standard-library Python:
- `tools/sprite_art.py` generates all original art from code.
- `tools/sprite_lookdev.py` builds the project through public commands and renders it.
- `tools/reference.py` is an independent CPU model of the documented contract.
- `tools/analyse.py` measures every frame from PNG bytes.
- `tools/crops.py` copies the frame set and makes labelled zoom crops.
- `tools/test_reference.py` holds seven self-checks for the reference model.
- `tools/pngio.py` reads and writes PNG files.

Evidence:
- `evidence/analysis-r1.json`, including the cross-run comparison with run 2.
- `evidence/helper-r1.json` and `evidence/helper-r2.json`.
- `evidence/scene-r1.json`, the authored entities, cameras and regions.
- `evidence/live_e.generated.ts`, one of the eight strict-TypeScript behaviours.
- `evidence/art/`, the source images and `regions.json`.
- `evidence/screenshots.sha256`, hashes of every committed image.

Screenshots:
- `screenshots/engine/`: ten unedited engine frames and eleven zoom crops.
- `screenshots/art/`: two previews of the source art. These are not engine frames.

Ignored and not committed, under `artifacts/0032-sprite-rendering/`:
- `engine-r1` to `engine-r3`, the final from-scratch runs with all raw frames.
- `engine-p1` to `engine-p3`, superseded probes made before the art and checks were final.
- `smoke-1`, scratch tools, analysis files and logs.

No Rust, SDK, transport, Cargo, CI or account file changed. No Cargo command ran.

## Commands and results

```sh
npm ci --ignore-scripts                                   # pinned lockfile, 0 vulnerabilities
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r1
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r2
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r3
python3 -I handoffs/0032-sprite-rendering/tools/analyse.py \
  artifacts/0032-sprite-rendering/engine-r1 artifacts/0032-sprite-rendering/analysis-r1.json \
  artifacts/0032-sprite-rendering/engine-r2                 # about 150 s
python3 -I handoffs/0032-sprite-rendering/tools/crops.py \
  artifacts/0032-sprite-rendering/engine-r1 artifacts/0032-sprite-rendering/crops-r1
python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py   # 7 tests OK
python3 -m unittest discover -s tools/tests                          # 7 tests OK
cd editor/ui && npx tsc -b --noEmit && npx vitest run && npm run build
cd handoffs/0032-sprite-rendering && shasum -a 256 -c evidence/screenshots.sha256   # 31 OK
```

Each helper run printed:
```
binary_unchanged: true, engine_calls: 114, authored_unchanged: true,
screenshot_repeats_equal: true, screenshots: 25, source_free_shots: 8,
play_repeats_equal: all 8 true, play_frames: 34 each
```

The UI checks are an unchanged baseline, not Inspector evidence. Strict typecheck
reports 0 errors, all 420 tests in 16 files pass, and the build succeeds.

Not run by me:
- Cargo, Clippy, the Rust and GPU test suites and portable checks;
- native editor captures, native WebKit and hosted CI;
- Windows, Linux, Android or iOS GPUs.

## Original art

All art is drawn from code in `tools/sprite_art.py` and regenerates byte for byte.
- **Wisp:** a 16 px rendition of the project's own mascot. It was rasterized from the
  vector trace kept in handoff 0004 at 50% coverage, then hand-finished with a deep
  purple outline, upper-left light and a glint. Four idle frames form the flipbook:
  rest, sway left, blink and sway right. The flame and eyes make it asymmetric on both
  axes. A teal palette twin exists for the reimport test.
- **Dialog panel:** 24 × 24 px with asymmetric borders of 3, 6, 4 and 8 px (left,
  right, top, bottom). Each edge is uniform along its stretch axis. Each corner has a
  unique mark: red, green, blue and a gold page tab.
- **Measurement art:** an orientation key, a 7 × 7 ruler whose every texel has a
  unique colour, a warm orb and a white aura with magenta RGB hidden under zero alpha,
  a crate inside an opaque magenta guard ring, and a sky strip beside a magenta guard.
- **Scenery:** two tiling grass tiles, a soil tile, stars, a crescent moon and a fog band.
- **Swatches:** 16 tone swatches, also as a 16-bit PNG twin, a 16 × 16 ramp holding
  every 8-bit gray once, and white and black alpha ramps.
- **Text:** a 3 × 5 pixel font for labels, and a gem badge in teal, orange and an
  undersized 8 × 8 variant.

## Engine scene

Everything is authored through public commands:
1. `init` creates the project.
2. Texture sources are copied into it and registered with `import --texture-usage
   color`. The occluder glTF is registered with `import`.
3. One `rpc command.execute` transaction with provenance creates all 315 entities:
   290 sprites, three parent transforms, an emissive glTF occluder and 21 cameras. The
   engine reports no diagnostic stand-in geometry for the parents.
4. `project.save` and `validate` follow.

No project or journal file is edited. Five 320 × 180 boards at 16 px per metre expose:

| Board | Content |
| --- | --- |
| A | source strip and frames, guarded crate, five pivots with origin markers, rulers on grid and aligned pivots, flips, mirror parent, rotations, tints |
| B | nine-slice native, 72 × 44 next to a simple stretch, three compressed targets, flips, densities 8 and 32, linear, rotated 12° |
| C | layer, order, ID tie, Z ignored, interleaved textures, 50% overlaps over dark, light and empty backdrop, alpha ramps, opaque-mesh depth, 0–255 gray ramp |
| D | 4× nearest and linear orbs, linear orb over a light plate, 2× Wisp, rotated crates, two 18-tile rows, tone swatches, sky strip, rotated tile row |
| E | showcase: sky gradient, stars, moon, fog, grass and soil, crates, Wisp with aura, gem, dialog with text |

Cameras:
- every board has a pixel-perfect camera at 16 px per metre and 320 × 180, captured
  at 320 × 180 and 960 × 540;
- every board has an orthographic camera at an exact 3× (vertical size 11.25 m at
  960 × 540);
- every board has an orthographic camera at a fractional 3.75× with a sub-pixel offset;
- boards B, D and E have a 7° rolled orthographic camera;
- board A also has an odd 319 × 179 camera and one offset by +0.6 and +0.2 px.

Live runs use eight strict-TypeScript behaviours with ordinary `set_component` commands
and no source files. Each moves the pixel-perfect camera and the Wisp together by
1/8 px per tick in one of eight directions, for 33 ticks. The Wisp's source region
advances to the next idle frame every 4 ticks, which makes a flipbook. Every tick is
captured, twice per direction.

## Measured results

All values come from `evidence/analysis-r1.json`, measured from PNG bytes.

**Contract model.** `tools/reference.py` predicts each output pixel from the published
contract. It snaps vertices to 1/256 px, applies the top-left fill rule and premultiplies
before filtering. It also models f16 blending, the tone curve and the backdrop. It
reproduces `incant_2d`'s unit-test vectors and the tone values stated in the shader.
Where a pixel centre lies exactly on a sprite edge, or a nearest sample lies exactly on
a boundary between different texels, it marks the pixel ambiguous instead of guessing.

| Frames compared | Count | Unexplained pixels |
| --- | --- | --- |
| Static screenshots | 25 | 0 |
| Reimport screenshots | 21 | 0 |
| Live frames, 8 directions × 34 | 272 | 0 |

Static frames hold 9,733,901 pixels. Outside ambiguous pixels the differences are:

| Sampling | Identical | 1 level | 2 levels |
| --- | --- | --- | --- |
| Nearest or uncovered | 7,181,024 | 8,905 | 0 |
| Linear | 2,210,412 | 313,136 | 4 |

20,420 static pixels are ambiguous and 1,812 of them differ from the model's choice.
All 1,812 lie on non-integer stretches or half-pixel placements:

| Sprite | Pixels |
| --- | --- |
| Simple panel stretched 24 to 72 × 44 | 772 |
| Nine-slice at density 32 | 367 |
| Rulers on a grid-point pivot | 372 |
| Sky strip stretched 32 to 38 px | 140 |
| Nine-slice 44 × 9 | 136 |
| Nine-slice 8 × 10 | 23 |
| Board edge and rotated crate | 2 |

**Repeatability.**
- Within each run, all 25 screenshot pairs and all 8 play pairs are hash-equal.
- Runs 1 and 2 were built from scratch and match byte for byte across all 623 PNGs.
- A third run with the final committed tools matches run 1 across all 631 images,
  which adds the 8 generated source images. Its analysis is identical, apart from the
  random scene ID that `init` assigns, which appears inside one rejection message.
- Rendering and playback leave the project and journal bytes unchanged.

**Pixel-perfect framing.**
- All five 960 × 540 frames are exact 3× replications of their 320 × 180 frames.
- The 319 × 179 frame equals the base frame without its first column and first row.
  57,099 of 57,101 pixels are identical, and 2 linear pixels differ by one level.
- A camera offset of +0.6 and +0.2 px snaps one pixel to the right, so the frame equals
  the base frame moved one pixel left. 57,419 of 57,420 pixels are identical, and one
  linear pixel differs by one level.

**Orientation.** Every pair below differs in 0 pixels:
- flip X against a horizontal mirror, and flip Y against a vertical mirror;
- both flips against a 180° turn, and a 180° transform against both flips;
- a mirror parent with scale −1 against flip X;
- a 90° transform against a counter-clockwise quarter turn on screen;
- the nine-slice flips against mirrors of the unflipped panel;
- the 24 × 24 nine-slice against the simple 24 × 24 panel.

**Painter order and depth.** All 13 probes pass:
- layer beats order, order beats ID, and equal values draw the larger ID last;
- a far sprite on layer 1 covers a near sprite on layer 0;
- interleaved key and badge textures keep their order;
- the opaque quad hides sprites behind it, including a layer-100 sprite;
- a sprite in front of the quad draws over it.

**Seams.** No background pixel appears between tiles or nine-slice cells. That holds in
all eight frames checked: pixel-perfect B and D, and exact 3×, fractional and rolled
orthographic B and D.

**Filtering fringe.** The 4× linear orbs hide magenta RGB under zero alpha.
- **Engine:** no pixel moves toward magenta by 3 levels or more, and the worst shift is
  1 level. No pixel over the light plate is darker than both the plate and the orb core.
- **Counterfactual:** the same frame with straight-alpha filtering would tint 1,128 pixels,
  by up to 35 levels. The premultiplied path therefore prevents a measurable fringe.

**Bleeding.** No frame contains magenta-like pixels. The model never samples outside a
region, and all guarded regions matched it. That covers the crate ring, the sky guard,
the aura between two guards and the panel guards.

**Live follow and flipbook.**
- Frame n shows the state after n scripted ticks.
- In all eight directions and all 272 frames, the Wisp's box stays at (98, 113) to
  (109, 127).
- The background steps one pixel only on camera snaps.
- The idle frame changes every 4 ticks.
- This confirms the corrected half-pixel camera ties for a followed sprite.

**Ordinary orthographic at exact 3× against pixel-perfect 3×.** Pixels differ only on
linear, rotated, half-pixel or stretched sprites:

| Board | Differing pixels |
| --- | --- |
| A | 8,054 |
| B | 17,624 |
| C | 4,764 |
| D | 57,442 |
| E | 102,286 |

The by-sprite lists in the analysis contain no aligned nearest sprite.

**Reimport through `import`.** Every accepted step keeps the asset ID.

| Step | Outcome |
| --- | --- |
| Teal sheet | New fingerprint. Frames match the model with teal texels. Only pixels drawn from Wisp regions change, and board B is unchanged. |
| Orange badge | Frames match the model. |
| 8 × 8 badge | Import exits 0. Every screenshot then exits 1 with no image, including board A, which never draws the badge. |
| Badge restored | Original fingerprint. Board C equals its earlier frame byte for byte. |
| Sheet as linear | Rejected with exit 1 and "sprite requires a color texture asset" per sprite. Project and journal unchanged. |
| Sheet restored | All five boards and orthographic E equal the original frames byte for byte. |
| Taller sheet, 128 × 80, same texels and regions | Frames match the model, but 43, 213 and 20 pixels change on boards A, B and D. All are ambiguous ties, except one linear pixel that moves by one level. |
| Restored again | Board E equals the original byte for byte. |

The 8 × 8 badge failure reads:
```
Error: "sprite source region must be nonempty and fit inside a texture of at most 8192 pixels per axis"
```

**Source-free.** Removing all four PNG sources and the glTF and bin leaves eight frames
byte-identical to the frames rendered with sources present.

**Tone.** Values come from 8-bit swatches. The 16-bit PNG twin gives identical results
for all 16 swatches.

| Authored sRGB | Shown | Shown with 0.5 tint |
| --- | --- | --- |
| 128 gray | 128 | 92 |
| 200 gray | 200 | 146 |
| 231 and 232 gray | unchanged | 170 |
| 240 gray | 238 | 176 |
| 255 white | 243 | 188 |
| 255, 0, 0 | 243, 30, 30 | 188, 0, 0 |
| 255, 214, 128 | 243, 205, 125 | 188, 157, 92 |
| 240, 200, 40 | 238, 198, 42 | 176, 146, 26 |
| 92, 60, 212 (Wisp body) | unchanged | 65, 42, 155 |
| 231, 120, 20 | unchanged | 170, 86, 11 |

The gray ramp measures every 8-bit value directly:
- grays 0 to 234 show exactly;
- the 21 inputs from 235 to 255 share only 10 outputs;
- 240 shows as 238, 246 as 240, and 252, 253, 254 and 255 all show as 243.

So near-white shades merge. The tint is a linear-light multiplier, so 0.5 turns 128
into 92, not 64.

**Alpha.** Each layer below is 50% over the board background. The engine matches the
linear-light contract at every probe. A paint program blending in sRGB space would
show darker results.

| Overlap | Engine | sRGB-space blend |
| --- | --- | --- |
| Red | 158, 48, 58 | 119, 43, 56 |
| Red and green | 124, 140, 90 | 96, 114, 84 |
| Red, green and blue | 104, 130, 173 | 86, 117, 153 |

Over 128 gray at alpha 128, white shows 205 against a paint program's 192, and black
shows 92 against 64.

**Budget.** Each engine invocation stays within the packet's limits. A screenshot writes
one frame, at most 960 × 540 or 2.0 MiB raw. A play capture writes 34 frames, or 7.5 MiB
raw. A whole helper run makes 114 invocations and writes 623 frames, 209 MiB raw in
total. Output directories and files are never overwritten.

## Appearance review

I viewed the frames themselves as well as the statistics.

- **Pixel-perfect frames are what a pixel artist expects.**
  - Edges are hard and texel blocks are uniform at 3×.
  - There is no shimmer while following.
  - Region selection is clean, with no neighbour bleed.
- **Nine-slice reads correctly.**
  - Corners stay crisp at their source density.
  - Edges stretch only along their length, so the gold tab and gems never distort.
  - The simple stretch beside it shows the corner distortion the mode avoids.
  - Compressed targets keep proportional corners.
  - Density 32 on a 16 px per metre camera drops border texels unevenly. Authors should
    keep border density at or below the camera density for pixel art.
- **Ordinary orthographic at 3.75× behaves as expected.** Nearest pixel art shows
  uneven 3 and 4 px texel widths, visible on the Wisp's 1 px outline. That is correct
  behaviour, and a reason to use pixel-perfect cameras for pixel art.
- **The 7° roll gives clean stair steps.** There are no cracks in tile rows or panels.
- **Linear sampling suits only smooth art.** On pixel art, as on the 2× Wisp and the
  linear panel, it reads soft. Nearest is the right default. Linear orbs, aura and sky
  are smooth, with no halo, dark ring or colour fringe.
- **Painter order and depth read naturally.** Overlaps never flicker between runs.
- **Highlights are slightly muted.** The tone shoulder dims full white to 243, so eye
  whites, star centres and the white aura core are slightly muted. Pure primaries lose
  saturation. This is fine for 3D look-dev but costs palette fidelity in 2D (request 3).
- **Translucency is lighter than in paint programs.** The aura, fog and 50% overlaps
  match the linear-light contract (open question 2).
- **The showcase reads as one scene.** The night sky, moon halo and fog give depth. The
  purple aura and teal gem glow are tintable from one white texture. The dialog text is
  legible at 1×.

Committed frame set in `screenshots/engine/`:
- `board-a` to `board-e`: pixel-perfect 960 × 540 frames.
- `board-e-showcase-ortho-3.75x`, `board-d-ortho-3.75x` and `board-b-nine-slice-ortho-roll7`:
  ordinary orthographic frames.
- `reimport-teal-sheet-board-e` and `reimport-orange-badge-board-e`: 320 × 180 frames.
- Zoom crops, each a nearest-neighbour enlargement with `-zoomN` in its name:
  - the ruler station, with the 64 px sheet beside the 80 px sheet;
  - nine-slice flips and small targets;
  - the linear orb fringe;
  - tone swatches, the gray ramp, alpha and depth;
  - live follow at ticks 3 to 5 and flipbook frames 0, 4, 8 and 12.
- `seams-rolled-tiles-ortho-crop`: an unscaled crop of the rolled orthographic frame.

## Requests for Astra

### 1. Nearest-sampling ties depend on rounding and sheet size (defect, medium for pixel art)

**What happens.** The ruler gives every texel a unique colour, so each rendered pixel
names its texel. Its edges lie on pixel centres when an odd-sized sprite uses a centred
pivot on a grid point.

| Placement | Columns, left to right | Rows, top to bottom |
| --- | --- | --- |
| Grid pivot, unflipped | 0 to 6, once each | 0 to 6, once each |
| Grid pivot, flip X | 6, 6, 5, 4, 3, 2, 1 | 0 to 6 |
| Grid pivot, flip Y | 0 to 6 | 6, 6, 5, 4, 3, 2, 1 |
| Grid pivot, 180° transform | 6, 6, 5, 4, 3, 2, 1 | 6, 6, 5, 4, 3, 2, 1 |
| Grid pivot, mirror parent | 6, 6, 5, 4, 3, 2, 1 | 0 to 6 |
| Pixel-centre pivot, every orientation | each texel once | each texel once |

After reimporting the same texels in a 128 × 80 sheet, the unflipped grid-pivot ruler
shows rows 0, 1, 2, 3, 4, 5, 5. Flip Y shows 6, 5, 5, 4, 3, 2, 1. The same reimport also
changed 213 pixels of nine-slice art on board B and 20 pixels of the sky strip on board D.
No texel or region changed.

**Why.** The shader samples normalized sheet UVs with the hardware nearest filter. At an
exact boundary, the chosen texel follows f32 interpolation and the sheet's dimensions.
Flips and mirrors reverse the texel direction, so an exact tie picks the texel on the
wrong side of the boundary. That repeats the outermost texel and drops the opposite one.

**Proposed fix.** This is a renderer change for Astra to integrate and verify. For
nearest sampling only:
1. Pass the region-local texel coordinate `t` after flips, in texels rather than
   normalized units.
2. In the fragment shader, pick
   `i = clamp(floor(t + e * (sign(dpdx(t)) + sign(dpdy(t)))), 0, size - 1)` per axis,
   with `e` about 1/1024 texel.
3. Fetch with `textureLoad` at the region origin plus `i`.

An exact tie then always picks the texel on the screen-right or screen-down side. That
matches the top-left fill rule, as the camera tie fix did. It is independent of flips,
mirrors, 180° turns and sheet size. Linear sampling keeps its current path.

**Regression test.** Render the 7 × 7 unique-texel ruler on a grid-point pivot in six
orientations: unflipped, flip X, flip Y, both, 180° and a mirror parent. Use sheets of
64 and 80 px height.
- Each case must show every texel exactly once, in screen order.
- Both sheet sizes must give identical frames.
- Aligned placements and integer scales must stay byte-identical to today.

A separate authoring option, pixel-grid snapping of sprite vertices on pixel-perfect
cameras, would avoid half-pixel placements altogether. It is open question 3, not part
of this fix.

### 2. Undersized reimports fail late and generically (diagnostics, low to medium)

A reimport that shrinks a texture below its sprites' regions commits without warning.
Every later render of the project then fails, including scenes that never draw that
sprite. The message names neither the entity, the texture, the region nor the actual
size, and its "at most 8192 pixels" wording points at the wrong limit. The failure is
explicit, so the contract holds.

Requests:
- name the entity, texture asset, region and cooked size in the render error;
- report, at import or `validate` time, the sprites whose regions no longer fit the new
  texture.

### 3. An exact display path for 2D palettes (look-dev recommendation)

Sprites share the 3D tone curve, which is the identity only below a 0.8 linear peak.
Measured on the gray ramp and swatches, 2D palettes above sRGB 234 therefore shift:
- 252 to 255 all become 243, so the top four grays are indistinguishable;
- 255, 0, 0 becomes 243, 30, 30;
- 240, 200, 40 becomes 238, 198, 42.

Recommendation: offer an authored display choice for 2D, such as a camera or project
setting of "neutral" (today's curve, kept as the default) or "clamp". "Clamp" would
replace the shoulder with a per-channel minimum of 1.0.

Acceptance: on a "clamp" camera, every 8-bit sRGB value of an opaque unlit sprite
round-trips exactly, and today's frames are unchanged on "neutral". The setting's level
and name are Astra's and the director's call.

## Deferred: read-only Sprite Inspector

**Dependency.** This work waits for packet 0033's Inspector redesign to be integrated.
It must then use that design's components and conventions. Nothing in `editor/ui`
changed in this round, and there are no Inspector captures or UI tests for sprites.

**Presentation notes for the next round**, drawn from this review:
- **Coordinate origins.** The component mixes two origins. The source region is in
  pixels from the top left, and the pivot is normalized from the bottom left. Each row
  must say which origin it uses, for example "Source, px from top-left" and "Pivot,
  0–1 from bottom-left".
- **Border order.** Nine-slice borders are stored as left, right, top, bottom. That is
  not CSS order, so every entry needs its own label.
- **Units.** Show border density in px/m. Show size in metres.
- **Tint.** Label tint "linear" and show a swatch. The tone and alpha results above
  explain why authors will otherwise misread it.
- **Mode rows.** Show borders and density only for nine-slice. Keep malformed values,
  engine diagnostics and the "Read-only" chip as the Camera Inspector does.

**Data binding needed from Astra.** The Inspector needs each texture asset's cooked
width and height. It can then show "Source 0, 0, 64 × 16 of 128 × 64" and flag a region
that no longer fits before a render fails. The project document does not store cooked
dimensions today.

## Limitations

- **Hardware.** One GPU and one operating system: Apple M5 Pro through the headless
  binary. No other GPU or platform was tested.
- **Native editor.** Not reviewed. Engine frames are not native Inspector acceptance.
- **Perspective cameras.** Sprites under perspective cameras were not reviewed. The
  model covers orthographic and pixel-perfect cameras only.
- **Minified sprites.** No mipmaps are used, so linear sprites drawn smaller than their
  texels will alias. Atlas packing and extruded mip levels remain open, as the spike states.
- **Not supplied, so not reviewed:**
  - transparent 3D interleaving;
  - atlas packing and tilemaps;
  - sprite animation libraries;
  - sprite lighting and shadows;
  - 2D physics.

  The flipbook here uses ordinary scripted component changes. It is not an animation
  library.
- **Model coverage.** The model's agreement covers the frames listed. Ambiguous pixels
  are reported, not judged.
- **Gate approval.** No phase gate is approved or claimed. Requests 1 and 2 and the
  Inspector are open.

## Open questions

1. Should 2D work get an exact "clamp" display path (request 3)? Should it be set per
   camera or per project?
2. Should translucent sprites stay in linear light, documented in the Inspector, or
   should 2D layers also offer display-space blending to match paint programs?
3. Is optional pixel-grid snapping of sprites on pixel-perfect cameras wanted as an
   authoring feature, beside the tie fix in request 1?
