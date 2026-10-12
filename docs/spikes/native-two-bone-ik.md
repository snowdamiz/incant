# Retained two-bone IK CPU integration

The runtime cooked-model animation player accepts an explicit bounded list of
rotation-only two-bone constraints after clip, crossfade or weighted sampling.
It changes the same retained local/global matrices and skin palettes used by
`SkinPose`, `skin_position` and `mesh_local_palette`. This is a CPU integration;
there is no authored IK component, native world control, SDK binding or rendered
acceptance in this slice. Existing sample methods and cooked bytes stay compatible.

## Contract

`TwoBoneIk` identifies `root`, `middle`, `tip` by cooked model-node index, with an
immediate parent relationship. The root may have non-joint ancestors. `target`
and `pole` are model-space points, before any outer scene entity transform. The
pole chooses the bend side. Weight is finite in `[0,1]`; it blends the two local
quaternion rotations on their shortest arcs, not the tip position. Rotations
use glam's column-vector matrix composition and `(x,y,z,w)` quaternions.

`AnimationPlayer::apply_two_bone_ik` accepts up to 32 constraints. The slice is
ordered: a later constraint observes earlier staged results, including overlapping
chains. All local/global matrices and all palettes publish in one swap only after
the full batch succeeds. Refusal preserves the complete previously accepted pose.
Sampling again starts from the authored rest/channels, so repeated per-frame IK
should follow sampling; targets are not secretly retained as simulation state.

Only root and middle linear columns rotate. Local translation columns, scale
lengths and every other local matrix remain unchanged. The tip has no orientation
target. Root/middle must be editable TRS nodes; their effective parent/local/global
frames must be nonsingular positive uniform similarities. Orthogonality and
relative uniformity use a dimensionless `1e-6` tolerance for decoded f32 transforms.
Reflected, nonuniform and sheared chain frames refuse explicitly. Fixed-matrix
ancestors are supported when their effective frame meets that contract. This
restriction does not change ordinary sampling or deformation capabilities.

## Geometry and degeneracy

For lengths `a,b` and target distance `d`, reach clamps to `[abs(a-b),a+b]`.
Normalized lengths avoid squaring large magnitudes. The knee's axial coordinate
is `(a*a-b*b+d*d)/(2*d)` and its perpendicular coordinate is
`sqrt(max(0,a*a-x*x))`. Neither bone stretches. Zero-distance equal-length targets
fold fully; unequal lengths clamp to minimum reach. A target coincident with the
root uses the old tip direction, or the first bone when the tip is also coincident.

A collinear pole falls back to the current bend; if that too is collinear, the
least-aligned Cartesian axis supplies a deterministic perpendicular (fixed ties).
Projection magnitude at most `1e-10` is treated as collinear. An ambiguous half
turn uses that bend plane instead of an unrelated twist axis. Weighted local
half-turns canonicalize an absolute quaternion scalar within `1e-12` of zero,
using the first significant XYZ component for a stable sign. Zero bone length,
unrepresentable differences/lengths or nonfinite composed palettes refuse. There
is no promise of continuity when crossing a mathematically ambiguous pole or
coincidence, and no cross-device bit-exact numerical claim.

## Retained work and open integration

The operation reuses the player's two existing pose buffers and fixed-size stack
values. It adds no retained heap allocation. Work is at most 32 hierarchy/palette
passes over the already bounded 4,096 nodes and 4,096 skin joints. This is logical
work accounting, not a shipping-device timing or RSS measurement.

The next authored route must bind chain indices to the exact model asset,
transform scene-space targets into model space, stage bounded native controls,
and feed them after the graph sample and before the canonical consumer publishes.
Save/SDK semantics, joint limits, stretching, general affine chains, root motion,
GPU skinning and Claude animation-feel/rendered acceptance remain open. No phase
gate or complete IK/game release claim is made.

## Validation

All 118 affected asset/runtime tests pass (18 result groups, no failures or
ignored cases), including ten new IK cases. These cover 620 3D reach controls,
source-deleted cooked load and actual skin deformation, whole-batch rollback,
both clamp limits, coincidence and pole degeneracy, scaled ancestors and local
rotations. A 512-iteration weighted-sample/ordered-solve/deformation/refusal loop
measures exactly zero allocations. Scoped all-target/all-feature Clippy, default
audio-only and wasm32 visual-reader compilation, lean graph, format, conventions
and diff checks pass. These are CPU checks, not shipping/device measurements.

[Exact source/build identities, commands, retained logs and initial test failures](evidence/native-two-bone-ik.json).
