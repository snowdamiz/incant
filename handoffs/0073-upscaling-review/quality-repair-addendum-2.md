# 0073 quality repair addendum 2: shading changes, camera ownership and owner identity

**Status: design correction for root review, CPU only.** No renderer source,
GPU, Cargo, build, target or capture was used, and nothing here is implemented.
No shader or resource work should start until root has reviewed it.

These stay unchanged:
- `quality-repair-brief.md` (e1b26907);
- `quality-repair-addendum.md`, `repair-model/model.py` and
  `repair-model/results.json` (7b3dc64c).

This addendum replaces the parts listed in section 2.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-admission-review2-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs, read before any change:**
- root's feedback `upscaling-addendum-feedback-20261013.md` `b85631cf…`;
- the readiness audit `upscaling-readiness-audit-001`, under
  `~/.codex/incant-evidence/`. Root's revision expected it under
  `~/.codex/incant-reviews/`, where no copy was present during this session:
  - `feedback-for-root.md` `faae6df3…`;
  - `implementation-map.md` `05a0b3fe…`;
  - `review.json` `85a05dc2…`;
  - `counterexamples.py` `2a7cdb3c…` and its output `counterexamples.json`
    `fcb38800…`;
- root's acceptance of the test-only oracle 2fe23dce,
  `upscaling-kernel-oracle-root-20261013.json` `935b1662…`;
- the oracle record `docs/spikes/upscaling-kernel-oracle.md` at 2fe23dce (blob
  `29905aad…`);
- renderer and cooker source at 2fe23dce, read from git objects and from the
  audit's source copies.

**Evidence.** Every number below comes from
[`repair-model/model_v2.py`](repair-model/model_v2.py). It imports the unchanged
`model.py`. Its output is
[`repair-model/results-2.json`](repair-model/results-2.json), which contains:
- all 50 retained cases, re-run under these rules once per shadow detector;
- 21 new cases. Ten of them are counterexamples or recorded failures and are
  expected to fail.

Every outcome matches its expectation. Section 10 covers reproduction and the
model's limits.

## 1. Feedback and disposition

| Finding | Source | Disposition |
|---|---|---|
| The scalar shadow key cancels across lights. Visibility (1, 0) → (0, 1) keeps the key at 0.5 while the direct term falls from 0.30876 to 0 | Root's item 1; audit P1 | Accepted. The key is withdrawn (section 3) |
| Zero screen displacement is not a static camera. An orbit about the receiver changes the response from 20.372 to 0.00969 at a fixed pixel. A dolly from z 5 to 4 reconstructs world z −1 from the stored depth | Root's item 2; audit P1 | Accepted. Camera-state ownership (section 4) |
| Root's original counterexample stays a negative control (0.729 after 3 frames, 0.43046721 after 8) | Audit item 3 | Kept. F1 and F2 still fail in both retained runs |
| Newcomers must not inherit old confidence. A third owner can enter or leave. Moving reprojection keeps provenance. Equal or tolerance-close depth is not identity | Implementation map | Accepted. Section 5, with counterexamples N4–N6 and P1 |
| Typed layouts, the cooker's per-binding native map, the budget, retained and abandoned ownership, the no-light fallback, exact integer transport, no hash identity | Root's mechanics paragraph; implementation map | Section 7 |
| Oracle misses are recorded, not forced into agreement. The original C7 and C9 stay failed. Replacement controls need root's review | Root's feedback; oracle acceptance | Section 8 |

## 2. What this replaces

**Replaced:**
- **Addendum 1 §2, steps 1–6** (static path, moving admission, shading key,
  frame-global flag): replaced by sections 3–5.
- **Addendum 1 §5, key cost and gating, with K1 and K2:** superseded. The
  gating rectangles become the detector itself (section 3.2).
- **Addendum 1 §7, data path and accounting:** replaced by section 7.
- **Addendum 1 §5, C9 "≥ 0.65 / ≥ 0.85", and the brief's 2.1 and C3 claims:**
  corrected in section 8.

**Unchanged:**
- brief 2.1–2.3 reconstruction, the `W` cap and the rescale;
- the 2.6 bias, which still waits for this admission;
- addendum 1 §6 coordinate, tie and edge rules;
- the near radius `R = max(r, 4ρ)` and the reset value 0.125.

## 3. Shading changes at static pixels (blocker 1)

### 3.1 Why a per-pixel visibility summary is not enough

N1 reproduces the audit's case on a textured receiver. Visibility for two
equal-luminance lights swaps from (1, 0) to (0, 1), and light 2 faces away.

| N1 rules | Lag 1 frame after the swap | 2 frames | 4 frames |
|---|---|---|---|
| 7b3dc64c scalar key (expected fail) | 0.378 | 0.357 | 0.320 |
| Changed-caster footprints (3.2) | 0.017 | 0.004 | 0.002 |
| Per-light signature (3.4) | 0.017 | 0.004 | 0.002 |

A per-light code fixes the cancellation across lights. It still samples one
point, which leaves three gaps:
- **(a) Wrong point.** The history was built from samples at their own jittered
  world points, up to `R` from P and at several depths. A change there need not
  change the visibility at P's centre.
- **(b) Wrong offset.** Shading offsets the lookup by
  `n · texel_world · 0.25` (`shadows/shade.wgsl` L8). The resolve has no
  normal, so it cannot use the same offset.
- **(c) Undecided changes.** An 8-bit code leaves a band where a change may or
  may not be detected (Q1).

So the required rule evaluates no visibility at all.

### 3.2 Required: changed-caster footprints

**Host, per prepared frame, from the accepted state (7.2):**
- **Changed casters:** owners in this frame's changed set whose batch has
  `casts_shadows`. This includes animated batches.
- **Light orientation:** for each shadowed directional light (at most
  `MAX_SHADOW_LIGHTS` = 4, `shadows/mod.rs` L13), the rotation
  `look_to_rh(0, forward, up)` exactly as `shadows/cascade.rs` L71 builds it.
  Its x and y rows are kept.
- **Rectangles:** for each changed caster, the 8 corners of its accepted world
  bounds and the 8 of its current bounds are projected to the light's xy. The
  bounds are the mesh bounds times the world, or the animated geometry bounds
  for skinned batches. The 16 points give one rectangle.
- **Limit:** at most 8 rectangles per light. A ninth merges the pair whose union
  has the smallest area (ties: the lowest indices). Merging only grows
  rectangles.

**Resolve, on static pixels only (so only with a static camera, section 4).** It
runs where history is kept without a reset (5.1 step 3):
- **Depth range:** `d_lo` is the minimum of the stored front depth and the near
  geometry taps' view depths. `d_hi` is the maximum of the stored far depth (a
  new meta field, 7.4) and the near taps' view depths. Depth 0 (clear) is
  excluded. A pixel with no geometry, stored or near, has no receiver and
  skips the test.
- **World points:** two points on P's unjittered ray, at `d_lo` and `d_hi`,
  through the unjittered inverse view-projection.
- **Per light with rectangles:**
  - Project both points to the light's xy and take their bounding box.
  - Grow it by `e = R · pixel_world(d_hi) + 2.25 · t`. Here `pixel_world(d)` is
    one output pixel's world size at view depth `d` (section 7.6). `t` is the
    larger `texel_world` of cascade `i` and, if it exists, cascade `i + 1`,
    where `i` is the cascade containing `min(d_hi, far_3)`.
  - Skip the light when `d_lo` lies beyond its last cascade. There visibility
    is constantly 1 (`directional_shadow`, `shade.wgsl` L20).
- **Hit:** if the grown box meets any rectangle (closed intervals), clip to the
  near box and set `W ← min(W, 0.125)`, as the key did.

**Why it misses no change.** Under these assumptions, a static receiver
sample's visibility for a light can change only if the receiver's grown box
meets a changed caster's rectangle:
- **A1 Fixed cascades.** With a static camera and a static light, cascade xy is
  fixed (`cascade.rs` L71–140). Changes to the depth range from caster bounds
  (L89, L145) are caught by the frame-global flag (3.3).
- **A2 Changed texels.** Shadow texels change only under a changed caster's
  previous or current rasterised footprint. Unchanged casters are rasterised
  deterministically, in a fixed batch order. A handedness flip reorders draws
  only for the flipped placement, which is itself changed.
- **A3 PCF reach.** PCF reads texel centres within 2 texels of the offset point
  (3×3 compare samples 1 texel apart, `shade.wgsl` L13–17). The normal offset
  moves the point by at most 0.25 texel. Together: 2.25 texels.
- **A4 Cascade blends.** A blend uses cascades `i` and `i + 1`, so the larger
  texel is used.
- **A5 Sample range.** Samples that contributed to P lie within `R` of P's ray
  and between `d_lo` and `d_hi`. The weight left out beyond `R` is bounded in
  section 8.

The rule can over-invalidate, but it cannot miss. It has no quantisation and no
collisions, reads no shadow map, and needs no lighting bind group.

**Cost:**
- **Host:** per changed caster, 16 corners × up to 4 lights, 2 dot products
  each.
- **Resolve:** nothing when no rectangle exists. Otherwise, per static pixel
  that keeps history without a reset:
  - 2 world reconstructions;
  - 4 dot products per light;
  - up to 32 box tests, each 4 comparisons.

**Quality cost (K3, a shadow band moving over a textured receiver):**

| Detector | Lag | Error in the shadow interior |
|---|---|---|
| Footprints | 0.018 | 0.039 |
| Per-light signature | 0.019 | 0.021 |

Receivers inside a swept footprint lose accumulated detail while a caster
moves, even where their shadow did not change. With many moving casters,
merged rectangles can cover the view. The static path then degrades to clip and
reset on every frame while they move. It stays correct, but it is aliased.

**Retained shadow cases under this rule:**
- C8 lag 0.018;
- F5 (no detector) still fails, at 0.144;
- K2 gives identical outputs;
- R1 at 0.125: lag 0.018, mean |difference| 0.065.

K1's gather variant no longer applies, because no visibility is sampled.

### 3.3 Frame-global flag in the retained path

**The flag is set when either of these holds:**
- any light node's world differs bitwise from the accepted frame's;
- any shadow cascade's matrix or parameters differ bitwise from the accepted
  frame's. This also covers depth-range changes from caster bounds.

The light list, kinds, shadow slots, radiance and range, and the environment,
cannot change inside a retained scene. `stage` refuses them and asks for a
replacement (`retained.rs` L259, L271–304), and a replacement resets history
(section 6). Addendum 1's environment and light-list triggers are therefore
moot on this path.

**Effect:** unchanged. There is no unclipped persistence. Pixels clip in place
with `W ← min(W, 0.125)` (retained C6 lag 0.006; F9 without the flag 0.231).

### 3.4 Measured alternative, not recommended: per-light signature

**Definition:**
- Per shadowed light slot, `code = floor(255 · vis + 0.5)`, where `vis` is
  `directional_shadow` at the stored point with a zero normal offset. Four
  8-bit codes pack into one u32.
- A slot invalidates when `|Δcode| ≥ 16`.

**Quantisation (Q1, exact over a 1/1020 grid):**
- `|Δvis| < 15/255` never triggers;
- `|Δvis| ≥ 17/255` always triggers;
- between them, 73% of pairs trigger.

On PCF levels it never misses the 1/16 rule, with 118 extra triggers among
5,329 pairs.

**Collisions:** none across lights. Within a light, equal codes mean equal
visibility at the stored point only, which is gap (a) of 3.1.

**Cost:**
- 4 lights × 2 cascades × 9 compare samples = 72, inside the footprint gate;
- group-3 shadow bindings in the resolve;
- a no-light fallback group (7.8).

It fixes N1 and gives a lower interior error in K3 (0.021), but it keeps gaps
(a)–(c). If root prefers it, those gaps must be recorded as accepted, and 3.2
becomes its gate.

## 4. Camera-state ownership (blocker 2)

### 4.1 Definition

`camera_static` is a host flag for the prepared frame. It is true only if all of
these are bitwise identical to the last accepted frame's values:
- the staged unjittered view matrix and projection matrix (`CameraView` from the
  frozen camera binding and the camera node's world, `retained.rs` L316);
- the upscale output size and output rectangle.

It is never derived from motion vectors or screen displacement.

**Excluded:**
- **Jitter:** it is the sampling pattern.
- **The internal DRS scale:** a scale change is handled by the confidence
  rescale, which uses the accepted previous source width (7.2).

### 4.2 Rules
- **Static pixel:** screen-static (the motion source moved less than 1/256 px)
  and `camera_static`. Without a motion source, static means `camera_static`
  (unchanged).
- **Static branch only:** unclipped persistence, the footprint test and any
  world point reconstructed from stored depth happen only on the static branch.
  They never run under a changed camera.
- **Changed camera:** every pixel with a motion source takes the moving path.
  That includes zero-displacement pixels, which fetch at `P_prev = P`.
- **Admission depth:** moving admission compares the stored front depth with the
  *expected previous* view depth from the motion target, never the current
  depth. The target's z is the previous NDC depth (`motion/shader.wgsl` L39).
  Under the audit's dolly, the stored depth 5 matches the expected 5, not the
  current 4.
- **Camera stops:** the first camera-static frame resumes persistence from the
  clipped moving-path history.

### 4.3 Evidence

| Case | Persisted while the camera changed | Footprint or key tests then | Error at the last changing frame | Static-camera reference |
|---|---|---|---|---|
| N2 orbit, 7b3dc64c (expected fail) | 6,144 | 6,144 | 0.421 | 0.143 |
| N2 orbit, this addendum | 0 | 0 | 0.000 | 0.143 |
| N3 dolly, 7b3dc64c (expected fail) | 3,072 | 3,072 | 0.136 | 0.143 |
| N3 dolly, this addendum | 0 | 0 | 0.091 | 0.143 |

**F10:** admitting by current depth during the dolly gives an error of 0.261,
against 0.091 with the expected previous depth (expected fail).

The model is 2D. It cannot show the dolly's wrong world point (z = −1), or the
key evaluated there. The 7b3dc64c dolly's error stays within the reference, so
N3's counterexample is structural: unclipped persistence and shading tests while
the camera changed.

## 5. Owner rules: newcomers, identity and provenance

### 5.1 Per output pixel (replaces addendum 1 §2 steps 1–6)

1. **Near taps:**
   - the taps within `R`;
   - their owners (the low 31 bits of the owner target) and changed bits
     (bit 31);
   - the front contributor (nearest view depth);
   - the rectification box, primary and motion source, as in addendum 1.
2. **Static:** as defined in 4.2.
3. **Static pixel with canonical stored history (7.4):**
   - **Persist (unclipped)** when all of these hold:
     - the stored owners did not overflow;
     - no stored owner is in the changed set;
     - no current contributor has its changed bit;
     - the frame-global flag is clear.

     Then apply the footprint test (3.2), and update the stored data:
     - owners ← stored ∪ contributors (overflow above two);
     - front ← the nearer of the stored and current fronts;
     - far ← the farther of the stored far depth and the near taps.
   - **Drop** when a changed stored owner has no near tap with its id. This is
     identity, not depth. With overflowed owners, drop when the front owner
     changed and no near tap is depth-compatible with the stored front depth.
   - **Otherwise clip in place.** If any changed owner is present (stored or
     current), or the frame-global flag is set, also set `W ← min(W, 0.125)`.
     Without either, apply the footprint test (3.2). This covers overflowed
     owners. Then owners ← contributors, and front and far come from the near
     taps.
4. **Moving pixel with a motion source:**
   - **Fetch** bilinear history at `P − motion`.
   - **Admit** a history tap only when its front owner owns a same-motion near
     tap, and its front depth is compatible with that owner's expected previous
     depth.
   - **Rectify:** renormalise, clip to the near box, apply the reactive term.
   - **Store:**
     - owners ← contributors ∪ the admitted taps' stored owners. This overflows
       above two, or when an admitted tap had overflowed;
     - front ← from the near taps;
     - far ← the farther of the near taps and the admitted taps' far depths.
5. **No motion source and a changed camera:** no history.

### 5.2 Counterexamples retained, one per repair

| Repair | Case | 7b3dc64c rules (expected fail) | This addendum |
|---|---|---|---|
| Persist only while current contributors are also unchanged, with a reset when a changed owner is present | N5: a bar enters and stops, once nearer than the floor and once at its depth. Worst residual against a bar that never moved | 0.292 for both | 0.031 for both |
| Drop by identity, not depth | N4: a coplanar panel leaves a textured wall. Worst residual against a matched-reset wall | 0.691 | 0.000 (314 drops) |
| Admit by owner identity on the moving path | N6: a striped wall 0.06 behind a sliding panel, against one 2.0 behind. Worst difference over 24 frames | 0.715 (4,451 pixel-frames) | 0.000 (0) |
| Admit against the expected previous depth | F10: a dolly, admitting by current depth (as 7b3dc64c does) or by expected previous depth. Mean error at the last changing frame | 0.261 | 0.091 |

N6 also fails for depth-only admission under the new rules: 0.499 over 195
pixel-frames. A wall within the depth tolerance supplied history fetched along
the panel's motion, which misregisters a static texture.

**N5's mechanism.** The bar reaches the pixel's near window in the frame it
first contributes. No *stored* owner has changed yet, so the 7b3dc64c rule kept
the old background at full confidence.

### 5.3 Provenance: required by the audit, no measured benefit

Step 4 keeps the admitted taps' owners, so the static premise holds: every
owner whose colour the history holds is recorded.

**P1:** a checker panel stops while a bar that crossed behind it keeps moving.
The residual is taken against the bar-free scene replaying the run's own drops
and resets, for stops 9–13.
- With and without provenance, the worst residual is the same in all four
  backgrounds and scales:
  - clear 8/16: 0.349;
  - clear 16/16: 0.221;
  - grey 8/16: 0.296;
  - grey 16/16: 0.063.
- Of the 20 runs (4 settings × 5 stops), it is lower with provenance in 4 and
  lower without it in 2.
- In 33 other runs (11 moving scenes × 3 scales), it changes no output bit.

It is kept as conservative bookkeeping, not as a measured repair. Root may
remove it, and no case depends on it.

### 5.4 Limit: exact depth ties

Among exactly equal depths, the front owner is the first tap in row-major order
(addendum 1 §6), so identity admission follows that order. N4 and N6 cover
coplanar and tolerance-close surfaces. Z-fighting surfaces remain an
integration control.

## 6. Retained-scene events

The portable upscaler exists only on `RetainedScene`. Its `upscaling` owner is
created by `enable_temporal_upscaling` (`retained.rs` L211) and cleared by every
motion re-enable (L169, L179). These are its events.

| Event | Source at 2fe23dce | Class | Upscaler response |
|---|---|---|---|
| Node count or ids; camera binding (projection, pixel resolution); light binding (kind, colour, intensity, range, shadows); mesh asset version or `cast_shadows`; sprite, tilemap and frame bindings; text presence; environment version, intensity or rotation | `retained.rs` L259, L271–304, L379–389 | Topology: `stage` refuses the frame and asks for a replacement | A replacement is a new `RetainedScene` with no upscaling owner (L100), so it gets new owner ids and empty history. Nothing is aliased, even when node ids, geometry, ticks and sizes match |
| Batch structure: model, primitive, shadows, mirrored set, animated, placements, vertex buffer size | `motion/retained.rs` L108–127 | Topology: `MotionError::Topology` refuses the frame | Frame refused; accepted state unchanged |
| Mesh node world transform | `retained.rs` L339; `pack_static` L437 | Owner change for each placement whose world × local differs bitwise | Changed bit; footprint rectangles if the batch casts shadows |
| Handedness flip | `pack_static` L449; `models.rs` L200–251 | Owner change: the transform changed, and the placement moves to the twin batch | Its owner id stays the same, because ids are per placement (7.1) |
| Animation sample (pose, blend, IK, root) | `retained.rs` L266–269; `animation::stage` | Owner change, unless the joint palette and motion world are bitwise equal to the accepted ones | As a transform change. If palettes cannot be compared, the owner is marked changed every frame (conservative) |
| Camera node transform | `retained.rs` L316 | Camera change | `camera_static` false (section 4) |
| Light node transform | `retained.rs` L312 | Lighting change | Frame-global flag |
| Caster bounds move a cascade's depth range | `cascade.rs` L89, L145 | Lighting change | Frame-global flag, through the cascade witness |
| Sprite frame or transform, text content, tilemaps, Blend models | `graph.rs` L288–309, L340–352; `motion/retained.rs` L389 | Current-only overlay, drawn after the resolve and composited at output resolution | None: never in history, so no owner |
| Material parameters | Part of the frozen mesh asset version | No per-frame event exists | A change is a topology change |
| DRS internal scale | Resolution owner | Not a camera change | Confidence rescale (brief 2.2) |
| Output rectangle | `retained.rs` L444–458 | Camera change | `camera_static` false |
| Output size | `upscaling/owner.rs` `prepare` | No continuation across sizes: `prepare` refuses one (existing) | History is not carried across sizes |
| `invalidate_motion_history`, a motion re-enable, or a pixel-resolution camera | `retained.rs` L157–188, L451 | Reset or bypass | Fresh history (`has_previous` = 0) |

This corrects addendum 1, which listed environment and light-list changes as
frame-global triggers. In the retained path they force a replacement, and only
light transforms and cascades set the flag.

## 7. Mechanics

### 7.1 Owner identity and lifetime
- **Assignment:** ids are assigned once, in `MotionState::new`. Each placement
  gets `id = 1 + group base + placement index`.
- **Groups:**
  - static batches that share one `placements` allocation (`Arc::ptr_eq`). The
    mirrored twins do (`models.rs` L200–251);
  - animated batches that share one animation binding, which gets one id.
- **Why not the packed row:** `pack_static` packs rows densely by current
  handedness (L449). A row index therefore shifts whenever another placement
  flips, so it cannot serve as an id.
- **Range:** `1 ≤ id ≤ N ≤ MAX_INSTANCES` = 100,000 (`incant_render_data` L16).
  Rows are capped at `2 × MAX_INSTANCES`, two per id.
  - `0` means clear: no geometry.
  - `0xFFFFFFFF` is the overflow sentinel, in meta only.
  - The owner target stores `id | changed << 31`.
- **Lifetime:** the `MotionState`. A motion re-enable,
  `enable_temporal_upscaling` and a replacement each create a new one and clear
  the upscaling owner (`retained.rs` L157–176), so ids and history always
  restart together. Ids are scene-local and are never compared across owners.

### 7.2 Accepted state and change witness

**Compared against the accepted frame:**
- **Placements:** the accepted node world (`MotionState.worlds`, updated only in
  `accept`, `motion/retained.rs` L427) × `local`, against the current
  world × local, bitwise.
- **Animated bindings:** the accepted joint palette and motion world, held by
  the live (accepted) scene.
- **Camera:** the accepted unjittered view and projection, output size and
  rectangle, and source width (for the rescale).
- **Lights:** the accepted light node worlds.
- **Cascades:** the accepted cascade matrices and parameters. A copy takes at
  most 4 × 4 × 80 = 1,280 B if the live scene does not already keep them.

**Preparation:**
- It computes the changed bitset, the rectangles, `camera_static` and the
  frame-global flag into the candidate only.
- A dropped or failed candidate (late validation, allocation refusal) changes
  nothing that was accepted. The witness, history slot, meta, sequence and
  epoch, jitter phase and confidence all stay as they were.
- A retry recomputes identical values. A pending cut survives a failed
  preparation.
- Acceptance happens only after the existing immediate queue submission
  succeeds.

### 7.3 Exact integer transport

**Row, still 80 B** (`motion/retained.rs` L35):

| Bytes | Content | Location and format |
|---|---|---|
| 0–63 | Previous world | 7–10, Float32x4 |
| 64–67 | `valid` | 11, Float32 |
| 68–75 | Owner word, then a reserved 0 | 12, Uint32x2 |
| 76–79 | 0 | — |

**Which kinds read the owner:**
- **The new portable-only `MotionOwner` kind** declares location 11 as `f32`
  and location 12 as `vec2u`.
- **The existing `Motion` and `Depth` kinds** keep `@location(11) valid:vec4f`
  and read only `.x` (`motion/shader.wgsl` L18–22). MetalFX and fixed temporal
  therefore keep their pipelines. Lanes y–w now hold bits they never read.

**Path through the pipeline:**
- **Varying:** `@location(3) @interpolate(flat) owner:u32`.
- **Fragment output:** `@location(1) u32`, written to an `R32Uint` owner target
  at internal size. The target is cleared to 0, MASK discards leave 0, and
  `depth_only` is unchanged.
- **Resolve:** `textureLoad` on `texture_2d<u32>`.

No owner value passes through a float or half-float lane, and no hash is used.

### 7.4 History meta

A pair of `Rgba32Uint` textures at output size. They flip with the colour and
depth pair and live in `Output`, so they follow the output's lifetime.

| Lane | Content |
|---|---|
| x | Front owner id |
| y | Second owner id: 0 = none, `0xFFFFFFFF` = overflow |
| z | `bitcast<u32>(W)`, f32 |
| w | `bitcast<u32>(far depth)`, f32: the farthest contributor's view depth |

The existing `R32Float` history depth keeps the front depth.

**Canonical read.** Anything else means no history:
- `W` finite and ≥ 0;
- both depths finite and ≥ 0. A depth of 0 is clear: either both are 0 (no
  geometry stored), or `0 < front ≤ far`;
- `x = 0` exactly when the front depth is 0, and `y = 0` whenever `x = 0`;
- ids no greater than the frame's highest id, apart from the sentinel.

Clear history (owners 0, depths 0, `W > 0`) is valid, for example around a thin
wire over clear. All-zero meta means no history. Pixels outside the output
rectangle write zeros (addendum 1 §6).

### 7.5 Resolve layout and native binding map

The resolve uses one bind group (group 0), visible to the fragment stage. It
needs an explicit layout, because `layout_entries(count)`
(`upscaling/system.rs` L21) can only express a uniform plus float and depth
textures.

| Binding | Resource | WGSL | Metal | DX12 | Vulkan |
|---|---|---|---|---|---|
| 0 | `Frame`, 96 B, unchanged | `var<uniform>` | buffer 0 | b0 | set 0, binding 0 |
| 1 | Current colour | `texture_2d<f32>` | texture 0 | t0 | 0, 1 |
| 2 | Motion | `texture_2d<f32>` | texture 1 | t1 | 0, 2 |
| 3 | Current depth | `texture_depth_2d` | texture 2 | t2 | 0, 3 |
| 4 | History colour | `texture_2d<f32>` | texture 3 | t3 | 0, 4 |
| 5 | History front depth | `texture_2d<f32>` | texture 4 | t4 | 0, 5 |
| 6 | History meta | `texture_2d<u32>` | texture 5 | t5 | 0, 6 |
| 7 | Owner, internal size | `texture_2d<u32>` | texture 6 | t6 | 0, 7 |
| 8 | `Portable`, 896 B | `var<uniform>` | buffer 1 | b1 | 0, 8 |
| 9 | `ChangedBits`, 12,512 B | `var<uniform>` | buffer 2 | b2 | 0, 9 |

**Outputs:**

| Location | Content | Format |
|---|---|---|
| 0 | HDR colour | Rgba16Float |
| 1 | Front depth | R32Float |
| 2 | Meta | Rgba32Uint |

That is 28 B per sample, within WebGPU's default
`maxColorAttachmentBytesPerSample` of 32. The `MotionOwner` pass writes
16 (Rgba32Float motion) + 4 = 20 B.

**Why `ChangedBits` is a uniform:** a fixed-size uniform avoids runtime-sized
arrays and Metal's buffer-sizes slot. It fits Vulkan's guaranteed 16 KiB uniform
range. Bit `id` is `words[id >> 7][(id >> 5) & 3] >> (id & 31)`.

### 7.6 Uniform layouts

**`Portable` (span 896 B; every member is 16-byte aligned):**

| Offset | Member | Content |
|---|---|---|
| 0 | `inverse_view_projection: mat4x4f` | Unjittered, current camera |
| 64 | `flags: vec4u` | x: bit 0 camera static, bit 1 frame-global. y: shadowed light count (0–4). z: rectangle count per light (4 bits each, 0–8). w: highest owner id |
| 80 | `kernel: vec4f` | ρ, norm, `W_max`, reset 0.125 |
| 96 | `window: vec4f` | `R`, `R²`, 1/256, confidence rescale |
| 112 | `footprint: vec4f` | x: world size of one output px per unit view depth (perspective, else 0). y: world size of one output px (orthographic, else 0). z, w: 0 |
| 128 | `light_x: array<vec4f, 4>` | Light-orientation x row per light |
| 192 | `light_y: array<vec4f, 4>` | Light-orientation y row per light |
| 256 | `cascade_far: array<vec4f, 4>` | Per light, the far view depth of cascades 0–3 |
| 320 | `cascade_texel: array<vec4f, 4>` | Per light, `texel_world` of cascades 0–3 (`cascade.rs` L180) |
| 384 | `rects: array<vec4f, 32>` | Light `s`, rectangle `k`, at index `8s + k`: min x, min y, max x, max y |

**`ChangedBits` (12,512 B):** `words: array<vec4u, 782>`. That is 3,128 words,
covering ids below 100,096.

**`history_weight`:** keeps the upscaling meaning given in brief 2.2, through
`W_max = h / (1 − h) · min(r², 4)`. This is documented, not renamed.

### 7.7 Cooker contract and emitter

**`contract.rs`, `Kind::Upscale`:**
- The resource count goes from 6 (L233) to 10.
- `Frame` member names stay the same.
- New exact checks for `Portable` (names, offsets, types, span 896) and
  `ChangedBits` (one member, `array<vec4u, 782>`, span 12,512).
- Texture classes:
  - bindings 1, 2, 4 and 5: sampled float;
  - binding 3: depth;
  - bindings 6 and 7: sampled uint.
- Outputs: `[Float(0,4), Float(1,1), Uint(2,4)]`.
- `Io` gains `Uint(location, lanes, flat)`, and integer IO must be flat. Today
  `io` accepts only float IO (L57).

**New `Kind::MotionOwner`:**
- three entry points, as for `Motion`;
- vertex inputs 0–10 as today, then 11 `Float(1)` and 12 `Uint(2)`;
- varyings as today, plus `Uint(3, 1, flat)`;
- fragment outputs `[Float(0,4), Uint(1,1)]`;
- `depth_only` has no output.

**`emit.rs`:**
- **Upscale on Metal:** an explicit per-binding map:
  - 0 → buffer 0;
  - 8 → buffer 1;
  - 9 → buffer 2;
  - 1–7 → texture (b − 1);
  - anything else fails.

  Today every non-motion uniform maps to buffer 0 (L92), so binding 8 or 9 would
  collide with binding 0.
- **`MotionOwner` vertex pulling:** buffer 12, stride 80, attributes as in 7.3.
- **HLSL and SPIR-V:** maps as in the 7.5 table. They are asserted against the
  pinned wgpu-hal layout rules: Metal compacts per stage, and DX12 counts the
  full declared layout (`emit.rs` L1–2).

**`cooked/host.rs`:**
- trusted source identities for the new portable sources;
- complete-cooked selection fails closed if any new kind is missing;
- default and web selection keep failing closed.

**Negative controls, each refused independently of the source hash:**
- bindings 8 and 9 swapped;
- binding 6 or 7 declared as `f32`;
- the meta output declared as float;
- the owner varying not flat, or location 12 missing;
- a changed `Portable` member offset or span;
- `ChangedBits` with length 781;
- an extra binding;
- a stale assembled source.

### 7.8 No lights and empty model scenes
- **Light count 0, or no changed casters:** there are no rectangles. The
  footprint design binds no lighting group, so it needs no fallback group.
- **No models:** the motion pass draws nothing, the owner target stays 0, and
  every pixel is clear. The resolve must still run, so that colour, depth and
  meta are replaced. The audit notes that `prepare_models` returns `None` for
  empty model scenes. A test must show that a frame with no models or lights
  clears history.
- **Under the signature alternative only:** a renderer-owned fallback group,
  created once at enable time:
  - a one-layer Depth32Float array;
  - a comparison sampler;
  - a zeroed `ShadowData` (1,280 B).

### 7.9 Worst-case work per output pixel

No timing claim is made.

| Item | 535003d7 | Persisting pixel | Moving pixel |
|---|---|---|---|
| Current colour, depth and owner loads | 13 + 17 + 0 | 9 + 9 + 9 | 9 + 9 + 9 |
| Motion loads | 1 | 1 | Up to 9 |
| History loads | 8 | 3 (colour, depth, meta) | 12 (bilinear colour, depth and meta) |
| Changed-bit reads | 0 | 2 (stored owners; current owners carry bit 31) | 0 |
| Kernel exponentials | 0 | 9 | 9 |
| Footprint work | 0 | None without rectangles; otherwise 2 reconstructions, 16 dot products and up to 32 box tests | 0 |
| Shadow map reads | 0 | 0 | 0 |
| Bytes written | 12 | 28 | 28 |

**Motion pass:** it writes 4 extra bytes per internal pixel.

**Not measured:** the mobile cost of dynamic indexing into a uniform array,
and the bandwidth of the `Rgba32Uint` meta (16 B read and 16 B written per
output pixel). Both are listed for device profiling.

### 7.10 Exact accounting (portable path)

| Term | Bytes |
|---|---|
| Output, per set: 16 colour + 8 depth + 8 composite + 32 meta = 64 B/px | |
| Internal, per set: 8 overlay + 24 motion + 4 owner = 36 B/px | |
| Pixels, old plus candidate: 2 × 100 B/px × `MAX_MOTION_PIXELS` (4,194,304) | 838,860,800 |
| Motion instance payloads, rows unchanged: 2 × 67,108,864 | 134,217,728 |
| `Portable` uniforms: 2 × 896 | 1,792 |
| `ChangedBits` uniforms: 2 × 12,512 | 25,024 |
| Existing fixed metadata allowance | 16,384 |
| **Total** | **973,121,728** |

**Compared with:**
- **Today:** 671,105,024 B (`upscaling/plan.rs` L5–6), so +302,016,704 B.
- **Addendum 1:** 973,147,152 B, so −25,424 B. That figure had per-row changed
  sets and a 1,120 B uniform.
- **MetalFX and fixed temporal:** they allocate none of this.

**CPU logical bytes, charged with the motion plan:**
- one 4 B group base per batch;
- the 12,512 B bitset staging and the 896 B uniform staging, reused every frame;
- the cascade witness, if one is needed (1,280 B).

**How to encode it.** Astra should derive the constant from these named terms
and assert both totals.

## 8. Numerical record

**Root's oracle (2fe23dce, accepted as test-only).** Its values are
definition-specific observations, not targets:
- **Interior mass at numerator 7:** 7.927397–8.072924.
- **Contrast at numerator 8 (lines / checker 2 / checker 4):** 0.3825 / 0.4720 /
  0.7122. Its bilinear control gives ≈0 / 0.0973 / 0.4305.
- **4/16 → 16/16:** 0.621 / 0.718 / 0.868 by the third frame; 0.886 / 0.900 /
  0.946 by the fourth.
- **12/16 → 4/16:** 94.25–96.78% retained.
- **Largest omitted individual tap:** 1.956e−5 of the peak on the tested
  interior lattice, and 1.027e−4 for unrestricted alignment.

**Corrections:**

| Claim | Status |
|---|---|
| Brief 2.1: "8.00 ± 0.07 kernel mass per cycle" | Withdrawn. The oracle reaches 0.0729 from 8. The [7.9, 8.1] bound holds on interior pixels only (model C2: 7.9274–8.0729). Clipped and fractional edges are unbounded (C21 edge 0.0018) |
| Brief 2.1: "any sample outside the window weighs < 10⁻⁴ of the kernel peak" | True only for the largest individual omitted tap on the tested interior lattice (1.956e−5). Unrestricted fractional alignment gives 1.027e−4. It is not a bound on total omitted mass |
| Addendum 1 §2: "taps beyond R carry at most e⁻⁸ ≈ 3.4·10⁻⁴ of the peak weight" | Holds per window tap. Up to 8 window taps can lie beyond `R`, so untracked weight is ≤ 8e⁻⁸ ≈ 2.7·10⁻³ of the peak. That bounds kernel weight, not colour error |
| Brief C3: "activity at 8/16 no higher than 535003d7's" | Not established. In the oracle, checker 4 rises (0.020983 → 0.021598, linear) and lines fall (0.027923 → 0.005050). This model's retained C3 is higher for every pattern than its own 535003d7 replica, in sRGB codes: lines 1.05 vs 0.00, checker 2 2.55 vs 1.69, checker 4 2.33 vs 2.11 |
| Brief C9: "≥ 75% by the third 16/16 frame, ≥ 90% by the fourth" | Recorded as failed under both references. The model gives 0.683 / 0.869 against the converged state and 0.741 / 0.943 against frame 39. It is kept as an expected failure in `results-2.json` |
| Addendum 1's C9 replacement, "≥ 0.65 / ≥ 0.85" | Withdrawn as a target. The oracle's lines reach 0.621 by the third frame. The model's 0.683 / 0.869 is one definition's observation. Pending root's numerical review |
| Brief C7: "≤ 0.06 per column" | Recorded as failed (0.0918) under these rules too. Addendum 1's C7a, C7b and C7d are replacement controls pending root's review |
| Brief C3 regression reference 0.00 / 0.05 / 0.42 | It comes from the model's replica of 535003d7 (C0: 0.000 / 0.050 / 0.423, against 0.032 / 0.061 / 0.434 measured in a5ae3179). That replica is not the oracle's control, and the oracle does not reproduce checker 2's 0.05 |

**What none of these do:**
- change a GPU gate;
- let a CPU model mark visual acceptance as passed;
- turn a replacement control into a gate before root's numerical review.

## 9. Tests for Astra

### 9.1 Mirroring the model

These go in a test-only admission oracle beside the accepted kernel oracle,
followed by GPU raw cases on the `device_tests.rs` pattern:
- **N1:** the footprint rule passes and the scalar key fails.
- **N2 and N3:** no persisted pixel and no footprint test while the camera
  changes.
- **F10 and N4–N6:** as in 5.2.
- **K3:** footprint lag ≤ 0.03.
- **The retained C and F cases under the new rules:** F1 and F2 still fail; C7
  and C9 as written stay recorded failures.

### 9.2 The readiness audit's controls, as integration tests
- **Moving foreground:** over a static floor and over clear, with no lights.
  Move, start, stop, remove, hide, deform, and replace with repeated entity ids.
  Fresh-history and never-occluded references stay separate.
- **Owners:**
  - a third owner entering and leaving;
  - owner overflow;
  - provenance through moving reprojection;
  - a replacement resets even when dense slots, geometry, tick and size match.
- **Camera:** lateral pan, orbit about the receiver, axial dolly and projection
  change. The camera stops at each of the 8 phases. Include zero-vector pixels
  under a changed camera, and output-edge history rejection.
- **Partial and malformed data:**
  - premultiplied partial coverage over black and over clear;
  - malformed alpha-zero RGB in current, history and neighbours;
  - finite overflow, negative or NaN confidence, and malformed meta;
  - depth zero is never an owner.
- **Viewports:** clipped fractional output and source rectangles at every scale,
  true raster sample centres, no clamped duplicate taps.
- **Lights and shadows:**
  - the two-light swap;
  - shadow-only caster changes with constant receiver and light ids;
  - light transforms and unshadowed paths;
  - a caster bound that moves a cascade's depth range.
- **Failure paths:**
  - prepare and drop, a failed late validation, and a resize allocation refusal
    and retry all leave the accepted state unchanged;
  - a fresh retry equals a clean run;
  - a cut survives a failed preparation;
  - portable and native mode replacement reset together.
- **Scale changes:** 4 → 16 and 12 → 4 on static and on moving or occluding
  scenes, with no double rescale on retries. The meta follows the output's
  lifetime while the internal extent changes.

### 9.3 Mechanics
- **Meta:** encode and decode round trip, canonicalisation and sentinels.
- **Owner transport:** ids 1, 65,537 and 100,000, with and without the changed
  bit, come through the motion pass and the resolve bit-exact.
- **Handedness flips:** keep ids.
- **Bitset:** bit order.
- **Rectangle merging:** deterministic.
- **`camera_static`:** bit-exact, including the output rectangle.
- **Frame-global flag:** set on a cascade change.
- **No models or lights:** the resolve clears history.
- **Accounting:** derived from named terms.
- **Cooker:** the negative controls in 7.7.
- **Other paths:** MetalFX and fixed temporal contracts and pipelines unchanged.

## 10. The model and its limits

**Run:**

```
python3 -I -B handoffs/0073-upscaling-review/repair-model/model_v2.py \
    --out handoffs/0073-upscaling-review/repair-model/results-2.json
```

- Standard library only. It imports the unchanged `model.py`.
- About 50 s of CPU, run in the foreground.
- Two runs give byte-identical output. Python 3.9.6 and 3.14.8 agree apart
  from the version field.
- The exit status is nonzero if any case departs from its expectation.

**What it does not model:**
- **Geometry:** it is 2D, with no world points, light space, parallax, cascades
  or normal offset. Footprints are a swept 1D band for the shadow scene, and a
  hit on every receiver at the swap frame for N1. The slab test, cascade choice
  and A1–A5 hold by argument, not by this model.
- **Unexercised rules:** the model applies the footprint test on the persist
  branch only. Overflowed pixels that clip in place (5.1 step 3) and the far
  depth are specified here, but no modelled case exercises them.
- **Camera:** the orbit is a view-dependent factor at zero motion. The dolly's
  wrong world point is not modelled, so N3 is structural.
- **P1:** a single crossing geometry.
- **Rendering features:** MASK, HDR, overlays and half floats. It checks rules,
  not GPU numerics.

**Remaining limits of the design:**
- over-invalidation inside swept footprints (K3), and merged rectangles when
  many casters move;
- identity at exact depth ties follows row-major order;
- provenance has no measured benefit;
- untracked weight beyond `R` is ≤ 2.7·10⁻³ of the peak;
- exactness rests on A1–A5;
- animated owners stay changed every frame if palettes are not compared;
- any camera change, even a sub-pixel drift, disables persistence for that
  frame;
- uniform-array indexing and meta bandwidth on mobile GPUs are unmeasured.

**Open questions for root:**
1. The footprint rule (exact, over-invalidating), or the per-light signature
   with gaps (a)–(c)?
2. Keep provenance?
3. Numerical review of the replacement controls (C7a, C7b, C7d, and the C9
   definition).
4. Palette comparison for animated owners, or always changed?

No appearance, performance, phone or phase-gate claim is made.
