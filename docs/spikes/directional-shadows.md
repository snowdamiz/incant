# Directional cascaded shadows — implementation in progress

DirectionalLight now accepts optional `shadows: { distance }`, with distance
validated from 0.01 to 10000 world meters. Omission preserves unshadowed lighting.
Four 1024×1024 depth cascades per enabled sun are fitted to the active perspective
camera, capped by its far plane, stabilized to texels and blended over the final
ten percent of each interval. The last interval fades to unshadowed lighting.
All imported caster bounds contribute to depth fitting, including offscreen
casters. Invalid GPU precision and capacity fail explicitly.

The scheduled shadow pass precedes imported geometry. Up to four enabled suns
use 16 depth layers (64 MiB at the maximum). One cached atlas size is retained;
queued commands retain old views when the enabled-light count changes. Immutable
per-frame camera/cascade uniforms avoid queue-write races. Every sampled layer
is cleared and redrawn within its command buffer before sampling.

MeshRenderer.cast_shadows separates caster and noncaster instances. Material
alpha masks, reflected winding and double-sided settings apply to the depth
pass. BLEND casters currently fail with a typed error before scene publication;
setting cast_shadows=false allows blended geometry to render and receive shadows.
Diagnostic fallback cubes do not cast. Point/spot shadows, adaptive quality,
caster culling, animated/skinned geometry and production game/device performance
gates remain open. The initial 3×3 comparison filter/bias awaits Claude look-dev.

Initial local verification passes 144 ordinary Rust tests, 38 GPU checks, 284 UI
tests, Clippy, generated schemas/SDK and UI production build. New real GPU checks
cover analytical occlusion, caster flags/range, retained scenes, offscreen
casters, four-light capacity, alpha holes/reflection/sidedness, cascade overlap
and distance fade, and reverse submission after scene/asset/source disposal.
The public CLI authors/imports through the shared command bus; light/caster edits,
Undo/Redo, atomic invalid batch rejection, journal reopening and source-free
cached rendering pass. Full evidence, performance, Claude appearance review and
native verification are still being completed. No release gate is claimed.
