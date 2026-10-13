# 0076 shadow receiver guard — visual comparison preparation

**Status: PREPARED. Execution is pending root's build and GPU grants.**

- Nothing was built, compiled, rendered or captured. No Cargo command, GPU
  process, screenshot, screen control or target lease ran.
- The evidence namespace `~/.codex/incant-evidence/0076-shadow-guard-review-001`
  did not exist when the self-check ran. With the grants absent, every guarded
  command refused and created nothing. Root has since created it to hold the
  two source freezes; see the post-commit note at the end.
- No production file changed. This is Phase 1 rendering correctness and quality
  preparation. It approves no phase gate and makes no performance or device claim.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max,
as this session's environment reports them.

**Packet:** `brief.md` sha256 `c83c30c845162687e7f039c64a9d80a61ed5cc44631108e5f03a1d227ebe0994`.
- Root committed it in `3f3630ba`, and it is unchanged.
- It holds no priority revision or director feedback beyond its own text. I
  rechecked it before committing.

## 1. What I read

This worktree's files are its committed HEAD. All 0072 material came from Git
objects at `48a688c5`; I did not open any live 0072 tool.
- **Conventions:** `CLAUDE.md`, `AGENTS.md` and `PLAN.md`. From `PLAN.md` I read
  §4.2–4.6 (routing, handoff protocol, operating rules), Phase 1, §6.2 Rendering
  and §8 Quality. This work is Claude's TAA and shadow quality judgment, split
  from Astra's correctness work.
- **The candidate:** `docs/spikes/shadow-receiver-guard.md` and the 10-path diff
  `0aa02559..f636a2d5`. That is 4 production files, 3 test files and 3 docs.
- **The 0072 diagnosis, at commit `48a688c5`:**
  - `sharpness-result.md`: §5 Viewport, §6 (the cascade row) and §10 question 1;
  - `result.md`;
  - `fixed-sharpness-inspection/cascade_texels.py` and its `.txt` output.
- **The 0072 framework, at `48a688c5`:** visual tree `b11a7d3a`, with root's
  `cd4f4cd7` build fix included. The live 0072 worktree has uncommitted
  preparation; I listed its status but never opened its modified files.

## 2. Findings that shape the run

### 2.1 The 0072 fixture needs one line to compile against main224

- **The break.** main224 added a required field, `UiWidget::halos` (`c6a7804a`).
  The fixture's one UI button literal lacks it, so the file cannot compile
  unchanged. That literal is `ui_input`, which only the `excluded` case uses.
  `a143b5c0`, the last source the fixture compiled against, has no such field.
- **The fix.** I inserted one line, `halos: None,`, at line 1207.
  - A Button must carry `None`: `incant_render` `ui/geometry.rs:118-122` refuses
    halos on any role but Label.
  - Nothing else changed. The `viewport` case, everything it calls and
    `OUTPUT_ENV` (still `INCANT_0072_REVIEW_OUT`) are byte-identical.
  - Both engines use the same file.
  - `selfcheck.py` proves the result is the 0072 blob plus exactly that one line,
    inside `ui_input`, which the viewport case never reaches.
- **Hashes.** The fixture goes from
  `d4fc40029f7e7995719bf058806f9a79936f11c8ac63e7905ac79414eb281338` to
  `5955efa7b3190c55b7629269a8736cc02ca2d678957b853b78471141e69f67e2`.
- **Other dependencies.** No other API the fixture uses changed between
  `a143b5c0` and main224: `incant_doc`, `incant_types`, `incant_assets`,
  `incant_assets_runtime`, `incant_render_authoring`, the test `support`,
  `materials` and skeletal fixtures are identical, and every used method exists.
- **Verification.** `rustfmt --edition 2024 --check` (stdin, toolchain 1.99.0)
  exits 0. It is not compiled here; root's build and strict Clippy decide.

### 2.2 Temporal-on frames should not change

- **Baseline.** In main224 every temporal frame with a jitter phase already
  carries `guard(plan.rect)`, set at `retained.rs:434-435`. Shadow preparation
  fits with that camera and `target.rect[2]/target.rect[3]`, where `target.rect`
  equals `plan.rect` (`model_draw.rs:76`).
- **Candidate.** `cascade.rs::fit_for_viewport` writes the same
  `guard(rect)` and the same aspect into a local copy. So temporal-on cascades are
  bit-identical across the two sources, and only the temporal-off references
  change.
- **The check.** The comparison requires all 24 temporal-on frame hashes, and the
  on-f24 PNG, to be byte-identical across roles. This is a derivation from
  source, not a measurement (see §9).

### 2.3 What the candidate's temporal-off reference should look like

0072's own pair at this exact pose shows the effect. Its temporal-on frame, with
the guarded fit, has a **solid** grille shadow. Its temporal-off frame, with the
unguarded fit, has a **faint hatch**. Under the candidate the off reference uses
the guarded fit, so I expect it to turn solid like the on frames.

**Neither look is representable detail.** At cascade 1, using the 0072 texel sizes
(`regions.py grille_sampling`):

| Fit | Bar width (shadow texels, x / y) | Vertical bar spacing | Horizontal bar spacing |
|---|---|---|---|
| Viewport, unguarded (baseline off) | 0.363 / 0.391 | 1.451 | 2.346 |
| Viewport, guarded (on; candidate on and off) | 0.361 / 0.389 | 1.445 | 2.337 |
| 960×540 | 0.360 / 0.388 | 1.439 | 2.328 |

- Bars are thinner than a texel, and the vertical bars repeat below the 2-texel
  Nyquist spacing.
- So the hatch and the solid shadow are both aliases. The grille is 42.3% solid,
  and its true average occlusion is neither.
- The candidate makes the alias consistent across temporal modes. It does not make
  it correct; that would need alpha-aware shadow filtering, which is out of scope.

### 2.4 The baseline should reproduce the defect

- **Unchanged code.** The shadow, camera, lighting, material and resolution code
  is byte-identical between `a143b5c0` (0072's C-reg run) and main224.
- **Resolve.** main224's resolve is the accepted fixed-coverage WGSL; the 0072
  run used C-reg.
- **Output path.** `output.rs` gained an opt-in bloom path, which the fixture never
  enables.
- **Expectation.** The baseline should show the same solid-on versus hatched-off
  mismatch, only blurrier. The mean-gap metric is blind to blur (§6.3).

## 3. Bounded changes (two commits, `Built-by: claude`)

**Commit 1: `1d60f936`.** It imports the committed 0072 `plan.json`, fixture and
tools verbatim; every blob id equals its 0072 blob. Diffing against that commit
shows the whole adaptation:

```
git diff 1d60f936 -- handoffs/0076-shadow-guard-review/visual
```

**Commit 2** (this commit) holds the adaptation, the receipts and these documents.
Full sha256 values for every tool are in `visual/preparation/selfcheck.json`
(`tools`), and the receipts' own values are listed in §7.
The final `visual/` tree is `8d9de3edbbe6f2c2f66ccff7e3ef97493e4dd65d`; root pins
it in both grants.

| Path (`visual/…`) | 0072 blob → 0076 sha256 | Change |
|---|---|---|
| `review/temporal_visual_review.rs` | `ae7cb9bd` → `5955efa7…e69f67e2` | The one line in §2.1 |
| `plan.json` | `ee2ca9ac` → `a8a0cd64…e4bfcc70` | Rewritten for 0076: sources, the 0072 viewport case spec verbatim, budgets, comparison, calibration, pairs |
| `tools/common.py` | `672b515d` → `302e39c9…38c3c71f` | Names; two roles with per-role checkout, build and capture directories; per-role source pins in `build-ready.json` (each commit must equal the plan's); per-role build-record pins in `review-ready.json`; `numeric_binary` removed |
| `tools/build.py` | `20174c88` → `9f06de6f…985d62e9` | `checkout`, `toolchain`, `review` and `lint` per role. `libtests` and the library-linkage comparison are removed, since no numeric control runs. `.cargo` joins the build inputs. |
| `tools/gpu.py` | `57629df3` → `e6702cd9…85f53120` | `list <role>` (the binary must list the fixture's 8 cases) and `review <role> viewport` (baseline first, once each). Numeric controls are removed. Lock, debit, supervise, ledger and latch are unchanged. |
| `tools/analyze.py` | `0c7ec469` → `b4b4e3dc…ebce9187` | Two-role integrity: both build records bound, plan commits, different binaries, order. `compare` uses `shadowfit`; `pairs` is new; publication is role-prefixed. |
| `tools/lease.py` | `8c95fa25` → `43adb11d…5f5dd153` | Wording only |
| `tools/sharpness.py` → `tools/shadowfit.py` | `fd7fd698` → `97241545…61b6e887` | The image reader, edge mask, blur and off-edge bins are reused. The comparison, guards, region metrics, calibration and inherited observations are new. Git records this as a delete plus an add. |
| `tools/regions.py` (new) | — → `9232e516…23b11598` | 0072's `cascade_texels.py` projection math, reproduced exactly, used for the grille-shadow, card and sphere masks and the sampling table |
| `tools/pairs.py` (new) | — → `1d09144f…05af0e8a` | Magnified side-by-side crops and difference maps (viewing aids) |
| `tools/selfcheck.py`, `test_analyze.py`, `test_lifecycle.py`, `test_source.py` | `570b0f20`, `99de1012`, `d3b15ae9`, `bd7516c8` → `71962f39…e1fd47d1`, `269d0e3c…5dd7b5cd`, `60a7c8f9…b5e96810`, `7d87daa9…cfc3c2a1` | New counts, sources, fixture proof, calibration and two-role controls |
| `tools/test_library_linkage.py` | `97a345ce` → removed | Its function left with `libtests` |
| `tools/pngread.py` | `d012faf2` (unchanged) | `b627e402…4f924ae0` |
| `preparation/*.json` (new) | — | Receipts (§7) |

**Not changed:** production code, `brief.md`, every 0072 file and evidence
directory, and any `~/.codex` file.

## 4. The run

Every count below was read from the fixture, not inferred.

**The viewport case, per role:**
- **Submissions.** `Report::open("viewport", 26, 0)` gives 26 world submissions
  and no UI submission. They are one temporal-off frame at the still pose, a
  loop of 24 temporal-on frames (`for k in 1..=24`) and one temporal-off frame at
  the pan pose.
- **Output.** Two PNGs: `viewport-on-f24.png` and `viewport-off-f24.png`, both
  960×540.

| Item | Per role | Total | Ceiling |
|---|---|---|---|
| World submissions | 26 | **52** | **60** |
| UI submissions | 0 | 0 | — |
| PNGs, 960×540 | 2 | **4** | **4** |
| Attempts | 1 | 2 | — |

- **Reserve.** The 8-submission reserve cannot fund a retry, because a case costs
  26. There is no retry, numeric control, shader control or whole-suite run.
- **Timeouts.** Listing 60 s, case 120 s, build 2,700 s.

**CPU builds**, root-supervised in the foreground under `build-ready.json`. Run
each from the worktree root with `/opt/homebrew/bin/python3 -I -B`, which is
3.12+ as `tarfile`'s data filter requires.

```
handoffs/0076-shadow-guard-review/visual/tools/lease.py acquire
handoffs/0076-shadow-guard-review/visual/tools/build.py checkout baseline
handoffs/0076-shadow-guard-review/visual/tools/build.py toolchain baseline
handoffs/0076-shadow-guard-review/visual/tools/build.py review baseline
handoffs/0076-shadow-guard-review/visual/tools/build.py lint baseline
handoffs/0076-shadow-guard-review/visual/tools/build.py checkout candidate
handoffs/0076-shadow-guard-review/visual/tools/build.py toolchain candidate
handoffs/0076-shadow-guard-review/visual/tools/build.py review candidate
handoffs/0076-shadow-guard-review/visual/tools/build.py lint candidate
handoffs/0076-shadow-guard-review/visual/tools/lease.py release
```

- **Review build:**

  ```
  <checkout>/<role>/tools/cargo test -p incant_render --test temporal_visual_review --no-run --locked --offline --profile ci -j2 --message-format=json-render-diagnostics
  ```

- **Lint:**

  ```
  clippy -p incant_render --test temporal_visual_review <same flags> -- -D warnings
  ```

- **Isolation.** Each role is a `git archive` export of its granted commit under
  `checkout/<role>`, and each binary is frozen by hash under `bin/<role>`. Both
  share the one granted target, one role after the other.

**GPU**, under `review-ready.json`. Each child takes the global non-waiting
`native-gpu.lock`, is debited before launch and has its process group killed and
reaped.

```
handoffs/0076-shadow-guard-review/visual/tools/gpu.py list baseline
handoffs/0076-shadow-guard-review/visual/tools/gpu.py review baseline viewport
handoffs/0076-shadow-guard-review/visual/tools/gpu.py list candidate
handoffs/0076-shadow-guard-review/visual/tools/gpu.py review candidate viewport
handoffs/0076-shadow-guard-review/visual/tools/gpu.py release
```

- **Child command:**

  ```
  bin/<role>/temporal_visual_review viewport --ignored --exact --nocapture --test-threads=1
  ```

  It runs in `checkout/<role>/crates/incant_render`.
- **Environment.**
  - `PATH=/usr/bin:/bin:/usr/sbin:/sbin`;
  - a fresh `TMPDIR`;
  - `RUST_BACKTRACE=1`;
  - `INCANT_EXPECT_GPU_BACKEND=Metal`;
  - `INCANT_0072_REVIEW_OUT=<evidence>/captures/<role>/viewport`;
  - `HOME`, passed through but not recorded.

**CPU analysis:**

```
handoffs/0076-shadow-guard-review/visual/tools/analyze.py check
handoffs/0076-shadow-guard-review/visual/tools/analyze.py compare
handoffs/0076-shadow-guard-review/visual/tools/analyze.py pairs
handoffs/0076-shadow-guard-review/visual/tools/analyze.py publish
```

1. `check` is the fail-closed integrity summary.
2. `compare` writes the verdict. It exits 1 on any failed check, and the record is
   kept.
3. `pairs` writes the viewing aids into `analysis/pairs-<time>/`.
4. My personal inspection follows; see §8.
5. `publish` writes `evidence/` and `screenshots/<role>-viewport-*.png`. It runs
   only from a current passing summary.

## 5. The grants root writes

The tools refuse anything else.

- **`build-ready.json`:**
  - `packet`: `0076-shadow-guard-review`; `stage`: `shadow-guard-build`;
  - `holder`;
  - `sources.baseline`: `source_commit`
    `0aa025591ab5c82e00d6d2b70002024ed3462e2a`, plus `source_freeze` (path and
    sha256);
  - `sources.candidate`: `source_commit`
    `f636a2d5e595cffae702d078bcee61f34ab35394`, plus `source_freeze`;
  - `visual_tree`: `8d9de3edbbe6f2c2f66ccff7e3ef97493e4dd65d`;
  - one `target` under `~/.codex/*-target`, for example the core target;
  - optional `prior_*_path` / `_sha256` pins.
- **Freeze format.** Each freeze uses root's 0072 format,
  `{"head": commit, "files": [{path, sha256, bytes}]}`.
  - It must cover every file under `crates`, `Cargo.toml`, `Cargo.lock`,
    `third_party`, `tools/cargo`, `rust-toolchain.toml` and `.cargo`.
  - `.cargo` is new to the build-input roots; root's 0072 freeze already listed
    `.cargo/config.toml`.
  - The candidate's freeze must differ from the baseline's in exactly the 7 Rust
    build-input paths of the delta.
- **`review-ready.json`:**
  - `packet` and `stage`: `shadow-guard-visual`;
  - `holder`;
  - the same `visual_tree`;
  - `build_records.baseline` and `build_records.candidate`, each the sha256 of
    `build/<role>/review.json`;
  - at least one `prior_*release*_path` / `_sha256` pin.

## 6. Measurements, thresholds and image pairs

### 6.1 The grille-shadow region

The region comes from the prior diagnosis.
- **The shadow.** `regions.py` carries the grille card's outline along the sun
  (travel direction 0.6824, −0.6157, 0.394) to the ground, then projects it at the
  final pan pose into the viewport.
  - This is 0072's `cascade_texels.py` math. Its card-shadow lines reproduce
    `cascade_texels.txt` exactly, which the self-check verifies.
  - Corners, in output pixels: (443.9, 278.8), (550.5, 274.5), (695.2, 300.5)
    and (579.9, 305.8).
- **Margins.** The region is grown by 4 px. The card quad and the analytic sphere,
  each grown by 3 px, are removed, and the result is clipped to the viewport.
- **Result.** 5,700 pixels, bounding box [440, 271, 699, 310). I checked by
  overlay on the retained 0072 pair that it encloses the whole shadow in both
  modes and touches neither the card nor the sphere.

### 6.2 Metrics

All metrics work in linear-light luminance (0–1), decoded from the PNGs, over the
region.

- **`mean_gap`**: `|mean(on) − mean(off)| / mean(off)`. This measures how much
  shadow there is. It is blind to blur, because a blur keeps the mean.
- **`blur_residual`**: the minimum, over σ ∈ {0, 0.25, …, 2} px, of
  `RMS(on − G_σ(off))`. This is the part of the on/off difference that blurring
  the reference cannot explain.
- **`lowpass_max`**: `max |G₂(on) − G₂(off)|`. This catches a local
  solid-versus-open patch that the means could hide.
- **Still pose:** the report's `still_f16_vs_off.interior.max_delta`. The fixture
  keeps no still-pose PNG.

Per-pixel max-channel steps are recorded but never judged: a pure blur reaches 36
to 49 steps (§6.3).

### 6.3 Calibration (retained evidence at `48a688c5`, recomputed by `selfcheck.py`)

**The rule:** each limit is the geometric mean of the worst consistent-fit control
and the best mismatch control, rounded down to two significant figures.

| Control | Kind | mean_gap | blur_residual | lowpass_max | max step |
|---|---|---|---|---|---|
| Fixed coverage, 960×540 still (main224's resolve) | consistent | 0.00052 | 0.00414 | 0.02188 | 26 |
| Fixed coverage, pan f6 | consistent | 0.00091 | 0.00316 | 0.01357 | 19 |
| C-reg, still | consistent | 0.00216 | 0.00307 | 0.00324 | 7 |
| C-reg, pan f6 | consistent | 0.00063 | 0.00410 | 0.01350 | 15 |
| Synthetic: reference blurred at σ 0.9 | consistent | 0.00009 | 0.00328 | 0.01231 | 36 |
| Synthetic: reference blurred at σ 1.3 | consistent | 0.00022 | 0.00189 | 0.02333 | 49 |
| **C-reg viewport pan f24 (the diagnosed defect)** | mismatch | 0.19741 | 0.17062 | 0.32108 | 95 |
| Synthetic: defect through a σ 0.9 blur | mismatch | 0.19742 | 0.16719 | 0.32075 | 95 |
| Synthetic: defect through a σ 1.3 blur | mismatch | 0.19757 | 0.16412 | 0.32023 | 95 |

**Still-pose interior max delta** (report values):
- consistent: fixed-coverage stationary 24 and C-reg stationary 46;
- mismatch: C-reg viewport still 96.

**Notes on the controls:**
- The consistent-fit controls are full-frame poses. There the guard leaves cascade
  1 unchanged.
- The synthetic controls are labelled. Each one blurs or splices retained PNGs.
  They test the metrics, not the renderer.

**Limits:** `mean_gap` ≤ **0.020**, `blur_residual` ≤ **0.026**, `lowpass_max` ≤
**0.086**, still interior max ≤ **66**.

**Separation:**
- every consistent control meets every limit;
- every mismatch control fails every limit;
- the margins are symmetric by construction: the limit over the worst consistent
  value equals the best mismatch value over the limit.

| Metric | Limit ÷ worst consistent | Best mismatch ÷ limit |
|---|---|---|
| `mean_gap` | 9.3× | 9.9× |
| `blur_residual` | 6.3× | 6.3× |
| `lowpass_max` | 3.7× | 3.7× |
| Still interior max | 1.43× | 1.45× |

The still-pose margin is the thinnest. That is why the region metrics, which are
measured on the PNGs, carry the main judgement.

### 6.4 Verdict checks (`analyze.py compare`; every one is required)

1. **Exactness.** If any of these fails, the comparison is void and nothing is
   attributed to the fix.
   - both reports and all four PNGs are present;
   - the source commits equal the plan's;
   - the same fixture sha;
   - the same features, `default` and `physics-deterministic`;
   - the same Cargo arguments, `Cargo.lock` and `rust-toolchain.toml`;
   - the same `rustc`, `cargo`, macOS version and hardware model;
   - the same adapter, with the Metal backend;
   - the same review config;
   - the same viewport rect and internal size ([37, 19, 885, 501] and [960, 540]);
   - **all 24 temporal-on frames and the on-f24 PNG are byte-identical.**
2. **Guards.**
   - **Outside the viewport:** a maximum delta of 0 within each run, and the
     surround in both PNGs is byte-identical across runs, with all 75,015
     pixels equal to the canvas.
   - **No new border artifact:**
     - the candidate's 3 px border band maximum is not above its inner
       maximum, at both the still and the pan pose;
     - the off-reference before/after change is not concentrated in the band.
   - **MASK geometry:** the grille-bar mask inside the projected card is
     identical in both off references. A bar pixel is G ≥ 60 and
     G ≥ 1.3·max(R, B). The 0072 pair has 5,047 of 11,891 card pixels as
     bars (42.4%), and at least 4,000 are required. The card's visible face
     turns away from the sun (N·L = −0.394), so its bars should not depend on
     the fit. The identical-mask check verifies this rather than assuming it.
3. **Agreement (the candidate, within its own source):** the three region limits
   and the still limit (§6.3).
4. **Defect demonstrated.**
   - The baseline fails all three region limits, as the mismatch controls do.
   - Every candidate value (the three region values and the still maximum) is
     below the baseline's.

The comparison controls in `test_analyze.py` behave as follows:
- **The diagnosed mismatch in both roles:** exact; guards pass; agreement fails;
  the defect is reproduced; it is not better; the verdict fails.
- **A synthetic candidate whose off grille shadow is taken from on:** passes,
  with mean_gap 0 and a still maximum of 24.
- **A blurred reference:** stays at a mean gap of 0.197 and fails.
- **One altered on-frame hash:** voids exactness, and nothing else fails.
- **Injected changes outside, at the border and in a bar pixel:** each fails only
  its own guard.

### 6.5 Observations (recorded, never deciding the verdict)

- **Inherited temporal quality.** These are 0072's viewport rules, unchanged, for
  both runs. They measure the resolve, which neither source changes:
  - activity at 33 steps or more, at most 318, and none saturated;
  - frames 9–16 edge mean at most 5 and edge max at most 50;
  - interior changed at most 100;
  - same-phase interior 0;
  - pan off-edge 9–16 and over 16 both 0.
- **Pan off-edge bins** for both runs, split inside and outside the region.
- **Off before/after:**
  - the band split;
  - the region metrics;
  - connected clusters above 8 steps, with size, bounding box, maximum and peak.
- **On before/after:** the maximum step, expected to be 0.
- **Bar colours:** whether their colours differ in the off references.
- **Report values:** the still and pan `*_vs_off` values for both runs.

### 6.6 Image pairs (`analyze.py pairs`, CPU viewing aids)

These are not screenshots and do not count against the 4-PNG budget.

| Name | Box (output px) | Scale | Panels |
|---|---|---|---|
| `grille-shadow` | [428, 259, 711, 322) | 4× | baseline on, baseline off, candidate on, candidate off |
| `card-off` / `card-on` | [418, 147, 559, 282) | 3× | baseline, candidate |
| `border-left-ground` / `border-right-ground` | [29, 122, 93, 170) / [866, 183, 930, 231) | 4× | all four |
| `border-bottom-left` / `border-bottom-right` | [29, 472, 93, 528) / [866, 472, 930, 528) | 4× | all four |
| `off-before-after-x4`, `on-off-baseline-x4`, `on-off-candidate-x4` | the viewport | ×4 difference | — |

Where the border crops sit:
- The sky inside the viewport is exactly the canvas colour (20, 21, 25).
- So only the ground can show a border artifact. It meets the left border at
  y = 146 and the right border at y = 207.

I built every pair from the retained 0072 PNGs into a scratch directory and
checked the layout. The self-check does the same in a temporary directory.

### 6.7 Expected outcomes (derivations, not measurements)

- **P1. Exactness passes,** including identical temporal-on frames (§2.2).
- **P2. The baseline reproduces the mismatch.** Its region values should be near
  the mismatch controls, a mean gap of about 0.2, and its still maximum near 96.
- **P3. The candidate's off grille shadow turns solid** like on. Its region values
  should be near the consistent controls, and its still maximum near 24.
- **P4. Off before/after changes stay in shadows.**
  - The largest cluster should be the grille shadow, from hatch to solid.
  - Smaller re-quantization changes should appear at shadow edges in cascades 0,
    1 and 3 (texels +0.752%, +0.389% and +0.106%).
  - Nothing should change outside the viewport or in the bars.
- **P5. Inherited results match across roles.** The still-pose inherited values
  should be identical across roles, because the on frames are the same.
  - The accepted full-frame fixed-coverage run had a same-phase interior of 17
    against a limit of 0. That inherited failure is likely again.
  - The pan off-edge rule should fail for the baseline, because of the grille
    shadow.

## 7. CPU self-checks run (all exit 0)

All were run from the worktree root with `/opt/homebrew/bin/python3` 3.14.8 and
`-I -B`. No bytecode was written.

| Command | Result | Receipt (sha256) |
|---|---|---|
| `tools/test_lifecycle.py --receipt …/lifecycle.json` | 14 tests pass: real `true`, `false`, `sleep` and `sh` children; signals, timeouts, masks, ceilings, busy lock, stop latch | `eb287f95…ce38c094` |
| `tools/test_source.py --receipt …/source.json` | 7 tests pass: a two-commit synthetic repository; both roles exported exactly; a branch name, a wrong or swapped commit, a missing role, a wrong freeze hash, a missing input and a baseline freeze for the candidate are each refused or stopped | `6454aa22…c539bcac` |
| `tools/test_analyze.py --receipt …/analyze.json` | 22 tests pass: 16 integrity controls (52 of 60 and 4 of 4 admitted; a retry, a fifth PNG and the 61st submission refused; 0072's real grants refused) and the 6 comparison controls in §6.4 | `726d0457…79c18b6d` |
| `tools/regions.py > …/regions.json` | Polygons, sampling table and the `cascade_texels.txt` lines | `056e48ce…25a435a0` |
| `tools/selfcheck.py --write …/selfcheck.json` | `all_pass: true`: plan 19, sources 7, fixture 12, regions 4, calibration 4 and pairs 1 checks | `ecd067fc…f1234252` |
| `rustfmt --edition 2024 --check` on the fixture (stdin, rustfmt 1.10.0, toolchain 1.99.0) | exit 0 | — |

**Refusals.** In `selfcheck.py`, all 19 guarded commands refused with
"`build-ready.json` is absent" or "`review-ready.json` is absent":
- `lease` ×2;
- `build` × 4 steps × 2 roles;
- `gpu` ×5;
- `analyze` ×4.

The evidence directory was still absent afterwards, `git status` was unchanged,
and the global `native-gpu.lock` was untouched (same inode, size and mtime).

## 8. Acceptance (after execution; all required)

1. `analyze.py check` passes.
2. `analyze.py compare` passes every exactness, guard, agreement and defect check
   in §6.4.
3. **My personal inspection** covers all 4 PNGs at full size and every pair. It
   must confirm:
   - visual temporal on/off agreement of the grille shadow in the candidate, and
     the mismatch in the baseline;
   - no new border artifact at any ground crossing;
   - intact MASK geometry;
   - that every off before/after cluster outside the grille shadow is a
     shadow-edge re-quantization.

   A numeric pass with a visual objection is a failure, and I will report it.

**What does not count as acceptance:**
- **CPU matrix equality** alone. The candidate's spike CPU tests do not count as
  visual acceptance.
- **The inherited temporal-quality rules.** They are reported separately and
  decide nothing here.
- **Any blur or on-frame difference.** That voids the comparison rather than
  counting as a fix.

**How "Preserved MASK shadow detail" is judged:**
- the temporal-on output is byte-identical;
- the off grille shadow agrees with it;
- texel growth is at most +0.752%;
- the bars are identical.

The hatch-to-solid change in the off reference is the intended alias change
(§2.3). It is reported, not hidden, and it is root's or the director's look-dev
call.

## 9. Limitations

- **Not compiled.** The one-line fixture change and both builds are checked by
  source reading and rustfmt only. Root's builds and strict Clippy decide.
- **The derivation needs the run.** That the temporal-on frames stay identical is
  derived from source, not measured. If GPU execution is not reproducible across
  the two processes, exactness fails and the run cannot isolate the fix. The
  60-submission cap leaves no room for a repeat.
- **One case, one pose.**
  - Only `viewport` runs. The stationary, motion, excluded, transitions, skinned,
    timing and probe cases do not.
  - The candidate also changes the full-frame temporal-off fit in cascade 3
    (+0.105% at 960×540). This slice does not image that.
  - UI, sprite and text exclusion are not exercised. The diff does not touch
    them, and identical on frames are the evidence for the temporal path.
  - Only the pan f24 pair is kept as PNGs, so the still pose is judged from
    report numbers only.
- **The thresholds are calibrated proposals, not plan gates.**
  - The consistent controls are full-frame poses, and two of them use C-reg, a
    resolve in neither source.
  - There is no earlier fixed-coverage viewport run.
- **The 0072 C-reg viewport pair is not a baseline.** It is used only for region
  placement and calibration.
- **Desktop only.** This is one local Apple-silicon Metal desktop, not a phone,
  device or performance measurement.
- **No per-frame shadow readback hook.** None was needed: on-frame identity, the
  region metrics and inspection cover it. None is requested.
- **Scratch files.** These are copies and crops of retained 0072 PNGs, made for
  my own viewing, in `/tmp/incant0076-prep-view`. They are not committed and are
  not screenshots.

## 10. Open questions for root

1. **The fixture fix.** Do you accept the one-line `halos: None` deviation from a
   byte-identical file (§2.1)? Without it the fixture cannot compile on main224.
   The viewport case is untouched.
2. **The grille shadow's look.** The candidate should show the solid alias in both
   modes. Should this slice accept consistency as its goal, with alpha-tested
   shadow fidelity tracked separately?
3. **A later full-frame run.** Should a later stationary run image the full-frame
   temporal-off change in cascade 3?
4. **If on-frame identity fails.** Would you authorize a same-source determinism
   repeat (26 submissions, beyond this cap), or decide on another basis? I
   propose no tolerance in advance.

## Post-commit note: root's source freezes

**What appeared.** After my self-check receipt (`selfcheck.json`, written 09:54:17
local time), root created
`~/.codex/incant-evidence/0076-shadow-guard-review-001` at 09:56:16. It holds only:
- `baseline-source-freeze.json` (sha256 `ab1d341ecd098348b1446f4c6e971001ce5bfb7e7e6bfef7fd76340fe90ee673`);
- `candidate-source-freeze.json` (sha256 `6b80b9e8ca7689773d10ebe5f809dff8673372c25e8eecdd5b86abf1d9791d52`).

There is no grant file, and none of my tools ran then.

**My read-only preflight.** It was a scratch script, against the Git objects. It
applied `build.py`'s exact-coverage rule:

| Freeze | Commit (`source_commit`) | Rows | Build inputs | Missing | Mismatched | `.cargo/config.toml` |
|---|---|---|---|---|---|---|
| Baseline | `0aa02559` | 3,141 | 3,141 | 0 | 0 | listed |
| Candidate | `f636a2d5` | 3,142 | 3,142 | 0 | 0 | listed |

- Both freezes are exact.
- They differ in exactly the 7 Rust delta paths.

**Refusals.** Because the namespace now exists, a fresh `selfcheck.py` skips its
refusal step by design, as it did at 10:41. I will not probe the guarded tools
against a namespace root is filling: if a grant landed mid-probe, `lease.py
acquire` would take a real lease. The committed receipt, made before the
namespace existed, remains the refusal evidence. Its tool, plan and fixture
hashes equal the committed files'; the 10:41 run confirmed this.
