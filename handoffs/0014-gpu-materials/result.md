# 0014 GPU materials: preview look-dev and pixel review — result

## Status

Review complete. Studio fill constants tuned. Awaiting Astra's regenerated captures
for final review. No phase gate is approved or claimed by this result.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session itself cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0014-gpu-materials/brief.md`. No newer director feedback or
  priority revisions were present beyond the brief itself.

## Verdict

**Material semantics: visually correct in all 16 captures.** Every color, alpha,
sidedness, filtering and blend result matches the glTF intent. I reproduced each lit
center pixel exactly with an independent CPU evaluation of `model_material.wgsl`.

**Neutral preview: was too dim; now acceptable as a fixed diagnostic preview, not as
production look-dev.** The original constants rendered a face-on surface at about 42%
of its albedo. Dark albedos and the default imported model nearly vanished into the
background. The default glTF material (metallic 1, roughness 1) stays a mid gray until
specular IBL exists. That is correct physics, and I did not fake it.

## Change

`crates/incant_render/src/studio.rs`, constants only. Eye and light direction unchanged.

| Constant | Before | After |
|---|---|---|
| LIGHT_RADIANCE | 0.65 | 2.0 |
| DIFFUSE_ENVIRONMENT | 0.25 | 0.30 |

Rationale: the shader divides direct diffuse by pi. With radiance 2.0, a face-on white
dielectric (n·l = 0.80 for this light) lands near 0.80 linear, about sRGB 231. A fully
lit white face reaches about 0.91, so it still does not clip. Unlit sides get the 0.30
fill, which gives roughly a 3:1 key-to-fill ratio, a conventional neutral studio
contrast. A comment in the file records this reasoning and the remaining limits.

No shader, geometry, alpha, color-space math, fixtures, tests, UI, dependencies or
project state changed.

## Inspected captures

All 16 PNGs in `artifacts/material-review/` were viewed and decoded. Background is
(20, 21, 25) in every frame. The table gives center pixels in sRGB bytes and the WCAG
contrast ratio against the background. "Tuned" values are my local re-captures,
described under Evidence.

| Capture | Original | Tuned | Contrast orig / tuned | Finding |
|---|---|---|---|---|
| base-color-texture | 85,42,22 | 117,61,36 | 1.50 / 2.13 | sRGB decode correct. Was muddy and nearly invisible. |
| reimported-blue-texture | 22,85,172 | 36,117,231 | 2.54 / 4.15 | Retained material swap is correct. |
| metallic-roughness-map | 125,98,71 | 167,132,98 | 3.22 / 5.32 | B=metallic, G=roughness read correctly. Warm tan, plausible. |
| normal-map | 150,150,150 | 182,182,182 | 6.17 / 9.00 | Tilt toward +X lowers n·l. Correct, but legibility is limited. See finding 3. |
| occlusion-map | 112,112,112 | 187,187,187 | 3.68 / 9.50 | Occlusion affects only indirect light, per glTF. Correct. |
| emissive-map | 128,64,32 | 128,64,32 | 2.32 / 2.32 | Exact pass-through. Unaffected by tuning, as intended. |
| sixteen-bit-color | 127,64,32 | 127,64,32 | 2.31 / 2.31 | 16-bit source matches 8-bit within one byte. |
| opaque-alpha-zero | 255,0,0 | 255,0,0 | 4.56 | OPAQUE ignores alpha. Pure red is a test signal. |
| mask-discarded | background | background | 1.00 | Intentionally background only. Bit-identical to the empty frame. |
| reflected-single-sided | 255,0,0 | 255,0,0 | 4.56 | Mirrored instance keeps its authored front face. |
| double-sided-back | 255,0,0 | 255,0,0 | 4.56 | Back face drawn when double-sided. Emissive only, so lit back-face normals are not shown. |
| layered-transparency | 188,6,137 | 188,6,137 | 3.06 | Linear 0.5 red over 0.5 blue over background. Back-to-front order correct. |
| opaque-between-transparent | 188,188,0 | 188,188,0 | 8.98 | Red blends over the opaque green layer. Far blue is hidden at center. |
| linear-filter | 188,188,0 | 188,188,0 | 8.98 | Bilinear blend of red and green happens in linear space. |
| minified-color-mips | 188,188,0 | 188,188,0 | 8.98 | 1000x repeat resolves to a uniform mip average. No moiré. |
| default-metal (640x360) | about 63 gray | about 110 gray | 1.73 / about 3.7 | Default material is correct physically, but read as almost black. See finding 1. |

The supplied `default-metal.png` has the same lit-pixel coverage as the render-check
*reimported* frame: triangles span x 294 to 454 over 4854 pixels in both. My tuned comparison therefore uses
the reimported frame.

## Concrete findings

1. **Default imported model is the weakest preview case.** A glTF with no material gets
   metallic 1, roughness 1. Without a specular environment, its only light is the broad
   direct GGX lobe. It rendered at about sRGB 63 on a 21 background. Tuning raises it to
   about 110, which is readable but still reads as dark gray, not white. The real fix is
   specular IBL, which is listed below for Astra. Metallic was not altered.
2. **Glossy highlights will clip and shift hue.** There is no tonemapping, and output
   clamps per channel. Low-roughness metals in particular will show a near-black body
   with a hot white spot. No current fixture exercises this, so no capture shows it yet.
3. **Normal-map legibility is limited by the fixture.** A constant 1x1 normal on a flat
   quad gives one uniform darker gray, which looks identical to a darker albedo. The
   result is correct, but a viewer cannot recognize it as normal mapping. A varying
   normal map or a curved mesh would make the effect visible. This is a fixture
   suggestion only, since fixtures were out of scope.
4. **Transparency edge strips are parallax, not sorting errors.** The layered quads sit
   0.2 units apart in depth. From the fixed eye, the near and far quads project slightly
   offset. That exposes thin red-only (188,13,15), blue-only (12,13,188) and green-only
   (0,255,0) strips. They have hard stair-stepped edges because there is no
   antialiasing.
5. **Dark albedos remain low-contrast against the near-black backdrop.** The tuned
   terracotta texture reaches only 2.13:1. The light can make it no brighter without
   clipping white albedos. A slightly lighter neutral backdrop or a ground plane would
   help. Both belong to the viewport background and are outside this packet's scope.
6. **Framing.** The default model is small and sits above and right of center in
   640x360. Eye was preserved as instructed. A frame-to-bounds preview camera would
   improve legibility later.
7. **Unverified lit sidedness.** The sidedness fixtures are all emissive. The shader's
   back-face normal flip under lighting is therefore not visually reviewed by any
   capture.

## Requests for Astra's next implementation

These do not exist yet and are not claimed:

- Specular image-based lighting, using a prefiltered environment and a BRDF lookup
  table. Default-material and metallic imports need it to read correctly.
- Tonemapping plus exposure. A neutral operator such as Khronos PBR Neutral suits a
  product-style preview. Once it exists, LIGHT_RADIANCE and DIFFUSE_ENVIRONMENT
  should be retuned against it.
- Optional fixtures: a varying normal map or curved mesh, a lit double-sided back face,
  and a low-roughness metal to show highlight clipping.
- `docs/spikes/evidence/imported-geometry-2026-10-09.json` records lit-model hashes
  taken with the old constants. No script re-checks them, but fresh runs will no longer
  reproduce those hashes. That record is Astra's to annotate or regenerate.

## Evidence and commands

All commands ran in this worktree with `CARGO_TARGET_DIR` unset, so the target is this
worktree's own `target/`. Hardware is an Apple M5 Pro, as reported by the engine.

```
INCANT_MATERIAL_EVIDENCE=$PWD/artifacts/material-review-tuned \
  ./tools/cargo test -p incant_render --test model_gpu -- --ignored
  -> 5 passed, 0 failed
./tools/cargo build --release -p incant_headless            -> ok
python3 -I tools/platforms/render-check.py --output artifacts/render-tuned
  -> native_renderer_executed true, source_independent true, reimport_changes_pixels true
./tools/cargo test -p incant_editor --test model_gpu -- --ignored  -> 1 passed
./tools/cargo fmt --check -p incant_render                  -> clean
./tools/cargo clippy -p incant_render --all-targets -- -D warnings -> clean
./tools/cargo test -p incant_render                         -> 2 passed, 5 GPU tests ignored by default
```

The occlusion test needs a margin of at least 20 bytes. With the new constants the
margin is 44, from flat 231 down to occluded 187. Black-base emissive fixtures receive
zero lighting at both settings, so their one-byte tolerance assertions are unaffected.

Screenshot paths are ignored and uncommitted:

- `artifacts/material-review/*.png` holds the 16 supplied originals, all reviewed.
- `artifacts/material-review-tuned/*.png` holds 16 local re-captures with the tuned
  constants. `default-metal.png` there is a copy of `artifacts/render-tuned/models/reimported.png`.
- `artifacts/render-tuned/` holds the full render-check output.

The local re-captures were produced by me for verification. They are not a
substitute for Astra's official regeneration. No native UI screenshots were taken,
per the brief. No visual-regression baseline exists or was claimed.

## Limitations (unchanged renderer scope)

This is a fixed preview. It has no authored lights, clustered render graph, shadows,
specular environment, SSAO, bloom, tonemapping or antialiasing. Pure red, green and
blue emissive images are numerical test signals, not art direction or game art. CPU
and GPU resource lifetime, source watching and Undo paths were not modified. The
retained-scene and source-removal GPU tests still pass.

## Changed paths

- `crates/incant_render/src/studio.rs`
- `handoffs/0014-gpu-materials/result.md`

## Open questions (first pass, now resolved)

Astra settled both first-pass questions as a routine integration decision. The
director's standing instruction is to continue without asking, and Astra acted under
it. This was not a new explicit human design decision. The neutral backdrop and fixed
camera stay for this increment. Specular IBL and tone mapping are the next renderer
work, and automatic framing can follow. No question remains open.

---

# Final review: integrated captures (priority follow-up)

Model: Claude Opus 5.5, model ID `claude-opus-5-5`, in the same handoff session
setup as the first pass. No builds, code changes or new screenshots were made in this
pass. Captures were decoded locally and inspected without modifying them.

## Packet feedback acknowledged

- Astra integrated the studio tuning. Astra reports that all seven explicit GPU tests
  pass, including the new lit back-face and varying normal-map checks. I did not
  rerun them in this pass.
- The neutral backdrop and fixed camera stay for this increment. That was Astra's
  routine integration decision under the director's continue-without-asking
  instruction. I withdrew the backdrop and framing questions rather than escalating
  them.
- Specular IBL and tone mapping are the next renderer work. Automatic framing follows.
- Historical captures and hashes are not treated as current output. This review uses
  only the 19 freshly generated PNGs and the two final native captures.

## Final verdict

**Accepted for this scoped increment. No regression found.** Every remaining issue
below is an accepted, documented preview limitation, not a defect.

- **Integrated source.** `studio.rs` and `model_material.wgsl` are unchanged since my
  tuning commit. Astra's merge changed only typed resource errors in `materials.rs` and
  `models.rs`, with no shading effect.
- **Carried-over captures.** All 16 are pixel-identical to my first-pass local
  recaptures with the tuned constants. Zero pixels differ in each. The first-pass
  findings table therefore holds unchanged for current output.

## The three new GPU captures

| Capture | Measured | Assessment |
|---|---|---|
| lit-double-sided-back | uniform 231,231,231 | Matches a lit white front face exactly. The back-face normal flip is correct. This closes first-pass finding 7. |
| varying-normal-map | halves of 231 and 182 | The flat half matches a flat white face. The tilted half matches the single-tilt normal map. The split reads clearly as shading, not albedo. This closes first-pass finding 3. |
| textured-cube (640x360) | front 199,136,65 and 40,109,199; top 179,122,57 and 34,97,179; side 156,106,48 and 28,84,156 | All three face tones match my independent shader evaluation within one byte. The cube reads clearly as a solid with three distinct planes. |

The cube's stripes stay straight and continuous across each face, with no visible
UV seams or swaps. The front face is brightest, the top is mid, and the right side is
darkest. That gradient follows the fixed light direction and gives a clear, neutral
three-tone read. The cube is a mathematical fixture, not game art.

`default-metal.png` is pixel-identical to my tuned recapture, about sRGB 110 gray on a
background of 20. It is legible. It still reads as gray rather than white, which is
the accepted no-specular-IBL limitation.

## Native captures

These are `10-final-wide.jpg` at 1440x900 and `11-final-minimum.jpg` at 1000x650.

- **Viewport material appearance.** The native viewport shows the same textured cube.
  It has the same three-tone face shading and stripe layout as the headless readback.
  It sits on the same neutral near-black backdrop. Native output and headless capture
  agree visually. I cannot claim a pixel comparison because both files are JPEGs.
- **Framing.** At wide size the cube is centered in the viewport and fills about
  half of its height. At minimum size it stays centered and fully visible, scaled
  down with the narrower viewport. The fixed camera frames this fixture acceptably.
  Small or offset models, such as the default triangles, will still need automatic
  framing later.
- **Sidebar and Inspector separation.** It is preserved at both sizes. The left
  sidebar holds the Assets tab, a filter, Import, and the selected cube row. The
  right Inspector holds the cube's model identity, source file, a Reimport action
  with help text, and the Identifiers disclosure. Neither panel duplicates the
  other, and the viewport keeps its own region between them.
- **Inspector overflow at 1000x650 is scrollable content, not clipping.** Corrected
  in the amendment below. In the unscrolled capture, the Reimport help and the
  Identifiers row sit below the fold of a working Inspector scroll area. That is the
  intended behavior, not a defect. A scroll-affordance polish follow-up is stated in
  the amendment.
- **Capture overlays ignored.** The purple screen-recording badge over the macOS
  window controls is an OS capture overlay. The pointer cursor in the wide capture is
  also not application chrome. Neither was used to judge traffic-light spacing or chrome.

## Accepted preview limitations, not regressions

- Metals and the default material look dark outside their direct highlight because
  there is no specular IBL. This is the next renderer work.
- Glossy highlights can clip and shift hue because there is no tone mapping. This is
  also next renderer work.
- Geometry edges are aliased because there is no antialiasing. The cube silhouettes
  and transparency parallax strips show hard stair steps.
- Dark albedos stay low contrast against the retained near-black backdrop. The
  terracotta texture measures 2.13:1.
- The fixed camera leaves small or offset imported models undersized. Automatic
  framing follows later.
- There are no authored lights, render graph, shadows, SSAO or bloom.

## Evidence for this pass

- 19 PNGs in `artifacts/material-review/` were viewed and decoded.
- 2 JPEGs in `artifacts/native-material-review/` were viewed. A temporary crop of the
  minimum-size Inspector region was used for inspection and then deleted.
- 16 carried-over PNGs were compared pixel by pixel against
  `artifacts/material-review-tuned/`, with zero differing pixels each.
- The cube's face tones were checked against a CPU evaluation of the preview shader.

No visual-regression baseline exists, and none is claimed. No phase gate is approved
by this review.

## Changed paths in this pass

- `handoffs/0014-gpu-materials/result.md`

---

# Amendment: minimum-height Inspector scrolling (priority follow-up)

Model: Claude Opus 5.5, model ID `claude-opus-5-5`, in the same handoff session setup.
This pass was review-only. I made no builds, code edits or new captures.

## Correction

My final review said the Reimport help was "clipped" at 1000x650 and the Identifiers
row was "pushed out of view". That wording was wrong. The content sat below the fold
of a working scroll viewport. It was never permanently clipped or inaccessible.

## Evidence

- **Capture.** `artifacts/native-material-review/12-minimum-inspector-scrolled.jpg`
  shows the 1000x650 window after a normal wheel scroll in the Inspector. The Source
  block, Reimport button and both lines of its help text are fully visible. The
  Identifiers disclosure row is visible below them with its own divider. A macOS
  overlay scrollbar appears at the Inspector's right edge, spanning only the
  Inspector region.
- **Fixed panels.** Compared with `11-final-minimum.jpg`, the viewport, cube, bottom
  Problems panel, Agent panel, asset sidebar and top bar are unchanged in position.
  Only the Inspector content moved. Its Model header scrolled up under the fixed
  Inspector title.
- **Code.** `.panel__scroll` in `editor/ui/src/styles/app.css` sets `flex: 1`,
  `min-height: 0` and `overflow: auto`. The Inspector body renders inside it in
  `InspectorPanel.tsx`. That is a correct independent flex scroll container. The
  zero minimum height is what lets it shrink and scroll rather than push the Agent
  panel.
- **Cursor.** The pointer glow in this capture is the capture tool's cursor, not
  application chrome.

## Verdict

Inspector scrolling is functional and accepted. This is not a defect and not a
regression of this increment.

## Optional polish follow-up

Overlay scrollbars on macOS hide at rest. In the unscrolled state, the last visible
help line meets the Agent divider with no sign that more content follows. A user can
reasonably read that as a hard cut.

Specific follow-up for a later UI pass: when `.panel__scroll` content overflows and
is not at its bottom, show a subtle bottom edge cue. A fade of about 16px, from
transparent to the panel background color, would do it. Remove the cue when the
scroll reaches the end. A matching top cue would help when scrolled down. This is
appearance polish only. It does not change behavior, and it does not block this
increment.

## Changed paths in this pass

- `handoffs/0014-gpu-materials/result.md`
