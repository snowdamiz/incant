# 0073 owner-history shader: result

**Status: production shader source written. Not compiled, not run.**
- The production source is `crates/incant_render/src/upscaling/resolve_owner.wgsl`
  (`df600750ab5cad117ae0cfa1e76fa635e07c96be4a76d8aa9e8470d9954c64ae`, 179 lines,
  ASCII).
- The shader is alone in three isolated commits, to be selected in order:
  - `f437deaa706e33285bae552a2ca8ea7b71269952` added it (`45a39859…`);
  - `9f3e6afbf9b96e5d0349593e7307d914218506cc` is the fallback-coordinate
    correction below (`1744fdff…`);
  - `df1136f29197906728930fb97069476a31d07b27` makes the support indices u32
    (`df600750…`; see the addendum).

  The checks and this document follow in separate commits.
- No Cargo, Rust, build, target, GPU, image, screenshot or visual tuning was
  used, and no host or root source was edited.
- Root compiles it, adds the host and cooker work, and alone schedules any GPU
  run.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
- Root's `0073-owner-consumer-shader-001/acp-start.json` (`06bb0638…`) reads
  `"model": "Opus 5.5", "effort": "max", "verified": true,
  "gpu_authorized": false`.
- Its `acp.log` header reads `Claude session verified: Opus 5.5; thinking: max`.
- No subagents, memory writes or pushes.

**Inputs:**
- the corrected prescription `d90b4aac`, over draft `7e4e0f2a`;
- `ace94f64` host source, read only through `git show`;
- Astra's qualifications, `temporal-owner-prescription5-correction-independent-20261013.json`
  (`36c54a78…`).

## Correction after `1bb9ae18`: fallback coordinates

**What the independent review found.** `45a39859` set
`oldq=select(old,p,still)` before it knew whether persistence applied. So a
static-camera pixel with motion under 1/256 px fell back to ordinary
reprojection at texel P, not at its actual old position. That happened when:
- global was set;
- an owner had changed;
- the own-texel record was malformed;
- the own-texel history was rejected.

Today's `resolve.wgsl` uses the actual old position in all of these.

**Fix.**
- Only the persistence branch reads `own=vec2i(p)`, with the same proof and the
  same threshold.
- Every fallback uses the original `oldq` line verbatim.
- No other behaviour changed.

**Qualification of earlier agreement.** The d90 reference `resolve()` and the
`45a39859` transcription made the same snap, and the earlier replay and fuzz held
no nonzero sub-1/256 displacement. So their agreement never covered this case.

## Addendum after `297e2724`: u32 support indices

**Root's result.**
- The composed host compiled, and its 152 CPU tests ran.
- The Naga 29.0.4 SPIR-V round trip rejected `UpscaleOwner` with
  `BuiltinArgumentsInvalid("Min")` at function 21, expression 374, span
  19672..19700.
- Root traced this to `write_restricted_index`, where a cached signed index feeds
  `UMin` with a u32 bound.

**Fix (`df1136f2`).** Only the index type changed, at three sites on two lines:
- line 95, the store `support[y*2+x]`, became `support[u32(y*2+x)]`;
- line 115, both `support[i]` reads became `support[u32(i)]`.

**Why the values cannot change.**
- **Line 95:** it sits in the line 85 loop `for(var y=0;y<2;y++) {for(var
  x=0;x<2;x++)`, whose body never reassigns x or y. The index is therefore in
  {0,1,2,3}.
- **Line 115:** the loop is `for(var i=0;i<4;i++)`, so `i` is in {0,1,2,3}.
- WGSL `u32()` of an i32 is the value mod 2^32, which is the same value for these
  non-negative indices.
- Every index is below 4, the vector length, so Naga's restrict clamp `min(.,3)`
  never fires.
- The only other dynamic index, `changes.words[n>>7u][(n>>5u)&3u]`, was already
  u32.
- Nothing else changed: coordinates, thresholds, provenance, branches, bindings,
  `common.wgsl` and `resolve.wgsl` are all untouched.

**Check.** A CPU assertion run on the edited file passed. It:
- matches the exact loop headers and the three converted sites;
- finds no reassignment of x or y;
- confirms that every non-comment dynamic index is u32-typed;
- enumerates `y*2+x` = [0,1,2,3] and `i` = [0,1,2,3].

**Not re-run.** `shader_v5_check.py` stays pinned to `1744fdff`, so it now refuses
the new file by design, and its results remain those of `1744fdff`. This change
alters only index types, and the Python transcription has no integer signedness.

**Still root's.** No build, compile or GPU work was done here. The Naga and
cooker validation belong to root. A clean compile would not prove rendered
behaviour.

## Base

The shader extends `ace94f64` `resolve.wgsl` (`b8056325…`, also `613ee59a`), the
per-tap renormalising version. This worktree's own `resolve.wgsl` is an older
all-or-nothing variant and was not used. `common.wgsl` is identical in all three
trees.

Against `ace94f64`, the only changed lines are:
- the three-output struct, the signature and the result initialiser;
- the two history-loop lines, which add the record join.

The `oldq` line is the original.

Every other colour and depth line is kept verbatim.

## ABI, as fixed by root

**Source:** `Kind::UpscaleOwner` = `common.wgsl` + `"\n"` + `resolve_owner.wgsl`.
**Entries:** fragment `resolve`; the vertex entry comes from `common.wgsl`.
There are no overrides and no other globals.

| Binding | Declaration |
|---|---|
| 0 | `frame:Frame`, 96 B (common, unchanged) |
| 1–5 | `current`, `motion`, `current_depth` (depth), `previous_color`, `previous_depth`, as in `resolve.wgsl` |
| 6 | `owner:texture_2d<u32>`, source-sized R32Uint |
| 7 | `previous_record:texture_2d<u32>`, output-sized Rg32Uint |
| 8 | `var<uniform> changes:OwnerChanges { header:vec4u, words:array<vec4u,782> }`, 12,528 B |

**Header:** `(C, flags, highest, 0)`.
- **flags:** bit0 is camera static, bit1 is global.
- **Changed lookup:** owner n is changed when `words[n>>7][(n>>5)&3]` bit `n&31` is
  set.

**Outputs:** location 0 `vec4f` colour, location 1 `f32` depth, location 2
`vec2u` record. No blending.

## Behaviour (d90 C1–C4)

**IDs.**
- An ID is the low 31 bits of the owner word.
- It is valid when 1 ≤ ID ≤ `min(highest, 100000)`. This also bounds every table
  read to index 781, and reserved high bits can never alias an ID.

**Changed signals.**
- A support owner counts as changed when either its texture bit or its table bit
  is set.
- A recorded owner uses the table.
- Overflow tests use `C` plus the classes of changed support owners.

**Support and anchor.**
- The support is the positive-weight 2×2 taps with depth; the anchor is the
  existing nearest of them.
- Clear and mixed coverage take today's current-only exits with depth 0 and
  record `(0,0)`.
- Any invalid or untracked support ID, or an invalid anchor, makes the pixel
  unprovable. It then writes `(0,0)`, never persists, and admits no tap.

**Record.**
- `encode()` writes the canonical form.
- An overflow mask always holds the anchor class, plus the class of every
  joined ID.
- `stored()` rejects:
  - a missing present bit;
  - non-zero reserved bits;
  - an invalid anchor;
  - an exact second ID that is invalid or equal to the anchor;
  - an overflow mask without the anchor class.
- Every one of the nine returns writes defined metadata.

**Static persistence: a separate branch with no anchor match.**
- It requires all of:
  - the camera-static flag and motion under 1/256 px. Only this branch reads
    texel P (`vec2i(p)`) directly;
  - no global flag;
  - a provable current support with no changed owner;
  - P inside the output with admitted colour, alpha 1, and depth matching the
    motion's expected depth;
  - a canonical record whose every recorded owner is unchanged: exact IDs by the
    table, overflow by class.
- **Result:** `clean(mix(current, H_P, options.x))` with no clamp or reactive term,
  and record `A ∪ S ∪ R`.
- **Otherwise** the pixel falls through to ordinary reprojection at the actual
  old position, using `resolve.wgsl`'s unchanged coordinates.

**Ordinary reprojection.**
- Each positive-weight tap inside the output that passes today's checks also
  needs:
  - a canonical record;
  - an anchor equal to the current anchor;
  - for an exact record, no changed owner absent from the support; for an
    overflow record, a class-disjoint changed set.
- Rejected taps leave the mass before renormalisation.
- The clamp adds every admitted 3×3 contributor to the record.
- A malformed clamp contributor makes the record `(0,0)`.

## Check: `repair-model/shader_v5_check.py` (`ac3e48fd…`)

**Method.**
- `shader()` is a statement-by-statement scalar transcription of the
  hash-pinned shader.
- It runs against the hash-pinned d90 reference `resolve()` in
  `correction_v5.py`.
- Check 5 also uses a scalar transcription of the unchanged `ace94f64`
  `resolve.wgsl` as its baseline.
- The output is `repair-model/results-shader-v5.json` (`10e52a44…`).

**Results.** All five checks pass.

| Check | Result |
|---|---|
| Replay: every corrected-mode pixel of the d90 controls (clamp chain, four-record renormalisation, every exit, untracked, malformed and zero-weight pixels) | 15 pixels, all identical, with 0 sub-1/256 displacements. The reference's static "drop" is the shader's single-tap current-only, with the same output. The d90 controls still pass. |
| Seeded fuzz, 20,000 pixels, with `still` consistent with displacement | 0 mismatches, 0 non-canonical records and 0 sub-1/256 displacements. All nine paths are covered, including 743 persist and 1,205 reproject. |
| Shader-only conservatism | A texture changed bit alone turns persistence (0.835) into clamped reprojection (0.25). A header `highest` above 100000 still bounds IDs. Persistence proceeds without an anchor match. With global set, the ordinary tap needs one. |
| Changed-table layout | All 100,000 IDs agree with set membership. |
| Static-camera fallback coordinates, displacement 3/1024 px, history 0.0 at P and 1.0 at P+x | See the table below. |

**Check 5 colours.** In every non-persisting case, the corrected taps are P with
weight 1−3/1024 and P+x with weight 3/1024. Both `45a39859` and the d90 reference
read P alone, with weight 1.

| Case | Baseline `resolve.wgsl` | d90 reference / `45a39859` | Corrected |
|---|---|---|---|
| Global set (root's case 7) | 0.0526367 | 0.05 | 0.0526367: same taps, weights and colour as baseline |
| Changed owner still in support | 0.0526367 | 0.05 | same as baseline |
| Own-texel depth rejected | 0.95 | 0.5 (current only) | 0.95, same as baseline |
| Malformed own-texel record | 0.0526… (no records) | 0.5 | 0.95: the same footprint, but P is refused by its record |
| Complete proof | — | 0.05 (persist) | 0.05 (persist, P only) |

```
python3 -I -B handoffs/0073-upscaling-review/repair-model/shader_v5_check.py \
    --out handoffs/0073-upscaling-review/repair-model/results-shader-v5.json
```

**Reproducibility.** Exit 0, two runs byte-identical, and `/usr/bin/python3`
3.9.6 output identical once the version string is normalised.

**Limits of this check.** The transcription is manual, so it checks the logic
rather than the WGSL compiler. Root's compile and cooker contract are the
remaining source checks.

## Unresolved source dependencies (root-owned)

1. **Kind and cooker:** `temporal_shader.rs` `Kind::UpscaleOwner` and its entry.
   The cooker contract must accept:
   - 9 resources;
   - uint sampled textures at bindings 6 and 7;
   - an `OwnerChanges` with an array member and a 12,528 B span. Today's checker
     expects 16 B per member, so it needs extending;
   - outputs `[Float(0,4), Float(1,1), Uint(2,2)]`;
   - native artifacts and Metal and HLSL maps.
2. **Host bindings and pipeline:**
   - a 9-entry layout and three colour targets (HDR, R32Float, Rg32Uint) with no
     blending;
   - `upscale_input` carrying the owner view;
   - an `Output` record pair (C6 reuse keys).
3. **Header writer:**
   - `C` is the candidate classes;
   - flags are `MotionFrame::camera_static()`, which `ace94f64` already supplies,
     plus root's `global` flag;
   - `highest` is the provenance highest;
   - the candidate words go into the table with a zero tail;
   - the charges and reset follow d90 C6/C7, under the unchanged
     671,105,024 B cap.
4. **Precondition, not certified by the flags:** a fixed renderer configuration.
   This branch has no mutable effect setters, and any later effect composition
   must reset or invalidate history.
5. **GPU control:** the 12-submission proposal is not granted.

## Still open

- the quality failures: edge and sub-pixel (a5ae3179), C7, C9 and P1;
- fixed temporal and MetalFX refusal of the attachment;
- native owner packing;
- appearance, performance, physical memory and phone gates.

No GPU, build or appearance claim is made.
