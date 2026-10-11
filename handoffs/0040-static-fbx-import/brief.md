# Current revision: bounded tangent delta

The original review is complete and immutable. Read and execute only
[tangent-delta-brief.md](tangent-delta-brief.md) for this continuation.
Return the new tangent-delta-result.md; never overwrite original result.md.
The historical packet below is context, not a request to rerun its full scope.

# 0040 — Static FBX appearance and import affordance review

This is a new, independent Claude Opus 5.5 / explicitly verified Max packet. Astra owns correctness and implementation; you own rendered appearance and visible UI review. Preserve all older handoffs. Read `docs/spikes/static-fbx-import.md` and the tests/fixture provenance before planning captures.

## Candidate and boundaries

Astra will place the frozen release `incant_headless` and a `binary.json` under ignored `artifacts/tools/` in this worktree. Check its exact SHA-256 before and after every execution; never rebuild or substitute an older binary. The same file proves the actual authoring importer and canonical renderer. Captures are authoring-preview evidence, not a native-game/shipping/device performance gate. No Autodesk SDK, account, external provider or source import at game startup exists.

Only write within `handoffs/0040-static-fbx-import/` and ignored `artifacts/0040-static-fbx-import/`. You may create helpers, original textures/art and small source fixture derivatives for your review, with provenance. Do not change Rust, renderer/shader, importer, schema, SDK, editor components/styles, lockfiles or existing evidence. Report concrete defects with minimal repro rather than repairing production. No Cargo, Rust build or dependency resolution. You may run the existing installed UI preview tooling for one bounded headless-browser affordance check. Never operate native/system-wide UI or another running application.

The importer intentionally accepts a bounded static subset: Lambert or explicitly mapped Blender metallic/roughness; constant material factors and base/emissive/normal maps; hierarchy/instances/polygon meshes. Animation/skinning/morph/legacy Phong/advanced or unrepresentable material data reject explicitly. Do not interpret these documented refusals as completed support. A newly observed silent loss within the declared subset is a defect.

## Required appearance checks

Use real included Maya/Blender/3ds Max sources from `crates/incant_assets/tests/fixtures/fbx`, with their recorded MIT provenance, via public `init`, `import`, `rpc` commands and `screenshot`. All project/scene/component changes must use `command.execute`; Undo/Redo and project saves must use the same public command path. Never edit a project or journal directly. Sources themselves are allowed authoring files. No Play simulation required.

1. Show accepted ASCII and binary static objects, instancing/hierarchy and seven-material Suzanne. Check source units/axes, reflection and nonuniform parent transforms using a coherent set of cameras. The numerical tests already assert centimeter/inch and Z-up conversions; appearance should expose wrong orientation, mirrored winding/culling, missing faces, lost material partitions or duplicated transforms. Where useful, compare independently authored simple glTF/ordinary geometry controls without copying renderer math.
2. Check embedded and project-relative image appearance, UV orientation and normal-map tangent orientation on a bounded, explicitly controlled fixture. `crates/incant_assets/tests/fbx.rs::textured_source` demonstrates legal FBX Video/Texture records based on an upstream Maya cube; you may write an equivalent helper/derivative with an explanation. Test asymmetric source artwork and reflected/nonuniform placements so inverted normals/UVs are observable. The CPU test verifies finite orthonormal normal-map tangents, but it does not replace your rendered review. Keep unsupported texture modes explicitly refused.
3. Exercise stable reimport, one accepted changed source, Undo/Redo, and a rejected unsupported/malformed reimport. Confirm retained asset identity/project state and preview continuity. Remove all FBX/image source files after successful cook and capture the same scene from its caches in a fresh process; no source parser fallback is allowed. Record exact before/after hashes and command outcomes.
4. Verify the existing editor import form accepts `.fbx`/`.FBX` as a model, advertises FBX in its current supported-format label and does not send stale image-usage settings. The only UI source changes are in `assets/paths.ts` and its behavior tests. Do not redesign the form; headless browser/fixture is sufficient for this affordance only. Actual native import correctness comes from the public engine CLI, not the fixture bridge.

Separate appearance observations from numerical/correctness claims. Retain shared renderer limitations honestly. Identify whether any error belongs to FBX conversion, a documented unsupported feature, or the existing canonical renderer.

## Bounded execution and coordination

Before any renderer execution, require `artifacts/tools/gpu-review-ready` containing Astra's explicit slot release. If absent, continue CPU-only planning and wait; do not touch another process or acquire the GPU. If a pause request arrives, finish the current bounded process, report its PID/exit, then wait. No native window or audio device review here.

At most 96 engine captures, each at most 960×540, at most 256 MiB raw capture output total. Start with a small useful matrix; avoid redundant views. Every engine subprocess must have a bounded timeout and be reaped. Keep a capture ledger with command, binary hash, project state, dimensions, actual adapter/device, exit and output hashes. State when all renderer/browser capture processes have exited so Astra can release the device slot while you write the report. Never visually inspect or modify another agent's captures.

## Deliverable

Write `handoffs/0040-static-fbx-import/result.md` with the confirmed model/Max effort, frozen source and binary IDs, actual OS/GPU/backend, supported subset exercised, exact commands, observable failures, scoped verdict and remaining limitations. Include original/derived source provenance, compact helpers/tests, a reproducible hash manifest and a small useful set of unedited engine captures (and any explicitly marked derived figures). Keep full raw output/projects/cache/binary ignored; no credentials or private account state. A source-free snapshot remains an authoring render, not shipped player evidence.

Commit only the packet with `Built-by: claude`. Do not publish or merge. Astra will independently review source/helpers/hashes and run checks, without substituting an Astra appearance judgment for yours.
