# Native Wasm complete animation poses

The public Rust SDK now observes complete copied native animation poses through
`Tick::animation_poses`. The additive v4 frame keeps the existing two-pose SLERP
route distinguishable from the normalized quaternion mean used by nested trees.
It neither changes interpolation nor drops contributions to fit an older frame.

## Contract

Published operations 0–41 and all previous constants, records, enums, magic and
namespace declarations remain unchanged. `incant_animation_v4` appends
`snapshot_len` (42) and `read_snapshot` (43). Existing controls continue through
the published animation and graph APIs; this slice adds no control opcode.
The older v1/v2/v3 observations explicitly refuse native weighted tree poses.

A frame has a 40-byte header carrying scene ID, row/sample counts and exact
strides. Sorted stable entity IDs identify 96-byte rows. A canonical contiguous
sample table holds 24-byte clip/time/weight entries. Clip indices and clocks
retain native semantics, including the explicit rest pose. All scalar bytes
are copied; no ECS pointer, animation asset, native handle or guest-independent
storage reference crosses the boundary.

Pose kind is explicit: Single is one sample; SlerpTransition and SlerpParameter
are ordered source/destination pairs using the established exact SLERP path;
WeightedQuaternionMean retains every native contribution for weighted local
translation/scale and normalized quaternion averaging. Native weighted poses
are authoritative, with the scalar clip/time serving only as the representative
first sample. Native sample order and f64 clocks/weights are preserved. Unknown
kinds and nonzero reserved fields reject, leaving future grouped operands for an
explicitly supported contract rather than an implicit change in interpolation.

The complete-frame parser validates lengths, ordering, contiguous offsets,
clip/range/numeric domains, flag/transition consistency, pose cardinality,
positive weighted contributions and normalized totals before exposing any
iterator. Repeated clips at different phases are valid; duplicate clip/time
contributions are not. Legacy pair endpoints may contain a zero weight and keep
their operand order. A weighted rest state may be completed.

The frame supports 100,000 animation rows, at most 1,024 weighted graph rows,
16 samples per row and 214,336 total samples. Its maximum is 14,744,104 bytes,
within the existing 32 MiB guest-memory bound. Existing 65,536 host calls and
128 MiB cumulative transferred bytes apply. Smaller initialized scratch is valid
only when the complete frame fits. Full destination range, capacity and quota
checks happen before copying any byte.

The host captures committed animation state at callback entry and lazily encodes
v4 on the first authorized observation. Queued native writes and graph controls
remain invisible to subsequent reads in that callback. An authorized bad read
or a caught SDK scratch/parser error rejects the entire staged transaction,
including numeric edits, structural commands, animation controls and logs.
Stale epochs cannot poison a different active callback. Both actual Wasmtime and
wasm2c hosts share this boundary; gameplay performs no document work or JSON
serialization.

## Verification and remaining scope

Production freeze001 passes the actual Wasmtime/wasm2c suite: 398 tests/doctests
in 70 groups, zero failures or ignored tests, and all-target Clippy. Its separate
three-process development smoke uses the existing bulk fixture; the new weighted
pose behavior is established by the actual-host parity tests. The preflight
passes 107 tests/doctests in 28 groups and all-target Clippy for the public SDK,
ABI and shared runtime, including the exact maximum frame, a synthetic
sixteen-contribution native value, all-or-nothing destination copies, stale
epochs, call/transfer limits and staged transaction rollback. Python checks
pass 234 cases (four existing skips); 22 Node checks, generated files,
conventions and authoring-free/static-player dependency checks also pass.
Independent source review accepted all 28 frozen source/test/probe hashes and
verified that every published ABI entry remains unchanged. Final evidence acceptance remains pending. The shared native constructor
corrections are tracked separately from the unchanged v4 source freeze. The
first union passes 300 tests in 54 groups. After the tail-only canonical-storage
fix, the focused constructor/SDK and cooked-tree checks pass 99 cases,
with Clippy, wasm32 and formatting passing again. Independent review accepts
the constructor correction. The full actual-host proof is not represented as a
rerun after those native-only changes.

The complete proof prepared 151 modules and rehashed all 756 prepared files
before cleanup. The eleven new guests retain 55 Wasm/C artifacts and three
source copies outside temporary storage; those retained files were reverified
after completion. Logs, manifests and review receipts are preserved in the
repository in the [durable evidence packet](evidence/native-animation-pose-sdk.json).
These are local CPU results, not shipping or device measurements.

The real cooked scene supplies eight active tree samples plus ordinary timed
and parameter-driven two-pose blends. The public guest observes their complete
contributions and stages supported controls across twenty ticks; its oracle
uses direct native state, not an ABI parser. Synthetic sixteen-sample frame
coverage proves bounded representation only. Nonzero tree-state fades, composite
SLERP/weighted operands, layered animation, IK, root motion and complete guest
checkpoints remain separate work. This slice does not establish a source-deleted
ordinary shipping package, rendered correctness, a device measurement, full SDK
coverage or any Phase 1 gate.
