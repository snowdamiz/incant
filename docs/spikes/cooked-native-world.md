# Cooked scene and native world foundation

Implementation follows PLAN.md revisions 4–6. `incant_cook` is the authoring-side
boundary; `incant_runtime` loads a checked binary scene and runs a disposable Bevy
world without `incant_doc`, `incant_cmd`, CRDT or agent dependencies. The existing
editor/headless `PlaySession` is still the legacy path; this foundation does not
claim to have migrated its scripts, physics, rendering, navigation or saves.

## Implemented path

`incant_cook::cook_scene` validates a project once and converts the selected scene.
This first slice supports Transform, Velocity and hierarchy. Missing Transform
projects to identity, preserving the old core's behavior. Every other component
fails with its entity ID and component name; nothing is silently discarded.
Cooking reads authoring state and does not emit project edits.

The data image uses a fixed little-endian versioned header, binary ULIDs, a scene
tick rate, count and payload size, followed by contiguous spatial and moving
archetypes sorted by stable ID. Each record carries parent index, component mask
and explicit f64 values. SHA-256 covers metadata and payload; it detects damage,
not authorship. No executable content or unchecked native-memory casts exist.
The loader checks file/count bounds before allocation, checksum, supported
version/masks, canonical order, unique IDs, finite and valid initial component
values, parent references/cycles and composed transform range. The current hard
limit is one million entities. The whole scene is checked before publication.

`NativeWorld` instantiates Bevy component storage, integrates local velocities
and propagates parent transforms. Native builds enable Bevy's multithreaded
executor and parallel motion query; browser compilation uses the serial path
until the worker host is implemented. A native bulk callback borrows component
values directly, with Rust preventing concurrent structural mutation. This is
not yet the TypeScript/Wasm view ABI. Typed inspection and snapshots are explicit
calls; stepping does not clone/serialize a project or validate an authoring
document. Loading the original image creates a fresh world and discards play
state without touching the authoring source.

The binary format is provisional version 1 for this narrow capability set.
Component/schema expansion must update its version or add explicit checked
versioning; readers must continue rejecting unknown data. A content digest is
not a trust boundary for future executable bytecode/modules.

## Reproduction

```sh
./tools/cargo test -j2 -p incant_runtime -p incant_cook --release --locked
python3 tools/check_runtime_dependencies.py incant_types incant_runtime
./tools/cargo build -j2 -p incant_cook -p incant_runtime --examples --release --locked
python3 tools/probes/cooked-runtime.py artifacts/cooked-runtime-cli
```

The public probe authors its fixture through `command.execute`, cooks it without
changing the project/journal, removes both authoring files and starts a separate
runtime executable from the image. It verifies repeated process results, motion,
reset and corrupt-image rejection. Outputs retain executable/image hashes.

## Validation and limits

Eleven focused Rust tests pass, including every byte truncation and single-bit
mutation of a valid image, rehashed invalid data, missing/cyclic parents, initial
numeric overflow, stable round trips, bulk mutation, ten-thousand-entity repeated
simulation and ten-thousand-level iterative hierarchy loading. Workspace Clippy
passes with warnings denied. The resolved dependency guard passes for the real
native-world crate as well as the shared component values.

Full workspace verification passes 329 Rust tests across 109 suites, 41 renderer GPU
checks plus three editor/headless GPU integration checks (44 total), 15 Python tool tests, generated schema/SDK parity and conventions.
WASM and iOS simulator compilation of `incant_runtime` pass. The standalone probe
passes with freshly built executables; its 304-byte image runs after source and
journal deletion, repeats across processes, resets, and rejects corruption.
See [recorded evidence](evidence/cooked-native-world-2026-10-10.json). Hosted CI
adds the native-world tests on three desktops and compilation on mobile/web.
These are correctness/compile checks, not phone performance measurements or a
shipping-profile regression gate; those requirements remain open.

The follow-up [structural buffer](runtime-structural-buffer.md) adds checked native
entity lifecycle operations. Still required: remaining cooked components/resources,
script host views, physics/navigation integration,
binary saves, editor/play-process wiring, renderer snapshots, shipping profile,
Tracy/device performance gates and the rest of Section 2.6. The native callback
is for trusted engine code; foreign script bindings must supply their sandbox,
lifetime and numeric-fault handling before exposing these values to game code.
