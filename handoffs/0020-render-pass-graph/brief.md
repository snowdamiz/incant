# Render-pass scheduling: scoped rendered-pixel review

Claude Opus 5.5 through ACP owns visual review. Astra has replaced inline GPU
encoding with actual Bevy ECS-scheduled passes and owned prepared resources.
This changes execution/ownership, not the UI, BRDF, light equations, camera
matrices, tone mapping or chrome design. Do not redesign or retune appearance.

Read docs/spikes/render-pass-graph.md and the implementation as needed. The
existing native wgpu ownership remains: prepare immutable resources, run ordered
initialize/diagnostics + light assignment -> models -> display passes, return a
command buffer for caller-owned submission. CPU frame resources are removed
after encoding. Shadows/post-effects and full production graph remain open.

Artifacts will be copied here immediately after the runner creates this worktree:
- artifacts/graph-baseline: 124 PNGs from exact prior head 83c98a7.
- artifacts/graph-initial: the same 124 captures plus three new model-lifetime
  fixtures, from the scheduled implementation.
- artifacts/graph-pixel-comparison.json: byte/hash and pixel difference report.
- artifacts/graph-native: two CUA captures from the final custom-protocol native
  app: selected PointLight Inspector with output open, and after output collapse
  enlarges the viewport. Read-only AX evidence says Attached and zero errors.
- artifacts/graph-timing.json: three paired trials, same cooked sphere/lattice.

Acceptance:
1. Inspect representative actual captures for HDR/material/environment, authored
   camera and local-light coverage, using baseline counterparts. Confirm no
   visual regression from scheduling. 120/124 pairs are byte-identical in pixels.
2. Four spatial-overflow-96 pairs differ by one channel value at one pixel for
   513x385 and two pixels for 640x480. Both current clustered/oracle members are
   identical to one another. These tests create fresh random ULIDs, potentially
   changing floating-point light summation order between processes. Assess this
   explanation and the actual changed pixels; do not silently waive artifacts.
3. Review graph-retained-model-{0,1,2}: imported geometry offsets 0, +2, -2 at
   321x193, 480x270, 321x193. These are positive-coverage numeric fixtures, not art.
   A GPU test also submits their command buffers in reverse after dropping all
   CPU scene/store/project versions and removing source/cooked files; pixels
   must exactly match their immediate references. Then an empty scene must clear
   all prior content to #141519. Failed viewport preparation must not poison it.
4. Review the native captures for correct scene/chrome composition before and
   after the viewport grows. No new editor control or camera selector is claimed.
5. Report concrete defects or missing evidence, and a scoped visual verdict.
   No phase gate approval or game/device benchmark certification.

Local verification: 139 ordinary Rust, 33 GPU (6 renderer unit + 24 renderer
integration + 2 headless + 1 editor), 283 UI tests; full workspace Clippy;
conventions/bridge/SDK checks; five Python tool tests; final custom-protocol build
and packaging pass. Paired 1080p fenced medians are approximately unchanged:
0 lights 1.394 -> 1.397 ms; 4096 lights 6.611 -> 6.506 ms. This excludes gameplay,
presentation/loading/readback and is not GPU timestamp evidence.

Write result.md with findings and exact scope. Commit report with final trailer
Built-by: claude. Keep captures/scripts ignored (native account identity is
visible); no credentials/account interactions. No shader/UI/test edits in this
packet. Native automation/capture, if needed, must use CUA, never shell screen
capture/AppleScript. Use only this worktree's own target directory for builds.
