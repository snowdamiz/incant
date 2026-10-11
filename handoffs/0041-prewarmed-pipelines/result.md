# 0041 result: appearance after pipeline preparation

## Status

**Appearance is preserved. I found no prewarming regression.** This covers the tested
path only: headless `screenshot` at 960×540 on one Apple M5 Pro, with `Rgba8UnormSrgb`
output. It makes no device, performance or phase-gate claim.

- **Identical builds.** All 11 baseline/candidate capture pairs are byte-identical: 7
  camera views in the initial state, plus the reimported, source-free (×2) and
  diagnostic-cube states. The deliberate refusal fails identically on both builds.
- **Expected appearance.** I inspected every view at full size and in enlarged crops.
  Each shows the intended behaviour: alpha modes, both culling choices, mirroring,
  depth/blend order, shadow shapes, point/spot light, tone, sprites, the reimported
  material and source-free rendering.
- **Two shared issues**, present in both builds and not caused by prewarming:
  - the known generated-tangent Y inversion (0040; a fix, PR77, is in progress);
  - OPAQUE textures whose alpha-0 texels darken toward black when minified.

  Details are under "Differences and classification".

## Priority revisions and director feedback

The packet has no priority-revision section, and no newer director feedback reached
this session. Astra's release text (`artifacts/tools/gpu-review-ready`) added three
instructions, all followed:

- I read the guard's source first, then sent every capture through
  `artifacts/tools/guarded-screenshot.py` (sha256 `a3f329a3…6eed`).
- I read `artifacts/tools/shared-tangent-baseline.md` before attributing any
  normal-map difference.
- At completion I removed the gate and stated that the slot was released.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`) with Max thinking:

- the system prompt names the model;
- the session environment reports `CLAUDE_EFFORT=max` and
  `CLAUDE_CODE_ENTRYPOINT=sdk-ts`.

I cannot observe the ACP handshake that set them.

## Sources, binaries and host

| Build | Source | sha256 | Bytes |
| --- | --- | --- | --- |
| Baseline (PR79) | `74238785fe92c50daea1d8dd056492359def61f2` | `53aa002cd11203beb893ca9c79c1aca348b871921be70348c7bebb0355a0e387` | 42,289,808 |
| Candidate | `21a2aa2fb9afcd5903453ca4c1cca32ebf72c6fb` | `064024f05be04d2f95ffef2bc8d3705bfa51f83881886cb7cee385f8f3d2accf` | 42,297,440 |

- **Binary checks.** Both binaries matched `artifacts/tools/binary.json` (hash, size and
  the packet's source commit) before and after each of my 80 executions. The guard
  re-checked the launched binary on all 26 captures.
- **Worktree.** HEAD is `8701dcd`. Its `crates/` and Cargo files are identical to
  `21a2aa2`. Nothing was rebuilt and Cargo was not run.
- **Host.** macOS 26.6 (25G72) on an Apple M5 Pro. The engine reported adapter
  "Apple M5 Pro" on every capture.
- **Backend.** Metal, inferred: wgpu's default instance on macOS. The CLI does not
  print the backend.

## What the production diff changes

This is from `git diff 74238785 21a2aa2`: 12 files, all in the render crate, and no WGSL
file changed.

- **Prepared at startup.** `Renderer::new` now builds these inside a validation error
  scope:
  - 12 material pipelines: OPAQUE/MASK/BLEND × single/double-sided × plain/mirrored,
    HDR target only;
  - 8 shadow-caster pipelines: OPAQUE/MASK only × culling × mirrored;
  - 4 output pipelines: RGBA/BGRA, linear/sRGB;
  - the diagnostic pipeline.

  Each pipeline family shares one shader module.
- **Drawing.** Drawing now looks up the prepared variant and fails with
  `UnpreparedPipeline` if it is missing. The pipeline descriptors are unchanged.
- **BLEND.** There is no BLEND shadow pipeline. BLEND casters are still refused earlier,
  with `TransparentShadowCaster`, in both `prepare_scene` and `prepare_shadow_draw`.

Identical pixels were therefore expected. The review looked for three kinds of
regression:

- a wrong variant lookup: culling, winding, blending, depth write or caster alpha;
- a missing variant;
- a changed refusal order.

## Fixtures

Four disposable projects were built only with public commands: `init`, `import`,
`validate`, and `rpc` using `project.read`, `command.execute`, `history.undo` and
`project.save`. No document or journal was written directly. All art and models are
original (`tools/fixtures.py`). Each view renders the same saved project and cache
with both binaries, in separate processes.

The candidate binary ran the CPU steps (`init`, `import`, `rpc`, `validate`). Their code
is identical in both builds, because the diff touches only the render crate. The
import refusal ran on both binaries.

| Project | What it covers | Views |
| --- | --- | --- |
| materials (35 entities, 12 assets) | 24 cards: 3 alpha modes × single/double-sided × plain/mirrored × facing the camera or away. One texture carries an F glyph, a domed normal map, an alpha-0 triangle and stripes at alpha 0.2/0.4/0.6/0.8. A striped opaque backdrop, an opaque bar and a BLEND glass strip test depth and blend order. 8 emissive swatches test tone. 6 large normal-map cards: generated tangents, explicit `TANGENT` w=+1 and w=−1, mirrored, and a double-sided back. | `board`, `normals` |
| shadows (22 entities, plus a camera added later) | 16 horizontal casters: OPAQUE/MASK × culling × mirrored × facing toward or away from a shadowed sun (from the left, 40° up). 2 BLEND non-casters. | `board` (oblique), `top` (orthographic) |
| lights (10 entities, plus a diagnostic cube added later) | Warm and cool point lights. A white spot that drives the floor into the HDR shoulder. A narrow magenta spot on the wall. White and metal spheres, a red cube, and a dim uniform environment. | `board` |
| sprites (48 entities, 3 assets) | Sheet and atlas glyphs: plain, flip X/Y/XY, rot90, reflection (−1 parent), tint and translucency. 5 nine-slices: sheet, atlas, reflected, flipped and compressed. Translucent orbs. Sprites behind and in front of an emissive model quad. | `pixel` (pixel-perfect 320×180 ×3), `perspective` |

**Reused helpers.**

- `handoffs/0032-sprite-rendering/tools/pngio.py` (sha256 `5b73bef8…9e86`), imported
  unmodified.
- The glTF writer and closed box, adapted from
  `handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py`.
- The fixed-prefix ULID scheme from 0030 and 0032.

**Later edits.** All of these went through `command.execute` and `project.save`:

- the top-down shadow camera;
- one Transform-only diagnostic cube, which exercises the diagnostic pipeline (also
  moved to startup);
- the refusal toggle, reverted with `history.undo`.

## Numeric comparison

This section records measurements only. The verdict follows in the next section.

| State / view | PNG sha256 (baseline = candidate) |
| --- | --- |
| initial / materials-board | `6b7ac74453193343e7364c449bc92c06654af493e9abdce10837b432dfc824a2` |
| initial / materials-normals | `7f2fdf749d433149555205d4a79537eadc6627899bad6bced956a6d89037f21a` |
| initial / shadows-board | `5c3de55545dd69561a4e57ac132a255208bbc3862b5ada42b714fc7e9ffc1283` |
| initial / shadows-top | `90ba7da77c11316845c03eea0b8defcf49e081570c3031d97f8530b87eacdf45` |
| initial / lights-board | `066c885bf0c93debb382216a0092ec4731d089db0f7592712d4c3eab07a8980a` |
| initial / sprites-pixel | `d84303ec45baddca3c7b1a043d47e257ae266e5b85c46f434b76a7f6982fb65b` |
| initial / sprites-perspective | `4a7ac04f0db869e7fb8feaacb78be4a187156735811e194fb65607bd8d767c50` |
| reimport / materials-board | `5d7b08542c86b0a2c36d768ec3ffcc2408775992dd20052e7805c29bf6a10c1a` |
| source-free / materials-board | `5d7b08542c86b0a2c36d768ec3ffcc2408775992dd20052e7805c29bf6a10c1a` |
| source-free / sprites-pixel | `d84303ec45baddca3c7b1a043d47e257ae266e5b85c46f434b76a7f6982fb65b` |
| diagnostic / lights-board | `7a9970dfefdfcfff83272616a833338c184bd3f69d01c6d29f7c5d6c88adc468` |

**Pixel comparisons.** The figures below come from `tools/analyse.py` and are recorded in
`evidence/numeric.json`.

- **Decoded pixels.** All 11 pairs have 0 differing pixels.
- **Fresh-process repeat.** materials-board, captured again with the order reversed
  (candidate first), is identical for both binaries.
- **Reimport against initial (materials-board).** 31,827 pixels change, with a maximum
  channel delta of 134. All of them lie inside the four OPAQUE-single card rectangles;
  0 lie outside.
- **Source-free against before deletion.** materials-board equals the reimport frame;
  sprites-pixel equals the initial frame.
- **Diagnostic cube against initial (lights-board).** 10,811 pixels change, all inside
  the cube's projected box.

**Point samples.** These sample analytically projected points, using the saved project's
own transforms and cameras:

- Material cards: 24/24 match the expected culling and alpha class initially, and 24/24
  after the reimport.
- Shadows (top view): 18/18 casters and non-casters match the expected shadowed or lit
  floor samples, including the MASK holes. Shadowed floor reads 0.112 linear luminance
  (environment only); lit floor reads 0.445.
- Nine-slice corner colours: 5/5 match.
- Lights:

  | Point | sRGB |
  | --- | --- |
  | White spot pool on the floor | 251/251/252 |
  | Top of the red cube under the spot | 254/164/160 |
  | Magenta pool on the wall | 172/112/162 |
  | Floor outside every light's range | 39 (predicted 39.6 from the 0.039 environment) |

**Tone ramp.** Measured sRGB against the engine's tone curve applied to the emissive
value alone:

| Swatch (emissive, linear) | Measured | Curve, emissive only |
| --- | --- | --- |
| white 0.05 | 70 | 63.2 |
| white 0.2 | 127 | 123.6 |
| white 0.5 | 189 | 187.5 |
| white 0.8 | 232 | 231.1 |
| white 1.0 | 244 | 243.4 |
| red 1.0 | 244 / 42 / 42 | 243.4 / 30.4 / 30.4 |
| green 1.0 | 43 / 244 / 43 | 30.4 / 243.4 / 30.4 |
| blue 1.0 | 43 / 43 / 244 | 30.4 / 30.4 / 243.4 |

The residual is a constant of about 0.01 linear. That is the uniform environment's
specular reflection off the black dielectric swatches.

**Normal-map relief.** The angle is the screen direction of the dome's bright side (0° is
right, 90° is up). The key light sits at 136° on screen. Luminance is sampled 0.55
dome radii from the centre along the image axes.

| Card (normals view) | Bright side | Luminance up / down / left / right |
| --- | --- | --- |
| generated tangents | 227°, lower-left | 0.165 / 0.668 / 0.658 / 0.190 |
| explicit `TANGENT` w=+1 | 133°, upper-left | 0.661 / 0.164 / 0.642 / 0.174 |
| explicit `TANGENT` w=−1 | 226°, lower-left | 0.164 / 0.657 / 0.668 / 0.190 |
| generated, mirrored | 228°, lower-left | 0.167 / 0.679 / 0.191 / 0.658 |
| generated, double-sided back | 48°, upper-right | 0.676 / 0.168 / 0.635 / 0.173 |
| w=+1, mirrored | 133°, upper-left | 0.662 / 0.165 / 0.173 / 0.636 |

On the board, all 12 visible generated-tangent cards agree regardless of alpha mode or
mirroring:

- front faces: 225–227°;
- double-sided back faces: 47–48°.

## Appearance verdict

I judged these from the engine pixels themselves, at full size and in enlarged crops,
not from the hashes.

- **Alpha modes, culling, mirroring.**
  - OPAQUE fronts show the teal triangle and four solid red stripes, so alpha is
    ignored as glTF requires.
  - MASK cuts the triangle and the 0.2/0.4 stripes to the backdrop and keeps
    0.6/0.8.
  - Single-sided back-facing cards are absent in every row. Double-sided back cards
    are present and lit like the fronts, with the glyph mirrored as seen from behind.
  - A mirrored front reads ꟻ and is still drawn: winding is reversed correctly in the
    Cw variant. A mirrored double-sided back card reads F (two reflections).
  - Every variant agrees with every other on UVs, culling and shading.
- **Normal-map orientation.** The X axis follows the map, and mirrored placements keep
  the bright side toward the light. The Y axis is inverted for generated tangents. The
  explicit w=−1 card matches the generated one exactly, while explicit w=+1 reads as a
  correct convex dome, both plain and mirrored.

  The double-sided back reverses the normal as glTF requires (a dent in X), and again
  shows the Y inversion. This is the shared tangent defect, not a prewarming effect;
  see below.
- **Silhouette, depth and blending.** Card edges are complete and straight, the spheres
  are round and the cube's edges are clean. There are no missing triangles, no
  z-fighting and no holes beyond the authored ones.

  BLEND cards are composited over the opaque backdrop, under the BLEND glass (sorted
  back to front, so the paper reads blue through the glass) and behind the opaque bar,
  which also hides the glass where the two overlap. The slivers below the bar show
  graded stripes and a see-through triangle. The shadows board's BLEND non-casters show
  the full alpha grading.
- **Shadow shape**, in the oblique and top views:
  - OPAQUE casters give solid squares.
  - MASK casters give the triangle and two stripe holes in the matching positions,
    mirrored where the card is mirrored.
  - Single-sided casters facing away from the sun are invisible and cast nothing.
  - Double-sided casters facing away are drawn and cast shadows.
  - BLEND non-casters cast nothing.
  - Offset and direction match a sun from the left at 40°. Edges look slightly
    serrated in the oblique view on both builds.
- **Point/spot light, tone and colour.**
  - The warm and cool point lights colour opposite sides of the white sphere, and the
    metal sphere shows two tinted highlights.
  - The white spot's pool reaches the shoulder without banding.
  - The cube top under the spot desaturates toward pink-white but keeps a red hue.
  - The magenta ellipse on the wall is soft-edged, and the dim surround stays
    readable.
  - The emissive ramp is monotonic. Saturated R/G/B at 1.0 keep their hue, matching
    the documented curve ("red [1,0,0] shows near sRGB [243, 30, 30]") plus the
    measured reflection.
- **Sprites.**
  - Flips, rot90 and the −1 reflection orient the sheet F and the atlas R correctly.
  - Tint and 50% translucency over the stripes are correct.
  - Nine-slice corners stay unstretched and in the right corners in all four
    orientations; the compressed 8-pixel panel shrinks its borders proportionally.
  - The orbs stack in painter order.
  - The emissive model hides the sprite behind it and is overdrawn by the sprites in
    front of it. The same depth relationship holds in the perspective view.
- **Reimported material** (OPAQUE single changed to a tinted, double-sided MASK with
  cutoff 0.3; same asset ID):
  - the paper shifts toward blue-green, from 207/191/161 to 170/176/161;
  - back cards appear;
  - the triangle and the 0.2 stripe become holes while the 0.4 stripe stays;
  - nothing else on the board changes.

  Baseline and candidate agree.
- **Source-free.** After all sources were deleted (26 materials files, 6 sprites files),
  fresh processes reproduce the pre-deletion frames exactly.
- **Diagnostic pipeline.** The Transform-only cube draws with the Phase 0 grey shading
  and occludes the sphere's base and the floor correctly. Its appearance belongs to the
  diagnostic shader and is out of scope.
- **Refusals.** Preserved on both builds; see "Refusals".

## Differences and classification

| Observation | Builds | Classification |
| --- | --- | --- |
| Any baseline/candidate pixel difference | none | — |
| Generated-tangent Y inversion (glTF without `TANGENT`) | both | existing shared defect (0040; PR77 fix pending) |
| OPAQUE alpha-0 texels darken with minification | both | existing shared cook behaviour; informational |
| Slightly serrated shadow edges in the oblique view | both | existing shared filtering limit; not targeted by this diff |
| BLEND material with `cast_shadows` under a shadowed light | both | unsupported; refused by design |
| `KHR_materials_emissive_strength` | both | unsupported; refused at import by design |

## Defects and minimal repros

**No new defect.** Two existing, shared issues were observed.

1. **Generated tangents invert normal-map Y.** This is already tracked by Astra (0040,
   PR77).
   - **Repro.** Take a quad facing +Z with glTF UVs (top-left origin) and a dome normal
     map, and no `TANGENT`; light it from the upper-left. The dome reads lit from below.
   - **Controls.** The same quad with `TANGENT` (1,0,0,+1) reads correctly. With
     (1,0,0,−1) it matches the generated result.
   - **Fixtures.** `nrm_generated`, `nrm_tangent_pos` and `nrm_tangent_neg` in the
     materials project, with the numbers in the relief table above.

   Existing cooked versions keep the old tangents until they are reimported (per
   Astra's note).
2. **OPAQUE colour textures lose the colour of alpha-0 texels in their mips.** The cook
   averages premultiplied colour, so fully transparent texels become black from mip 1
   onward. OPAQUE ignores alpha, so those texels darken as they shrink on screen.

   | Placement | Approx. LOD | Triangle sRGB |
   | --- | --- | --- |
   | Magnified (materials board, 1.43 px/texel) | 0 | 63/158/150 |
   | Top view | ~0.47 | 49/121/114 |
   | Oblique board, near row | ~0.43–0.58 | 40–43 / 103–111 / 97–105 |
   | Oblique board, far row | ~0.83–0.91 | 33–40 / 33–49 / 32–46 |

   In the top view, upward- and downward-facing cards at equal LOD match within one
   level, so facing is not the cause.

   **Inference, untested here.** An OPAQUE asset that ships an RGBA texture with an
   unused all-zero alpha channel would render correctly up close and go black at a
   distance.

   **Minimal repro.** An OPAQUE glTF quad whose texture has teal RGB and alpha 0,
   viewed at 1:1 and then minified 2–4×.

   This is outside this packet. The cook decision belongs to Astra.

## Refusals

- **Unsupported material extension.** Importing `KHR_materials_emissive_strength`
  exits 1 on both builds with "unsupported asset feature: glTF extension
  KHR_materials_emissive_strength". The project and journal bytes are unchanged.
- **BLEND shadow caster.** I set `cast_shadows: true` on a BLEND card through
  `command.execute` and saved it. Both builds then exit 1 at `screenshot` with "BLEND
  materials cannot cast directional shadows; disable cast_shadows or use MASK" and
  write no PNG. The candidate does not surface the new `UnpreparedPipeline` error in
  its place. `history.undo` plus `project.save` restored the project file byte for
  byte.

## GPU slot

| Item | Value |
| --- | --- |
| Captures | 26 of 40, all through the guard (sequence 1–26), all 960×540 |
| Expected refusals | 2 of the 26 |
| Capture window (UTC) | 05:01:40–05:04:56 on 2026-10-11 |
| Per-capture wall time | at most 0.57 s; no timeouts |
| Children | all reaped |

The gate was removed at 05:05:15Z, after `pgrep` found no running process from either
frozen binary or the guard. No capture was launched after that. Wall times are review
bookkeeping, not performance measurements.

## Commands run (worktree root)

```
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py plan             # analytic framing check
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py build            # CPU: 4 projects, 44 executions
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py refuse-import    # CPU: both binaries
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture initial materials/board
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture initial materials/normals shadows/board lights/board sprites/pixel sprites/perspective
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture initial materials/board --order candidate-first
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py refuse-shadow
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py reimport
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture reimport materials/board
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py strip materials
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py strip sprites
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture sourcefree materials/board sprites/pixel
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py diagnostic
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture diagnostic lights/board
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py top-camera
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture initial shadows/top
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py finish           # remove gate, verify no renderer process
python3 -I handoffs/0041-prewarmed-pipelines/tools/analyse.py                  # CPU numeric evidence
python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py package          # copy unedited captures and evidence
```

Every step succeeded. The four non-zero exits in the ledger (sequences 45, 46, 63 and
64) are the expected refusals.

**Not run.** The Rust GPU regression (`pipeline_warmup`, all four presentation formats),
Cargo, fmt and clippy were not run. Numerical GPU checks are Astra's.

## Screenshots (unedited engine captures; each is byte-identical to its baseline twin)

- `handoffs/0041-prewarmed-pipelines/screenshots/materials-board.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/materials-normals.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/materials-board-reimported.png` (the
  source-free frame is identical)
- `handoffs/0041-prewarmed-pipelines/screenshots/shadows-board.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/shadows-top.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/lights-board.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/lights-board-diagnostic-cube.png`
- `handoffs/0041-prewarmed-pipelines/screenshots/sprites-pixel.png` (the source-free
  frame is identical)
- `handoffs/0041-prewarmed-pipelines/screenshots/sprites-perspective.png`

Hashes are in `evidence/screenshots.sha256`.

## Changed paths

**Committed.** Everything is new and under `handoffs/0041-prewarmed-pipelines/`;
`brief.md` is unchanged.

| Path | Contents |
| --- | --- |
| `result.md` | this report |
| `tools/fixtures.py` | original art, glTF sources, scene specs, framing check |
| `tools/review.py` | guarded runner and ledger |
| `tools/analyse.py` | CPU numeric evidence |
| `evidence/numeric.json` | measurements |
| `evidence/ledger.jsonl` | command, hash, exit, adapter and dimensions per execution, with repo-relative paths |
| `evidence/fixtures.json` | asset/entity/camera IDs, source and cache hashes |
| `evidence/screenshots.sha256` | screenshot hashes |
| `evidence/gpu-slot-released.json` | slot release record |
| `evidence/strip-materials.json`, `evidence/strip-sprites.json` | deleted sources with hashes |
| `screenshots/*.png` | the nine captures listed above |

**Ignored and not committed.** `artifacts/0041-prewarmed-pipelines/` holds the fixture
projects and caches, the 24 raw PNGs, the full ledger and build record, and scratch
texture and crop previews.

**Outside my two paths:**

- I removed `artifacts/tools/gpu-review-ready`, as Astra's release text instructs.
- The guard itself wrote `artifacts/tools/capture-count` (26) and
  `artifacts/tools/capture-ledger.jsonl` (26 lines).
- Importing the reused `pngio` briefly created an ignored bytecode cache in
  `handoffs/0032-sprite-rendering/tools/__pycache__`. I removed it, and the tools now
  disable bytecode writing.

No production, shader, renderer, script, schema, UI or lockfile change.

## Limitations

- **Host.** One host and adapter (Apple M5 Pro, macOS 26.6, Metal inferred). There is
  no phone, performance or reference-device result, and no gate is claimed.
- **Output format.** CLI captures cover only `Rgba8UnormSrgb` output. The other three
  presentation formats are covered by the Rust GPU regression, which I did not run.
- **Fresh processes.** Every screenshot runs in a fresh process with a new renderer. The
  CLI therefore cannot show first draws against later draws inside one renderer, or
  in-process retained reimport, which are what prewarming targets.
  `pipeline_warmup` covers pipeline allocation counts there.
- **BLEND alpha.** On the materials board the deliberate opaque bar hides most of the
  BLEND cards' lower alpha features. Full BLEND alpha was judged on the shadows board's
  non-mirrored non-casters. The mirrored BLEND variants were judged from the slivers
  and the glyph/dome area.
- **Normal maps.** The normal-map verdict uses glTF's +Y-up convention. Because of the
  shared tangent defect, I cannot approve generated-tangent normal maps in either build.
- **Shadows and lights.**
  - Shadows were reviewed at one sun angle and one shadow distance; cascade transitions
    were not targeted, since the diff does not touch cascades.
  - Point and spot lights cast no shadows in this engine.

## Open questions

1. Is the OPAQUE alpha-0 mip darkening intended? Should colour textures used only by
   OPAQUE materials average RGB without alpha weighting? This is a cook decision for
   Astra.
2. Once PR77 lands, should the `normals` view be re-run with these fixtures for the
   normal-map delta? Its explicit w=±1 controls are ready. The cached cards need to be
   reimported first.
