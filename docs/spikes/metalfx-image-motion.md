# MetalFX real-material image/motion fixture

The bounded capture passed its runtime assertions, but **appearance failed**.
MetalFX remains default-off. No upscaler quality or performance acceptance follows.
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
not a vendor coverage readback. Image review inspected detail, silhouettes,
disocclusion, lag and cut response in the sampled frames. There is no image-difference acceptance
threshold, 60 fps claim, or claim that MetalFX completes built-in upscaling.

One explicitly ignored GPU child ran with a 120-second limit, no retries,
and a create-only output directory capped at 128 MiB. Source-level explicit
counts are 96 scene submissions plus 15 readback submissions = 111 queue submits,
15 maps, 64 native encodes and 239 submitted command buffers. Startup uploads and
vendor-internal work are not inferred from these counters. Raw display payload is
31,104,000 bytes; generated source/cache retention is separately capped at 16 MiB
and 128 files. A runner must maintain the 6 GiB free-space floor and capture
terminal/descendant cleanup. No window or user pointer interaction is involved.

The two CPU fixtures passed the chronology, actual opaque cooked asset domain,
semantic binding and stable asset version checks; the GPU test was ignored in
that CPU invocation. Native-feature all-target strict Clippy passed. Rust 1.99,
explicit aarch64-apple-darwin, locked/offline CI profile, jobs 2 and Rayon 2 were
bound in the retained commands. Source001 first failed compilation with E0432:
the reused integration fixture imported the crate's external name inside its own
library tests. Source002 adds only a macOS/native-feature/test-gated self alias.
The original failed attempt and both exact source generations remain retained.

Actual capture on Apple M5 Pro / Metal completed once: 96 scene frames, 15 maps,
111 explicit queue submissions and 64 native encodes. Every arm's authored input
records matched exactly. FULL frames 1 and 16 are byte-identical. The copied
binary reported one test passed, no failures or ignored tests; the owned child
and descendants were reaped. Its 2.08-second test duration is fixture execution
context, **not** a shipping performance measurement.

Root and producer inspected all 15 unchanged PNGs. At 50%, two horizontal thin
wires disappear, the diagonal becomes dotted/thicker, and the horizon and sphere
edges remain stepped. At 75%, horizontal lines survive but the silhouette and
horizon remain coarse. The chart is substantially softer than FULL at both
scales. These defects remain visible in accumulated static frame 16, moving frame
24, cut frame 25 and continued frame 32. The mover/cut occupies the expected
positions in these samples without an obvious large trail; five still images do
not establish continuous-motion or disocclusion quality. FULL itself remains a
single-sample reference, not an alias-free oracle.

Source diagnosis separates evidence from hypotheses. `metalfx/normalize.wgsl`
uses vendor RGB only if every positive current bilinear contributor is fully
opaque and depth-compatible. Other footprints use only current spatial HDR.
This structurally excludes reconstructed thin-over-clear and mixed-depth
silhouettes. It does not by itself establish the cause of interior chart blur.
The material shader also uses implicit render-resolution mip selection and the
native route retains eight jitter phases. Apple recommends broader sample
coverage and a texture LOD bias for temporal reconstruction, while warning of
flicker/moire tradeoffs; neither recommendation has been applied or qualified
here. [Apple's MetalFX integration guidance](https://developer.apple.com/videos/play/wwdc2022/10103/)
supports inspecting these inputs separately, not changing thresholds to admit
these pixels. Motion-direction/unit algebra and fixed exposure have no identified
source mismatch; raw vendor/color/depth/motion/selection readback is still needed.

The editor backdrop is display-referred and composed after tone mapping.
Composing a real scene-linear sky/background before MetalFX would be an explicit
opaque-output domain, with its own depth/motion/exposure contract. Transparent
black cannot simply be relabeled as opaque. The present guard and supported
domain remain unchanged; no follow-up GPU run is included in this evidence.

Portable replay: [manifest](evidence/metalfx-image-motion/manifest.json),
[archive](evidence/metalfx-image-motion/records.json.gz), and
[verifier](evidence/metalfx-image-motion/verify.py). The archive includes exact
sources, commands, original compile failure, successful CPU/GPU logs, all 15 PNG
and raw display pairs, cooked assets, all 96 semantic input records and both pixel
reviews. The verifier independently decodes PNGs against the raw RGBA8 files,
checks chronology/reset/selection/input bindings, and preserves appearance FAIL.

```sh
python3 -B docs/spikes/evidence/metalfx-image-motion/verify.py
# Optional retained local originals, copied executable and immutable Git source:
python3 -B docs/spikes/evidence/metalfx-image-motion/verify.py --originals --binaries --git-source
```

Executed source: `dcc208ac3868e669064e840aa00aa50e4b3c3b2b`.
Source002 freeze: `fbebc3449e0607acd1002fc338a678e4626ef245243ca1beeb96d853d8bd6e73`.
GPU report: `a2e075743afd89dc07a3bc15cb70068d2d18dd2114f516973013f08ad64e8ef8`.
External originals: `~/.codex/incant-evidence/metalfx-image-motion-001`.
