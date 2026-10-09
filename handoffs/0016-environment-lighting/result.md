# 0016 Environment lighting: default studio look-dev — result

## Status

**Headless GPU review complete. The studio look is approved with one tuning change,
pending regenerated captures. Native integration is not reviewed or approved.**

- **First pass:** complete. It added `preview_environment.rs` with numeric checks.
- **Integrated headless GPU review:** complete. I reviewed the actual PNGs in
  `artifacts/environment-initial/`. Reflection readability, roughness progression,
  dark albedos, neutrality and seams pass. The fill-to-key hierarchy on bright metal
  did not pass, so I retuned two panel intensities. The current captures predate that
  change, and the regenerated set listed below needs a confirming look.
- **Native integration:** not reviewed. The packet says native captures follow and
  must not be claimed yet.

No phase gate is approved or claimed.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0016-environment-lighting/brief.md`. Its priority revision is the
  integrated GPU pixel review. It carries no new typed director feedback. It is
  Astra's routine follow-up under the standing continuation instruction. The packet's
  points are applied as follows:
  - The old constant fill is gone.
  - The direct key stays separate, and the aligned key panel is intentional.
  - The reflected floor is distant environment only, not a visibility surface.
  - The stale `studio.rs` comments are refreshed.
  - No full build or native capture was run.

## Changed paths

Changed in this review:

- `crates/incant_render/src/preview_environment.rs`. The key panel radiance goes from
  1.7 to 1.4, and the fill card radiance goes from 0.7 to 0.45. The header comment now
  documents the panel hierarchy, the new diffuse budget and the fact that the floor is
  not geometry.
- `crates/incant_render/src/studio.rs`. Comments only. They now describe the separate
  direct key, the aligned key panel, the absence of shadows, the convolved environment
  that replaced the 0.3 fill and how the 0015 shoulder treats the key reflection. No
  constants changed.
- `handoffs/0016-environment-lighting/result.md` (this file).

From the first pass, already committed: `crates/incant_render/src/preview_environment.rs`
was created. No shaders, math, tests, UI or project files were touched in either pass.

## Pixel review of `artifacts/environment-initial/`

**What I inspected:**

- All twelve 640×480 studio sphere captures, at native resolution.
- 3× nearest-neighbor enlargements of the smooth metal and smooth dark spheres, made
  to check edges and seams.
- The authored-environment, highlight and material-map captures.
- Per-pixel statistics from a stdlib-only PNG decoder. Pillow is not installed. The
  scripts live under the ignored `target/env_check/` folder and are not committed.

The spheres are numeric geometry fixtures, not game art. The test materials are:

| Material | Base color | Metallic |
|---|---|---|
| Metal | 0.8 | 1 |
| Dielectric | 0.5 | 0 |
| Dark | 0.03 | 0 |

Each material renders at roughness 0.045, 0.3, 0.7 and 1.0.

| Criterion | Verdict | Evidence |
|---|---|---|
| Studio reflection readability | Pass | Smooth metal shows the key rectangle, the fill card on the right, the rim arc along the top edge and the horizon split. Smooth dark shows the same panels at low contrast, with the direct key's point highlight centered in the key panel as designed. |
| Roughness progression | Pass | On metal, the panels go from sharp rectangles to soft blobs, then a broad glow, then an almost flat sphere. The center profile reads 249, then 254, then 229, then 128. The rough and matte dielectric spheres differ only slightly, which is correct for an 0.04 specular. |
| Dark albedos | Pass | The dark spheres keep their base color. Matte dark reads about 44 to 59 against a background of about 21. Smooth and satin dark still show panel reflections. |
| Neutrality | Pass | Inside every sphere, the largest channel spread is 1 sRGB level, or 2 on metal. No neutral material shows a cast. |
| Bright highlight rolloff | Pass with a change | Highlights roll smoothly onto the 0015 shoulder without hue shift. Satin metal has a white core of about 530 clipped pixels, about 13 px in radius, which is plausible for a light source. On metal, the fill reflection reached 226 against the key's 249, too close in brightness. That is fixed below. |
| Seams and artifacts | Pass | The 3× crops show no cubemap seams, texel stepping or band edges. Panel edges and the horizon band blur smoothly. Mirror metal has a high count of sharp second-difference spikes, but they sit on the panel edges, which a near-mirror should render sharply. |

**Diffuse readability:** dielectric spheres read soft. The center is about 192, the
top and bottom are about 173 and 157, and the lower-left edge is about 125. The key
lights the visible face from only about 13 degrees off the view direction, so the
terminator is hidden. This comes from the 0014 camera and key geometry. The
environment does not cause it, and it does not block readability.

**Silhouette aliasing:** sphere edges show stair-stepping in the enlargements. This is
geometry rasterization without antialiasing. It is outside environment look-dev, and
I note it only for Astra.

**Authored captures:** `environment-rotation-zero` and `environment-rotation-half-turn`
show a faint vertical band near the right edge of the quad. It most likely comes from
the authored 64×32 map's own color split, so I did not judge it as a studio defect.
Astra should confirm this is expected.

## Tuning change and why

The key panel reflection on an 0.8 metal lands at 1.3 to 1.6 linear. That sits on the
0.8 tone-map shoulder at about 248 to 250 sRGB. The old fill reflection landed at
about 0.79 linear, just under the shoulder, at about 229 to 230 sRGB. Shoulder
compression squeezed the key-to-fill difference to about 20 levels. In the smooth and
satin metal captures, the fill competes with the key.

| Panel | Old | New | Predicted reflection on 0.8 metal |
|---|---|---|---|
| Key softbox | 1.7 | 1.4 | about 1.35 linear, about 249 sRGB |
| Fill card | 0.7 | 0.45 | about 0.59 linear, about 202 sRGB |
| Rim strip | 1.6 | 1.6 | unchanged |

The predicted sRGB values come from the 0015 curve applied to panel plus backdrop,
times the metal base color. They are predictions, not measurements. The key still
reads near white. Trimming it lowers its diffuse double count with the direct key and
gives a little more headroom around the satin highlight.

## Numerical checks after the change

The scratch harness mounts the real source file through `#[path]` and builds with the
pinned 1.99.0 toolchain. It is not a crate test and makes no full workspace build.

```
rustc --edition 2024 -O -o target/env_check/check target/env_check/main.rs && target/env_check/check
rustfmt --edition 2024 --check crates/incant_render/src/preview_environment.rs crates/incant_render/src/studio.rs   # clean
clippy-driver --crate-type lib target/env_check/lib.rs -W clippy::all -W clippy::pedantic   # no warnings
```

| Check over 2,000,000 directions | Result |
|---|---|
| Channel range | 0.146 to 1.88, against a limit of 16 |
| Non-finite, negative or over-limit values | 0 |
| Largest relative chroma | 0.027 |
| Largest step between texels on a 128 px face | 0.24 |
| NaN, infinity, zero, 1e30 and 1e-30 inputs | finite and in range |

Cosine-weighted mean radiance replaces the old 0.3 fill. The values are for the green
channel. The other channels differ by at most 0.003.

| Normal | First pass | Now |
|---|---|---|
| Mean over all normals | 0.323 | 0.302 |
| Up | 0.450 | 0.422 |
| Toward the key | 0.414 | 0.380 |
| Camera right, fill side | 0.368 | 0.319 |
| Away from the key | 0.252 | 0.249 |
| Down | 0.150 | 0.149 |

**Existing assertions that depend on the default studio:** I checked their margins
without running the tests.

- **Dark-fill test:** the surface normal of [1,0,-0.4] faces the fill. Its irradiance
  term falls from 0.369 to 0.318. A two-term fit to the current captures predicts
  about 58, 74 and 112 sRGB, against 62, 78 and 120 now. The test requires each value
  to fall between 20 and 160 and to rise by more than 12 per step, so it should still
  pass.
- **Glossy-highlight test:** the reflected direction is the key. The environment
  radiance there falls by 0.3, which becomes about 0.012 linear after the 0.04
  Fresnel term. The capture is 251 now and must stay between 245 and 255, so it should
  still pass.

These are predictions. Astra's GPU run is the evidence.

## Captures that need regeneration

These captures are lit by the default studio. All are in `artifacts/environment-initial/`:

- `studio-metal-smooth.png`, `studio-metal-satin.png`, `studio-metal-rough.png`,
  `studio-metal-matte.png`
- `studio-dielectric-smooth.png`, `studio-dielectric-satin.png`,
  `studio-dielectric-rough.png`, `studio-dielectric-matte.png`
- `studio-dark-smooth.png`, `studio-dark-satin.png`, `studio-dark-rough.png`,
  `studio-dark-matte.png`
- `dark-fill-10.png`, `dark-fill-18.png`, `dark-fill-50.png`
- `hdr-glossy-highlight.png`
- `base-color-texture.png`, `reimported-blue-texture.png`
- `metallic-roughness-map.png`, `lit-double-sided-back.png`, `normal-map.png`,
  `varying-normal-map.png`, `occlusion-map.png`

These captures need no regeneration:

- **Authored-environment captures:** every `environment-*.png` uses an authored map,
  and `environment-disabled.png` uses zero intensity.
- **Emissive captures:** the emissive helper's black, fully metallic base color has
  zero reflectance. These are `opaque-alpha-zero`, `mask-discarded`,
  `reflected-single-sided`, `double-sided-back`, `layered-transparency`,
  `opaque-between-transparent`, `hdr-transparent-over-black`, `emissive-map`,
  `sixteen-bit-color`, `linear-filter` and `minified-color-mips`.

All of these run under `INCANT_MATERIAL_EVIDENCE`. The command below follows the CI and desktop
workflow invocation. I did not run it.

```
INCANT_MATERIAL_EVIDENCE=artifacts/environment-initial cargo test -p incant_render --release --locked --test model_gpu -- --ignored
```

## Screenshots

- **Reviewed:** the 41 PNGs in `artifacts/environment-initial/`, rendered by Astra
  before this change.
- **Made by me:** two 3× enlargements for edge inspection,
  `target/env_check/metal-smooth-x4.png` and `target/env_check/dark-smooth-x4.png`.
  They live in the ignored `target/` folder, are not committed and are not evidence of
  the new tuning.

I made no new renders and claim no visual-regression result.

## Pending visual-review scope

- **Regenerated studio set:** confirm that the fill now sits clearly below the key on
  smooth and satin metal. Confirm that unlit sides at about 0.25 still read and that
  nothing else moved.
- **Native integration captures:** these are not reviewed and not approved.
- **Authored environments in the editor:** these are not reviewed and not approved.

## Questions for Astra

- **Rotation captures:** is the faint vertical band on the authored rotation quads the
  map's own split, or a sampling or seam issue in the authored-texture path?
- **Specular occlusion:** without a specular occlusion or horizon-fade term, concave or
  downward-facing glossy surfaces can reflect the floor and fill where geometry should
  block them. The spheres cannot show this. A later fixture with concavity would.
- **Silhouette antialiasing:** sphere edges alias. Is preview MSAA planned? It is
  outside this packet.
