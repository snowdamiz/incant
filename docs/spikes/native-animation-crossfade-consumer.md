# Canonical native crossfade consumption

This slice connects the committed native two-clock transition to the existing
canonical render input and retained CPU deformation. It does not add a second
animation clock, an animation graph or another render path.

`SpatialNode.animation` and `animation_blend` come from the same committed
`AnimationState`. `MeshInput` carries the destination sample plus the optional
outgoing sample and destination weight. An absent primary sample still denotes
the authored rest pose. The cold binding path includes an already-active
transition, and ordinary motion updates both values through the same preallocated
undo buffer. Consumer errors and Rust unwind restore both along with transforms;
structural rebuilds publish a complete candidate only after acceptance.

The canonical model consumer validates both clip/time pairs and the finite
zero-through-one weight, including unused endpoint inputs, before calling the
existing `AnimationPlayer::sample_blend`. Invalid indices, negative/nonfinite or
out-of-duration times and noncanonical rest samples refuse. Both canonical cold
resolution and retained stage refuse either animation field on a static model.
The authoring still-scene adapter explicitly supplies no transition.

Both immutable model inputs and the two TRS scratch arrays are already included
in `AnimationPlayer::retained_bytes`; the existing aggregate retained-animation
admission uses that exact value for both live/staging players. This adapter adds
one fixed-size optional blend value per mesh/undo row, without per-frame vectors,
asset reads, cache loads, source parsing or JSON. The existing CPU deformation,
orientation refusal, bounds, materials, shadow path and GPU double staging remain
the consumer. Mixed skin orientation and singular deformation remain explicit
refusals. This is not a new whole-heap, driver, frame-time or mobile-memory claim.

## Numerical checks

The source-generated mathematical fixture has two joints, actual weights and
inverse-bind matrices, a non-joint ancestor, two mesh instances sharing a skin,
and independent translation/rotation/scale clips. Tests remove source and cache
before canonical pose evaluation. Five destination fractions compare actual
expanded vertices to an independent TRS/weighted-skin calculation; each fraction
runs 64 measured sample-and-deform calls with zero allocator calls. Endpoint
refusals preserve the published local/global/palette arrays and allow retry.
A reflected-scale blend checks actual deformed positions and orientation on both
sides of zero; the finite singular midpoint is a valid sampled matrix but refuses
canonical vertex deformation, and a subsequent valid sample succeeds. The initial
test wrongly expected sampling itself to refuse; that failed run is retained.
Retained GPU publication remains a separate pending execution check.

A binary native world starts an actual crossfade before host preparation. After
source/cache deletion, 60 fixed ticks retain the same input/source allocations and
match the native state plus an independent translation result; another instance
stays paused. Motion rejection, Rust unwind, structural rebuild refusal/retry and
explicit REST selection verify lifetime/publication behavior. Allocation
instrumentation is limited to adapter synchronization or CPU sampling/deformation,
not world stepping, initialization, callbacks generally or complete rendering.

## Remaining acceptance

This packet contains CPU execution only. GPU numerical submission/rollback and
Claude appearance acceptance for crossfades remain pending; existing ignored GPU
tests do not count as executed checks. Text continues to refuse GPU consumption
on this branch and its drawing work is independently owned by Claude. Native
crossfade TypeScript/Wasm controls, authored blend trees/state machines, IK,
retargeting, root motion and reference-device budgets remain separate work.

The affected six-package run passed 349 tests (70 GPU tests ignored); the final
focused run passed 17 crossfade-related tests, including the added reflection
case, for 350 distinct passing tests. Eight-package all-target Clippy with the
optional native-window compile feature, wasm32 render-data/native compilation,
normal dependency guards, formatting and conventions passed. Exact sources,
commands/log identities and the preserved failed test are recorded in
[evidence/native-animation-crossfade-consumer.json](evidence/native-animation-crossfade-consumer.json).

The final independent CPU-scoped review found no actionable issue and verified
all 16 source identities and recorded logs. Its exact receipt is preserved in
[evidence/native-animation-crossfade-consumer-review.json](evidence/native-animation-crossfade-consumer-review.json).
GPU execution and appearance acceptance remain open.
