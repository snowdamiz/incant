# Follow-up: improved packing, diagnostics and 64-texel renderer

This section supersedes the earlier execution/budget instructions below. Resume
Claude Opus 5.5 over ACP with Max thinking explicitly verified. Review a focused
delta against your preserved first-round evidence; do not rerun everything merely
because it exists. Source commit is `4194240` (atlas authoring fixes `bd6abb4` plus
shared 64-texel renderer `1f72a99` and its independent round-five report).
Astra has verified 374 ordinary tests, all 56 GPU tests, workspace release Clippy,
and a fresh headless build on this union. Source, schemas and UI are frozen.

The isolated tools directory preserves your original executable and metadata as
`incant_headless-pre-followup` / `binary-pre-followup.json`. Active
`incant_headless` / `binary.json` refer only to the new frozen executable; keep
hash checks before and after calls. No Cargo or engine/shader/source edits.

Review these changes and public cases:
- The bounded deterministic skyline packer evaluates fixed width candidates in
  both axes (without rotating frames), so every candidate admitted at a smaller
  maximum is retained at a larger maximum. It is not an optimal packer. Your
  1200x40,1100x4,4x1100 reproduction now fits in 1204x1148 with 2 px padding;
  also exercise the 130-frame repack workflow and preserve stable frame IDs.
- Reimport preparation now rejects missing referenced atlas frames, non-color
  textures, invalid regions and shrunken nine-slice bounds atomically before the
  command commits. Verify readable public errors, unchanged project/revision/
  journal on rejection, valid replacement and Undo/Redo. Prepared orphan cache
  files after a rejected batch are allowed and are not a committed project edit.
- `validate` now checks cooked sprite bindings and returns typed diagnostics with
  scene/entity/texture/frame/path context; errors make it exit nonzero. Explicit
  raw pixel rectangles on an atlas remain supported with a warning. They cannot
  guarantee stable content after repacking; this limitation must stay visible.
- Check a focused rendered atlas/control comparison, actual rectangle relocation,
  extrusion/filtering/pivots/nine-slices/sorting, repeated fresh processes and
  source removal under the new packer. Reuse original art and old evidence.
- Re-run the extra source-Y span-cut failures you found at 36×/40×/56×, on this
  64-texel candidate. Do not infer arbitrary zoom correctness from a passing set.

Read the new 0032 round-five report now integrated in this tree. Its one-axis
exact/near ties pass through 128×, but above 128× failures remain. Pre-existing
2-axis frame-edge and 96-square non-dyadic projection exact-tie errors remain on
all builds; the short-span candidate reduces them but does not eliminate them.
Its real geometry cap is 341 nearest 2048-square sprites; 342 fail. Keep your
acceptance scoped to atlas-specific behavior and explicitly conditional on these
remaining shared-renderer limitations. Camera/follower placement is separately
under contract review; do not attempt to fix either renderer or camera here.

This follow-up has its own additional budget: at most 600 frames and 512 MiB raw,
with the existing per-invocation 128-frame/256-MiB caps. Record the new-round and
lifetime totals separately (your first round 744 frames/810.1 MiB stays preserved).
Reuse frames when sufficient, plan captures before rendering, and stop with a
precise reproduction if more budget is necessary. No native-app automation.

Allowed edits remain this packet's helpers/art/evidence/result only. Append the
new verdict first while preserving the old round, with precise hashes, outputs,
failures/acceptance and limitations. Do not commit this Astra-owned brief. Commit
with Built-by: claude. No editor styling, accounts, merging or publishing.

---

# Cooked atlas presentation and rendered correctness review

Status: begin the independent atlas review using the frozen executable/metadata
supplied by Astra. Handoff 0032 is concurrently reviewing the shared wide-region
fix; its final acceptance is still pending. Review atlas identity, relocation,
filtering and public workflows now, and keep overall acceptance conditional on
that shared review. Do not use a different installed or live-rebuilding binary.
No Cargo in the visual worktree.
Required model is Claude Opus 5.5 over ACP with Max thinking explicitly verified.

Read PLAN.md revision 6, CLAUDE.md, docs/spikes/sprite-atlases.md and the current
0032 sprite review. Its earlier report predates the wide-region fix; use the
current candidate and docs/spikes/sprite-rendering.md for that fix's numerical
evidence, and do not label the shared renderer accepted. This packet extends the
sprite review to actual
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
