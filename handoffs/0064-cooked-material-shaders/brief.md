# 0064 — fixed material backend acceptance

Use Claude Opus 5.5 over ACP with explicitly verified Max thinking on every
new or resumed session. Read CLAUDE.md. You own rendered pixels and appearance;
Astra owns compiler/runtime correctness. No substitute model or effort.

## Frozen source and purpose

Source commit 6bcb839a7264a12fbcb3666041e17b3445468b09 adds the opt-in
incant_render/cooked-materials path. Read docs/spikes/cooked-material-shaders.md
and its evidence manifest. Independent source review, 92 enabled CPU cases,
63 default cases, Clippy, browser compilation and actual Metal compile/link pass.
All three material/lighting/shadow sampling WGSL files are unchanged. This path
has no native GPU or rendered-pixel acceptance yet.

Build baseline and candidate from this same frozen source. Both enable
incant_render/cooked-output,incant_render/cooked-geometry,
incant_render/cooked-sprites,incant_render/cooked-ui,incant_render/cooked-text.
Candidate additionally enables incant_render/cooked-materials. Freeze source,
helpers and exact executable feature identities before executing. Do not change
production files, shading math, tolerances or goldens to manufacture parity.

Run all existing native renderer GPU tests in both variants, including the full
model_gpu suite with material maps, alpha modes, culling, lights, shadow cascades,
environment sampling, HDR output, minification and retained resource changes.
Use small meaningful fixtures for actual baseline/candidate rendered-pixel
review, not solely test counts. Cover source-deleted cooked material execution,
reimport retention, many lights and changing cluster masks. Use incant_cmd for
all project mutations. Reuse fixtures and helpers from earlier completed packets
read-only where suitable. Compare unmasked whole frames; explain any bounded
controls separately and report actual appearance findings.

The explicit Metal fragment lengths uniform at group3 binding7 supplies the
actual allocated byte lengths of lights/counts/masks in that order plus padding.
The pinned passthrough backend drops automatic sized_bindings reflection. Check
runtime behavior across light count/cluster changes and empty/minimal scenes;
do not remove the bounds protection. Vertex pull slots15/14 and automatic vertex
length slot1 remain as before. The default WGSL path must keep working.

## Ownership and bounded execution

0063 currently owns the GPU and shared Cargo target. Do not build, use GPU,
launch a native window or capture until Astra creates
/Users/andreyyurlov/.codex/incant-evidence/0064/review-ready.json naming this packet
and releasing the target from0063. Only Astra may grant it. If the grant is
absent, preserve packet preparation and stop rather than poll or take a lease.
After the grant, exclusively use
/Users/andreyyurlov/.codex/incant-core-presentation-target with ./tools/cargo,
locked resolutions, ci profile and at most two jobs. Reuse compatible artifacts.
Never use another target or edit another packet's worktree.

Evidence root: /Users/andreyyurlov/.codex/incant-evidence/0064. Hold an exclusive
GPU lock and run serial bounded process groups: five minutes per GPU suite,
two minutes per capture, all children reaped. Retain at most24 actual frames,
each at most960x540. These are offscreen engine frames only; no desktop/browser
screen capture, native windows, audible playback, credentials or accounts.
Record every command/exit, source/binary identity, and GPU/target release receipt.

## Deliverables and limits

Return result.md, reproducible packet helpers, raw machine evidence and a compact
unedited capture selection. Record verified model/effort, OS/adapter/backend,
exact test totals and separate numerical/appearance verdicts. Commit only this
packet and its evidence with Built-by: claude. No pushes, PRs, merges or CI edits.
Report production defects precisely to Astra. Metal desktop execution does not
establish Vulkan/DX12 behavior, reference-phone performance, full offline renderer
completion, pipeline caching or any phase gate. Cluster compute remains WGSL.
