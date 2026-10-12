# 0062 — fixed cooked sprite backend acceptance

Use Claude Opus 5.5 over ACP with explicitly verified Max thinking on every
new/resumed session. Read CLAUDE.md. You own all rendered-pixel, screenshot and
appearance review. Astra owns shader compiler/runtime correctness. No substitute
model, effort or visual owner. Do not edit personal memories.

## Scope

Review the fixed sprite backend path in source commit
23f92d716bde0afc137b160c5497a502848d63b8. Read
`docs/spikes/cooked-sprite-shaders.md` and its evidence manifest. Independent
Astra source review, 82 enabled CPU tests, 63 default tests, Clippy, wasm32 and
actual offline Metal compilation/linking already pass. All 82 GPU tests remain
unexecuted for this change. The existing sprite WGSL is byte-identical.

Compare the same source with `incant_render/cooked-output,incant_render/cooked-geometry`
(baseline sprite WGSL) against those features plus `incant_render/cooked-sprites`
(candidate). This isolates the sprite path from the already accepted output and
shadow/diagnostic cook paths. The candidate checks stage IO/resources, 12/52-byte
Metal vertex buffers, unsigned/flat sampling data and exact backend bindings.
It must preserve appearance, nearest tie behavior, filtering, premultiplied alpha,
depth/order, atlas/tilemap regions, reflection/rotation and flipbooks. Never change
shader math or goldens merely to make the comparison pass.

## Work and evidence

Prepare guarded binaries and reproducible fixtures. Existing test suites under
`crates/incant_render/tests/sprites/`, cameras, pipeline_warmup and retained sprite,
tilemap and flipbook tests provide meaningful numerical controls. Existing visual
packets0032/0041 and later pixel-grid packets are reference helpers, not substitutes
for executing the current candidate. Reuse public command-bus authoring; no direct
project JSON mutation path. Keep fixture art and helpers inside this packet or its
ignored evidence directory. Do not modify production code, other handoffs or
another agent's evidence. Report exact reproducible defects for Astra to fix.

Once GPU ownership is explicitly released below, run relevant native numerical
GPU tests and rendered baseline/candidate comparisons. Include nearest and linear
sampling, atlas edges, transparency, reflected/rotated/wide sprites, tilemaps and
animated first/final frames. Verify a scene still renders after deleting its
private authoring/asset source while preserving the accepted cooked inputs. Inspect
actual pixels personally; passing tests alone is not visual acceptance. Compare
hashes/pixels and explain any difference. Record OS, adapter/backend, source and
binary hashes, features, exact commands/exits, image identities and scoped verdict.
A Mac Metal result never claims Vulkan/DX12, phones, performance or a phase gate.

## Ownership and execution

CPU builds may start now. Exclusive Cargo target:
`/Users/andreyyurlov/.codex/incant-core-presentation-target` (0061 released it;
Astra assigns it to 0062). Use `./tools/cargo`, locked dependencies, ci profile and
at most two jobs. Keep all binaries/evidence in
`/Users/andreyyurlov/.codex/incant-evidence/0062` and record a target lease.
Do not touch root's physics-variant target or any other agent's target.

0060 currently owns the shared native/GPU review slot. **Do not start any GPU,
window, screenshot or renderer process until**
`/Users/andreyyurlov/.codex/incant-evidence/0062/gpu-review-ready` exists and explicitly
names 0062. Only Astra may write this release marker. Prepare CPU work while waiting;
when ready, return a concise ready/pending report instead of polling indefinitely.
Astra will resume the same packet when the slot is released.

No desktop/browser capture, native UI, device audio or account action is needed.
Use offscreen engine frames only. At most32 retained captures, each at most 960x540;
all GPU commands serial, bounded by 120seconds with all child processes reaped.
Numerical GPU test processes have a five-minute ceiling and run one test thread.
Record a ledger and release the GPU slot promptly after the last process exits.
Never extend a lease or reuse it after release without coordination from Astra.

Deliver `result.md`, a compact unedited capture selection, machine evidence and
reproducible helpers; keep large raw logs/private fixtures/binaries ignored.
Commit only the packet with `Built-by: claude`. No pushes, PRs, merges, CI changes,
production code edits or claims of completed rendering/phase gates.
