# 0073 output-resolution overlays: single granted execution and appearance review

**Verdict:**
- **Overlay route accepted.** Sprites, the mixed-script world text and the BLEND card now render at output resolution and match FULL closely. Order, opaque occlusion and the physical UI are correct.
- **Overall upscaling quality still rejected.** The opaque reconstruction is unchanged, so thin wires, texture detail, silhouettes, the DRS wipe and the C7/C9/P1 failures all stand.

No phase gate is approved or claimed.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP.

## Execution

**The run:**
- **Command:** the exact packet command `python3 …/owner_output_quality/execution/runner.py run …/owner_output_quality/execution/plan.json`, run once.
- **Retries and preparation:** no retry, no rebuild, and no direct test invocation.
- **Duration:** started 18:48 local and released 2026-10-13T22:48:23Z.

**Identities.** All of these re-verified before the run:
- **Plan:** `46189ae9…ae18e854`
- **Runner:** `5bd9fa2f…1406d9c`
- **Grant:** `/Users/andreyyurlov/.codex/incant-evidence/0073-owner-output-quality-gpu-001/grant.json`, sha256 `f90ed59d…40a3c6`
- **Binary:** `30f0d929…032f3210`
- **Production source:** `76be9608e99bf089fcb1e0908827330f70c28ac1`
- **Preparation commit:** `c58256c4`
- **Root review:** `af6f10b8…`
- **Lifecycle:** `b9e12895…`

**Release:**
- **Path:** `/Users/andreyyurlov/.codex/incant-evidence/0073-owner-output-quality-gpu-001/release.json`, sha256 `0073a788791329ed4ebcd88ed58127fb5082a4ff6a127c1d2c808ac510c01be8`.
- **State:** `released: true`, `accepted: true`, no exception, no owned or alive process groups, image inventory matched.
- **Predecessor:** the gpu-002 release `645cd2f1…`.

**Groups:** five, in order, all exit 0 and none timed out, on an Apple M5 Pro (Metal):

| Group | Seconds |
|---|---|
| stationary | 1.27 |
| motion | 1.13 |
| transitions | 2.03 |
| c7_disocclusion | 1.59 |
| p1_crossing | 1.81 |

**Numeric contracts:** all 45 checks pass (9, 7, 9, 10 and 10 per group). They include:
- `ui_button_interior_identical_on_all_eighteen_ui_frames`;
- the UP and OWN owner, phase, history and size checks;
- `attachment_only_on_own`.

**Budget:** consumed exactly as planned:
- 380 scene frames, plus 18 UI submits;
- 380 readbacks and 380 maps;
- 778 queue submits;
- 15 PNGs at 960×540.

Every identity and per-child record is in `result.json`.

## The 15 current PNGs

I re-hashed all 15 and read every one with the image-read tool. For comparison I also re-read the gpu-002 `stationary-up50`, `stationary-detail` and `motion-own50`.

**Ten are byte-identical to the rejected gpu-002 run:**
- stationary: `full`, `drs50`, `phases`;
- motion: `full`, `detail`;
- transitions: `full`, `strip`;
- c7: `detail`, `own-moving`;
- p1: `detail`.

These crops contain no upscaled current-only overlays, or are controls. Being identical, they confirm that reconstruction, history and jitter are unchanged.

**Five changed:**
- stationary: `up50`, `own50`, `detail`;
- motion: `up50`, `own50`.

## Appearance review

**Sprites (detail row 4, full frames):** now crisp nearest-sampled pixel art. Cell and corner pixels are indistinguishable from FULL. In gpu-002 they were blurred with soft corners and halos. The runner's sprite-region error against FULL fell from 8.37 codes to **0.0** for both UP and OWN.

**Mixed-script world text (`ffi سلام 中文`, `Ag`):** sharp glyph edges and counters, matching FULL. In gpu-002 they were soft. The text-region error fell from 2.61 to 0.17 codes (UP) and from 2.62 to 0.19 (OWN). The missing-glyph box after 中文 also appears in FULL, so it comes from the fixture font, not this change.

**BLEND card:**
- The orange translucent card's edges are crisp at output resolution, and its tint and opacity are unchanged. The card-edge region error fell from 1.09 to 0.79 codes; the rest of that crop is reconstructed opaque geometry.
- It composites in front of the grey cube in the same order as FULL.
- I saw no fringe, halo or dark edge, and no NaN, black or saturated pixels.

**Occlusion and order:** text over the fence posts and sprites over the ground match FULL. I saw no overlay drawn over geometry that FULL shows in front, nor the reverse.

**Physical UI:** the "Upscale 50%" button and its label are unchanged and identical across all 18 UI frames (contract pass).

**FULL, DRS, UP and OWN detail and stability (unchanged, still failing):**
- **Thin geometry:** the antenna's three wires are mostly missing in UP and OWN. Only the 1.6 px wire is stable, the diagonal is a dotted line, and the mast is jagged. The phase strip is byte-identical: the 0.8 px wire appears in about half the phases and the 0.4 px wire is absent.
- **Texture detail:** the sign's 2-texel checker and 1-texel lines are still near-flat grey in UP and OWN, while DRS shows aliased contrast.
- **Silhouettes:** the sphere and horizon edges are stair-stepped.
- **Motion and transitions:** the mover and pan crops are byte-identical. The transition strip still wipes to grey on the first 4/16 frame and recovers by the third and fourth 16/16 frames.
- **C7 and P1:** both images and both reports are identical to gpu-002, so their observations are unchanged.
- **OWN versus UP:** OWN remains nearly indistinguishable from UP.

**Metrics that moved because of overlays, not reconstruction:**
- **Transitions:** the whole-frame change at scale boundaries fell (UP 4→16: 24.4 → 17.9 codes; 16→8: 18.0 → 13.1). Output-resolution overlays no longer resample with the internal scale, while the sign-strip images, and with them the opaque reconstruction, are byte-identical.
- **Motion:** the whole-frame UP/OWN-versus-FULL errors and the pan-region number fell for the same reason.

These are tone-mapped sRGB-code observations against the runner's references. They are not the linear CPU-model C7/C9/P1 definitions and cannot close those gates.

## Relation to prior results

- **Rejected comparison:** this run builds on the rejected repair comparison `9843a6c3` (`owner_quality_repair/result.md`). Its reconstruction findings stand unchanged.
- **Historical failures, all still open:**
  - ebc7e690 failed baseline;
  - 9843a6c3 rejection;
  - a5ae3179 visual FAIL;
  - GPU001 owner-history case 3;
  - C7 linear CPU model 0.0918 against 0.06;
  - C9 0.683/0.869 and 0.741/0.943;
  - P1 unresolved.
- **No reconstruction claim:** global history mass is still not per-pixel confidence, and thin geometry and reconstruction are not fixed.

## Limitations

- **Scope:** one local Apple M5 Pro on Metal, portable WGSL path only. No phone, Device Farm, cooked, native-MetalFX, performance or memory claim.
- **Coverage:** these are still frames and fixed crops, judged by eye at native size plus the runner's observations. There was no real-time viewing.
- **No dedicated stress scene:** there is no case for BLEND-over-sprite overlap, overlays partially occluded by slanted surfaces, or a non-finite BLEND output. The guarded sanitation is only exercised indirectly.
- **Untimed pass:** the second light-cluster dispatch is outside the existing AssignLights timestamps.

## Files

**New:** `owner_output_quality/result.md` and `result.json`.

**Unchanged:**
- the engine, the runner, the plan, the fixture and all images;
- every prior packet and result, and the top-level `result.md`.

**Left untracked:** the dirty brief and the three `owner-attachment-*` inputs.

**Commands besides the single runner:**
- read-only `git`, `shasum`, `ls` and `cat`;
- Python reads of the release and report JSON, and one Python step that wrote `result.json`;
- image reads of the 15 current and 3 prior PNGs.

**Scratch:** the runner's stdout and stderr were redirected to `/tmp/0073-output-runner.log`; it is not evidence. No memory was read or written.
