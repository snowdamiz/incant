# Atomic ordinary subtree duplication

Source preparation from main312 `a74473ea796b1ac1a476496f776a70307be1313c`.
This is qualified Phase 2 backend overlap, with no editor UI or phase-gate claim.
The final code and generated contracts are frozen at
`25b038a126c0afeab3b0b215b57b051fa3035d2a`. The portable
[receipt](evidence/subtree-duplication/manifest.json) retains the initial source,
both failed attempts, their narrow repairs, and the successful CPU checks.

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

## Qualification

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

Pinned Rust 1.99, arm64 CI, locked/offline, jobs 2 and Rayon 2 qualification passed
133 distinct Rust tests: 99 command, 28 document, four actual headless RPC, and
two cook/native-world controls. Passing command/doc cases from earlier attempts
were retained; only affected or previously unexecuted suites were resumed.
Strict all-target Clippy for command/doc/headless/cook, workspace formatting,
conventions, diff checks, generated SDK freshness and strict TypeScript consumer
controls also passed. This is scoped CPU evidence, not a full-workspace CI result.

The copied headless executable generated 78 schema files. Exactly five changed:
Command, EnginePinReview, ProjectFileResponse, ProjectPinReview and agent-tools.
All other generated schema bytes remained equal. The generated SDK exposes the
same command; consumer controls reject an omitted map, numerical target IDs,
implicit detachment and cross-scene destinations. Runtime validation remains
responsible for exact subtree membership, valid ULIDs and bounds.

Two failures remain in the evidence:

- Source `2db6566d` compiled but its reference fixture placed a Collider under a
  parent, violating the existing physics root restriction before copying. Source
  `6053559e` changes that fixture to a valid mesh navigation source; the internal
  and external reference assertions remain intact.
- The live headless schema assertion then found that pinned Schemars ignored
  `length(...)` on a map. Source `7422d13d` publishes explicit `minProperties: 1`
  and `maxProperties: 1024`; the runtime checks and test assertion were unchanged.

All managed children returned after cleanup. Cumulative supervisor execution was
412.18 seconds; sampled target growth was 547,250,870 bytes and minimum sampled
free storage was 89,226,092,544 bytes. The target lease was released after the
final checks. No GPU, window, shipping performance or appearance run occurred.

Replay without executing tests:

```sh
python3 -B docs/spikes/evidence/subtree-duplication/verify.py
```

`--originals`, `--binaries` and `--git-source` additionally check retained external
receipts, copied executable bytes and immutable producing Git objects. The
archive embeds 319 records, including all four source freezes, raw test outputs,
the failed exported schema, actual generated contracts and resource receipts.
The copied executables remain external. No STATUS or editor UI changes are part
of this producer slice; final current-main integration remains the integrator's
separate responsibility.
