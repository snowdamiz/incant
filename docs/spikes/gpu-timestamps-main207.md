# GPU timestamps on current main207

The opt-in renderer timestamp path is composed with actual main207 (`9aa8165f`),
preserving the corrected producer `9da53be0`. The checked source was `7e6ac08b`;
joining actual main at `5686cd2d` leaves its complete tree unchanged. Final independent
integration review is pending before publication.

Ten scoped commands pass against 2,906 unchanged implementation/build inputs:
default renderer/GPU-host tests (103 CPU cases), the full existing cooked feature
combination (130 overlapping CPU cases), all-target Clippy, native-window consumer
compile, default/cooked wasm32 compile, dependency guard, formatting, conventions
and diff checks. Each test configuration intentionally ignores 94 GPU cases.
No GPU or native-window process ran in this integration.

Fifteen of eighteen timestamp source/test files are byte-identical to the corrected
producer. Three reviewed composition files preserve main's retained-motion pass,
initialize its optional owner alongside the timing graph, and document timing
coverage. All other implementation/build changes remain exact main207 bytes;
Cargo.lock and dynamic-resolution/controller consumers are unchanged. The packet
rehashes all 148 retained producer evidence records against their raw originals.

The prior Claude review executed 92 default-feature GPU/regression cases on Apple
M5 Pro Metal and recorded identical timing-off/on PNG bytes. Three raw lit samples
exposed earlier concurrent InitializeScene work outside the former first/last
encoding-order interval. The accepted CPU decoder now uses the extrema of all
validated pass pairs and refuses malformed or ambiguous-wrap intervals. That
final decoder correction and this main composition have CPU evidence; the original
GPU execution remains scoped to `f33495b2`. Claude owns the recorded appearance
review; this integration does not introduce a new pixel judgment.

Exactly seven pass groups are instrumented. The frame envelope spans the earliest
measured start through the latest measured end, including intervening gaps. It
does not measure complete-frame GPU cost or attribute uninstrumented work. The
separate retained-motion pass has no query pair. Five browser compile warnings
for native-only timing members remain recorded; browser timing admission refuses
explicitly. Native-window execution, other native backend execution, reference
phones, performance and complete-profiler gates remain open.

The [machine-readable packet](evidence/gpu-timestamps-main207.json),
[assertion-only collector](evidence/gpu-timestamps-main207/collect.py), and
[producer review](evidence/gpu-timestamps-main207/producer-final-review.json)
preserve exact inputs, commands, lossless logs and evidence scope.
