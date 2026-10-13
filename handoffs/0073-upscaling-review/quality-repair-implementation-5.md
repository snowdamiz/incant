# 0073 implementation 5: owner-history consumer in the existing portable resolve

**Status: implementation prescription for root, CPU and source design only.** No
GPU, image, Cargo, build, target, production edit, memory write or subagent.
All earlier reports, evidence, root's dirty `brief.md` (`d155f5e7…`) and the
three untracked input files are unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max,
configured and verified by root for this resumed ACP session.

**Source identities (read only, from git objects):**
- implementation `613ee59aec1f2bce080d4a1e335dc45f732b993f` and closure
  `4c4310874405f569b60e32a0d8d5f21073546c9d` in `native-agent-play`. The
  closure adds only documentation and receipts; every source path cited below
  is at `613ee59a`.
- `quality-repair-implementation-4.md` (`924fc75c…`).
- root's GPU acceptance `temporal-owner-attachment-gpu-root-20261013.json`
  (`ad7cde4b…`);
- the independent main228 source review
  `temporal-owner-main228-source-independent-20261013.json` (`fcfc04e6…`).

**Supporting model:**
[`repair-model/consumer_v5.py`](repair-model/consumer_v5.py) (`196cad70…`),
with output [`repair-model/results-5.json`](repair-model/results-5.json)
(`5fbc9b19…`).
- It covers arithmetic and provenance only.
- It reproduces the producer's own admission test values exactly.
- Two runs are byte-identical, and Python 3.9.6 matches 3.14.8.

## 1. Where this lands

- **Private branch.** The consumer is implemented on `native-agent-play` after
  `4c431087`. That branch holds the owner attachment and the private portable
  upscaler ancestry. It changes private `upscaling/*` and motion/retained host
  files.
- **Main integration is a separate step.** Root's acceptance limit
  (`ad7cde4b`) forbids importing the private upscaler into main, and the
  pending main228 composition is owner-only. So this consumer can reach main
  only after the private upscaler itself is curated there (section 7, item 5).
  main228 is unaffected.
- **Combinations stay closed.** Fixed temporal and MetalFX still refuse with
  `OwnerCombination` while the owner attachment is on (`retained.rs`). This
  slice does not admit them, and it does not add native owner packing.

## 2. What the producer gives a consumer (verified in source)

- **Owner word:** `id | changed << 31` in an R32Uint target at source size
  (`motion/targets.rs`).
  - It is written by the same fragment and depth test as motion
    (`motion/owner.wgsl`, `motion/draw.rs`).
  - It is 0 for clear, BLEND and MASK-discarded texels.
  - It is written even when motion validity is 0.
  - `changed` is the candidate frame's bit against the accepted frame
    (`motion/provenance.rs`).
- **Admission:** `motion/attachment.rs::admit` counts an equal extent once,
  keeps per-component old-plus-new sub-caps, and holds the aggregate at
  `MAX_UPSCALE_TRANSITION_BYTES` = 671,105,024 B. The owner path uses it
  through `upscaling::State::preflight_owner`.
- **Lifecycle:** one synchronous candidate per `submit`: prepare, encode, queue
  writes, submit, swap live and staging, then accept (`retained.rs` L525–650).
  A failed candidate drops its Arcs. `Targets`, `Output` and `Overlay` reuse
  the exact accepted Arc when the extent matches, and `Targets` also requires
  the same owner flag.
- **Not consumed yet:** `Prepared::upscale_input` passes only motion and depth
  (`motion/retained.rs` L90–104).
- **The production resolve** (`upscaling/resolve.wgsl`, 70 lines):
  - a bilinear 2×2 current estimate, current-only when the 2×2 is not one opaque
    surface;
  - the nearest-depth tap as anchor and motion source;
  - reprojected 2×2 history admitted by depth;
  - a 3×3 same-surface clamp, a reactive term and the fixed `options.x` blend.

  The accepted kernel oracle (2fe23dce) is test-only.

## 3. The consumer

It is the smallest complete one. It records owner provenance with each output
pixel's history and uses it three ways:
1. identity-gated history taps;
2. changed-owner drops;
3. unclipped persistence where "unchanged" is proven.

Everything else is today's resolve. With the attachment off, the existing
`Upscale` kind, shader and pipeline run byte-identically.

### 3.1 Support, anchor and record

**Support S.** The ids (bits 0–16 of the owner word) of the 2×2 current taps
with `tap_weight > 0` and linear depth > 0, after the existing clamp to
`lo..hi`. Their changed bits are bit 31. Clear taps contribute no owner.

**Anchor owner.** The id at the existing nearest-depth anchor tap.

**Record:** an Rg32Uint pair at output size, flipping with colour and depth.

| Lane | Bits | Meaning |
|---|---|---|
| x | 31 | Present |
| x | 0–16 | Anchor owner id (0 = clear) |
| x | 17–30 | Zero |
| y, bit 31 clear | 0–16 | The second recorded owner (0 = none) |
| y, bit 31 set | 0–30 | Overflow: class mask, bit `id mod 31`, of every owner recorded since the last drop |

**Canonical read.** Anything else is no history:
- present bit set and reserved bits zero;
- ids no greater than this frame's highest id;
- y differs from x when nonzero;
- y is 0 when x's id is 0;
- an overflow mask is nonzero.

Pixels outside the output rectangle write `(0, 0)`. `consumer_v5.py`
`encode`/`decode` is the executable reference.

### 3.2 Per-pixel rules: `upscaling/resolve_owner.wgsl`

This is a copy of `resolve.wgsl` with these changes:

1. **Early exits.** On every existing early exit (outside, all clear, not one
   surface, or `frame.depth.w < 0.5`), write the colour as today and a fresh
   record: `encode(anchor, S)`.
2. **Static.** A pixel is static when the owner flags' `camera_static` is set and
   `|old − p| < 1/256` px on both axes. For a static pixel, `old = p`, so the
   existing history fetch is the single texel at P.
3. **Static, exact record R:**
   - **Drop:** if any changed owner in R is absent from S, use the current value
     only, with a fresh record.
   - **Persist:** if no owner in R or S changed, `global` is clear, and the tap
     at P passes the existing colour/alpha/depth checks:
     `color = clean(mix(current, H_P, frame.options.x))`, with no clamp and no
     reactive term. The record becomes `R ∪ S`. No anchor match is required,
     because the whole recorded set is proven unchanged.
4. **Static, overflow mask M:**
   - **Drop** if `(M | classes(S)) & C ≠ 0`.
   - **Otherwise persist** as in step 3, with mask `M | classes(S)`.
5. **Every other case** takes today's reprojection path, with one extra admission
   condition per history tap: its record is present and its anchor id equals
   the current anchor's owner. The new record is S joined with every admitted
   tap's record: exact ids while there are at most 2, otherwise the class mask.
   Clamp, reactive term and blend are unchanged.

"Global" and "camera static" come from the host (3.3). Persistence never
evaluates any lighting.

`decide()` is the reference. Its 15 cases cover:
- persist, including a thin owner missed in one phase;
- drop, by an absent changed owner, by an overflow class hit, and by an
  overflow collision (the cost);
- reproject, by a present changed owner, a changed newcomer, a camera change,
  a light or caster change, or motion;
- an unrelated change that does not disturb an exact record;
- no history, for a missing, malformed or out-of-range record.

### 3.3 Host flags

The header is the 16 bytes `vec4u(C, flags, highest, 0)`.

**`camera_static`:**
- `motion/owner.rs` keeps an `unjittered: Mat4` in `Pending` and `Accepted`.
  `prepare_temporal` already computes it before applying jitter.
- `camera_static` = `has_previous` and the two matrices are bitwise equal.
- `Extent::compatible` already requires the same output size, output rectangle
  and projection whenever `has_previous` holds.

**`global`** is set when either holds:
- the frame's packed lights differ bytewise from the accepted ones.
  `RetainedScene` keeps `accepted_lights: Vec<Light>`, 64 B each, with
  `packed_lights`' capacity, copied on accept with no per-frame allocation;
- any changed owner's batch has `casts_shadows`. `MotionState::prepare` sets
  this while marking rows.

**Why that covers lighting.** Shading inputs are view-projection and eye (camera
static), the frozen environment (`retained.rs` L259), packed lights, shadow
maps and owner geometry. Cascade fits depend only on the camera, the light
direction and caster bounds (`shadows/cascade.rs`). So with the camera static,
lights unchanged and no caster changed, static shading cannot change.

**`C`** is the class mask of the candidate's changed set, and **`highest`** is the
highest owner id.

**This replaces addendum 3's footprint rectangles for now,** with a footprint
covering the whole view: any light or caster change disables unclipped
persistence everywhere for that frame. That is exact and conservative.
Addendum 3's unbounded flag, `n_min` and support depth range exist only to
make rectangles sound, so they are deferred with them (section 7, item 2).
Full positive-support provenance, conservative overflow and exact changed-owner
semantics are unchanged.

### 3.4 Shader kind and bindings

**New `Kind::UpscaleOwner`:** source is `common.wgsl` plus `resolve_owner.wgsl`.
`Kind::Upscale` and `resolve.wgsl` stay byte-identical.

**Group 0, fragment visibility:**

| Binding | Resource | WGSL | Metal | HLSL |
|---|---|---|---|---|
| 0 | `Frame`, 96 B, unchanged | `var<uniform>` | buffer 0 | b0 |
| 1–5 | Current, motion, depth, previous colour, previous depth, unchanged | as today | texture 0–4 | t0–t4 |
| 6 | Owner (source size, R32Uint) | `texture_2d<u32>` | texture 5 | t5 |
| 7 | Previous record (output size, Rg32Uint) | `texture_2d<u32>` | texture 6 | t6 |
| 8 | `OwnerChanges { header: vec4u, words: array<vec4u, 782> }`, 12,528 B | `var<uniform>` | buffer 1 | b1 |

- **Outputs:** colour Rgba16Float at location 0, depth R32Float at 1, record
  Rg32Uint at 2. That is 20 B per sample.
- **Changed-bit lookup:** bit `id` is
  `(words[id >> 7][(id >> 5) & 3] >> (id & 31)) & 1`.

**Cooker (`incant_shader_cook/src/temporal/{mod,contract,emit}.rs`):**
- the `UpscaleOwner` contract:
  - 9 resources;
  - sampled-uint textures at bindings 6 and 7;
  - the exact uniform span and members at binding 8;
  - outputs `[Float(0,4), Float(1,1), Uint(2,2)]`, using the existing
    `Io::Uint` from `MotionOwner`;
- an explicit per-binding Metal map;
- native artifacts and a trusted identity in `temporal_shader/cooked{,/host}.rs`;
- in complete-cooked mode, refusal when they are missing.

### 3.5 Host changes (files at `613ee59a`)

| File | Change |
|---|---|
| `motion/owner.rs` | `unjittered` in `Pending`/`Accepted`; expose `camera_static` |
| `motion/provenance.rs` | Read access to the candidate words, class mask and highest id |
| `motion/retained.rs` | Allocate `changes: Option<wgpu::Buffer>` (12,528 B, `UNIFORM \| COPY_DST`) when attached. Charge it through `plan::charge` under the existing 64 MiB instance cap, and write header plus candidate words in `write()`. `upscale_input` also carries the owner view and the buffer. Set the changed-caster flag |
| `retained.rs` | `accepted_lights`, the `global` flag, flags into `motion.write`; copy lights in accept |
| `upscaling/targets.rs` | `Output` gains `record: Option<[TextureView; 2]>` (Rg32Uint). Reuse requires the same size **and** the same owner flag; 48 B/px when attached |
| `upscaling/system.rs` | Explicit 9-binding owner layout and pipeline, built only when the renderer has owner pipelines and upscaling. `prepare` selects it when attached |
| `upscaling/owner.rs`, `motion/attachment.rs` | `admit(..., record: bool)` adds `component(old_output, output, 16, MAX_OWNER_RECORD_BYTES = 16P)` |
| `temporal_shader.rs` | The `UpscaleOwner` kind and source |

### 3.6 Lifetimes and checked accounting

**Lifetimes:**
- **Record pair:** it lives inside `Output`, so it shares the colour and depth
  history's Arc lifetime. A candidate pair exists only when the output extent
  or owner flag changes. A failed candidate drops it, and accept replaces it.
- **`OwnerChanges` buffer:** one per `MotionState`. It is rewritten before each
  submit, as the motion rows are, so queued earlier submissions keep their
  data in wgpu's write ordering.

**Accounting.** Live admission, unchanged ceiling:
- total = 2 × 64 MiB + 16,384 + every component;
- source components: motion 24, owner 4, overlay 8 B/px;
- output components: output 32, **record 16** B/px.

| Case (results-5.json) | Owner only today | With the record |
|---|---|---|
| Steady 2048² at 16/16 | 419,446,784 | 486,555,648 |
| DRS 16↔15 at 2048² | 552,157,184 | 619,266,048 |
| 1920×1080 → 2560×1440 at 16/16 | 525,914,112 | 618,074,112 |
| 2560×1440 → 2560×1600 at 8/16 | 453,312,512 | 577,830,912 |
| 2048² → 2048×1920 at 16/16 | refused | refused |

- **Steady states and DRS changes always fit.**
- **Output resizes refuse** once the old and new outputs together exceed about
  6.39M px at 16/16, 7.87M at 12/16, 9.42M at 8/16 and 10.68M at 4/16
  (attachment rounding up ignored). The exact refusal is the checked sum.
- **Remedy for a refused resize:** cut, then retry. Output history never
  survives a size change.
- **Logical only:** queued work and wgpu's deferred destruction can keep replaced
  `Output`/`Targets` resident, and no physical bound is claimed.
- **Outside this ceiling:** the `accepted_lights` copy is retained-scene CPU
  memory, at most 4,112 × 64 = 263,168 B, like `packed_lights`.

### 3.7 Reset and commit semantics

**Resets.** Each of these gives `has_previous = false`, and the first frame then
writes fresh records:
- an attachment toggle;
- `enable_temporal_upscaling`;
- a replacement;
- a cut;
- a pixel-camera bypass.

**Commit.** A record written at frame t is read at t+1 only through the accepted
`Output` slot, like colour.

**Id scope.** Ids are compared only when `has_previous` holds. Upscaling
`prepare` already requires the same motion owner, the same epoch and the next
sequence, so ids never cross owners.

## 4. CPU controls (root's Cargo tests)

1. **Contract.**
   - `UpscaleOwner` is accepted with the exact interface.
   - It is refused for:
     - a float owner or record texture;
     - a record output declared float, or at location 1;
     - a uniform span other than 12,528 B;
     - an extra binding;
     - a stale source.
   - `Upscale` stays byte-identical.
2. **Admission.** Every row of the 3.6 table, plus overflow and oversize refusal.
   The producer's existing `owner_attachment_admission…` test stays unchanged.
3. **Record.** Encode, decode and canonicalisation, and all 15 `decide()` cases,
   as data-driven Rust tests.
4. **Host flags.**
   - `camera_static`: true only for bit-equal unjittered matrices.
   - `global`: set by a single changed light byte, set by a changed caster, and
     not set by a changed non-caster.
5. **Lifecycle.**
   - `Output` is not reused across owner-flag changes.
   - A failed candidate keeps the accepted record.
   - Toggles reset.
   - `OwnerCombination` still refuses fixed temporal and MetalFX.

## 5. Later bounded GPU control (separate grant)

**Form.** Extend `upscaling/device_tests.rs`'s direct pipeline style, which uses
synthetic input textures:
- one ignored test;
- 8 cases, each 1 render and 1 batched readback;
- **16 explicit queue submissions** in total;
- at most 32×32, with sources 32 and 16;
- no images;
- final marker
  `OWNER_HISTORY renders=8 readback_submissions=8 max_extent=32x32 no_images=true`.

**The 8 cases:**
1. a fresh record;
2. static persistence, with history deliberately outside the clamp range;
3. a changed absent owner drops;
4. an overflow class hit drops;
5. an overflow without a hit persists;
6. a tolerance-close tap with a different owner is rejected;
7. `global` set;
8. camera not static.

**Asserts:**
- every record texel against `decide()`;
- colour against the f64 `oracle.rs` reference plus the persistence formula;
- in cases 7 and 8, whose stored anchors match the current anchor, colour and
  depth identical to running the existing `Upscale` pipeline on the same inputs.

## 6. Not duplicated

No fixed-resolution flicker or sharpness model is included; 0072 owns that.
The consumer is portable-only, and fixed temporal still refuses the owner
attachment. No image simulation was run.

## 7. Unresolved dependencies (precise)

1. **Quality.** This slice keeps the 2×2 reconstruction and the "not one
   surface, so current only" rule. Edges and sub-pixel features stay
   current-only, so the a5ae3179 FAIL's edge and sub-pixel instability is
   **not** addressed. Interior texture detail may stabilise where persistence
   holds, but that is unmeasured.
   - *Next:* move the accepted kernel oracle (2fe23dce) into production, then a
     visual review.
2. **Footprint rectangles.** These replace the whole-view `global` flag. They
   need host per-caster light-space bounds and a cascade witness, plus the
   addendum-3 record extension (unbounded flag, `n_min`, f16 support range),
   which means an Rgba32Uint record (+16 B/px).
   - *Next:* specify after item 1, using the same admission function.
3. **Fixed temporal or MetalFX with the owner attachment.** Both are refused.
   - *Next:* a reviewed accounting of their retained caches: fixed-temporal
     colour/depth history at source size, and MetalFX pack/normalize targets
     plus any 0074 coverage history, as components of `admit`.
4. **Native owner packing (0074).** The stored-id width and the MetalFX pack
   binding are undecided, and depend on item 3.
5. **Main integration.** Per root's `ad7cde4b` limit, this needs the private
   upscaler curated into main, then a scoped selection of this slice's files.
   main228 owner-only is independent.
6. **Not established:** physical GPU memory, performance and phone gates.
7. **Still open from earlier:** the a5ae3179 visual FAIL, C7 and C9 as written,
   and P1.
