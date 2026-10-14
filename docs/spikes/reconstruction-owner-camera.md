# Camera reprojection of unchanged owner coverage

This private, test-only experiment follows the rejected protected-history
reference derivative in [the reference comparison](reconstruction-reference-comparison.md).
It changes the source of history admission: an unchanged exact object record and
camera projection can carry fractional coverage even when the current internal
samples miss a thin surface. A colour lock alone could not distinguish that miss
from the disappearance that produced trails in the preceding experiment.

No production shader, cooker ABI, renderer preset or default changes. This
worktree includes unmerged reconstruction/MetalFX ancestry and must not be merged
wholesale. The current main engine is not represented as having this algorithm.

## Bounded algorithm

The standalone pipeline composes the existing owner resolver with one test-only
entry point. The static-camera and single opaque-surface paths remain the
existing resolver. Moving fractional coverage seeds an exact owner/depth/mass
record and considers a bounded neighborhood of old output texels. The initial
experiment uses radius 2 (25 centers); the support successor below uses radius 3
(49 centers). Each old center is projected
through the previous unjittered inverse camera matrix and the current unjittered
matrix, using its retained linear depth. A positive tent weight requires its
projected center to lie within one output pixel of the destination.

Only unchanged, single-owner, fractional records with valid mass/depth/age
contribute. Mixed-owner records have no single depth that can transport all
their colour and are refused. The nearest compatible candidate owner wins;
current foreground and different current owners refuse deeper/foreign history.
Lighting/caster changes refuse persistence. The existing eight-absent-frame
expiry, depth tolerance and mass cap remain unchanged. Premultiplied zero-alpha
cleanup is preserved. No shader adds a colour-clamp exception to validate an
otherwise unproven object.

This is a small-motion experiment, not arbitrary camera reprojection. A bounded
search may retain a partial tent partition near its support limit, before losing
history altogether. Neither radius proves arbitrary displacement support.
Only fixed camera projection is exercised. A stored nearest depth approximates
the colour footprint; it is not a complete layered reconstruction of self
occlusion. Projection-area density and larger-motion correspondence remain open.

## Executed evidence

External evidence is under
`~/.codex/incant-evidence/reconstruction-owner-camera-00{1,2,3}`. Each generation
retains exact source hashes, source copies, command/log identities, compiler
artifact identity and an immediate executable copy. Rust 1.99, locked/offline
targeted tests, two Cargo jobs and two Rayon threads were used. All owned command
processes terminated and were reaped. These are test-profile observations, not
shipping timing or memory-gate evidence.

Generation 001 failed compilation because the new readback-plane type was not
in scope and its numeric extent was ambiguous. Its failure is retained.
Generation 002 defines that local type and passes compilation, the CPU shader
validation/projection control, and strict all-target renderer Clippy.

Generation 002 freeze SHA-256 is
`2d6240677db4c7d5336f9dcc1068c939d9c0e40179cfbf4cc4053e51c9e1abfc`;
executed binary is
`f928e18807750db252ce6cbf60cef2ac57aad263a2ea7ee87e60bee9ea3019b8`.
Its Apple M5 Pro Metal run executes 64 camera-only frames and 64 matching frames
where the whole opaque stage moves out of view after frame 32. There are 256
submissions, 128 maps and 1,556,152,320 retained raw bytes. Inputs use the existing
eight-phase jitter, 480x270 internal and 960x540 output, and the explicitly
diagnostic internal texture footprint; production LOD policy is unchanged.

The replayable `analyze.py` verifies all 1,024 raw-plane hashes. All 256 planes
before eviction match across the two cases. Candidate output has no negative RGB
or nonzero RGB at zero alpha. At frame 24, all 200 preselected diagonal centers
have RGB above 1e-5, versus 101 in the owner baseline. The two thinner horizontal
wires also remain present where the baseline loses them. These center samples
describe retained geometry, not a visual acceptance metric.

In the fixed old-wire ROI `[0,48,440,130]`, every candidate RGBA value is exactly
zero from eviction frame 33 through 64. This scene control changes a shadow
caster, so it exercises global rejection as well as the owner table; it does
not alone prove the isolated owner-bit rejection. Analysis SHA-256:
`a4174e5cad5b008f4b7ebe799261711ad4b326fdb5128940cb97e7aca06e6fdf`.

Generation 003 adds small actual-shader refusal controls, without changing the
camera shader. Its freeze is
`7c4dd07321743012e2753931f9c5a1d3a4018f9518d602b736670848997b9ceb`;
binary is `d489ad983b68ea36350dc4e85666d5a6c30cf9138ab7a970e830ad1da30355a2`.
Fourteen 16x16 cases pass in one Metal submission/map: positive unchanged
coverage; changed owner with no caster flag; caster change; eighth absent frame;
zero mass; cold history; opaque history; mixed record; invalid owner, depth and
matrix; out-of-window movement; and foreground occlusion with the same or a
different owner. The two foreground controls admit fresh green and no old red.
CPU validation and strict Clippy pass again. The full scene execution remains
generation 002 evidence, not a claimed generation 003 repeat.

## Pixel review and decision

Root reviewed actual frame-24 candidate/baseline and frame-33 eviction PNGs.
Thin-wire retention is materially improved, but diagonal and horizontal detail
still show dotted intensity and jagged edges. The texture chart also retains
visible sampling artifacts. **Not visually accepted and not adopted.**
Remaining work concerns reconstruction/filtering quality, broader motion and
occlusion coverage, and eventual shipping measurements. Numerical presence and
refusal controls do not pass appearance, performance or phase gates.

## Support correction experiment

Review of the retained frame-24 inputs found a concrete support truncation in
the initial radius-2 gather. The wire history moves about -2.39376 output pixels
horizontally, so both old offsets +2 and +3 have positive tent weight. Radius 2
loses the +3 column. A uniform field would retain only about 60.6% of its tent
partition; the actual omitted old tap also has positive valid color and mass.

The successor isolates radius 3, keeping the current Gaussian reconstruction,
all rejection rules and the actual scene fixture unchanged. For constant-depth
translation the complete tent partition requires each displacement magnitude
to be at most the radius. This does not establish support for arbitrary camera
rotation, depth gradients or image-boundary disocclusion. The larger search adds
48 tap evaluations across the two loops, with no new history allocation.

Independent small GPU controls compare radii 2 and 3 using uniform fractional
history and known dyadic translations at, inside and outside their support
bounds. They check blended color and evidence mass against a separable analytic
partition; transparent current samples still contribute current mass. A split
red/green history also distinguishes recovery of the missing tap from merely
renormalizing the retained tap. Generation 004 stopped before launching Cargo because its external supervisor
used the wrong cancellation keyword; its failure and empty log are retained.
Generation 005 corrects only that supervisor call and executes the same source
freeze `31ca777555434653f0da0ad244233f13dcf5b84b139b229f8d9a0bf3bbdb3154`.
The immediate binary copy is
`5c0f4bde79f2880aa5e37664b4af8a749ba2288668ffe8ada354924bfba57e84`.
Two CPU controls, 38 actual Metal support cases, the 14 existing refusals, 128
scene frames, and strict all-target renderer Clippy pass. Support controls use
one submission/map and retain both assembled radius shaders plus color and mass.

The replay compares all 128 frame identities, 512 unchanged input/baseline
planes and 2,048 raw-plane hashes across generations 002 and 005. Both runs'
disappearance ROI remains exactly zero after eviction. The original captured
premultiplied-alpha range checks pass. The first comparison guard unnecessarily
rejected a changed limitations sentence in the copied analyzer; that failed
attempt is retained. The corrected verifier allows precisely that prose delta
and still executes the unchanged generation-002 analysis logic. Comparison:
`0633fc2889792f721e8ca551deba8386278e98cf3b26df3746834c0d5ec25de8`.

Root reviewed actual frames 14, 24, 32 and eviction 33. Thin wires are markedly
smoother at frames 24 and 32; the slower-motion frame 14 remains unchanged and
jagged, and the texture chart still aliases. Center-presence counts were already
full and cannot establish this quality difference. The support defect is fixed
within the tested translation bound, but **appearance is still not accepted**.
No shipping-performance or production-adoption claim follows. Box-integrated
reconstruction remains a separate, unimplemented hypothesis.


## Integrated output-footprint experiment

The next private variant integrates the existing positive Gaussian over a unit
output pixel instead of sampling it only at the output center. Its radius-3
history gather, source coordinates, rho, r-squared scale, eight jitter phases,
3x3 current stencil, ownership and rejection policy remain fixed. The normalized
ratio of integrated numerator and mass is not the exact area integral of the
normalized estimate. This cannot recover never-sampled geometry or establish
correctness for arbitrary motion or layered occlusion.

The separable kernel uses positive eight-point Gauss-Legendre weights on each
axis. Independent adaptive integration and erf references cover 173 distinct
weights and twelve sample matrices; quadrature moments through degree 15 pass.
A scalar f32 model is preparation for actual Metal execution, not GPU evidence.
The external generator and raw numerical reports are retained under
`~/.codex/incant-evidence/reconstruction-owner-footprint-oracle-001`. The shader
performs sixteen scalar exponentials per weight instead of the point kernel's
one; there is no performance claim or production adoption. Generation 006 retains the actual shader probe, refusal controls and scene
comparison separately from generation 005.

Generation 006 source freeze is
`c4c48f3ef6f5c6364b158e3643b73948bcbe005f7736a965667a9e219f94ebba`;
its executed binary is
`d87a9f94202e8fe7fda9b4395de141b82c602b5e4ddf82070b9ebcc9de181f99`.
Two CPU controls, all 173 actual Metal weight cases, the fourteen refusal cases,
the 128-frame scene and strict all-target renderer Clippy pass. The maximum
absolute GPU weight error is 5.83548e-7 against the independent reference. This
checks the additive 2e-6 + 2e-5*abs(expected) envelope and positive normal weights;
it does not establish a uniform relative-error bound for arbitrarily tiny tails.

A read-only comparison verifies all 128 frame identities, 512 unchanged input
and baseline planes, 2,048 raw-plane hashes and 56 PNG hashes against generation
005. Both disappearance ROIs remain exactly clear after eviction; captured PMA
invariants remain valid. All 128 candidate frames change, including cold outputs
as expected from the different spatial kernel. The first comparison stopped
because this generation has no copied analyzer; its failure is retained. The
successor pins and executes the immutable original analysis body and preserves
all data/source checks. Its output SHA-256 is
`65843658d4a70d9dd37b92d86b10fd2deed28fd09649feada68f35c490676458`.

Root inspected actual frames 14, 24 and 64 and eviction frame 33. The spatial
change is modest. Frame 14 still shows dotted thin detail and jagged edges;
frame 64 shows smeared thin detail during camera motion, and sphere silhouettes
remain stepped. **Not visually accepted and not adopted.** Static sampling,
repeated history resampling and depth-layer admission need separate diagnosis;
these observations do not establish which mechanism dominates each artifact.


## Opacity and history-filter successor — rejected quality candidate

A bounded inspection of generation 006 found a correctness defect independent
of the footprint kernel. At stationary frame 14, pixel (512, 282), all current
contributors have alpha 1 and the same combined stage owner, but depths span
approximately 7.73 to 13.66. The single-depth predicate is false, so the former
canonicalization does not force alpha 1; the stored half-float alpha is 0.9995117.
That numerical residue can accidentally qualify as fractional coverage later.
The stage owner represents the floor, cube, sphere, posts and wires together;
owner identity is not a proof of one depth layer.

The next test-assembly-only guard separates current opaque coverage from a
single depth layer. It canonicalizes proved full alpha and refuses the camera
fractional-history path for fully opaque current mixtures. Production shader
source is unchanged. Actual mixed-depth controls must pass before comparison.

Separately, the candidate history filter reconstructs color with normalized
Catmull-Rom at one coherent old-grid query. Positive tent evidence mass remains
separate. Every nonzero signed contributor and central clamp contributor must
prove exact unchanged ownership, positive valid evidence, age, compatible depth,
current occlusion and the same projected translation within 1/256 output pixel.
Only canonical zero color/record/depth/evidence can participate as empty history.
Missing support causes whole-filter fallback. The full 4x4 footprint must lie
inside the old image and can extend beyond the radius-3 positive search. The
central 2x2 range limits overshoot, followed by premultiplied-alpha cleanup.
This is an estimator change, not a signed numerator divided by positive mass.

The comparison will use the same guarded source and executable for Tent and
Cubic, keeping the point kernel, eight jitter phases and age limit unchanged.
It targets history diffusion only. Static sample coverage and opaque mixed-depth
silhouette reconstruction remain separate, unresolved work.


Generations 007–009 implement this separation and paired color-filter test.
Generation 007 passed the three CPU and 23 paired actual Metal controls, but
strict Clippy rejected a constant assertion in the new test. Generation 008
reads the test's actual input depths instead; CPU controls, the 23 paired cases,
fourteen refusal cases, strict all-target Clippy and the 128-frame Tent scene
pass. Generation 009 uses the same binary and sources with only the history
selection changed to Cubic; its refusal cases and 128-frame scene complete.

The strict paired comparison rejects generation 009: camera frame 62, pixel
(165,119), stores RGBA16 bits `[5,4,4,0]`, violating canonical transparent color.
A separate, explicitly failed-contract diagnostic verifies 128 matching frame
identities, 512 unchanged input/baseline planes, 2,048 raw-plane hashes and 56
PNG hashes. The cold frames match and both eviction ROIs remain clear. Cubic
reduces the observed diagonal alpha-profile spread during motion, but changes
profile mass and peaks too; spread alone is not a quality verdict. Actual pixel
review finds materially less wire blur but continuing static jaggies and opaque
mixed-depth silhouette aliasing. No visual acceptance or production adoption.

## Half-precision output correction — diagnostic only

Cleaning a positive f32 alpha before attachment conversion does not guarantee
canonical RGBA16 output. Generation 010 adds two paired tiny-alpha regressions:
all four variants fail on actual Metal, including the value which the CPU
round-to-nearest reference places at the smallest positive half subnormal.
Generation 011 tries a final `pack2x16float`/`unpack2x16float` round trip. It fixes
the zero-midpoint case but still fails both surviving-subnormal variants. Both
failed generations and complete raw controls remain retained.

This is not proof of a particular Metal optimization. The pinned Naga 29.0.4
MSL writer implements those built-ins through native half casts. The
[WGSL conversion rules](https://www.w3.org/TR/WGSL/#floating-point-conversion)
allow either adjacent representable result; packing/unpacking can also flush
subnormals under the specification's floating-point rules. Assuming one CPU
rounding policy for those separate conversions was insufficient.

Generation 012 instead computes the final alpha's binary16 round-to-nearest,
ties-to-even representative with integer significand rounding and expresses it
exactly as f32. Zero quantized alpha clears RGBA; other metadata is unchanged.
No intermediate current/history color or evidence mass is quantized. A zero
color can therefore retain nonzero metadata, which the cubic path conservatively
refuses as noncanonical empty history. That is an explicit retained behavior.

Generation 012 passes three CPU controls, all 25 paired Metal cases, fourteen
refusal cases, strict all-target Clippy and the 128-frame Cubic scene. The tiny
positive control now stores alpha bits 1, while the underflow control stores
exact zero RGBA. Every scene frame reports zero negative-color pixels and zero
nonzero-color/zero-alpha pixels. Its source freeze is
`aa6390864cd3cc127b10018edb4531c414524720642b96a890629d84540edf38`.
Root inspected actual frames 14, 24 and 64: the correction retains the sharper
moving wires, but static sampling and opaque depth-edge problems still prevent
visual acceptance. Independent attachment-boundary controls and a bounded raw
comparison remain additional qualification work, not a reason to claim adoption.

The final-store boundary now also has an independent 107-case actual GPU probe
(`output_controls::actual_owner_output_precision`): runtime RGBA32 inputs,
paired RGBA16 and RGBA32 outputs, one draw/submission/map, exact half-crate
round-to-nearest/ties-to-even expected bits, and separate input/helper/final
alpha observations. Generation 013 failed shader validation before drawing
because the probe used the reserved WGSL field name `diagnostic`. Generation 014
renames that probe field to `observed`; build, 3 CPU controls, all 107 actual
precision cases and strict all-target renderer Clippy pass. Six GPU diagnostics
remain explicitly ignored in the ordinary CPU inventory. The 014 source freeze
is `c49de212e764de37fd2d8d422c9bb50f45b8319f1320cbb2e65ed381a5b03b0c`;
the complete native outputs are retained under
`~/.codex/incant-evidence/reconstruction-owner-camera-014`. No scene algorithm
changed after 012, so this probe does not relabel or replace the 012 scene run.

The independently checked 009-to-012 comparison binds all 128 identities, 512
unchanged input/baseline planes, 2,048 raw planes (3.112 GB), and 56 PNGs. The
changed-owner disappearance region stays exactly zero. The old frame-62 PMA
counterexample is preserved; 012 has no reported zero-alpha/nonzero-color
violations. Cold RGB is byte-identical but 3,191 cold alpha pixels differ by at
most one half ULP (0.00048828125). All 128 feedback outputs differ, so this is
not described as a one-pixel correction. The bounded comparison is retained at
`~/.codex/incant-evidence/reconstruction-owner-output-comparison-001/stdout.json`,
SHA-256 `de4246e5c9fdf254f6f25fdd5de80149d451d19cfb206fdf294fbbfb26cdc203`.
Root's rendered-pixel review still rejects the candidate's remaining aliasing.
