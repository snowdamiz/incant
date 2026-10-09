# 0016 Environment lighting: default studio look-dev — result

## Status

**First pass complete: the source function exists and its numeric range is verified.
No pixels are approved.** No captures exist yet, so this is a design and numeric
deliverable only. Visual approval waits for Astra's integrated captures. No phase gate
is approved or claimed.

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- Packet: `handoffs/0016-environment-lighting/brief.md`. It has no priority revisions
  and no new director feedback. It is a routine increment under the standing
  continuation instruction. No interrupted attempt existed in this worktree.

## Changed paths

- `crates/incant_render/src/preview_environment.rs` (new). It defines
  `pub(crate) fn radiance(direction: [f32; 3]) -> [f32; 3]` and
  `pub(crate) const MAX_RADIANCE: f32 = 16.0`, plus private helpers. It uses scalar
  f32 only, with no glam, dependencies, BRDF math, GPU bindings or tone mapping.
- `handoffs/0016-environment-lighting/result.md` (this file).

**Not changed:** `lib.rs` does not declare the module yet. Adding an unused
`pub(crate)` function there would raise a dead-code warning before Astra's sampler
uses it. Astra should add `mod preview_environment;` together with the cubemap
sampling code. `studio.rs` still holds `DIFFUSE_ENVIRONMENT = 0.3`. Astra removes it
when the convolution replaces it. No tests, UI or project files were touched.

## Intent

A neutral, restrained photographic cove, built from original procedural shapes:

- **Backdrop.** The upper hemisphere is nearly flat, with radiance 0.29 at the horizon
  and a slightly cooler 0.27 at the zenith. The floor is 0.15 and very slightly warm.
  A smoothstep horizon band about 14 degrees wide joins them. Metals therefore show a
  clear sky/ground split, and diffuse shapes read as grounded.
- **Key softbox.** A rectangular panel of radiance 1.7 sits around the directional key
  [1,2,3]. Reflected and direct highlights therefore agree. From the camera at
  [6,5,9], the key lies about 13 degrees off the view direction. Its reflection lands
  near the center of spheres, where roughness blur is easiest to judge.
- **Fill card.** A broad, dim panel of radiance 0.7 sits on the camera's right,
  opposite the key. It lifts the shadow side and gives metals a second, larger and
  softer reflection.
- **Rim strip.** A long, short horizontal strip of radiance 1.6 sits behind and above
  the subject. It produces edge highlights at grazing angles.
- **Panel shape.** Panels are planar rectangles projected gnomonically, with smoothstep
  edges and a mild center hotspot of 20%. Rectangles make roughness legible as edge
  blur and corner rounding, which round blobs do not show as well.

All channels stay within 3% relative chroma, so neutral materials pick up no visible
cast. Degenerate input never fails. NaN, infinity or a zero vector returns the horizon
value. Huge or tiny vectors are prescaled before normalization so they cannot overflow.

## Numerical checks

The checks used a scratch harness, kept untracked under the ignored `target/env_check/`.
It mounts the real source file through `#[path]` and builds with the pinned 1.99.0
toolchain. This is an independent numeric check, not a crate test. The packet asks
for no test changes.

```
rustc --edition 2024 -O -o target/env_check/check target/env_check/main.rs && target/env_check/check
rustfmt --edition 2024 --check crates/incant_render/src/preview_environment.rs   # clean
clippy-driver --crate-type lib target/env_check/lib.rs -W clippy::all -W clippy::pedantic   # no warnings
```

Range over 2,000,000 Fibonacci-sphere directions:

| Check | Result |
|---|---|
| Minimum channel | 0.146 |
| Maximum channel | 1.98, against a limit of 16 |
| Non-finite, negative or over-limit values | 0 |
| Largest relative chroma | 0.027 |
| Mean radiance | 0.323 per channel |
| NaN, infinity, zero, 1e30 and 1e-30 inputs | finite and in range |

Diffuse budget is the cosine-weighted mean radiance, which is irradiance divided by pi.
This is the value that replaces the old constant fill of 0.3 in `albedo * environment`.
The table shows the green channel. The other channels differ by at most 0.004.

| Normal | Value |
|---|---|
| Mean over all normals | 0.323 |
| Up | 0.450 |
| Toward the key | 0.414 |
| Toward the camera | 0.423 |
| Camera right, fill side | 0.368 |
| Camera left | 0.269 |
| Away from the key | 0.252 |
| Down | 0.150 |

Unlit sides land a little below the old 0.3, and tops land above it. That adds shape
cues while keeping the average. The key panel slightly reinforces the directional
key's diffuse, which is deliberate. White tops get roughly 0.15 more than before. Under
the 0015 curve, that moves fully lit white tops further onto the shoulder, which is
intended to absorb it. This must be checked in captures.

Cubemap resolution: the largest luminance step between neighboring texels on all six
128 px faces is 0.24, at the key panel edge. Edge ramps span about 10 degrees or more,
which is about 14 px. The narrowest panel is about 25 degrees tall. No feature is a
one-texel edge, so the shapes survive the base face and blur progressively through the
roughness levels.

Tuning history: the first values gave 0.22 away from the key and 0.52 facing up, with
a 10% warm floor. They also had an overflow on huge inputs. The backdrop, key panel and
tints were rebalanced, and normalization was prescaled, before this commit.

## Screenshots

None. No captures exist, and the packet assigns native capture to Astra only. Nothing
here was rendered, and no visual-regression result is claimed.

## Pending visual-review scope

These require Astra's captures before any approval:

- Metal and dielectric spheres across the roughness range. Check that the key, fill and
  rim reflections read as distinct panels when smooth and blur evenly when rough. Check
  for seams or texel stepping on the 128 px base level.
- The horizon split on mirror metals, and whether 0.15 floor reads as grounded rather
  than muddy.
- Diffuse readability against the 0014 and 0015 references. Check that white tops on
  the tone-map shoulder hold detail and that shadow sides at about 0.25 are not too
  dark.
- Neutral materials for any cast from the tints of 3% or less after convolution.
- Authored environments and native integration, as the packet lists.

## Open questions

- Whether a specular occlusion or horizon-fade term is planned. Without one, the floor
  and fill panel can show through on concave or downward-facing glossy surfaces.
- Whether the direct key should eventually come only from the environment panel, to
  avoid the small diffuse double count. This first pass keeps both.
