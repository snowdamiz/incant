# Accepted temporal placement provenance

## Scope

This CPU foundation implements the stable placement identity and accepted change
witness required by the reviewed 0073 quality-repair addendum 3 and 0074 native
coverage repair. It does not implement either image-history repair: there is no
owner attachment, pixel provenance, contributor tracking, changed-caster
footprint, new history target, or additional render pass. All existing WGSL,
cooker contracts, configuration defaults and memory ceilings remain unchanged.
No GPU execution or appearance acceptance is claimed by this slice.

The base is `2fe23dceb4cbedfae860011ad79eab7da95511fc`, the accepted cooked/native
composition plus the independent reconstruction oracle. Broad-phase producer
`88c10fe1068a805f4d77f6873db555f6cc36a732` remains on its separate branch.

## Exact identity and changes

Each eligible OPAQUE/MASK placement has an integer ID in 1..=100000. Zero denotes
excluded geometry. Static batches group only by the exact shared placement Arc
allocation; equal values in different allocations do not alias. Animated batches
group by the existing immutable Binding (instance, model node, optional deformed
geometry index). Both handedness batches therefore share IDs even when the
active packed row moves to the other batch. IDs are assigned in immutable batch
and placement order; allocator addresses are used only for temporary grouping,
never as published IDs. BLEND batches receive zero and cannot contribute changes.

The namespace is the retained MotionState owner token. A replacement, including
replacement by byte-identical assets, starts a fresh token. Explicit cuts and
existing incompatible projection/extent changes advance the accepted epoch but
preserve placement numbers within that owner. An invalidated frame is published
only after a successful submission. The first valid frame and every reset mark
all eligible placements changed, even if their transforms compare equal.

For compatible frames, static changes compare exact bits of the accepted and
candidate world-times-local matrices. Animated changes compare the entity world
matrix and already retained accepted/staged node globals or joint palette. The
comparison uses no clip/time approximation, hashing, resampling or full-scene
clone. IK and other accepted palette changes are included. Missing or mismatched
palette/model access is conservatively changed. Camera movement alone does not
mark a placement changed; ordinary camera reprojection remains separate.

The existing 80-byte Previous row remains 80 bytes: previous world at offset 0,
validity f32 at 64, integer `id | (changed << 31)` at 68, two zero words at 72.
The current motion and overlay-depth shaders read only validity.x from that
last 16-byte lane. This slice does not change its vertex format, shader source,
output or cooked artifacts. A later reviewed owner target must read the integer
word without interpreting it as a floating-point ID.

## Transaction and bounds

Change bitsets have separate accepted and candidate storage. Preparing a frame
clears and fills only candidate storage. Failed staging, validation, encoding or
abandoned candidates cannot publish it. The existing successful queue-submit
boundary swaps the bitsets together with accepted transforms, geometry and
MotionFrame. `RetainedScene::motion_changes()` borrows only accepted data, returns
None before first submission, and exposes the corresponding MotionFrame,
highest placement ID, exact per-ID membership, count and a conservative 31-bit
class union. Class collisions never establish identity; ID 1 and 32 may share a
class while retaining distinct exact membership. Clear/out-of-range IDs return
None. Pixel-camera bypass has no placement provenance and reports zero owners.

Grouping descriptors, per-batch bases, both bitsets and larger row headers are
charged before reserve under the existing 64 MiB motion instance-payload cap;
old plus candidate ownership still uses its existing overlap admission. At
100000 owners the two bitsets contain 25008 payload bytes. Temporary descriptor
and base storage is conservatively included in the retained admission estimate.
Count arithmetic, unique owner admission and row-buffer bounds reject before GPU
buffer creation. Per-frame grouping is absent; clearing/marking/swap and palette
comparison reuse retained storage. Logical payload excludes allocator rounding
and opaque driver memory, as before. No new 973 MB allocation proposal is adopted.

## Verification boundary

Focused CPU tests cover mirrored/shared versus distinct allocation identity,
excluded slots, maximum count and overflow refusal, allocation-free preflight,
exact matrix bits, class collisions, candidate privacy/retry, cuts and replacement.
The actual cooked skeletal fixture exercises retained palette equality, movement,
failed sample rollback and repeated allocation-free comparisons after source
removal. Existing packing tests check stable integer words through handedness
changes without allocation.

The two existing ignored retained-device tests add metadata assertions at their
existing submission points: accepted live/staging swaps, failed zero-size/singular
staging, mirrored packing, cut, unchanged frame, replacement and changed skin/IK.
They add no submissions or readbacks. They are compiled but require a separately
authorized Claude execution. Focused CPU results and source review will be bound
in the evidence packet before broader qualification.
