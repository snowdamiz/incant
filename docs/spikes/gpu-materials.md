# Imported GPU material previews

Phase 1 increment, 2026-10-09. The indexed model path now binds retained textures,
samplers and material uniforms from the cooked glTF cache. Each published model
version owns its resources, so a failed replacement leaves the old scene usable,
and retained scenes keep their original textures through reimport and Undo.

The loader resolves raw material metadata into checked factors and five role-bound
maps. Base color and emissive use sRGB image formats; numeric MR, normal and
occlusion data remain linear. Sixteen-bit color PNGs retain linear float CPU data
and upload to filterable RGBA16Float; conversion that exceeds its finite range
fails explicitly. Sampler wrap, magnification, minification and mip selection
are preserved. This follows the core
[Khronos glTF 2.0 material specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#materials).

The shader evaluates metallic/roughness GGX, correlated Smith visibility and
Schlick Fresnel under fixed preview illumination, tangent-space normals and
emissive. AO affects the diffuse preview environment. Opaque and mask draws write
depth; blended instances draw afterward, sorted by primitive-center view depth.
Reflections split instance batches to preserve winding, tangent handedness and
back-face culling. Double-sided back faces reverse the final shading normal.
Transformed bounds outside finite GPU coordinates fail before scene publication.

## Verification so far

- 119 workspace Rust behavior tests pass, including checked material defaults,
  role/color-space resolution, invalid factors, extensions and unresolved maps.
- Seven real GPU tests pass on Apple M5 Pro: the previous geometry/editor/playback
  cases plus four material cases. These check sRGB map/factor equivalence, MR G/B
  channels, normal scale, AO strength, emissive round trips, 16-bit image upload,
  nearest/linear filtering, clamp/repeat/mirror addressing and color-correct mips.
  Alpha tests cover opaque alpha zero, mask discard and cutoff equality, layered
  transparent order, intervening opaque depth, reflected winding and double sides.
  Source removal and a retained old texture version remain renderable after reimport.
- Workspace Clippy, formatting, all 282 UI/bridge tests and the UI build pass.
  Clippy also passes after the final additional GPU assertions.
- Native build `43f0d3f` opened a disposable model project created through CLI
  init/import and command-bus RPC. External texture writes reimport automatically;
  Undo restores the original viewport pixels and stays restored through polling;
  Redo restores the new pixels. A malformed image adds one source diagnostic and
  leaves geometry/materials unchanged. Repair restores the original pixels and
  clears diagnostics. Assets use the left sidebar and the main Inspector.
  The app remains Attached at 1440×900 and 1000×650 and restores the saved account.
- Actual headless PNGs are in ignored artifacts/material-review; native CUA JPEGs
  are in artifacts/native-material-review. Only test metadata is committed.
  Claude Opus 5.5 through ACP approved all 19 final headless captures and native
  captures at both sizes in handoff 0014. Its review also confirms the additional
  lit-back-face and varying-normal cases. A follow-up native scroll shows the
  full Inspector content remains reachable at minimum height; Claude corrected
  its initial clipping concern and accepted the independent scroll behavior.
- Claude tuned key radiance to 2.0 and diffuse fill to 0.30, preserving camera and
  direction. These improve preview legibility without changing authored factors.
  Historical geometry evidence remains tied to its original revision; those image
  hashes are not current lighting baselines. The final integrated tree again
  passes all 119 Rust tests, seven explicit GPU tests and workspace Clippy.

## Limits

This is an imported-material preview increment, not the complete Phase 1 render
graph. There are no authored lights, clustered lists, cascaded shadows, specular
IBL, SSAO, bloom, tone mapping, TAA or mobile quality tiers. Rough metals have only
fixed direct illumination; very bright outputs clip. Transparency uses center
sorting and does not solve intersecting surfaces or crossing triangles. The fixed
camera and diagnostic cubes for entities without mesh bindings remain. glTF core
static materials/UV0 are supported; extensions, animation and authored material
asset overrides still fail explicitly. CPU float images retain full precision;
GPU images use half precision. Hosted checks remain pending.
