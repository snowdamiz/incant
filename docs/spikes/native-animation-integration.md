# Native animation integration

The local integration combines authored Animation components, checked cached
model metadata, native clocks and atomic TypeScript controls with retained
render deformation. Actual main113, including the optional cooked shader paths and checkpoint6
repairs, is included in the current local union. It is not
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

The source-free exported player is verified below. Claude0052 has completed the
native animation review, with a nullable Inspector defect requiring0053 follow-up. Handoff0052 covers actual visible playback, pause/step/restart, resize,
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


## Main110 integration and fresh export

Frozen `990ccef` preserves actual main110, the reviewed animation source `f74379e`
and original public export proof `43400d`. Only STATUS required manual conflict
resolution. The animation consumer changes and default-off cooked shader paths
merge additively. This union passes 519 ordinary tests in 92 groups, 57 additional
cooker/cooked-geometry CPU tests in seven groups, and affected all-target Clippy.
The scopes overlap. Each Rust test run leaves 71 GPU/optional tests ignored; no
new integrated GPU or native-window run is claimed. Tooling passes 214 Python
cases (four skipped) and 21 Node cases, plus formatting, generated contracts,
strict SDK types and normal runtime dependency checks.

The CPU export's 210 normal/build package records and 356 local package/helper
inputs match the prior proof. However, the conservative script compatibility ID
hashes the entire Cargo.lock. Main110's shader-only lock additions therefore
require a new trusted cook despite the unchanged CPU dependency closure. The
original proof stays historical; a fresh integrated headless build and unchanged
public animation probe now pass against this new identity.

The new fresh shipping player runs only after its authoring directory and private
build storage are removed. Three 160-tick replays produce identical reports and
checkpoint bytes; a 41+119 continuation matches the final save, committed logs,
CPU model input and concatenated PCM. Corrupt checkpoint and wrong-observation
controls fail without save or PCM publication. All seven packaged-player
processes are reaped. The player SHA256 is
`07bf8028643e34be20c5164ba10e31b888a22078785c7d89d2a3c744a04575f1`;
this is CPU functional shipping evidence, not GPU, performance or device evidence.

[Exact integrated source, logs, dependency audit and fresh-process receipts](evidence/native-animation-main110-integration.json)
bind these results. [Independent export probe review](evidence/native-animation-export-independent-review.json)
records the original probe's unchanged assertions and retained results. Native0052
evidence is preserved; the focused nullable Inspector recheck remains required
before publication.

## Main113 and the nullable Inspector correction

The clean integration preserves original Claude0052 commit `9b70ce7`, the export
probe and reviewed animation ancestry. Only STATUS required manual resolution
when main113 was merged. The independently reviewed three-file bridge fix
`30eacbc` was applied as `38d1446`; its separately queued Wasm-animation parent
was excluded. It normalizes schemars nullable type arrays without losing bounds.

The current union passes 36 player/observation tests, seven budget tests, affected
all-target Clippy, 52 bridge tests, strict UI test types, the production UI build,
generated bridge, formatting and conventions. The renderer/runtime/cook/script
sources and root lockfile are unchanged from the main110 proof. The reviewed
player startup-routing correction is checked by the unchanged process regression;
the earlier fresh shipping proof remains bound to its recorded source and binary.

Claude0052 reports native skeletal playback, transport, resize and unchanged
authoring passes on the exact granted build. Ten committed image hashes were
independently verified without a new pixel review. Its unsupported nullable Clip
row prompted the bridge correction; focused Claude0053 acceptance is pending.
Script Play logs still do not surface in the editor Console and remain a separate
transport task. No integrator GPU/window/provider work or broader gate claim is
made by this source/CPU integration.

[Current source, tests and preserved native evidence](evidence/native-animation-main113-integration.json).
