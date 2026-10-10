# 0031 pixel-perfect cameras: appearance and Inspector result

## Status

The corrected-renderer follow-up is complete. Native review is pending.

- **Exact half-pixel follow flick: fixed.** On the corrected build, a grid-aligned
  object moving with the camera holds its pixels in every pixel-perfect frame of
  the original runs. The foundation build failed 17 of those frames.
- **Odd logical sizes and inherited roll: fixed.** Across 12 new configurations,
  followed geometry holds its expected pixels in 1560 of 1560 follower frames. The
  foundation build managed 1056.
- **No regressions found.** Replication, letterboxing, explicit failures, sub-pixel
  stability, projection switching and repeatability all still pass. Against the
  foundation run, exactly the 34 images that the documented rule changes predict
  have changed, and no others.
- **One defect remains, much narrower than the original.** When the camera sits
  just past a half-pixel tie, within about 1/512 logical pixel, a followed object
  still jumps 1 px for that frame. Exact ties are now correct. The band matches the
  GPU's sub-pixel precision. See "Request for Astra".
- **Browser Inspector: unchanged and passing.** This round made no UI edits.
  Strict typecheck, 420 UI tests, the build and 7 tool tests pass.
- **Native Inspector and the broader editor redesign: pending.** Handoff 0033 owns
  the redesign. Native review follows its integration.

This verdict covers the reviewed camera path and the read-only Camera presentation.
It approves no phase gate. It does not cover sprites, atlases, tilemaps, 2D physics
or an agent-ready claim.

## Model, effort and transport

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. This session's reasoning effort is set to max.
  `tools/acp/config.json` pins model `claude-opus-5-5` with effort `max`.
- The runner started this session in the handoff worktree through the Claude agent
  harness. I cannot inspect the ACP handshake from inside the session, so I cannot
  independently confirm how the effort setting was verified. No model or effort
  substitution happened on my side.
- The first round did not run at Max. That round built the Inspector and the foundation
  engine review, and its session showed a lower reasoning-effort setting of 10. Its
  engine measurements are reproducible from committed helpers, and this Max
  session re-ran the whole engine suite. I did not re-review the Inspector design at
  Max, because handoff 0033 now owns the Inspector redesign.

## Director feedback and priority revision

I read the priority revision before changing anything and applied it:
- This resumed session runs Opus 5.5 at Max thinking.
- Scope is limited to this packet's engine helpers, screenshots, measurements and
  result. No edits were made to editor/ui, Rust, SDK, CI, the bridge or unrelated UI.
- I read Astra's correction at `72ede59` and the updated spike note before any capture:
  - camera-right half ties round down, and camera-up half ties round up;
  - a band of 8 scaled f64 epsilons, capped at 1e-7 px, stabilises exact ties after a roll;
  - odd widths frame half a pixel right, and odd heights frame half a pixel down.
- The original results stay intact as historical evidence. All new output is versioned.
- The original look-dev helper ran byte-unchanged. Any difference therefore comes from the
  binary alone.
- Expectations changed only where the documented rule changes them: the direction of
  +x and −y ties, and the odd-size framing phase. The follower check is strict and
  rule-independent, and it reports the remaining near-tie jump as a failure.

## Provenance

| Item | Value |
| --- | --- |
| Corrected binary | `artifacts/tools/incant_headless` |
| Corrected sha256 | `e88177b23628cccff53e189a62bc189f175cce6978c780f065bad423f01a9cec` |
| Corrected source | `72ede59a7d1a908439f18c38bf35f47b07b5ddd4`, per `binary.json` |
| Foundation binary, baseline only | `artifacts/tools/foundation-incant_headless` |
| Foundation sha256 | `f9a9ed5506cd49b5b2597230b5f34cc8ea35713e6cc57b0321c779abe0fa5cb6` |
| Foundation source | `04a39acf7ff87aa24f4996fc9672fca6b782c00e`, per `foundation-binary.json` |
| Binary check | Each binary matched its metadata before and after every engine call |
| Review base | `2c20e3a` |

I ran the foundation binary only as a labelled before-baseline for the new
scenarios. Its earlier evidence, `engine-r3` and `engine-r4`, is untouched.

## Before and after

### Original follow captures, repeated twice from scratch

These runs use the same helper and scene on both binaries. A mover keeps a fixed
offset from the camera and should hold its home pixels.

| Run, camera step 1/8 px per tick | Mover at home, foundation | Mover at home, corrected |
| --- | --- | --- |
| x: +x, with projection switches | 74 / 85 | 85 / 85 |
| diag: −x, +y | 49 / 49 | 49 / 49 |
| diag2: +x, −y | 43 / 49 | 49 / 49 |

- Every corrected pixel-perfect frame shows the predicted whole-pixel content shift,
  with 0 residual mismatches.
- The content now steps at ticks 5, 13, 21 and so on, rather than 4, 12, 20, because
  a right tie now rounds down. Frames 5 to 12 equal the static +1 px screenshot,
  where previously frames 4 to 11 did.
- Both corrected runs are byte-identical: 341 of 341 images. Their frame-set digest
  is `4a1b6d952a10bb248c227311edb6843081a3ebb9ced058380c1ddaef177a8ac9`.

### Followed geometry at odd sizes and inherited roll

The new helper `pixel_follow.py` builds 12 configurations. These are 4 logical sizes
(320×180, 255×143, 256×143 and 255×144) times 3 inherited rolls (0°, 90° and 0.37 rad).
Each configuration has:
- a camera as the child of a rolled rig;
- two followers, an 8×8 and a 5×3 px quad, rotated with the rig and placed on the
  rolled pixel grid;
- its own copy of the test card, 100 m apart in depth.

One script moves every rig and follower along 16 one-pixel legs at 1/8 px per tick.
The legs cover ±x, ±y and all four diagonals, over −1 to 1 px. Every leg crosses an
exact tie on each moving axis. Each configuration gives 65 frames at logical size.
The rolled 255×143 configuration is also captured at 768×432.

Three checks run on every frame:
- each follower sits exactly at x from `floor(W/2) + r` and y from `floor(H/2) − u − h`,
  where (r, u) is its grid corner and (w, h) its size;
- frames inside one snapping cell are byte-identical;
- the background shows the predicted whole-pixel shift.

| Configuration | Followers exact, before | after | Cells identical, before | after | Background exact, before | after |
| --- | --- | --- | --- | --- | --- | --- |
| 320×180, 0° | 110 / 130 | 130 / 130 | 4 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 320×180, 90° | 110 / 130 | 130 / 130 | 4 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 320×180, 0.37 rad | 106 / 130 | 130 / 130 | 2 / 9 | 9 / 9 | 57 / 65 | 65 / 65 |
| 255×143, 0° | 70 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 255×143, 90° | 70 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 255×143, 0.37 rad | 66 / 130 | 130 / 130 | 1 / 9 | 9 / 9 | 57 / 65 | 65 / 65 |
| 256×143, 0° | 86 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 256×143, 90° | 86 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 256×143, 0.37 rad | 90 / 130 | 130 / 130 | 2 / 9 | 9 / 9 | 57 / 65 | 65 / 65 |
| 255×144, 0° | 86 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 255×144, 90° | 86 / 130 | 130 / 130 | 3 / 9 | 9 / 9 | 65 / 65 | 65 / 65 |
| 255×144, 0.37 rad | 90 / 130 | 130 / 130 | 2 / 9 | 9 / 9 | 57 / 65 | 65 / 65 |
| **Total** | **1056 / 1560** | **1560 / 1560** | **33 / 108** | **108 / 108** | **748 / 780** | **780 / 780** |

The foundation failures fall into three classes. The corrected build has none:
- **Even sizes at 0° and 90°.** Each follower is wrong on 10 of 65 frames. All are
  exact ties where the camera coordinate is a positive x or a negative y.
- **Odd sizes.** World-grid edges land on pixel centres, so a follower alternates
  between two positions within each cell. At 255×143 it is wrong on 30 of 65 frames
  per follower.
- **0.37 rad roll.** Eight frames per configuration land in a different cell than any
  rule predicts. All eight are exact up ties where rolled f64 dot products fell on
  the other side.

Other results:
- The 768×432 capture replicates its logical frame exactly in 33 of 33 frames, with
  1 px of backdrop at the left and top and 2 px at the right and bottom.
- Each binary repeats 813 of 813 frames byte for byte across two from-scratch runs.
- The corrected digest is
  `2701de542d23079453c4dbb9fc0eca0fe1186dd7ca861959e6973f3e911e3ba3`.
- The foundation digest is
  `f7747f725dea8f4756e6107aeeaa324e026488e26c4c0078dc577d3292d1e229`.
- Followers land on `floor(W/2) + r` at odd widths and `floor(H/2) − u − h` at odd
  heights. This confirms the documented direction of the half-pixel framing phase.

### Regression check of the other contracts

All of these come from the unchanged original helper on the corrected binary.
- **Replication and letterboxing.** The table under "Contract measurements" shows 0
  mismatches at every size. The values are identical to the foundation run.
- **Other configurations.** 160×90, 256×224 and 255×143 replicate with 0 mismatches.
- **Explicit failures.** Outputs of 319×180 and 320×179 for the 320×180 camera, and
  320×180 for the 256×224 camera, each exit 1 with the same messages and write no
  image.
- **Static camera motion.** All 14 cameras match the documented rule. Only the two
  tie cameras changed: +0.50 px in x and −0.50 px in y now stay in place, where they
  previously moved one pixel.
- **Roll.** The direct and inherited 90° rolls are byte-identical. A 0.3 px rig offset
  leaves the frame unchanged, and 0.7 px moves it one pixel. The 10° roll replicates
  exactly at 3x.
- **Projection switching.** The orthographic segment differs from a pure translation
  by 21, 38, 106, 213, 112, 46, 28 and 0 pixels. That matches the foundation exactly.
  In the 961×541 run, 22 of 22 pixel-perfect frames are exact 3x replications with 1 px
  of backdrop at the right and bottom. The 4 orthographic and perspective frames fill
  the output.
- **Repeatability.** 341 of 341 images match across the two corrected runs.
- **Against the foundation run.** Of 283 images, 249 are identical and 34 changed. The
  34 are exactly the predicted set:
  - 11 tie frames each in the x run at 1x and at 961×541;
  - 6 tie frames in the diag2 run;
  - the two tie cameras at two sizes each;
  - the odd 255×143 camera at two sizes, from the half-pixel phase.

  No image changed unexpectedly, and none that should have changed stayed the same.

### Real pixels reviewed

I inspected before-and-after crops at 8 to 12 times nearest-neighbour zoom. The
crops are kept in the ignored artifacts folder.
- **x run, tick 4.** Before, the mover sits at x = 39, 1 px left of home. After, it
  is at x = 40.
- **diag2 run, tick 4.** Before, the mover is off by (−1, −1). After, it is home.
- **255×143 follower, tick 2.** Before, the follower sits 1 px right of its pixels.
  After, it is exact. The background stair steps sit half a pixel over, as documented.
- **0.37 rad, tick 36, an exact up tie.** Before, the follower jumps 1 px up. After, it
  holds while the background takes its whole-cell step.
- **Edges.** Edges are hard in every frame, with no blended colours. Blocks are uniform
  in the 3x frames. The 1 and 2 px letterbox is placed as specified.
- **Odd static frame.** The 255×143 frame keeps complete bars with no stretching. Its
  framing moved half a pixel, as documented.

## Remaining defect: near-tie follow jump

The new probe `pixel_neartie.py` places the camera and both followers at a tie plus a
small delta, from 1e-15 to 1e-2 px:
- on the right axis at +0.5 px and at −0.5 px;
- on the up axis at +0.5 px.

That gives 117 positions, one per frame. They run at 320×180 with 0° and 0.37 rad,
and at 255×143 with 0°. Results on the corrected build are the same in all three
configurations:

| Camera position | Followed object |
| --- | --- |
| Exact tie, or within ±1e-15 px of it | Holds its pixels |
| Right axis, tie +1e-14 to +1.9e-3 px | Jumps 1 px left |
| Right axis, tie +1.95e-3 px or more, or any tie − delta | Holds its pixels |
| Up axis, tie −1e-14 to −1.9e-3 px | Jumps 1 px up |
| Up axis, tie −1.95e-3 px | In 2 of 3 configurations, the 5×3 follower jumps or stretches to 5×4 |
| Up axis, tie −2e-3 px or more, or any tie + delta | Holds its pixels |

- 30, 31 and 31 of 117 frames are displaced per configuration. The foundation build
  shows the same band. It is also wrong at some exact ties, and at 255×143 about half
  of all positions are wrong.
- **Cause.** The CPU treats a coordinate as a tie only within 8 f64 epsilons, about
  1.8e-15 px. The GPU snaps vertex positions to its sub-pixel grid. The measured band
  edge lies between 1.9e-3 and 2.0e-3 px, which matches 1/512 px and is consistent
  with 8-bit sub-pixel precision. Inside that band the GPU sees an exact tie and
  applies the top-left rule. The CPU has already snapped the camera the other way.
- **Impact.** Steps in multiples of 1/256 px, such as the original 1/8 px runs, never
  land in the band. Arbitrary continuous motion does. The band covers about 1/512 of
  each pixel. That predicts a single-frame jump on about 0.2% of frames per moving
  axis, or roughly once every 4 seconds of diagonal motion at 60 fps. This frequency
  is an estimate from the measured band width, not a measured rate.
- **Why the existing regression passes.** Astra's GPU regression uses offsets of 0.25,
  0.49, 0.5, 0.51 px and similar. The nearest non-tie is 0.01 px from a tie, outside
  the band.

## Request for Astra

Severity is low to medium. The camera itself stays exact. The jump affects
grid-aligned objects that follow the camera under arbitrary-speed motion, such as a
player sprite.

- **Fix.** Quantize the camera-plane coordinate, in logical pixels, to the rasterizer's
  sub-pixel grid before the tie rule. For example:

  ```rust
  let pixel = (pixel * 256.0).round() / 256.0;
  ```

  Then apply `ceil(pixel - 0.5)` along right and `floor(pixel + 0.5)` along up. CPU and
  GPU then agree on what counts as a tie, and the f64 epsilon band becomes unnecessary.
- **Precision.** I measured 1/256 on this Apple M5 Pro adapter. Other adapters may
  differ, and wgpu does not report the value. It should be a documented constant,
  guarded by a GPU regression that fails explicitly where it does not hold.
- **Second-order case.** Within a few millionths of a pixel of the 1/512 rounding
  boundary, f32 vertex noise can round one edge of a follower differently from the
  other. At 1.95e-3 px, 3.1e-6 px inside the boundary, a 5×3 follower stretched to
  5×4. At 1.9e-3 px, 5.3e-5 px inside, it behaved consistently. This case would remain
  after the fix, but far more rarely.
- **Regression.** Add near-tie offsets on both axes: tie ±1e-12, 1e-6, 1e-3, 1.9e-3 and
  2e-3 px. Run them at 0°, 90°, 0.37 rad and odd sizes. This probe is a ready reference.
- **Wording.** The code comment says a grid-aligned follower "then stays still". That is
  true at exact ties and dyadic steps, but not yet for arbitrary positions.

## Changed paths

This follow-up, all under `handoffs/0031-pixel-perfect-cameras/`:
- `tools/analyse.py`: now takes `--rule` (required), `--repeat`, `--baseline` and
  `--baseline-rule`. Static tie cameras get explicit expected shifts. With
  `--rule away-from-zero` on `engine-r3`, it reproduces every value in the committed
  `analysis-r3.json`.
- `tools/pixel_follow.py` and `tools/analyse_follow.py`: the follow scenarios at odd
  sizes and inherited roll.
- `tools/pixel_neartie.py` and `tools/analyse_neartie.py`: the near-tie probe.
- `tools/summarise.py`: the compact before/after summary and run digests.
- `evidence/followup/`:
  - `summary.json`;
  - `analysis-corrected-r1.json`;
  - `helper-corrected-r1.json` and `helper-corrected-r2.json`;
  - `follow-helper-corrected-r1.json` and `neartie-helper-corrected-r1.json`;
  - both scene definitions and both generated scripts.
- `screenshots/corrected/`: 11 unedited corrected-build frames.
- `screenshots/foundation-baseline/`: 2 unedited foundation frames of the new scenarios.
- `evidence/screenshots.sha256`: 13 entries appended, 49 in total.
- `result.md`.

Unchanged in this follow-up:
- `tools/pixel_lookdev.py` and `tools/capture-inspector.mjs`;
- all earlier evidence and screenshots;
- every UI, Rust, SDK, CI and bridge file.

Earlier round, at `34f348d`: the Inspector presentation and tests, the browser
captures, the foundation engine evidence and helpers.

Ignored and not committed, under `artifacts/0031-pixel-perfect-cameras/`:
- `engine-corrected-r1` and `engine-corrected-r2`;
- `follow-corrected-r1`, `follow-corrected-r2`, `follow-foundation-r1` and
  `follow-foundation-r2`;
- `neartie-corrected-r1`, `neartie-corrected-r2`, `neartie-foundation-r1` and
  `neartie-foundation-r2`;
- the full analyses;
- the probes `follow-probe-1`, `follow-probe-f1`, `neartie-probe-c1` and `neartie-probe-f1`;
- the inspection crops;
- every earlier run.

## Commands and results

```sh
shasum -a 256 artifacts/tools/incant_headless artifacts/tools/foundation-incant_headless
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_lookdev.py artifacts/0031-pixel-perfect-cameras/engine-corrected-r1
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_lookdev.py artifacts/0031-pixel-perfect-cameras/engine-corrected-r2
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_follow.py artifacts/0031-pixel-perfect-cameras/follow-corrected-r1
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_follow.py artifacts/0031-pixel-perfect-cameras/follow-foundation-r1 --binary foundation
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_neartie.py artifacts/0031-pixel-perfect-cameras/neartie-corrected-r1
python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_neartie.py artifacts/0031-pixel-perfect-cameras/neartie-foundation-r1 --binary foundation
# The follow and near-tie helpers also ran into -r2 directories for both binaries.
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse.py artifacts/0031-pixel-perfect-cameras/engine-r3 \
  artifacts/0031-pixel-perfect-cameras/reanalysis-r3-away.json --rule away-from-zero \
  --repeat artifacts/0031-pixel-perfect-cameras/engine-r4
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse.py artifacts/0031-pixel-perfect-cameras/engine-corrected-r1 \
  artifacts/0031-pixel-perfect-cameras/analysis-corrected-r1.json --rule top-left \
  --repeat artifacts/0031-pixel-perfect-cameras/engine-corrected-r2 \
  --baseline artifacts/0031-pixel-perfect-cameras/engine-r3 --baseline-rule away-from-zero
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse_follow.py artifacts/0031-pixel-perfect-cameras/follow-corrected-r1 \
  artifacts/0031-pixel-perfect-cameras/follow-analysis-corrected-r1.json --rule top-left \
  --repeat artifacts/0031-pixel-perfect-cameras/follow-corrected-r2
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse_follow.py artifacts/0031-pixel-perfect-cameras/follow-foundation-r1 \
  artifacts/0031-pixel-perfect-cameras/follow-analysis-foundation-r1.json --rule away-from-zero \
  --repeat artifacts/0031-pixel-perfect-cameras/follow-foundation-r2
python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse_neartie.py artifacts/0031-pixel-perfect-cameras/neartie-corrected-r1 \
  artifacts/0031-pixel-perfect-cameras/neartie-analysis-corrected-r1.json \
  --repeat artifacts/0031-pixel-perfect-cameras/neartie-corrected-r2
# The same near-tie analysis ran for the foundation pair.
python3 -I handoffs/0031-pixel-perfect-cameras/tools/summarise.py artifacts/0031-pixel-perfect-cameras \
  artifacts/0031-pixel-perfect-cameras/followup-summary.json
(cd handoffs/0031-pixel-perfect-cameras && shasum -a 256 -c evidence/screenshots.sha256)   # 49 OK
cd editor/ui && npx tsc -b --noEmit && npx vitest run && npm run build   # 0 errors, 16 files / 420 tests passed, built
python3 -m unittest discover -s tools/tests                              # 7 tests OK
```

Every engine helper run printed these results:
- the binary under test unchanged;
- authored project and journal bytes unchanged;
- every capture completed.

The original-suite runs also printed equal screenshot and play repeats, and the same
three expected failures.

Re-analysing the foundation run under its own rule reproduced every previously
reported value in `analysis-r3.json`, with 0 differences. The adapted analysis
therefore measures the same way; only the rule differs.

| Run pair | Frames | Byte-identical | Digest |
| --- | --- | --- | --- |
| `engine-r3` and `engine-r4`, foundation | 283 | yes | `6b08363b94e0b28edf1cf4937ed0fc943d77a1f6e3b3e50b03abec4f475b387a` |
| `engine-corrected` r1 and r2 | 283 | yes | `4a1b6d952a10bb248c227311edb6843081a3ebb9ced058380c1ddaef177a8ac9` |
| `follow-foundation` r1 and r2 | 813 | yes | `f7747f725dea8f4756e6107aeeaa324e026488e26c4c0078dc577d3292d1e229` |
| `follow-corrected` r1 and r2 | 813 | yes | `2701de542d23079453c4dbb9fc0eca0fe1186dd7ca861959e6973f3e911e3ba3` |
| `neartie-foundation` r1 and r2 | 354 | yes | `502202529a805d1a494192ce87448b54648c3aeee245ef5d32ada0307e84b39c` |
| `neartie-corrected` r1 and r2 | 354 | yes | `e2ed97754f50fe482b3b766fa0e9ad9ff08af182bc223ad57d01d10df3b039b3` |

Each digest is the sha256 of the sorted "capture/frame sha256" lines from the r1 run's
`helper.json`. The original-suite digests cover r1 screenshots and r1 play frames.

Not run by me:
- Cargo, Clippy and the GPU tests, which Astra reports in `binary.json`;
- native capture;
- a browser re-capture, because no UI changed.

## Committed frames

Corrected build, in `screenshots/corrected/`:
- `live-x-1x-t04.png`, `live-diag2-1x-t04.png` and `live-x-961x541-t04.png`: the original
  tie ticks, now with the mover at home. Compare with `screenshots/engine/live-x-1x-t04.png`
  and `live-diag2-1x-t04.png` from the foundation build.
- `ppu32-255x143-960x540.png`: the odd-size framing phase.
- `follow-255x143-r0-t002.png`, `follow-320x180-r0.37-t036.png` and
  `follow-255x143-r0.37-768x432-t036.png`: followers at an odd size, at an exact
  rolled tie, and at 3x with an odd spare.
- `neartie-320x180-r0-f001-right-tie.png`: the follower holds at an exact tie.
- `neartie-320x180-r0-f010-right-tie+1e-6.png`: the remaining defect, a 1 px jump.
- `neartie-320x180-r0-f026-right-tie+2e-3.png`: just outside the band, where it holds.
- `neartie-320x180-r0.37-f103-up-tie-1.95e-3.png`: the band edge, where the 5×3
  follower stretches.

Foundation build, in `screenshots/foundation-baseline/`, using the same names for direct
comparison:
- `follow-255x143-r0-t002.png`;
- `follow-320x180-r0.37-t036.png`.

## Engine scene and contract measurements

These tables come from the corrected build, `evidence/followup/analysis-corrected-r1.json`.
They are identical to the foundation measurements in `evidence/analysis-r3.json`,
except for the documented changes listed above.

**Scene.** The scene is original and authored only through public commands:
1. `init` creates the project.
2. Mathematical glTF is registered with `import`.
3. One `rpc command.execute` transaction with provenance creates every entity.
4. `project.save` and `validate` follow.

The scene is a 10 × 5.625 m "pixel test card" with every edge on the 1/32 m grid.
Materials are emissive, so pixels are flat and independent of lighting. The card
contains:
- a 1 px amber ring on the outermost logical pixels;
- four coloured corner markers;
- bar gauges of 1 to 4 px;
- checkers with 1, 2 and 4 px cells;
- a disc;
- strips at 45°, 1:2 and 17°;
- 1 px crosshairs;
- an 8 × 8 px mover.

Cameras use 32 px/m at 320×180 unless named otherwise.

**Native frame.** The 320×180 frame has exactly 8 colours. All four bar gauges and all
three checkers are exact, all 1000 ring pixels are amber, and every corner marker is
correct.

**Integer replication and letterboxing.** Every output has 0 mismatched pixels. The
letterbox measured from the image alone matches the contract. Odd spare pixels go to
the right and bottom.

| Output | Scale | Left | Right | Top | Bottom |
| --- | --- | --- | --- | --- | --- |
| 320×180 | 1 | 0 | 0 | 0 | 0 |
| 640×360 | 2 | 0 | 0 | 0 | 0 |
| 960×540 | 3 | 0 | 0 | 0 | 0 |
| 961×541 | 3 | 0 | 1 | 0 | 1 |
| 1001×563 | 3 | 20 | 21 | 11 | 12 |
| 1283×723 | 4 | 1 | 2 | 1 | 2 |
| 800×600 (4:3) | 2 | 80 | 80 | 120 | 120 |
| 360×640 (portrait) | 1 | 20 | 20 | 230 | 230 |
| 1280×360 (32:9) | 2 | 320 | 320 | 0 | 0 |

| Camera | Output | Scale | Left / Right | Top / Bottom |
| --- | --- | --- | --- | --- |
| 16 px/m, 160×90 | 320×180 | 2 | 0 / 0 | 0 / 0 |
| 16 px/m, 160×90 | 960×540 | 6 | 0 / 0 | 0 / 0 |
| 32 px/m, 256×224 | 960×540 | 2 | 224 / 224 | 46 / 46 |
| 32 px/m, 255×143 | 320×180 | 1 | 32 / 33 | 18 / 19 |
| 32 px/m, 255×143 | 960×540 | 3 | 97 / 98 | 55 / 56 |

**Camera motion at 1x.** Each frame is compared with the base frame shifted by whole
pixels. Every case has 0 residual mismatches, and every 960×540 version is an exact 3x
replication.

| Camera offset, logical px | Content shift, corrected | Foundation, where different |
| --- | --- | --- |
| x +0.25, +0.49, −0.49; y +0.49; depth +3.3 m | none | |
| x +0.50 (tie) | none | −1 |
| x −0.50 (tie) | +1 | |
| y +0.50 (tie) | +1 down | |
| y −0.50 (tie) | none | −1 |
| x +0.51, +1.00, +1.49 | −1 | |
| y −0.51 | −1 | |
| x +2.3, y +2.3 | −2, +2 | |

**Appearance.** The appearance findings from the foundation review still hold:
- edges are hard and blocks uniform;
- aspect is never stretched;
- the letterbox uses the neutral backdrop `(20, 21, 25)`, which is darker than the
  card background `(48, 52, 63)`;
- the camera is stable inside a cell, while orthographic motion shimmers on curves.

At 16 px/m, art smaller than one logical pixel is sampled honestly: the 1 px checker
fills solid and half-pixel lines vanish. Authors should keep details at least one
logical pixel wide.

## Inspector design, first round

The Inspector presentation is unchanged since `34f348d`. Handoff 0033 now owns the
broader redesign, so this record documents the shipped state only.

The Camera section keeps its schema order:

| Row | Pixel-perfect value | Notes |
| --- | --- | --- |
| Projection | Pixel perfect | |
| Pixel density | `32 px/m` | Indented beneath Projection |
| Resolution `px` | `W 320`, `H 180` | Indented beneath Projection |
| Field of view | `60 °` | Tagged "Unused" |
| Near clip | `m` | |
| Far clip | `m` | |

- **Copy.** "Pixel density" with the schema's `px/m` reads as pixels per metre, and the
  schema help is announced and shown on hover. "Resolution" replaced "Logical
  resolution", which clipped its unit at the 280 px minimum width.
- **Width and height entries.** The prefixes are `W` and `H`, in a neutral colour.
  Their accessible names are "Logical width" and "Logical height".
- **Strict "Unused" rule.** The field-of-view tag appears only for a well-formed
  projection: exactly the three fields, a density from 1 to 4096, two whole sides from
  16 to 4096, and at most 4,194,304 pixels.
- **Malformed data stays visible.** A wrong-length resolution shows "Cannot show as 2
  numbers (logical width and logical height); raw value:". Missing or text values use
  the existing mismatch notices, extra fields are named, and out-of-range values are
  shown as authored with the engine diagnostic on the exact entry.
- **Read-only.** There are no new controls or write paths, and the "Read-only" chip
  stays. Tab order follows the rows.

Tests covering this live in `editor/ui/src/components/inspector/CameraFields.test.tsx`.
They include exact native-schema parity, three variants, the density bounds, the
two-integer `px` resolution, keyboard order, malformed and out-of-range cases, tiny
values, axe checks and a 20-case detection table.

Browser measurement from the first round is in `evidence/inspector-report.json`. It
covers headless Chrome 155, seven states, and 1440×900 and 1000×650 with the
280 px minimum. Across all 14 captures:
- horizontal overflow, clipping, page errors, remote requests and axe violations
  were all zero;
- contrast was 7.98:1 for labels, 16.28:1 for values, 6.96:1 for units and 8.62:1
  for tags.

These are browser captures, not native WebKit evidence.

## Native review, pending

The packet defers native review until handoff 0033's redesign is integrated. When it
is, these native captures are still needed:
- the sample "Pixel Camera" at 1440×874 and at the 280 px minimum;
- a Tab-focus frame on Logical width or Logical height;
- an out-of-range or short-resolution camera;
- a legacy camera, for regression.

I will review them and commit only account-free crops.

## Limitations and open items

- **The near-tie follow jump is open.** See "Request for Astra".
- **Native acceptance is open,** for both the camera presentation and the 0033 redesign.
- **Max effort in the first round.** The Inspector design and the first engine review
  ran below Max. I re-ran the engine work at Max here; the Inspector design was not
  re-reviewed.
- **Sub-pixel precision.** 1/256 was measured on one Apple M5 Pro adapter through the
  headless path. Other adapters were not tested.
- **Untested camera rotations.** Pitched or yawed pixel-perfect cameras were not
  exercised. Only roll about the view axis was.
- **Followers off the grid.** Followers that are not grid-aligned still move by up to
  1 px as they cross pixel centres. That is the stated contract, because objects are
  not snapped.
- **Native chrome masking** at physical resolution is outside the headless path and was
  not exercised.
- **Gate approval** remains human-owned. No phase gate is approved or claimed.
