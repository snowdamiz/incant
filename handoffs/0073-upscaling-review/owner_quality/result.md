# 0073 owner_quality: execution and appearance review result

**Status:** the run executed once, completed and released. All five groups were accepted on process, contract and artifact checks.

**Appearance verdict: FAIL.**
- OWN, the owner-aware UP, shows no visible improvement over ordinary UP in any retained capture.
- Neither variant is acceptable against nearest-neighbour DRS at 8/16.
- The a5ae3179 visual FAIL and every historical C7/C9/P1/GPU001 failure stand.

No phase gate is approved or claimed.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort Max. Root configured and verified both through ACP before prompting; the supervisor is `0073-owner-quality-supervision-001`. This session runs at reasoning effort max.

## Authority and identities

**Brief**
- Path: `handoffs/0073-upscaling-review/brief.md`.
- Root-owned and left unstaged. sha256 `612a9d245c0f0067c16a992899626d8ac14bb0ed13e324cba780fb8d03dc2a1e`.

**Grant**
- Path: `~/.codex/incant-evidence/0073-owner-quality-gpu-001/grant.json`.
- sha256 `a47643b32981088bf26f4ae85db41ff6d430ab24191771275f233cdf5901a8d8`.
- I hashed it once before the run, and it matched the brief.

**Runner, plan and helper**
- Runner: `owner_quality/execution/runner.py`, commit 8bc0e9a5, sha256 `5eacd386161a8bbc669cf00df0a43450c78553bd9e37a3b675895ff00b75d479`.
- Execution plan: sha256 `f2ffc19ddfd9a616979631e056b4846ad74ba56e0109cb9fe2b1f5ea092f957a`.
- Lifecycle helper: sha256 `b9e12895…2aff34`.
- The execution directory matched commit 8bc0e9a5 byte for byte.

**Binary and sources**
- Frozen binary: sha256 `9644fc96660204aedafd997836fdef557284d40000a06e111c0aefd90b605199`.
- Production commit: `fd5b86d03a2cabfcc9bd5a8441a137645fb858ae`.
- Fixture: `650316a2…5130d997`.
- Quality plan: `ff0f0f48…7bb824dbc4`.

**Predecessor release (GPU002):** `c1bdfed0…9e310899`.

## Execution

The single run:

```
python3 -I -B handoffs/0073-upscaling-review/owner_quality/execution/runner.py run /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/owner_quality/execution/plan.json
```

- **When:** 2026-10-13T18:41:52Z to 18:42:02Z. Exit 0, with no retry.
- **Interpreter:** Python 3.14.8 (`/opt/homebrew/bin/python3`).
- **How it was launched:** as one background tool process, so the tool's 600 s foreground limit could not interrupt a worst case of 5 × 90 s.
- **Console output:** the printed JSON equals `release.json`.

**Preflight (all read-only):**
- the evidence directory held only `grant.json`;
- nothing held `native-gpu.lock`;
- the only other related process was root's ACP supervisor `supervise.py`, which is not a GPU process.

**Release**
- `release.json`: sha256 `261e14eb5a611d32b32ce17a7bbfaa1fd4580c806ce4d8c1527f141216f19729`.
- Fields: `released: true` at 18:42:02.595Z, `accepted: true`, `image_inventory_matches: true`.
- No owned processes or live process groups, and no exception.
- `ledger.jsonl`: `b8f39bfe8c0963ba05e7b8af1300df7b7d7b9d39f79a62e5c8185cfe3b622a45`.
- `debits.jsonl`: `cf5bf53168f060bbfec730413a3ee9fa414ff8a90694801f457644d0b8d1eefd`.
- Every `tmp/<group>` directory is empty, and no frozen-binary process survived.

**Adapter and host:** Apple M5 Pro, Metal, IntegratedGpu, on Darwin 27.0.0 arm64. The adapter was the same for all five children.

**Actual counts**

| Group | Seconds | Scenes = frames | UI | Readbacks = maps | Queue | PNGs | report.json sha256 |
|---|---|---|---|---|---|---|---|
| stationary | 2.528 | 50 | 18 | 50 | 118 | 6 | `d2dbdc0efafdb53c…` |
| motion | 1.201 | 44 | 0 | 44 | 88 | 4 | `a3f10eeae97b998e…` |
| transitions | 2.141 | 106 | 0 | 106 | 212 | 2 | `51336bd70631ef52…` |
| c7_disocclusion | 1.679 | 82 | 0 | 82 | 164 | 2 | `3fdd291442664243…` |
| p1_crossing | 1.882 | 98 | 0 | 98 | 196 | 1 | `53e84d0004129a77…` |
| **Total (5 processes)** | | **380** | **18** | **380** | **778** | **15** | |

Every typed contract check was true and every budget was consumed exactly. Each check covers one owner per run, the expected phases, history, epochs and sizes, the attachment only on OWN, plain FULL and DRS controls, UI interior identity, chrome identity, measurable C7/P1 regions and nonblank frames.

There were no failures. No other GPU work ran: no Cargo, build, test listing, supplemental probe, alternative renderer, screenshot, window or extra image.

## Retained PNGs

All PNGs are under `~/.codex/incant-evidence/0073-owner-quality-gpu-001/`. They are 960×540 RGBA8, and I re-verified each hash after release.

| PNG | Bytes | sha256 |
|---|---|---|
| `stationary/artifacts/stationary-full.png` | 84389 | `992c5543b4f9bbdf6fd703eb764e143f7649836452df28aaac6f1dfc2debceb9` |
| `stationary/artifacts/stationary-drs50.png` | 40131 | `724fa644d3561ff00aa2301fd40ab6b49c4111b0376df5469028ef6265cb40e0` |
| `stationary/artifacts/stationary-up50.png` | 78351 | `1493c6b514f3b528e6d3426016b54ad3189e9fede336b4db6f14cda4adc314ce` |
| `stationary/artifacts/stationary-own50.png` | 78848 | `385b4f424a602a9ffc967c15aff2aedda50c3c51af382368280b9592ee4c192b` |
| `stationary/artifacts/stationary-phases.png` | 11679 | `5850713d0ccaa211eb6f6e43855109469aee427440bc68f71343fb9c4c83ec7b` |
| `stationary/artifacts/stationary-detail.png` | 37846 | `0c32509738d5c4198ce67f69c5b1692a5422b7db3a14a325736e812e6841554d` |
| `motion/artifacts/motion-full.png` | 77464 | `9eeb1222becd6f4d2cec7afa88343ccb528ea9f6e27246cb3410a42bade35b52` |
| `motion/artifacts/motion-up50.png` | 68626 | `fd13d7b902ad8b7bb311a1db5282de2006c9840f3c12b894da38aace626bc073` |
| `motion/artifacts/motion-own50.png` | 68771 | `1c8b673b2b3cb8a757881a1a70407a40b77a875661729ee615407942b3678c53` |
| `motion/artifacts/motion-detail.png` | 101782 | `2c995ca703370760e77e362826ca5b9b0652581c9771028862c64a56ecc2078f` |
| `transitions/artifacts/transitions-full.png` | 72999 | `1be9b9308bb1d404291ec976f0ff1b0c6b3853d3effa0bb2a36fe15270e0f88e` |
| `transitions/artifacts/transitions-strip.png` | 88790 | `4c6ba2fc1d77a582164e65dc47cfbda2e2acfc010e14bcc270098942ddf1da78` |
| `c7_disocclusion/artifacts/c7-detail.png` | 61558 | `a5992bf604f96a25dee3318a8a181c1390179200eec6db9077f4f4e432e3dc7a` |
| `c7_disocclusion/artifacts/c7-own-moving.png` | 264173 | `187bc0ddf3ac826fe0dd845f97f79005544f4c82a8461cd1a6c3283c9fc70aea` |
| `p1_crossing/artifacts/p1-detail.png` | 204560 | `b2018ef7edf00328af5dac693252f743afb7797820f8aef499ed537a255d7d54` |

I viewed all 15 directly with the image viewer, at their native size. I made no crops, enlargements or new images. The figure PNGs carry their own fixed zoom of 2× or 3×.

## Appearance review

### Stationary

Static stage at 8/16. The comparison uses frame 24 with the UI on; the phase strip covers frames 17-24.

**OWN against UP**
- I can't see a difference in the full frames, the detail grid or the phase strip.
- The wires flicker in exactly the same phase pattern in both.

**Against FULL:** both variants are clearly softer.
- **Texture:** on the sign, the 1-texel lines and the 2-texel checker collapse to near-flat grey. The rings become soft white dots, and the 4-texel checker is blurred.
- **Sprite:** the checker sprite has blurred cell edges and smeared corner marks.
- **Wires:** the diagonal wire becomes a row of separate soft dots. The 0.4 px wire is absent in every phase.
- **Phase flicker:** the 0.8 px wire appears in only 4 of the 8 jitter phases, at two brightness levels. The 1.6 px wire dims in 2 of 8 phases. At the real frame rate this is an 8-frame blink on thin geometry, and it is unacceptable.

**Against DRS (nearest)**
- **Where UP and OWN do better:**
  - silhouettes are smoother (the sphere, the mast, the card edge);
  - the 1.6 px wire stays, where DRS drops all three wires at this framing.
- **Where they do worse:**
  - the checker, rings and sprite are clearly softer and lower in contrast than in DRS;
  - DRS keeps a blocky but legible checker where UP and OWN show grey.

This is the a5ae3179 finding, "texture detail below nearest-neighbour DRS", unchanged in both variants.

**Acceptable here:**
- the UI overlay is identical in every variant;
- the text "Ag" and the mixed-script line are as legible as in DRS.

The "physical UI 960x540" label appears clipped at its bottom edge in every variant, FULL included. That comes from the fixture's UI layout, not from upscaling.

### Motion

Object motion runs over steps 9-14 and a camera pan over 15-20.

**OWN against UP:** no visible difference in any of these:
- the mover crops at steps 9, 11 and 14;
- the pan crops at 16 and 20;
- the ×4 error maps against FULL;
- the full frames at step 20.

**Trails:** I see no colour trail or long ghost behind the moving red cube in either variant. The cube's edges are soft and stepped, and the sign texture it uncovers is as soft as everywhere else.

**Pan:** the fence posts are soft and their thin crossbars break up. The ×4 error maps light up every post edge, equally for UP and OWN.

**Dark block (unacceptable):** in the step-20 full frames of both UP and OWN, a dark grey block about 35×20 px replaces the lower ring row on the sign, left of the cube.
- FULL shows crisp rings there.
- No retained DRS frame or crop covers that spot at step 20.
- So I can't determine from the retained images whether it is stale history (for example lighting or shadow) or low-resolution aliasing of the ring texture.
- The owner attachment does not remove it.

**Against DRS:** DRS is blocky and aliased.

### Transitions

Fractional viewport. The schedule runs 8/16, 12/16, 4/16, 16/16 (four frames), 8/16, then a cut.

**OWN against UP:** in the strip, the two look the same at every boundary and at frame 10.

**The 4/16 segment:** the first 4/16 frame wipes the sign's texture to uniform grey in both variants. That is expected at 240×135 internal, but the drop is abrupt.

**16/16 recovery is slow and visibly incomplete**
- The third and fourth 16/16 frames are still far flatter than frame 24 of the separate fixed-16/16 runs, which show clear lines and checker.
- The numbers say OWN recovers more slowly than UP, but in this strip crop I can't see that difference.

**At 8/16**
- UP and OWN at frame 10 are nearly flat on the 2-texel checker.
- DRS keeps a blocky, high-contrast checker there.
- After the cut, both variants restart cleanly.

**Viewport chrome:** the rounded corners and the canvas are intact and identical.

### C7 disocclusion control

The 4-texel checker is static and a white square moves over it at 3 px per frame.

**OWN against UP:** no visible difference.

**Trailing edge:** both show a faint ghost band a few frames wide behind the trailing edge. It is visible as faint cell outlines in the ×4 difference maps, and only as a slightly lighter strip at 1× in `c7-own-moving.png`. It is mild by eye at step 20.

**Static checker:** soft everywhere against FULL. This is acceptable as a disocclusion result only; the softness is the stationary defect again.

### P1 crossing control

A checker panel stops while a bar keeps crossing behind it.

- Neither variant leaves a visible bar residual on the panel after it stops. The only differences in the maps are the bar itself, visible above and below the panel.
- OWN and UP look the same.
- The panel checker is soft against FULL.
- This minimal control doesn't reproduce the CPU model's P1 settings, and it cannot replay per-pixel drops. P1 stays unresolved.

### Verdict

**Acceptable**
- the UI and chrome are stable;
- no strong motion ghosting appears in these scenes;
- the disocclusion trail is mild, and the P1 panel stays clean;
- silhouettes are smoother than DRS's;
- the cut resets cleanly.

**Unacceptable**
- texture and sprite detail below DRS;
- sub-pixel wire blinking with the jitter, and the 0.4 px wire lost;
- the 4/16 detail wipe with slow 16/16 recovery;
- the unexplained dark block in the step-20 motion frame.

**Owner attachment:** it brings no visible benefit in any of these scenes, and no visible regression either. Its numerical differences, below, are small and mixed. The owner-aware path does not earn an appearance claim from this run.

## Measured values, with units

These are rendered-output observations on tone-mapped 8-bit sRGB codes. They inform the review but do not decide it.

- **Contrast:** luminance standard deviation, Rec. 709 on sRGB codes.
- **Region error and activity:** mean of the per-pixel largest channel difference, in codes.
- **Wire presence:** the strongest deviation from the backdrop, in codes; 0 means absent.
- **C7 and P1:** per-pixel mean RGB difference ÷ 255.

**Stationary chart contrast (codes)**

| Chart region | FULL | DRS | UP f24 | OWN f24 | UP last-cycle mean | OWN last-cycle mean |
|---|---|---|---|---|---|---|
| lines_1_texel | 44.4 | 2.4 | 1.39 | 0.37 | 0.46 | 0.44 |
| checker_2_texel | 53.4 | 26.0 | 3.34 | 4.47 | 3.27 | 4.47 |
| rings | 50.9 | 24.6 | 14.7 | 14.9 | 14.7 | 14.9 |
| checker_4_texel | 71.5 | 61.4 | 31.1 | 31.2 | 31.1 | 31.2 |

**Stationary wire presence (codes)**

| Wire | FULL | DRS | UP and OWN, phases 0-7 |
|---|---|---|---|
| 1.6 px | 169 | 0 | 158, 158, 158, 62, 62, 158, 158, 158 |
| 0.8 px | 169 | 0 | 0, 62, 158, 0, 0, 158, 62, 0 |
| 0.4 px | 169 | 0 | 0 in all eight phases |

**Stationary region error against FULL at frame 24 (codes, DRS / UP / OWN)**
- sprite: 5.95 / 8.37 / 8.37;
- sign: 37.3 / 38.8 / 38.4;
- wires: 17.6 / 17.0 / 17.0;
- text: 2.75 / 2.63 / 2.64;
- card edge: 1.17 / 1.07 / 1.07.

**OWN against UP at frame 24:**
- edge pixels: 31% differ, mean 0.41 codes, p99 5 codes;
- interior: mean 0.011 codes.

**Motion region error against FULL (codes, DRS / UP / OWN)**
- mover crop, step 14: 15.5 / 19.28 / 19.27;
- mover crop, step 20: 21.5 / 24.36 / 24.36;
- pan crop, step 20: 1.25 / 1.28 / 1.28.

**Transitions C9 (rendered-output observation only)**

The reference is the finite matched-cycle reference: the mean of frames 17-24 of a separate 24-frame fixed-16/16 run, about 12 / 28 / 27.5 / 55.5 codes for the four regions. No region is undefined.

| Variant | Third 16/16 ratio | Fourth 16/16 ratio | First 4/16 ÷ last 12/16 |
|---|---|---|---|
| UP | 0.287 / 0.284 / 0.500 / 0.599 | 0.473 / 0.385 / 0.553 / 0.684 | 0.004 / 0.017 / 0.743 / 0.510 |
| OWN | 0.239 / 0.255 / 0.471 / 0.508 | 0.399 / 0.341 / 0.516 / 0.544 | 0.004 / 0.015 / 0.735 / 0.511 |

Regions are in the order lines_1, checker_2, rings, checker_4. Every all-region summary is false for both variants against the unchanged 0.75 / 0.90 / 0.90 values. This is not a pass or fail of the historical gate.

**Transitions contrast detail**
- 4-texel checker contrast on 16/16 frames 17-20: UP 24.3, 31.3, 33.5, 38.2; OWN 25.3, 26.6, 28.0, 30.0 codes.
- 2-texel checker contrast at frame 10 (8/16): UP 2.26, OWN 4.30, DRS 25.9, FULL 50.7 codes.
- Frame 24, after the cut, has the same readback hash as frame 1 in both variants, and UP and OWN frame 1 are identical.

**C7 worst trailing column (codes ÷ 255)**

| Step | UP | OWN | FULL |
|---|---|---|---|
| 12 | 0.0738 | 0.0990 | — |
| 16 | 0.0780 | 0.0790 | — |
| 20 | 0.0504 | 0.0533 | 0.0 |

The worst value at step 12 is the first trailing column, where the square started. A value at or below the unchanged 0.06 at step 20 is an observation, not a C7 pass.

**P1 residual (codes ÷ 255)**
- Panel interior after the stop: 0.0 mean and 0.0 worst at steps 16, 17, 19 and 24, for both variants.
- The 6 px margin peaks at 0.2 on steps 16-19 because the bar itself is visible above and below the panel there.
- At step 24 the margin worst is 0.0 for UP and 0.0013 for OWN.

## Historical failures preserved

None of these is closed:
- **a5ae3179:** visual quality FAIL.
- **GPU001:** owner-history case-3 failure (0.40478516 against 0).
- **C7:** linear CPU model 0.0918 against 0.06, failed.
- **C9:** linear CPU model 0.683 / 0.869 against its long-run reference and 0.741 / 0.943 against frame 39, failed.
- **P1:** unresolved; CPU-model worst residuals 0.349, 0.221, 0.296 and 0.063.

These tone-mapped sRGB observations use definitions different from the linear-model gates. The 24-frame C9 reference is finite and does not demonstrate convergence. P1 has no per-pixel drop replay.

## Limitations and pending

**Hardware and path**
- One local Apple M5 Pro on macOS. No phone, Device Farm, performance, memory or phase-gate claim.
- Default portable WGSL path only. Cooked-temporal and native MetalFX are not covered.

**Review method**
- Still frames and fixed figure crops only. Temporal judgments come from the phase strip and per-frame metrics, not from real-time viewing.

**Coverage**
- Most groups run at 8/16. C7 and P1 each use one scale and one setting.

**API limits**
- Jitter phases are aligned only by identical submission counts.
- There is no public readback of owner records or per-pixel decisions, so owner effects are seen only through colour.

**Open item**
- The cause of the step-20 dark block is undetermined.

## Changed paths and commands

**New and committed:** `handoffs/0073-upscaling-review/owner_quality/result.md` and `owner_quality/result.json`.

**Unchanged:**
- the fixture, runner, plan, production sources, binary and evidence;
- the root-owned brief, the top-level `result.md`, and every historical file.

**Left unstaged:** the dirty `brief.md` and the three untracked `owner-attachment-*` inputs.

**Commands run besides the single runner**
- read-only `git`, `shasum`, `ls`, `ps` and `lsof` preflight;
- read-only Python hash and report analysis of the evidence;
- one Python step that wrote only `result.json`;
- image viewing of the 15 PNGs.

**Screenshots:** none new. The retained runner PNGs listed above are the only images.
