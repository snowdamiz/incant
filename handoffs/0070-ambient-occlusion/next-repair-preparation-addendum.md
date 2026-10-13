# 0070 SSAO next repair, addendum: C2 oracle and revised quality report

## Status

**CPU-only preparation is complete; nothing is accepted.**

- **Not used:** Cargo, target lease, GPU, screenshots, a new execution
  namespace, ignored tests and engine source edits.
- **Unchanged:** 001, `ssao-repair/`, `tools/` and every earlier report and
  evidence file keep their bytes.

**Root's decisions, applied:**

- **C2 is selected** for the next run: Hammersley tile pairing plus a per-ray
  Sobol' phase, at most 64 probes. The engine pin stays pending until root
  commits the frozen sources.
- **N6T is adopted** as the revised scoped experimental quality measure. It
  covers the authored 70 pairs on white_slab perspective row 97, with the
  bound unchanged at 1/64 and the R3 filter unchanged.
  - Root's review: `~/.codex/incant-reviews/ssao-n6t-metric-root-20261013.json`
    (sha256 `22ea6c42…`); control replay `ce63d89d…`. I verified both hashes.
- **Authored N6 is not passed retroactively.** Every report computes it with
  its actual value and verdict, beside 001's retained failure (7.00/255 at
  x 161).
- **Orthographic N6T stays a named diagnostic.** It currently fails.
- **N8's 0.25 bound and all other thresholds are unchanged.**

**Delivered in `ssao-filter-002/`:**

- `c2_analysis.py`: the C2 oracle and the revised quality report;
- `c2_controls.py`: the C2 controls;
- `analyze.py`, rewired to the C2 analysis;
- `make_plan.py` and `plan.json`, bound to C2;
- `selftest.py`.

**Results:**

- The full C2 controls pass (exit 0, 193 s).
- The 002 selftest passes (exit 0).
- The C2 analysis rejects the retained 001 R1/R3 device output (§3).

**Still open:**

- GPU quality is not measured. C2's CPU-model N8 margin is 0.97 codes, which
  is not acceptance.
- Appearance needs the device run's 16 PNGs.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**. ACP verification is root's.

**Screenshots:** none. `result.md` and the earlier reports are preserved; this
addendum is the packet's return document.

## Correction

**What I claimed.** I wrote "keep N6 alone, in which case no estimator can
pass" (`next-repair-preparation.md` §8). In `ssao-quality-repair-addendum.md` I
wrote that in orthographic "no estimator could pass" N6.

**Both are wrong as absolute claims.** A constant field passes raw N6
trivially, but it removes the occlusion the effect exists to produce.

**The accurate statement:** on this row, a field that reproduces the converged
contact gradient fails raw N6 once R8 stores it. The independent smooth
control reads 4.00/255 against a bound of 3.98.

## 1. The C2 oracle, local to 002

**Phase.** This is root's frozen `ao.wgsl`:

```
stratum = B[y % 4][x % 4]                              (rotation unchanged)
t_ray   = ((rev4(stratum) ^ SOBOL[ray]) + 0.5) / 16    (exact in f32)
SOBOL   = 0 8 12 4 10 2 6 14 15 7 3 11 5 13 9 1
```

**`C2FilterOracle`** subclasses the shared `FilterOracle` and overrides
`k_interval` alone, moving the phase into the ray loop. Everything else is
inherited unchanged:

- R8 code intervals and ties;
- tap weights and tolerance bands;
- ambiguity decisions and the decisive-width bound.

**Wiring.** `install()` rebinds two things in the shared analyzer:
`analyze_filter.oracle_for`, to the C2 factory with its own cache, and the
plan check, to the 002 plan. Every numeric gate is the shared one, including
the stage readback of all 15 declared stage frames.

**Source binding.** `c2_analysis.REUSED` pins the sha256 of the nine reused
files:

- `analyze_filter.py`, `oracle.py`, `controls.py` and `cases.json`;
- `quality_model.py`, `quality_report.py` and `sphere_rise_bound.py`;
- `tools/analyze.py` and `tools/ssao_reference.py`.

Import refuses (`SourceBindingError`) if any byte differs. A control copies the
files, changes one constant in `oracle.py`, and checks that the copy is
refused.

**Engine binding.** `make_plan.py --engine-commit <sha>` refuses a commit
unless two things hold:

- its `ao.wgsl` is byte-identical to root's frozen C2 shader
  (`4aaa0cc0ed375555…`);
- it carries exactly the four C2 phase lines and the SOBOL table, and not the
  R1/R3 phase.

`verify_engine(5dfc522e)` is refused, as the selftest shows. Root's frozen
shader, which is present on this host, matches both the hash and the phase
lines. The table equals the Sobol' direction-number construction that root's
CPU mirror uses.

**Fidelity.** Given the R1/R3 phase, `C2FilterOracle` reproduces the shared
oracle's k intervals on all 43,489 stripes-perspective interior texels. The
override changes the phase and nothing else.

## 2. Controls (`next-repair/c2-controls-001.json`)

**Positive.** The model is an independent f64 C2 model: `quality_model`'s test
with the phase rebuilt from Sobol' direction numbers and an arithmetic bit
reversal.

| Fixture | Filtered reference | Stage readback | Certainly occluded | Final widths (codes: texels) | Ties up / down |
|---|---|---|---|---|---|
| `white_slab` p | pass | pass | 11,352 | 0: 38,181, 1: 4,516, 2: 792 | 354 / 0 |
| `white_slab` o | pass | pass | 9,821 | 0–3 | 114 / 0 |
| `edge_slab` p | pass | pass | 12,727 | 0–3 | 393 / 0 |
| stripes p | pass | pass | 18,773 | 0–3 | 554 / 0 |
| stripes o | pass | pass | 6,670 | 0–3, plus **66 non-decisive texels of 4–6** | 83 / 0 |

The shared perturbations still fail as before: a raw code +1 gives
`raw_outside 1`, and a final code below its interval gives `v_outside 1` and
`v_outside_final 1`.

**R1/R3 cannot pass as C2, and C2 cannot pass the shared R1/R3 oracle.** The
first five rows feed R1/R3 model planes to the C2 gates. The last two feed C2
planes to the shared R1/R3 oracle. Every row fails.

| Fixture | Raw texels differing | C2 stage `raw_outside` | Frames-only gate |
|---|---|---|---|
| `white_slab` p | 3,284 | 3,374 | fails, 1,012 disagree |
| `white_slab` o | 2,718 | 2,689 | fails, 632 disagree |
| `edge_slab` p | 5,389 | 5,401 | fails, 2,732 disagree |
| stripes p | 9,249 | 9,330 | fails, 4,452 disagree |
| stripes o | 3,100 | 3,080 | fails, 1,062 disagree |
| C2 planes under the R1/R3 oracle, `white_slab` p | — | 3,270 | — |
| C2 planes under the R1/R3 oracle, stripes p | — | 9,296 | — |

**Wrong variants fail.** Each count is the stage-gate tally on `white_slab` p,
then stripes p. The frames-only gate also fails on both fixtures in every row.

| Variant | Stage gate (`white_slab` p / stripes p) |
|---|---|
| Shared phase (C1) | raw 2,728 / 9,094; final 6,510 / 13,845 |
| Transposed base index | raw 2,546 / 7,561; final 6,079 / 14,192 |
| Reversed ray order | raw 1,190 / 4,976; final 5,302 / 12,905 |
| Stratum start offset (no +0.5) | raw 769 / 2,935; final 4,240 / 10,055 |
| Vertical pass missing | V 8,954 / 17,002; final 8,109 / 16,512 |
| Both filter passes missing | H 8,261 / 16,278; V 7,540 / 15,186; final 9,249 / 18,225 |

**Shared refusals, reused unchanged:**

- the shared `controls.ambiguous_and_empty`, run against C2 evidence:
  conflicting RGB, no diffuse signal, every ray ambiguous, inflated widths, an
  absence mismatch, and a missing plane or frame;
- 002 report controls:
  - empty evidence;
  - a foreign build record, and 001's own build record;
  - an empty index and a zero-length plane;
  - the 002 CLI, which writes a failing report (exit 1) and never replaces it
    (exit 2).

**N6T on fields whose answer is known.** Values are in /255; the bound is
3.98.

| Field | N6T | Verdict | Authored N6 |
|---|---|---|---|
| Independent smooth contact (finer limit, R3, R8) | 1.02 | accept | 4.00, fail |
| Shipped stepped output (= build003 device), against the unfiltered truth | 29.22 | reject | 31.88 |
| Shipped stepped output, against the R3 truth | 29.34 | reject | 31.88 |
| Control banded to 1/16 | 14.78 | reject | 15.94 |
| Control with a ±1/32 hatch | 16.96 | reject | 19.94 |
| Control plus a 4-code step | 4.65 | reject | 4.00 |
| Control plus a 3-code step | 3.65 | accept | 4.00 |

## 3. The revised quality report

The report that `analyze.py <evidence>` writes contains:

- **`quality_gates`, the revised set:**
  - `quality_n6t_revised_persp`;
  - N7 on the three fixtures;
  - N8 on the perspective and orthographic ROI rows.

  These give `quality_pass`.
- **`authored_n6`:** the actual value and verdict, with `retained_001` beside
  it. These give `authored_quality_pass`.
- **`diagnostics`:**
  - `n6t_ortho`;
  - `n8_rows_persp`, rows 124–130;
  - `n8_rows_ortho`, rows 110–116.

  Each row carries its own verdict, and failing rows are listed.
- **`pass`:** numeric pass and revised quality pass.
- **`acceptance`:** none. These are measurements for review.

**C2 model** (controls):

- Revised gates all pass: N6T 3.19, N7 0 / 1 / 0, N8 0.220 and 0.075.
- Authored N6: **7.00 at x 164, failed**.
- Orthographic N6T: **4.14, failing** (diagnostic).
- N8 rows:

  | Projection | Row ratios | Failing rows |
  |---|---|---|
  | Perspective, 124–130 | 0.220 0.220 0.220 0.094 0.105 0.146 0.125 | none |
  | Orthographic, 110–116 | 0.075 0.075 0.125 0.125 0.105 0.105 0.084 | none |

**R1/R3 model** (controls):

- N8 perspective fails.
- N6T 3.37; authored N6 **7.00 at x 161, failed**; orthographic N6T 4.63.
- N8 rows:

  | Projection | Row ratios | Failing rows |
  |---|---|---|
  | Perspective | 0.376 0.439 0.502 0.533 0.356 0.335 0.272 | **all 7** |
  | Orthographic | 0.176 0.163 0.173 0.167 0.167 0.125 0.146 | none |

**R1/R3 device evidence from 001** (`next-repair/c2-on-001-device.json`; the
C2 analysis ran in process and read only, and the evidence stayed
byte-identical):

**Numeric: `numeric_pass` is false.** The 001 build record also belongs to 001's
plan (`plan_matches_build` false), but the data fails on its own: **26 of 43
numeric gates fail**.

- **Failing:** 13 of the 15 filtered-reference gates (125–4,578 texels
  disagree) and the same 13 stage gates (2,689–9,330 raw codes outside the
  C2 intervals).
- **Passing:** the two absence frames (motion far, skin t0), where the two
  samplings cannot differ, and all 13 sampling-independent gates (identities,
  determinism, DRS, bloom centroid, mixed pane).

**Quality, from the device V planes:**

| Measure | Result |
|---|---|
| Revised gates | only N8 perspective fails |
| N6T | 3.37, pass |
| N7 | 0 / 0 / 0 |
| Authored N6 | **7.00 at x 161, failed** |
| Orthographic N6T (diagnostic) | 4.63, failing |
| N8 perspective rows 124–130 | 0.376 0.439 0.502 0.533 0.356 0.335 0.272: **all 7 fail** |
| N8 orthographic rows 110–116 | 0.176 0.163 0.188 0.167 0.167 0.125 0.146: all pass |

## 4. The 002 plan and selftest

**Unchanged:**

- 9 GPU children;
- 100 of 128 new submissions;
- 16 PNGs and 11 pairs, with `pngs_max` 32 and `pairs_max` 16;
- the timeouts: controls 60 / 120 / 60 s, numeric 150 s, appearance 300 s;
- the guard build step;
- 001 untouched.

**Changed:**

- `source_binding` records root's frozen C2 source, the pending pin and its
  `ao.wgsl` binding, and the pinned C2 oracle.
- `analysis` records the C2 sampling, the reused pins, the revised measure,
  authored N6 retained, and the diagnostics.

**Selftest additions** (`next-repair/ssao-filter-002-selftest-002.json`):

- the plan's C2 bindings;
- `verify_engine` refusing 5dfc522e;
- the tampered-pin refusal;
- shader acceptance and refusal;
- the phase tables;
- a 207-texel fidelity sample;
- digest coverage of the new modules.

**A first run failed, exit 1.** The shared analyzer puts `tools/` first on
`sys.path`, so `import make_plan` loaded build004's file. The selftest now
loads 002's `make_plan.py` by path, and the rerun passes.

| Binding | sha256 |
|---|---|
| `ssao-filter-002/plan.json` (engine pin `null`) | `7c18cb5bd778db317b16096d850ccb5ae00a549753204ccf55e6cff44c91f611` |
| 002 helper digest (29 files: namespace, 6 reused tools, 7 analyzer files) | `27c76cdb72c0bd616cf735bdb5365e5af58e6087ed84d6de498addac469ca74d` |
| `c2_analysis.py` | `43ef7d44f8658b087de0e76ad8ebcd5c535e6263a96020db1d97251d1cdf36c0` |
| `c2_controls.py` | `040ffcff99dc19a255ff098ea06891d51325a05834df1d3807d1f11cc0ff7f32` |
| `analyze.py` | `853ae0dde222ddbe7a195b6cc890530ce1a218ad671673d52b71c3a414d8c7fb` |
| `make_plan.py` | `e926c500b87eccccb4272a4ee0bb29d2ffceec529ea001d6dabe67e43f0d0440` |
| `selftest.py` | `e81ed6e4b75c132710cd1fd0e4596c3f641190ba472996f0987c97c62a05c68b` |
| `effects/src/numeric.rs` (unchanged since 766dc101) | `6f8108a18b7c74651d79f162dce68066fcb37ff6d491c8a1276ce9595d7e7a37` |

Pinning the engine changes `plan.json`, and so changes the digest.

## 5. The Rust guard is still uncompiled

The three `sphere_guard_tests` in the 002 `numeric.rs` have still not been
compiled, because no packet so far has allowed Cargo.

- **When they run:** root's 002 build runs them as `build-helper-guard-tests`
  (`cargo test --lib -- sphere_guard`), CPU only, before any GPU child.
- **What blocks the build:** `guard_verdict` refuses unless exactly those
  three tests pass.
- **What has been shown so far:** the Python twin in the selftest shows the
  fixture separates the predicates. 001's predicate counts 601 rim texels and
  002's counts 0. For the perspective disc, the predicates count 3,419 and
  2,818.

## 6. Commands

All CPU-only, with `R = env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`.

| Command | Result |
|---|---|
| `R ssao-filter-002/analyze.py --selftest > next-repair/c2-controls-001.json` | exit 0 in 193 s; stderr empty |
| `R ssao-filter-002/make_plan.py` (plan removed first), then `--check` | exit 0: reproducible, pin `null` |
| `R ssao-filter-002/selftest.py > next-repair/ssao-filter-002-selftest-002.json` | exit 0 (the first attempt exited 1, as described in §4) |
| `R next-repair/c2_on_001.py > next-repair/c2-on-001-device.json` | exit 0 in 699 s wall (112 s CPU); the evidence file hashes were identical before and after |

## 7. Limitations

**C2:**

- **CPU model and oracle only.** C2 has no device result, and its N8 margin is
  under one code. GPU quality and appearance stay open.
- **The C2 oracle is less decisive on orthographic stripes.** 66 non-decisive
  texels have final intervals 4–6 codes wide; R1/R3 had at most 3. Decisive
  texels keep the 3-code bound, but the contract is looser at those texels.
- **Orthographic N6T fails** for both C2 (4.14) and R1/R3 (4.63). It is a
  diagnostic only.

**Engine binding:**

- It requires root's committed `ao.wgsl` to be byte-identical to the frozen
  file. A review change needs re-binding before the pin.
- The `quality_oracle.rs` and `quality_tests.rs` hashes are recorded only.
  They do not affect the GPU run.

**Coverage:**

- Fidelity is exhaustive on one fixture and sampled in the selftest.
- The Rust guard is uncompiled (§5).

## 8. Root's next steps

No further permission is needed from me.

1. Commit the three frozen C2 sources on this branch.
2. Run `make_plan.py --engine-commit <sha>`. It refuses a non-matching shader.
3. Issue the 002 build grant. The guard regression runs inside the build.
4. Issue the GPU grant for 9 children and 100 submissions, then release.
5. Run `ssao-filter-002/analyze.py <evidence>`, then my review of the 16 PNGs.

## Changed paths

**Committed with `Built-by: claude`:**

- **Changed in `ssao-filter-002/`:** `analyze.py`, `make_plan.py`,
  `plan.json` and `selftest.py`.
- **New in `ssao-filter-002/`:** `c2_analysis.py` and `c2_controls.py`.
- **New in `next-repair/`:**
  - `c2-controls-001.json` and `ssao-filter-002-selftest-002.json`;
  - `c2_on_001.py` and `c2-on-001-device.json`.
- **New:** this file.

**Unchanged:** `ssao-filter-001/`, `ssao-repair/`, `tools/`, every earlier
report and evidence file, all 001 evidence and the engine source. `brief.md`
is not committed.
