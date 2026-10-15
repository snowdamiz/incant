# Atomic ordinary subtree duplication

Source preparation from main312 `a74473ea796b1ac1a476496f776a70307be1313c`.
This is Phase 2 backend overlap. No editor UI, release gate, or executed test
result is claimed by this initial source packet. Generated command schemas and
SDK types remain pending the separately allocated CPU qualification.

`DuplicateSubtree { scene_id, root_entity_id, entities }` uses the shared
`incant_cmd` transaction. `entities` is an explicit source-to-target ULID map;
the operation generates no entity IDs. It copies the complete subtree in the
same scene beside the original root. Names, local transforms and components
remain unchanged except for internal hierarchy and registered entity edges.
External parents and references remain external. Copying does not add a position
offset, rename nodes, import assets, execute scripts, or write project files.

The identity map must contain exactly the subtree, with 1–1024 nodes and distinct
canonical uppercase target ULIDs. Targets must be globally fresh at this command's
position in the transaction, including project, scene, entity, asset, script,
translation table, prototype and prototype-node IDs. Occupied IDs are compared as
parsed ULIDs, also rejecting aliases of older noncanonical spellings. A later
deletion cannot legitimize a collision. Command ordering remains meaningful:
earlier edits are copied, and later commands can address supplied target IDs.

Linked prefab nodes refuse copying; callers can explicitly detach a complete
instance first in the same transaction. The command never silently detaches.
AudioListener, global EnvironmentLight and a scene UI root refuse copying.
The existing final project validation still enforces all other resource limits
and relationships. New nodes receive the transaction provenance; original nodes,
prototype records, assets, scripts and their provenance remain unchanged.

Closure validation scans the scene once and retains at most the mapped 1024
nodes' edges. It rejects omitted descendants, unrelated roots and disconnected
cycles. Incoming maps share the existing prefab preflight payload allowance.
Output copies then share the existing transaction expansion allowance with all
prefab operations: 10,000 affected nodes and a conservative 32 MiB logical
payload charge, at most one million JSON values, depth 64 and 64 component slots.
Names, copied IDs, parent IDs, component values and new provenance strings are
charged before component cloning or insertion. Admission and staged expansion
are separate allowances, matching the existing prefab protocol. These are not
RSS bounds; the ordinary command bus still clones the project and CRDT state.

## Reference audit

The complete registered component schema set was inspected, including nested
definitions and map keys. Entity.parent is handled separately by the hierarchy
copy. Only these component leaves identify other scene/project entities:

| Component | Entity edge | Copy behavior |
| --- | --- | --- |
| AudioBus | parent | Remap a copied bus; retain an outside bus |
| AudioSource | bus | Remap a copied bus; retain an outside bus |
| NavigationMesh | sources[].entity | Remap a copied source; retain an outside source |

The reference-bearing components are decoded to their existing typed Rust forms
before writing only those JSON leaves. They are not reserialized wholesale:
omitted defaults, numerical JSON representation, external IDs and unrelated
fields remain exact. The existing stricter prefab remapper is unchanged.

Every other identity-like schema slot is deliberately literal in this operation:

| Component group | Retained identity/value domain |
| --- | --- |
| MeshRenderer, EnvironmentLight, AudioSource.clip, Text/UiNode fonts | Assets |
| Script.source and Script.props | Script asset and arbitrary literal JSON |
| AnimationGraph and nested AnimationTree | Graph-local state, transition, parameter and node IDs |
| AnimationIk.constraints[].id; AnimationRootMotion | Component-local constraint IDs; model asset/fingerprint and numeric model node |
| Collider compound parts | Shape-local IDs |
| NavigationMesh.links[].id | Navigation-link identity, not an entity reference |
| SpriteRenderer / SpriteFlipbook | Texture/atlas asset and atlas frame IDs |
| Tilemap | Tileset asset, layer-local IDs and atlas frame IDs |
| Text / UiNode content | Literal text |
| Transform, Velocity, AngularVelocity/2D, Animation, Camera, Collider2D, lights, NavigationGrid, RigidBody/2D, AudioListener | No entity-reference slots |

`subtree/references/inventory.json` records all 29 component schemas' non-enum
string leaves, typed map keys and arbitrary JSON slots. A test derives that
inventory from the live schema registry; added components or changed string slots
require a new reference audit. This detects schema drift, not semantic meaning
from a string type alone. The runtime remapper also refuses an unreviewed new
component kind rather than guessing that it contains no entity edges.

## Qualification prepared, not yet executed

Source controls cover nested copies and provenance; exact internal/external audio
and navigation remapping; literal script/graph values and omitted defaults; all
identity-map refusals; 1024-node admission; cycles, missing descendants and global
collisions; singleton and prefab refusal with explicit detach; cumulative mixed
prefab/copy node limits and cumulative payload refusal; atomic project, CRDT,
revision, undo/redo and journal behavior; deterministic re-execution; host-owned
save/reopen; live headless RPC schema/unknown-field/stale-revision handling; and
ordinary cooked NativeWorld consumption with independent copied motion.

Production scope is incant_cmd plus Command exposure in headless schema.list.
The existing generic editor engine_execute, agent command tool and headless
command.execute all deserialize the same Command type. No new mutation endpoint
or shipped-runtime representation is introduced. Source inventory found only
the command dispatcher is exhaustive; source-write, agent-budget and prefab
preflight consumers already have explicit fallback arms.

Planned qualification uses pinned Rust 1.99, arm64 CI, locked/offline, jobs 2 and
Rayon 2 under a separately granted target lease. Run command/doc regressions,
the new headless and cook controls, strict scoped all-target lint, then generate
the actual Rust Command/agent schemas and TypeScript SDK and check freshness.
Retain the first failure and its producing source before any repair. No native
window, GPU or performance run is needed for this slice.
