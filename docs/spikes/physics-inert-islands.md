# Checked inert island ownership

The std-only snapshot format now joins the checked body, attachment and contact
manifold records to both pinned Rapier island maps. It retains exact map slots,
ordered membership and reuse stacks, and checks reciprocal ownership before any
record can be exposed as validated. This is an inert completed-step component:
there is no live constructor, runtime consumer or physics-save admission.
`PhysicsUnsupported` remains unchanged.

## Native meaning and admission

The active map and persistent map serve different purposes. One active container
is designated awake; other active containers are sleeping chunks. Persistent
islands describe contact connectivity with eager merges and deferred splits.
They are not required to correspond one-to-one to active containers, and a
persistent island need not already be connected when removals await a split.

The record preserves both VecMap live counts and every slot (including trailing
holes), exact free-stack order, the active epoch and awake option, both ordered
body arrays, ordered per-island contact links, the edge-to-link location vector,
constraint-removal counts, split cooldowns, sleeping flags, pending split option,
sleep-scan stamp and bootstrap flag. Missing contact parents retain native
(MAX, MAX) handles; fixed parents retain their actual generational handles.
The location vector may end before the final edge: that tail is unlinked, as in
the pinned implementation. No sorting, rebuilding or counter reset occurs.

Completed-step checks require:

- Exact body generations and both reciprocal island/index pairs. Enabled
  nonfixed bodies appear once in each map. Disabled bodies appear in neither.
  Fixed bodies have no persistent membership, but can retain an active entry
  after a public change from a moving body type to Fixed.
- A complete live/free partition for each map; no duplicate free entry, missing
  hole, forged live count, duplicate member or unused membership payload.
- Nonempty persistent islands. An active container may be empty only when it is
  the designated awake container. Actual native multi-component sleep extraction
  leaves this empty container, sometimes even after every body is removed.
- Sleeping persistent members are asleep and outside the awake container.
  Non-sleeping persistent members are in the awake container; an individual body
  manually put to sleep may remain there until the component can sleep together.
  Retained fixed active rows are not assigned nonfixed sleep semantics.
- Every cached contact link resolves the exact contact edge and current parent
  handles, has a touching solver-manifold row, and has at least one member side
  in its owning persistent island. Every member side agrees on that island.
  Every touching edge with a member side is linked, and every link/location
  resolves back exactly. This uses the native touching predicate (nonempty solver
  contact arrays, including solver clusters), not solver-color qualification.
- A pending split names an existing non-sleeping persistent island. Nonzero
  removal counts and retained cooldown/stamp values are valid inert data.

Body and manifold witnesses must borrow the same checked attachment witness,
not merely similar arena shapes. This prevents accidental cross-witness mixing;
it does not authenticate an enclosing saved world or establish collider
configuration, actual source provenance, contact geometry, solver masks or colors.

Unsupported bootstrap state, joint links/location maps, impulse/multibody joints
and soft bodies return `UnsupportedIsland`. A nonempty removal journal returns
`PendingMutation`. Body and manifold admission already requires empty authoring
mutation journals. The external domain counts and those journal witnesses must
ultimately be derived from the checked complete owner, not trusted as labels.
There is no silent conversion of unsupported state to an empty collection.

## Bounded wire format

`ICISLD01` version 1 uses the existing independently expected build/dimension
identity. The 116-byte header includes counters, options, unsupported counts,
map live counts and eight array lengths. Rows are 12 bytes per active slot,
4 per free entry, 8 per body handle, 32 per persistent slot, 20 per contact link
and 8 per location. Empty slots require zero payloads; absent locations require
both MAX words; booleans are exactly 0 or 1. All count domains, expected frame
length, phase/domain and identity are checked before payload allocation.

Every array is capped at 262144 items and the whole frame at 32 MiB. Decode
charges all eight Vec payloads to the shared Budget, followed by validation
scratch: two bytes per checked body and one byte per slot in each map. Encoding
charges its exact output length. This is cumulative requested payload, not RSS
or allocator overhead. Failed charges are not refunded. All iterations are
bounded linear passes over those admitted arrays and the already checked
manifold/solver spans. There is no recursion, unsafe code or new dependency.
Caller-owned input vectors are not retroactively counted as decode allocations.

## Actual observations and retained corrections

The trusted tests only serialize existing native objects one way; they never
feed data into Rapier Deserialize or construct a live solver from these records.
Both dimensions exercise ordinary and clustered contacts, warm starts, contact
edge swap-removal, collider/body generation reuse, exact sleep/wake ordering,
extra-iteration active reorder, body disable/enable/type changes, contact merges,
hot-endpoint deferred disconnect, later split nomination and execution, free
slot reuse, and the fully empty bootstrapped world. The existing manifold tests
now also validate each observed island/body composition.

Initial native fixtures exposed the real empty-awake and retained-fixed-active
states above; those observations were preserved and the overly strict rules
corrected. Other retained early failures were test-observer/fixture expectations:
Index has named fields, the shared geometry observer accepts the cuboid fixture,
and never-sleep bodies defer a split instead of immediately creating two islands.
A final empty awake container is retained rather than assumed to disappear.
The 115-case CPU suite passes (17 nonempty groups, 19 including empty groups).
Scoped checks and independent review are recorded in the evidence manifest;
this document does not substitute source review for native execution.

## Continuation still closed

The format intentionally cannot reconstruct live Rapier structures. In particular,
serde-skipped workspaces are not all proven discardable. Local/global split stamp
maps clear on wrap, and substep groups are recomputed before their next solver
use, but sleep_scan is different: begin_sleep_scan wraps its stamp without
clearing the old entries. Replacing those entries with zero can change the next
scan when the stamp wraps to zero. A small model regression demonstrates that
exact branch arithmetic only; it is not a native restore experiment or proof that
all other scratch can be omitted. The cooldown update also uses ordinary u32
addition, so future executing admission needs an explicit overflow policy.

Colored solver buckets, masks/hints, force-event membership and epochs remain
unchecked by this component. Full collider payloads, broad/narrow owner indexes,
CCD and integration managers, complete journals, retained caches/workspaces,
whole-owner construction and exact continued-step comparisons remain required.
This component does not prove derived mass consistency or authenticate matched
world identity. No runtime dependency edge, live constructor or save capability
is added, and no performance, phone, visual or release gate is claimed.
