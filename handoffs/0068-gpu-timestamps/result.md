# 0068 result (revision 2) — corrected pass timestamps on current-main200

## Status

**Executed under root's fresh r2 grant on one Apple M5 Pro (Metal), default features
only. The GPU and the Cargo target are released.**

- **Numeric verdict: pass. The GPU timestamp measurement is accepted.**
  - **The brief's numeric test** ran exactly once, selected by its full compiled name:
    1 test run, 1 passed, 0 ignored. It returned **real, nonzero counters** for both
    instrumented frames: 312,041 ns and 438,958 ns overall. Absent groups were absent.
  - **The review fixture** collected 8 further samples. All are accepted: content-derived
    labels, finite nonzero endpoints, ordered spans, conversion matching the period, and
    a positive aggregate.
  - **Bookkeeping on the real GPU:** backpressure, the instrumentation-skipped frame,
    drain order and slot reuse all match the pre-registered values.
  - **Superseded result:** the original 0068 all-zero result is rejected and stays
    preserved as history.
- **Regression suites: 92 of 92 pass, one attempt each.**

  | Test binary | Passed |
  |---|---|
  | Renderer library (numeric test skipped, absent from output) | 33 |
  | `model_gpu` | 50 |
  | `timestamp_backend_probe` | 1 |
  | GPU-host game | 5 |
  | GPU-host UI | 2 |
  | Numeric test (run separately) | 1 |

- **Appearance verdict: unchanged.** Enabling GPU timing changed no rendered pixel.
  - **Frames:** 15 new frames, and all 5 retained timing-off/on pairs are byte-identical.
  - **Controls:** these include the instrumentation-skipped and reused-slot frames.
    The second-device control is also byte-identical.
  - **Personal inspection:** I inspected all 10 new PNGs; every frame is complete, with
    no artifacts.
  - **Same as the previous source:** the three reused fixtures are byte-identical to the
    original 0068 renders.
- **Four-cascade shadows, scoped as root directed.** The four-pass `Shadows` group
  encodes:
  - interior cascades carry no descriptor, and wgpu accepts them;
  - pixels are unchanged with timing on;
  - both endpoints are written, nonzero and ordered (415,792 ns);
  - the unshadowed control renders differently (10,646 pixels) and reports no `Shadows`
    group.
- **`owner_drop`: passes** as a no-panic/no-hang smoke check only. It does not establish
  callback-after-drop ordering.
- **Finding for Astra (not an acceptance failure).** The reported frame interval runs
  from the first-encoded group's beginning to the last-encoded group's end. In 3 of the
  4 `lit` samples, the later-encoded `InitializeScene` pass began 12–15 µs before the
  first-encoded `AssignLights` compute pass, so it fell outside the frame interval.
  Details are below.
- **No production failure occurred**, so there was no stop. **No fixture correction was
  needed:** the reviewed fixture compiled cleanly on its first build and ran unchanged.
- **Not claimed:**
  - cooked-shader acceptance: default features only, as root directed;
  - other GPU backends;
  - reference-phone performance;
  - the complete profiler;
  - any phase gate.

  Sample durations are raw correctness data, not benchmarks.

## Brief, grant and director feedback

- **Brief.** SHA-256 `0fb286c62335f965f1ed1ef91c182f08a9fdab02317ad284867dc58b52348940`,
  committed by root in `e8938530`. Revision 3 was acknowledged and applied:
  - I followed the `prep-result.md` sequence exactly.
  - No per-pass observer was added; root deemed the source/CPU descriptor checks plus
    scoped GPU evidence sufficient.
  - No cooked variant was run. Execution is default-only and **is not cooked
    acceptance**.
  - The 24/48 caps include failures, and all 92 regressions got one attempt.
  - Stop and release applies on production failure. None occurred: every GPU child
    passed, and the release still followed immediately.
  - No fixture correction was needed.
  - Every historical 0068 result is preserved.
  - This file was written only after pixel inspection and the release receipts.
  - No unrelated work, and no memory edits.
- **Grant.** `/Users/andreyyurlov/.codex/incant-evidence/0068-r2/review-ready.json`, SHA-256
  `50caea4b417232665a1fc75926ad7e4d9d897cf2f08e5544d7abcd8d3d28ee29`.
  - It names packet `0068-gpu-timestamps-r2`, source `f33495b2…`, the core-presentation
    target, caps of 24/48 and `approved_extra_cases: ["owner_drop"]`.
  - Both pinned prior receipts verified:
    - `0072-temporal/gpu-released.json` (`97ff1283…`), listing no live processes;
    - `0068/target-lease.json` (`bb32a241…`).
  - Root's review receipt `/Users/andreyyurlov/.codex/incant-reviews/gpu-timestamps-r2-prep-final-20261013.json`
    matches its pinned hash `fe2bef3f…`. Its decision: proceed with the existing bounded
    fixture build and execution, stop on production failure, and retain all failed
    attempts.
- **Historic results preserved and verified unchanged:**
  - `previous-result.md` (original 0068 execution, `0d94e74c…`);
  - `prep-result.md` (r2 preparation);
  - `evidence/`, `screenshots/` and `preparation/`;
  - `/Users/andreyyurlov/.codex/incant-evidence/0068`, whose latch `eba4f536…`, lease
    `bb32a241…`, ledger `1ca53607…` and grant `b07d3062…` are unchanged.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max.
- **ACP verification.** Line 2 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0068-r2-execution-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. I read only that line.
- No subagent or auxiliary reviewer was used. No memory edits.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0 arm64, `Mac17,9` |
| GPU / adapter | Apple M5 Pro, `IntegratedGpu`, backend `Metal` (wgpu 29.0.4) |
| Adapter timestamp features | `TIMESTAMP_QUERY` and `TIMESTAMP_QUERY_INSIDE_ENCODERS` yes; `TIMESTAMP_QUERY_INSIDE_PASSES` no |
| Device features | `Renderer::new`: none. `Renderer::new_with_gpu_timing`: `TIMESTAMP_QUERY` only, so pass descriptors are used |
| Timestamp period | `queue.get_timestamp_period()` = **1.0** (wgpu-hal's fixed non-Intel Metal value) |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 (5f94df478 2026-08-27) |
| Python | 3.14.8, `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, exclusive. Leased 06:08:49.88Z, released 06:15:09.64Z, no process on it. Left as warm cache |

## Source and frozen binaries

- **Child checkout.** A `git archive` export of `f33495b2211ac37f172cd83e2552b2041df2357f`
  into `incant-evidence/0068-r2/checkout`:
  - it matches the plan's tree pin `bb4d2322…`;
  - all 17,860 files are blob-verified, with manifest SHA-256 `346ec0b7…48b33`;
  - every one of root's **2,879** freeze entries matches (`12f94454…67e45`);
  - there is no build-input diff from tested head `ed341fa5`.
- **Builds.** All used `--locked --offline --profile ci -j2` with **default features
  only**. Each was a bounded process group under the global `native-gpu.lock` and the
  packet lock.

  | Build | Time | Notes |
  |---|---|---|
  | `tests-render` (lib, `model_gpu`, `timestamp_backend_probe`) | 62.4 s | 348 artifacts, 317 fresh |
  | `tests-host` (game, ui) | 55.3 s | — |
  | `review` (fixture, first compile) | 1.5 s | Clean, no warnings. Linked the identical fresh `incant_render` library artifact as `tests-render` |

- **Frozen binaries.** They live in `incant-evidence/0068-r2/bin`. Every binary was
  hash-checked before and after each child, and none changed.

  | Binary | SHA-256 | `incant_render` features |
  |---|---|---|
  | `incant_render-lib-tests` | `a76e7901895947a3edd4cc5245bd8099cb9292b800d4157cdaea0c6eaf9cc97d` | `default`, `physics-deterministic` |
  | `model_gpu` | `ea8562f5c721878b7d89d1140ee1edb113e03fc06caf0b24b1748e0faef706d8` | same |
  | `timestamp_backend_probe` | `95184b69be1b50038aa597fe53b3bf1c1077512711d13533bb51b5b66ac12b55` | same |
  | `gpu_host-game` | `0aea0fb15bd979ee302990c425e74478fd257b1ec1f585c5ade930f22b94850a` | host's `physics-deterministic` library build |
  | `gpu_host-ui` | `38dbfb3de7e0da2565bae050e207066ec9554ca2027843a39c8bce65bec8db0c` | same |
  | `gpu_timestamp_review` | `7cc9405ee3eb7a82f472344b4c55616f0b905e053ebdc9e4e5c32b60bf0e6ced` | `default`, `physics-deterministic` |

- **Tools.** The reviewed tools commit is `1548670f`, with plan SHA-256 `8183d41e…` and
  helper SHA-256 `563a3fc5…`. At run time HEAD was `e8938530`, and the tools and plan
  were byte-identical to the reviewed commit.

## GPU interval, ledger and release

- **Ledger.** 21 children, serial, every process group empty afterwards
  (`gpu-ledger.jsonl` `00e4e20d…`):
  - 3 builds;
  - 6 listings;
  - 1 numeric test;
  - 5 suites;
  - 6 review cases.

  All **18 GPU attempts were debited before launch** and each has its result row, with
  no attempt left unresolved. There were no refusals during the interval, no reruns and
  no amendments.
- **Budgets debited, failures included:**
  - **15 of 24** appearance frames, all 640×360, with 10 PNGs;
  - **26 of 48** numeric submissions;
  - the existing regression frames are counted separately.
- **GPU latch.** `gpu-released.json` (`c0ed8fe5…`) at 06:14:39.64Z, with no owned
  processes and no live ledger PIDs.
- **Lint.** The optional CPU-only lint (below) ran afterwards, under the lease, and is
  recorded in `post-release-ledger.jsonl`.
- **Lease release.** `target-lease.json` (`c615cfd7…`) at 06:15:09.64Z, with no process
  on the target.
- **Post-release refusals.** `gpu.py review lit`, `gpu.py numeric` and `build.py tests`
  each refuse ("GPU and target already released by this packet") and are logged in
  `refusals.jsonl`.

## Numeric results

### The brief's numeric test (exactly one attempt, full compiled name)

- **Command.** The frozen library binary ran with these exact libtest arguments, from
  `crates/incant_render` in the child checkout:

  ```
  timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment --ignored --exact --nocapture --test-threads=1
  ```

- **Result.** `running 1 test`, then
  `test timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment ... ok`,
  then `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out; finished in 0.38s`.
  So `exactly_one_test_ran: true` and `test_passed: true`.
- **Raw samples** (verbatim from stderr; `passes` are in `GpuPass::ALL` order):

  ```
  {"sequence":1,"timestamp_period_ns":1.0,"frame":{"start_ticks":56034489422250,"end_ticks":56034489734291,"nanoseconds":312041.0},"passes":[{"start_ticks":56034489422250,"end_ticks":56034489438375,"nanoseconds":16125.0},null,null,{"start_ticks":56034489626875,"end_ticks":56034489673666,"nanoseconds":46791.0},null,null,{"start_ticks":56034489701791,"end_ticks":56034489734291,"nanoseconds":32500.0}]}
  {"sequence":2,"timestamp_period_ns":1.0,"frame":{"start_ticks":56034488505583,"end_ticks":56034488944541,"nanoseconds":438958.0},"passes":[{"start_ticks":56034488505583,"end_ticks":56034488522583,"nanoseconds":17000.0},null,null,{"start_ticks":56034488758500,"end_ticks":56034488806250,"nanoseconds":47750.0},null,null,{"start_ticks":56034488916166,"end_ticks":56034488944541,"nanoseconds":28375.0}]}
  ```

- **Analysis.** Both samples are accepted:
  - **Groups:** the present groups are exactly `InitializeScene`, `Models` and `Display`.
    `AssignLights`, `Shadows`, `Sprites` and `Text` are absent, which is correct with no
    lights, shadows, sprites or text.
  - **Values:** every endpoint is a large nonzero counter, spans are ordered, and the
    aggregates are positive.
  - **Out-of-order submission:** sequence 2's counters precede sequence 1's. That
    matches the deliberate submission order (second, unmeasured, first), and
    abandonment, backpressure and slot reuse passed inside the test.

### Regression suites (one attempt each, serial)

| Suite | Listed | Ran | Result | Time |
|---|---|---|---|---|
| library | 34 | 33 (the `--skip` numeric test is absent from output) | 33 passed, 0 failed | 1.9 s |
| `model_gpu` | 50 | 50 | 50 passed | 47.3 s |
| `timestamp_backend_probe` | 1 | 1 | 1 passed | 0.1 s |
| game | 5 | 5 | 5 passed | 0.6 s |
| ui | 2 | 2 | 2 passed | 0.6 s |

**Total: 91 + the numeric test = 92 of 92.** The source has 93 `#[ignore]` attributes;
`persistent_cache.rs` is feature- and Linux-gated, so it doesn't compile here.

**Root's backend probe** (stderr) reproduced the Metal diagnosis on this host:

| Timestamp write | Resolve | Ticks |
|---|---|---|
| Encoder-level | same command buffer | zero |
| Encoder-level | later command buffer | zero |
| Pass descriptor | same command buffer | zero |
| Pass descriptor | later command buffer | positive, `56097961066916 → 56097961107375` (40,459 ns) |

The compute output verified in every case. This explains the original 0068 all-zero
result.

### Review fixture samples (raw counters; durations are not performance)

| Case | Seq | Frame (ns) | Present groups (span ticks) | Accepted |
|---|---|---|---|---|
| `lit` | 1 | 3,779,209 | Initialize 20,042 · AssignLights 38,250 · Shadows 496,250 · Models 447,292 · Display 367,959 | yes |
| `lit` | 2 | 601,250 | Initialize 21,875 · AssignLights 41,792 · Shadows 172,625 · Models 444,000 · Display 439,667 | yes |
| `lit` | 3 | 628,708 | Initialize 22,000 · AssignLights 40,916 · Shadows 171,750 · Models 472,125 · Display 467,833 | yes |
| `lit` | 5 | 629,250 | Initialize 20,667 · AssignLights 41,208 · Shadows 171,875 · Models 473,458 · Display 468,125 | yes |
| `sprites_text` | 1 | 620,583 | Initialize 18,750 · Sprites 34,500 · Text 34,791 · Display 53,916 | yes |
| `viewport` | 1 | 3,510,500 | Initialize 20,417 · AssignLights 31,583 · Shadows 380,792 · Models 348,125 · Display 294,042 | yes |
| `cascades` (shadowed) | 1 | 1,760,292 | Initialize 20,542 · Shadows 415,792 · Models 351,333 · Display 253,542 | yes |
| `cascades` (unshadowed) | 2 | 230,708 | Initialize 22,583 · Models 155,375 · Display 146,667 | yes |

Each case passed all of its fixture checks:

- **`probe`: 8/8.** The default device requests no timestamp feature and its stats are
  disabled. The timing device has `TIMESTAMP_QUERY`, its stats start empty, `take` is
  none, and the period is 1.0.
- **`lit`: 14/14.**
  - Stats after frames 1 to 4 (pending/ready/skipped) are 0/1/0, 0/2/0, 0/3/0, then
    **0/3/1**: frame 4 rendered with instrumentation skipped.
  - The drain yields sequences 1, 2, 3 and then none. The stats after it are 0/0/1.
    Frame 5 gives 0/1/1 and reuses a slot as sequence 5. The final stats are 0/0/1.
  - Abandoned and retired stay 0 throughout.
- **`sprites_text`, `viewport`: 7/7 each.**
- **`cascades`: 12/12.**
- **`owner_drop`: 5/5.**
  - It had one pending measurement before submit.
  - Waiting on the retained device returned `QueueEmpty`, both for that submission and
    for all work including the renderer's resolve.
  - A fresh timing owner started empty, and its `take` returned none.

**Independent CPU re-check** (`analyze.py`, `measurement_accepted: true`):

- all 10 PNGs decode to the helper's RGBA hashes;
- all 5 retained pairs differ in 0 pixels;
- all 10 samples are accepted under labels re-derived from the recorded scene statistics.

### Finding for Astra: frame-interval semantics and group overlap (observations only)

- **The frame interval misses the earliest pass.** `decode` sets
  `frame = span(ticks[0], ticks[count-1])`: the beginning of the **first-encoded** group
  to the end of the **last-encoded** group. In `lit`, the schedule encodes
  `AssignLights` (a compute pass) first.

  | `lit` sample | Earliest group | Started before the frame interval by |
  |---|---|---|
  | 1 | `AssignLights` (in order; `InitializeScene` started 1.8 ms later) | — |
  | 2 | `InitializeScene` (clear pass) | 12,042 ns |
  | 3 | `InitializeScene` | 14,084 ns |
  | 5 | `InitializeScene` | 15,042 ns |

  So the reported frame interval omits up to about 15 µs of concurrently started work.
  `viewport`, `sprites_text` and `cascades` had no such case.
- **Why it matters.** `docs/spikes/gpu-timestamps.md` says the overall interval "starts
  at the first instrumented pass". On Metal the first pass in encoding order need not be
  the first to start on the GPU.
- **What it does not affect.** No acceptance rule is violated, and pixels are identical.
- **Options, for source review:** compute the frame interval as the minimum beginning to
  the maximum end over present groups, or document encoding-order semantics. I made no
  change.
- **Group overlap.** In every `lit`, `viewport` and `cascades` sample, `Display` begins
  before `Models` ends. In the numeric test's two samples it does not. In `lit` samples
  2, 3 and 5, `Models` also begins before `Shadows` ends. That is tile-based-GPU pass
  overlap. It is consistent with the documented "not a strict
  barrier", and the shadow-dependent pixels are byte-identical, so it is reported only.

## Four-cascade evidence (scope as directed by root)

- **Scene:** the `cascades` case renders the look-dev scene with only a shadowing sun.
  That gives four cascade passes: the first writes the group beginning, the last writes
  the end, and the two interior passes carry no descriptor.
- **Established:**
  - wgpu accepted the four-pass group with descriptor-free interior passes; there was no
    validation failure, and the frame rendered;
  - the timing-on frame is byte-identical to timing off;
  - both `Shadows` endpoints are written, nonzero and ordered (415,792 ns), with `Models`
    and `Display` correctly following;
  - the same scene without the sun's shadows renders differently (10,646 pixels) and
    reports **no** `Shadows` group;
  - absent groups (`AssignLights`, `Sprites`, `Text`) stayed absent in both samples.
- **Not established, accepted by root:** proof from public data that interior passes
  never overwrote an endpoint. Root's CPU `range_edges` tests cover that, and no per-pass
  observer was requested.

## Appearance verdicts (personal inspection of complete, unmasked frames)

| Frames (byte-identical within each set) | Content checked |
|---|---|
| `lit-off-1`, `lit-on-1`, `lit-on-4-skipped` (instrumentation skipped under backpressure) | Look-dev scene: cube, sphere, thin wall, ramp with post, post row to the horizon. Consistent sun shadows with contact darkening. Warm and cool clustered point-light pools. No missing geometry, seams, banding or black/NaN pixels |
| `sprites-text-off`, `sprites-text-on` | 2× pixel grid. `ffi` ligature, joined right-to-left Arabic, CJK 中文, the intended `.notdef` box, crisp "Ag". Three nearest-sampled badges with transparent corners and dark rims. The flipped third badge draws over the second |
| `viewport-off`, `viewport-on` | The lit scene in the fractional offset rect with 12 px rounded corners over canvas `[64,80,96]`. Undistorted aspect, clean edges and corners |
| `cascades-off`, `cascades-on` | Sun-only lighting. Shadows from the cube, sphere, wall, ramp post and posts receding through the cascade ranges. No local-light tint, as intended |
| `cascades-on-unshadowed` (control) | The same scene with no cast shadows anywhere and shading intact. It confirms the shadow group's visible contribution |

Not visible in any image, but confirmed by hash: the timing-on frames 2, 3 and 5, and the
second-device control in `lit`. All are byte-identical to `lit-off-1`.

**Cross-source observation:** `lit-off-1` (`4f019b64…`), `sprites-text-off` (`bdadd584…`)
and `viewport-off` (`d6215b53…`) are byte-identical to the original 0068 renders from the
earlier source. On default features, main200 with corrected timestamps renders these
fixtures exactly as before.

## Screenshots

Unedited byte copies; each stands for its byte-identical set:

| File | SHA-256 | Stands for |
|---|---|---|
| `handoffs/0068-gpu-timestamps/screenshots-r2/lit-r1-lit-off-1.png` | `4f019b64…` | `lit-off-1`, `lit-on-1`, `lit-on-4-skipped` |
| `handoffs/0068-gpu-timestamps/screenshots-r2/sprites_text-r1-sprites-text-off.png` | `bdadd584…` | `sprites-text-off`, `-on` |
| `handoffs/0068-gpu-timestamps/screenshots-r2/viewport-r1-viewport-off.png` | `d6215b53…` | `viewport-off`, `-on` |
| `handoffs/0068-gpu-timestamps/screenshots-r2/cascades-r1-cascades-off.png` | `7049cf0a…` | `cascades-off`, `-on` |
| `handoffs/0068-gpu-timestamps/screenshots-r2/cascades-r1-cascades-on-unshadowed.png` | `2dd1d970…` | `cascades-on-unshadowed` |

All 10 PNGs remain in `incant-evidence/0068-r2/captures/<case>-r1/`.

## Helper lint (optional, CPU only, after the GPU latch)

`cargo clippy -p incant_render --test gpu_timestamp_review … -- -D warnings` is **clean**:
exit 0, no warnings, 5.5 s. The four style findings from the original run were fixed in
the reviewed r2 tools.

## Commands run (worktree root, `/opt/homebrew/bin/python3 -I -B`)

```
handoffs/0068-gpu-timestamps/tools/lease.py acquire
handoffs/0068-gpu-timestamps/tools/build.py checkout
handoffs/0068-gpu-timestamps/tools/build.py toolchain
handoffs/0068-gpu-timestamps/tools/build.py tests        # background; both cargo builds exit 0
handoffs/0068-gpu-timestamps/tools/build.py review
handoffs/0068-gpu-timestamps/tools/gpu.py list lib|model_gpu|timestamp_backend_probe|game|ui|review
handoffs/0068-gpu-timestamps/tools/gpu.py numeric
handoffs/0068-gpu-timestamps/tools/gpu.py suite lib|model_gpu|timestamp_backend_probe|game|ui
handoffs/0068-gpu-timestamps/tools/gpu.py review probe|lit|sprites_text|viewport|cascades|owner_drop
handoffs/0068-gpu-timestamps/tools/analyze.py check
handoffs/0068-gpu-timestamps/tools/gpu.py release
handoffs/0068-gpu-timestamps/tools/build.py lint         # background; clean
handoffs/0068-gpu-timestamps/tools/lease.py release
handoffs/0068-gpu-timestamps/tools/{gpu.py review lit, gpu.py numeric, build.py tests}   # refused after release
handoffs/0068-gpu-timestamps/tools/analyze.py publish
```

Every child's exact argv, constructed environment overrides, debit, hashes and outcome
are in `evidence-r2/gpu/ledger.jsonl` and `evidence-r2/gpu/post-release-ledger.jsonl`.
Receipts record only the allowlisted constant `PATH` values and per-child scratch
`TMPDIR`; `HOME` appears as a key name only. A scan of the published evidence found no
secret-like values.

## Changed paths (this session)

- `handoffs/0068-gpu-timestamps/result.md`, this final r2 report.
- `handoffs/0068-gpu-timestamps/evidence-r2/**`: 64 published receipts plus
  `manifest.json`. Text was normalized only for trailing whitespace and final blank
  lines; the manifest records the raw and committed SHA-256 of each.
- `handoffs/0068-gpu-timestamps/screenshots-r2/*.png`, 5 files.

The tools, plan, fixture, `brief.md`, `previous-result.md`, `prep-result.md` and all
historic evidence are unchanged. There was no crate, CI or `main` change, no push and no
PR. Writes outside the worktree went only to the granted `incant-evidence/0068-r2` and
the granted Cargo target.

## Limitations

- **Default features only.** Cooked shader paths are not exercised, and this is **not
  cooked acceptance**.
- **One adapter and backend:** an Apple M5 Pro on Metal. No Vulkan, DX12, browser or
  phone evidence.
- **Durations are raw correctness observations.** They are not benchmarks or frame
  budgets. Groups must not be summed or treated as strict barriers.
- **Frame-interval semantics.** See the finding above: the interval is in encoding order
  and can omit concurrently started earlier work.
- **`owner_drop` scope.** Smoke only. `queue.submit` may deliver the callback before the
  owner drop, and nothing exposes callback completion.
- **Not exercised:** a real mapping failure or device loss (CPU failed-map tests are not
  evidence of either), and the unsupported-adapter refusal (CPU-test coverage only).
- **Interior cascade overwrite** is not observable from public data. Root accepted the
  scoped evidence.
- **Conservative submission counting.** Each timing-on frame counted 2 (frame plus
  resolve), so the numeric test was debited 6 although its actual count was 5.

## Open question for Astra

1. Should `GpuFrameTiming::frame` span the minimum beginning to the maximum end over
   present groups, or should the docs state the encoding-order semantics? Evidence is in
   `evidence-r2/captures/lit-r1/report.json`, samples 2, 3 and 5.
