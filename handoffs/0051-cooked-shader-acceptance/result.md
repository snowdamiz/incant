# 0051 result — cooked output and geometry appearance review (Metal)

## Status

Complete within the packet scope. All eight bounded image-review children ran once,
serially, through the unmodified helper. Each exited 0 with the expected PNG count:
24 captures total. The helper withdrew the grant and wrote its RELEASED latch at
`2026-10-12T00:17:45.941527Z`. I confirmed every child PID absent and reported the
release in session output before any CPU-only analysis or documentation. No renderer
call was made after the release.

All 12 default/cooked capture pairs are byte-identical, both as PNG files and as
decoded RGBA. The shared appearance is correct for every reviewed property.
Five limitations, one of them sub-visible, appear identically in both paths, so none is
a cooked regression (see "Defects and observations").

This is a scoped Metal appearance verdict. It is not a phase gate approval or
default enablement, and it is not Vulkan, DX12, device or performance evidence.

## Model and thinking effort

- Model: Claude Opus 5.5 (`claude-opus-5-5`). Thinking effort: Max, the session's
  configured reasoning effort.
- ACP confirmation: root staged `artifacts/tools/0051/acp-settings-confirmation.log`
  (SHA256 `a4fe4889e947e3e810bb0b259256561df750aacf8052ee01c9a7b0f9a37bdabd`). It reads
  `Claude session verified: Opus 5.5; thinking: max`. The runner sends the prompt only after
  ACP returns both values (`tools/handoff/main.py`, `configure_model_and_effort`).
- Per root's `coordination-note.md` (`9ef1b939…`), I did not read the coordinator-owned
  transport log or any account data. No auxiliary reviewer or subagent was used.

## Graphics interval and release

| Item | Record |
|---|---|
| Grant | `gpu-review-ready.json`, `root_grant: true`, granted `2026-10-12T00:12:09.890469Z`. Its SHA256 `058aa21c4ce98fcc5996da3bcf89f5445b7a17af8853dc3f090bec49a9e4c4dd` was unchanged at all eight pre-checks. Its prepared-inputs (`983383a9…`) and helper (`7e6882c3…`) hashes matched the packet files |
| First child start | `2026-10-12T00:16:35.865811Z` |
| Release | `RELEASED.json` (`4dde458f39d8290a47f232202e5a52ec5ab5039cfb5165426abaabe7dab1233d`): reason "all eight bounded children reaped", `all_reaped: true`, `ledger_sha256` `65d90167…` equals `children.json` |
| Gate after release | `gpu-review-ready.json` absent (removed by the helper) |
| PIDs | 8889, 8955, 9036, 9108, 9156, 9576, 9678, 9739: all absent at `2026-10-12T00:17:50Z`. No process referencing `artifacts/tools/0051` remained |
| Helper control files | `capture.lock` (created by the helper), `RELEASED.json` (helper). I created or restored no grant |

Between `masks-default` (ended `00:16:55.718Z`) and `masks-cooked` (started
`00:17:37.624Z`) there is a 42 s pause. During it I read the masks result (two
within-run identical images) before continuing. No child was running then.

Other sessions' `cargo`/`rustc` processes using `/tmp/*` target directories were
running before the interval. I did not check whether they used the GPU. My pre-checks
found no frozen 0051 binary running before any child.

## Children (exact commands and results)

Each pair ran as
`python3 -I -B handoffs/0051-cooked-shader-acceptance/review/preflight.py <case> <mode> && python3 handoffs/0051-cooked-shader-acceptance/capture.py <case> <mode>`.
`preflight.py` is CPU-only and advisory. It re-hashed the child's binary against every
manifest that names it: `staged-inputs.json`, plus `prepared-inputs.json` and
`geometry-v2/manifest.json`, or plus `output-v1/manifest.json` and
`output-v1/source-pairing.json`. It also checked the gate and latch and looked for
stray frozen-binary processes. The helper then ran
`<binary> <test> --exact --ignored --test-threads=1 --nocapture`, with only the listed
evidence variable set to a fresh directory.

| Child | Binary (SHA256) | Test | PID | Start → end (UTC) | Exit | Result (log SHA256) |
|---|---|---|---|---|---|---|
| output-default | `output-v1/default-incant-render-tests` `7ab2bf2d…19cb` | `hdr_output::hdr_values_survive_until_tone_mapping_and_transfer_runs_once` | 8889 | 00:16:35.866 → 00:16:36.801 | 0 | 1 passed, 49 filtered (`97952cd2…`) |
| output-cooked | `output-v1/cooked-incant-render-tests` `5271c38c…9d6a` | same | 8955 | 00:16:40.292 → 00:16:40.793 | 0 | 1 passed, 53 filtered (`dc13c6d3…`) |
| lookdev-default | `geometry-v2/default-model_gpu` `2e0d6d6c…7b9e` | `shadows::lookdev::directional_shadow_lookdev_contact_slopes_and_curves` | 9036 | 00:16:43.623 → 00:16:45.999 | 0 | 1 passed, 49 filtered (`f69f3254…`) |
| lookdev-cooked | `geometry-v2/cooked-geometry-model_gpu` `5dfc98bf…0305` | same | 9108 | 00:16:48.753 → 00:16:50.770 | 0 | 1 passed, 49 filtered (`e8157100…`) |
| masks-default | `geometry-v2/default-model_gpu` `2e0d6d6c…7b9e` | `shadows::alpha_masked_and_reflected_casters_preserve_holes_and_blend_casters_fail_explicitly` | 9156 | 00:16:53.441 → 00:16:55.718 | 0 | 1 passed, 49 filtered (`07659011…`) |
| masks-cooked | `geometry-v2/cooked-geometry-model_gpu` `5dfc98bf…0305` | same | 9576 | 00:17:37.625 → 00:17:39.627 | 0 | 1 passed, 49 filtered (`296d18e1…`) |
| diagnostic-default | `geometry-v2/default-incant_render` `7d6b574d…d94f` | `diagnostic_output::diagnostic_vertices_draw_reflected_geometry_and_preserve_display_formats` | 9678 | 00:17:42.306 → 00:17:42.697 | 0 | 1 passed, 54 filtered (`c24f4c43…`) |
| diagnostic-cooked | `geometry-v2/cooked-geometry-incant_render` `24af19ad…200a` | same | 9739 | 00:17:45.537 → 00:17:45.940 | 0 | 1 passed, 60 filtered (`5539ddee…`) |

Full hashes, commands and per-file hashes are in `evidence/helper/children.json`, an
unedited copy of the ledger. The child logs are in
`evidence/helper/logs/<child>.child-log.txt`. These copies differ from the originals in
two ways: the `.txt` suffix, because the repository ignores `*.log`, and removal of only
libtest's final blank line, so that `git diff --check` passes. `evidence/manifest.json`
records each raw hash, which equals the ledger's `child.log` hash, alongside the
committed hash. The pre-check records are in `evidence/preflight/`.

For geometry-v2, 1 run plus the filtered count matches the staged listings (55 default
and 61 cooked lib tests; 50 model_gpu each). Output-v1 has no staged listing, so its
totals of 50 and 54 come from the logs alone. The four focused tests asserted their
existing analytic checks in both modes, including the 38° acne guard, the mask
hole/reflection/culling probes, the `TransparentShadowCaster` error, and the diagnostic
bounds and upward motion.

Sources: output pair from `550b5635ac04d47ea9eee2f8baacc25ae88b4652`
(`output-v1/source-pairing.json` `f4458bf1…`, matching `prepared-inputs.json`). Geometry
pair from `85e1c5fca86beca95ce33dd8f882d44ed40d7e45` (`geometry-v2/manifest.json`
`b063b8b1…`, 745 source hashes). `crates/` is unchanged between `85e1c5f` and this branch.
`hdr_output/mod.rs`, `output.wgsl`, `tone_map.wgsl` and `fixture.wgsl` are identical
between `550b563` and `85e1c5f`.

Host: macOS 26.6 (25G72), arm64, Apple M5 Pro, 16-core GPU, Metal 4. The four focused
tests do not print the adapter. The numerical receipts from these same binaries record
the wgpu adapter `Apple M5 Pro`, `IntegratedGpu`, backend `Metal`. The output and
diagnostic captures decode byte-for-byte to those receipts' raw readbacks (next section).

## Comparison results (CPU only, after release)

`review/analyze.py` decodes all 24 PNGs with a stdlib decoder. That decoder is validated
by six exact matches with independent raw readbacks. The script compares the pairs and
repeats the tests' probes. Output: `evidence/analysis.json`
(`5b8e79bf68a70ad9f4fe76eda5d42ee969376f12858856634963febda9e9eab8`), which
reproduces byte-for-byte on a rerun.

| Case | Capture | Size | PNG SHA256 (default = cooked) | Decoded RGBA SHA256 |
|---|---|---|---|---|
| output | `hdr-reference-swatches.png` | 384×128 | `de99290cfd11f0af713beaafeb14ca4dbdc410840347c9949ae7f6d772709130` | `9f8f492c60954c350f8f450d18d1c43b0746129239184ebc77b07c7c95653d45` |
| lookdev | `lookdev-sun-12.png` | 640×480 | `9c25abff830a24300190bb608dbdea67f7a7a63883e4b4d792a6dd10479389b7` | `df59747c237bb929d831fee84fa28c6cb96744efdb2eb1f50542a3ae62f20bec` |
| lookdev | `lookdev-sun-38.png` | 640×480 | `be3da8cad70181dd67c2499a6d94d8af843b94ea253d4dd864ced637e94b4eb5` | `283784f2024c24d3a6be61b7d039a3bca98309277754d01e213c80faa5a9ed1c` |
| lookdev | `lookdev-sun-70.png` | 640×480 | `06e846ead6866fae23146c0ac06c0ccf6138f1d3a26ce0e62e7a94b2968d7595` | `5d4d53ac605a077d10f7b80eccbc09b0604daded2dcea83510a5d53d61736297` |
| lookdev | `lookdev-unshadowed.png` | 640×480 | `7fae7886e1067bfa0f0b910f5daaf7ce4a1cb6c07882442d1eb8fbb38b7f095c` | `3bf60feae72467a699336b962cede9a6494714d9fffe66a51edb3a80619d6a32` |
| masks | `shadow-alpha-mask.png` | 640×480 | `51d25bad87d2c36f1a6a0b46a9ccf6ed8e3e4777a8c6f11a8e93572abab16c0c` | `1245a09b45d79d3c42b78bfb5b82d88e804af1f13aab4342e1c22c580f583415` |
| masks | `shadow-reflected-mask.png` | 640×480 | `c79d992829e5805d90b3a3fd47c1fa24a4434cf6dc96a5e04a51d42e60cda2ae` | `944e2a0f0f893eb2bd034f7bf4eb0f18a1d7e5184221cdf2b7e4710fbc6640a9` |
| masks | `shadow-double-sided-back.png` | 640×480 | same as reflected mask | same as reflected mask |
| masks | `shadow-single-sided-back.png` | 640×480 | `2a9abc8efdd367d3515da296fe3876db8d0b73c3233a60e4cffc8b1788ff576b` | `5a516e916b5aa6ce2d6f73b0fb410a63e1df9510a7331fa7536193c977bf46d6` |
| masks | `shadow-blended-noncaster.png` | 640×480 | same as single-sided back | same as single-sided back |
| diagnostic | `diagnostic-base.png` | 160×96 | `46c5129850d0c275fc6c03f0b13a8e153e3a1e2957eb3c871c54dd9dd93ec958` | `72d5dc5db09525812163a0cec39c5e59ec83f5dbece6f2da01b708b071386898` |
| diagnostic | `diagnostic-raised.png` | 160×96 | `98266d7a5bfeee74a0db58ab947ddd330295e47110ee8aff51636e1826c5e286` | `297f944bfacf4627fca44039b61f9891a1efea386a3e2a788bd98d9b0007b58b` |

- **Pairs:** 12/12 PNG files and 12/12 decoded images are identical, with 0 differing pixels.
  The PNGs carry only IHDR/IDAT/IEND, with no colour-space chunks. I reviewed them as
  sRGB-encoded display values.
- **Diagnostic cross-check:** both diagnostic captures, on both sides, equal the
  numerical `diagnostic-{base,raised}-Rgba8UnormSrgb.rgba` readbacks of the same binaries.
- **Output cross-check:** the old output pair's swatch (default and cooked) decodes to
  `9f8f492c…`. That equals the `hdr-swatches` raw hash in that pair's own Metal receipt,
  `docs/spikes/evidence/cooked-output-metal-numerical.json`, whose file SHA256 is
  `af64a4df…`, exactly the receipt named in the brief. It also equals the geometry-v2
  numerical readbacks.
- **Self-checks:** a single 1-LSB pixel mutation and a non-zero ledger exit were both
  detected. All 18 crops round-trip exactly through the encoder. Rerunning
  `review/package.py --check` reproduces the evidence manifest and all 31 retained copies
  exactly; the eight log copies are normalized as described above.

## Appearance verdicts (Claude pixel inspection)

I opened all 24 captures at native size, both sides. I also inspected magnified crops
and difference maps derived on the CPU from those captures. These are not renders; see
"Screenshot paths".

### Output (historical pair `550b563`): pass for the scoped swatch

- The ramp reads cleanly: black, 25, 118, 188, 231, 243. Signals of 2, 4, 16 and 65504
  roll off to 252/254/255/255 with no clipping step.
- All 12 band centres equal the expected sRGB codes exactly, and every 32-px band is flat.
- 0.18 → 118 shows the transfer is applied once; a double encode would be far brighter.
- The HDR colour (4, 0.2, 0.2) becomes a desaturated salmon (254, 158, 158), and
  (65504, 0, 0) resolves to white. That is the curve's path-to-white, not hue clipping.
- Edges are crisp and there is no banding.

This PNG shows only `Rgba8UnormSrgb`. The four-format claim rests on the numerical
receipts, not this image. This is not geometry, Vulkan or DX12 coverage.

### Diagnostic geometry (v2 pair): pass

- Both the plain and the X-reflected cuboid render as solid boxes. Each shows a bright
  front face (230) and a narrower, darker left-facing side (153). The fixture shader
  `0.25 + 0.65·|n·normalize(1,2,3)|` gives intensities of 0.795 (front), 0.320 (side) and
  0.597 (underside) for these rotated faces. The measured swatch curve (0.18→118,
  0.5→188, 0.8→231) maps them to about 230, a little above 149, and a little above 202.
  The concave curve sits above straight-line interpolation, so these agree with the
  measured codes.
- The right cube shows more of its side, as perspective requires. Top edges step down
  away from the near vertical edge (foreshortening).
- Depth and occlusion are correct: no back faces show through, and with
  `cull_mode: None` that is the depth test working.
- In the raised pose, both cubes move up about 10.5 px (mean row 47.5 → 37.0). Both
  reveal a 1-px underside (203) once they are above eye level.
- Edges are crisp (no MSAA). Only the expected face colours appear (two in the base pose,
  three when raised), with no stray pixels. The background is a uniform (20, 21, 25).

### Shadows: lookdev (v2 pair): pass, with shared limitations noted

- **Direction and length:** shadows fall right and toward the camera, consistent with the
  240° azimuth. Length scales correctly with sun elevation: long bands at 12°, medium at
  38°, compact pools at 70°.
- **Contacts:**
  - The 1 m cube's face runs straight into its shadow at 38° and 70°, with no gap.
  - The sphere's shadow attaches beneath it.
  - Post shadows attach at their bases, including the post on the ramp.
  - Exception: the 2 cm thin wall (below).
- **Slopes:** the 25° ramp shades darker than the ground at 38°, as N·L requires. At 12°
  it is back-lit (N·L ≈ −0.02), so it correctly renders at the unlit ambient level. As a
  single-sided plane the light sees from behind, it then casts nothing. That matches the
  engine's culling rule, which the mask fixture also asserts.
  - The post shadow kinks where it crosses from the slope onto flat ground.
  - The ramp's own shadow wedge sits where I hand-projected it.
- **Curves:** the sphere terminator is a clean clamp kink, not a discontinuity. Values fall
  linearly 129→115→97 into the 82 ambient floor. The shadowed and unshadowed profiles
  match within 0–4 codes along the measured diagonal.
- **Cascades and fade:** the far post row thins and fades toward the 60 m shadow distance.
  I saw no cascade seam in the magnified 38° post row or in the 70° frame.
- **Acne:** none visible at 1×. The open ground in rows 360–479 is exactly identical to the
  unshadowed control (0 pixels changed). Sub-visible modulation on curved and sloped
  receivers is listed below.
- **Unshadowed control:** all N·L shading retained and no cast shadows. The 38° capture
  differs from it only by darkening (18,436 px, none brighter).

### Shadows: masks (v2 pair): pass for shadow semantics; main-pass caster not observable

- **Alpha mask:** only the opaque half casts. The result is a solid black rectangle at
  x 175–217, y 200–279, where the analytic projection places it. The hole boundary at the
  texel split is straight, with the same 2–3 px PCF penumbra as the outer edges, and there
  is no dither or leakage inside.
- **Reflected mask:** the footprint moves to the mirrored half, at the same size.
- **Double-sided back:** byte-identical to the reflected mask. For a planar caster, a 180°
  Y rotation is an X mirror, so that is expected.
- **Single-sided back:** casts nothing.
- **Blended noncaster** (`cast_shadows: false`): casts nothing. Its explicit
  `TransparentShadowCaster` refusal is asserted by the test, which passed on both sides.
- **Limitation:** the caster itself is invisible in all five frames. It is a white,
  +Z-facing quad under the same directional light and black environment as the white
  receiver, so it shades identically to what is behind it. The five frames differ only
  inside the shadow footprint (3,440 px). These captures therefore do not show
  main-pass caster visibility or culling. Shadow-pass holes, reflection and
  culling are shown and correct.

## Defects and observations (all identical in default WGSL and cooked; none is a cooked regression)

1. **Thin-wall contact light leak (most visible).**
   - At 38° and 70°, the first ground row below the 2 cm wall's base in the image (the
     ground directly in front of the wall) is lit in 69/69 sampled columns (x 212–280). At x=240 it reads 174 against 182 open ground at 38°, and 203
     against 214 at 70°; the shadow starts one row lower. At 12° the row is lit in 43/69
     columns.
   - At 1× this shows as a thin light seam at the wall base. That is roughly a 3–6 cm gap
     at that distance.
   - The 1 m cube and 0.24 m posts show no gap. This fits the PCF kernel and/or bias
     exceeding an occluder that is thin in light space at the contact. I have not verified
     the cause, and no source change is authorized here.
   - Repro: run the lookdev fixture with either build; compare `lookdev-sun-38.png` against
     `lookdev-unshadowed.png` at column 240, rows 214–220. Alternatively, read
     `observations.thin_wall_contact_gap` from `review/analyze.py`.
2. **Grazing-light stair-steps (12°).** The far end of the cube's long shadow shows about
   three teeth of 6–8 px near (460–485, 355–380). That end lies near world (2, 0, 3),
   which projects to about (442, 362). This is projective aliasing and faintly visible
   at 1×.
3. **Far thin-caster break-up (12°).** Shadows from the far end of the post row break into
   dashes aligned to the far-cascade texel grid before the fade (≈ x 255–345, y 45–75).
   This is expected undersampling. It would likely shimmer in motion, but motion was not
   captured.
4. **Sub-visible self-shadow modulation (38°)** against the unshadowed control. Not
   visible at 1×; motion behaviour is unknown.
   - Sphere lit side: 1,565 px are 1–6 codes darker (899×1, 470×2, 154×3, 35×4, 5×5, 2×6).
   - Ramp interior: 485/1,750 px are exactly 1 code darker.
5. **Faint grazing edge fringe (12°).** There is a 1-px line 3–11 codes darker along the
   cube top face's light-facing edges (interior 130; edges 119–129).

Shared renderer traits, not defects of this change: geometric edges are aliased
(no MSAA), and the black environment makes every umbra pure black in the mask fixture.

## Numerical evidence references (not rerun)

- `handoffs/0051-cooked-shader-acceptance/numerical-acceptance.json`
  (`f5b716093bcc9d6f2744baa96e02b297d214428604a93d1979d41ea9ef95d7d5`).
  138 tests passed (19 lib + 50 model per mode). 16 output and 12 diagnostic raw cases are
  exact. The originals are in `artifacts/tools/0051/numerical/`.
- `docs/spikes/evidence/cooked-output-metal-numerical.json` (`af64a4df…`): the historical
  output-only pair, with 16 exact cases.

## Screenshot paths

- **All 24 captures (ignored):** `artifacts/0051-cooked-shader-acceptance/{output,lookdev,masks,diagnostic}-{default,cooked}/*.png`,
  with the ledger at `artifacts/0051-cooked-shader-acceptance/children.json`.
- **Retained unedited selection (committed):** `handoffs/0051-cooked-shader-acceptance/evidence/captures/`.
  These are 12 default-side byte copies; the cooked files are byte-identical and listed
  with their hashes in `evidence/manifest.json`.
- **Derived inspection aids (ignored, not committed):**
  `artifacts/0051-cooked-shader-acceptance/review/derived/`. These are 18 nearest-neighbour
  crops and 2 amplified difference maps (sun-38 against unshadowed), with source hashes in
  `derived-manifest.json` and in `evidence/manifest.json`. They are CPU transforms of the
  24 captures, not renders. No other images were produced.
- No browser, native UI, system screenshots or windows were used.

## Changed paths

Committed (this packet only):
- `handoffs/0051-cooked-shader-acceptance/result.md`
- `handoffs/0051-cooked-shader-acceptance/review/preflight.py`, `analyze.py`, `views.py`,
  `package.py`. These are CPU-only scripts that launch no GPU children.
- `handoffs/0051-cooked-shader-acceptance/evidence/`:
  - `manifest.json`
  - `analysis.json`
  - `helper/children.json`, `helper/RELEASED.json`, `helper/logs/*.child-log.txt` (8)
  - `preflight/*.json` (8)
  - `captures/*.png` (12)

Ignored, written by me: `artifacts/0051-cooked-shader-acceptance/preflight/`,
`review/analysis.json` and `review/derived/`. Written by the helper:
`artifacts/0051-cooked-shader-acceptance/children.json`, the eight child folders, and in
`artifacts/tools/0051/` the files `capture.lock` and `RELEASED.json`, plus removal of the
gate.

Unchanged: `capture.py`, `brief.md`, `prepared-inputs.json`, `staged-inputs.json`,
`numerical-acceptance.json`, `diagnostic-preparation.json`, all staged binaries,
manifests and numerical files, and the root's notes. No production, shader, layout,
math, test-fixture or dependency edits were made. Nothing was pushed, opened as a PR or
merged. I touched no account and read no credentials.

## CPU-only commands after release

```sh
python3 -I -B handoffs/0051-cooked-shader-acceptance/review/analyze.py artifacts/0051-cooked-shader-acceptance/review/analysis.json
python3 -I -B handoffs/0051-cooked-shader-acceptance/review/views.py
python3 -I -B handoffs/0051-cooked-shader-acceptance/review/package.py
python3 -I -B handoffs/0051-cooked-shader-acceptance/review/package.py --check
```

All succeeded. Notes:
- `views.py` was rerun once from an empty `derived/` folder after its diff-map step moved
  into the script; it reproduced the recorded hashes.
- The first `analysis.json` was regenerated after I added the measured observations. Both
  are my own ignored scratch files.
- The `evidence/` folder was regenerated from the unchanged sources twice: once after I
  renamed the log copies to `.txt`, and once after I dropped the final blank line flagged
  by `git diff --check`. `--check` then reproduced it.
- Pixel probes and inspection used these scripts plus the image viewer, on the captures
  only.

## Pending and unexecuted gates (not claimed)

- **Backends:** actual Vulkan and DX12 driver acceptance. HLSL/DXIL compilation (no
  verified DXC). SPIR-V and HLSL remain CPU-emission evidence only. This review covers
  Metal on one Apple M5 Pro host.
- **Devices:** reference-phone and other hosted-device runs (AWS Device Farm); none were
  executed.
- **Performance:** pipeline caches, recorded per-level permutations, asynchronous
  missing-permutation fallback, and device hitch measurements. There is no timing claim:
  the durations above are process records, not measurements.
- **Default enablement:** `cooked-output` and `cooked-geometry` stay off by default. This
  review does not enable them.
- **Remaining families:** material, sprite and light-grid shader families still translate
  from WGSL at runtime.
- **Motion:** temporal stability (shimmer or crawl of observations 3 and 4) was not
  evaluated, because stills only were in scope.
- **Phase gates:** gate approval remains a human action. I have not marked any gate as
  passed.

## Unavailable tests

- No Vulkan or DX12 adapter, no DXC, no phone, and no motion capture. The mask fixture
  cannot show main-pass caster visibility or culling (see the masks verdict).
- I made no further captures: the brief bounds this interval to 24 images, and the
  interval is closed.

## Open questions

1. Should the thin-wall contact leak (observation 1) be tracked as a shadow look-dev item?
   The lookdev scene includes the 2 cm wall precisely to probe thin-caster leaking.
   Tracking it would be a renderer change outside this packet.
2. If main-pass caster culling needs Claude appearance review, a later, coordinated
   interval would need a caster that is distinguishable from the receiver. For example, a
   tinted or tilted caster through the existing typed fixture path.
3. Do the dashed far-cascade post shadows and the sub-visible curved-receiver modulation
   need a motion capture before default enablement?
