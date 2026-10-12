# Cooked rigid root-motion sampling

`RootMotionSampler` samples one cooked clip's selected TRS node in its local
coordinate frame. Startup validates the immutable borrowed model; sampling
retains no gameplay clock and copies no tracks. LINEAR translation and shortest-arc
rotation, STEP endpoints and missing-channel rest values reuse the existing
skeletal channel arithmetic. The glTF importer still refuses CUBICSPLINE.

The output is a rigid delta that right-multiplies the previous accepted root
frame. Translation is expressed in that previous frame. Nonunit selected-node
rest scale, any scale track on that node and matrix-only selection refuse
explicitly. Ancestor transforms, model/entity placement and body coordinates are
not incorporated. Selecting a child node intentionally observes its local track;
it does not infer a character's physical root.

Clamped queries accept finite endpoints and clamp each to the clip interval.
Zero-duration clips produce identity deltas. Looping queries require a positive
duration and explicit signed integer cycle counters plus a phase in
`[0, duration)`. A clip may last at most 86,400 seconds; this is a sampler admission
bound, not a change to the cooked model format. The checked cycle difference is
bounded to `2^32`, requiring at most 33 repeated-squaring steps. Absolute cycle
counters never pass through a floating-point conversion. Invalid times, phases,
cycle spans and cooked inputs return typed errors.

For local pose `P(t)`, define `F(t) = inverse(P(0)) * P(t)` and cycle displacement
`C = F(duration)`. The unwrapped trajectory is `C^cycle * F(phase)`. A query returns
`inverse(F(from)) * C^(to.cycle - from.cycle) * F(to)`. This computes small deltas
without constructing huge absolute positions and preserves the order of rotation
and translation across loop boundaries. Negative powers produce reverse motion.
The next cycle's displacement turns with the previous cycle; adding wrapped
translations would give the wrong path.

The sampler owns only fixed-size state and references to the admitted channels.
No scratch vectors, source IO, project documents or model validation enter a query.
Startup uses the existing cooked-model validator, which may allocate. The query
error variants carry no allocated diagnostic. All ten focused tests and 128
assets/runtime tests across 19 groups pass,
with no failed or ignored cases. All-target/all-feature Clippy, wasm32 visual
compilation, the authoring-free dependency guard and formatting checks pass.
The allocation counter measures zero allocations across 4096 iterations containing
8192 accepted queries and 4096 typed refusals. Startup allocation is excluded.
The numerical oracle independently composes matrices over 243 forward/reverse
cycle intervals; additional tests cover real source-deleted LINEAR/STEP glTF,
CUBICSPLINE import refusal, quaternion signs, angle wrapping, nonidentity root
basis, missing tracks, duration/cycle bounds and invalid cooked values.

[The evidence inventory](evidence/native-root-motion-core.json) retains the exact
commands, frozen source hashes and logs. The first test build failed on a fixture
callback signature before executing tests; that failed log is preserved. The
corrected fixture and added basis control pass focused and broad runs. Independent
source and retained evidence review accepts the frozen implementation without
findings. Current-main integration remains publisher-owned and pending.

This is a CPU numerical foundation. Authored root selection, native graph and
clock integration, full pose motion extraction, physics/body movement, blending,
save/restore and public TypeScript/Wasm controls remain open. It adds no scene or
save format revision and makes no rendered, reference-device, performance or
Phase 1 completion claim.
