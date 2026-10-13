# Checked inert arena and generation records

This extends the [inert physics format](physics-inert-snapshot.md) with exact
Rapier 0.36.0 arena layout and generation-tagged Coarena rows. It remains a
standalone normal-std-only library with no live constructor, solver consumer,
world/session version change or physics-save admission. `PhysicsUnsupported`
remains in place. Body/collider values and contact payloads are not validated by
this component.

## Arena structure

An arena record retains its global generation, free-list head, stored live count
and every ordered slot. Free slots retain their exact successor. Occupied slots
retain their generation and a row in the future separately checked payload table.
The payload mapping is a bijection over `0..live`; it is not sorted or rewritten.
The free chain must visit every free slot exactly once and no occupied slot.
Cycles, unreachable free slots, out-of-range successors, wrong stored counts and
duplicate/missing payload rows are rejected. No dependency indexing, recursion,
panic catching or live construction is used to establish these properties.

Slot generations cannot exceed the arena's global generation. `u32::MAX` is
explicitly refused as an arena generation: Coarena reserves it as invalid, and
the pinned arena increments its generation without an overflow guard on removal.
The last non-sentinel generation remains an inert representable value; admitting
it does not prove that subsequent native removals can continue indefinitely.
Any future mutable restore needs a checked exhaustion policy before solver
mutation. At least one slot is required because the pinned zero-slot slow insert
would call `reserve(0)` and fail to produce an insertion slot.

Checked lookup requires an exact occupied `(slot, generation)` and returns its
payload row. Stale, invalid, out-of-range and free handles return no row. The
validation witness keeps its record private and immutable.

## Coarena rows must not disappear

The pinned Coarena retains `(generation, payload)` for every position, including
default/invalidated generation `u32::MAX`. Removal installs a caller-supplied
payload, which need not be zero. `get_unknown_gen` can observe that value. A valid
old generation can also remain after its owner was removed or its arena slot was
reused. The format preserves the entire ordered vector and a bijection over every
payload row, including these invalidated and stale values.

Validation receives and borrows one immutable arena witness. Dimension and build
identity must match, vector length cannot exceed the arena's slot count, and a
non-sentinel row cannot claim a future generation relative to that arena or its
current occupied slot. Readback classifies rows as current, stale or invalidated.
Classification is structural; it does not authenticate the world/body owner or
prove that a contact/adjacency payload is safe. Those relationships must be
validated by the enclosing owner and payload families. This component cannot
replace those checks by treating a stale row as absent.

Coarena generation-aware lookup preserves access to matching stale generations,
as the native type does. Native `Coarena::get` also returns an invalidated payload
when given a matching `u32::MAX` generation. Our checked API deliberately refuses
that sentinel handle, a stricter admission boundary verified against native calls;
all invalidated payloads remain available through explicit generation-agnostic
row access. Current owner access requires the arena lookup or current-row state.

Trusted observations establish another relevant pinned behavior:
`Arena::clear` does not increment its global generation. Therefore a free slot's
retained Coarena generation can equal the global generation and still be stale.
The validator preserves that state. It does not rewrite generations or claim to
eliminate the underlying clear/reuse alias behavior.

## Bounded codecs

Both frames use little-endian version 1, a dimension/f32 marker, and the existing
independently expected 32-byte build identity. Unknown tags, reserved fields,
versions or identities fail explicitly. Header/count/total-size admission occurs
before vectors are allocated. A typed error leaves no live dependency object.

| Frame | Header | Ordered rows |
| --- | --- | --- |
| `ICARN001` | 64 bytes: common identity, global generation, live count, slot count, free head, zero reserved word | 12 bytes: free/occupied tag and two words; free padding must be zero, absent successor/head is `u32::MAX` |
| `ICCOA001` | 52 bytes: common identity, row count, zero reserved word | 8 bytes: exact generation and payload-row reference |

The previous 262,144-item, 32 MiB frame and 64 MiB cumulative requested Vec payload
limits apply. The arena validator allocates bounded one-byte slot and payload
marks. Coarena allocates one-byte payload marks. Each free link or row is visited
at most a bounded number of times. Decoders charge record storage and validator
scratch; encoders charge output. One shared `Budget` is required across sections.
Allocator rounding/RSS, caller-owned inputs and independently supplied records
are not covered by that accounting.

These inert layout frames omit payload contents and private allocator capacity.
No live object is reconstructed, so they do not demonstrate post-load insertion,
removal or `clear` continuation. The latter can inspect the dependency Vec's
capacity. Exact construction/continuation must account for that behavior before
this format is composed into a complete owner restore.

## Evidence and remaining scope

Thirty CPU tests pass across five nonempty groups, including twelve new arena
tests. Two new tests observe actual pinned 2D/3D arenas and Coarenas through
trusted one-way serialization, compare complete inert records and canonical bytes,
and compare every live/stale/invalidated lookup to the trusted native values.
They exercise capacity exhaustion, reserve/growth, removals, reuse, retain and
clear. They never deserialize a live solver or dependency structure.

Hostile tests cover free chains, rows/counts, generations/sentinels, every
truncation, malformed headers/tags/padding, mismatched identity/arena witness,
cumulative cross-section allocation refusal and byte mutations. All 965 existing
locked package identities are preserved. The only resolution delta is dev-only
Rapier observation and serde dependency edges; the normal crate remains std-only.
All-target Clippy, normal wasm32 compilation, the selected std-only graph,
existing physics runtime dependency guards, formatting and convention checks
also pass. Independent source review accepted all seventeen frozen inputs;
final evidence review remains pending.
[Retained evidence](evidence/physics-inert-arenas.json) binds both frozen source
iterations, the pinned dependency files, command receipts and lossless raw logs.

This is not a whole-physics or session-save result. Contact and intersection graph
adjacency, warm-start values, islands, solver conflict ownership, broad-phase
pending state, geometry/payload correspondence and native stable-ID/body/collider
relationships remain required. All must validate together before any future
live solver is installed. No GPU, shipping package, device or performance gate
is claimed.
