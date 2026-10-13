# 0069 GPU-only review, attempt 002: two regressions failed, no captures

## Status

**Failed at the regression stage and released cleanly.**

- 6 of the 8 exact GPU regressions passed. The other 2 each failed because a few
  dark output channels landed one code outside my reference's admitted range.
- The wrapper starts appearance captures only after every regression passes, so
  none of the 10 capture processes ran: 0 of 22 frames.
- There is no Core HUD appearance verdict.
- The release is verified and no owned process remains.
- I did not retry, and I changed no gate, source, wrapper or authority.

## Model and effort

- **This session:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort **max**.
- **Root's ACP record** (`~/.codex/incant-evidence/claude-0069-gpu-only-002-acp.log`):
  "Claude session verified: Opus 5.5; thinking: max". I checked it before
  running.

## Inputs

| Input | Identity |
| --- | --- |
| Authority `0069-gpu-review-ready-002.json` | SHA-256 `3b2d9e0b…`; every field matched |
| Wrapper `gpu_only.py` | `0cf37992…` (root commit `f81e1a18`) |
| Brief | `b018e839…`, equal to the authority's `brief_sha256` |
| Predecessor release | `d0998c95…` |
| Evidence path `0069-gpu-002` | absent beforehand |
| Source | generation 5: commit `15a59060`, freeze `acb68a62…` |
| Test executables | copies listed in `build-g5.json` (`dba7bc05…`); hashes in `gpu-provenance-002.json` |

The read-only `gpu_only.py check` passed immediately before the run.

## Run

The command ran once, in the foreground, from 08:42:13Z to 08:42:26Z, and exited
with status 1: `one or more exact GPU regressions failed`.

All results are on Apple M5 Pro with Metal. "vs exact" is the GPU's distance from
exact arithmetic.

| Case | Coverage | Frames | Result | Diagnostics |
| --- | --- | --- | --- | --- |
| `render-scales` | existing | 4 | **fail** (101) | 1x sRGB: 0 outside, vs exact worst 1. 2x sRGB: 1 channel outside, by 1 code, but vs exact worst 1 and none over 1. 1.5x and UNORM were not reached. |
| `render-nested` | existing | 2 | pass | 0 outside, vs exact worst 1 |
| `render-halo-backdrops` | new | 2 | **fail** (101) | White: 0 outside; text 16.85:1 against the darkest pixel. Black: 105 channels outside, by 1 code; text 18.08:1. |
| `render-over-world` | existing | 3 | pass | 0 outside, vs exact worst 1. The wrong layer order is still rejected (279 outside, by up to 47). |
| `render-retained` | existing | 6 | pass | |
| `render-region` | existing | 1 | pass | 0 outside, vs exact worst 1 |
| `host-composed` | existing | 17 | pass | |
| `host-relayout` | existing | 4 | pass | |

## Release

| Check | Result |
| --- | --- |
| `release.json` | `a3b090e2…`, latched at 08:42:26.417Z |
| Children | 8 started, 8 reaped |
| Frames debited | 39 regression, 0 appearance |
| Owned processes / target users | none / none |
| Ledger | 26 rows; its hash equals the release record; the latch binds `release.json` |
| After return | no 0069 process; no process has `native-gpu.lock` open |

## Failure analysis

**What the failures are.** In both, a GPU channel is exactly one code outside the
reference's admitted range, and only in dark output.

- The GPU stays within one code of exact arithmetic in every case that reports
  it. The failing 2x panel itself has a worst of 1 and no channel over 1.
- A geometry, clip, layering or coverage error would miss by tens of codes. For
  scale, the wrong-order control misses by 47.

**Measurement on the retained black and white halo frames**, 692 halo-only pixels:

- Fully covered halo pixels store exactly (10, 12, 16) over both backdrops (64 and
  57 pixels), so there is no systematic truncation.
- The white frame's three channels agree on each pixel's coverage to a median of
  0.001. That coverage predicts what the black frame should hold under exact
  arithmetic.
- Metal's stored dark values differ from that prediction by up to 0.98 code, in
  both directions. 466 of 2,076 channels (22%) fall beyond ±0.5, and 153 beyond
  ±0.67. Round-to-nearest alone would keep every channel within ±0.5, plus
  under 0.1 code of error in the coverage estimate.
- The reference's portable model allows an sRGB store 0.67 code from exact,
  rounding included (0.17 before rounding, taken from llvmpipe).

**Interpretation.** Metal's sRGB store or blend near black is slightly less
precise than the portable model admits. The new opaque dark halo puts many more
blended values in that range; the pre-halo panel passed on Metal. This is not a
visible defect: over black the halo is invisible and the text measures 18.08:1.

**Not established:**

- Metal's documented precision.
- Whether the error comes from the store encode or the blend.
- Whether the 1.5x and UNORM cases would pass.

## Repairs for root's next source/build interval (not applied)

1. Rerun only the two failing cases with a test-only diagnostic that prints each
   out-of-range channel's reference interval and GPU value. Alternatively, run a
   small near-black ramp probe on Metal.
2. **If backend precision is confirmed:** widen the reference's sRGB store
   allowance for dark values to the measured bound (likely about one code). Cite
   the measurement in the reference header. Keep every mutation check; they
   reject wrong output by 8 to 47 codes, which shows admission still
   discriminates.
3. **If it is not backend precision:** investigate the halo path at those pixels
   before changing any tolerance.

Then rerun all eight regressions and, once they pass, the ten capture processes.
I would not relax either test in any other way.

## Appearance

No Core HUD frame was rendered. Still without a verdict:

- narrow layouts, Arabic RTL, Simplified Chinese and pseudo-locale;
- won, lost and restart states;
- contrast;
- the label padding decision.

**Scoped observation of the 14 retained regression images.** These are synthetic
test scenes, not the Core HUD; I inspected each one personally, using lossless
enlargements that I deleted afterwards. The halo renders as designed:

- It is a thin, crisp dark contour around each glyph, not a blur or a glow.
- Over pure black it disappears.
- Over pure white the light fill nearly vanishes and the contour carries every
  letter shape: legible, with an outlined look.
- Over the saturated test background (panel 1x and 2x, offset region, clips), the
  mixed Latin, Arabic and Chinese label stays legible.
- In the composed game at 2x, the halo separates text that crosses a light grey
  world triangle.
- A clipped RTL token's halo is cut cleanly at its clip.

Seen but unrelated to the halo, and coming from the test fixtures or known
limits: button labels clipped by tight test-button sizes, a wrapped second line
clipped by the narrow host label's box, and soft 2x glyphs from the 1x atlas.

## Retained evidence (this commit)

- `gpu-attempt-002/`: start and finish times, exit status (1), stdout (the eight
  regression rows) and stderr (the wrapper's failure traceback).
- `gpu-attempt-002/images/`: byte-identical copies of five retained images:
  `ui-halo-white`, `ui-halo-black`, `ui-panel-2x-Rgba8UnormSrgb`,
  `hosted-ui-narrow-2x` and `ui-clip-open`.
- `gpu-provenance-002.json`: authority, wrapper, brief, ACP record, predecessor
  release, source, executables, CPU packet, every run record and log, all 14
  image hashes and the release checks.
- The full external record is in
  `/Users/andreyyurlov/.codex/incant-evidence/0069-gpu-002`.

Earlier evidence is unchanged: `gpu-result.md`, `gpu-provenance.json` and
`gpu-attempt-001/`. The CPU lane's files and root's review stay uncommitted.

## Limitations

- No appearance captures ran, so there is no Core HUD verdict.
- These are GPU results from Apple M5 Pro with Metal, using CI-profile test
  binaries. No phone, performance or phase-gate claim.
- The failure analysis is a measurement on two retained frames, not a backend
  specification.
