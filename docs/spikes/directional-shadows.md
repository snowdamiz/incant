# Directional cascaded shadows

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
gates remain open. Claude's initial look-dev accepts the 3×3 comparison filter/bias with the thin-caster
contact leak and distant thin-post aliasing documented in handoff 0021.

Integrated local verification passes 144 ordinary Rust tests, 40 GPU checks, 294 UI
tests, Clippy, generated schemas/SDK and UI production build. New real GPU checks
cover analytical occlusion, caster flags/range, retained scenes, offscreen
casters, four-light capacity, alpha holes/reflection/sidedness, cascade overlap
and distance fade, and reverse submission after scene/asset/source disposal.
The public CLI authors/imports through the shared command bus; light/caster edits,
Undo/Redo, atomic invalid batch rejection, journal reopening and source-free
cached rendering pass. Native verification and measurements are complete; Claude accepted the scoped native and appearance review;
exact-head hosted checks remain pending. No release gate is claimed.

Initial native checks show the enabled Sun's distance as 40 m in the read-only
Inspector, an attached viewport and zero errors before/after output resizing.
Wide (1440×900) and short (1720×669) captures await Claude review. The latter
is an external-monitor quarter window; the actual 1000×650 minimum remains pending. Assets remains a separate left
workspace, with Problems/Console/History in the bottom output. The saved account
restores across the rebuilt app without authentication or Keychain interaction.

Three-trial fenced frame measurements at 1920×1080 on Apple M5 Pro, with five
warm-up and thirty measured frames per trial, give these medians of trial medians:

| Enabled shadow suns | Imported quad casters | Median milliseconds |
| --- | ---: | ---: |
| 0 | 1 | 1.389 |
| 1 | 1 | 1.470 |
| 1 | 64 | 1.479 |
| 4 | 64 | 3.096 |

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
and repeatability; scoped look-dev/native approval is recorded in handoff 0021. The
[evidence ledger](evidence/directional-shadows-2026-10-09.json) retains hashes,
checks, CLI results and raw timing samples.

A live `gpt-6-astra` turn using the saved OAuth account inspected the schema,
proposed one Sun distance edit, read back the result and captured the selected
camera at 640×480. Astra approved that exact patch on a disposable fixture through
the normal CLI approval prompt. An independent full-document comparison confirms
only distance (40→24 m) and the Sun's agent provenance changed; journal reopening
matches the resulting document. All five tools succeeded over six steps, using
14,405 input and 359 output tokens. Two earlier test launches had stdin closed and
correctly denied the mutation; they are not counted as successful edit checks.
No account login or Keychain prompt occurred.

The shadow retention GPU case also reimports wider geometry, confirms a changed
new frame and an exact original frame from the retained scene. This checks
caster-bounds and geometry-version coherence across cache replacement.

Claude recommended separating the shadow lookup offset from the normal-mapped
shading normal. An analytical mirrored-normal fixture reproduces the old defect:
the same geometric boundary changes by up to 12 channel levels. Passing the
unperturbed interpolated surface normal to the shadow lookup fixes that test;
BRDF and environment shading still use the normal map. The full integrated suite passes, and all four original look-dev images remain
pixel-identical. Claude accepted the correction after independently comparing before/after pixels.


Final CUA captures use the actual cooked look-dev geometry in the native editor
at sun elevations 12° and 38°, changed live through Undo and restored with Redo.
The native default preview camera differs from the authored headless camera.
Wide captures measure 1440×900 pixels. Minimum checks on the built-in Retina
display measure 2002×1302 device pixels including the border; the configured
inner minimum is 1000×650 logical points. Enabled distance reads 60 m; omitted
and explicit-null settings read Off with distinct accessible descriptions. The
read-only field receives focus. Assets stays in the left workspace, and the
bottom remains diagnostics only. No native errors or account prompts occurred.
Claude accepted these captures. The thin-wall contact leak and far thin-post
aliasing remain documented quality limits. A final Assets root-label typography
fix passes 294 UI tests and a rebuilt native package; the wide/short review is
accepted, and Claude also accepted the supplemental 2002×1302 Retina minimum
capture after verifying its decoded dimensions.
