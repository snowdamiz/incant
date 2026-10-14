# R4 numerical fixture alignment (unqualified candidate)

Claude's immutable R4 packet `dc7701b29ab7eb3a0dc91695a2d1e9badb66c138`
was applied exactly to `76be9608e99bf089fcb1e0908827330f70c28ac1` as
`0aed5802`. This Astra follow-up changes test/oracle code only. It does not change
shaders, reconstruction settings, visual thresholds, or the retained failed
appearance results. CPU validation is necessary before any delegated GPU test.

The previous executable fixtures disabled the new evidence-origin flag and
compared new Gaussian reconstruction against fixed bilinear expectations. The
fixtures now provide valid per-texel mass/age and equal-area history density.
A separate f64 reference enumerates current Gaussian support and associated
history color/depth/evidence/provenance. Explicit scalar counterexamples cover
full clear-sample measure, malformed inputs, aggregate density before cap,
paired-tap rejection, all positive motion support, carried-primary identity and
retirement before blending even at reaction zero. The reference intentionally
models these fixtures' spatially uniform native depth. It is not a complete
rasterizer or a substitute for actual shader execution and rendered review.

Both ignored numerical fixtures retain their original submission counts:
ordinary reconstruction 19 submissions/maps at 8×8; owner reconstruction 12 at
16×16, including the existing extra current-only case-3 pass and ordinary
baselines in cases 7/8. Readback storage grows from two to four planes (8,192 B)
and from six to eight planes (32,768 B) respectively to observe RG16Float mass
and age. Case 11 adds three age controls in unused existing tiles, without a
new pass. These are new test inputs; they do not relabel prior GPU evidence.

Case 3 retains exact whole-plane equality to a history-disabled owner render,
now including evidence. R4's Gaussian gives the changed neighboring tap positive
support (fresh center ≈0.0079), unlike the prior bilinear 2^-20 bound. Its fresh
color has an independently calculated Gaussian bound; stale ≈0.405 remains
forbidden. The original bilinear failure and correction remain historical.
Cases 7/8 still require exact owner/ordinary color, depth and evidence equality
for the *current* algorithm. No prior-baseline equivalence is asserted.

The original model-only kernel/confidence tests explicitly keep the historical
fixed jitter lattice and their unchanged thresholds. New tests separately check
current split-lattice coordinates and normalized constants on odd offset
viewports. Eight distinct axis strata do not prove arbitrary thin 2D geometry
coverage. No C7/C9/P1 or other appearance gate is closed by these models.

Initial qualification is retained under external
`reconstruction-r4-cpu-001`: 160 CPU tests passed, two failed, and 43 GPU tests
were ignored. Failures were the new WGSL helper's reserved identifier `coherent`
and the old MASK sampling literal. The latter test now asserts the actual
shared material/MASK `textureSampleBias(..., frame.lod.x)` contract. Claude owns
the identifier-only shader correction. There has been no GPU execution.
