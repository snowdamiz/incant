# 0066 — fixed cluster compute backend acceptance

Use Claude Opus 5.5 over ACP with explicitly verified Max thinking on every new
or resumed session. Read CLAUDE.md. Claude owns rendered pixels and appearance;
Astra owns compiler/runtime correctness. No substitute model or effort.
This is packet preparation only: do not launch until the root grant below exists.

## Frozen source and comparison

The root readiness grant will name the exact source commit containing this
packet and the reviewed `cooked-clusters` implementation. Read
`docs/spikes/cooked-cluster-shaders.md`, its evidence manifest and independent
review receipt. The original grid WGSL is byte-identical. CPU checks, feature
isolation, Clippy, browser compilation, actual Metal compile/link and independent
Vulkan SPIR-V validation passed. No GPU/appearance acceptance exists yet.

Build both variants from that same frozen source. Both enable
`incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,incant_render/cooked-ui,incant_render/cooked-text,incant_render/cooked-materials`.
Only candidate additionally enables `incant_render/cooked-clusters`.
Baseline therefore retains WGSL compute while every other cooked shader family
is identical. Use locked resolutions, the ci profile and at most two jobs.
Capture actual Cargo compiler-artifact features, source and copied binary hashes
before execution. Do not change shader math, tolerances, goldens or production
files to manufacture parity. Naga's SPIR-V debug metadata is profile-dependent;
compare the actual ci artifacts, not assumed dev-profile bytes.

Run all current 89 native GPU cases in each variant: incant_render's library
and model_gpu suites plus incant_render_gpu_host game/UI suites. List and retain
the exact cases before running; explain any source-union count difference rather
than reporting an assumed total. Run serially with all ignored GPU tests actually
enabled. Separate completed, failed, skipped and refused cases.

In addition to the full suite, exercise actual cluster count/mask readbacks at
local-light counts 1, 31, 32, 33 and 4096, plus a minimal/directional-only control.
Compare baseline and candidate masks/counts exactly. Cover retained moving-light
updates without a grid resize, grid resizing, different viewports, perspective
and orthographic cameras. Reuse existing crate-internal numerical cluster tests
and completed packet helpers read-only. If new raw-buffer access requires an
engine test observer, report the precise cfg(test) seam to Astra first; do not
introduce a production bypass. All authored fixture edits use incant_cmd.

Inspect a few complete baseline/candidate frame pairs, including a populated
cluster boundary, retained movement and differing camera projections. Prefer
3–6 pairs, compare unmasked whole frames and report actual appearance findings.
Record numerical membership independently of visual similarity. Include a
source-deleted cooked fixture where existing packet infrastructure supports it;
state the exact deletion boundary. No frame-time/device-performance claim.

## Critical contracts to preserve

Metal compute binding0–3 is grid/lights/counts/masks. Explicit uniform binding4
supplies the three actual allocation byte lengths plus padding; fragment
binding4 remains its shadow texture. The kernel uses static 4+512-byte workgroup
locals and exactly 64×1×1 invocations, with all original zero-init and barriers.
No automatic passthrough memory reflection is assumed. Runtime MSL is private
embedded code only. Vulkan removes precisely one generated workgroup-only
ArrayStride decoration; the rest of its words are preserved. DX12 keeps grid b0,
lights t0, counts u0/masks u1, lengths b1 and special b2. Report any backend defect to
Astra; do not disable bounds or weaken these contracts.

## Ownership and bounded execution

0064's material review and then 0065's Core review precede this packet. Do not
build, use GPU, launch a native window or capture until root Astra creates
`/Users/andreyyurlov/.codex/incant-evidence/0066/review-ready.json` naming this
packet, exact source commit and exclusive target release. Only root grants it.
If absent, preserve preparation and stop; do not poll or acquire a lease.

After the grant, exclusively use
`/Users/andreyyurlov/.codex/incant-core-presentation-target` with `./tools/cargo`.
Do not use Astra's physics target or another packet's checkout. Evidence root is
`/Users/andreyyurlov/.codex/incant-evidence/0066`. Hold the exclusive GPU lock.
Run one bounded process group at a time: five minutes per GPU suite, two minutes
per capture, all children terminated/reaped. At most 24 actual frames, each at
most 960×540. Offscreen engine frames only: no desktop/browser capture, native
windows, audible playback, credentials or accounts. Recheck the grant and
source/binary hashes for each child. Explicitly release GPU/target and verify
no owned child remains when finished.

## Deliverables and limitations

Return result.md, reproducible packet-only helpers, raw machine evidence and a
compact unedited capture selection. Record verified model/effort, OS, adapter,
backend, exact tests and separate numerical/appearance verdicts. Commit packet
files and evidence only with Built-by: claude. No pushes, PRs, merges or CI edits.
This desktop Metal packet cannot establish Vulkan/DX12 execution, reference-phone
performance, persistent pipeline caching, async prewarming or any phase gate.
