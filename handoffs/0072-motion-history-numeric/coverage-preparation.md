# 0072 fixed-coverage temporal review — preparation

**Status: PREPARED.** This is preparation only.
- No GPU process, Cargo, target, build, test-binary execution, frame or capture
  ran.
- The new evidence namespace
  `~/.codex/incant-evidence/0072-temporal-coverage-001` does not exist.
- I did not access the warm core-presentation target.
- All older results, screenshots, evidence, grants and ledgers are unchanged.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max, as this session reports them.

**Director feedback, applied:** the priority revision "prepare focused
fixed-coverage GPU review".
- Root implemented the neighbourhood-depth, closest-motion and
  premultiplied-coverage repair at `e23776df`. This prepares the focused review
  of exactly that source.
- **Small delta only:**
  - counts, the numeric test, source identities, namespace, stages and
    publication directories;
  - docstrings that name them;
  - a few focused controls.
- **Unchanged:** the fixture is byte-exact; the seven cases, their pixel
  measurements and policy are unchanged; the command framework is not
  redesigned; there was no repository resurvey.
- **Also kept:** no old broad suite, no retries, and failed attempts stay charged.
- **Process:** no PR, push, persistent memory or routine questions. Root's
  uncommitted `brief.md` is left unstaged.

## Source read (exact commit, read-only)

**Source commit.** `e23776df275fa1de81b2027110d9c7b6916999ac`, based on
`077f7c39c0f4122aae36ea3a88537be006722822`. It is a descendant of the
corrected-source `535003d7`.

**Design doc.** I read `docs/spikes/temporal-coverage.md` with
`git show e23776df:…`.

**Root's source and CPU review.**
`/Users/andreyyurlov/.codex/incant-reviews/temporal-coverage-source-cpu-final-20261013.json`,
SHA-256 `678e67b083603de9ce5fef5103be4761933b07b1d84f64efe2bf6b71625818cc`, as
the brief states.
- Status: `accepted_scoped_no_remaining_findings`.
- Default CPU: 129 passed, 41 ignored. Cooked: 144 passed, 42 ignored. Cooker: 3
  passed.
- The reviewer executed no binary.

**What the repaired resolve does** (`temporal/resolve.wgsl` at `e23776df`):
- **Depth range.** A 3×3 range of expected previous view depths, with clear as
  the far plane, widened by the existing tolerances.
- **Motion.** The closest valid expected-depth motion, with centre priority on
  ties. An invalid opaque centre motion still refuses history.
- **Admission.** Positive in-viewport bilinear taps are admitted against that
  range, then renormalized.
- **Coverage and clamp.** Canonical-zero clear, and premultiplied RGBA,
  including fractional alpha, in the 3×3 clamp.
- **Unchanged.** The reaction, frame layout, bindings and `copy.wgsl`.

**Build-input changes since `535003d7`:**
- MetalFX internals;
- temporal internals and tests;
- the shader cooker.

No public item changed and `Cargo.lock` is identical, so the byte-identical
fixture has no new API surface. Compilation is still root's build step.

## Focused identity diff

| Item | Corrected review (`53fd64b6`) | This plan |
|---|---|---|
| Source commit | `535003d7…` | **`e23776df275fa1de81b2027110d9c7b6916999ac`** |
| Freeze | `temporal-metalfx-union-001/…` `bd8295b0…` (2,448 inputs + 1) | **`/Users/andreyyurlov/.codex/incant-evidence/temporal-coverage-001/committed-build-source.json`** `1e9628677f6631476140d7efe75242042721083c4d21966fdfc81755bb193fe0` (2,452 inputs + 1 probe) |
| Numeric test | `temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion` (14) | **`temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection`** (150); the old test is not run |
| Numeric binary (root pin) | `incant_render-90f6082e9b891519` `865b8863…` | **`…/temporal-coverage-001/frozen-binaries/source004/default/render-tests`** `d900fe08fb2023b42aca223868e6ac367cc5c85b7d8fdaeaa55e2e49c3d19537` |
| Stages | `temporal-corrected-build` / `-visual` | **`temporal-coverage-build`** / **`temporal-coverage-visual`** |
| Evidence namespace | `0072-temporal-corrected-001` | **`0072-temporal-coverage-001`** |
| Publication | `temporal-corrected-screenshots/`, `-evidence/` | **`temporal-coverage-screenshots/`**, **`temporal-coverage-evidence/`** |
| Planned / ceiling | 93 / 128 | **229 / 256** (reserve 27) |
| PNGs, size | 14 of 16, 960×540 | unchanged |
| Timeouts per GPU child | listing 60 s, numeric 120 s, case 120 s | unchanged (all at most 120 s) |
| Fixture | `9841c6b1…` | **byte-identical** (`9841c6b1384a9f02db140d58d588f40a262e63ea061127518781e1c799952dde`) |
| Seven cases, checks, PNGs, config | unchanged from the original `9009f088` | unchanged |

**Freeze check (read-only, git object store only).** I evaluated build.py's
exact-coverage rule against `e23776df`, with no export, checkout or build:
- 2,452 build inputs and 2,453 freeze rows;
- the one non-input row is `tools/probes/cooked-temporal-compile.py`, which is
  present and matches;
- no inputs are missing, no rows are absent from the commit, and nothing
  mismatches.

So `exact` is true. `build.py checkout` remains the authoritative check under
the grant.

## Expected test listing

These are derived from the source mounts at `e23776df` and root's counts. They
are expectations, not results.

**`gpu.py list lib`** runs on root's pinned default binary with
`--list --ignored`. It should list **41** ignored tests (root's default count),
including exactly one
`temporal::system::coverage_device::actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection: test`.

The name comes from these mounts:
- `lib.rs:35` `pub mod temporal`;
- `temporal/mod.rs:12` `pub(crate) mod system`;
- `temporal/system.rs:319-321` `#[cfg(test)] #[path = "coverage_device.rs"] mod coverage_device`;
- `coverage_device.rs:5-7` `#[test] #[ignore = "requires native GPU; Claude owns numerical execution"] fn actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection()`.

The name matches the brief exactly, so there is no difference to report.

**`gpu.py numeric`** runs that exact name with
`--ignored --exact --nocapture --test-threads=1`. The expected output is
`running 1 test` and
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out`
(129 + 41 − 1).
- The contract needs only the single exact test, run once and passed.
- It is never broadened, and never run unless the fresh listing names it.

**`gpu.py list review`** should list the fixture's 7 cases (probe, stationary,
motion, excluded, transitions, skinned, timing), exactly as before.

## Actual counts

| Child | Submissions | PNGs | Timeout |
|---|---|---|---|
| `list lib`, `list review` | 0 | 0 | 60 s each |
| `numeric` | **150**: `for ortho in [false, true]` × `CASES` (5) × `tick in 1..16` (15), with one `queue.submit` per tick (`coverage_device.rs:261`) on 12×8 targets | 0 | 120 s |
| review `probe` | 0 | 0 | 120 s |
| review `stationary` | 17 | 3 | 120 s |
| review `motion` | 16 | 4 | 120 s |
| review `excluded` | 9 + 9 UI | 3 | 120 s |
| review `transitions` | 13 | 2 | 120 s |
| review `skinned` | 7 | 2 | 120 s |
| review `timing` | 8 | 0 | 120 s |
| **Total** | **229** (150 + 70 + 9) of the 256 ceiling | **14** of 16 | — |

Each charged child is debited, fsynced, under the global non-waiting
`native-gpu.lock`, before launch. Debits are never refunded, and failed or
interrupted attempts stay charged. There is one attempt per child, with no
retry. Every child is killed and reaped, and `gpu.py release` latches the
release.

## CPU validation (no build, no GPU, no grant)

| Command | Result |
|---|---|
| `python3 -I -B visual/tools/selfcheck.py --write visual/preparation/coverage-selfcheck.json` | **`all_pass: true`** (receipt `ab5fba65…`) |
| `python3 -I -B visual/tools/test_analyze.py --receipt visual/preparation/coverage-analyze.json` | **13/13** (receipt `f00cdf3f…`) |
| Mutation check (in memory, not committed): the old stage names restored | The old-grant control **fails**, as it must |

**`selfcheck` covered:**
- **Counts:** 150 numeric + 79 = 229 within 256; 14 of 16 PNGs; reserve 27; GPU
  children at most 120 s.
- **New identities:** the exact numeric name, the new namespace, stages and
  publication directories.
- **Fixture byte-identical** to the corrected review's tree `53fd64b6`, and
  differing from the original only at line 1418.
- **Cases unchanged:** the case definitions and review config are unchanged,
  and the fixture matches the plan (tests, limits, PNGs, size, output variable,
  config and per-case checks).
- **Refusals:** all 15 guarded commands refuse with both grants absent. No
  evidence directory was created, `git status` is unchanged, and the global
  lock is untouched.

**`test_analyze` covered** the 9 existing analyzer controls, re-run at the new
counts:
- complete evidence passes and publishes; this run also asserts 229 debited;
- empty evidence fails;
- a missing case fails;
- **a numeric run that never happened fails;**
- a missing report fails;
- a false check fails;
- a failed child fails;
- a stop latch fails;
- a stale summary blocks publication.

It also covered 4 new controls:
- **Admission at the new counts.** The plan's 229 is admitted. 257 submissions
  and 17 PNGs are refused, exactly 256 and 16 are admitted, and anything after
  that is refused. That is 3 refusals, all before any charge.
- **Old grant refusal.** The real corrected-source grants were read only from
  the old namespace (`review-ready.json` `4fa5101f…`, `build-ready.json`
  `1a1b226d…`) and both are refused. The tools never consult the old
  namespace.
- **The old numeric identity fails.** A ledger naming the old test with a
  14-submission debit fails on both test and debit.
- **A missing library listing fails.**

**Not re-run, per the brief** ("no need broad unchanged helper suite"):
- `test_lifecycle.py` (14 controls) and `test_source.py` (4 controls). Their
  mechanisms are unchanged, and their earlier receipts remain.
- `build.py`, `lease.py`, `test_lifecycle.py` and `test_source.py` are
  byte-unchanged, including root's reviewed lint-exit change. Their docstrings
  and lease note still say "corrected"; that is cosmetic only.

## Identities for root's grants

| Item | Value |
|---|---|
| **Visual tree** (pin in both grants) | **`b1327f3d9abbaf10f4ba8701dc4ef81231b6d60e`** |
| Plan | `bd480ba1c843aa707b0f9ad641a3ff6b4d5f23d37d0356fda0186bbf1e978c08` (was `df1d01c2…`) |
| Fixture | `9841c6b1384a9f02db140d58d588f40a262e63ea061127518781e1c799952dde` (unchanged) |
| Changed tools | `common` `d3a437a4…` (was `9f7ab0ad…`); `gpu` `230ef701…` (was `2d97eb72…`); `analyze` `91d6b90d…` (was `570abe91…`); `selfcheck` `39adb1d0…` (was `41350bb7…`); `test_analyze` `7157308d…` (was `c0bc9a87…`) |
| Unchanged tools | `build` `319430e2…`, `lease` `13acaa49…`, `test_lifecycle` `8f33f44d…`, `test_source` `483abef0…` |

**`build-ready.json`:**
- `packet`;
- **`"stage": "temporal-coverage-build"`**;
- `holder`;
- `source_commit` `e23776df…`;
- `source_freeze {path, sha256}` (`1e962867…`);
- `visual_tree`;
- `target`. The warm core-presentation target is allowed by name.

**`review-ready.json`,** written after the build:
- `packet`;
- **`"stage": "temporal-coverage-visual"`**;
- `holder`;
- `visual_tree`;
- `build_record_sha256` of `build/review.json`;
- at least one `prior_*release*_path`/`_sha256` pin;
- **`numeric_binary {path, sha256}`** = root's `render-tests` `d900fe08…`, so
  no libtests build is needed.

**Order** (unchanged):
1. **Root CPU:**
   1. `lease.py acquire`
   2. `build.py checkout`
   3. `build.py toolchain`
   4. `build.py review`
   5. `build.py lint`
   6. `lease.py release`
2. **GPU:**
   1. `gpu.py list lib`
   2. `gpu.py numeric`
   3. `gpu.py list review`
   4. the seven `gpu.py review` cases, in order
   5. `gpu.py release`
3. **CPU:**
   1. `analyze.py check`
   2. personal inspection against both earlier runs
   3. `analyze.py publish`

## Open quality questions for the GPU review

The numeric contract and appearance acceptance stay separate.
- **The numeric test** compares the actual resolve and copy with an f64 oracle
  on **controlled post-raster occupancy**:
  - an alternating 1-px depth or clear edge;
  - alternating one-texel MASK occupancy;
  - a 1 px/tick mover;
  - malformed clear.
  It uses its own config: tolerances 0.001 and **reaction strength 0**.
- **It cannot establish** real model or MASK rasterization under jitter, or
  rendered quality. Only the seven fixture cases, run under the review policy
  (history 0.9, depth 0.05/0.02, reaction 4.0/0.5), and my inspection can.

1. **Silhouette convergence.** Do the sphere, cubes, wall, posts and the
   horizon against clear now converge to stable coverage blends? The
   corrected-run baselines are:
   - 12,061 pixels with a peak change of 33 steps or more (frames 9–16), 3,620
     of them saturated;
   - edge mean 10.6–16.3 and edge max 99–121 per frame.

   My earlier targets were at most 3,000, 100, 5 and 50. They are reviewer
   judgement and assumed about a 10% step.
2. **The reaction makes larger steps likely.** The review policy keeps reaction
   on, so an edge whose history and current differ by about 0.5 linear lowers
   the weight to about 0.84. Its step can then be about 16% of contrast, not
   the 10% bound the numeric test asserts with reaction off. On the dark side of
   the horizon that could approach 40 sRGB steps. I will judge against the
   fixed policy, report the measured distribution, and not retune.
3. **The real MASK grille.** The numeric grille is idealised occupancy. Does
   the alpha-tested one-texel grille converge to a stable blend, or to moiré or
   residual noise? It was saturated noise in both earlier runs.
4. **Trailing edges and smear.** Closest-depth motion and range admission are a
   one-pixel policy.
   - Check for a 1-px trail behind the moving cube, around the skinned
     triangle, and on silhouettes against clear during the pan, where clear
     pixels adopt foreground motion.
   - Background pixels next to a moving edge reproject with foreground motion,
     which could smear texture there.
   - Baselines (off-edge, at least 3 px from an edge): object 823 / 30 / 0;
     pan 240 / 0 / 0; skinned 0. Edge-local differences need inspection at
     magnification.
5. **Larger review tolerances.** They widen admission (relative 0.05, absolute
   0.02, against the numeric test's 0.001). Look for coverage blending across
   nearly coplanar surfaces, for example post bases on the ground and wall
   contact.
6. **Unchanged paths should stay unchanged.** Excluded overlays and UI are
   drawn after history (UI byte-identical; edge activity of 33 steps or more on
   505 pixels). Transitions should reset as before, with current-only frames
   after a cut. The moving-shadow lag (30 pixels, up to 10 steps) is unchanged
   by design. Registration should stay within 0.1 px, since `copy.wgsl` is
   unchanged.
7. **Timing.** The resolve now loads nine motion samples. The Models ratio
   (×2.04 last run) is an observation only; there is no performance claim.
8. **Upscaling is out of scope.** The portable upscaling resolve (0073) still
   uses the earlier per-surface admission. That belongs to 0073, not this
   packet.

## Files (this commit)

**New:**
- `handoffs/0072-motion-history-numeric/coverage-preparation.md`
- `visual/preparation/coverage-selfcheck.json`
- `visual/preparation/coverage-analyze.json`

**Changed:**
- `visual/plan.json`
- `visual/tools/common.py`, `gpu.py`, `analyze.py`, `selfcheck.py` and
  `test_analyze.py`

**Left unchanged:**
- the fixture, `build.py`, `lease.py`, `test_lifecycle.py` and `test_source.py`;
- `result.md` and every earlier result document;
- the screenshots and evidence directories;
- `brief.md`, which is root's and left uncommitted.

## Limitations

- **Expectations, not results.** Nothing was compiled or executed. The listing
  count, filtered-out count, freeze coverage and fixture compilation are
  expectations from source, root's counts and a read-only check. Root's build
  and the guarded GPU run decide them.
- **No numeric or appearance claim yet.** The numeric pass and the appearance
  verdict are pending. Neither will substitute for the other.
- **Scope.** This is default portable fixed-mode quality on one local Apple M5
  Pro / Metal desktop. There is no upscaling, MetalFX, phone, performance or
  phase-gate claim, and this preparation approves nothing.
