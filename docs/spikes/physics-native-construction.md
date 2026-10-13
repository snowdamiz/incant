# Private exact native storage and rigid fields

This default-off `incant-checked-restore` layer constructs private native
Arena/Coarena/Graph storage and regular body/collider fields from checked inert
records. It exposes no public constructor or installer. NumericFrame selects
body/collider rows through the exact previously joined witnesses. The public
runtime still refuses physics saves.

Arena generations, free links, occupied payload rows, Coarena invalidated/stale
payloads and graph adjacency/removal order survive construction. This first
constructor rejects arena generation MAX-1 and MAX; future generation-changing
operations still need an authenticated enclosing-owner fence. It preserves saved
float bits, signed zeros, sentinel values, material flags and ordered attachments.
Only admitted box/ball/Y-capsule shapes are constructed, without compound or soft
fallbacks. It does not use serde input to build a native object.

One shared allocation permit is charged before constructing any staged owner.
Vec uses fallible exact reserves, including an exact capacity check because
native Arena::clear observes capacity. Scalar Box/Arc allocations are charged
before calling stable Rust constructors, which have no fallible allocator API;
process OOM retains the normal Rust abort behavior. Arc accounting includes two
pointer-sized counters and layout alignment in addition to the primitive payload.
Dedicated allocator-forwarding tests measure actual allocation request sizes for
all six dimensional primitives and both boxed mass payloads. These logical
requests are not allocator bookkeeping, committed physical memory or RSS.
Failure drops staged values and does not refund the outer cumulative budget.

The private test workspace is prepared with
`python3 tools/probes/physics-native-construction/prepare.py --output PATH`.
It copies source bytes, changes only the copied optional snapshot path, and seeds
the root lock. Resolve that workspace once with offline Cargo metadata and audit
its lock: existing root package identities must stay exact; only the vendor's
already-declared dev dependencies bincode 1.3.3 and oorandom 11.1.5 are added.
No root normal dependency is added. In the copied workspace run both Rapier
`--lib` tests with `incant-checked-restore,incant-checkpoint,serde-serialize` and
filter `checked_`, explicitly enabling `parallel` plus either
`enhanced-determinism` or `simd8` on both selected Rapier packages. The root `construction_allocations` integration test uses the
existing `native-admission` test feature.

Focused native tests encode/decode the inert body/collider and geometry sections,
construct exact arena/field storage, and compare native serialization bytewise.
They then perform 144 actual continuation steps per dimension covering primitive
shapes, additional mass, sleep/wake, body type, disabled-parent state,
detach/reparent, materials, sensors and signed zero. Remaining world owners are
explicitly trusted typed clones, not reconstructed from inert input. This proves
these fields only. It does not prove full NumericFrame-to-PhysicsWorld installation.
The two one-way test observers are copied from the existing snapshot test
observers; they never deserialize native solver state.

Remaining construction/admission work includes exact island/narrow/broad/CCD
owners and their internal allocations, pending historical bounds, full global
solver arithmetic, generation fences, authenticated completed-step/runtime
ownership, stable IDs and native event state. Fresh-workspace continuation and
partial field reconstruction do not waive those barriers. No whole-world save,
phone, performance or GPU gate is claimed.

Qualification retained in [the manifest](evidence/physics-native-construction.json)
passes 216 snapshot tests per deterministic/SIMD8 mode (24 nonempty groups;
26 including empty library/doctest groups), plus 22 private native controls per
parallel mode. Each native mode performs 144 continuation steps per dimension.
These suites overlap the focused runs; counts are not independent unique cases.
All five solver feature sets were checked from the selected Cargo graphs.
The normal snapshot graph remains std-only and all 968 root lock identities are
unchanged. The copied native test workspace selects 88 of those identities and
adds only its harness and the pinned upstream dev dependencies bincode 1.3.3 and
oorandom 11.1.5.

Strict snapshot all-target Clippy, constructor-feature native library Clippy,
wasm32 compilation, 31 dependency/feature policy tests, runtime graph guards,
formatting and conventions pass. A broader native observer-feature Clippy attempt
is retained as a failure: the inherited `checkpoint_fixed_targets` return type
triggers `type_complexity`. That observer source was unchanged in this constructor producer; the separately
reviewed [alias cleanup](physics-native-observer-lint.md) now closes this lint. The passing native
library check excludes the separate `incant-checkpoint` observation feature; it
is not an all-target or all-feature Clippy claim.

Independent storage/field source reviews are retained. Final source002 only
reorders two module declarations per rustfmt and updates corresponding pin hashes;
the source001 execution logs and eight executable binaries keep their original
identity. Source002 received a fresh constructor-library Clippy/compile and format
check. Source003 is documentation/evidence only. Initial harness, observer-fixture
and formatting failures remain available. Final execution review is accepted at immutable `0a96dc0f`; the root receipt
`17f6b3e6c079e73bf5511c4b410b516f23d4ef82ff04988aefbf72f960a06521`
is retained in the alias follow-up packet. The constructor producer and its
historical failure logs remain unchanged.
