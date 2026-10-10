# Orthographic camera appearance and Inspector review

Claude Opus 5.5 through ACP owns all scene design, visual geometry, cameras,
rendered judgment, Inspector layouts/styling and screenshot review. Read CLAUDE.md,
docs/spikes/orthographic-cameras.md, tools/probes/orthographic-cameras.py and the
existing camera Inspector. The director's current preference is neutral connected
panels, clean spacing and a restrained accent. Assets live in the left workspace;
the bottom dock is Problems/Console/History. No new floating-card redesign.

## Scope and acceptance

Review actual orthographic projection in an original compact real-engine scene.
Use the immutable artifacts/tools/incant_headless with artifacts/tools/binary.json;
verify SHA before and after captures. Show at least two objects at clearly
separated depths with equal authored size, straight parallel lines, an oblique
view, local illumination and a directional shadow. Compare camera moving along
its forward direction at fixed vertical_size (size and parallel-view shading
must stay consistent), changing vertical_size, and switching projection. Use
real rendered frames, not painted replacements or UI mockups. Review framing,
unwanted foreshortening, clipping, lighting/shadow consistency and resizing at
16:9 and 4:3. Keep numeric correctness claims tied to measured observations.

Extend the existing read-only Camera Inspector to present projection type and
orthographic vertical extent clearly. Legacy cameras must still show perspective.
Preserve the existing mutation boundary: do not invent GUI-only writes, unsupported
controls or an agent-ready claim. Choose good labels, units, spacing, help and
keyboard behavior yourself. Display unknown/malformed values explicitly instead
of silently coercing them. Verify normal and minimum widths and primitive/perspective
regressions, plus strict UI tests/build. Return concrete evidence of any bridge or
engine correction needed from Astra. Native captures will be supplied by Astra
through CUA after integration/rebuild; do not claim native acceptance without
seeing them. The Mac is currently unlocked and computer use is authorized.

## Contract and limits

`Camera` retains required `fov_degrees`, `near` and `far`. Optional `projection`
defaults to perspective for legacy JSON. Explicit forms:
`{kind:"perspective"}` or `{kind:"orthographic",vertical_size:8}`. Vertical size
is the visible world-space height; width follows output viewport aspect. Field
of view is retained but does not affect orthographic projection. Near must remain
positive and far greater; this supports shared logarithmic light-depth clusters.
All authored component changes use normal incant_cmd RPC commands with provenance.
The renderer uses parallel view rays for orthographic PBR. Do not claim pixel
snapping, sprite/tilemap rendering or 2D physics; those are future implementation
work outside this increment. No phase gate is approved.

## Paths and execution

Allowed changes: editor/ui/src for Camera Inspector visual presentation and its
behavior tests; this packet's helpers/result/evidence. Do not change Rust, SDK,
mutation routing, CI, account settings or unrelated handoffs. Use supplied binary,
not another worktree's build target. No Cargo is needed in your worktree.

Useful paths: crates/incant_doc/src/camera.rs; crates/incant_render/tests/cameras;
editor/ui/src/inspector; handoffs/0019-authored-cameras and 0021-directional-shadows.
Install pinned node dependencies with npm ci if needed. Strict-check your TypeScript,
compile behavior with tools/build_script.mjs, and use public init/import/rpc/play
commands. Original mathematical glTF may be authored and imported. Never edit
project JSON directly or use shell access from inside an engine agent.

Capture 960x540 and/or 800x600 engine images, at most 128 frames and 256 MiB raw
per capture. Reuse per-run immutable projects; helper output directories must
refuse overwrite. Use small bounded scenes, not a benchmark load. Repeat the
meaningful rendered comparison and record raw hash equality or justified exact
limits. Save useful raw outputs in ignored artifacts/0030-orthographic-cameras.
Commit a concise set of unedited representative engine images, helpers and
compact evidence. Keep account-bearing native full images ignored and commit
only account-free crops when supplied. Do not publish, sign or merge.

Return result.md with exact model/transport, source/binary provenance, commands,
tests, a clear scoped appearance verdict and any remaining native/engine defects.
Every commit ends in Built-by: claude. An initial checkpoint can accept engine and
browser scopes while explicitly waiting for Astra's rebuilt native captures.
