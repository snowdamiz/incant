# 0073 owner_quality_repair: rendered repair comparison result

**Status:** executed once and released. All five groups were accepted on process, contract and artifact checks.

**Appearance verdict: REJECTED.** The repair is a real but partial improvement.

**Better**
- 4/16 → 16/16 detail recovery is visibly faster.
- Interior texture contrast is measurably higher, with no ringing or halos seen.

**Not fixed**
- Texture detail is still well below nearest DRS.
- Thin-geometry blinking and sprite/text softness are exactly unchanged.

**Worse**
- The C7 disocclusion residual is about 24% higher.

**OWN against UP:** still visually indistinguishable.

**Also standing:** the ebc7e690 baseline FAIL and every historical failure. No phase gate is approved or claimed.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort Max. Root configured and verified both through ACP before prompting, and this session ran at reasoning effort max.

## Identities

**Candidate and build**
- Candidate production: `ce6171957eda212d6796335623d8040a0f06b500`. It is fd5b86d0 plus my d4599e90 eight-file patch, five cooker hashpins and rustfmt-only hunks, all applied by root.
- Production freeze: `eaebaccb63aa7dfce86fc8e32af381fd276b3ed2c761f63dbec7607ba55ccf30`.
- Build receipt: `1974b41c…`.
- Source/build audit: `53fb3b23…`.
- Frozen binary: `owner-quality-repair-build-002/upscaling_owner_review`, sha256 `436856967a0ac23d8764fcee6423e33de3ed867da923df2b2388af130ca5f92c`, 18,341,992 bytes, mode 0555.

**Runner, plan and grant**
- Runner: `owner_quality_repair/execution/runner.py`, sha256 `10e0cfde16592e624c93552907d8ed5fe823b817355ca2931aa794b8bec4a721`, commit 6bb00f66.
- Plan: `11fb0f702be37dbd2631922fe9794a1d7a1daeed0f62e3ff39f78e1be8b39013`.
- Lifecycle helper: `b9e12895…`.
- Grant: `0073-owner-quality-gpu-002/grant.json`, sha256 `0b5a664e1202946dfaaef13eaecb6f077d6704201a229ad9d91159abb10a41b9`, matching the brief.
- Predecessor release: `261e14eb…`.

**Fixture and quality plan:** unchanged at `650316a2…` and `ff0f0f48…`.

**Runner change:** against the reviewed predecessor, it differs only in four constants: packet path, producer, freeze and evidence directory.

## Execution and closure

```
python3 handoffs/0073-upscaling-review/owner_quality_repair/execution/runner.py run /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/owner_quality_repair/execution/plan.json
```

**The run**
- **Where:** from the worktree root, using `/opt/homebrew/bin/python3` 3.14.8.
- **When:** once, 2026-10-13T20:16:05Z to 20:16:15Z, exit 0. There was no retry, build or extra GPU work.
- **How it was launched:** as a single background tool process, so the tool's foreground limit could not interrupt it.
- **Console output:** the printed JSON equals `release.json`.

**Preflight (all read-only):**
- the evidence directory held only `grant.json`;
- nothing held `native-gpu.lock`;
- the only related process was root's ACP supervisor, which is not a GPU process.

**Release**
- `release.json`: sha256 `645cd2f1cba857da1eca5f73729ffe5f8313d0b797c383a8dee29de635f33c78`.
- Fields: `released: true` at 20:16:15.459Z, `accepted: true`, `image_inventory_matches: true`.
- No owned processes or live groups, and no exception.
- `ledger.jsonl`: `0f56ef7a…`.
- `debits.jsonl`: `bce14baa…`.

**Closure:** every `tmp/<group>` directory is empty, no frozen-binary process survived, and no process held the GPU lock afterwards.

**Adapter and host:** Apple M5 Pro, Metal, IntegratedGpu, on Darwin 27.0.0 arm64.

**Actual consumption:** 5 processes, 380 scene / 18 UI / 380 readback submissions, 380 maps, 778 queue submissions, 380 frames and 15 PNGs. This exactly equals the plan. The groups took 2.30, 1.28, 2.35, 2.00 and 2.15 s, and every contract check passed.

## Retained PNGs

All PNGs are under `~/.codex/incant-evidence/0073-owner-quality-gpu-002/`. They are 960×540 RGBA8, and every hash re-verified. The last column shows whether the decoded RGBA is identical to the failed ebc7e690 run.

| PNG | Bytes | sha256 | Identical to failed run |
|---|---|---|---|
| `stationary/artifacts/stationary-full.png` | 84389 | `992c5543b4f9bbdf6fd703eb764e143f7649836452df28aaac6f1dfc2debceb9` | yes |
| `stationary/artifacts/stationary-drs50.png` | 40131 | `724fa644d3561ff00aa2301fd40ab6b49c4111b0376df5469028ef6265cb40e0` | yes |
| `stationary/artifacts/stationary-up50.png` | 81441 | `c47c231b3307aa4f34617fc1cc5ad2ddc935614faef3d61f18417f970d18a3d5` | no |
| `stationary/artifacts/stationary-own50.png` | 81888 | `54132e6c2784be45bd2525f8b8e7d2700b691fb60b9e32c33da1ff5051f7186a` | no |
| `stationary/artifacts/stationary-phases.png` | 11679 | `5850713d0ccaa211eb6f6e43855109469aee427440bc68f71343fb9c4c83ec7b` | yes |
| `stationary/artifacts/stationary-detail.png` | 39182 | `1798ba89dc9cb2ad793a9c4f5d51535fa147c8b86f270775a9b0167a36b63357` | no |
| `motion/artifacts/motion-full.png` | 77464 | `9eeb1222becd6f4d2cec7afa88343ccb528ea9f6e27246cb3410a42bade35b52` | yes |
| `motion/artifacts/motion-up50.png` | 70413 | `0bc20b65c3bec370dcc0e16255e2f1dc02f24a5ef19ab6a0db6c05680300f2b0` | no |
| `motion/artifacts/motion-own50.png` | 70026 | `f3fdff6c547536c031c71540d14a45c71b8411cf2baef0c8234fdd51cd4cb5c2` | no |
| `motion/artifacts/motion-detail.png` | 103758 | `ca01cedc4fa740292763152596371fed168442031f28de8055a2c53fec0dffd8` | no |
| `transitions/artifacts/transitions-full.png` | 72999 | `1be9b9308bb1d404291ec976f0ff1b0c6b3853d3effa0bb2a36fe15270e0f88e` | yes |
| `transitions/artifacts/transitions-strip.png` | 107615 | `3231d90e3ec242ae4627e3469b6bef1544ab126f66b7cfdf6078bfa32c04527b` | no |
| `c7_disocclusion/artifacts/c7-detail.png` | 68218 | `667842d540f486ea3728f9ac2042306b20413bafcb4c189188c70951a5164f0d` | no |
| `c7_disocclusion/artifacts/c7-own-moving.png` | 337545 | `8a27a72c7b3a3a60f258a2018c4972d4f311287f76b49e42f1e1495755871364` | no |
| `p1_crossing/artifacts/p1-detail.png` | 197976 | `9c0223081ca5fff3fcc898a4bbc32c1d8ce28c17f1a3e48035daabf597c0e416` | no |

**How I reviewed them:**
- I viewed all 15 at native size with the image viewer, after the release. I also re-viewed the failed run's stationary detail, motion detail, transitions strip and C7 figures for direct comparison.
- I made no crops or new images.
- **What changed, frame by frame:** FULL and DRS are byte-identical to the failed run. Every UP shot changed, and so did every OWN shot except first frames, where all owners count as changed.

## Appearance review

### High-frequency detail (stationary, 8/16)

**Visible:** at full-frame and detail-grid scale, the change is hard to see.
- The 2-texel checker on the sign is still near-flat grey in UP and OWN.
- The sprite badge is exactly as blurred as before.
- The 4-texel checker and the rings are slightly firmer, but I would not have noticed without the before/after pairing.

**Measured:** interior texture contrast rose clearly (section "Measured values"). OWN's sign error against FULL now equals DRS's.

**Verdict: not acceptable.** It is still clearly below nearest-DRS texture detail.

**Sprites and world text did not change at all.** They are current-only overlays, which `composite.wgsl` upsamples bilinearly from the internal resolution. That path is outside this repair, and it is a softness cause my diagnosis did not list.

### Thin geometry and phase dependence

**Unchanged.** `stationary-phases.png` is byte-identical to the failed run.
- The 0.8 px wire still appears in only 4 of 8 phases.
- The 1.6 px wire still dims in 2 of 8 phases.
- The 0.4 px wire is still absent.

**Verdict: not acceptable.** This is expected: mixed-clear history was explicitly deferred, and the 4-row jitter lattice is unchanged.

### Motion (object motion steps 9–14, pan steps 15–20)

**Visible:** the mover and pan crops look essentially the same as the failed run. I saw no colour trail, ringing or halo at the cube's edges.

**Measured:** mover-region error against FULL improved, but is still worse than DRS. The fence-post pan region is unchanged; those are mostly edge pixels on the bilinear current-only path.

**Verdict: not acceptable.** Still softer than DRS, with no visible gain.

### Transitions (8 → 12 → 4 → 16 → 8 → cut → 8)

**Clear visible improvement.** In the strip, UP and OWN on the third and fourth 16/16 frames now show strong diagonal lines and a clear checker, close to the fixed-16/16 frame 24. The failed run was still faint there.

**Unchanged:** the first 4/16 frame still wipes the texture to grey. That is the existing invariant: `camera_static` is false on DRS changes, so the box clamp applies. The 8/16 tiles look much as before.

**Costs:** the pops at the 4→16 and 16→8 switches are numerically larger, because more detail now arrives and leaves at once.

**Verdict:** recovery is acceptable as an improvement; the transitions as a whole are not.

### Step-20 dark block: current shadow versus temporal contribution

`review/shadow_ghost.py` ray-casts the fixture's step-20 geometry per pixel and compares luma in the retained step-20 FULL, UP and OWN frames of both runs.

**Current shadow:** all 770 classifiable pixels of the recorded block lie in the cube's step-20 shadow on the sign.
- FULL, with no history, has the same shadow: mean 66.7 against 142.3 in the lit rings.
- UP and OWN are lighter there, not darker: mean 87.1, against 87.2 in the failed run.
- Their ring texture is flattened: SD 17.6, against 30.7 in FULL.

The block's "flat dark" look is the current shadow with lost texture. The repair did not change it.

**Temporal contribution**
- **Area the cube vacated on steps 15–19:** UP is 9.5 codes lighter than FULL, about the same bias as the lit control (+11.7). No added dark ghost was measured there.
- **Ground, at the trailing edge of the shadow:** UP is 4.6 codes darker than FULL over 15 pixels, a small temporal trail with low confidence.
- **The sign's own shadow-trail strip:** this geometry makes it too small to measure (2 pixels). So ghosting there is not excluded by this run, only not measured.

### C7 and P1 controls

**C7:** the figures look the same as the failed run at the retained 2× zoom. The trailing band is faint, and the static board is still soft against FULL. Numerically, the worst trailing column rose about 24% (below), so it is worse, though not visible here.

**P1:** no bar residual on the panel interior after the stop, unchanged.

### OWN against UP

Still indistinguishable in every retained pair.

On a first frame, where every owner is marked changed, and on moving owners, OWN's provenance rule falls back to the bilinear estimate. OWN therefore gains nothing over UP there.

## Measured values

These are sRGB-code observations on tone-mapped output, taken from the runner reports and the review script. They are not closure of the differently defined linear-model C7/C9/P1 gates, all of which stay failed. Each pair is failed run → repair.

**Stationary chart contrast, frame 24** (luminance SD in codes)

| Region | UP | OWN | DRS | FULL |
|---|---|---|---|---|
| 4-texel checker | 31.1 → 43.9 | 31.2 → 43.5 | 61.4 | 71.5 |
| 2-texel checker | 3.34 → 4.60 | 4.47 → 7.03 | 26.0 | 53.4 |
| Rings | 14.7 → 16.3 | 14.9 → 17.0 | 24.6 | 50.9 |
| 1-texel lines | 1.39 → 1.40 | 0.37 → 0.29 | | |

**Stationary region error against FULL** (codes)
- **Sign:** UP 38.8 → 38.3, OWN 38.4 → 37.3, DRS 37.3.
- **Sprite:** 8.37 → 8.37, against DRS 5.95.
- **Wires:** 17.02, unchanged.

**Stationary activity on the sign** (last-cycle range): UP 3.06 → 3.75, OWN 1.49 → 1.76 codes, so slightly more shimmer.

**Motion mover-region error against FULL** (codes)
- **Step 14:** UP 19.28 → 17.33, OWN 19.27 → 17.48, DRS 15.50.
- **Step 20:** 24.36 → 23.60, DRS 21.54.
- **Pan region:** unchanged, 1.28.

**Transitions C9 rendered-output ratios against the finite matched-cycle reference** (third and fourth 16/16 frames)

| Variant | Region | Failed run | Repair |
|---|---|---|---|
| UP | lines | 0.29 / 0.47 | 0.76 / 1.02 |
| UP | 2-texel checker | 0.28 / 0.38 | 0.78 / 0.84 |
| UP | rings | 0.50 / 0.55 | 0.83 / 0.89 |
| UP | 4-texel checker | 0.60 / 0.68 | 0.83 / 0.88 |
| OWN | all regions | | 0.67–0.83 / 0.85–0.89 |

- **Reference:** the repair also raised the reference itself (2-texel checker 28 → 41).
- **First 4/16 against last 12/16:** worse, 4-texel checker 0.51 → 0.41.
- **Boundary edge change (codes):** at the 4→16 switch 17.4 → 24.4, at the 16→8 switch 13.8 → 18.0.

**C7 worst trailing column** (codes ÷ 255)

| Step | UP | OWN |
|---|---|---|
| 12 | 0.074 → 0.081 | 0.099 → 0.102 |
| 16 | 0.078 → 0.095 | 0.079 → 0.095 |
| 20 | 0.0504 → 0.0623 | 0.0533 → 0.0654 |

**C7 board sharpness** (OWN, step 20): luma SD 70.2 → 73.0, horizontal gradient 8.4 → 8.8.

**P1:** interior residual 0.0 at every step in both runs.

## Historical failures preserved

All of these stay open, with their thresholds unchanged:
- **ebc7e690:** the failed baseline.
- **a5ae3179:** visual FAIL.
- **GPU001:** owner-history case-3 failure.
- **C7 linear CPU model:** 0.0918 against 0.06, failed.
- **C9 linear CPU model:** 0.683/0.869 and 0.741/0.943, failed.
- **P1:** unresolved.

The repaired UP's third-frame rendered ratios all reach 0.75. That is an observation against a finite 24-frame reference; it does not close the historical C9 gate.

## Limitations

**Hardware and path**
- One local Apple M5 Pro (Metal). No phone, Device Farm, performance or memory claim.
- Default portable WGSL path only.

**Method**
- Still frames and fixed figure crops. Shimmer is judged from the phase strip and per-frame metrics, not real-time viewing.

**Known gaps**
- Thin mixed-clear geometry was deferred.
- The global frame mass is not per-pixel confidence.

**Measurability**
- The sign's shadow-trail strip cannot be measured in this geometry.
- The ground-trail sample is only 15 pixels.

**Out of scope:** no follow-up production edits are proposed in this turn.

## Files and commands

**New:**
- `owner_quality_repair/result.md` (this file);
- `owner_quality_repair/result.json` (sha256 `bca73f5f82bd3630b9fd9f7698cf889617e7f2af6986aacc55009346f3fcdffb`);
- `owner_quality_repair/review/shadow_ghost.py` (`463f4952…`) and its output `shadow_ghost.json` (`84b145ee…`).

**Unchanged:**
- earlier packet files and evidence;
- the fixture, runner, plan and binary;
- the top-level `result.md`.

**Left unstaged:** the dirty `brief.md` and the three untracked `owner-attachment-*` inputs.

**Commands besides the single runner (CPU only):**
- read-only `git`, `shasum`, `ls`, `ps` and `lsof` preflight and closure checks;
- in-memory decoding of retained PNGs for the review metrics, with `shadow_ghost.py` re-run byte-identically;
- one Python step that wrote `result.json`;
- image viewing of the 15 repair PNGs, plus selected failed-run PNGs.

**Not written:** no scratch outside the packet, and no memory.
