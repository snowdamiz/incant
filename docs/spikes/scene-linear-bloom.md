# Scene-linear bloom (opt-in, CPU/source reviewed)

`Renderer::enable_bloom(Bloom { threshold, intensity, levels })` prepares seven
fixed pipelines once: three filter entries and four output formats. The default
renderer, original WGSL output/tone curve, HDR/depth formats and physical output
size are unchanged. Zero intensity and pixel-perfect cameras take the original
output path and allocate no bloom targets. Screen-space UI is drawn later and is
never extracted. World text belongs to scene radiance and can contribute.

Bloom is encoded at the start of the existing **Display** graph group, after
world Text and before tone mapping. The pending seven-group GPU timestamp
integration should include bloom within Display, without new query slots. This
branch does not incorporate pending DRS or timestamps. Future DRS integration
must pass its actual source extent and scene rectangle **and map physical output
position into internal scene coordinates before the bloom lookup**. A paired
DRS+bloom integration check is mandatory. Bloom footprint is in
internal scene texels. No resolution-independent apparent-radius claim is made.

Native `cooked-output` (including features which forward it) refuses bloom before
pipeline creation: the approved fixed artifact has no bloom binding or output
variant. Browser WGSL can use the portable path. This opt-in is not an excuse to
bypass the cooked artifact contract. There is no window/editor enablement here.

## Numerical contract

The source is finite nonnegative scene-linear Rec.709 premultiplied RGB `H` and
coverage `a` (clamped to `[0,1]`), with `H = a R`. Source radiance is clamped to
half-float max 65,504. This contract treats alpha as coverage, not physical
transmission. A zero-alpha texel extracts exactly zero, even if it contains
residual nonzero RGB. For other texels, with `R = clamp(H/max(a,1e-8))`, extraction
is `H * max(Y(R)-threshold,0)/max(Y(R),1e-8)`, where Y is Rec.709 luminance. Thus
fractional coverage reduces emitted bloom rather than promoting faint coverage
to a full-coverage source. Only pixel centers in the half-open scene rectangle
contribute; outside samples are zero, without edge reweighting.

Each level has ceil-half dimensions. Its reduction integrates the intersecting
source texel cells over the destination's normalized footprint (at most 3x3
texels), with nonnegative weights summing to one, including odd dimensions.
Reconstruction uses clamped bilinear texel-center samples and convex weights:
`A_i = (D_i + (n-1) upsample(A_(i+1)))/n` for n remaining levels. Every level has
equal weight, so a constant field's normalized gain does not grow with levels.
No Gaussian/temporal/physical optics equivalence is claimed. RGBA16 intermediate
rounding and device sampling execution remain subject to actual device tests.

Let L be reconstructed light times finite gain, clamped to 65,504, B the existing
display-linear backdrop and T the unchanged tone curve (`T(0)=0`). Before the
unchanged chrome and sRGB stages, the optional output is:

`clamp(a*T(clamp(R+L)) + (1-a)*(B+T(L)), 0, 1)`.

Opaque light is added in scene space before tone mapping. Transparent pixels
receive an additive display halo over the backdrop. Fractional coverage blends
these policies without adding alpha. At L=0 this is the original composition;
zero configured gain also bypasses the variant entirely. Clamps bound the
nonnegative finite arithmetic, including maximal half-float source and gain.

## Admission and ownership

Threshold is `[0,65504]`, intensity `[0,8]`, and requested levels `1..=6`; NaN and
infinities refuse. The pyramid stops at 1x1. Admission checks nonzero extent,
8192/device dimensions, four megapixels and the exact aggregate texture payload
before allocating. One RGBA16 down texture per level and one reconstruction per
nonterminal level cost `8*sum(area(down)) + 8*sum(area(up))`, capped at 32 MiB.
The source attachment is the existing frame cache, outside this additional cap.

The cache retains one extent. Old command buffers own the views/bindings needed
until their normal queue lifetime ends; resize does not invalidate them. The cap
is logical retained texture payload, **not** driver padding, peak old+new cache
memory, arbitrary caller-held command buffers, RSS or total renderer memory.
There are at most eleven filter passes and bounded immutable settings/bind groups
per prepared frame. This slice does not claim allocation-free frame preparation.
There is no per-frame pipeline creation, authored data loading or serialization.
A failed startup replacement leaves the prior bloom configuration intact.

## Verification and open work

Eight initial CPU tests pass: typed bounds/refusals, counted targets, constant
fields and odd-area means, source alpha/luminance behavior, nonnegative impulse
spread, offset viewport inclusion, coverage/tone limits and Naga validation of
both fixed WGSL variants. Two real-device tests are present but ignored: isolated
emitter/transparent-residual/resize resource lifetime, and canonical zero-gain/
pixel-camera bypass plus failed replacement. No GPU has run in this lane.

The full default suite passed 73 CPU tests, with 84 GPU tests ignored. Default
and cooked-output all-target Clippy, wasm32 library compilation, focused cooked
configuration tests, formatting, the normal runtime graph and conventions passed.
The first broad runner called a nonexistent dependency-guard filename; its exit 2
is retained and the existing guard passed on the same source. Root accepted source
review in [the receipt](evidence/bloom-source-review.json); exact source, logs and
frozen binary identities are in [the evidence](evidence/bloom.json). These are CPU
and source checks, with no GPU efficacy/appearance claim. Claude
must validate actual nonzero bloom, transparent/fractional/opaque composition,
output/UI/chrome isolation, odd/offset extents and current projection/retained
scene paths, with matched baseline, bounded render budgets and an explicit GPU
lease. Styling, default enablement, phone/device performance, temporal upscaling,
MetalFX and full renderer acceptance remain open.
