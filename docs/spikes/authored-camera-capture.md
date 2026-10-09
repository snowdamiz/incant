# Authored perspective camera capture

Phase 1 capture increment, 2026-10-09. The existing Camera component previously
stored projection fields that captures ignored. Selecting its stable entity ID
now applies its inherited pose, vertical field of view and near/far clip planes.

- `RenderScene::with_camera(id)` selects a camera on an immutable prepared scene.
- `incant_headless screenshot PROJECT OUTPUT --camera ID` selects it for one PNG.
- `incant_headless play PROJECT --camera ID --output DIR` follows it at each
  captured simulation tick and records the ID in the report.
- The agent `view_screenshot` tool accepts optional `camera` alongside width and
  height. The project-scoped host forwards that selection without filesystem or
  mutation capabilities.

Omitting the selector preserves the fixed editor preview. Unknown IDs and IDs
without Camera fail explicitly. Playback requires an output directory when a
camera is specified; failed captures never publish a completed report.
Camera-only entities and mesh-less ancestors used as camera-rig frames no longer
create diagnostic cube geometry. Explicit MeshRenderer bindings on those
ancestors still render; unrelated diagnostic entities are preserved.

## Shared camera frame

Geometry projection, material eye position, transparent depth sorting and the
clustered light grid all consume the selected frame. Cluster FOV and logarithmic
slices use the authored clip range. The grid still uses 64-pixel tiles/24 slices,
with viewport-origin subtraction. Buffer reuse is safe across camera changes:
each encoded draw supplies immutable camera/grid uniforms and rebuilds membership.
Prepared scenes retain their camera poses across later project edits. A skewed
basis from a rotated child under uneven parent scale matches the independently
flattened orthonormal pose in captured pixels.

Local −Z points forward and +Y supplies roll, matching the axis convention in
[Khronos's camera specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#view-matrix).
Incant normalizes the inherited forward/up vectors into a right-handed view;
parent transforms affect position and aim, while scale does not multiply clip
distances. Near/far and matrices must remain finite and distinguishable at GPU
precision. Singular/near-collinear view bases and unrepresentable projections
return typed errors before drawing. Selected views also validate local-light
positions against the actual view transform. This is not glTF camera import.

The component's existing JSON shape and shared command bus are unchanged.
Selecting a camera is read-only renderer configuration, not a new mutation path.

## Verification

139 Rust tests and 32 explicit real GPU cases pass on Apple M5 Pro. The focused
checks cover FOV, near/far clipping, camera-facing direction, inherited/flattened
pose equality, retained views, reversed translucent-plane ordering, and exact
all-light oracle matches at three camera poses and two sizes. Offset viewport
and queued resize checks run both authored and preview selections. GPU-precision
failures and tool-to-host selection routing have separate behavior tests.

A headless test moves the camera in simulation while keeping geometry still,
verifies changed frames, matches the initial authored-camera screenshot, and
preserves the authored document and journal byte for byte. Invalid selection
fails without a finished report. Public CLI commands author and move a camera
through the command bus; Undo/Redo restore exact captures, an invalid clip range
leaves document/revision/history unchanged, and journal reopen and source-free
cooked rendering retain the image.

283 UI tests, the UI build, workspace Clippy, generated checks, SDK typechecking,
five Python tests and the custom-protocol native release build pass. Claude approved the initial and expanded rendered evidence (`c5e02e6`,
`ddc8645`). Final review `a7fc35f` approves all 36 recorded images, including an
independent projected-corner check of the skewed camera basis. No pixel-evidence
gap remains for this scoped increment. Native controls/layout are unchanged and no
new native UI review is claimed.

## Limits

This increment supplies explicit perspective capture selection. It does not add
orthographic/infinite projection, lens shift, an active-game-camera setting,
editor camera controls, glTF camera import, camera rigs, or multi-camera capture
in one playback invocation. Missing those features is not a Phase 1 completion
claim. Shadows, the render graph and remaining core-engine/game gates stay open.

## Saved-account agent check

A live `gpt-6-astra` run used the existing saved OAuth account without a login or
Keychain prompt. It queried the Camera component, called `view_screenshot` with
the selected ID, and finished in three steps with no transactions. The host-
confirmed camera ID is retained in the tool observation and the model's tool
result without storing image pixels in the report. Usage was 3,897 input and
145 output tokens. Project and journal hashes stayed unchanged. This verifies
live tool/account integration, not model-authored game completion or pixel review.
An initial 16,000-token configured budget was rejected by the conservative local
reservation; the ordinary 64,000-token budget completed the probe.

## Hosted schema follow-up

Windows/Linux desktop and all three credential-storage jobs passed at `83c98a7`.
The source workflow reached generated-schema verification and found the checked-in
`agent-tools.json` snapshot missing the optional camera field/updated description.
The runtime tool schema was already correct, as used by the live agent check.
Follow-up `d9ace60` regenerates the snapshot; a second local schema generation
leaves the tree unchanged and SDK generation checks pass. All six exact-head hosted
checks passed at `d9ace60`; PR #19 merged into main as `695f3b3` on 2026-10-09.
The actual merge commit ends in `Built-by: astra`. This is an artifact correction,
not a new runtime or appearance change.
