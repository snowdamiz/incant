# Exclusive native checkpoint capture

This default-off `incant-checked-restore` route adds direct typed native capture
and an opaque in-memory checkpoint. It does not admit arbitrary persisted bytes,
replace outer runtime scenes, or enable public physics file saves. The outer
stable-ID/configuration/event joins are a separate required integration.

`CheckpointOwner::for_simulation` always creates ordinary native simulation.
Its immutable `get` preserves completion; `access_mut` invalidates completion
before returning the native borrow. `step` poisons before advancing and only
reissues completion after a normal, positive finite step. An unwind remains
poisoned. Unsupported capture modes refuse checkpoint operations without newly
preventing ordinary simulation. There is no mutable Deref, adoption of a raw
world, completion boolean, or authority-preserving owner replacement API.

Capture checks the retained default-dispatcher Arc identity, exact unsupported
joint/soft history gates, deferred ownership, quarantine reports, and pending
mutation journals. These gates do not clear reports or pretend that zero counters
authenticate completion. The short-lived capability holds an exclusive borrow.

All records come directly from pinned native fields. Arena generations/free
order, invalidated Coarena payloads, graph adjacency order, contact/solver caches,
BVH pending flags, island stamps, CCD cache
states, retained pipeline verdict and impulse assembly epoch are preserved.
Primitive geometry admits exact box, ball and canonical Y-capsule fields only.
Unsupported body/material variants, composite workspaces and soft histories
return typed errors. No production serde-to-live path is involved.

The capture uses one cumulative `snapshot::Budget` for records, validation
scratch and encoding. The shared `WorkBudget` covers the documented logical
validation/solver visit model; neither is an RSS or elapsed-time guarantee.
The existing complete constructor pre-admits one cumulative `AllocationBudget`
and constructs the full candidate before returning it. Known infallible native
VecMap/Box/Arc allocation limits remain unchanged. The source owner is never
replaced by a failed capture or candidate reconstruction.

`NativeFrame` has private fields and no importer or Clone. Its immutable encoded
view is data, not a way to authorize arbitrary bytes. Reconstruction rechecks the
actual linked build, decodes all joins and numerical coherence, and invokes the
existing complete private primitive-world constructor. The explicit native
thread-pool Arc is retained only in memory. Future persisted admission must
supply a reviewed policy rather than infer this capability from the envelope.

Build identity binds the actual compiled Rapier and linked Parry sources, selected
features, target/configuration, compiler report and explicit enclosing workspace
lock. It hashes logical source paths and source bytes, normalizing only the
optional inert dependency path. Ignored upstream vendor locks do not substitute
for the actual workspace lock. The optional already-locked sha2 edges are absent
from ordinary feature graphs; the snapshot crate remains std-only. This identity
is conservative compatibility metadata, not a cryptographic authenticity claim.

Focused implementation evidence is retained under
`~/.codex/incant-evidence/physics-native-capture-001`. Earlier lock-selection,
block-solver, recursive-format traversal and compile failures remain historical;
they are not relabeled as passing results. Scoped CPU qualification and independent
source review pass; final execution review remains pending. No GPU or
device/performance work is part of this slice.

Focused source008 qualification passed 21 capture tests plus one independent
reference test per dimension. The opaque continuation test compares 96 complete
frames; the mutation test compares 40 original/reconstructed steps including
removal, generation reuse, sleep/wake and body-kind changes. The independent
old observer agrees on all 22 encoded sections at 60 admitted boundaries during
64 native churn steps. Both routes explicitly refuse the wake journal at tick 7
and the pre-existing numeric LocalMass domain at ticks 21–23. Those latter states
are real disabled bodies whose historical native mass is retained while their
colliders become DisabledByParent. The validator recomputes from enabled
colliders; this capture slice does not normalize or widen that domain.

Source004 is the first complete compiled export used by the root's separate
outer runtime build (20 focused cases per dimension). Source008 adds independent
reference/churn tests, retaining the earlier test compile and assumption
failures. The final source handoff additionally removes six obsolete non-test
dead-code expectations now that production capture invokes those gates; test
configuration and runtime algorithms are unchanged.

Source009 (`f19f54ac`) passes 182 native tests and 228 snapshot tests in each
deterministic and SIMD8 mode, with no failed or ignored tests. The two native
groups are 90 planar and 92 spatial tests. Snapshot output contains 27 summary
rows, including one empty group. Counts overlap the focused cases above and are
not additive. All 24 broad commands pass: native strict library Clippy in both
modes, snapshot all-target Clippy, default/construction wasm checks, normal graph
isolation, feature guards, formatting and conventions. Each native executable was
copied immediately after its successful mode-specific run; four executable
source009 binaries and their exact logs are retained.

Source010 (`d6ca9867`) only renames the conditional `capture_context` argument
and updates the two vendor pins. Native and wasm strict nonparallel library
Clippy and scoped formatting pass. These compiler checks do not relabel the
source009 test executions or binaries. Independent source009 review is accepted
under receipt `f43b5b74444ac96c269a6c973497f26129befb91a28f553ca867ba2479831737`;
the complete final execution packet still needs independent acceptance.

The lossless packet is
[`evidence/physics-native-capture.json`](evidence/physics-native-capture.json).
Its archive retains exact source generations, commands, historical failures and
the collector. Revalidate the originals without rebuilding:

```sh
python3 ~/.codex/incant-evidence/physics-native-capture-001/collect.py --verify-only
```

The separate outer runtime's stable-ID/event integration results are not counted
as evidence for this native producer. Persisted physics saves remain refused.
