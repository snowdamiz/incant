# Captured root-motion catalog admission

The new lean `incant_anim` crate owns immutable root-only tracks, checked rigid
sampling and a bounded binary payload. Its only runtime dependencies are `glam`
and `thiserror`; it loads no files or models. The accepted channel sampler and
root-motion arithmetic moved mechanically from `incant_assets_runtime`, whose
public cooked-model adapter remains compatible. Full-pose channel interpolation
uses the same extracted arithmetic, including existing f32 result rounding.

`AnimationRootMotion` is an ordinary typed component with a model ULID, captured
fingerprint, selected root index, application policy and enabled flag. Normal
command transactions validate its model/mesh/revision relationship. The tests
exercise provenance, atomic rejection, Undo, Redo and persistent journal reopen.
Changing the model revision requires an explicit rebind. Schema and TypeScript
component declarations are generated through the existing registry.

The startup resource loader reads only the project-owned captured model cache.
It validates the selected TRS root, unit-scale static ancestor chain and dominance
of every bound skeleton in the rendered model scene. Animated ancestors, root
scale tracks, nonunit rest scale, matrix paths and unrelated skeleton roots refuse
explicitly. LINEAR and STEP tracks retain their existing behavior; CUBICSPLINE
remains an importer refusal. No source glTF or source buffer is read by this loader.

Catalogs retain only selected translation/rotation channels, all clip durations,
rest and model reference transforms, cached cycle endpoints and binding identity.
Identical fingerprint/root pairs share one immutable Arc. Limits are 256 clips,
1,000,000 aggregate keys and 32 MiB retained catalog data, with 64 MiB unique data
and 1024 bindings per scene. Retained-byte accounting includes owned structures
and arrays, excludes allocator/Arc overhead and discards spare input capacity.
Each sampling call borrows the admitted catalog and scans no full track.

The standalone nested payload uses explicit little-endian fields, version 1,
reserved zeros and checked counts. Readers reject malformed framing, nonfinite
values, invalid quaternions, unordered times, unknown tags and trailing bytes.
The payload itself does not authenticate data; the future scene section will
carry the existing cooked-scene checksum. Catalog construction admits numerical
invariants; only the captured-model loader establishes topology/content binding.

All 134 asset/numeric tests and 98 document/command/cook tests pass, with no
failed or ignored cases. Both scoped all-target Clippy sets, wasm32 visual
compilation and the lean runtime dependency guards pass. The catalog allocation
control measures zero allocations across 4096 sampling iterations after admission.
The real captured-model test encodes the root-only payload, drops all project,
model and catalog ownership, deletes sources and cache, then decodes and checks
an exact multi-cycle delta. It also verifies shared catalog ownership before
deletion. This is a source-deleted catalog test, not a packaged game proof.

[The evidence inventory](evidence/native-root-motion-catalog.json) binds the
source freezes, literal commands and retained logs. The second freeze adds only
generated declarations and the stronger captured binary round-trip control to
the first passing broad suite. Independent source and retained evidence review accepts the foundation without
findings; full runtime connection remains ongoing.

This is the admission foundation of ongoing connected root-motion work. Ordinary
scene cooking still explicitly refuses the component until cooked v17 and native
binding are connected; it never silently drops it. Animation-step spans, compound
interval blending, pose stripping, body policies, world v11 save continuation,
TS/Wasm gameplay access and ordinary source-deleted exports remain pending. No
rendered, device, performance, complete root-motion or Phase 1 gate is claimed.
