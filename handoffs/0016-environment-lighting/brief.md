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
