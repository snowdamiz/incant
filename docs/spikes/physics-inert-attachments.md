# Checked inert rigid attachments

This component connects two checked arena layouts through exact body/collider
attachment membership. It is the next prerequisite for checking the body handles
in contact manifolds; it does not restore bodies, colliders or contacts. The
std-only `incant_physics_snapshot` crate still has no runtime consumer, live
constructor or physics-save admission.

## Survey and selected boundary

Pinned Rapier 0.36.0 stores each body's colliders as an ordered vector and each
collider's parent as an optional body handle plus local pose. Native detach uses
`swap_remove`; attaching appends. Reparenting an attached collider preserves its
local pose, whereas attaching a standalone collider creates an identity local
pose. Disabled and sleeping owners still retain their attachments. These values
must survive intact before contact body references can be interpreted.

The contact family needs further components. `ContactPair` stores both collider
handles, event bits and rigid contacts. Rigid contacts contain ordered manifolds,
solver clusters, persistent solver color/body slots, a dispatcher workspace and
recycle poses. Manifold data adds body handles, solver-body indices, solver graph
positions, normals and ordered solver contacts; each contact retains warm-start
impulses and frozen anchors. Parry manifold points carry feature IDs and subshape
identity/poses; composite dispatcher workspaces need shape-specific validation.
Rapier also marks `solver_clusters_prev` as skipped by serde. Whether that storage
is safely reconstructible must be established at the eventual continuation
boundary; native serde is not being treated as a complete checked save format.
Body/collider modified/removal queues and the default fixed-body value are also
outside this component. The source survey is hash-bound in the evidence.

## Records and validation

`AttachmentRecord` has one span per live body payload row, one optional parent
record per live collider payload row, and a flat ordered table of collider
handles. The body and collider arena witnesses are separate immutable borrows.
Their dimensions/build identities and exact live counts must match the record.
Payload-row permutations already admitted by the arenas remain supported.

Body spans must form a complete contiguous partition of the flat table. Every
listed collider handle must resolve at its exact live generation. It must appear
once, and its parent handle must resolve to the body row owning that span. Every
collider with a parent must appear in that parent's list; a standalone collider
must appear in none. Free, stale, sentinel and out-of-range handles fail. Neither
list sorting nor reconstruction is used. Checked lookups distinguish invalid
handles from live standalone colliders and return the original order.

The record retains every local-pose f32 bit on native active lanes: planar
translation XY and rotation `[real, imaginary]`, spatial translation XYZ and
quaternion XYZW. Planar unused lanes are positive-zero padding. Signed zeros,
subnormals and opposite quaternion signs are preserved without conversion,
normalization or sign folding. All finite local translations are representable,
including f32 extremes: Rapier's public local-pose setter/reparenting does not
impose a magnitude bound, and the existing ±1,000,000 world-position policy is
not a local-offset limit. Later world composition must check world range and
arithmetic overflow before accepting a live state.

The admitted rotation domain uses finite f64 evaluation of squared length with
strict error below `2e-4`, derived from the pinned glam/glamx rotation checks.
This is a documented supported-domain policy, not a claim of exact equivalence
to their rounded f32 boundary predicates. Unchecked non-unit native values are
refused; stored values are never repaired. Trusted tests also show that native
public setters can create a missing-parent or nonfinite pose state which this
boundary deliberately refuses.

Matching these witnesses proves structure, not provenance. Equal unrelated arena
layouts can satisfy the same relationships. Enclosing-owner authentication,
section integrity, body/collider semantic payloads and captured geometry binding
must be supplied by the future complete format. This table cannot establish that
a parent is a supported rigid body rather than an unsupported soft-frame proxy.

## Wire format and bounds

`ICATT001` version 1 uses a 60-byte little-endian header: the common dimension/f32
marker and independently expected build identity, three counts and a zero reserved
word. Body spans occupy 8 bytes, collider parent slots 40 and listed handles 8.
A standalone collider's tag and all nine remaining words must be zero. Parent
tag 1 carries slot/generation plus seven pose words. Unknown tags/versions,
identity mismatch, reserved words, wrong exact length and trailing bytes fail.

Header counts are checked against both arena witnesses and the 262,144-item
limit before record allocation. Attached count cannot exceed collider count.
Wire size is bounded by 32 MiB. All requested record Vec payloads, one byte per
collider of validation scratch, and encoding output use the same cumulative
64 MiB Budget; failure does not refund it. Validation takes O(bodies + colliders
+ attachments) work without recursion. The budget does not claim allocator
rounding, caller-owned input or process RSS. Enclosing sections must share it.

## Verification

The full crate suite passes 52 CPU tests in nine nonempty groups. Eight new
hostile/codec tests cover reciprocal membership, omissions/duplicates, spans,
row permutations, stale/free/sentinel handles, foreign layouts, numeric domains,
headers/tags/padding, every truncation, exact cumulative budgets, 4,096 ordered
attachments and hostile byte mutations. Accepted mutations re-encode exactly.

Four actual Rapier 2D/3D tests use public body/collider sets and public mutations.
They retain native ordered membership and exact local-pose bits after attaching,
detaching, reparenting, collider removal/reuse, body removal with and without
attached-collider removal, and body-slot reuse. Disabled/sleeping and standalone
cases remain present. Two cases observe and reject unchecked native invalid
states. Arena serialization is one-way trusted observation only; no untrusted
serde or panic-catching constructs a live object.

The first build failed on a test-only Pose3 padding initializer and emitted
warnings for deprecated builder names. That freeze/output is retained. Freeze002
changes only the trusted fixture to use the current public builders and identity
padding. Production source is unchanged. Full tests, all-target Clippy, wasm32
library compilation, std-only and runtime dependency checks, formatting,
conventions and diff checks pass. Cargo.lock is byte-identical to the graph base.
Independent source/final evidence review is pending.
[Retained evidence](evidence/physics-inert-attachments.json).

Full rigid-body/collider state, shape/subshape bindings, contact features,
workspaces, warm starts, islands, broad-phase pending state, solver coloring,
native owner identity and exact continuation remain open. No live restore,
full-save completion, shipping, GPU, phone or performance gate is claimed.
