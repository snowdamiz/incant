# Linked prefab authoring (Phase 2 overlap)

Base: main268 `fdd480355ba70b28bbebe35f58b8af188f0abb82`. This is authoring backend work; no editor UI, runtime prefab loader, phase-gate approval or publication is implied.

The document stays at schema version 1. Optional `Project.prefabs` and `Entity.prefab` are omitted when empty/absent, so existing canonical documents retain their bytes. Existing strict older readers refuse documents that use these new fields rather than silently losing linkage. New readers continue to accept old documents and journals.

A prototype has its own stable ULID, name, one root node and an ordered map of fresh node ULIDs to ordinary names, parents and registered component values. Nodes are not live scene entities. A linked entity records prototype ID, instance-root entity ID, prototype-node ID and a set of overridden component names. Its ordinary component map is the resolved value; an override name with no component is an explicit removal. There is no duplicate override-value store and no arbitrary JSON-path patch. Whole-component values still pass the existing typed component and complete scene validators.

Create-from-subtree requires an explicit source-entity-to-fresh-node map. Instantiation requires an exact prototype-node-to-fresh-entity map. Neither command allocates implicit IDs during replay. Known entity references in AudioBus, AudioSource and NavigationMesh are remapped by typed decoding; assets, scripts and localization remain shared. Opaque Script.props values remain literal data, not guessed entity references. External entity references from a prototype are refused.

The first contract is fixed topology and no nested instances. Every prototype node must have exactly one instance entity, all child links must match, and non-overridden components/names must equal the resolved prototype. Ordinary children cannot be attached inside a linked subtree. A root may have an ordinary external parent; placing it uses a Transform component override. Linked rename/child reparent/deletion refuse until detach. Prototype replacement preserves node IDs/root/parents and propagates names and non-overridden components atomically; component revert removes the explicit override. Detach clears all links while preserving resolved content and identities. Complete-instance deletion is a separate atomic command.

Enforced bounds: 256 prototypes, 1,024 nodes per prototype, 16,384 prototype nodes per project, 100,000 linked entities, JSON depth 64, and 32 MiB cumulative conservative logical payload charge for prototypes/linked components. Each transaction has a cumulative 10,000 affected-prefab-node and 32 MiB logical expansion budget. Charges precede new clone/expansion work. These bound prefab processing after the existing JSON decoder; they do not claim a process RSS or arbitrary whole-project JSON parse limit. Existing unlinked project limits remain unchanged.

Validation applies on text load, CRDT projection/merge, command final state and journal replay. Malformed linkage cannot bypass commands by forging a resolved component. Templates are checked as standalone resolved subtrees using the existing scene/component graph validators; materialized instances are checked in their actual scene, preserving constraints such as singleton listeners, physics parenting, navigation references and UI topology.

Implementation paths: new `incant_doc::prefab` records/validation/remapping; small Project/Entity/schema seams and shared scene-validation extraction; new `incant_cmd::prefab` operations/budget with enum/apply seams; focused document/command/headless/cook integration tests. Cooking and native runtime production code retain their ordinary entity path.

Required evidence: two distinct linked instances; independent typed override/removal and revert; prototype propagation preserving overrides; detach and subsequent independent edit; deterministic explicit-ID command replay; canonical text and CRDT reopen; undo/redo and persistent journal reopen; malformed links/topology/references/component values and expansion-budget rollback without revision/history/journal mutation; live headless command.execute/schema.list; cook both instances, discard authoring metadata, and step the existing NativeWorld. Verification is recorded below.

The additive-v1 choice follows existing, executable precedent rather than assuming unknown fields are tolerated: `incant_doc/tests/physics_mode.rs::absent_mode_preserves_legacy_canonical_bytes_and_fast_round_trips` and `tests/agent_budget.rs::legacy_projects_default_without_changing_canonical_bytes` add optional fields while retaining version 1. Project.string_tables and settings.input_actions/localization also default and omit absent state. ADR 0008 specifies derived schema/semantic validation without a frozen v1 field-set promise; PLAN Phase 7 still lists publishing version guarantees. `deny_unknown_fields` remains deliberate: a pinned main268 reader refuses the new fields. Version 2 remains unsupported; no implicit version migration is added.


The entity-reference inventory covers every registered component at main268.
`incant_types/src/audio.rs` identifies AudioBus.parent and AudioSource.bus;
`incant_doc/src/navigation.rs` identifies NavigationMesh.sources[].entity.
These three typed fields are remapped and refuse external references. Entity.parent
is handled by the topology map. MeshRenderer/Sprite, text/UI fonts, animation
model/clip IDs, Script.source, and localization table IDs refer to shared assets,
scripts or tables; they are preserved. Animation graph/tree/IK and collider-part
identities belong to local component subobjects, not entities. UiLink is a derived
runtime structure; authoring UiNode uses Entity.parent. Arbitrary Script.props
remain literal, even when a string happens to equal a mapped entity ID. A dedicated
control exercises all three remappers and preserves coincident asset/literal IDs.

Deletion charges the same cumulative 10,000 affected-node transaction budget:
DeletePrefab charges all removed prototype nodes, and DeleteScene charges only
its linked entities before removal. Ordinary unlinked scene deletion retains its
prior behavior. The original uncharged implementation is retained as two failing
atomic-refusal controls; no partial document/history/revision changes are allowed.


## Verification and boundaries

Full `incant_doc` + `incant_cmd` default CPU regression passed **87 tests**, zero
failures/ignored. This includes ten prefab command cases and five document cases.
The live headless RPC case passed create/two-instance/save/process-reopen/undo/redo.
The cooked consumer case passed byte-identical cooking before/after detachment,
then 60 native fixed steps with distinct instance velocities after discarding the
authoring document. These are deterministic-mode CPU observations, not performance
or rendering evidence. No alternate physics rebuild was needed: no solver, cooked
format or native runtime implementation changed.

Strict Clippy passed all targets (including examples/tests) of `incant_doc`,
`incant_cmd`, `incant_cook` and `incant_headless`. The final two-line semantic-neutral
lint follow-up removes an unnecessary reference in the extracted comparison and
records the intentional CreatePrefab argument count. Earlier full behavioral
results apply to this exact behavior; the strict final compiler checks bind the
new source. Historical evidence retains the initial test-only IndexMut compile
error, both deletion-budget counterexamples (eight other command cases passed),
and the initial op-ref lint failure. No failed attempt is presented as passing.

The freshly copied headless binary generated Project, Prefab, PrefabLink and
Command schemas, plus agent-tool contracts. Legacy GameSave embeds Project and
therefore receives the same additive schema definitions; its runtime behavior is
unchanged. SDK declarations were regenerated, and strict TypeScript checks include
all seven commands, old optional-project fields, override names and rejected
missing maps/JSON patches/untyped prototype updates. SDK freshness, conventions,
workspace formatting and whitespace checks passed.

Workspace consumer inventory found only two existing concrete document Entity
literals outside its constructors: `incant_cook/tests/runtime.rs` and
`physics_2d.rs`, both now explicitly set `prefab: None`. Other Entity/Project
matches are functions or distinct observer/ECS/cache types. The sole exhaustive
authoring Command match is `incant_cmd::apply`; source-write extraction already
uses `_ => None`. Runtime command enums are separate types. Editor agent-text
formatting has an explicit unknown-operation fallback and is unchanged; a linked
prefab editor interface remains future work. No GUI mutation path was added.

Evidence is retained by `docs/spikes/evidence/prefab-authoring/collect.py` and its
manifest/lossless records archive. The private source remains subject to final
independent review and current-main intake. Fixed topology, no nested instances,
whole-component overrides and external-entity-reference refusal remain explicit
limitations; this does not claim complete Phase 2 or any release gate.
