# Private reconstruction recurrence experiment

This is an unadopted, test-only investigation on top of the opaque feedback
correction at `72e5d626`. It does not change production shaders, targets, cooker
contracts, admission limits or defaults. The ordinary moving-scene and thin-line
quality failures remain open.

## Reason for the experiment

Raw Metal feedback showed that a phase-dependent current-color clamp can erase
valid accumulated texture detail even after opacity no longer rejects history.
A CPU prototype therefore retained four **fresh** luminance observations, gathered
with the same positive bilinear-times-mass taps as historical color. Missing
observations invalidate an entire lag rather than renormalizing a different tap
set. A sign reversal outside adjacent binary16 intervals, an observed intensity
envelope, and a current-color chroma check can permit unclipped history.

The shader is a heuristic. Real oscillating light can satisfy the same predicate;
the hardware control deliberately records that counterexample. Half-neighbor
intervals describe storage precision, not a bound on all shader arithmetic.
Depth, motion, current-support and existing-history admission remain unchanged.

## Actual execution

Evidence is retained outside Git under
`~/.codex/incant-evidence/reconstruction-recurrence-gpu-*`. Every generation has
its source freeze, owned-process supervisor, logs and terminal status.

| Generation | Actual outcome |
|---|---|
| 001 | Compile refused an ambiguous fixture integer type; no GPU execution. |
| 002 | Build passed; CPU validation refused the omitted Naga packed-half capability. No GPU execution. |
| 003 | Two CPU controls passed. Metal ran all 16 classifier controls in one submission/map, then 48 actual retained-scene frames with 48 experimental resolves, 96 submissions and 48 maps. Ten PNGs and 384 raw planes were retained. Strict Clippy subsequently refused three constant-size slice iterators. |
| 004 | Mechanical iterator/type-preserving cleanup; build, two CPU controls and strict all-target Clippy passed. Its shaders are byte-identical to 003; no second GPU run is claimed. |

Generation 003 source freeze is
`35e0a12af1ea49d62296d2a82180275708c73c1599e50eb63e506cae7d05a04a`;
004 is `9e0b20c813a7ac0ffbe54d658857911c15c4daad76a2108b1daad070571a93f8`.
The adapter is Apple M5 Pro / Metal. These are test-profile functional runs,
not shipping-performance measurements.

The fixture renders an ordinary production UP scene and feeds its same current
HDR, motion and depth into an independent experimental history. Stationary and
moving cases each have 24 frames. Full baseline/candidate linear half-float
planes are retained. The PNGs use the same explicit CPU transcription of the
production tone transform for both inputs; they exclude current-only overlays
and do not claim bit-exact final display output. Cold-frame comparisons pass
the predeclared one-half-code bound.

## Decision and remaining work

Root's rendered review finds a material stationary-chart improvement by frame
24, but the moving chart remains strongly aliased and thin diagonal lines remain
broken. Neither passing scalar controls nor this limited stationary benefit is
appearance acceptance.

Independent raw replay rules out motion-coherence refusal as the primary cause
in the moving chart: none of 7,920 sampled chart points on frames 15–24 exceeds
the unchanged motion threshold, and 5,411 retain recurrence history. At one
stripe point on frame 19, the last five fresh luminances rise monotonically
(`0.105896, 0.109558, 0.300049, 0.354004, 0.606445`) even though the longer
sequence oscillates. The short-window classifier therefore drops valid history;
the current clamp moves its red channel by about 0.242 despite a current sample
fraction of only 0.0076. Later re-admission retains that bias. This motivates a
different temporal decision, not a wider motion tolerance. Mixed support and
thin-line coverage remain separate failures.

The independent execution/source review is retained at
`~/.codex/incant-reviews/reconstruction-recurrence-raw-and-source004-independent-20261014.json`
(SHA-256 `cc96abdd33effb5ee8d4b3233205ade0daa084013106217fe8b8a43cfbae17a2`).
It accepts execution integrity and the mechanical 004 source correction, not
appearance or adoption. The moving replay is a diagnostic f64 transcription,
not a bit-exact shader oracle. Both cold full color planes are byte-identical.

The prototype's two additional RGBA16F textures would cost 16 bytes per output
pixel in production. The documented large overlapping transition would exceed
the unchanged 671,105,024-byte admission cap. No cap increase or production
allocation is proposed. Host resets, atomic publication, moving/occlusion
controls and complete rendered review would all be required for adoption.

## Rectification and interpolation ablations

Runs 005 and 006 each completed the same 48-frame Metal scene, CPU controls and
bounded readbacks with frozen sources. Run 005 disables color rectification only;
run 006 additionally replaces admitted history interpolation with a support-bounded
signed Catmull–Rom numerator/mass interpolation. Any invalid cubic tap falls back
to the original bilinear history. The signed filter is explicitly not positive
sample evidence. Neither mode is reachable by a production pipeline.

Unclipped history materially reduces moving stripe aliasing but leaves an obvious
trail from the moving cube's shadow. The sharper interpolation changes that image
only modestly and does not remove the trail or broken thin lines. Root inspected
the actual PNGs; there is no quality acceptance. Run 006 strict all-target Clippy
passes. The next experiment targets bounded temporal retention, preserving the
unclipped shadow failure as a counterexample rather than adopting either ablation.

## Bounded retention experiment

The next test carries at most seven remaining frames after detecting a new
reversal (one eight-phase cycle including recognition). Reprojection takes the
minimum remaining count of every contributing history tap. An old reversal still
inside the observation window cannot renew it. Invalid observations, inconsistent
chroma and the observed luminance envelope can retire it immediately.

Run 007 passes 20 actual Metal classifier controls, including the monotonic-window
counterexample, expiration, hue/missing-observation refusal and a permanent bright
step that rejects the fixed old color on its fourth sample. Its scene **does not
exercise the new retention**: a missing pipeline constant left the shader default
off, caught by strict Clippy's unused-variable error. The scene report's
`bounded_lock: true` is incorrect and must not count as execution evidence.

Run 008 fixes that binding and checks actual readback for nonzero retention state.
Build, two CPU controls and strict all-target Clippy pass; 48 actual Metal frames
complete with 96 submissions and 48 maps. The readback contains 521,282 nonzero
retention samples in the captured regions. The helper shader is unchanged from
007's executed classifier controls; no second classifier run is claimed. Root's
moving-frame review still rejects the result: striped aliasing remains conspicuous
and thin lines remain broken. No production source, allocation or gate changed.

These experiments identify two different signals needed by a replacement design:
recurring detail should survive changes in sampling phase, while moving shadows
and real shading changes must retire history promptly. The current short-window
color rule, with or without this bounded extension, is not sufficient.

## Nearest-surface guidance ablation

Run 009 admits mixed depth and partial coverage under the nearest covered
sample's motion/depth, keeping the original positive color/mass reprojection and
recurrence clamp. This conventional guidance is a heuristic, not evidence that
every contributing color follows that surface. It does not change the motion
tolerance, use bounded retention or enable cubic history. Alpha is canonicalized
only when all actual contributors are opaque; partial coverage is preserved.

The frozen source is
`0a42971d2de464c059241b3caf471270bedba8da4369b1b626ca8f8b20830499`.
Build, two CPU controls and strict all-target Clippy pass. The same 48 actual
Metal scene frames complete with 96 submissions, 48 maps and cold-frame error at
most one half-float code. Root inspected stationary frame 24 and moving frames
8 and 24: moving stripe aliasing and broken thin lines remain visible. This
result is rejected for adoption. In particular, lifting the mixed-support
refusal alone does not repair the temporal color decision, and completely clear
current support still cannot recover an intermittently sampled thin surface.
No production shader, allocation, admission cap or gate changed.

## Coarse shading transfer experiment

Run 010 replaces the experimental four-luminance history with a spatial RGB
mean and luminance standard deviation over a separable seven-tap binomial
kernel. It carries the old fine residual onto the current coarse mean and
rectifies against the union of the current color box and mean plus/minus one
standard deviation. Geometric admission and sample mass stay unchanged. This
tests a different signal decomposition; the deviation box is a heuristic, not a
statistical confidence bound or a shading-change proof. No external shader code
was copied.

Build, CPU controls, strict Clippy and the 48-frame Metal scene all pass on frozen
source `46559d64444354e1a919f18c82dc5b8fdb9bd7f6a53b9bdfb0e6fdd4669fe720`.
Root inspected moving frames 8/24 and stationary frame 24. Broad stripe bands
remain conspicuous even while stationary, and shadow trailing and thin-line
failure remain. The result is rejected. A current spatial mean can itself alias;
transferring it directly into the history is not sufficient to distinguish
lighting changes from sampling changes. No production adoption is proposed.
