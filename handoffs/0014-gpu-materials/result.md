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

## Open questions

- Should the viewport backdrop move to a lighter neutral? That would lift dark-albedo
  contrast, but it is a viewport and UI decision outside this packet.
- Is a frame-to-bounds preview camera wanted before or after specular IBL lands?
