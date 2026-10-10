# 0031 follow-up: opt-in pixel-grid placement contract

Consultation, 2026-10-10. This file recommends a motion contract and the rendered
cases that should later accept it. It is not rendered acceptance, not an
implementation and not an API decision.

## Status

- **Consultation complete.** No Rust, SDK, schema, UI, CI or renderer file changed.
  No Cargo, build, screenshot or capture ran.
- **Recommendation: adopt the candidate in a narrowed form.**
  - An explicit marker makes an entity a *placement root*. It governs that entity's
    drawables and those of its unmarked descendants. The nearest marked ancestor wins.
  - Each pixel-perfect camera snaps the root's render origin to its own world grid,
    using its own tie rule. One rigid offset then moves everything the root governs.
  - A root locked to the camera at a whole-pixel offset inherits the camera's own tie
    decision, so it holds its home pixels at every camera position.
  - In the first version lighting and shadows keep sampling the continuous pose.
  - Perspective and ordinary orthographic cameras ignore the marker.
- **Unmarked content keeps today's behaviour,** including the near-tie follow jump.
  The contract states that limit explicitly. It does not promise stability that a
  camera-only transform cannot give.
- **My earlier fix request is withdrawn as the primary fix.** In `result.md`,
  "Request for Astra" asked to quantize the camera to 1/256 px. That ties the result
  to one adapter's unreported precision and keeps the stretch case. The section stays
  unchanged as history.
- **Earlier evidence is untouched.** No image, helper, measurement or verdict in this
  handoff changed. `result.md` gains only a link here, and all 49 committed screenshot
  hashes still verify.
- **No phase gate** is approved or claimed.

## Model, effort and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`, as named in this session's
  system context.
- Effort: Max was required by the packet and runner. `tools/acp/config.json` pins model
  `claude-opus-5-5` with effort `max`.
- I have no in-session readout of the effort parameter. I cannot inspect the ACP
  handshake, so I cannot independently confirm how the effort was set or verified.
- No model or effort substitution happened on my side.
- Transport: the runner started this session in the handoff worktree through the
  Claude agent harness.

## Packet and director feedback

I read the priority follow-up before writing anything and applied it:
- It supersedes the old packet's implementation and capture steps for this session. I
  wrote this file and appended one short link section to `result.md`, nothing else.
- I did not open, resume or touch handoffs 0032 and 0033.
- I treat the candidate as a candidate. I recommend intent, guarantees and acceptance.
  Astra owns the math, renderer, schema and numerical proof. A new frozen binary is
  needed before any rendered review.

What I read:
- this handoff: [`result.md`](result.md) (both rounds), [`summary.json`](evidence/followup/summary.json),
  [`pixel_neartie.py`](tools/pixel_neartie.py), [`analyse_neartie.py`](tools/analyse_neartie.py)
  and [`pixel_follow.py`](tools/pixel_follow.py);
- docs: `docs/spikes/pixel-perfect-cameras.md`, `docs/spikes/directional-shadows.md`
  and the 2D lines of `PLAN.md`;
- renderer code, read-only, so that the contract matches the real structures:
  `crates/incant_render/src/camera.rs`, `scene.rs`, `pixel_grid.rs`,
  `model_material.wgsl` and `shadows/cascade.rs`;
- Astra's retained log, `/tmp/incant-pixel-camera-precision-before.log`. Its summary
  lines match the packet: release profile, 846 of 1,248 cases passed, 402 mismatches,
  2.57 s.

I did not read the uncommitted test source in Astra's precision worktree. The camera,
scene, pixel-grid and model-shader sources are byte-identical at `72ede59`, this
branch's head and Astra's base `5518b1a`.

## Terms

| Term | Meaning here |
| --- | --- |
| Logical px | One pixel of the camera's logical resolution. All offsets below are in logical px. |
| Camera cell | The camera's snapped position. Static content renders identical frames while the camera stays in one cell. |
| World grid | For one pixel-perfect camera, the points whose coordinates along its right and up axes are whole multiples of 1/pixels_per_unit. With the existing odd-size framing phase, these points land on logical pixel corners at odd and even sizes alike. |
| Placement root | An entity that carries the opt-in marker. The marker's name and schema are Astra's to choose. |
| Render origin | The translation of a placement root's world transform. It is the only point this contract snaps. |
| Camera-locked | Moving with the camera at a constant camera-plane offset, like the followers in the 0031 probes. |
| Home pixels | The pixels a camera-locked object covers when the camera sits exactly on a cell. |
| Footprint | The set of logical pixels a drawable covers in one frame. |

## Why the camera alone cannot hold followers still

Take a follower at a whole-pixel offset n from the continuous camera position c. The
camera renders from its snapped cell s. The follower's edges then sit (c − s) px away
from pixel boundaries, and (c − s) sweeps a full pixel as the camera moves.

At every camera snap (c − s) reaches ±½, so the follower's edges sit on pixel centres,
which is exactly where coverage is decided:
- the CPU decides, in f64, which side of the tie the camera is on;
- the GPU decides which side the follower's edges are on, after rounding vertices to
  its sub-pixel grid;
- on one side of each tie, within about 1/512 px, the GPU rounds the edges onto the
  tie and applies its top-left rule. The CPU has already snapped the camera the other
  way.

The evidence:
- My probe put the band edge between 1.9e-3 and 2.0e-3 px on Apple M5 Pro, with 30 or
  31 of 117 positions displaced per configuration.
- At 1.95e-3 px the 5×3 follower jumped or stretched to 5×4 in 2 of 3 configurations.
- Astra's GPU reproduction found 402 mismatches in 1,248 cases.

Every camera snap is therefore a coverage tie for a continuous follower. Changes to
the camera alone can only move the disagreement:
- a wider CPU tie band moves it to the band's new edge;
- 1/256 px quantization matches one adapter, which wgpu does not report, and keeps the
  f32 edge-noise stretch;
- camera-relative f64 matrices reduce f32 error, but the follower's edges still pass
  through sample points.

Snapping the follower's origin to the same world grid removes the race instead of
moving it. Its edges then sit on pixel boundaries, half a pixel from every sample
point, at every camera position. Coverage gets ½ px of margin rather than depending
on sub-pixel precision. That is why I recommend the candidate rather than another
camera-only fix.

## Recommended contract

### 1. Unmarked content: unchanged, with an explicit limit

- Camera snapping stays exactly as at `72ede59`: right ties toward −right, up ties
  toward +up, the same f64 tie band and the same odd-size framing phase.
- **Static unmarked content:** identical frames within a camera cell, with whole-pixel
  steps between cells. This is already measured exact.
- **Moving unmarked content:** no sub-pixel stability promise. Its footprint changes
  whenever its continuous edges cross sample points.
  - A camera-locked unmarked follower at a whole-pixel offset holds at exact ties and
    at dyadic steps.
  - Near each camera snap it may jump 1 px for one frame, within a band set by the
    adapter's sub-pixel precision.
  - This is documented behaviour, not a regression gate.
- **Code comment.** The renderer comment "A grid-aligned object following the camera
  then stays still" is true only at exact ties and dyadic steps. I recommend that Astra
  narrow it; I did not edit Rust.
- **No markers.** In a scene without markers, frames stay byte-identical to the
  current build, or exactly a predicted and documented set changes.

### 2. Opt-in ownership

**Recommendation: a placement-root marker that governs a subtree. The nearest marked
ancestor wins.**

- The marker may sit on any entity with a Transform, including an empty root such as
  "Player".
- It governs the root's own drawables and those of every unmarked descendant. Today
  that means MeshRenderer, including every imported glTF node and primitive under it.
  Sprites join when they exist.
- A marked descendant is its own placement root. Each drawable follows its nearest
  marked ancestor, or itself if it is marked.
- The marker is ordinary typed component data. It is authored through `incant_cmd`,
  stays visible and reversible, and is cooked into the runtime scene. A template may
  author it explicitly; the camera never implies it.

What the marker never moves:
- cameras, which snap only through their own projection;
- lights, colliders and physics, audio, navigation, scripts, or simulated and authored
  transforms.

Diagnostic placeholder cubes are not content. V1 need not place them, and acceptance
scenes avoid them.

A marker never makes anything drawable. A marked camera-rig frame stays non-drawable.
A camera parented under a marked player still snaps from its own continuous pose.

Three origins stay distinct:

| Origin | Treatment |
| --- | --- |
| Entity render origin | Snapped once per root, per pixel-perfect camera, per frame. |
| Imported-node pivot (glTF node transforms) | Local geometry. Never snapped on its own; moves rigidly with the root's offset, so one object's shape cannot tear. |
| Authored sprite pivot | Local geometry, never snapped by this contract. For crisp texels, texel centres must land on pixel centres when the origin sits on a pixel corner, for example whole-texel pivots at one texel per logical px. A half-texel pivot puts pixel centres on texel boundaries, where nearest sampling becomes adapter-dependent. That rule belongs to the sprite owner, 0032. |

Why a nearest-wins subtree:
- **Per glTF node or primitive:** nodes at non-whole offsets would snap differently and
  tear one object apart.
- **Per drawable, without inheritance:** an unmarked child such as a weapon, hat or blob
  shadow would stay continuous while its parent steps. It would separate by 1 px at
  snap transitions.
- **Outermost root wins:** assemblies stay rigid, but a crate sliding on a marked
  platform could not be quantized. It would crawl across the grid with the same tie
  fragility as today.
- **Nearest wins:** an unmarked child stays rigidly attached. A marked child lands on
  the same world grid, so its motion relative to its parent is quantized to whole
  pixels. A marked child at a whole-pixel local offset never separates from its parent.

### 3. Placement rule

For each pixel-perfect camera, each frame and each placement root:

1. Express the root's render origin in the camera's f64 right/up basis, in logical px.
   Inherited roll defines the basis, as it does for the camera.
2. Snap each axis to a whole px with the camera's tie rule: right ties toward −right
   and up ties toward +up. On screen that is left and up.
3. Do not add the odd-size framing phase to the root. The phase belongs to camera
   framing, and it is what puts world-grid points on pixel corners. Every placed origin
   therefore lands on a logical pixel corner at odd and even sizes.
4. The placement offset is the snapped minus the continuous origin, in the camera plane
   only. It is at most ½ px per axis, and forward depth is unchanged.
5. Apply that one offset rigidly to everything the root governs. Rotation, scale, local
   geometry, node transforms, UVs, animation and depth stay continuous. There is no
   per-vertex or per-bone snapping.
6. **Camera-locked roots inherit the camera's decision.** Suppose the root sits a whole
   number of px from the camera's continuous position, within a tolerance Astra
   declares. Then its placed offset from the camera's eye equals that whole number at
   every camera position, including exact and near ties.
   - Independent absolute snaps can break this. The current tie band is 8 scaled f64
     epsilons. With the camera near the world origin, that is about 1.8e-15 px for the
     camera but about 1.8e-13 px for a root 100 px away.
   - A camera 1e-14 px past a tie would then snap across it, while the root is treated
     as sitting on it. The 1 px jump would return at f64 scale.
   - Deciding the root relative to the camera's own decision avoids this. The method is
     Astra's.
7. Perspective and ordinary orthographic cameras ignore markers, and their frames are
   byte-identical with markers removed. The editor's preview camera is unaffected.
8. Placement is per camera and per frame. Two pixel-perfect cameras with different grids
   may place the same root differently. Neither may change the other's frames, the
   retained scene or frames already queued.
9. Placement happens in the logical image, so whole-number replication, centred
   letterboxing and the explicit too-small failure are unchanged.

A note for implementation, not a design choice: the model vertex stage derives both
raster position and lighting position from one absolute f32 world position
(`out.position=frame.view_projection*p; out.world=p.xyz`). Placement with continuous
lighting (section 5) therefore needs a separate raster translation per placed instance
and camera. That is per-instance data, not per-vertex work.

### 4. What holds still and what may move

| Content | Promise | Strength |
| --- | --- | --- |
| Static, unmarked | Identical within a cell; whole-pixel steps with the world between cells | Exact; measured today |
| Static, marked, origin on the grid | Byte-identical to the same object unmarked | Exact |
| Static, marked, origin off the grid | Drawn at its snapped cell; steps with the world, never against it | Exact |
| Marked, camera-locked at a whole-px offset | Holds home pixels at every camera position | Exact; independent of sub-pixel precision by construction |
| Marked, camera-locked at offset n + f, with 0 < f < 1 | Alternates between n and n + 1: at n + 1 for a share f of each pixel of camera travel | Exact to the world-grid rule; the alternation is intended |
| Unmarked, camera-locked | May jump 1 px for one frame near camera snaps | None (section 1) |
| Unmarked child of a marked root | Rigid with its root | Exact relative to the root |
| Marked child at a whole-px local offset | Never separates from its parent | Exact |
| Marked child at an off-grid or animated local offset | Lands on pixel corners; moves relative to its parent only in whole px | Exact to the rule |
| Constant local shape, with any constant rotation, scale or pivot | Footprint is a whole-px translation of one pattern | Exact for edges on the grid. Repeatable on one adapter for edges exactly on pixel centres. Adapter-dependent for edges within about 1/512 px of a sample point but not on it |
| Rotating, scaling, animated or deforming shape | Aliases honestly; the origin rule still holds | No stability promise |
| Shading of marked content | Constant within a cell when unlit, emissive or under unshadowed directional light; otherwise it evolves with the continuous pose | As lighting dictates (section 5) |

**Half-pixel ties for roots.** A root exactly half a pixel off the grid follows the
camera's tie rule: left on the right axis, up on the up axis. A marked pair half a
pixel apart therefore alternates between two spacings as it moves. Authors who want
parts to hold together should keep them at whole-px offsets, or leave the child
unmarked.

**Smoothly following cameras.** A camera that eases toward a marked player has a
continuously changing, off-grid offset from it. The player then steps 1 px against the
screen while the camera catches up, although every frame is still on the grid. Holding
the player perfectly still needs a camera that follows the player's placed origin.
That is a separate camera feature (open question 5). It will matter, because PLAN's 2D
platformer template must play with a following camera.

### 5. Light and shadow: minimum honest scope

**Recommendation for v1: projection-only placement.**
- Passes rasterized from the pixel-perfect camera's viewpoint, its colour and depth,
  use placed geometry.
- Everything else uses the continuous pose:
  - shadow-map rendering and cascade fitting;
  - light evaluation and clustered light assignment;
  - environment lighting;
  - the positions of lights, including lights parented under a marked root.

What this looks like, to be documented and measured rather than hidden:
- A marked object's own shading and self-shadowing stay self-consistent, because both
  use the continuous pose.
- Its cast shadow on unmarked receivers follows the continuous pose. Relative to the
  placed object it can sit up to ½ px off, which renders as 0 or 1 px. The phase changes
  at snap transitions, so a 1 px "breathing" contact gap is possible.
- A world-fixed shadow edge crossing a marked object can step by up to 1 px at the
  object's silhouette.
- A torch light under a marked root shines from its continuous position, at most ½ px
  from the placed torch.
- Outside the placed and continuous footprints of marked objects, every pixel equals the
  frame with markers removed.
- A pixel-art contact shadow drawn as an unmarked child, such as a blob or sprite, moves
  rigidly with its root and stays exactly attached.

**Shifting shadows and lights with placement is deferred.** It would need:
- per-camera caster offsets in that camera's cascades, which are already camera-fitted;
- lighting evaluated at placed positions;
- lights under marked roots placed as well;
- its own review scene.

Even then, cast shadows are quantized by shadow-map texels, not logical pixels. They
would not step in whole px unless the receiver is parallel to the camera plane.
Nothing about this mode should be claimed until it is built and reviewed. The Phase 4
2D lights in PLAN must state their own relationship to placement.

### 6. Numeric range and precision

- **Declared range.** Astra declares the range in which these guarantees hold: the
  largest world coordinate in metres at the largest pixels_per_unit, and the whole-px
  tolerance in rule 6. Acceptance then tests the edge of that range at both signs.
- **Why it matters.** At 1,000 m the f32 spacing is 2^-14 m. That is 1/512 px at 32 px/m,
  the width of the measured band, and 1/4 px at 4096 px/m.
- **Camera-relative conversion.** Placed instances need a camera-relative f64-to-f32
  conversion.
  - Edges on the grid have ½ px of margin and tolerate far more error.
  - Edges exactly on pixel centres, and other constant edge positions, need error well
    under 1/512 px to stay translation-invariant.
  - Today's absolute f32 path loses that at large coordinates.
- **Unmarked content** keeps the existing absolute f32 path in v1. Its behaviour at the
  range edge is reported, not gated. If Astra changes that path, the changed frames
  must be predicted.
- **Coplanar surfaces.** Coplanar marked and unmarked surfaces have no defined order,
  because placed depth comes from a different f32 path. Separate them in depth or by
  sort layer.

### 7. Independence and cost

- Placement data is computed per pixel-perfect camera per frame and stored with that
  frame's immutable data. The shared resolved scene and its instance buffers are never
  mutated.
- Work is bounded by placement roots plus the instances they govern, per pixel-perfect
  camera per frame. The runtime path has no per-vertex CPU work, no per-tick project
  cloning and no JSON.
- Any future culling must use placed bounds, or bounds grown by ½ px in the camera
  plane. Otherwise an object entering at the frame edge could be culled on its first
  visible frame.
- These are design requirements. Cost claims need Astra's measurements on the plan's
  device path; I make none.

## Tradeoffs considered

| Option | Visual outcome | Verdict |
| --- | --- | --- |
| Camera-only snapping plus documentation | Unmarked followers keep a single-frame 1 px jump in an adapter-dependent band | Kept as the unmarked default; not enough on its own |
| Wider CPU tie band, or 1/256 px quantization | Moves the disagreement; depends on one adapter's unreported precision; keeps the stretch case | Rejected, including my own earlier request |
| Camera-relative f64 matrices alone | Less f32 error, but edges still cross sample points at every camera snap | Needed for placed instances; not sufficient |
| Snap every object when the camera is pixel-perfect | Silently changes existing content, smooth layers, effects and default views | Rejected |
| Per-vertex snapping | Deforms rotated and scaled shapes, warps textures, breaks assemblies, costs O(vertices) | Rejected |
| Snapping each glTF node or primitive | Tears one object's shape | Rejected |
| Per drawable, no inheritance | Unmarked parts separate from their parent by 1 px | Rejected |
| Subtree marker, outermost wins | Rigid assemblies, but child motion cannot be quantized | Not preferred |
| Snapping against the continuous camera instead of the world grid | Followers hold trivially, but static marked content steps against the background | Rejected |
| **Subtree marker, nearest wins, on the world grid** | Rigid by default; marked parts quantized on one grid; static content stable | **Recommended** |
| **Projection-only lighting** | Self-consistent shading; cast shadows up to ½ px detached | **Recommended for v1** |
| Shifting shadows and lights with placement | Contact-consistent shadows; per-camera shadow work; still texel-quantized | Deferred |

## Future rendered acceptance

### Method

The method follows the 0031 follow-up, with predictions made explicit:
- **Binary.** A new frozen binary, with metadata naming its source and sha256, verified
  before and after every engine call.
- **Scenes.** Original scenes built only through public `init`, `import` and
  `rpc command.execute` commands. Authored project and journal bytes stay unchanged.
- **Materials.** Emissive flat colours make footprints exactly measurable. Lit
  materials appear only in A12.
- **Budgets.** At most 128 frames and 256 MiB raw per capture. New output directories
  refuse overwrite.
- **Repeatability.** Every case runs twice from scratch and must match byte for byte.
- **Twin run.** Every case with markers also runs once with all markers removed.
- **Locality, in every case.** Outside the union of each marked object's placed
  footprint and its footprint in the markers-removed run, every pixel must equal the
  markers-removed frame.
- **Predictions.** Expected footprints come from the rule, applied in exact rational
  arithmetic to the authored values as represented. They are never read back from
  engine output.
- **Failures.** Every case reports pass counts and lists every failing frame. A failure
  is reported with its frames and a fix request; an expectation is never adapted to
  accept a defect.
- **Review.** I review real pixels at 8–12× nearest-neighbour zoom, plus frame sequences
  for motion. A concise unedited frame set is committed, and full data stays in ignored
  artifacts.

Reusable geometry from `pixel_follow.py`:
- the test card, follower A (8×8 at −40, −30) and follower B (5×3 at 37, 17);
- resolutions 320×180, 255×143, 256×143 and 255×144 at 32 px/m;
- rolls 0°, 90° and 0.37 rad, inherited from a rig.

The near-tie set is δ = 0 and ±{1e-15, 1e-14, 1e-12, 1e-9, 1e-6, 1e-4, 1e-3, 1.9e-3,
1.95e-3, 2e-3, 4e-3, 1e-2} px. It is applied around four ties: right +½, right −½, up +½
and up −½. That makes 100 positions per configuration.

### Cases

**A0. Default compatibility.**
- Run the existing helpers byte-unchanged, with no markers: `pixel_lookdev.py`,
  `pixel_follow.py` and `pixel_neartie.py`.
- Pass: run digests equal the corrected build's `4a1b6d95…`, `2701de54…` and
  `e2ed9775…`, or exactly a predicted and documented set changes.
- Also capture the A5 scene through perspective and ordinary orthographic cameras, with
  and without markers. Pass: byte-identical.

**A1. Near ties with mixed marker states.**
- Setup: all 12 configurations through the near-tie set, one capture of 101 frames each.
  Each configuration has:
  - marked followers A and B;
  - unmarked twins A′ and B′ at other whole-px offsets;
  - a marked off-grid follower O at (+20.5, +10.25) px.
- Pass:
  - A and B are at home pixels, at exact size, in all 2,400 follower frames;
  - O matches the world-grid prediction, at constant size, in every frame;
  - A′ and B′ match their own footprints in the markers-removed run. Markers must not
    change unmarked objects, for better or worse.
- Visual: no blended colours on marked edges, and unmarked jumps look like the
  documented band.

**A2. Arbitrary-speed motion.**
- Setup: at 320×180 0° and 255×143 0.37 rad, the camera and A1's objects move for 127
  ticks at non-dyadic steps, such as 0.137 px per tick right and 0.071 px per tick up.
- Pass: as A1, against predictions for every frame.
- Visual: the camera-locked marked followers are perfectly still, and O's alternation
  cadence reads as intended.

**A3. Signs and large translations.**
- Setup: repeat A1 at 320×180 0° and 255×143 0.37 rad, with the whole configuration
  moved to world (+1000, −1000) m and (−1000, +1000) m. Repeat both sign pairs at the
  edge of Astra's declared range.
- Deltas below the f64 spacing at those coordinates collapse. Predictions use the
  represented values.
- Pass: marked results equal the origin case, frame by frame.
- Report, without gating, whether the unmarked card and twins stay exact.

**A4. Static marked content.**
- Setup: four static marked quads:
  - S0 on the grid;
  - S1 at (+0.3, −0.7) px;
  - S2 at (+½, +½);
  - S3 at (−½, −½).
- The camera follows the `pixel_follow.py` path and the near-tie set, at all four
  resolutions at 0° and at 320×180 0.37 rad.
- Pass:
  - every quad steps with the background and never against it;
  - S0's frames equal the same scene with S0 unmarked, byte for byte;
  - S1, S2 and S3 sit at their predicted cells in every frame.

**A5. Assemblies, nodes and pivots.**
- Setup: a marked root carries one MeshRenderer whose glTF has three primitives in
  nested nodes:
  - a 7×9 body at (0, 0);
  - a 5×2 hat at (+1, +9), under a node rotated 90°;
  - a 2×2 badge at a non-whole (+2.37, +3) px.
- The root has three children:
  - an unmarked 6×1 weapon at (+7.25, +4) px;
  - a marked 3×5 shield at a whole-px (−3, +2);
  - a marked 2×2 charm whose local up offset bobs continuously between 0 and 2 px.
- The root follows A2's motion and the near-tie set, at 320×180 0° and 255×143 0.37 rad.
- Pass:
  - body, hat, badge and weapon keep constant pixel distances from each other;
  - each part's footprint is a whole-px translation of its frame-0 footprint;
  - the shield never separates;
  - the charm lands on pixel corners and moves relative to the root only in whole px.
- Visual: the assembly reads as one object, with no part shimmering against another.

**A6. Inherited roll and tilted views.**
- Setup: A1 to A5 already cover roll. Add a camera pitched 30°, and one pitched 30° and
  yawed 45°. In each, a marked emissive box and an unmarked twin move along world x and
  z over a ground plane.
- Pass: the marked box's footprint is a whole-px translation of its reference footprint
  in every frame.
- Report the unmarked twin's behaviour.

**A7. Rotation, scale and animation honesty.**
- Setup:
  - a marked shape rotates 1.7° per tick about its origin, with an unrotated marked
    2×2 pivot indicator as a sibling at the same origin;
  - a marked shape pulses in scale between 0.9 and 1.1;
  - a marked shape at a constant 17° translates with A2's motion.
- Pass:
  - the indicator follows the rule;
  - the rotating and scaling shapes stay within their placed bounds;
  - the constant-17° shape's footprint is translation-invariant.
- The appearance of rotating and scaling shapes is documented, not gated.

**A8. Several camera grids and independent frames.**
- Setup: one scene and script, rendered through three pixel-perfect cameras:
  - 32 px/m at 320×180 and 0°;
  - 16 px/m at 160×90 and 0°;
  - 32 px/m at 255×143 and 0.37 rad.
- Pass:
  - each camera's frames equal its single-camera reference run, byte for byte;
  - placed origins lie on each camera's own pixel corners;
  - if the tool can queue frames for several cameras in one run, interleaving changes
    no frame.
- Where the frozen tool cannot queue frames that way, Astra's GPU test covers queue
  independence.

**A9. Projection switches and retained views.**
- Setup: one 128-tick run while A5's assembly moves. The selected camera switches:
  - from pixel-perfect to orthographic at tick 32;
  - to perspective at tick 64;
  - back to pixel-perfect at tick 96.
- Output size also changes mid-run, where the tool supports it.
- Pass:
  - pixel-perfect segments equal an uninterrupted pixel-perfect run at the same ticks;
  - orthographic and perspective segments equal the same run with markers removed.

**A10. Replication and letterboxing.**
- Setup: A1 at 320×180 0°, and A5, at 640×360, 961×541, 800×600 and 360×640. Capture
  255×143 at 768×432. Capture every second tick where the budget needs it.
- Pass:
  - every frame is an exact whole-number replication of its logical frame;
  - the letterbox matches the 0031 table;
  - the logical content equals the 1x frame.

**A11. Frame-edge entry.**
- Setup: a marked 8×8 mover crosses each of the four logical frame edges through the
  near-tie set, at 320×180 0° and 255×143 0.37 rad.
- Pass: on every frame its visible pixels equal its placed footprint clipped to the
  frame. It appears on the first frame that footprint overlaps the frame.

**A12. Light and shadow, projection-only scope.**
- Setup, at 320×180, through the 30° pitched camera and the 30°/45° camera from A6:
  - unmarked lit receivers: a ground plane, and a backdrop parallel to the camera plane;
  - one oblique, shadowed directional light;
  - a marked lit box, and its unmarked twin;
  - a point light parented under the marked root;
  - an unmarked static caster whose shadow crosses the marked box's path.
- Motion: 1/8 px steps, then the near-tie set, then A2's motion.
- Pass:
  - the marked box's coverage is translation-invariant;
  - the locality check holds. This proves that lighting and shadows did not move, so
    the box's cast shadow is exactly the markers-removed shadow;
  - the change in box-to-shadow distance against the markers-removed run is reported,
    and is at most 1 px per axis;
  - the static shadow's step at the box's silhouette is at most 1 px.
- Visual:
  - no lighting seam where the box crosses cluster tile boundaries;
  - a verdict on whether the contact "breathing" is acceptable for v1, or whether
    shifting shadows must come out of deferral.

**Budget check.**
- An A1 capture is 101 frames at 320×180 or smaller, about 23 MB raw.
- A 961×541 frame is about 2.1 MB, so 128 frames would sit just under 256 MiB. A10
  therefore captures every second tick.

## Coverage of the packet's questions

| Packet question | Answered in | Acceptance |
| --- | --- | --- |
| Opt-in ownership; entity origin, node pivot and sprite pivot | §2 | A5 |
| Parent chains, inheritance and camera rigs | §2; §3 rule 5 | A5, A6 |
| Whole-px versus off-grid offsets; half ties; odd-size framing | §3 rules 2, 3 and 6; §4 | A1, A2, A4 |
| Static and environment content; animation | §1, §4 | A4, A5, A7 |
| Rotation and scale | §4 | A7 |
| Light and shadow | §5 | A12 |
| Perspective and ordinary orthographic cameras | §3 rule 7 | A0, A9 |
| Near ties on both sides; positive and negative positions; large translations | §3 rule 6; §6 | A1, A3 |
| Inherited roll | §3 rule 1 | A1 to A6 |
| Mixed marker states | §1, §2 | A1, A5, A12 |
| Multiple primitive nodes and non-zero local pivots | §2 | A5 |
| Retained views, projection switches and several camera grids | §3 rule 8; §7 | A8, A9 |
| Integer replication and letterboxing | §3 rule 9 | A10 |
| Bounded cost and independent frames | §7 | A8 for frames; Astra's measurements for cost |

## Limitations

- **Analysis only.** This session produced no frames or measurements. The numbers come
  from committed evidence and Astra's retained log.
- **One adapter.** The ½ px margin argument rests on standard rasterization rules and the
  measured band, and only Apple M5 Pro is measured. Portability of marked content is "by
  construction" until a second rasterizer confirms it, such as a CI software adapter or
  a hosted phone GPU. I am not requesting either here.
- **Test source not read.** I relied on the packet and the log, not on Astra's
  uncommitted precision test.
- **Effort not independently verified;** see "Model, effort and transport".
- **Gaps carried forward.** Pitched and yawed pixel-perfect cameras are still untested;
  A6 and A12 add them. Outside this contract, apart from the edge-entry requirement:
  native chrome masking, editor picking and gizmos in pixel-perfect views, world-space
  particles, and future culling.
- **No performance claim.**

## Open questions

1. Director and Astra: is projection-only lighting accepted for v1, with the documented
   shadow detachment? I recommend yes.
2. Astra: what are the declared numeric range and the whole-px tolerance, and how does a
   root's decision stay consistent with the camera's?
3. Astra: what are the marker's name and schema, and its cooked runtime form? It must be
   typed component data, with no document dependency in the shipped runtime.
4. The sprite owner, when 0032 is ready: what pivot rule keeps texel centres on pixel
   centres, and should sprite templates author the marker?
5. A later camera feature: should a camera follow a marked target's placed origin, so
   that eased follow cameras do not create off-grid offsets?
6. Should a pixel-perfect camera be able to ignore markers, for example for cinematics?
   I do not propose it for v1.
7. The marker's Inspector presentation is a later visual task. Handoff 0033 owns the
   editor redesign.

## Changed paths and commands

- New: `handoffs/0031-pixel-perfect-cameras/origin-contract.md`.
- Appended: a short link section at the end of `handoffs/0031-pixel-perfect-cameras/result.md`.
  No earlier text changed.

All commands were read-only:

```sh
git log / git show / git diff / git merge-base     # history; renderer sources equal at 72ede59, HEAD and 5518b1a;
                                                   # impl/pixel-camera-precision has no commits beyond 5518b1a
grep and sed over PLAN.md, docs/spikes and crates/incant_render
head and tail of /tmp/incant-pixel-camera-precision-before.log
(cd handoffs/0031-pixel-perfect-cameras && shasum -a 256 -c evidence/screenshots.sha256)   # 49 OK
```

Not run: Cargo, Clippy, the GPU tests, any build, the headless binary, screenshots,
captures and UI tests. This session produced no screenshots.
