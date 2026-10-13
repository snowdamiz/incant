# Fixed temporal registration experiment

This implements only C-reg from Claude Opus 5.5 Max's
`handoffs/0072-motion-history-numeric/fixed-sharpness-design.md` at
`e2b499e75c26b31874cd63a2a689201d6533b8e8`. The accepted fixed-mode producer
`e23776df275fa1de81b2027110d9c7b6916999ac` is the comparison baseline. This is
an opt-in experiment; its new appearance has not been measured or accepted.

History color now occupies the unjittered pixel grid. Reprojection subtracts
the difference between the previous accepted and current raster jitter, so a
static surface does not resample its accumulated color on every frame. The
presentation pass copies that registered color directly. Reset pixels and
pixels that cannot use history reconstruct only current radiance at the
original presentation location.

The existing depth range, motion selection, coverage admission, clamp and
reactive blend stay intact. Motion and stored depth retain their raster grids;
their relationship to the newly registered color needs explicit moving-edge
and diagonal/thin-feature evaluation. This change does not establish a new
depth or object-identity admission rule.

Two signed eighth-pixel jitter deltas fit exactly in the existing uniform's
spare scalar. The encoded zero delta is 136. State comes from the accepted
frame, so preparation alone cannot advance history. There are no new targets,
bindings, passes, or persistent owner fields. Current-only reconstruction uses
four loads only on refusal/reset paths instead of retaining a nine-value local
array; this is a source-level choice, not a measured performance improvement.

Local CPU qualification in `~/.codex/incant-evidence/fixed-sharpness-001`:

- Focused temporal checks: 15 passed, two device tests ignored.
- Native shader-cooker contract tests: three passed.
- Renderer default library: 130 passed, 41 device tests ignored.
- Renderer cooked-temporal library: 144 passed, 41 device tests ignored.
- Strict Clippy on both affected crates and all targets with cooked temporal:
  passed.

The device test retains its 14 previous controls and adds all 64 pairs of
previous/current jitter phases plus eight registered reset frames. The original
150-frame coverage suite remains present. These tests compiled but have not
been executed for this experiment. CPU results do not count as GPU or rendered
appearance evidence. Cooked shader hashes were updated; no native recook or
shipping/device performance claim is made here.

Next: independent source review, then bounded numerical execution and Claude's
rendered comparison against the accepted baseline, including resets, current-only
overlays, moving edges, MASK detail and temporal stability. Catmull-Rom history,
upscaling, MetalFX coverage changes and default enablement are outside this scope.
