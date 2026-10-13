# 0070 SSAO next repair: CPU design and guard preparation

## Status

**CPU-only preparation is complete; nothing is accepted.** No renderer, GPU,
screenshot, Cargo or target directory was used, and no production file
changed. Nothing was granted, built or run on a device.

**Candidate result:** one candidate meets N8 in the CPU model at the current
64-probe budget.

- **C2** pairs each texel's rotation stratum with its bit reversal and gives
  every ray its own Sobol' step phase.
  - N8 passes on all 14 screened rows: perspective 124–130 (0.220 at the ROI)
    and orthographic 110–116.
  - N7 and the proposed N6T also pass.
- **The 16-probe candidate (S16)** I proposed in `filter-result.md` fails: N8
  perspective reads 0.282 at the ROI, and N6T reads 4.37/255. My
  extrapolation that it would pass was wrong.
- **C1 and C3** also fail N8.

**Quality stays unaccepted, for three reasons:**

- N6 authored remains failed, for every candidate and for an exact smooth
  control.
- N6T awaits root's review.
- C2 clears the ROI bound by only 0.97 codes, so only a device run can
  confirm it.

**Guard:** the sphere-coverage predicate is corrected in a new namespace,
`0070-ssao-filter-002`.

- **Regression:** a synthetic Rust regression is added. It is uncompiled here,
  because Cargo was not allowed. The 002 build runs it before any GPU child.
- **Plan:** the 002 plan repeats 001's nine children and 100 submissions, with
  the engine pin pending.
- **001 stays as it was:** its files, evidence and latch are unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**. ACP verification is root's.

**Screenshots:** none; no renderer use was permitted. The three PNGs in
`next-repair/` are CPU-model planes. `result.md` and `filter-result.md` are
preserved; this file is the packet's return document.

## 1. Sphere-coverage guard and the 002 namespace

**Fix:** in `ssao-filter-002/effects/src/numeric.rs`, `sphere_texels` counts
only texels wholly inside the viewport (`x >= x0`, `y >= y0`,
`x + 1 <= x0 + w`, `y + 1 <= y0 + h`).

- The 10% tolerance is unchanged. It now lives in a named
  `sphere_covers()`.
- On the retained 001 frames, the fix excludes exactly the 601 rim texels:
  columns x 13 and 293, and row 17.

**Regression:** the `sphere_guard_tests` module (3 tests) builds a 321×193
frame like the retained ones. It has the canvas outside the viewport, the
measured rim blends (22, 32, 45) and (26, 47, 70), and the backdrop inside.

- **Rim alone:** the rim counts as no sphere. It holds exactly 601 rim
  texels.
- **Interior disc:** a disc of the analytic area counts exactly its texels,
  within 10%, in both projections.
- **Tolerance:** stays 10%. 3,419 fails and 2,818 passes for perspective.

**Build step:** the 002 build runs `cargo test --manifest-path <helper> --lib --
sphere_guard` as `build-helper-guard-tests`. This is CPU-only. `guard_verdict`
refuses the build unless exactly the three named tests pass. It refuses a
filter that matches nothing, a missing test, an extra test and a failed test.

**Python twin:** a twin of the same fixture runs in the selftest.

| Fixture | 001 predicate | 002 predicate |
|---|---|---|
| Rim only | 601 | 0 |
| Perspective disc (expected 2,824.6) | 3,419 | 2,818 |
| Orthographic disc (expected 455.2) | 1,057 | 456 |

**Namespace:** `ssao-filter-002/` is a copy of the 13 tracked 001 files.
`selftest.py` proves the copy differs from 001 only as follows:

- **Unchanged:** `appearance.rs`, `common.rs`, `composed.rs` and `gltf.rs` are
  identical.
- **Comments only:** `lib.rs`, `main.rs` and `Cargo.toml`.
- **`numeric.rs`:** reverts to 001's byte for byte once the predicate, the
  named tolerance and the test module are undone.
- **Python bindings:** `NAMESPACE` in `filter_common.py`; the guard step in
  `filter_runner.py`; `make_plan.py`; and a new `analyze.py`.
  - `analyze.py` runs the unchanged `analyze_filter.py`, with its plan check
    bound to the 002 plan.
- **Untouched:** 001's helper, `ssao-repair/`, `tools/` and all 001 evidence.

**Plan (`ssao-filter-002/plan.json`):** the same controls, cases, appearance
set, budgets and timeouts as 001.

- 9 children, 100 of 128 submissions, 16 PNGs and 11 pairs.
- `engine_commit` is `null`, so a source freeze refuses until root runs
  `make_plan.py --engine-commit <sha>`.
- A 002 grant must name the 002 namespace and evidence directory. The
  selftest shows a 001 grant is refused.

| Binding | Value |
|---|---|
| `ssao-filter-002/plan.json` sha256 | `9674a991310a75b38004392270cb2f2d25b482a873d7d9a90bca54baaf7261a2` |
| 002 helper digest (27 files: namespace, reused tools, analyzer) | `d2cbd33cc6fc3719bb26dd9297097ae5060290b7452e16ee42d0ea5133567d54` |
| Selftest | exit 0; `next-repair/ssao-filter-002-selftest.json` |

Both hashes change when root pins the engine (the plan), and again if the C2
oracle below is added (the digest).

## 2. Candidate screen

`next-repair/screen.py` writes `screen-001.json`. It is CPU-only and ran in
42 s. Every configuration keeps:

- the 16 directions and the binary single-layer test;
- radius 2 and bias 0.01;
- the Bayer rotation table;
- R3's plane-weighted 5-tap H/V filter with its R8 writes.

All five configurations were declared before any was measured.

**Binding to the device:** the model's design (D) reproduces the retained 001
V planes.

- Every metric matches, except orthographic row 112 (11 against 12 codes).
- Texel agreement:

  | Region | Equal / compared | Largest difference |
  |---|---|---|
  | N8 windows, perspective | 71 / 77 | 1 code |
  | N8 windows, orthographic | 46 / 49 | 1 code |
  | `white_slab` perspective interior | 40,291 / 41,949 | 1 code |
  | `white_slab` orthographic interior | 41,112 / 42,593 | 1 code |
  | `edge_slab` interior | 40,289 / 42,653 | 1 code |

- The generalised estimator equals `quality_model.raw` bit for bit for D and
  S16.

**Results.** N8 is each row's peak-to-peak over the shipped value on that row,
with the bound 0.25 unchanged. N6 and N6T are in /255; the N6T bound is 3.98.

| Config | Probes/texel | N8 p, row 124 | N8 p, rows 124–130 | N8 o, rows 110–116 | N6 authored | N6T p | N6T o (diag.) | N7 white p / white o / edge p |
|---|---|---|---|---|---|---|---|---|
| D: R1/R3, device 001 | 64 | **0.376** | 0.376 0.439 0.502 0.533 0.356 0.335 0.272 | 0.176 0.163 0.188 0.167 0.167 0.125 0.146 | 7.00 | 3.37 | 4.63 | 0/0/0 |
| D: R1/R3, model | 64 | **0.376** | identical to device | 0.176 0.163 0.173 0.167 0.167 0.125 0.146 | 7.00 | 3.37 | 4.63 | 0/0/0 |
| S16: 16 steps, full stratum | 256 | **0.282** | 0.282 0.282 0.251 0.251 0.146 0.146 0.125 | 0.063 0.075 0.094 0.125 0.105 0.084 0.042 | 8.00 | **4.37** | 4.69 | 0/0/4 |
| C1: Hammersley tile pairing | 64 | **0.345** | 0.345 0.471 0.502 0.408 0.167 0.146 0.125 | 0.100 0.113 0.157 0.251 0.272 0.335 0.314 | 7.00 | 3.19 | 4.29 | 0/0/0 |
| **C2: C1 + per-ray Sobol' phase** | **64** | **0.220** | 0.220 0.220 0.220 0.094 0.105 0.146 0.125 | 0.075 0.075 0.125 0.125 0.105 0.105 0.084 | 7.00 | **3.19** | 4.14 | 0/1/0 |
| C3: C2 + per-step decorrelation | 64 | **0.439** | 0.439 0.376 0.408 0.502 0.314 0.230 0.209 | 0.113 0.075 0.094 0.105 0.063 0.084 0.105 | 8.00 | 4.39 | 4.14 | 0/0/0 |

Earlier results, all preserved and not re-run:

| Candidate | Probes/texel | N8 p, row 124 | N6T p | N7 | Source |
|---|---|---|---|---|---|
| Full-stratum shared phase | 64 | 0.471 | 3.39 | 0/0/0 | addendum, `cpu-evidence-001.json` |
| 8×8 Bayer, 9-tap filter | 64 | 0.314 | 2.44 | 0/0/0 | addendum, `cpu-evidence-001.json` |
| 8 probes, upper-half phase | 128 | 0.220; rows 125–130 up to 0.408 | 5.37 | 0/0/0 | addendum, `cpu-evidence-001.json` |
| Per-ray phase `t_i = frac((B[x][y] + 0.5)/16 + i/16)` | 64 | 0.439 | — | 0/0/0 | `evidence/ssao-filter-001/next-step/per-ray-jitter-001.json` |
| 8 probes, full stratum | 128 | 0.314 | 3.76 | 0/0/0 | `evidence/ssao-filter-001/next-step/steps8-full-stratum-001.json` |

**C2 in codes.** Its window peak-to-peak is 7, 7, 7, 3, 5, 7 and 6 codes on
rows 124–130. The bounds are 7.97 on rows 124–127 and 11.95 on rows 128–130.
The design reads 12–17 codes. C2's one N7 texel is a 16-code vertical step at
(201, 134) in `white_slab` orthographic. N7 counts steps of 16 codes or more,
and the design's largest there is 15.

**Why the comb is sampling structure, not estimator bias.** The full-stratum
family's converged expectation has a 4.14/255 peak-to-peak at the ROI, against
4.81 for the continuous limit (`cpu-evidence-001.json`). The 15-code comb of
its per-texel realisation is therefore how the samples are arranged, not what
they estimate.

**The design's tile is correlated.** Rotation uses `B[y][x]`; the phase uses
the transposed `B[x][y]`. Both coarse strata depend only on texel parity, so
the 4×4 tile fills just 4 of the 16 coarse rotation×phase cells. This is
verified in `screen-001.json` under `constructions`.

**How C1–C3 test this:**

- **C1** pairs rotation stratum `r` with phase `rev4(r)`, a 2-D Hammersley
  (0,4,2)-net. This fixes the joint stratification, but every ray of a texel
  still shares one phase: 4 distinct radii per texel. Not enough.
- **C2** also digitally shifts each ray's phase by `s(i)`, the second Sobol'
  coordinate.
  - **Per texel:** (elevation `i`, azimuth `rev4(i)`, phase `s(i)`) is a
    (0,4,3)-net, so the 64 probes fill all 64 radial strata.
  - **Per ray:** the tile pairing stays a (0,4,2)-net.
  - **Expectation:** each ray's expectation is the full-stratum family's, so
    the expectation is unchanged; only the arrangement improves.
- **C3** keeps C2's per-texel coverage but breaks each ray's equal step
  spacing (XOR 0, 5, 10, 15 per step). It is worse (0.439). Equal spacing
  along a ray helps on these stripes; C2's gain comes from stratifying the
  phase across rays.
- **256 probes are neither necessary nor sufficient here.** S16 keeps the
  correlated pairing, fails the ROI and fails N6T.

**CPU-model views.** These are CPU model planes, not renders:
`next-repair/stripes_persp_D_C2_S16.png`, `white_slab_persp_D_C2.png` and
`edge_slab_persp_D_C2.png` (darkening ×6, shown ×3).

- **Stripes:** below the stripes, D shows coarse vertical streaks at the stripe
  period. C2 leaves finer, fainter striations.
- **S16:** smoother, but it darkens a much wider area, because it catches the
  thin stripes from farther away.
- **`white_slab`:** D and C2 match.
- **`edge_slab`:** C2's outer halo loses D's blocky steps.

Rendered appearance remains unjudged.

## 3. N6: the authored metric stays failed; N6T is proposed

**Definitions.** Both use the authored 70 pairs on row 97 of `white_slab`
perspective (x 100..169, both texels Chebyshev-2 interior) and the same 1/64
bound (3.98/255).

```
N6  = max |A(x+1) - A(x)|                                                    (authored, retained, FAILED)
N6T = max |(A(x+1) - A(x)) - (T(x+1) - T(x))|                                (proposed)
T   = continuous limit of the same test (24 x 64 cosine directions, 32 steps),
      through the candidate's own H/V taps without R8 rounding (unfiltered for an unfiltered output)
```

**Evidence** (`next-repair/n6t_controls.py` writes `n6t-controls-001.json`;
CPU-only, 42 s). Values are in /255:

| Field | How it was obtained | N6 | N6T |
|---|---|---|---|
| build003, stepped and hatched | inferred per texel from the retained build003 device frames: 72 texels, 0 ambiguous, equal to the shipped model | 31.88 fail | **29.22 reject** |
| Smooth legitimate control, exact | finer limit (32 × 96 directions, 48 steps; not T's grid) through R3 | 3.79 pass | 0.39 accept |
| The same control with R3's R8 writes | as a real output stores it | **4.00 fail** | **1.02 accept** |
| The control unfiltered, one R8 write | — | 5.00 fail | 1.98 accept |
| T's own grid unfiltered, R8 | — | 5.00 fail | 0.83 accept |
| Control banded to 1/16 levels | shipped quantum | 15.94 fail | 14.78 reject |
| Control with an alternating ±1/32 hatch | — | 19.94 fail | 16.96 reject |
| Control plus one step of s codes at x 135 | s = 2, 3 / s ≥ 4 | 4.00 fail for s = 2–5 | accept for s = 2, 3 (2.65, 3.65); reject for s ≥ 4 (4.65 …) |
| R1/R3, device 001 V plane | — | 7.00 fail | 3.37 accept |
| C2, model | — | 7.00 fail | 3.19 accept |

**Orthographic diagnostic** (outside the authored scope):

| Field | N6T |
|---|---|
| Control | 1.05, accept |
| Shipped | 41.67, reject |
| R1/R3, device | 4.63, would fail |
| C2 | 4.14, would fail |

**Why the raw adjacent gradient conflates contact shape with quantization.**
On this row the true contact gradient itself steps 4.98/255 between
neighbours, or 3.82 after the filter. N6's bound, 3.98, sits on top of that
gradient.

- The exact smooth answer passes at 3.79, then fails at 4.00 once R8 stores
  it. Quantization alone decides the verdict.
- The design's 7.00 is its converged contact gradient (7.09, from the
  addendum), not banding.
- N6 cannot separate an injected 2-code step from a 5-code one: all read
  4.00.

**How N6T keeps the bound meaningful.** The bound is unchanged at 1/64. N6T
spends it only on error in the local gradient.

- **Budget used by legitimate sources:**
  - R3's three R8 writes add about 1.0/255 (the control, 1.02);
  - T's grid precision adds about 0.4/255 (exact control, 0.39; addendum,
    0.39).
- **That leaves about 2.6/255 for sampling error.**
- **Detection:**
  - one 1/16 band (15.9) exceeds the bound about 4×, and so does the hatch;
  - a single extra step of 4 codes is rejected.
- **Discrimination** (from the addendum): the design passes by 0.61, and its
  closest mutation, jitter off, fails by 0.53.

**Adoption is root's decision.** If root adopts N6T, these are proposed:

- N6 authored stays recorded as failed.
- N6T's scope stays as authored: `white_slab` perspective.
- Orthographic stays a recorded diagnostic. If root extends the scope, neither
  R1/R3 nor C2 passes it today.
- N8's 0.25 bound is unchanged.

## 4. Source changes for root, if root picks C2

**Engine, `crates/incant_render/src/ambient_occlusion/ao.wgsl`.** No uniform,
layout, target or Rust allocation change:

```diff
 const BAYER=array<u32,16>(0u,8u,2u,10u,12u,4u,14u,6u,3u,11u,1u,9u,15u,7u,13u,5u);
+// Second Sobol' coordinate of the ray index (sixteenths): per texel the rays'
+// (elevation, azimuth, phase) strata form a (0,4,3)-net.
+const SOBOL=array<u32,16>(0u,8u,12u,4u,10u,2u,6u,14u,15u,7u,3u,11u,5u,13u,9u,1u);
@@
-    // origin and deterministic across frames. Transpose the step phase.
+    // origin and deterministic across frames. The step phase pairs the rotation
+    // stratum with its bit reversal (a Hammersley net over the 4x4 tile) and is
+    // digitally shifted per ray, so each texel probes all 64 radial strata.
     let phase=vec2u(xy)%vec2u(4u);
-    let rotation=ao.rotations[BAYER[phase.y*4u+phase.x]].xy;
+    let stratum=BAYER[phase.y*4u+phase.x];
+    let rotation=ao.rotations[stratum].xy;
     let rotated_tangent=tangent*rotation.x+bitangent*rotation.y;
     let rotated_bitangent=bitangent*rotation.x-tangent*rotation.y;
-    let jitter=f32(33u+2u*BAYER[phase.x*4u+phase.y])/64.;
+    let base=reverseBits(stratum)>>28u;
     var blocked=0.;
     for(var ray=0u;ray<16u;ray++) {
+        let jitter=(f32(base^SOBOL[ray])+0.5)/16.;
         let k=ao.directions[ray].xyz;
```

**The engine's CPU mirror** is `quality_oracle.rs`, lines 77–81 and its
`for k in kernel` loop. It needs the same phase per ray:
`t = (f64::from(rev4(BAYER[y % 4][x % 4]) ^ SOBOL[ray]) + 0.5) / 16.`, with
`rev4(b) = (b as u32).reverse_bits() >> 28`. Root updates any engine test
that pins the old phase and runs the engine suite.

**Analysis side, before the 002 freeze:**

- **Oracle:** `ssao-repair/filter/oracle.py` uses `jitter(x, y)` at lines 63
  and 127. The phase must become per ray inside `k_interval`'s direction loop.
- **Positive model:** `controls.py` builds its positive model with
  `quality_model.raw` at line 33. It needs the same phase; `screen.py`'s `raw`
  already models C2.
- **Where the change goes:** put it in the 002 namespace, for example a phase
  hook installed by `ssao-filter-002/analyze.py`. That keeps 001's analyzer
  bytes, and hence its bound digest, reproducible at HEAD.
- **Without this change**, every C2 numeric gate fails by construction. With
  R1/R3 kept, no analysis change is needed.

**Then:**

1. Root runs `make_plan.py --engine-commit <sha>` in 002.
2. Root issues the build grant. The guard regression runs inside the build.
3. Root issues the GPU grant for the 9-child, 100-submission sequence.
4. After release, `ssao-filter-002/analyze.py <evidence>` runs, followed by my
   review of the 16 PNGs.

## 5. Cost

These are analytical counts per texel only. They make no performance claim,
and the phone gates need device measurements.

| | D (R1/R3) | C1 | C2 | C3 | S16 |
|---|---|---|---|---|---|
| Probe depth loads, at most | 64 | 64 | 64 | 64 | 256 |
| Centre loads (depth + normal) | 2 | 2 | 2 | 2 | 2 |
| Phase ALU added vs D | — | about +2 | about +80 (5 per ray: constant load, XOR, convert, add, multiply) | about +320 (5 per probe) | probe loop about ×4 |
| Probe-loop ALU (2 mat4 transforms, tests and dots: about 65 ops per probe) | about 4.2 k | same | about +2% | about +8% | about ×4 |
| Filter: 2 passes, 5 taps | unchanged | unchanged | unchanged | unchanged | unchanged |
| Targets, uniform, retained 14 B/px | unchanged | unchanged | unchanged, plus a 64 B shader constant | unchanged, plus 64 B | unchanged |

## 6. Commands

All CPU-only, run from the worktree root, with
`R = env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`.

| Command | Result |
|---|---|
| `R handoffs/0070-ambient-occlusion/next-repair/screen.py > …/screen-001.json` | exit 0 in 42.5 s |
| `R handoffs/0070-ambient-occlusion/next-repair/n6t_controls.py > …/n6t-controls-001.json` | exit 0 in 42.2 s |
| `R handoffs/0070-ambient-occlusion/next-repair/model_views.py` | exit 0 in 11.8 s: 3 PNGs and a manifest |
| `R …/ssao-filter-002/make_plan.py` (copied plan removed first, so the pin is null), then `--check` | exit 0: reproducible |
| `R …/ssao-filter-002/selftest.py > …/next-repair/ssao-filter-002-selftest.json` | exit 0 |
| `rustfmt 1.10.0 --edition 2024 --check` on the 002 `numeric.rs` | only the four inherited 001 differences; the new lines are clean |

**Not run:** `cargo test` and clippy for the helper, `make_plan.py
--engine-commit`, any grant, build or GPU child. The Rust regression is
uncompiled until the 002 build.

## 7. Limitations

**C2:**

- **CPU-model evidence only.** The model matched the device design to within
  1 code per texel, but C2 itself is not device evidence.
- **The ROI margin is below one code** (7 against 7.97). On the design, 6 of
  77 N8 window texels differed by one code between model and device, so C2
  could read 8 codes (0.251) on device.
- **Its N6T fails the orthographic diagnostic** (4.14), and it adds one
  threshold-level N7 texel.
- **Thin stripes:** like D, C2 detects less thin-stripe occlusion at distance
  than S16. That is a level difference that N8 does not measure.

**Scope of the evidence:**

- Fixtures are planar, with a single surface layer: radius 2, bias 0.01.
- No rendered appearance exists for any candidate.
- Costs are operation counts, not timings.

**Uncompiled and untested:**

- The Rust guard tests and the WGSL/CPU-mirror diffs are uncompiled.
- `analyze.py` was exercised only on its usage path.

## 8. Open questions for root

1. Adopt N6T for `white_slab` perspective, with N6 authored recorded as
   failed? Or keep N6 alone, in which case no estimator can pass?
2. Pick C2 for the 002 run, or keep R1/R3? R1/R3 fails N8 perspective, as
   001 measured.
3. If C2: who adds the 002-local oracle and controls phase hook, and should
   N6T's orthographic diagnostic stay a diagnostic?

## Changed paths

**New, committed with `Built-by: claude`:**

- `handoffs/0070-ambient-occlusion/next-repair-preparation.md`
- `handoffs/0070-ambient-occlusion/next-repair/`:
  - scripts: `screen.py`, `n6t_controls.py`, `model_views.py`;
  - results: `screen-001.json`, `n6t-controls-001.json`,
    `ssao-filter-002-selftest.json`;
  - model views: `model-views-manifest.json` and the 3 PNGs.
- `handoffs/0070-ambient-occlusion/ssao-filter-002/`: the namespace (14
  files).

**Unchanged:**

- `result.md`, `filter-result.md` and every earlier report and evidence file;
- `ssao-filter-001/`, `ssao-repair/` and `tools/`;
- production source;
- all 001 evidence and its latch.

Root's `brief.md` stays uncommitted, and the generated `Cargo.lock` files stay
untracked.
