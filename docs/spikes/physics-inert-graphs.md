# Checked inert graph adjacency

The third checked physics component preserves the pinned Rapier 0.36.0 graph's
ordered nodes, ordered edges and doubly linked adjacency lists. Like the
[BVH](physics-inert-snapshot.md) and [arena](physics-inert-arenas.md) components,
it is inert, uses no normal dependency and cannot construct a live dependency
object. Physics save/load and world/session admission are unchanged.

Each node has two list heads and a bijective payload-row reference. Each edge
has two node endpoints, two next links, two previous links and its own bijective
payload-row reference. The vectors and links retain exact order, including the
effects of native swap-removal. There is no sorting, rebuilding or normalization.

The validator admits all scalar indices before allocating traversal scratch.
It then walks each node's outgoing and incoming lists iteratively. Every edge
must occur exactly once in its source's outgoing list and once in its target's
incoming list. A list head must have no previous edge; each successor must point
back to the previous edge, and each visited edge's endpoint must equal the list
owner. Duplicate visits, cycles within lists, shared tails, detached edges,
wrong endpoints and nonreciprocal links fail explicitly. Separate marks for each
side allow legitimate self-loops without treating them as aliasing errors.

General graph cycles, self-loops and parallel edges are supported by the pinned
graph and remain representable. This is not an assertion that every such graph
is a valid physics contact graph. Collider-handle uniqueness, contact/intersection
pair policy, Coarena reverse mappings, payload references and parallel solver
conflict ownership still require their own checked composition. The current
proof covers adjacency ownership, not exclusive mutable body ownership.

An immutable validation witness exposes side-specific adjacency and native
undirected incident order. The latter reads outgoing edges first, then incoming
edges, omitting a self-loop's second occurrence. These iterators allocate nothing
and can only be created from the checked immutable graph. They return edge
indices; interpreting payload values remains outside this component.

## Format and bounds

`ICGRF001` is a little-endian version-1 frame. Its 56-byte header holds the
dimension/f32 marker and expected 32-byte build identity used by the other inert
records, node and edge counts, and a zero reserved word. A node occupies 12 wire
bytes; an edge occupies 28. Missing next/previous/head links are `u32::MAX`.
Node endpoints cannot use that sentinel. Unknown versions/identities, reserved
bits, truncation and trailing bytes fail before record allocation.

At most 262,144 nodes and 262,144 edges are admitted, with the existing 32 MiB
section and 64 MiB cumulative requested Vec payload limits. Checked header
arithmetic admits exact total input length before any vector. Record allocation,
one-byte node/edge payload marks, two-byte per-edge adjacency marks and output
all charge the shared budget. Validation takes O(nodes + edges) work and no
recursive calls; every valid edge is traversed twice. As before, this is requested
payload accounting, not allocator rounding, caller-owned inputs or a process RSS
bound. An enclosing format must retain one budget across all its sections.

The build identity is supplied independently by the caller; it is not authority
derived from the input. Owner/world authentication, section integrity and actual
payload tables are still the enclosing format's responsibility. No arbitrary
serde bytes enter a live graph, and no panic-catching substitute is used.

## Verification and remaining work

The first full crate run passes 40 CPU tests across seven nonempty groups, ten of
them new graph cases. Hostile tests cover each side's bounds and backlinks,
duplicate/orphan ownership, malformed list cycles, dense payload mappings,
headers and allocation limits. Every truncation and single-byte mutation is
tested; admitted mutations must re-encode to the exact same bytes. A 4,096-edge
self-loop fixture exercises long adjacency with bounded iterative work.

Two tests use actual pinned 2D/3D native graph methods through a cloned public
`InteractionGraph::raw_graph()` view. They observe exact records, bytes and every
node's incident edge order/weight after graph construction, nonterminal edge
swap-removal, last-edge removal, node swap-removal, reinsertion and complete
draining. Self-loops, parallel edges and ordinary graph cycles are included.
Observation is one-way; no live graph is reconstructed from serialized input.

Initial Clippy reported iterator-style lints after all tests passed. That source
and raw failure are retained; equivalent `find`/head enumeration corrections are
in freeze002. The final full test run, all-target Clippy, normal wasm32 compile,
std-only and physics runtime dependency checks, formatting and conventions all
pass. Independent producer review is accepted. The lockfile and dependencies are
byte-unchanged. [Evidence](evidence/physics-inert-graphs.json) retains both
iterations and command outputs.

Contact values, feature/workspace correspondence, warm starts, islands, broad-phase
pending state, solver coloring and native body/collider ownership remain open.
Those families and complete exact continuation must be validated before enabling
a live restore. No full save, shipping, GPU, device or performance gate is claimed.
