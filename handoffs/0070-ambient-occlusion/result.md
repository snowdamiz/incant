# 0070 result — combined SSAO + bloom: analyzer acceptance repair

## Status

**The analyzer repair and the helper compile fix are ready for root's source
review.** Root's supervised CPU build of the helper comes next.

This session was preparation only. I ran no Cargo or rustc, took no target or
GPU lease, and rendered or captured nothing. **I make no efficacy, appearance
or visual-success claim.** No screenshots exist.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**, as configured and verified by root before this prompt. I cannot
  inspect the effort setting from inside the session.
- **Earlier results:** the revision 2 result is in history at `1cb718f2`, and
  v1 is still in `preparation-result-v1.md`.
- **Blockers:** none for source review.

## Root's revisions, acknowledged

**Revision 3 decisions.** I made no runner change for these.

- The precise-retry latch stays strict.
- Reconcile takes only the packet lock.
- CPU builds take only the packet lock.
- The 45-minute ceiling stays wall-clock.
- The four listings count within the 16 children.

**Supervised build outcome.** In root's evidence the engine test binaries
compiled, and the helper failed with exactly three `E0689` errors at
`composed.rs:108-110`: ambiguous `{float}` for `sqrt`, `atan2` and `asin`.

- I listed `~/.codex/incant-evidence/0070-effects-build` and read only
  `logs/build-helper.stderr` and the compiler messages in
  `logs/build-helper.stdout`.
- I did not modify that evidence.

**Root's runner commits are preserved untouched.**

- `6b06c1c8` gives each build child its own scratch `TMPDIR`.
- `bfecb8d6` resolves pruned-lock dependency edges by package identity.
- `runner.py` still matches the hash in the review receipt (`a5adf3df…`).

**Other constraints honoured:**

- `lifecycle.py` is unchanged.
- The generated helper `tools/effects/Cargo.lock` (sha256 `9dcff7a5…`) stays
  untracked and uncommitted; the root `Cargo.lock` is untouched.

**Review receipt.** I read
`~/.codex/incant-reviews/effects-0070-fixtures-source-20261013.json`; its
sha256 matches `64774dd5…`. Both of its findings are fixed below. Of the
twelve files it hashes, only `analyze.py` and `composed.rs` changed.

## Changes

### 1. Compile fix (`tools/effects/src/composed.rs:106`)

`let (eye, at): ([f64; 3], [f64; 3]) = ([1.0, -0.55, 2.6], [1.25, -1.0, 0.4]);`

Only the type annotation was added; the camera values are unchanged.
`rustfmt` accepts it. It removes all three reported errors. rustc reports every
type error in a crate before borrow checking, so typeck should now be clean.
Later phases (borrow checking, lints, Clippy `-D warnings`) only run in root's
supervised build.

### 2. Analyzer finding 1: inference, coverage, occlusion and furnace (`tools/analyze.py`)

**`infer_k` now rejects instead of comparing.** It returns
`(interval, None)` or `(None, reason)`. The reasons are:

| Reason | Meaning |
|---|---|
| `low_signal` | The diffuse signal is at most 0.02 linear |
| `nonfinite` | A non-finite bound, or a strength that is not finite and positive |
| `inverted` | A channel's low end is above its high end |
| `empty` | The RGB channel intervals do not intersect |

**SSAO agreement passes only when all of the following hold:**

- every precondition is met;
- the reference defines, and the readbacks compare, **every viewport-interior
  pixel**;
- no `nonfinite`, `inverted` or `empty` rejection occurred;
- there is no disagreement and nothing is out of range;
- the gate's declared occlusion is real. Occlusion gates must compare at least
  1,000 reference-occluded pixels, and all of them. Absence controls
  (`g_ao_skin_t0`, `g_ao_motion_far`) must have none.

The old counts are kept: `pixels`, `agree`, `disagree`, `uninferable`,
`out_of_range`, `reference_ambiguous`, `occluded_reference` and
`first_disagreements`. These are added: `compared`, the `rejected` counts by
reason, `occluded_compared`, `coverage_required`, `first_rejections`,
`problems` and `preconditions`.

**Minimum coverage is 100% of the interior** (277 × 157 = 43,489 pixels). It
is fixed from the frozen geometry before any readback exists:

- **The receiver fills the viewport.** It is 16 × 12 at z = −4, and at that
  depth the visible half-extents are 3.82 × 2.19 in perspective and
  6.99 × 4.00 in orthographic.
- **The reference defines every interior pixel** of every declared scene. It
  finds no background, silhouette or cut-edge ambiguity at any interior texel
  centre. I computed this from the reference alone.
- **The diffuse signal is far above the floor.** The white material's diffuse
  term is about 0.67 linear, roughly 30× the 0.02 inference floor.

A faithful render therefore leaves no pixel uninferable.

**The occlusion floor of 1,000 pixels** only guards against a vacuous
reference. The reference itself, again from geometry alone, marks this many
pixels occluded:

| Scene | Occluded pixels |
|---|---|
| `white_slab`, perspective | 8,558 |
| `white_slab`, orthographic | 7,075 |
| `edge_slab` | 11,025 |
| `mask_cutout` | 10,853 |
| far controls | 0 |

**Furnace precondition.** Every SSAO reference gate reuses bases. The specular
basis is always a separately rendered black scene, the edge gate borrows
`p_off_black`, and all nine geometry gates share `g_off_white` and
`g_off_black`. So uniformity of both bases is now an enforced precondition of
each gate, not just an observation, and each gate's result reports the
uniformity of its bases.

### 3. Analyzer finding 2: fail closed

**Declared cases are required.** The five declared cases come from
`plan.json` (`case-ssao_core`, `case-ssao_materials`, `case-ssao_geometry`,
`case-lifecycle_drs`, `case-bloom_numeric`), and all 27 named gates always
appear in the report.

**Missing or bad evidence fails the dependent gates** with the reason
retained, as a `missing` or `error` field and in the report's
`cases[...].status`. This covers:

- a missing case directory or `index.json`;
- an invalid index or a hash or length mismatch, which marks the case
  `invalid` instead of crashing;
- a missing frame.

The other gates still run, and their results are kept.

**The overall verdict** passes only when every declared case is loaded and
every declared gate passes. The report lists `missing_or_invalid_cases` and
`failed_gates`.

**The CLI always writes `analysis.json`** (create-new, never replaced) and
prints a summary. Its exit status is:

| Exit | Meaning |
|---|---|
| 0 | The analysis passed |
| 1 | It failed; the report survives |
| 2 | Usage error, or the report already exists |

**Numerical invariants stay separate from appearance.** The report states
`appearance.judged_here = false`. The PNGs await my later appearance review,
and their in-run invariants remain the helper's own verdict. Retry output
directories are listed as not analysed.

**Moved code is unchanged.** All 14 numerical helpers and the four constants
are byte-identical to v1: the sRGB and tone curves, the interior, nearest
expansion, projection, halo, centroid and identity helpers. The bloom, DRS,
mixed-pane and identity gate bodies moved into named functions with the same
arithmetic and thresholds.

## Evidence (CPU only)

Commands ran from the worktree root. `python3` means
`/opt/homebrew/bin/python3 -I -B`, and `tools/` means
`handoffs/0070-ambient-occlusion/tools/`.

All of these ran at `c04c51a4`. The next commit, `6e5f2465`, changes only two
docstring lines in `analyze.py`, and I re-ran the analyzer selftest there
(exit 0).

| Command | Result |
|---|---|
| `python3 tools/analyze.py --selftest` | Exit 0 in about 33 s. Details below |
| `python3 tools/selftest.py` | **26 of 26 runner controls pass**, exit 0, with root's runner commits in place |
| `python3 tools/mutations.py` | 4 of 4 re-introduced runner defects caught, exit 0 |
| `python3 tools/ssao_reference.py --selftest` | Unchanged: flat [0, 0], near slab [5, 5], reversed normal [0, 0] in both projections |
| `python3 tools/listing.py` | 94 ignored GPU tests |
| `python3 tools/make_plan.py --check` | `plan.json matches the source-derived plan` |
| `rustfmt --edition 2024 --check tools/effects/src/lib.rs tools/effects/src/main.rs` | Exit 0, via `/opt/homebrew/opt/rustup/bin/rustfmt`; no Cargo |
| AST parse of the ten helper scripts | All parse |

**What `analyze.py --selftest` shows:**

- **Positive controls:**
  - Synthetic white-slab readbacks agree on all 43,489 interior pixels: 8,558
    occluded pixels compared in perspective, 7,075 in orthographic.
  - A +2-ray perturbation is caught, with 457 disagreements per projection.
  - Complete synthetic evidence for all five cases passes all 27 gates, and
    `main` exits 0. That evidence includes:
    - DRS references with a non-uniform pattern, so the expansion rule is
      really exercised;
    - halo rings for the bloom centroid and coverage gates;
    - a 0.5-α pane;
    - tone-curve field values.
- **Negative controls, each failing for the stated reason:**
  - empty inference (off equals spec): 0 compared, 43,489 `low_signal`;
  - conflicting RGB: `empty` rejections;
  - `nonfinite` (NaN and zero strength);
  - `inverted` (off bytes 69..92 over a black pixel);
  - below the specular floor: out of range, never agreement;
  - inadequate coverage: a 600-pixel low-signal block with zero disagreements
    still fails on coverage;
  - a vacuous occlusion gate and an absence-control mismatch;
  - a failed furnace precondition (one off pixel −1 code);
  - a missing frame: only the edge gate fails, with "p_ao_edge absent";
  - missing cases: every dependent gate fails, the bloom gates still pass,
    and all 27 gates are reported;
  - a tampered file: the case is `invalid`;
  - the CLI on failing evidence exits 1 with `analysis.json` kept
    (`pass: false`), and a second run exits 2 with the report byte-identical.

**Before and after on the same inputs.** These were throwaway runs of the
committed v1 analyzer (the HEAD version at the time), in a scratch directory
I then deleted:

| Input | v1 analyzer | Repaired analyzer |
|---|---|---|
| Every pixel uninferable | `pass: true` with `agree: 0` and `uninferable: 43489` | Fails: "coverage 0 of 43489", "0 of 8558 reference-occluded pixels compared" |
| Four of five cases missing | `pass: true` on 3 gates; CLI exit 0, writing `pass: true` | Fails; CLI exit 1 with the report kept |

**Hygiene:**

- No processes survive (checked with `ps`).
- All scratch directories were removed.
- Every write stayed inside the worktree.
- `git status` shows only root's untracked generated helper `Cargo.lock`.

## Identities

| Item | Value |
|---|---|
| HEAD / tree | `6e5f2465d11d0b3b95b2aa0bd5e41e1019f5a0fa` / `162d932c2d1614f7ec3525dc4fe1542ba5c5aee4` (this result commit adds only `result.md`; re-read HEAD before granting) |
| Engine inputs | Byte-identical to `b633f92d` |
| Helper digest | `66d18150f62e98ad3a89bf6319663b314324f2d043385ade2867541740174709` (19 files; excludes the generated lock) |
| `tools/plan.json` | `27ea7b4425100c8ec6786c02cc8da21b41c670ec2ff3824ed6c497d9ccf859cf` (unchanged) |
| `tools/analyze.py` | `16ec434de72b27163958445b4a694d819b92ffffd7f420714811864c6ec67445` |
| `tools/effects/src/composed.rs` | `fccb7f0935650039d206db36cd69e6edea415b4591c868c6b76537594d24f139` |
| `tools/runner.py` (root) | `a5adf3df71793090c03fa94e929e42eaea286ec5890119e189c19413d77c0189` (unchanged since review) |
| `tools/lifecycle.py` | `574db13f25dc27939b99becb7fdb5272b6b5734bcda694487d1e42b127e764a0` (unchanged) |

## Limitations and remaining checks

- **The helper has still never compiled.** The fix removes the only three
  errors rustc reported. Borrow checking, lints and Clippy `-D warnings` first
  run in root's supervised CPU build.
- **The analyzer controls are synthetic.** They prove the acceptance logic,
  not any GPU result.
- **Two deliberate fail-closed choices** await their first real evidence:
  - **Full coverage:** if a faithful render leaves any interior pixel
    uninferable, the SSAO gates fail. That would mean the fixture assumptions
    are wrong, and the counts are retained for review.
  - **Strict furnace uniformity:** if the off or spec bases are not exactly
    uniform, for example because of view-dependent specular or dithering,
    every SSAO reference gate fails at the precondition. The distinct values
    are reported.
- **Retry outputs are not analysed.** The analyzer reads the original case
  labels; after a root-authorized retry, which outputs count needs root's
  decision.
- **Still pending:** the GPU run, my appearance judgment, and everything v1
  and v2 list as pending:
  - cooked SSAO and bloom;
  - native default exposure;
  - profiling subpass entries;
  - TAA and upscaling;
  - device budgets;
  - complete Phase 1 rendering.
- **Remaining checks for root:**
  1. source review of `c04c51a4` and `6e5f2465`;
  2. a fresh supervised CPU build (helper compile and Clippy);
  3. review of the exact binaries;
  4. then the GPU grant and instruction.

## Changed paths and commits

- **`c04c51a4`:** `handoffs/0070-ambient-occlusion/tools/analyze.py`
  (rewritten acceptance logic and controls) and
  `handoffs/0070-ambient-occlusion/tools/effects/src/composed.rs` (one type
  annotation).
- **`6e5f2465`:** `tools/analyze.py`, a more precise coverage-basis
  docstring, nothing else.
- **This result commit:** `handoffs/0070-ambient-occlusion/result.md`.

All three commits carry `Built-by: claude`. Nothing was pushed or merged.
