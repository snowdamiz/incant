# Joined inert physics owner boundary

This slice composes accepted arena, geometry, attachment, body, collider,
manifold, pair, island, rigid, narrow, broad, integration, CCD and pipeline
witnesses. It adds a canonical inert frame and a separately reviewed trusted
live-to-live workspace experiment. It does not make physics worlds restorable
from bytes and does not remove `PhysicsUnsupported`.

The private base merges pipeline `706a89a8` with broad owner `88c10fe1` at
`64f7db33`. The isolated default-off vendor adapter is `219e83bf`; both numerical
modes retain their actual selected solver features. No normal dependency was
added to the std-only inert crate.

## Frame and joins

`ICWORLD1` has the common version/dimension/build header, a fixed section count,
and exactly 22 ordered tag/length entries (224 header bytes). All lengths and
whole-input admission are checked before allocation. Unknown, duplicate,
reordered, missing and trailing sections refuse. Nested geometry lengths are
also preflighted before collection allocation. Existing codecs validate each
section; their contracts are not weakened.

The section order is metadata, body arena, collider arena, attachment, geometry
collection, body state, collider state, contact graph, manifolds, pair state,
islands, rigid owner, narrow index Coarena, intersection graph, narrow owner,
pending AABB queue, pending BVH, broad adjacency Coarena, broad owner,
integration config, CCD and pipeline state. The duplicate pending queue breaks
only a Rust borrow cycle and must equal the broad record exactly. Metadata
retains pending journal and unsupported family counts as explicit refusal
fields, plus the historical multibody epoch. These declared counts do not
authenticate actual native emptiness or provenance.

The top join requires the exact collider witness used by both broad/rigid and
CCD owners, plus the exact integration witness already joined to the pipeline.
Existing lower joins retain the body, geometry, graph and arena associations.
Equal records independently validated for another owner cannot satisfy these
pointer joins. A completely internally matched frame still does not authenticate
which native world produced it.

One caller-owned `Budget` accumulates all decoded sections and validation
scratch. Encoding also charges temporary section buffers and final frame;
charges do not refund on failure. Existing 32 MiB frame and 64 MiB cumulative
requested-Vec-payload limits apply. These are neither allocator/RSS measurements
nor bounds on future solver work. `with_decoded_world_boundary` invokes its
higher-ranked callback only after every join succeeds; decoded borrowed storage
cannot escape. No live unsafe dependency object is constructed.

## Trusted workspace experiment

`checkpoint_fresh_workspace` accepts only an existing live `PhysicsPipeline`.
It uses the non-consuming observer and refuses actual deferred receiver/inline
work, poisoned ownership, or quarantine reports/workspaces. It returns fresh
`new()` scratch/counters with the exact retained `None`/false/true CCD verdict.
An already configured parallel pool is Arc-shared as execution policy. It never
accepts a record, receives/joins a task, clears poison, installs a saved world or
proves a completed phase. Even an unstepped pipeline can have empty scratch;
only the trusted test harness establishes its own completed positive step.

The actual continuation tests construct a second world with typed native Clone
owners and this fresh workspace. They compare native serialized owners, exact
CCD cache and observer data, substep counters, ordered events, and the complete
inert frame after each step. Inert decoded data is never installed into that
native world. There are 96 contact/sleep/wake, 32 sensor/removal/reuse, and 12
CCD-toggle/removal steps per dimension: 280 paired steps total. Deliberately
omitting the retained verdict still changes cache/substep observations while
serialized owners remain equal; no trajectory difference is claimed.

The initial 3D warm-contact assertion only inspected ordinary manifolds. Actual
clustered solving retains warm-start impulses in solver clusters; checking both
actual stores proves the original nonzero-warm-start criterion without changing
the fixture, native solver or threshold. All failed compile/assertion/diagnostic
attempts remain retained.

## Qualification

Independent root reviews accepted the isolated workspace adapter and source003
aggregate implementation. The unchanged source then passed all ten scoped
qualification commands: 202 tests in each of the deterministic and fast8 modes
(23 nonempty groups, 25 summaries each), ten focused parallel controls with two
Rayon threads, strict all-target Clippy, wasm32 compilation, the normal dependency
graph and runtime guard, formatting, conventions and diff checks. There were no
failed or ignored cases in those successful suites. Modes and focused cases
overlap; their counts are not distinct coverage. Each actual mode exercises the
same 280 paired continuation steps. The normal dependency graph remains the
inert crate alone.

The [lossless evidence manifest](evidence/physics-world-boundary.json) retains
every command and failed attempt, exact source generations, the two source
review receipts and three hash-bound executable test binaries outside the
shared build directory. Final independent execution/evidence review remains
pending. The binaries and CPU records do not establish a restored-byte world.

## Remaining material barriers

This is a joined inert consistency boundary, not safe hostile live admission.
Derived mass/inertia and shape/subshape/AABB coherence, checked total body-added
iteration/PGS/substep/gravity work, enclosing shape/dispatcher/joint/soft family
provenance, native event queues and engine stable-ID ownership, and authenticated
capture phase still require complete composition. A reviewed constructor for
all owners and restored-byte continuation proof remain necessary. No runtime
consumer, world/session save format or save capability changed. No GPU, device,
shipping or performance gate is claimed.
