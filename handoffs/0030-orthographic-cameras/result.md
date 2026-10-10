# 0030 orthographic cameras: appearance and Inspector result

## Status

Initial checkpoint: **engine and browser scopes accepted; native scope waiting.**

- **Engine appearance: pass.** The orthographic projection shows equal-size
  objects across depth, parallel lines and parallel-view shading in real
  `incant_headless` frames. The checks cover 16:9 and 4:3, moving the camera
  along its forward axis, changing `vertical_size`, and switching projection
  live. One minor shadow observation is listed below for Astra.
- **Camera Inspector (browser): pass.** The Inspector shows projection type and
  vertical size, legacy cameras as default perspective, and malformed values
  explicitly. Normal and minimum widths, keyboard order, axe, strict typecheck,
  tests and build all pass.
- **Native editor: not reviewed.** Astra will supply native CUA captures after
  integration and rebuild. I make no native acceptance claim until I see them.
  I did not use computer use in this session.

No phase gate is approved or claimed. Pixel snapping, sprites, tilemaps and 2D
physics are not part of this increment and are not claimed.

## Model and transport

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner started this session in the handoff worktree through the Claude
  agent harness. I cannot independently inspect the ACP transport from inside
  the session. No model substitution occurred.

## Director feedback acknowledged

This is the first attempt at this packet; no earlier result or artifacts existed.
I applied the stated preferences:
- neutral connected panels, clean spacing and a restrained accent;
- no new floating-card redesign;
- Assets stays in the left workspace, and the bottom dock stays
  Problems/Console/History. The captures confirm this.

The Inspector change adds no new panel, card or colour. It uses the existing row
grid, value wells and one hairline tag style.

## Provenance

| Item | Value |
| --- | --- |
| Source commit | `298b846` (branch `handoff/0030-orthographic-cameras`) |
| Binary | `artifacts/tools/incant_headless`, supplied by Astra |
| `binary.json` sha256 | `ce79c966d5d4264482829904c0fb9158f244b61fb7ae1dac1c8150c521a4d2ff` |
| Measured before, after every call, and at end | same value |

Both helpers check the SHA before and after every engine invocation, and both
runs report `binary_unchanged_every_call: true`. I used no Cargo and no other
worktree's build.

## Changed paths

Inspector, all under `editor/ui/src`:
- `components/inspector/presentation.ts`: adds a Camera profile. It supplies
  labels, fallback units, help text and variant display names. It also declares
  the documented default for an omitted projection and a status that marks field
  of view unused while orthographic. `FieldHint` gains optional `unit`,
  `variants`, `whenOmitted` and `status`; schema metadata still wins.
- `components/inspector/FieldView.tsx`: implements the fallback unit, the
  variant display name, the "Default" and "Unused" tags, and the implied default
  for an omitted union. A variant with no fields no longer renders an empty
  group.
- `components/inspector/InspectorPanel.tsx`: passes the component value to field
  context so sibling-dependent tags can be computed.
- `styles/app.css`: adds the `.control--choice` value box and the `.control__tag`
  hairline tag.
- `bridge/fixture.ts`: replaces the stale `incant.Camera { fov }` sample schema
  with an exact mirror of the native `Camera` schema. The legacy "Camera Rig"
  stays and an orthographic "Map Camera" is added.
- `components/inspector/CameraFields.test.tsx`: 16 new behaviour tests.
- `App.test.tsx`: the sample tree count goes from 19 to 20 for the added camera.

Handoff, all under `handoffs/0030-orthographic-cameras`:
- `tools/ortho_lookdev.py`: builds the engine scene and captures it.
- `tools/analyse.py`: stdlib PNG measurement.
- `tools/capture-inspector.mjs`: browser captures.
- `screenshots/engine/*.png`: 10 unedited engine frames.
- `screenshots/inspector/*.png`: 6 browser captures.
- `evidence/`: helper summaries, scene description, measurements, Inspector
  report, live script log, the generated live-camera behaviour, and
  `screenshots.sha256`. The generated behaviour has its absolute SDK import path
  replaced by `<repo>` and is otherwise byte-identical.
- `result.md`.

Ignored, not committed, under `artifacts/0030-orthographic-cameras/`:
- `engine-r1/` and `engine-r2/`: full projects, 56 screenshots and 44 live
  frames.
- `analysis-r1.json`, `analysis-r2.json`, and `inspector/r1/` with 16 PNGs.
- `review-r1/`: derived diff masks and zoom sheets, plus their two one-off
  scripts.

No Rust, SDK, bridge, mutation routing, CI or account settings were changed.

## Commands and results

```sh
shasum -a 256 artifacts/tools/incant_headless        # matches binary.json
npm ci                                               # pinned deps
cd editor/ui && npx tsc -b --noEmit                  # strict: 0 errors
npx vitest run                                       # 15 files, 331 tests passed
npm run build                                        # built
python3 -m unittest discover -s tools/tests          # 5 tests OK
node handoffs/0030-orthographic-cameras/tools/capture-inspector.mjs r1
python3 -I handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py artifacts/0030-orthographic-cameras/engine-r1
python3 -I handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py artifacts/0030-orthographic-cameras/engine-r2
python3 -I handoffs/0030-orthographic-cameras/tools/analyse.py artifacts/0030-orthographic-cameras/engine-r2 artifacts/0030-orthographic-cameras/analysis-r2.json
```

Each helper run reports:
```
binary_unchanged: true, authored_unchanged: true,
screenshot_repeats_equal: true, play_repeats_equal: {16x9: true, 4x3: true}
```

Not run: Cargo, GPU unit tests, the native editor and native WebKit. Those are
Astra's.

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
  penumbrae soften slightly at distance 34.
- **Resizing.** Both 960×540 and 800×600 are correct; see the height table above.

Committed frames are in `screenshots/engine/`:
- persp-d16 and ortho-d16-v9 at both aspects;
- ortho-d34-v9, ortho-d16-v6, ortho-d16-v13 and ortho-d5-v9 for clipping;
- live t35 at V6 and live t45 as perspective at distance 24.

## Inspector design

The Camera section reads, in schema order:

| Row | Value | Notes |
| --- | --- | --- |
| Field of view | `°` | Tagged "Unused" while orthographic |
| Near clip | `m` | |
| Far clip | `m` | |
| Projection | Perspective or Orthographic | |
| Vertical size | `m` | Indented beneath Projection when orthographic |

- **Legacy cameras.** A camera without `projection` shows "Perspective" with a
  hairline "Default" tag. Its accessible description reads "Not authored.
  Cameras without a projection use perspective." Explicit perspective shows no
  tag, so authored and implied values stay distinguishable.
- **Malformed values.** These show the existing dashed notice with the raw JSON,
  as a focusable group, and never fall back to perspective:
  - unknown kind;
  - missing kind;
  - `null` or a non-object;
  - missing or non-numeric `vertical_size`;
  - extra fields.

  The "Unused" tag is suppressed when the projection is malformed. Non-positive
  sizes are shown as authored, and engine diagnostics mark them invalid.
- **Read-only.** Every value is a read-only input. The section has no projection
  selector, button or write path, and the existing "Read-only" chip is unchanged.
  No agent-ready claim is made.
- **Keyboard.** Tab order is Field of view, Near, Far, Projection, Vertical size,
  and each stop has the existing 2 px focus ring. Tags are `aria-hidden`, and
  their full sentences are in `aria-describedby`.
- **Measured in Chrome.** Results are in `evidence/inspector-report.json`.
  - Inspector widths of 331 px at 1440×900 and the 280 px minimum at 1000×650.
  - Zero horizontal overflow, no clipped labels or values, and zero axe
    violations in all 8 captures.
  - Contrast: labels 7.98:1, values 16.28:1, units 6.96:1, tags 8.62:1.
  - "Vertical size" wraps to two lines at 280 px, the designed label behaviour.
    The malformed notice is tall at 280 px but fully readable.

These are browser captures of the built UI in headless Chrome. They are not
native WebKit evidence. The malformed state uses a labelled evidence test-double
bridge that rejects all commands.

## Requests for Astra

These are concrete bridge and engine corrections. None blocks this checkpoint.

1. **Camera schema metadata.** `schemas/Camera.schema.json` has no
   `x-incant-unit` on `fov_degrees`, `near`, `far` or `vertical_size`, and no
   titles. The Inspector supplies fallback `°` and `m` units that the schema
   would override. Please add the units at the source.
2. **Projection default.** The bridge resolver drops a property's `default` when
   it resolves a `$ref`, so the Camera projection default never reaches the UI.
   The Inspector currently declares the documented perspective default in its
   presentation profile. Forwarding `default` would let it be derived from the
   schema.
3. **Schema order.** `order` lists only `fov_degrees`, `near` and `far`, so
   `projection` is placed last by the alphabetical fallback. Please list it
   explicitly; placing it first would read better.
4. **Developer-facing description.** The projection description, "Missing in
   legacy documents; those retain perspective projection.", wins over the
   Inspector's user-facing help. A user-facing sentence in the schema would
   improve the tooltip.
5. **Shadow cascades and orthographic distance.** This is low severity. Moving an
   orthographic camera from distance 16 to 34 changes 3.5 % of pixels, with a
   maximum delta of 43, all on shadow edges. Receivers move into coarser
   cascades because splits follow view depth from the near plane. Fitting
   orthographic cascades to the actual receiver depth range would make shadow
   quality independent of dolly distance. Otherwise, document it.

## Open items

- **Native captures.** Astra's CUA captures are needed for native acceptance.
  Account-bearing full images should stay ignored, with account-free crops
  committed.
- **Gate approval** remains human-owned.
