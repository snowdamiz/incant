# Static FBX import through ufbx

The existing shared asset transaction now accepts `.fbx` (case insensitive) alongside glTF/GLB. The authoring-only importer converts supported static scenes into the same `ImportedModel` and `INCMOD02` meshopt/KTX2 writer. No project schema or runtime asset format changes. Registered model ULIDs survive reimport; command history, batch atomicity, cache loading and retained asset publication keep their existing paths. The editor and headless tools remain usable offline without an engine account.

Source import is available on native hosts. The pinned ufbx C build requires a native libc and does not build for `wasm32-unknown-unknown`; browser FBX import/cooking returns a typed `Unsupported` error before source or cache access. The pinned meshopt package also does not supply browser encoder/optimizer symbols, so browser glTF/GLB source cooking explicitly refuses before filesystem access. This previously unreachable writer was not a working browser capability. Native writer behavior and portable checked cooked-model readers remain unchanged; full browser model-source cooking remains open. The actual browser smoke exercises direct FBX import, FBX/glTF cooking and case-insensitive mixed-format model dispatch refusals; no browser build or prior smoke check is skipped.

## Source parser and conversion

The exact registry dependency is `ufbx = 0.11.5`, which bundles ufbx C 0.23.1. Its Cargo.lock checksum is `260522f1eb74139f0e78b29dd82a58e5437d728c5498024a39e5af7282694240`. This uses the MIT alternative, with the full notice in `licenses/ufbx-MIT.txt`; no Autodesk SDK is included. Primary implementation references are [ufbx-rust](https://github.com/ufbx/ufbx-rust), [ufbx C v0.23.1](https://github.com/ufbx/ufbx/tree/v0.23.1) and the [ufbx documentation](https://ufbx.github.io/).

`ufbx::load_memory` receives the bounded source snapshot, an explicit FBX format, strict parsing and abort policies for invalid Unicode or indices. External file loading is disabled. Geometry and nonstandard inheritance transforms become ordinary helper nodes, preserving mesh instancing and hierarchy. Conversion uses right-handed Y-up meter space. Nonpositive/nonfinite source units reject explicitly: ufbx intentionally skips unit conversion for nonpositive settings, which cannot satisfy this import contract.

Triangles/polygons preserve corner normals and UV seams. Triangulation is bounded per face; UV0's V coordinate is converted to the existing top-down image convention. Normal-map tangents use the existing MikkTSpace path after that conversion. Instances with different material bindings receive distinct primitive bindings while identical instances share geometry. The output uses the existing finite f32 model format, including negative determinants and nonuniform hierarchy transforms.

The material subset is Lambert base/emissive color and achromatic opacity, plus the explicitly requested ufbx Blender metallic/roughness interpretation. Base-color, emissive and normal file textures support embedded content or project-relative sources; repeat/clamp modes are retained. PNG/JPEG/EXR decoding, color/normal roles, mip generation, KTX2 and material validation are shared with glTF. Texture dependencies are snapshotted by `SourceSet`, never by the C parser, and absolute/network/escaping paths refuse. Cached recipe fingerprints cover source/dependency bytes and the pinned conversion recipe. They are cache identity, not authentication.

## Explicit unsupported data

Authored animation channels, skinning/bones, morph targets, vertex caches, cameras/lights, constraints/characters/LOD, procedural/NURBS/curve geometry, hidden nodes and unknown objects reject the whole import. Empty animation containers contain no animation data and are accepted. Point/line/empty/hole faces, subdivision, vertex colors/crease and multiple UV sets are unsupported.

Legacy Phong/custom shaders, ambient/reflection contributions, colored transparency, advanced lobes, unsupported or disabled texture channels, layered/procedural textures, UV transforms, non-UV0 references, metallic/roughness/factor/transparency maps and out-of-range factors refuse. This is a static import subset; it neither approximates every DCC material nor completes the plan's animation/skinning or full FBX capability.

## Bounds and failure semantics

The existing source snapshot cap is 128 MiB across at most 1,024 dependencies. ufbx temporary/result allocator limits are 128/256 MiB and one million allocations each; node depth is 256. Conversion caps are 100,000 nodes, 10,000 meshes/materials/output primitives, 1,024 source textures, one million instance/material references, 4,096 corners per polygon, one million expanded vertices and three million indices. Names are at most 4,096 bytes. At most 256 decoded role-specific images and 128 MiB of decoded mip bytes are retained. Existing bounded image decoding and 128 MiB cooked-model limits still apply.

These are individual resource limits, not an aggregate peak-memory or wall-clock deadline claim. Source snapshots, parser storage and converted output can coexist; image decoding can temporarily hold one bounded candidate beyond the retained aggregate. No runtime/device performance gate is claimed.

Import preparation occurs before one existing `incant_cmd` transaction. Failure preserves project, journal, registry identity and retained asset version. Content-addressed orphan cache blobs from an earlier prepared item are allowed; no partial project mutation is published. Runtime model readers consume only the cooked format after sources are removed. The normal runtime graph rejects `ufbx` explicitly, while authoring tools link it.

## Evidence

Scoped release tests cover real ASCII/binary Maya, Blender and 3ds Max fixtures; explicit centimeter/inch and Z-up numeric oracles; reflected nonuniform transforms; geometry instancing; seven-material primitive assignments; embedded/external images; source-free cache reads; malformed indices/UTF-8/truncation; parser allocator refusal; nonpositive units; and actual animated/skinned/morphed input refusal. Import integration covers stable IDs, cache hits, changed reimport, Undo/Redo, journal recovery and invalid batch/reimport atomicity. Fixture provenance and SHA-256 hashes are beside the unmodified upstream files, with their license.

The final scoped run passes 71 Rust tests; current-main integration additionally passes 127 asset/script/game tests and matching Clippy. All 531 UI tests, strict TypeScript/build, 50 tools tests and normal runtime dependency checks pass. Eleven real CLI processes exercise stable IDs, cache hits, reimport, Undo/Redo and unchanged project/journal on refusal. Cached model consumption after source deletion is tested directly in Rust; CLI validation alone does not claim that it loads model geometry. All ten committed fixture hashes are verified, including original CRLF bytes preserved by a narrow Git attribute.

Exact executed checks and source/binary identities are recorded in the accompanying evidence JSON. Claude Opus 5.5 at explicitly verified Max owns the separate static geometry/material/texture rendered acceptance, including normal-map orientation. No screenshot or visual judgment by Astra substitutes for that review. Hosted desktop results and full FBX/animation/device/export gates remain separate.

The PR77 platform correction passes the same 71 native tests and native/wasm release Clippy. An actual Chromium 156.0.8078.4 run executes all seven source-cooking refusal routes plus the existing 120-tick platform smoke; wasm-bindgen is pinned to 0.2.126 and GPU is disabled for this browser check. The native writer body and all three FBX conversion files are byte-identical to the original reviewed head. The Windows journal fixture now releases its persistent lock before byte inspection and verifies recovered project/revision after reopening, matching the existing atlas test pattern. [Correction identities and logs](evidence/static-fbx-platform-correction.json) preserve the original failures; hosted Windows rerun remains required.

## Shared normal-map correction

Claude0040 accepted static conversion and colour/reimport controls but exposed an
existing generated-tangent handedness inversion shared with glTF. The original
review is preserved; [the narrow correction and cache policy](generated-tangent-correction.md)
record 101 ordinary and 61 numerical GPU tests plus the accepted, hash-pinned
Claude Opus 5.5 Max delta (12 captures, 34 successful processes). Supplied tangents,
UVs, shaders and old immutable cooked versions retain their behavior. This is a
scoped tangent correction; the separate shared OPAQUE/emissive/environment
alpha-weighted mip defect remains open. Corrected integrated hosted checks remain
pending.
