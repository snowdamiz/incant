# 0070 C2 SSAO comparison (ssao-filter-002): GPU result

## Status

**All 9 granted GPU children ran once and succeeded.**

- **Release:** at 14:57:13Z, with 100 of 100 planned submissions (128
  cap), 16 PNGs, 11 pairs, 0 retries and nothing latched. No owned or
  frozen-binary process remains.
- **Analysis** (`ssao-filter-002/analyze.py`, exit 0):
  - **Numeric:** 43 of 43 gates pass.
  - **Revised quality:** passes. All **14 N8 rows** are within 0.25, not
    only the two ROI rows.
  - **Authored N6:** reads **7.00/255 and fails**. That failure is retained
    and kept separate from the adopted N6T, as is 001's earlier 7.00 at
    x 161.
- **Appearance:** I reviewed all 16 PNGs and the 11 planned pairs.
  build003's staircases, wedges, hatch and mottling are gone, and I found no
  new artifact class.
- **Acceptance:** none here, of any phase, performance, device, cooked or
  default-on kind. Root decides.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**. ACP verification is root's.

## Identities

| Item | Value |
|---|---|
| Source / engine | `25f48ee7696e89cf903b82a49ba38d80dc027996` / `02c71fdb60715a24ca519bf3258ebaf22025d42f` (C2) |
| Plan / helper digest | `eb330258…`, equal to the grant / `e31b8d67…`, equal to the grant |
| Pin binding | `verify_engine(02c71fdb)`: `ao.wgsl` is byte-identical to root's frozen C2 shader |
| GPU grant | `gpu-grant.json`, sha256 `26d8251d…` |
| Build review | `ac93e4af…` |
| Source review | `2d2c3b74…` |
| Previous holder | 0076's release record matches `68e22d6d…` |
| Build | Record `95d471dc…`. 7 children passed, including strict Clippy and the **3 sphere-guard CPU tests**. Root's Windows-only Cargo-config refusal and its amendment are retained |
| Frozen binaries | `effects_gpu` `46c6a88e…`, `lib` `f995d7c5…`, `lib-cooked` `0b918d57…`; each unchanged at completion |
| Adapter | Apple M5 Pro, Metal, IntegratedGpu: a desktop, not a phone |
| Release / ledger | `released.json` `60c6afd2…` / ledger `62bf9e4e…`, equal to the release record |
| Analysis | `filter-analysis.json` `7bbd372b…`; plan matches the build |

The evidence directory is `/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-002`.

## Device sequence

All children ran serially in the foreground through `filter_runner.py`.

| Child | PID | Submissions | Wall (s) | Exit | In-run checks |
|---|---|---|---|---|---|
| control `filter_device` | 30774 | 2 | 0.36 | 0 | 1 test passed |
| control `ao_device` | 31095 | 7 | 0.91 | 0 | 1 test passed |
| control `ao_device_cooked` | 31723 | 0 | 0.38 | 0 | typed refusal |
| case `ssao_core` | 32435 | 20 | 10.32 | 0 | 18/18 |
| case `ssao_materials` | 33775 | 10 | 5.20 | 0 | 8/8 |
| case `ssao_geometry` | 34444 | 17 | 6.38 | 0 | 13/13 |
| case `lifecycle_drs` | 35004 | 18 | 3.14 | 0 | 17/17 |
| case `ssao_quality` | 35147 | 10 | 4.98 | 0 | 9/9; sphere 2,818 against 2,824.6 and 454 against 455.2 |
| `appearance` | 35481 | 16 | 2.25 | 0 | 16 PNGs, 11 pairs |

Every child was reaped, with no signal, no timeout and no surviving group
member.

## Numeric contract: 43 of 43

**Interval oracle, against the C2 oracle in `c2_analysis.py`:**

- All 15 filtered-reference gates agree on **43,489 / 43,489** interior
  texels, with 0 out of range, 0 uninferable and 0 decisive texels too wide.
- All 15 stage gates have 0 raw, H, V, final, clear or forward texels outside,
  over 45,241 viewport texels each.

**Measured R8 rounding:** every exact tie rounded up, with residuals in
[−0.4375, +0.5].

**Other gates:**

- **Identities:** 9 of 9, each with 0 differing pixels.
- **DRS expansion:** 0 mismatches at 12/16 and 8/16.
- **DRS bloom centroid:** passes.
- **Mixed pane:** median 0.521, with all 2,467 intervals containing ½.

**Contract looser at 66 texels.** The C2 oracle is less decisive on
orthographic stripes. 66 non-decisive texels have intervals 4–6 codes wide,
against at most 3 for R1/R3, so the contract is looser there. The device
nevertheless agreed at every texel.

## Quality (actual device values)

**Revised gates.** N6T is the measure root adopted; the other bounds are
unchanged.

| Gate | Device | Bound | Result |
|---|---|---|---|
| N6T, `white_slab` perspective, 70 pairs | **3.19/255** at x 164 | 3.98 | pass, 0.79 margin |
| N7: `white_slab` p / o, `edge_slab` p | 0 / 1 / 0 | ≤ 138.2 / 176.7 / 202.7 | pass |
| N8 perspective ROI, row 124 | 6 codes (0.188) | 7.97 codes (0.25) | pass |
| N8 orthographic ROI, row 110 | 7 codes (0.088) | 19.92 codes (0.25) | pass |

**Authored N6 (retained):** **7.00/255 at x 164, failed**, against a bound of
3.98. This is unchanged from 001's 7.00 at x 161.

**Orthographic N6T (diagnostic only):** **3.14/255** at x 158, within the
bound. The CPU model predicted 4.14, which would fail. The worst pair moved by
one code between model and device, so this is a device measurement and not a
robust pass.

**All 14 N8 rows, each against 0.25 of its own shipped value:**

| Perspective row | 124 | 125 | 126 | 127 | 128 | 129 | 130 |
|---|---|---|---|---|---|---|---|
| Device (codes) | 6 | 7 | 7 | 3 | 5 | 7 | 6 |
| Bound (codes) | 7.97 | 7.97 | 7.97 | 7.97 | 11.95 | 11.95 | 11.95 |
| Margin (codes) | 1.97 | **0.97** | **0.97** | 4.97 | 6.95 | 4.95 | 5.95 |

| Orthographic row | 110 | 111 | 112 | 113 | 114 | 115 | 116 |
|---|---|---|---|---|---|---|---|
| Device (codes) | 7 | 7 | 7 | 6 | 5 | 5 | 4 |
| Bound (codes) | 19.92 | 19.92 | 15.94 | 11.95 | 11.95 | 11.95 | 11.95 |

**Every one of the 14 rows passes, but two pass by under one code.** For
comparison, R1/R3 in 001 failed all 7 perspective rows, at 12–17 codes.

**Device against the C2 model:**

- N8 row 124: the device reads 6 codes, against the model's 7.
- N8 orthographic rows 110–112: the device reads 7, 7 and 7, against the
  model's 6, 6 and 8.
- N6T, N7, the other N8 rows and authored N6 match the model exactly.

## Appearance review (16 PNGs, 11 pairs)

**What I reviewed:** each PNG at native scale, plus 9 review aids in
`evidence/ssao-filter-002/review/`. These are derived from the retained PNGs
and decode-verified against `index.json`. They put build003, on the left,
beside 002, on the right, using build003's own rectangles and gains.

**v1 and v2, SSAO:**

- build003's banded pillar, blocky floor halos, mottled sphere underside,
  radial fan bands across the v1 floor and hatched crate are gone.
- C2 gives smooth contact halos and a clean crescent under the sphere.
- At ×3 with ×16 gain, the stripe bases darken evenly between the stripes and
  fade smoothly below them. **There is no visible comb.**

**v4 close-up:**

- The faceted triangular patches on the crate's shadowed face and the wedges
  on the floor are gone.
- The crate face next to the sphere darkens strongly but smoothly, which is
  plausible at radius 2 and strength 1.

**v3, DRS 12/16 and 8/16:**

- AO is smooth.
- Silhouettes step from nearest-neighbour upscaling, as designed.
- The bloom halo is compact and centred on the emitter.

**Bloom pairs:** bloom only brightens and is unchanged in character.

**SSAO pixels brighter than off:**

- **Counts:** the runner's pair receipts report 7, 5 and 84 such pixels for
  v1, v2 and v4.
- **Size:** my decode finds 6, 5 and 83 pixels whose mean RGB is higher. Each
  is a single channel raised by exactly one code, on the sphere's lit surface.
  build003 had 1, 0 and 0.
- **Visibility:** invisible. AO is clamped to 1 or below, so this looks like
  composite rounding at near-unoccluded texels. I have not verified the cause.

**My verdict, on rendered pixels:** at these fixture parameters on this Metal
adapter, C2 removes every sampling artifact I flagged in build003, and I see
no new one. As a look, it is acceptable here. This is not a default-on,
device-class or performance judgment.

## Risks and remaining failures

- **Authored N6 fails** by definition and stays recorded as failed.
- **Thin margins on one adapter:** N8 rows 125 and 126 pass by 0.97 codes, and
  N6T by 0.79. The orthographic N6T diagnostic moved by 1 code between model
  and device.
- **The C2 oracle is less decisive** at 66 orthographic-stripe texels, where
  its intervals are 4–6 codes wide.

## Next concrete recommendation

1. **Root's review** of this 002 evidence for adopting C2 as the opt-in SSAO
   sampling. The default stays off.
2. **One replay on a non-Metal backend.** Run the identical frozen 9-child,
   100-submission sequence, for example Vulkan on lavapipe or Dx12 on WARP,
   before treating the sub-code N8 and N6T margins as robust. No threshold
   change.
3. **Reference-phone measurement** per PLAN (AWS Device Farm) before any
   default-on decision. Nothing about performance is claimed from this
   desktop run.

## Commands (all exit 0)

`R = env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`, run
from the worktree root.

| Command | Result |
|---|---|
| Read-only preflight: plan, digest, engine binding, build record, binaries, source freeze, prior release | all match the grant |
| `R …/ssao-filter-002/filter_runner.py control --grant E/gpu-grant.json filter_device`, then `ao_device`, then `ao_device_cooked` | exit 0, three times |
| `… case …` for `ssao_core`, `ssao_materials`, `ssao_geometry`, `lifecycle_drs`, `ssao_quality` | exit 0 each |
| `… appearance --grant E/gpu-grant.json` | exit 0 |
| `… release --grant E/gpu-grant.json` | exit 0 at 14:57:13Z |
| `R …/ssao-filter-002/analyze.py E` | exit 0 in 158 s |
| `R evidence/ssao-filter-002/review/make_review.py` and `make_stripes_review.py` | 9 aids and 2 manifests |
| `R evidence/ssao-filter-002/make_index.py` | `index.json` |

**What did not happen:** no build, Cargo, source or helper edit, retry,
ignored test, or GPU use after the release.

## Changed paths

**New, committed with `Built-by: claude`:**

- `filter-002-result.md`
- `evidence/ssao-filter-002/`: `index.json`, `make_index.py`, and `review/`
  (2 scripts, 2 manifests, 9 PNGs)

**Unchanged:**

- `result.md` and every earlier report;
- `ssao-filter-001/`, `ssao-filter-002/`, `ssao-repair/` and `tools/`;
- the engine;
- all 001 evidence.

Root's `brief.md` and the generated `Cargo.lock` files stay uncommitted.

## Limitations

- **Single adapter:** one desktop Metal adapter. There are no phone, Vulkan or
  Dx12 results.
- **Fixture scope:** the appearance verdict covers these fixture parameters
  only, at radius 2, bias 0.01 and strength 1.
- **Sub-code margins** are unproven beyond this run.
