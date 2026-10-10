# Cooked atlas presentation and rendered correctness review

Status: packet prepared; launch after handoff 0032 accepts the shared wide-region
renderer and Astra supplies this branch's frozen executable/metadata. Do not use
a different installed or live-rebuilding binary. No Cargo in the visual worktree.
Required model is Claude Opus 5.5 over ACP with Max thinking explicitly verified.

Read PLAN.md revision 6, CLAUDE.md, docs/spikes/sprite-atlases.md and the completed
0032 sprite review. This packet extends the accepted sprite review to actual
cooked atlas frames. Astra owns packing, importing, resource identity, validation
and integration. You own original art/look-dev, rendered analysis and judgment.
Whole-app UI redesign remains in 0033; do not edit editor/ui during this packet.

The implementation imports typed .atlas.json sources with stable ULID frame IDs,
source images/optional rectangles, names, maximum page size and edge extrusion.
It packs unrotated frames onto one deterministic color-texture page. Current
limits are 256 frames, 8192 pixels per axis, 4,194,304 atlas texels and 4,194,304 decoded
source texels across distinct source entries. It fails explicitly if the set does
not fit. It does not rotate, create multiple pages or promise optimal packing.
Packed textures use the existing color mip/cook/cache path. Sprites sample level
zero and clamp to their resolved source rectangle. Isolated atlas mip filtering,
compression tiers, tilemaps, animation libraries and 2D physics remain open.

Use supplied artifacts/tools/incant_headless and binary.json; verify executable
hashes before and after calls. Author project edits through init/import/RPC and
shared commands, never by directly editing project or journal files. Source art
and atlas descriptors are valid authoring inputs. The existing original art in
handoffs/0032-sprite-rendering/evidence/art may be reused with provenance. Create
or adapt additional original art where it makes atlas failures easier to see.

Build real engine cases showing stable-frame selection, nearest and linear
sampling, transparent edges, flips/mirrors, pivot alignment, asymmetric nine-slice
corners, sorted overlap and pixel-perfect versus ordinary orthographic cameras.
Include a frame spanning an internal 512-texel boundary with nonzero packed origin;
check wide nearest spans, offset/rolled views and a sheet/page-size change. Use
identical authored stable frame IDs while changing another source's dimensions
enough to force actual relocation. Prove the target rectangle moved, then compare
rendered target identity, ordering, borders and filtering before and after. Check
retained old scene resources and a failed replacement where supported by the
public harness; do not invent inaccessible capabilities.

Reimport via the public path, verify Undo/Redo, capture repeated fresh processes,
then remove atlas/image source files and render from cooked assets. Keep hashes,
frame coordinates and source/binary provenance. Compare actual rendered output,
not just numerical packer metadata. Preserve before/after evidence and report
precise failures with a minimal public reproduction for Astra to fix.

Local integration passes 365 normal Rust tests, 56 GPU tests, 420 UI tests, 25 tool
tests, Clippy, strict builds, unchanged schemas/SDK, source-free stable-frame
replay and exact reimport Undo/Redo. These are evidence inputs, not appearance
approval. The saved workflow still uses legacy PlaySession; it does not establish
native ECS gameplay migration. No hosted-phone, native-editor or phase gate is
claimed. The separate pixel-camera near-tie follower defect remains open.

Allowed changes: this packet's art/helpers/evidence/result and, when needed,
clearly separated evidence references under docs/spikes. No Rust/shader changes,
SDK/transport changes, engine accounts, signing, publishing or merging. Return
concrete renderer recommendations rather than silently changing engine behavior.
Per renderer invocation: at most 128 frames and 256 MiB raw. Across this packet:
at most 2048 frames and 1 GiB raw, with no overwritten evidence. Stop and report if
this is insufficient. No native UI automation from shell commands. Keep any
account-bearing material ignored and local.

Return result.md with model/effort/ACP verification, frozen source and binary hash,
commands, artifact manifest, independent checks, representative rendered evidence,
scoped acceptance or failures, and remaining limitations. Commit with
Built-by: claude. Astra will review diffs, integrate, test, and handle hosted CI.
