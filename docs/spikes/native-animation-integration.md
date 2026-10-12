# Native animation integration

The local integration combines authored Animation components, checked cached
model metadata, native clocks and atomic TypeScript controls with retained
render deformation. Actual main109 editor/agent changes are included. It is not
published yet and does not complete animation or the Phase 1 exit gate.

The first runtime/script/cook/Play/renderer union passes 397 CPU tests in 60 result
groups, with 70 GPU cases unrun in that command, plus scoped all-target Clippy.
Those renderer cases separately pass on frozen `cdd0903`, using numerical Metal
readback on Apple M5 Pro; their recorded evidence is preserved at its tested
source. The two integration differences from those renderer inputs are reviewed
additive font-asset variants and main109 internal dependency edges. No renderer
implementation file changed. This is not an integrated GPU rerun or pixel review.

Independent review found two existing exact-v3 guards that rejected the new v4
animation saves. `4309727` corrects world restoration and script-session loading,
with real continuation and refusal regressions. The subsequent main109/editor
union at `edfc443` passes 357 CPU tests in 41 groups, with one existing GPU test
unrun, and all-target Clippy. The scopes overlap; these counts are not additive.
The current production editor UI and native editor/headless build pass, as do
strict SDK types, bridge generation and repository conventions.

The native acceptance executables are copied and hash-pinned under
`/tmp/incant-native-animation-0052-bin-v1`. The first build's old local UI output
was detected before any launch; the UI was rebuilt from current source and the
editor rebuilt. Runtime HTML/JS/CSS/assets match the previously reviewed main109
UI. Only source-map dependency paths differ because node_modules lives in another
checkout; its mappings and embedded source content match. Source code and build
output identities are in the immutable manifest, not inferred from a branch name.

The source-free exported player and Claude's native animation review remain
pending. Handoff0052 covers actual visible playback, pause/step/restart, resize,
and unchanged authoring. Its v2 fixture and binary identities are now complete;
the earlier shader GPU interval is released and the exclusive native interval is
granted to0052. The initial retained fixture used a journal filename different
from the editor default. A separate v2 was reauthored through public routes and
verified with the rebuilt headless executable and the editor-default journal,
including 180 CPU native ticks. The superseded v1 remains historical. This is a
test-fixture correction, not a claimed product fix or native visual success.

Blending, state machines, IK, root motion, Wasm animation controls, GPU skinning,
and reference-device performance remain separate work. CPU allocation checks and
desktop numerical frames do not satisfy the phone or Core Sample gates.

[Integration receipt](evidence/native-animation-integration.json),
[verified identities](evidence/animation-integration-hash-review.json),
[core review](evidence/native-animation-core-review.json),
[integrated seam review](evidence/native-animation-integrated-seam-review.md),
[checkpoint correction](native-animation-checkpoint-compatibility.md),
[renderer numerical evidence](native-animation-rendering.md).
