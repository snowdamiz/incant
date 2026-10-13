# 0074 native late-frame review: preparation (2026-10-13)

**Status: preparation only. Nothing was granted, built, run or captured.** This
session had no GPU, Cargo, target or lease access. The review is ready for root to
build from the same source and grant one GPU interval for the new `metalfx-late`
stage. All earlier fixtures, plans, reports, grants and evidence are unchanged.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Line
  2 of root's ACP log `claude-0074-native-late-preparation-acp.log` reads "Claude
  session verified: Opus 5.5; thinking: max" (log SHA-256 `5f29ccad…` when read). No
  subagents were used.
- **Root input:** `0074-corrected-scale-execution-root-20261013.json`, SHA-256
  `80bde0f1…`, status accepted for numeric, source and evidence scope. Both earlier
  verdicts stand:
  - the 75%/50% judgment covers initial quality only;
  - the shared silhouette quality is still rejected.

  The faint 50% reset striation needs this late-frame follow-up.

## What the review captures

There are two new cases, each run as one foreground child within 120 s. Together
they use 126 world submissions (cap 160), no UI submissions, and 15 PNGs (cap 16)
at 960×540. That leaves a reserve of 34 submissions and 1 PNG.

**`late_stationary`** (75 frames, 8 PNGs): static view B. This is transitions'
second viewpoint, where the 002 striation appeared, rendered here without the cut.

| Step | Arm | Scale | Frames | Retained |
|---|---|---|---|---|
| 1 | off | 16/16 | 1 | none; its RGBA must equal 002 `off-b` (`4f3f379e…`) |
| 2 | off | 12/16 | 1 | none (the unreconstructed 12/16 control) |
| 3 | off | 8/16 | 1 | none; its RGBA must equal 002 `off-b-d8` (`ea8f6b94…`) |
| 4 | native, one preferring owner | 16/16 | 16 | f16 (the scale-1 converged control) |
| 5 | same owner | 12/16 | 16 | f01 (reset), f08, f16 |
| 6 | same owner | 8/16 | 16 | f01 (reset), f08, f16 |
| 7 | portable, fresh owner | 8/16 | 16 | f16 (same-source control) |
| 8 | current-only, fresh owner | 8/16 | 8 | none (current-coverage band at 8/16) |

**`late_motion`** (51 frames, 7 PNGs): view A with modest combined motion. Each
frame the mover advances 0.0875 world units (a quarter of the motion case's step,
about 5 px instead of 21) toward the thin wall. The camera pans at a quarter of
`camera_world`'s speed (0.03 units and −0.003 rad per frame).

| Step | Arm | Scale | Frames | Retained |
|---|---|---|---|---|
| 1 | native, fresh owner | 12/16 | 16 | f01 (reset), f08, f16 |
| 2 | native, fresh owner | 8/16 | 16 | f01 (reset), f08, f16 |
| 3 | portable, fresh owner | 8/16 | 16 | none |
| 4 | off at path steps 1, 8 and 16 | 16/16 | 3 | step 16 |

**Why these frames:**
- **f01** is the reset frame. In `late_stationary` it is a content-size reset inside
  one owner, the exact 002 `d12-7` / `d8-10` condition (the 002 records confirm the
  sequence and phase continue across DRS). In `late_motion` it is the owner's first
  frame.
- **f08 and f16** are one and two full jitter cycles. In 002 at 16/16, native frames
  one cycle apart at the same phase differed by only 0.80 mean edge codes, so f16
  is a fair late frame.
- **Controls:**
  - the same owner's 16/16 f16 is the quality target;
  - portable and current-only at 8/16 are the same-source controls;
  - the off frames at 12/16 and 8/16 show what native adds over plain DRS;
  - the two off frames are bound to 002, which proves the scene is the striation
    condition.
- **Kept comparable:** the same `Stage`, materials, `REVIEW` and `CURRENT_ONLY`
  configs, thresholds, edge masks and 960×540 output as the five 002 cases.

**Contracts (14, all required by the analyzer):**
- **`late_stationary`:**
  - `selections_as_planned`: every native step is `Expect::Native`, so a portable
    fallback fails;
  - `controls_have_no_metalfx_owner`;
  - `one_native_owner_advances_through_scales`: one owner, sequence and phase +1
    per frame, history only absent on frame 1, planned internal size per segment;
  - `internal_sizes_as_planned`;
  - `portable_run_uninterrupted` and `current_only_run_uninterrupted`;
  - `references_have_no_upscale_state`;
  - `frames_match_plan`.
- **`late_motion`:**
  - `selections_as_planned`;
  - `controls_have_no_metalfx_owner`;
  - `native_runs_uninterrupted`;
  - `portable_run_uninterrupted`;
  - `references_have_no_upscale_state`;
  - `frames_match_plan`.

**Observations** (for the reviewer; they never decide pass or fail):
- **`late_stationary`:**
  - f01, f08 and f16 against off at every scale;
  - f16 against the unreconstructed 12/16 and 8/16 references;
  - 12/16 and 8/16 f16 against the 16/16 f16;
  - native against portable at 8/16;
  - frame-to-frame change per segment;
  - Lucas-Kanade registration of frames 9–16 per arm;
  - the current-only band at 8/16;
  - inferred resets;
  - the striation measure on `STRIATION_RECT` (x 422–461, y 330–379): column means
    of mean sRGB and their mean absolute second difference, exactly as measured in
    `corrected-scale-execution.md`, for every off frame, native milestone and
    control.
- **`late_motion`:** each milestone against the off frame of the same path step;
  native against portable at 8/16; 8/16 against 12/16; per-frame registration
  against the same-step reference; inferred resets.

## Smallest helper delta

`native-late-helper.diff` (SHA-256 `b5bc888d…`) holds the whole change against
`ce7c9cbd`:

| Path | Change |
|---|---|
| `visual/review/metalfx_visual_review.rs` | **append only**, +470/−0. The first 83,695 bytes are still blob `31257a55` (SHA-256 `c8b1219a…`, the 002 fixture). Two tests and their helpers were added: `striation`, `segment`, `segmented`, `late_mover`, `late_camera`, `late_run`, `milestones`, `striation_milestones`. Still rustfmt-identical. |
| `visual/late-plan.json` | new plan for the late stage. `plan.json` stays the frozen 002 plan (blob `1f5f8ec2`). |
| `visual/tools/common.py` | 4 constants: `STAGE` `metalfx-late`, `PLAN_PATH` `late-plan.json`, `EVIDENCE` `0074-metalfx-late-001`, `PUBLISHED` `visual-screenshots-late-001`; plus the docstring. |
| `visual/tools/gpu.py` | the listing takes `list_args` from the plan: `--list --ignored late_`. The frozen five cases are compiled in but never listed, and `review` already refuses unplanned cases. |
| `visual/tools/selfcheck.py` | plan caps 160/16/120 s; frozen-then-late test order; listing filter; per-plan sizes and preferences; reuse from the prior plan; root's freeze now verified read-only; the 002 strict-native replay kept on `plan.json`; new `late` section. |
| `visual/tools/test_analyze.py` | existing controls pinned to the 002 plan, plus `test_late_plan_complete_evidence_checks_and_publishes`. |
| `visual/tools/test_lifecycle.py` | plus `test_listing_passes_the_plan_filter`. |

`analyze.py`, `build.py` and `lease.py` are byte-unchanged; their logic already
reads the plan. One stale detail remains: `analyze.py`'s docstring still says "the
five plan cases". I left it unchanged to keep the delta minimal.

## Source, fixture and build requirements for root

- **Source:** the same as the 002 run, commit `077f7c39c0f4122aae36ea3a88537be006722822`
  (tree `408c4bb2…`). Root's freeze
  `metalfx-scale-correction-001/committed-build-source.json` (`62db43cb…`) applies
  unchanged. The selfcheck verified it read-only: 2,448 build inputs plus the probe,
  and all 2,449 blob SHA-256s match.
- **Features:** `cooked-temporal` and `native-metalfx`, as in 002.
- **Fixture:** this commit's `visual/review/metalfx_visual_review.rs` (SHA-256
  `4ec5d81c…`), copied by `build.py checkout`. `SAMPLES` is unchanged, so the
  jitter check passes.
- **Build:**
  - a fresh frozen binary is required, because the fixture grew;
  - `build.py checkout`, `toolchain`, `review` and strict `lint`, in the warm
    core-presentation target, as for 002;
  - the build must compile the five frozen tests unchanged; nothing runs them.
- **Grant:** `review-ready.json` in `/Users/andreyyurlov/.codex/incant-evidence/0074-metalfx-late-001/`
  with:
  - `stage` `metalfx-late`;
  - the source pins above;
  - `features`;
  - `visual_tree` set to this commit's tree of `handoffs/0074-metalfx-review/visual`;
  - `target` `/Users/andreyyurlov/.codex/incant-core-presentation-target`;
  - `holder`;
  - `prior_<name>_path`/`_sha256` pins, for example the 002 `target-lease.json`
    (`52218a7b…`) and `gpu-released.json` (`81aa15a6…`).
- **Run (one interval):**
  1. `gpu.py list review`, which lists only `late_stationary` and `late_motion`.
  2. `gpu.py review late_stationary`.
  3. `gpu.py review late_motion`.
  4. `gpu.py release`.
  5. `lease.py release`.
  6. `analyze.py check`.
  7. Personal inspection of all 15 PNGs.
  8. `analyze.py publish` to `visual-screenshots-late-001`.
- **Old evidence:** the five 002 cases are not rerun. Their evidence stays bound to
  source `077f7c39` and binary `059d28ac`. The late fixture keeps them byte-identical
  as a prefix, and the late stage cannot list or run them.

## CPU verification in this session

- **`visual/preparation/selfcheck-late.json`:** `all_pass` true (SHA-256 `f04a1307…`).
  It covers:
  - plan caps and accounting;
  - the footprint model on the measured [1, 3] range (every native step native);
  - fixture agreement;
  - the listing filter;
  - the 63 0072 reused items;
  - 19 grant-policy cases;
  - rustfmt identity;
  - exact freeze coverage;
  - the 002 strict-native replay;
  - the `late` section: 002 plan blob, exact fixture prefix, exactly the late tests
    appended, late steps bound to MetalFx, 002 reference hashes, striation
    rectangle;
  - 12 absent-grant refusals.

  The new namespace stayed absent and the global GPU lock was not touched.
- **`visual/preparation/focused-tests-late.json`:** 17/17 in 0.17 s (SHA-256
  `50007faf…`). That is all 12 analyzer tests (their shared setup changed) plus five
  lifecycle tests: the new listing test, three grant-policy tests and the freeze
  test.
- **In-memory mutation controls** (run once, uncommitted). Each was caught by its
  intended check:
  - one byte changed in the frozen prefix;
  - an extra late test appended;
  - a late step bound to `Fractional`;
  - the listing filter removed;
  - a wrong 002 reference hash;
  - the striation rectangle moved;
  - `plan.json` edited.

## Limitations and open questions

1. **The extended fixture has not been compiled.** I had no Cargo access. The code
   copies the fixture's existing patterns (`named`, `object_run`, `continuous`).
   The report body is split so that no single `json!` expansion is deeper than the
   existing ones. Root's build and strict Clippy will be the first compile.
2. **Motion speeds are approximate.** About 5 px per frame for the mover is scaled
   from the 002 motion case (about 21 px per 0.35 units at that depth). The run does
   not measure speed.
3. **"Late" means 16 frames.** That is two jitter cycles, not an unbounded
   convergence. MetalFX's internal convergence horizon is not exposed.
4. **The striation rectangle exists only in view B** (`late_stationary`). The motion
   case uses view A and does not measure the striation.
5. **The coverage policy is unchanged in 077f7c39.** Late frames will still show
   silhouettes stepped at the input pitch, from the current frame. This review
   separates vendor convergence (interiors, plus the current-only band at 8/16) from
   that policy. It cannot validate a repair; see `native-coverage-repair.md`.
6. **Only one PNG remains in reserve.** Any additional retained frame needs a root
   amendment.
