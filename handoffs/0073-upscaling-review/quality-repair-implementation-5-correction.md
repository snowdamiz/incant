# 0073 implementation 5: correction

**Status: corrected prescription for root, CPU and source only.** No GPU,
image, Cargo, build, target, production edit, memory write or subagent.

**Scope of the change:**
- Implementation 5 (`7e4e0f2a`) stays unchanged:
  - `quality-repair-implementation-5.md` (`855b22ae…`);
  - `consumer_v5.py` (`196cad70…`);
  - `results-5.json` (`5fbc9b19…`).
- This document replaces only the sections it names.
- Root's dirty `brief.md` (`e08f6c74…`) and the three untracked inputs are
  untouched.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
- Root's `0073-owner-history-prescription-002/acp-start.json` (`1abcc4c4…`)
  reads `"model": "Opus 5.5", "effort": "max", "verified": true,
  "gpu_authorized": false`.
- Its `acp.log` header reads `Claude session verified: Opus 5.5; thinking: max`.

**Inputs:**
- the brief's first section;
- Astra's `draft5-findings.json` (`dfc92b3b…`);
- producer `613ee59a` source, read through `git show`.

**Model:** [`repair-model/correction_v5.py`](repair-model/correction_v5.py)
(`96464a75…`), with output
[`repair-model/results-5-correction.json`](repair-model/results-5-correction.json)
(`7d2cd52f…`).
- It refuses to run unless `consumer_v5.py` hashes to `196cad70…`, then loads
  it by path.
- Every control runs both the draft rule and the corrected rule.
- All six checks pass.
- Two runs are byte-identical, and Python 3.9.6 output matches 3.14.8 once the
  version string is normalised.

## C1. 3×3 clamp provenance (replaces 3.2 step 5's record rule)

**Source.** In `resolve.wgsl` L57–68, the min/max of the admitted 3×3 around the
anchor bounds history whenever history is used. A ring owner's colour therefore
reaches the output even when the 2×2 omits it.

**Rule.**
- Whenever the clamp runs, collect K: the ids of every 3×3 tap the clamp admits
  (admitted, alpha 1, compatible with `nearest`).
- K always contains S, because every positive-weight 2×2 tap lies within ±1 of
  the anchor after source clamping and has already passed `same_surface`.
- Records by path:
  - reprojection: `record(A, K ∪ U)`, where U is the union of the accepted
    history taps (C2);
  - persistence: `R ∪ S`;
  - current-only: S.

**Invariant.** Every owner whose colour has reached the stored history since
the last drop is in the record, exactly or by class.

**Control (Astra's numbers).**

| | Frame t output | Frame t record | Frame t+1 (7 changed, static) |
|---|---|---|---|
| Draft | 0.45 | {5} | persists 0.405 |
| Corrected | 0.45 | {5,7} | drops to current 0.0, record {5} |

**Cost.** Ring owners overflow records sooner, which means more conservative
drops at depth-compatible edges.

## C2. Every bilinear history tap (replaces 3.2 step 5's tap admission)

**Accepting a tap.** A tap is considered when its weight is positive, it lies
inside the output, and it passes today's colour, alpha and depth checks. It is
accepted only if all of these hold:
1. its record reads canonically (C4);
2. its anchor equals the current anchor A;
3. for an exact record, no owner is both changed and absent from S. For an
   overflow record, `(M | classes(S)) & C = 0`.

**Aggregation.**
- `history = Σ w·H / Σ w`, summed over accepted taps only. This is today's
  normalisation restricted to the accepted set.
- If `Σ w = 0`, take the current-only exit with a fresh record.
- `U` is the exact ids of accepted exact records plus the OR of accepted masks.

**The static tap at P** is the single-tap case of this rule. All 15 draft
`decide()` cases give the same branch and record. The draft's
`reproject_no_record` is reported here as "no history".

**Control.** Four taps with weights 0.375, 0.125, 0.375 and 0.125. Tap (1,0)
holds Astra's `[2147483653, 7]`; owner 7 changed, and S = {5}.

| | Taps accepted | History | Colour | Record |
|---|---|---|---|---|
| Draft | 4 | 0.55 | 0.52 | overflow mask 672, carrying class 7 |
| Corrected | 3, renormalised over 0.875 | 0.5 | 0.475 | {5,9} |

On that record alone, the draft's scalar `decide()` returns
`drop [2147483653, 0]`, exactly as Astra found.

## C3. `camera_static` (replaces 3.3's camera rule)

**Source.** Shading reads three camera inputs besides the matrix:
- **`shading_eye()`:** the material `Frame.eye` (`model_draw.rs` L37, offset 64).
  It is the eye with w=1, or −forward with w=0 for orthographic cameras
  (`camera.rs` L185–191).
- **`camera.view`:** the lighting grid uniform (`lighting/mod.rs` L337) and
  `cascade::fit` (`cascade.rs` L72).
- **The internal rect:** `jitter::guard(rect) = 1/size` (`jitter.rs` L18–20).
  - It scales cascade receivers (`cascade.rs` L104–116).
  - It selects the guarded cluster pipeline (`lighting/mod.rs` L402).
  - The cluster grid and shadow aspect also take the rect.

  The upscaled `Extent` ignores the internal rect, so `has_previous` survives
  DRS.

**Rule.**
- `History` keeps a `StaticKey` in `Pending` and `Accepted`. It holds the bits of:
  - `camera.unjittered().matrix(aspect)` (16 words);
  - `camera.view` (16);
  - `camera.shading_eye()` (4);
  - `plan.rect` (4).
- `camera_static` = `has_previous` and the two keys are bit-equal.
- Use `unjittered()` explicitly; do not rely on the staging camera's jitter.

**Controls.**
1. **Eye and view.** A perspective camera looks down −Z with eye.x = 1.5 or the
   next f32, 1.5000001.
   - The unjittered matrix bits are identical: its only eye.x term is
     `round32(w·−eye.x)`, with w = 0.974…
   - View and eye bits differ.
   - The draft calls the camera static; the corrected key does not.
   - Such pairs exist for every w in (0.5, 1) by pigeonhole. The scan finds them
     at 0.55, 0.75, 0.9 and 0.99.
2. **Internal rect.** DRS 2048² → 1920² at a fixed output.
   - Aspect, matrix, view and eye bits are all equal.
   - The guard bits differ (`973078528` against `973637769`).
   - A matrix-plus-eye key calls it static; the corrected key does not.

**Cost.** No persistence on frames where DRS changes; reprojection still runs.

## C4. Canonical records, positive weight, clear and malformed owners (replaces 3.1's canonical read and 3.2 step 1)

**Owner word.**
- The id is the low 31 bits, so bits 17–30 cannot alias. It is valid when
  1 ≤ id ≤ `highest`.
- An owner is changed when its bitset bit is set **or** the word's bit 31 is set.

**Support and anchor.**
- S is the ids of 2×2 taps with `tap_weight > 0` and depth > 0. Zero-weight taps
  never count.
- A is the id at the existing anchor: the nearest positive-weight tap with depth.

**Writer: `record(A, owners, mask)`.**
- It writes `(0,0)` unless A and every owner are valid.
- Otherwise `x = P|A`.
- `y` is the second id. It switches to overflow when `mask ≠ 0` or there are
  more than two ids. Overflow writes `O | mask | classes(all ids, A included)`.

**Reader.** A record is canonical only if all of these hold:
- x is present, its reserved bits are zero, and A is valid;
- an exact y is 0, or a valid id different from A;
- an overflow mask contains A's class.

The present-clear `[P|0,0]` is no longer canonical.

| Exit (`resolve.wgsl`) | Depth written | Record |
|---|---|---|
| Outside (L10), all clear (L27), not one surface (L33) | 0 | (0,0) |
| History off (L36), bad motion (L38), old outside (L43), no accepted tap (L55), static drop | nearest | `record(A, S)` |
| Persist | nearest | `record(A, R ∪ S)` |
| Reproject | nearest | `record(A, K ∪ U)` |

**Invalid ids.** Any invalid A, or invalid id in S or K, writes `(0,0)`. Such a
pixel never persists, and no tap passes its anchor gate.

**In the shader,** `result.record` starts at `(0,0)` and is set to the fresh record
immediately after the same-surface test.

Depth-0 texels are never read as history, because `compatible` requires depth >
0. Writing `(0,0)` there loses nothing.

**Controls.**

| Input | Draft | Corrected |
|---|---|---|
| `encode(0,{7})` | `[2147483648, 7]`, which its own decoder refuses | `(0,0)` |
| An untracked nearest tap (owner 0 at depth 9.0, owner 7 beside it) | the resolve writes that same malformed record | `(0,0)` |
| Owner word `5\|1<<20` | aliased to owner 5 | `(0,0)` |
| Id 100001 | — | `(0,0)` |
| Malformed previous `[P\|0,7]` | — | tap rejected; current only; fresh {5} |
| Record {5,7}, 7 changed, 7 present only in a zero-weight tap | — | drop. Counting zero weight would wrongly reproject. |

**Where owner 0 at depth can come from.** BLEND, sprites and world text
composite after the resolve (`upscaling/mod.rs`, `composite.wgsl`), so they
never reach history. Owner 0 at depth would need a depth-writing draw that
carries no owner. GPU case 11 checks that such pixels lose history safely.

## C5. Future GPU control: actual submissions (replaces section 5)

**Form.** This follows the `upscaling/device_tests.rs` pattern. Each case uses one
command encoder containing:
- the depth fixture pass;
- the owner resolve;
- in cases 7 and 8, the existing `Upscale` resolve into separate targets;
- copies of every output into one readback buffer.

Each case then makes one `queue.submit` and one `map_async` plus `poll(Wait)`.
`write_texture` and `create_buffer_init` are queued writes, not submissions.
`Renderer::headless`, `enable_temporal_upscaling` and `test_support::checked`
make no submissions; I checked their source.

**The 12 cases:**
1. a fresh record, with outside texels `(0,0)`;
2. static persistence, with history outside the clamp range;
3. `{5,7}` with 7 changed drops (Astra's frame t+1), and so does 7 present only in
   a zero-weight tap;
4. an overflow class hit drops;
5. an overflow without a hit persists;
6. a tolerance-close tap with a different anchor is rejected;
7. `global` set, with colour and depth equal to `Upscale`;
8. camera not static, with colour and depth equal to `Upscale`;
9. a clamp-only owner is recorded: 0.45 and `{5,7}` (Astra's frame t);
10. the four-record renormalisation;
11. an untracked anchor, an id above `highest`, an alpha < 1 edge and a malformed
    previous record;
12. all clear (depth cleared to 1.0): every record is `(0,0)`.

**Totals:**
- 12 owner renders and 2 baseline renders;
- **12 queue submissions** and 12 readback maps;
- at most 32×32, and no images.

**Counting.** The test counts its own `queue.submit` calls, asserts there were 12,
and prints the marker from its counters:

```
OWNER_HISTORY cases=12 owner_renders=12 baseline_renders=2 queue_submits=12 readback_maps=12 max_extent=32x32 no_images=true
```

**Budget:** 1 launch and 12 explicit submissions. Any other structure must be
recounted before a grant.
- The draft claimed 8 renders and 16 submissions.
- Its own plan actually had 10 renders, which makes 20 submissions if render
  and readback are separate.

## C6. Admission and storage (replaces the 3.5 rows and the 3.6 accounting)

### Reuse requires extent and flag

**Prepare functions.**
- `Output::prepare` may reuse only when the size **and** the record flag are
  equal.
- `Targets::prepare` already requires an equal owner flag (`motion/targets.rs`
  L19).

**`admit()` uses the same keys:**
- An old `Output` is charged 32 B/px, plus 16 B/px if it has a record.
- Output reuse requires the same extent and record flag.
- Motion and owner reuse likewise require the same extent and owner flag.

**Effect.**
- With equal flags, every results-5 row is unchanged: steady 486,555,648 B, DRS
  619,266,048 B, and the same resize values and refusals.
- A same-extent flag mismatch would be under-counted by the extent-only rule:
  by 134,217,728 B for the record and 100,663,296 B for the owner. Today's API
  cannot reach that state, because a toggle drops both states.

### New retained storage, charged

These allocations exist only when the attachment is on. `MotionState::new`
charges them through `plan::charge` under the 64 MiB motion-instance sub-cap:

| Storage | Bytes |
|---|---|
| `OwnerChanges` GPU uniform | 12,528 |
| CPU upload array `[u32; 3132]`: header, 3,126 words, 2 zero; one `write_buffer` per submit | 12,528 |
| `accepted_lights`, moved from `RetainedScene` into `MotionState`: packed-light capacity × 64 | ≤ 4,112 × 64 = 263,168 |
| **Total** | **≤ 288,224** |

**Wiring.**
- `MotionState::new` takes the attachment flag.
  `set_motion_owner_attachment` and `enable_temporal_upscaling` pass it
  rather than setting it after construction.
- `MotionState::prepare` compares the staged `packed_lights` for `global`.
  `accept` copies them without allocating.

**Ceiling.** The aggregate already counts 2 × 64 MiB for this sub-cap, so
**671,105,024 B is unchanged.**

**Other storage.**
- **Already charged:** `provenance::charge` covers the provenance bitsets, 2 ×
  3,126 × 4 B at 100,000 ids; the uniform holds 3,128 words.
- **Queued:** each `write_buffer` stages a 12,528 B copy, which wgpu holds until
  that submission completes. Like every existing write, this is outside logical
  admission and not claimed.

## C7. Reset before a resize retry (replaces 3.6's "cut, then retry")

**A cut frees nothing.**
- `invalidate_motion_history` (`retained.rs` L219) calls
  `MotionState::invalidate`, which only invalidates `History` (`motion/retained.rs`
  L335).
- `State` keeps `output` and `overlay`, and `preflight_owner` charges them
  (`upscaling/owner.rs` L78–79).
- `MotionState` keeps `targets`.
- The retry is therefore refused identically.

**The real release** is `enable_temporal_upscaling(renderer, same config)`
(`retained.rs` L253):
- it replaces `MotionState` (the new one has `targets: None`, L286). The old one
  drops with its `Targets` Arc;
- it replaces `State` (the new one has no `Output` or `Overlay`). The old one
  drops with the record pair;
- the attachment flag is preserved.

The retry is then admitted as fresh. A fresh admission is never larger than the
steady state at that size, so the retry always fits.

| 2048² → 2048×1920 at 16/16 | Result |
|---|---|
| First attempt | refused (ceiling) |
| After a cut | refused (ceiling) |
| After the reset | admitted at 464,535,552 B |

`disable_motion_history` also releases all three but leaves upscaling off.
Queued work can keep the old textures physically resident; that is not claimed.

## Root's CPU tests (replaces section 4, items 2–5)

- **Record:**
  - the corrected writer and reader;
  - the 15 draft cases, unchanged;
  - the clamp chain, the four-record aggregation, zero weight, every exit row
    and malformed words.
- **Host:** `StaticKey` is false when any one of matrix, view, eye or rect
  differs. `global` is unchanged.
- **Admission:** flag-keyed rows. The new charges are present only when
  attached. The ceiling is constant.
- **Lifecycle:**
  - a refused resize, followed by `invalidate`, is still refused, with
    `target_size()` and `Output` retained;
  - after `enable_temporal_upscaling` it is admitted, with the attachment
    preserved.

## Unchanged and still open

**Unchanged.** Everything else in implementation 5 stands. 0072 owns flicker and
sharpness, and nothing here duplicates it.

**No GPU control has run.**

**Still open:**
- **Quality:** the 2×2 reconstruction keeps the a5ae3179 edge and sub-pixel
  FAIL, along with C7 and C9 as written and P1.
- footprint rectangles;
- fixed temporal and MetalFX with the attachment;
- native owner packing;
- main integration;
- physical memory, performance and phone gates.

## Paths and commands

**New files:**
- this document;
- `repair-model/correction_v5.py`;
- `repair-model/results-5-correction.json`.

There are no screenshots, since images are out of scope.

```
python3 -I -B handoffs/0073-upscaling-review/repair-model/correction_v5.py \
    --out handoffs/0073-upscaling-review/repair-model/results-5-correction.json
```

**Results:**
- exit 0, with six checks true;
- a repeat run is byte-identical (`7d2cd52f…`);
- `/usr/bin/python3` 3.9.6 output is identical apart from its version string.
