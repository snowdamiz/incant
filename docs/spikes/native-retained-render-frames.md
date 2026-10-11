# Retained native render frames

The typed renderer now prepares a native camera/model/light/environment scene once
and updates its world poses without rereading files, traversing the decoded glTF
hierarchy, cloning project data, parsing JSON, or uploading geometry/materials on
each frame. `Renderer::prepare_retained` creates a `RetainedScene`; its `submit`
method validates fixed bindings, fills reserved CPU staging, encodes the frame,
writes instance/light buffers, immediately submits, and only then accepts the
staged CPU metadata. Structural or configuration changes prepare a replacement.

## Ownership and failure behavior

The public retained object never exposes a mutable `RenderScene` or an unsubmitted
command buffer. Two private scene versions have separate instance/light buffers;
immutable decoded assets, GPU geometry, materials and environment resources remain
shared through their exact loaded-version identity. Both winding signs reserve
space at startup, so a reflection change cannot overflow a previously empty batch.
Zero-count slots do not draw or contribute invalid shadow bounds. Cached model-local
placements use the same root-times-local algebra as ordinary scene preparation.

Node order, IDs, mesh versions, shadow flags, camera/light configuration and the
environment binding are fixed for a retained scene. Invalid transforms, changed
bindings, invalid output settings, and a different renderer are rejected. These
ordinary checked failures leave the last accepted CPU scene and its GPU buffers
unchanged. A subsequent valid call retries normally. Staging can be partly changed
on rejection and is recomputed before reuse. Device loss, driver failures and
unwinds are terminal host conditions; already submitted GPU work is not rolled back.

## Verification

On Apple M5 Pro / Metal, the release suite passed 27 ordinary renderer tests and
57 native GPU tests (11 unit + 46 integration), followed by scoped all-target
release Clippy. The three new retained tests verify:

- Eight moving frames exactly match freshly prepared reference RGBA output while
  camera, directional shadow light, point light, scaling, rotation and winding
  change. Source project, asset store, source files and cooked cache are deleted
  before the rendering loop.
- Target-validation failure after CPU staging, singular transforms, changed IDs
  and mesh flags preserve the accepted scene; retry succeeds. Another renderer
  cannot submit the retained object.
- Sixty-four accepted CPU pose-staging calls keep their storage addresses and
  capacities, including reflection changes. Thread-local allocator instrumentation
  measures **zero allocations inside those staging calls**. GPU encoding,
  submission, initialization, ECS simulation and test assertions are excluded.

The final instrumented tests and Clippy also passed. The resolved normal dependency
guard still excludes authoring from `incant_render` and `incant_render_data`.
Machine-readable evidence includes source and command-log hashes.

## Remaining requirements

This is a correctness and retention slice, not an engine performance gate.
Encoding still creates bounded per-frame draw packets, camera/grid uniform buffers,
and bind groups. Startup performs two resolutions and allocates private double
frame storage, including both winding-sign capacities; worst-case memory must be
measured and optimized against the plan's device budgets. Sprites and diagnostic
cubes are explicitly unsupported by this native retained entry point; the existing
ordinary authoring renderer still supports them. Native sprite/tilemap bindings,
frame culling and the camera-relative pixel placement contract remain separate
work. No rendered appearance approval or real-phone measurement is claimed.
