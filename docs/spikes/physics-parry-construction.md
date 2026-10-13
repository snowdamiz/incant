# Shared allocation and exact typed Parry BVH construction

The `construction` feature of the std-only snapshot crate exposes the existing
cumulative allocation plan, budget and non-Clone permit. The implementation is
mechanically identical after documentation/import/visibility changes; Rapier's
private module re-exports it. Failed staging does not refund the outer budget,
integer overflow refuses before allocation, and ordinary vectors use fallible
exact reserves. Box/Arc and the pinned VecMap API retain ordinary infallible
allocator behavior; precharges measure requested payload, not RSS or recoverable
OOM. This utility has no solver dependency and constructs no world.

Parry's default-off `incant-checked-restore` feature adds an optional snapshot
edge; Rapier's same feature forwards it. Ordinary selected runtime dependencies
remain unaffected. Existing lock package identities remain unchanged; only the
two optional Parry-to-snapshot edges are added. Both selected native mode graphs
and the ordinary runtime graph were checked; the ordinary graph contains neither
the snapshot dependency nor either experimental checkpoint feature.

Four public Parry functions are the narrow cross-crate seam: strict and pending
allocation plans and constructors. They accept only immutable `ValidatedBvh` or
`ValidatedPendingBvh` witnesses, an independently supplied exact identity and the
same allocation permit. Raw-record helpers stay private. Every node, signed-zero
bound bit, parent, leaf-map tail/count, free-stack order and optimization counter
is reconstructed directly without sorting, rebuilding or refitting. No unsafe or
native serde deserialization is introduced. Parent/global/shape correspondence
and completed-phase authentication remain the enclosing owner's obligations.

Pending construction preserves stale ancestor bounds and flags exactly. Its
result must undergo the matched owner's normal pending processing before query
use; it does not promote the witness to strict/query-ready. Tests compare a
captured whole-frame pending tree and a separately mutated native tree, preserve
its exact bytes, require strict validation to refuse the pending state, then
compare ordinary refit and optimization against the native original. Test AABBs
are deliberately independent of shapes and do not establish geometric coherence.

Focused tests cover 2D/3D typed encode/decode/construction, sparse empty tails,
remove/reuse/refit/optimization order, shared/short budgets, mismatched build and
dimension, truncated/invalid graph bytes, and unnormalized pending flags. All
other world owners remain outside this component. There is no public world
installer, stable-ID/event restoration or `PhysicsUnsupported` change.

Focused source controls pass: six 2D/3D BVH tests, 226 paired native mutation
steps per dimension, and one migrated allocation-budget test. Pending trees
compare exact pre-refit fields, then one native refit and 32 optimization calls.
Initial unused-result/test-private-access compile failures and fixture capture
at unfinished native update phases remain retained. Choosing a non-root leaf
is necessary to exercise stale ancestor bounds in both dimensions; validation
was not weakened.

Source001 received independent root acceptance. Final source003 changes only
test syntax: a direct return replaces a redundant binding, and a lexical scope
replaces dropping a non-Drop permit. Both original Clippy failures remain in the
packet. The implementation and constructor semantics are unchanged. Final docs
are recorded separately from the compiled source generations.

Both deterministic and fast8 modes pass 225 snapshot tests (27 summary rows,
including one empty doctest group) and 26 private native constructor tests. The
private tests include inherited body/island controls, including 156 paired
island steps per dimension per mode; these are not additional new BVH coverage.
The six new BVH controls include 226 paired native mutation steps per dimension
per mode and retain pending-state processing controls. The final test-helper
revision passes the six focused tests; the final allocation-test revision passes
its isolated control. All counts overlap the corresponding full suites.

Strict native-library Clippy passes in both modes; snapshot all-target Clippy,
default and constructor-only wasm32 compilation, both std-only normal graphs,
31 probe/physics policy tests, runtime dependency guard, workspace formatting,
owned vendor formatting and conventions checks pass. No all-vendor format or
all-target vendor Clippy claim is made. Native test binaries are retained with
their actual source generation and executable mode. The lossless
[evidence manifest](evidence/physics-parry-construction.json) retains every
failed attempt and command receipt. Final independent evidence review is pending.

The next proposed component is private broad-phase manager construction using
the exact pending tree and Coarena witnesses. Its pair-map allocation accounting
and scratch overwrite proof still require review; it is not implemented here.
Narrow-phase and CCD/pipeline construction, complete derived-data arithmetic,
authenticated whole-owner installation and stable-ID/event composition remain
open. This packet does not enable public physics saves, GPU or device gates.
