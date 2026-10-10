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
