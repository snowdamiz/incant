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

Initial native checks show the enabled Sun's distance as 40 m in the read-only
Inspector, an attached viewport and zero errors before/after output resizing.
Wide and minimum captures await Claude review. Assets remains a separate left
workspace, with Problems/Console/History in the bottom output. The saved account
restores across the rebuilt app without authentication or Keychain interaction.

Three-trial fenced frame measurements at 1920×1080 on Apple M5 Pro, with five
warm-up and thirty measured frames per trial, give these medians of trial medians:

| Enabled shadow suns | Imported quad casters | Median milliseconds |
| --- | ---: | ---: |
| 0 | 1 | 1.398 |
| 1 | 1 | 1.479 |
| 1 | 64 | 1.472 |
| 4 | 64 | 3.109 |

These are small analytical fixtures with an authored camera, not game scenes.
They include CPU encoding, queue submission and a GPU completion fence, excluding
asset loading, readback, presentation and simulation. One first-trial single-sun
median was 2.050 ms; raw samples preserve this variation. Disabled-shadow checks
using the same cooked sphere/local-light fixtures measure 1.397→1.405 ms at zero
local lights and 6.504→6.486 ms at 4096, relative to the preceding graph binary.
No performance optimization or release-device gate is inferred.

Across 124 previous unshadowed captures, 122 remain pixel-identical. Two
camera-cluster images differ at one pixel by one red-channel level. That fixture
used random light ULIDs and therefore variable floating-point summation order;
the fixture now uses stable IDs. All 16 resulting camera captures match across
two independent processes. Claude independently verified the original differences
and repeatability; full look-dev approval is still pending. The
[evidence ledger](evidence/directional-shadows-2026-10-09.json) retains hashes,
checks, CLI results and raw timing samples.
