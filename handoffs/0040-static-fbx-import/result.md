# 0040 static FBX appearance and import affordance review: result

## Status

**Scoped verdict: accepted for static geometry, hierarchy, materials, colour maps, reimport
and the import-form affordance. Not accepted for FBX normal maps:** every FBX normal map
renders with its tangent-space Y (green) axis inverted. Raised relief reads as pressed in,
and the apparent light direction changes with object orientation. The cause is the
shared tangent generation, not the FBX parser or the shader. Details and a minimal repro
are under "Defect 1".

What holds, on the frozen binary, Apple M5 Pro, Metal, headless:

- **Geometry, units, axes, hierarchy and reflection.** I compared real Maya ASCII, Blender
  binary and 3ds Max ASCII sources against independently authored glTF twins with the
  same entity IDs, transforms and cameras. 8 of 9 views are byte-identical, and the ninth
  differs by one edge-tie pixel. The views cover centimetre/inch/metre units, Z-up and
  Y-up axes, geometric transforms, shared instances, a ZYX-ordered nonuniform parent, and
  a mirrored parent with a sheared child.
- **Material partitions.** The seven Suzanne partitions sit exactly where the source data
  puts them.
- **Colour maps.** Embedded and project-relative colour maps match each other and a glTF
  UV control pixel for pixel. Mirrored, nonuniform and rotated placements are exact
  against Maya-frame twins (0 differing pixels in the colour rows).
- **Lambert constants and maps.** Diffuse × factor, emissive map, constant emissive and
  50% achromatic opacity match hand-authored glTF values pixel for pixel.
- **Reimport workflow.** A stable reimport is a no-op cache hit. An accepted change and a
  project-relative dependency change both keep the asset IDs. Undo/Redo, two rejected
  reimports and a source-free fresh process all reproduce the expected frames byte for
  byte, and project/journal bytes stay unchanged on every rejection.
- **Import form.** All 10 headless-browser checks pass: `.fbx`/`.FBX` read as Model, the
  label advertises FBX, and no stale usage is sent.

This is authoring-preview evidence from one Mac. It is not shipped-player, native-window,
device or performance evidence. It approves no phase gate.

## Packet, model and identities

- **Packet.** This is a new, independent packet. The worktree held only Astra's
  `brief.md`, so there was no interrupted attempt and no new director feedback beyond the
  brief. I applied every boundary in it:
  - only the frozen binary ran, hash-checked before and after every call;
  - rendering started only after `artifacts/tools/gpu-review-ready` held Astra's slot
    release;
  - writes went only to this packet and to ignored `artifacts/0040-static-fbx-import/`;
  - no Cargo, Rust build, dependency resolution, production edit, account, merge or
    publish.
- **Model and effort.** Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, as stated by
  this session. The prompt arrived in the exact template of `tools/handoff/main.py:235`.
  That runner sends a prompt only after `configure_model_and_effort` gets ACP
  confirmation of "Opus 5.5" and effort "max" (`tools/acp/config.json`; adapter
  `@agentclientprotocol/claude-agent-acp` 0.88.0, `@anthropic-ai/claude-code` 2.1.295).
  I cannot inspect that handshake from inside the session. No substitution occurred.
- **Frozen source.** Worktree HEAD `90abec1c10bef0e0f0ef3110583faebb812851a6`, equal to
  `binary.json` `source_commit`. Production implementation `f77c8ba`, integrated main
  `aef9804`.
- **Frozen binary.** `artifacts/tools/incant_headless`, release, Rust 1.99.0, 39,470,912
  bytes, SHA-256 `e5f686ffb6cc00e7761cb643593f48c2047caaadce2a43ee94f1fbfffa6a67fa`.
  All 220 engine calls matched it before and after (`evidence/ledger.jsonl`).
- **Host.** macOS 26.6 (25G72), arm64. `screenshot` reports adapter "Apple M5 Pro" for
  all 49 captures, and `system_profiler` reports Metal 4 support. The CLI does not print
  the wgpu backend; Metal is the macOS backend and the one `binary.json` expects.
- **Tooling.** Python 3.14.8 standard library only. For check 4: Node 22.23.3 with the
  already-installed, pinned vite 8.3.4, @vitejs/plugin-react 6.1.2, react 19.3.0 and
  playwright-core 1.64.0, plus its cached Chrome Headless Shell 156.0.8078.4. Nothing was
  installed.

## Supported subset exercised

| Area | Exercised with |
| --- | --- |
| ASCII and binary sources, three DCCs | `maya_cube_7500_ascii`, `maya_parented_cubes_7500_ascii`, `max_geometry_transform_instances_7700_ascii`, `blender_272_cube_7400_binary`, `blender_suzanne_multimaterial_7400_binary` (unmodified, MIT ufbx v0.23.1) |
| Units and axes | Maya cm, Max inch (2.54) Z-up, Blender 2.72 metre (100) with `CoordAxisSign -1`/`FrontAxisSign -1`, Blender 4.0 cm with exporter -90° X and ×100 |
| Hierarchy, instancing, geometric transforms | Max 4-node chain sharing one mesh with geometric T/R/S; Maya ZYX-ordered nonuniform parent with child |
| Reflection and nonuniform transforms | derived mirrored parent (−1.1, 1.2, 1.3) with a 40°-rotated (sheared) child; derived instances: mirror X, mirror Y, nonuniform (1.4, 0.7, 1), 90° Z |
| Materials | Blender metallic/roughness (seven partitions, glossy/metallic slots); Lambert diffuse × factor, emissive map, constant emissive, achromatic opacity 0.5 |
| Images | embedded and project-relative (Windows separator, as Maya writes) colour maps; embedded normal maps; dependency-only change |
| Explicit refusals | UV transform, bump, transparency map, specular channel, legacy Phong, second UV set, escaping path, missing image, malformed bytes, inherited ambient template; upstream animated, morph, skinned, Phong-textured and Blender roughness/metal-map files |

## Check 1: static objects, units, axes, hierarchy, reflection, materials

**Method.** The `static-fbx` project imports the FBX sources. The `static-twin` project
has identical entity IDs, transforms and cameras, but each FBX asset is replaced by a glTF
twin. My own code builds each twin from the FBX file:
- per-model T·R(rotation order)·S, with the geometric transform on a non-inherited
  helper;
- the declared axes and `UnitScaleFactor` on one root;
- per-polygon material slots as separate primitives, and the file's own normals.

The twin builder's world positions reproduce the importer's numeric oracles in
`crates/incant_assets/tests/fbx.rs`. Box001 sits at (1.016, 0, 0) and Box002 at
(0, 0, −1.016), and the Maya parent and child origins match (`tools/test_tools.py`).
Nine cameras use the default preview lighting.

**Appearance (my review of the frames):**
- **Suzanne** (`screenshots/engine/static-suzanne-front.png`, `…-profile.png`) stands
  upright and faces +Z.
  - The source's `RightEar`/`RightEye` (blue/red, source −X) are on her right, image left
    from the front. `LeftEar`/`LeftEye` (pink/purple) are on her left.
  - The pupils, the metallic nose/mouth slot (metallic 0.42) and the dark brown head are
    all present.
  - The faceting is the exported flat normals. The glossy left ear (roughness 0) shows
    sharp studio reflections; the matte right ear does not.
  - The profile camera on +X sees the face (+Z) on screen-left, as it should.
- **3ds Max instances** (`static-max-zup-inch-instances-3q.png`, `…-top.png`) form four
  0.508 m boxes on a 1.016 m ring, with heights 0.508 / 0.762 / 1.016 / 1.27 m.
  - Heights rise counter-clockwise seen from above (+X → −Z → −X → +Z), as Max's top view
    has them (+X → +Y → −X → −Y), so handedness is preserved.
  - No box is tilted, so source Z became Y.
- **Mirrored, sheared hierarchy** (`static-maya-mirrored-nonuniform-parent-sheared-child.png`).
  Both solids are closed and outward-lit. No inside-out faces appear, so culling and
  winding are correct under a negative determinant.
- **Units** (`static-overview-true-scale.png`, `static-maya-1cm-cube-on-1cm-ruler.png`):
  - the Blender cube spans two 1 m ruler lengths;
  - Suzanne is about 2.7 m wide, as authored in Blender metres;
  - the Max boxes are 20 inches;
  - the Maya cube is a one-pixel speck at overview scale but matches one 1 cm ruler
    segment in close-up.
- **Material-less meshes** (Max boxes, Blender 2.72 cube) render with the shared glTF
  default material, white metallic 1 / roughness 1, so they read as neutral light grey.
  See Observation 2.

**Measured** (`evidence/analysis.json` → `static_fbx_vs_twin`): 8 of 9 FBX frames are
byte-identical to the twin frames:
- overview, Suzanne back 3/4 and profile;
- Max 3/4 and top;
- parented, shear/mirror and units.

Suzanne front differs in 1 pixel at (532, 256): a rasterization tie on the purple-eye /
brown-head edge, with a 131-level delta. It is not a partition change. The CLI import
details report 7 Suzanne meshes and 2,904 expanded corners (32 triangles + 468 quads),
which is what the source states (`fbx_probe.py`).

## Check 2: colour maps, UV orientation, normal-map orientation, placements

**Method.**
- **Derived sources.** Derivatives of the real Maya cube carry my asymmetric art on Maya's
  six UV islands: an `F` glyph, an axis label, and red/yellow top corner marks. They use
  the legal Video/Texture records of `fbx.rs::textured_source`, plus Maya's `UVSet "map1"`.
- **Normal map.** The OpenGL/glTF-convention normal map holds a raised 35° button per
  island.
- **Controls** in the same frames, ortho at 163.75 px/m with integer 262 px spacing:
  - a real-geometry relief;
  - a glTF quad with explicit `TANGENT (1,0,0,+1)`;
  - the same quad without `TANGENT`;
  - a glTF colour quad.
- **Twin project.** `textures-twin` swaps the FBX assets for glTF twins carrying Maya's
  own exported tangent/binormal frame from the source file.
- **Source-free.** I deleted every FBX, PNG and glTF source and re-rendered two frames.

**Appearance:**
- **Colour maps are right on every face.** Front, right and top islands read upright in
  the front-right-top view. Back, left and bottom follow Maya's wrap: the back is rotated
  180°, not mirrored. The glyphs appear correctly:
  - mirror X shows a mirrored `F` and brings the −X island to +X;
  - mirror Y shows it upside down;
  - nonuniform shows it stretched;
  - no face is missing.
- **Island borders stay clean** in these magnified views. Bilinear softening of
  128-texel islands is expected.
- **Normal maps are wrong.** Under the preview light from the upper right:
  - The real relief and the explicit-tangent glTF quad read as raised buttons, brightest
    on the top bevel.
  - The glTF quad without `TANGENT` reads as a pressed-in button, brightest on the bottom
    bevel. So do the FBX normal cube and every FBX normal instance
    (`textures-controls-colour-and-normal-ortho.png`, `textures-fbx-instances-ortho.png`).
  - The 90°-rotated FBX instance instead reads as lit from the upper left. The same
    symmetric button changes apparent light direction with object orientation.
- **Maya-frame twins are consistent.** The twins with Maya's tangents read raised and lit
  from the upper right in every placement (`textures-twin-maya-tangents-instances-ortho.png`).

**Measured** (`analysis.json`): bevel luminance (sRGB-weighted) at the bevel band centres,
15×15 px windows.

| Object (controls frame) | top | bottom | left | right | top − bottom | right − left |
| --- | --- | --- | --- | --- | --- | --- |
| Real-geometry relief | 228.0 | 159.9 | 183.0 | 213.0 | +68.1 | +30.0 |
| glTF, explicit glTF tangent | 228.0 | 159.9 | 183.0 | 214.0 | +68.1 | +31.0 |
| glTF, engine-generated tangent | 159.9 | 228.0 | 183.0 | 213.0 | −68.1 | +30.0 |
| FBX normal cube (embedded map) | 159.9 | 228.0 | 183.0 | 213.0 | −68.1 | +30.0 |

| FBX instance | FBX: top − bottom / right − left | Twin with Maya's frame |
| --- | --- | --- |
| identity | −68.1 / +30.0 | +68.1 / +31.0 |
| mirror X | −69.0 / +30.0 | +69.0 / +31.0 |
| rotated 90° Z | +68.1 / −31.0 | +69.0 / +31.0 |

The nonuniform instance's fixed sample offsets do not track its scaled bevels. Visually
it matches the other FBX instances.

Pixel comparisons, all 0 differing pixels:
- FBX colour rows against the Maya-frame twins, in both the controls and instances frames.
- Embedded against project-relative front face (147 × 147 px crop at an exact 262 px
  offset).
- glTF colour UV quad against the FBX front face.
- Both source-free frames against the with-source frames (byte-identical).

Every FBX-vs-twin difference lies on normal-mapped primitives:
- 13,291 px in the controls frame, all on the FBX normal cube;
- 52,835 px in the instances frame, on the four normal instances;
- in the perspective views, only on bevels. The colour cubes' six faces show no
  difference from either side (`screenshots/derived/DERIVED-diff-fbx-vs-twin-instances-front-right-top.png`
  and `…-back-left-bottom.png`).

**Lambert constants and maps** (`materials-lambert-cases-vs-expected-gltf.png`, run
`r5`). Diffuse (0.2, 0.5, 0.9) × factor 0.5, emissive map, constant emissive
(0.9, 0.5, 0.1) and 50% achromatic opacity over a uniform backdrop each match a glTF quad
authored with the expected glTF values: 0 of 21,609 pixels differ, four times. In 3/4
view (`materials-lambert-cases-3q.png`) the emissive cube is evenly bright on every face,
and the transparent cube shows its front faces blended over the backdrop.

## Check 3: reimport, Undo/Redo, rejection, source-free continuity

There are two assets. `crate` (asset ID `01M4MKJ8F8NZ5FYNC1VBVRK31K`) embeds its art in
`crate.fbx`. `panel` (`01M4MKJ8HEMH4DJKF3TEC98093`) references
`models/textures/panel.png`. Every step used `import`, `rpc command.execute`,
`history.undo`/`redo`, `project.save`, `screenshot` and `validate` (`evidence/r2-reimport.json`).

| State | Step | Fingerprints (crate / panel) | Rev. | Frame `r3q` |
| --- | --- | --- | --- | --- |
| s0 | import v1 + panel v1 | `b335bdd4…` / `215b638c…` | 3 | A `691737e5…` |
| s1 | reimport both, unchanged: `changed:false`, `cache_hit:true`, project and journal bytes unchanged | same | 3 | A |
| s2 | `crate.fbx` → v2 (embedded art changed): `changed:true`, same ID | `f058ed13…` / `215b638c…` | 4 | B `e9f814d3…` |
| s3 | only `panel.png` → v2 (dependency) | `f058ed13…` / `83738da4…` | 5 | C `c7949b2f…` |
| s4 | Undo (panel import `01M4MKJA9Z4N…`) | `f058ed13…` / `215b638c…` | 6 | B |
| s5 | Undo (crate import `01M4MKJ9VMQQ…`) | `b335bdd4…` / `215b638c…` | 7 | A |
| s6 | Redo crate | `f058ed13…` / `215b638c…` | 8 | B |
| s7 | Redo panel | `f058ed13…` / `83738da4…` | 9 | C |
| s8 | reimport with texture also on `SpecularColor`: exit 1 | unchanged | 9 | C |
| s9 | reimport of truncated v2: exit 1 | unchanged | 9 | C |
| s10 | all 3 source files deleted; fresh `screenshot` process | unchanged | 9 | C (and `rfront` = s3 `rfront`) |
| s11 | reimport without source: exit 1, "asset IO: No such file or directory" | unchanged | 9 | – |

- **Byte-identical frame groups:**
  - A: s0, s1, s5;
  - B: s2, s4, s6;
  - C: s3, s7, s8, s9, s10;
  - front views: s3 = s10.
- **Rejections leave everything unchanged.** Across s8, s9 and s11:
  - project `8cb088da…` and journal `ba4fc6b5…` keep their exact bytes;
  - assets, revision and the 5-entry history are unchanged.
- **Source-free.** `validate` passes with no diagnostics after source removal.
- **Preview continuity.** Each state renders in a fresh process. Retained GPU scenes
  across a reimport cannot be reached through the public CLI, so this checks cache and
  undo continuity, not retained-scene swapping.
- **Rejection messages** (verbatim after `Error:`):
  - "could not cook models/crate.fbx: unsupported asset feature: static FBX: material
    texture channel SpecularColor";
  - "… invalid FBX: Unknown: Failed to load" (Observation 4).

## Check 4: editor import form affordance (browser fixture, not engine)

`tools/ui_affordance.mjs` builds this worktree's `editor/ui` into artifacts with
`configFile: false`, so nothing is written under `editor/`. It serves the build on
127.0.0.1 and drives the cached headless shell against an injected recording test double
that advertises only `asset.import`. All 10 checks pass (`evidence/ui-affordance.json`):

- The label reads "Accepts glTF, GLB, FBX, PNG, JPEG, EXR, .atlas.json, WAV or OGG."
- `models/static.fbx` and `models/static.FBX` show type "Model", no interpretation radios
  and no problem. The row recognises the registered FBX asset ("Updates “static”").
- **Stale usage.** I entered `textures/stale.png`, chose "Normal map", then retyped the
  same row as `models/static.FBX`. The radios disappear, and the dispatched command is
  exactly `{"type":"asset.import","sources":[{"source":"models/static.FBX"}]}`.
- An FBX reimport from the details pane sends the same payload, with no usage.
- `models/static.obj` gets "Unsupported file type. Use glTF, GLB, FBX, …".
- There were no page errors and no non-localhost requests.

Appearance: the FBX row carries the model glyph and the same "Model · Updates …" meta
line as glTF rows, and the format line wraps cleanly in the inspector width
(`screenshots/ui/`). The form design is unchanged, as required. These are fixture
captures, not native-window evidence.

## Defect 1 (blocking for FBX normal maps): generated tangents invert the normal map's Y axis

- **Symptom.** Every FBX normal map shows relief inverted along the texture's V axis. A
  raised feature reads as pressed in, and the apparent light direction rotates with the
  object, as in the table above. glTF primitives without `TANGENT` do the same. glTF with
  explicit tangents renders correctly.
- **Minimal repro.**
  1. Run `python3 -I tools/make_sources.py` and `python3 -I tools/review.py textures RUN`.
  2. In frame `controls_ortho`, compare:
     - `controls/quad_normal_explicit.glb` (top bevel 228, bottom 160);
     - `controls/quad_normal_generated.glb` (160 / 228);
     - `derived/normal_embedded.fbx` (160 / 228).
  3. The two glTF files differ only in the `TANGENT` attribute.
- **Cause.**
  - `crates/incant_assets/src/tangents.rs` runs MikkTSpace on top-left-origin UVs and
    stores its sign unchanged. FBX V is flipped at `fbx/geometry.rs:99`; glTF V is native.
  - The pinned `mikktspace` 0.3.0 sets the sign to +1 only for a positive UV signed area
    (`generated.rs:1061-1064`), so an ordinary mapping gets w = −1.
  - `model_material.wgsl:49` builds `b = cross(n, t) * w` as the glTF image-up (+Y) axis,
    so b points toward image-down.
  - Maya's own exported binormal for the cube's front face is +Y
    (`maya_cube_7500_ascii.fbx`, `LayerElementBinormal`), which means w = +1. A twin
    using that frame renders correctly in every placement.
- **Ownership.**
  - The shared asset-cook tangent generation, used by every FBX normal map and by glTF
    without `TANGENT`.
  - Not ufbx parsing or UV conversion: colour maps are exact.
  - Not the shader: explicit tangents are exact, and mirrored placements flip correctly
    via the determinant sign.
  - The existing CPU test
    `normal_texture_keeps_linear_role_and_generates_orthonormal_corner_tangents` checks
    orthonormality and |w| = 1, not the sign.
- **Suggested direction.** This is Astra's decision; I made no production change.
  - Negate the generated sign in `tangents::generate`, or feed MikkTSpace (u, 1 − v).
  - Add a CPU assertion: the Maya cube's +Z face gets w = +1, so the bitangent points
    toward decreasing top-left V.
  - Add a rendered regression comparing generated and explicit tangents on the same quad,
    as in `controls_ortho`.
  - The fix also changes current glTF-without-`TANGENT` output, toward the glTF
    definition.

## Other observations

| # | Observation | Owner | Severity |
| --- | --- | --- | --- |
| 2 | FBX meshes with no material get the glTF default (white, metallic 1, roughness 1). They read as neutral light grey, not the Max object colour or a DCC default. | Shared model/renderer default (`incant_assets_runtime::Material::default`) | Low, appearance |
| 3 | A connected colour or emissive map is multiplied by the exported constant colour (ufbx's documented tint). The real upstream Maya textured export writes `EmissiveColor 0.5`, so an equivalent Lambert map would emit at 50%. My derivatives set white explicitly. I cannot verify Maya's own display here. | FBX conversion convention | Open question |
| 4 | Truncated FBX reports "invalid FBX: Unknown: Failed to load", which is explicit but uninformative. | FBX conversion diagnostics | Low |
| 5 | Lambert materials that omit `AmbientColor` inherit the FBX template's 0.2 ambient and are refused ("ambient material color"). My first materials derivative hit this (run `r4`). Real Maya exports write `AmbientColor 0,0,0`. | Documented unsupported feature, working as designed | Note |
| 6 | One-pixel FBX/twin difference on a Suzanne edge. | Rasterization tie | None |

## Capture ledger and budget

| Run | Content | Captures |
| --- | --- | --- |
| smoke | pipeline smoke (Maya cube) | 1 |
| r1 static-fbx / static-twin | 9 cameras each | 18 |
| r1 textures | 5 with sources + 2 source-free | 7 |
| r1 textures-twin | 5 | 5 |
| r2 reimport | s0–s10 | 14 |
| r3 repeat | 2 fresh-process repeats (both byte-identical) | 2 |
| r5 materials | 2 | 2 |
| **Total** | 49 of 96; every frame 960 × 540; 1,440,714 bytes raw (1.37 of 256 MiB); 0 timeouts; adapter Apple M5 Pro | **49** |

Each `evidence/ledger.jsonl` record carries:
- the command, PID, binary hash before and after, exit code, timeout and elapsed time;
- project/journal/asset state;
- camera, size and adapter;
- output SHA-256.

Runs `probe` and `probe2` are CPU-only imports. Two runs aborted before any capture, both
ledgered:
- `r1/reimport`: my tool used the invalid ULID character "I";
- `r4`: the ambient case above.

**Device slot.** All renderer and browser capture processes had exited by
2026-10-11T00:35:16-04:00 (`artifacts/0040-static-fbx-import/capture-processes-exited.txt`).
`probe2` afterwards ran CPU-only imports.

## Commands and results

From the repository root, with `T=handoffs/0040-static-fbx-import/tools`:

```sh
python3 -I $T/test_tools.py                       # 10 tests OK (twin math vs importer oracles, art, probe, PNG)
python3 -I $T/make_sources.py                     # 51 sources; two runs reproduce byte for byte, manifest included
python3 -I $T/probe_imports.py probe2             # 29 accepted, 15 refused, all refusals leave project/journal unchanged
python3 -I $T/review.py static r1                 # 18 captures
python3 -I $T/review.py textures r1               # 7 captures, sources then source-free; validate valid
python3 -I $T/review.py textures-twin r1          # 5 captures
python3 -I $T/review.py reimport r2               # 14 captures; 3 expected exit-1 imports (s8, s9, s11)
python3 -I $T/review.py repeat r3 "<static-fbx project>,01JA40ST0000000000000000W6,suzanne_front" \
    "<textures project>,01JA40TX0000000000000000W5,controls_ortho-source-free"   # 2 captures, byte-identical
python3 -I $T/review.py materials r5              # 2 captures (r4 aborted on the ambient refusal)
node $T/ui_affordance.mjs                         # 10/10 PASS
python3 -I $T/report_numbers.py                   # evidence/analysis.json
python3 -I $T/assemble.py                         # 20 screenshots, ledger, manifests
cd handoffs/0040-static-fbx-import && shasum -a 256 -c evidence/screenshots.sha256   # 20 OK
```

The repeat projects were `artifacts/0040-static-fbx-import/runs/r1/static-fbx/static-fbx.incant.json`
and `…/r1/textures/textures.incant.json`. The first probe run and the smoke capture used
inline code with the same logic as `probe_imports.py` and `review.py`; their calls are in
the ledger.

**Not run:**
- Cargo, Clippy and the Rust/GPU test suites;
- the UI unit suite, strict TypeScript, and `npm` scripts;
- Play, native windows, other GPUs, phones and hosted CI.

These belong to Astra's checks or are out of scope.

## Provenance and hash manifest

- **Upstream sources.** The 10 files in `crates/incant_assets/tests/fixtures/fbx` are from
  ufbx v0.23.1 `data/`, MIT alternative A. Each was verified against `provenance.json`
  before copying. Byte-identical copies went into the projects.
- **Derived sources.** `tools/derive.py` writes them from the Maya cube and Maya parented
  cubes. Every edit is a named, single-occurrence anchor replacement, listed per file in
  `evidence/source-manifest.json`.
- **Original art.** `tools/art.py` generates five PNGs from my own 5×7 font and
  geometry: `face_art_v1/v2`, `panel_v1/v2` and `bevels`.
- **glTF controls and twins.** `tools/controls.py` and `tools/twins.py` generate them
  with no engine code. The twins follow two documented encodings:
  - ufbx's quad-diagonal rule;
  - Blender's `Shininess = (10(1 − r))²` and `ReflectionFactor` = metallic encoding.

  The textured twins carry Maya's exported tangent/binormal frame.
- **Hashes.**
  - `evidence/source-manifest.json` holds SHA-256 for all 51 sources.
  - `evidence/screenshots.sha256` covers the committed images. Each engine image equals
    its ledger output hash; `evidence/screenshots.json` maps images to ledger records,
    cameras and project state.
  - Every source hash used by a run is recorded in that run's import replies.
  - Sources added after earlier runs (textured twins, material cases, backdrop) left
    every earlier output byte-identical. I verified this against each run's recorded
    import hashes.
  - After the run, I rewrote one absolute local path in `r2-reimport.json`
    (`remaining_models_dir`) relative to the repository; the tool now records it that
    way. No other evidence was edited.

## Screenshots

- **Unedited engine frames:** `screenshots/engine/`, 16 files:
  - static: overview, Suzanne front and profile, Max 3/4 and top, mirrored/sheared
    hierarchy, 1 cm units;
  - textures: controls ortho, FBX instances ortho, Maya-frame twin instances ortho, FBX
    instances 3/4;
  - reimport: s0, s3, and s10 source-free;
  - materials: ortho and 3/4.
- **Derived:** `screenshots/derived/DERIVED-diff-fbx-vs-twin-instances-{front-right-top,back-left-bottom}.png`
  are amplified difference maps of FBX against the Maya-frame twins, in which red marks
  differing pixels. They are not engine frames.
- **Browser fixture:** `screenshots/ui/`, 2 element captures of the existing form at
  device scale 2. They are not native or engine pixels.

## Changed paths

All changes are under `handoffs/0040-static-fbx-import/`. `brief.md` is Astra's and
unchanged.

- `result.md`.
- `tools/`: `engine.py`, `scene.py`, `pngio.py`, `fbx_probe.py`, `art.py`, `derive.py`,
  `gltfio.py`, `controls.py`, `twins.py`, `make_sources.py`, `probe_imports.py`,
  `review.py`, `analyse.py`, `report_numbers.py`, `assemble.py`, `test_tools.py` and
  `ui_affordance.mjs`.
- `evidence/`:
  - `ledger.jsonl`, `analysis.json`, `source-manifest.json`, `probe-imports.json`;
  - `r1-static-fbx.json`, `r1-static-twin.json`, `r1-textures.json`,
    `r1-textures-twin.json`, `r2-reimport.json`, `r3-repeat.json`, `r5-materials.json`;
  - `ui-affordance.json`, `screenshots.json`, `screenshots.sha256`.
- `screenshots/engine/` (16), `screenshots/derived/` (2), `screenshots/ui/` (2).

Projects, caches, raw frames, sources, the UI build and the binary stay ignored under
`artifacts/`.

## Limitations

- **Hardware.** One Mac, one GPU (Apple M5 Pro), Metal, headless, preview lighting only.
  No native window, other GPU, phone, shipped player or performance claim. A source-free
  snapshot is an authoring render.
- **Twin independence.** The twins share the renderer with the FBX path. Their agreement
  verifies conversion and partitions, and my visual review covers renderer-level
  correctness for these views. Twin material values rest on the documented Blender
  encoding, not on Blender itself.
- **Coverage.** These DCC files and derivatives only. Normal-map orientation is a
  convention for Maya/Blender-style OpenGL maps. FBX carries no flag for DirectX-style
  (3ds Max) maps; that remains an authoring convention. Minification bleed between UV
  islands was not tested.
- **Not supported.** Animation, skinning, morphs, legacy Phong and the advanced material
  features remain documented refusals, not support.
- **Retained GPU scenes.** Retained-scene GPU swapping across a reimport is not reachable
  through the public CLI and was not verified.

## Open questions

1. Should generated tangents be fixed in shared code, changing glTF-without-`TANGENT`
   output too? I recommend yes.
2. Should a connected Lambert colour or emissive map ignore the exported constant, as
   Maya's connection semantics suggest, rather than be tinted by it? This needs a real
   Maya check (Observation 3).
3. Should material-less FBX meshes get a neutral dielectric default instead of glTF's
   metallic default?
