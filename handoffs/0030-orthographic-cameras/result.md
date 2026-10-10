# 0030 orthographic cameras: appearance and Inspector result

## Status

Correctness revision complete: **engine and browser scopes accepted; native
scope still waiting.**

- **Correctness fixes: done.** Tiny nonzero numbers no longer display as zero.
  The field-of-view "Unused" tag now appears only for a well-formed
  orthographic projection. Both have new regression tests.
- **Schema defaults: consumed.** The Inspector reads the projection default from
  Astra's preserved schema `default` and no longer hardcodes it. Units come from
  the schema, and projection is listed first as the schema now orders it.
- **Engine appearance: unchanged pass.** No rendering code changed, so I did not
  repeat the real-render captures. The binary SHA still matches `binary.json`.
- **Camera Inspector (browser): pass.** Strict typecheck and build pass. All 12
  new browser captures at both widths have zero axe violations and no overflow.
- **One failing test, not mine.** Astra's new bridge test fails because of a
  fixture or bridge mismatch outside my allowed paths. Details are under
  "Requests for Astra".
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

The 2026-10-10 correctness review accepted checkpoint `d9f8769` for its engine
and browser scope and asked for three changes. I applied all three before
anything else:
1. **Small numbers.** `formatNumber` rounded 0.000001 to "0". It now keeps every
   nonzero value visible, including negative small vector components.
2. **Strict orthographic check.** The "Unused" tag was derived from `kind`
   alone. It now requires exactly `kind` plus a finite, positive numeric
   `vertical_size`, which matches what my earlier result claimed.
3. **Schema metadata.** The fixture, order tests and presentation now use
   Astra's schema units, order, help text and preserved `default`. The "Default"
   tag still distinguishes implied values from authored ones.

The standing preferences still apply: neutral connected panels, clean spacing,
a restrained accent and no floating cards. Assets stays in the left workspace,
and the bottom dock stays Problems/Console/History.

## Provenance

| Item | Value |
| --- | --- |
| Engine captures source commit | `298b846` |
| This revision's base | `9dccfb1` (Astra's schema and bridge changes) |
| Binary | `artifacts/tools/incant_headless`, supplied by Astra |
| `binary.json` sha256 | `ce79c966d5d4264482829904c0fb9158f244b61fb7ae1dac1c8150c521a4d2ff` |
| Measured again for this revision | same value |

The engine helpers check the SHA before and after every engine invocation, and
both earlier runs report `binary_unchanged_every_call: true`. I used no Cargo and
no other worktree's build.

## Changed paths

This revision, all under `editor/ui/src`:
- `components/inspector/FieldView.tsx`:
  - `formatNumber` keeps up to four decimals for ordinary values. It switches to
    four significant digits in exponent form below 0.01 or at 1e9 and above. An
    input whose display differs from the authored number gets the exact value
    as a hover title.
  - `presentableDefault` shows an omitted optional field as its schema
    `default` only when that default is presentable. That means a finite number
    for a number field, or a record naming a known variant for a tagged union.
    Otherwise the field stays "Not set", with no guessing or repair.
  - The profile unit fallback is removed; units now come only from the schema.
- `components/inspector/presentation.ts`:
  - `isOrthographicCamera` replaces `projectionKind` and is strict.
  - `FieldHint` loses `unit` and `whenOmitted` and gains `omittedNote`, which is
    wording only.
  - The Camera profile keeps labels, variant names and near/far help, in the new
    schema order.
- `bridge/fixture.ts`: the Camera mirror gains units, the `projection`-first
  order, the new description and the `default`.
- `components/inspector/CameraFields.test.tsx`: updated order and default
  expectations, plus 36 new cases covering (52 in the file):
  - orthographic detection, 12 cases;
  - number formatting, 16 cases including a small vector;
  - small Camera clip planes and vertical size;
  - schema-default handling;
  - the Default-versus-authored distinction;
  - no "Unused" tag for malformed orthographic values.

Handoff:
- `tools/capture-inspector.mjs`: the test double is now built from the current
  schema, with two new states, "sizeless-ortho" and "tiny-values".
- `screenshots/inspector/*.png`: replaced with 8 crops from set `r2`.
- `evidence/inspector-report.json` and `evidence/screenshots.sha256`: updated.
- `result.md`.

Unchanged from the first checkpoint:
- the engine helpers;
- the 10 engine frames in `screenshots/engine/`;
- the engine evidence files.

Ignored, not committed: `artifacts/0030-orthographic-cameras/inspector/r2/` with
24 PNGs, plus the earlier engine runs, analyses and review sheets.

No Rust, SDK, bridge, mutation routing, CI or account settings were changed.

## Commands and results

This revision:
```sh
shasum -a 256 artifacts/tools/incant_headless        # matches binary.json
cd editor/ui && npx tsc -b --noEmit                  # strict: 0 errors
npx vitest run                                       # 15 files: 367 passed, 1 failed (bridge test, see below)
npm run build                                        # built
node handoffs/0030-orthographic-cameras/tools/capture-inspector.mjs r2   # 12 captures
```

The single failure is `editor/bridge/native.test.ts`, "retains property
defaults across local references and unions without authoring them":
```
expected { type: 'tagged-union', default: { kind: 'perspective' }, optional: true }
received ... optional: false
```

This revision does not change `native.ts` or `native.test.ts`, and that test
imports only `snapshotFromEngine`. The failure is therefore independent of my
changes.

The first checkpoint ran the engine commands below. They are not repeated
because the binary and rendering did not change.
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

**Resolved by `9dccfb1` and consumed in this revision:** Camera units, the
projection order, user-facing projection help, and preserved schema defaults.

**New: the bridge default test fails.** In `editor/bridge/native.test.ts`, the
test's `Camera` schema has no `required` array. `snapshotFromEngine` treats a
missing `required` as "every property required", so `projection` comes back
with `optional: false` and the test expects `true`. Choose one fix:
- add `required: []` to the test schema; or
- treat a missing `required` as "none required", which matches JSON Schema
  semantics. Real generated schemas always carry `required`, so this changes no
  shipped behaviour.

**Still open, low severity:** fitting orthographic shadow cascades to the
receiver depth range. Measurements are in the limitation above.

## Open items

- **Native captures.** Astra's CUA captures are needed for native acceptance.
  Account-bearing full images should stay ignored, with account-free crops
  committed.
- **Gate approval** remains human-owned.
