# Compound collider Inspector and real-engine look-dev

Claude Opus 5.5 through ACP owns visual design, layout, styling, rendered-pixel
review and screenshots. Read CLAUDE.md. Astra owns the new runtime and bridge
correctness. Preserve the director's neutral, clean integrated editor: Assets
has its own left workspace, asset details use the main Inspector, and the bottom
dock is exclusively Problems/Console/History. The director rejected messy crowded
bottom panels. Do not regress that separation or add floating panel gaps.

## Implementation and scope

`Collider.shape` now also accepts `{type:"compound",parts:[...]}`. Each of 1–64
parts contains `id` (stable ULID unique within the collider), local `translation`
in meters, local `rotation` as an xyzw unit quaternion, and primitive `shape`
(`box`, `sphere`, or local-Y `capsule`, with their existing dimensions). Nested
compounds, invalid IDs/duplicates, out-of-range dimensions/offsets and non-unit
rotations fail atomically. All parts share the parent collider's material,
sensor flag and collision masks. They form one body, and runtime hits/events
return the parent entity ID. Physics still requires scene roots with unit scale.
Characters remain primitive bodies, but can move against compound obstacles.

Runtime tests verify hollow arch openings, offset/rotated parts, falling compound
bodies, part reordering without losing solver state, aggregated sensor events,
query masks and explicit rejection of compound character bodies. Twenty-four
300-tick walking combinations cover capsule/sphere/box characters on primitive
and compound floors/walls, with autostep on/off. The command-bus test covers
atomic invalid edits, provenance, Undo/Redo and journal reopening.

The actual schema is `schemas/Collider.schema.json`; the bridge already resolves
its nested definitions, bounds and tagged unions. A new bridge test uses this
real schema. `FieldView` currently treats every array as a numeric vector, so a
compound's object parts appear as a mismatch: that is the primary UI work.

## Visual work and acceptance

Design and implement a polished, readable, read-only Inspector presentation for
compound parts. All authored fields and stable IDs must remain accessible;
preserve the raw data and show explicit notices for unsupported/malformed values.
Use the shared schema and exact diagnostic paths, including array index and nested
shape/axis paths. Do not add authoring controls, a new mutation path or pretend a
read-only component can be edited. Reuse generic object-array presentation where
appropriate, without assuming all schema arrays are vectors. Avoid making a
64-part collider flood the screen or create an unwieldy tab sequence by default.
You own the layout, grouping, spacing, field labels and visual treatment.

Test valid mixed parts, missing/unknown nested shape tags, invalid part values,
part-level/axis diagnostics and a large list. Verify keyboard access, meaningful
accessible names, scrolling and narrow layouts at 1440×900 and 1000×650. Preserve
the primitive Inspector, mask presentation, schema order and saved-account shell.
Native captures will follow from Astra through CUA; request specific states.
Do not launch, drive or capture native apps through shell-based automation.

Build a small attractive real-engine review scene using compound colliders that
match actual imported geometry. Useful coverage includes an arch with a genuine
opening, a rotated local part, a falling compound prop and a character moving
against compound floor/obstacles. Choose clear cameras/materials yourself. Use
actual public `init`/`import`/`rpc command.execute` edits with journal/provenance,
then actual `play --camera` captures. Do not fake physics or generate replacement
frames. Read numeric logs as well as images and report defects to Astra.

Use `artifacts/tools/incant_headless` and verify its source/hash in `binary.json`.
It is the built implementation, avoiding a second Rust build; never share another
worktree's Cargo target directory. Use `npm ci` for your own JS dependencies.
The previous 0022/0023 look-dev helpers are useful references. New helpers must
refuse an existing output directory. Keep each rendered run within 128 frames
and 256 MiB raw capture data; prefer 960×540 and a meaningful fixed-tick interval.
Repeat the run and distinguish local equality from cross-host determinism.

## Paths, checks, output

You may edit `editor/ui/src`, the browser fixture/capture helpers, and
`handoffs/0024-compound-inspector`. Shared schema presentation metadata changes
belong in `crates/incant_doc/src/physics.rs` only if required for presentation;
coordinate any runtime or bridge-contract correction with Astra. Do not edit
solver behavior, account storage, CI or unrelated landing-page files.

Run UI tests/build and strict TypeScript for any behavior. Retain a small useful
set of public screenshots and reproducible helpers; keep raw runs and all
account-bearing native captures ignored. Record exact model/transport, commands,
evidence, performance, defects, unresolved native review and limitations in
`result.md`. Commit with `Built-by: claude`; do not publish or merge. This is a
scoped feature increment, not phase-gate approval.
