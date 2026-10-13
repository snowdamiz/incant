# 0075 corrected bloom: GPU verification and visual review

## Status

**Executed once as briefed; the GPU is released.**

- All 8 serial children succeeded. The analyzer, run after release, passes all 27 original gates.
- **Bloom numeric verdict: pass.** The DRS centroid gate that failed in build003 now passes at all three scales, with the original 1.5 / 2.0 / 3.0 px tolerances.
- **Bloom appearance verdict: pass, with one look-dev note that the correction does not touch.** At 75% and 50% the halo keeps its 100% placement, footprint and brightness. I found no disqualifying bloom artifact in the 16 renders.
- **SSAO is not accepted.** Its renders are byte-identical to build003, and its known quality defects remain.
- **Acceptance, integration and phase gates belong to root.** This report approves none of them.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort **max**. Root's ACP transport log `~/.codex/incant-evidence/claude-0075-bloom-validation-acp.log` records "Claude session verified: Opus 5.5; thinking: max" and permission mode bypassPermissions. The log keeps growing during the session, so I cite that line rather than a hash.

**Brief:** `brief.md`, sha256 `f1ca1f4b67eb377c8b7f1d903fe755c79f7c75b8f19374235950d63a8716b3c3`. It is Astra's routing commit `a89f93b3`. There was no priority revision or new director feedback in it, and it did not change during the session.

## Identities (verified before the first child)

| Item | Value |
|---|---|
| Execution checkout | `/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant`, HEAD `f91266b94bb3c7e8dcf8ad30e4becec9cd957e2a`, tree `b8c647f5…` |
| Engine identity | `ENGINE_COMMIT` `d2d2294e6eb3ed9c0c467a436be0f0218006e09f`, engine paths equal to root-reviewed `5d02ad6c` (runner `source_freeze`, re-run before every child) |
| Untracked in that checkout | only the build-generated `tools/effects/Cargo.lock`, `9dcff7a5…`, unchanged before and after |
| GPU grant | `0070-effects-build-004/gpu-grant.json`, sha256 `5716de8327acad53940e0db8b1e886ee54c80bd10b06f3e4766b60645dacb877` |
| Build record | `build/build.json`, sha256 `794464ff8cf8bfaa3b9fdede86377e6f158d67be288f98cbf19fb2f6a3648424` |
| Root build review | `0070-bloom004-build-root-20261013.json`, sha256 `fc5207779e4915f115734a07b4d10d4bb9df55d749bb05d6686a9f31a9af36ff`, verdict "accepted source-frozen CPU build; no actual new GPU yet" |
| Helper digest / plan | `5ec4955232b6e7b41856fa300a3b2776a5ab8b2dc487178d61853196c429cd48` / `fa6067e518204e8cd5b9fdae7223ec7e1eb4981d3dce2e5d38c9a5078729a028` |
| Runner / analyzer | `runner.py` `a5adf3df…`, `analyze.py` `91245f64…`, both unchanged and run by absolute path |
| Predecessor release | `0074-metalfx-late-001/gpu-released.json`, sha256 `168cdac7…`, released 10:36:19.82Z, no owned processes |
| Frozen binaries | `lib` `472b6d93…`, `effects_gpu` `a90f249b…` (both executed); `model_gpu` `1317efda…`, `gpu_host-game` `780ab27f…`, `gpu_host-ui` `64c9ef11…` (not executed). All five match `build.json` on disk after the run |
| Features / toolchain | `incant_render` and `incant_render_gpu_host` `default,physics-deterministic` in both builds; rustc 1.99.0 (from `build.json`; I ran no Cargo) |
| Actual adapter | **Metal, "Apple M5 Pro", IntegratedGpu**, driver strings empty, from 17 helper receipts. Host Mac17,9, macOS 27.0.1 (26A434) per `build.json` |

## Execution (current, build004)

Commands ran from the execution checkout, serially and in the foreground. Each used:
`env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B /Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant/handoffs/0070-ambient-occlusion/tools/runner.py <cmd> --grant /Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004/gpu-grant.json [target]`

| # | Child | PID | Start (Z) | Time | Submissions | Result |
|---|---|---|---|---|---|---|
| 1 | `list lib` | 59784 | 10:45:22.04 | 0.28 s | 0 | 37 / 37 listed, match |
| 2 | `suite lib` | 59865 | 10:45:31.03 | 2.31 s | 425 existing + 20 new | `ok. 37 passed; 0 failed`, including all 3 bloom device tests and the AO device test |
| 3 | `case ssao_core` | 59979 | 10:45:40.69 | 6.73 s | 20 | 18 / 18 in-run checks |
| 4 | `case ssao_materials` | 60039 | 10:45:52.68 | 3.28 s | 10 | 8 / 8 |
| 5 | `case ssao_geometry` | 60064 | 10:46:01.10 | 3.97 s | 17 | 13 / 13 |
| 6 | `case lifecycle_drs` | 60168 | 10:46:10.66 | 2.09 s | 18 | 17 / 17 |
| 7 | `case bloom_numeric` | 60215 | 10:46:18.10 | 2.69 s | 8 | 12 / 12 |
| 8 | `appearance` | 60265 | 10:46:25.70 | 1.40 s | 16, 16 PNGs | 58 / 58 |

Every child ended with exit 0, no signal or timeout, its process group verified absent, its frozen binary unchanged, a valid verdict and `success: true`.

**Totals:** 8 of 16 children, 534 submissions (425 existing + 109 new), 16 PNGs of 32, no retries and no refusals. The largest PNG is 960×540. The execution ceiling is 2,700 s; first launch to release took 70 s.

**Failures:** none. No child failed or latched, no command was refused (`refusals.jsonl` does not exist), no retry ran and no analyzer gate failed.

**Release:** `runner.py release` exited 0 at 10:46:32.21Z.
- `released.json`, sha256 `e1cb312a…`: 8 children, `latched_failures: {}`, no owned or frozen-binary process alive.
- Ledger at release: `7e14f939…`.
- My own check at 10:46:43Z found all 8 recorded PIDs gone, no process executing anything under `bin/`, and no holder of `native-gpu.lock` or `runner.lock` (`evidence/console/10-post-release-process-check.txt`).

**Analyzer, run after release:**
`env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B …/tools/analyze.py /Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004`
- Exit 0 (10:46:51–10:47:21Z): `pass: true`, 27 of 27 gates, no missing case.
- `analysis.json` sha256 `ab129fbbc07f9a8dbdf5ee2980cf1c96f4a5c2e98f37c064e9d46e2cc783b353`.

## Current versus reused checks

**Current (this session, on the build004 binaries):**
- the 8 GPU children above and their in-run verdicts;
- the unchanged analyzer;
- my CPU-only review of retained outputs:
  - hash verification;
  - frame-by-frame comparison with build003;
  - bloom-only measurements;
  - figures.

**Reused, not re-executed:**
- `model_gpu` (50), `gpu_host-game` (5) and `gpu_host-ui` (2). The brief excluded them. Their build002 results (07:57–07:58Z) remain evidence for those earlier binaries only and are claimed for nothing here.
- Root's CPU build review. I verified its hash only.
- The 0070 preparation's CPU checks at `694ebf57`: selftest 26/26, mutations 4/4, analyzer selftest, radiometry 9/9. I did not run them: `selftest.py` would write scratch files into the immutable checkout.
- Build003's appearance notes, for the 14 renders that are byte-identical in build004. I nevertheless inspected all 16 myself.

## What changed from build003

| Evidence | Frames | Byte-identical to build003 | Changed |
|---|---|---|---|
| Appearance renders | 16 | 14 | `v3_both_12` (88,177 px), `v3_both_8` (147,230 px) |
| `lifecycle_drs` | 17 | 15 | `e_bloom_12` (24,595 px), `e_bloom_8` (37,816 px) |
| `ssao_core`, `ssao_materials`, `ssao_geometry`, `bloom_numeric` | 55 | 55 | none |

Exactly the four frames with bloom at a reduced DRS scale changed. Every bloom frame at 100% is unchanged, and so is every frame without bloom. The engine diff is bloom-only, and the frames confirm that scope. Sources: `evidence/review/builds-003-vs-004-*.json`.

## Bloom numeric verdict: pass

**Original gates, unchanged analyzer and thresholds:**

| Gate | build004 | Tolerance or expectation | build003 |
|---|---|---|---|
| `drs_bloom_halo_centroid_on_physical_emitter` 16/16 | (186.16, 83.21), 0.42 px | 1.5 px | 0.42 px |
| … 12/16 | (186.62, 83.36), **0.31 px** | 2.0 px | 5.25 px, fail |
| … 8/16 | (186.77, 83.21), **0.27 px** | 3.0 px | 5.33 px, fail |
| `bloom_coverage` | alpha 0 unchanged; ratio 0.497 | 0.4–0.6 | 0.497 |
| `bloom_threshold` | max change below / at / above = 0 / 0 / 49 | 0, ≤1, >0 | same |
| `bloom_constant_field_levels` | 248 at 1/4/6 levels, 243 off | predicted 248 / 243 | same |

The expected centre is (186.55, 83.06).

**Also current:**
- lib bloom device tests: `actual_pyramid_nonzero_halo_resize_and_zero_alpha_control`, `canonical_graph_zero_gain_and_pixel_camera_bypass_and_invalid_replace` and `bloom_output_keeps_emitter_position_at_reduced_internal_resolution` all pass;
- the helper's in-run checks: halo present and chrome unchanged at 16/12/8, invalid bloom configuration refused, no validation error, no pipeline created after setup.

**Bloom-only measurements, CPU, not gates.** These come from the gate's own single-emitter frames and use the analyzer's centroid pixel set. My tool reproduces build003's published figures exactly (324 / 32.9 px / 12% …), so the two builds are measured identically.

| Scale | Energy | rms radius | Beyond 48 px | Left / right | Up / down | Emitter px | Energy per emitter px |
|---|---|---|---|---|---|---|---|
| 16/16 | 324.1 | 32.9 px | 12.2% | 0.499 / 0.501 | 0.510 / 0.490 | 650 | 0.4986 |
| 12/16 | 334.9 | 33.3 px | 12.7% | 0.490 / 0.510 | 0.511 / 0.489 | 675 | 0.4962 |
| 8/16 | 334.7 | 33.2 px | 12.5% | 0.492 / 0.508 | 0.510 / 0.490 | 676 | 0.4952 |
| build003 12/16 | 378.3 | 39.2 px | 20.8% | 0.475 / 0.525 | 0.451 / 0.549 | 675 | 0.5604 |
| build003 8/16 | 424.7 | 46.1 px | 26.6% | 0.531 / 0.469 | 0.520 / 0.480 | 676 | 0.6282 |

- **The +3.3% energy at reduced scale is geometry, not bloom gain.** At reduced internal resolution the emitter rasterizes to 675 or 676 pixels instead of 650. Energy per emitter pixel stays within 0.7% of 100%; build003 grew it by 12% and 26%.
- **The left/right residual is sub-texel.** It is 1–2% at 75% and 50%, against 0.1% at 100%. The near-field (24 px) centroid sits 0.66 px off at 12/16, where one internal texel is 1.33 output px.

## Bloom appearance verdict: pass

**Renders:**
- **v1:** perspective, 959×539, offset viewport, with UI.
- **v2:** orthographic, 960×540.
- **v3:** v1 at DRS 12/16 and 8/16.
- **v4:** a 640×480 close-up.

**Method:**
- I decoded every PNG to its recorded RGBA hash (`evidence/review/verify-appearance-004.json`).
- I viewed all 16 at true contrast.
- I compared the pairs as gain maps and zoomed crops (figures below).

**Placement and footprint at 100 / 75 / 50%.** The comparison uses composed both-minus-off renders, the only renders available at all three scales; my tool reproduces build003's 90 / 97 / 108 px exactly.

| Scale | Brightened px (in-run pair count) | Energy | rms spread | Centroid |
|---|---|---|---|---|
| 100% (`v1_both`−`v1_off`) | 58,902 | 2727.9 | 90.3 px | (429.3, 243.4) |
| 75% (`v3_both_12`−`v3_off_12`) | 58,968 | 2722.7 | 89.9 px | (429.7, 243.9) |
| 50% (`v3_both_8`−`v3_off_8`) | 59,198 | 2692.4 | 91.0 px | (428.9, 243.0) |
| build003 75% / 50% | 74,534 / 114,773 | 3006.5 / 3374.9 | 96.7 / 108.2 px | (429.0, 243.4) / (430.2, 238.6) |

**What I see:**
- **The same glow at every scale.** It surrounds the peach emissive band, the BLEND pane, the sphere and the stripes with the same extent, brightness and position at 75% and 50% as at 100%. In build003 the reduced-scale glow visibly spread wider and shifted; the before/after figures show it.
- **The glow stays smooth at reduced scale.**
  - Over the dark backdrop the glow gradient remains smooth while the geometry under it shows the expected nearest-expansion stair steps.
  - There are no 2×2 blocks at 50%: horizontal steps of the glow change measure 0.91 / 0.95 for even and odd x, against 0.92 / 0.94 at 100% (`texture-appearance-004.json`).
  - There is no beat pattern at 75% and no ringing or banding at true contrast.
- **The gain-map rings are quantization only.** On the white sphere bloom adds just 1–3 codes, so the rings visible under ×8 gain never show at true contrast.
- **Bloom never darkens:** 0 darker pixels in `v1_bloom`, `v2_bloom` and `v4_bloom`.
- **UI and chrome are untouched.** Chrome, physical UI layout and the opaque UI interiors are identical to `v1_off` for every v1 and v3 variant (helper in-run checks).
- **The far field matches across scales.** Under ×24 gain, the faint outline beyond about 48 px (12% of the energy) is non-circular but the same at 16/12/8. It is identical to build003 at 100% and not visible at true contrast.

**Remaining bloom note (unchanged, look-dev, not a correction defect):**
- **Lit diffuse surfaces bloom like emitters.** The white sphere, under the fixture's key light of 6 against threshold 1.0, glows like a lamp, most visibly in v4. That is a threshold and art-preset matter, the soft-knee proposal in build003's report; it does not block this correction.

**DRS observation outside bloom:**
- **A bright row at the pane's lower edge at 50%.** A two-pixel bright cyan row, (174, 231, 249) at (430, 268–269), appears in `v3_both_8`, and also in `v3_off_8`, which has no effects, and in build003. It is nearest-expansion sampling of thin geometry, not bloom.

## SSAO (explicit, not repaired or accepted)

**This run, numerically:**
- all 13 SSAO reference gates and the identity, determinism, DRS-expansion and mixed-pane gates pass on build004;
- they compare 43,489 interior pixels per reference gate.

**Appearance is unchanged.** The renders are byte-identical to build003, and in v2 and v4 I re-observed:
- the faceted, mottled lower sphere;
- hard-edged patches on the crate face;
- banding on the pillar;
- the comb pattern under the MASK stripes;
- the heavy, broad v4 darkening;
- 2×2 blocks at 50%.

**SSAO remains not visually acceptable at the fixture parameters.** Its repair belongs to the separate SSAO design session.

## Screenshots and figures

**Retained exact copies of the 16 renders:** `evidence/retained/outputs/appearance/*.png`. Hashes are in `evidence/retained-manifest.json` and `evidence/retained/outputs/appearance/index.json`.

**Review aids** (`evidence/review/figures/`; provenance and hashes in `figures-manifest.json`; they are not renders and not acceptance evidence):

| Figure | Content |
|---|---|
| `drs_halo_lifecycle_16_12_8.png` | the centroid gate's frames: bloom-only change ×24 at 16/12/8; red cross = projected emitter, blue ring = 48 px |
| `drs_halo_lifecycle_before_after_12_8.png` | the same at 12/16 and 8/16, build003 against build004 |
| `drs_emitter_true_contrast_16_12_8.png` | the emitter halo at true contrast, ×3 |
| `appearance_bloom_gain_100_75_50.png` | both-minus-off gain maps ×8 for v1, v3@12 and v3@8 |
| `appearance_bloom_gain_before_after.png` | cropped gain maps, build003 against build004, at 100 / 75 / 50% |
| `crop_emissive_panel_box_grid.png`, `crop_sphere_crate_grid.png` | true-contrast ×2 crops: off, 100%, 75%, 50% |

## Changed paths

All are in `handoffs/0075-bloom-validation/`:

- `result.md`.
- `tools/review.py`: stdlib-only CPU review tool. It covers verify, builds, halo, spread, texture, figures, execution and retain. It never writes into an evidence directory.
- `evidence/console/`: each command's console output, exit status and start/end time, plus the post-release process check.
- `evidence/execution-004.json`: per-child facts from the ledger.
- `evidence/retained/`: 37 exact copies, listed with their original hashes in `retained-manifest.json`.
  - The libtest log and the six `lifecycle_drs` RGBA frames are kept byte-exact in deterministic gzip. The libtest log ends in a blank line, and raw RGBA without NUL bytes would be treated as text.
  - Every copy was re-read and hash-matched to its original.
- `evidence/review/`: measurement JSON for build003 and build004, the build comparisons, the figures and their manifest.

**Unchanged:**
- `brief.md` (Astra's commit);
- the immutable checkout (HEAD, status and tool hashes re-verified after the run; no `__pycache__`);
- every external original.

The runner itself wrote only its sanctioned namespace in `0070-effects-build-004`: `ledger.jsonl`, `listings/`, `logs/`, `outputs/`, `cwd/`, `tmp/` and `released.json`. The analyzer wrote `analysis.json` there. Nothing was pushed.

## Limits

- **Desktop only.** Results are Metal on one Apple M5 Pro Mac, ci-profile test and helper binaries. There is no reference-phone, shipping-profile, Vulkan or DX12 result and no performance claim.
- **Bloom isolation at reduced scale.** Bloom-only frames at 75% and 50% exist only as the `lifecycle_drs` single emitter. In the composed appearance renders, bloom at those scales is judged through both-minus-off pairs, which include the same SSAO darkening at every scale.
- **Still pending:**
  - cooked bloom and SSAO, native default exposure, profiling subpass entries and TAA/upscaling;
  - device budgets and phone performance;
  - complete Phase 1 rendering.

## Open questions for root

1. Whether bloom proceeds as an independent merge scope on this numeric and appearance evidence. That is root's decision, and no gate is approved here.
2. Whether to schedule the look-dev threshold work (soft knee or exposure-relative threshold for art presets) separately, so lit white diffuse surfaces stop glowing.
