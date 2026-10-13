# 0070 corrected GPU execution (build003): SSAO + bloom result

## Status

**All six granted children ran once and succeeded; the numeric analysis
failed on one gate.**

**This run:**

- The six children were the five corrected numeric cases and `appearance`.
  Each ran once, serially, in the foreground, and each succeeded.
- They used 89 new frames and produced 16 PNGs, on Metal (Apple M5 Pro).
- I released the GPU through `runner.py release` at 09:02:45Z. No owned or
  frozen-binary process remains, no retry ran, and nothing is latched.

**Numeric analysis** (run after release on build003; exit 1, report kept):

- 26 of 27 gates pass.
- The known-open `drs_bloom_halo_centroid_on_physical_emitter` still fails,
  with centroids identical to build002.
- **Numeric acceptance is therefore not complete.** No gate was loosened.

**Appearance:** I reviewed all 16 PNGs and 11 pairs.

- **SSAO** is numerically right but visibly artifact-prone at the fixture
  parameters.
- **Bloom** looks good at full resolution and changes size and balance under
  dynamic resolution.
- **This is not visual acceptance.**

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**, as configured and verified by root for this resumed session.

## Exact identities

| Item | Value |
|---|---|
| Source HEAD | `fd625299d90fc9b0e359a7bf60baf03795fc9f4f`; root's `brief.md` uncommitted during execution |
| Grant | `0070-effects-build-003/gpu-grant.json`, sha256 `76dbd1db…` |
| Root build review | `effects0070-corrected-build-root-20261013.json`, sha256 `b08e2b3d…` (pinned) |
| Build record | sha256 `de2f040f55bc0cf3d1e924d64304916428ccdb7083404c9fb85431efdd54ebdc` |
| Helper digest / plan | `942e528b…` / `27ea7b44…` |
| Helper binary `effects_gpu` | `cd431a216c32124fef9946b7e0b59e7a74326f030df43d103b4b759eb41b2db5` (new in build003) |
| Regression binaries | `lib` `732ac9ed…`, `model_gpu` `64edd8ae…`, `gpu_host-game` `f64e5c95…`, `gpu_host-ui` `b6a33514…`; I checked each on disk as byte-identical to build002 |
| Toolchain / features | rustc 1.99.0; renderer and host feature sets identical in the regression and helper builds |
| Backend | **Metal**, Apple M5 Pro, integrated GPU; driver strings empty. No other backend ran |
| Analysis | `analysis.json` sha256 `96a76ed26fef3822760f35f2b4bea628a7b35354c7a949b9024125b96ba4ba7b` (format v3) |
| Release / ledger | `released.json` `e2aedc7e…` / ledger `c1a500db…` |

The full per-child record (PIDs and kernel start times, argv, environment
keys, log hashes and PNG hashes) is in `evidence/execution-003.json`.

## Reused coverage versus new work

- **Reused, not rerun:** the 94 ignored GPU regression tests (lib 37,
  model_gpu 50, gpu_host-game 5, gpu_host-ui 2). They ran in build002 at
  07:57–07:58Z and all passed. They are bound to this source by the four
  byte-identical regression binaries. I ran no listing or suite.
- **New in this session:**

| Child | PID | Time | Frames | In-run checks |
|---|---|---|---|---|
| `case ssao_core` | 20021 | 6.44 s | 20 / 20 | 18 ok |
| `case ssao_materials` | 20081 | 3.29 s | 10 / 10 | 8 ok |
| `case ssao_geometry` | 20112 | 3.97 s | 17 / 17 | 13 ok |
| `case lifecycle_drs` | 20143 | 2.13 s | 18 / 18 | 17 ok |
| `case bloom_numeric` | 20159 | 2.68 s | 8 / 8 | 12 ok |
| `appearance` | 20240 | 1.37 s | 16 / 16, 16 PNGs | 58 ok |

Every child exited 0, its group was verified absent, and the runner recorded
`success: true`. The appearance in-run checks include:

- chrome, physical UI layout and opaque UI interiors identical to off for every
  variant;
- an actual DRS receipt for each v3 render.

**Budgets:** 6 of 6 children, 89 of 89 new frames, 16 of 16 PNGs, all within
960×540. About 44 seconds passed from the first child to release.

## Numeric analysis (build003, after release)

Exit 1; `pass: false`; the report is retained.

**Pass (26):**

- **All 13 SSAO reference gates.** They compare 43,489 of 43,489 interior
  pixels with zero disagreements, and every reference-occluded pixel matches
  (8,558 / 7,075 / 11,025 / 10,853). All four new per-pixel preconditions are
  met.
- **Identity and determinism:** the four core identity gates and the three
  determinism gates, each with zero differing pixels.
- **DRS SSAO nearest expansion** at 12/16 and 8/16, with zero mismatches.
- **Mixed BLEND pane:** median ratio 0.506.
- **Bloom:**
  - coverage ratio 0.497;
  - threshold changes [0, 0, 49];
  - constant field (predicted 248 and 243).

These passes come from root's corrected oracle, `fd625299`. It sets an
explicit zero key light (an empty light list had enabled a preview key
light), matches specular bases per pixel, and propagates tone-curve intervals.
No build002 failure was upgraded; that run stays as recorded in `result.md`.

**Fail (1): `drs_bloom_halo_centroid_on_physical_emitter`.** The expected
centre is (186.55, 83.06).

| Scale | Whole-viewport centroid | Distance | Tolerance |
|---|---|---|---|
| 16/16 | (186.16, 83.21) | 0.42 px | 1.5 px |
| 12/16 | (190.54, 86.47) | 5.25 px | 2.0 px |
| 8/16 | (181.23, 83.21) | 5.32 px | 3.0 px |

### Evidence on the open centroid gate (CPU-only, after release)

Bloom-only measurements on the same single-emitter frames the gate uses:

| Scale | Halo energy | rms radius | Share beyond 48 px | Left / right | Up / down |
|---|---|---|---|---|---|
| 16/16 | 324 | 32.9 px | 12% | 0.499 / 0.501 | 0.510 / 0.490 |
| 12/16 | 378 (+17%) | 39.2 px | 21% | 0.475 / 0.525 | 0.451 / 0.549 |
| 8/16 | 425 (+31%) | 46.1 px | 27% | 0.531 / 0.469 | 0.520 / 0.480 |

The emitter footprint and the halo within 24 px stay centred within 0.5 px at
every scale, as in my build002 diagnosis. The appearance renders agree: the
same v1/v3 framing spreads 90 → 97 → 108 px rms at 100/75/50%.

**My read:**

- The bloom footprint is not resolution-independent. Pyramid levels are
  counted in internal texels, so the glow grows about 40% in radius by 50%.
- Its faint far field drifts asymmetrically at the odd internal sizes
  (241×145 and 161×97): toward the bottom-right at 75% and the left at 50%.
- The gate's whole-viewport centroid is measuring that real far-field drift,
  not an emitter-position error. The engine's own near-field emitter test
  passed in build002.

Whether to fix the bloom or re-specify the gate is root's decision. The gate
stays failed.

## Appearance review (all 16 PNGs)

The renders and views:

- **v1:** perspective at 959×539, an offset rounded viewport with the UI.
- **v2:** orthographic at 960×540.
- **v3:** the v1 framing with DRS 12/16 and 8/16, off and both.
- **v4:** a 640×480 close-up of the sphere and crate contact.

I decoded each PNG to exactly its recorded RGBA hash, then compared every pair
at true contrast, as amplified differences, and as zoomed crops. The crops are
in `evidence/appearance-review/`.

### SSAO

**What works:**

- **Readable contact grounding** under the sphere, at the pillar base and around
  the crate.
- **No effect where AO must not apply.** AO touches no emissive, BLEND,
  sprite, world-text, UI or chrome pixel; the in-run checks prove the UI and
  chrome parts byte for byte.
- **It never brightens anything.** The one exception is benign: v1 (541, 313)
  goes from 249 to 250 in green. AO removed some warm diffuse while the
  neutral specular stayed, so the hue moved toward neutral with red pinned at
  the tone shoulder.
- **It behaves consistently under DRS.**

**Concrete artifacts, at the fixture's radius 2.0, bias 0.01 and strength
1.0:**

1. **Tessellation-revealing self-occlusion on the smooth sphere.** The lower
   hemisphere turns mottled and faceted (v2, v4). At 0.01 the bias is smaller
   than the depth error of the faceted mesh.
2. **Hard-edged polygonal patches.** They appear on the crate's side face, which
   reads as dirty or stained in v4, and as wedge-shaped bands across the floor.
   This is the fixed 16-direction kernel, identical per pixel with no rotation
   or blur, rendering visibility as piecewise-constant regions.
3. **Banding.** Horizontal stripes in discrete steps appear on the pillar's
   lower face and faintly on the crate's front (v2).
4. **A blocky comb pattern** on the floor under the MASK stripes (v2).
5. **Over-broad, heavy darkening.** A 2.0 radius against props of about 1 m
   darkens 72% of the v4 close-up, by up to 63 codes; the crate's whole side
   face is about half-occluded. Mid-distance views look better: in v1 15% of
   pixels darken, softly.
6. **At 50% DRS** the AO structures become coarser, with 2×2 blocks.

**Judgment:**

- At mid-distance (v1, v2) SSAO reads as plausible soft grounding with minor
  patchiness.
- Up close (v4) it reads as dirty and noisy, not as ambient occlusion.

**Not visually acceptable as a look** at these fixture parameters, even though
the numerics match the reference exactly.

### Bloom

**What works:**

- **Clean emissive halos.** They are smooth and symmetric at full resolution.
  The falloff is monotonic, for example 20, 21, 22, 23, 25 … 92 next to the
  emitter at 253, with no ringing or banding visible.
- **It never darkens anything:** 0 darker pixels in every bloom render.
- **The BLEND emissive pane blooms by its coverage.**
- **Opaque UI is untouched.**

**Issues:**

1. **Lit diffuse surfaces bloom like emitters.** The white sphere under the
   fixture's key light (intensity 6), against threshold 1.0, glows heavily in
   v4, like a lamp. This is a look-dev and threshold matter, not an engine
   defect.
2. **DRS scale dependence.** As measured above, the glow is about 40% larger
   and 31% brighter at 50%, and the outer halo is lopsided at reduced scales.
   If DRS changes at runtime, the glow would visibly breathe.

**Judgment:** good at 100%, not yet resolution-stable.

### DRS and the combined renders

- **DRS:** nearest expansion produces the expected stair-stepping, roughly
  doubled at 50%, while UI text stays crisp at physical resolution. That is
  consistent with the accepted canonical treatment.
- **Both effects together:** no adverse interaction. The glow lays over
  AO-darkened areas naturally.

## Proposed bounded visual repair direction

Root owns the numerics. Each change needs an updated oracle and the same
bounded re-run.

1. **SSAO:**
   - a per-pixel 4×4 interleaved rotation of the 16 directions;
   - then a 4×4 depth- and normal-aware blur of the R8 accessibility before
     shading;
   - a bias scaled with depth or pixel footprint, against the faceting;
   - an optional distance falloff;
   - a separate art radius of about 0.3–0.5 for props of this scale, keeping
     2.0 only as the numeric fixture.

   Validate with the same 16-render set and a CPU reference that includes the
   rotation and blur.
2. **Bloom under DRS:**
   - derive the level count and kernel from the physical output size;
   - resample odd sizes with centre alignment (or pad levels to even sizes) to
     remove the far-field drift.

   Then re-run `lifecycle_drs`, with the centroid gate unchanged, and v3.
3. **Look-dev only:** a soft-knee or exposure-relative threshold for art
   presets, so lit white diffuse doesn't glow.

## Screenshots

- **The 16 renders:**
  `~/.codex/incant-evidence/0070-effects-build-003/outputs/appearance/{v1,v2}_{off,ssao,bloom,both}.png`,
  `v3_{off,both}_{12,8}.png` and `v4_{off,ssao,bloom,both}.png`. Their hashes
  are in `evidence/execution-003.json`.
- **Review aids** in `evidence/appearance-review/`:
  - crops: SSAO on the v4 crate and sphere, the v2 sphere and crate, and the
    v2 pillar and stripes; bloom on the v2 panel and the v4 sphere edge;
  - amplified differences: v4 SSAO, v2 SSAO and v3 both at 8/16;
  - the provenance `manifest.json` and the `pngtool.py` used.

## Unmet gates and pending work

- **`drs_bloom_halo_centroid_on_physical_emitter`:** failed.
- **Appearance acceptance:** not granted; see the artifacts above.
- **Still pending:** cooked SSAO and bloom, native default exposure, profiling
  subpass entries, TAA and upscaling, device budgets, phone performance and
  complete Phase 1 rendering.
- **Retries:** none ran; the reserve slots are not approval to rerun.

## Changed paths

- `corrected-gpu-result.md` (this report)
- `evidence/execution-003.json`
- `evidence/appearance-review/`: 8 PNGs, `manifest.json` and `pngtool.py`

`result.md`, `preparation-result-v1.md`, `evidence/execution-002.json` and all
evidence directories are unchanged. No source, helper or plan was edited, and
nothing was pushed.
