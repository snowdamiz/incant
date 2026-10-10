# Cook-time sprite atlases (in progress)

This work extends the sprite foundation with typed `.atlas.json` source documents,
stable ULID frame identities, deterministic single-page packing and dependency-aware
reimport. The descriptor gives each frame a source path, optional source rectangle
and name, plus atlas size bounds and 1..16 pixels of edge extrusion. Source paths
resolve relative to the descriptor inside the granted project; source files are
not needed by runtime rendering.

The unrotated skyline packer compares bounded candidate widths, frame orders and
axis sweeps and chooses the smallest occupied area it finds. Widths come from a
fixed 32-texel grid, the widest frame and cumulative frame widths. The search
family does not depend on `maximum_size`: raising either bound retains all former
choices, so it cannot increase the selected occupied area. It preserves every
frame size/identity, but can still reject a feasible set; failure describes that
heuristic limitation, not a proof of impossibility. There is no rotation, global
optimality guarantee or multiple-page fallback. The limit is 256 frames, 8192 pixels per axis, at most 4194304
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


## Authoring follow-up to review 0034, 2026-10-10

The frozen [Claude atlas review](../../handoffs/0034-sprite-atlas-review/result.md)
accepted atlas-specific rendering conditionally on the separate wide-region work.
Its binary, captured images, hashes and conclusions remain unchanged. This
follow-up changes packing and authoring validation, so a new candidate still needs
Claude's rendered delta review after the shared renderer update is integrated.

Two independently reproduced packing regressions now pass: the explicit three
frames (1200×40, 1100×4 and 4×1100, padding 2) fit 1204×1148; an 11-frame case no
longer grows from 31,744 to 32,000 page texels when its height bound increases.
The packer remains bounded by 256 frames and 8192 texels per axis, and preserves
unrotated source sizes and nonoverlapping extrusions. Page-area selection is a
cook-time heuristic; no phone memory, cook-time or runtime performance gate is
claimed by these tests.

`incant_import::validate_cached_sprites` checks authored sprites against the
current cooked texture and atlas layout. It reads each unique referenced texture
once, drops its pixel payload immediately, retains layout metadata, and never
opens source images. Diagnostics identify scene, entity, texture, optional frame,
JSON pointer, severity and typed code. Missing frames, a frame ID on an ordinary
texture, out-of-page rectangles, incompatible nine-slice borders, non-color
textures and missing/corrupt caches are errors. Headless `validate` retains its
existing fields and adds `diagnostics`; `valid` now includes these cached sprite
checks and hard errors return a failing exit code. Other asset types' caches are
not covered. Pure `Project::validate` and generic document commands remain
filesystem-free; a syntactically valid but unresolved frame can still be authored
and is identified by explicit cached validation.

Import preparation checks every sprite referencing the prepared texture versions
before returning a committable batch. A hard binding error rejects the whole
explicit batch: no asset-registry publication, revision, journal transaction or
history entry changes. Cooking can leave unreferenced content-addressed cache
blobs, which are not authored state. Stale snapshot checks still precede commit.
The watcher uses the same preparation path: it keeps a rejected asset's old
fingerprint, reports affected entities and can continue importing unrelated valid
sources under its existing per-source failure policy. Repair, Undo and Redo use
existing command-bus history and cached versions, including after source removal.

Raw pixel rectangles on atlas assets remain valid for compatibility and deliberate
page inspection. They now carry a `raw_atlas_region` warning in import outcomes,
agent import replies and cached validation: repacking can make them address
unrelated content. Stable frame IDs are the relocation-safe form. No automatic
rewrite of authored rectangles or GUI-only mutation was added. Native Inspector
presentation of these cached diagnostics remains future read-only UI binding.
The renderer's fallback diagnostics and sampling code are unchanged.

CLI failures now print error Display text instead of Rust Debug structures.
Atlas dimension, texel-budget and padding failures have separate typed errors.
Linear-sampling repack differences of up to two levels remain the reviewed
limitation; this work makes no byte-equality claim for those pixels.

The rebuilt public CLI also packs the three-frame reproduction into 1204×1148
under all three tested bounds (1204×1148, 1204×1200 and 1204×2048). Claude's
unchanged 130-frame source set packs into 1204×1148 under a 1204×1170 bound and
1222×1104 under a 2048×2048 bound. Independent metadata checks preserve all source
sizes and verify padded bounds and nonoverlap; no pixels were rendered or reviewed.
The executable SHA-256 is
`c5256ad99407cea62002edf0ede9e8787d71c097418509b0d133f46a8e9c5fc2`.

Verification passes 374 normal workspace release tests (56 GPU-dependent tests
remain ignored), workspace all-target release Clippy with warnings denied, 25
Python tool tests, formatting, generated conventions, the authoring-free dependency
guard for `incant_types` and `incant_runtime`, and unchanged regenerated schemas
and SDK. The import/CLI tests exercise atomic batch failure, shared references,
watcher recovery, exact Undo/Redo, source removal, corrupt caches and warning-only
success. Source and workflow inputs stayed frozen during Cargo. Compact results,
source-art and log hashes are in
[authoring evidence](evidence/sprite-atlas-authoring-2026-10-10.json).


### Short-span integration for follow-up review

`4194240` combines the reviewed skyline/diagnostic authoring corrections with the
64-texel shared sprite renderer. The single conflict kept atlas region resolution
and the new 101,400-vertex bound. All 374 ordinary workspace tests, 56 ignored GPU
checks, release workspace Clippy and the headless build pass. The frozen binary
SHA256 is `734811cb6053e32cce1ccd0fbb98fb7a515106cf7a8442f93b0be297af065469`.
Claude's focused atlas delta is pending; 0032's above 128× and two-axis frame-edge
sampling limitations still apply. This is not a native gameplay or device gate.


## Follow-up rendered acceptance, 2026-10-10

Claude Opus 5.5 at explicitly verified Max reviewed the frozen `4194240`
headless binary (`734811cb6053e32cce1ccd0fbb98fb7a515106cf7a8442f93b0be297af065469`).
The [follow-up report](../../handoffs/0034-sprite-atlas-review/result.md) accepts
atlas-specific behavior, conditional on the shared renderer limits from 0032.
It adds 201 frames / 239.8 MiB under a separate 600-frame / 512-MiB budget,
preserving round one's evidence unchanged. All 505 engine calls retained the
executable hash. Thirteen evidence-tool tests and all 55 retained screenshot
hashes pass independently after integration of Claude's `5a9ad02` commit.

The three-frame packing reproduction fits 1204×1148. The 130-frame page uses
1222×1104 instead of 1204×1865, and all nested pairs in the 69-bound sweep
preserve fit and nonincreasing page area. Thirteen rejected imports preserve
project/journal/revision/history and asset fingerprints; accepted replacements
and Undo/Redo restore exact fingerprints. Nearest atlas/control views, frame
relocation, source-free rendering and repeated fresh processes agree. The
source-Y cuts at 36×/40×/56× pass 4,972 tested exact ties.

Limits remain explicit: the heuristic can reject a feasible 1217×1104 layout;
missing-cache diagnostics repeat once per sprite; decoded pixels, not PNG
container bytes, define cross-build equality. Shared one-axis failures above
128×, two-axis edge/non-dyadic ties, and the 342-sprite geometry limit remain
open. This is one Apple M5 Pro/Metal result, not arbitrary-zoom, cross-platform,
native editor, reference-phone performance or phase-gate approval.

Actual main53 was merged as `70184b1` after the review. Conflict resolution
retains main's CI timing and pinned-tool cache changes and all atlas/sprite
workflow probes; no renderer source conflict occurred. The resulting union
requires its own full correctness checks and hosted CI before publication.

## Reviewed runtime union ready for hosted checks, 2026-10-10

The publication candidate integrates PR57's reviewed physics, native numeric/audio
extraction and Tauri build-context stack. Renderer, atlas packer and import source
remain unchanged by those merges. It passes 451 ordinary release workspace tests,
all 56 explicitly executed GPU tests, all-target release Clippy, 38 tool tests,
420 UI tests, UI/SDK builds, unchanged generated schemas, workflow lint and the
authoring-free dependency guard for all seven shipping roots. The combined
custom-protocol editor/headless/cook build passes.

The rebuilt public atlas, ordinary-sprite and both camera probes preserve stable
IDs, exact Undo/Redo and saved/repeated state after source removal. These probes
still use the legacy PlaySession; native 2D migration is open. Exact source, binary
and log identities are in [publication evidence](evidence/sprite-atlas-publication-union-2026-10-10.json).
The accepted atlas-specific visual evidence and all shared rendering limitations
above remain unchanged. Hosted checks, native editor integration and phone gates
remain distinct requirements.
