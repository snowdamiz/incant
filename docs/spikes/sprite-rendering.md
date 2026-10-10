# Sprite rendering foundation (in progress)

`incant_2d` supplies bounded sprite values and geometry. A SpriteRenderer references
an existing cooked color texture by stable ID and selects an explicit top-left
source rectangle. Local dimensions and pivot define an XY plane; the entity's
inherited Transform positions it in the world. Optional nine-slice borders keep
their source-pixel density, compress proportionally for small targets, and mirror
correctly when asymmetric borders are flipped.

Sprite entities use ordinary component commands, schema validation and generated
TypeScript. They require a Transform; mesh and sprite renderers use separate
entities. Normal/linear data maps cannot masquerade as color textures. Source
rectangles outside the actual cooked image reject render preparation explicitly.
A failed replacement leaves prior prepared scenes valid.

The shared render graph draws sprites after model geometry, before display
composition. Sprites test opaque scene depth but do not write it. Deterministic
painter order uses layer, order, then stable entity ID, independently of Z depth;
only adjacent texture/sampler runs batch, preserving transparent order. Full
interleaving with transparent 3D models is not implemented. Sprites are unlit and
use the existing HDR display transform; lighting and shadows are not provided.

The GPU copy converts the source's level-zero pixels to linear premultiplied
RGBA16Float before filtering. This removes invisible RGB fringes. Region edges
clamp to their texel centers, and sampling explicitly uses level zero so unrelated
source-sheet frames do not bleed through lower mip levels. Runtime source files
are unnecessary. Imported replacements retain old texture and geometry versions.
Cook-time atlas packing and extruded atlas mip levels remain open.

Claude's first Max-effort engine review is preserved in
[handoff 0032](../../handoffs/0032-sprite-rendering/result.md). It exposed nearest
texel ties that duplicated/dropped rows under flips and changed when a source
sheet was resized. The new GPU regression reproduced that defect before the
fix. Nearest sampling now interpolates region-local texels and uses an integer
`textureLoad`, with a derivative-scaled 1/1024-screen-pixel tie tolerance toward
increasing screen coordinate on the dominant derivative axis (X wins equal
slopes). This is a defined sampling rule, not a guarantee of GPU raster precision.
Linear sampling retains its normalized-UV filtering path. The regression passes
eight orientations (including quarter turns and a mirrored parent), scales 1–3,
and sheets of 64 and 80 rows, with every texel represented and resized-sheet
frames byte-identical. The initial correction passed 325 Rust tests, Clippy and
53 GPU checks.

Claude's integrated round-two review accepts the duplicated-texel and sheet-resize
corrections, but reports a magnification regression from the original fixed
1/1024-texel tolerance. A new numerical GPU sweep reproduces exactly 15 failures
among 70 cases before changing the shader: magnifications 1, 2, 3, 4, 5, 6 and 8,
offsets 0, 0.625, 1, 2 and 3 in units of 1/256 screen pixel, unflipped and flipped
on both axes. At 8x the failing widths are 7, 8, 8, 8, 8, 8 and 9 pixels rather
than eight each. Every pixel is compared against the correctly ordered,
independently aligned source palette; horizontal and vertical run widths are
also checked. This is actual GPU readback, not a shader-model-only test.

The old signed-unit bias spans M/1024 screen pixels at magnification M: at 4x it
equals the observed 1/256-pixel raster step and at 8x spans two steps. Using the
signed derivative instead yields 1/(1024 M) texel, or 1/1024 screen pixel at each
tested scale. That is one quarter of this GPU's observed raster step and does
not consume the next non-tie. The corrected shader passes all 70 cases, all 48
existing orientation/scale/sheet-height combinations and all six sprite GPU tests
on Apple M5 Pro/Metal. Scoped all-target renderer Clippy and formatting pass. The
linear branch is unchanged. The test creates no screenshot artifact and does not
approve appearance.

Exact commands, the 15 measured failures, source hashes and the rebuilt release
`incant_headless` hash are recorded in
[magnification evidence](evidence/sprite-magnified-nearest-2026-10-10.json). The
headless executable is ready for a subsequent Claude review; that review and
hosted/cross-adapter checks remain open. This sweep does not establish floating
point behavior at extreme magnification or large local texel coordinates near
the 8192-region bound. No arbitrary texel-space minimum bias was added because
it would restore the demonstrated magnification failure.

The undersized-reimport diagnostic now names the sprite entity, texture ID,
requested source region and actual cooked dimensions through a typed resource
error. Geometry validates before that sprite allocates GPU resources. The GPU
regression shrinks a 2×2 texture to 1×1, verifies every diagnostic field and proves
retained scenes still render their previous content. Workspace Clippy passes.
The optional exact 2D display curve remains open. The existing
linear-light blending and neutral curve are unchanged. The pixel-camera near-tie
follow defect is separate from these texel-sampling ties and is not fixed here.

Bounds: 32,768 sprites and 256 MiB unique sprite texture storage per prepared
scene; linear geometry is bounded to 54 vertices per sprite; nearest geometry to 1944
vertices per sprite and 128 MiB total (see the wide-region correction below). These
limits exclude retained prior scenes, staged replacements and allocator overhead.
Source textures retain the asset pipeline's own dimension/texel limits. Each
source region is nonempty and at most 8192 pixels per axis. World sizes are finite,
positive and <=1,000,000 metres; normalized pivots/tints cannot contain NaNs.

Numerical verification passes 325 workspace Rust tests, Clippy and 52 native GPU
checks, including four sprite tests. Coverage includes source regions, flips,
painter order, alpha, nine-slice borders, source-free loading, inherited/reflected
transforms and reverse-order submission of queued reimports after scene drop.
Sprite rig ancestors no longer draw diagnostic cubes over their children.

The strict-TypeScript public probe passes shared Undo/Redo, atomic invalid-region
rollback and a source-free scripted flipbook/nine-slice edit. A 25-tick save plus
35-tick restore exactly matches uninterrupted and repeated 60-tick runs; project
and journal bytes are unchanged. This is numerical behavior evidence, not
appearance approval. Integrated checks also pass 420 UI tests, strict typecheck/build, generated
contracts and seven tool tests. The core compiles for WebAssembly and iOS
simulator. Claude Opus 5.5 Max's first two reviews of original engine art and real
captures are integrated in handoff 0032; verification of the derivative-scaled
tie tolerance remains pending. Sprite Inspector work follows the full editor redesign in 0033;
final native review and hosted CI remain open. No atlas, tilemap, sprite animation library, 2D physics or phase gate
is claimed complete by this increment.

## Native runtime and CI integration, 2026-10-10

The sprite branch integrates PR #48 source `588b3ab`, including the shared-value
crate, cooked/native runtime, numeric columns, shipping profile, Windows CRT
policy and CI build reuse. PixelPerfect's unchanged Camera data moved into
`incant_types`; authoring validation stays in `incant_doc`. Regenerated schemas
and the TypeScript SDK are byte-identical. The runtime dependency guard passes.

The integrated source passes 359 workspace Rust tests, all 54 ignored GPU tests
(including the 70-case magnification and 48-case orientation/sheet sweeps),
workspace all-target Clippy, formatting, 25 tool tests and 420 UI tests. The GPU
pass reuses the full workspace graph: zero compile events, with Cargo's initial
finish reported at 0.37 seconds. The editor/headless custom-protocol release
build, workflow lint and generated bridge checks also pass.

The sprite, pixel-perfect camera and ordinary orthographic public workflows all
pass strict TypeScript, exact save/resume/repeat behavior and unchanged authored
files on rebuilt headless SHA-256
`bb3747805921569ac14c277e2085090c61da4e05acf1154781e0c35282ad93d4`.
The sprite workflow also passes source-file removal and shared-command atomic
rollback/Undo/Redo. These workflows still use the legacy PlaySession; they do not
prove migration of rendering or gameplay into the new native world. The native
script host is separate ongoing work. The 20-case eval-check is schema-only,
with live inference and the phase gate explicitly false.

Claude's ongoing review uses the prior frozen `a4b5ddb9…` renderer binary. The
integration does not change its shader or sampling rules, but it does not replace
the required final native review. The near-tie pixel-camera defect, hosted checks
and physical-device/performance gates remain open.

## Wide nearest regions, 2026-10-10

Claude's third review accepts the magnification correction but exposes large-region
precision loss near the f32 source coordinate bound. On Apple M5 Pro/Metal, the
initial outer-edge regression fails 34/600 cases. Separating the fractional bias
from the integer part reduces that to 24/600; interpolation precision still fails.

Nearest geometry now splits authored source rectangles into spans of at most
512 texels per axis. Each span interpolates small local coordinates and carries
an exact flat integer source base. The shader biases the fractional part before
adding the integer coordinates. The geometry preserves the original affine source
mapping, flips and nine-slice cuts. Linear sampling and the display curve keep their
previous paths. The existing 64-byte vertex layout and draw batches are unchanged.
Large regions add CPU/vertex work: the bound is 1944 vertices per nearest sprite,
including nine-slice cuts, versus 54 for linear. The 128 MiB geometry/device limit
is checked before extending the CPU buffer or allocating the next texture.

The expanded regression covers 600 outer-edge and 264 internal-span cases:
512–8192 source pixels, magnifications 1/2/4/8/16/32, both axes/flips and offsets
around pixel-center ties. Independent aligned GPU crops collect a unique color
for every source texel; uniqueness after the display transform is asserted.
All 864 cases pass. Independent review caught an earlier periodic palette that
would have missed a lost source base. Removing only that shader base now causes
720/864 failures, proving the strengthened test detects this regression; the
shader was then restored exactly. This is numerical readback, not appearance
approval.

The unchanged runtime code passes 360 normal workspace tests, all 55 ignored GPU
tests, workspace Clippy and the combined editor/headless/cook/collaboration release
build. After strengthening the test, all seven sprite GPU tests and renderer
Clippy pass again, including the earlier 70-case offset and 48-case orientation/
sheet-height sweeps. A pure geometry test covers area/source mapping, flips,
asymmetric/collapsed slices, span bounds and maximum vertex count. Independent
Astra review found no further blocker. Exact hashes, commands' local logs and
negative-control evidence are in [wide-region evidence](evidence/sprite-wide-nearest-2026-10-10.json).

The rebuilt frozen executable is ready for Claude's independent rendered follow-up.
Hosted adapters, native editor acceptance and reference-phone performance remain
open. This correction does not fix the separate pixel-camera follower near-tie
defect or approve a phase gate.

## High-magnification follow-up: 64-texel spans

Claude's round-four review fixed the region-width defect through 16× but exposed
remaining exact and near-tie errors beyond it. Astra expanded the independent
GPU colour-run oracle; the 512-texel geometry fails 95 of 2,856 cases. Reducing
source spans to 64 texels fixes that set. Adding ±1/256-pixel interior offsets
produces 5,768 passing cases: 1,400 outer-edge and 4,368 interior cases, across
512–8192 source widths, both axes/directions, and the fourteen recorded scales
from 1× through 128×. This is the tested range, not a bound on all possible zoom.
The oracle derives integer source indices from screen-pixel centres and measures
unique post-display colours independently; it does not reuse the shader formula.

The shader and its tie bias are unchanged. Shorter spans bound interpolation and
projected endpoint error while retaining camera-independent geometry: moving a
camera does not retessellate sprites. The material tradeoff is more vertices for
large nearest-sampled sprites. At most 130 spans per source axis, including the
two nine-slice cuts, gives 101,400 vertices (6,489,600 bytes at 64 bytes each) for
a theoretical 8192×8192 nine-slice, versus the previous 1,944-vertex bound. This is
about 6.19 MiB and 52.2× the previous bound. The unchanged texture budget can reject
that theoretical source before geometry is allocated. The unchanged 128 MiB total
geometry and device-buffer caps reject oversized batches before extending retained
vertices or allocating GPU resources; large nearest sprites reach that cap sooner.
Small regions remain one quad per slice, and linear sampling remains unchanged.

The independent Astra review confirms the oracle arithmetic, case counts and bound.
All 360 workspace tests, 55 GPU tests and workspace Clippy pass. These are desktop
correctness checks on Apple M5 Pro/Metal, not reference-phone performance results.
Claude's rendered round-five review is integrated below.
The separate pixel-camera near-tie follower issue is still open. Machine-readable
source hashes and exact case sets are in
[evidence](evidence/sprite-short-spans-2026-10-10.json).


## Independent 64-texel review and remaining limits

Claude Opus 5.5 Max reviewed the frozen `1f72a99` executable in
[round five](../../handoffs/0032-sprite-rendering/result.md#round-5-64-texel-spans).
All 50,862 exact and 76,764 near one-axis ties through tested 128× pass. The
previous failing rows are fixed; unchanged boards, live playback, source-free
copies and resized texture reimports remain byte-identical. Two fresh runs match
across 432 common images. Root verified all seven committed image hashes and
19 independent reference-helper tests. The review consumed 1,006 frames and
560.6 MiB raw, within its explicit 2,048-frame/1-GiB budget.

This does not close general nearest-sampling correctness. Above 128×, 138/1,404
exact and 4/2,808 near ties fail, beginning at 160× in HD. Two-axis nine-slices
also expose a pre-existing defect on every tested build: 121 exact-tie pixels
near 128-square frame edges and 646 pixels in non-dyadic 96-square views fail on
the candidate. The reported clipping/raster-grid explanation is an inference,
not a proven hardware cause. Both prior builds fail more pixels, but the new
geometry moves some failures; it must not be called pixel-correct for all views.
A future correction needs independent two-axis/projection/zoom coverage without
unboundedly shrinking retained spans or widening the near-tie acceptance band.

The total geometry budget now rejects a real scene of 342 nearest 2048-square
sprites; 341 render successfully. The previous 512-span build renders both.
The error prevents output but needs a better resource/count diagnostic. No
portable backend, native-editor, phone performance, arbitrary zoom or phase gate
is approved. The separate camera/follower placement contract remains open.
