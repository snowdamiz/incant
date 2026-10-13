# 0076 result: shadow receiver guard visual comparison — prepared

**Status: PREPARED. Execution is pending root's grants.**

- **What exists:** the bounded baseline/candidate comparison is ready,
  self-checked on the CPU and committed. It compares main224 (`0aa02559`) with
  the candidate (`f636a2d5`) on 0072's exact viewport case.
- **Not done:** no build, Cargo command, GPU process, render, screenshot, screen
  control or target lease.
- **No namespace created by me:** `~/.codex/incant-evidence/0076-shadow-guard-review-001`
  did not exist when the self-check ran, and every guarded command refused
  without its grant. Root has since created it to hold the two source freezes;
  see the post-commit note at the end.
- **No claims:** this is not a phase-gate, performance or device claim.

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max, as reported by this session's environment.
  - Root's ACP transport log for this session is
    `~/.codex/incant-evidence/claude-0076-shadow-guard-prep-acp.log`.
  - I did not open it.
- **Packet:** `brief.md` sha256 `c83c30c8…7ebe0994`.
  - Root committed it in `3f3630ba`, and it is unchanged and uncommitted by me.
  - There was no priority revision or new director feedback beyond the brief;
    I rechecked before committing.

The full account is in [`preparation.md`](preparation.md).

## What root needs to know first

1. **The fixture needs one line to compile against main224.**
   - **Why.** main224 added the required `UiWidget::halos` field (`c6a7804a`), and
     the 0072 fixture's one UI button lacks it.
   - **The fix.** I inserted `halos: None,` at line 1207. It sits in
     `ui_input`, which only the `excluded` case uses. A Button must carry `None`.
   - **Scope.** The viewport case and everything it calls are byte-identical, and
     the fixture is rustfmt-clean.
   - **Not yet compiled.** It is unverified until root builds it. See open
     question 1.
2. **Only the temporal-off references should change.**
   - Temporal-on cascades are bit-identical in both sources. Both use
     `guard(plan.rect)`, at `retained.rs:434-435` and in the candidate's
     `fit_for_viewport`.
   - The comparison therefore requires all 24 on-frames to be byte-identical
     across roles. Any blur or source-identity difference voids it, rather than
     counting as a fix.
3. **Expected look.**
   - The candidate's temporal-off grille shadow should turn **solid**, matching
     temporal-on. 0072's on image already shows the guarded fit.
   - The baseline's faint hatch should go away.
   - Both are aliases: cascade-1 bars are 0.36–0.39 texels wide, and the
     vertical bars repeat every 1.45 texels, below Nyquist. The fix makes the
     shadow consistent, not correct.

## Changed paths (my commits, `Built-by: claude`)

**`1d60f936`** imports 0072's committed `visual/` files verbatim; blob ids are
equal at `48a688c5`, tree `b11a7d3a`.

**This commit** adapts them narrowly. The final `visual/` tree is
`8d9de3edbbe6f2c2f66ccff7e3ef97493e4dd65d`.
- **`handoffs/0076-shadow-guard-review/visual/plan.json`:**
  - two sources;
  - 0072's viewport case spec, verbatim;
  - budgets of 52 planned against a ceiling of 60, and 4 PNGs;
  - the comparison, calibration and pairs.
- **`…/visual/review/temporal_visual_review.rs`:** the one line above.
- **`…/visual/tools/`:**
  - `common.py`, `build.py`, `gpu.py`, `analyze.py` and `lease.py`, adapted for
    two roles with no numeric controls;
  - `shadowfit.py`, renamed from `sharpness.py`, holding the new comparison;
  - `regions.py` and `pairs.py`, which are new;
  - `selfcheck.py`, `test_analyze.py`, `test_lifecycle.py` and `test_source.py`,
    adapted;
  - `test_library_linkage.py`, removed.
- **`…/visual/preparation/`:** the receipts `selfcheck.json`, `lifecycle.json`,
  `source.json`, `analyze.json` and `regions.json`.
- **`handoffs/0076-shadow-guard-review/preparation.md`** and this file.

**Unchanged:** production code, `brief.md`, all 0072 files and evidence, and
every `~/.codex` file.

## Commands run and results

All CPU only, run from the worktree root with `/opt/homebrew/bin/python3`
3.14.8 `-I -B`.

| Command | Result |
|---|---|
| `visual/tools/test_lifecycle.py --receipt visual/preparation/lifecycle.json` | exit 0, 14 tests pass |
| `visual/tools/test_source.py --receipt visual/preparation/source.json` | exit 0, 7 tests pass |
| `visual/tools/test_analyze.py --receipt visual/preparation/analyze.json` | exit 0, 22 tests pass (16 integrity, 6 comparison) |
| `visual/tools/regions.py > visual/preparation/regions.json` | exit 0; reproduces 0072's `cascade_texels.txt` card-shadow lines exactly |
| `visual/tools/selfcheck.py --write visual/preparation/selfcheck.json` | exit 0, `all_pass: true` |
| `rustfmt --edition 2024 --check < visual/review/temporal_visual_review.rs` (toolchain 1.99.0) | exit 0 |
| `git diff --cached --check` | clean |

**What the self-check covers:**
- **Refusals.** It ran all 19 guarded commands with the grants absent: lease,
  build, gpu and analyze steps for both roles.
  - Each refused for the absent grant.
  - No evidence directory was created.
  - `git status` was unchanged.
  - The global `native-gpu.lock` was untouched.
- **Fixture counts.** It reads the counts from the fixture: 26 world
  submissions, 0 UI submissions and 2 PNGs per role.
- **Fixture delta.** It proves the fixture is 0072's plus exactly one line.
- **Source delta.** It verifies the delta against Git: the candidate's parent is
  main224; there are 7 Rust build-input files and no shader or cooker files; 4
  production files reach the review binary.
- **Calibration.** It recomputes the threshold calibration from retained 0072
  evidence.

**Thresholds.** They come from one rule: the geometric mean of the worst
consistent-fit control and the best mismatch control.
- In the grille-shadow region: mean gap ≤ 0.020, blur residual ≤ 0.026 and
  low-pass max ≤ 0.086.
- At the still pose: interior max ≤ 66.
- Consistent controls stay at or below 0.0022, 0.0042 and 0.023. The diagnosed
  defect measures 0.197, 0.171 and 0.321, and stays there through a
  fixed-coverage-like blur.

**Not run, and not available in this stage:** any compile, Clippy, GPU test,
render or capture. Those wait for root's grants (`preparation.md` §4–§5).

## Screenshots

None. This stage forbids rendering and screenshots.
- **Prior evidence I looked at.** To place the grille-shadow region and to check
  the planned pair layout, I viewed retained 0072 PNGs and CPU crops of them.
- **Where those crops are.** They are scratch viewing aids in
  `/tmp/incant0076-prep-view`. They are not committed and are not screenshots.
- **After execution.** The run will publish 4 PNGs as
  `handoffs/0076-shadow-guard-review/screenshots/<role>-viewport-{on,off}-f24.png`.

## Rationale

The design answers the brief's acceptance criteria directly:
- **On/off agreement must be measured blind to blur.** The mean gap and blur
  residual are unaffected by a blur, as the synthetic controls show, so the
  softer main224 resolve cannot pass or fail the check by itself.
- **"No outside or border change" and "preserved MASK geometry" are exact
  guards.** The surround must be byte-identical and all canvas. The 3 px band
  must not exceed the inner maximum. The grille-bar mask must be identical.
- **The baseline must reproduce the defect.** Otherwise the candidate's
  agreement demonstrates nothing.
- **The inherited temporal-quality rules are kept apart.** They are reported for
  both roles and never decide the verdict.

## Limitations

- **Not compiled here.** Temporal-on identity is derived from source, not
  measured.
- **No room for a repeat.** The 60 cap cannot fund one.
- **Narrow coverage.** Only the viewport case and its pan f24 PNG pair are run.
  The full-frame temporal-off change in cascade 3 (+0.105%) and UI, sprite and
  text exclusion are not imaged.
- **Thresholds are calibrated proposals.** They come from full-frame and C-reg
  controls, not plan gates.
- **Desktop Metal only.**
- **No per-frame shadow readback hook is needed, and none is requested.**

## Open questions for root

1. Do you accept the one-line `halos: None` fixture deviation? The file cannot
   compile on main224 without it.
2. Should this slice accept consistency of the grille shadow as its goal, with
   alpha-tested shadow fidelity tracked separately?
3. Should a later stationary run image the full-frame temporal-off cascade-3
   change?
4. If the temporal-on frames are not byte-identical, will you authorize a
   same-source determinism repeat beyond this cap, or decide another way?

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
