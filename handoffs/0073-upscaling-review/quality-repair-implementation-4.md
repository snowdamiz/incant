# 0073 implementation 4: owner attachment substrate within the existing memory admission

**Status: implementation packet for root, design and CPU arithmetic only.** No
GPU, Cargo, target, image, production edit or account action. Addenda 1–3,
their models and results, and `result.md`'s earlier sections are unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-owner-substrate-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs:**
- root's priority revision in `brief.md` (`11944b7a…`, uncommitted);
- the provenance foundation `docs/spikes/temporal-owner-provenance.md`
  (`9208b343…`), read only;
- `motion/{provenance,retained,system,draw,targets,plan}.rs` and
  `upscaling/plan.rs` in `native-agent-play` (`codex/temporal-owner-provenance`,
  uncommitted over 2fe23dce; root's source SHA `e1f3cde4…`).

**Evidence:** [`repair-model/admission_v4.py`](repair-model/admission_v4.py) and
its output [`repair-model/results-4.json`](repair-model/results-4.json).
- All 3 checks pass.
- Two runs are byte-identical.
- Python 3.9.6 and 3.14.8 agree apart from the version field.

## 1. The slice

**In this slice:**
- an exact R32Uint owner target written by the **existing** motion pass, as a
  second colour attachment;
- the integer vertex attribute that feeds it;
- one new shader kind, with its cooker contract;
- lifecycle and refusal rules;
- the combined memory admission.

**Not in this slice:**
- any consumer: the portable resolve, MetalFX pack and fixed temporal resolve
  are unchanged;
- history meta, uniforms or an uploaded bitset;
- any colour change.

All visual failures stay open (section 9).

## 2. Owner target

**Format and extent:** R32Uint at internal (source) size. It is owned by motion
`Targets`, beside the motion and depth pair, and shares their extent and Arc
lifetime. It is allocated only while a consumer has enabled owner attachment.

**Value:** the instance row's owner word, unchanged: `id | changed << 31`.
- The id is 1…100,000, from the foundation.
- `changed` is the candidate frame's bit against the accepted frame.
- 0 means no opaque or MASK surface at the texel.

**Written with the motion vector:**
- The same fragment invocation writes the owner and the motion vector, so they
  share one depth test (Depth32Float, `Less`, motion `depth[slot]`) and one
  rasterisation.
- MASK discards go through the same `mask()`, with the same UV, cutoff and
  derivative order.
- So owner, motion and motion depth agree at every texel.
- The upscaling resolve already reads that same depth.

**Clear:** `LoadOp::Clear(0)` on every motion pass.

**Validity is independent of identity:**
- When motion validity is 0 (first frame, reset, or the previous position is
  off-screen or clipped), the fragment still writes the owner word.
- On the first frame and after resets, the foundation marks every placement
  changed.

**Zero-owner cases:**
- clear, sky and environment;
- BLEND batches: not drawn in the motion pass, and the foundation gives them 0;
- sprites, text and tilemaps: overlays, never in the motion pass;
- MASK-discarded texels;
- pixel-camera bypass: no motion pass, so no owner target, and
  `motion_changes()` reports no owners.

**Mirror and skinning:**
- Handedness twins share ids (foundation). The owner pipelines exist for each
  mirrored and double-sided variant.
- A skinned binding has one row and one id. Its previous vertices are deformed
  as today. Its changed bit comes from the foundation's exact palette and world
  comparison.

**Overlay-depth pass:** unchanged. It has no colour attachment and never writes
owners.

**Known boundary:** the owner reflects the motion pass's visibility, not the
colour pass's. At exact depth ties the two passes can choose different surfaces.
This already holds for motion vectors.

## 3. Integer vertex ABI

**Previous rows: buffer 3, stride 80, per instance. Layout used only by
`MotionOwner`:**

| Location | Format | Offset | Content |
|---|---|---|---|
| 7–10 | Float32x4 | 0, 16, 32, 48 | Previous world |
| 11 | Float32 | 64 | Validity |
| 12 | **Uint32** | 68 | Owner word |

Bytes 72–79 stay zero and unread.

**Existing kinds:**
- `Motion` and `Depth` keep `@location(11) valid: vec4f` (Float32x4 at 64) and
  read only `.x` (`motion/shader.wgsl` L18–22).
- Their pipelines and cooked artifacts stay byte-identical, so fixed temporal
  and MetalFX are untouched.

**Forbidden:** carrying an id in any float lane. That includes bitcasting
location 11's lanes, where small ids would be subnormal floats.

**Shader interface:**
- Varying: `@location(3) @interpolate(flat) owner: u32`.
- Fragment outputs:
  - `@location(0) vec4f`: motion, byte-identical maths to today;
  - `@location(1) u32`: the owner word.

## 4. Source and cooker changes

| Path | Change |
|---|---|
| `motion/owner.wgsl` (new) | `MotionOwner` module with vertex and fragment entries only. Motion maths and `mask()` must stay textually identical to `motion/shader.wgsl`, which is not edited |
| `temporal_shader.rs` | `Kind::MotionOwner`, its source and `stages()` entry |
| `motion/system.rs` | Four owner pipelines (mirrored × double-sided), targets `[Rgba32Float, R32Uint]`, vertex layout `OLD_WORLD_OWNER` (section 3). Built only by the owner opt-in |
| `motion/targets.rs` | Optional `owner` view with separate `owner_bytes` and its own overlap check (section 6) |
| `motion/draw.rs` | Colour attachments become `[motion, owner]` when an owner target exists. The depth-only path is unchanged |
| `motion/retained.rs` | Select the owner pipelines when attached. Rows already carry the word |
| `incant_shader_cook/src/temporal/mod.rs` | `Kind::MotionOwner`: entry `fragment`, new source identity |
| `incant_shader_cook/src/temporal/contract.rs` | `Io::Uint(location, lanes, flat)`; integer IO must be flat with `first` sampling (today `io` accepts float IO only, L57) |
| `incant_shader_cook/src/temporal/emit.rs` | Metal vertex pulling for `MotionOwner`: buffer 12, stride 80, 7–10 Float32x4, 11 Float32 at 64, 12 Uint32 at 68. Bindings as `Motion` |
| `temporal_shader/cooked/host.rs` | Trusted `MotionOwner` identity; same host layouts as `Motion` |
| `upscaling/plan.rs` | Combined admission (section 6) |

**The `MotionOwner` contract:**
- 2 entry points;
- vertex inputs 0–10 as `Motion`, then 11 `Float(1)` and 12 `Uint(1)`;
- varyings: `[Position, Float(0,4), Float(1,2), Float(2,1,flat), Uint(3,1,flat)]`;
- fragment outputs: `[Float(0,4), Uint(1,1)]`;
- the same 4 resources as `Motion`.

## 5. Lifecycle, feature and refusal boundaries

**Startup opt-in:** a new `Renderer::enable_motion_owner()`, called after
`enable_motion_vectors`. It refuses in any of these cases:
- motion is not enabled;
- R32Uint lacks `RENDER_ATTACHMENT | TEXTURE_BINDING`;
- in complete-cooked native mode, `MotionOwner` artifacts are missing or their
  identity mismatches (fail closed).

Default and web builds compile from WGSL.

**Per retained scene:**
- Owner attachment is off by default, and only a consumer turns it on.
- Turning it on or off re-enables motion history: a new `MotionState`, a new
  owner token, and history reset.
- MetalFX and fixed temporal never attach in this slice, so they allocate
  nothing new.

**Ids are frame-scoped:** a consumer must check that `motion_changes().frame()`
is the frame that wrote the owner target. On a mismatch it has no provenance and
treats the history as fresh.

**Other boundaries:**
- More than 100,000 owners: `MotionState::new` refuses (foundation).
- A failed candidate drops its newly sized owner target; accepted targets are
  untouched.

## 6. Memory admission inside today's bound

**Unchanged:**
- `MAX_UPSCALE_TRANSITION_BYTES` = 671,105,024;
- `MAX_MOTION_PIXELS` = 4,194,304, written P below;
- `MAX_OUTPUT_BYTES` = 32P;
- `MAX_OVERLAY_BYTES` = 8P;
- `MAX_MOTION_TARGET_BYTES` = 24P;
- `MAX_MOTION_INSTANCE_BYTES` = 64 MiB;
- the 16,384 B fixed metadata;
- all defaults.

**One new sub-cap:** `MAX_OWNER_TARGET_BYTES` = 4P = 16,777,216, used only for
the owner target's own old-plus-new overlap. It adds nothing to the total.

**Logical bytes per target:**

| Target | Extent | B/px | Slice |
|---|---|---|---|
| Output colour pair, Rgba16Float × 2 | output | 16 | existing |
| Output depth pair, R32Float × 2 | output | 8 | existing |
| Composite, Rgba16Float | output | 8 | existing |
| Overlay, Rgba16Float | internal | 8 | existing |
| Motion, Rgba32Float | internal | 16 | existing |
| Motion depth pair, Depth32Float × 2 | internal | 8 | existing |
| **Owner, R32Uint** | internal | **4** | **this slice** |
| Meta pair, Rgba32Uint × 2 | output | 32 | later history slice |

**Fixed terms:**
- Previous rows (80 B each) and the foundation's two CPU bitsets (25,008 B at
  100,000 owners) are already charged inside the 64 MiB instance cap.
- The later slice adds the `Portable` uniform (912 B) and the `ChangedBits`
  uniform (12,512 B) per prepared frame.

**Proposed combined formula, with checked arithmetic:**

```
live = accepted set
     + new allocations: output components when the output extent changes,
       internal components when the internal extent changes
     + 2 * MAX_MOTION_INSTANCE_BYTES + 16384 (+ 2 * 13424 in the later slice)
admit iff live <= 671,105,024, and each component's old + new <= 2 * its cap
```

**Why the formula changes.** Today's preflight adds a full candidate set even
when `Output`, `Overlay` and `Targets` reuse the accepted allocation, which
they do whenever the extent is unchanged. With the owner target added, that
double count would refuse steady 16/16 above 3,944,196 px (largest square),
for example 2560×1600 and 2048×2048. Counting each live allocation once stays
within the same cap.

**Refusals with live accounting, owner slice:**
- **Steady state:** never. The largest set is 68P.
- **DRS change at a fixed output:** never. The worst case is 104P.
- **Output resize** refuses when
  `32·(out_old + out_new) + 36·(src_old + src_new) > 536,870,912`. For
  example, at 16/16 that is when `out_old + out_new > 7,895,160` px, so
  2048×2048 ↔ 2048×1920 refuses.
- **Remedy for a refused resize:** a cut that releases the accepted history,
  then a retry. Output history never continues across sizes today.

**Later history slice, for planning:**
- **With live accounting:**
  - every steady state is admitted;
  - DRS 16/16 → 15/16 refuses at 2560×1600 and 2048×2048;
  - output resizes refuse once the two outputs sum to more than about 5.37M px
    (16/16) up to 8.10M px (4/16).
- **With today's accounting,** it would refuse steady 2560×1440 at 8/16 and
  2560×1080 at 16/16.
- So live accounting must land before the meta allocation. The table is in
  `results-4.json`.

**Excluded, as in every earlier estimate:** these are logical bytes only. That
applies to the existing constant, addendum 3's 973,121,760 B and this packet.
They exclude:
- row pitch, tiling and compression metadata;
- depth-format padding;
- allocator and page rounding;
- driver memory;
- allocations that queued GPU work still holds.

**Remaining conservatism:** both instance payloads (2 × 64 MiB) are counted,
even with one motion state.

## 7. Shared substrate for portable and MetalFX

Both use the same motion pass, owner kind, R32Uint target, ids, changed bits and
lifecycle. No extra pass.
- **Portable:** attaches the owner target once its resolve consumes it.
- **MetalFX:** attaches nothing until root approves 0074's owner variant. Then it
  reuses the same pipelines and target, and its pack pass binds the target read
  only. That is not authorized here.

## 8. Acceptance controls

**CPU (root's Cargo tests):**
1. **Contract:**
   - `MotionOwner` is accepted with the exact IO above.
   - It is refused for any of these:
     - an owner varying that is float or not flat;
     - a missing location 12;
     - location 11 declared Float32x4;
     - the owner output declared float, or at a wrong location;
     - an extra binding;
     - a stale source.
   - `Motion` and `Depth` contracts and cooked identities stay byte-identical.
2. **Metal vertex pulling:** exact `MotionOwner` attributes, offsets and formats.
3. **Host layout:**
   - the owner pipelines' targets and vertex layout;
   - the existing 8 motion and depth pipelines unchanged.
4. **Admission:** the live formula with checked arithmetic, using rows from
   `results-4.json`:
   - steady 2048×2048 at 16/16 is admitted;
   - 2048×2048 → 2048×1920 at 16/16 is refused;
   - DRS 16 → 15 at 2048×2048 is admitted;
   - arithmetic overflow refuses;
   - the cap constant still equals 671,105,024.
5. **Lifecycle:**
   - enabling or disabling attachment resets motion history;
   - bypass leaves no owner;
   - a failed candidate keeps the accepted targets;
   - missing cooked `MotionOwner` refuses.

**GPU: one bounded device test, ignored by default, run only with a separate
grant:**
- **Size:** internal 32×32 at 16/16 and at 8/16.
- **Scene, at most 6 placements:**
  - an opaque quad;
  - a MASK quad with a cutout;
  - a mirrored twin pair;
  - a skinned mesh;
  - a BLEND quad;
  - a clear background.
- **Frames:** first (all changed), unchanged, then one placement moved and
  flipped.
- **Read back:** owner, motion and motion depth for each frame.
- **Assert:**
  - each texel's owner equals the analytically covering placement's id, with the
    changed bit matching `motion_changes()`;
  - the MASK cutout, BLEND quad and background are 0;
  - the mirrored placement keeps its id through the flip;
  - the skinned binding follows its palette changes;
  - motion and depth are bit-identical between the owner and non-owner
    pipelines;
  - the composited colour is bit-identical with attachment on and off.
- **Limits:** at most 6 frames, no saved images, no visual or performance
  claim.

## 9. Not part of this slice

**Blockers for later pixel-history integration.** These leave addendum 3's full
positive support, conservative overflow and unbounded moving history unchanged:
1. Live-accounting admission must land before the meta pair.
2. **GPU transport of the accepted changed set:**
   - the `ChangedBits` uniform and the changed-class mask, and where they are
     charged;
   - the mobile cost of indexing a uniform array is unmeasured.
3. **Host data for caster footprints:**
   - per-instance light-space bounds;
   - the cascade witness;
   - the camera-static witness.

   None of these is in the foundation.
4. **f16 directed rounding of the support range:** WGSL `pack2x16float` is
   indeterminate out of range, so inputs must be clamped first.
5. **0074's consumer:** the stored-id format and its memory under this same
   admission.

**Visual gates still unmet:**
- a5ae3179 FAIL: soft 8/16 convergence, lost texture detail, phase-unstable
  edges and sub-pixel geometry;
- C7 and C9 as written: failed;
- P1: unresolved;
- no appearance, phone or performance claim exists.

## 10. Limits of this packet

- **Not run:** GPU or Cargo. The foundation was read uncommitted and is not
  reverified here.
- **Admission model:** arithmetic on logical bytes.
  - It reproduces 671,105,024 B and 973,121,760 B exactly.
  - Its resize sums ignore attachment rounding up.
  - The refusal predicate above is exact.
