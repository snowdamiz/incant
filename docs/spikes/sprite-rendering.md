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
scene; geometry is bounded to 54 vertices per sprite and 128 MiB total. These
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
