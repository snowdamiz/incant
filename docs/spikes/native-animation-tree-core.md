# Native nested animation tree evaluator

This is the numeric evaluator needed by PLAN.md Phase 1 blend trees. Its original
isolated-core checks are recorded here. The later [authored playback connection](native-animation-tree-playback.md)
adds cooked scenes, native clocks, render bindings, TypeScript observations and
checkpoint continuation with separate end-to-end evidence. Existing clip/flat1D
graph behavior is unchanged.

## Admitted numeric contract

A compiled tree contains at most 64 nodes in child-before-parent order; the last
node is its root. Every node must reach that root. Child ordering prevents cycles
and fixed arrays bound the work. Parameters retain canonical sorted stable IDs,
finite ranges within ±1,000,000, and valid initial values. Model clip indices and
finite nonnegative durations are checked before admission.

Nodes are a clip or bind pose, a piecewise 1D blend, or a rectangular bilinear 2D
grid. Each axis has 2–8 strictly increasing thresholds inside its parameter range.
Grid children are row-major. Values outside a grid clamp to its edges, while
values outside the declared parameter range fail. Leaves share normalized phase
but scale it by their own durations. Bind pose and zero-duration clips stay at 0.

Nested trees are bounded to depth 8 and conservatively at most 8 simultaneous pose
contributions. Each adjacent 1D pair and 2D cell is checked using the sum of child
bounds. Shared children can therefore overcount; this deliberate admission bound
may reject a graph whose distinct output could fit. Trees can contain more than 8
leaves overall as long as every sampled neighborhood meets the bound.

Evaluation walks the admitted nodes once in reverse order into 64 stack weight
slots, coalescing equal clip/time entries. It does not recurse, allocate, clone
project data, serialize JSON or validate authoring schemas per step. Two returned
poses can fade into at most 16 contributions. The destination weight lies in [0, 1];
endpoints retain exact input values, and distinct phases of the same clip remain
distinct. These are local-pose weights intended for the retained weighted TRS
sampler, not blended final matrices. Its quaternion averaging contract remains
different from the unchanged two-input SLERP path.

## Verification

The full native runtime suite passes 200 tests in 29 groups, including 11 new
observable tree cases. Independent tensor-product weights, nonuniform rectangular
cells, corners/clamping, shared children, normalized times, sixteen-sample fades,
capacity failures and immutable invalid-input handling are covered. 4096 accepted
nested sampling/fade steps perform zero allocations under thread-local allocator
instrumentation. All-target Clippy, wasm32 compilation, authoring-free dependency
checks and formatting pass. Independent source review accepts this numeric scope.

[Exact sources, commands and retained logs](evidence/native-animation-tree-core.json).
These are local CPU results, not a frame-time benchmark or a reference-phone gate.

## Connection status

- Stable-ULID authoring, checked binary tables, committed sampling, complete
  renderer/TS observations and ordinary source-deleted continuation are now
  verified locally in the linked playback scope; its current-main integration
  remains pending. Older observation APIs explicitly refuse weighted trees.
- Independently advancing outgoing blended states and nonzero fades still need
  connection; the numeric sixteen-contribution helper is not that implementation.
- The Rust version-4 SDK has separate host and public shipping evidence; it is
  not certified by this numeric core's tests. Additional graph save state must
  coordinate with the audio and IK format owners.
- Obtain Claude rendered acceptance and actual required device measurements; IK,
  root motion and the complete animation/Phase 1 requirements remain open.
