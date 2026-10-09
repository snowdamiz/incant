# Authored lights and clustered forward shading

Phase 1 increment, 2026-10-09. This implements real GPU light assignment and
punctual shading. It does not complete the render graph, production lighting,
mobile tier, or a phase gate. Environment lighting is the prerequisite in PR #16.

## Authoring

`DirectionalLight`, `PointLight`, and `SpotLight` are registered typed components
with derived JSON schemas and TypeScript bindings. Every project edit uses the
existing atomic command bus, including CLI/agent edits and durable Undo/Redo.
The current Inspector displays fields read-only; interactive field editing is a
separate editor requirement, not implemented by this increment.

All three use linear RGB multipliers from 0 to 1. Intensity is illuminance in lux
for directional lights and luminous intensity in candela for points/spots, from
0 to 1,000,000. Local lights have an explicit finite range from 0.001 to 10,000
meters. Spot angles require 0 ≤ inner < outer ≤ 89.9 degrees, with outer ≥ 0.1.
Only one punctual-light component is permitted per entity. World transforms
inherit the entity hierarchy; lights emit along transformed local −Z. Scaling
changes position/orientation through the hierarchy, but not intensity or range.
These conventions follow the [Khronos punctual light specification](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_lights_punctual/README.md).
Importing that glTF extension remains unsupported and fails explicitly.

Scene preparation rejects nonfinite/unrepresentable light transforms, spot
cones that collapse at GPU float precision, more than 16 directional lights, or
more than 4,096 local lights. It publishes immutable GPU light records with the
scene. Existing prepared scenes retain their original lighting after edits.
Light-only entities do not become diagnostic cubes. No authored punctual lights
means the existing studio preview key; an explicitly authored zero-intensity
light disables that fallback. Environment illumination remains independent.

The initial list-based assignment described below is superseded by
[exact membership masks](light-masks.md), with a separate all-light diagnostic
reference and paired performance evidence. This document retains the initial
PR #17 implementation and review record.

## Assignment and shading

The camera view is divided into 64-pixel tiles and 24 logarithmic depth slices
from 0.1 to 1,000, matching the current fixed preview projection. One GPU compute
workgroup tests local-light range spheres against the six cluster planes.
Spot bounds are deliberately conservative spheres. The position-based approach
is described in [Olsson, Billeter and Assarsson, 2012](https://research.chalmers.se/en/publication/161725).
Our implementation does not include their optional normal-based rejection.

Each cluster stores at most 64 sorted indices plus its actual intersection count.
Sorting removes nondeterministic atomic insertion order from floating-point
lighting sums. On overflow, fragments evaluate every local light; illumination
is never silently truncated. Directional lights are evaluated separately for
every material fragment. All geometry, including blended materials, uses the
same cluster lookup. Lookup subtracts the viewport origin before tile selection.

Grid buffers are retained for one size. Commands retain replaced resources and
use immutable uniforms, so resize and out-of-encoding-order submission remain
valid. Each dispatch overwrites all cluster counts, including empty clusters.
Without local lights, compute is skipped and shaders never read the lists. The
index allocation is bounded to 131,072 clusters (32 MiB) and checked against the
actual device buffer/binding limits. Unsupported grid sizes return a typed error.

Punctual light evaluation uses the existing GGX/Smith/Schlick material model
before HDR display conversion. The inverse-square denominator clamps distance
to one centimeter to avoid a singularity. Finite-range windowing and the squared
angular spot ramp are documented in [Filament's attenuation section](https://google.github.io/filament/Filament.html).
Claude's review identified a visible rim in the initial unsquared quartic range
window. The reviewed correction squares the window, giving zero value and zero
slope at the cutoff. A real GPU regression samples 98% and 99% of the radius
and confirms irradiance decreases quadratically near the boundary. All physical
and spatial-oracle checks pass with this correction. Tiny fixtures with lights less than 1 cm from the surface
intentionally expose the finite-source plateau.

## Evidence and limits

Final local validation passes 135 workspace Rust behavior tests, 24 explicit
GPU checks, 283 UI/bridge tests, workspace Clippy and the native release build.
Generated files, SDK typechecking and five Python tests also pass. The new
range-edge regression fails on the initial shader and passes on the correction.
A pre-existing account-dialog test race was corrected: it now waits for the
pending request to finish before clicking the next action. No authentication
behavior changed.

The GPU checks cover inverse-square falloff, spot direction and penumbra,
directional color, range rejection, transform inheritance, retained versions,
64/65/128-light energy, stable repeated renders, all 24 depth slices, sparse
culling, clearing stale counts, offset viewports and queued resize commands.
Spatial lighting, 96 distinct overlapping lights, and 24 perspective-scaled
depth-boundary patches match an all-local-light GPU oracle exactly at even and
odd sizes. The oracle isolates assignment correctness; it shares shading math,
so it cannot independently validate the BRDF. Separate physical checks serve
that purpose. The sparse readback fixture selects 24 memberships in 360 clusters
instead of evaluating all 24 lights in every cluster; this is not a frame-time
benchmark or a performance gate.

A disposable public-CLI project renders 3,968 sphere triangles in one draw.
Intensity edits change the image; Undo/Redo restore exact hashes. Invalid range
edits leave the document, revision and history unchanged. Durable journal reopen
and source-independent rendering pass. Six separate screenshot processes took
0.32–1.60 seconds each on Apple M5 Pro, including initialization, asset loading,
rendering and PNG writing; these are not GPU frame timings.

Native captures verify point intensity Undo/Redo, all new read-only fields,
RGB channel names, unit annotations and viewport rendering at 1440×900 and
1000×650. Saved authentication
survives the rebuild without interaction. Claude owns native and headless pixel
review. Claude approved all 45 final GPU captures and seven native captures in
`301d389`, including units, RGB labels, native intensity Undo/Redo and scroll
access to the lower Inspector fields. Exact revisions and image hashes are in
[the evidence ledger](evidence/clustered-lighting-2026-10-09.json).

No shadowing, light occlusion, area/IES lights, exposure UI, gizmos, authored
camera selection, render-graph scheduler, SSAO, bloom, antialiasing, or mobile
quality tiers are supplied here. Diagnostic fallback cubes still use their
separate preview shader; authored light shading applies to imported materials.
Cluster building tests every local light per cluster: dense scenes and overflow
can be expensive. There is no hierarchical light binning, GPU timing benchmark,
per-device tuning, or completed performance gate yet.


## Local frame timing

`./tools/cargo run -p incant_render --example frame_benchmark --release --locked
-- /absolute/project.incant.json` reads a cooked project without changing it.
It measures CPU encoding, submission and GPU completion at 1920×1080 after five
warm-up frames, for 30 samples. Loading, readback, PNG encoding, presentation and
simulation are excluded. These are fenced wall times, not GPU timestamps.

On this Apple M5 Pro, a 3,968-triangle imported sphere with a black environment
and a 0.7-meter-spaced point-light lattice (2-meter light ranges) measured:

| Local lights | Median ms | p95 ms |
| --- | ---: | ---: |
| 0 | 1.385 | 1.458 |
| 64 | 2.715 | 3.278 |
| 256 | 2.659 | 2.784 |
| 1024 | 7.765 | 9.060 |
| 4096 | 28.050 | 31.856 |

The 4,096-light case exceeds a 16.7 ms frame budget even before gameplay or
presentation. The overlap/overflow path needs optimization for dense scenes.
These small synthetic scenes and this faster Mac do not satisfy PLAN.md's Core
Sample performance gate on its named devices. Raw sorted samples are retained
in the evidence ledger. The desktop app was closed during these measurements.

Windows hosted validation on `8b94eff` ended in a native process access violation
while 18 renderer GPU tests ran concurrently; it did not report an assertion
failure. Follow-up `788da07` serializes native GPU tests and exposes individual
case output. All 18 renderer cases pass locally with that command. The exact-head
Windows rerun remains required before merging PR #17; no driver root cause or
Windows success is inferred from the local result.
