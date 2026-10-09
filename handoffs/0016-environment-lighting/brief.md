# Priority: confirm retuned headless captures

Your two panel-intensity changes are integrated. All 13 model/editor/headless GPU
tests pass again; the two unchanged display tests passed earlier. The updated
native release build passes. Every PNG in artifacts/environment-final/ is current,
including all twelve studio spheres, revised prior-material captures, an actual
CLI sphere in the default studio, authored red/blue sphere environments, and the
new constant-environment white-furnace test. Confirm your fill/key correction
from these pixels, and update result.md with a precise final headless verdict.

Native review is pending because CUA reports the director's Mac locked. Do not
claim native integration or phase-gate approval. No native images were captured.
The built app is ready when the Mac is unlocked.

Answers: the authored environment deliberately has a sharp red/blue hemisphere
boundary at U=.5 and at the wrap seam. Its appearance moves with reflection
vectors; +90/-90-degree yaw now have independent GPU assertions, in addition to
180. Please distinguish this fixture boundary from a cube-face seam. No geometric
visibility/shadow term is implemented (occlusion texture only); that limitation is
explicit. Temporal antialiasing is still on PLAN.md and is not yet implemented.

Review/report only unless an actual look defect remains. Do not build or edit
implementation/test/UI code. Preserve the current document of first-pass evidence
as historical, while making the final verdict unambiguous. Built-by: claude.

---

# Priority: integrated GPU pixel review

The first implementation is integrated. Review the actual PNGs in
`artifacts/environment-initial/` in this worktree. Eleven GPU tests pass;
these include twelve 640×480 analytic sphere captures across metal/dielectric/
dark materials and smooth/satin/rough/matte settings, as well as authored
constant/directional environments, retained versions, rotation, disabled
intensity, and the prior material cases. The spheres are numerical geometry
fixtures. Do not treat them as authored game art.

Judge the studio reflection readability, roughness progression, dark albedos,
neutrality, bright highlight rolloff, and seams/artifacts in the actual pixels.
Inspect each relevant image at native resolution. You own look-dev. If adjustment
is necessary, edit only preview_environment.rs and studio.rs; ask Astra in your
report for any correctness/prefilter issue. Do not edit shaders/math/tests/UI.
Refresh the stale studio.rs comments to describe the new environment accurately.
The old constant diffuse fill is gone. Direct key stays separate and includes
your intentionally aligned environment panel. There is no floor geometry or
shadowing: reflected floor is a distant environment, not a visibility surface.
Native integration captures will follow. Do not claim native approval yet.

Update result.md and commit Built-by: claude. Avoid full local builds: current
source, captures and existing numerical evidence suffice for this review. If
changing appearance, list exactly which captures need regeneration. No shell
native capture. Do not touch credentials. Permissive mode remains authorized.

---

# Environment lighting: default studio look-dev

Claude Opus 5.5 via ACP owns the default studio appearance. Astra is implementing
actual diffuse convolution, GGX specular prefiltering, a correlated-Smith BRDF
lookup, immutable GPU environment resources and authored texture bindings.
All scene layers continue through your approved HDR output transform from 0015.
This is a routine next increment under the director's standing continuation
instruction, not new human art direction. No further permission is needed.

First pass: create `crates/incant_render/src/preview_environment.rs`, defining
`pub(crate) fn radiance(direction: [f32; 3]) -> [f32; 3]`. Input is a normalized
world-space direction (+Y up); output is finite, nonnegative linear Rec.709
radiance, with each channel <=16. Use original procedural studio lighting:
neutral, restrained, with broad soft reflection features that make roughness
and metallic surfaces legible. No downloaded HDRI/art or external dependencies.
This is a direction-only infinite studio environment, not scene geometry. The
fixed camera remains [6,5,9] looking at the origin. The current directional key
is [1,2,3] with radiance [2,2,2]. Existing diffuse fill [.3,.3,.3] will be replaced
by the actual environment convolution rather than added again. Aim to preserve
roughly the same diffuse readability without oversaturating neutral materials.
Astra will sample the function to a cubemap and prefilter it; no roughness/BRDF
math, GPU bindings or tone-map code belongs in this file. Modest smooth features
must survive a 128-pixel cubemap face and its roughness levels.

You may use scalar f32 arithmetic or existing glam Vec3. Do not use imagegen,
create new dependencies, implement a camera, change tests, edit project files or
alter UI. No build is required for this initial isolated source function; verify
its numeric range independently if useful. Do not inspect credentials or launch
native capture through shell tools. Only Astra uses CUA for native verification.

Return handoffs/0016-environment-lighting/result.md with exact model/transport,
intent, source changes, numerical range checks and pending visual-review scope.
Commit with Built-by: claude in the final trailer paragraph. Do not approve pixels
until Astra supplies actual captures. Later review will include metal/dielectric
spheres, roughness variations, authored environments and native integration.
Permission mode bypassPermissions is director-authorized.
