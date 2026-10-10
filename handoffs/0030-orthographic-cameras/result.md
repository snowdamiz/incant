# 0030 orthographic cameras: appearance and Inspector result

## Status

Final native review complete. Final scoped verdict: **pass for engine, browser
and native Camera presentation, with no new Camera defect.**

- **Native Camera Inspector: pass.** I reviewed all ten original native JPEGs
  from integrated source `59f3384`. Projection, vertical size, units, the
  "Default" and "Unused" tags, tiny values, minimum-width spacing and keyboard
  focus all render as designed. No Camera UI fix was needed.
- **Engine appearance: pass, unchanged.** No rendering code changed and the
  binary SHA still matches. The measured shadow-distance limitation remains, as
  documented below.
- **Browser Inspector: pass.** Strict typecheck, all 368 UI tests on this branch,
  the build and the five tool tests pass. The earlier bridge test failure is
  resolved by Astra's `b769347`.

This verdict covers the read-only Camera presentation and the reviewed
orthographic rendering only. It does not approve a phase gate, an authored-camera
viewport selection feature, an agent-ready claim, pixel snapping, sprites,
tilemaps or 2D physics.

## Model and transport

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner started this session in the handoff worktree through the Claude
  agent harness. I cannot independently inspect the ACP transport from inside
  the session. No model substitution occurred.

## Director feedback acknowledged

**Final native captures, 2026-10-10.** Astra supplied ten native captures under the director’s screen-capture authorization
from `59f3384` and asked for a final scoped native verdict or a concrete defect.
I did the following:
- verified every image against `manifest.json`;
- confirmed `integrated-ui.diff` adds only PR25's compact Agent and compound
  Inspector, and leaves the Camera paths intact;
- reviewed every state;
- committed only account-free Inspector crops;
- re-ran tests before rewording the earlier bridge failure.

Because there was no Camera defect, there was no UI change for Astra to
integrate or recapture.

**Correctness review, 2026-10-10.** This was applied in `cb5d2db`:
- tiny values stay nonzero;
- the strict orthographic check gates the "Unused" tag;
- defaults, units and order come from the schema, and the "Default" tag stays.

**First checkpoint.** `d9f8769` was accepted for its engine and browser scope.

The standing preferences still apply: neutral connected panels, clean spacing,
a restrained accent and no floating cards. The native captures confirm Assets
in the left workspace and Problems/Console/History in the bottom dock.

## Provenance

| Item | Value |
| --- | --- |
| Engine captures source | `298b846` |
| Headless binary | `artifacts/tools/incant_headless` |
| Headless binary sha256 (`binary.json`, re-verified) | `ce79c966d5d4264482829904c0fb9158f244b61fb7ae1dac1c8150c521a4d2ff` |
| Native captures source | `59f3384cf8e1626da99b7ff9817f297f84482d97` |
| Native app sha256 | `7a9cb8a5285ccda04b98cf9ff55e2206b4543469b29304dce4ddea1388ef6c54` |
| Integrated headless sha256 | `da0f407d4a29788e4a1496ce5a5700e7a61baa306562452fa203999254aa9303` |
| Native project sha256 | `180a615e13b4cc2ac5e9b5dff7adad208ebe2c20c31509e5e29e54392f5f95e0` |
| Native image hashes | all 10 match `manifest.json` |
| This review's base | `5ac55b4` |

The native project is my engine-r1 scene with two cameras Astra added through
shared commands: "Camera explicit perspective" and "Camera tiny values". The
native viewport uses its existing preview camera. These are Inspector review
captures, not a claim that the viewport renders through an authored camera.

## Changed paths

This final review:
- `screenshots/native/*.jpg`: 10 account-free Inspector crops.
- `evidence/native-crops.json`: each crop's hash, source image, source hash,
  entity and logical size.
- `evidence/screenshots.sha256`: now covers the engine, browser and native
  images, 28 in total.
- `result.md`.

No UI source changed in this round.

Earlier rounds:
- `cb5d2db`: the correctness fixes, schema-driven defaults and the browser
  capture set `r2`.
- `d9f8769`: the Camera profile, the engine look-dev helpers, the engine frames
  and evidence.

Ignored, not committed:
- `artifacts/0030-native-captures/`: Astra's full native JPEGs, which show the
  saved account label.
- `artifacts/0030-orthographic-cameras/native-review/`: the crop working copies.
- Earlier engine runs, analyses and browser sets.

No Rust, SDK, bridge, mutation routing, CI or account settings were changed in
any round.

## Commands and results

This final review:
```sh
python3 -I -c '...'      # all 10 native JPEG sha256 values match manifest.json
sips -c H W --cropOffset Y X <copy>.jpg   # Inspector crops below the titlebar
cd editor/ui && npx tsc -b --noEmit       # strict: 0 errors
npx vitest run                            # 15 files, 368 tests passed
npm run build                             # built
python3 -m unittest discover -s tools/tests   # 5 tests OK
shasum -a 256 artifacts/tools/incant_headless # matches binary.json
shasum -a 256 -c evidence/screenshots.sha256  # 18 earlier images verified before adding the crops
```

Astra reports results on the combined integrated root that I did not run:
- 318 Rust tests, Clippy, 387 UI tests and build, five tool tests, contracts,
  and the native release and package;
- the public camera save and repeat workflow on the frozen integrated binary;
- an independent analysis that reproduces my numbers and verifies all 72 engine
  images byte for byte.

The first checkpoint ran the engine commands below. They are not repeated
because rendering did not change.
```sh
python3 -I handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py artifacts/0030-orthographic-cameras/engine-r1
python3 -I handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py artifacts/0030-orthographic-cameras/engine-r2
python3 -I handoffs/0030-orthographic-cameras/tools/analyse.py artifacts/0030-orthographic-cameras/engine-r2 artifacts/0030-orthographic-cameras/analysis-r2.json
```

Each of those helper runs reported:
```
binary_unchanged: true, authored_unchanged: true,
screenshot_repeats_equal: true, play_repeats_equal: {16x9: true, 4x3: true}
```

Not run by me: Cargo, GPU tests, and native capture. Astra captured natively
through CUA, and I did not use computer use.

## Engine scene

The scene is original and authored only through public commands:
1. `init` creates the project.
2. Mathematical glTF is written by the helper and registered with `import`.
3. One `rpc command.execute` transaction with provenance creates every entity,
   followed by `project.save` and `validate`.

No project JSON was edited.

The scene contents:
- a 1 m checker floor;
- two blue rails running 23 m along world Z;
- three pillars of identical authored size, 1 × 1.6 × 1 m, at z = 4, −3 and −10;
- a shadowed directional sun;
- a warm point lamp beside the middle pillar and a cool lamp beside the far one.

Every camera shares one oblique axis, yaw 35° and pitch −30°. Cameras differ
only in distance along that axis, projection and `vertical_size`. Near is 0.1,
far is 80 and fov is 40°.

Live switching uses a strict-TypeScript behaviour that sends ordinary
`set_component` commands, run by `play`. The sequence is:
1. legacy perspective at distance 16;
2. orthographic, vertical size 9, at tick 10;
3. moved to distance 24 at tick 20;
4. vertical size 6 at tick 30;
5. explicit `{kind: "perspective"}` at tick 40.

Frames are captured every 5 ticks, 11 frames per run, about 23 MB raw at
960×540.

## Measured observations

All values come from `evidence/analysis-r2.json`. Analytic boxes project each
pillar's 8 corners through the authored camera.

**Equal size across depth and agreement with analytic projection.** Pillar
silhouette heights in pixels:

| Frame | Near | Middle | Far | Predicted | Worst edge error |
| --- | --- | --- | --- | --- | --- |
| ortho d16 V9, 960×540 | 124 | 124 | 124 | 124.92 | 0.48 px |
| ortho d16 V9, 800×600 | 139 | 138 | 139 | 138.80 | 0.46 px |
| ortho d16 V13, 960×540 | 86 | 86 | 86 | 86.48 | 0.33 px |
| persp d16, 960×540 | 147 | 97 | 72 | 147.1 / 96.5 / 71.7 | 0.46 px |

- Every pillar fully inside the frame matches its analytic box within 0.69 px.
- At V6, the near and far pillars are cropped by the frame, which is the expected
  zoom. The middle pillar measures 186 against 187.38 predicted.
- The 16:9 and 4:3 heights are the same fraction of frame height, 0.2313. Vertical
  extent is preserved and only width follows aspect.

**Parallel lines.** The angle between the two fitted rails is 0.002° to 0.10° in
every orthographic frame. The perspective frame measures 7.67° at 16:9 and
7.62° at 4:3. The residual from each rail's fitted straight line is 2.5 to
6.1 px RMS. That spread equals the rail's own rendered thickness, and I saw no
curvature.

**Moving along the forward axis at fixed V9.** All distances render identical
silhouettes and identical pillar boxes. Pixel differences:

| Pair | Changed pixels | Max channel delta |
| --- | --- | --- |
| d16 vs d24, 16:9 | 1.7 % | 11 |
| d16 vs d34, 16:9 | 3.5 % | 43 |
| d16 vs d34, 4:3 | 4.2 % | 20 |

The diff masks place every change on directional-shadow edges and faint
self-shadow texture on pillar faces. The lit floor, point-light pools and
specular highlights are byte-identical. Parallel-view shading and clustered
local lights are therefore distance-invariant here.

**Projection switching.** These live `play` frames are byte-identical to the
static `screenshot` of the same pose and projection, at both aspects:
- tick 5, legacy perspective;
- tick 15, orthographic V9;
- tick 25, orthographic V9 after the move.

The commands take effect on the next tick, as documented.

**Repeatability.** Within each run, every screenshot pair and both `play`
repeats are hash-equal. Across two independent from-scratch runs, all 28
screenshots and 22 live frames are hash-equal. The authored project files differ
between runs because `init` assigns new project and scene IDs. Within a run,
authored files are unchanged by playback.

## Appearance review

- **Framing.** V9 at 16:9 frames all three pillars on a clean diagonal. 4:3
  keeps identical vertical framing and trims the sides. V6 crops the outer
  pillars and V13 shows the floor edge; both are honest consequences of the
  authored size.
- **Foreshortening.** None in orthographic frames: equal heights, parallel rails,
  a uniform checker. Perspective converges as expected.
- **Clipping.** At distance 5, the near pillar partly crosses behind the 0.1 m
  near plane and is cut open. The floor behind the camera is also cut, leaving a
  hard band at the bottom. The pillar's shadow remains, which is correct because
  shadow casters are not clipped by the view near plane. This is expected
  orthographic behaviour. Authors must pull the camera back along its axis,
  since moving does not change size.
- **Lighting and shadows.** Lamp pools, specular response and sun direction are
  consistent across every orthographic distance and both aspects. Shadow
  penumbrae soften slightly at distance 34; see the known limitation below.
- **Resizing.** Both 960×540 and 800×600 are correct; see the height table above.

Committed frames are in `screenshots/engine/`:
- persp-d16 and ortho-d16-v9 at both aspects;
- ortho-d34-v9, ortho-d16-v6, ortho-d16-v13 and ortho-d5-v9 for clipping;
- live t35 at V6 and live t45 as perspective at distance 24.

## Inspector design

The Camera section reads, in schema order:

| Row | Value | Notes |
| --- | --- | --- |
| Projection | Perspective or Orthographic | |
| Vertical size | `m` | Indented beneath Projection when orthographic |
| Field of view | `°` | Tagged "Unused" while orthographic |
| Near clip | `m` | |
| Far clip | `m` | |

Units and help come from the schema. The profile adds only labels, variant
display names and near/far help, which the schema lacks.

- **Legacy cameras.** A camera without `projection` shows the schema default,
  "Perspective", with a hairline "Default" tag. Its accessible description reads
  "Not authored. Cameras without a projection use the schema default." Explicit
  perspective shows no tag. If the schema had no presentable default, the row
  would read "Not set" instead of guessing.
- **Malformed values.** These show the existing dashed notice with the raw JSON,
  as a focusable group, and never fall back to perspective:
  - unknown kind;
  - missing kind;
  - `null` or a non-object;
  - missing or non-numeric `vertical_size`;
  - extra fields.

  The "Unused" tag appears only for a well-formed orthographic projection. A
  missing, mistyped, `null`, zero or negative size, or any extra field,
  suppresses it, and the browser and unit tests check this. A non-positive size
  is still displayed as authored, and engine diagnostics mark it invalid.
- **Numbers.** Ordinary values keep up to four decimals, such as 0.1 and 1.5708.
  Nonzero magnitudes below 0.01 or at 1e9 and above use four significant digits
  in exponent form, such as 1e-6, -2.5e-6 and 4.2e-4. The exact authored value
  is the hover title, for example "0.000001". The same rule applies to every
  numeric field and vector axis in the Inspector.
- **Read-only.** Every value is a read-only input. The section has no projection
  selector, button or write path, and the existing "Read-only" chip is unchanged.
  No agent-ready claim is made.
- **Keyboard.** Tab order is Projection, Vertical size, Field of view, Near, Far,
  and each stop has the existing 2 px focus ring. Tags are `aria-hidden`, and
  their full sentences are in `aria-describedby`.
- **Measured in Chrome 155.** Results are in `evidence/inspector-report.json`.
  - Six states at 1440×900 (Inspector 331 px) and at the 280 px minimum
    (1000×650): legacy, orthographic, keyboard focus, unknown kind, sizeless
    orthographic and tiny values.
  - Zero horizontal overflow, no clipped labels or values, no page errors, no
    remote requests and zero axe violations in all 12 captures.
  - Contrast: labels 7.98:1, values 16.28:1, units 6.96:1, tags 8.62:1.

These are browser captures of the built UI in headless Chrome. They are not
native WebKit evidence. The malformed, sizeless and tiny states use a labelled
evidence test-double bridge that rejects all commands.

## Native review

I reviewed these states from the original CUA JPEGs at Retina 2x. Each committed
crop is named here.

| State | Size | Crop |
| --- | --- | --- |
| Legacy camera (`persp-d16`) | 1440×874, 331 px Inspector | `legacy-wide-inspector.jpg` |
| Legacy camera | 1000×650, 280 px Inspector | `legacy-min-crop.jpg` |
| Explicit perspective | 1440×874 | `explicit-wide-inspector.jpg` |
| Explicit perspective | 1000×650, 280 px | `explicit-min-crop.jpg` |
| Orthographic V9 | 1440×874 | `ortho-wide-inspector.jpg` |
| Orthographic V9 | 1000×650, 280 px | `ortho-min-crop.jpg` |
| Orthographic, Tab focus | 1000×650, 280 px | `ortho-min-keyboard-crop.jpg` |
| Tiny values | 1440×874 | `tiny-wide-inspector.jpg` |
| Tiny values | 1000×650, 331 px | `tiny-min-331-crop.jpg` |
| Tiny values | 1000×650, 280 px | `tiny-min-inspector-crop.jpg` |

Findings, all matching the browser design:
- **Order and labels.** Rows read Projection, Vertical size when orthographic,
  Field of view, Near clip, Far clip. Labels are sentence case, and units come
  from the schema: `°`, `m`, `m`, `m`.
- **Default versus authored.** The legacy camera shows "Perspective" with the
  hairline "Default" tag. Explicit perspective shows "Perspective" with no tag.
  The two states are visibly distinct at both widths.
- **Unused tag.** Orthographic and tiny-value cameras show "Unused" beside field
  of view. Both perspective cameras show none.
- **Tiny values.** Vertical size and near clip read `1e-6 m`, not `0`, and far
  clip reads `80`.
- **Minimum width.** At 280 px, "Vertical size" wraps to two lines, as designed.
  Every value, unit and tag stays inside its box, with no Camera-row overflow or
  clipping. At 331 px the label fits on one line.
- **Keyboard.** Clicking Projection and pressing Tab moves focus to Vertical
  size, which shows the 2 px focus ring around the whole value box. The value
  text is selected, which is normal for a read-only input.
- **Mutation boundary.** The "Read-only" chip is present. There are no projection
  controls and no new floating surfaces.

Capture artifacts, which are not defects:
- The pointer covers part of "Orthographic" in the keyboard frame.
- The 280 px tiny-values frame shows a text selection across the Inspector
  header and the first Camera rows. This most likely comes from the CUA resize
  drag, because no other frame shows it. If a manual splitter drag reproduces
  it, the splitter should suppress text selection while dragging. That is a
  shell issue, not a Camera one.
- The OS sharing pill covers the traffic lights, which is outside this scope.

Outside Camera scope, for a future Inspector pass: at 280 px, the shared
Transform vector rows truncate four-axis rotation to almost nothing, such as
`X -.` and `Y 0.`. At 331 px they show `X 7.9…`. The truncated text has no full
value on hover. This existed before this handoff and is not caused by the Camera
changes.

## Known limitation: shadow quality and orthographic distance

This was measured at the first checkpoint and no cascade change is in scope.
Moving an orthographic camera back along its forward axis at a fixed vertical
size leaves silhouettes, lit shading and point-light pools byte-identical.
Directional-shadow edges change slightly:

| Pair | Changed pixels | Max channel delta |
| --- | --- | --- |
| distance 16 vs 24, 16:9 | 1.7 % | 11 |
| distance 16 vs 34, 16:9 | 3.5 % | 43 |
| distance 16 vs 34, 4:3 | 4.2 % | 20 |

At distance 34, penumbrae are slightly softer and pillar faces show faint
banding at 3× zoom. Receivers fall into coarser cascades because cascade splits
follow view depth from the near plane. The image is still consistent and
readable. Authors who need stable shadow detail should keep orthographic
cameras near their subject.

## Requests for Astra

All Camera requests are resolved:
- `9dccfb1` supplied Camera units, the projection order, user-facing projection
  help and preserved defaults.
- `b769347` corrected the bridge default test fixture. All 368 UI tests on this
  branch pass after it.

No Camera UI change came out of the native review, so nothing needs
re-integration or recapture.

These remain open and non-blocking:
- **Shadow cascades and orthographic distance**, low severity. Fitting
  orthographic cascades to the receiver depth range would remove the
  limitation measured above.
- **Transform vectors at 280 px**, outside this scope. They need a full value on
  hover or a wider layout.

## Open items

- **Gate approval** remains human-owned. No phase gate is approved or claimed.
