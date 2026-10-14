# MetalFX real-material image/motion fixture

Source-only preparation; no capture, visual acceptance or performance result yet.
The native integration and its separate numerical/lifetime controls remain in
[metalfx-main285.md](metalfx-main285.md). This fixture adds no production shader,
pipeline, policy, feature, manifest or native interop change.

The test-only `metalfx_image` module selects the opaque portion of the existing
0073 Stage from immutable renderer experiment `6cf4660a71a30767e2f4e09749e44ab1678d2a53`:
floor, cube, sphere, receding posts, antenna wires/diagonal, trilinear chart and
moving opaque cube. Geometry/winding, material factors, chart pixels, mip cooking,
environment and sun follow that fixture. The BLEND card, sprites, world text and
bloom are explicitly excluded from this first native domain. No private renderer
or reconstruction algorithm is imported. The ordinary material fixture cooks
actual assets, and the output retains their complete bounded source/cache files,
project, fingerprints, process-local version identities and semantic node roles.

One actual Metal renderer prepares three independent retained owners against the
same cooked `SceneInput`, IDs and world matrices: FULL at 960×540 without temporal
history, required MetalFX at 480×270, and required MetalFX at 720×405. Every native
scaler is explicitly prepared at 960×540 before the chronology. Any fallback,
wrong extent/phase/history epoch or changed input binding refuses the run. The
FULL reference is an unjittered single-sample render, not an alias-free ground
truth or supersampled reference. Existing native eight-phase jitter is unchanged.

Each arm runs 32 frames in fixed FULL / MFX50 / MFX75 order. Frames 1–16 use the
same camera and mover pose. Frames 17–24 advance camera X by 0.05 m and mover X
by 0.12 m per frame. Frame 25 advances camera X by 1.05 m and explicitly resets
both native histories; frames 26–32 resume the 0.05 m path. Mover motion continues
at 0.12 m across the cut. Camera pitch, field of view, near/far, lighting and all
other transforms are fixed. Native phases are 0–7 repeated; frame 25 starts phase
0 and epoch 2 without a previous frame. Shared input records are compared exactly
across all arms before submission, including semantic IDs, camera/light values,
model versions, environment, shadow flags and matrices.

Final display RGBA8 sRGB and PNG pairs are captured at frames 1, 16, 24, 25 and 32
in each arm: 15 raw files and 15 PNGs. These are final composited output; alpha is
not a vendor coverage readback. Image review will inspect detail, silhouettes,
disocclusion, lag and cut response. There is no image-difference acceptance
threshold, 60 fps claim, or claim that MetalFX completes built-in upscaling.

One explicitly ignored GPU child is proposed with a 120-second limit, no retries,
and a create-only output directory capped at 128 MiB. Source-level explicit
counts are 96 scene submissions plus 15 readback submissions = 111 queue submits,
15 maps, 64 native encodes and 239 submitted command buffers. Startup uploads and
vendor-internal work are not inferred from these counters. Raw display payload is
31,104,000 bytes; generated source/cache retention is separately capped at 16 MiB
and 128 files. A runner must maintain the 6 GiB free-space floor and capture
terminal/descendant cleanup. No window or user pointer interaction is involved.

Pending qualification: the two CPU fixtures verify the chronology, actual opaque
cooked asset domain, semantic bindings and stable asset versions. An explicit
native-feature CI-profile build and strict lint precede the one GPU child. Exact
source/cooked identities, binary and `--list` test name must be bound in the final
execution plan before capture. Source-only preparation is not execution evidence.
