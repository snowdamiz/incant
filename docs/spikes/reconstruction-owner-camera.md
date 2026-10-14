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
record and considers at most 25 old output texels. Each old center is projected
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

This is a small-motion experiment, not arbitrary camera reprojection. The 5x5
search can miss history after large displacement; it then stays current-only.
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
