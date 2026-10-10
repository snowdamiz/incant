# Native spatial scene and committed extraction

This is the data prerequisite for a native renderer, not rendering or the
pixel-placement feature. No GPU, shader, UI, legacy Play, or marker path changes.

The authoring cook converts Camera (perspective/orthographic/pixel-perfect),
MeshRenderer, DirectionalLight, PointLight, SpotLight and EnvironmentLight to
checked typed declarations. Model and environment asset references become binary
stable IDs and 32-byte existing cache/source-recipe fingerprints. Those
fingerprints are version identities, not necessarily whole-payload digests or
authentication. Nonempty material overrides and every other unmigrated component
still fail explicitly.

Scene v4 appends a canonical asset table and masked spatial records after v3's
physics/audio sections. Encodings are little-endian, explicit tags and fields;
no native object casts. Versions 1–3 retain their existing encoding when spatial
state is absent. Decoder verifies checksum, byte/count bounds, ordering, tags,
booleans, finite/ranged declarations, asset kinds/references, duplicate/colliding
IDs, one punctual light kind per entity, and final entity hierarchy. Limits remain
128 MiB scene/one million entities, with at most 100,000 mesh declarations,
16 directional lights, 4096 point/spot lights, four directional shadow lights,
one environment and 100,001 visual references. Expanded primitive and GPU geometry
limits remain host startup checks: declaration count is not primitive count.

`CookedScene::with_spatial` and `StructuralCommand::SetSpatial` have typed values.
SetSpatial adds/replaces/removes only configuration against an immutable startup
asset table. A batch sees final configuration/topology; rejection retains its
queue and publishes no component, parent, or generation prefix. Configuration-only
batches borrow unchanged declarations and avoid copying the parent graph.
Generations advance once for an accepted relevant change, not numeric motion or
no-op replacement; overflow rejects atomically. Despawn removes declarations.
Plain-world restore invalidates both generations. Spatial saves/restores refuse,
including after removing the last declaration (the prior asset/host state has no
save format yet).

`NativeWorld::spatial_view` lends stable parent-before-child indices, parent index,
binary ID, continuous f64 local Transform, committed f64 global matrix and optional
configuration. It reads retained topology and ECS components, never hierarchy
validation scratch. There is no frame allocation, project/ancestor clone, string
formatting or JSON conversion in this extraction. It is a read-only scoped borrow;
no mutation overlaps it. Indices are valid for that world's topology generation,
not Bevy table rows; declaration metadata is keyed by its binding generation.
Consumers must still read numeric values each committed frame and reset their
caches for a newly constructed world. Generation equality across world instances
is not an identity guarantee. Immediate trusted native numeric loans retain their
existing responsibility for valid values; the checked script adapter restores
failed numeric/structural work. A terminal solver failure makes this view refuse.

Startup binding uses the shared sealed
`incant_assets_runtime::RuntimeAsset::load_visual(&VisualAssetRef, generation, cache)`.
It checks the selected model/texture cache version with the bounded readers and
exposes immutable consumed data. No competing retained wrapper or authoring I/O
enters NativeWorld. The eventual spatial host must retain these versions keyed by
binary IDs, reject missing or incompatible bindings, enforce aggregate payload and
expanded primitive bounds, select a camera explicitly, check GPU representability,
and reject a Normal texture for EnvironmentLight after loading. Authoring already
rejects that texture usage, but binary scene bytes are untrusted. The generic
asset loader intentionally accepts normal textures for other native consumers.

The current run_scene and embedded script probes have no spatial host and refuse
v4 spatial declarations or asset manifests instead of silently ignoring them.
The separately developed native_scene_audio example needs the same refusal when
these branches are integrated, as does the separately developed game session host. Native rendering, pixel placement, spatial save
schemas, asset replacement, broader components and the complete Play migration
remain open. No appearance or shipping/device performance gate is claimed.


## Verification (2026-10-10)

On macOS 26.6 (25G72), Apple Silicon, pinned Rust/Cargo 1.99.0:

- `./tools/cargo test -j2 --release --locked -p incant_runtime -p incant_cook -p incant_script`: **120 tests/doctests pass**, including the immutable loan compile-fail proof, canonical old/new formats, invalid/truncated v4 data, numeric ranges/light/asset caps, generation exhaustion, final-state structural topology/configuration rejection, archetype relocation, retained continuous f64 values, failed integration scratch isolation, script exception/deferred-work/structural rollback, and explicit save refusal.
- Real model cook plus scene cook, source directory removal, v4 loading and shared sealed loading pass in `incant_cook/tests/spatial.rs`. Wrong kind, changed recipe identity and damaged cooked bytes fail; the prior loaded version remains unchanged. This is startup data integration evidence, not a rendered/shipping game.
- `./tools/cargo clippy -j2 --release --locked -p incant_runtime -p incant_cook -p incant_script --all-targets -- -D warnings`: pass.
- Release `run_scene` and `incant_script_player` binaries: **six subprocess cases pass**. Each runs an ordinary v1 scene and refuses both a valid camera-only v4 scene and a valid manifest-only v4 scene with an explicit no-spatial-host error.
- `python3 tools/check_runtime_dependencies.py incant_runtime incant_script incant_assets_runtime`, workspace fmt, generated conventions, and diff whitespace checks pass. Runtime normal dependencies are unchanged. The cook adds only dev dependency edges for the real source-free integration test; no dependency versions/resolutions changed.

Machine-readable results and source hashes:
`docs/spikes/evidence/native-spatial-scene-2026-10-10.json`.
Local full logs are `/tmp/incant-native-spatial-{tests,clippy,runners-build,dependencies}.log`
and `/tmp/incant-native-spatial-runners.json`. This scope did not run hosted/device
performance, render pixels, or claim completion of native rendering/Play migration.

## Retained consumer identity follow-up

`SpatialView::identity()` now lends an opaque `WorldIdentity`. The consumer clones
it once at attachment and compares borrowed identities before using generation or
index caches. Its private `Arc<()>` allocation survives world moves and in-place
restores; restore still advances both generations. A newly constructed/loaded
world always has a distinct allocation, and retaining the old identity prevents
address reuse from matching a different world. This adds one startup allocation,
no frame refcount operation, no serialized identifier and no unsafe code.

The nine spatial integration tests (including move/restore/new-world distinction)
and runtime release all-target Clippy pass. Logs:
`/tmp/incant-world-identity-{tests,clippy}.log`. The native renderer adapter will
reject a different world before mutating its retained input.
