# Canonical native text drawing

Claude Opus 5.5 through ACP, thinking explicitly verified as Max, owns this
rendering implementation and all appearance review. Implement the actual
canonical text draw path on the reviewed composed-text foundation. This is new
engine capability, not another whole-editor restyle. Read PLAN.md Phase 1 Game UI,
AGENTS.md, docs/spikes/native-text-input.md and
**docs/spikes/native-text-draw-handoff.md** first; the latter is the exact technical
contract for layouts, glyph bearings, units, atlas ownership and publication.

## Outcome and boundaries

The ordinary native game renderer and native editor Play must draw the owned
shaped text alongside existing 3D/2D content, using the same SceneInput,
NativeRenderHost and GPU consumer. Remove the current explicit TextUnsupported
refusals only where their implementation is actually complete. No text-only
player, fake HTML text overlay, screenshot substitution or separate frame loop.
Use the packaged fonts and retained glyph atlas; no system font fallback or
network lookup. The app remains usable without an engine account.

Choose and document a coherent initial supported drawing route: explicit logical
pixel-to-world or screen coordinates, origin/baseline, transformed placement,
depth/clipping and coverage/color blending. TextNode.world currently is the
unchanged native hierarchy transform, not a pre-agreed widget plane. Do not
silently reinterpret it or guess glyph baseline units. Keep visual policy yours,
and make its numerical meaning reviewable in source and tests. Preserve shaped
bidi order, font fallback, bearings, empty spaces and observable missing glyphs.
Read the handoff's precise alpha/atlas contracts before writing the shader.

General widget layout, editable text/IME, focus/gamepad navigation and the full
Game UI gate remain separate. Do not claim them from drawing strings. Preserve
existing unsupported-capability failures outside this supported route.

## Retention, publication and limits

Add the route to canonical scene resolution, retained preparation/staging and
actual rendering. Ordinary motion must not reshape, rerasterize, reread fonts,
rebuild unchanged glyph geometry or upload unchanged atlas pixels. Reuse immutable
layout/atlas identity across unrelated topology changes. A revision number alone
is not atlas identity; validate every handle against its actual atlas.

Preflight bounds before GPU allocation or geometry expansion. Account for accepted
old plus candidate buffers/textures and temporary geometry; do not label the
4MiB alpha plane as total text memory. Preserve current composed-world transaction
ordering: rejected preflight/consumer/unwind does not publish a partial candidate;
post-submit failures follow the existing terminal host contract. Keep existing
sprites, meshes, animation, lights, PBR, shadows and numerical tests intact.
No per-frame authoring/JSON/project clone, no credentials in artifacts or logs.

## Parallel ownership and worktree

Root and runtime_review implement numeric animation crossfade binding in a
separate checkout. They own SpatialNode.animation_blend, MeshInput.animation_blend,
render_native numeric undo/update and models/animation.rs. They may add one small
static-model guard in scene.rs that rejects animation_blend without an animation
player, plus mechanical initializer fields. Leave that logic intact on future
integration. You own text draw geometry/shader/pipeline and its canonical
scene/retained/render integration. Do not rewrite animation sampling or core
simulation, authored text documents, save codecs, script ABI or unrelated UI.
If a genuine nonvisual foundation defect blocks this route, describe the narrow
fix concretely in result.md; root can own/review that repair. Work only in this
handoff checkout, commit Built-by: claude, and do not publish or merge.

## Validation and native/GPU grant

The previous0053 editor and helpers all exited; its root runner is reaped and its
native/GPU lease is released. This packet grants0054 exclusive GPU/native testing
for this implementation. Other agents continue CPU work only. You may build your
changed source. Root grants exclusive use of CARGO_TARGET_DIR=/tmp/incant-agent-native-target
until your result and cleanup; do not use another agent's target directory.
Use ./tools/cargo and pinned --locked resolutions. Use bounded -j2 builds.

Run applicable CPU behavior and numeric Metal tests, strict warnings and format.
Prove baseline/bearing/transform and RTL placement, spaces, missing glyphs,
foreign/reset-handle rejection, atlas/layout reuse, accepted/rejected motion,
changed text, resize and failure atomicity. The existing CPU tests/probe already
prove source-deleted mixed model/text/QuickJS/audible audio input; preserve those
properties. Add actual draw checks, not tests that merely echo shader constants.

Use a separate disposable mixed scene for actual native rendered review. The
existing tools/probes/native-text-export.py creates through public imports and
command transactions and is a useful recipe, but its preserved proof directory
must not be mutated and its CPU player is not appearance evidence. Retain a
reviewable fresh fixture with Latin ligatures, Arabic, CJK fallback and spaces,
plus existing scene content. Do not edit project JSON behind incant_cmd. Reuse the
ordinary package/window or editor Play entry point; report which was exercised.
Capture only the app/window needed, with a small useful set of views at large and
minimum supported size after normal movement/resize. Review pixels yourself.
Record actual source, binary/build-profile, fixture and screenshot identities;
CPU/headless success alone is insufficient for appearance acceptance. Do not
claim phone/shipping/frame-pacing gates from a ci/debug desktop build.

Avoid disturbing the user: wait for a short idle interval before taking front,
stop input if the user switches apps and restore the previous front app/pointer
after each short burst. No provider turn or account action is needed or authorized
by this packet. If an account avatar appears, redact only that area in committed
captures and retain raw captures locally ignored. No external artwork needed.

## Return

Commit source, behavior tests and self-contained handoffs/0054-native-text-draw/result.md
with explicitly verified model/effort, coordinate/presentation contract, retained
resource accounting, commands/results, actual rendered acceptance, image paths,
known limitations and remaining Game UI work. Stop and reap every renderer,
window, build/helper process you started, then record native/GPU and target-cache
release. Keep any imperfect or unverified result explicit; never replace missing
checks with a completion claim.

## Frozen starting point and prior tooling

The reviewed CPU foundation is commit6144e1e8f7d5b6d6db212b3b7de4df9b5915407e.
Its independent source review is
`docs/spikes/evidence/native-text-input-independent-review.json`, SHA-256
`b71456904cde808afba1bce8231c483ed2cf28cf512a7abac9b422077ea44ba0`.
120 local CPU tests passed;74 GPU cases were intentionally ignored. Three real
fresh shipping CPU package runs after source/build deletion passed mixed text,
model motion and audio; this is foundation evidence, not rendered acceptance.

Prior read-only native helper examples are under
`/Users/andreyyurlov/.codex/worktrees/native-agent-play/incant/.worktrees/0053-nullable-inspector/handoffs/0053-nullable-inspector/tools/native/`.
You may read them as examples, but create/adapt helpers inside this packet with
fresh0054 scratch paths, your own tested binary identity and the lease recorded
in work.json. Never reopen a released0053 lease or change its files. All previous
0053 native/renderer/helper processes have been reaped.
