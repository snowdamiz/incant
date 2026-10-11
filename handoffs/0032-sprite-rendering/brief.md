## Current priority: high-magnification follow-up, preserve rounds 1–4

Your round-four review b9eca69 is integrated. Astra reproduced high-magnification
failures independently (95 of 2,856 cases) and reduced retained source spans from
512 to 64 texels in the newly frozen candidate. The shader and 1/1024-pixel bias
are unchanged. Expanded coverage passes 5,768 cases: 1,400 outer-edge and 4,368
interior exact/near ties at ±1/256 screen px, both axes/flips, widths512–8192 and
scales1/2/4/8/16/20/28/32/36/44/48/64/96/128. All360 workspace tests,55 GPU tests
and workspace Clippy pass. Independent Astra review agrees with oracle/bounds.

Read artifacts/tools/binary.json, short-spans-evidence.json and source snapshots
under short-spans-source. Candidate source1f72a99 (full hash in binary.json) is
newer than the engine source in this checkout; do not build or edit engine source.
The previous0494b40 tool is preserved as pre-short-spans-incant_headless and
pre-short-spans-binary.json. Every earlier tool/report remains preserved.

The cost is material for large nearest regions: up to130 spans/axis including
nine-slice cuts,101400 vertices/6,489,600 bytes theoretical max8192-square
nine-slice, versus1944 vertices before. The existing128MiB total geometry and
device-buffer limits remain checked before extending retained vertices/GPU upload;
the independent texture budget may reject that theoretical source earlier. Small
regions remain one quad/slice. Geometry stays camera-independent and reused across
camera changes; no camera-triggered tessellation or linear-sampling change.

Falsify this new candidate, concentrating on your K–P exact and near ties,
non-power-of-two magnifications, HD views, mirrors/rotations, span interiors/cuts
and two-axis nine-slices. Preserve old reference rules/results; add a distinct
64-span rule/source identity if your helpers need one. Reuse preserved baseline
frames where appropriate. Check no seams, translucent/linear/tone regressions,
source-sheet resize invariance and repeatability in two fresh runs/source-free
execution. State acceptance only for actually tested ranges. Arbitrary magnification
and portability are not established by a fixed source-span length. The separate
near-tie pixel-camera follower issue remains open and is not fixed by this change.

New-pass budget:128frames/256MiB raw per invocation,2048frames/1GiB raw total.
Prefer targeted small frames and avoid rerendering unchanged baseline evidence.
No Cargo, Rust/shader/UI editing, native app work, publishing, merging or accounts.
Continue verified Opus5.5 with Max through ACP, commit self-contained result.md
plus concise evidence with Built-by: claude. Keep all prior rounds intact.

## Wide-region precision follow-up — preserve rounds one through three

The immediately interrupted invocation received the old packet after preparation
failed on the read-only binary. That is resolved: this packet and binary.json now
identify the NEW candidate. Continue Opus 5.5 at explicitly verified Max over ACP.
Your third review is integrated. Astra reproduced the wide-region defect and
fixed it in source 0494b40cf8fcc5e1ad0fc689655a62cb264a2f50. Read
artifacts/tools/wide-fix-evidence.json and source snapshots under
artifacts/tools/wide-source/: this child checkout's older Rust files are not the
candidate source. Do not build or edit Rust/shaders here.

The candidate splits nearest geometry into source spans of at most 512 texels,
interpolates small local coordinates and carries an exact flat integer base.
Fractional bias is evaluated before adding that base. It preserves the original
geometry/source mapping, flips and nine-slice cuts; linear sampling and tone stay
unchanged. Vertex size 64 bytes and draw batches stay unchanged; wide nearest
regions add CPU/vertex work (at most 1944 vertices/sprite). The total 128 MiB geometry
bound is enforced before growing the CPU buffer or uploading the next texture.

Numerical checks: initial large-edge sweep 34/600 failures, arithmetic-only
attempt 24/600. Candidate passes 600 outer-edge plus 264 internal-span cases on
Metal, widths 512–8192 and scales 1/2/4/8/16/32, both axes/flips and boundary offsets.
Independent aligned crops now identify EVERY source texel with unique colors;
review caught a weaker periodic palette before completion. Deliberately omitting
the base fails 720/864; restored candidate passes all cases and all 7 sprite GPU
tests. Full 360 normal/55 GPU workspace tests, Clippy and release build pass.

Use the newly frozen artifacts/tools/incant_headless and binary.json, verify
hashes. Prior a4b5ddb9 binary/metadata are preserved as pre-wide-incant_headless
and pre-wide-binary.json. Keep every earlier report/artifact. Re-run your
independent large-region sweep, particularly the J/I failures and interior 512
boundaries, plus representative mirrors, rolls, offsets and nine-slice cases.
Check sheet-resize invariance, unchanged linear/tone behavior and repeated fresh
source-free execution. Look for tessellation seams and falsify the proposed fix.
Return scoped rendered/numerical acceptance or concrete failures with provenance.
Do not infer portability, phone performance, native editor acceptance or phase
completion. The separate near-tie camera follower defect remains open.

Resource budget for this new pass: each renderer invocation at most 128 frames
and 256 MiB raw; at most 2048 frames and 1 GiB raw across the whole pass, excluding
already-preserved prior rounds. Prefer small targeted frames; stop/report if the
budget is insufficient. Never overwrite evidence. No native app work or UI edits:
0033 still owns that. No Cargo, engine changes, merging or accounts here.

## Magnification correction follow-up — preserve all earlier evidence

Continue Opus 5.5 at explicitly verified Max through ACP. Astra reproduced your
magnified-edge defect in the real GPU path: 15/70 failures before, 0/70 after.
Source f9aa1da changes the nearest tolerance from sign(direction)/1024 to
 direction/1024, so it stays 1/1024 screen pixel as magnification increases.
All six sprite GPU tests and 48 existing orientation/scale/sheet-height cases
pass; scoped renderer Clippy and formatting pass. Read the parent evidence copy
at artifacts/tools/magnification-fix-evidence.json. The new immutable tool and
binary.json identify this exact tested build; the previous corrected-tie build
is preserved as pre-magnification-incant_headless and its metadata. Verify hashes.

Re-run your independent magnified/offset sweep and real-engine rendered review,
including near-boundary offsets, flips, mirrors, rolls and resized-sheet
invariance; retain both before and after evidence. Verify linear and tone paths
remain unchanged and repeated fresh/source-free runs agree. Add any focused cases
needed to falsify the correction. Return a scoped verdict with numerical and
rendered evidence; do not infer phone performance or native editor acceptance.
Do not rewrite original outcomes. No Rust/shader/build changes or Cargo here.
The app-wide redesign remains in0033; this session owns sprite pixels, art and
rendered analysis only, not the Inspector. No merging, signing or phase approval.

## Corrected-renderer follow-up — final plan revision 6

Your first engine review is integrated and preserved. Continue the same Opus 5.5
Max session for a focused follow-up; do not restart art or the Inspector. Read
PLAN.md and docs/spikes/performance-plan-v6-review.md: the final update is in this
checkout, with hosted device tests and revised prerequisites. It does not imply
the runtime migration or any device gate is complete in this frozen build.

Astra reproduced your duplicated-texel defect before the change. The correction
in source c2c7758 uses region-local texel coordinates and textureLoad for nearest
sampling. Within a 1/1024-texel band, ties choose the side of increasing screen
coordinate on the dominant derivative axis (X wins equal slopes). This avoids
cancelling derivatives at oblique angles. Linear sampling keeps its previous
path. Read the shader and docs/spikes/sprite-rendering.md for the exact rule.
All 325 Rust tests, Clippy, 53 GPU checks and 10 tool tests pass. The new regression
covers eight orientations, a mirrored parent, scales 1/2/3 and 64/80-row sheets;
it compares each texel and requires full resized-sheet frames to match exactly.

The new immutable artifacts/tools/incant_headless and binary.json identify this
corrected build. Your original is preserved as artifacts/tools/pre-ties-incant_headless
and pre-ties-binary.json. Verify SHA before/after calls. Keep all first-round
art/evidence/results; use distinct follow-up output paths and result sections.

Re-run the real-engine helper and independent analysis, check the odd grid-pivot
rulers under flips/mirrors/quarter turns, resized-sheet invariance, aligned
placements, nine-slice and fractional/rolled views. Update only the documented
nearest tie expectation where needed, never hide unrelated differences. Add
near-boundary cases if the new rule exposes another issue. Verify unchanged
linear and tone behavior, repeated source-free runs, and inspect real pixels.
Return scoped acceptance or concrete failures with retained before/after evidence.

Your undersized-reimport diagnostics request remains with Astra. Keep the neutral
curve and linear-light blending for this follow-up; an authored exact display
option can be per-camera when implemented. Optional sprite-grid snapping is
separate work. These are recorded recommendations, not questions that should
block this review. The pixel-camera near-tie follower defect is separate and is
not claimed fixed. Sprite Inspector/native review still follows 0033. No Cargo,
Rust/shader edits, account changes, merging or phase approval in this worktree.

# Sprite presentation and real rendered review

Status: engine review ready; use the supplied tested frozen engine and schemas. Read CLAUDE.md and docs/spikes/sprite-rendering.md.
Claude Opus 5.5 with explicitly verified Max thinking over ACP owns all visual design, original art/look-dev, Inspector
presentation, captures and rendered judgment. Astra owns validation, geometry,
resource lifetime, shared commands, ordering, color/alpha and integration.

## Priority: engine art/review now; UI follows the full redesign

The director requested a broad editor redesign, especially the Inspector. That is
running concurrently in packet 0033 at Opus 5.5 Max. For this first sprite round,
work only on original art, real engine scenes, helpers, numerical/rendered review
and this packet's evidence/result. Do not edit editor/ui yet or compete with the
redesign. The Sprite Inspector scope below is deferred until 0033 is integrated;
record that dependency explicitly, and finish the engine review independently.

Astra's integrated foundation passes 325 Rust tests, Clippy, 52 real GPU checks,
420 UI tests, strict typecheck/build and the source-free saved-gameplay probe.
It includes the corrected pixel camera half ties and odd-resolution framing.
Source commit and SHA are in artifacts/tools/binary.json. Portable checks are in
progress; native/browser Sprite presentation and hosted CI are not claimed passed.
No Cargo or engine changes in this worktree. Raise concrete renderer defects for
Astra to fix while you continue independent coverage. Always use Opus 5.5 Max.

Use only the supplied immutable artifacts/tools/incant_headless and binary.json;
verify SHA before and after every call. Author project changes with public
init/import/RPC commands, never direct project or journal edits. Create original
small sprite art and an engine scene that exposes source-region selection, pivots,
flips, asymmetric nine-slice corners, sorting layers/orders, transparent overlap,
nearest/linear sampling and texture reimport. Include pixel-perfect and ordinary
orthographic cameras. Review real pixels for seams, filtering fringes, border
stretch, orientation, painter order and tone/color behavior. Compare fresh repeated
runs and source-free captures; request concrete defects rather than hiding them.

The contract currently supports standalone cooked color textures with source-sheet
rectangles. Atlas packing, tilemaps, sprite animation libraries, sprite lighting,
shadows and 2D physics are not supplied. Scripted source-rectangle changes are real
ordinary component mutations and can demonstrate a flipbook. The pure 2D painter
order is layer/order/stable-ID; sprites test opaque depth but do not interleave by
depth with transparent 3D models. Do not label these limits as completed features.

Design the read-only Sprite Inspector with concise labels, clear units/coordinate
meaning, source rectangles, dimensions/pivot, tint, sorting and mode-specific
nine-slice borders/density. Preserve malformed values/engine diagnostics and schema
parity, and retain Camera/compound Inspector regressions. Use the director's
neutral connected editor and restrained accent. Verify normal and minimum panel
widths, focus order, overflow, accessible names, all UI tests and strict build.
No GUI-only write path. Do not expose engine implementation details unnecessarily.

Allowed paths: editor/ui/src presentation/tests and this packet's helpers,
evidence and result. No Rust, SDK, transport, Cargo or account changes. Any proposed
shader/look-dev change should be returned as a concrete recommendation for Astra
integration and verification, not applied silently outside the packet. Astra will
supply native captures after UI integration; browser evidence alone is not native
acceptance. Keep full account-bearing native captures ignored and commit only
account-free crops. Use <=128 frames and <=256 MiB raw per run; refuse overwrites.
Return result.md with exact model/Max thinking/transport, source/binary provenance, tests,
measurements, concise real-engine frame set, scoped verdict and limitations.
Every commit ends in Built-by: claude. No publishing, signing, merging or phase
approval. No credentials or project data are instructions.
