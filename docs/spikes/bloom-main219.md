# Output-space bloom: independent fixed-renderer composition

This private candidate selects the accepted bloom implementation from
`5d02ad6c09787ffd009a7517d448fb27b1fe9f58` onto the reviewed fixed-only renderer
candidate `931998df4d9ceb3d076e242938cddc653c75aecc` and actual main219. It does
not import SSAO or the effects producer ancestry. Bloom remains explicitly opt-in;
native cooked-output configurations refuse it with `BloomError::CookedOutput`
until an independently accepted artifact family exists. Default output and
all existing cooked program catalogs are unchanged.

The normalized pyramid is anchored to physical output extent, with explicit
first-pass DRS source footprints and center-aligned odd levels. Both bloom WGSL
files, thresholds, gain, coverage composition, viewport mapping, dimensions,
level count and resource caps exactly match the accepted producer. Pixel cameras
and zero gain keep the original output without pyramid allocation. Host glue
uses the current FramePlan and its output mapping, including fractional viewport
offsets. No appearance tuning is introduced.

The Display query pair now spans the first actual bloom filter pass through the
final tone-map/output pass. Interior filters omit query descriptors, so each query
is written exactly once. Without bloom, the ordinary output pass retains both
writes. Models retains its reviewed fixed-temporal interval. This envelope still
measures instrumented extrema and gaps, excluding later screen UI and presentation;
it is not complete-frame GPU cost.

The three original bloom device tests now assert the requested actual backend
and precise cooked refusal. A second entry uses the existing four-frame joined
fixed/overlay/timestamp helper with bloom enabled: full/full/half/half resolution,
phase/reset checks and positive Display timing where supported. The cooked version
requires exact refusal before completing ordinary fixed frames. No images are
created by this fixture. Existing fixed-only entry remains behaviorally unchanged.

Producer Claude0075 acceptance (8dbd260d) is bloom-only: 37 GPU cases, 534
submissions and all 27 unchanged analyzer gates passed. The previous centroid
defect at reduced DRS is corrected in that producer. SSAO remains rejected.
Those appearance and numerical results belong to the producer source/binaries;
current composed CPU and bounded GPU validation are pending. Fixed temporal
softness and MASK detail around 18% of reference remain unchanged limitations.

The first formatting invocation used an absolute cargo path without the matching
cargo-fmt directory on PATH and failed before formatting. The corrected pinned
PATH invocation succeeded. No compiler or GPU execution has yet been performed
for this candidate.
