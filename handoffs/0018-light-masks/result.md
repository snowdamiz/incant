# 0018 light masks: pixel review result

## Status

**Complete. No visual defect found. Appearance is preserved within this
packet's scope.** No code, shader or appearance change was made or needed.

- The two changed images differ from the approved run by one 8-bit level in
  one green channel of one pixel. That pixel sits on a rounding boundary inside
  a smooth gradient. It is not consistent with a missing light.
- Every supplied oracle pair is pixel-identical, including both 4096-light
  full-mask pairs.
- Not approved here: the complete lighting system, Phase 1, or any phase gate.
  Shadows, authored cameras, post-processing, mobile tiers and the full render
  graph remain open. Performance numbers in the brief were not re-measured
  and are not a game or mobile gate.

## Model and transport

- Model: Claude Opus 5.5 (`claude-opus-5-5`), Anthropic.
- Transport: Claude Code through ACP protocol 1, as in handoff 0017.
- No native capture was needed or taken. The brief claims no native UI change.
- No credentials were read. No file outside this worktree was edited. No
  image was published. Review scripts live in `artifacts/review0018/`, which
  git ignores.

## Packet feedback acknowledged

The packet is new for this handoff, with no earlier attempt in this worktree.
I waited for the artifacts to be copied in before reviewing. I treated the
one-level difference as a numerical result to inspect, not as proof of a
missing light, as the brief directs.

## Images reviewed

All 50 PNGs in `artifacts/mask-initial/` were decoded and compared.
These were also viewed directly:

- `full-mask-320x240.png`, `full-mask-257x193.png`, plus a 4x upscaled crop
  of the 320x240 quad.
- `spatial-overflow-96-640x480.png`, the image with the changed pixel.
- `cluster-31-lights.png` and `cluster-127-lights.png`.
- `depth-boundary-patches-640x480.png`, `visible-range-edge-640x480.png`,
  `spatial-clusters-640x360.png`, `spot-penumbra.png` and
  `punctual-hdr-metal-0.3.png`.

The 37 byte-identical images were already approved in handoff 0017. Byte
identity carries that verdict over without a second visual pass.

## Method

1. A standalone pure-Python PNG decoder compared every shared name against
   `artifacts/cluster-final/` channel by channel.
2. Each `-oracle` image was compared with its clustered partner.
3. All nine `cluster-N-lights` images were hashed against each other.
4. A seam scan measured luminance second differences on every 64 px tile row
   and column inside the lit geometry. It compared those lines with the image
   mean and maximum. It also counted clipped pixels.
5. The shader diff was read to confirm only selection code changed. The host
   side rejects mask word counts outside 1 to 128, so the 128-word workgroup
   array cannot overflow.

Commands, run from the worktree root:

```
python3 -I artifacts/review0018/check.py
python3 -I artifacts/review0018/diff2.py artifacts/mask-initial/cluster-127-lights.png artifacts/mask-initial/cluster-128-lights.png
python3 -I artifacts/review0018/seam.py artifacts/mask-initial/full-mask-320x240.png artifacts/mask-initial/full-mask-257x193.png artifacts/mask-initial/spatial-overflow-96-640x480.png
```

## Findings

**Prior comparison matches the brief exactly.** 39 names are shared. 37 are
identical in decoded pixels. The two changed images each have one channel
changed by one level.

| Image | Pixel | Channel | Prior | New |
|---|---|---|---|---|
| spatial-overflow-96-640x480 | 429, 150 | G | 27 | 28 |
| spatial-overflow-96-640x480-oracle | 429, 150 | G | 27 | 28 |

The surrounding 5x3 block is otherwise identical between runs. Green falls
smoothly from 29 to 27 across that row, so both 27 and 28 fit the gradient.
A dropped light would change a contiguous region. The oracle changed in the
same way, and it bypasses culling entirely. So the cause is upstream of
selection. The brief's explanation fits: the rebuilt scene has fresh entity
IDs, which can reorder lights and the floating-point sum.

**Oracle pairs are identical.** These six pairs match pixel for pixel:
depth-boundary 513x385 and 640x480, full-mask 257x193 and 320x240, and
spatial-overflow 513x385 and 640x480.

**Full-mask captures show no missing light.** The upscaled quad shows a
regular lattice of point and spot highlights in varied colors. There are no
holes, tile seams or color breaks. No pixel clips, and the maximum channel is
205 in both sizes. Tile lines score within the range of ordinary lattice
lines in the seam scan.

**No tile or depth seams elsewhere.** In the 640x480 overflow image, every
tile line is below the image mean or near it. The depth patches show smooth
falloff in all 24 patches. The range edge and spot penumbra are smooth.

**The 127-light capture differs by one gray level at the center pixel.** It
reads 107 where the other eight counts read 108. The test splits a fixed
intensity into 127 equal parts, and 8/127 is not exact in binary. The test
tolerance is one level. This is expected rounding, not a selection error.
This image is new, so it has no prior to compare against.

## Limitations and additional evidence

- **The oracle shares light packing with the clustered path.** It is
  independent of culling and selection only. A fault in building the light
  buffer would affect both images the same way. The GPU readback test
  covers this for counts and masks, but I did not run it.
- **Pixels do not prove the final mask bit.** The full-mask lights form a
  32x32x4 lattice about 16 by 16 by 1.5 units, each with a 1.2 range. Some lights, including probably the
  highest-index layer, may not reach visible geometry. Correct selection of
  bit 4095 is shown by the mask readback test in
  `crates/incant_render/tests/lighting_gpu/mod.rs`, not by these images. If
  pixel evidence for the last word is wanted, add a capture where only light
  4095 lights the visible surface.
- No GPU test, workspace validation or timing was re-run by me. The pass
  counts and timings in the brief are Astra's.
- No screenshots were produced by this review. The only derived image is the
  ignored crop `artifacts/review0018/fmz.png`.

## Changed paths

- `handoffs/0018-light-masks/result.md`

## Open questions

None blocking. The final-bit pixel capture above is optional extra evidence.
