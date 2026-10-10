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
`textureLoad`, with a 1/1024-texel tie tolerance toward increasing screen
coordinate on the dominant derivative axis (X wins equal slopes). This is a
defined sampling rule, not an assumption about GPU raster subpixel precision.
Linear sampling retains its normalized-UV filtering path. The regression passes
eight orientations (including quarter turns and a mirrored parent), scales 1–3,
and sheets of 64 and 80 rows, with every texel represented and resized-sheet
frames byte-identical. Full checks pass 325 Rust tests, Clippy and 53 GPU checks.
Claude's broader corrected-renderer review remains required.

The review also requests asset/entity-specific diagnostics for undersized
reimports and an optional exact 2D display curve. Those remain open. The existing
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
simulator. Claude Opus 5.5 Max is reviewing original engine art and real captures
in handoff 0032. Sprite Inspector work follows the full editor redesign in 0033;
final native review and hosted CI remain open. No atlas, tilemap, sprite animation library, 2D physics or phase gate
is claimed complete by this increment.
