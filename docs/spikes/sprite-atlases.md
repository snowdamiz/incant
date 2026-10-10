# Cook-time sprite atlases (in progress)

This work extends the sprite foundation with typed `.atlas.json` source documents,
stable ULID frame identities, deterministic single-page packing and dependency-aware
reimport. The descriptor gives each frame a source path, optional source rectangle
and name, plus atlas size bounds and 1..16 pixels of edge extrusion. Source paths
resolve relative to the descriptor inside the granted project; source files are
not needed by runtime rendering.

The unrotated guillotine packer compares several candidate widths and chooses the
smallest occupied area. It preserves all frame sizes/identities and rejects an
unfittable set explicitly; it does not promise an optimal layout or split into
multiple pages. The limit is 256 frames, 8192 pixels per axis, at most 4194304
atlas texels, and at most 4194304 decoded source texels across distinct source-path
entries. Existing source-byte and runtime resource budgets also apply.

Packed content and repeated edge pixels use the existing color texture cooker,
linear-light alpha-aware mip generation and KTX2 cache. Sprites continue sampling
level zero with their own source-region clamp; isolated atlas mip filtering and
texture compression tiers remain open. Atlas textures are color assets and cannot
be reinterpreted as normal or linear data maps.

Version-two texture metadata records the stable frame layout and all source hashes;
ordinary version-one image cache entries retain their existing representation and
keys. The importer and source watcher reuse the shared atomic command path. Stable
frame lookup is integrated so a repack can change rectangle placement without
changing an authored sprite reference.

Local checks completed before the plan migration was prioritized: 76 focused
2D/assets/import/command tests, five sprite GPU tests (including stable frame
relocation, retained versions and source-free loading), workspace Clippy, generated
schemas/SDK and strict TypeScript. The public probe passes source-free saved and
repeated legacy PlaySession state, stable frame/asset IDs and exact reimport
Undo/Redo. Evidence logs remain in `/tmp/incant-atlas-*`; probe artifacts are in
`artifacts/atlases/public-r1`.

The foundation is preserved while PLAN.md revisions 4/5 runtime work takes
priority. Full workspace/device performance, atlas Claude review, native packaging
and hosted CI remain open. The old document-backed gameplay probe does not
satisfy the new runtime-data-path or reference-phone gates. This is not a completed
tilemap/2D physics implementation or phase-gate pass.

## Integration verification, 2026-10-10

The region-local nearest sampling path and typed reimport diagnostics are now
integrated with stable atlas frame resolution. The diagnostic carries the final
packed pixel rectangle; a failed nine-slice after frame lookup identifies the
entity and texture without publishing a replacement scene. Its retained-scene
regression and the ordinary-texture shrink regression both pass.

Before the magnification follow-up, the integrated branch passed workspace
Clippy, 330 Rust tests across 106 suites, and all 54 explicit GPU checks
(including six sprite checks). The subsequent magnification correction is now
integrated as `7fe87ef`: the tie tolerance scales with the signed derivative to
remain fixed in screen pixels. All seven affected sprite/atlas GPU tests pass,
including the new 70-case magnification sweep, the 48 orientation/scale/sheet
cases and retained atlas-frame behavior. The focused command was
`./tools/cargo test -j2 -p incant_render --release --locked --test model_gpu sprites -- --ignored --test-threads=1`.
Its log is `/tmp/incant-atlas-magnification-integrated-gpu-r2.log`.
Claude's independent correction review and scoped atlas rendered acceptance
remain open; passing numerical tests do not replace that evidence.
Logs: `/tmp/incant-atlas-integrated-tests.log`,
`/tmp/incant-atlas-integrated-clippy-r3.log`, and
`/tmp/incant-atlas-integrated-all-gpu.log`.

## Runtime foundation and wide-span integration, 2026-10-10

The atlas branch integrates the sprite candidate `e4ec2a1`, including PR48's
native runtime/CRT foundation and the reviewed CI tool-build follow-up. Conflict
resolution retains stable atlas frame lookup and passes its resolved integer
pixel rectangle to the nearest span splitter and typed diagnostic. The shader is
byte-identical to the sprite candidate. Geometry tests retain the typed
`SpriteRegion` wrapper; both ordinary-sprite and atlas GPU suites remain enabled.

The integrated source passes 365 normal workspace tests and all 56 ignored GPU
tests, including stable atlas-frame relocation/retained scenes and the 864-case
wide nearest sweep. Workspace Clippy, formatting, 25 tool tests, 420 UI tests,
strict UI/SDK builds, workflow lint and the authoring-free dependency guard pass.
The combined editor/headless/cook/collaboration release command succeeds; generated
schemas and SDK are unchanged by the merge. The GPU invocation reuses the workspace
graph instead of rebuilding a package-specific graph.

On headless SHA-256
`0257f7617d81a479788aaacd40697fe524978ace4a1c136e3a3b057ec0ffd8aa`,
the atlas probe passes stable asset/frame IDs, actual packed-frame relocation,
exact import Undo/Redo, source removal and exact saved/repeated runtime state.
Ordinary sprites and both camera projections pass their public saved-workflow
probes on the same executable. Authored project/journal bytes remain unchanged
by play. These still exercise the legacy PlaySession; they do not establish
migration to native gameplay. The eval check remains schema-only.

Evidence is in [integration results](evidence/sprite-atlas-runtime-spans-2026-10-10.json).
Claude's rendered wide-region review currently uses the ordinary-sprite frozen
candidate, not this atlas binary. Atlas-specific rendered/native review, hosted
checks and phone/performance gates remain open.
