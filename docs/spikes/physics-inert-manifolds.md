# Checked inert contact graph and manifold ownership

This component joins the checked graph, arena, attachment, geometry and contact
point records. It preserves the complete fixed metadata and ordered point arrays
of current rigid contact manifolds, including both plain manifolds and spatial
solver clusters. It does not construct a live solver, expose a runtime consumer
or enable physics saves. `PhysicsUnsupported` remains unchanged.

## Ownership and supported phase

The caller supplies a checked generic contact graph, paired body/collider arena
attachment witness and one checked geometry record per collider payload row.
Node rows carry exact collider slot/generation handles. Handles must be live and
unique. Each edge payload names the same two handles in the graph's exact endpoint
order; self-contact edges and duplicate unordered collider pairs are refused.
Generic graph support for loops/parallel edges remains unchanged outside this
contact-specific composition. Row permutations preserve their explicit mappings.

Each pair owns contiguous ordered plain and cluster manifold ranges. Each
manifold similarly owns contiguous ordered tracked-point and solver-contact
ranges. Complete partitions prohibit overlaps, gaps and orphan rows. Existing
point validators and wire helpers are reused; contact IDs remain local to their
own manifold, with exact flags and repeated solver selections preserved.

This format admits **completed narrow-phase observations only**. Any nonzero
modified-body, modified-collider or removed-collider journal count returns typed
`PendingMutation`, including before decoder allocation. The journal counts are
an explicit independent caller witness; the eventual whole-owner decoder must
derive them from checked journal payloads. Supplying zero counts does not itself
prove the world phase. Pending state is refused, never treated as an absent
contact. Current manifold body handles must exactly equal the current collider
parents, including None for a standalone collider.

Witness identities and row relations establish structural consistency, not the
provenance/authentication of an enclosing world. Same-shaped unrelated witnesses
can still be substituted unless a later owner envelope binds them. Native entity
ULIDs, complete body/collider properties and actual geometry resource ownership
are not authenticated here.

## Numeric metadata and coordinate frames

Pinned Rapier 0.36.0 / patched Parry 0.31.1 define the fields. Every admitted float
is kept as its original f32 bits; there is no normalization or sign folding.
Metadata includes both local normals, world normal, subshape indices and optional
child poses, optional body handles, solver flags/color/body IDs/GraphPos,
relative dominance, user data, friction and restitution.

Finite normals use a strict f64 squared-length tolerance of less than 0.0002 from
one, matching the inert local-rotation tolerance. Zero local normals are admitted
only for zero tracked points; a zero world normal only for zero solver contacts.
Planar unused Z is positive zero. Material coefficients must be finite and
nonnegative; signed zero, subnormal values and values above one remain intact.
These are explicit supported numeric-domain restrictions, stricter than unchecked
public native setters. They do not establish physical consistency, future
arithmetic safety or exact equality to a body's effective material/dominance.

Known solver flags are limited to bit 0. Native dominance difference is bounded
to -256..=256. Colors admit 0..=128 and the uncolored 255 sentinel. Solver body
IDs admit the format's bounded row indices or MAX; GraphPos admits native bucket
0..=129 and a bounded low-22-bit row, or exact MAX. These are scalar-domain checks
only: island/active-set and reciprocal solver-color/graph ownership remain
unchecked until their families are available.

Primitive shape indices must be zero with no child pose. Composite child indices
must address an existing checked part; ordinary manifold child poses equal that
part's exact bits. Both dimensions preserve outer None versus an allocated
Some([None, None]) because the native setters retain that allocation state.
Planar tile children retain their identity planar rotation. Unsupported geometry
kinds remain refused by the existing closed geometry validator.

Spatial clusters differ from ordinary manifolds: their points/normals are already
transformed into collider coordinates, child poses are absent, and subshape IDs
are representative IDs. Their points can originate from multiple children. The
validator checks the representative indices but does **not** assert that every
point belongs to that child. Native generic feature encodings are retained; their
shape-specific meanings vary by dispatcher and no invented box feature-index
range is used. Full mixed-child feature provenance remains pending. Clusters
admit at most the native 255 tracked points. Planar clusters are refused because
the pinned native implementation does not implement them.

## Wire, work and allocation bounds

`ICMAN001`, version 1, has a 68-byte header with the common expected build,
dimension/f32 identity, five counts and a zero reserved word. Fixed row strides
are node handle 8, pair 32, manifold 188, tracked point 100 and solver contact 48
bytes. The manifold stores tagged optional child poses and handles inline;
inactive option payloads must be zero. All integers/float bits are little endian.

Each count is at most 262,144 and the complete frame at most 32 MiB. Header,
expected identity, phase/count agreement and exact byte length precede allocation.
The shared Budget charges all five requested Vec payloads before allocating and
then charges collider-count bytes plus 8 bytes per edge for uniqueness scratch.
Encoded output is charged separately. Charges are cumulative and not refunded
on failed validation. These are requested Vec payload bounds, not total RSS or
allocator overhead. Caller-owned records and independently created witnesses
remain subject to the enclosing caller's own aggregate admission.

Work is O(colliders + manifolds + points + solver rows + edges log edges), with
bounded unstable sorting of scratch endpoint pairs. Payload order is untouched.
There is no recursion, unchecked arithmetic, unsafe code, live deserialization,
panic-catching or normal dependency beyond std.

## Observable verification

Frozen source002 passes **79 CPU tests in 13 nonempty groups** (15 result rows
including empty library/doc groups), zero failed or ignored. Fourteen are new:
ten hostile/format tests and four actual native-world tests. All-target Clippy,
wasm32 library compilation, normal dependency graph, three-crate runtime
dependency guard, formatting, conventions and diff checks pass. Cargo.lock is
unchanged. Source001 also passed all its 77 tests; source002 adds two hostile
cases, strengthens multi-edge native removal/reuse coverage and formats the
capture test, with no production change. There were no failed build/test attempts.
Raw outputs, commands, source copies and pinned references are retained in
[the manifest](evidence/physics-inert-manifolds.json).

The native tests use actual public PhysicsWorld operations in both dimensions.
Sixty plain and sixty compound steps per dimension capture exact graph order,
collider/body generations, geometry read from actual shapes, metadata and all
contact rows. Both observe nonzero warm starts. Plain tests use two simultaneous
ground-contact edges, remove one collider and reuse its slot with a new
generation, verifying surviving graph/payload ownership. Pending insertion,
velocity mutation, removal and reinsertion are refused until a native step
clears the journals. The standalone ground exercises absent body owners.

The spatial compound test independently transforms each plain contact through
its actual child poses and matches each cluster point bit-for-bit. It observes
both changed coordinate values and multiple contributing children in one
cluster, together with nonzero warm starts. The planar test observes the plain
compound route with no clusters. Every captured table validates, round-trips and
re-encodes byte-for-byte, including exact optional allocation and field order.
No validated inert record is fed back into native structures.

Hostile tests cover stale/duplicate handles, orientation, self/parallel contacts,
foreign/missing witnesses, malformed range order and local references, child
poses, all metadata domains, exact 255/256 cluster boundary, every truncation,
header/tags/reserved bytes, cumulative exact allocation and per-byte mutations.
Any admitted byte mutation must re-encode identically.

## Remaining complete-restore requirements

Independent source/final evidence review is pending. This is not a complete
ContactPair or narrow-phase snapshot: pair events, pair colors/body masks,
dispatcher/workspace and recycle state, previous cluster storage, soft contacts,
intersection state and journals are absent. Complete body/collider properties,
shape acceleration/feature correspondence, solver graph/island reciprocities,
owner authentication and checked live construction remain required. Physics
save admission stays refused until full composition and exact continuation are
proven. No shipping/device/performance/GPU gate is claimed.
