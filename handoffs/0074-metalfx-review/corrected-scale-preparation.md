# 0074 corrected-scale preparation: second frozen execution (2026-10-13)

**Status: preparation complete; nothing has been granted, built or run.** This session
had no GPU, capture, Cargo, target or lease access, and it started no background
processes. The review is ready for root to freeze 077f7c39, build in the warm
core-presentation target and grant GPU execution separately. The first execution
(2dc2db04) and all of its evidence are unchanged.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Line
  2 of root's ACP log `claude-0074-scale-preparation-acp.log` reads "Claude session
  verified: Opus 5.5; thinking: max" (the live log's SHA-256 was `ff97d604…` when I
  read it). No subagents were used.
- **Brief:** root's uncommitted revision, SHA-256 `9da02d85…`. I left it uncommitted.

## Root input verified

| Input | Result |
|---|---|
| Root acceptance `0074-corrected-execution-root-20261013.json` | SHA-256 `8708082e…` matches. Root keeps the silhouette-quality rejection and the native-upscale limitation. |
| Scale query `metalfx-scale-query-001/receipt.json` | SHA-256 `e5610fe0…` matches. Apple M5 Pro, macOS 27.0.1 (26A434), supported, `raw_min_scale` 1, `raw_max_scale` 3, `gpu_submissions` 0. This was an API query, not a render. |
| Corrected source `077f7c39` | Commit tree is `408c4bb2…`. It descends from the reviewed `535003d7` through `f57ac7d3` and `0c1864df`. Its only build-input changes are `metalfx/native.rs`, `metalfx/numerics.rs` and `metalfx/plan.rs`. The jitter table is unchanged. The commit has 2448 build inputs, which root's freeze must cover exactly. |

At 077f7c39, `plan::upscale_range` keeps the raw values as output/input factors and
clamps them to [1, 4]. `Plan::new` admits a frame when output width / source width
falls inside that range. The descriptor's min/max content scale comes from the same
range. On [1, 3], the factors 1, 4/3 (12/16) and 2 (8/16) are all admitted. At
535003d7 the same values were read as content fractions in [0.25, 1]. That collapsed
the range to [1, 1], which explains the first execution's `Portable(Some(Scale))`
selections at 12/16 and 8/16.

## Changes (focused diff: `corrected-scale-helper.diff`, SHA-256 `cb6702fd…`)

- **`common.py`:**
  - `EVIDENCE` is now `/Users/andreyyurlov/.codex/incant-evidence/0074-metalfx-visual-002`.
  - `PUBLISHED` is now `visual-screenshots-002`. `analyze.py publish` refuses an existing
    directory, so the first publication stays intact.
  - 535003d7 and its freeze `bd8295b0…` are now refused by name as superseded, as
    4bc810fe/b68c62d5 already were.
  - The grant fields, freeze rule, feature allowlist, target rule and `STAGE`
    (`metalfx-visual`) are unchanged.
- **Fixture `metalfx_visual_review.rs`:**
  - `Expect::NativeOrScale` is removed.
  - The transitions d12 and d8 steps and the viewport framed-d8 step now bind
    `Expect::Native`, which accepts only `Some(Selection::MetalFx)`.
  - The report step labels and doc comments are updated to match.
  - Nothing else changed: tests, limits, PNG names, check names, configs, viewports,
    preferences, observations, cut and owner reuse are the same. The file is still
    rustfmt-identical.
- **`plan.json`:**
  - The same three steps now expect `native`.
  - The denied lists match `common.py`.
  - The `expected_source`, `arms.native` and two purpose texts are updated.
  - Unchanged: 108 frames + 11 UI = **119 submissions**, **18 PNGs** at 960x540,
    reserves 9/2, budgets 128/20, the 35 checks, all steps and preferences, the
    review and current-only configs, and the order.
- **`selfcheck.py`:**
  - The native model now uses factors on the measured range, read from the pinned
    receipt.
  - The announced-source checks now cover 077f7c39.
  - A new strict-native section was added (below).
  - Four grant-policy variants were added: 077f7c39 accepted, the core-presentation
    target accepted, 535003d7 refused, and its freeze refused.
- **`test_analyze.py`:** one test was added, `test_scale_fallback_selection_refuses`.
- **`test_lifecycle.py`:** the freeze-names-another-commit test now uses 077f7c39.
  535003d7 is refused earlier, as a superseded source.

## Why a portable fallback at 12/16 or 8/16 can no longer pass

At runtime, a `Portable(Some(Scale))` frame at a native step fails the case:

1. `steps_as_planned` returns false, so the required `selections_as_planned` check is
   false.
2. `report.finish` returns false and the child exits nonzero.
3. `gpu.py` latches stop, so no later case runs.
4. `analyze.py check` refuses because of the false check and the failed child, and
   `publish` refuses too.

CPU evidence (selfcheck `strict_native`, all true):

- **Plan:** exactly three native steps run at 12/16 or 8/16, and all of them expect
  `native`. No plan expectation other than `native` or `fractional` remains.
- **Fixture:** `Expect` has exactly the variants `Native` and `Fractional`.
  `impl Expect` matches the expected text exactly. Neither `NativeOrScale` nor
  `Fallback::Scale` appears anywhere. The `(Expect, group)` pairs in transitions and
  viewport match the plan. Both cases compute `selections_as_planned` from those
  steps.
- **Negative control:** I ran the same fixture check on the first execution's granted
  fixture (`b2c3e14e:review/metalfx_visual_review.rs`). Each of the four detectors
  refuses it on its own.
- **Replay of recorded selections:** I replayed the first execution's committed
  reports (`transitions.json` `7bcc1679…`, `viewport.json` `3cf84253…`, pinned to the
  committed manifest):
  - The 535003d7 unit model reproduces all 9 recorded native-step selections: MetalFx
    at 16/16, Scale at 12/16 and 8/16, FractionalViewport at 10/16 and in the
    fractional viewport.
  - The corrected model predicts MetalFx at the three reduced steps.
  - The recorded Scale fallbacks, which passed `selections_as_planned` at 2dc2db04,
    are refused by the current expectations. The other six recorded steps are still
    accepted.
  - Recorded frame counts and internal sizes match the plan model.
- **Analyzer:** `test_scale_fallback_selection_refuses` makes `selections_as_planned`
  false in transitions and viewport. `check` then exits nonzero and keeps a failing
  summary, and `publish` refuses.

I also ran some in-memory mutation controls once. They are not committed and wrote
no files:

- A range of [1, 1] or a missing receipt misclassifies the plan.
- Reverting 12/16 to `native_or_scale` fails three strict checks and the
  classification.
- Widening the `Native` arm to accept Scale fails two detectors.
- Rebinding framed-d8 to `Fractional` fails the binding check.
- Replacing the checked expression with `true` fails the steps check.
- On [1, 3], factor 4 and downscale are both refused.

The first of these runs crashed on the unknown expectation instead of reporting it
(a `KeyError`). Unknown expectations now accept nothing and report false.

## Commands and results

| Command (from the worktree root, Python 3.14.8) | Result |
|---|---|
| `python3 -I -B handoffs/0074-metalfx-review/visual/tools/selfcheck.py --write handoffs/0074-metalfx-review/visual/preparation/selfcheck-002.json` | exit 0, `all_pass` true; receipt SHA-256 `5f2b6dff…`. It covers the plan, the footprints on [1, 3], the fixture, 63 reused items, 19 grant-policy cases, rustfmt 1.10.0 identity, the announced source, strict-native, and 12 absent-grant refusals. The 002 directory stays absent and the GPU lock is untouched. |
| Inline unittest runner over 7 named tests (text in `result.md`), receipt `visual/preparation/focused-tests-002.json` | 7/7 pass, 0.06 s; receipt SHA-256 `91066636…`. The tests: analyzer complete-evidence publish, false-check refusal and the new scale-fallback refusal; lifecycle superseded-source refusal (now 3 sources), ill-formed sources, explicit source gate, and freeze naming another commit. |
| `git apply --check -R corrected-scale-helper.diff` | applies cleanly against this tree |

Not rerun: the full lifecycle suite (24 tests) and the full analyzer suite (now 11),
per the brief's "without broad reruns". No Cargo, Clippy, build, GPU or capture was
run. Root builds and lints. The 2448-input freeze coverage is enforced by `build.py
checkout` at grant time, because root has not announced the freeze.

## Grant for the second execution

Root writes `review-ready.json` in the new namespace, using the same protocol as the
first execution:

- `packet` `0074-metalfx-review`, `stage` `metalfx-visual`.
- `source_commit` `077f7c39c0f4122aae36ea3a88537be006722822`, `source_tree`
  `408c4bb2948983cfde980fa40c31f443b5d78a0c`.
- `source_freeze` {absolute path, SHA-256} of root's `{commit: 077f7c39…, files:
  [{path, sha256, bytes}]}` freeze. It must cover the 2448 build inputs exactly;
  probes outside the roots must exist in the commit and match.
- `features`: one allowlisted set. The first execution used `[cooked-temporal,
  native-metalfx]`.
- `visual_tree`: this commit's tree of `handoffs/0074-metalfx-review/visual`.
- `target` `/Users/andreyyurlov/.codex/incant-core-presentation-target` (accepted by
  the policy check), and a root-assigned `holder`.
- `prior_<name>_path`/`_sha256` pins, including at least one release receipt whose
  live-process fields are empty.

The execution order is unchanged:

1. `lease.py acquire`
2. `build.py checkout`, then `toolchain`, then `review` (root supervises)
3. `gpu.py list review`
4. The five cases, once each, in order
5. `gpu.py release`
6. Optional `build.py lint`
7. `lease.py release`
8. `analyze.py check`
9. Personal inspection
10. `analyze.py publish` (writes `visual-screenshots-002`)

## What the next review must assess

- **Native upscale appearance at 75% and 50%:**
  - transitions d12 (720x405 to 960x540) and d8 (480x270);
  - the viewport framed-d8 step (384x216 to 768x432, inside the offset viewport;
    outside pixels must stay byte-identical to off).
- **The unchanged full-resolution controls:** stationary, motion and excluded at
  16/16; transitions view A, cut and resume; the framed viewport.
- **The unchanged FractionalViewport fallbacks:** 10/16 and the fractional viewport.
- **Separate verdicts:** numerical/contract and appearance get separate verdicts, as
  in the first execution.

## Limitations and open questions

1. **Retained 75%/50% frames are reset frames.** 077f7c39 resets native history on
   every content-size change. `owner.rs` says it will "Conservatively reset native
   history on content-size changes", and the reset is driven by `History::reset`
   comparing dimensions. So `transition-d12-f7.png` and `transition-d8-f10.png`
   (each step's first frame, unchanged) will show native output immediately after a
   reset. Each scale then gets only two more frames, and the viewport case keeps no
   8/16 PNG; its 8/16 evidence is the numeric `native_8_16_last_vs_off`. The 75%/50%
   verdict will therefore describe early post-reset native output, not converged
   quality. The brief forbids changing retained frames or step lengths, so I left
   them unchanged.
   **Open question for root:** should a later revision retain d12-f9 and d8-f12, or
   lengthen these steps within the 9-submission/2-PNG reserve?
2. **Silhouette policy is still an open engine defect.** The shared current-coverage
   boundary policy (silhouette shimmer in both native and portable arms) is not
   addressed by this unit correction. I do not call it fixed.
3. **Selections at 12/16 and 8/16 are predictions until the GPU run.** They come from
   models over the measured range. Vendor output at factors 4/3 and 2 has not been
   observed on this device. The model uses f64 where the engine uses f32; the planned
   factors are far from the [1, 3] bounds.
4. **Any portable fallback at a native step fails the case.** That includes transient
   `Busy` fallbacks, by design.
5. **No aspect step.** No step exercises an aspect fallback, the same as in the first
   execution. The engine's `Fallback::Aspect` contract and the selfcheck's aspect
   class are unchanged.
6. **No device or phase claims.** This is no phone, performance or phase-gate claim.
   It is local desktop CPU preparation only.

## Integrity

- These are unchanged: the earlier `result.md` content (a short section was appended),
  `visual-execution-result.md`, `visual-screenshots/`, `visual-evidence/`, the first
  receipts in `visual/preparation/`, and `~/.codex/incant-evidence/0074-metalfx-visual`.
- The `0074-metalfx-visual-002` directory does not exist. Outside this worktree I
  only read files: root's acceptance receipt, the scale-query receipt, the ACP log
  and the first execution's grant (for its target). I wrote nothing outside this
  worktree.
- I made no source changes in root's worktree and no memory writes, publication,
  push, PR or merge.
