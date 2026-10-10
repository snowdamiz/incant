# Shared runtime component boundary

First implementation step under PLAN.md revisions 4/5. The new `incant_types`
crate owns the existing Transform, Velocity, camera, render binding, light, audio,
physics and collider value types and the stable identifier alias. These are the
same Rust definitions, serializers and schema annotations previously owned by
incant_doc; incant_doc re-exports them to preserve its public authoring API.

Project/scene documents, provenance, command semantics, cross-reference
validation, JSON decoding and CRDT ownership stay in incant_doc. Camera and
collider validation become authoring-side functions over the shared values.
Navigation declarations still depend on the navigation schema and need their
own subsequent boundary migration; script property values remain authoring-side.

This is a real dependency extraction, not a new runtime facade. incant_types has
only serde and schemars as direct dependencies. `tools/check_runtime_dependencies.py`
walks Cargo's resolved normal dependency edges and rejects incant_doc, incant_cmd,
incant_agent, incant_editor, Loro and Automerge, including transitive, renamed and
target-specific edges. Dev/build-only edges are excluded. Explicit root names
are mandatory, and an absent/unresolved root fails instead of passing vacuously.
CI currently checks **incant_types only**. There is no incant_runtime crate yet,
so this is not the plan's completed shipped-runtime dependency gate.

## Validation

- Workspace Clippy with warnings denied and all **318 Rust behavior tests** pass.
- All **41 GPU checks** pass, including camera/material/light/shadow regressions.
- Regenerated published schemas are byte-for-byte unchanged, and generated SDK
  checking passes. No project format or TS API migration is introduced.
- All **12 Python tool tests** pass, including five dependency-guard regressions.
  The guard passes against the real locked Cargo graph for incant_types.
- The core compiles for browser/WASM and the iOS simulator. Those are compile
  checks, not physical-device execution or performance evidence.

Local logs: `/tmp/incant-types-{clippy,workspace,headless,schema,gpu-unit,gpu,wasm,ios}.log`.
No UI/layout change or new rendered appearance is introduced. Existing tests of
authoring validation and gameplay provide compatibility coverage; duplicate
tests of every unchanged field declaration would not add useful evidence.

## Next boundary

Implement checked, versioned binary scenes and a native ECS world using these
types, with the authoring-to-runtime conversion outside the runtime dependency
graph. Migrate existing physics/input/navigation and script behavior to that
world before replacing PlaySession. Enable the dependency guard for the actual
runtime root when it exists and runs real cooked scenes. The current gameplay
path still clones and serializes the Project; this change claims no frame-time
improvement or pass of the new phone performance gates.
