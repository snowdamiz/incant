# Authored physics mode and startup admission

`ProjectSettings.physics_mode` admits `fast` and `deterministic`. An omitted
field retains deterministic semantics and is omitted when serializing that
default, preserving old canonical project bytes. Eight-lane SIMD is a separate
target optimization candidate, not an authoring enum value.

`set_physics_mode` uses the shared command bus with normal transaction provenance,
revision checks, persistent recovery and Undo/Redo. A later invalid operation
rolls the entire transaction back. Settings do not alter an existing simulation.

Scene cooking, authored-script preparation, Play startup and the trusted native
export producer require the project mode to match the process's compiled kernel
family before asset loading or script compilation. Successful export producer
receipts identify both authored mode and exact cooked variant. The cooked-scene
boundary continues to enforce the exact variant, including four- versus
eight-lane fast, before publishing a world.

The complete document/command/cook/Play test selection passed 134 tests in 49
groups for each of the three compiled variants, followed by Clippy in each.
These include legacy canonical bytes, strict schema, public command recovery,
atomic refusal, unchanged project state on a mismatched cook, and a typed Play
refusal before fallback compilation or filesystem changes. Fixture projects
explicitly select the tested build mode; they do not rely on the host default.
The [retained execution manifest](evidence/physics-project-setting.json) binds the
25 changed Rust files and the unchanged 1,378-file compilation freeze.

Mode-aware development launcher and shipping export execution are separate
follow-on work. The editor still runs its existing in-process worker and cannot
switch its compiled kernel dynamically. No complete process-selection, physics
save, speedup, device or phase gate is claimed by this setting/admission change.
