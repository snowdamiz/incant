# 0031 boundaries: the bounded v1 placement contract

Consultation, 2026-10-10. This note answers the "final placement-contract boundaries"
packet and covers the visible-motion and light/shadow contract for v1, before Astra
writes source or schema. It is not an implementation, a schema, rendered acceptance or a
gate decision. Astra owns the numerical implementation, the declared range and the tests.

## Status

- **Recommendation: one marker per hierarchy branch, with the rendering camera as the
  only other reference.** No marked entity may have a marked ancestor, and cameras do not
  carry the marker. For each pixel-perfect camera, a marked root is in one of three roles:
  - **free:** the camera is outside the root's ancestor chain and subtree. The root uses
    the world grid.
  - **camera-attached:** the camera is an ancestor of the root. The root rounds its local
    offset from the camera's anchor.
  - **governing:** the camera is in the root's subtree. The root uses the world grid, and
    the camera is *governed*: its anchor is the root's placed origin plus the camera's own
    rounded local offset.

  Nesting and camera markers are rejected with typed errors wherever the hierarchy
  changes. Lighting stays projection-only.
- **All three corrections are accepted.** My arithmetic reproduces each one (see the next
  section). Unrestricted nearest-marked-ancestor rounding is deferred for v1, not
  reworded.
- **One further correction in the same direction.** Even without nesting, a
  camera-attached root sits up to 1 px from its continuous pose, not ½ px: the camera's
  own snap and the offset's rounding each contribute ½ px. The earlier notes did not
  state this.
  - The ≤ 1 px contact-shadow bound still holds.
  - The ½ px culling growth does not.
- **Margin attribution corrected.** The ½ px between a whole-px edge and the nearest
  sample point is arithmetic and holds on every adapter. Only the raster precision that
  must fit inside it is adapter-specific. Apple M5 Pro (Metal) is the only measured
  adapter, with a band near 1/512 px. The "One adapter" limitation in origin-contract.md
  conflated the two.
- **Scope kept.**
  - New: this file. Appended: one link section at the end of
    [origin-contract-clarification.md](origin-contract-clarification.md).
  - Nothing else changed: not `result.md`, evidence, helpers or images. The runner prompt
    asked for `result.md`; the packet's writable scope excludes it. This note therefore
    carries the result fields. `result.md` already links origin-contract.md, which links
    the clarification, which now links here.
  - No Cargo, build, capture, screenshot, browser or GPU tool ran.
  - No phase gate is approved or claimed. No visual, shipping or phone-performance
    acceptance is claimed.

## Model, effort and transport

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, per this session's system
  context.
- **Effort:** Max is required by the packet. `tools/acp/config.json` pins
  `claude-opus-5-5` with effort `max`.
- **Verification:** the packet says the runner sets and verifies both before prompting. I
  see no runner output, ACP handshake or effort readout in this session, so I cannot
  confirm that independently. I substituted nothing.
- **Transport:** the runner started this session in the handoff worktree through the
  Claude agent harness.

## The corrections, checked

I emulated the tie rule and f64 band from `crates/incant_render/src/camera.rs` in a
scratch `python3 -I` stdin run. It wrote no file, ran no engine and is not a test.

1. **Nested rounding accumulates.** PPU 1, right axis, each level at local +0.49 px:
   - placed origins: 0, 0, 0;
   - continuous origins: 0.49, 0.98, 1.47;
   - displacement: −0.49, −0.98, −1.47, reaching −3.92 at eight levels.

   Each independently rounded level adds up to ½ px. A whole-px level adds 0.
2. **Anchor and eye differ at odd sizes.** Width 255, camera at 10.2 px, so the anchor is
   10 and the eye is 10.5. Take a root attached at +3 px:
   - from the anchor, its origin is 13 and lands at image x 130.0, a pixel corner;
   - from the eye, its origin is 13.5 and lands at 130.5, a pixel centre.
3. **Governance moves the whole image.** Root at 0.49 px, camera at local +0.49 px:
   - the ungoverned anchor is snap(0.98) = 1;
   - the governed anchor is round(0.49) + round(0.49) = 0.

   Over a 0.01 px lattice, the governed minus the ungoverned anchor took only the values
   −1, 0 and +1. With a whole-px camera offset it was 0 at every lattice point.
4. **Camera-attached bound.** anchor + round(offset) − continuous lies in [−1, +1) on the
   right axis and (−1, +1] on the up axis. A camera at 0.5 px with an offset of 3.5 px
   gives exactly −1: placed 3, continuous 4.

## Recommended v1 contract

### 1. Marker scope, hierarchy restriction and unsupported cases

**The marker** is unchanged from the earlier notes:
- It is opt-in typed component data, authored through `incant_cmd` and reversible.
- It is cooked into the runtime scene as immutable typed data. Its name and schema are
  Astra's.
- It never makes anything drawable.
- It never moves lights, colliders, physics, audio, navigation, scripts, or authored and
  simulated transforms.

**Ownership.** A marked entity is a placement root. It governs its own drawables and every
descendant drawable, rigidly. Under the restriction every descendant is unmarked, so each
drawable has at most one root.

**Restriction.** At most one marker on any root-to-leaf path, and no marker on an entity
with a Camera component. A marked rig that carries the camera expresses what a marked
camera would, under one rule.

**Roles per pixel-perfect camera K and frame:**

| Role of marked root M | Condition | Placed origin |
| --- | --- | --- |
| Free | K is neither an ancestor nor a descendant of M | M's origin rounded to K's world grid |
| Camera-attached | K is an ancestor of M | K's anchor + round(M's local-chain offset from K) |
| Governing | K is a descendant of M | M's origin rounded to K's world grid; K is governed |

- **Governed camera.** Its anchor is the governing root's placed origin plus round(K's
  local-chain offset from that root). Its eye is that anchor plus the usual odd-size
  phase, at K's continuous forward depth.
- **The roles are exclusive.**
  - A camera-attached root's camera is never governed: its governing root would be the
    attached root's marked ancestor, which the restriction forbids.
  - In a governed camera's frames, every other marked root is free.
- **Rounding** everywhere is the camera's own function at `72ede59`: per axis, in logical
  px, after the existing f64 band. Right-axis ties go toward −right and up-axis ties
  toward +up.
- **Local-chain offsets.** These come only from the f64 product of the transforms between
  the two entities, expressed in K's right and up axes.
  - They never come from the difference of two absolute world positions, or from
    re-snapping an absolute position.
  - An exact-half offset therefore resolves the same way in every frame, at any
    translation.
  - Clarification rule 3 had the camera "move rigidly with that root's placement offset,
    then apply its own snap". That gives the same value in exact arithmetic, but it
    re-snaps an absolute position. This computation replaces it.
- **Marker-blind views.** Perspective and ordinary orthographic cameras ignore markers, and
  so does the editor preview. So do pixel-perfect cameras other than K in K's frames,
  unless the rule above names them.

**Unsupported cases, made observable.** v1 rejects each of these with a typed error that
names both entities, and applies nothing:
- adding the marker to an entity that has a marked ancestor or a marked descendant;
- adding the marker to a Camera entity, or adding a Camera to a marked entity;
- reparenting, duplicating, pasting or instantiating a template so that a marked entity
  gains a marked ancestor;
- the same structural changes made at runtime, rejected on the runtime's structural
  command path and reported to the issuing script;
- cooking or loading a scene that contains either case, because project content is
  untrusted.

A rejected command leaves the project and journal bytes unchanged. If the renderer still
meets nesting, the pixel-perfect frame fails with a typed `SceneError`. It never picks
one marker, sums offsets or ignores one. How the editor shows these errors belongs to
0033 or later.

**Deferred, not supported in v1:**
- independently snapping a crate that rides a marked platform;
- marked parts of a marked character;
- marked followers under a governed camera;
- any second marker on a path.

A game that picks up a marked object onto a marked carrier must remove one marker in the
same structural batch.

### 2. What holds still and what steps

| Content | Behaviour | Strength |
| --- | --- | --- |
| Unmarked, no marked ancestor | Today's behaviour. Static content is identical while K's anchor holds and steps with it. Camera-locked content may jump 1 px near camera ties, inside the adapter's raster band | Static exact; moving none |
| Free root, static, any origin including half-grid | One placement at every camera position; it steps only with the world | Exact |
| Free root, moving | Whole-px steps on K's world grid, never against static content | Exact |
| Free root moved by script beside the camera, or free roots moved together without a shared marked ancestor | Holds its screen position or spacing, except inside the f64 near-tie window that Astra declares, where it can jump 1 px for one frame | Exact to the rule; no stillness promise |
| Camera-attached root, constant local offset | Holds round(offset) from the anchor in every frame, for any camera motion and translation in the declared range | Exact; ½ px margin |
| Camera-attached root, changing local offset | Moves on screen in whole px | Exact to the rule |
| Camera-attached root kept still in the world while the camera moves | Steps up to 1 px against static content | Documented |
| Governing root, constant camera offset | Holds −round(offset) on screen and stays on the world grid; the world scrolls in whole px | Exact |
| Governing root, changing camera offset (shake, look-ahead, dolly) | Never steps against the world; the view steps in whole px | Exact |
| Unmarked descendants of any root | Rigid with the root; their own local animation stays continuous | Exact relative to the root |
| Unmarked children of a governed camera | Rigid with the governing root, so on screen they carry the camera's residual, offset − round(offset). They are still and whole only when that offset is constant and whole | Exact to the rule; otherwise reported |
| Constant local shape, any constant rotation, scale or pivot | The footprint is a whole-px translation of one pattern | Exact for edges on whole px. Adapter-dependent for edges within the raster band of a sample point |
| Rotating, scaling, deforming or animated local geometry | Continuous about the placed origin | No new stability promise |

**Authoring guidance:**
- To hold a subject still under a following camera, parent the camera under the marked
  subject at a whole-px offset.
- To hold content still in front of a script-driven camera, mark it under that camera, or
  mark one rig that carries both the camera and the content.

### 3. Maximum render-versus-continuous displacement

Displacement is the rendered origin minus the continuous origin, along K's right and up
axes, in logical px.
- It is 0 in every case for forward depth, rotation, scale, pivots and local geometry.
- Every drawable takes its root's displacement.
- All intervals widen by the existing tie band of at most 1e-7 px.

| Case | Right axis | Up axis | Notes |
| --- | --- | --- | --- |
| Ungoverned camera anchor | [−½, +½) | (−½, +½] | Today's snap |
| Free root | [−½, +½) | (−½, +½] | |
| Governing root | [−½, +½) | (−½, +½] | |
| Camera-attached root | [−1, +1) | (−1, +1] | ±1 only when the camera and the offset both sit exactly on ties. The screen offset from the anchor is exactly round(offset) |
| Governed camera anchor, against its continuous position | [−1, +1) | (−1, +1] | |
| Governed camera anchor, against its ungoverned anchor (Δ) | −1, 0 or +1 | −1, 0 or +1 | Δ = 0 for a whole-px camera offset, except where the ungoverned camera falls in the f64 window |
| Unmarked content without a marked ancestor | 0 | 0 | It renders at its continuous position relative to whichever anchor K uses |

**Consequences:**
- **Culling** uses placed bounds, or continuous bounds grown by ½ px for free and governing
  roots and by 1 px for camera-attached roots. A governed camera culls against its
  governed view. This replaces origin-contract §7's flat ½ px.
- **The 1 px ceiling.** No v1 case exceeds 1 px. Nested roots would add up to ½ px per
  level, which is why v1 rejects nesting instead of bounding its depth.

**Why these tradeoffs are acceptable for v1:**
- Every supported case holds a 1 px ceiling, and stillness becomes exact for the two
  natural follow structures: camera under subject, and content under camera.
- Nothing is inferred.
- The costs are visible and opt-in:
  - restructured hierarchies instead of nesting;
  - camera-attached shadows that shimmer (section 4);
  - a governed camera's whole-image shift, which follows directly from marking the
    camera's ancestor.

### 4. Light, torch and contact shadow

**Projection-only placement.**
- **What changes:** only the raster translation of governed drawables in K's colour and
  depth passes, and, for a governed K, its effective view.
- **What stays continuous:**
  - shading positions and normals;
  - self-shadowing, cast shadows and shadow casters;
  - every light, including torches under marked roots;
  - environment lighting;
  - clustered light assignment and lookup.
- **Cluster lookup.** For placed fragments, the lookup must project the continuous world
  position into the light grid. Today `lighting/shade.wgsl:50` derives the tile from the
  fragment pixel. A placed fragment can land in a tile whose list misses a light that
  reaches its continuous position.
- **Raster translation.** The vertex stage (`model_material.wgsl:32`) derives both raster
  and lighting positions from one world position. Placement therefore needs a separate
  raster translation per placed instance and camera. That is per-instance data, not
  per-vertex work.
- **The effective view** is anchor plus phase. It drives projection, the cluster grid and
  cascade fitting, as it does today.
  - For an ungoverned K nothing changes.
  - For a governed K, all three follow the governed view. `shadows/cascade.rs` fits from
    `camera.view`. It snaps x and y to texels, but its depth range follows the eye. A
    governed frame's shadows can therefore differ from the markers-removed frame
    anywhere, beyond the whole-px scroll.

**Visible consequences:**
- **Light and shadow offset.** A drawn object is at most ½ px from its continuous light and
  shadow pose when free or governing, and at most 1 px when camera-attached. A torch light
  shines from its continuous position, at most that far from the drawn torch.
- **Contact gap.** Against the markers-removed run, the rendered contact gap changes by at
  most 1 px per axis. A pixel-corner edge moved by at most 1 px crosses at most one
  sample point.
- **Camera-attached followers.** The follower holds still, but its continuous cast shadow
  and torch light move with the camera's position inside its cell.
  - Soft shadow edges shimmer in a sawtooth, with one period per pixel of camera travel.
  - Hard edges can still jump at camera ties, exactly as an unmarked follower's do today.
  - This is recorded and reviewed, not gated. v1 makes no stability promise for shadows
    or lights.
- **Governing roots.** A player carrying the camera stays within ½ px of its shadow.

**Comparisons:**
- **Ungoverned K: locality against the markers-removed run.** Outside the union of every
  governed drawable's placed and continuous footprints, frames are byte-equal. This is
  valid because the camera, lights, shadow maps and cluster grid are unchanged.
  - Today's passes after rasterization are per-pixel: tone mapping and the
    `textureLoad` replication in `output.wgsl`.
  - A future effect that spreads pixels must grow the footprints by its radius.
- **Governed K: split by Δ per frame.**
  - **Δ = 0:** the same locality test against the markers-removed run. This requires
    Astra to build the governed eye with the existing formula (`camera.rs:84`),
    replacing only the snapped integer.
  - **Δ ≠ 0:** the frame must differ from the markers-removed frame, because the
    background moves by Δ.
    - Compare it instead with a **pinned twin**: markers removed, rendered through an
      unparented pixel-perfect camera with the same projection, rotation and forward
      depth.
    - The twin is authored at the predicted anchor. That anchor lies on the grid, so the
      twin's own snap returns it.
    - Outside the footprints, the frames must be byte-equal.
    - Precondition: Astra's numeric test shows the twin's view, cluster grid and cascade
      matrices bit-identical to the governed ones for each acceptance configuration. If
      they differ, the case fails; it does not gain a pixel tolerance.

### 5. Odd-size anchor and default-view compatibility

**Definitions:**
- **Anchor:** K's snapped camera-plane position. It is a whole number of logical px on
  K's world grid, before the odd-size phase. In `camera.rs` it is the per-axis `snapped`
  value at lines 70–74. Earlier notes called it the camera cell.
- **Eye:** the anchor plus the phase, at K's continuous forward depth. The phase is +½
  right for odd widths and −½ up for odd heights (`camera.rs:75–84`).
- **Placement never reads the eye.** It reads only anchors and grid points, and the phase
  is added once, at projection. Every placed origin therefore lands on a pixel corner, at
  odd and even sizes alike.
- **Governed eye:** the governed anchor plus the same phase.
- **Camera data.** `CameraView` must carry the integer anchor and the f64 right/up basis.
  Rebuilding the anchor from the f32 eye would bring back both the phase and f32 error.

**Compatibility:**
- **No markers:** frames are byte-identical to the current build (A0), and the ungoverned
  eye stays bit-identical.
- **Other views:** perspective cameras, ordinary orthographic cameras and the editor
  preview render byte-identical frames with or without markers.
- **Presentation:** odd-size framing, whole-number replication, centred letterboxing and
  the explicit too-small failure are unchanged.
- **Ungoverned pixel camera with markers:** changes stay inside governed footprints.
- **Governed camera:** this is the only marker-induced change outside footprints. The
  whole image shifts by Δ, −1, 0 or +1 px per axis, with lighting refit. It is
  intentional, and opt-in by marking the camera's ancestor.

### 6. Acceptance changes

Rendered cases need a new frozen binary and my later pixel review. Numeric cases are
Astra's.

**Revised:**
- **A1:**
  - Camera-attached capture: as before. Marked A, B and O sit under the ungoverned
    camera, and O holds (20, 10).
  - Marked-rig capture: A, B and O sit unmarked under the governing rig.
    - A and B are gated at home pixels.
    - O is reported, not gated, because its x edges sit on sample points.
  - Free twins match world-grid predictions, and window frames are reported.
  - Governed frames use the comparisons in section 4.
- **A4:** S4, the marked child under S0, moves to A14 as a rejection.
- **A5:** only the root is marked.
  - The shield and charm become unmarked children. The shield is gated as rigid; the
    bobbing charm moves continuously relative to the root and is reported.
  - Marking either one is an A14 rejection.
- **A12:** split into two parts.
  - **A12a, ungoverned.** Free and camera-attached marked lit boxes, each carrying a point
    light. Gates:
    - locality against the markers-removed run;
    - per-frame displacement inside section 3;
    - rendered contact change of at most 1 px.

    A visual verdict covers the attached box's shadow shimmer.
  - **A12b, governed.** A marked lit rig carries the camera at a whole-px offset, then at
    (+0.25, −0.25) px. Frames are compared split by Δ, and the cascade and cluster refit
    is reviewed.
- **A13:** a governed camera under a marked player, at a whole-px offset, then at
  (+0.25, −0.25) px.
  - The player holds constant pixels.
  - Unmarked followers under the player hold.
  - Unmarked children of the camera hold at the whole-px offset and are reported at the
    other.
  - Under scripted camera-local shake, the player never steps against the world and the
    view steps in whole px. The camera's children report the residual.
  - Marked followers under the camera, and a marked child under a moving marked parent,
    move to A14.
- **A3 and A8:** add the governed configuration. A8 also renders one scene through a
  governed and an ungoverned camera. Governance changes only the governed camera's frames.
- **A11:** placed footprints apply in governed views too.
- **Unchanged:** A0, A2, A6, A7, A9 and A10.

**Added:**
- **A14. Nesting and camera-marker rejection.**
  - Inputs: every authoring path in section 1, one runtime structural change and one
    crafted cooked scene.
  - Pass:
    - a typed error naming both entities;
    - no partial change;
    - unchanged project and journal bytes;
    - no frame rendered with a silently chosen placement.
  - Correction 1's three-level chain is rejected when it is marked at more than one level.
    With only its root marked, all three origins share the root's −0.49 px.
- **A15. Odd-size anchor.**
  - Setup: 255×143, 256×143 and 255×144, with camera-attached and governing roots at
    whole-px offsets.
  - Pass: their footprint edges are integer pixel boundaries.
  - A prediction built from the eye must disagree by ½ px, so the test can tell the two
    apart.
- **A16. Displacement extremes.**
  - Astra's numeric tests sweep camera and offset lattices, including exact ties, both
    signs and the edge of the declared range.
  - Pass: the maxima match section 3, including the ±1 px double ties.
  - Rendered spot checks at those frames confirm the footprints.
- **A17. Governed comparisons.**
  - Correction 3's configuration, a root at 0.49 px with the camera at local +0.49 px,
    must equal its pinned twin and differ from the markers-removed frame by a 1 px
    background shift.
  - A whole-px camera offset equals the markers-removed run, except in the predicted
    f64-window frames.

## Options not chosen

| Option | Why not in v1 |
| --- | --- |
| Unrestricted nearest-marked-ancestor rounding (earlier notes) | Correction 1: displacement grows by up to ½ px per level, which breaks the displacement, contact and culling bounds |
| A bounded nesting depth | Honest only with depth-dependent displacement, contact and culling bounds and shifted lighting. Riders and marked parts can wait for that work |
| An explicit world/camera reference field, without governance | It keeps the camera unchanged and covers script-only followers. But the field can contradict the hierarchy, and a camera-referenced subject steps against the world whenever its offset from the camera changes: eased, shaking or look-ahead cameras |
| Inverse attachment: a root that carries the camera is placed relative to it, with no governance | It keeps the camera unchanged, but a still root twitches against the world while its camera's local offset changes. A root at 0.3 px, with that offset easing from 0 to 1 px, is drawn at 1 for offsets in (0.2, 0.5] and at 0 otherwise |
| Children of a governed camera follow the camera rather than the root | It would keep camera children whole at fractional or shaking offsets, but adds a second ownership rule. Deferred |

Governance makes the camera absorb the second ½ px instead of the subject. The subject
stays within ½ px of its continuous pose and never steps against the world. The price is
the Δ-split comparison.

## Superseded claims in the earlier notes

[origin-contract.md](origin-contract.md):
- §2 "A marked descendant is its own placement root…", the nearest-wins rationale and the
  two marked-child rows of §4: v1 rejects marked descendants.
- §3 rule 4, "at most ½ px per axis": replaced by the per-role table in section 3.
- §5 "at most ½ px from the placed torch", "can sit up to ½ px off" and the unconditional
  locality sentence: replaced by section 4.
- §7 "bounds grown by ½ px": replaced by section 3.
- Limitations, "One adapter": the ½ px margin is arithmetic. Only the raster precision it
  must contain is adapter-specific, and only one adapter is measured.

[origin-contract-clarification.md](origin-contract-clarification.md):
- "Placed ancestor … marked or … the rendering camera": in v1 the rendering camera is the
  only possible placed ancestor.
- Rule 2, "The rendering camera's placed origin is its snapped position": read the
  unphased anchor.
- Rule 3: its computation is replaced by section 1, and "A marked camera governs its
  unmarked descendants" is withdrawn.
- Rule 5's list of unchanged items: still true for A0, but locality is conditional under
  governance.
- Table rows about marked ancestors other than the camera, and A1, A4, A5 and A13:
  revised as in section 6.

## Data, cost and range

- **Per pixel-perfect camera and frame:**
  - one rounding per root;
  - one anchor per governed camera;
  - one f64 local-chain product per attached root or governed camera.

  The result is one camera-plane translation per root, applied per instance in that
  camera's raster path. It stays immutable with the frame. The shared resolved scene and
  instance buffers are never mutated.
- **Work** is bounded by roots times hierarchy depth, plus governed instances. There is no
  per-vertex CPU work, and no per-tick document processing, project cloning or JSON.
  Root membership and nesting validity are maintained where the hierarchy changes, not
  rediscovered every tick.
- **Runtime path.** Placement belongs on the runtime path, over cooked typed markers and
  parent links.
  - This branch has no cooked runtime hierarchy: `scene.rs:103` resolves the document
    projection, and `incant_cmd` carries `ReparentEntity`.
  - Placement must not be added only to the document-resolved play path.
- **Declared range.** This is Astra's to set. Inside it:
  - free-root and anchor rounding are exact to the rule in f64;
  - the f32 raster error of placed instances stays a small fraction of the ½ px margin.

  At 1,000 m, the f32 spacing is 1/512 px at 32 px/m but ¼ px at 4,096 px/m. Placed
  instances therefore still need a camera-relative conversion.
- **Deformed and animated content** keeps continuous local geometry, with no new
  simulation or texture-stability guarantee. Sprite UV and pivot sampling stay with 0032.

## Limitations

- **Analysis only.** No frames, captures or measurements came from this session.
- **One adapter.** Raster precision is measured on one adapter. Portability of the ½ px
  margin needs a second rasterizer, which I am not requesting here.
- **Effort not independently verifiable;** see "Model, effort and transport".
- **No runtime hierarchy inspected.** I found no cooked runtime hierarchy to inspect. The
  data requirements in "Data, cost and range" are stated, not checked against one.
- **Script-only followers** stay outside the stillness guarantee.
- **Governed-camera children** keep a residual at non-whole camera offsets.
- **Earlier gaps** carry forward: native chrome masking, editor picking and gizmos in
  pixel-perfect views, and world-space particles.

## Open questions

These are for later. None blocks v1, and none asks for a routine implementation choice.

1. Shadows and lights shifted with placement, if A12a's attached-shadow shimmer or A12b's
   refit proves visually unacceptable. That needs its own review scene.
2. Nested roots, once cumulative bounds and shifted lighting exist.
3. A camera that tracks a placed target without parenting.
4. Inspector presentation of roles and rejection errors, in 0033 or later.

## Changed paths and commands

- **New:** `handoffs/0031-pixel-perfect-cameras/origin-contract-boundaries.md`.
- **Appended:** a link section at the end of
  `handoffs/0031-pixel-perfect-cameras/origin-contract-clarification.md`. No earlier text
  changed.
- **Screenshots:** none produced.

All commands were read-only or scratch:

```sh
git status / git log                                   # clean tree at 5db26b6
sed, grep and reads over camera.rs, pixel_grid.rs, scene.rs, lib.rs,
  shadows/cascade.rs, lighting/grid.wgsl, lighting/shade.wgsl,
  model_material.wgsl, output.wgsl, tone_map.wgsl, incant_cmd and incant_core
python3 -I - <<'EOF' … EOF                              # scratch tie-rule arithmetic above
(cd handoffs/0031-pixel-perfect-cameras && shasum -a 256 -c evidence/screenshots.sha256)   # 49 OK
```

Not run: Cargo, Clippy, GPU tests, any build, the headless binary, captures,
screenshots, browsers or UI tests.
