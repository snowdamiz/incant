# Authored camera capture: rendered-pixel review

Claude Opus 5.5 through ACP owns visual review. Astra implemented the renderer,
CLI and agent-tool correctness. The existing typed Camera component now drives
actual captures when its stable entity ID is selected. It uses vertical FOV,
near/far clips and inherited world position/orientation (local -Z forward, +Y
roll, normalized basis). Scale changes inherited position/aim but not clip units.
Default editor preview remains selected when no camera ID is provided. Camera-only
entities no longer appear as diagnostic cubes. This is perspective capture, not
an editor camera-selection UI, orthographic support or glTF camera import.

Review actual PNGs in artifacts/camera-initial. They will be copied to this
worktree immediately after the runner creates it. Do not silently review the old
lighting packet. This increment changes matrices and selection, not BRDF, tone
mapping, authored lighting equations or UI styling.

Acceptance:
- fov-30 versus fov-80 must show the same red emissive quad with a narrower versus
  wider field. near-clipped, far-clipped and camera-facing-away deliberately show
  no geometry; they test clipping/direction, not missing content bugs.
- camera-clusters pairs use three distinct camera poses, FOVs and near/far ranges,
  at odd/even sizes. Every -oracle image is identical to the culling-free All
  path. Check visible seams, cutoffs, unexpected distortion or illumination loss.
- camera-flat-parent-reference and camera-inherited-pose are identical. One uses
  the same position expressed through a translated parent.
- camera-transparent-front/back show two colored translucent planes. The near
  plane changes with camera position, so front should be red-dominant and back
  blue-dominant. The quads are numerical fixtures, not a visual design proposal.
- cli-camera-preview/authored/moved use a 3968-triangle imported sphere and
  authored point. Changing camera should change viewpoint/framing consistently.
  Undo exactly matches authored; Redo/reopened/source-free exactly match moved.
  Do not interpret the titleless image as an editor redesign.

138 Rust behavior tests pass. Three new renderer GPU checks and a public
headless playback test pass. The latter follows a camera moved by simulation,
compares initial screenshot pixels, rejects missing camera IDs, and leaves the
authored document/journal unchanged. Public CLI camera edits, invalid clip-range
atomic rejection, durable history and source-free loading pass. Full combined
GPU/Clippy/build verification is in progress. Unit tests cover normalized world
basis and GPU-unrepresentable projection errors. Viewport offset and queued
resize tests now run both preview and authored camera selections.

Review pixels, state any concrete defects/evidence gaps, and give a scoped
verdict. Do not edit shaders, matrices, tests, GUI styling or project content in
this packet. If more numerical fixtures are needed, identify them for Astra.
No phase gate or production engine approval. Native UI has no new controls and
is outside this packet's scope. No account interaction or credentials needed.
Native capture, if needed, must use CUA. Keep any review scripts/images ignored;
write result.md and commit the report with Built-by: claude in the final trailer.
Use only this worktree's own target directory for optional builds.
