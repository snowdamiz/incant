# Bounded multi-pose animation sampling

`AnimationPlayer::sample_weighted` mixes up to sixteen independent clip samples
in local translation, rotation and scale space, then composes the hierarchy and
skin palettes once. Missing channels start from the authored rest pose; static
matrix nodes remain unchanged. This is the numeric prerequisite for multi-input
animation trees. Authored nested/2D trees, graph transitions between blended
states, complete SDK observations and editor controls remain separate work.

Weights are finite, nonnegative relative contributions with at least one positive
entry. Scaling by the largest weight before summing supports both maximum finite
weights and subnormal-only sets without overflow. Every clip, clock and weight is
validated, including zero-weight entries. A single positive contribution follows
the original sampler exactly. Translation and scale use a weighted arithmetic
mean. Rotations use a normalized quaternion mean, aligned to the largest-weight
sample's hemisphere; clip and clock order break equal-weight ties. Canonicalizing
quaternion signs also handles exactly orthogonal inputs. This method uses nlerp;
the existing two-pose `sample_blend` retains its spherical interpolation behavior.

The unpublished local matrix buffer supplies fourteen f64 accumulation lanes per
node. Finished local TRS values replace that scratch before hierarchy composition
can publish. Neither startup memory nor per-sample allocation increases. A late
nonfinite hierarchy or palette calculation leaves all published poses unchanged,
and the next accepted sample can reuse the same player.

All 108 affected asset/runtime tests passed in 18 groups, including an independent
four-pose hierarchy and skinned-vertex oracle, antipodal/orthogonal rotations,
extreme weights, exact endpoints, malformed unused inputs and late overflow.
1,024 sixteen-input samples followed by palette/deformation operations allocated
nothing under the thread-local allocator instrument. A fresh child process
repeated the numeric oracle from a cooked model after glTF and source-buffer
deletion. All-target Clippy, the wasm32 visual-reader compile, normal runtime
dependency guard, formatting and conventions passed.

[Frozen source and checks](evidence/runtime-weighted-pose.json) record the scope.
No rendered appearance, reference-phone timing or Phase 1 gate is claimed.

## Current-main integration

The combined sampler and tree core passes 310 CPU tests in 48 groups, all-target
Clippy, wasm32 and both runtime dependency guards. Integration of actual main
#143 preserves all 1487 tested build inputs and the twelve accepted source/test
files byte-for-byte. [Retained checks and independent review](evidence/animation-core-main142.json)
record that boundary; authored/runtime tree wiring and rendered/device acceptance
remain separate.
