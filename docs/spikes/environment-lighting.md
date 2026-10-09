# Distant environment lighting

Phase 1 increment, 2026-10-09. Real diffuse and specular image-based lighting is
implemented for imported glTF materials. This does not complete the render graph,
production lighting, or a phase gate.

## Authoring and lifetime

`EnvironmentLight` is a registered, schema-derived component with a stable texture
asset ID, intensity (0–100), and rotation about world +Y (−360 to 360 degrees).
It uses the existing component commands, Inspector, agent tools and durable
Undo/Redo. Referenced assets cannot be removed or reinterpreted as normal maps.
Only one global environment is accepted across the currently loaded project;
active-scene selection is not implemented. Entity transforms do not rotate the
environment; the component's explicit world yaw does. A light-only entity does
not create a diagnostic cube.

The texture is imported normally as Color or Linear. PNG/JPEG and EXR use the
existing cooked KTX2 cache. The renderer accepts 2:1 equirectangular images within
the importer's four-megatexel budget and a 4096-axis ceiling. RGB radiance must
be nonnegative, finite, and at most 65504. Alpha is ignored. +Y is the north pole,
+X the image center, +Z three-quarter U; rows run north to south. Positive yaw
rotates the environment around +Y, implemented by inverse rotation of lookup
vectors. The background remains the editor's display backdrop.

Scene preparation requires the currently granted asset and exact document
fingerprint before accessing the GPU cache. A prepared scene retains its immutable
GPU resources and decoded source. Reimport publishes a replacement; older scenes
and Undo retain the old appearance. Missing, stale, malformed or unsupported
sources return typed errors. The editor's existing render worker retains its last
valid scene on such errors. No project source read occurs during drawing.

## Integration

The implementation follows the split-sum equations and prefiltered importance
sampling described in [Filament sections 5.3 and 9.2–9.5](https://google.github.io/filament/main/filament.html).
It is independently written from the equations, not copied shader code.

- 128-pixel cube faces, eight roughness levels; 16-pixel diffuse faces.
- 256 Hammersley samples per filtered texel. Source mip selection uses sample
  solid angle; CPU bilinear taps cross cubemap edges. Large input images use their
  cooked color-correct low-pass mips before projection.
- Cosine convolution stores irradiance divided by pi. GGX specular prefiltering
  assumes normal equals view direction. A 64×64 two-channel lookup integrates
  correlated Smith visibility and Schlick Fresnel with 1024 samples per texel.
- The material shader combines environment diffuse and specular terms with
  metallic/roughness factors, normal maps and occlusion. It reserves the remaining
  single-scattering energy for diffuse. All lighting precedes the existing HDR
  output transform. The fixed directional preview key remains separate.
- Generated maps use RGBA16Float; the lookup uses RG16Float. GPU cache entries are
  weak references; published scenes retain versions. Default studio and lookup
  CPU data are generated once per process, and GPU data once per renderer.

With no authored component, Claude's original procedural studio is convolved by
exactly the same path. Handoff 0016 owns its look development. The revised key and
fill panel intensities preserve a clear hierarchy under the HDR output shoulder.

## Verification and current limits

- 130 workspace Rust tests passed, followed by one additional passing low-pass
  source test (131 behavior tests in the current tree). Independent uniform-angle
  quadrature checks the BRDF integral; other tests cover cube axes/edges, constant
  radiance, HDR/sRGB/linear decode, invalid radiance, filter broadening and document
  transaction semantics. All lookup values are finite, nonnegative and bounded.
- Fifteen explicit GPU checks pass, including the final look-dev retune
  (13 model/editor/headless checks rerun; two unchanged display checks passed).
  The final captures and rerun are recorded in the evidence ledger. Actual environment
  checks cover color spaces, positive/negative yaw, intensity, roughness,
  constant-environment white-furnace energy, retained versions and source removal.
- 282 UI/bridge tests, UI build, SDK typecheck, generated files, Python tools,
  workspace Clippy and the native release build pass.
- A disposable project was created/imported through the public CLI and bound
  through the command bus. Public screenshot commands render 3968 sphere
  triangles in one draw. Rotation changes pixels; Undo and Redo restore exact
  image hashes; deleting the sources preserves the cooked rendering. Six separate
  CLI processes completed in 0.32–0.79 seconds each on Apple M5 Pro, including
  device creation, asset loading, filtering, drawing and PNG output. This is a
  local fixture measurement, not a frame-time or platform performance gate.
- Claude Opus 5.5 through ACP approved the final headless captures in `e192fe8`,
  confirming key/fill hierarchy, neutral colors, smooth roughness progression and
  seam-free studio reflections. CLI/default-studio and authored-map captures pass.
- Native application verification is pending: CUA reports the Mac is locked.
  The running prior build was left intact. Claude reviews the saved GPU images;
  this is not a substitute for native integration evidence.

Filtering is synchronous during scene preparation on the existing render worker;
there is no persistent prefiltered-environment disk cache, background job progress
or cancellation yet. Fixed resolution, finite sampling, box source mips and the
split-sum view assumption limit sharp/high-frequency lighting. Single scattering
loses energy on rough metals; multiscattering compensation is not implemented.
There are no local reflection probes, visibility rays, parallax correction,
shadows, skybox, exposure controls, authored direct lights, clustered culling,
SSAO, bloom or temporal antialiasing. Reflected studio floor is distant radiance,
not geometry. Native and final visual review remain explicit before merge.
