# 0016 Environment lighting: default studio look-dev — result

## Final label verdict

**Approved: the Inspector label fix, natively confirmed.** No styling defect remains in
the reviewed native scope. No phase gate is approved or claimed.

**Packet:** the final label-fix recapture revision. It carries no new typed director
feedback, and no further design expansion was requested. Astra reports:

- The native app was rebuilt and reopened from `441b3a6` on `impl/environment-lighting`.
- All 282 UI tests pass, and the build passes.
- Source lighting is unchanged.

I did not rerun any of these.

**Source identity:** `a756fa9` is not a direct ancestor of `441b3a6`, so the fix was
carried over by rebase or cherry-pick. A content comparison between this branch and
`441b3a6` is empty for three areas:

- `editor/ui/src/styles/app.css`, which contains the two-line label clamp;
- `editor/ui/DESIGN.md`;
- `crates/incant_render/src`.

The captured build therefore contains exactly the reviewed label fix and the reviewed
lighting source.

**Captures reviewed at native resolution:** these are real CUA captures. Like 01 to 07,
they are JPEGs and stay in the ignored `artifacts/` folder.

| Capture | Size | Result |
|---|---|---|
| `08-label-fixed-wide.jpg` | 1440×900 | Pass |
| `09-label-fixed-minimum.jpg` | 1000×650 | Pass |

- **Label:** "Rotation degrees" now reads in full as "Rotation" over "degrees", with no
  ellipsis. Its value well stays vertically centered against the two-line label.
- **Other rows:** Texture and Intensity stay single-line and look the same as in 01 and
  07. The row gap is preserved, and nothing overlaps.
- **Minimum size:** at 1000×650 the Inspector still fits all three fields above the Agent
  section without scrolling.
- **Value wells:** they keep their full 66% column. The Texture ID is truncated in mono
  with its hover title, as intended.
- **Viewport:** the sphere matches 01 and 07 at the same sizes. It stays round and
  centered, so the label change did not affect rendering.

**Limitations, kept precise:**

- **Two-line limit:** wrapping is verified natively for one real two-line label only.
  Labels longer than two lines, and labels in other components, were checked only in
  the headless Chrome render of the stylesheet, with a system font. They were not
  checked natively.
- **Not recaptured natively after the fix:** the undo, redo, invalid-source and repair
  states of captures 02 to 06. Their native verdict comes from the `42a991d` build.
  The label change is CSS only and does not touch that behavior.
- **Account chip:** the captures show the saved account in the titlebar. No account
  identifier is recorded here, and the images are not committed.
- **Rendering limitations unchanged:** there is no geometric visibility or shadow term,
  only the occlusion texture applies, and temporal antialiasing is not implemented.
  Those limitations are listed below.

**Model and transport:** Claude Opus 5.5, model ID `claude-opus-5-5`. The packet
states ACP protocol 1. The session runs as a Claude Code agent launched by the handoff
harness in this worktree, under `bypassPermissions`. The session cannot independently
confirm the ACP hop. No model substitution was made.

**Changed paths in this pass:** this file and the current packet,
`handoffs/0016-environment-lighting/brief.md`, which is committed as requested.

## Final native verdict

**Native integration approved.** The approval covers rendering, retention, recovery
and viewport sizing in the CUA-native captures in `artifacts/environment-native/`.
Astra recorded them from PR16 source commit `42a991d`, built independently.

- **One styling defect fixed:** Inspector field labels were clipped. The fix is now
  natively confirmed by captures 08 and 09, in the final label verdict above.
- **No phase gate:** none is approved or claimed.
- **Headless verdict unchanged:** the headless and command-line approval below still
  stands.

**Model and transport:** Claude Opus 5.5, model ID `claude-opus-5-5`. The packet
states ACP protocol 1. This session runs as a Claude Code agent launched by the
handoff harness in this worktree, under director-authorized `bypassPermissions`. The
session cannot independently confirm the ACP hop. No model substitution was made.

**Packet:** the final native review revision. It carries no new typed director
feedback. Astra's routine notes are acknowledged:

- **Capture indicator:** the macOS capture indicator overlaps the window corner. It is
  an OS overlay and was not judged as app chrome.
- **Saved account:** it was restored without a prompt. No account identifier is
  recorded here. The captures show the account chip in the titlebar. They stay in the
  ignored `artifacts/` folder and are not committed.
- **Fixture journal path:** Astra corrected a setup mistake in the disposable fixture.
  This is not an application change, and no validation was bypassed.

**Source identity:** `git diff 42a991d HEAD -- crates` is empty. The captured build
therefore used the same renderer and environment source as the approved headless
review. The two commits differ only in documentation and evidence files.

### Captures reviewed at native resolution

Captures 01 to 06 are 1440×900 JPEGs. `07-minimum.jpg` is 1000×650, and
`resize-check.jpg` is 1000×900.

| Capture | What it shows | Verdict |
|---|---|---|
| 01 authored at yaw 0 | The metallic sphere reflects the authored red/blue map. It matches the headless authored capture: a red reflection lobe, a blue rim and the white key highlight. The Inspector shows Texture, Intensity of 1 and Rotation of 0. There are no problems. | Pass |
| 02 source changed | The watched source switches to uniform green. The sphere is evenly green with only the key highlight, as a uniform environment should look. Undo becomes available, and there are no problems. | Pass |
| 03 undo | The original red/blue environment returns, identical in look to 01. | Pass |
| 04 redo | Green returns, identical to 02. | Pass |
| 05 invalid source | Problems shows the error, quoted in full below. The status bar shows one error, and the sphere keeps the last valid green lighting. | Pass |
| 06 source repaired | The original environment returns. Problems and the status bar clear. | Pass |
| 07 minimum 1000×650 | All panels stay visible. The viewport shrinks and the sphere stays round and centered. The output dock, the Inspector and the Agent composer fit without overlap. | Pass |
| resize 1000×900 | The sphere is round and centered at a different aspect ratio. | Pass |

The error in capture 05 reads:

```
could not cook environment.png: invalid asset: The image format could not be determined
```

**Retention and recovery:** the viewport never goes blank or falls back to the
default studio during the invalid-source state. Valid lighting is retained, the error
is visible in two places, and repair clears it. The error text is clear, and the
offending file name sits right-aligned in mono, like other problems.

**Viewport sizing:** the sphere is centered in the viewport in every capture. At
1440×900 the viewport center is about x 677, y 352. At 1000×650 it is about x 457,
y 227. At 1000×900 it is about x 457, y 352. There is no stretching, letterboxing or
offset between the native surface and the web viewport hole.

**Inspector clipping and scrolling:** the Inspector column is a fixed width of about
332 px at every window size. In every capture the "Rotation degrees" label was clipped
to "Rotation degre...". The full name was available only through the hover title.
That is a concrete legibility defect on the component this handoff adds, so I fixed
it below.

The Texture asset ID is also truncated with an ellipsis. That is the intended behavior
for a mono value well, and the full ID is in its hover title. The three fields fit
without scrolling, even at the minimum window size.

**Colours:** the saturated red, blue and green are numerical test colours, not art
direction. They were judged only for correctness.

### Styling fix: Inspector field labels

- **`editor/ui/src/styles/app.css`:** `.field__label` now clamps to two lines at the
  16 px tight leading, instead of a single-line ellipsis. A third line is the first
  one truncated. Unbreakable words wrap. The 34/66 label/value grid is unchanged, so
  value wells lose no width. The hover title and the real `<label>` association are
  unchanged.
- **`editor/ui/DESIGN.md`:** the Inspector entry now records the wrapping rule.

**Verification:** I rendered the real `tokens.css` and before/after `app.css` with
FieldView's markup at the 332 px Inspector width, using headless Chrome at 2× scale.
This is a browser render of the stylesheet, not a native capture. The render files are
`target/env_check/ui/before.png` and `after.png`, in an ignored folder and not
committed.

- "Rotation degrees" now reads in full on two lines.
- A long multi-word label clamps at two lines with an ellipsis.
- A long single word wraps.
- One-line rows are unchanged.

The Inter web font is not installed in this worktree, so the render used the system UI
font. Inter is about as wide or slightly narrower, so it should fit at least as well.

**Not run:**

- **UI unit tests and typecheck:** not run, because `node_modules` is absent here. No
  test references this class's styling.
- **Native recapture:** not done. Native capture is Astra's role.

**Native recapture needed (resolved by captures 08 and 09):** a native capture of the Environment Inspector,
like 01, plus the minimum-size capture 07 are needed to confirm the wrapped label in
the app. The renderer and the viewport are untouched, so no other capture needs
regeneration.

**Optional data request for Astra:** the schema could give this field
`title: "Rotation"` with `x-incant-unit: "°"`. FieldView already renders units inside
the value well, which would read better than a wrapped label. Texture could also use
`format: "asset-ref"` to get the link icon. These are schema bindings, so I left them
untouched.

### Native limitations, recorded and not approved

- **No visibility or shadow term:** geometric visibility and shadows are not
  implemented, and only the occlusion texture applies.
- **No temporal antialiasing:** it is still planned in PLAN.md. The sphere silhouettes
  alias natively too.
- **No authoring in the Inspector:** it is read-only. The environment is changed
  through the source and command bus, not edited in the Inspector.
- **Default studio not shown natively:** this capture sequence shows only authored
  environments. The default studio is covered by the headless command-line capture,
  which renders the same source.

## Final headless verdict

**Headless verdict (still current): approved for headless rendering: the default studio environment, as rendered in
`artifacts/environment-final/`.** The fill and key correction is confirmed in the
pixels. No look defect remains, and no source changes were needed in this pass.

- **Native integration:** at the time of this headless pass, no native images existed
  because the Mac was locked. That has since been superseded by the final native
  verdict above.
- **Not approved: any phase gate.** None is claimed.
- **Scope:** this verdict covers the preview appearance only. It does not approve the
  limitations listed below.

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Transport:** a Claude Code agent session launched by the handoff harness in this
  worktree, under director-authorized `bypassPermissions`. The session cannot
  independently confirm the ACP hop. No model substitution was made.
- **Changed path in this pass:** this file only.

## Final review of `artifacts/environment-final/`

**Packet:** the confirmation revision. It carries no new typed director feedback. It
is Astra's routine follow-up. It reports these results, which I did not rerun:

- All 13 GPU tests pass for the model, editor and headless crates.
- The two unchanged display tests passed earlier.
- The native release build passes.

I made no build and no native capture. I edited no implementation, test or UI code.
I inspected the studio, command-line and authored captures at native resolution. I
measured all of them with the stdlib PNG scripts in the ignored `target/env_check/`
folder.

**Fill and key correction.** The 90th-percentile brightness of the fill reflection
region moved as predicted:

| Capture | Initial fill | Final fill | Predicted | Key peak |
|---|---|---|---|---|
| Smooth metal | 227 | 200 | about 202 | 255, unchanged |
| Satin metal | 215 | 190 | n/a | 255, unchanged |
| Smooth dark | 72 | 66 | n/a | 255, unchanged |
| Smooth dielectric | 187 | 181 | n/a | 255, unchanged |

On smooth and satin metal, the key now reads as the clearly dominant panel and the
fill as a secondary one. Satin metal's clipped core shrank from 529 to 507 pixels.
That is a direct-light highlight and is acceptable.

**Other checks:**

- **Neutrality:** inside every sphere, the largest channel spread is 1 sRGB level, or 2
  on metal. This is unchanged.
- **Roughness progression:** it is still monotonic on all three materials.
- **Seams:** none appear.
- **Diffuse:** sphere means fell by 1 to 4 levels, as expected from the dimmer fill.

**Prior-material captures:** these are center pixels, initial to final.

| Capture | Initial | Final | My prediction |
|---|---|---|---|
| dark-fill-10 | 62 | 58 | 58 |
| dark-fill-18 | 78 | 73 | 74 |
| dark-fill-50 | 120 | 113 | 112 |
| hdr-glossy-highlight | 251 | 251 | in 245 to 255 |
| normal-map | 195 | 187 | not predicted |
| base-color-texture and metallic-roughness-map | | 1 to 2 levels lower | not predicted |

All of these are consistent with the dimmer fill and key panel.

**New captures:**

- **Command-line sphere in the default studio:** `cli-default-studio.png` differs from
  `studio-metal-satin.png` by at most 1 level per channel. The command-line path
  therefore renders the same studio as the test fixture.
- **White furnace:** `environment-white-furnace.png` is a uniform 188 on the surface,
  with no variation across the quad. That matches its asserted value.
- **Authored red/blue sphere environments:** the authored, rotated, undo, redo,
  source-free and native-ready captures behave as described. Byte-identical files pair
  up as expected: authored, undo and native-ready match, and rotated, redo and
  source-free match. The red/blue boundary is a soft curved arc on the sphere. It
  follows reflection vectors, and the colors swap under rotation.

**Fixture boundary versus cube-face seam:** the authored map has a deliberate sharp
red/blue split at U=0.5 and at its wrap seam. That boundary sits in the map itself,
so it moves with yaw and reflection direction and blurs with roughness.

A cube-face seam would look different:

- it would be fixed to the cubemap axes;
- it would not swap with rotation;
- it would also appear on the default studio spheres, which use the same cubemap path.

None of the twelve studio spheres shows such a line. The faint vertical band on the
authored rotation quads is therefore that fixture boundary, crossed by the slightly
varying reflection vectors across a flat quad. It is not a seam. I consider this
question closed.

**Limitations, recorded and not approved:**

- **No visibility or shadow term:** only the occlusion texture applies. Concave or
  downward-facing glossy surfaces can reflect the distant floor and fill where geometry
  should block them.
- **No temporal antialiasing:** it is still planned in PLAN.md and is not
  implemented. Sphere silhouettes alias in headless captures.
- **Fixed camera and key:** the 0014 camera and key geometry stay as they are, so the
  key lights the visible face almost head-on and dielectric spheres read soft.

**Screenshots reviewed:** the PNGs in `artifacts/environment-final/`. I made no new
renders. No native screenshots exist.

**Pending:** a native integration review of the built app when the Mac is unlocked,
and authored environments inside the editor. Neither is approved.

---

# Historical record: earlier passes

Everything below is preserved as written for the first pass and the
`artifacts/environment-initial/` review. Its "pending regeneration" items are
resolved by the final review above.

## Status at the integrated GPU review (historical)

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

## Questions for Astra (historical, answered in the final review)

- **Rotation captures:** is the faint vertical band on the authored rotation quads the
  map's own split, or a sampling or seam issue in the authored-texture path?
- **Specular occlusion:** without a specular occlusion or horizon-fade term, concave or
  downward-facing glossy surfaces can reflect the floor and fill where geometry should
  block them. The spheres cannot show this. A later fixture with concavity would.
- **Silhouette antialiasing:** sphere edges alias. Is preview MSAA planned? It is
  outside this packet.
