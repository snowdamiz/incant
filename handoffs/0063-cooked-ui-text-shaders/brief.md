# 0063 — fixed UI and world-text backend acceptance

Use Claude Opus 5.5 over ACP with explicitly verified Max thinking on every
new or resumed session. Read CLAUDE.md. You own rendered-pixel and appearance
review; Astra owns compiler/runtime correctness. No substitute model or effort.

## Frozen scope

Review the fixed UI and world-space text shader paths at source commit bacdd6f2,
including preceding UI commits f8b95de5 and 972a36f1. Read
`docs/spikes/cooked-ui-shaders.md`, `docs/spikes/cooked-text-shaders.md` and their
evidence manifests. Independent source reviews, affected CPU tests, Clippy,
default/web compilation and actual Apple Metal compilation/linking pass. The
UI/text WGSL, geometry, themes, depth and blending are unchanged. Neither new
path has native GPU or rendered-pixel acceptance yet.

Build baseline and candidate from the same frozen source. Baseline enables
`incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites`.
Candidate additionally enables `incant_render/cooked-ui,incant_render/cooked-text`.
This isolates these two shader families. If a comparison fails, use one added
feature at a time to identify the family; do not change shader math or goldens.

Run the relevant native numerical GPU suites, including UI/text coverage,
clipping, depth/occlusion, atlas isolation, multiple dynamic node offsets and
retained-scene updates. Reuse existing public authoring and frozen fixtures from
earlier text/UI packets where appropriate. All project edits use incant_cmd.
Fixtures should exercise Arabic and CJK shaping, reflected/rotated/scaled world
text, transparent coverage edges and minification, packed UI colour, rounded
corners, clipping and multiple panels. Include source-deleted cooked input
execution. Inspect actual baseline/candidate pixels and explain any differences.
Exact image equality is evidence of parity, not a substitute for visual review.
Do not mask a region to claim equivalence: record full-frame and bounded-control
results separately, including any known tolerances and their physical reason.

## Ownership and bounded execution

0062 currently owns the GPU and Cargo target. Do not build, use GPU/window,
capture or launch a renderer until Astra creates
`/Users/andreyyurlov/.codex/incant-evidence/0063/review-ready.json` naming this
packet and releasing the shared target from 0062. Only Astra may grant it.
After that grant, use the exclusive Cargo target
`/Users/andreyyurlov/.codex/incant-core-presentation-target`, `./tools/cargo`, locked
dependencies, ci profile and at most two jobs. Reuse compatible build artifacts;
do not touch another target or another packet's files.

Evidence root: `/Users/andreyyurlov/.codex/incant-evidence/0063`. Freeze exact
source/helper/binary hashes before execution. Use an exclusive GPU lock and
serial bounded child groups: five minutes for each test suite, two minutes for
each capture, all children reaped. Maximum 24 retained frames, each no larger
than 960x540. No desktop/browser screen capture, native windows, device audio,
credentials or accounts are needed. These are offscreen engine frames only.
Write a complete command/exit ledger and release receipt when done; never extend
or reacquire a lease without Astra. Stop rather than poll if the grant is absent.

## Deliverables

Return `result.md`, reproducible helpers, machine evidence and a compact unedited
capture selection under this packet. Record model/effort, OS, adapter/backend,
source and executable identities, feature sets, exact test totals and separate
numerical/visual verdicts. Report production defects precisely for Astra; edit
only this packet and its evidence. Commit with `Built-by: claude`; no pushes,
PRs, merges or CI edits. Metal desktop results do not establish Vulkan/DX12,
reference-phone performance, a fully cooked renderer or phase gates.
