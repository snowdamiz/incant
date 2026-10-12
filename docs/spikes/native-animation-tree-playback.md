# Authored nested animation tree playback

Stable-ULID animation trees now connect the command bus, captured model assets,
cooking, native graph clocks, copied TypeScript observations and the retained
weighted model-pose consumer. This extends the numeric evaluator described in
[the tree core](native-animation-tree-core.md). It does not complete animation:
timed transitions involving blended states, layering, IK, root motion, rendered
acceptance and device measurements remain separate requirements.

## Authoring and runtime contract

`AnimationGraph` states can contain `motion: { kind: "tree", cycle_seconds,
tree }`. The tree names a root ULID and maps stable node ULIDs to clip/rest,
piecewise 1D or rectangular bilinear 2D nodes. Node IDs are disjoint from the
graph's parameter, state and transition IDs. References must resolve; unreachable
nodes, cycles, excessive depth and neighborhoods exceeding eight simultaneous
contributions fail validation. Full command transactions, Undo and Redo retain
the same typed project representation as other graph edits.

Cooking resolves ULIDs to deterministic child-before-parent numeric tables and
binds clips and parameters to the exact captured model and graph. Trees use
cooked scene version 15; scenes without trees retain their prior encoding. The
binary reader repeats bounded reference, topology and model-context checks. It
rejects a version-15 declaration without a tree and malformed tables before any
world becomes usable.

Committed graph clocks advance the shared normalized phase. Leaves sample that
phase against their own clip durations; rest always samples time zero. Explicit
master pause, negative or zero speed, natural nonlooping completion and immediate
state selection preserve their existing meanings. The evaluator emits reverse
topological sample order. Equal clip/time contributions coalesce and normalized
weights remain canonical even if tiny contributions underflow to zero. Discarded
array slots are cleared so complete-value equality agrees with logical equality.

The immutable numeric tables remain shared. A tick evaluates bounded numeric
state without document cloning, JSON, schema validation or source access. World
save version 6 already retains the tree's state selection, phase, parameters and
pause/elapsed clocks; the complete pose is derived from those values on restore.
No tree-only checkpoint version or serialized model copy is added.

## Observation and consumer semantics

Native snapshots carry a bounded complete pose in addition to their scalar
representative. TypeScript exposes owned `poseOffsets`, `poseClips`, `poseSeconds`
and `poseWeights` columns, with a graph `weighted` marker. Changing the returned
arrays cannot mutate the runtime. Existing Wasm observation versions 1–3 refuse
weighted trees explicitly; the additive version-4 SDK has a separate review and
ordinary exported-module proof.

The canonical native render binding carries every sample into the existing
retained local TRS sampler. Translations/scales use weighted means and rotations
use the documented hemisphere-aligned quaternion mean. Existing two-pose SLERP
keeps its behavior. A simultaneous weighted pose and legacy pair, or a mismatched
scalar representative, is refused rather than silently selecting one input.
CPU numerical sampling and binding rollback are tested here; GPU appearance is
not implied by these results.

## Local and ordinary shipping verification

The frozen implementation passes 665 affected CPU tests across 117 result groups,
all-target Clippy, wasm32 compilation, runtime dependency guards, a fresh headless
build, exact full-registry schema generation, SDK generation and strict TypeScript
checking. The 82 deliberately ignored renderer GPU tests were not executed.

The public command probe authors two graph entities, imports a real seven-clip
model, and observes eight simultaneous contributions including rest. An
independent tensor-product and integer-quarter-clock oracle checks 228 selected
vectors and every full sample record for all 56 ticks. It also checks atomic
Undo/Redo, unchanged authoring during Play, copied observations, script state,
save and fresh-process journal/source recovery. This preflight passes. The first
attempt's incorrect forward-order oracle is retained as a harness failure; actual
weights, times and metadata already matched the independent expectations.

An ordinary host-native shipping export passes three fresh 56-tick runs and six
split/continuation pairs after deleting authoring inputs, private build storage
and external source maps. Active, paused, rest-completed-paused, reverse,
second-state and zero-speed continuations preserve exact final saves, full logs,
PCM and CPU render-input summaries. Repeated full processes also produce equal
reports, saves and PCM bytes. Both corrupted-checkpoint and wrong-observation
requests fail with their precise diagnostics and publish neither checkpoint nor
PCM. The 1,170 frozen source/build/probe inputs remained unchanged throughout.

The actual executable, seven-file runtime package and original outputs are
retained outside the repository; the evidence packet contains their identities,
commands, full observation reports, input/output artifacts and build diagnostics.
The observed player uses cooked QuickJS bytecode with no authoring/runtime source
dependency. This is a CPU headless package proof on macOS; it does not certify
rendered deformation or another platform. Current-main integration is still
required before merging.

Earlier broad-check harness failures (a stale unknown
version sentinel and use of the document-only schema generator against the full
host registry) are retained alongside the corrected results. No renderer,
reference-phone, complete SDK, Phase 1 or release gate is claimed.

[Exact sources, checks, independent reviews and retained package evidence](evidence/native-animation-tree-playback.json).
