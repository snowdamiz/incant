# Retained native renderer input (2026-10-10)

This implements the CPU binding/extraction prerequisite after scene v4 and the
canonical renderer/authoring split. `incant_render_native` has no normal Project,
command-bus, importer, cook, script VM or GPU dependency. It feeds the shared
`incant_render_data::SceneInput` from committed native world views. It does not
route native gameplay through legacy Play, render a frame, implement the pixel
placement marker, or satisfy a performance/device gate.

## Ownership and startup

`NativeRenderInput::load(&SpatialView, VisualCaches, camera: StableId)` binds every
binary model/texture declaration, including unused declarations, through the same
sealed checked `RuntimeAsset::load_visual` readers used by the canonical renderer.
The loaded version identity remains process-local; the scene fingerprint is an
existing source/recipe identity, **not authentication** or a claim that it is the
full payload digest. Reader checksums detect corruption. No source path is read.

The adapter retains an opaque world identity, binary-ID asset table, selected
camera, parent-first topology-index-to-render-slot mapping, typed input and an
owned numeric undo buffer. It formats informational IDs as 32 lowercase hex bytes
once at startup/rebuild. The callback receives the cached selected camera label;
callers need not clone or reformat it. Camera selection is explicit and fixed for
that adapter. Removing it rejects the next sync; a host can restore it or construct
another adapter with a different explicit selection. There is no preview fallback.
Transform-only and environment-only nodes do not synthesize diagnostic geometry.

All native scene declaration/count/range checks and old/new binary compatibility
remain in `incant_runtime`. The adapter enforces a 256 MiB aggregate decoded
vertex/index/mip payload cap across all loaded versions. Rust metadata, allocator
and GPU overhead are excluded and retain their separate reader/backend bounds.
Startup may hold **one reader-bounded staged asset beyond the aggregate cap**
before rejecting; an older retained adapter also contributes to process peak.
The immutable startup manifest cannot be extended by structural script commands.
A Normal texture cannot supply environment radiance; Color and Linear retain the
existing environment policy. Unsupported materials/configuration are refused by
the cook/runtime or the consumer, never discarded by this adapter.

## Committed synchronization

`try_sync(view, |input, camera_label, UpdateKind| -> Result<T, E>)` rejects a different
world identity before any writes or callback, even if scene IDs and generations
match. Moving a world retains its identity. Accepted relevant topology/configuration
generations select the update path:

- **Motion:** copy the old matrices into a preallocated undo vector, then copy
  committed continuous f64 globals into existing input slots. The callback gets
  the complete proposed frame. On `Err` or Rust unwinding, an RAII guard restores
  the previous numeric input. Tick/generations publish only on success. No labels,
  Arcs, vectors, source reads, authoring serialization/validation, or model-local
  hierarchy traversal are added to this path. ECS archetype relocation is irrelevant
  because the mapping stores topology indices, never table pointers.
- **Rebuilt:** prepare a bounded candidate from the retained sealed assets. This
  path may allocate new labels/vectors and clone Arcs. A rejected or unwinding
  callback leaves the prior input and generations intact; the same committed world
  can be retried. Accepted candidates replace the input/mapping/undo together.

The view lends only accepted world globals. A rejected native integration or
structural batch leaves its old committed values visible. A terminal solver world
cannot lend a view. The retained adapter can lag a committed world after consumer
rejection; it does not roll the simulation back or claim an old frame is current.
Spatial saves remain explicitly unsupported by the native world.

The callback is a **transaction boundary for the adapter only**. It must preflight
before mutating externally published GPU state, return a newly prepared resource
for publication alongside the accepted adapter, or make that external consumer
terminal if it cannot roll back. An error/panic after arbitrary external side
effects cannot be undone by this CPU guard. Process abort cannot run cleanup.
GPU representability, primitive expansion, materials and allocation limits remain
in the canonical consumer; finite f64 poses alone are not sufficient GPU validation.
Root's retained GPU frame/update work is separate from this commit.

## Verification

The scoped tests use actual authoring cooks to create checked model/texture data,
then remove all source files before native binding. They also remove all cache
files after binding and continue motion plus structural rebuilds from the sealed
versions. A separate Project cook → scene v4 binary → native world → input case
runs after the Project is dropped. Missing/stale/wrong-kind/corrupt and unused bad
bindings reject startup; a Normal environment rebind preserves the prior input.

Observable transaction tests cover continuous parent/child poses, real Velocity
archetype moves, topology reparent/despawn/spawn, configuration changes, explicit
camera loss/recovery, equal-generation cross-world rejection, repeated ticks,
consumer error/unwind and failed native ticks/structural batches. Input storage,
labels and loaded Arc identities remain stable on motion. An isolated thread-local
allocator counts **zero allocations in 64 accepted plus 64 rejected motion calls**;
world stepping, startup and other threads are excluded; the measured callbacks
only return success/rejection. This is a
bounded allocation property, not a whole-renderer or device performance result.

All 87 scoped runtime/assets/adapter release tests and doctests pass, including the
11 adapter tests. Release all-target Clippy with warnings denied, resolved normal
dependency checks, workspace formatting, generated conventions and workflow lint
pass. No dependency resolution changed; the lockfile adds only the new local crate.

Exact command results and hashes are recorded in
[evidence/native-render-input-2026-10-10.json](evidence/native-render-input-2026-10-10.json).
No visual appearance review, screenshots, live GPU integration, mobile execution,
shipping-profile timing or phase completion is claimed.
