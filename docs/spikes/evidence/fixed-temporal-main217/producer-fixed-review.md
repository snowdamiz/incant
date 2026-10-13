# 0072 fixed-coverage temporal review — result (executed)

## Verdicts

**Numerical: PASS.**
- **Regression.** The exact regression ran once on root's frozen source004
  default library binary: 1 passed, 0 failed, 169 filtered out, 0.71 s, with
  150 tiny resolve submissions. Its full name is
  `temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection`.
- **Review cases.** All seven pass every contract check.
- **Integrity.** `analyze.py check` reports no problems.
- **Spend.** Exactly 229 submissions (150 numeric + 70 world frames + 9 UI)
  against the ceiling of 256, and 14 PNGs against 16, at 960×540. There was no
  failure, retry or stop, and the release was clean.
- **Scope of the numeric test.** It is a synthetic, post-raster oracle
  comparison: controlled occupancy inputs, its own config (tolerances 0.001,
  reaction 0) and 12×8 targets. It proves the resolve's arithmetic on those
  inputs. It does not prove raster or appearance quality. The findings below
  rest only on the actual scene frames.

**Appearance: the rejected defect is fixed. Remaining cost: new softness.**
I checked each question the brief asked against the original rejection and the
corrected-source run.

| Question | Judgement |
|---|---|
| Static silhouettes | **Improved.** The shimmer is gone and the edges are anti-aliased and stable (finding 1). |
| Moving and disoccluded owners | **Improved, with no stale ghosts** (findings 3–5). |
| Clear coverage | **Correct.** The horizon against clear sky is anti-aliased and its coverage is preserved (finding 2). |
| Thin masks | **Stable, but detail is washed out.** The one-texel MASK grille keeps 18% of its local contrast (finding 1). |
| New erosion | **No coverage erosion.** Every region's linear mean is within ±0.3%. The image is **softer**: about a σ 0.8–0.9 px Gaussian on the reference, against about 0.4–0.55 px before (finding 1). |
| Excluded-overlay contamination | **None.** Overlays match the corrected run, UI is byte-identical, and there is no overlay activity (finding 6). |

**Recommendation.**
- The fix is acceptable for opt-in fixed-mode accumulation. It removes the
  defect that drove both earlier rejections, without ghosts, erosion or overlay
  contamination.
- It is **not yet default quality**, because of the softness, and especially
  thin MASK detail.
- A quality follow-up, not a correctness repair, is proposed below. Root
  decides default enablement and any gate.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max, as this session reports them.
- Grant holder: "Claude Opus 5.5 explicitly Max, root scheduled bounded GPU
  review".

**Brief applied:** "Root priority: execute fixed-coverage GPU review now".
- I verified the named records and no older grant.
- I ran the prepared plan once, in the foreground and serially, and released
  the GPU before the pixel analysis.
- I made no change to source, fixture, helpers, grants or thresholds.
- There was no build, target use, push, memory write or PR. Root's
  `brief.md` stays uncommitted.

**Process note (CPU only, my error).** One pixel-analysis command had an O(N²)
bug and exceeded the tool's 600 s limit. The harness moved it to the
background.
- I stopped it with TaskStop after about ten minutes and confirmed that no
  process remained.
- It involved no GPU, build or evidence write. Its finished part (the activity
  histograms) was later re-produced in the foreground.
- All analysis then ran in the foreground.

## Identity

| Item | Value |
|---|---|
| Source | Commit `e23776df275fa1de81b2027110d9c7b6916999ac` (base `077f7c39`). Freeze `1e9628677f6631476140d7efe75242042721083c4d21966fdfc81755bb193fe0`. **Exact coverage: 2,452 build inputs + 1 probe row.** `build/source.json` `f85c6d75…`, tree `b876ed43…`. |
| Root's source and CPU review | `temporal-coverage-source-cpu-final-20261013.json` `678e67b0…` |
| Root build receipt | `/Users/andreyyurlov/.codex/incant-reviews/0072-coverage-build-root-20261013.json` `b7f0ee24396457faf5c240d47691c362afc98c0b756bae17756c3fdc8766275c` (`accepted_cpu_build_for_scoped_gpu`) |
| Grants | `review-ready.json` `e83f97da3017a947e8e84ff29d468da5cbd2978e64700ca3974d41d6ddc039f2`; `build-ready.json` `a62dbb56…` |
| Build record | `build/review.json` `037ffc22438c05d93acda0c9feed9e298348e64da0a4e27654de6941357c02cc`. It is bound by the grant and names `b1327f3d`, plan `bd480ba1…` and fixture `9841c6b1…`. |
| Review binary ("fixture 27756c0d") | `temporal_visual_review` `27756c0dad957287c328f690d15ef14565dd3fb83be3156275fefa042c934d64`, 18,231,000 bytes, mode 0555; features `default`, `physics-deterministic`. Strict Clippy exited 0 (`build/lint.json` `0ebf04b6…`). |
| Numeric binary | Root-frozen `…/temporal-coverage-001/frozen-binaries/source004/default/render-tests` `d900fe08fb2023b42aca223868e6ac367cc5c85b7d8fdaeaa55e2e49c3d19537`, mode 0755 |
| Visual tree | `b1327f3d9abbaf10f4ba8701dc4ef81231b6d60e` (HEAD `91c59e86`; `visual/` clean before every child) |
| Pins | 0074 `gpu-released.json` `81aa15a6…`: released 09:42:53Z with 8 children, none owned or alive. Build lease `ffdee29b…`: released 09:59:46Z by root. |
| Toolchain and host | rustc/cargo 1.99.0; macOS 27.0.1, Mac17,9, Apple M5 Pro |
| Adapter (recorded) | **Apple M5 Pro, Metal, IntegratedGpu**; the required backend, Metal, was enforced |
| Review config | history 0.9, relative depth 0.05, absolute depth 0.02, reactive threshold 4.0, strength 0.5. This is unchanged since the original review. |
| Comparability | The fixture is byte-identical to the corrected run's. All five temporal-off reference PNGs are byte-identical across the original, corrected and coverage runs. The cut and DRS captures are byte-identical to the corrected run's, as expected for history-free frames. |

## Execution

The GPU children ran from 10:03:30Z to 10:04:16Z, all in the foreground,
serially, under the global non-waiting `native-gpu.lock`, which had no other
holder.

| Step (pid) | Result |
|---|---|
| `gpu.py list lib` (44859) | **41** ignored tests listed, exactly as prepared; the exact name appears once |
| `gpu.py numeric` (44935) | **1 passed, 0 failed, 169 filtered out, 0.71 s**; debit 150; `exact_test_ran_once_and_passed` |
| `gpu.py list review` (44957) | 7 cases, matching the plan |
| `review probe` (44983) | Pass: 0 frames |
| `review stationary` (45012) | Pass: 17 frames, 3 PNGs |
| `review motion` (45026) | Pass: 16 frames, 4 PNGs |
| `review excluded` (45043) | Pass: 9 frames + 9 UI, 3 PNGs |
| `review transitions` (45086) | Pass: 13 frames, 2 PNGs |
| `review skinned` (45098) | Pass: 7 frames, 2 PNGs |
| `review timing` (45126) | Pass: 8 frames |
| `gpu.py release` (10:04:19.585Z) | 12 children (root's 2 builds + 10 GPU). Debited **220 frames + 9 UI = 229** and **14 PNGs**. No stop; no owned or live process; no orphan debit; the grant was unchanged at release. Ledger `10dcb338…`, debits `707abc64…`, latch `730249ba…`. |
| `analyze.py check` | **pass**, no problems. Summary `c126ad9d…` (10:04:24Z). |
| `analyze.py publish` | 14 screenshots, 49 evidence files and `manifest.json` `e337bc34…` |

No owned process or lock holder remained after the release.

## Appearance findings

**Method.** I personally inspected all 14 PNGs at full frame. I also inspected
3–4× crops of the horizon, posts, sphere, grille, cube, moving cube, pan,
skinned vertex and overlays, comparing off, original, corrected and coverage.
The figures come from two sources:
- the fixture's own reports;
- my CPU-only scripts in `temporal-coverage-inspection/`, run on the published
  PNGs, with exact commands in `commands.txt` and outputs alongside.

The "original" run is `9009f088` and the "corrected" run is `535003d7`.

### 1. Stationary convergence — shimmer fixed; softer; thin MASK detail washed out

**Flicker** (fixture statistics; 34,327 edge pixels):

| Metric | Original | Corrected | **Coverage** |
|---|---|---|---|
| Consecutive edge mean, frames 9–16 | 13.3–24.2 | 10.6–16.1 | **2.0–3.3** |
| Consecutive edge max, frames 9–16 | 169 | 99–119 | **28–44** |
| Interior pixels changed per frame, frames 9–16 | 2,706–7,021 | 33–84 | **8–54** |
| First history frame (f1→f2): edge mean / interior changed | 24.9 / 6,833 | 16.3 / 665 | 10.6 / 650 |
| Same phase f8 vs f16: edge / interior changed (max) | 614 (6) / 1 (3) | 889 (10) / 0 (2) | 3,063 (7) / 17 (4) |

From frame 3 onwards the coverage edge mean is 2.0–3.8 and the edge max 28–54.
Edges converge more slowly than before (blends still settle by at most 7 steps
between same-phase frames), but they do converge.

**Activity map** (frames 9–16; pixels by peak consecutive change):

| Peak change | Original | Corrected | **Coverage** |
|---|---|---|---|
| 33 steps or more | 11,143 | 12,061 | **159** |
| Saturated (64 or more) | 10,246 | 3,631 | **0** |
| 17–32 | 2,691 | 2,818 | 711 |
| 3–16 (edge) | 5,888 | 6,364 | 23,562 |

The flicker energy moved from large to small changes. That is the expected
residual of a coverage blend at history weight 0.9.

**What I see:**
- **Before.** The original and corrected activity maps had a bright one-pixel
  rim on every silhouette and a saturated MASK grille.
- **Now.** The rims are dim. The horizon is the brightest remaining line, at
  28–44 steps per frame. The grille is moderate speckle, not noise.
- **Magnified, silhouettes are anti-aliased.** At 3–4×, the horizon diagonals,
  the sphere outline, the cube edges and the posts are smooth and graded
  (example: the horizon pixel rows 0.36 / 0.82 / 0.98 coverage). The other
  three panels show single-sample stair-steps.

**Coverage and softness** (linear light; detail = mean |L − 3×3 mean|; my
scripts):

| Region | Mean vs off: original / corrected / **coverage** | Detail retained: original / corrected / **coverage** | Reference blurred to match |
|---|---|---|---|
| MASK grille | −0.3% / −0.7% / **+0.3%** | 100% / 77% / **18%** | σ ≈ 0.9 px |
| Posts + shadows | +0.1% / −0.1% / **0.0%** | 84% / 73% / **43%** | σ ≈ 0.9 px |
| Far posts | +0.5% / +0.2% / **−0.3%** | 82% / 69% / **35%** | σ ≈ 0.9 px |
| Horizon diagonal | +0.3% / 0.0% / **−0.1%** | 100% / 85% / **41%** | σ ≈ 0.85 px |
| Sphere edge | 0.0% / −0.2% / **0.0%** | 90% / 77% / **45%** | σ ≈ 0.8 px |
| Flat ground (control) | −0.1% / −0.1% / −0.1% | 98% / 98% / 98% | — |

**Reading the table:**
- **No erosion or dilation.** The mean coverage of thin features and edges is
  unchanged (within ±0.3%).
- **Softer than before.** The image matches a discrete Gaussian blur of the
  reference of about σ 0.8–0.9 px. The corrected run matched about 0.4–0.55 px.
  Some loss is inherent in replacing stair-steps with anti-aliased edges, but
  this is softer than that alone.
- **Thin MASK detail suffers most.** At 3× the one-texel grille reads as a
  low-contrast green haze. At 1:1 it is a soft mesh, and its lattice is barely
  visible. In the 6-frame pan it is crisper, with less accumulated history.
- **Thin features stay visible.** Thin post shadows remain visible at 1:1 and
  fainter at magnification.

### 2. Clear coverage — horizon against clear is anti-aliased and preserved

These are linear-light profiles across the flat top of the ground at
x 420–510 (rows 29–41):

| Run | Rows 35 / 36 / 37 | Integrated coverage |
|---|---|---|
| Aliased reference | 0 / 1 / 1 | 6.00 |
| Corrected | 0 / 0.87 / 1 | 5.87 |
| **Coverage** | **0.36 / 0.82 / 0.98–0.99** | **6.16–6.17** |

**Reading the profiles.** Clear now blends as far-plane coverage. The
+0.17-row difference is within the ±0.5 px quantization of the aliased
reference's true edge. The edge spans about three rows, which is the softness
from finding 1.

**Registration (my scripts, Lucas–Kanade):**
- **Linear light, coverage run:** horizon −0.10 px, posts −0.07, grille +0.03,
  shadows −0.01, sphere (−0.09, +0.01).
- **The cube-and-wall region:** −0.11 stationary and −0.20 moving, the same as
  the corrected run's −0.19.
- **Global sRGB-luma fit:** −0.32 px for stationary and the moving object. That
  is a measurement artifact: correct linear-light anti-aliasing of dark sky
  over bright ground looks pushed towards the dark side in sRGB when compared
  with an aliased reference. The horizon band alone reads −0.46 in sRGB and
  −0.10 in linear light.
- **Skinned (+0.10) and excluded (0.00)** match the corrected run.

So there is no registration defect.

### 3. Object motion and disocclusion — no stale ghost; moving edges anti-aliased

Screenshots: `motion-object-on-f8.png` and `motion-object-off-f8.png`. The
bright cube moves about 21 px/frame to the right, behind the sphere.

**Away from edges** (at least 3 px from any edge of either image, edge
threshold 12):

| Run | 3–8 steps | 9–16 steps | More than 16 |
|---|---|---|---|
| Original | 918 | 45 | 0 |
| Corrected | 823 | 30 | 0 |
| **Coverage** | **830** | **30** | **0** |

All 9–16-step pixels are the known moving-shadow-lag patch (x 579–600,
y 291–305; max 10, unchanged).

**At the edges** (orangeness R−B relative to off; cube–wall contrast 120.7):

| Edge | 1 px | 2 px | 3 px |
|---|---|---|---|
| Trailing (vacated) | +47.7 | +6.0 | +0.1 |
| Top (perpendicular to motion, control) | +44.3 | 0 | 0 |
| Leading | +80.7 | +31.8 | +6.6 |

- **Trailing edge.** It matches the no-motion control, so the coverage run
  leaves no trail behind the object.
- **Leading edge.** The extra spread there is the same softness acting on a
  2-px dark strip just right of the cube (raw rows in
  `rows-motion-leading.txt`). It is not a forward smear.
- **Vacated region and occlusion.** The vacated wall is clean at 3×, and the
  occlusion behind the sphere is clean.
- **Flicker while moving.** Consecutive object edge means are 3.4–12.4, against
  12.3–18.0 in the corrected run.

### 4. Camera pan — no smear

Screenshots: `motion-pan-on-f6.png` and `motion-pan-off-f6.png`.
- Off-edge counts are 391 / 0 / 0 (max 7); the corrected run had 240 / 0 / 0.
  The extra are small, 3–8-step differences.
- Consecutive edge means are 18.6–34.8, against 23.1–43.1.
- At 3×, the posts and their shadows move without trailing. Clear pixels beside
  silhouettes adopt foreground motion without smear against the sky.

### 5. Skinned motion — clean

Screenshots: `skinned-on-f6.png` and `skinned-off-f6.png`.
- **No ghost.** There is no off-edge difference (0, as before).
- **Near-edge softness.** The fixture's interior count rose from 235 to 463
  (max 20). All of these lie within 2 px of edges, which is the softness, not a
  ghost.
- **A small pre-existing notch.** At 4× there is a 1–2 px notch on the checker
  boundary near x 615, y 236. It is also present in the corrected run.
- No trail follows the deforming vertices.

### 6. Excluded surfaces — no overlay contamination; UI unchanged

Screenshots: `excluded-on-f8.png`, `excluded-off.png` and
`excluded-activity.png`.
- **UI.** It is **byte-identical to temporal-off in all 8 phases** (fixture
  check).
- **Overlay activity.** Pixels with a peak of 33 steps or more fell from 505
  (corrected) to **0**, and of 17–32 steps from 49 to 0. The per-frame edge max
  fell from 33 to 10–23.
- **Overlays against the corrected run** (my scripts, f8):
  - world text, glass card and sprites: mean 0.01–0.08 steps, max 10–13;
  - only 17–57 pixels above 8 steps, at overlay borders over the anti-aliased
    backdrop;
  - UI button: 0;
  - the backdrop-only control differs more (mean 0.40), which is its own
    anti-aliasing.
- **Magnified.** At 1× and in crops, the text, `.notdef`, sprites and glass are
  crisp and identical across the runs.

### 7. Reset, cut and DRS transitions — unchanged and correct

- **Sequence.** The epochs are 1 / 2 / 3 / 4 for view A, the cut, half-scale DRS
  (480×270) and output scale, with phase 0 after each reset and one owner.
- **History-free frames.** `transition-cut-f7.png` and `transition-drs-f9.png`
  are **byte-identical** to the corrected run's.
- **Accumulated frames.** DRS consecutive edge means are 4.3–14.8 (corrected
  15.7–22.9). The second view-B frame (b8) against off has edge mean 19.6
  (corrected 14.5), which is the softness again.

### 8. Timing — observation only

Each frame produced one sample, with none skipped or abandoned. Median
Models span, temporal off against on:

| Run | Off | On | Change |
|---|---|---|---|
| **Coverage** | 0.814 ms | 1.935 ms | **×2.38** |
| Corrected | — | — | ×2.04 |
| Original | — | — | ×1.89 |

- Shadows, AssignLights and InitializeScene are unchanged.
- Display (0.83 → 1.90 ms) is group overlap.
- `groups_within_frame` is false.
- The coverage resolve's nine motion loads plausibly add about 0.27 ms to the
  Models span here. That is an inference, not a performance measurement; the
  period is 1.0.

## Against my proposed acceptance criteria (`corrected-visual-result.md`)

| Proposed | This run | Met |
|---|---|---|
| At most 3,000 pixels at 33 steps or more (frames 9–16) | 159 | yes |
| At most 100 saturated | 0 | yes |
| Edge mean at most 5; edge max at most 50 | 2.0–3.3; 28–44 (frames 9–16). Frames 3+: 2.0–3.8; max 54 at frames 3–4. | yes after convergence |
| No saturated grille; intermediate coverage at 3× | yes; yes | yes |
| Interiors at most 100 changed per frame | 8–54 (650 on the first history frame) | yes after convergence |
| Same-phase f8/f16 interior 0 | 17 pixels, max 4 steps | **no** (minor; slower edge convergence) |
| Object 9–16-step pixels at most 30 and none above 16; pan none at 9 or more; skinned 0 | 30 / 0; 0; 0 | yes |
| Excluded at most 505 at 33 or more; UI identical | 0; identical | yes |
| Transitions as before; registration within 0.1 px | yes; linear light within 0.12 px except the cube-and-wall region, which equals the corrected run | yes in substance |

Those criteria targeted shimmer, so they did not price in sharpness. The
softness in finding 1 is the new trade-off.

## Quality follow-up (softness; for Astra, the implementation owner)

These are not correctness defects. I changed nothing.
1. **History reconstruction.** The bilinear history fetch at `resolve.wgsl:66–80`
   compounds blur every frame. Consider a sharper reconstruction, such as
   Catmull-Rom with negative lobes clamped or renormalized over admitted taps,
   that keeps the current admission policy. Then re-measure the grille and
   posts with `detail.py` and `blur_equivalent.py`: aim for a grille detail
   ratio well above 18% and an effective σ near the corrected run's 0.5 px,
   while keeping the activity results above.
2. **Presentation.** The bilinear un-jitter in `copy.wgsl` adds a further
   sub-pixel softening on every displayed frame. A sharper presentation sample,
   or an optional mild, opt-in sharpen, is worth evaluating separately.
3. **Thin MASK content.** One-texel MASK detail is the worst case for any
   accumulation. Asset guidance (mip/alpha-to-coverage-friendly masks) may
   matter as much as the resolve.
4. **Moving-shadow lag** (30 pixels, up to 10 steps) is unchanged, so the
   earlier minor note still stands.

## Files

**New in this commit** (all in `handoffs/0072-motion-history-numeric/`):
- `temporal-coverage-result.md`: this document.
- `temporal-coverage-screenshots/`: 14 PNGs at 960×540, published by
  `analyze.py publish`:

  | Screenshot | SHA-256 prefix |
  |---|---|
  | `stationary-off` | `2485512777e5` |
  | `stationary-on-f16` | `a540c7f5983f` |
  | `stationary-activity` | `795b9e8a29e0` |
  | `motion-object-off-f8` | `212ac1a0e0af` |
  | `motion-object-on-f8` | `f6d3cf64d8de` |
  | `motion-pan-off-f6` | `30aa3e196bca` |
  | `motion-pan-on-f6` | `31a48a3acddf` |
  | `excluded-off` | `d1592ae5ed1f` |
  | `excluded-on-f8` | `150fc93fd956` |
  | `excluded-activity` | `ed3a7382c742` |
  | `transition-cut-f7` | `5bc0618bc5a5` |
  | `transition-drs-f9` | `89097514f9ef` |
  | `skinned-off-f6` | `9dc53d04deeb` |
  | `skinned-on-f6` | `ee0d9553c6fb` |
- `temporal-coverage-evidence/`: 50 files, about 0.9 MB, copied unchanged from
  `~/.codex/incant-evidence/0072-temporal-coverage-001`. `manifest.json`
  (`e337bc34…`) gives each file's source and SHA-256. Included:
  - both grants and the released lease;
  - the ledger, the debits and the release latch;
  - the analysis summary;
  - the build records;
  - the seven reports and event logs;
  - the stdout and stderr of all 10 GPU children and root's 2 build children.
- `temporal-coverage-inspection/`: 14 CPU-only scripts, 13 recorded outputs and
  `commands.txt`. They read published PNGs, using the 0067 `pngio.py` reader
  read-only. The magnified crops were for viewing and are not committed.

**Unchanged:**
- `result.md` and every earlier result, preparation and evidence directory;
- `visual/` (tree `b1327f3d…`);
- `brief.md`, which is root's and uncommitted.

## Commands (this session)

```
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py list lib        # exit 0
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py numeric         # exit 0
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py list review     # exit 0
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py review probe    # then stationary, motion,
    # excluded, transitions, skinned, timing: one command each, in that order, every exit 0
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/gpu.py release         # exit 0
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/analyze.py check       # exit 0, pass
python3 -I -B handoffs/0072-motion-history-numeric/visual/tools/analyze.py publish     # exit 0
# CPU-only inspection: temporal-coverage-inspection/commands.txt (13 invocations, all exit 0)
```

**Preflight, before any child:**
- the grant, build record, review and numeric binaries (SHA, mode, executable
  bit), the root build receipt and both release pins;
- source coverage and lint;
- the visual tree and a clean `visual/`;
- no stop or release latch;
- a free global lock, with no renderer or test process running.

## Limitations

- **The numeric test is not an appearance test.** It shows the resolve matches
  an f64 oracle on controlled post-raster occupancy, with its own config. Real
  model and MASK rasterization under jitter, the review policy's reaction and
  tolerances, and visual quality are judged only from the seven scene cases
  above.
- **One host and backend.** One local desktop on one adapter (Apple M5 Pro,
  Metal, macOS 27.0.1). No phone, shipping profile, other backend,
  performance or phase-gate claim is made. Timings are interval observations
  with a fixed period of 1.0.
- **Fixed mode only.** Portable upscaling (0073) and native MetalFX (0074) are
  not covered. The upscaling resolve still uses its earlier admission.
- **Metrics are observations against an aliased reference.**
  - The "detail retained" and blur figures compare an anti-aliased result with
    a point-sampled reference. Some loss is inherent; the run-to-run comparison
    is the meaningful part.
  - Registration is reported in linear light for that reason.
  - Flicker was judged from activity maps and statistics; one PNG cannot show
    it.
- **Fixed scenes.** One config and one composition per case. Other content
  (dense foliage, HDR highlights, fast cuts without a host signal) is untested.
- **Root decides what follows:** default enablement, follow-up work, evidence
  retention and any phase gate. This result approves nothing.
